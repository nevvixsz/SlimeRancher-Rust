//! World streaming, regions, zone/cell spawners, doors/gates/treasure pods, and Glass Desert weather.

pub mod region;

pub use region::{region_set_for_zone, Region, RegionRegistry, RegionStepEvents};
