//! Port of `ExchangeDirector.cs`, `OfferGenerator.cs`, `ExchangeAcceptor.cs`,
//! and `ExchangeEjector.cs` (Range Exchange & Special Rancher Progress Quests).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GameMode, IdentifiableId, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{
    ExchangeItemEntry, ExchangeOffer, ExchangeRequestedItemEntry, GameModel, MailType,
    NonIdentReward, OfferType, WorldModel,
};
use sr_physics::PhysicsWorld;

use crate::actors::spawn_physical_actor;
use crate::player::PlayerController;

pub const DAILY_OFFER_HOUR: f32 = 12.083333; // 12:05 PM
pub const EARLY_EXCHANGE_HOURS: f64 = 2.0;
pub const MIN_OFFER_VAL: i32 = 50;
pub const MAX_OFFER_VAL: i32 = 100;
pub const BONUS_PROB: f32 = 0.125;
pub const RARE_PROB: f32 = 0.5;
pub const REWARD_VAL_MULT: f32 = 1.75;
pub const REWARD_BONUS_MULT: f32 = 2.5;

pub const OGDEN_RECUR_AMOUNT: i32 = 20;
pub const MOCHI_RECUR_AMOUNT: i32 = 50;
pub const VIKTOR_RECUR_AMOUNT: i32 = 25;

pub const RANCHER_IDS: [&str; 5] = ["thora", "viktor", "ogden", "mochi", "bob"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RancherProfile {
    pub name: String,
    pub num_blurbs: i32,
    pub requests: Vec<IdentifiableId>,
    pub rewards: Vec<IdentifiableId>,
    pub rare_rewards: Vec<IdentifiableId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressRewardLevel {
    pub requested_item: IdentifiableId,
    pub count: i32,
    pub reward: NonIdentReward,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExchangeStepEvents {
    pub expired_general: bool,
    pub created_pending_ranchers: Vec<String>,
    pub added_recur_offers: Vec<OfferType>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcceptOutcome {
    pub accepted: bool,
    pub completed: bool,
    pub early_bonus: bool,
    pub spawned_rewards: Vec<i64>,
    pub newbucks_awarded: i32,
    pub time_extension_secs: f64,
    pub unlocked_progress: Vec<ProgressType>,
}

/// Port of `ExchangeDirector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeDirector {
    pub item_values: HashMap<IdentifiableId, f32>,
    pub init_unlocked: Vec<IdentifiableId>,
    pub unlock_lists: Vec<(ProgressType, Vec<IdentifiableId>)>,
    pub ranchers: Vec<RancherProfile>,
    pub ogden_levels: Vec<ProgressRewardLevel>,
    pub mochi_levels: Vec<ProgressRewardLevel>,
    pub viktor_levels: Vec<ProgressRewardLevel>,
}

impl Default for ExchangeDirector {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ExchangeDirector {
    pub fn builtin() -> Self {
        use IdentifiableId as Id;
        let mut item_values = HashMap::new();
        for (id, v) in [
            (Id::PogoFruit, 10.0),
            (Id::CuberryFruit, 15.0),
            (Id::MangoFruit, 20.0),
            (Id::LemonFruit, 25.0),
            (Id::PearFruit, 30.0),
            (Id::CarrotVeggie, 10.0),
            (Id::BeetVeggie, 15.0),
            (Id::OcaocaVeggie, 20.0),
            (Id::OnionVeggie, 25.0),
            (Id::ParsnipVeggie, 30.0),
            (Id::Hen, 15.0),
            (Id::StonyHen, 20.0),
            (Id::BriarHen, 25.0),
            (Id::PaintedHen, 30.0),
            (Id::Rooster, 20.0),
            (Id::PinkPlort, 7.0),
            (Id::RockPlort, 15.0),
            (Id::TabbyPlort, 15.0),
            (Id::PhosphorPlort, 15.0),
            (Id::RadPlort, 30.0),
            (Id::HoneyPlort, 30.0),
            (Id::BoomPlort, 30.0),
            (Id::CrystalPlort, 40.0),
            (Id::HunterPlort, 45.0),
            (Id::QuantumPlort, 45.0),
            (Id::DervishPlort, 50.0),
            (Id::MosaicPlort, 55.0),
            (Id::TanglePlort, 55.0),
            (Id::PinkSlime, 10.0),
            (Id::RockSlime, 15.0),
            (Id::TabbySlime, 15.0),
            (Id::PhosphorSlime, 15.0),
            (Id::PrimordyOilCraft, 25.0),
            (Id::DeepBrineCraft, 25.0),
            (Id::SpiralSteamCraft, 35.0),
            (Id::LavaDustCraft, 40.0),
            (Id::BuzzWaxCraft, 25.0),
            (Id::WildHoneyCraft, 40.0),
            (Id::HexacombCraft, 45.0),
            (Id::JellystoneCraft, 25.0),
            (Id::IndigoniumCraft, 40.0),
            (Id::SilkySandCraft, 35.0),
            (Id::StrangeDiamondCraft, 90.0),
            (Id::GoldPlort, 100.0),
        ] {
            item_values.insert(id, v);
        }

        let init_unlocked = vec![
            Id::PogoFruit,
            Id::CuberryFruit,
            Id::CarrotVeggie,
            Id::BeetVeggie,
            Id::Hen,
            Id::StonyHen,
            Id::Rooster,
            Id::PinkPlort,
            Id::RockPlort,
            Id::TabbyPlort,
            Id::PhosphorPlort,
            Id::PinkSlime,
            Id::RockSlime,
            Id::TabbySlime,
            Id::PhosphorSlime,
            Id::PrimordyOilCraft,
            Id::DeepBrineCraft,
            Id::JellystoneCraft,
        ];

        let unlock_lists = vec![
            (
                ProgressType::UnlockQuarry,
                vec![Id::OcaocaVeggie, Id::OnionVeggie, Id::RadPlort, Id::CrystalPlort, Id::IndigoniumCraft],
            ),
            (
                ProgressType::UnlockMoss,
                vec![Id::MangoFruit, Id::BriarHen, Id::HoneyPlort, Id::BoomPlort, Id::HunterPlort, Id::WildHoneyCraft],
            ),
            (
                ProgressType::UnlockRuins,
                vec![Id::LemonFruit, Id::QuantumPlort, Id::StrangeDiamondCraft],
            ),
            (
                ProgressType::UnlockDesert,
                vec![Id::PearFruit, Id::ParsnipVeggie, Id::PaintedHen, Id::DervishPlort, Id::MosaicPlort, Id::TanglePlort, Id::LavaDustCraft, Id::SilkySandCraft],
            ),
        ];

        let common_foods = vec![
            Id::PogoFruit, Id::CuberryFruit, Id::MangoFruit, Id::CarrotVeggie,
            Id::BeetVeggie, Id::OcaocaVeggie, Id::Hen, Id::StonyHen, Id::BriarHen,
        ];
        let common_plorts = vec![
            Id::PinkPlort, Id::RockPlort, Id::TabbyPlort, Id::PhosphorPlort,
            Id::RadPlort, Id::HoneyPlort, Id::BoomPlort, Id::CrystalPlort,
        ];
        let craft_mats = vec![
            Id::PrimordyOilCraft, Id::DeepBrineCraft, Id::JellystoneCraft,
            Id::IndigoniumCraft, Id::WildHoneyCraft, Id::LavaDustCraft,
        ];

        let ranchers = vec![
            RancherProfile {
                name: "thora".into(),
                num_blurbs: 3,
                requests: vec![Id::PogoFruit, Id::CarrotVeggie, Id::PinkPlort, Id::RockPlort, Id::TabbyPlort, Id::RadPlort, Id::HoneyPlort],
                rewards: common_foods.clone(),
                rare_rewards: vec![Id::StrangeDiamondCraft],
            },
            RancherProfile {
                name: "viktor".into(),
                num_blurbs: 3,
                requests: common_plorts.clone(),
                rewards: craft_mats.clone(),
                rare_rewards: vec![Id::StrangeDiamondCraft],
            },
            RancherProfile {
                name: "ogden".into(),
                num_blurbs: 3,
                requests: vec![Id::PogoFruit, Id::CuberryFruit, Id::MangoFruit, Id::CarrotVeggie, Id::BeetVeggie, Id::OcaocaVeggie, Id::LemonFruit],
                rewards: common_foods.clone(),
                rare_rewards: vec![Id::PearFruit, Id::ParsnipVeggie],
            },
            RancherProfile {
                name: "mochi".into(),
                num_blurbs: 3,
                requests: common_plorts,
                rewards: vec![Id::PinkPlort, Id::RockPlort, Id::HoneyPlort, Id::CrystalPlort, Id::Rooster],
                rare_rewards: vec![Id::PaintedHen],
            },
            RancherProfile {
                name: "bob".into(),
                num_blurbs: 3,
                requests: vec![Id::Hen, Id::StonyHen, Id::BriarHen, Id::PaintedHen, Id::Rooster],
                rewards: common_foods,
                rare_rewards: vec![Id::Rooster],
            },
        ];

        let ogden_levels = vec![
            ProgressRewardLevel { requested_item: Id::KookadobaFruit, count: 40, reward: NonIdentReward::OgdenMix },
            ProgressRewardLevel { requested_item: Id::KookadobaFruit, count: 80, reward: NonIdentReward::OgdenGarden },
            ProgressRewardLevel { requested_item: Id::KookadobaFruit, count: 160, reward: NonIdentReward::OgdenRanch },
        ];
        let mochi_levels = vec![
            ProgressRewardLevel { requested_item: Id::QuicksilverPlort, count: 50, reward: NonIdentReward::MochiExtraMile },
            ProgressRewardLevel { requested_item: Id::QuicksilverPlort, count: 100, reward: NonIdentReward::MochiCoop },
            ProgressRewardLevel { requested_item: Id::QuicksilverPlort, count: 200, reward: NonIdentReward::MochiRanch },
        ];
        let viktor_levels = vec![
            ProgressRewardLevel { requested_item: Id::GlitchBugReport, count: 35, reward: NonIdentReward::ViktorChickenCloner },
            ProgressRewardLevel { requested_item: Id::GlitchBugReport, count: 70, reward: NonIdentReward::ViktorDeluxeDrones },
            ProgressRewardLevel { requested_item: Id::GlitchBugReport, count: 140, reward: NonIdentReward::ViktorRanch },
        ];

        Self {
            item_values,
            init_unlocked,
            unlock_lists,
            ranchers,
            ogden_levels,
            mochi_levels,
            viktor_levels,
        }
    }

    pub fn rancher_progress_type(rancher_id: &str) -> Option<ProgressType> {
        match rancher_id {
            "thora" => Some(ProgressType::ExchangeThora),
            "viktor" => Some(ProgressType::ExchangeViktor),
            "ogden" => Some(ProgressType::ExchangeOgden),
            "mochi" => Some(ProgressType::ExchangeMochi),
            "bob" => Some(ProgressType::ExchangeBob),
            _ => None,
        }
    }

    /// Port of `ExchangeDirector.InitModel`.
    pub fn init_model(world: &mut WorldModel, mode: &GameModeSettings) {
        world.curr_offers.clear();
        world.last_offer_rancher_ids.clear();
        world.pending_offer_rancher_ids.clear();
        let full_days = (mode.exchange_start_day - 1).max(0) as f64;
        world.next_daily_offer_create_time =
            full_days * 86400.0 + (DAILY_OFFER_HOUR as f64) * 3600.0;
    }

    pub fn get_count_for_value(&self, id: IdentifiableId, value: i32) -> i32 {
        self.item_values
            .get(&id)
            .map(|&v| ((value as f32) / v).round() as i32)
            .unwrap_or(0)
    }

    fn next_daily_offer_create_time(game_mode: GameMode, curr_day: i32, world_time: f64) -> f64 {
        let target_secs = (DAILY_OFFER_HOUR as f64) * 3600.0;
        if game_mode == GameMode::TimeLimitV2 && curr_day == 1 {
            return 86400.0 + target_secs;
        }
        let today_base = (world_time / 86400.0).floor() * 86400.0;
        let candidate = today_base + target_secs;
        if candidate > world_time {
            candidate
        } else {
            today_base + 86400.0 + target_secs
        }
    }

    fn next_noon_at_least_half_day(world_time: f64) -> f64 {
        let mut t = (world_time / 86400.0).floor() * 86400.0 + 12.0 * 3600.0;
        while t < world_time + 43200.0 {
            t += 86400.0;
        }
        t
    }

    /// Port of `ExchangeDirector.Update`.
    pub fn step(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        curr_day: i32,
        rng: &mut Randoms,
    ) -> ExchangeStepEvents {
        let mut ev = ExchangeStepEvents::default();
        let wt = model.world.world_time;

        // 1. Expire general offer
        if let Some(offer) = model.world.curr_offers.get(&OfferType::General) {
            if wt >= offer.expire_time {
                model.world.curr_offers.remove(&OfferType::General);
                ev.expired_general = true;
            }
        }

        // 2. Prepare daily pending ranchers at 12:05
        if !model.world.curr_offers.contains_key(&OfferType::General)
            && wt >= model.world.next_daily_offer_create_time
        {
            model.world.next_daily_offer_create_time =
                Self::next_daily_offer_create_time(model.game_mode, curr_day, wt);
            self.setup_pending_offer_ranchers(model, rng);
            model.world.last_offer_rancher_ids = model.world.pending_offer_rancher_ids.clone();
            ev.created_pending_ranchers = model.world.pending_offer_rancher_ids.clone();
        }

        // 3. Special recurring DLC rancher offers
        if !model.world.curr_offers.contains_key(&OfferType::OgdenRecur)
            && (model.world.curr_offers.contains_key(&OfferType::Ogden)
                || model.progress.get_progress(ProgressType::OgdenRewards) >= 3)
        {
            model.world.curr_offers.insert(
                OfferType::OgdenRecur,
                ExchangeOffer {
                    offer_id: "m.offer.ogden_recur".into(),
                    rancher_id: "ogden".into(),
                    expire_time: f64::INFINITY,
                    early_exchange_time: f64::NEG_INFINITY,
                    requests: vec![ExchangeRequestedItemEntry {
                        id: IdentifiableId::KookadobaFruit,
                        count: OGDEN_RECUR_AMOUNT,
                        progress: 0,
                        spec_reward: NonIdentReward::None,
                    }],
                    rewards: vec![ExchangeItemEntry {
                        id: IdentifiableId::SpicyTofu,
                        count: 1,
                        spec_reward: NonIdentReward::None,
                    }],
                },
            );
            ev.added_recur_offers.push(OfferType::OgdenRecur);
        }

        if !model.world.curr_offers.contains_key(&OfferType::MochiRecur)
            && (model.world.curr_offers.contains_key(&OfferType::Mochi)
                || model.progress.get_progress(ProgressType::MochiRewards) >= 3)
        {
            model.world.curr_offers.insert(
                OfferType::MochiRecur,
                ExchangeOffer {
                    offer_id: "m.offer.mochi_recur".into(),
                    rancher_id: "mochi".into(),
                    expire_time: f64::INFINITY,
                    early_exchange_time: f64::NEG_INFINITY,
                    requests: vec![ExchangeRequestedItemEntry {
                        id: IdentifiableId::QuicksilverPlort,
                        count: MOCHI_RECUR_AMOUNT,
                        progress: 0,
                        spec_reward: NonIdentReward::None,
                    }],
                    rewards: vec![ExchangeItemEntry {
                        id: IdentifiableId::None,
                        count: 1,
                        spec_reward: NonIdentReward::NewbucksMochi,
                    }],
                },
            );
            ev.added_recur_offers.push(OfferType::MochiRecur);
        }

        if !model.world.curr_offers.contains_key(&OfferType::ViktorRecur)
            && (model.world.curr_offers.contains_key(&OfferType::Viktor)
                || model.progress.get_progress(ProgressType::ViktorRewards) >= 3)
        {
            model.world.curr_offers.insert(
                OfferType::ViktorRecur,
                ExchangeOffer {
                    offer_id: "m.offer.viktor_recur".into(),
                    rancher_id: "viktor".into(),
                    expire_time: f64::INFINITY,
                    early_exchange_time: f64::NEG_INFINITY,
                    requests: vec![ExchangeRequestedItemEntry {
                        id: IdentifiableId::GlitchBugReport,
                        count: VIKTOR_RECUR_AMOUNT,
                        progress: 0,
                        spec_reward: NonIdentReward::None,
                    }],
                    rewards: vec![ExchangeItemEntry {
                        id: IdentifiableId::ManifoldCubeCraft,
                        count: 1,
                        spec_reward: NonIdentReward::None,
                    }],
                },
            );
            ev.added_recur_offers.push(OfferType::ViktorRecur);
        }

        let _ = mode;
        ev
    }

    fn setup_pending_offer_ranchers(&self, model: &mut GameModel, rng: &mut Randoms) {
        model.world.pending_offer_rancher_ids.clear();
        let mut available: Vec<String> = Vec::new();
        for r in &self.ranchers {
            let Some(ptype) = Self::rancher_progress_type(&r.name) else {
                continue;
            };
            if model.progress.get_progress(ptype) <= 0 {
                model.world.pending_offer_rancher_ids.push(r.name.clone());
                model
                    .mail
                    .send_mail(MailType::Exchange, format!("exchangeintro_{}", r.name));
                return;
            }
            if !model.world.last_offer_rancher_ids.contains(&r.name) {
                available.push(r.name.clone());
            }
        }
        if available.len() < 2 {
            return;
        }
        let i1 = rng.get_in_range_i32(0, available.len() as i32) as usize;
        let r1 = available.swap_remove(i1);
        let i2 = rng.get_in_range_i32(0, available.len() as i32) as usize;
        let r2 = available.swap_remove(i2);
        model.world.pending_offer_rancher_ids.push(r1);
        model.world.pending_offer_rancher_ids.push(r2);
    }

    fn create_whitelist(&self, model: &GameModel, curr_day: i32) -> HashSet<IdentifiableId> {
        let mut set: HashSet<IdentifiableId> = self.init_unlocked.iter().copied().collect();
        if model.game_mode == GameMode::TimeLimitV2 {
            if curr_day >= 2 {
                for (p, ids) in &self.unlock_lists {
                    if *p == ProgressType::UnlockQuarry || *p == ProgressType::UnlockMoss {
                        set.extend(ids.iter().copied());
                    }
                }
            }
            if curr_day >= 3 {
                for (p, ids) in &self.unlock_lists {
                    if *p == ProgressType::UnlockDesert || *p == ProgressType::UnlockRuins {
                        set.extend(ids.iter().copied());
                    }
                }
            }
        } else {
            for (p, ids) in &self.unlock_lists {
                if model.progress.get_progress(*p) > 0 {
                    set.extend(ids.iter().copied());
                }
            }
        }
        set
    }

    fn generate_item_list(
        &self,
        pool: &[IdentifiableId],
        whitelist: &HashSet<IdentifiableId>,
        disallowed: &mut Vec<IdentifiableId>,
        total_value: i32,
        item_count: usize,
        rng: &mut Randoms,
    ) -> Option<Vec<(IdentifiableId, i32)>> {
        let mut weights = Vec::with_capacity(item_count);
        let mut sum = 0.0f32;
        for _ in 0..item_count {
            let w = rng.get_in_range_f32(0.5, 1.5);
            sum += w;
            weights.push(w);
        }

        let mut out = Vec::with_capacity(item_count);
        for w in weights {
            let val = ((total_value as f32) * w / sum).round() as i32;
            let candidates: Vec<IdentifiableId> = pool
                .iter()
                .copied()
                .filter(|id| whitelist.contains(id) && !disallowed.contains(id))
                .collect();
            if candidates.is_empty() {
                return None;
            }
            let pick = candidates[rng.get_in_range_i32(0, candidates.len() as i32) as usize];
            let count = self.get_count_for_value(pick, val);
            if count <= 0 {
                return None;
            }
            disallowed.push(pick);
            out.push((pick, count));
        }
        Some(out)
    }

    /// Port of `ExchangeDirector.SelectDailyOffer`.
    pub fn select_daily_offer(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        curr_day: i32,
        rancher_id: &str,
        is_first_offer: bool,
        rng: &mut Randoms,
    ) -> Option<ExchangeOffer> {
        if model.world.curr_offers.contains_key(&OfferType::General) {
            return None;
        }
        let rancher = self.ranchers.iter().find(|r| r.name == rancher_id)?;
        let whitelist = self.create_whitelist(model, curr_day);
        let wt = model.world.world_time;
        let expire_time = Self::next_noon_at_least_half_day(wt);
        let early_exchange_time = wt + EARLY_EXCHANGE_HOURS * 3600.0;

        let mut offer = None;
        for _ in 0..10 {
            let mut used = Vec::new();
            let in_range_val = rng.get_in_range_i32(MIN_OFFER_VAL, MAX_OFFER_VAL);
            let req_count = rng.get_in_range_i32(2, 4) as usize;
            let Some(req_pairs) = self.generate_item_list(
                &rancher.requests,
                &whitelist,
                &mut used,
                in_range_val,
                req_count,
                rng,
            ) else {
                continue;
            };
            let requests: Vec<ExchangeRequestedItemEntry> = req_pairs
                .into_iter()
                .map(|(id, count)| ExchangeRequestedItemEntry {
                    id,
                    count,
                    progress: 0,
                    spec_reward: NonIdentReward::None,
                })
                .collect();

            let mut rewards = Vec::new();
            let is_bonus;
            if model.game_mode == GameMode::TimeLimitV2 {
                is_bonus = false;
                rewards.push(ExchangeItemEntry {
                    id: IdentifiableId::None,
                    count: 1,
                    spec_reward: NonIdentReward::TimeExtension12h,
                });
                rewards.push(ExchangeItemEntry {
                    id: IdentifiableId::GingerVeggie,
                    count: 6,
                    spec_reward: NonIdentReward::None,
                });
            } else if mode.exchange_rewards_gold_plorts {
                is_bonus = false;
                rewards.push(ExchangeItemEntry {
                    id: IdentifiableId::GoldPlort,
                    count: 3,
                    spec_reward: NonIdentReward::None,
                });
            } else {
                is_bonus = !is_first_offer && rng.get_probability(BONUS_PROB);
                let use_rare = is_bonus
                    && rancher.rare_rewards.iter().any(|id| whitelist.contains(id))
                    && rng.get_probability(RARE_PROB);
                let mult = if is_bonus {
                    REWARD_BONUS_MULT
                } else {
                    REWARD_VAL_MULT
                };
                let reward_total_val = ((in_range_val as f32) * mult).round() as i32;
                let rew_count = if use_rare {
                    1
                } else {
                    rng.get_in_range_i32(2, 4) as usize
                };
                let pool = if use_rare {
                    &rancher.rare_rewards
                } else {
                    &rancher.rewards
                };
                let target_val = if use_rare { 100 } else { reward_total_val };
                let Some(rew_pairs) = self.generate_item_list(
                    pool,
                    &whitelist,
                    &mut used,
                    target_val,
                    rew_count,
                    rng,
                ) else {
                    continue;
                };
                for (id, count) in rew_pairs {
                    rewards.push(ExchangeItemEntry {
                        id,
                        count,
                        spec_reward: NonIdentReward::None,
                    });
                }
                let cash_reward = if is_bonus {
                    NonIdentReward::NewbucksHuge
                } else {
                    let r = rng.get_in_range_i32(0, 6);
                    if r < 3 {
                        NonIdentReward::NewbucksSmall
                    } else if r < 5 {
                        NonIdentReward::NewbucksMedium
                    } else {
                        NonIdentReward::NewbucksLarge
                    }
                };
                rewards.push(ExchangeItemEntry {
                    id: IdentifiableId::None,
                    count: 1,
                    spec_reward: cash_reward,
                });
            }

            let blurb = if is_first_offer {
                1
            } else {
                rng.get_in_range_i32(1, rancher.num_blurbs + 1)
            };
            let offer_id = if is_bonus || mode.exchange_rewards_gold_plorts {
                format!("m.bonusoffer.{}", rancher.name)
            } else {
                format!("m.offer_{}.{}", blurb, rancher.name)
            };

            offer = Some(ExchangeOffer {
                offer_id,
                rancher_id: rancher.name.clone(),
                expire_time,
                early_exchange_time,
                requests,
                rewards,
            });
            break;
        }

        let created = offer?;
        model
            .world
            .curr_offers
            .insert(OfferType::General, created.clone());
        if let Some(ptype) = Self::rancher_progress_type(&created.rancher_id) {
            let cur = model.progress.get_progress(ptype);
            model.progress.set_progress(ptype, cur + 1);
        }
        model.world.pending_offer_rancher_ids.clear();
        Some(created)
    }

    /// Starts the next special progress quest for `Ogden`, `Mochi`, or `Viktor`.
    pub fn maybe_start_progress_offer(&self, model: &mut GameModel, offer_type: OfferType) -> bool {
        if model.world.curr_offers.contains_key(&offer_type) {
            return false;
        }
        let (ptype, levels, rancher_id) = match offer_type {
            OfferType::Ogden => (ProgressType::OgdenRewards, &self.ogden_levels, "ogden"),
            OfferType::Mochi => (ProgressType::MochiRewards, &self.mochi_levels, "mochi"),
            OfferType::Viktor => (ProgressType::ViktorRewards, &self.viktor_levels, "viktor"),
            _ => return false,
        };
        let curr = model.progress.get_progress(ptype);
        if curr < 0 || (curr as usize) >= levels.len() {
            return false;
        }
        let lvl = &levels[curr as usize];
        let offer = ExchangeOffer {
            offer_id: format!("m.offer.{}_level{}", rancher_id, curr + 1),
            rancher_id: rancher_id.into(),
            expire_time: f64::INFINITY,
            early_exchange_time: f64::NEG_INFINITY,
            requests: vec![ExchangeRequestedItemEntry {
                id: lvl.requested_item,
                count: lvl.count,
                progress: 0,
                spec_reward: NonIdentReward::None,
            }],
            rewards: vec![ExchangeItemEntry {
                id: IdentifiableId::None,
                count: 1,
                spec_reward: lvl.reward,
            }],
        };
        model.world.curr_offers.insert(offer_type, offer);
        true
    }

    pub fn newbucks_for_reward(reward: NonIdentReward) -> i32 {
        match reward {
            NonIdentReward::NewbucksSmall => 100,
            NonIdentReward::NewbucksMedium => 250,
            NonIdentReward::NewbucksLarge => 500,
            NonIdentReward::NewbucksHuge => 1000,
            NonIdentReward::NewbucksMochi => 650,
            _ => 0,
        }
    }

    /// Port of `ExchangeAcceptor.TryAccept` + `ExchangeDirector.Offer.TryAccept` + `ExchangeEjector`.
    pub fn try_accept_item(
        &self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        offer_type: OfferType,
        item_id: IdentifiableId,
        eject_pos: Vec3,
        eject_region: RegionSetId,
        rng: &mut Randoms,
    ) -> AcceptOutcome {
        let mut outcome = AcceptOutcome::default();
        let wt = model.world.world_time;

        let Some(offer) = model.world.curr_offers.get_mut(&offer_type) else {
            return outcome;
        };

        let mut matched = false;
        for req in &mut offer.requests {
            if req.id == item_id && !req.is_complete() {
                req.progress += 1;
                matched = true;
                break;
            }
        }
        if !matched {
            return outcome;
        }
        outcome.accepted = true;

        if !offer.is_complete() {
            return outcome;
        }

        outcome.completed = true;
        let completed_offer = model.world.curr_offers.remove(&offer_type).unwrap();

        if offer_type == OfferType::General && wt < completed_offer.early_exchange_time {
            outcome.early_bonus = true;
            *model
                .game_achieves
                .counts
                .entry("FULFILL_EXCHANGE_EARLY".to_string())
                .or_insert(0) += 1;
        }

        if offer_type == OfferType::General {
            let unlock = match completed_offer.rancher_id.as_str() {
                "ogden" => Some(ProgressType::UnlockOgdenMissions),
                "mochi" => Some(ProgressType::UnlockMochiMissions),
                "viktor" => Some(ProgressType::UnlockViktorMissions),
                _ => None,
            };
            if let Some(p) = unlock {
                if model.progress.get_progress(p) == 0 {
                    model.progress.set_progress(p, 1);
                    outcome.unlocked_progress.push(p);
                }
            }
        }

        for rew in &completed_offer.rewards {
            if rew.id != IdentifiableId::None {
                for _ in 0..rew.count {
                    let vel = Vec3::new(
                        rng.get_in_range_f32(-1.5, 1.5),
                        3.5,
                        rng.get_in_range_f32(-1.5, 1.5),
                    );
                    let aid = spawn_physical_actor(
                        model,
                        physics,
                        rew.id,
                        eject_region,
                        eject_pos,
                        vel,
                    );
                    outcome.spawned_rewards.push(aid);
                }
            }

            let cash = Self::newbucks_for_reward(rew.spec_reward);
            if cash > 0 {
                PlayerController::add_currency(&mut model.player, cash);
                outcome.newbucks_awarded += cash;
            }

            match rew.spec_reward {
                NonIdentReward::TimeExtension12h => {
                    let ext = 12.0 * 3600.0;
                    outcome.time_extension_secs += ext;
                    if let Some(ref mut end_t) = model.player.end_game_time {
                        *end_t += ext;
                    }
                }
                NonIdentReward::OgdenMix
                | NonIdentReward::OgdenGarden
                | NonIdentReward::OgdenRanch => {
                    let v = model.progress.get_progress(ProgressType::OgdenRewards) + 1;
                    model.progress.set_progress(ProgressType::OgdenRewards, v);
                    outcome.unlocked_progress.push(ProgressType::OgdenRewards);
                }
                NonIdentReward::MochiExtraMile
                | NonIdentReward::MochiCoop
                | NonIdentReward::MochiRanch => {
                    let v = model.progress.get_progress(ProgressType::MochiRewards) + 1;
                    model.progress.set_progress(ProgressType::MochiRewards, v);
                    outcome.unlocked_progress.push(ProgressType::MochiRewards);
                }
                NonIdentReward::ViktorChickenCloner
                | NonIdentReward::ViktorDeluxeDrones
                | NonIdentReward::ViktorRanch => {
                    let v = model.progress.get_progress(ProgressType::ViktorRewards) + 1;
                    model.progress.set_progress(ProgressType::ViktorRewards, v);
                    outcome.unlocked_progress.push(ProgressType::ViktorRewards);
                }
                _ => {}
            }
        }

        outcome
    }
}
