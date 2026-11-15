//! Step 9: Post-launch content updates (Ogden's Wilds, Mochi's Nimble Valley,
//! Viktor's Slimulation, HolidayDirector, and SlimeAppearanceDirector).

pub mod appearances;
pub mod holiday;
pub mod slimulations;
pub mod valley;
pub mod wilds;

pub use appearances::SlimeAppearanceDirector;
pub use holiday::{
    EventEchoNoteGordoDef, EventGordoDef, HolidayDate, HolidayDirector, ECHO_NOTE_GORDO_SCHEDULE,
    PARTY_CRATE_CHANCE, PARTY_GORDO_SCHEDULE, PARTY_GORDO_TARGET_FOOD, RARE_ORNAMENTS,
    RARE_ORNAMENT_CHANCE,
};
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

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_math::Randoms;
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

use crate::ammo::AmmoController;
use crate::story::{PediaDirector, TutorialDirector};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DlcStepEvents {
    pub wilds: WildsStepEvents,
    pub valley: ValleyStepEvents,
    pub slimulations: SlimulationsStepEvents,
}

/// Unified Step 9 runtime coordinating Ogden's Wilds, Mochi's Nimble Valley,
/// Viktor's Slimulation, Holiday events, and Secret Style appearances.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DlcRuntime {
    pub wilds: WildsSystem,
    pub valley: ValleySystem,
    pub slimulations: SlimulationsSystem,
    pub holiday: HolidayDirector,
    pub appearances: SlimeAppearanceDirector,
}

impl DlcRuntime {
    pub fn step(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        physics: &mut PhysicsWorld,
        ammo: &mut AmmoController,
        pedia: &mut PediaDirector,
        tutorials: &mut TutorialDirector,
        rng: &mut Randoms,
    ) -> DlcStepEvents {
        let wilds = self.wilds.step(model, physics, rng);
        let valley = self.valley.step(model, mode, physics, ammo, pedia, rng);
        let slimulations = self
            .slimulations
            .step(model, mode, physics, tutorials, rng);
        DlcStepEvents {
            wilds,
            valley,
            slimulations,
        }
    }
}
