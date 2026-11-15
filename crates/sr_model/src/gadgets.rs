//! Port of `GadgetModel.cs`, `ExtractorModel.cs`, `WarpDepotModel.cs`, `SnareModel.cs`,
//! `DroneModel.cs`, `GadgetSiteModel.cs`, and `GadgetsModel.cs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, IdentifiableId};
use sr_math::Vec3;

/// Port of `GadgetDirector.BlueprintLockData`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct BlueprintLockData {
    pub timed_lock: bool,
    pub locked_until: f64,
}

/// Port of `ExtractorModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExtractorModel {
    pub cycles_remaining: i32,
    pub next_produce_time: f64,
    pub queued_to_produce: i32,
}

/// Port of `WarpDepotModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WarpDepotModel {
    pub is_linked: bool,
    pub ammo_id: IdentifiableId,
    pub ammo_count: i32,
}

/// Port of `SnareModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SnareModel {
    pub bait_type_id: IdentifiableId,
    pub gordo_type_id: IdentifiableId,
    pub gordo_eaten_count: i32,
    pub fashions: Vec<IdentifiableId>,
}

/// Single drone program slot (`DroneModel.ProgramData`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DroneProgramData {
    pub target: IdentifiableId,
    pub source: String,
    pub destination: String,
}

/// Port of `DroneModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DroneModel {
    pub battery_deplete_time: f64,
    pub fashions: Vec<IdentifiableId>,
    pub no_clip_position: Vec3,
    pub ammo_id: IdentifiableId,
    pub ammo_count: i32,
    pub programs: Vec<DroneProgramData>,
}

/// Subclass payload of `GadgetModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GadgetKind {
    Basic,
    Extractor(ExtractorModel),
    WarpDepot(WarpDepotModel),
    Snare(SnareModel),
    Drone(DroneModel),
}

/// Port of `GadgetModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GadgetModel {
    pub ident: GadgetId,
    pub site_id: String,
    pub y_rotation: f32,
    pub wait_for_chargeup_time: f64,
    pub kind: GadgetKind,
}

impl GadgetModel {
    pub fn new(ident: GadgetId, site_id: impl Into<String>, y_rotation: f32) -> Self {
        let kind = if ident.is_extractor() {
            GadgetKind::Extractor(ExtractorModel::default())
        } else if ident.is_warp_depot() {
            GadgetKind::WarpDepot(WarpDepotModel::default())
        } else if ident.is_snare() {
            GadgetKind::Snare(SnareModel::default())
        } else if ident.is_drone() {
            GadgetKind::Drone(DroneModel::default())
        } else {
            GadgetKind::Basic
        };

        Self {
            ident,
            site_id: site_id.into(),
            y_rotation,
            wait_for_chargeup_time: 0.0,
            kind,
        }
    }
}

/// Port of `GadgetSiteModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GadgetSiteModel {
    pub id: String,
    pub attached: Option<GadgetModel>,
}

/// Port of `GadgetsModel` (refinery, blueprints, placed gadgets).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GadgetsModel {
    pub blueprints: Vec<GadgetId>,
    pub avail_blueprints: Vec<GadgetId>,
    pub blueprint_locks: HashMap<GadgetId, BlueprintLockData>,
    pub gadgets: HashMap<GadgetId, i32>,
    pub placed_gadget_counts: HashMap<GadgetId, i32>,
    pub craft_mat_counts: HashMap<IdentifiableId, i32>,
    pub registered_blueprints: Vec<GadgetId>,
}

impl Default for GadgetsModel {
    fn default() -> Self {
        Self::new()
    }
}

impl GadgetsModel {
    /// Initial blueprints available in Builder's Shop matching `GadgetsModel.AVAILABLE_BLUEPRINTS`.
    pub const INITIAL_AVAILABLE_BLUEPRINTS: &'static [GadgetId] = &[
        GadgetId::ExtractorDrillNovice,
        GadgetId::ExtractorPumpNovice,
        GadgetId::ExtractorApiaryNovice,
        GadgetId::TeleporterPink,
        GadgetId::WarpDepotPink,
        GadgetId::FashionPodGoogly,
        GadgetId::SlimeStage,
        GadgetId::SlimeHoop,
    ];

    pub fn new() -> Self {
        let mut model = Self {
            blueprints: Vec::new(),
            avail_blueprints: Vec::new(),
            blueprint_locks: HashMap::new(),
            gadgets: HashMap::new(),
            placed_gadget_counts: HashMap::new(),
            craft_mat_counts: HashMap::new(),
            registered_blueprints: Vec::new(),
        };
        model.init_initial_blueprints();
        model
    }

    /// Port of `GadgetsModel.InitInitialBlueprints`.
    pub fn init_initial_blueprints(&mut self) {
        self.blueprints.clear();
        self.avail_blueprints.clear();
        self.blueprints.push(GadgetId::ExtractorDrillNovice);
        for &bp in Self::INITIAL_AVAILABLE_BLUEPRINTS {
            if bp != GadgetId::ExtractorDrillNovice && bp != GadgetId::HydroShower {
                self.avail_blueprints.push(bp);
            }
        }
    }

    pub fn add_refinery_item(&mut self, id: IdentifiableId, count: i32) {
        let entry = self.craft_mat_counts.entry(id).or_insert(0);
        *entry += count;
    }

    pub fn get_refinery_count(&self, id: IdentifiableId) -> i32 {
        self.craft_mat_counts.get(&id).copied().unwrap_or(0)
    }
}
