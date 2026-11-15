//! Step 7: Slime Science (`GadgetDirector`, Refinery, Fabricator, Extractors,
//! Teleporters/WarpDepots/GordoSnares/Utilities, and Drones).

pub mod director;
pub mod drone_ai;
pub mod drones;
pub mod extractors;
pub mod gadgets;

pub use director::{
    builtin_blueprint_unlock_rules, builtin_gadget_definitions, BlueprintUnlockRule,
    GadgetDirector, GadgetDirectorStepEvents, GadgetError, REFINERY_MAX,
};
pub use drone_ai::{
    DroneAiSystem, DronePhase, DroneStepEvents, DRONE_FASTFORWARD_CYCLE_SECS, DRONE_FLY_SPEED,
};
pub use drones::{
    drone_battery_fraction, drone_has_battery, drone_program_count, recharge_drone_battery,
    set_drone_program, DroneDestKind, DroneNetworkScope, DroneSourceKind, DroneTargetCategory,
    DRONE_AMMO_CAPACITY, DRONE_BATTERY_DURATION_HOURS, DRONE_BATTERY_DURATION_SECS,
};
pub use extractors::{
    spec_for_extractor, ExtractorProduceEntry, ExtractorSpec, ExtractorStepEvents, ExtractorSystem,
};
pub use gadgets::{
    bait_gordo_snare, check_gordo_snares, deposit_warp_depot, feed_snared_gordo,
    step_utility_gadgets, trigger_chicken_cloner, use_teleporter, withdraw_warp_depot,
    UtilityGadgetEvents, HYDRO_TURRET_RANGE, MED_STATION_HEAL_PER_SEC, SNARED_GORDO_TARGET_COUNT,
    TAMING_BELL_RANGE, WARP_DEPOT_MAX_STACK,
};

use serde::{Deserialize, Serialize};
use sr_data::{DataRegistry, GameModeSettings};
use sr_math::Randoms;
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

use crate::economy::EconomyDirector;
use crate::ranch::RanchRuntime;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScienceStepEvents {
    pub director: GadgetDirectorStepEvents,
    pub extractors: ExtractorStepEvents,
    pub utilities: UtilityGadgetEvents,
    pub drones: DroneStepEvents,
}

/// Unified Step 7 runtime coordinating blueprints, extractors, utility gadgets, and drones.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScienceRuntime {
    pub director: GadgetDirector,
    pub extractors: ExtractorSystem,
    pub drones: DroneAiSystem,
}

impl ScienceRuntime {
    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        data: &DataRegistry,
        mode: &GameModeSettings,
        ranch: &RanchRuntime,
        econ: &EconomyDirector,
        passed_5am: bool,
        curr_hour: f32,
        dt: f32,
        rng: &mut Randoms,
    ) -> ScienceStepEvents {
        let director = self.director.step_blueprint_locks(model, mode);
        let extractors =
            self.extractors
                .step(model, physics, &self.director.site_zones, rng);
        let utilities = step_utility_gadgets(
            model,
            physics,
            data,
            &self.extractors.site_positions,
            passed_5am,
            dt,
            rng,
        );
        let drones = self.drones.step(
            model,
            physics,
            ranch,
            econ,
            mode,
            &self.extractors.site_positions,
            curr_hour,
            dt,
            rng,
        );

        ScienceStepEvents {
            director,
            extractors,
            utilities,
            drones,
        }
    }
}
