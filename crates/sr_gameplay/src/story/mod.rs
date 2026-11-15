//! Step 8: Story, Progress, Mail, Journals, Slimepedia, Tutorials, Credits & Achievements.

pub mod mail;
pub mod pedia_tutorials;
pub mod progress;

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
