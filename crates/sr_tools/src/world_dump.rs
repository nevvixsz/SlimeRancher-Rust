//! JSON schemas for world placement & balance dumps exported from the original game
//! (via `SrWorldDumper.cs` or AssetStudio scene parsing).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::localization::ToolsResult;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3Dto {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlacedObjectDump {
    pub id: String,
    pub kind: String,
    pub pos: Vec3Dto,
    pub rot_euler_deg: Vec3Dto,
    #[serde(default)]
    pub zone: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpawnerDump {
    pub pos: Vec3Dto,
    pub zone: String,
    #[serde(default)]
    pub spawn_ids: Vec<i32>,
    #[serde(default)]
    pub min_spawn: i32,
    #[serde(default)]
    pub max_spawn: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldDump {
    #[serde(default)]
    pub land_plots: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub gadget_sites: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub gordos: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub treasure_pods: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub access_doors: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub puzzle_slots: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub switches: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub oases: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub quicksilver_generators: Vec<PlacedObjectDump>,
    #[serde(default)]
    pub spawners: Vec<SpawnerDump>,
    /// IdentifiableId (`i32`) -> RGBA `[r, g, b, a]` color from `LookupDirector.GetColor`.
    #[serde(default)]
    pub item_colors: HashMap<i32, [f32; 4]>,
}

impl WorldDump {
    pub fn load_json(path: &Path) -> ToolsResult<Self> {
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save_json(&self, path: &Path) -> ToolsResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
