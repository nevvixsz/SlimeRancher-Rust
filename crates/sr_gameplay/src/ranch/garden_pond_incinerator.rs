//! Port of `SpawnResource.cs`, `GardenCatcher.cs`, `PondUI.cs`, `IncineratorUI.cs`,
//! `FillableAshSource.cs`, and `SlimeEatAsh.cs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, LandPlotId, LandPlotUpgrade};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, LandPlotModel, ResourceCycleState, SpawnResourceId};
use sr_physics::{PhysicsWorld, RigidBody};

use crate::ranch::corral_coop::PlotBounds;
use crate::ranch::plots::upgrade_to_deluxe_crop;

pub const MAX_STORED_WATER: f32 = 3.0;
pub const WATER_EVAP_PER_HOUR: f32 = 0.13043478;
pub const GARDEN_SPAWN_MIN_HOURS: f32 = 10.0;
pub const GARDEN_SPAWN_MAX_HOURS: f32 = 14.0;
pub const GARDEN_EDIBLE_HOURS: f64 = 24.0;
pub const GARDEN_ROTTEN_HOURS: f64 = 6.0;
pub const MAX_ASH_UNITS: f32 = 20.0;
pub const FIRE_SLIME_ASH_EAT_INTERVAL: f64 = 3.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GardenRuntimeState {
    pub stored_water: f32,
    pub next_spawn_time: f64,
    pub next_scare_time: f64,
}

impl Default for GardenRuntimeState {
    fn default() -> Self {
        Self {
            stored_water: 0.0,
            next_spawn_time: 0.0,
            next_scare_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GardenPondIncineratorEvents {
    pub crops_spawned: Vec<(String, IdentifiableId, usize)>,
    pub incinerated_actors: Vec<(String, IdentifiableId)>,
    pub fire_plorts_produced: Vec<i64>,
    pub scared_slimes: Vec<i64>,
}

/// Maps a plantable fruit/veggie `IdentifiableId` to its `SpawnResourceId`.
pub fn crop_for_item(item: IdentifiableId) -> Option<(SpawnResourceId, IdentifiableId)> {
    let pair = match item {
        IdentifiableId::CuberryFruit => (SpawnResourceId::CuberryTree, IdentifiableId::CuberryFruit),
        IdentifiableId::MangoFruit => (SpawnResourceId::MangoTree, IdentifiableId::MangoFruit),
        IdentifiableId::CarrotVeggie => (SpawnResourceId::CarrotPatch, IdentifiableId::CarrotVeggie),
        IdentifiableId::OcaocaVeggie => (SpawnResourceId::OcaocaPatch, IdentifiableId::OcaocaVeggie),
        IdentifiableId::PearFruit => (SpawnResourceId::PearTree, IdentifiableId::PearFruit),
        IdentifiableId::PogoFruit => (SpawnResourceId::PogoTree, IdentifiableId::PogoFruit),
        IdentifiableId::ParsnipVeggie => {
            (SpawnResourceId::ParsnipPatch, IdentifiableId::ParsnipVeggie)
        }
        IdentifiableId::BeetVeggie => (SpawnResourceId::BeetPatch, IdentifiableId::BeetVeggie),
        IdentifiableId::OnionVeggie => (SpawnResourceId::OnionPatch, IdentifiableId::OnionVeggie),
        IdentifiableId::LemonFruit => (SpawnResourceId::LemonTree, IdentifiableId::LemonFruit),
        IdentifiableId::GingerVeggie => (SpawnResourceId::GingerPatch, IdentifiableId::GingerVeggie),
        _ => return None,
    };
    Some(pair)
}

/// Plants a crop in a `Garden` plot (automatically selecting `Dlx` if `DeluxeGarden` is installed).
pub fn plant_garden_crop(
    plot: &mut LandPlotModel,
    garden_state: &mut GardenRuntimeState,
    item: IdentifiableId,
    world_time: f64,
) -> bool {
    if plot.type_id != LandPlotId::Garden {
        return false;
    }
    let Some((base_sr, primary_item)) = crop_for_item(item) else {
        return false;
    };
    plot.attached_id = if plot.has_upgrade(LandPlotUpgrade::DeluxeGarden) {
        upgrade_to_deluxe_crop(base_sr)
    } else {
        base_sr
    };
    plot.attached_resource_id = primary_item;
    garden_state.next_spawn_time = world_time + (GARDEN_SPAWN_MIN_HOURS as f64) * 3600.0;
    true
}

pub fn clear_garden_crop(plot: &mut LandPlotModel) {
    plot.attached_id = SpawnResourceId::None;
    plot.attached_resource_id = IdentifiableId::None;
}

pub fn water_garden(state: &mut GardenRuntimeState, amount: f32) {
    state.stored_water = (state.stored_water + amount).clamp(0.0, MAX_STORED_WATER);
}

pub fn step_gardens_ponds_incinerators(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    bounds: &[PlotBounds],
    gardens: &mut HashMap<String, GardenRuntimeState>,
    ash_eat_timers: &mut HashMap<i64, f64>,
    world_time: f64,
    delta_world_time: f64,
    elapsed_time: f64,
    rng: &mut Randoms,
) -> GardenPondIncineratorEvents {
    let mut ev = GardenPondIncineratorEvents::default();
    let delta_hours = delta_world_time * 0.00027777778450399637;

    for b in bounds {
        let Some(plot) = model.land_plots.get(&b.plot_id).cloned() else {
            continue;
        };

        match plot.type_id {
            LandPlotId::Garden => {
                let gstate = gardens.entry(b.plot_id.clone()).or_default();
                if delta_hours > 0.0 {
                    gstate.stored_water =
                        (gstate.stored_water - (WATER_EVAP_PER_HOUR as f64 * delta_hours) as f32)
                            .clamp(0.0, MAX_STORED_WATER);
                }

                // Scareslime upgrade pushes curious slimes out of the garden
                if plot.has_upgrade(LandPlotUpgrade::Scareslime) && elapsed_time >= gstate.next_scare_time {
                    gstate.next_scare_time = elapsed_time + 1.0;
                    for (&aid, actor) in &model.actors {
                        if actor.ident.is_slime() && !actor.ident.is_tarr() && b.contains_xz(actor.pos)
                        {
                            let away = (actor.pos - b.center).normalize_or_zero();
                            let push = if away.length_squared() < 0.01 {
                                Vec3::new(1.0, 0.5, 0.0)
                            } else {
                                away + Vec3::Y * 0.5
                            };
                            physics.add_force(aid, push * 10.0);
                            ev.scared_slimes.push(aid);
                        }
                    }
                }

                if plot.attached_id != SpawnResourceId::None
                    && plot.attached_resource_id != IdentifiableId::None
                {
                    let is_watered =
                        plot.has_upgrade(LandPlotUpgrade::Sprinkler) || gstate.stored_water > 0.0;
                    if gstate.next_spawn_time == 0.0 {
                        gstate.next_spawn_time =
                            world_time + (GARDEN_SPAWN_MIN_HOURS as f64) * 3600.0;
                    }
                    if world_time >= gstate.next_spawn_time {
                        let interval_hours = rng
                            .get_in_range_f32(GARDEN_SPAWN_MIN_HOURS, GARDEN_SPAWN_MAX_HOURS)
                            * if is_watered { 0.5 } else { 1.0 };
                        gstate.next_spawn_time = world_time + (interval_hours.max(1.0) as f64) * 3600.0;

                        let yield_count = if plot.has_upgrade(LandPlotUpgrade::DeluxeGarden) {
                            rng.get_in_range_i32(15, 21) as usize
                        } else if plot.has_upgrade(LandPlotUpgrade::Soil) {
                            rng.get_in_range_i32(12, 16) as usize
                        } else {
                            rng.get_in_range_i32(8, 12) as usize
                        };

                        let edible_mult = if plot.has_upgrade(LandPlotUpgrade::MiracleMix) {
                            2.0
                        } else {
                            1.0
                        };

                        for _ in 0..yield_count {
                            let jitter = Vec3::new(
                                rng.get_in_range_f32(-b.half_extents.x * 0.7, b.half_extents.x * 0.7),
                                0.5,
                                rng.get_in_range_f32(-b.half_extents.z * 0.7, b.half_extents.z * 0.7),
                            );
                            let pos = b.center + jitter;
                            let cid = model.create_actor(
                                plot.attached_resource_id,
                                b.region,
                                pos,
                                Quat::IDENTITY,
                            );
                            if let Some(a) = model.actors.get_mut(&cid) {
                                if let ActorKind::Produce(ref mut pm) = a.kind {
                                    pm.state = ResourceCycleState::Edible;
                                    pm.progress_time =
                                        world_time + GARDEN_EDIBLE_HOURS * 3600.0 * edible_mult;
                                }
                            }
                            physics.insert_body(RigidBody {
                                actor_id: cid,
                                pos,
                                radius: 0.25,
                                ..Default::default()
                            });
                        }
                        ev.crops_spawned
                            .push((b.plot_id.clone(), plot.attached_resource_id, yield_count));
                    }
                }
            }
            LandPlotId::Incinerator => {
                let has_trough = plot.has_upgrade(LandPlotUpgrade::AshTrough);
                let mut to_burn: Vec<(i64, IdentifiableId)> = Vec::new();
                let mut fire_slimes: Vec<(i64, Vec3)> = Vec::new();

                for (&aid, actor) in &model.actors {
                    if !b.contains_xz(actor.pos) || actor.pos.y > b.center.y + 2.5 {
                        continue;
                    }
                    if actor.ident == IdentifiableId::FireSlime {
                        fire_slimes.push((aid, actor.pos));
                    } else if !actor.ident.is_toy() {
                        to_burn.push((aid, actor.ident));
                    }
                }

                let mut ash_added = 0.0f32;
                for (aid, ident) in to_burn {
                    model.destroy_actor(aid);
                    physics.remove_body(aid);
                    if has_trough && (ident.is_food() || ident.is_meat()) {
                        ash_added += 1.0;
                    }
                    ev.incinerated_actors.push((b.plot_id.clone(), ident));
                }

                if ash_added > 0.0 {
                    if let Some(pm) = model.land_plots.get_mut(&b.plot_id) {
                        pm.ash_units = (pm.ash_units + ash_added).min(MAX_ASH_UNITS);
                    }
                }

                // Fire slimes eat ash from AshTrough every 3.0s
                if has_trough {
                    for (fid, fpos) in fire_slimes {
                        let next_eat = ash_eat_timers.entry(fid).or_insert(0.0);
                        if elapsed_time >= *next_eat {
                            let can_eat = model
                                .land_plots
                                .get(&b.plot_id)
                                .map_or(false, |p| p.ash_units >= 1.0);
                            if can_eat {
                                *next_eat = elapsed_time + FIRE_SLIME_ASH_EAT_INTERVAL;
                                if let Some(pm) = model.land_plots.get_mut(&b.plot_id) {
                                    pm.ash_units = (pm.ash_units - 1.0).max(0.0);
                                }
                                let p_pos = fpos + Vec3::Y * 0.4;
                                let pid = model.create_actor(
                                    IdentifiableId::FirePlort,
                                    b.region,
                                    p_pos,
                                    Quat::IDENTITY,
                                );
                                physics.insert_body(RigidBody {
                                    actor_id: pid,
                                    pos: p_pos,
                                    vel: Vec3::new(0.0, 2.5, 0.0),
                                    radius: 0.25,
                                    ..Default::default()
                                });
                                ev.fire_plorts_produced.push(pid);
                            }
                        }
                    }
                }
            }
            LandPlotId::Pond => {
                // PuddleSlimes inside a Pond stay calmed and hydrated
                for actor in model.actors.values_mut() {
                    if actor.ident == IdentifiableId::PuddleSlime && b.contains_xz(actor.pos) {
                        if let ActorKind::Slime(ref mut sm) = actor.kind {
                            sm.agitation.curr_val = (sm.agitation.curr_val - 0.1 * (delta_hours as f32)).max(0.0);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Age produce (Edible -> Rotten -> Destroy)
    let mut expired_produce = Vec::new();
    for (&aid, actor) in &mut model.actors {
        if let ActorKind::Produce(ref mut pm) = actor.kind {
            if !pm.progress_time.is_nan() && world_time >= pm.progress_time {
                match pm.state {
                    ResourceCycleState::Edible => {
                        pm.state = ResourceCycleState::Rotten;
                        pm.progress_time = world_time + GARDEN_ROTTEN_HOURS * 3600.0;
                    }
                    ResourceCycleState::Rotten => {
                        expired_produce.push(aid);
                    }
                    _ => {}
                }
            }
        }
    }
    for aid in expired_produce {
        model.destroy_actor(aid);
        physics.remove_body(aid);
    }

    ev
}
