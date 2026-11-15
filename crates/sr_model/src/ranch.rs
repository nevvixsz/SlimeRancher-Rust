//! Port of `LandPlotModel.cs`, `RanchCellModel.cs`, and `RanchModel.cs`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, LandPlotId, LandPlotUpgrade};

use crate::ammo::AmmoModel;
use crate::enums::{FeedSpeed, Palette, PaletteType, SpawnResourceId, StorageType};

/// Port of `LandPlotModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandPlotModel {
    pub id: String,
    pub type_id: LandPlotId,
    pub upgrades: HashSet<LandPlotUpgrade>,
    pub next_feeding_time: f64,
    pub remaining_feed_operations: i32,
    pub feeder_cycle_speed: FeedSpeed,
    pub collector_next_time: f64,
    pub attached_death_time: f64,
    pub attached_id: SpawnResourceId,
    pub attached_resource_id: IdentifiableId,
    pub silo_ammo: HashMap<StorageType, AmmoModel>,
    pub silo_storage_indices: [i32; 4],
    pub ash_units: f32,
}

impl Default for LandPlotModel {
    fn default() -> Self {
        Self {
            id: String::new(),
            type_id: LandPlotId::Empty,
            upgrades: HashSet::new(),
            next_feeding_time: 0.0,
            remaining_feed_operations: 0,
            feeder_cycle_speed: FeedSpeed::Normal,
            collector_next_time: 0.0,
            attached_death_time: 0.0,
            attached_id: SpawnResourceId::None,
            attached_resource_id: IdentifiableId::None,
            silo_ammo: HashMap::new(),
            silo_storage_indices: [0; 4],
            ash_units: 0.0,
        }
    }
}

impl LandPlotModel {
    pub fn new(id: impl Into<String>, type_id: LandPlotId) -> Self {
        Self {
            id: id.into(),
            type_id,
            ..Default::default()
        }
    }

    pub fn has_upgrade(&self, upgrade: LandPlotUpgrade) -> bool {
        self.upgrades.contains(&upgrade)
    }

    /// Checks whether every valid upgrade for a Corral/Coop/Silo is installed
    /// (matching `LandPlotModel.HasAllUpgrades` used by the achievement).
    pub fn has_all_core_upgrades(&self) -> bool {
        match self.type_id {
            LandPlotId::Corral => [
                LandPlotUpgrade::Walls,
                LandPlotUpgrade::MusicBox,
                LandPlotUpgrade::AirNet,
                LandPlotUpgrade::SolarShield,
                LandPlotUpgrade::PlortCollector,
                LandPlotUpgrade::Feeder,
            ]
            .iter()
            .all(|u| self.upgrades.contains(u)),
            LandPlotId::Coop => [
                LandPlotUpgrade::Walls,
                LandPlotUpgrade::Feeder,
                LandPlotUpgrade::Vitamizer,
            ]
            .iter()
            .all(|u| self.upgrades.contains(u)),
            LandPlotId::Silo => [
                LandPlotUpgrade::Storage2,
                LandPlotUpgrade::Storage3,
                LandPlotUpgrade::Storage4,
            ]
            .iter()
            .all(|u| self.upgrades.contains(u)),
            _ => false,
        }
    }
}

/// Port of `RanchCellModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RanchCellModel {
    pub id: String,
    pub selected: Palette,
}

/// Port of `RanchModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RanchModel {
    pub palettes: HashMap<PaletteType, Palette>,
    pub cells: HashMap<String, RanchCellModel>,
    pub ranch_fast_forward_amount: f64,
}

impl RanchModel {
    pub fn select_palette(&mut self, palette_type: PaletteType, palette: Palette) {
        self.palettes.insert(palette_type, palette);
    }

    /// Port of `RanchModel.IncludesFullyUpgradedCorralCoopAndSilo`.
    pub fn includes_fully_upgraded_corral_coop_and_silo(
        land_plots: &HashMap<String, LandPlotModel>,
    ) -> bool {
        let mut has_corral = false;
        let mut has_coop = false;
        let mut has_silo = false;
        for plot in land_plots.values() {
            match plot.type_id {
                LandPlotId::Corral if plot.has_all_core_upgrades() => has_corral = true,
                LandPlotId::Coop if plot.has_all_core_upgrades() => has_coop = true,
                LandPlotId::Silo if plot.has_all_core_upgrades() => has_silo = true,
                _ => {}
            }
        }
        has_corral && has_coop && has_silo
    }
}
