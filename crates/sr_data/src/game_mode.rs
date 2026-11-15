//! Port of `GameModeSettings.cs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::{GameMode, IdentifiableId};

/// Port of `GameModeSettings`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameModeSettings {
    pub init_currency: i32,
    pub pct_currency_lost_on_death: f32,
    pub hours_lost_on_death: f32,
    pub hours_til_dawn_on_death: bool,
    pub exchange_start_day: i32,
    pub exchange_rewards_gold_plorts: bool,
    pub plort_market_dynamic: bool,
    /// The day on which we should end the game, or 0.0 if endless.
    pub end_at_noon_day: f64,
    pub assume_experienced_user: bool,
    pub immediate_upgrades: bool,
    pub enable_partner_rewards: bool,
    pub suppress_story: bool,
    pub player_damage_multiplier: f32,
    pub prevent_hostiles: bool,
    pub blueprints_enabled: bool,
    pub enable_ogden_missions: bool,
    pub enable_mochi_missions: bool,
    pub enable_viktor_missions: bool,
    pub enable_event_gordos: bool,
    pub enable_echo_note_gordos: bool,
    pub enable_dlc: bool,
}

impl Default for GameModeSettings {
    fn default() -> Self {
        Self::classic()
    }
}

impl GameModeSettings {
    /// Port of `GameModeSettings.AllowMail()`.
    pub fn allow_mail(&self) -> bool {
        !self.assume_experienced_user || !self.suppress_story
    }

    /// Port of `GameModeSettings.EndTime()`.
    pub fn end_time(&self) -> Option<f64> {
        if self.end_at_noon_day > 0.0 {
            Some(86400.0 * (self.end_at_noon_day - 0.5))
        } else {
            None
        }
    }

    /// Exact port of `GameModeSettings.GetScoreMultiplier(Identifiable.Id)` (Rush Mode V2).
    pub fn score_multiplier(id: IdentifiableId) -> f32 {
        match id {
            IdentifiableId::DervishPlort
            | IdentifiableId::FirePlort
            | IdentifiableId::MosaicPlort
            | IdentifiableId::QuantumPlort
            | IdentifiableId::TanglePlort => 0.08,
            _ => 0.05,
        }
    }

    /// Exact port of `GameModeSettings.PlortBonusReached(int)`.
    pub const fn plort_bonus_reached(plorts_collected: i32) -> bool {
        plorts_collected >= 25
    }

    /// Standard Adventure mode (`GameMode::Classic`).
    pub fn classic() -> Self {
        Self {
            init_currency: 250,
            pct_currency_lost_on_death: 0.0,
            hours_lost_on_death: 0.0,
            hours_til_dawn_on_death: true,
            exchange_start_day: 2,
            exchange_rewards_gold_plorts: false,
            plort_market_dynamic: true,
            end_at_noon_day: 0.0,
            assume_experienced_user: false,
            immediate_upgrades: false,
            enable_partner_rewards: true,
            suppress_story: false,
            player_damage_multiplier: 1.0,
            prevent_hostiles: false,
            blueprints_enabled: true,
            enable_ogden_missions: true,
            enable_mochi_missions: true,
            enable_viktor_missions: true,
            enable_event_gordos: true,
            enable_echo_note_gordos: true,
            enable_dlc: true,
        }
    }

    /// Casual mode (`GameMode::Casual`): no Tarr, no feral attacks.
    pub fn casual() -> Self {
        Self {
            prevent_hostiles: true,
            ..Self::classic()
        }
    }

    /// Legacy 5-Day Rush (`GameMode::TimeLimit`).
    pub fn time_limit() -> Self {
        Self {
            init_currency: 1000,
            hours_til_dawn_on_death: false,
            hours_lost_on_death: 12.0,
            exchange_start_day: 1,
            exchange_rewards_gold_plorts: true,
            plort_market_dynamic: false,
            end_at_noon_day: 6.0,
            assume_experienced_user: true,
            immediate_upgrades: true,
            enable_partner_rewards: false,
            suppress_story: true,
            enable_ogden_missions: false,
            enable_mochi_missions: false,
            enable_viktor_missions: false,
            ..Self::classic()
        }
    }

    /// Rush Mode V2 (`GameMode::TimeLimitV2`).
    pub fn time_limit_v2() -> Self {
        Self {
            end_at_noon_day: 4.0,
            ..Self::time_limit()
        }
    }

    pub fn default_map() -> HashMap<GameMode, GameModeSettings> {
        let mut map = HashMap::new();
        map.insert(GameMode::Classic, Self::classic());
        map.insert(GameMode::Casual, Self::casual());
        map.insert(GameMode::TimeLimit, Self::time_limit());
        map.insert(GameMode::TimeLimitV2, Self::time_limit_v2());
        map
    }
}
