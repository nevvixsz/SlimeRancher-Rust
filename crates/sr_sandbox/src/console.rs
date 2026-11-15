//! Developer Console (`Shift + Ctrl`) for fast spawning, upgrades, time control, and testing.

use sr_gameplay::{AmmoController, PlayerController};
use sr_ids::{IdentifiableId, PlayerUpgrade, RegionSetId};
use sr_math::Vec3;
use sr_physics::PlayerMoveInput;
use sr_gameplay::VacInput;
use sr_sim::{Simulation, TimeDirector};

/// Developer console controller toggled via `Shift + Ctrl`.
#[derive(Debug, Clone, Default)]
pub struct DevConsole {
    pub is_open: bool,
    pub history: Vec<String>,
    pub output_log: Vec<String>,
    prev_chord_down: bool,
}

impl DevConsole {
    /// Edge-triggered `Shift + Ctrl` hotkey handler. Returns `true` if console toggled state.
    pub fn handle_hotkey(&mut self, shift_held: bool, ctrl_held: bool) -> bool {
        let chord_down = shift_held && ctrl_held;
        let toggled = chord_down && !self.prev_chord_down;
        self.prev_chord_down = chord_down;
        if toggled {
            self.is_open = !self.is_open;
            self.output_log.push(if self.is_open {
                "[DevConsole] OPENED (Shift+Ctrl)".into()
            } else {
                "[DevConsole] CLOSED (Shift+Ctrl)".into()
            });
        }
        toggled
    }

    /// Executes a console command line against `Simulation` and returns output text.
    pub fn execute(&mut self, sim: &mut Simulation, raw_line: &str) -> String {
        let line = raw_line.trim();
        if line.is_empty() {
            return String::new();
        }
        self.history.push(line.to_string());
        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_ascii_lowercase();

        let resp = match cmd.as_str() {
            "help" | "?" => Self::help_text(),
            "spawn" => self.cmd_spawn(sim, &parts[1..]),
            "give" => self.cmd_give(sim, &parts[1..]),
            "money" | "newbucks" => self.cmd_money(sim, &parts[1..]),
            "upgrade" => self.cmd_upgrade(sim, &parts[1..]),
            "time" => self.cmd_time(sim, &parts[1..]),
            "tp" => self.cmd_tp(sim, &parts[1..]),
            "slot" => self.cmd_slot(sim, &parts[1..]),
            "vac" => self.cmd_vac(sim, &parts[1..]),
            "shoot" => self.cmd_shoot(sim),
            "airburst" => self.cmd_airburst(sim),
            "jump" => self.cmd_jump(sim),
            "move" => self.cmd_move(sim, &parts[1..]),
            "step" => self.cmd_step(sim, &parts[1..]),
            "clear" => self.cmd_clear(sim),
            "stats" => Self::cmd_stats(sim),
            _ => format!("Unknown command '{}'. Type 'help' for commands.", parts[0]),
        };

        self.output_log.push(format!("> {line}\n{resp}"));
        resp
    }

    fn help_text() -> String {
        [
            "DevConsole Commands (toggle with Shift + Ctrl):",
            "  spawn <ident_id> [count]   - Spawn actor(s) in front of player (e.g. spawn pink_slime 5)",
            "  give <ident_id> [count]    - Add item(s) straight into vacpack slots",
            "  money <amount>             - Add Newbucks (or negative to subtract)",
            "  upgrade <name|all>         - Grant player upgrade (jetpack, liquid_slot, air_burst, ammo_4, all)",
            "  time <hour|+hours|day N>   - Set hour (0..24), advance +N hours, or jump to day N",
            "  tp <x> <y> <z>             - Teleport player",
            "  slot <0..4>                - Select vacpack slot",
            "  vac [ticks]                - Hold vac beam for N ticks (default 15)",
            "  shoot                      - Fire selected vacpack slot / held object",
            "  airburst                   - Trigger Air Burst pulse",
            "  jump                       - Jump once and step 0.25s",
            "  move <x> <z> [secs]        - Move along local axes for duration",
            "  step <secs>                - Advance simulation by <secs>",
            "  clear                      - Remove all spawned actors",
            "  stats                      - Show player vitals, time, vacpack slots, actor count",
        ]
        .join("\n")
    }

    fn cmd_spawn(&self, sim: &mut Simulation, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: spawn <identifiable_id> [count]".into();
        }
        let Some(ident) = IdentifiableId::from_name(args[0]) else {
            return format!("Unknown IdentifiableId: '{}'", args[0]);
        };
        let count: usize = args
            .get(1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
            .clamp(1, 200);
        let fwd = sim.motor.look_dir();
        let right = sim.motor.right_xz();
        let base = sim.model.player.position + fwd * 3.0 + Vec3::Y * 1.0;

        for i in 0..count {
            let offset = right * (((i % 5) as f32 - 2.0) * 0.8) + Vec3::Y * ((i / 5) as f32 * 0.9);
            sim.spawn_actor(ident, RegionSetId::Home, base + offset, Vec3::ZERO);
        }
        format!("Spawned {count}x {}", ident.as_csharp_name())
    }

    fn cmd_give(&self, sim: &mut Simulation, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: give <identifiable_id> [count]".into();
        }
        let Some(ident) = IdentifiableId::from_name(args[0]) else {
            return format!("Unknown IdentifiableId: '{}'", args[0]);
        };
        let count: i32 = args
            .get(1)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1)
            .clamp(1, 100);
        let world_time = sim.model.world.world_time;
        let mut added = 0;
        for _ in 0..count {
            if sim
                .ammo
                .maybe_add_to_slot(&mut sim.model.player, ident, None, 1, world_time)
            {
                added += 1;
            } else {
                break;
            }
        }
        format!("Added {added}/{count}x {} to vacpack", ident.as_csharp_name())
    }

    fn cmd_money(&self, sim: &mut Simulation, args: &[&str]) -> String {
        let Some(amount) = args.first().and_then(|s| s.parse::<i32>().ok()) else {
            return "Usage: money <amount>".into();
        };
        PlayerController::add_currency(&mut sim.model.player, amount);
        format!("Currency is now {} Newbucks", sim.model.player.currency)
    }

    fn cmd_upgrade(&self, sim: &mut Simulation, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: upgrade <upgrade_name|all>".into();
        }
        let wt = sim.model.world.world_time;
        if args[0].eq_ignore_ascii_case("all") {
            for &u in PlayerUpgrade::ALL {
                PlayerController::grant_upgrade(&mut sim.model.player, u, wt);
            }
            return "Granted ALL player upgrades".into();
        }
        let Some(u) = PlayerUpgrade::from_name(args[0]) else {
            return format!("Unknown PlayerUpgrade: '{}'", args[0]);
        };
        let newly = PlayerController::grant_upgrade(&mut sim.model.player, u, wt);
        if newly {
            format!("Granted upgrade {u:?}")
        } else {
            format!("Upgrade {u:?} already owned")
        }
    }

    fn cmd_time(&self, sim: &mut Simulation, args: &[&str]) -> String {
        if args.is_empty() {
            return format!(
                "Day {} ({})",
                TimeDirector::curr_day(&sim.model.world),
                TimeDirector::format_time(TimeDirector::curr_time_minutes(&sim.model.world))
            );
        }
        if args[0].eq_ignore_ascii_case("day") {
            let day: i32 = args
                .get(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(1)
                .max(1);
            sim.model.world.world_time = (day - 1) as f64 * 86400.0 + 9.0 * 3600.0;
            sim.model.world.last_world_time = Some(sim.model.world.world_time);
        } else if let Some(rest) = args[0].strip_prefix('+') {
            let Ok(h) = rest.parse::<f64>() else {
                return "Usage: time <hour|+hours|day N>".into();
            };
            sim.model.world.world_time += h * 3600.0;
            sim.model.world.last_world_time = Some(sim.model.world.world_time);
        } else if let Ok(h) = args[0].parse::<f64>() {
            let day_base = (TimeDirector::curr_day(&sim.model.world) - 1) as f64 * 86400.0;
            sim.model.world.world_time = day_base + h.clamp(0.0, 23.99) * 3600.0;
            sim.model.world.last_world_time = Some(sim.model.world.world_time);
        } else {
            return "Usage: time <hour|+hours|day N>".into();
        }
        format!(
            "Time set to Day {} ({})",
            TimeDirector::curr_day(&sim.model.world),
            TimeDirector::format_time(TimeDirector::curr_time_minutes(&sim.model.world))
        )
    }

    fn cmd_tp(&self, sim: &mut Simulation, args: &[&str]) -> String {
        if args.len() < 3 {
            return "Usage: tp <x> <y> <z>".into();
        }
        let (Ok(x), Ok(y), Ok(z)) = (
            args[0].parse::<f32>(),
            args[1].parse::<f32>(),
            args[2].parse::<f32>(),
        ) else {
            return "Usage: tp <x> <y> <z>".into();
        };
        sim.model.player.position = Vec3::new(x, y, z);
        sim.motor.stop();
        format!("Teleported to ({x:.2}, {y:.2}, {z:.2})")
    }

    fn cmd_slot(&self, sim: &mut Simulation, args: &[&str]) -> String {
        let Some(idx) = args.first().and_then(|s| s.parse::<usize>().ok()) else {
            return "Usage: slot <0..4>".into();
        };
        if sim.ammo.select_slot(&sim.model.player, idx) || sim.ammo.selected_idx == idx {
            format!("Selected vacpack slot {idx}")
        } else {
            format!(
                "Slot {idx} unavailable (usable slots: {})",
                AmmoController::usable_slot_count(&sim.model.player)
            )
        }
    }

    fn cmd_vac(&self, sim: &mut Simulation, args: &[&str]) -> String {
        let ticks: usize = args
            .first()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(15)
            .clamp(1, 300);
        let mut total = 0;
        let mut held = None;
        for _ in 0..ticks {
            let ev = sim.step_with_input(
                1.0 / 60.0,
                PlayerMoveInput {
                    yaw: sim.motor.yaw,
                    pitch: sim.motor.pitch,
                    ..Default::default()
                },
                VacInput {
                    vac_held: true,
                    ..Default::default()
                },
            );
            total += ev.vac.captured.len();
            if ev.vac.held_actor.is_some() {
                held = ev.vac.held_actor;
            }
        }
        format!("Vac ran {ticks} ticks: captured {total} item(s), held={held:?}")
    }

    fn cmd_shoot(&self, sim: &mut Simulation) -> String {
        sim.vacuum.elapsed_time += 0.3;
        let ev = sim.step_with_input(
            1.0 / 60.0,
            PlayerMoveInput {
                yaw: sim.motor.yaw,
                pitch: sim.motor.pitch,
                ..Default::default()
            },
            VacInput {
                shoot_held: true,
                ..Default::default()
            },
        );
        if ev.vac.expelled.is_empty() {
            "Nothing to shoot in selected slot".into()
        } else {
            format!("Shot {:?}", ev.vac.expelled)
        }
    }

    fn cmd_airburst(&self, sim: &mut Simulation) -> String {
        let ev = sim.step_with_input(
            1.0 / 60.0,
            PlayerMoveInput {
                yaw: sim.motor.yaw,
                pitch: sim.motor.pitch,
                ..Default::default()
            },
            VacInput {
                burst_pressed: true,
                ..Default::default()
            },
        );
        if ev.vac.air_burst_fired {
            format!("AirBurst fired! Energy remaining: {:.1}", sim.model.player.curr_energy)
        } else if ev.vac.air_burst_no_energy {
            "AirBurst failed: not enough energy".into()
        } else {
            "AirBurst not unlocked (run 'upgrade air_burst')".into()
        }
    }

    fn cmd_jump(&self, sim: &mut Simulation) -> String {
        sim.step_with_input(
            1.0 / 60.0,
            PlayerMoveInput {
                yaw: sim.motor.yaw,
                pitch: sim.motor.pitch,
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            VacInput::default(),
        );
        for _ in 0..14 {
            sim.step_with_input(
                1.0 / 60.0,
                PlayerMoveInput {
                    yaw: sim.motor.yaw,
                    pitch: sim.motor.pitch,
                    jump_held: true,
                    ..Default::default()
                },
                VacInput::default(),
            );
        }
        format!("Jumped -> pos={:?}, vel={:?}", sim.model.player.position, sim.motor.velocity)
    }

    fn cmd_move(&self, sim: &mut Simulation, args: &[&str]) -> String {
        let x: f32 = args.first().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let z: f32 = args.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
        let secs: f32 = args
            .get(2)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.05, 30.0);
        let steps = (secs * 60.0).round() as usize;
        for _ in 0..steps {
            sim.step_with_input(
                1.0 / 60.0,
                PlayerMoveInput {
                    move_axes: [x, z],
                    yaw: sim.motor.yaw,
                    pitch: sim.motor.pitch,
                    ..Default::default()
                },
                VacInput::default(),
            );
        }
        format!("Moved ({x}, {z}) for {secs:.2}s -> pos={:?}", sim.model.player.position)
    }

    fn cmd_step(&self, sim: &mut Simulation, args: &[&str]) -> String {
        let secs: f32 = args
            .first()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.01, 120.0);
        let steps = (secs * 60.0).round().max(1.0) as usize;
        for _ in 0..steps {
            sim.step(1.0 / 60.0);
        }
        format!(
            "Stepped {secs:.2}s -> Day {} ({}), actors={}",
            TimeDirector::curr_day(&sim.model.world),
            TimeDirector::format_time(TimeDirector::curr_time_minutes(&sim.model.world)),
            sim.model.actors.len()
        )
    }

    fn cmd_clear(&self, sim: &mut Simulation) -> String {
        let count = sim.model.actors.len();
        sim.vacuum.drop_all_vacced(&mut sim.physics);
        sim.model.actors.clear();
        sim.model.actors_by_ident.clear();
        sim.physics.bodies.clear();
        format!("Cleared {count} actors")
    }

    fn cmd_stats(sim: &Simulation) -> String {
        let p = &sim.model.player;
        let wt = sim.model.world.world_time;
        let usable = AmmoController::usable_slot_count(&sim.model.player);
        let mut slots = Vec::new();
        for i in 0..usable {
            let id = sim.ammo.slot_id(p, i, wt);
            let cnt = sim.ammo.slot_count(p, i);
            let sel = if i == sim.ammo.selected_idx { "*" } else { "" };
            slots.push(format!("[{sel}{i}:{}x{cnt}]", id.as_csharp_name()));
        }
        format!(
            "Day {} ({}) | Pos ({:.1}, {:.1}, {:.1}) | HP {:.0}/{} | EN {:.0}/{} | RAD {:.0}/{} | {} NB | Actors: {}\nVacpack: {}",
            TimeDirector::curr_day(&sim.model.world),
            TimeDirector::format_time(TimeDirector::curr_time_minutes(&sim.model.world)),
            p.position.x,
            p.position.y,
            p.position.z,
            p.curr_health,
            p.max_health,
            p.curr_energy,
            p.max_energy,
            p.curr_rads,
            p.max_rads,
            p.currency,
            sim.model.actors.len(),
            slots.join(" ")
        )
    }
}
