//! Step 7: Slime Science (`GadgetDirector`, Refinery, Fabricator, Extractors,
//! Teleporters/WarpDepots/GordoSnares/Utilities, and Drones).

pub mod director;
pub mod extractors;
pub mod gadgets;

pub use director::{
    builtin_blueprint_unlock_rules, builtin_gadget_definitions, BlueprintUnlockRule,
    GadgetDirector, GadgetDirectorStepEvents, GadgetError, REFINERY_MAX,
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
