//! Port of `DroneNetwork.cs`, `DroneStation.cs`, `DroneStationBattery.cs`,
//! `DroneGadget.cs`, and `DroneAmmo.cs`.

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, IdentifiableId, ZoneId};
use sr_model::{DroneModel, DroneProgramData, GadgetKind, GameModel};

pub const DRONE_BATTERY_DURATION_HOURS: f64 = 28.0;
pub const DRONE_BATTERY_DURATION_SECS: f64 = 100_800.0;
pub const DRONE_AMMO_CAPACITY: i32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DroneSourceKind {
    None,
    Corral,
    PlortCollector,
    Garden,
    Coop,
    Pond,
    Incinerator,
    Silo,
    FreeRange,
    OutsidePlots,
}

impl DroneSourceKind {
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::None => "drone.behaviour.none",
            Self::Corral => "drone.behaviour.corral",
            Self::PlortCollector => "drone.behaviour.plort_collector",
            Self::Garden => "drone.behaviour.garden",
            Self::Coop => "drone.behaviour.coop",
            Self::Pond => "drone.behaviour.pond",
            Self::Incinerator => "drone.behaviour.incinerator",
            Self::Silo => "drone.behaviour.silo",
            Self::FreeRange => "drone.behaviour.free_range",
            Self::OutsidePlots => "drone.behaviour.outside_plots",
        }
    }

    pub fn from_key(s: &str) -> Self {
        match s {
            "drone.behaviour.corral" => Self::Corral,
            "drone.behaviour.plort_collector" => Self::PlortCollector,
            "drone.behaviour.garden" => Self::Garden,
            "drone.behaviour.coop" => Self::Coop,
            "drone.behaviour.pond" => Self::Pond,
            "drone.behaviour.incinerator" => Self::Incinerator,
            "drone.behaviour.silo" => Self::Silo,
            "drone.behaviour.free_range" => Self::FreeRange,
            "drone.behaviour.outside_plots" => Self::OutsidePlots,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DroneDestKind {
    None,
    Silo,
    Feeder,
    PlortMarket,
    Refinery,
    Incinerator,
}

impl DroneDestKind {
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::None => "drone.behaviour.none",
            Self::Silo => "drone.behaviour.silo",
            Self::Feeder => "drone.behaviour.feeder",
            Self::PlortMarket => "drone.behaviour.plort_market",
            Self::Refinery => "drone.behaviour.refinery",
            Self::Incinerator => "drone.behaviour.incinerator",
        }
    }

    pub fn from_key(s: &str) -> Self {
        match s {
            "drone.behaviour.silo" => Self::Silo,
            "drone.behaviour.feeder" => Self::Feeder,
            "drone.behaviour.plort_market" => Self::PlortMarket,
            "drone.behaviour.refinery" => Self::Refinery,
            "drone.behaviour.incinerator" => Self::Incinerator,
            _ => Self::None,
        }
    }
}

/// Optional broad category override alongside specific `IdentifiableId` target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DroneTargetCategory {
    AnyMatching,
    AllPlorts,
    AllFruits,
    AllVeggies,
    AllMeat,
    Specific(IdentifiableId),
}

impl DroneTargetCategory {
    pub fn matches(self, id: IdentifiableId) -> bool {
        match self {
            Self::AnyMatching => id != IdentifiableId::None,
            Self::AllPlorts => id.is_plort(),
            Self::AllFruits => id.is_fruit(),
            Self::AllVeggies => id.is_veggie(),
            Self::AllMeat => id.is_meat(),
            Self::Specific(t) => t == id,
        }
    }
}

pub fn drone_program_count(gadget_id: GadgetId) -> usize {
    match gadget_id {
        GadgetId::DroneAdvanced => 2,
        GadgetId::Drone => 1,
        _ => 0,
    }
}

pub fn drone_battery_fraction(drone: &DroneModel, world_time: f64) -> f32 {
    (((drone.battery_deplete_time - world_time) / DRONE_BATTERY_DURATION_SECS) as f32)
        .clamp(0.0, 1.0)
}

pub fn drone_has_battery(drone: &DroneModel, world_time: f64) -> bool {
    drone_battery_fraction(drone, world_time) > 0.0
}

/// Port of `DroneStationBattery.AddLiquid`.
pub fn recharge_drone_battery(
    model: &mut GameModel,
    site_id: &str,
    liquid_id: IdentifiableId,
) -> bool {
    if !liquid_id.is_water() {
        return false;
    }
    let wt = model.world.world_time;
    let Some(site) = model.gadget_sites.get_mut(site_id) else {
        return false;
    };
    let Some(ref mut g) = site.attached else {
        return false;
    };
    let GadgetKind::Drone(ref mut dm) = g.kind else {
        return false;
    };
    dm.battery_deplete_time = wt + DRONE_BATTERY_DURATION_SECS;
    true
}

/// Configures a drone program slot (`0` for basic Drone, `0..2` for `DroneAdvanced`).
pub fn set_drone_program(
    model: &mut GameModel,
    site_id: &str,
    slot_idx: usize,
    target: IdentifiableId,
    source: DroneSourceKind,
    destination: DroneDestKind,
) -> bool {
    let Some(site) = model.gadget_sites.get_mut(site_id) else {
        return false;
    };
    let Some(ref mut g) = site.attached else {
        return false;
    };
    let max_slots = drone_program_count(g.ident);
    if slot_idx >= max_slots {
        return false;
    }
    let GadgetKind::Drone(ref mut dm) = g.kind else {
        return false;
    };
    if dm.programs.len() < max_slots {
        dm.programs.resize_with(max_slots, Default::default);
    }
    dm.programs[slot_idx] = DroneProgramData {
        target,
        source: source.as_key().to_string(),
        destination: destination.as_key().to_string(),
    };
    true
}

/// Groups plots, refinery/market endpoints, and drone stations belonging to a single ranch zone.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DroneNetworkScope {
    pub zone: ZoneId,
    pub plot_ids: Vec<String>,
    pub has_plort_market: bool,
    pub has_refinery_link: bool,
}
