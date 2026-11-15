//! Port of `ProgressModel.cs`, `MailModel.cs`, `PediaModel.cs`, `TutorialsModel.cs`,
//! `HolidayModel.cs`, `AppearancesModel.cs`, `InstrumentModel.cs`, and `AchievesModel.cs`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, PediaId, ProgressType};

use crate::enums::{AppearanceSaveSet, Instrument, MailType, ProgressTrackerId, TutorialId};

/// Port of `ProgressModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProgressModel {
    pub progress_dict: HashMap<ProgressType, i32>,
    pub delayed_progress_time_dict: HashMap<ProgressType, f64>,
    #[serde(default)]
    pub delayed_trackers: HashMap<ProgressTrackerId, f64>,
}

impl ProgressModel {
    pub fn set_progress(&mut self, ptype: ProgressType, val: i32) {
        self.progress_dict.insert(ptype, val);
    }

    pub fn get_progress(&self, ptype: ProgressType) -> i32 {
        self.progress_dict.get(&ptype).copied().unwrap_or(0)
    }

    pub fn has_progress(&self, ptype: ProgressType) -> bool {
        self.get_progress(ptype) > 0
    }

    pub fn set_unique_progress(&mut self, ptype: ProgressType) -> bool {
        if !self.progress_dict.contains_key(&ptype) {
            self.progress_dict.insert(ptype, 1);
            true
        } else {
            false
        }
    }

    pub fn add_progress(&mut self, ptype: ProgressType) -> i32 {
        let entry = self.progress_dict.entry(ptype).or_insert(0);
        *entry += 1;
        *entry
    }

    pub fn get_delayed_tracker_time(&self, id: ProgressTrackerId) -> f64 {
        self.delayed_trackers
            .get(&id)
            .copied()
            .unwrap_or(f64::INFINITY)
    }

    pub fn set_delayed_tracker_time(&mut self, id: ProgressTrackerId, time: f64) {
        self.delayed_trackers.insert(id, time);
    }
}

/// Port of `MailModel.Mail`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MailEntry {
    pub mail_type: MailType,
    pub key: String,
    pub read: bool,
}

/// Port of `MailModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MailModel {
    pub all_mail: Vec<MailEntry>,
    pub has_new_mail: bool,
    #[serde(default)]
    pub has_partner_mail: bool,
    #[serde(default)]
    pub has_secret_style_mail: bool,
}

impl MailModel {
    pub fn send_mail(&mut self, mail_type: MailType, key: impl Into<String>) -> bool {
        let key = key.into();
        if !self
            .all_mail
            .iter()
            .any(|m| m.mail_type == mail_type && m.key == key)
        {
            self.all_mail.push(MailEntry {
                mail_type,
                key,
                read: false,
            });
            self.has_new_mail = true;
            true
        } else {
            false
        }
    }

    pub fn mark_read(&mut self, mail_type: MailType, key: &str) {
        for m in &mut self.all_mail {
            if m.mail_type == mail_type && m.key == key {
                m.read = true;
            }
        }
        self.has_new_mail = self.all_mail.iter().any(|m| !m.read);
    }

    pub fn has_read_mail(&self, mail_type: MailType, key: &str) -> bool {
        self.all_mail
            .iter()
            .any(|m| m.mail_type == mail_type && m.key == key && m.read)
    }
}

/// Port of `PediaModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PediaModel {
    pub unlocked: HashSet<PediaId>,
    pub completed_tuts: HashSet<TutorialId>,
    pub progress_count: i32,
}

impl PediaModel {
    pub fn unlock(&mut self, id: PediaId) -> bool {
        self.unlocked.insert(id)
    }
}

/// Port of `TutorialsModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TutorialsModel {
    pub completed_ids: HashSet<TutorialId>,
    pub popup_queue: Vec<TutorialId>,
}

/// Port of `HolidayModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HolidayModel {
    pub event_gordos: HashSet<String>,
    pub event_echo_note_gordos: HashSet<String>,
}

/// Port of `AppearancesModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AppearancesModel {
    pub unlocks: HashMap<IdentifiableId, HashSet<AppearanceSaveSet>>,
    pub selections: HashMap<IdentifiableId, AppearanceSaveSet>,
}

/// Port of `InstrumentModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstrumentModel {
    pub unlocked: HashSet<Instrument>,
    pub selection: Instrument,
}

/// Port of `AchievesModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AchievesModel {
    pub earned_achievements: HashSet<String>,
    pub counts: HashMap<String, i32>,
    pub enums: HashMap<String, HashSet<i32>>,
    #[serde(default)]
    pub doubles: HashMap<String, f64>,
    #[serde(default)]
    pub game_int_stats: HashMap<String, i32>,
    #[serde(default)]
    pub game_id_dicts: HashMap<String, HashMap<IdentifiableId, i32>>,
}
