//! In-game HUD: crosshair, HP / Energy / Rads, Newbucks, day & clock, vacpack slots, and controls.

use bevy::prelude::*;
use sr_sim::TimeDirector;

use crate::assets::ClientAssets;
use crate::player_input::CameraLookState;
use crate::world_sync::SimResource;

#[derive(Component)]
pub struct HudTopLeftText;

#[derive(Component)]
pub struct HudBottomSlotsText;

#[derive(Component)]
pub struct HudToastText;

pub fn setup_hud(mut commands: Commands, assets: Res<ClientAssets>) {
    let font = assets.ui_font.clone();

    // Center crosshair
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(6.0),
            height: Val::Px(6.0),
            margin: UiRect {
                left: Val::Px(-3.0),
                top: Val::Px(-3.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
    ));

    // Top-left stats panel
    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(12.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.60)),
        HudTopLeftText,
    ));

    // Bottom vacpack slots bar
    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            bottom: Val::Px(14.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.65)),
        HudBottomSlotsText,
    ));

    // Toast / center prompt
    commands.spawn((
        Text::new(""),
        TextFont {
            font,
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.92, 0.45)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(150.0),
            ..default()
        },
        HudToastText,
    ));
}

pub fn update_hud(
    sim_res: Res<SimResource>,
    assets: Res<ClientAssets>,
    look: Res<CameraLookState>,
    mut top_q: Query<
        &mut Text,
        (
            With<HudTopLeftText>,
            Without<HudBottomSlotsText>,
            Without<HudToastText>,
        ),
    >,
    mut bot_q: Query<
        &mut Text,
        (
            With<HudBottomSlotsText>,
            Without<HudTopLeftText>,
            Without<HudToastText>,
        ),
    >,
    mut toast_q: Query<
        &mut Text,
        (
            With<HudToastText>,
            Without<HudTopLeftText>,
            Without<HudBottomSlotsText>,
        ),
    >,
) {
    let sim = &sim_res.sim;
    let p = &sim.model.player;
    let day = TimeDirector::curr_day(&sim.model.world);
    let hour_f = TimeDirector::curr_hour(&sim.model.world);
    let hh = hour_f.floor() as i32;
    let mm = ((hour_f.fract()) * 60.0).floor() as i32;

    let day_str = assets
        .active_loc()
        .format("ui.m.day", &[&day.to_string()]);
    let label_newbucks = assets
        .ui_text("ui.l.endgame.newbucks", "Newbucks:")
        .trim_end_matches(':');
    let (label_hp, label_en, label_rads) = match assets.language {
        crate::assets::Language::Ru => ("Здоровье", "Энергия", "Рады"),
        crate::assets::Language::En => ("HP", "Energy", "Rads"),
    };

    let tex_count = assets.manifest.textures.len();
    let aud_count = assets.manifest.audio.len();

    if let Ok(mut t) = top_q.get_single_mut() {
        t.0 = format!(
            "{day_str} | {hh:02}:{mm:02} | {label_newbucks}: {}\n\
             {label_hp}: {:.0}/{} | {label_en}: {:.0}/{} | {label_rads}: {:.0}/{}\n\
             Actors: {} | Pos: ({:.1}, {:.1}, {:.1}) | Local assets: {} tex / {} audio [{}]",
            p.currency,
            p.curr_health,
            p.max_health,
            p.curr_energy,
            p.max_energy,
            p.curr_rads,
            p.max_rads,
            sim.model.actors.len(),
            p.position.x,
            p.position.y,
            p.position.z,
            tex_count,
            aud_count,
            assets.language.code().to_uppercase(),
        );
    }

    if let Ok(mut b) = bot_q.get_single_mut() {
        let usable = sr_gameplay::AmmoController::usable_slot_count(p);
        let mut slots_line = String::new();
        for idx in 0..usable {
            let id = sim.ammo.slot_id(p, idx, sim.model.world.world_time);
            let cnt = sim.ammo.slot_count(p, idx);
            let name = assets.ident_name(id);
            let marker = if idx == sim.ammo.selected_idx { ">" } else { " " };
            if idx > 0 {
                slots_line.push_str("  |  ");
            }
            slots_line.push_str(&format!("{marker}[{}] {} x{}", idx + 1, name, cnt));
        }

        let hint = if look.cursor_locked {
            "RMB: Vac | LMB: Shoot | Q: Pulse | 1-4/Wheel: Slot | F1: RU/EN | F2: Spawn | F5/F9: Save/Load | Esc: Unlock"
        } else {
            "Click window to lock mouse | WASD + Space + Shift | F1: RU/EN | F2: Spawn | F5/F9: Save/Load"
        };
        b.0 = format!("{slots_line}\n{hint}");
    }

    if let Ok(mut toast) = toast_q.get_single_mut() {
        toast.0 = sim_res
            .toast
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default();
    }
}
