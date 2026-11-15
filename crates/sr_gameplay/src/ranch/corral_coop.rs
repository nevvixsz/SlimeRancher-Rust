//! Port of `CorralUI`, `SlimeFeeder`, `PlortCollector`, `AirNet`, and `CoopRegion` / chicken growth.

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, LandPlotId, LandPlotUpgrade, RegionSetId};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, AmmoModel, AmmoSlot, FeedSpeed, GameModel, LandPlotModel, StorageType};
use sr_physics::{PhysicsWorld, RigidBody};

pub const FEEDER_HOURS_SLOW: f32 = 9.0;
pub const FEEDER_HOURS_NORMAL: f32 = 6.0;
pub const FEEDER_HOURS_FAST: f32 = 3.0;
pub const FEEDER_ITEMS_PER_CYCLE: i32 = 6;
pub const FEEDER_MAX_AMMO: i32 = 100;

pub const COLLECTOR_PERIOD_HOURS: f32 = 1.0;
pub const COLLECTOR_MAX_PER_SLOT: i32 = 100;

pub const AIR_NET_HIT_FORCE_TO_DESTROY: f32 = 300.0;
pub const AIR_NET_HOURS_TO_RECOVER: f32 = 0.1;
pub const AIR_NET_NEW_STRENGTH: f32 = 0.33;

pub const COOP_BASE_CAPACITY: usize = 12;
pub const COOP_DELUXE_CAPACITY: usize = 24;
pub const CHICKEN_MATURE_HOURS: f64 = 12.0;
pub const CHICKEN_REPRODUCE_HOURS: f64 = 12.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotBounds {
    pub plot_id: String,
    pub center: Vec3,
    pub half_extents: Vec3,
    pub region: RegionSetId,
}

impl PlotBounds {
    pub fn contains_xz(&self, pos: Vec3) -> bool {
        (pos.x - self.center.x).abs() <= self.half_extents.x
            && (pos.z - self.center.z).abs() <= self.half_extents.z
    }
}

/// Port of `AirNet.cs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirNetState {
    pub strength: f32,
    pub recover_start_time: f64,
}

impl Default for AirNetState {
    fn default() -> Self {
        Self {
            strength: 1.0,
            recover_start_time: 0.0,
        }
    }
}

impl AirNetState {
    pub fn is_active(&self) -> bool {
        self.strength > 0.0
    }

    pub fn on_slime_hit(&mut self, impulse_magnitude: f32, world_time: f64) {
        let dmg_per_impulse = 1.0 / AIR_NET_HIT_FORCE_TO_DESTROY;
        self.strength = (self.strength - impulse_magnitude * dmg_per_impulse).max(0.0);
        self.recover_start_time = world_time + (AIR_NET_HOURS_TO_RECOVER as f64) * 3600.0;
    }

    pub fn step(&mut self, world_time: f64, delta_world_time: f64) {
        if self.strength < 1.0 && world_time >= self.recover_start_time {
            let recover_factor = 1.0 / (AIR_NET_HOURS_TO_RECOVER * 3600.0);
            self.strength = ((self.strength + (delta_world_time as f32) * recover_factor)
                .clamp(AIR_NET_NEW_STRENGTH, 1.0))
            .min(1.0);
        }
    }
}

pub const fn feeder_cycle_hours(speed: FeedSpeed) -> f32 {
    match speed {
        FeedSpeed::Slow => FEEDER_HOURS_SLOW,
        FeedSpeed::Normal => FEEDER_HOURS_NORMAL,
        FeedSpeed::Fast => FEEDER_HOURS_FAST,
    }
}

pub fn step_feed_speed(plot: &mut LandPlotModel) -> FeedSpeed {
    plot.feeder_cycle_speed = match plot.feeder_cycle_speed {
        FeedSpeed::Normal => FeedSpeed::Fast,
        FeedSpeed::Fast => FeedSpeed::Slow,
        FeedSpeed::Slow => FeedSpeed::Normal,
    };
    plot.feeder_cycle_speed
}

/// Deposits food into a Corral or Coop auto-feeder slot (`FEEDER` upgrade required).
pub fn deposit_feeder_food(
    plot: &mut LandPlotModel,
    food_id: IdentifiableId,
    count: i32,
) -> i32 {
    if !plot.has_upgrade(LandPlotUpgrade::Feeder) || !food_id.is_food() || count <= 0 {
        return 0;
    }
    let ammo = plot
        .silo_ammo
        .entry(StorageType::Food)
        .or_insert_with(|| AmmoModel::new(1, 1));
    let slot = ammo.slots[0].get_or_insert_with(AmmoSlot::default);
    if slot.is_empty() {
        slot.id = food_id;
        slot.count = 0;
    }
    if slot.id != food_id {
        return 0;
    }
    let space = (FEEDER_MAX_AMMO - slot.count).max(0);
    let added = space.min(count);
    slot.count += added;
    added
}

/// Withdraws up to `max_take` plorts from a `PlortCollector` slot `slot_idx` (`0..4`).
pub fn withdraw_collector_plorts(
    plot: &mut LandPlotModel,
    slot_idx: usize,
    max_take: i32,
) -> Option<(IdentifiableId, i32)> {
    let ammo = plot.silo_ammo.get_mut(&StorageType::Plort)?;
    let slot = ammo.slots.get_mut(slot_idx)?.as_mut()?;
    if slot.is_empty() || max_take <= 0 {
        return None;
    }
    let id = slot.id;
    let taken = slot.count.min(max_take);
    slot.count -= taken;
    if slot.count <= 0 {
        slot.clear();
    }
    Some((id, taken))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CorralCoopEvents {
    pub feeder_ejected: Vec<(String, IdentifiableId)>,
    pub collector_stored: Vec<(String, IdentifiableId)>,
    pub chicks_matured: Vec<(i64, IdentifiableId)>,
    pub chicks_spawned: Vec<(i64, IdentifiableId)>,
}

/// Advances Corral feeders, Plort Collectors, AirNets, and Coop chicken growth/breeding.
pub fn step_corrals_and_coops(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    bounds: &[PlotBounds],
    air_nets: &mut std::collections::HashMap<String, AirNetState>,
    world_time: f64,
    delta_world_time: f64,
    rng: &mut Randoms,
) -> CorralCoopEvents {
    let mut ev = CorralCoopEvents::default();

    for b in bounds {
        let Some(plot_snapshot) = model.land_plots.get(&b.plot_id).cloned() else {
            continue;
        };

        // 1. Auto-Feeder (Corral & Coop)
        if matches!(plot_snapshot.type_id, LandPlotId::Corral | LandPlotId::Coop)
            && plot_snapshot.has_upgrade(LandPlotUpgrade::Feeder)
        {
            let mut eject_food: Option<IdentifiableId> = None;
            if let Some(plot) = model.land_plots.get_mut(&b.plot_id) {
                while world_time >= plot.next_feeding_time {
                    plot.remaining_feed_operations += FEEDER_ITEMS_PER_CYCLE;
                    plot.next_feeding_time +=
                        (feeder_cycle_hours(plot.feeder_cycle_speed) as f64) * 3600.0;
                }
                if plot.remaining_feed_operations > 0 {
                    if let Some(food_ammo) = plot.silo_ammo.get_mut(&StorageType::Food) {
                        if let Some(Some(slot)) = food_ammo.slots.get_mut(0) {
                            if !slot.is_empty() {
                                eject_food = Some(slot.id);
                                slot.count -= 1;
                                if slot.count <= 0 {
                                    slot.clear();
                                }
                            }
                        }
                    }
                    plot.remaining_feed_operations = (plot.remaining_feed_operations - 1).max(0);
                }
            }
            if let Some(fid) = eject_food {
                let jitter = Vec3::new(
                    rng.get_in_range_f32(-1.2, 1.2),
                    1.5,
                    rng.get_in_range_f32(-1.2, 1.2),
                );
                let pos = b.center + jitter;
                let aid = model.create_actor(fid, b.region, pos, Quat::IDENTITY);
                physics.insert_body(RigidBody {
                    actor_id: aid,
                    pos,
                    vel: Vec3::new(jitter.x, 1.2, jitter.z),
                    radius: 0.25,
                    ..Default::default()
                });
                ev.feeder_ejected.push((b.plot_id.clone(), fid));
            }
        }

        // 2. Corral PlortCollector & AirNet
        if plot_snapshot.type_id == LandPlotId::Corral {
            if plot_snapshot.has_upgrade(LandPlotUpgrade::AirNet) {
                let net = air_nets.entry(b.plot_id.clone()).or_default();
                net.step(world_time, delta_world_time);
            }

            if plot_snapshot.has_upgrade(LandPlotUpgrade::PlortCollector)
                && world_time >= plot_snapshot.collector_next_time
            {
                if let Some(plot) = model.land_plots.get_mut(&b.plot_id) {
                    plot.collector_next_time =
                        world_time + (COLLECTOR_PERIOD_HOURS as f64) * 3600.0;
                }

                let plorts_in_corral: Vec<(i64, IdentifiableId)> = model
                    .actors
                    .iter()
                    .filter(|(_, a)| a.ident.is_plort() && b.contains_xz(a.pos))
                    .map(|(id, a)| (*id, a.ident))
                    .collect();

                for (pid, p_ident) in plorts_in_corral {
                    let stored = if let Some(plot) = model.land_plots.get_mut(&b.plot_id) {
                        store_in_collector(plot, p_ident)
                    } else {
                        false
                    };
                    if stored {
                        model.destroy_actor(pid);
                        physics.remove_body(pid);
                        ev.collector_stored.push((b.plot_id.clone(), p_ident));
                    }
                }
            }
        }

        // 3. Coop Chicken Maturation & Breeding
        if plot_snapshot.type_id == LandPlotId::Coop {
            let has_vitamizer = plot_snapshot.has_upgrade(LandPlotUpgrade::Vitamizer);
            let is_deluxe = plot_snapshot.has_upgrade(LandPlotUpgrade::DeluxeCoop);
            let cap = if is_deluxe {
                COOP_DELUXE_CAPACITY
            } else {
                COOP_BASE_CAPACITY
            };
            let speed_mult = if has_vitamizer { 2.0 } else { 1.0 };

            let mut adults = 0usize;
            let mut has_roostro = false;
            let mut hens: Vec<(i64, IdentifiableId, Vec3)> = Vec::new();
            let mut chicks_to_mature: Vec<(i64, IdentifiableId)> = Vec::new();

            for (&aid, actor) in &mut model.actors {
                if !b.contains_xz(actor.pos) {
                    continue;
                }
                if actor.ident == IdentifiableId::Rooster {
                    has_roostro = true;
                    adults += 1;
                } else if actor.ident.is_meat() {
                    if matches!(
                        actor.ident,
                        IdentifiableId::Chick
                            | IdentifiableId::BriarChick
                            | IdentifiableId::PaintedChick
                            | IdentifiableId::StonyChick
                    ) {
                        if let ActorKind::Animal(ref mut am) = actor.kind {
                            if am.transform_time == 0.0 {
                                am.transform_time =
                                    world_time + (CHICKEN_MATURE_HOURS * 3600.0) / speed_mult;
                            } else if world_time >= am.transform_time {
                                let adult_id = mature_chick_id(actor.ident, rng);
                                chicks_to_mature.push((aid, adult_id));
                            }
                        }
                    } else {
                        adults += 1;
                        if matches!(
                            actor.ident,
                            IdentifiableId::Hen
                                | IdentifiableId::BriarHen
                                | IdentifiableId::PaintedHen
                                | IdentifiableId::StonyHen
                        ) {
                            hens.push((aid, actor.ident, actor.pos));
                        }
                    }
                }
            }

            for (cid, adult_id) in chicks_to_mature {
                if let Some(a) = model.actors.get_mut(&cid) {
                    a.ident = adult_id;
                    if let ActorKind::Animal(ref mut am) = a.kind {
                        am.transform_time = 0.0;
                    }
                    ev.chicks_matured.push((cid, adult_id));
                }
            }

            if has_roostro && adults < cap {
                for (hid, h_ident, h_pos) in hens {
                    let should_lay = if let Some(a) = model.actors.get_mut(&hid) {
                        if let ActorKind::Animal(ref mut am) = a.kind {
                            if am.next_reproduce_time == 0.0 {
                                am.next_reproduce_time =
                                    world_time + (CHICKEN_REPRODUCE_HOURS * 3600.0) / speed_mult;
                                false
                            } else if world_time >= am.next_reproduce_time {
                                am.next_reproduce_time =
                                    world_time + (CHICKEN_REPRODUCE_HOURS * 3600.0) / speed_mult;
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if should_lay {
                        let chick_id = chick_for_hen(h_ident);
                        let pos = h_pos + Vec3::new(0.25, 0.25, 0.25);
                        let cid = model.create_actor(chick_id, b.region, pos, Quat::IDENTITY);
                        physics.insert_body(RigidBody {
                            actor_id: cid,
                            pos,
                            radius: 0.2,
                            ..Default::default()
                        });
                        ev.chicks_spawned.push((cid, chick_id));
                        adults += 1;
                        if adults >= cap {
                            break;
                        }
                    }
                }
            }
        }
    }

    ev
}

fn store_in_collector(plot: &mut LandPlotModel, plort: IdentifiableId) -> bool {
    let ammo = plot
        .silo_ammo
        .entry(StorageType::Plort)
        .or_insert_with(|| AmmoModel::new(4, 4));
    // Try matching existing slot first
    for opt in &mut ammo.slots {
        if let Some(slot) = opt {
            if !slot.is_empty() && slot.id == plort && slot.count < COLLECTOR_MAX_PER_SLOT {
                slot.count += 1;
                return true;
            }
        }
    }
    // Otherwise take first empty slot
    for opt in &mut ammo.slots {
        if opt.as_ref().map_or(true, |s| s.is_empty()) {
            *opt = Some(AmmoSlot {
                id: plort,
                count: 1,
                emotions: Vec::new(),
            });
            return true;
        }
    }
    false
}

fn mature_chick_id(chick: IdentifiableId, rng: &mut Randoms) -> IdentifiableId {
    if rng.get_probability(0.15) {
        return IdentifiableId::Roostro;
    }
    match chick {
        IdentifiableId::BriarChick => IdentifiableId::BriarHen,
        IdentifiableId::PaintedChick => IdentifiableId::PaintedHen,
        IdentifiableId::StonyChick => IdentifiableId::StonyHen,
        _ => IdentifiableId::Hen,
    }
}

fn chick_for_hen(hen: IdentifiableId) -> IdentifiableId {
    match hen {
        IdentifiableId::BriarHen => IdentifiableId::BriarChick,
        IdentifiableId::PaintedHen => IdentifiableId::PaintedChick,
        IdentifiableId::StonyHen => IdentifiableId::StonyChick,
        _ => IdentifiableId::Chick,
    }
}
