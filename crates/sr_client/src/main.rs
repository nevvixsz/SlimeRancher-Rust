//! Entry point for `sr_client` — Bevy 3D client for SlimeRancher-Rust.

mod assets;
mod hud;
mod player_input;
mod ranch_visuals;
mod sky_lighting;
mod slime_visuals;
mod world_sync;

use std::path::PathBuf;

use bevy::pbr::PbrPlugin;
use bevy::prelude::*;

use assets::{find_workspace_root, load_client_assets, start_bgm_system, Language};
use hud::{setup_hud, update_hud};
use player_input::{
    handle_cursor_grab, step_simulation_system, sync_vacpack_visuals_system, CameraLookState,
};
use ranch_visuals::sync_ranch_visuals_system;
use sky_lighting::{setup_sky_and_light, update_sky_and_light};
use slime_visuals::update_slime_animations_system;
use world_sync::{
    build_initial_sim, setup_world_scene, sync_actors_and_gordos_system, TargetInfo,
    VisualRegistry,
};

struct CliArgs {
    save_path: Option<PathBuf>,
    lang: Language,
}

fn parse_cli() -> CliArgs {
    let mut save_path = None;
    let mut lang = Language::Ru;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--save" => {
                if let Some(p) = it.next() {
                    save_path = Some(PathBuf::from(p));
                }
            }
            "--lang" => {
                if let Some(l) = it.next() {
                    lang = if l.eq_ignore_ascii_case("en") {
                        Language::En
                    } else {
                        Language::Ru
                    };
                }
            }
            _ => {}
        }
    }
    CliArgs { save_path, lang }
}

fn bootstrap_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cli: NonSend<CliArgsResource>,
) {
    let client_assets = load_client_assets(&asset_server, cli.0.lang);
    let sim_res = build_initial_sim(cli.0.save_path.clone(), &client_assets);
    commands.insert_resource(client_assets);
    commands.insert_resource(sim_res);
}

struct CliArgsResource(CliArgs);

fn main() {
    let cli = parse_cli();
    let asset_root = find_workspace_root();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Slime Rancher (Rust + Bevy)".to_string(),
                        resolution: (1280.0_f32, 720.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(PbrPlugin {
                    // Keep forward PBR lean and fall back to CPU mesh uniform building so
                    // DX12 FL 11_0 GPUs (e.g. Fermi GTX 580) avoid unsupported compute/prepass/deferred pipelines.
                    prepass_enabled: false,
                    add_default_deferred_lighting_plugin: false,
                    use_gpu_instance_buffer_builder: std::env::var_os("SR_GPU_PREPROCESS")
                        .is_some(),
                }),
        )
        .insert_non_send_resource(CliArgsResource(cli))
        .init_resource::<CameraLookState>()
        .init_resource::<TargetInfo>()
        .init_resource::<VisualRegistry>()
        .add_systems(
            PreStartup,
            bootstrap_system,
        )
        .add_systems(
            Startup,
            (
                setup_sky_and_light,
                setup_world_scene,
                setup_hud,
                start_bgm_system,
            ),
        )
        .add_systems(
            Update,
            (
                handle_cursor_grab,
                step_simulation_system.after(handle_cursor_grab),
                sync_vacpack_visuals_system.after(step_simulation_system),
                sync_actors_and_gordos_system.after(step_simulation_system),
                update_slime_animations_system.after(sync_actors_and_gordos_system),
                sync_ranch_visuals_system.after(step_simulation_system),
                update_sky_and_light.after(step_simulation_system),
                update_hud.after(step_simulation_system),
            ),
        )
        .run();
}
