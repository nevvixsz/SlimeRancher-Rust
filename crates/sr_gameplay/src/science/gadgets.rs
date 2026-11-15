//! Port of placed Slime Science gadgets: `TeleporterGadget`, `LinkedSiloStorage` (`WarpDepot`),
//! `GordoSnare`, `GadgetChickenCloner`, `EchoNet`, `MedStation`, `HydroTurret`, `TamingBell`,
//! `DashPad`, `PortableWaterTap`, and `PortableSlimeBait*`.

use std::collections::HashMap;

use sr_data::DataRegistry;
use sr_ids::{GadgetId, IdentifiableId, RegionSetId, SlimeEmotion, ZoneId};
use sr_math::{Randoms, Vec3};
use sr_model::{GadgetKind, GameModel};
use sr_physics::{PhysicsWorld, PlayerMotor};

use crate::actors::spawn_physical_actor;
use crate::player::PlayerController;

pub const WARP_DEPOT_MAX_STACK: i32 = 300;
pub const SNARED_GORDO_TARGET_COUNT: i32 = 50;
pub const MED_STATION_HEAL_PER_SEC: f32 = 15.0;
pub const HYDRO_TURRET_RANGE: f32 = 14.0;
pub const TAMING_BELL_RANGE: f32 = 15.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UtilityGadgetEvents {
    pub snared_gordos: Vec<(String, IdentifiableId)>,
    pub popped_snared_gordos: Vec<(String, IdentifiableId)>,
    pub healed_amount: f32,
    pub hydro_turret_kills: Vec<i64>,
    pub tamed_slimes: Vec<i64>,
}

fn slime_to_gordo(slime: IdentifiableId) -> Option<IdentifiableId> {
    use IdentifiableId as Id;
    Some(match slime {
        Id::PinkSlime => Id::PinkGordo,
        Id::RockSlime => Id::RockGordo,
        Id::TabbySlime => Id::TabbyGordo,
        Id::PhosphorSlime => Id::PhosphorGordo,
        Id::RadSlime => Id::RadGordo,
        Id::BoomSlime => Id::BoomGordo,
        Id::HoneySlime => Id::HoneyGordo,
        Id::HunterSlime => Id::HunterGordo,
        Id::CrystalSlime => Id::CrystalGordo,
        Id::QuantumSlime => Id::QuantumGordo,
        Id::DervishSlime => Id::DervishGordo,
        Id::MosaicSlime => Id::MosaicGordo,
        Id::TangleSlime => Id::TangleGordo,
        Id::GoldSlime => Id::GoldGordo,
        _ => return None,
    })
}

fn gordo_to_slime(gordo: IdentifiableId) -> Option<IdentifiableId> {
    use IdentifiableId as Id;
    Some(match gordo {
        Id::PinkGordo => Id::PinkSlime,
        Id::RockGordo => Id::RockSlime,
        Id::TabbyGordo => Id::TabbySlime,
        Id::PhosphorGordo => Id::PhosphorSlime,
        Id::RadGordo => Id::RadSlime,
        Id::BoomGordo => Id::BoomSlime,
        Id::HoneyGordo => Id::HoneySlime,
        Id::HunterGordo => Id::HunterSlime,
        Id::CrystalGordo => Id::CrystalSlime,
        Id::QuantumGordo => Id::QuantumSlime,
        Id::DervishGordo => Id::DervishSlime,
        Id::MosaicGordo => Id::MosaicSlime,
        Id::TangleGordo => Id::TangleSlime,
        Id::GoldGordo => Id::GoldSlime,
        _ => return None,
    })
}

fn region_for_zone(z: ZoneId) -> RegionSetId {
    match z {
        ZoneId::Desert => RegionSetId::Desert,
        ZoneId::Valley | ZoneId::MochiRanch => RegionSetId::Valley,
        ZoneId::Slimulations => RegionSetId::Slimulations,
        ZoneId::ViktorLab => RegionSetId::ViktorLab,
        _ => RegionSetId::Home,
    }
}

/// Teleports the player from `from_site_id` to its paired `Teleporter*` gadget.
pub fn use_teleporter(
    model: &mut GameModel,
    motor: &mut PlayerMotor,
    site_positions: &HashMap<String, Vec3>,
    site_zones: &HashMap<String, ZoneId>,
    from_site_id: &str,
) -> Option<(String, Vec3, RegionSetId)> {
    let tp_id = model
        .gadget_sites
        .get(from_site_id)?
        .attached
        .as_ref()?
        .ident;
    if !tp_id.is_teleporter() {
        return None;
    }

    let partner_id = model
        .gadget_sites
        .iter()
        .find(|(sid, s)| {
            sid.as_str() != from_site_id
                && s.attached.as_ref().map_or(false, |g| g.ident == tp_id)
        })
        .map(|(sid, _)| sid.clone())?;

    let dest_pos = site_positions.get(&partner_id).copied().unwrap_or(Vec3::ZERO);
    let dest_region = region_for_zone(site_zones.get(&partner_id).copied().unwrap_or(ZoneId::Ranch));

    model.player.position = dest_pos + Vec3::Y * 1.2;
    model.player.curr_region_set_id = dest_region;
    motor.velocity = Vec3::ZERO;

    Some((partner_id, dest_pos, dest_region))
}

/// Deposits up to `count` items into a `WarpDepot*`, syncing both linked depots of that color.
pub fn deposit_warp_depot(
    model: &mut GameModel,
    site_id: &str,
    item_id: IdentifiableId,
    count: i32,
) -> i32 {
    if count <= 0 || item_id == IdentifiableId::None {
        return 0;
    }
    let depot_ident = {
        let Some(site) = model.gadget_sites.get(site_id) else {
            return 0;
        };
        let Some(ref g) = site.attached else {
            return 0;
        };
        if !g.ident.is_warp_depot() {
            return 0;
        }
        g.ident
    };

    let mut cur_id = IdentifiableId::None;
    let mut cur_count = 0;
    for s in model.gadget_sites.values() {
        if let Some(ref g) = s.attached {
            if g.ident == depot_ident {
                if let GadgetKind::WarpDepot(ref wd) = g.kind {
                    if wd.ammo_count > 0 && wd.ammo_id != IdentifiableId::None {
                        cur_id = wd.ammo_id;
                        cur_count = wd.ammo_count;
                        break;
                    }
                }
            }
        }
    }

    if cur_id != IdentifiableId::None && cur_id != item_id {
        return 0;
    }
    let added = count.min((WARP_DEPOT_MAX_STACK - cur_count).max(0));
    if added <= 0 {
        return 0;
    }
    let new_count = cur_count + added;
    let has_pair = model
        .gadget_sites
        .values()
        .filter(|s| s.attached.as_ref().map_or(false, |g| g.ident == depot_ident))
        .count()
        >= 2;

    for s in model.gadget_sites.values_mut() {
        if let Some(ref mut g) = s.attached {
            if g.ident == depot_ident {
                if let GadgetKind::WarpDepot(ref mut wd) = g.kind {
                    wd.is_linked = has_pair;
                    wd.ammo_id = item_id;
                    wd.ammo_count = new_count;
                }
            }
        }
    }
    added
}

/// Withdraws up to `count` items from a `WarpDepot*`, syncing both linked depots.
pub fn withdraw_warp_depot(
    model: &mut GameModel,
    site_id: &str,
    count: i32,
) -> Option<(IdentifiableId, i32)> {
    if count <= 0 {
        return None;
    }
    let depot_ident = {
        let site = model.gadget_sites.get(site_id)?;
        let g = site.attached.as_ref()?;
        if !g.ident.is_warp_depot() {
            return None;
        }
        g.ident
    };

    let (cur_id, cur_count) = {
        let s = model.gadget_sites.get(site_id)?;
        let GadgetKind::WarpDepot(ref wd) = s.attached.as_ref()?.kind else {
            return None;
        };
        if wd.ammo_count <= 0 || wd.ammo_id == IdentifiableId::None {
            return None;
        }
        (wd.ammo_id, wd.ammo_count)
    };

    let taken = count.min(cur_count);
    let rem = cur_count - taken;
    let new_id = if rem > 0 { cur_id } else { IdentifiableId::None };

    for s in model.gadget_sites.values_mut() {
        if let Some(ref mut g) = s.attached {
            if g.ident == depot_ident {
                if let GadgetKind::WarpDepot(ref mut wd) = g.kind {
                    wd.ammo_id = new_id;
                    wd.ammo_count = rem;
                }
            }
        }
    }
    Some((cur_id, taken))
}

/// Port of `GordoSnare.AttachBait`.
pub fn bait_gordo_snare(model: &mut GameModel, site_id: &str, food_id: IdentifiableId) -> bool {
    if !food_id.is_food() {
        return false;
    }
    let Some(site) = model.gadget_sites.get_mut(site_id) else {
        return false;
    };
    let Some(ref mut g) = site.attached else {
        return false;
    };
    let GadgetKind::Snare(ref mut sn) = g.kind else {
        return false;
    };
    if sn.bait_type_id != IdentifiableId::None || sn.gordo_type_id != IdentifiableId::None {
        return false;
    }
    sn.bait_type_id = food_id;
    true
}

/// Port of `GadgetDirector.CheckGordoSnares` / `GordoSnare.SnareGordo` (called at 05:00 AM).
pub fn check_gordo_snares(
    model: &mut GameModel,
    data: &DataRegistry,
    rng: &mut Randoms,
) -> Vec<(String, IdentifiableId)> {
    let mut snared = Vec::new();
    let site_ids: Vec<String> = model.gadget_sites.keys().cloned().collect();

    for sid in site_ids {
        let (gadget_id, bait) = {
            let Some(s) = model.gadget_sites.get(&sid) else {
                continue;
            };
            let Some(ref g) = s.attached else {
                continue;
            };
            let GadgetKind::Snare(ref sn) = g.kind else {
                continue;
            };
            if sn.bait_type_id == IdentifiableId::None || sn.gordo_type_id != IdentifiableId::None
            {
                continue;
            }
            (g.ident, sn.bait_type_id)
        };

        let mut favored_gordo: Option<IdentifiableId> = None;
        let mut potential_gordos: Vec<IdentifiableId> = Vec::new();

        for def in data.slimes.slimes.values() {
            if def.is_largo {
                continue;
            }
            let Some(gid) = slime_to_gordo(def.identifiable_id) else {
                continue;
            };
            if gid == IdentifiableId::PinkGordo {
                continue;
            }
            if def.diet.favorites.contains(&bait) {
                favored_gordo = Some(gid);
            } else if def
                .diet
                .major_food_groups
                .iter()
                .any(|fg| IdentifiableId::ids_in_food_group(*fg).contains(&bait))
            {
                potential_gordos.push(gid);
            }
        }

        let chosen = if gadget_id == GadgetId::GordoSnareMaster && favored_gordo.is_some() {
            favored_gordo.unwrap()
        } else {
            let (fav_w, food_w, pink_w) = match gadget_id {
                GadgetId::GordoSnareMaster => (1000.0f32, 10.0, 1.0),
                GadgetId::GordoSnareAdvanced => (50.0, 30.0, 10.0),
                _ => (15.0, 35.0, 50.0),
            };
            let mut weights: Vec<(IdentifiableId, f32)> = Vec::new();
            if let Some(fg) = favored_gordo {
                weights.push((fg, fav_w));
            }
            if !potential_gordos.is_empty() {
                let per = food_w / (potential_gordos.len() as f32);
                for pg in potential_gordos {
                    weights.push((pg, per));
                }
            }
            weights.push((IdentifiableId::PinkGordo, pink_w));

            let total: f32 = weights.iter().map(|w| w.1).sum();
            let mut roll = rng.get_in_range_f32(0.0, total);
            let mut pick = IdentifiableId::PinkGordo;
            for (gid, w) in weights {
                roll -= w;
                if roll <= 0.0 {
                    pick = gid;
                    break;
                }
            }
            pick
        };

        if let Some(s) = model.gadget_sites.get_mut(&sid) {
            if let Some(ref mut g) = s.attached {
                if let GadgetKind::Snare(ref mut sn) = g.kind {
                    sn.bait_type_id = IdentifiableId::None;
                    sn.gordo_type_id = chosen;
                    sn.gordo_eaten_count = 0;
                }
            }
        }
        if chosen == IdentifiableId::HunterGordo {
            *model
                .game_achieves
                .counts
                .entry("SNARED_HUNTER_GORDOS".to_string())
                .or_insert(0) += 1;
        }
        snared.push((sid, chosen));
    }

    snared
}

/// Feeds a Gordo caught in a `GordoSnare`; returns `(eaten, popped)` and spawns 10 base slimes on pop.
pub fn feed_snared_gordo(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    data: &DataRegistry,
    site_id: &str,
    food_id: IdentifiableId,
    pos: Vec3,
    region: RegionSetId,
    rng: &mut Randoms,
) -> (bool, bool) {
    let gordo_id = {
        let Some(s) = model.gadget_sites.get(site_id) else {
            return (false, false);
        };
        let Some(ref g) = s.attached else {
            return (false, false);
        };
        let GadgetKind::Snare(ref sn) = g.kind else {
            return (false, false);
        };
        if sn.gordo_type_id == IdentifiableId::None {
            return (false, false);
        }
        sn.gordo_type_id
    };

    let Some(base_slime) = gordo_to_slime(gordo_id) else {
        return (false, false);
    };
    let Some(def) = data.slimes.get(base_slime) else {
        return (false, false);
    };
    let eats = def.diet.favorites.contains(&food_id)
        || def
            .diet
            .major_food_groups
            .iter()
            .any(|fg| IdentifiableId::ids_in_food_group(*fg).contains(&food_id));
    if !eats {
        return (false, false);
    }

    let delta = if def.diet.favorites.contains(&food_id) {
        2
    } else {
        1
    };

    let mut popped = false;
    if let Some(s) = model.gadget_sites.get_mut(site_id) {
        if let Some(ref mut g) = s.attached {
            if let GadgetKind::Snare(ref mut sn) = g.kind {
                sn.gordo_eaten_count += delta;
                if sn.gordo_eaten_count >= SNARED_GORDO_TARGET_COUNT {
                    popped = true;
                    sn.gordo_type_id = IdentifiableId::None;
                    sn.gordo_eaten_count = 0;
                }
            }
        }
    }

    if popped {
        for _ in 0..10 {
            let vel = Vec3::new(
                rng.get_in_range_f32(-4.0, 4.0),
                4.5,
                rng.get_in_range_f32(-4.0, 4.0),
            );
            spawn_physical_actor(model, physics, base_slime, region, pos + Vec3::Y, vel);
        }
    }

    (true, popped)
}

/// Port of `GadgetChickenCloner.OnTriggerEnter`: 50% chance to spawn 2 clones of `is_meat()`.
pub fn trigger_chicken_cloner(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    chicken_actor_id: i64,
    cloner_pos: Vec3,
    region: RegionSetId,
    rng: &mut Randoms,
) -> Option<Vec<i64>> {
    let ident = model.actors.get(&chicken_actor_id)?.ident;
    if !ident.is_meat() {
        return None;
    }
    model.destroy_actor(chicken_actor_id);
    physics.remove_body(chicken_actor_id);

    if rng.get_probability(0.5) {
        let mut clones = Vec::with_capacity(2);
        for i in 0..2 {
            let sign = if i == 0 { 1.0 } else { -1.0 };
            let vel = Vec3::new(sign * 3.0, 2.5, 4.0);
            clones.push(spawn_physical_actor(
                model,
                physics,
                ident,
                region,
                cloner_pos + Vec3::Y * 0.5,
                vel,
            ));
        }
        Some(clones)
    } else {
        Some(Vec::new())
    }
}

/// Steps placed utility gadgets (`MedStation`, `HydroTurret`, `TamingBell`, `GordoSnare` 05:00 reset).
pub fn step_utility_gadgets(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    data: &DataRegistry,
    site_positions: &HashMap<String, Vec3>,
    passed_5am: bool,
    dt: f32,
    rng: &mut Randoms,
) -> UtilityGadgetEvents {
    let mut ev = UtilityGadgetEvents::default();

    if passed_5am {
        ev.snared_gordos = check_gordo_snares(model, data, rng);
    }

    let wt = model.world.world_time;
    let player_pos = model.player.position;
    let gadgets: Vec<(String, GadgetId, Vec3)> = model
        .gadget_sites
        .iter()
        .filter_map(|(sid, s)| {
            let g = s.attached.as_ref()?;
            let pos = site_positions.get(sid).copied().unwrap_or(Vec3::ZERO);
            Some((sid.clone(), g.ident, pos))
        })
        .collect();

    for (_sid, gid, gpos) in gadgets {
        match gid {
            GadgetId::MedStation | GadgetId::RapidMedStation => {
                if player_pos.distance_squared(gpos) <= 9.0
                    && model.player.curr_health < model.player.max_health as f32
                {
                    let before = model.player.curr_health;
                    let gain = (MED_STATION_HEAL_PER_SEC * dt).max(1.0) as i32;
                    PlayerController::heal(&mut model.player, gain, wt);
                    ev.healed_amount += model.player.curr_health - before;
                }
            }
            GadgetId::HydroTurret | GadgetId::SuperHydroTurret => {
                let r2 = HYDRO_TURRET_RANGE * HYDRO_TURRET_RANGE;
                let tarrs: Vec<i64> = model
                    .actors
                    .iter()
                    .filter(|(_, a)| a.ident.is_tarr() && a.pos.distance_squared(gpos) <= r2)
                    .map(|(id, _)| *id)
                    .collect();
                for tid in tarrs {
                    model.destroy_actor(tid);
                    physics.remove_body(tid);
                    ev.hydro_turret_kills.push(tid);
                }
            }
            GadgetId::TamingBell | GadgetId::EliteTamingBell => {
                let r2 = TAMING_BELL_RANGE * TAMING_BELL_RANGE;
                for (aid, actor) in &mut model.actors {
                    if actor.pos.distance_squared(gpos) <= r2 {
                        if let Some(sm) = actor.as_slime_mut() {
                            if sm.is_feral {
                                sm.is_feral = false;
                                ev.tamed_slimes.push(*aid);
                            }
                            if let Some(ag) = sm.emotion_mut(SlimeEmotion::Agitation) {
                                ag.curr_val = (ag.curr_val - 0.5 * dt).max(0.0);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ev
}
