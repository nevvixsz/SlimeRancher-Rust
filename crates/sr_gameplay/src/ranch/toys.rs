//! Port of `ToyDirector.cs` and `RanchRuntime` (coordinating ranch plots & fast-forwarding).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, LandPlotId, LandPlotUpgrade, ProgressType, RegionSetId};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, ResourceCycleState, SpawnResourceId};
use sr_physics::{PhysicsWorld, RigidBody};

use crate::player::PlayerController;
use crate::ranch::corral_coop::{
    step_corrals_and_coops, AirNetState, CorralCoopEvents, PlotBounds,
};
use crate::ranch::garden_pond_incinerator::{
    step_gardens_ponds_incinerators, GardenPondIncineratorEvents, GardenRuntimeState,
    GARDEN_EDIBLE_HOURS, GARDEN_SPAWN_MAX_HOURS, GARDEN_SPAWN_MIN_HOURS,
};

pub const CORPORATE_LEVEL_UNLOCK: i32 = 10;
pub const FASTFORWARD_MIN_SECS: f64 = 7200.0;
pub const FASTFORWARD_CHUNK_SECS: f64 = 14400.0;

pub const BASE_TOYS: &[IdentifiableId] = &[
    IdentifiableId::BeachBallToy,
    IdentifiableId::BigRockToy,
    IdentifiableId::YarnBallToy,
    IdentifiableId::NightLightToy,
    IdentifiableId::PowerCellToy,
    IdentifiableId::BombBallToy,
    IdentifiableId::BuzzyBeeToy,
    IdentifiableId::RubberDuckyToy,
    IdentifiableId::OctoBuddyToy,
];

pub const UPGRADED_TOYS: &[IdentifiableId] = &[
    IdentifiableId::CrystalBallToy,
    IdentifiableId::StuffedChickenToy,
    IdentifiableId::PuzzleCubeToy,
    IdentifiableId::DiscoBallToy,
    IdentifiableId::GyroTopToy,
    IdentifiableId::CharcoalBrickToy,
    IdentifiableId::SolMateToy,
    IdentifiableId::StegoBuddyToy,
];

/// Port of `ToyDirector`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToyDirector {
    pub registered: Vec<IdentifiableId>,
}

impl ToyDirector {
    pub fn register(&mut self, id: IdentifiableId) {
        self.registered.retain(|&x| x != id);
        self.registered.push(id);
    }

    pub fn purchaseable_toys(&self, model: &GameModel) -> Vec<IdentifiableId> {
        let mut out = BASE_TOYS.to_vec();
        if model.progress.get_progress(ProgressType::CorporatePartner) >= CORPORATE_LEVEL_UNLOCK {
            out.extend_from_slice(UPGRADED_TOYS);
        }
        for &id in &self.registered {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    pub fn toy_cost(id: IdentifiableId) -> i32 {
        if UPGRADED_TOYS.contains(&id) {
            500
        } else {
            250
        }
    }

    pub fn buy_toy(
        &self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        id: IdentifiableId,
        spawn_pos: Vec3,
    ) -> Result<i64, &'static str> {
        if !self.purchaseable_toys(model).contains(&id) {
            return Err("Toy not unlocked");
        }
        let cost = Self::toy_cost(id);
        if !PlayerController::spend_currency(&mut model.player, cost) {
            return Err("Insufficient Newbucks");
        }
        let aid = model.create_actor(id, RegionSetId::Home, spawn_pos, Quat::IDENTITY);
        physics.insert_body(RigidBody {
            actor_id: aid,
            pos: spawn_pos,
            radius: 0.45,
            ..Default::default()
        });
        Ok(aid)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RanchStepEvents {
    pub corral_coop: CorralCoopEvents,
    pub garden_pond_incinerator: GardenPondIncineratorEvents,
}

/// Top-level runtime coordinating all ranch plots, air nets, gardens, ash troughs, and toys.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RanchRuntime {
    pub bounds: Vec<PlotBounds>,
    pub air_nets: HashMap<String, AirNetState>,
    pub gardens: HashMap<String, GardenRuntimeState>,
    pub ash_eat_timers: HashMap<i64, f64>,
    pub toys: ToyDirector,
    pub elapsed_time: f64,
}

impl RanchRuntime {
    pub fn register_bounds(&mut self, b: PlotBounds) {
        self.bounds.retain(|x| x.plot_id != b.plot_id);
        self.bounds.push(b);
    }

    pub fn has_solar_shield_at(&self, model: &GameModel, pos: Vec3) -> bool {
        self.bounds.iter().any(|b| {
            b.contains_xz(pos)
                && model.land_plots.get(&b.plot_id).map_or(false, |p| {
                    p.type_id == LandPlotId::Corral && p.has_upgrade(LandPlotUpgrade::SolarShield)
                })
        })
    }

    pub fn has_music_box_at(&self, model: &GameModel, pos: Vec3) -> bool {
        self.bounds.iter().any(|b| {
            b.contains_xz(pos)
                && model.land_plots.get(&b.plot_id).map_or(false, |p| {
                    p.type_id == LandPlotId::Corral && p.has_upgrade(LandPlotUpgrade::MusicBox)
                })
        })
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        world_time: f64,
        delta_world_time: f64,
        dt: f32,
        rng: &mut Randoms,
    ) -> RanchStepEvents {
        self.elapsed_time += dt as f64;
        let corral_coop = step_corrals_and_coops(
            model,
            physics,
            &self.bounds,
            &mut self.air_nets,
            world_time,
            delta_world_time,
            rng,
        );
        let garden_pond_incinerator = step_gardens_ponds_incinerators(
            model,
            physics,
            &self.bounds,
            &mut self.gardens,
            &mut self.ash_eat_timers,
            world_time,
            delta_world_time,
            self.elapsed_time,
            rng,
        );
        RanchStepEvents {
            corral_coop,
            garden_pond_incinerator,
        }
    }

    /// Port of `RanchCellFastForwarder` for Gardens & Feeders/Collectors when catch-up
    /// spans `>= FASTFORWARD_MIN_SECS` (2 game hours).
    pub fn fast_forward_plots(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        start_time: f64,
        end_time: f64,
        rng: &mut Randoms,
    ) -> usize {
        if end_time - start_time < FASTFORWARD_MIN_SECS {
            return 0;
        }
        model.ranch.ranch_fast_forward_amount += end_time - start_time;
        let mut total_events = 0usize;

        for b in &self.bounds {
            let Some(plot) = model.land_plots.get(&b.plot_id).cloned() else {
                continue;
            };
            if plot.type_id == LandPlotId::Garden
                && plot.attached_id != SpawnResourceId::None
                && plot.attached_resource_id != IdentifiableId::None
            {
                let gstate = self.gardens.entry(b.plot_id.clone()).or_default();
                let is_watered =
                    plot.has_upgrade(LandPlotUpgrade::Sprinkler) || gstate.stored_water > 0.0;
                let mut t = gstate.next_spawn_time.max(start_time);
                let interval = (rng.get_in_range_f32(GARDEN_SPAWN_MIN_HOURS, GARDEN_SPAWN_MAX_HOURS)
                    * if is_watered { 0.5 } else { 1.0 })
                .max(1.0) as f64
                    * 3600.0;

                while t <= end_time {
                    if end_time - t < GARDEN_EDIBLE_HOURS * 3600.0 {
                        let count = if plot.has_upgrade(LandPlotUpgrade::DeluxeGarden) {
                            16
                        } else if plot.has_upgrade(LandPlotUpgrade::Soil) {
                            12
                        } else {
                            8
                        };
                        for _ in 0..count {
                            let jitter = Vec3::new(
                                rng.get_in_range_f32(-2.0, 2.0),
                                0.5,
                                rng.get_in_range_f32(-2.0, 2.0),
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
                                    pm.progress_time = t + GARDEN_EDIBLE_HOURS * 3600.0;
                                }
                            }
                            physics.insert_body(RigidBody {
                                actor_id: cid,
                                pos,
                                radius: 0.25,
                                ..Default::default()
                            });
                            total_events += 1;
                        }
                    }
                    t += interval;
                }
                gstate.next_spawn_time = t;
            }
        }

        // Advance Corral feeders/collectors up to end_time
        let ff_ev = step_corrals_and_coops(
            model,
            physics,
            &self.bounds,
            &mut self.air_nets,
            end_time,
            end_time - start_time,
            rng,
        );
        total_events += ff_ev.feeder_ejected.len() + ff_ev.collector_stored.len();
        total_events
    }
}
