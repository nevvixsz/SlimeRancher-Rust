//! Item, gadget, and economy configuration schemas ported from
//! `VacItemDefinition.cs`, `GadgetDefinition.cs`, and `EconomyDirector.cs`.

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, IdentifiableId, PediaId};

/// Port of `VacItemDefinition`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VacItemDefinition {
    pub id: IdentifiableId,
    /// RGBA color in `[0.0, 1.0]` used by vacpack HUD / effects.
    pub color: [f32; 4],
}

/// Port of `GadgetDefinition.CraftCost`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftCost {
    pub id: IdentifiableId,
    pub amount: i32,
}

/// Port of `GadgetDefinition`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GadgetDefinition {
    pub id: GadgetId,
    pub pedia_link: PediaId,
    pub blueprint_cost: i32,
    #[serde(default)]
    pub craft_costs: Vec<CraftCost>,
    pub buy_count_limit: i32,
    pub count_limit: i32,
    #[serde(default)]
    pub count_other_ids: Vec<GadgetId>,
    pub destroy_on_removal: bool,
    pub buy_in_pairs: bool,
}

impl GadgetDefinition {
    /// Port of `GadgetDefinition.GetGadgetsToCountIds()`.
    pub fn gadgets_to_count_ids(&self) -> Vec<GadgetId> {
        let mut list = self.count_other_ids.clone();
        list.push(self.id);
        list
    }
}

/// Port of `EconomyDirector.ValueMap`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlortBaseValue {
    pub plort: IdentifiableId,
    pub base_value: f32,
    pub full_saturation: f32,
}
