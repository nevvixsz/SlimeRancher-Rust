//! Step 9: Post-launch content updates (Ogden's Wilds, Mochi's Nimble Valley,
//! Viktor's Slimulation, HolidayDirector, and SlimeAppearanceDirector).

pub mod valley;
pub mod wilds;

pub use valley::{
    AmmoReplacerState, ElectricFieldState, GeneratorConfig, ValleyStepEvents, ValleySystem,
    DEFAULT_GENERATOR_ACTIVE_MINS, DEFAULT_GENERATOR_COOLDOWN_MINS,
    DEFAULT_GENERATOR_COUNTDOWN_MINS, DEFAULT_PLORT_COLLECT_MINS,
};
pub use wilds::{
    KookadobaPatchNodeState, WildsStepEvents, WildsSystem, KOOKADOBA_SPAWN_DELAY_HOURS,
    KOOKADOBA_SPAWN_DELAY_SECS,
};
