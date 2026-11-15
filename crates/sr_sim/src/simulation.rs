//! Root simulation coordinator combining `DataRegistry`, `GameModel`, `GameModeSettings`,
//! `TimeDirector`, `PhysicsWorld`, `PlayerMotor`, `PlayerController`, `AmmoController`, and `WeaponVacuum`.

use sr_data::{DataRegistry, GameModeSettings};
use sr_gameplay::{
    spawn_physical_actor, step_actor_lifecycle, ActorLifecycleEvents, AmmoController, DeathOutcome,
    DeathSource, PlayerController, VacEvents, VacInput, WeaponVacuum,
};
use sr_ids::{GameMode, IdentifiableId, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::GameModel;
use sr_physics::{PhysicsWorld, PlayerMotor, PlayerMoveInput};

use crate::time::TimeDirector;

/// Events emitted by a single simulation tick.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TickEvents {
    pub fast_forward_finished: bool,
    pub game_over_triggered: bool,
    pub delta_world_time: f64,
    pub vac: VacEvents,
    pub lifecycle: ActorLifecycleEvents,
    pub player_death: Option<DeathOutcome>,
}

/// Top-level simulation instance encapsulating world state, time, physics, and gameplay systems.
#[derive(Debug, Clone)]
pub struct Simulation {
    pub data: DataRegistry,
    pub mode_settings: GameModeSettings,
    pub model: GameModel,
    pub time: TimeDirector,
    pub physics: PhysicsWorld,
    pub motor: PlayerMotor,
    pub player_ctrl: PlayerController,
    pub ammo: AmmoController,
    pub vacuum: WeaponVacuum,
    pub rng: Randoms,
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
            physics: PhysicsWorld::new(),
            motor: PlayerMotor::default(),
            player_ctrl: PlayerController::default(),
            ammo: AmmoController::default(),
            vacuum: WeaponVacuum::default(),
            rng: Randoms::default(),
            was_game_over: false,
        }
    }

    /// Spawns a physical actor into both `GameModel` and `PhysicsWorld`.
    pub fn spawn_actor(
        &mut self,
        ident: IdentifiableId,
        region: RegionSetId,
        pos: Vec3,
        vel: Vec3,
    ) -> i64 {
        spawn_physical_actor(&mut self.model, &mut self.physics, ident, region, pos, vel)
    }

    /// Advances simulation time and core systems by `dt` real seconds (no player input).
    pub fn step(&mut self, dt: f32) -> TickEvents {
        self.step_with_input(dt, PlayerMoveInput::default(), VacInput::default())
    }

    /// Full simulation tick including player motor, vacpack, physics, and lifecycle events.
    pub fn step_with_input(
        &mut self,
        dt: f32,
        move_input: PlayerMoveInput,
        vac_input: VacInput,
    ) -> TickEvents {
        let mut events = TickEvents::default();
        if self.time.is_paused() {
            return events;
        }

        if let Some(false) = self.time.update(&mut self.model.world, dt) {
            events.fast_forward_finished = true;
        }

        let scaled_dt = dt * self.time.time_scale;
        let world_time = self.model.world.world_time;

        // Player movement & jetpack
        self.motor
            .step(&mut self.model.player, &self.physics, move_input, scaled_dt, world_time);

        // Vacpack capture / shoot / airburst
        events.vac = self.vacuum.step(
            &mut self.model,
            &self.motor,
            &mut self.ammo,
            &mut self.physics,
            vac_input,
            scaled_dt,
        );

        // Physics world step + crate impact / out-of-bounds / water rules
        let (impacts, overlaps) = self.physics.step(scaled_dt);
        events.lifecycle = step_actor_lifecycle(
            &mut self.model,
            &mut self.physics,
            &impacts,
            &overlaps,
            &mut self.rng,
        );

        // Player recovery
        self.model
            .player
            .recover(scaled_dt, self.model.world.world_time);

        // Sea kill zone / zero health check
        if self.motor.in_sea_kill_zone || self.model.player.curr_health <= 0.0 {
            let src = if self.motor.in_sea_kill_zone {
                DeathSource::Water
            } else {
                DeathSource::Other
            };
            self.vacuum.drop_all_vacced(&mut self.physics);
            let mut wt = self.model.world.world_time;
            let outcome = self.player_ctrl.handle_death(
                &mut self.model.player,
                &mut self.motor,
                &self.mode_settings,
                src,
                &mut wt,
            );
            self.model.world.world_time = wt;
            events.player_death = Some(outcome);
        }

        if !self.was_game_over && self.model.is_game_over() {
            self.was_game_over = true;
            events.game_over_triggered = true;
        }

        events.delta_world_time = TimeDirector::delta_world_time(&self.model.world);
        self.time.late_update(&mut self.model.world);
        events
    }
}
