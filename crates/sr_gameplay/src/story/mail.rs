//! Port of `MailDirector.cs` and `MailUI.cs` story triggers (`welcome`, `casey_1..11`,
//! `hobson_1`, `partner_rewards`, `ogden_invite`, `mochi_invite`, `viktor_invite`, `secret_styles`).

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{PediaId, ProgressType};
use sr_model::{GameModel, MailEntry, MailType, TutorialId};

use crate::story::progress::{ProgressDirector, ProgressStepEvents};

pub const PARTNER_MAIL_KEY: &str = "partner_rewards";
pub const PARTNER_UNLOCK_TIME: f64 = 561_600.0;
pub const CASEY_MAIL_PREFIX: &str = "casey_";
pub const CASEY_MAIL_FINAL: &str = "casey_11";
pub const HOBSON_MAIL_KEY: &str = "hobson_1";
pub const OGDEN_MAIL_KEY: &str = "ogden_invite";
pub const MOCHI_MAIL_KEY: &str = "mochi_invite";
pub const VIKTOR_MAIL_KEY: &str = "viktor_invite";
pub const SECRET_STYLE_MAIL_KEY: &str = "secret_styles";
pub const SECRET_STYLE_MAIL_DELAY: f64 = 600.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MailStepEvents {
    pub sent_mail: Vec<(MailType, String)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MailReadOutcome {
    pub was_unread: bool,
    pub wistful_music_queued: bool,
    pub credits_queued: bool,
    pub unlocked_pedia: Option<PediaId>,
    pub triggered_tutorial: Option<TutorialId>,
    pub progress_events: ProgressStepEvents,
}

/// Port of `MailDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailDirector {
    pub secret_style_dlc_enabled: bool,
    pub secret_style_due_time: Option<f64>,
}

impl MailDirector {
    pub fn init_model(model: &mut GameModel, mode: &GameModeSettings) {
        model.mail.all_mail.clear();
        model.mail.has_new_mail = false;
        model.mail.has_partner_mail = false;
        model.mail.has_secret_style_mail = false;
        if mode.allow_mail() {
            model.mail.send_mail(MailType::Personal, "welcome");
        }
    }

    pub fn send_mail(
        model: &mut GameModel,
        mode: &GameModeSettings,
        mail_type: MailType,
        key: impl Into<String>,
    ) -> bool {
        if !mode.allow_mail() {
            return false;
        }
        let key = key.into();
        let sent = model.mail.send_mail(mail_type, key.clone());
        if sent && key == PARTNER_MAIL_KEY {
            model.mail.has_partner_mail = true;
        }
        if sent && key == SECRET_STYLE_MAIL_KEY {
            model.mail.has_secret_style_mail = true;
        }
        sent
    }

    pub fn get_mail_recent_first(model: &GameModel) -> Vec<MailEntry> {
        let mut list = model.mail.all_mail.clone();
        list.reverse();
        list
    }

    pub fn new_mail_count(model: &GameModel) -> usize {
        model.mail.all_mail.iter().filter(|m| !m.read).count()
    }

    pub fn has_read_mail(model: &GameModel, mail_type: MailType, key: &str) -> bool {
        model.mail.has_read_mail(mail_type, key)
    }

    pub fn step(&mut self, model: &mut GameModel, mode: &GameModeSettings) -> MailStepEvents {
        let mut ev = MailStepEvents::default();
        let wt = model.world.world_time;

        if !model.mail.has_partner_mail
            && mode.enable_partner_rewards
            && wt >= PARTNER_UNLOCK_TIME
        {
            if Self::send_mail(model, mode, MailType::Personal, PARTNER_MAIL_KEY) {
                ev.sent_mail
                    .push((MailType::Personal, PARTNER_MAIL_KEY.to_string()));
            }
        }

        if self.secret_style_dlc_enabled && mode.enable_dlc && !model.mail.has_secret_style_mail {
            let due = *self
                .secret_style_due_time
                .get_or_insert(wt + SECRET_STYLE_MAIL_DELAY);
            if wt >= due {
                if Self::send_mail(model, mode, MailType::Personal, SECRET_STYLE_MAIL_KEY) {
                    ev.sent_mail
                        .push((MailType::Personal, SECRET_STYLE_MAIL_KEY.to_string()));
                }
            }
        }

        ev
    }

    /// Port of `MailUI.Select` + `MailDirector.MarkRead` + `TutorialDirector.OnMailRead`.
    pub fn select_and_read_mail(
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
        mail_type: MailType,
        key: &str,
    ) -> MailReadOutcome {
        let mut out = MailReadOutcome::default();
        let Some(existing) = model
            .mail
            .all_mail
            .iter()
            .find(|m| m.mail_type == mail_type && m.key == key)
            .cloned()
        else {
            return out;
        };

        out.was_unread = !existing.read;

        if !existing.read && key.starts_with(CASEY_MAIL_PREFIX) {
            progress.queue_ranch_wistful_music();
            out.wistful_music_queued = true;
        }
        if !existing.read && key == CASEY_MAIL_FINAL {
            progress.queue_credits(model);
            out.credits_queued = true;
        }

        model.mail.mark_read(mail_type, key);

        if key == PARTNER_MAIL_KEY {
            if model.pedia.unlock(PediaId::Partner) {
                out.unlocked_pedia = Some(PediaId::Partner);
            }
            if !model
                .progress
                .has_progress(ProgressType::CorporatePartnerUnlock)
            {
                progress.add_progress(
                    model,
                    mode,
                    ProgressType::CorporatePartnerUnlock,
                    &mut out.progress_events,
                );
            }
        } else if key == HOBSON_MAIL_KEY
            && !model.progress.has_progress(ProgressType::HobsonEndUnlock)
        {
            progress.add_progress(
                model,
                mode,
                ProgressType::HobsonEndUnlock,
                &mut out.progress_events,
            );
        }

        out.triggered_tutorial = match key {
            OGDEN_MAIL_KEY => Some(TutorialId::EnterZoneOgdenRanch),
            MOCHI_MAIL_KEY => Some(TutorialId::EnterZoneMochiRanch),
            VIKTOR_MAIL_KEY => Some(TutorialId::EnterZoneViktorLab),
            _ => None,
        };

        progress.check_trackers(model, mode);
        out
    }
}
