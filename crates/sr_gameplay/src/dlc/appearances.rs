//! Port of `SlimeAppearanceDirector.cs` and `AppearancesModel` (Secret Style DLC).

use serde::{Deserialize, Serialize};
use sr_data::SlimeDefinitions;
use sr_ids::IdentifiableId;
use sr_model::{AppearanceSaveSet, GameModel};

/// Port of `SlimeAppearanceDirector` managing Classic / Secret Style unlocks,
/// selections, and automatic dependent Largo appearance propagation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlimeAppearanceDirector {
    pub unlocked_all_secret_styles: bool,
}

impl SlimeAppearanceDirector {
    /// Ensures every slime in `defs` has `AppearanceSaveSet::Classic` unlocked and selected by default.
    pub fn init_defaults(model: &mut GameModel, defs: &SlimeDefinitions) {
        for def in defs.slimes.values() {
            model
                .appearances
                .unlocks
                .entry(def.identifiable_id)
                .or_default()
                .insert(AppearanceSaveSet::Classic);
            model
                .appearances
                .selections
                .entry(def.identifiable_id)
                .or_insert(AppearanceSaveSet::Classic);
        }
    }

    pub fn is_unlocked(model: &GameModel, slime_id: IdentifiableId, set: AppearanceSaveSet) -> bool {
        if set == AppearanceSaveSet::Classic {
            return true;
        }
        model
            .appearances
            .unlocks
            .get(&slime_id)
            .map(|s| s.contains(&set))
            .unwrap_or(false)
    }

    pub fn selected_appearance(model: &GameModel, slime_id: IdentifiableId) -> AppearanceSaveSet {
        model
            .appearances
            .selections
            .get(&slime_id)
            .copied()
            .unwrap_or(AppearanceSaveSet::Classic)
    }

    /// Unlocks `set` (e.g. `AppearanceSaveSet::SecretStyle` from a Secret Style pod) for `slime_id`.
    pub fn unlock_appearance(
        &self,
        model: &mut GameModel,
        slime_id: IdentifiableId,
        set: AppearanceSaveSet,
    ) -> bool {
        model
            .appearances
            .unlocks
            .entry(slime_id)
            .or_default()
            .insert(set)
    }

    /// Selects `set` for `slime_id` (if unlocked) and updates any Largo `DependentAppearances`
    /// whose base slimes both use the matching style (`SlimeAppearanceDirector.UpdateChosenSlimeAppearance`).
    pub fn select_appearance(
        &self,
        model: &mut GameModel,
        defs: &SlimeDefinitions,
        slime_id: IdentifiableId,
        set: AppearanceSaveSet,
    ) -> Vec<IdentifiableId> {
        if !Self::is_unlocked(model, slime_id, set) {
            return Vec::new();
        }
        model.appearances.selections.insert(slime_id, set);

        let mut updated = vec![slime_id];
        for def in defs.slimes.values() {
            if def.is_largo && def.base_slimes.contains(&slime_id) {
                let all_secret = !def.base_slimes.is_empty()
                    && def.base_slimes.iter().all(|&base_id| {
                        Self::selected_appearance(model, base_id) == AppearanceSaveSet::SecretStyle
                    });
                let largo_set = if all_secret {
                    AppearanceSaveSet::SecretStyle
                } else {
                    AppearanceSaveSet::Classic
                };
                if all_secret {
                    model
                        .appearances
                        .unlocks
                        .entry(def.identifiable_id)
                        .or_default()
                        .insert(AppearanceSaveSet::SecretStyle);
                }
                model
                    .appearances
                    .selections
                    .insert(def.identifiable_id, largo_set);
                updated.push(def.identifiable_id);
            }
        }
        updated
    }
}
