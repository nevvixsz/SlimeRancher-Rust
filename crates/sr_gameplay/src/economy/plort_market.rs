//! Port of `ScorePlort.cs`, `MarketUI.cs`, and `MarketLink` gadget deposit handling.

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GameMode, IdentifiableId, ProgressType};
use sr_math::{Randoms, Vec3};
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

use crate::economy::market::EconomyDirector;
use crate::player::PlayerController;

pub const MOCHI_BONUS_CHANCE: f32 = 0.05;
pub const MOCHI_BONUS_FACTOR: f32 = 2.0;
pub const RUSH_PLORT_BONUS_COUNT: i32 = 25;
pub const STAT_PLORTS_SOLD: &str = "PLORTS_SOLD";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoinsType {
    Norm,
    Mochi,
    Drones,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PriceTrend {
    Up,
    Down,
    Unchanged,
    BonusComplete,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DepositResponse {
    pub deposits: i32,
    pub currency: i32,
    pub mochi_bonus_triggers: i32,
    pub coins_type: Option<CoinsType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDepositZone {
    pub id: String,
    pub pos: Vec3,
    pub radius: f32,
    pub is_market_link: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketListing {
    pub id: IdentifiableId,
    pub curr_value: i32,
    pub change_in_value: i32,
    pub trend: PriceTrend,
    pub sold_count: i32,
    pub rush_bonus_complete: bool,
}

pub fn plort_sold_stat_key(id: IdentifiableId) -> String {
    format!("PLORT_TYPES_SOLD:{}", id as i32)
}

pub fn get_plort_sold_count(model: &GameModel, id: IdentifiableId) -> i32 {
    model
        .game_achieves
        .counts
        .get(&plort_sold_stat_key(id))
        .copied()
        .unwrap_or(0)
}

/// Port of `ScorePlort.Deposit`.
pub fn deposit_plorts(
    econ: &EconomyDirector,
    model: &mut GameModel,
    mode: &GameModeSettings,
    id: IdentifiableId,
    count: i32,
    curr_hour: f32,
    plort_price_factor: f32,
    ignore_market_shutdown: bool,
    rng: &mut Randoms,
) -> DepositResponse {
    if count <= 0 {
        return DepositResponse::default();
    }
    if !ignore_market_shutdown && econ.is_market_shutdown(mode, curr_hour) {
        return DepositResponse::default();
    }
    let Some(base_val) = econ.get_curr_value(id) else {
        return DepositResponse::default();
    };
    let effective_val = ((base_val as f32) * plort_price_factor).round() as i32;
    let has_mochi = model.progress.get_progress(ProgressType::MochiRewards) >= 1;

    let mut resp = DepositResponse {
        deposits: count,
        currency: 0,
        mochi_bonus_triggers: 0,
        coins_type: Some(CoinsType::Norm),
    };

    for _ in 0..count {
        if has_mochi && rng.get_probability(MOCHI_BONUS_CHANCE) {
            resp.coins_type = Some(CoinsType::Mochi);
            resp.mochi_bonus_triggers += 1;
            resp.currency += ((effective_val as f32) * MOCHI_BONUS_FACTOR).round() as i32;
        } else {
            resp.currency += effective_val;
        }
    }

    PlayerController::add_currency(&mut model.player, resp.currency);
    *model
        .game_achieves
        .counts
        .entry(STAT_PLORTS_SOLD.to_string())
        .or_insert(0) += count;
    *model
        .game_achieves
        .counts
        .entry(plort_sold_stat_key(id))
        .or_insert(0) += count;

    econ.register_sold(&mut model.world, id, count);
    resp
}

/// Scans world plort actors overlapping any `MarketDepositZone` (Ranch Market or `MarketLink`),
/// sells them into the economy, and removes the physical actors.
pub fn step_market_deposits(
    econ: &EconomyDirector,
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    mode: &GameModeSettings,
    zones: &[MarketDepositZone],
    curr_hour: f32,
    rng: &mut Randoms,
) -> Vec<(IdentifiableId, DepositResponse)> {
    if zones.is_empty() || econ.is_market_shutdown(mode, curr_hour) {
        return Vec::new();
    }

    let mut to_sell: Vec<(i64, IdentifiableId)> = Vec::new();
    for (&aid, actor) in &model.actors {
        if econ.get_curr_value(actor.ident).is_none() {
            continue;
        }
        let hit = zones
            .iter()
            .any(|z| actor.pos.distance_squared(z.pos) <= z.radius * z.radius);
        if hit {
            to_sell.push((aid, actor.ident));
        }
    }

    let mut out = Vec::with_capacity(to_sell.len());
    for (aid, ident) in to_sell {
        let resp = deposit_plorts(econ, model, mode, ident, 1, curr_hour, 1.0, false, rng);
        if resp.deposits > 0 {
            model.destroy_actor(aid);
            physics.remove_body(aid);
            out.push((ident, resp));
        }
    }
    out
}

/// Port of `MarketUI.PlortCountUpdate`: computes display values and price delta trends.
pub fn build_market_listings(
    econ: &EconomyDirector,
    model: &GameModel,
    mode: &GameModeSettings,
) -> Vec<MarketListing> {
    let mut keys: Vec<IdentifiableId> = econ.curr_values.keys().copied().collect();
    keys.sort_by_key(|k| *k as i32);

    let mut listings = Vec::with_capacity(keys.len());
    for id in keys {
        if id == IdentifiableId::SaberPlort && model.game_mode == GameMode::TimeLimitV2 {
            continue;
        }
        let curr_value = econ.get_curr_value(id).unwrap_or(0);
        let change = econ.get_change_in_value(id).unwrap_or(0);
        let sold_count = get_plort_sold_count(model, id);
        let rush_bonus_complete =
            model.game_mode == GameMode::TimeLimitV2 && sold_count >= RUSH_PLORT_BONUS_COUNT;

        let trend = if model.game_mode == GameMode::TimeLimitV2 {
            if rush_bonus_complete {
                PriceTrend::BonusComplete
            } else {
                PriceTrend::Hidden
            }
        } else if !mode.plort_market_dynamic {
            PriceTrend::Hidden
        } else if change == 0 {
            PriceTrend::Unchanged
        } else if change > 0 {
            PriceTrend::Up
        } else {
            PriceTrend::Down
        };

        listings.push(MarketListing {
            id,
            curr_value,
            change_in_value: change,
            trend,
            sold_count,
            rush_bonus_complete,
        });
    }
    listings
}
