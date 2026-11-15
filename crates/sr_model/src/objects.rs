//! Port of world object models: `AccessDoorModel`, `GordoModel`, `EchoNoteGordoModel`,
//! `TreasurePodModel`, `PuzzleSlotModel`, `MasterSwitchModel`, `OasisModel`,
//! `QuicksilverEnergyGeneratorModel`, `DecorizerModel`, and `GlitchSlimulationModel`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_ids::IdentifiableId;

use crate::enums::{
    AccessDoorState, QuicksilverGeneratorState, SwitchState, TreasurePodState,
};

/// Port of `AccessDoorModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AccessDoorModel {
    pub id: String,
    pub state: AccessDoorState,
}

/// Port of `GordoModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GordoModel {
    pub id: String,
    pub gordo_eaten_count: i32,
    pub target_count: i32,
    pub fashions: Vec<IdentifiableId>,
}

impl GordoModel {
    pub fn is_popped(&self) -> bool {
        self.gordo_eaten_count >= self.target_count && self.target_count > 0
    }
}

/// Port of `EchoNoteGordoModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EchoNoteGordoModel {
    pub id: String,
    pub has_been_triggered: bool,
}

/// Port of `TreasurePodModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TreasurePodModel {
    pub id: String,
    pub state: TreasurePodState,
    pub spawn_queue: Vec<IdentifiableId>,
}

/// Port of `PuzzleSlotModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PuzzleSlotModel {
    pub id: String,
    pub filled: bool,
}

/// Port of `MasterSwitchModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MasterSwitchModel {
    pub id: String,
    pub state: SwitchState,
}

/// Port of `OasisModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OasisModel {
    pub id: String,
    pub is_live: bool,
}

/// Port of `QuicksilverEnergyGeneratorModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuicksilverEnergyGeneratorModel {
    pub id: String,
    pub state: QuicksilverGeneratorState,
    pub timer: Option<f64>,
}

/// Port of `DecorizerModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecorizerModel {
    pub contents: HashMap<IdentifiableId, i32>,
    pub settings: HashMap<String, IdentifiableId>,
}

/// Port of `GlitchImpostoModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GlitchImpostoModel {
    pub id: String,
    pub is_active: bool,
}

/// Port of `GlitchTarrNodeModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GlitchTarrNodeModel {
    pub id: String,
    pub is_active: bool,
}

/// Port of `GlitchSlimulationModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GlitchSlimulationModel {
    pub impostos: HashMap<String, GlitchImpostoModel>,
    pub nodes: HashMap<String, GlitchTarrNodeModel>,
    pub active_slime_spawn_points: HashSet<String>,
}
