//! Player mouse/keyboard input, FPS camera, animated vacpack beam & shockwave,
//! look-at raycast targeting, quick save/load, time skip, plot upgrades, and SFX playback.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use sr_gameplay::{
    build_market_listings, upgrade_purchase_cost, PlotOpError, RanchDirector, VacInput,
    WeaponVacuum,
};
use sr_ids::{IdentifiableId, LandPlotUpgrade, RegionSetId};
use sr_model::ActorKind;
use sr_persist::GameV12;
use sr_physics::PlayerMoveInput;
use sr_sandbox::landmarks;
use sr_sim::TimeDirector;

use crate::assets::{play_sfx, ClientAssets, Language};
use crate::world_sync::{
    ident_color, AirBurstVisual, ShootFlashVisual, SimResource, TargetInfo, VacBeamVisual,
    VacMuzzleVisual, VisualRegistry,
};

#[derive(Resource)]
pub struct CameraLookState {
    pub yaw: f32,
    pub pitch: f32,
    pub cursor_locked: bool,
    pub spawn_cycle: usize,
    pub vac_active: bool,
    pub recoil_timer: f32,
    pub flash_timer: f32,
    pub burst_timer: f32,
    pub burst_origin: Vec3,
}

impl Default for CameraLookState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.08,
            cursor_locked: false,
            spawn_cycle: 0,
            vac_active: false,
            recoil_timer: 0.0,
            flash_timer: 0.0,
            burst_timer: 0.0,
            burst_origin: Vec3::ZERO,
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

fn ray_sphere_dist(origin: Vec3, dir: Vec3, center: Vec3, radius: f32, max_dist: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.length_squared() - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    let hit_t = if t >= 0.0 { t } else { -b + disc.sqrt() };
    if hit_t >= 0.0 && hit_t <= max_dist {
        Some(hit_t)
    } else {
        None
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
    mut target_info: ResMut<TargetInfo>,
    mut assets: ResMut<ClientAssets>,
) {
    let dt = time.delta_secs().min(0.05);

    // Animation timers
    look.recoil_timer = (look.recoil_timer - dt).max(0.0);
    look.flash_timer = (look.flash_timer - dt).max(0.0);
    look.burst_timer = (look.burst_timer - dt).max(0.0);

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
        let label = match assets.language.code() {
            "ru" => "Язык: Русский (RU)",
            _ => "Language: English (EN)",
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
            IdentifiableId::RadSlime,
            IdentifiableId::CrystalSlime,
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

    // T: fast-forward +3 game hours (for testing day/night, Phosphor slimes, and midnight market refresh)
    if keys.just_pressed(KeyCode::KeyT) {
        let until = TimeDirector::hours_from_now(&sim_res.sim.model.world, 3.0);
        TimeDirector::fast_forward_to(&mut sim_res.sim.model.world, until);
        let msg = match assets.language {
            Language::Ru => "Ускорение времени: +3 ч",
            Language::En => "Fast-forward: +3 hours",
        };
        sim_res.set_toast(msg);
    }

    // U: buy next upgrade on nearby/targeted Ranch plot
    if keys.just_pressed(KeyCode::KeyU) {
        if let Some(plot_id) = target_info.nearby_plot_id.clone() {
            let candidates: &[LandPlotUpgrade] = if plot_id == "sandbox_corral" {
                &[
                    LandPlotUpgrade::Walls,
                    LandPlotUpgrade::MusicBox,
                    LandPlotUpgrade::AirNet,
                    LandPlotUpgrade::SolarShield,
                    LandPlotUpgrade::Feeder,
                    LandPlotUpgrade::PlortCollector,
                ]
            } else if plot_id == "sandbox_garden" {
                &[
                    LandPlotUpgrade::Soil,
                    LandPlotUpgrade::Sprinkler,
                    LandPlotUpgrade::Scareslime,
                ]
            } else {
                &[]
            };

            let next_upgrade = sim_res.sim.model.land_plots.get(&plot_id).and_then(|plot| {
                candidates
                    .iter()
                    .copied()
                    .find(|u| !plot.has_upgrade(*u))
                    .map(|u| (plot.type_id, u))
            });

            if let Some((plot_type, upg)) = next_upgrade {
                let cost = upgrade_purchase_cost(plot_type, upg).unwrap_or(0);
                match RanchDirector::buy_upgrade(&mut sim_res.sim.model, &plot_id, upg) {
                    Ok(()) => {
                        sim_res.set_toast(format!("+ {upg:?} (-{cost} NB)"));
                    }
                    Err(PlotOpError::InsufficientFunds) => {
                        let msg = match assets.language {
                            Language::Ru => format!("Нужно {cost} NB для {upg:?}"),
                            Language::En => format!("Need {cost} NB for {upg:?}"),
                        };
                        sim_res.set_toast(msg);
                    }
                    Err(err) => {
                        sim_res.set_toast(format!("Upgrade error: {err:?}"));
                    }
                }
            } else {
                let msg = match assets.language {
                    Language::Ru => "Все улучшения уже установлены",
                    Language::En => "All upgrades already installed",
                };
                sim_res.set_toast(msg);
            }
        }
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

    // Sound & visual FX triggers from simulation events
    if !events.vac.captured.is_empty() {
        play_sfx(&mut commands, assets.sfx_vac_capture.as_ref(), 0.65);
    }
    if !events.vac.expelled.is_empty() {
        play_sfx(&mut commands, assets.sfx_vac_shoot.as_ref(), 0.65);
        look.recoil_timer = 0.14;
        look.flash_timer = 0.09;
    }
    if events.vac.air_burst_fired {
        play_sfx(&mut commands, assets.sfx_airburst.as_ref(), 0.70);
        look.burst_timer = 0.38;
        look.burst_origin =
            WeaponVacuum::muzzle_origin(&sim_res.sim.model, &sim_res.sim.motor);
    }
    if !events.economy.market_deposits.is_empty() {
        play_sfx(&mut commands, assets.sfx_plort_sell.as_ref(), 0.75);
        let gained: i32 = events
            .economy
            .market_deposits
            .iter()
            .map(|(_, d)| d.currency)
            .sum();
        if gained > 0 {
            sim_res.set_toast(format!("+{gained} NB"));
        }
    }
    if events.fast_forward_finished {
        let msg = match assets.language {
            Language::Ru => "Время перемотано",
            Language::En => "Time skip complete",
        };
        sim_res.set_toast(msg);
    }

    look.vac_active = vac_input.vac_held;
    sim_res.last_events = events;

    // Look-at target raycast + nearby plot check for HUD prompt
    let eye = sim_res.sim.model.player.position + Vec3::Y * 1.55;
    let look_dir = sim_res.sim.motor.look_dir();
    update_target_info(&sim_res, &assets, eye, look_dir, &mut target_info);
}

#[allow(clippy::too_many_arguments)]
pub fn sync_vacpack_visuals_system(
    time: Res<Time>,
    look: Res<CameraLookState>,
    sim_res: Res<SimResource>,
    registry: Res<VisualRegistry>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cam_q: Query<
        &mut Transform,
        (
            With<Camera3d>,
            Without<VacMuzzleVisual>,
            Without<VacBeamVisual>,
            Without<ShootFlashVisual>,
            Without<AirBurstVisual>,
        ),
    >,
    mut muzzle_q: Query<
        &mut Transform,
        (
            With<VacMuzzleVisual>,
            Without<Camera3d>,
            Without<VacBeamVisual>,
            Without<ShootFlashVisual>,
            Without<AirBurstVisual>,
        ),
    >,
    mut beam_q: Query<
        (&mut Transform, &mut Visibility),
        (
            With<VacBeamVisual>,
            Without<Camera3d>,
            Without<VacMuzzleVisual>,
            Without<ShootFlashVisual>,
            Without<AirBurstVisual>,
        ),
    >,
    mut flash_q: Query<
        (&mut Transform, &mut Visibility),
        (
            With<ShootFlashVisual>,
            Without<Camera3d>,
            Without<VacMuzzleVisual>,
            Without<VacBeamVisual>,
            Without<AirBurstVisual>,
        ),
    >,
    mut burst_q: Query<
        (&mut Transform, &mut Visibility),
        (
            With<AirBurstVisual>,
            Without<Camera3d>,
            Without<VacMuzzleVisual>,
            Without<VacBeamVisual>,
            Without<ShootFlashVisual>,
        ),
    >,
) {
    let elapsed = time.elapsed_secs();

    // Update vacpack tank core material color to active slot's item color
    let active_slot_id = sim_res.sim.ammo.slot_id(
        &sim_res.sim.model.player,
        sim_res.sim.ammo.selected_idx,
        sim_res.sim.model.world.world_time,
    );
    if let Some(mat) = materials.get_mut(&registry.vac_core_material) {
        if active_slot_id == IdentifiableId::None {
            mat.base_color = Color::srgb(0.28, 0.80, 0.96);
            mat.emissive = LinearRgba::rgb(0.12, 0.45, 0.75);
        } else {
            let c = ident_color(active_slot_id, sim_res.world_dump.as_ref());
            mat.base_color = c;
            let lin = c.to_linear();
            mat.emissive = LinearRgba::rgb(lin.red * 0.45, lin.green * 0.45, lin.blue * 0.45);
        }
    }

    // Sync FPS camera transform
    let eye = sim_res.sim.model.player.position + Vec3::Y * 1.55;
    let look_dir = sim_res.sim.motor.look_dir();
    for mut cam_tf in &mut cam_q {
        *cam_tf = Transform::from_translation(eye).looking_to(look_dir, Vec3::Y);
    }

    // Sync Vacpack muzzle with shoot recoil & vac hum vibration
    let muzzle = WeaponVacuum::muzzle_origin(&sim_res.sim.model, &sim_res.sim.motor);
    let recoil_offset = look_dir * (-0.12 * (look.recoil_timer / 0.14));
    let jitter = if look.vac_active {
        Vec3::new(
            (elapsed * 48.0).sin() * 0.008,
            (elapsed * 55.0).cos() * 0.008,
            0.0,
        )
    } else {
        Vec3::ZERO
    };
    let muzzle_pos = muzzle + recoil_offset + jitter;
    let muzzle_rot = Quat::from_rotation_arc(Vec3::Y, look_dir);
    for mut m_tf in &mut muzzle_q {
        *m_tf = Transform {
            translation: muzzle_pos,
            rotation: muzzle_rot,
            scale: Vec3::ONE,
        };
    }

    // Animated vacuum cone beam (cone apex at muzzle, base extending 6m outward)
    for (mut b_tf, mut vis) in &mut beam_q {
        if look.vac_active {
            *vis = Visibility::Visible;
            let pulse = 0.88 + 0.16 * (elapsed * 18.0).sin();
            let spin = Quat::from_axis_angle(look_dir, elapsed * 7.0);
            let rot = spin * Quat::from_rotation_arc(-Vec3::Y, look_dir);
            *b_tf = Transform {
                translation: muzzle_pos + look_dir * 3.15,
                rotation: rot,
                scale: Vec3::new(pulse, 1.0, pulse),
            };
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Shoot muzzle flash
    for (mut f_tf, mut vis) in &mut flash_q {
        if look.flash_timer > 0.0 {
            *vis = Visibility::Visible;
            let s = 0.7 + 0.8 * (look.flash_timer / 0.09);
            *f_tf = Transform::from_translation(muzzle_pos + look_dir * 0.42)
                .with_scale(Vec3::splat(s));
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Expanding AirBurst shockwave torus
    for (mut a_tf, mut vis) in &mut burst_q {
        if look.burst_timer > 0.0 {
            *vis = Visibility::Visible;
            let progress = 1.0 - (look.burst_timer / 0.38);
            let radius = 0.6 + progress * 10.0;
            *a_tf = Transform {
                translation: look.burst_origin + look_dir * (progress * 2.5),
                rotation: Quat::from_rotation_arc(Vec3::Z, look_dir),
                scale: Vec3::new(radius, radius, 1.2),
            };
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn update_target_info(
    sim_res: &SimResource,
    assets: &ClientAssets,
    eye: Vec3,
    look_dir: Vec3,
    target_info: &mut TargetInfo,
) {
    let sim = &sim_res.sim;
    let mut best_dist = 14.0_f32;
    let mut title = String::new();
    let mut subtitle = String::new();

    // 1. Check actors
    for actor in sim.model.actors.values() {
        let rad = WeaponVacuum::radius_of(actor.ident) * 1.2;
        if let Some(d) = ray_sphere_dist(eye, look_dir, actor.pos, rad, best_dist) {
            best_dist = d;
            title = assets.ident_name(actor.ident);
            subtitle = if let ActorKind::Slime(ref s) = actor.kind {
                let hunger_pct = (s.hunger.curr_val * 100.0).round() as i32;
                let agi_pct = (s.agitation.curr_val * 100.0).round() as i32;
                match assets.language {
                    Language::Ru => {
                        if s.is_feral {
                            format!("Дикий! | Голод: {hunger_pct}%")
                        } else {
                            format!("Голод: {hunger_pct}% | Волнение: {agi_pct}%")
                        }
                    }
                    Language::En => {
                        if s.is_feral {
                            format!("Feral! | Hunger: {hunger_pct}%")
                        } else {
                            format!("Hunger: {hunger_pct}% | Agitation: {agi_pct}%")
                        }
                    }
                }
            } else if actor.ident.is_plort() {
                let price = sim
                    .economy
                    .market
                    .curr_values
                    .get(&actor.ident)
                    .map(|e| e.curr_value)
                    .unwrap_or(0.0);
                if price > 0.0 {
                    format!("{:.0} NB", price.round())
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
        }
    }

    // 2. Check Gordos
    for g in &sim.ai.world_gordos {
        if let Some(gm) = sim.model.gordos.get(&g.gordo_id) {
            if gm.gordo_eaten_count == -1 {
                continue;
            }
            if let Some(_d) = ray_sphere_dist(eye, look_dir, g.pos + Vec3::Y * 2.0, 2.3, best_dist)
            {
                let slime_name = assets.ident_name(g.slime_ident);
                title = match assets.language {
                    Language::Ru => format!("Гордо ({slime_name})"),
                    Language::En => format!("Gordo ({slime_name})"),
                };
                subtitle = format!("{} / {}", gm.gordo_eaten_count, gm.target_count);
            }
        }
    }

    // 3. Check Market or Ranch Plots by proximity
    let p_pos = sim.model.player.position;
    let mut nearby_plot_id = None;
    if p_pos.distance(landmarks::CORRAL_CENTER) < 9.5 {
        nearby_plot_id = Some("sandbox_corral".to_string());
    } else if p_pos.distance(landmarks::GARDEN_CENTER) < 9.5 {
        nearby_plot_id = Some("sandbox_garden".to_string());
    }

    if title.is_empty() {
        if p_pos.distance(landmarks::MARKET_CENTER) < 8.0 {
            let listings =
                build_market_listings(&sim.economy.market, &sim.model, &sim.mode_settings);
            let preview: String = listings
                .iter()
                .take(4)
                .map(|l| format!("{}: {} NB", assets.ident_name(l.id), l.curr_value))
                .collect::<Vec<_>>()
                .join(" | ");
            title = assets.ui_text("ui.t.market", "Plort Market").to_string();
            subtitle = preview;
        } else if let Some(ref pid) = nearby_plot_id {
            let count = sim
                .model
                .land_plots
                .get(pid)
                .map(|p| p.upgrades.len())
                .unwrap_or(0);
            if pid == "sandbox_corral" {
                title = assets.ui_text("ui.t.corral", "Corral").to_string();
                subtitle = match assets.language {
                    Language::Ru => format!("Улучшений: {count}/6 | [U] Улучшить"),
                    Language::En => format!("Upgrades: {count}/6 | [U] Upgrade"),
                };
            } else {
                title = assets.ui_text("ui.t.garden", "Garden").to_string();
                subtitle = match assets.language {
                    Language::Ru => format!("Улучшений: {count}/3 | [U] Улучшить"),
                    Language::En => format!("Upgrades: {count}/3 | [U] Upgrade"),
                };
            }
        } else if p_pos.distance(landmarks::POND_CENTER) < 7.0 {
            title = assets.ui_text("ui.t.pond", "Pond").to_string();
            subtitle = match assets.language {
                Language::Ru => "Источник воды (ПКМ для набора)".to_string(),
                Language::En => "Water source (RMB to vac water)".to_string(),
            };
        }
    }

    target_info.title = title;
    target_info.subtitle = subtitle;
    target_info.nearby_plot_id = nearby_plot_id;
}
