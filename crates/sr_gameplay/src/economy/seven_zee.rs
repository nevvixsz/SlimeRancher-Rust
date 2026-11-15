//! Port of `CorporatePartnerUI.cs` and 7Zee Rewards Club rank progression
//! from `ProgressDirector.cs`.

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, PediaId, PlayerUpgrade, ProgressType};
use sr_model::{GameModel, MailType};

use crate::player::PlayerController;

pub const CORPORATE_PARTNER_MAX_RANK: i32 = 28;
pub const CHROMA_UNLOCK_RANK: i32 = 5;
pub const SLIME_TOYS_UNLOCK_RANK: i32 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SevenZeeRank {
    pub rank: i32,
    pub cost: i32,
    pub title_key: &'static str,
}

/// Canonical 28 ranks of the 7Zee Rewards Club.
pub const SEVEN_ZEE_RANKS: [SevenZeeRank; 28] = [
    SevenZeeRank { rank: 1, cost: 750, title_key: "m.partner_rank.1" },
    SevenZeeRank { rank: 2, cost: 1_500, title_key: "m.partner_rank.2" },
    SevenZeeRank { rank: 3, cost: 2_500, title_key: "m.partner_rank.3" },
    SevenZeeRank { rank: 4, cost: 4_000, title_key: "m.partner_rank.4" },
    SevenZeeRank { rank: 5, cost: 6_000, title_key: "m.partner_rank.5" },
    SevenZeeRank { rank: 6, cost: 8_500, title_key: "m.partner_rank.6" },
    SevenZeeRank { rank: 7, cost: 11_500, title_key: "m.partner_rank.7" },
    SevenZeeRank { rank: 8, cost: 15_000, title_key: "m.partner_rank.8" },
    SevenZeeRank { rank: 9, cost: 20_000, title_key: "m.partner_rank.9" },
    SevenZeeRank { rank: 10, cost: 25_000, title_key: "m.partner_rank.10" },
    SevenZeeRank { rank: 11, cost: 30_000, title_key: "m.partner_rank.11" },
    SevenZeeRank { rank: 12, cost: 37_500, title_key: "m.partner_rank.12" },
    SevenZeeRank { rank: 13, cost: 45_000, title_key: "m.partner_rank.13" },
    SevenZeeRank { rank: 14, cost: 52_500, title_key: "m.partner_rank.14" },
    SevenZeeRank { rank: 15, cost: 60_000, title_key: "m.partner_rank.15" },
    SevenZeeRank { rank: 16, cost: 67_500, title_key: "m.partner_rank.16" },
    SevenZeeRank { rank: 17, cost: 75_000, title_key: "m.partner_rank.17" },
    SevenZeeRank { rank: 18, cost: 82_500, title_key: "m.partner_rank.18" },
    SevenZeeRank { rank: 19, cost: 90_000, title_key: "m.partner_rank.19" },
    SevenZeeRank { rank: 20, cost: 97_500, title_key: "m.partner_rank.20" },
    SevenZeeRank { rank: 21, cost: 105_000, title_key: "m.partner_rank.21" },
    SevenZeeRank { rank: 22, cost: 115_000, title_key: "m.partner_rank.22" },
    SevenZeeRank { rank: 23, cost: 125_000, title_key: "m.partner_rank.23" },
    SevenZeeRank { rank: 24, cost: 135_000, title_key: "m.partner_rank.24" },
    SevenZeeRank { rank: 25, cost: 150_000, title_key: "m.partner_rank.25" },
    SevenZeeRank { rank: 26, cost: 175_000, title_key: "m.partner_rank.26" },
    SevenZeeRank { rank: 27, cost: 200_000, title_key: "m.partner_rank.27" },
    SevenZeeRank { rank: 28, cost: 250_000, title_key: "m.partner_rank.28" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SevenZeeError {
    Locked,
    MaxRankReached,
    InsufficientCurrency { required: i32, current: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SevenZeePurchaseResult {
    pub new_rank: i32,
    pub cost_paid: i32,
    pub unlocked_upgrades: Vec<PlayerUpgrade>,
    pub unlocked_blueprints: Vec<GadgetId>,
    pub unlocked_pedia: Vec<PediaId>,
}

pub fn unlock_seven_zee(model: &mut GameModel) {
    if model.progress.get_progress(ProgressType::CorporatePartnerUnlock) <= 0 {
        model
            .progress
            .set_progress(ProgressType::CorporatePartnerUnlock, 1);
    }
}

pub fn current_seven_zee_rank(model: &GameModel) -> i32 {
    model.progress.get_progress(ProgressType::CorporatePartner)
}

pub fn next_seven_zee_rank(model: &GameModel) -> Option<&'static SevenZeeRank> {
    let curr = current_seven_zee_rank(model);
    if curr < 0 || curr >= CORPORATE_PARTNER_MAX_RANK {
        return None;
    }
    SEVEN_ZEE_RANKS.get(curr as usize)
}

fn add_blueprint(model: &mut GameModel, id: GadgetId, out: &mut Vec<GadgetId>) {
    if !model.gadgets.blueprints.contains(&id) {
        model.gadgets.blueprints.push(id);
        out.push(id);
    }
    model.gadgets.avail_blueprints.retain(|&b| b != id);
}

fn grant_upgrade(
    model: &mut GameModel,
    u: PlayerUpgrade,
    wt: f64,
    out: &mut Vec<PlayerUpgrade>,
) {
    if PlayerController::grant_upgrade(&mut model.player, u, wt) {
        out.push(u);
    }
}

/// Port of `CorporatePartnerUI.Purchase()` + `ProgressDirector.CheckProgressUpgrades()`.
pub fn purchase_next_seven_zee_rank(
    model: &mut GameModel,
    require_unlock_flag: bool,
) -> Result<SevenZeePurchaseResult, SevenZeeError> {
    if require_unlock_flag && model.progress.get_progress(ProgressType::CorporatePartnerUnlock) <= 0
    {
        return Err(SevenZeeError::Locked);
    }

    let curr = current_seven_zee_rank(model);
    if curr >= CORPORATE_PARTNER_MAX_RANK {
        return Err(SevenZeeError::MaxRankReached);
    }

    let next = &SEVEN_ZEE_RANKS[curr as usize];
    if !PlayerController::spend_currency(&mut model.player, next.cost) {
        return Err(SevenZeeError::InsufficientCurrency {
            required: next.cost,
            current: model.player.currency,
        });
    }

    let rank = next.rank;
    model
        .progress
        .set_progress(ProgressType::CorporatePartner, rank);
    *model
        .game_achieves
        .counts
        .entry("REWARD_LEVELS".to_string())
        .or_insert(0) += 1;

    model
        .mail
        .send_mail(MailType::Personal, format!("rewards_level_{rank}"));
    if rank >= CORPORATE_PARTNER_MAX_RANK {
        model.mail.send_mail(MailType::Personal, "rewards_level_max");
    }

    let mut res = SevenZeePurchaseResult {
        new_rank: rank,
        cost_paid: next.cost,
        ..Default::default()
    };

    let wt = model.world.world_time;
    if rank >= CHROMA_UNLOCK_RANK && model.pedia.unlock(PediaId::Chroma) {
        res.unlocked_pedia.push(PediaId::Chroma);
    }
    if rank >= SLIME_TOYS_UNLOCK_RANK && model.pedia.unlock(PediaId::SlimeToys) {
        res.unlocked_pedia.push(PediaId::SlimeToys);
    }

    // Exact thresholds from ProgressDirector.CheckVacpackUpgrades & CheckBlueprintUpgrades
    if rank >= 11 {
        add_blueprint(model, GadgetId::MarketLink, &mut res.unlocked_blueprints);
    }
    if rank >= 19 {
        grant_upgrade(model, PlayerUpgrade::RunEfficiency, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::RunEfficiency2, wt, &mut res.unlocked_upgrades);
    }
    if rank >= 20 {
        grant_upgrade(model, PlayerUpgrade::Ammo1, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Ammo2, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Ammo3, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Ammo4, wt, &mut res.unlocked_upgrades);
        add_blueprint(model, GadgetId::GordoSnareMaster, &mut res.unlocked_blueprints);
    }
    if rank >= 21 {
        grant_upgrade(model, PlayerUpgrade::Health1, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Health2, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Health3, wt, &mut res.unlocked_upgrades);
        grant_upgrade(model, PlayerUpgrade::Health4, wt, &mut res.unlocked_upgrades);
    }
    if rank >= 22 {
        grant_upgrade(model, PlayerUpgrade::GoldenSureshot, wt, &mut res.unlocked_upgrades);
    }
    if rank >= 23 {
        add_blueprint(model, GadgetId::ExtractorDrillTitan, &mut res.unlocked_blueprints);
    }
    if rank >= 24 {
        add_blueprint(model, GadgetId::ExtractorPumpAbyssal, &mut res.unlocked_blueprints);
    }
    if rank >= 25 {
        add_blueprint(model, GadgetId::ExtractorApiaryRoyal, &mut res.unlocked_blueprints);
    }
    if rank >= 26 {
        add_blueprint(model, GadgetId::LampGold, &mut res.unlocked_blueprints);
    }
    if rank >= 27 {
        add_blueprint(model, GadgetId::WarpDepotGold, &mut res.unlocked_blueprints);
    }
    if rank >= 28 {
        add_blueprint(model, GadgetId::TeleporterGold, &mut res.unlocked_blueprints);
    }

    Ok(res)
}
