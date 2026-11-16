//! Step 11.4 In-game UI screens using original Slime Rancher RU/EN localization & `Texture2D` icons:
//! - `[P]` Slimepedia (`Slimes`, `Plorts`, `Food`, `Ranch` categories with `pedia.*` entries)
//! - `[E]` Ranch Plot Upgrades (`Corral` / `Garden`) & Live Plort Market Board

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use sr_gameplay::{build_market_listings, upgrade_purchase_cost, PlotOpError, RanchDirector};
use sr_ids::{IdentifiableId, LandPlotUpgrade};
use sr_sandbox::landmarks;

use crate::assets::{play_sfx, ClientAssets, Language};
use crate::player_input::CameraLookState;
use crate::world_sync::{SimResource, TargetInfo};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveScreen {
    #[default]
    None,
    Slimepedia,
    PlotUpgrades(String),
    PlortMarket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PediaCategory {
    #[default]
    Slimes,
    Plorts,
    Food,
    Ranch,
}

#[derive(Resource, Default)]
pub struct UiScreenState {
    pub active: ActiveScreen,
    pub pedia_cat: PediaCategory,
    pub pedia_selected: usize,
    pub dirty: bool,
    pub root_entity: Option<Entity>,
}

impl UiScreenState {
    pub fn is_open(&self) -> bool {
        self.active != ActiveScreen::None
    }
}

#[derive(Component, Clone)]
pub enum UiAction {
    Close,
    SetPediaCategory(PediaCategory),
    SelectPediaEntry(usize),
    BuyUpgrade(String, LandPlotUpgrade),
}

#[derive(Component)]
pub struct UiModalRoot;

struct PediaItem {
    ident: Option<IdentifiableId>,
    pedia_key: &'static str,
    fallback_title: &'static str,
    is_slime: bool,
}

const SLIME_PEDIA: &[PediaItem] = &[
    PediaItem { ident: Some(IdentifiableId::PinkSlime), pedia_key: "pink_slime", fallback_title: "Pink Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::TabbySlime), pedia_key: "tabby_slime", fallback_title: "Tabby Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::RockSlime), pedia_key: "rock_slime", fallback_title: "Rock Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::PhosphorSlime), pedia_key: "phosphor_slime", fallback_title: "Phosphor Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::HoneySlime), pedia_key: "honey_slime", fallback_title: "Honey Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::BoomSlime), pedia_key: "boom_slime", fallback_title: "Boom Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::RadSlime), pedia_key: "rad_slime", fallback_title: "Rad Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::CrystalSlime), pedia_key: "crystal_slime", fallback_title: "Crystal Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::HunterSlime), pedia_key: "hunter_slime", fallback_title: "Hunter Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::PuddleSlime), pedia_key: "puddle_slime", fallback_title: "Puddle Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::FireSlime), pedia_key: "fire_slime", fallback_title: "Fire Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::GoldSlime), pedia_key: "gold_slime", fallback_title: "Gold Slime", is_slime: true },
    PediaItem { ident: Some(IdentifiableId::TarrSlime), pedia_key: "tarr_slime", fallback_title: "The Tarr", is_slime: true },
];

const PLORT_PEDIA: &[PediaItem] = &[
    PediaItem { ident: Some(IdentifiableId::PinkPlort), pedia_key: "pink_slime", fallback_title: "Pink Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::TabbyPlort), pedia_key: "tabby_slime", fallback_title: "Tabby Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::RockPlort), pedia_key: "rock_slime", fallback_title: "Rock Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::PhosphorPlort), pedia_key: "phosphor_slime", fallback_title: "Phosphor Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::HoneyPlort), pedia_key: "honey_slime", fallback_title: "Honey Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::BoomPlort), pedia_key: "boom_slime", fallback_title: "Boom Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::RadPlort), pedia_key: "rad_slime", fallback_title: "Rad Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::CrystalPlort), pedia_key: "crystal_slime", fallback_title: "Crystal Plort", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::GoldPlort), pedia_key: "gold_slime", fallback_title: "Gold Plort", is_slime: false },
];

const FOOD_PEDIA: &[PediaItem] = &[
    PediaItem { ident: Some(IdentifiableId::CarrotVeggie), pedia_key: "carrot", fallback_title: "Carrot", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::BeetVeggie), pedia_key: "beet", fallback_title: "Heart Beet", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::OcaocaVeggie), pedia_key: "ocaoca", fallback_title: "Oca Oca", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::PogoFruit), pedia_key: "pogofruit", fallback_title: "Pogofruit", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::CuberryFruit), pedia_key: "cuberry", fallback_title: "Cuberry", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::MangoFruit), pedia_key: "mint_mango", fallback_title: "Mint Mango", is_slime: false },
    PediaItem { ident: Some(IdentifiableId::Hen), pedia_key: "henhen", fallback_title: "Hen Hen", is_slime: false },
];

const RANCH_PEDIA: &[PediaItem] = &[
    PediaItem { ident: None, pedia_key: "corral", fallback_title: "Corral", is_slime: false },
    PediaItem { ident: None, pedia_key: "garden", fallback_title: "Garden", is_slime: false },
    PediaItem { ident: None, pedia_key: "pond", fallback_title: "Pond", is_slime: false },
    PediaItem { ident: None, pedia_key: "coop", fallback_title: "Coop", is_slime: false },
    PediaItem { ident: None, pedia_key: "silo", fallback_title: "Silo", is_slime: false },
    PediaItem { ident: None, pedia_key: "plort_market", fallback_title: "Plort Market", is_slime: false },
];

fn pedia_items_for(cat: PediaCategory) -> &'static [PediaItem] {
    match cat {
        PediaCategory::Slimes => SLIME_PEDIA,
        PediaCategory::Plorts => PLORT_PEDIA,
        PediaCategory::Food => FOOD_PEDIA,
        PediaCategory::Ranch => RANCH_PEDIA,
    }
}

fn unlock_cursor(look: &mut CameraLookState, windows: &mut Query<&mut Window, With<PrimaryWindow>>) {
    look.cursor_locked = false;
    if let Ok(mut win) = windows.get_single_mut() {
        win.cursor_options.grab_mode = CursorGrabMode::None;
        win.cursor_options.visible = true;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn toggle_ui_screens_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    sim_res: Res<SimResource>,
    target_info: Res<TargetInfo>,
    assets: Res<ClientAssets>,
    mut ui_state: ResMut<UiScreenState>,
    mut look: ResMut<CameraLookState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        if ui_state.active == ActiveScreen::Slimepedia {
            ui_state.active = ActiveScreen::None;
        } else {
            ui_state.active = ActiveScreen::Slimepedia;
            unlock_cursor(&mut look, &mut windows);
            play_sfx(&mut commands, assets.sfx_ui_open.as_ref(), 0.6);
        }
        ui_state.dirty = true;
        return;
    }

    if keys.just_pressed(KeyCode::KeyE) {
        if ui_state.is_open() {
            ui_state.active = ActiveScreen::None;
            ui_state.dirty = true;
            return;
        }
        let p_pos = sim_res.sim.model.player.position;
        if let Some(ref plot_id) = target_info.nearby_plot_id {
            ui_state.active = ActiveScreen::PlotUpgrades(plot_id.clone());
            unlock_cursor(&mut look, &mut windows);
            play_sfx(&mut commands, assets.sfx_ui_open.as_ref(), 0.6);
            ui_state.dirty = true;
        } else if p_pos.distance(landmarks::MARKET_CENTER) < 9.5 {
            ui_state.active = ActiveScreen::PlortMarket;
            unlock_cursor(&mut look, &mut windows);
            play_sfx(&mut commands, assets.sfx_ui_open.as_ref(), 0.6);
            ui_state.dirty = true;
        }
        return;
    }

    if keys.just_pressed(KeyCode::Escape) && ui_state.is_open() {
        ui_state.active = ActiveScreen::None;
        ui_state.dirty = true;
        play_sfx(&mut commands, assets.sfx_ui_click.as_ref(), 0.5);
    }
}

#[allow(clippy::type_complexity)]
pub fn handle_ui_buttons_system(
    mut commands: Commands,
    mut ui_state: ResMut<UiScreenState>,
    mut sim_res: ResMut<SimResource>,
    assets: Res<ClientAssets>,
    mut interactions: Query<
        (&Interaction, &UiAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (inter, action, mut bg) in &mut interactions {
        match *inter {
            Interaction::Hovered => {
                bg.0 = Color::srgba(0.22, 0.45, 0.72, 0.95);
            }
            Interaction::None => {
                bg.0 = Color::srgba(0.12, 0.24, 0.40, 0.92);
            }
            Interaction::Pressed => {
                match action {
                    UiAction::Close => {
                        ui_state.active = ActiveScreen::None;
                        ui_state.dirty = true;
                        play_sfx(&mut commands, assets.sfx_ui_click.as_ref(), 0.55);
                    }
                    UiAction::SetPediaCategory(cat) => {
                        ui_state.pedia_cat = *cat;
                        ui_state.pedia_selected = 0;
                        ui_state.dirty = true;
                        play_sfx(&mut commands, assets.sfx_ui_click.as_ref(), 0.55);
                    }
                    UiAction::SelectPediaEntry(idx) => {
                        ui_state.pedia_selected = *idx;
                        ui_state.dirty = true;
                        play_sfx(&mut commands, assets.sfx_ui_click.as_ref(), 0.55);
                    }
                    UiAction::BuyUpgrade(plot_id, upg) => {
                        let plot_type = sim_res
                            .sim
                            .model
                            .land_plots
                            .get(plot_id)
                            .map(|p| p.type_id);
                        let cost = plot_type
                            .and_then(|pt| upgrade_purchase_cost(pt, *upg))
                            .unwrap_or(0);
                        match RanchDirector::buy_upgrade(&mut sim_res.sim.model, plot_id, *upg) {
                            Ok(()) => {
                                play_sfx(&mut commands, assets.sfx_ui_buy.as_ref(), 0.7);
                                sim_res.set_toast(format!("+ {upg:?} (-{cost} NB)"));
                                ui_state.dirty = true;
                            }
                            Err(PlotOpError::InsufficientFunds) => {
                                play_sfx(&mut commands, assets.sfx_ui_error.as_ref(), 0.65);
                                let msg = match assets.language {
                                    Language::Ru => format!("Нужно {cost} NB"),
                                    Language::En => format!("Need {cost} NB"),
                                };
                                sim_res.set_toast(msg);
                            }
                            Err(err) => {
                                play_sfx(&mut commands, assets.sfx_ui_error.as_ref(), 0.65);
                                sim_res.set_toast(format!("{err:?}"));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn rebuild_ui_screen_system(
    mut commands: Commands,
    mut ui_state: ResMut<UiScreenState>,
    assets: Res<ClientAssets>,
    sim_res: Res<SimResource>,
) {
    if !ui_state.dirty {
        return;
    }
    ui_state.dirty = false;

    if let Some(old) = ui_state.root_entity.take() {
        commands.entity(old).despawn_recursive();
    }

    if ui_state.active == ActiveScreen::None {
        return;
    }

    let font = assets.ui_font.clone();
    let active = ui_state.active.clone();

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(10.0),
                top: Val::Percent(10.0),
                width: Val::Percent(80.0),
                height: Val::Percent(80.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderRadius::all(Val::Px(14.0)),
            BorderColor(Color::srgba(0.35, 0.82, 1.0, 0.85)),
            BackgroundColor(Color::srgba(0.05, 0.09, 0.16, 0.94)),
            UiModalRoot,
        ))
        .with_children(|modal| {
            // Header bar
            modal
                .spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|hdr| {
                    let title = match active {
                        ActiveScreen::Slimepedia => {
                            assets.ui_text("ui.t.pedia", "Slimepedia").to_string()
                        }
                        ActiveScreen::PlotUpgrades(ref pid) => {
                            if pid.contains("corral") {
                                assets.ui_text("ui.t.corral", "Corral").to_string()
                            } else {
                                assets.ui_text("ui.t.garden", "Garden").to_string()
                            }
                        }
                        ActiveScreen::PlortMarket => {
                            assets.ui_text("ui.t.market", "Plort Market").to_string()
                        }
                        ActiveScreen::None => String::new(),
                    };
                    hdr.spawn((
                        Text::new(title),
                        TextFont { font: font.clone(), font_size: 26.0, ..default() },
                        TextColor(Color::srgb(0.50, 0.92, 1.0)),
                    ));
                    hdr.spawn((
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderRadius::all(Val::Px(8.0)),
                        BorderColor(Color::srgb(0.9, 0.4, 0.45)),
                        BackgroundColor(Color::srgba(0.35, 0.12, 0.16, 0.9)),
                        UiAction::Close,
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("X [Esc]"),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                });

            match active {
                ActiveScreen::Slimepedia => {
                    build_slimepedia_body(modal, &ui_state, &assets, &font);
                }
                ActiveScreen::PlotUpgrades(ref plot_id) => {
                    build_plot_upgrades_body(modal, plot_id, &sim_res, &assets, &font);
                }
                ActiveScreen::PlortMarket => {
                    build_market_body(modal, &sim_res, &assets, &font);
                }
                ActiveScreen::None => {}
            }
        })
        .id();

    ui_state.root_entity = Some(root);
}

fn build_slimepedia_body(
    modal: &mut ChildBuilder,
    ui_state: &UiScreenState,
    assets: &ClientAssets,
    font: &Handle<Font>,
) {
    // Category tabs
    let cats = match assets.language {
        Language::Ru => [
            (PediaCategory::Slimes, "Слаймы"),
            (PediaCategory::Plorts, "Плорты"),
            (PediaCategory::Food, "Еда"),
            (PediaCategory::Ranch, "Ранчо"),
        ],
        Language::En => [
            (PediaCategory::Slimes, "Slimes"),
            (PediaCategory::Plorts, "Plorts"),
            (PediaCategory::Food, "Food"),
            (PediaCategory::Ranch, "Ranch"),
        ],
    };

    modal
        .spawn(Node {
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|tabs| {
            for (cat, label) in cats {
                let selected = ui_state.pedia_cat == cat;
                tabs.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BorderRadius::all(Val::Px(8.0)),
                    BorderColor(if selected {
                        Color::srgb(0.4, 0.92, 1.0)
                    } else {
                        Color::srgba(0.4, 0.5, 0.65, 0.5)
                    }),
                    BackgroundColor(if selected {
                        Color::srgba(0.18, 0.42, 0.68, 0.95)
                    } else {
                        Color::srgba(0.12, 0.24, 0.40, 0.92)
                    }),
                    UiAction::SetPediaCategory(cat),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(label),
                        TextFont { font: font.clone(), font_size: 16.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });

    let items = pedia_items_for(ui_state.pedia_cat);
    let sel_idx = ui_state.pedia_selected.min(items.len().saturating_sub(1));
    let sel_item = &items[sel_idx];

    modal
        .spawn(Node {
            flex_grow: 1.0,
            column_gap: Val::Px(14.0),
            ..default()
        })
        .with_children(|split| {
            // Left list
            split
                .spawn((
                    Node {
                        width: Val::Px(260.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.03, 0.06, 0.11, 0.7)),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|list| {
                    for (idx, item) in items.iter().enumerate() {
                        let active = idx == sel_idx;
                        let title = item
                            .ident
                            .map(|id| assets.ident_name(id))
                            .unwrap_or_else(|| {
                                assets
                                    .ui_text(&format!("pedia.t.{}", item.pedia_key), item.fallback_title)
                                    .to_string()
                            });
                        list.spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                column_gap: Val::Px(8.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderRadius::all(Val::Px(6.0)),
                            BackgroundColor(if active {
                                Color::srgba(0.22, 0.48, 0.78, 0.95)
                            } else {
                                Color::srgba(0.10, 0.18, 0.30, 0.85)
                            }),
                            UiAction::SelectPediaEntry(idx),
                        ))
                        .with_children(|row| {
                            if let Some(id) = item.ident {
                                if let Some(icon) = assets.icon_for(id) {
                                    row.spawn((
                                        ImageNode::new(icon.clone()),
                                        Node {
                                            width: Val::Px(26.0),
                                            height: Val::Px(26.0),
                                            ..default()
                                        },
                                    ));
                                }
                            }
                            row.spawn((
                                Text::new(title),
                                TextFont { font: font.clone(), font_size: 15.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });

            // Right detail card
            split
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.03, 0.06, 0.11, 0.75)),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|detail| {
                    let heading = sel_item
                        .ident
                        .map(|id| assets.ident_name(id))
                        .unwrap_or_else(|| {
                            assets
                                .ui_text(
                                    &format!("pedia.t.{}", sel_item.pedia_key),
                                    sel_item.fallback_title,
                                )
                                .to_string()
                        });
                    detail
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.0),
                            ..default()
                        })
                        .with_children(|hdr| {
                            if let Some(id) = sel_item.ident {
                                if let Some(icon) = assets.icon_for(id) {
                                    hdr.spawn((
                                        ImageNode::new(icon.clone()),
                                        Node {
                                            width: Val::Px(56.0),
                                            height: Val::Px(56.0),
                                            ..default()
                                        },
                                    ));
                                }
                            }
                            hdr.spawn((
                                Text::new(heading),
                                TextFont { font: font.clone(), font_size: 24.0, ..default() },
                                TextColor(Color::srgb(1.0, 0.92, 0.45)),
                            ));
                        });

                    let mut body = String::new();
                    let k = sel_item.pedia_key;
                    if let Some(intro) = assets.active_loc().get(&format!("pedia.m.intro.{k}")) {
                        if !intro.is_empty() {
                            body.push_str(intro);
                            body.push_str("\n\n");
                        }
                    }
                    if sel_item.is_slime {
                        if let Some(diet) = assets.active_loc().get(&format!("pedia.m.diet.{k}")) {
                            let lbl = if assets.language == Language::Ru { "Рацион" } else { "Diet" };
                            body.push_str(&format!("{lbl}: {diet}\n"));
                        }
                        if let Some(fav) = assets.active_loc().get(&format!("pedia.m.favorite.{k}")) {
                            let lbl = if assets.language == Language::Ru { "Любимая еда" } else { "Favorite" };
                            body.push_str(&format!("{lbl}: {fav}\n\n"));
                        }
                        if let Some(sl) = assets.active_loc().get(&format!("pedia.m.slimeology.{k}")) {
                            body.push_str(sl);
                            body.push_str("\n\n");
                        }
                        if let Some(risk) = assets.active_loc().get(&format!("pedia.m.risks.{k}")) {
                            body.push_str(risk);
                            body.push_str("\n\n");
                        }
                        if let Some(pl) = assets.active_loc().get(&format!("pedia.m.plortonomics.{k}")) {
                            body.push_str(pl);
                        }
                    } else if let Some(desc) = assets.active_loc().get(&format!("pedia.m.desc.{k}")) {
                        body.push_str(desc);
                    }

                    detail.spawn((
                        Text::new(body),
                        TextFont { font: font.clone(), font_size: 15.0, ..default() },
                        TextColor(Color::srgb(0.90, 0.94, 0.99)),
                    ));
                });
        });
}

fn build_plot_upgrades_body(
    modal: &mut ChildBuilder,
    plot_id: &str,
    sim_res: &SimResource,
    assets: &ClientAssets,
    font: &Handle<Font>,
) {
    let Some(plot) = sim_res.sim.model.land_plots.get(plot_id) else {
        return;
    };
    let (prefix, items): (&str, &[(&str, LandPlotUpgrade)]) = if plot_id.contains("corral") {
        (
            "corral",
            &[
                ("walls", LandPlotUpgrade::Walls),
                ("music_box", LandPlotUpgrade::MusicBox),
                ("air_net", LandPlotUpgrade::AirNet),
                ("solar_shield", LandPlotUpgrade::SolarShield),
                ("feeder", LandPlotUpgrade::Feeder),
                ("plort_collector", LandPlotUpgrade::PlortCollector),
            ],
        )
    } else {
        (
            "garden",
            &[
                ("soil", LandPlotUpgrade::Soil),
                ("sprinkler", LandPlotUpgrade::Sprinkler),
                ("scareslime", LandPlotUpgrade::Scareslime),
            ],
        )
    };

    modal.spawn((
        Text::new(format!(
            "{}: {} NB",
            assets.ui_text("ui.l.endgame.newbucks", "Newbucks:"),
            sim_res.sim.model.player.currency
        )),
        TextFont { font: font.clone(), font_size: 18.0, ..default() },
        TextColor(Color::srgb(1.0, 0.88, 0.35)),
    ));

    modal
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|list| {
            for &(key, upg) in items {
                let installed = plot.has_upgrade(upg);
                let cost = upgrade_purchase_cost(plot.type_id, upg).unwrap_or(0);
                let name = assets.ui_text(&format!("pedia.m.upgrade.name.{prefix}.{key}"), key);
                let desc = assets.ui_text(&format!("pedia.m.upgrade.desc.{prefix}.{key}"), "");

                list.spawn((
                    Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.14, 0.24, 0.85)),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|row| {
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        max_width: Val::Percent(75.0),
                        ..default()
                    })
                    .with_children(|txt| {
                        txt.spawn((
                            Text::new(name),
                            TextFont { font: font.clone(), font_size: 18.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                        txt.spawn((
                            Text::new(desc),
                            TextFont { font: font.clone(), font_size: 13.0, ..default() },
                            TextColor(Color::srgb(0.75, 0.82, 0.92)),
                        ));
                    });

                    if installed {
                        let done_lbl = if assets.language == Language::Ru { "Установлено" } else { "Installed" };
                        row.spawn((
                            Text::new(done_lbl),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(Color::srgb(0.35, 0.95, 0.45)),
                        ));
                    } else {
                        let buy_lbl = if assets.language == Language::Ru {
                            format!("Купить ({cost} NB)")
                        } else {
                            format!("Buy ({cost} NB)")
                        };
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderRadius::all(Val::Px(8.0)),
                            BorderColor(Color::srgb(0.35, 0.85, 1.0)),
                            BackgroundColor(Color::srgba(0.12, 0.24, 0.40, 0.92)),
                            UiAction::BuyUpgrade(plot_id.to_string(), upg),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(buy_lbl),
                                TextFont { font: font.clone(), font_size: 15.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });
            }
        });
}

fn build_market_body(
    modal: &mut ChildBuilder,
    sim_res: &SimResource,
    assets: &ClientAssets,
    font: &Handle<Font>,
) {
    let listings = build_market_listings(
        &sim_res.sim.economy.market,
        &sim_res.sim.model,
        &sim_res.sim.mode_settings,
    );
    modal
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|grid| {
            for entry in listings {
                grid.spawn((
                    Node {
                        width: Val::Px(220.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        column_gap: Val::Px(10.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.14, 0.24, 0.88)),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|card| {
                    if let Some(icon) = assets.icon_for(entry.id) {
                        card.spawn((
                            ImageNode::new(icon.clone()),
                            Node {
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                ..default()
                            },
                        ));
                    }
                    card.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .with_children(|txt| {
                        txt.spawn((
                            Text::new(assets.ident_name(entry.id)),
                            TextFont { font: font.clone(), font_size: 15.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                        let arrow = if entry.change_in_value > 0 {
                            "+"
                        } else if entry.change_in_value < 0 {
                            "-"
                        } else {
                            "="
                        };
                        txt.spawn((
                            Text::new(format!(
                                "{} NB ({arrow}{})",
                                entry.curr_value,
                                entry.change_in_value.abs()
                            )),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(if entry.change_in_value > 0 {
                                Color::srgb(0.35, 0.95, 0.45)
                            } else if entry.change_in_value < 0 {
                                Color::srgb(0.95, 0.40, 0.40)
                            } else {
                                Color::srgb(1.0, 0.88, 0.35)
                            }),
                        ));
                    });
                });
            }
        });
}
