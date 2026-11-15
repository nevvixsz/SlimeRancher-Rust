//! Ranch, LandPlot, 18 plot upgrades, Silos, Gardens, Corrals, Coops, Ponds,
//! Incinerators, and ToyDirector.

pub mod plots;

pub use plots::{
    plot_purchase_cost, upgrade_purchase_cost, upgrade_to_deluxe_crop, PlotOpError,
    RanchDirector, RanchExpansion,
};
