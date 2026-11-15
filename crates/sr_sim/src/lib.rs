//! Core Slime Rancher simulation loop, `TimeDirector`, and game-mode state stepping.

pub mod simulation;
pub mod time;

pub use simulation::{Simulation, TickEvents};
pub use time::{
    TimeDirector, TimeOfDayIcon, DEFAULT_FF_SECS_PER_GAME_DAY, DEFAULT_SECS_PER_GAME_DAY,
    SECS_PER_DAY, SECS_PER_HOUR, START_HOUR, START_WORLD_TIME,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_ids::GameMode;

    #[test]
    fn time_director_matches_csharp_constants_and_day_math() {
        let mut sim = Simulation::new(GameMode::Classic);
        assert_eq!(sim.model.world.world_time, 32400.0);
        assert_eq!(TimeDirector::curr_day(&sim.model.world), 1);
        assert!((TimeDirector::curr_hour(&sim.model.world) - 9.0).abs() < 1e-4);
        assert_eq!(TimeDirector::format_time(TimeDirector::curr_time_minutes(&sim.model.world)), "09:00");

        // 1 real second = 60 game seconds = 1 game minute
        let ev = sim.step(1.0);
        assert_eq!(sim.model.world.world_time, 32460.0);
        assert_eq!(ev.delta_world_time, 60.0);
        // After LateUpdate, last_world_time catches up to world_time
        assert_eq!(TimeDirector::delta_world_time(&sim.model.world), 0.0);

        // Next dawn (06:00) from 09:01 on day 1 should be 06:00 on day 2
        let dawn = TimeDirector::get_next_dawn(&sim.model.world);
        assert!((TimeDirector::day_fraction(dawn) - 0.25).abs() < 1e-6);
        assert!((dawn - (86400.0 + 6.0 * 3600.0)).abs() < 1.0);
    }

    #[test]
    fn fast_forward_sleep_and_midnight_hour_crossing() {
        let mut sim = Simulation::new(GameMode::Classic);
        // Jump to 23:58 on Day 1
        sim.model.world.world_time = 23.0 * 3600.0 + 58.0 * 60.0;
        sim.model.world.last_world_time = Some(sim.model.world.world_time);

        // Fast-forward to 06:00 Day 2
        let next_dawn = TimeDirector::get_next_dawn(&sim.model.world);
        TimeDirector::fast_forward_to(&mut sim.model.world, next_dawn);
        assert!(TimeDirector::is_fast_forwarding(&sim.model.world));

        // Step 2 real seconds at ff_time_factor (17280 game sec / real sec) -> reaches 06:00 Day 2
        let ev = sim.step(2.0);
        assert!(ev.fast_forward_finished);
        assert!(!TimeDirector::is_fast_forwarding(&sim.model.world));
        assert_eq!(TimeDirector::curr_day(&sim.model.world), 2);
    }

    #[test]
    fn on_passed_hour_handles_midnight_wrap() {
        let mut sim = Simulation::new(GameMode::Classic);
        sim.model.world.last_world_time = Some(23.5 * 3600.0);
        sim.model.world.world_time = 24.5 * 3600.0;
        assert!(TimeDirector::on_passed_hour(&sim.model.world, 0.0));
        assert!(!TimeDirector::on_passed_hour(&sim.model.world, 1.0));
    }

    #[test]
    fn pause_stack_and_rush_mode_game_over() {
        let mut sim = Simulation::new(GameMode::TimeLimitV2);
        sim.time.pause(false);
        sim.time.pause(false);
        let t0 = sim.model.world.world_time;
        sim.step(5.0);
        assert_eq!(sim.model.world.world_time, t0);

        sim.time.unpause(false);
        assert!(sim.time.is_paused());
        sim.time.unpause(false);
        assert!(!sim.time.is_paused());

        // Jump right before endGameTime and cross it
        let end = sim.model.player.end_game_time.unwrap();
        sim.model.world.world_time = end - 30.0;
        sim.model.world.last_world_time = Some(end - 30.0);
        let ev = sim.step(1.0);
        assert!(ev.game_over_triggered);
        assert!(sim.model.is_game_over());
    }
}
