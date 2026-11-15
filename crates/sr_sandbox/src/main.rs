//! Interactive CLI runner for `sr_sandbox` (test arena + `Shift + Ctrl` DevConsole).

use std::io::{self, BufRead, Write};

use sr_ids::GameMode;
use sr_sandbox::{render_ascii_map, setup_test_arena, DevConsole};
use sr_sim::Simulation;

fn main() {
    let mut sim = Simulation::new(GameMode::Classic);
    setup_test_arena(&mut sim);
    let mut console = DevConsole::default();
    // Simulate pressing Shift + Ctrl to open console on launch in CLI mode
    console.handle_hotkey(true, true);
    console.handle_hotkey(false, false);

    println!("=== SlimeRancher-Rust Sandbox Arena ===");
    println!("{}", console.execute(&mut sim, "stats"));
    println!("{}", render_ascii_map(&sim));
    println!("Type 'help' for DevConsole commands, 'map' to view radar, or 'quit' to exit.");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("exit") {
            break;
        }
        if trimmed.eq_ignore_ascii_case("map") {
            println!("{}", render_ascii_map(&sim));
        } else if trimmed.eq_ignore_ascii_case("shift+ctrl") {
            console.handle_hotkey(true, true);
            console.handle_hotkey(false, false);
            println!("Console open = {}", console.is_open);
        } else if !trimmed.is_empty() {
            let out = console.execute(&mut sim, trimmed);
            println!("{out}");
        }
        print!("sr> ");
        let _ = stdout.flush();
    }
}
