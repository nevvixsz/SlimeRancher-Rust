//! Step 9: Post-launch content updates (Ogden's Wilds, Mochi's Nimble Valley,
//! Viktor's Slimulation, HolidayDirector, and SlimeAppearanceDirector).

pub mod wilds;

pub use wilds::{
    KookadobaPatchNodeState, WildsStepEvents, WildsSystem, KOOKADOBA_SPAWN_DELAY_HOURS,
    KOOKADOBA_SPAWN_DELAY_SECS,
};
