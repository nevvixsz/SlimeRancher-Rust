//! Slime Rancher style HUD: crosshair, look-at prompt, HP / Energy / Rads bars with original icons,
//! Newbucks, day & clock, visual Vacpack slots with `Texture2D` item icons, and dirty-checked updates.

use bevy::prelude::*;
use sr_ids::IdentifiableId;
use sr_sim::TimeDirector;

use crate::assets::{ClientAssets, Language};
use crate::player_input::CameraLookState;
use crate::world_sync::{SimResource, TargetInfo};

#[derive(Component)]
pub struct HudTopLeftText;

#[derive(Component)]
pub struct HudDayIcon;

#[derive(Component)]
pub struct HudHintText;

#[derive(Component)]
pub struct HudToastText;

#[derive(Component)]
pub struct HudTargetText;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum HudStatBar {
    Hp,
    Energy,
    Rads,
}

#[derive(Component)]
pub struct HudSlotBox {
    pub idx: usize,
}

#[derive(Component)]
pub struct HudSlotIcon {
    pub idx: usize,
}

#[derive(Component)]
pub struct HudSlotLabel {
    pub idx: usize,
}

#[derive(Default)]
pub struct HudCache {
    pub top_sig: (i32, i32, i32, i32, i32, i32, i32, usize, u8),
    pub hp_pct: i32,
    pub en_pct: i32,
    pub rad_pct: i32,
    pub target_title: String,
    pub target_sub: String,
    pub toast: String,
    pub cursor_locked: Option<bool>,
    pub lang_code: &'static str,
    pub selected_slot: usize,
    pub slots: [(IdentifiableId, i32); 4],
}

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
        BorderRadius::all(Val::Px(3.0)),
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.90)),
    ));

    // Look-at Target Prompt below crosshair
    commands.spawn((
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.98, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(54.0),
            margin: UiRect {
                left: Val::Px(-140.0),
                ..default()
            },
            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
            ..default()
        },
        BorderRadius::all(Val::Px(8.0)),
        BackgroundColor(Color::srgba(0.04, 0.07, 0.14, 0.0)),
        HudTargetText,
    ));

    // Top-left stats panel + HP / Energy / Rads bars
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(12.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BorderRadius::all(Val::Px(10.0)),
            BorderColor(Color::srgba(0.35, 0.80, 1.0, 0.45)),
            BackgroundColor(Color::srgba(0.05, 0.09, 0.16, 0.72)),
        ))
        .with_children(|panel| {
            // Optional original HUD status icons row (Sun/Moon, Newbucks, SlimeKey)
            if assets.icon_sun.is_some()
                || assets.icon_moon.is_some()
                || assets.icon_newbucks.is_some()
                || assets.icon_key.is_some()
            {
                panel
                    .spawn(Node {
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|icons| {
                        if let Some(ref sun_or_moon) = assets.icon_sun.as_ref().or(assets.icon_moon.as_ref()) {
                            icons.spawn((
                                ImageNode::new((*sun_or_moon).clone()),
                                Node {
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                                HudDayIcon,
                            ));
                        }
                        if let Some(ref nb) = assets.icon_newbucks {
                            icons.spawn((
                                ImageNode::new(nb.clone()),
                                Node {
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                            ));
                        }
                        if let Some(ref key_icon) = assets.icon_key {
                            icons.spawn((
                                ImageNode::new(key_icon.clone()),
                                Node {
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                            ));
                        }
                    });
            }

            panel.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                HudTopLeftText,
            ));

            spawn_stat_bar(
                panel,
                assets.icon_health.as_ref(),
                Color::srgba(0.15, 0.05, 0.08, 0.85),
                Color::srgb(0.92, 0.26, 0.36),
                HudStatBar::Hp,
                8.0,
                100.0,
            );
            spawn_stat_bar(
                panel,
                assets.icon_energy.as_ref(),
                Color::srgba(0.05, 0.12, 0.18, 0.85),
                Color::srgb(0.24, 0.78, 0.98),
                HudStatBar::Energy,
                8.0,
                100.0,
            );
            spawn_stat_bar(
                panel,
                assets.icon_rads.as_ref(),
                Color::srgba(0.06, 0.14, 0.06, 0.85),
                Color::srgb(0.32, 0.95, 0.38),
                HudStatBar::Rads,
                6.0,
                0.0,
            );
        });

    // Bottom vacpack slots + controls hint container
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            bottom: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|bottom| {
            // 4 visual vacpack slots
            bottom
                .spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    for idx in 0..4 {
                        row.spawn((
                            Node {
                                width: Val::Px(132.0),
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                column_gap: Val::Px(6.0),
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(10.0)),
                            BorderColor(Color::srgba(0.35, 0.45, 0.60, 0.65)),
                            BackgroundColor(Color::srgba(0.05, 0.09, 0.16, 0.78)),
                            HudSlotBox { idx },
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                ImageNode::default(),
                                Node {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    ..default()
                                },
                                Visibility::Hidden,
                                HudSlotIcon { idx },
                            ));
                            slot.spawn((
                                Text::new(format!("[{}] -", idx + 1)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                HudSlotLabel { idx },
                            ));
                        });
                    }
                });

            // Controls hint bar
            bottom
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                        ..default()
                    },
                    BorderRadius::all(Val::Px(8.0)),
                    BackgroundColor(Color::srgba(0.04, 0.07, 0.14, 0.68)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.84, 0.90, 0.98)),
                        HudHintText,
                    ));
                });
        });

    // Toast prompt
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
            top: Val::Px(165.0),
            ..default()
        },
        HudToastText,
    ));
}

fn spawn_stat_bar(
    panel: &mut ChildBuilder,
    icon: Option<&Handle<Image>>,
    bg_color: Color,
    fill_color: Color,
    kind: HudStatBar,
    height_px: f32,
    init_pct: f32,
) {
    panel
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            if let Some(h) = icon {
                row.spawn((
                    ImageNode::new(h.clone()),
                    Node {
                        width: Val::Px(14.0),
                        height: Val::Px(14.0),
                        ..default()
                    },
                ));
            }
            row.spawn((
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(height_px),
                    ..default()
                },
                BorderRadius::all(Val::Px(4.0)),
                BackgroundColor(bg_color),
            ))
            .with_children(|bg| {
                bg.spawn((
                    Node {
                        width: Val::Percent(init_pct),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BorderRadius::all(Val::Px(4.0)),
                    BackgroundColor(fill_color),
                    kind,
                ));
            });
        });
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_hud(
    sim_res: Res<SimResource>,
    target_info: Res<TargetInfo>,
    assets: Res<ClientAssets>,
    look: Res<CameraLookState>,
    mut cache: Local<HudCache>,
    mut top_q: Query<
        &mut Text,
        (
            With<HudTopLeftText>,
            Without<HudHintText>,
            Without<HudToastText>,
            Without<HudTargetText>,
            Without<HudSlotLabel>,
        ),
    >,
    mut hint_q: Query<
        &mut Text,
        (
            With<HudHintText>,
            Without<HudTopLeftText>,
            Without<HudToastText>,
            Without<HudTargetText>,
            Without<HudSlotLabel>,
        ),
    >,
    mut toast_q: Query<
        &mut Text,
        (
            With<HudToastText>,
            Without<HudTopLeftText>,
            Without<HudHintText>,
            Without<HudTargetText>,
            Without<HudSlotLabel>,
        ),
    >,
    mut target_q: Query<
        (&mut Text, &mut BackgroundColor),
        (
            With<HudTargetText>,
            Without<HudTopLeftText>,
            Without<HudHintText>,
            Without<HudToastText>,
            Without<HudSlotLabel>,
            Without<HudSlotBox>,
        ),
    >,
    mut bars_q: Query<(&HudStatBar, &mut Node)>,
    mut slot_box_q: Query<
        (&HudSlotBox, &mut BorderColor, &mut BackgroundColor),
        Without<HudTargetText>,
    >,
    mut slot_icon_q: Query<(&HudSlotIcon, &mut ImageNode, &mut Visibility), Without<HudDayIcon>>,
    mut day_icon_q: Query<&mut ImageNode, (With<HudDayIcon>, Without<HudSlotIcon>)>,
    mut slot_label_q: Query<
        (&HudSlotLabel, &mut Text),
        (
            Without<HudTopLeftText>,
            Without<HudHintText>,
            Without<HudToastText>,
            Without<HudTargetText>,
        ),
    >,
) {
    let sim = &sim_res.sim;
    let p = &sim.model.player;
    let day = TimeDirector::curr_day(&sim.model.world);
    let hour_f = TimeDirector::curr_hour(&sim.model.world);
    let hh = hour_f.floor() as i32;
    let mm = ((hour_f.fract()) * 60.0).floor() as i32;
    let lang_code = assets.language.code();
    let lang_tag = if assets.language == Language::Ru { 1 } else { 0 };

    // 1. Top-left stats (dirty-checked)
    let top_sig = (
        day,
        hh,
        mm,
        p.currency,
        p.curr_health.round() as i32,
        p.curr_energy.round() as i32,
        p.curr_rads.round() as i32,
        sim.model.actors.len(),
        lang_tag,
    );
    if cache.top_sig != top_sig {
        cache.top_sig = top_sig;
        let day_str = assets.active_loc().format("ui.m.day", &[&day.to_string()]);
        let label_newbucks = assets
            .ui_text("ui.l.endgame.newbucks", "Newbucks:")
            .trim_end_matches(':');
        let (label_hp, label_en, label_rads) = match assets.language {
            Language::Ru => ("Здоровье", "Энергия", "Рады"),
            Language::En => ("HP", "Energy", "Rads"),
        };

        if let Ok(mut img) = day_icon_q.get_single_mut() {
            let is_day = (6..19).contains(&hh);
            if let Some(h) = if is_day {
                assets.icon_sun.as_ref()
            } else {
                assets.icon_moon.as_ref()
            } {
                img.image = h.clone();
            }
        }

        if let Ok(mut t) = top_q.get_single_mut() {
            t.0 = format!(
                "{day_str} | {hh:02}:{mm:02} | {label_newbucks}: {} | Keys: {}\n\
                 {label_hp}: {:.0}/{} | {label_en}: {:.0}/{} | {label_rads}: {:.0}/{}\n\
                 Actors: {} | Local: {} tex / {} audio / {} mesh [{}]",
                p.currency,
                p.keys,
                p.curr_health,
                p.max_health,
                p.curr_energy,
                p.max_energy,
                p.curr_rads,
                p.max_rads,
                sim.model.actors.len(),
                assets.manifest.textures.len(),
                assets.manifest.audio.len(),
                assets.manifest.models.len(),
                lang_code.to_uppercase(),
            );
        }
    }

    // 2. Stat bars (dirty-checked)
    let hp_pct = (p.curr_health / (p.max_health.max(1) as f32) * 100.0).clamp(0.0, 100.0) as i32;
    let en_pct = (p.curr_energy / (p.max_energy.max(1) as f32) * 100.0).clamp(0.0, 100.0) as i32;
    let rad_pct = (p.curr_rads / (p.max_rads.max(1) as f32) * 100.0).clamp(0.0, 100.0) as i32;
    if hp_pct != cache.hp_pct || en_pct != cache.en_pct || rad_pct != cache.rad_pct {
        cache.hp_pct = hp_pct;
        cache.en_pct = en_pct;
        cache.rad_pct = rad_pct;
        for (kind, mut node) in &mut bars_q {
            let pct = match kind {
                HudStatBar::Hp => hp_pct,
                HudStatBar::Energy => en_pct,
                HudStatBar::Rads => rad_pct,
            };
            node.width = Val::Percent(pct as f32);
        }
    }

    // 3. Look-at target prompt (dirty-checked)
    if cache.target_title != target_info.title || cache.target_sub != target_info.subtitle {
        cache.target_title = target_info.title.clone();
        cache.target_sub = target_info.subtitle.clone();
        if let Ok((mut tt, mut bg)) = target_q.get_single_mut() {
            if target_info.title.is_empty() {
                tt.0.clear();
                bg.0 = Color::srgba(0.04, 0.07, 0.14, 0.0);
            } else {
                if target_info.subtitle.is_empty() {
                    tt.0 = target_info.title.clone();
                } else {
                    tt.0 = format!("{}\n{}", target_info.title, target_info.subtitle);
                }
                bg.0 = Color::srgba(0.04, 0.07, 0.14, 0.72);
            }
        }
    }

    // 4. Vacpack slots (dirty-checked)
    let usable = sr_gameplay::AmmoController::usable_slot_count(p).min(4);
    let mut current_slots = [(IdentifiableId::None, 0_i32); 4];
    for (idx, entry) in current_slots.iter_mut().enumerate().take(usable) {
        let id = sim.ammo.slot_id(p, idx, sim.model.world.world_time);
        let cnt = sim.ammo.slot_count(p, idx);
        *entry = (id, cnt);
    }

    if current_slots != cache.slots
        || sim.ammo.selected_idx != cache.selected_slot
        || lang_code != cache.lang_code
    {
        cache.slots = current_slots;
        cache.selected_slot = sim.ammo.selected_idx;

        for (box_meta, mut border, mut bg) in &mut slot_box_q {
            let selected = box_meta.idx == sim.ammo.selected_idx;
            *border = BorderColor(if selected {
                Color::srgb(0.32, 0.90, 1.0)
            } else {
                Color::srgba(0.35, 0.45, 0.60, 0.65)
            });
            *bg = BackgroundColor(if selected {
                Color::srgba(0.12, 0.28, 0.46, 0.90)
            } else {
                Color::srgba(0.05, 0.09, 0.16, 0.78)
            });
        }

        for (icon_meta, mut img, mut vis) in &mut slot_icon_q {
            let id = current_slots.get(icon_meta.idx).map(|s| s.0).unwrap_or(IdentifiableId::None);
            if id != IdentifiableId::None {
                if let Some(h) = assets.icon_for(id) {
                    img.image = h.clone();
                    if *vis != Visibility::Visible {
                        *vis = Visibility::Visible;
                    }
                    continue;
                }
            }
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }

        for (lbl_meta, mut txt) in &mut slot_label_q {
            let (id, cnt) = current_slots
                .get(lbl_meta.idx)
                .copied()
                .unwrap_or((IdentifiableId::None, 0));
            if id == IdentifiableId::None || cnt == 0 {
                txt.0 = format!("[{}] -", lbl_meta.idx + 1);
            } else {
                txt.0 = format!("[{}] {}\nx{}", lbl_meta.idx + 1, assets.ident_name(id), cnt);
            }
        }
    }

    // 5. Controls hint bar (dirty-checked)
    if cache.cursor_locked != Some(look.cursor_locked) || cache.lang_code != lang_code {
        cache.cursor_locked = Some(look.cursor_locked);
        cache.lang_code = lang_code;
        if let Ok(mut h) = hint_q.get_single_mut() {
            h.0 = match (look.cursor_locked, assets.language) {
                (true, Language::Ru) => {
                    "ПКМ: Вак | ЛКМ: Выстрел | Q: Импульс | P: Слаймопедия | E: Меню | T: +3ч | U: Апгрейд | F1: RU/EN | F2: Спавн | F5/F9: Сохр/Загр"
                        .to_string()
                }
                (false, Language::Ru) => {
                    "Клик по окну: захват мыши | WASD + Space + Shift | P: Слаймопедия | E: Меню участка/маркета | T: +3ч | F1: RU/EN"
                        .to_string()
                }
                (true, Language::En) => {
                    "RMB: Vac | LMB: Shoot | Q: Pulse | P: Slimepedia | E: Plot/Market | T: +3h | U: Upgrade | F1: RU/EN | F2: Spawn | F5/F9: Save/Load"
                        .to_string()
                }
                (false, Language::En) => {
                    "Click window to lock mouse | WASD + Space + Shift | P: Slimepedia | E: Plot/Market UI | T: +3h | F1: RU/EN"
                        .to_string()
                }
            };
        }
    }

    // 6. Toast (dirty-checked)
    let cur_toast = sim_res
        .toast
        .as_ref()
        .map(|(m, _)| m.as_str())
        .unwrap_or("");
    if cache.toast != cur_toast {
        cache.toast.clear();
        cache.toast.push_str(cur_toast);
        if let Ok(mut toast) = toast_q.get_single_mut() {
            toast.0 = cache.toast.clone();
        }
    }
}
