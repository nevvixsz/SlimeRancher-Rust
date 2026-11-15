//! Step 8: Story, Progress, Mail, Journals, Slimepedia, Tutorials, Credits & Achievements.

pub mod achievements;
pub mod mail;
pub mod pedia_tutorials;
pub mod progress;

pub use achievements::{
    AchievementId, AchievementTier, AchievementsDirector, EndGamePlortEntry, EndGameSummary,
};
pub use mail::{
    MailDirector, MailReadOutcome, MailStepEvents, CASEY_MAIL_FINAL, CASEY_MAIL_PREFIX,
    HOBSON_MAIL_KEY, MOCHI_MAIL_KEY, OGDEN_MAIL_KEY, PARTNER_MAIL_KEY, PARTNER_UNLOCK_TIME,
    SECRET_STYLE_MAIL_DELAY, SECRET_STYLE_MAIL_KEY, VIKTOR_MAIL_KEY,
};
pub use pedia_tutorials::{
    JournalEntryDef, PediaDirector, TutorialDirector, PEDIA_CATEGORY_HEADERS,
    PEDIA_HIDDEN_ENTRIES, PEDIA_INIT_UNLOCKED,
};
pub use progress::{DelayedTrackerRule, ProgressDirector, ProgressStepEvents};

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_model::GameModel;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StoryStepEvents {
    pub progress: ProgressStepEvents,
    pub mail: MailStepEvents,
    pub earned_achievements: Vec<AchievementId>,
}

/// Unified Step 8 runtime coordinating Progress, Mail, Slimepedia, Tutorials, and Achievements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryRuntime {
    pub progress: ProgressDirector,
    pub mail: MailDirector,
    pub pedia: PediaDirector,
    pub tutorials: TutorialDirector,
}

impl StoryRuntime {
    pub fn init_model(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        self.progress.init_model(model, mode);
        MailDirector::init_model(model, mode);
        PediaDirector::init_model(model);
    }

    pub fn step(&mut self, model: &mut GameModel, mode: &GameModeSettings) -> StoryStepEvents {
        let progress = self.progress.step(model, mode);
        if progress.science_tutorial_ready {
            self.tutorials
                .on_progress(model, mode, sr_ids::ProgressType::UnlockLab);
        }
        let mail = self.mail.step(model, mode);
        let earned_achievements = AchievementsDirector::evaluate_all(model);

        StoryStepEvents {
            progress,
            mail,
            earned_achievements,
        }
    }

    pub fn on_returned_to_ranch(&mut self) -> (bool, bool) {
        self.progress.note_returned_to_ranch()
    }
}
