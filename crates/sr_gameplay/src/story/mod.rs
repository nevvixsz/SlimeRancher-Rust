//! Step 8: Story, Progress, Mail, Journals, Slimepedia, Tutorials, Credits & Achievements.

pub mod mail;
pub mod progress;

pub use mail::{
    MailDirector, MailReadOutcome, MailStepEvents, CASEY_MAIL_FINAL, CASEY_MAIL_PREFIX,
    HOBSON_MAIL_KEY, MOCHI_MAIL_KEY, OGDEN_MAIL_KEY, PARTNER_MAIL_KEY, PARTNER_UNLOCK_TIME,
    SECRET_STYLE_MAIL_DELAY, SECRET_STYLE_MAIL_KEY, VIKTOR_MAIL_KEY,
};
pub use progress::{DelayedTrackerRule, ProgressDirector, ProgressStepEvents};
