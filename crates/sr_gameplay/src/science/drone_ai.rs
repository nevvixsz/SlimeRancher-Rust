//! Port of `Drone.cs`, `DroneSubbehaviourPlexer.cs`, `DronePather.cs`,
//! `DroneProgramSource*.cs`, `DroneProgramDestination*.cs`, and `DroneFastForwarder.cs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GadgetId, IdentifiableId, LandPlotId};
use sr_math::{Randoms, Vec3};
use sr_model::{DroneModel, GadgetKind, GameModel, StorageType};
use sr_physics::PhysicsWorld;

use crate::economy::{deposit_plorts, EconomyDirector};
use crate::ranch::{
    deposit_feeder_food, withdraw_collector_plorts, PlotBounds, RanchRuntime, SiloController,
    MAX_ASH_UNITS,
};
use crate::science::director::GadgetDirector;
use crate::science::drones::{
    drone_has_battery, DroneDestKind, DroneSourceKind, DRONE_AMMO_CAPACITY,
};

pub const DRONE_FLY_SPEED: f32 = 12.0;
pub const DRONE_FASTFORWARD_CYCLE_SECS: f64 = 7200.0;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DronePhase {
    #[default]
    Rest,
    Idle,
    MovingToSource {
        program_idx: usize,
        target_pos: [i32; 3],
    },
    MovingToDest {
        program_idx: usize,
        target_pos: [i32; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DroneStepEvents {
    pub gathered: Vec<(String, IdentifiableId, i32)>,
    pub delivered: Vec<(String, DroneDestKind, IdentifiableId, i32)>,
    pub plorts_sold_currency: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DroneAiSystem {
    pub phases: HashMap<String, DronePhase>,
    pub next_program_idx: HashMap<String, usize>,
    pub market_pos: Option<Vec3>,
    pub refinery_pos: Option<Vec3>,
}

impl DroneAiSystem {
    fn target_matches(prog_target: IdentifiableId, item: IdentifiableId) -> bool {
        if item == IdentifiableId::None {
            return false;
        }
        if prog_target == IdentifiableId::None {
            return true;
        }
        prog_target == item
    }

    fn item_allowed_for_source(src: DroneSourceKind, item: IdentifiableId) -> bool {
        match src {
            DroneSourceKind::PlortCollector => item.is_plort(),
            DroneSourceKind::Corral | DroneSourceKind::Pond | DroneSourceKind::Incinerator => {
                item.is_plort()
            }
            DroneSourceKind::Garden => item.is_fruit() || item.is_veggie(),
            DroneSourceKind::Coop => item.is_meat(),
            DroneSourceKind::Silo | DroneSourceKind::FreeRange | DroneSourceKind::OutsidePlots => {
                item.is_food() || item.is_plort()
            }
            DroneSourceKind::None => false,
        }
    }

    fn bounds_for_plot<'a>(bounds: &'a [PlotBounds], plot_id: &str) -> Option<&'a PlotBounds> {
        bounds.iter().find(|b| b.plot_id == plot_id)
    }

    /// Attempts to gather up to `DRONE_AMMO_CAPACITY` items from `src` matching `prog_target`.
    fn try_gather(
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        ranch_bounds: &[PlotBounds],
        src: DroneSourceKind,
        prog_target: IdentifiableId,
        carry: &mut (IdentifiableId, i32),
    ) -> Option<(Vec3, IdentifiableId, i32)> {
        if src == DroneSourceKind::None || carry.1 >= DRONE_AMMO_CAPACITY {
            return None;
        }
        let wanted_id = carry.0;
        let max_take = DRONE_AMMO_CAPACITY - carry.1;

        match src {
            DroneSourceKind::PlortCollector => {
                let plot_ids: Vec<String> = model
                    .land_plots
                    .iter()
                    .filter(|(_, p)| p.type_id == LandPlotId::Corral)
                    .map(|(id, _)| id.clone())
                    .collect();
                for pid in plot_ids {
                    let pos = Self::bounds_for_plot(ranch_bounds, &pid)
                        .map_or(Vec3::ZERO, |b| b.center);
                    let Some(plot) = model.land_plots.get_mut(&pid) else {
                        continue;
                    };
                    for slot_idx in 0..4 {
                        let peek = plot
                            .silo_ammo
                            .get(&StorageType::Plort)
                            .and_then(|a| a.slots.get(slot_idx))
                            .and_then(|s| s.as_ref())
                            .map(|s| (s.id, s.count));
                        if let Some((id, cnt)) = peek {
                            if cnt > 0
                                && Self::target_matches(prog_target, id)
                                && (wanted_id == IdentifiableId::None || wanted_id == id)
                            {
                                if let Some((got_id, got_cnt)) =
                                    withdraw_collector_plorts(plot, slot_idx, max_take)
                                {
                                    carry.0 = got_id;
                                    carry.1 += got_cnt;
                                    return Some((pos, got_id, got_cnt));
                                }
                            }
                        }
                    }
                }
                None
            }
            DroneSourceKind::Silo => {
                let plot_ids: Vec<String> = model
                    .land_plots
                    .iter()
                    .filter(|(_, p)| p.type_id == LandPlotId::Silo)
                    .map(|(id, _)| id.clone())
                    .collect();
                for pid in plot_ids {
                    let pos = Self::bounds_for_plot(ranch_bounds, &pid)
                        .map_or(Vec3::ZERO, |b| b.center);
                    let Some(plot) = model.land_plots.get_mut(&pid) else {
                        continue;
                    };
                    let usable = SiloController::usable_slot_count(plot);
                    for slot_idx in 0..usable {
                        let peek = plot
                            .silo_ammo
                            .get(&StorageType::NonSlimes)
                            .and_then(|a| a.slots.get(slot_idx))
                            .and_then(|s| s.as_ref())
                            .map(|s| (s.id, s.count));
                        if let Some((id, cnt)) = peek {
                            if cnt > 0
                                && Self::target_matches(prog_target, id)
                                && (wanted_id == IdentifiableId::None || wanted_id == id)
                            {
                                if let Some((got_id, got_cnt)) =
                                    SiloController::withdraw_from_slot(plot, slot_idx, max_take)
                                {
                                    carry.0 = got_id;
                                    carry.1 += got_cnt;
                                    return Some((pos, got_id, got_cnt));
                                }
                            }
                        }
                    }
                }
                None
            }
            DroneSourceKind::Corral
            | DroneSourceKind::Garden
            | DroneSourceKind::Coop
            | DroneSourceKind::Pond
            | DroneSourceKind::Incinerator
            | DroneSourceKind::FreeRange
            | DroneSourceKind::OutsidePlots => {
                let required_plot_type = match src {
                    DroneSourceKind::Corral => Some(LandPlotId::Corral),
                    DroneSourceKind::Garden => Some(LandPlotId::Garden),
                    DroneSourceKind::Coop => Some(LandPlotId::Coop),
                    DroneSourceKind::Pond => Some(LandPlotId::Pond),
                    DroneSourceKind::Incinerator => Some(LandPlotId::Incinerator),
                    _ => None,
                };

                let mut matching_actors: Vec<(i64, IdentifiableId, Vec3)> = Vec::new();
                for (&aid, actor) in &model.actors {
                    if !Self::item_allowed_for_source(src, actor.ident)
                        || !Self::target_matches(prog_target, actor.ident)
                    {
                        continue;
                    }
                    if wanted_id != IdentifiableId::None && actor.ident != wanted_id {
                        continue;
                    }
                    let in_matching_plot = if let Some(pt) = required_plot_type {
                        ranch_bounds.iter().any(|b| {
                            b.contains_xz(actor.pos)
                                && model
                                    .land_plots
                                    .get(&b.plot_id)
                                    .map_or(false, |p| p.type_id == pt)
                        })
                    } else if src == DroneSourceKind::FreeRange {
                        !ranch_bounds.iter().any(|b| b.contains_xz(actor.pos))
                    } else {
                        // OutsidePlots: anywhere outside plots or in ranch
                        !ranch_bounds.iter().any(|b| b.contains_xz(actor.pos))
                    };
                    if in_matching_plot {
                        matching_actors.push((aid, actor.ident, actor.pos));
                    }
                }
                matching_actors.sort_by_key(|x| x.0);
                if matching_actors.is_empty() {
                    return None;
                }
                let first_id = matching_actors[0].1;
                let first_pos = matching_actors[0].2;
                let mut gathered = 0;
                for (aid, ident, _) in matching_actors {
                    if ident != first_id || gathered >= max_take {
                        continue;
                    }
                    model.destroy_actor(aid);
                    physics.remove_body(aid);
                    gathered += 1;
                }
                if gathered > 0 {
                    carry.0 = first_id;
                    carry.1 += gathered;
                    Some((first_pos, first_id, gathered))
                } else {
                    None
                }
            }
            DroneSourceKind::None => None,
        }
    }

    /// Attempts to deliver carried items to `dest`.
    fn try_deliver(
        &self,
        model: &mut GameModel,
        ranch_bounds: &[PlotBounds],
        econ: &EconomyDirector,
        mode: &GameModeSettings,
        curr_hour: f32,
        dest: DroneDestKind,
        carry: &mut (IdentifiableId, i32),
        rng: &mut Randoms,
    ) -> Option<(Vec3, IdentifiableId, i32, i32)> {
        if dest == DroneDestKind::None || carry.1 <= 0 || carry.0 == IdentifiableId::None {
            return None;
        }
        let item_id = carry.0;
        let amount = carry.1;

        match dest {
            DroneDestKind::PlortMarket => {
                if econ.get_curr_value(item_id).is_none() || econ.is_market_shutdown(mode, curr_hour)
                {
                    return None;
                }
                let resp = deposit_plorts(
                    econ, model, mode, item_id, amount, curr_hour, 1.0, false, rng,
                );
                if resp.deposits > 0 {
                    carry.1 -= resp.deposits;
                    if carry.1 <= 0 {
                        carry.0 = IdentifiableId::None;
                    }
                    let pos = self.market_pos.unwrap_or(Vec3::ZERO);
                    return Some((pos, item_id, resp.deposits, resp.currency));
                }
                None
            }
            DroneDestKind::Refinery => {
                let added = GadgetDirector::add_to_refinery(model, item_id, amount);
                if added > 0 {
                    carry.1 -= added;
                    if carry.1 <= 0 {
                        carry.0 = IdentifiableId::None;
                    }
                    let pos = self.refinery_pos.unwrap_or(Vec3::ZERO);
                    return Some((pos, item_id, added, 0));
                }
                None
            }
            DroneDestKind::Silo => {
                let plot_ids: Vec<String> = model
                    .land_plots
                    .iter()
                    .filter(|(_, p)| p.type_id == LandPlotId::Silo)
                    .map(|(id, _)| id.clone())
                    .collect();
                for pid in plot_ids {
                    let pos = Self::bounds_for_plot(ranch_bounds, &pid)
                        .map_or(Vec3::ZERO, |b| b.center);
                    let Some(plot) = model.land_plots.get_mut(&pid) else {
                        continue;
                    };
                    let added = SiloController::deposit_any(plot, item_id, amount);
                    if added > 0 {
                        carry.1 -= added;
                        if carry.1 <= 0 {
                            carry.0 = IdentifiableId::None;
                        }
                        return Some((pos, item_id, added, 0));
                    }
                }
                None
            }
            DroneDestKind::Feeder => {
                if !item_id.is_food() {
                    return None;
                }
                let plot_ids: Vec<String> = model
                    .land_plots
                    .iter()
                    .filter(|(_, p)| {
                        p.type_id == LandPlotId::Corral || p.type_id == LandPlotId::Coop
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                for pid in plot_ids {
                    let pos = Self::bounds_for_plot(ranch_bounds, &pid)
                        .map_or(Vec3::ZERO, |b| b.center);
                    let Some(plot) = model.land_plots.get_mut(&pid) else {
                        continue;
                    };
                    let added = deposit_feeder_food(plot, item_id, amount);
                    if added > 0 {
                        carry.1 -= added;
                        if carry.1 <= 0 {
                            carry.0 = IdentifiableId::None;
                        }
                        return Some((pos, item_id, added, 0));
                    }
                }
                None
            }
            DroneDestKind::Incinerator => {
                let plot_ids: Vec<String> = model
                    .land_plots
                    .iter()
                    .filter(|(_, p)| p.type_id == LandPlotId::Incinerator)
                    .map(|(id, _)| id.clone())
                    .collect();
                for pid in plot_ids {
                    let pos = Self::bounds_for_plot(ranch_bounds, &pid)
                        .map_or(Vec3::ZERO, |b| b.center);
                    let Some(plot) = model.land_plots.get_mut(&pid) else {
                        continue;
                    };
                    plot.ashes_remaining = (plot.ashes_remaining + amount as f32).min(MAX_ASH_UNITS);
                    carry.1 = 0;
                    carry.0 = IdentifiableId::None;
                    return Some((pos, item_id, amount, 0));
                }
                None
            }
            DroneDestKind::None => None,
        }
    }

    fn execute_one_drone_step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        ranch_bounds: &[PlotBounds],
        econ: &EconomyDirector,
        mode: &GameModeSettings,
        curr_hour: f32,
        site_id: &str,
        site_pos: Vec3,
        dt: f32,
        rng: &mut Randoms,
        ev: &mut DroneStepEvents,
    ) {
        let wt = model.world.world_time;
        let Some(site) = model.gadget_sites.get(site_id) else {
            return;
        };
        let Some(ref g) = site.attached else {
            return;
        };
        let GadgetKind::Drone(ref dm) = g.kind else {
            return;
        };
        if !drone_has_battery(dm, wt) || dm.programs.is_empty() {
            self.phases.insert(site_id.to_string(), DronePhase::Rest);
            if let Some(s) = model.gadget_sites.get_mut(site_id) {
                if let Some(ref mut gg) = s.attached {
                    if let GadgetKind::Drone(ref mut d) = gg.kind {
                        d.no_clip_position = site_pos;
                    }
                }
            }
            return;
        }

        let programs = dm.programs.clone();
        let mut carry = (dm.ammo_id, dm.ammo_count);
        let mut pos = dm.no_clip_position;
        if pos == Vec3::ZERO && site_pos != Vec3::ZERO {
            pos = site_pos;
        }

        let start_idx = self.next_program_idx.get(site_id).copied().unwrap_or(0) % programs.len();

        // 1. If carrying items, deliver to matching program destination
        if carry.1 > 0 && carry.0 != IdentifiableId::None {
            let mut delivered_any = false;
            for offset in 0..programs.len() {
                let idx = (start_idx + offset) % programs.len();
                let dest = DroneDestKind::from_key(&programs[idx].destination);
                if let Some((dpos, item, cnt, cash)) =
                    self.try_deliver(model, ranch_bounds, econ, mode, curr_hour, dest, &mut carry, rng)
                {
                    let dir = dpos - pos;
                    let dist = dir.length();
                    if dist > 0.5 {
                        pos += (dir / dist) * (DRONE_FLY_SPEED * dt).min(dist);
                    } else {
                        pos = dpos;
                    }
                    ev.delivered.push((site_id.to_string(), dest, item, cnt));
                    ev.plorts_sold_currency += cash;
                    self.phases.insert(
                        site_id.to_string(),
                        DronePhase::MovingToDest {
                            program_idx: idx,
                            target_pos: [dpos.x as i32, dpos.y as i32, dpos.z as i32],
                        },
                    );
                    if programs.len() > 1 {
                        self.next_program_idx
                            .insert(site_id.to_string(), (idx + 1) % programs.len());
                    }
                    delivered_any = true;
                    break;
                }
            }
            if !delivered_any {
                self.phases.insert(site_id.to_string(), DronePhase::Idle);
            }
        } else {
            // 2. Otherwise try to gather from program source
            let mut gathered_any = false;
            for offset in 0..programs.len() {
                let idx = (start_idx + offset) % programs.len();
                let src = DroneSourceKind::from_key(&programs[idx].source);
                let target = programs[idx].target;
                if let Some((spos, item, cnt)) =
                    Self::try_gather(model, physics, ranch_bounds, src, target, &mut carry)
                {
                    let dir = spos - pos;
                    let dist = dir.length();
                    if dist > 0.5 {
                        pos += (dir / dist) * (DRONE_FLY_SPEED * dt).min(dist);
                    } else {
                        pos = spos;
                    }
                    ev.gathered.push((site_id.to_string(), item, cnt));
                    self.phases.insert(
                        site_id.to_string(),
                        DronePhase::MovingToSource {
                            program_idx: idx,
                            target_pos: [spos.x as i32, spos.y as i32, spos.z as i32],
                        },
                    );
                    gathered_any = true;
                    break;
                }
            }
            if !gathered_any {
                self.phases.insert(site_id.to_string(), DronePhase::Idle);
            }
        }

        if let Some(s) = model.gadget_sites.get_mut(site_id) {
            if let Some(ref mut gg) = s.attached {
                if let GadgetKind::Drone(ref mut d) = gg.kind {
                    d.ammo_id = carry.0;
                    d.ammo_count = carry.1;
                    d.no_clip_position = pos;
                }
            }
        }
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        ranch: &RanchRuntime,
        econ: &EconomyDirector,
        mode: &GameModeSettings,
        site_positions: &HashMap<String, Vec3>,
        curr_hour: f32,
        dt: f32,
        rng: &mut Randoms,
    ) -> DroneStepEvents {
        let mut ev = DroneStepEvents::default();
        let drone_sites: Vec<String> = model
            .gadget_sites
            .iter()
            .filter(|(_, s)| s.attached.as_ref().map_or(false, |g| g.ident.is_drone()))
            .map(|(id, _)| id.clone())
            .collect();

        for sid in drone_sites {
            let spos = site_positions.get(&sid).copied().unwrap_or(Vec3::ZERO);
            self.execute_one_drone_step(
                model,
                physics,
                &ranch.bounds,
                econ,
                mode,
                curr_hour,
                &sid,
                spos,
                dt,
                rng,
                &mut ev,
            );
        }

        ev
    }

    /// Port of `DroneFastForwarder.FastForward` when sleeping / catching up ranch plots.
    pub fn fast_forward_drones(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        ranch: &RanchRuntime,
        econ: &EconomyDirector,
        mode: &GameModeSettings,
        curr_hour: f32,
        start_time: f64,
        end_time: f64,
        rng: &mut Randoms,
    ) -> DroneStepEvents {
        let mut ev = DroneStepEvents::default();
        let drone_sites: Vec<(String, GadgetId, DroneModel)> = model
            .gadget_sites
            .iter()
            .filter_map(|(id, s)| {
                let g = s.attached.as_ref()?;
                if let GadgetKind::Drone(ref dm) = g.kind {
                    Some((id.clone(), g.ident, dm.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (sid, _gid, dm) in drone_sites {
            if dm.battery_deplete_time <= start_time || dm.programs.is_empty() {
                self.phases.insert(sid, DronePhase::Rest);
                continue;
            }
            let mut rem = dm.battery_deplete_time.min(end_time) - start_time;
            let mut carry = (dm.ammo_id, dm.ammo_count);
            let mut prog_idx = 0usize;

            while rem >= DRONE_FASTFORWARD_CYCLE_SECS {
                let mut did_work = false;
                for offset in 0..dm.programs.len() {
                    let idx = (prog_idx + offset) % dm.programs.len();
                    let prog = &dm.programs[idx];
                    let src = DroneSourceKind::from_key(&prog.source);
                    let dest = DroneDestKind::from_key(&prog.destination);
                    if carry.1 == 0 {
                        if let Some((_, item, cnt)) = Self::try_gather(
                            model,
                            physics,
                            &ranch.bounds,
                            src,
                            prog.target,
                            &mut carry,
                        ) {
                            ev.gathered.push((sid.clone(), item, cnt));
                        }
                    }
                    if carry.1 > 0 {
                        if let Some((_, item, cnt, cash)) = self.try_deliver(
                            model,
                            &ranch.bounds,
                            econ,
                            mode,
                            curr_hour,
                            dest,
                            &mut carry,
                            rng,
                        ) {
                            ev.delivered.push((sid.clone(), dest, item, cnt));
                            ev.plorts_sold_currency += cash;
                            did_work = true;
                            prog_idx = (idx + 1) % dm.programs.len();
                            break;
                        }
                    }
                }
                if !did_work {
                    break;
                }
                rem -= DRONE_FASTFORWARD_CYCLE_SECS;
            }

            if let Some(s) = model.gadget_sites.get_mut(&sid) {
                if let Some(ref mut g) = s.attached {
                    if let GadgetKind::Drone(ref mut d) = g.kind {
                        d.ammo_id = carry.0;
                        d.ammo_count = carry.1;
                    }
                }
            }
            if dm.battery_deplete_time <= end_time {
                self.phases.insert(sid, DronePhase::Rest);
            }
        }

        ev
    }
}
