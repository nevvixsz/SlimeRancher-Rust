//! Port of `EconomyDirector.cs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::{DataRegistry, GameModeSettings};
use sr_ids::IdentifiableId;
use sr_math::{perlin_noise, Randoms};
use sr_model::WorldModel;

pub const DEFAULT_SATURATION_RECOVERY: f32 = 0.25;
pub const DEFAULT_DAILY_SHUTDOWN_MINS: f32 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrValueEntry {
    pub base_value: f32,
    pub curr_value: f32,
    pub prev_value: f32,
    pub full_saturation: f32,
}

/// Port of `EconomyDirector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomyDirector {
    pub saturation_recovery: f32,
    pub daily_shutdown_mins: f32,
    pub next_update_time: f64,
    pub curr_values: HashMap<IdentifiableId, CurrValueEntry>,
}

impl Default for EconomyDirector {
    fn default() -> Self {
        Self {
            saturation_recovery: DEFAULT_SATURATION_RECOVERY,
            daily_shutdown_mins: DEFAULT_DAILY_SHUTDOWN_MINS,
            next_update_time: 0.0,
            curr_values: HashMap::new(),
        }
    }
}

impl EconomyDirector {
    /// Port of `EconomyDirector.InitModel`.
    pub fn init_model(
        &mut self,
        world: &mut WorldModel,
        data: &DataRegistry,
        mode: &GameModeSettings,
        rng: &mut Randoms,
    ) {
        self.next_update_time = 0.0;
        self.curr_values.clear();
        world.econ_seed = rng.get_float(1_000_000.0);

        for (id, entry) in &data.plort_base_values {
            self.curr_values.insert(
                *id,
                CurrValueEntry {
                    base_value: entry.base_value,
                    curr_value: entry.base_value,
                    prev_value: entry.base_value,
                    full_saturation: entry.full_saturation,
                },
            );
            world
                .market_saturation
                .insert(*id, entry.full_saturation * 0.5);
        }

        self.reset_prices(world, mode, 0);
        self.next_update_time = 86400.0;
    }

    /// Port of `EconomyDirector.ResetPrices`.
    pub fn reset_prices(&mut self, world: &mut WorldModel, mode: &GameModeSettings, day: i32) {
        let mut keys: Vec<IdentifiableId> = self.curr_values.keys().copied().collect();
        keys.sort_by_key(|k| *k as i32);

        for id in keys {
            if self.next_update_time > 0.0 {
                if let Some(sat) = world.market_saturation.get_mut(&id) {
                    *sat *= 1.0 - self.saturation_recovery;
                }
            }
            let (base_value, full_saturation) = {
                let entry = &self.curr_values[&id];
                (entry.base_value, entry.full_saturation)
            };
            let target = Self::compute_target_value(
                world,
                mode,
                id,
                base_value,
                full_saturation,
                day as f32,
            );
            if let Some(entry) = self.curr_values.get_mut(&id) {
                entry.prev_value = entry.curr_value;
                entry.curr_value = target;
            }
        }
    }

    /// Port of `EconomyDirector.Update`: triggers daily midnight price update.
    /// Returns `true` if prices were refreshed this tick.
    pub fn step(
        &mut self,
        world: &mut WorldModel,
        mode: &GameModeSettings,
        curr_day: i32,
    ) -> bool {
        if mode.plort_market_dynamic && world.world_time >= self.next_update_time {
            self.reset_prices(world, mode, curr_day);
            let next_day = (world.world_time / 86400.0).floor() + 1.0;
            self.next_update_time = next_day * 86400.0;
            return true;
        }
        false
    }

    /// Port of `EconomyDirector.IsMarketShutdown`.
    pub fn is_market_shutdown(&self, mode: &GameModeSettings, curr_hour: f32) -> bool {
        mode.plort_market_dynamic && curr_hour * 60.0 < self.daily_shutdown_mins
    }

    /// Port of `EconomyDirector.GetCurrValue`.
    pub fn get_curr_value(&self, id: IdentifiableId) -> Option<i32> {
        self.curr_values.get(&id).map(|e| e.curr_value.round() as i32)
    }

    /// Port of `EconomyDirector.GetChangeInValue`.
    pub fn get_change_in_value(&self, id: IdentifiableId) -> Option<i32> {
        self.curr_values
            .get(&id)
            .map(|e| (e.curr_value.round() as i32) - (e.prev_value.round() as i32))
    }

    /// Port of `EconomyDirector.RegisterSold`.
    pub fn register_sold(&self, world: &mut WorldModel, id: IdentifiableId, count: i32) {
        if count <= 0 {
            return;
        }
        let sat = world.market_saturation.entry(id).or_insert(50.0);
        *sat += count as f32;
    }

    /// Exact port of `EconomyDirector.GetTargetValue`.
    pub fn compute_target_value(
        world: &WorldModel,
        mode: &GameModeSettings,
        id: IdentifiableId,
        base_value: f32,
        full_saturation: f32,
        day: f32,
    ) -> f32 {
        if !mode.plort_market_dynamic {
            return base_value * 1.5;
        }
        let sat = world
            .market_saturation
            .get(&id)
            .copied()
            .unwrap_or(full_saturation * 0.5);
        let sat_mult = 1.0 + ((full_saturation - sat) / full_saturation).clamp(0.0, 1.0);
        let market_noise =
            perlin_noise(day as f64, world.econ_seed, -10000.0, 10.0, 0.6, 1.0) + 0.7;
        let item_noise = perlin_noise(
            day as f64,
            world.econ_seed,
            ((id as i32) * 10000) as f32,
            10.0,
            0.6,
            1.0,
        ) + 0.7;
        base_value * sat_mult * market_noise * item_noise
    }
}
