//! Port of `AchievementsDirector.cs`, `EndGameUI.cs`, and `EndGameV2UI.cs`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GameMode, IdentifiableId};
use sr_model::{GameModel, RanchModel};

use crate::economy::{get_plort_sold_count, EconomyDirector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AchievementTier {
    Tier1,
    Tier2,
    Tier3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AchievementId {
    SellPlortsA,
    SellPlortsB,
    SellPlortsC,
    SellPlortsD,
    SellPlortsE,
    FeedSlimesChickens,
    FruitTreeTypes,
    VeggiePatchTypes,
    EarnCurrencyA,
    EarnCurrencyB,
    EarnCurrencyC,
    DayCurrency,
    AwakeUntilMorning,
    KnockoutMorning,
    AwayFromRanch,
    PinkSlimesFoodTypes,
    FeedAirborne,
    DiscoveredQuarry,
    DiscoveredMoss,
    DiscoveredDesert,
    BurstGordo,
    OpenSlimeGate,
    IncinerateElderChicken,
    FilledSilo,
    RanchUpgradedStorage,
    FulfillExchangeEarly,
    FeedFavorites,
    DayCollectPlorts,
    GoldSlimeTriplePlort,
    ExtendedRadExposure,
    ExtendedTarrHold,
    TabbyHeadbutt,
    LaunchedBoomExplode,
    ManySlimesInVac,
    CorralSlimeTypes,
    CorralLargoTypes,
    PondSlimeTypes,
    RanchLargoTypes,
    EnteredCorralSlimes,
    IncinerateChick,
    TimeLimitCurrencyA,
    TimeLimitCurrencyB,
    TimeLimitCurrencyC,
    DiscoveredRuins,
    FabricateGadgetsA,
    FabricateGadgetsB,
    FabricateGadgetsC,
    SlimeStageTarr,
    SlimeballScore,
    JoinRewardsClub,
    UseChromas,
    CollectSlimeToys,
    FindHobsonsEnd,
    SnareHunterGordo,
    ActivateOasis,
    FinishAdventure,
    CompleteSlimepedia,
}

impl AchievementId {
    pub const ALL: &'static [Self] = &[
        Self::SellPlortsA,
        Self::SellPlortsB,
        Self::SellPlortsC,
        Self::SellPlortsD,
        Self::SellPlortsE,
        Self::FeedSlimesChickens,
        Self::FruitTreeTypes,
        Self::VeggiePatchTypes,
        Self::EarnCurrencyA,
        Self::EarnCurrencyB,
        Self::EarnCurrencyC,
        Self::DayCurrency,
        Self::AwakeUntilMorning,
        Self::KnockoutMorning,
        Self::AwayFromRanch,
        Self::PinkSlimesFoodTypes,
        Self::FeedAirborne,
        Self::DiscoveredQuarry,
        Self::DiscoveredMoss,
        Self::DiscoveredDesert,
        Self::BurstGordo,
        Self::OpenSlimeGate,
        Self::IncinerateElderChicken,
        Self::FilledSilo,
        Self::RanchUpgradedStorage,
        Self::FulfillExchangeEarly,
        Self::FeedFavorites,
        Self::DayCollectPlorts,
        Self::GoldSlimeTriplePlort,
        Self::ExtendedRadExposure,
        Self::ExtendedTarrHold,
        Self::TabbyHeadbutt,
        Self::LaunchedBoomExplode,
        Self::ManySlimesInVac,
        Self::CorralSlimeTypes,
        Self::CorralLargoTypes,
        Self::PondSlimeTypes,
        Self::RanchLargoTypes,
        Self::EnteredCorralSlimes,
        Self::IncinerateChick,
        Self::TimeLimitCurrencyA,
        Self::TimeLimitCurrencyB,
        Self::TimeLimitCurrencyC,
        Self::DiscoveredRuins,
        Self::FabricateGadgetsA,
        Self::FabricateGadgetsB,
        Self::FabricateGadgetsC,
        Self::SlimeStageTarr,
        Self::SlimeballScore,
        Self::JoinRewardsClub,
        Self::UseChromas,
        Self::CollectSlimeToys,
        Self::FindHobsonsEnd,
        Self::SnareHunterGordo,
        Self::ActivateOasis,
        Self::FinishAdventure,
        Self::CompleteSlimepedia,
    ];

    pub const fn as_key(self) -> &'static str {
        match self {
            Self::SellPlortsA => "SELL_PLORTS_A",
            Self::SellPlortsB => "SELL_PLORTS_B",
            Self::SellPlortsC => "SELL_PLORTS_C",
            Self::SellPlortsD => "SELL_PLORTS_D",
            Self::SellPlortsE => "SELL_PLORTS_E",
            Self::FeedSlimesChickens => "FEED_SLIMES_CHICKENS",
            Self::FruitTreeTypes => "FRUIT_TREE_TYPES",
            Self::VeggiePatchTypes => "VEGGIE_PATCH_TYPES",
            Self::EarnCurrencyA => "EARN_CURRENCY_A",
            Self::EarnCurrencyB => "EARN_CURRENCY_B",
            Self::EarnCurrencyC => "EARN_CURRENCY_C",
            Self::DayCurrency => "DAY_CURRENCY",
            Self::AwakeUntilMorning => "AWAKE_UNTIL_MORNING",
            Self::KnockoutMorning => "KNOCKOUT_MORNING",
            Self::AwayFromRanch => "AWAY_FROM_RANCH",
            Self::PinkSlimesFoodTypes => "PINK_SLIMES_FOOD_TYPES",
            Self::FeedAirborne => "FEED_AIRBORNE",
            Self::DiscoveredQuarry => "DISCOVERED_QUARRY",
            Self::DiscoveredMoss => "DISCOVERED_MOSS",
            Self::DiscoveredDesert => "DISCOVERED_DESERT",
            Self::BurstGordo => "BURST_GORDO",
            Self::OpenSlimeGate => "OPEN_SLIME_GATE",
            Self::IncinerateElderChicken => "INCINERATE_ELDER_CHICKEN",
            Self::FilledSilo => "FILLED_SILO",
            Self::RanchUpgradedStorage => "RANCH_UPGRADED_STORAGE",
            Self::FulfillExchangeEarly => "FULFILL_EXCHANGE_EARLY",
            Self::FeedFavorites => "FEED_FAVORITES",
            Self::DayCollectPlorts => "DAY_COLLECT_PLORTS",
            Self::GoldSlimeTriplePlort => "GOLD_SLIME_TRIPLE_PLORT",
            Self::ExtendedRadExposure => "EXTENDED_RAD_EXPOSURE",
            Self::ExtendedTarrHold => "EXTENDED_TARR_HOLD",
            Self::TabbyHeadbutt => "TABBY_HEADBUTT",
            Self::LaunchedBoomExplode => "LAUNCHED_BOOM_EXPLODE",
            Self::ManySlimesInVac => "MANY_SLIMES_IN_VAC",
            Self::CorralSlimeTypes => "CORRAL_SLIME_TYPES",
            Self::CorralLargoTypes => "CORRAL_LARGO_TYPES",
            Self::PondSlimeTypes => "POND_SLIME_TYPES",
            Self::RanchLargoTypes => "RANCH_LARGO_TYPES",
            Self::EnteredCorralSlimes => "ENTERED_CORRAL_SLIMES",
            Self::IncinerateChick => "INCINERATE_CHICK",
            Self::TimeLimitCurrencyA => "TIME_LIMIT_CURRENCY_A",
            Self::TimeLimitCurrencyB => "TIME_LIMIT_CURRENCY_B",
            Self::TimeLimitCurrencyC => "TIME_LIMIT_CURRENCY_C",
            Self::DiscoveredRuins => "DISCOVERED_RUINS",
            Self::FabricateGadgetsA => "FABRICATE_GADGETS_A",
            Self::FabricateGadgetsB => "FABRICATE_GADGETS_B",
            Self::FabricateGadgetsC => "FABRICATE_GADGETS_C",
            Self::SlimeStageTarr => "SLIME_STAGE_TARR",
            Self::SlimeballScore => "SLIMEBALL_SCORE",
            Self::JoinRewardsClub => "JOIN_REWARDS_CLUB",
            Self::UseChromas => "USE_CHROMAS",
            Self::CollectSlimeToys => "COLLECT_SLIME_TOYS",
            Self::FindHobsonsEnd => "FIND_HOBSONS_END",
            Self::SnareHunterGordo => "SNARE_HUNTER_GORDO",
            Self::ActivateOasis => "ACTIVATE_OASIS",
            Self::FinishAdventure => "FINISH_ADVENTURE",
            Self::CompleteSlimepedia => "COMPLETE_SLIMEPEDIA",
        }
    }

    pub const fn tier(self) -> AchievementTier {
        match self {
            Self::SellPlortsA
            | Self::SellPlortsB
            | Self::FeedSlimesChickens
            | Self::FruitTreeTypes
            | Self::VeggiePatchTypes
            | Self::EarnCurrencyA
            | Self::AwakeUntilMorning
            | Self::KnockoutMorning
            | Self::AwayFromRanch
            | Self::FeedAirborne
            | Self::PinkSlimesFoodTypes
            | Self::TabbyHeadbutt
            | Self::IncinerateElderChicken
            | Self::FeedFavorites
            | Self::FilledSilo
            | Self::IncinerateChick
            | Self::TimeLimitCurrencyA
            | Self::FabricateGadgetsA
            | Self::SlimeStageTarr
            | Self::JoinRewardsClub
            | Self::UseChromas => AchievementTier::Tier1,

            Self::SellPlortsC
            | Self::SellPlortsD
            | Self::EarnCurrencyB
            | Self::LaunchedBoomExplode
            | Self::ManySlimesInVac
            | Self::DiscoveredQuarry
            | Self::DiscoveredMoss
            | Self::DiscoveredRuins
            | Self::OpenSlimeGate
            | Self::BurstGordo
            | Self::ExtendedRadExposure
            | Self::DayCollectPlorts
            | Self::FulfillExchangeEarly
            | Self::RanchUpgradedStorage
            | Self::EnteredCorralSlimes
            | Self::PondSlimeTypes
            | Self::CorralSlimeTypes
            | Self::CorralLargoTypes
            | Self::TimeLimitCurrencyB
            | Self::FabricateGadgetsB
            | Self::CollectSlimeToys
            | Self::SnareHunterGordo
            | Self::ActivateOasis => AchievementTier::Tier2,

            Self::SellPlortsE
            | Self::EarnCurrencyC
            | Self::DayCurrency
            | Self::DiscoveredDesert
            | Self::ExtendedTarrHold
            | Self::GoldSlimeTriplePlort
            | Self::RanchLargoTypes
            | Self::TimeLimitCurrencyC
            | Self::FabricateGadgetsC
            | Self::SlimeballScore
            | Self::FindHobsonsEnd
            | Self::FinishAdventure
            | Self::CompleteSlimepedia => AchievementTier::Tier3,
        }
    }

    pub fn is_allowed_in_mode(self, mode: GameMode) -> bool {
        if mode == GameMode::TimeLimitV2 {
            matches!(
                self,
                Self::TimeLimitCurrencyA
                    | Self::TimeLimitCurrencyB
                    | Self::TimeLimitCurrencyC
            )
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndGamePlortEntry {
    pub id: IdentifiableId,
    pub count: i32,
    pub total_value: i32,
}

/// Port of `EndGameUI` + `EndGameV2UI` data summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGameSummary {
    pub currency: i32,
    pub deaths: i32,
    pub currency_spent: i32,
    pub plort_lines: Vec<EndGamePlortEntry>,
    pub rush_bonus_plorts: Vec<IdentifiableId>,
    pub rush_score_multiplier: f32,
    pub rush_final_score: i32,
}

/// Port of `AchievementsDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementsDirector;

impl AchievementsDirector {
    pub fn add_int_stat(model: &mut GameModel, stat: &str, amount: i32) {
        if model.game_mode == GameMode::TimeLimitV2 && stat != "TIME_LIMIT_V2_CURRENCY" {
            return;
        }
        *model
            .game_achieves
            .counts
            .entry(stat.to_string())
            .or_insert(0) += amount;
    }

    pub fn maybe_update_max_stat(model: &mut GameModel, stat: &str, val: i32) {
        if model.game_mode == GameMode::TimeLimitV2 && stat != "TIME_LIMIT_V2_CURRENCY" {
            return;
        }
        let entry = model
            .game_achieves
            .counts
            .entry(stat.to_string())
            .or_insert(0);
        if val > *entry {
            *entry = val;
        }
    }

    pub fn add_enum_stat(model: &mut GameModel, stat: &str, val: i32) {
        if model.game_mode == GameMode::TimeLimitV2 {
            return;
        }
        model
            .game_achieves
            .enums
            .entry(stat.to_string())
            .or_default()
            .insert(val);
    }

    pub fn add_game_int_stat(model: &mut GameModel, stat: &str, amount: i32) {
        *model
            .game_achieves
            .game_int_stats
            .entry(stat.to_string())
            .or_insert(0) += amount;
    }

    pub fn set_game_double_stat(model: &mut GameModel, stat: &str, val: f64) {
        model
            .game_achieves
            .doubles
            .insert(stat.to_string(), val);
    }

    pub fn add_game_plort_sold(model: &mut GameModel, id: IdentifiableId, amt: i32) {
        let dict = model
            .game_achieves
            .game_id_dicts
            .entry("PLORT_TYPES_SOLD".to_string())
            .or_default();
        *dict.entry(id).or_insert(0) += amt;
    }

    pub fn has_achievement(model: &GameModel, ach: AchievementId) -> bool {
        model.game_achieves.earned_achievements.contains(ach.as_key())
    }

    pub fn calculate_game_progress(model: &GameModel) -> f32 {
        (model.game_achieves.earned_achievements.len() as f32) / (AchievementId::ALL.len() as f32)
    }

    fn int_stat(model: &GameModel, k: &str) -> i32 {
        model.game_achieves.counts.get(k).copied().unwrap_or(0)
    }

    fn enum_count(model: &GameModel, k: &str) -> usize {
        model
            .game_achieves
            .enums
            .get(k)
            .map_or(0, |set| set.len())
    }

    fn game_int_stat(model: &GameModel, k: &str) -> i32 {
        model
            .game_achieves
            .game_int_stats
            .get(k)
            .copied()
            .unwrap_or(0)
    }

    fn ranch_resource_count(model: &GameModel, fruit: bool) -> usize {
        let mut set = HashSet::new();
        for p in model.land_plots.values() {
            let id = p.attached_resource_id;
            if id != IdentifiableId::None && ((fruit && id.is_fruit()) || (!fruit && id.is_veggie()))
            {
                set.insert(id);
            }
        }
        set.len()
    }

    /// Evaluates all 57 achievement conditions and awards newly earned ones.
    pub fn evaluate_all(model: &mut GameModel) -> Vec<AchievementId> {
        let mut newly = Vec::new();
        let mode = model.game_mode;

        for &ach in AchievementId::ALL {
            if !ach.is_allowed_in_mode(mode) || Self::has_achievement(model, ach) {
                continue;
            }
            let reached = match ach {
                AchievementId::SellPlortsA => Self::int_stat(model, "PLORTS_SOLD") >= 100,
                AchievementId::SellPlortsB => Self::int_stat(model, "PLORTS_SOLD") >= 500,
                AchievementId::SellPlortsC => Self::int_stat(model, "PLORTS_SOLD") >= 1000,
                AchievementId::SellPlortsD => Self::int_stat(model, "PLORTS_SOLD") >= 2500,
                AchievementId::SellPlortsE => Self::int_stat(model, "PLORTS_SOLD") >= 5000,
                AchievementId::DayCurrency => Self::int_stat(model, "DAY_CURRENCY") >= 5000,
                AchievementId::EarnCurrencyA => Self::int_stat(model, "CURRENCY") >= 5000,
                AchievementId::EarnCurrencyB => Self::int_stat(model, "CURRENCY") >= 25000,
                AchievementId::EarnCurrencyC => Self::int_stat(model, "CURRENCY") >= 100000,
                AchievementId::FeedSlimesChickens => {
                    Self::int_stat(model, "CHICKENS_FED_SLIMES") >= 100
                }
                AchievementId::PinkSlimesFoodTypes => {
                    Self::enum_count(model, "PINK_SLIMES_FOOD_TYPES") >= 10
                }
                AchievementId::FeedAirborne => Self::int_stat(model, "FED_AIRBORNE") >= 1,
                AchievementId::FeedFavorites => Self::int_stat(model, "FED_FAVORITE") >= 50,
                AchievementId::AwayFromRanch => {
                    let left = model.game_achieves.doubles.get("LAST_LEFT_RANCH").copied();
                    let entered = model
                        .game_achieves
                        .doubles
                        .get("LAST_ENTERED_RANCH")
                        .copied();
                    matches!((left, entered), (Some(l), Some(e)) if e - l >= 86400.0)
                }
                AchievementId::AwakeUntilMorning => {
                    let slept = model.game_achieves.doubles.get("LAST_SLEPT").copied();
                    let awoke = model.game_achieves.doubles.get("LAST_AWOKE").copied();
                    matches!((slept, awoke), (Some(s), Some(a)) if a > s && (model.world.world_time - a) >= 86400.0)
                }
                AchievementId::KnockoutMorning => Self::int_stat(model, "DEATH_BEFORE_10AM") >= 1,
                AchievementId::FruitTreeTypes => Self::ranch_resource_count(model, true) >= 3,
                AchievementId::VeggiePatchTypes => Self::ranch_resource_count(model, false) >= 3,
                AchievementId::DiscoveredQuarry => Self::int_stat(model, "VISITED_QUARRY") >= 1,
                AchievementId::DiscoveredMoss => Self::int_stat(model, "VISITED_MOSS") >= 1,
                AchievementId::DiscoveredDesert => Self::int_stat(model, "VISITED_DESERT") >= 1,
                AchievementId::DiscoveredRuins => Self::int_stat(model, "VISITED_RUINS") >= 1,
                AchievementId::BurstGordo => Self::int_stat(model, "BURST_GORDOS") >= 1,
                AchievementId::OpenSlimeGate => Self::int_stat(model, "OPENED_SLIME_GATES") >= 1,
                AchievementId::IncinerateElderChicken => {
                    Self::int_stat(model, "INCINERATED_ELDER_CHICKENS") >= 1
                }
                AchievementId::IncinerateChick => Self::int_stat(model, "INCINERATED_CHICKS") >= 1,
                AchievementId::FilledSilo => Self::int_stat(model, "FILLED_SILO") >= 1,
                AchievementId::RanchUpgradedStorage => {
                    RanchModel::includes_fully_upgraded_corral_coop_and_silo(&model.land_plots)
                }
                AchievementId::FulfillExchangeEarly => {
                    Self::int_stat(model, "FULFILL_EXCHANGE_EARLY") >= 1
                }
                AchievementId::DayCollectPlorts => {
                    Self::int_stat(model, "DAY_COLLECT_PLORTS") >= 50
                }
                AchievementId::GoldSlimeTriplePlort => {
                    Self::int_stat(model, "GOLD_SLIME_TRIPLE_PLORT") >= 1
                }
                AchievementId::ExtendedRadExposure => {
                    Self::int_stat(model, "EXTENDED_RAD_EXPOSURE") >= 15
                }
                AchievementId::ExtendedTarrHold => {
                    Self::int_stat(model, "EXTENDED_TARR_HOLD") >= 15
                }
                AchievementId::TabbyHeadbutt => Self::int_stat(model, "TABBY_HEADBUTT") >= 1,
                AchievementId::LaunchedBoomExplode => {
                    Self::int_stat(model, "LAUNCHED_BOOM_EXPLODE") >= 1
                }
                AchievementId::ManySlimesInVac => Self::int_stat(model, "SLIMES_IN_VAC") >= 15,
                AchievementId::CorralSlimeTypes => {
                    Self::int_stat(model, "CORRAL_SLIME_TYPES") >= 6
                }
                AchievementId::CorralLargoTypes => {
                    Self::int_stat(model, "CORRAL_LARGO_TYPES") >= 3
                }
                AchievementId::PondSlimeTypes => Self::int_stat(model, "POND_SLIME_TYPES") >= 5,
                AchievementId::RanchLargoTypes => Self::int_stat(model, "RANCH_LARGO_TYPES") >= 10,
                AchievementId::EnteredCorralSlimes => {
                    Self::int_stat(model, "ENTERED_CORRAL_SLIMES") >= 40
                }
                AchievementId::TimeLimitCurrencyA => {
                    Self::int_stat(model, "TIME_LIMIT_V2_CURRENCY") >= 10000
                }
                AchievementId::TimeLimitCurrencyB => {
                    Self::int_stat(model, "TIME_LIMIT_V2_CURRENCY") >= 35000
                }
                AchievementId::TimeLimitCurrencyC => {
                    Self::int_stat(model, "TIME_LIMIT_V2_CURRENCY") >= 75000
                }
                AchievementId::FabricateGadgetsA => {
                    Self::game_int_stat(model, "FABRICATED_GADGETS") >= 1
                }
                AchievementId::FabricateGadgetsB => {
                    Self::game_int_stat(model, "FABRICATED_GADGETS") >= 35
                }
                AchievementId::FabricateGadgetsC => {
                    Self::game_int_stat(model, "FABRICATED_GADGETS") >= 100
                }
                AchievementId::SlimeballScore => Self::int_stat(model, "SLIMEBALL_SCORE") >= 50,
                AchievementId::SlimeStageTarr => Self::int_stat(model, "SLIME_STAGE_TARRS") >= 1,
                AchievementId::JoinRewardsClub => Self::int_stat(model, "REWARD_LEVELS") >= 1,
                AchievementId::UseChromas => Self::enum_count(model, "USE_CHROMAS") >= 3,
                AchievementId::CollectSlimeToys => {
                    Self::enum_count(model, "SLIME_TOYS_BOUGHT") >= 10
                }
                AchievementId::SnareHunterGordo => {
                    Self::int_stat(model, "SNARED_HUNTER_GORDOS") >= 1
                }
                AchievementId::ActivateOasis => Self::int_stat(model, "ACTIVATED_OASES") >= 1,
                AchievementId::CompleteSlimepedia => {
                    Self::int_stat(model, "COMPLETED_SLIMEPEDIA") >= 1
                }
                AchievementId::FindHobsonsEnd => Self::int_stat(model, "FIND_HOBSONS_END") >= 1,
                AchievementId::FinishAdventure => Self::int_stat(model, "FINISH_ADVENTURE") >= 1,
            };

            if reached {
                model
                    .game_achieves
                    .earned_achievements
                    .insert(ach.as_key().to_string());
                newly.push(ach);
            }
        }

        newly
    }

    /// Port of `EndGameUI` & `EndGameV2UI` summary + Rush score computation.
    pub fn build_end_game_summary(model: &mut GameModel, econ: &EconomyDirector) -> EndGameSummary {
        let mut counts_by_plort: HashMap<IdentifiableId, i32> = HashMap::new();
        if let Some(dict) = model.game_achieves.game_id_dicts.get("PLORT_TYPES_SOLD") {
            for (&id, &cnt) in dict {
                *counts_by_plort.entry(id).or_insert(0) += cnt;
            }
        }
        for &id in IdentifiableId::ALL {
            if id.is_plort() {
                let c = get_plort_sold_count(model, id);
                if c > 0 {
                    counts_by_plort.entry(id).or_insert(c);
                }
            }
        }

        let mut plort_lines: Vec<EndGamePlortEntry> = counts_by_plort
            .iter()
            .filter(|(_, &c)| c > 0)
            .map(|(&id, &count)| {
                let val = econ.get_curr_value(id).unwrap_or(0);
                EndGamePlortEntry {
                    id,
                    count,
                    total_value: count * val,
                }
            })
            .collect();
        plort_lines.sort_by(|a, b| b.total_value.cmp(&a.total_value));

        let mut rush_bonus_plorts = Vec::new();
        let mut rush_score_multiplier = 1.0f32;
        for (&id, &cnt) in &counts_by_plort {
            if GameModeSettings::plort_bonus_reached(cnt) {
                rush_bonus_plorts.push(id);
                rush_score_multiplier += GameModeSettings::score_multiplier(id);
            }
        }
        rush_bonus_plorts.sort_by_key(|x| *x as i32);

        let currency = model.player.currency;
        let rush_final_score = ((currency as f32) * rush_score_multiplier).ceil() as i32;
        if model.game_mode == GameMode::TimeLimitV2 {
            Self::maybe_update_max_stat(model, "TIME_LIMIT_V2_CURRENCY", rush_final_score);
        }

        EndGameSummary {
            currency,
            deaths: Self::game_int_stat(model, "DEATHS"),
            currency_spent: Self::game_int_stat(model, "CURRENCY_SPENT"),
            plort_lines,
            rush_bonus_plorts,
            rush_score_multiplier,
            rush_final_score,
        }
    }
}
