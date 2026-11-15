//! Port of `Ammo.cs` — vacpack inventory slots, liquid rules, magic water, and slime emotions.

use serde::{Deserialize, Serialize};
use sr_ids::{AmmoMode, IdentifiableId};
use sr_model::{AmmoModel, AmmoSlot, PlayerModel, SlimeEmotionSnapshot};

pub const GLITCH_DEBUG_SPRAY_MAX_AMMO: i32 = 20;
pub const GLITCH_SLIME_MAX_AMMO: i32 = 100;

/// Port of `Ammo` runtime controller operating on `PlayerModel::ammo_dict`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoController {
    pub selected_idx: usize,
    pub water_is_magic_until: f64,
}

impl Default for AmmoController {
    fn default() -> Self {
        Self {
            selected_idx: 0,
            water_is_magic_until: f64::NEG_INFINITY,
        }
    }
}

impl AmmoController {
    /// Checks whether `id` is allowed in vacpack `slot_idx` under `mode`.
    pub fn slot_accepts(mode: AmmoMode, slot_idx: usize, id: IdentifiableId) -> bool {
        if id == IdentifiableId::None {
            return false;
        }
        match mode {
            AmmoMode::Default => {
                if slot_idx < 4 {
                    !id.is_liquid()
                } else if slot_idx == 4 {
                    id.is_liquid()
                } else {
                    false
                }
            }
            AmmoMode::NimbleValley => match slot_idx {
                0 => id == IdentifiableId::QuicksilverPlort,
                1 | 2 => matches!(
                    id,
                    IdentifiableId::ValleyAmmo1
                        | IdentifiableId::ValleyAmmo2
                        | IdentifiableId::ValleyAmmo3
                        | IdentifiableId::ValleyAmmo4
                ),
                _ => false,
            },
        }
    }

    /// Checks whether `id` is vacpack-storable in `mode`.
    pub fn is_potential_ammo(mode: AmmoMode, id: IdentifiableId) -> bool {
        match mode {
            AmmoMode::Default => {
                id != IdentifiableId::None
                    && id != IdentifiableId::Player
                    && id != IdentifiableId::TarrSlime
                    && !id.is_largo()
                    && (id.is_slime()
                        || id.is_plort()
                        || id.is_food()
                        || id.is_animal()
                        || id.is_craft()
                        || id.is_liquid()
                        || id.is_echo()
                        || id.is_ornament()
                        || id == IdentifiableId::GlitchBugReport
                        || id == IdentifiableId::LemonPhase)
            }
            AmmoMode::NimbleValley => matches!(
                id,
                IdentifiableId::QuicksilverPlort
                    | IdentifiableId::ValleyAmmo1
                    | IdentifiableId::ValleyAmmo2
                    | IdentifiableId::ValleyAmmo3
                    | IdentifiableId::ValleyAmmo4
            ),
        }
    }

    /// Port of `PlayerState.GetMaxAmmo_Default` & `GetMaxAmmo_NimbleValley`.
    pub fn slot_max_count(player: &PlayerModel, id: IdentifiableId, slot_idx: usize) -> i32 {
        match player.ammo_mode {
            AmmoMode::Default => {
                if id == IdentifiableId::GlitchDebugSprayLiquid {
                    GLITCH_DEBUG_SPRAY_MAX_AMMO
                } else if id == IdentifiableId::GlitchSlime || id == IdentifiableId::GlitchBugReport {
                    GLITCH_SLIME_MAX_AMMO
                } else {
                    player.max_ammo
                }
            }
            AmmoMode::NimbleValley => match slot_idx {
                0 => 999,
                1 => 1,
                2 => 3,
                _ => 0,
            },
        }
    }

    pub fn adjust_id(&self, id: IdentifiableId, world_time: f64) -> IdentifiableId {
        if id.is_water() && world_time < self.water_is_magic_until {
            IdentifiableId::MagicWaterLiquid
        } else {
            id
        }
    }

    pub fn usable_slot_count(player: &PlayerModel) -> usize {
        player
            .active_ammo()
            .map(|a| a.usable_slots)
            .unwrap_or(0)
    }

    pub fn select_slot(&mut self, player: &PlayerModel, idx: usize) -> bool {
        let usable = Self::usable_slot_count(player);
        if self.selected_idx != idx && idx < usable {
            self.selected_idx = idx;
            true
        } else {
            false
        }
    }

    pub fn next_slot(&mut self, player: &PlayerModel) {
        let usable = Self::usable_slot_count(player).max(1);
        self.selected_idx = (self.selected_idx + 1) % usable;
    }

    pub fn prev_slot(&mut self, player: &PlayerModel) {
        let usable = Self::usable_slot_count(player).max(1);
        self.selected_idx = (self.selected_idx + usable - 1) % usable;
    }

    pub fn selected_id(&self, player: &PlayerModel, world_time: f64) -> IdentifiableId {
        let Some(ammo) = player.active_ammo() else {
            return IdentifiableId::None;
        };
        let raw = ammo.get_slot_id(self.selected_idx);
        if raw == IdentifiableId::None {
            IdentifiableId::None
        } else {
            self.adjust_id(raw, world_time)
        }
    }

    pub fn has_selected_ammo(&self, player: &PlayerModel) -> bool {
        player
            .active_ammo()
            .and_then(|a| a.slots.get(self.selected_idx))
            .and_then(|s| s.as_ref())
            .map(|s| s.count > 0)
            .unwrap_or(false)
    }

    pub fn slot_id(&self, player: &PlayerModel, slot_idx: usize, world_time: f64) -> IdentifiableId {
        let Some(ammo) = player.active_ammo() else {
            return IdentifiableId::None;
        };
        let raw = ammo.get_slot_id(slot_idx);
        if raw == IdentifiableId::None {
            IdentifiableId::None
        } else {
            self.adjust_id(raw, world_time)
        }
    }

    pub fn slot_count(&self, player: &PlayerModel, slot_idx: usize) -> i32 {
        player
            .active_ammo()
            .map(|a| a.get_slot_count(slot_idx))
            .unwrap_or(0)
    }

    /// Port of `Ammo.MaybeAddToSlot` (prefers existing matching stack, then first compatible empty slot).
    pub fn maybe_add_to_slot(
        &mut self,
        player: &mut PlayerModel,
        raw_id: IdentifiableId,
        emotions: Option<SlimeEmotionSnapshot>,
        count: i32,
        world_time: f64,
    ) -> bool {
        let mut id = raw_id;
        if id == IdentifiableId::MagicWaterLiquid {
            id = IdentifiableId::WaterLiquid;
            // Magic water lasts 30 game-minutes (1800s) when collected from Oasis
            self.water_is_magic_until = world_time + 1800.0;
        }
        let mode = player.ammo_mode;
        if !Self::is_potential_ammo(mode, id) {
            return false;
        }

        let usable = Self::usable_slot_count(player);
        // 1. Try existing matching slot
        for idx in 0..usable {
            let max_c = Self::slot_max_count(player, id, idx);
            if let Some(ammo) = player.ammo_dict.get_mut(&mode) {
                if let Some(Some(slot)) = ammo.slots.get_mut(idx) {
                    if slot.id == id && slot.count + count <= max_c {
                        slot.count += count;
                        if let Some(em) = emotions {
                            slot.emotions.push(em);
                        }
                        return true;
                    }
                }
            }
        }
        // 2. Try first empty slot passing predicate
        for idx in 0..usable {
            if !Self::slot_accepts(mode, idx, id) {
                continue;
            }
            let max_c = Self::slot_max_count(player, id, idx);
            if count > max_c {
                continue;
            }
            if let Some(ammo) = player.ammo_dict.get_mut(&mode) {
                if let Some(slot_opt) = ammo.slots.get_mut(idx) {
                    if slot_opt.is_none() {
                        let mut em_vec = Vec::new();
                        if let Some(em) = emotions {
                            em_vec.push(em);
                        }
                        *slot_opt = Some(AmmoSlot {
                            id,
                            count,
                            emotions: em_vec,
                        });
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn replace_slot(&mut self, player: &mut PlayerModel, slot_idx: usize, mut id: IdentifiableId, world_time: f64) -> bool {
        if id == IdentifiableId::MagicWaterLiquid {
            id = IdentifiableId::WaterLiquid;
            self.water_is_magic_until = world_time + 1800.0;
        }
        let mode = player.ammo_mode;
        if !Self::slot_accepts(mode, slot_idx, id) {
            return false;
        }
        if let Some(ammo) = player.ammo_dict.get_mut(&mode) {
            if let Some(slot_opt) = ammo.slots.get_mut(slot_idx) {
                *slot_opt = Some(AmmoSlot {
                    id,
                    count: 1,
                    emotions: Vec::new(),
                });
                return true;
            }
        }
        false
    }

    /// Port of `Ammo.DecrementSelectedAmmo`. Pops one emotion snapshot if present.
    pub fn decrement_selected(
        &mut self,
        player: &mut PlayerModel,
        amount: i32,
        world_time: f64,
    ) -> Option<SlimeEmotionSnapshot> {
        let mode = player.ammo_mode;
        let idx = self.selected_idx;
        let ammo: &mut AmmoModel = player.ammo_dict.get_mut(&mode)?;
        let slot_opt = ammo.slots.get_mut(idx)?;
        let slot = slot_opt.as_mut()?;

        if slot.id.is_water() && world_time < self.water_is_magic_until {
            return None;
        }

        let popped_em = slot.emotions.pop();
        slot.count = (slot.count - amount).max(0);
        if slot.count == 0 {
            *slot_opt = None;
        }
        popped_em
    }

    pub fn clear_slot(&mut self, player: &mut PlayerModel, idx: usize) {
        let mode = player.ammo_mode;
        if let Some(ammo) = player.ammo_dict.get_mut(&mode) {
            if let Some(slot_opt) = ammo.slots.get_mut(idx) {
                if slot_opt
                    .as_ref()
                    .map(|s| s.id.is_water())
                    .unwrap_or(false)
                {
                    self.water_is_magic_until = f64::NEG_INFINITY;
                }
                *slot_opt = None;
            }
        }
    }
}
