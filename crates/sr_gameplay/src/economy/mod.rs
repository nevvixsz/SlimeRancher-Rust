//! Step 6: Economy, Plort Market, MarketLink, 7Zee Rewards Club, and Range Exchange.

pub mod exchange;
pub mod market;
pub mod plort_market;
pub mod seven_zee;

pub use exchange::{
    AcceptOutcome, ExchangeDirector, ExchangeStepEvents, ProgressRewardLevel, RancherProfile,
    BONUS_PROB, DAILY_OFFER_HOUR, EARLY_EXCHANGE_HOURS, MAX_OFFER_VAL, MIN_OFFER_VAL,
    MOCHI_RECUR_AMOUNT, OGDEN_RECUR_AMOUNT, RANCHER_IDS, RARE_PROB, REWARD_BONUS_MULT,
    REWARD_VAL_MULT, VIKTOR_RECUR_AMOUNT,
};
pub use market::{
    CurrValueEntry, EconomyDirector, DEFAULT_DAILY_SHUTDOWN_MINS, DEFAULT_SATURATION_RECOVERY,
};
pub use plort_market::{
    build_market_listings, deposit_plorts, get_plort_sold_count, plort_sold_stat_key,
    step_market_deposits, CoinsType, DepositResponse, MarketDepositZone, MarketListing, PriceTrend,
    MOCHI_BONUS_CHANCE, MOCHI_BONUS_FACTOR, RUSH_PLORT_BONUS_COUNT, STAT_PLORTS_SOLD,
};
pub use seven_zee::{
    current_seven_zee_rank, next_seven_zee_rank, purchase_next_seven_zee_rank, unlock_seven_zee,
    SevenZeeError, SevenZeePurchaseResult, SevenZeeRank, CHROMA_UNLOCK_RANK,
    CORPORATE_PARTNER_MAX_RANK, SEVEN_ZEE_RANKS, SLIME_TOYS_UNLOCK_RANK,
};

use serde::{Deserialize, Serialize};
use sr_data::{DataRegistry, GameModeSettings};
use sr_ids::IdentifiableId;
use sr_math::Randoms;
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EconomyStepEvents {
    pub prices_updated: bool,
    pub market_deposits: Vec<(IdentifiableId, DepositResponse)>,
    pub exchange: ExchangeStepEvents,
}

/// Unified Step 6 runtime coordinating `EconomyDirector`, deposit zones (`Market` / `MarketLink`),
/// and `ExchangeDirector` (Range Exchange).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EconomyRuntime {
    pub market: EconomyDirector,
    pub deposit_zones: Vec<MarketDepositZone>,
    pub exchange: ExchangeDirector,
    pub initialized: bool,
}

impl EconomyRuntime {
    pub fn ensure_initialized(
        &mut self,
        model: &mut GameModel,
        data: &DataRegistry,
        mode: &GameModeSettings,
        rng: &mut Randoms,
    ) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.market.init_model(&mut model.world, data, mode, rng);
        ExchangeDirector::init_model(&mut model.world, mode);
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        data: &DataRegistry,
        mode: &GameModeSettings,
        curr_day: i32,
        curr_hour: f32,
        rng: &mut Randoms,
    ) -> EconomyStepEvents {
        self.ensure_initialized(model, data, mode, rng);

        let prices_updated = self.market.step(&mut model.world, mode, curr_day);
        let market_deposits = step_market_deposits(
            &self.market,
            model,
            physics,
            mode,
            &self.deposit_zones,
            curr_hour,
            rng,
        );
        let exchange = self.exchange.step(model, mode, curr_day, rng);

        EconomyStepEvents {
            prices_updated,
            market_deposits,
            exchange,
        }
    }
}
