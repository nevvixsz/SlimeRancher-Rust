//! Port of `ProgressDirector.cs` and `ProgressDirector.DelayedProgressTracker`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GadgetId, PediaId, PlayerUpgrade, ProgressType};
use sr_model::{GameModel, MailType, ProgressTrackerId};

use crate::player::PlayerController;

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedTrackerRule {
    pub id: ProgressTrackerId,
    pub required_read_mail: Option<(MailType, &'static str)>,
    pub required_progress: Vec<(ProgressType, i32)>,
    pub delay_hours: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressStepEvents {
    pub triggered_trackers: Vec<ProgressTrackerId>,
    pub sent_mail_keys: Vec<String>,
    pub unlocked_pedia: Vec<PediaId>,
    pub granted_upgrades: Vec<PlayerUpgrade>,
    pub unlocked_blueprints: Vec<GadgetId>,
    pub science_tutorial_ready: bool,
}

/// Port of `ProgressDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressDirector {
    pub unlocked_trackers: HashSet<ProgressTrackerId>,
    pub ready_for_wistful_music: bool,
    pub ready_for_credits: bool,
}

impl ProgressDirector {
    pub fn rules(mode: &GameModeSettings) -> Vec<DelayedTrackerRule> {
        let mut out = vec![DelayedTrackerRule {
            id: ProgressTrackerId::TimeAfterLabDocks,
            required_read_mail: None,
            required_progress: vec![
                (ProgressType::UnlockLab, 1),
                (ProgressType::UnlockDocks, 1),
            ],
            delay_hours: 120.0,
        }];

        if !mode.suppress_story {
            out.extend([
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey1,
                    required_read_mail: None,
                    required_progress: vec![(ProgressType::SlimeDoors, 0)],
                    delay_hours: 3.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey2,
                    required_read_mail: Some((MailType::Personal, "casey_1")),
                    required_progress: vec![(ProgressType::SlimeDoors, 0)],
                    delay_hours: 48.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey3,
                    required_read_mail: Some((MailType::Personal, "casey_2")),
                    required_progress: vec![(ProgressType::SlimeDoors, 1)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey4,
                    required_read_mail: Some((MailType::Personal, "casey_3")),
                    required_progress: vec![(ProgressType::SlimeDoors, 2)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey5,
                    required_read_mail: Some((MailType::Personal, "casey_4")),
                    required_progress: vec![(ProgressType::SlimeDoors, 3)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey6,
                    required_read_mail: Some((MailType::Personal, "casey_5")),
                    required_progress: vec![(ProgressType::PlortDoor, 1)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey7,
                    required_read_mail: Some((MailType::Personal, "casey_6")),
                    required_progress: vec![(ProgressType::SlimeDoors, 4)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey8,
                    required_read_mail: Some((MailType::Personal, "casey_7")),
                    required_progress: vec![(ProgressType::SlimeDoors, 5)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey9,
                    required_read_mail: Some((MailType::Personal, "casey_8")),
                    required_progress: vec![(ProgressType::UnlockDesert, 1)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey10,
                    required_read_mail: Some((MailType::Personal, "casey_9")),
                    required_progress: vec![(ProgressType::SlimeDoors, 5)],
                    delay_hours: 24.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Casey11,
                    required_read_mail: Some((MailType::Personal, "casey_10")),
                    required_progress: vec![(ProgressType::HobsonEnd, 1)],
                    delay_hours: 6.0,
                },
                DelayedTrackerRule {
                    id: ProgressTrackerId::Hobson1,
                    required_read_mail: Some((MailType::Personal, "casey_11")),
                    required_progress: vec![(ProgressType::HobsonEnd, 1)],
                    delay_hours: 0.167,
                },
            ]);
        }

        out
    }

    pub fn init_model(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        self.unlocked_trackers.clear();
        for rule in Self::rules(mode) {
            model
                .progress
                .delayed_trackers
                .insert(rule.id, f64::INFINITY);
        }
    }

    pub fn game_fully_loaded(&mut self, model: &mut GameModel, mode: &GameModeSettings) -> ProgressStepEvents {
        let mut ev = ProgressStepEvents::default();
        if !model.progress.has_progress(ProgressType::UnlockLabDocksExtra)
            && model.progress.has_progress(ProgressType::UnlockLab)
            && model.progress.has_progress(ProgressType::UnlockDocks)
            && model
                .progress
                .get_delayed_tracker_time(ProgressTrackerId::TimeAfterLabDocks)
                .is_infinite()
        {
            self.add_progress(model, mode, ProgressType::UnlockLabDocksExtra, &mut ev);
        }
        self.check_progress_upgrades(model, &mut ev);
        self.check_trackers(model, mode);
        ev
    }

    pub fn set_unique_progress(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ptype: ProgressType,
        ev: &mut ProgressStepEvents,
    ) -> bool {
        if model.progress.set_unique_progress(ptype) {
            self.note_progress_changed(model, mode, ptype, ev);
            true
        } else {
            false
        }
    }

    pub fn set_progress(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ptype: ProgressType,
        count: i32,
        ev: &mut ProgressStepEvents,
    ) {
        if model.progress.progress_dict.get(&ptype).copied() == Some(count) {
            return;
        }
        model.progress.set_progress(ptype, count);
        self.note_progress_changed(model, mode, ptype, ev);
    }

    pub fn add_progress(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ptype: ProgressType,
        ev: &mut ProgressStepEvents,
    ) -> i32 {
        let val = model.progress.add_progress(ptype);
        self.note_progress_changed(model, mode, ptype, ev);
        val
    }

    fn note_progress_changed(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ptype: ProgressType,
        ev: &mut ProgressStepEvents,
    ) {
        if ptype == ProgressType::CorporatePartner {
            let level = model.progress.get_progress(ptype);
            if level >= 1 && level <= 28 && mode.allow_mail() {
                let key = format!("rewards_level_{level}");
                if model.mail.send_mail(MailType::Personal, &key) {
                    ev.sent_mail_keys.push(key);
                }
            }
            if level >= 28 && mode.allow_mail() {
                if model.mail.send_mail(MailType::Personal, "rewards_level_max") {
                    ev.sent_mail_keys.push("rewards_level_max".to_string());
                }
            }
            self.check_progress_upgrades(model, ev);
        } else if ptype == ProgressType::OgdenRewards {
            if model.progress.get_progress(ptype) >= 3
                && model.pedia.unlock(PediaId::OgdenRetreat)
            {
                ev.unlocked_pedia.push(PediaId::OgdenRetreat);
            }
        } else if ptype == ProgressType::MochiRewards {
            if model.progress.get_progress(ptype) >= 3
                && model.pedia.unlock(PediaId::MochiManor)
            {
                ev.unlocked_pedia.push(PediaId::MochiManor);
            }
        } else if ptype == ProgressType::UnlockLab {
            ev.science_tutorial_ready = true;
        } else if ptype == ProgressType::HobsonEnd {
            let cur = model
                .game_achieves
                .counts
                .get("FIND_HOBSONS_END")
                .copied()
                .unwrap_or(0);
            if cur < 1 {
                model
                    .game_achieves
                    .counts
                    .insert("FIND_HOBSONS_END".to_string(), 1);
            }
        }

        self.check_trackers(model, mode);
    }

    pub fn check_progress_upgrades(&self, model: &mut GameModel, ev: &mut ProgressStepEvents) {
        let progress = model.progress.get_progress(ProgressType::CorporatePartner);
        let wt = model.world.world_time;

        let mut grant = |u: PlayerUpgrade| {
            if PlayerController::grant_upgrade(&mut model.player, u, wt) {
                ev.granted_upgrades.push(u);
            }
        };
        if progress >= 19 {
            grant(PlayerUpgrade::RunEfficiency);
            grant(PlayerUpgrade::RunEfficiency2);
        }
        if progress >= 20 {
            grant(PlayerUpgrade::Ammo1);
            grant(PlayerUpgrade::Ammo2);
            grant(PlayerUpgrade::Ammo3);
            grant(PlayerUpgrade::Ammo4);
        }
        if progress >= 21 {
            grant(PlayerUpgrade::Health1);
            grant(PlayerUpgrade::Health2);
            grant(PlayerUpgrade::Health3);
            grant(PlayerUpgrade::Health4);
        }
        if progress >= 22 {
            grant(PlayerUpgrade::GoldenSureshot);
        }

        let mut add_bp = |g: GadgetId| {
            if !model.gadgets.blueprints.contains(&g) {
                model.gadgets.blueprints.push(g);
                ev.unlocked_blueprints.push(g);
            }
        };
        if progress >= 11 {
            add_bp(GadgetId::MarketLink);
        }
        if progress >= 20 {
            add_bp(GadgetId::GordoSnareMaster);
        }
        if progress >= 23 {
            add_bp(GadgetId::ExtractorDrillTitan);
        }
        if progress >= 24 {
            add_bp(GadgetId::ExtractorPumpAbyssal);
        }
        if progress >= 25 {
            add_bp(GadgetId::ExtractorApiaryRoyal);
        }
        if progress >= 26 {
            add_bp(GadgetId::LampGold);
        }
        if progress >= 27 {
            add_bp(GadgetId::WarpDepotGold);
        }
        if progress >= 28 {
            add_bp(GadgetId::TeleporterGold);
        }
    }

    pub fn maybe_unlock_ogden_missions(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ev: &mut ProgressStepEvents,
    ) -> bool {
        if mode.enable_ogden_missions
            && !model.progress.has_progress(ProgressType::UnlockOgdenMissions)
            && model.progress.has_progress(ProgressType::UnlockMoss)
            && model.progress.has_progress(ProgressType::UnlockOvergrowth)
        {
            self.add_progress(model, mode, ProgressType::UnlockOgdenMissions, ev);
            if mode.allow_mail() && model.mail.send_mail(MailType::Personal, "ogden_invite") {
                ev.sent_mail_keys.push("ogden_invite".to_string());
            }
            return true;
        }
        false
    }

    pub fn maybe_unlock_mochi_missions(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ev: &mut ProgressStepEvents,
    ) -> bool {
        if mode.enable_mochi_missions
            && !model.progress.has_progress(ProgressType::UnlockMochiMissions)
            && model.progress.has_progress(ProgressType::UnlockQuarry)
            && model.progress.has_progress(ProgressType::UnlockGrotto)
        {
            self.add_progress(model, mode, ProgressType::UnlockMochiMissions, ev);
            if mode.allow_mail() && model.mail.send_mail(MailType::Personal, "mochi_invite") {
                ev.sent_mail_keys.push("mochi_invite".to_string());
            }
            return true;
        }
        false
    }

    pub fn maybe_unlock_viktor_missions(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ev: &mut ProgressStepEvents,
    ) -> bool {
        if mode.enable_viktor_missions
            && !model
                .progress
                .has_progress(ProgressType::UnlockViktorMissions)
            && model.progress.has_progress(ProgressType::UnlockLab)
            && model.progress.has_progress(ProgressType::UnlockRuins)
            && model.player.has_upgrade(PlayerUpgrade::TreasureCracker2)
        {
            self.add_progress(model, mode, ProgressType::UnlockViktorMissions, ev);
            if mode.allow_mail() && model.mail.send_mail(MailType::Personal, "viktor_invite") {
                ev.sent_mail_keys.push("viktor_invite".to_string());
            }
            return true;
        }
        false
    }

    /// Evaluates tracker start conditions and schedules `delayed_trackers` timestamps.
    pub fn check_trackers(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        let wt = model.world.world_time;
        for rule in Self::rules(mode) {
            if model.progress.get_delayed_tracker_time(rule.id).is_infinite() {
                let progress_ok = rule
                    .required_progress
                    .iter()
                    .all(|&(pt, min_v)| model.progress.get_progress(pt) >= min_v);
                let mail_ok = rule
                    .required_read_mail
                    .map_or(true, |(mt, key)| model.mail.has_read_mail(mt, key));
                if progress_ok && mail_ok {
                    let target = wt + (rule.delay_hours as f64) * 3600.0;
                    model.progress.set_delayed_tracker_time(rule.id, target);
                }
            }
        }
    }

    /// Steps delayed progress trackers and fires story mails / unlocks when world time reaches them.
    pub fn step(&mut self, model: &mut GameModel, mode: &GameModeSettings) -> ProgressStepEvents {
        let mut ev = ProgressStepEvents::default();
        self.check_trackers(model, mode);

        let wt = model.world.world_time;
        let rules = Self::rules(mode);
        let mut fired = Vec::new();
        for rule in rules {
            if self.unlocked_trackers.contains(&rule.id) {
                continue;
            }
            let due = model.progress.get_delayed_tracker_time(rule.id);
            if wt >= due {
                self.unlocked_trackers.insert(rule.id);
                fired.push(rule.id);
            }
        }

        for id in fired {
            ev.triggered_trackers.push(id);
            match id {
                ProgressTrackerId::TimeAfterLabDocks => {
                    self.set_unique_progress(model, mode, ProgressType::UnlockLabDocksExtra, &mut ev);
                }
                other => {
                    if let Some(key) = other.mail_key() {
                        if mode.allow_mail() && model.mail.send_mail(MailType::Personal, key) {
                            ev.sent_mail_keys.push(key.to_string());
                        }
                    }
                }
            }
        }

        self.maybe_unlock_ogden_missions(model, mode, &mut ev);
        self.maybe_unlock_mochi_missions(model, mode, &mut ev);
        self.maybe_unlock_viktor_missions(model, mode, &mut ev);

        ev
    }

    pub fn queue_credits(&mut self, model: &mut GameModel) {
        self.ready_for_credits = true;
        let cur = model
            .game_achieves
            .counts
            .get("FINISH_ADVENTURE")
            .copied()
            .unwrap_or(0);
        if cur < 1 {
            model
                .game_achieves
                .counts
                .insert("FINISH_ADVENTURE".to_string(), 1);
        }
    }

    pub fn queue_ranch_wistful_music(&mut self) {
        self.ready_for_wistful_music = true;
    }

    /// Port of `ProgressDirector.NoteReturnedToRanch`: returns `(play_credits, play_wistful_music)`.
    pub fn note_returned_to_ranch(&mut self) -> (bool, bool) {
        if self.ready_for_credits {
            self.ready_for_credits = false;
            return (true, false);
        }
        if self.ready_for_wistful_music {
            self.ready_for_wistful_music = false;
            return (false, true);
        }
        (false, false)
    }
}
