//! Ranch, LandPlot, 18 plot upgrades, Silos, Gardens, Corrals, Coops, Ponds,
//! Incinerators, and ToyDirector.

pub mod corral_coop;
pub mod plots;

pub use corral_coop::{
    deposit_feeder_food, feeder_cycle_hours, step_corrals_and_coops, step_feed_speed,
    withdraw_collector_plorts, AirNetState, CorralCoopEvents, PlotBounds,
    AIR_NET_HIT_FORCE_TO_DESTROY, AIR_NET_HOURS_TO_RECOVER, AIR_NET_NEW_STRENGTH,
    COLLECTOR_MAX_PER_SLOT, COLLECTOR_PERIOD_HOURS, COOP_BASE_CAPACITY, COOP_DELUXE_CAPACITY,
    FEEDER_HOURS_FAST, FEEDER_HOURS_NORMAL, FEEDER_HOURS_SLOW, FEEDER_ITEMS_PER_CYCLE,
};
pub use plots::{
    plot_purchase_cost, upgrade_purchase_cost, upgrade_to_deluxe_crop, PlotOpError,
    RanchDirector, RanchExpansion,
};
