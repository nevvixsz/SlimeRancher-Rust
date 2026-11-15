//! Step 9: Post-launch content updates (Ogden's Wilds, Mochi's Nimble Valley,
//! Viktor's Slimulation, HolidayDirector, and SlimeAppearanceDirector).

pub mod slimulations;
pub mod valley;
pub mod wilds;

pub use slimulations::{
    GlitchTarrRuntime, ImpostoConfig, SlimulationsStepEvents, SlimulationsSystem, TarrNodeConfig,
    DEFAULT_DAMAGE_LOSS_COOLDOWN_MINS, DEFAULT_EXIT_TELEPORTER_DELAY_HOURS,
    DEFAULT_IMPOSTO_COOLDOWN_HOURS, DEFAULT_TARR_BASE_MULTIPLY_CHANCE, DEFAULT_TARR_LIFETIME_MINS,
    DEFAULT_TARR_MULTIPLY_DEGRADATION, DEFAULT_TARR_NODE_ACTIVATION_DELAY_HOURS,
    DEFAULT_TARR_SPAWNER_THROTTLE_MINS, GLITCH_STORAGE_MAX_COUNT,
};
pub use valley::{
    AmmoReplacerState, ElectricFieldState, GeneratorConfig, ValleyStepEvents, ValleySystem,
    DEFAULT_GENERATOR_ACTIVE_MINS, DEFAULT_GENERATOR_COOLDOWN_MINS,
    DEFAULT_GENERATOR_COUNTDOWN_MINS, DEFAULT_PLORT_COLLECT_MINS,
};
pub use wilds::{
    KookadobaPatchNodeState, WildsStepEvents, WildsSystem, KOOKADOBA_SPAWN_DELAY_HOURS,
    KOOKADOBA_SPAWN_DELAY_SECS,
};
