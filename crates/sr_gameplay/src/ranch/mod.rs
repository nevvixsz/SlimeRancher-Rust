//! Ranch, LandPlot, 18 plot upgrades, Silos, Gardens, Corrals, Coops, Ponds,
//! Incinerators, and ToyDirector.

pub mod corral_coop;
pub mod garden_pond_incinerator;
pub mod plots;
pub mod silo;

pub use corral_coop::{
    deposit_feeder_food, feeder_cycle_hours, step_corrals_and_coops, step_feed_speed,
    withdraw_collector_plorts, AirNetState, CorralCoopEvents, PlotBounds,
    AIR_NET_HIT_FORCE_TO_DESTROY, AIR_NET_HOURS_TO_RECOVER, AIR_NET_NEW_STRENGTH,
    COLLECTOR_MAX_PER_SLOT, COLLECTOR_PERIOD_HOURS, COOP_BASE_CAPACITY, COOP_DELUXE_CAPACITY,
    FEEDER_HOURS_FAST, FEEDER_HOURS_NORMAL, FEEDER_HOURS_SLOW, FEEDER_ITEMS_PER_CYCLE,
};
pub use garden_pond_incinerator::{
    clear_garden_crop, crop_for_item, plant_garden_crop, step_gardens_ponds_incinerators,
    water_garden, GardenPondIncineratorEvents, GardenRuntimeState, FIRE_SLIME_ASH_EAT_INTERVAL,
    GARDEN_EDIBLE_HOURS, GARDEN_ROTTEN_HOURS, GARDEN_SPAWN_MAX_HOURS, GARDEN_SPAWN_MIN_HOURS,
    MAX_ASH_UNITS, MAX_STORED_WATER, WATER_EVAP_PER_HOUR,
};
pub use plots::{
    plot_purchase_cost, upgrade_purchase_cost, upgrade_to_deluxe_crop, PlotOpError,
    RanchDirector, RanchExpansion,
};
pub use silo::{
    SiloController, SILO_FULL_ACHIEVE_AMOUNT, SILO_FULL_ACHIEVE_SLOTS, SILO_MAX_PER_SLOT,
    SILO_MAX_SIDES, SILO_SLOTS_PER_SIDE, SILO_TOTAL_SLOTS,
};
