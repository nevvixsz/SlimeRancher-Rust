//! Port of `SiloStorage.cs` and `LinkedSiloStorage.cs` (1–4 storage sides,
//! 3 slots per side, 300 capacity per slot).

use sr_ids::{IdentifiableId, LandPlotId, LandPlotUpgrade};
use sr_model::{AmmoModel, AmmoSlot, LandPlotModel, StorageType};

pub const SILO_SLOTS_PER_SIDE: usize = 3;
pub const SILO_MAX_SIDES: usize = 4;
pub const SILO_TOTAL_SLOTS: usize = SILO_SLOTS_PER_SIDE * SILO_MAX_SIDES; // 12
pub const SILO_MAX_PER_SLOT: i32 = 300;
pub const SILO_FULL_ACHIEVE_SLOTS: usize = 12;
pub const SILO_FULL_ACHIEVE_AMOUNT: i32 = 50;

pub struct SiloController;

impl SiloController {
    /// Returns how many storage sides (`1..=4`) are unlocked on this Silo.
    pub fn unlocked_sides(plot: &LandPlotModel) -> usize {
        if plot.type_id != LandPlotId::Silo {
            return 0;
        }
        if plot.has_upgrade(LandPlotUpgrade::Storage4) {
            4
        } else if plot.has_upgrade(LandPlotUpgrade::Storage3) {
            3
        } else if plot.has_upgrade(LandPlotUpgrade::Storage2) {
            2
        } else {
            1
        }
    }

    pub fn usable_slot_count(plot: &LandPlotModel) -> usize {
        Self::unlocked_sides(plot) * SILO_SLOTS_PER_SIDE
    }

    /// Mirrors `SiloStorage.StorageType.NON_SLIMES` filter.
    pub fn can_store_item(id: IdentifiableId) -> bool {
        id != IdentifiableId::None
            && !id.is_slime()
            && !id.is_liquid()
            && !id.is_toy()
            && !id.is_gordo()
            && !id.is_standard_crate()
    }

    /// Deposits up to `count` of `id` into specific Silo `slot_idx` (`0..usable_slot_count`).
    /// Returns number of items actually stored.
    pub fn deposit_to_slot(
        plot: &mut LandPlotModel,
        slot_idx: usize,
        id: IdentifiableId,
        count: i32,
    ) -> i32 {
        let usable = Self::usable_slot_count(plot);
        if slot_idx >= usable || !Self::can_store_item(id) || count <= 0 {
            return 0;
        }
        let ammo = plot
            .silo_ammo
            .entry(StorageType::NonSlimes)
            .or_insert_with(|| AmmoModel::new(SILO_TOTAL_SLOTS, usable));
        ammo.usable_slots = usable;

        let slot = ammo.slots[slot_idx].get_or_insert_with(AmmoSlot::default);
        if slot.is_empty() {
            slot.id = id;
            slot.count = 0;
        }
        if slot.id != id {
            return 0;
        }
        let space = (SILO_MAX_PER_SLOT - slot.count).max(0);
        let added = space.min(count);
        slot.count += added;
        added
    }

    /// Deposits `count` into matching existing slots first, then empty usable slots.
    pub fn deposit_any(plot: &mut LandPlotModel, id: IdentifiableId, mut count: i32) -> i32 {
        let usable = Self::usable_slot_count(plot);
        if usable == 0 || !Self::can_store_item(id) || count <= 0 {
            return 0;
        }
        let mut total_added = 0;
        // Pass 1: stack onto matching slots
        for idx in 0..usable {
            if count <= 0 {
                break;
            }
            let matches = plot
                .silo_ammo
                .get(&StorageType::NonSlimes)
                .and_then(|a| a.slots.get(idx))
                .and_then(|o| o.as_ref())
                .map_or(false, |s| !s.is_empty() && s.id == id);
            if matches {
                let added = Self::deposit_to_slot(plot, idx, id, count);
                total_added += added;
                count -= added;
            }
        }
        // Pass 2: empty slots
        for idx in 0..usable {
            if count <= 0 {
                break;
            }
            let added = Self::deposit_to_slot(plot, idx, id, count);
            total_added += added;
            count -= added;
        }
        total_added
    }

    /// Withdraws up to `max_take` from `slot_idx`.
    pub fn withdraw_from_slot(
        plot: &mut LandPlotModel,
        slot_idx: usize,
        max_take: i32,
    ) -> Option<(IdentifiableId, i32)> {
        let usable = Self::usable_slot_count(plot);
        if slot_idx >= usable || max_take <= 0 {
            return None;
        }
        let ammo = plot.silo_ammo.get_mut(&StorageType::NonSlimes)?;
        let slot = ammo.slots.get_mut(slot_idx)?.as_mut()?;
        if slot.is_empty() {
            return None;
        }
        let id = slot.id;
        let taken = slot.count.min(max_take);
        slot.count -= taken;
        if slot.count <= 0 {
            slot.clear();
        }
        Some((id, taken))
    }

    /// Cycles the active slot index (`0..3`) on storage side `side_idx` (`0..4`).
    pub fn cycle_side_slot(plot: &mut LandPlotModel, side_idx: usize, delta: i32) -> Option<i32> {
        if side_idx >= Self::unlocked_sides(plot) {
            return None;
        }
        let cur = plot.silo_storage_indices[side_idx];
        let next = (cur + delta).rem_euclid(SILO_SLOTS_PER_SIDE as i32);
        plot.silo_storage_indices[side_idx] = next;
        Some(next)
    }

    pub fn active_slot_for_side(plot: &LandPlotModel, side_idx: usize) -> usize {
        let local = plot.silo_storage_indices[side_idx].clamp(0, (SILO_SLOTS_PER_SIDE - 1) as i32)
            as usize;
        side_idx * SILO_SLOTS_PER_SIDE + local
    }

    /// Matches `SiloStorage.HasFullSlots(12, 50)` for the `FILLED_SILO` achievement.
    pub fn has_filled_silo_achievement(plot: &LandPlotModel) -> bool {
        if Self::unlocked_sides(plot) < SILO_MAX_SIDES {
            return false;
        }
        let Some(ammo) = plot.silo_ammo.get(&StorageType::NonSlimes) else {
            return false;
        };
        ammo.slots
            .iter()
            .take(SILO_FULL_ACHIEVE_SLOTS)
            .filter(|o| o.as_ref().map_or(false, |s| !s.is_empty() && s.count >= SILO_FULL_ACHIEVE_AMOUNT))
            .count()
            >= SILO_FULL_ACHIEVE_SLOTS
    }
}
