//! Root simulation coordinator combining `DataRegistry`, `GameModel`, `GameModeSettings`,
//! and `TimeDirector`.

use sr_data::{DataRegistry, GameModeSettings};
use sr_ids::GameMode;
use sr_model::GameModel;

use crate::time::TimeDirector;

/// Events emitted by a single simulation tick.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TickEvents {
    pub fast_forward_finished: bool,
    pub game_over_triggered: bool,
}

/// Top-level simulation instance encapsulating world state, time, and static game definitions.
#[derive(Debug, Clone)]
pub struct Simulation {
    pub data: DataRegistry,
    pub mode_settings: GameModeSettings,
    pub model: GameModel,
    pub time: TimeDirector,
    was_game_over: bool,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new(GameMode::Classic)
    }
}

impl Simulation {
    pub fn new(game_mode: GameMode) -> Self {
        let data = DataRegistry::builtin();
        let mode_settings = data
            .game_modes
            .get(&game_mode)
            .cloned()
            .unwrap_or_else(|| GameModeSettings::for_mode(game_mode));
        let mut model = GameModel::new(game_mode);
        let time = TimeDirector::default();
        time.init_world_model(&mut model.world);
        model.player.reset(&mode_settings);
        Self {
            data,
            mode_settings,
            model,
            time,
            was_game_over: false,
        }
    }

    /// Advances simulation time and core per-frame models by `dt` real seconds.
    pub fn step(&mut self, dt: f32) -> TickEvents {
        let mut events = TickEvents::default();
        if self.time.is_paused() {
            return events;
        }

        if let Some(false) = self.time.update(&mut self.model.world, dt) {
            events.fast_forward_finished = true;
        }

        // Player health / energy / rad recovery (active when not fast-forwarding or unpaused)
        self.model
            .player
            .recover(dt * self.time.time_scale, self.model.world.world_time);

        if !self.was_game_over && self.model.is_game_over() {
            self.was_game_over = true;
            events.game_over_triggered = true;
        }

        self.time.late_update(&mut self.model.world);
        events
    }
}
