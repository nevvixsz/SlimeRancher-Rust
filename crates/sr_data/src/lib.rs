//! Static & JSON-loadable game definitions (`LookupDirector` data layer).

pub mod game_mode;
pub mod items;
pub mod presets;
pub mod slime;

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, GameMode, IdentifiableId};

pub use game_mode::GameModeSettings;
pub use items::{CraftCost, GadgetDefinition, PlortBaseValue, VacItemDefinition};
pub use slime::{EatMapEntry, SlimeDefinition, SlimeDefinitions, SlimeDiet};

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Unified data registry mirroring the static lookup tables of `LookupDirector`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRegistry {
    pub slimes: SlimeDefinitions,
    pub game_modes: HashMap<GameMode, GameModeSettings>,
    pub plort_base_values: HashMap<IdentifiableId, PlortBaseValue>,
    pub gadgets: HashMap<GadgetId, GadgetDefinition>,
    pub vac_items: HashMap<IdentifiableId, VacItemDefinition>,
}

impl Default for DataRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl DataRegistry {
    pub fn builtin() -> Self {
        let mut plort_base_values = HashMap::new();
        for entry in presets::builtin_plort_base_values() {
            plort_base_values.insert(entry.plort, entry);
        }
        Self {
            slimes: presets::builtin_slime_definitions(),
            game_modes: GameModeSettings::default_map(),
            plort_base_values,
            gadgets: HashMap::new(),
            vac_items: HashMap::new(),
        }
    }

    pub fn from_json_str(json: &str) -> Result<Self, DataError> {
        let mut reg: Self = serde_json::from_str(json)?;
        reg.slimes.refresh_all();
        Ok(reg)
    }

    pub fn load_json_file(path: impl AsRef<Path>) -> Result<Self, DataError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_generates_largos_and_eat_maps() {
        let reg = DataRegistry::builtin();
        let pink = reg.slimes.get(IdentifiableId::PinkSlime).unwrap();
        assert!(!pink.diet.eat_map.is_empty());

        let pink_rock = reg.slimes.get(IdentifiableId::PinkRockLargo).unwrap();
        assert!(pink_rock.is_largo);
        assert!(pink_rock.diet.produces.contains(&IdentifiableId::PinkPlort));
        assert!(pink_rock.diet.produces.contains(&IdentifiableId::RockPlort));

        let mut entries = Vec::new();
        pink_rock.diet.add_eat_map_entries(IdentifiableId::BeetVeggie, &mut entries);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.is_favorite && e.num_to_produce() == 2));

        // Eating a third plort turns a Largo into a Tarr.
        let mut tarr_entries = Vec::new();
        pink_rock.diet.add_eat_map_entries(IdentifiableId::TabbyPlort, &mut tarr_entries);
        assert_eq!(tarr_entries.len(), 1);
        assert_eq!(tarr_entries[0].becomes_id, IdentifiableId::TarrSlime);
    }

    #[test]
    fn json_roundtrip_works() {
        let reg = DataRegistry::builtin();
        let json = serde_json::to_string(&reg).unwrap();
        let loaded = DataRegistry::from_json_str(&json).unwrap();
        assert_eq!(loaded.game_modes.len(), 4);
        assert!(loaded.slimes.get(IdentifiableId::HoneySlime).is_some());
    }
}
