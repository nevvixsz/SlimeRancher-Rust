//! Port of `RanchDirector.cs`, `LandPlot.cs`, and `LandPlotUI.cs` — plot purchasing,
//! demolishing, all 18 `LandPlotUpgrade`s, ranch expansions, and palette selection.

use serde::{Deserialize, Serialize};
use sr_ids::{LandPlotId, LandPlotUpgrade, ProgressType};
use sr_model::{
    AccessDoorState, AmmoModel, FeedSpeed, GameModel, LandPlotModel, Palette, PaletteType,
    SpawnResourceId, StorageType,
};

use crate::player::PlayerController;

/// Canonical Newbuck costs for building a plot on an `Empty` slot.
pub fn plot_purchase_cost(plot_type: LandPlotId) -> Option<i32> {
    match plot_type {
        LandPlotId::Corral | LandPlotId::Coop | LandPlotId::Garden => Some(250),
        LandPlotId::Silo | LandPlotId::Pond | LandPlotId::Incinerator => Some(450),
        LandPlotId::Empty | LandPlotId::None => None,
    }
}

/// Canonical Newbuck costs for each `LandPlotUpgrade` on a given `LandPlotId`.
pub fn upgrade_purchase_cost(plot_type: LandPlotId, upgrade: LandPlotUpgrade) -> Option<i32> {
    match (plot_type, upgrade) {
        // Corral (6)
        (LandPlotId::Corral, LandPlotUpgrade::Walls) => Some(350),
        (LandPlotId::Corral, LandPlotUpgrade::MusicBox) => Some(350),
        (LandPlotId::Corral, LandPlotUpgrade::AirNet) => Some(425),
        (LandPlotId::Corral, LandPlotUpgrade::SolarShield) => Some(425),
        (LandPlotId::Corral, LandPlotUpgrade::PlortCollector) => Some(500),
        (LandPlotId::Corral, LandPlotUpgrade::Feeder) => Some(500),
        // Coop (4)
        (LandPlotId::Coop, LandPlotUpgrade::Walls) => Some(350),
        (LandPlotId::Coop, LandPlotUpgrade::Feeder) => Some(500),
        (LandPlotId::Coop, LandPlotUpgrade::Vitamizer) => Some(425),
        (LandPlotId::Coop, LandPlotUpgrade::DeluxeCoop) => Some(0), // Mochi reward unlock
        // Garden (5)
        (LandPlotId::Garden, LandPlotUpgrade::Soil) => Some(350),
        (LandPlotId::Garden, LandPlotUpgrade::Sprinkler) => Some(500),
        (LandPlotId::Garden, LandPlotUpgrade::Scareslime) => Some(425),
        (LandPlotId::Garden, LandPlotUpgrade::MiracleMix) => Some(0), // Ogden reward unlock
        (LandPlotId::Garden, LandPlotUpgrade::DeluxeGarden) => Some(0), // Ogden reward unlock
        // Silo (3)
        (LandPlotId::Silo, LandPlotUpgrade::Storage2) => Some(450),
        (LandPlotId::Silo, LandPlotUpgrade::Storage3) => Some(500),
        (LandPlotId::Silo, LandPlotUpgrade::Storage4) => Some(575),
        // Incinerator (1)
        (LandPlotId::Incinerator, LandPlotUpgrade::AshTrough) => Some(500),
        _ => None,
    }
}

/// Canonical Newbuck cost to unlock a ranch expansion door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RanchExpansion {
    Overgrowth,
    Grotto,
    Docks,
    Lab,
}

impl RanchExpansion {
    pub const fn cost(self) -> i32 {
        match self {
            Self::Overgrowth | Self::Grotto | Self::Docks => 1795,
            Self::Lab => 10000,
        }
    }

    pub const fn door_id(self) -> &'static str {
        match self {
            Self::Overgrowth => "door_overgrowth",
            Self::Grotto => "door_grotto",
            Self::Docks => "door_docks",
            Self::Lab => "door_lab",
        }
    }

    pub const fn progress_type(self) -> ProgressType {
        match self {
            Self::Overgrowth => ProgressType::UnlockOvergrowth,
            Self::Grotto => ProgressType::UnlockGrotto,
            Self::Docks => ProgressType::UnlockDocks,
            Self::Lab => ProgressType::UnlockLab,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotOpError {
    PlotNotFound,
    PlotNotEmpty,
    AlreadyEmpty,
    InvalidPlotType,
    InvalidUpgradeForPlot,
    PrerequisiteUpgradeMissing,
    AlreadyHasUpgrade,
    InsufficientFunds,
    ExpansionLockedOrMissing,
    AlreadyUnlocked,
}

/// Stateless/controller operations mutating `GameModel::land_plots`, `ranch`, and `player`.
pub struct RanchDirector;

impl RanchDirector {
    /// Registers an empty plot slot on the ranch if not already present.
    pub fn ensure_plot(model: &mut GameModel, plot_id: impl Into<String>, type_id: LandPlotId) {
        let id = plot_id.into();
        model
            .land_plots
            .entry(id.clone())
            .or_insert_with(|| LandPlotModel::new(id, type_id));
    }

    /// Buys a new `LandPlotId` on an `Empty` plot slot, deducting Newbucks and initializing
    /// plot-specific `AmmoModel` storage / feeder / collector timestamps.
    pub fn buy_plot(
        model: &mut GameModel,
        plot_id: &str,
        plot_type: LandPlotId,
    ) -> Result<(), PlotOpError> {
        let cost = plot_purchase_cost(plot_type).ok_or(PlotOpError::InvalidPlotType)?;
        let current_type = model
            .land_plots
            .get(plot_id)
            .ok_or(PlotOpError::PlotNotFound)?
            .type_id;
        if current_type != LandPlotId::Empty {
            return Err(PlotOpError::PlotNotEmpty);
        }
        if !PlayerController::spend_currency(&mut model.player, cost) {
            return Err(PlotOpError::InsufficientFunds);
        }

        let wt = model.world.world_time;
        let plot = model
            .land_plots
            .get_mut(plot_id)
            .ok_or(PlotOpError::PlotNotFound)?;
        *plot = LandPlotModel::new(plot_id, plot_type);

        match plot_type {
            LandPlotId::Corral => {
                plot.next_feeding_time = wt + 6.0 * 3600.0;
                plot.feeder_cycle_speed = FeedSpeed::Normal;
                plot.collector_next_time = wt + 3600.0;
                plot.silo_ammo.insert(StorageType::Food, AmmoModel::new(1));
                plot.silo_ammo.insert(StorageType::Plort, AmmoModel::new(4));
            }
            LandPlotId::Coop => {
                plot.next_feeding_time = wt + 6.0 * 3600.0;
                plot.feeder_cycle_speed = FeedSpeed::Normal;
                plot.silo_ammo.insert(StorageType::Food, AmmoModel::new(1));
            }
            LandPlotId::Silo => {
                // Storage1 starts with 3 slots; STORAGE2..4 expand to 6, 9, 12 slots
                plot.silo_ammo
                    .insert(StorageType::NonSlimes, AmmoModel::new(12));
            }
            LandPlotId::Incinerator => {
                plot.ash_units = 0.0;
            }
            _ => {}
        }

        Ok(())
    }

    /// Demolishes a built plot back to `LandPlotId::Empty`.
    pub fn demolish_plot(model: &mut GameModel, plot_id: &str) -> Result<(), PlotOpError> {
        let plot = model
            .land_plots
            .get_mut(plot_id)
            .ok_or(PlotOpError::PlotNotFound)?;
        if plot.type_id == LandPlotId::Empty || plot.type_id == LandPlotId::None {
            return Err(PlotOpError::AlreadyEmpty);
        }
        *plot = LandPlotModel::new(plot_id, LandPlotId::Empty);
        Ok(())
    }

    /// Purchases and installs a `LandPlotUpgrade` onto an existing plot, checking
    /// plot compatibility, Silo storage ordering, and Newbucks balance.
    pub fn buy_upgrade(
        model: &mut GameModel,
        plot_id: &str,
        upgrade: LandPlotUpgrade,
    ) -> Result<(), PlotOpError> {
        let (plot_type, already_has, missing_prereq) = {
            let plot = model
                .land_plots
                .get(plot_id)
                .ok_or(PlotOpError::PlotNotFound)?;
            let missing_prereq = match (plot.type_id, upgrade) {
                (LandPlotId::Silo, LandPlotUpgrade::Storage3) => {
                    !plot.has_upgrade(LandPlotUpgrade::Storage2)
                }
                (LandPlotId::Silo, LandPlotUpgrade::Storage4) => {
                    !plot.has_upgrade(LandPlotUpgrade::Storage3)
                }
                _ => false,
            };
            (plot.type_id, plot.has_upgrade(upgrade), missing_prereq)
        };

        let cost = upgrade_purchase_cost(plot_type, upgrade)
            .ok_or(PlotOpError::InvalidUpgradeForPlot)?;
        if already_has {
            return Err(PlotOpError::AlreadyHasUpgrade);
        }
        if missing_prereq {
            return Err(PlotOpError::PrerequisiteUpgradeMissing);
        }
        if !PlayerController::spend_currency(&mut model.player, cost) {
            return Err(PlotOpError::InsufficientFunds);
        }

        let wt = model.world.world_time;
        let plot = model
            .land_plots
            .get_mut(plot_id)
            .ok_or(PlotOpError::PlotNotFound)?;
        plot.upgrades.insert(upgrade);

        match upgrade {
            LandPlotUpgrade::Feeder => {
                plot.next_feeding_time = wt + 6.0 * 3600.0;
                plot.remaining_feed_operations = 0;
            }
            LandPlotUpgrade::PlortCollector => {
                plot.collector_next_time = wt + 3600.0;
            }
            LandPlotUpgrade::AshTrough => {
                plot.ash_units = 20.0;
            }
            LandPlotUpgrade::DeluxeGarden => {
                plot.attached_id = upgrade_to_deluxe_crop(plot.attached_id);
            }
            _ => {}
        }

        Ok(())
    }

    /// Purchases a ranch expansion (Overgrowth / Grotto / Docks / Lab).
    pub fn buy_expansion(
        model: &mut GameModel,
        expansion: RanchExpansion,
    ) -> Result<(), PlotOpError> {
        let ptype = expansion.progress_type();
        if model.progress.get_progress(ptype) > 0 {
            return Err(PlotOpError::AlreadyUnlocked);
        }
        if !PlayerController::spend_currency(&mut model.player, expansion.cost()) {
            return Err(PlotOpError::InsufficientFunds);
        }
        model.progress.set_progress(ptype, 1);
        if let Some(door) = model.doors.get_mut(expansion.door_id()) {
            door.state = AccessDoorState::Open;
        }
        Ok(())
    }

    pub fn set_palette(model: &mut GameModel, palette_type: PaletteType, palette: Palette) {
        model.ranch.select_palette(palette_type, palette);
    }
}

pub fn upgrade_to_deluxe_crop(id: SpawnResourceId) -> SpawnResourceId {
    match id {
        SpawnResourceId::CuberryTree => SpawnResourceId::CuberryTreeDlx,
        SpawnResourceId::MangoTree => SpawnResourceId::MangoTreeDlx,
        SpawnResourceId::CarrotPatch => SpawnResourceId::CarrotPatchDlx,
        SpawnResourceId::OcaocaPatch => SpawnResourceId::OcaocaPatchDlx,
        SpawnResourceId::PearTree => SpawnResourceId::PearTreeDlx,
        SpawnResourceId::PogoTree => SpawnResourceId::PogoTreeDlx,
        SpawnResourceId::ParsnipPatch => SpawnResourceId::ParsnipPatchDlx,
        SpawnResourceId::BeetPatch => SpawnResourceId::BeetPatchDlx,
        SpawnResourceId::OnionPatch => SpawnResourceId::OnionPatchDlx,
        SpawnResourceId::LemonTree => SpawnResourceId::LemonTreeDlx,
        SpawnResourceId::GingerPatch => SpawnResourceId::GingerPatchDlx,
        other => other,
    }
}
