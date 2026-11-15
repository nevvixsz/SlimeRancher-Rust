//! Player mouse/keyboard input, FPS camera, vacpack beam, quick save/load, and SFX playback.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use sr_gameplay::{VacInput, WeaponVacuum};
use sr_ids::{IdentifiableId, RegionSetId};
use sr_persist::GameV12;
use sr_physics::PlayerMoveInput;

use crate::assets::{play_sfx, ClientAssets};
use crate::world_sync::{SimResource, VacBeamVisual, VacMuzzleVisual};

#[derive(Resource)]
pub struct CameraLookState {
    pub yaw: f32,
    pub pitch: f32,
    pub cursor_locked: bool,
    pub spawn_cycle: usize,
}

impl Default for CameraLookState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.08,
            cursor_locked: false,
            spawn_cycle: 0,
        }
    }
}

pub fn handle_cursor_grab(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut look: ResMut<CameraLookState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) && !look.cursor_locked {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
        look.cursor_locked = true;
    } else if keys.just_pressed(KeyCode::Escape) && look.cursor_locked {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        look.cursor_locked = false;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn step_simulation_system(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut wheel_evr: EventReader<MouseWheel>,
    mut look: ResMut<CameraLookState>,
    mut sim_res: ResMut<SimResource>,
    mut assets: ResMut<ClientAssets>,
    mut cam_q: Query<&mut Transform, (With<Camera3d>, Without<VacMuzzleVisual>, Without<VacBeamVisual>)>,
    mut muzzle_q: Query<
        &mut Transform,
        (With<VacMuzzleVisual>, Without<Camera3d>, Without<VacBeamVisual>),
    >,
    mut beam_q: Query<
        (&mut Transform, &mut Visibility),
        (With<VacBeamVisual>, Without<Camera3d>, Without<VacMuzzleVisual>),
    >,
) {
    let dt = time.delta_secs().min(0.05);

    // Toast timer
    if let Some((_, ref mut ttl)) = sim_res.toast {
        *ttl -= dt;
        if *ttl <= 0.0 {
            sim_res.toast = None;
        }
    }

    // F1: toggle RU / EN localization
    if keys.just_pressed(KeyCode::F1) {
        assets.language = assets.language.toggled();
        let label = match assets.language {
            crate::assets::Language::Ru => "Язык: Русский (RU)",
            crate::assets::Language::En => "Language: English (EN)",
        };
        sim_res.set_toast(label);
    }

    // F2: spawn a test slime or food right in front of the player
    if keys.just_pressed(KeyCode::F2) {
        let cycle = [
            IdentifiableId::PinkSlime,
            IdentifiableId::TabbySlime,
            IdentifiableId::RockSlime,
            IdentifiableId::PhosphorSlime,
            IdentifiableId::HoneySlime,
            IdentifiableId::CarrotVeggie,
            IdentifiableId::CuberryFruit,
        ];
        let ident = cycle[look.spawn_cycle % cycle.len()];
        look.spawn_cycle += 1;
        let look_dir = sim_res.sim.motor.look_dir();
        let spawn_pos = sim_res.sim.model.player.position + Vec3::Y * 1.4 + look_dir * 3.0;
        sim_res
            .sim
            .spawn_actor(ident, RegionSetId::Home, spawn_pos, look_dir * 2.0);
        let name = assets.ident_name(ident);
        sim_res.set_toast(format!("+ {name}"));
    }

    // F5: quick save to local_assets/saves/quicksave.sav
    if keys.just_pressed(KeyCode::F5) {
        let save_dir = assets
            .local_root
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("local_assets"))
            .join("saves");
        let _ = std::fs::create_dir_all(&save_dir);
        let save_file = save_dir.join("quicksave.sav");
        let v12 = GameV12::from_game_model(&sim_res.sim.model, "quicksave", "Quick Save", 1, 0);
        match v12.to_bytes().and_then(|b| Ok(std::fs::write(&save_file, b)?)) {
            Ok(()) => sim_res.set_toast(format!("Saved: {}", save_file.display())),
            Err(err) => sim_res.set_toast(format!("Save error: {err}")),
        }
    }

    // F9: quick load from local_assets/saves/quicksave.sav
    if keys.just_pressed(KeyCode::F9) {
        let save_file = assets
            .local_root
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("local_assets"))
            .join("saves")
            .join("quicksave.sav");
        if save_file.is_file() {
            match std::fs::read(&save_file)
                .map_err(sr_persist::PersistError::from)
                .and_then(|b| GameV12::from_bytes(&b))
                .and_then(|v12| v12.into_game_model())
            {
                Ok(model) => {
                    sim_res.sim.model = model;
                    sim_res.sim.physics.bodies.clear();
                    let actors: Vec<_> = sim_res
                        .sim
                        .model
                        .actors
                        .values()
                        .map(|a| (a.actor_id, a.pos, WeaponVacuum::radius_of(a.ident)))
                        .collect();
                    for (id, pos, rad) in actors {
                        sim_res.sim.physics.insert_body(sr_physics::RigidBody {
                            actor_id: id,
                            pos,
                            radius: rad,
                            ..Default::default()
                        });
                    }
                    sim_res.set_toast("Quick save loaded");
                }
                Err(err) => sim_res.set_toast(format!("Load error: {err}")),
            }
        } else {
            sim_res.set_toast("No quicksave.sav found (press F5 first)");
        }
    }

    // Camera look (when cursor is captured)
    if look.cursor_locked {
        for ev in motion_evr.read() {
            look.yaw -= ev.delta.x * 0.0024;
            look.pitch = (look.pitch - ev.delta.y * 0.0024).clamp(-1.45, 1.45);
        }
    } else {
        motion_evr.clear();
    }

    // Movement input aligned with PlayerMotor's basis
    let forward = (keys.pressed(KeyCode::KeyW) as i32 - keys.pressed(KeyCode::KeyS) as i32) as f32;
    let strafe = (keys.pressed(KeyCode::KeyA) as i32 - keys.pressed(KeyCode::KeyD) as i32) as f32;

    let move_input = PlayerMoveInput {
        move_axes: [strafe, forward],
        yaw: look.yaw,
        pitch: look.pitch,
        sprint: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
        jump_held: keys.pressed(KeyCode::Space),
        jump_pressed: keys.just_pressed(KeyCode::Space),
    };

    if move_input.jump_pressed && sim_res.sim.motor.grounded {
        play_sfx(&mut commands, assets.sfx_jump.as_ref(), 0.55);
    }

    // Slot switching (1..4 + scroll wheel)
    let mut select_slot = None;
    if keys.just_pressed(KeyCode::Digit1) {
        select_slot = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) {
        select_slot = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) {
        select_slot = Some(2);
    } else if keys.just_pressed(KeyCode::Digit4) {
        select_slot = Some(3);
    }

    let mut next_slot = false;
    let mut prev_slot = false;
    for wh in wheel_evr.read() {
        if wh.y > 0.0 {
            prev_slot = true;
        } else if wh.y < 0.0 {
            next_slot = true;
        }
    }

    let vac_input = VacInput {
        vac_held: look.cursor_locked && mouse_btn.pressed(MouseButton::Right),
        shoot_held: look.cursor_locked && mouse_btn.pressed(MouseButton::Left),
        burst_pressed: look.cursor_locked && keys.just_pressed(KeyCode::KeyQ),
        gadget_toggle_pressed: false,
        next_slot,
        prev_slot,
        select_slot,
    };

    let events = sim_res.sim.step_with_input(dt, move_input, vac_input);

    // Sound triggers from simulation events
    if !events.vac.captured.is_empty() {
        play_sfx(&mut commands, assets.sfx_vac_capture.as_ref(), 0.65);
    }
    if !events.vac.expelled.is_empty() {
        play_sfx(&mut commands, assets.sfx_vac_shoot.as_ref(), 0.65);
    }
    if events.vac.air_burst_fired {
        play_sfx(&mut commands, assets.sfx_airburst.as_ref(), 0.70);
    }
    if !events.economy.market_deposits.is_empty() {
        play_sfx(&mut commands, assets.sfx_plort_sell.as_ref(), 0.75);
    }

    sim_res.last_events = events;

    // Sync FPS camera transform
    let eye = sim_res.sim.model.player.position + Vec3::Y * 1.55;
    let look_dir = sim_res.sim.motor.look_dir();
    for mut cam_tf in &mut cam_q {
        *cam_tf = Transform::from_translation(eye).looking_to(look_dir, Vec3::Y);
    }

    // Sync Vacpack muzzle and beam visuals
    let muzzle = WeaponVacuum::muzzle_origin(&sim_res.sim.model, &sim_res.sim.motor);
    for mut m_tf in &mut muzzle_q {
        let rot = Quat::from_rotation_arc(Vec3::Y, look_dir);
        *m_tf = Transform {
            translation: muzzle,
            rotation: rot,
            scale: Vec3::ONE,
        };
    }

    for (mut b_tf, mut vis) in &mut beam_q {
        if vac_input.vac_held {
            *vis = Visibility::Visible;
            let rot = Quat::from_rotation_arc(Vec3::Y, look_dir);
            *b_tf = Transform {
                translation: muzzle + look_dir * 3.0,
                rotation: rot,
                scale: Vec3::ONE,
            };
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
