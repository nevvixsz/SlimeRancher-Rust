//! `sr_sandbox` — test arena ("пробная карта") + `Shift + Ctrl` developer console.

pub mod arena;
pub mod console;

pub use arena::{landmarks, render_ascii_map, setup_test_arena};
pub use console::DevConsole;

#[cfg(test)]
mod tests {
    use super::*;
    use sr_ids::{GameMode, IdentifiableId};
    use sr_sim::Simulation;

    #[test]
    fn dev_console_shift_ctrl_hotkey_and_commands_work() {
        let mut sim = Simulation::new(GameMode::Classic);
        setup_test_arena(&mut sim);
        let initial_actors = sim.model.actors.len();
        assert!(initial_actors >= 7);

        let mut console = DevConsole::default();
        assert!(!console.is_open);
        // Holding only Shift does not toggle
        assert!(!console.handle_hotkey(true, false));
        assert!(!console.is_open);
        // Pressing Shift + Ctrl toggles open (edge-triggered)
        assert!(console.handle_hotkey(true, true));
        assert!(console.is_open);
        // Holding Shift + Ctrl on next frame does not double-toggle
        assert!(!console.handle_hotkey(true, true));
        assert!(console.is_open);
        // Releasing & pressing again closes it
        assert!(!console.handle_hotkey(false, false));
        assert!(console.handle_hotkey(true, true));
        assert!(!console.is_open);

        // Spawn 4 PinkSlimes, vac them, upgrade AirBurst, shoot, and clear
        console.execute(&mut sim, "spawn pink_slime 4");
        assert_eq!(sim.model.actors.len(), initial_actors + 4);

        console.execute(&mut sim, "give carrot_veggie 3");
        assert_eq!(sim.ammo.slot_count(&sim.model.player, 0), 3);

        console.execute(&mut sim, "money 500");
        assert_eq!(sim.model.player.currency, 750);

        console.execute(&mut sim, "upgrade all");
        assert!(sim.model.player.has_jetpack);
        assert!(sim.model.player.has_air_burst);
        assert_eq!(sim.model.player.max_ammo, 100);

        console.execute(&mut sim, "shoot");
        assert_eq!(sim.ammo.slot_count(&sim.model.player, 0), 2);

        console.execute(&mut sim, "airburst");
        assert_eq!(sim.model.player.curr_energy, 80.0);

        console.execute(&mut sim, "clear");
        assert_eq!(sim.model.actors.len(), 0);
    }

    #[test]
    fn arena_sea_kill_and_respawn_integration() {
        let mut sim = Simulation::new(GameMode::Classic);
        setup_test_arena(&mut sim);
        let mut console = DevConsole::default();

        console.execute(&mut sim, "give pink_slime 2");
        assert_eq!(
            sim.ammo.slot_id(&sim.model.player, 0, sim.model.world.world_time),
            IdentifiableId::PinkSlime
        );

        // Teleport below sea kill height (-8.0) and step once
        console.execute(&mut sim, "tp 0 -10 -60");
        let ev = sim.step(1.0 / 60.0);
        assert!(ev.player_death.is_some());
        assert_eq!(sim.model.player.position, landmarks::PLAYER_SPAWN);
        assert_eq!(sim.ammo.slot_count(&sim.model.player, 0), 0);
    }
}
