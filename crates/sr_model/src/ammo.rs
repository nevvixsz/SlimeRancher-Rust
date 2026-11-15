//! Port of `AmmoModel.cs` and `Ammo.Slot`.

use serde::{Deserialize, Serialize};
use sr_ids::IdentifiableId;

use crate::actor::SlimeEmotionSnapshot;

/// Port of `Ammo.Slot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmoSlot {
    pub id: IdentifiableId,
    pub count: i32,
    /// Per-captured-actor emotions (`SlimeEmotionData` stack in `Ammo.Slot.emotions`).
    pub emotions: Vec<SlimeEmotionSnapshot>,
}

impl Default for AmmoSlot {
    fn default() -> Self {
        Self {
            id: IdentifiableId::None,
            count: 0,
            emotions: Vec::new(),
        }
    }
}

impl AmmoSlot {
    pub fn is_empty(&self) -> bool {
        self.id == IdentifiableId::None || self.count <= 0
    }

    pub fn clear(&mut self) {
        self.id = IdentifiableId::None;
        self.count = 0;
        self.emotions.clear();
    }
}

/// Port of `AmmoModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmoModel {
    pub slots: Vec<Option<AmmoSlot>>,
    pub usable_slots: usize,
}

impl Default for AmmoModel {
    fn default() -> Self {
        Self::new(5, 4)
    }
}

impl AmmoModel {
    pub fn new(num_slots: usize, init_usable_slots: usize) -> Self {
        Self {
            slots: vec![None; num_slots],
            usable_slots: init_usable_slots,
        }
    }

    pub fn reset(&mut self, num_slots: usize, init_usable_slots: usize) {
        self.slots = vec![None; num_slots];
        self.usable_slots = init_usable_slots;
    }

    pub fn increase_usable_slots(&mut self, usable_slots: usize) {
        self.usable_slots = self.usable_slots.max(usable_slots).min(self.slots.len());
    }

    pub fn get_slot_id(&self, idx: usize) -> IdentifiableId {
        self.slots
            .get(idx)
            .and_then(|s| s.as_ref())
            .filter(|s| !s.is_empty())
            .map(|s| s.id)
            .unwrap_or(IdentifiableId::None)
    }

    pub fn get_slot_count(&self, idx: usize) -> i32 {
        self.slots
            .get(idx)
            .and_then(|s| s.as_ref())
            .map(|s| s.count)
            .unwrap_or(0)
    }
}
