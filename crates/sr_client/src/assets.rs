//! Local asset discovery (`local_assets/`), cached AssetStudio indexing,
//! original textures/icons/meshes/SFX/BGM loading, and embedded RU/EN localization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::audio::Volume;
use bevy::gltf::GltfAssetLabel;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use sr_ids::IdentifiableId;
use sr_model::GameModel;
use sr_sim::TimeDirector;
use sr_tools::{AssetManifest, LocalizationSet};

use crate::obj_loader::ObjLoaderSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Ru,
    En,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Ru => Self::En,
            Self::En => Self::Ru,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveBgmTrack {
    #[default]
    None,
    Day,
    Night,
    Tarr,
}

#[derive(Resource, Default)]
pub struct AudioState {
    pub current_track: ActiveBgmTrack,
    pub bgm_entity: Option<Entity>,
    pub last_slime_jump_t: f32,
    pub last_slime_chomp_t: f32,
    pub last_slime_plort_t: f32,
    pub last_slime_voice_t: f32,
    pub last_damage_t: f32,
    pub voice_cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlimeExpr {
    Happy,
    Blink,
    Elated,
    Hungry,
    Starving,
    Angry,
    Feral,
    Scared,
    ChompOpen,
}

impl SlimeExpr {
    pub const ALL: &'static [Self] = &[
        Self::Happy,
        Self::Blink,
        Self::Elated,
        Self::Hungry,
        Self::Starving,
        Self::Angry,
        Self::Feral,
        Self::Scared,
        Self::ChompOpen,
    ];

    pub fn tokens(self) -> &'static [&'static str] {
        match self {
            Self::Happy => &["happy", "smile", "default"],
            Self::Blink => &["blink", "closed"],
            Self::Elated => &["elated", "joy", "blush"],
            Self::Hungry => &["hungry"],
            Self::Starving => &["starving", "grimace"],
            Self::Angry => &["angry", "mad"],
            Self::Feral => &["feral"],
            Self::Scared => &["scared", "alarm", "fear"],
            Self::ChompOpen => &["chomp", "open"],
        }
    }
}

#[derive(Resource)]
pub struct ClientAssets {
    pub local_root: Option<PathBuf>,
    pub manifest: AssetManifest,
    pub language: Language,
    pub loc_ru: LocalizationSet,
    pub loc_en: LocalizationSet,
    pub ui_font: Handle<Font>,

    // Original UI icons & 3D diffuse textures
    pub item_icons: HashMap<IdentifiableId, Handle<Image>>,
    pub item_textures: HashMap<IdentifiableId, Handle<Image>>,
    pub face_eyes_textures: HashMap<SlimeExpr, Handle<Image>>,
    pub face_mouth_textures: HashMap<SlimeExpr, Handle<Image>>,
    pub ground_texture: Option<Handle<Image>>,
    pub water_texture: Option<Handle<Image>>,
    pub wall_texture: Option<Handle<Image>>,
    pub soil_texture: Option<Handle<Image>>,
    pub wood_texture: Option<Handle<Image>>,
    pub icon_newbucks: Option<Handle<Image>>,
    pub icon_key: Option<Handle<Image>>,
    pub icon_health: Option<Handle<Image>>,
    pub icon_energy: Option<Handle<Image>>,
    pub icon_rads: Option<Handle<Image>>,
    pub icon_sun: Option<Handle<Image>>,
    pub icon_moon: Option<Handle<Image>>,

    // Original 3D meshes (.obj normalized for actors/items, or .glb/.gltf scene)
    pub item_meshes: HashMap<IdentifiableId, Handle<Mesh>>,
    pub plort_base_mesh: Option<Handle<Mesh>>,
    pub vacpack_mesh: Option<Handle<Mesh>>,
    pub world_scene_model: Option<Handle<Scene>>,
    pub world_obj_mesh: Option<Handle<Mesh>>,

    // Original SFX & Music
    pub sfx_vac_start: Option<Handle<AudioSource>>,
    pub sfx_vac_capture: Option<Handle<AudioSource>>,
    pub sfx_vac_shoot: Option<Handle<AudioSource>>,
    pub sfx_jump: Option<Handle<AudioSource>>,
    pub sfx_damage: Option<Handle<AudioSource>>,
    pub sfx_plort_sell: Option<Handle<AudioSource>>,
    pub sfx_airburst: Option<Handle<AudioSource>>,
    pub sfx_slime_jump: Option<Handle<AudioSource>>,
    pub sfx_slime_chomp: Option<Handle<AudioSource>>,
    pub sfx_slime_plort: Option<Handle<AudioSource>>,
    pub sfx_slime_voices: Vec<Handle<AudioSource>>,
    pub sfx_ui_click: Option<Handle<AudioSource>>,
    pub sfx_ui_open: Option<Handle<AudioSource>>,
    pub sfx_ui_buy: Option<Handle<AudioSource>>,
    pub sfx_ui_error: Option<Handle<AudioSource>>,
    pub bgm_ranch_day: Option<Handle<AudioSource>>,
    pub bgm_ranch_night: Option<Handle<AudioSource>>,
    pub bgm_tarr: Option<Handle<AudioSource>>,
}

impl ClientAssets {
    pub fn active_loc(&self) -> &LocalizationSet {
        match self.language {
            Language::Ru => &self.loc_ru,
            Language::En => &self.loc_en,
        }
    }

    pub fn ident_name(&self, id: IdentifiableId) -> String {
        if id == IdentifiableId::None {
            return "-".to_string();
        }
        let lower = id.as_csharp_name().to_ascii_lowercase();
        let key = format!("actor.l.{lower}");
        if let Some(name) = self.active_loc().get(&key) {
            return name.to_string();
        }
        id.as_csharp_name().replace('_', " ")
    }

    pub fn ui_text<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.active_loc().get(key).unwrap_or(fallback)
    }

    /// Picks the best available UI icon for an identifiable (prefers `item_icons`, falls back to `item_textures`).
    pub fn icon_for(&self, id: IdentifiableId) -> Option<&Handle<Image>> {
        self.item_icons.get(&id).or_else(|| self.item_textures.get(&id))
    }
}

pub fn find_workspace_root() -> PathBuf {
    for cand in [PathBuf::from("."), PathBuf::from(".."), PathBuf::from("../..")] {
        if cand.join("Cargo.toml").is_file() || cand.join("local_assets").is_dir() {
            let abs = std::fs::canonicalize(&cand).unwrap_or(cand);
            #[cfg(target_os = "windows")]
            {
                let s = abs.to_string_lossy();
                if let Some(stripped) = s.strip_prefix(r"\\?\") {
                    return PathBuf::from(stripped);
                }
            }
            return abs;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn find_local_assets_dir() -> Option<PathBuf> {
    let root = find_workspace_root().join("local_assets");
    if root.is_dir() {
        Some(root)
    } else {
        None
    }
}

fn asset_rel_path(rel_inside_local: &str) -> String {
    format!("local_assets/{}", rel_inside_local.trim_start_matches('/'))
}

fn load_font(asset_server: &AssetServer, workspace_root: &Path) -> Handle<Font> {
    let cached_local = workspace_root.join("local_assets").join("fonts").join("ui.ttf");
    if cached_local.is_file() {
        return asset_server.load("local_assets/fonts/ui.ttf");
    }
    if workspace_root.join("assets").join("fonts").join("ui.ttf").is_file() {
        return asset_server.load("assets/fonts/ui.ttf");
    }

    let mut sys_fonts: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        sys_fonts.push(PathBuf::from(r"C:\Windows\Fonts\segoeui.ttf"));
        sys_fonts.push(PathBuf::from(r"C:\Windows\Fonts\arial.ttf"));
        sys_fonts.push(PathBuf::from(r"C:\Windows\Fonts\tahoma.ttf"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        sys_fonts.push(PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"));
        sys_fonts.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
    }

    for src in sys_fonts {
        if src.is_file() {
            if let Some(parent) = cached_local.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::copy(&src, &cached_local).is_ok() {
                info!("Cached UI font from {} -> {}", src.display(), cached_local.display());
                return asset_server.load("local_assets/fonts/ui.ttf");
            }
        }
    }
    Handle::default()
}

fn ident_search_keys(id: IdentifiableId) -> Vec<String> {
    let csharp = id.as_csharp_name().to_ascii_lowercase();
    let compact = csharp.replace('_', "");
    let mut keys = vec![
        csharp.clone(),
        compact.clone(),
    ];
    // Reversed species/category tokens: e.g. `pink_slime` <-> `slime_pink`
    let parts: Vec<&str> = csharp.split('_').collect();
    if parts.len() == 2 {
        keys.push(format!("{}_{}", parts[1], parts[0]));
        keys.push(format!("{}{}", parts[1], parts[0]));
    }
    // Strip `_veggie`, `_fruit`, `_slime`, `_plort` suffix for generic food/slime meshes like `carrot`
    for suffix in ["_veggie", "_fruit"] {
        if let Some(base) = csharp.strip_suffix(suffix) {
            keys.push(base.to_string());
        }
    }
    keys
}

pub fn load_client_assets(
    asset_server: &AssetServer,
    initial_lang: Language,
) -> ClientAssets {
    let workspace_root = find_workspace_root();
    let local_root = find_local_assets_dir();
    let manifest = if let Some(ref root) = local_root {
        let m_path = root.join("manifest.json");
        let loaded_explicit = if m_path.is_file() {
            AssetManifest::load_json(&m_path).ok().filter(|m| {
                !m.textures.is_empty() || !m.audio.is_empty() || !m.models.is_empty()
            })
        } else {
            None
        };
        let m = loaded_explicit.unwrap_or_else(|| AssetManifest::scan_cached(root).unwrap_or_default());
        info!(
            "AssetManifest ({}): {} textures, {} audio clips, {} models",
            root.display(),
            m.textures.len(),
            m.audio.len(),
            m.models.len()
        );
        if !m.textures.is_empty() || !m.audio.is_empty() || !m.models.is_empty() {
            let sample_tex: Vec<&str> = m.textures.keys().take(6).map(String::as_str).collect();
            let sample_aud: Vec<&str> = m.audio.keys().take(6).map(String::as_str).collect();
            let sample_mod: Vec<&str> = m.models.keys().take(6).map(String::as_str).collect();
            info!(
                "Sample keys -> tex: {:?} | audio: {:?} | models: {:?}",
                sample_tex, sample_aud, sample_mod
            );
        }
        m
    } else {
        info!("No local_assets/ directory found; using procedural visuals & built-in localization.");
        AssetManifest::default()
    };

    let loc_ru =
        LocalizationSet::load_with_override("ru", local_root.as_deref()).unwrap_or_default();
    let loc_en =
        LocalizationSet::load_with_override("en", local_root.as_deref()).unwrap_or_default();

    let ui_font = load_font(asset_server, &workspace_root);

    let mut ground_texture = None;
    let mut water_texture = None;
    let mut wall_texture = None;
    let mut soil_texture = None;
    let mut wood_texture = None;
    let mut icon_newbucks = None;
    let mut icon_key = None;
    let mut icon_health = None;
    let mut icon_energy = None;
    let mut icon_rads = None;
    let mut icon_sun = None;
    let mut icon_moon = None;
    let mut item_icons = HashMap::new();
    let mut item_textures = HashMap::new();
    let mut face_eyes_textures = HashMap::new();
    let mut face_mouth_textures = HashMap::new();
    let mut item_meshes = HashMap::new();
    let mut plort_base_mesh = None;
    let mut vacpack_mesh = None;
    let mut world_scene_model = None;
    let mut world_obj_mesh = None;

    let mut sfx_vac_start = None;
    let mut sfx_vac_capture = None;
    let mut sfx_vac_shoot = None;
    let mut sfx_jump = None;
    let mut sfx_damage = None;
    let mut sfx_plort_sell = None;
    let mut sfx_airburst = None;
    let mut sfx_slime_jump = None;
    let mut sfx_slime_chomp = None;
    let mut sfx_slime_plort = None;
    let mut sfx_slime_voices = Vec::new();
    let mut sfx_ui_click = None;
    let mut sfx_ui_open = None;
    let mut sfx_ui_buy = None;
    let mut sfx_ui_error = None;
    let mut bgm_ranch_day = None;
    let mut bgm_ranch_night = None;
    let mut bgm_tarr = None;

    if local_root.is_some() {
        let load_repeat_tex = |candidates: &[&str]| -> Option<Handle<Image>> {
            manifest.find_diffuse_texture(candidates).map(|rel| {
                asset_server.load_with_settings(
                    asset_rel_path(rel),
                    |s: &mut ImageLoaderSettings| {
                        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::Repeat,
                            ..default()
                        });
                    },
                )
            })
        };
        let load_icon = |candidates: &[&str]| -> Option<Handle<Image>> {
            manifest
                .find_icon_texture(candidates)
                .map(|rel| asset_server.load(asset_rel_path(rel)))
        };
        let load_aud = |candidates: &[&str]| -> Option<Handle<AudioSource>> {
            manifest
                .find_audio(candidates)
                .map(|rel| asset_server.load(asset_rel_path(rel)))
        };
        let load_obj_unit = |candidates: &[&str]| -> Option<Handle<Mesh>> {
            manifest.find_model(candidates).and_then(|rel| {
                if rel.to_ascii_lowercase().ends_with(".obj") {
                    Some(asset_server.load_with_settings(
                        asset_rel_path(rel),
                        |s: &mut ObjLoaderSettings| {
                            s.normalize_unit_radius = true;
                        },
                    ))
                } else {
                    None
                }
            })
        };

        // Environment tiling textures
        ground_texture = load_repeat_tex(&[
            "ranch_grass",
            "grass_01",
            "terrain_grass",
            "grass",
            "ranch_ground",
            "dirt_grass",
        ]);
        water_texture = load_repeat_tex(&["water", "pond_water", "ocean", "sea_water"]);
        wall_texture = load_repeat_tex(&["stone_wall", "rock_wall", "stone", "cliff", "concrete"]);
        soil_texture = load_repeat_tex(&["garden_soil", "soil", "dirt", "mud"]);
        wood_texture = load_repeat_tex(&["wood", "plank", "timber"]);

        // HUD UI icons
        icon_newbucks = load_icon(&["iconnewbucks", "newbucks", "currency", "coin"]);
        icon_key = load_icon(&["iconslimekey", "slimekey", "iconkey"]);
        icon_health = load_icon(&["iconhealth", "health", "heart"]);
        icon_energy = load_icon(&["iconenergy", "energy", "bolt"]);
        icon_rads = load_icon(&["iconrad", "rad", "radiation"]);
        icon_sun = load_icon(&["iconsun", "sun", "dayicon"]);
        icon_moon = load_icon(&["iconmoon", "moon", "nighticon"]);

        // Slime face expressions (`eyes_*` & `mouth_*`)
        for &expr in SlimeExpr::ALL {
            if let Some(rel) = manifest.find_face_texture(true, expr.tokens()) {
                face_eyes_textures.insert(expr, asset_server.load(asset_rel_path(rel)));
            }
            if let Some(rel) = manifest.find_face_texture(false, expr.tokens()) {
                face_mouth_textures.insert(expr, asset_server.load(asset_rel_path(rel)));
            }
        }

        // Per-item UI icons, 3D diffuse textures (never for slime jelly bodies!), and 3D `.obj` meshes
        for &id in IdentifiableId::ALL {
            if id == IdentifiableId::None {
                continue;
            }
            let base_keys = ident_search_keys(id);
            let mut icon_keys: Vec<String> = Vec::with_capacity(base_keys.len() * 2 + 2);
            for k in &base_keys {
                icon_keys.push(format!("icon{k}"));
                icon_keys.push(format!("icon_{k}"));
            }
            icon_keys.extend(base_keys.iter().cloned());

            let icon_refs: Vec<&str> = icon_keys.iter().map(String::as_str).collect();
            if let Some(rel) = manifest.find_icon_texture(&icon_refs) {
                item_icons.insert(id, asset_server.load(asset_rel_path(rel)));
            }

            // Slime bodies in SR use palette gradient shaders, not 2D sprites/icons!
            // Only load 3D diffuse textures for non-slime items (foods, plorts, crates, etc.).
            if !id.is_slime() {
                let tex_refs: Vec<&str> = base_keys.iter().map(String::as_str).collect();
                if let Some(rel) = manifest.find_diffuse_texture(&tex_refs) {
                    item_textures.insert(id, asset_server.load(asset_rel_path(rel)));
                }
                if let Some(mesh_h) = load_obj_unit(&tex_refs) {
                    item_meshes.insert(id, mesh_h);
                }
            }
        }

        plort_base_mesh = load_obj_unit(&["plort", "plort_base"]);
        vacpack_mesh = load_obj_unit(&["vacpack", "vac_gun", "vac_nozzle", "vacgun"]);

        let face_samples: Vec<&str> = manifest
            .textures
            .keys()
            .filter(|k| k.contains("eye") || k.contains("mouth") || k.contains("slime"))
            .take(15)
            .map(String::as_str)
            .collect();
        let slime_mesh_samples: Vec<&str> = manifest
            .models
            .keys()
            .filter(|k| k.contains("slime") || k.contains("plort") || k.contains("vac"))
            .take(15)
            .map(String::as_str)
            .collect();
        info!(
            "Face/slime tex samples: {:?} | mesh samples: {:?}",
            face_samples, slime_mesh_samples
        );

        // Original world scene (.glb/.gltf or raw .obj)
        if let Some(rel) =
            manifest.find_model(&["world", "ranch", "slimerancher", "zone_ranch", "arena"])
        {
            let lower = rel.to_ascii_lowercase();
            if lower.ends_with(".glb") || lower.ends_with(".gltf") {
                world_scene_model = Some(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset_rel_path(rel))),
                );
            } else if lower.ends_with(".obj") {
                world_obj_mesh = Some(asset_server.load(asset_rel_path(rel)));
            }
        }

        // Original SFX & Music
        sfx_vac_start = load_aud(&["vac_start", "vacstart", "vac_on"]);
        sfx_vac_capture = load_aud(&["vac_capture", "vaccapture", "vac_suck", "vacin", "suck"]);
        sfx_vac_shoot = load_aud(&["vac_shoot", "vacshoot", "vacout", "shoot"]);
        sfx_jump = load_aud(&["player_jump", "jump", "boing"]);
        sfx_damage = load_aud(&["player_damage", "damage", "hurt", "ouch"]);
        sfx_plort_sell = load_aud(&["plort_sell", "newbucks", "cash", "coin", "market"]);
        sfx_airburst = load_aud(&["airburst", "vac_pulse", "pulse", "shockwave"]);

        sfx_slime_jump = load_aud(&["slime_jump", "slimejump", "slime_hop"]);
        sfx_slime_chomp = load_aud(&["slime_chomp", "slimechomp", "chomp", "gulp", "eat"]);
        sfx_slime_plort = load_aud(&["slime_plort", "slimeplort", "plort_pop", "plort"]);
        for rel in manifest.find_audio_many(
            &["slime_voice", "slimevoice", "slime_fun", "slime_awe", "slime_happy", "voice_fun"],
            6,
        ) {
            sfx_slime_voices.push(asset_server.load(asset_rel_path(rel)));
        }

        sfx_ui_click = load_aud(&["ui_click", "button_click", "click", "select"]);
        sfx_ui_open = load_aud(&["ui_open", "pedia_open", "menu_open", "popup"]);
        sfx_ui_buy = load_aud(&["ui_buy", "purchase", "upgrade", "cash"]);
        sfx_ui_error = load_aud(&["ui_error", "error", "denied", "buzz"]);

        bgm_ranch_day = load_aud(&["ranch_day", "ranchday", "ranchmusic", "ranch", "day_01"]);
        bgm_ranch_night = load_aud(&["ranch_night", "ranchnight", "night_01", "night_ranch"]);
        bgm_tarr = load_aud(&["tarr_music", "tarr_theme", "tarr", "danger"]);

        info!(
            "Matched original assets -> icons: {}, 3D textures: {}, face eyes/mouth: {}/{}, 3D meshes: {}, slime voices: {}, BGM: {}/{}/{}",
            item_icons.len(),
            item_textures.len(),
            face_eyes_textures.len(),
            face_mouth_textures.len(),
            item_meshes.len(),
            sfx_slime_voices.len(),
            bgm_ranch_day.is_some(),
            bgm_ranch_night.is_some(),
            bgm_tarr.is_some(),
        );
    }

    ClientAssets {
        local_root,
        manifest,
        language: initial_lang,
        loc_ru,
        loc_en,
        ui_font,
        item_icons,
        item_textures,
        face_eyes_textures,
        face_mouth_textures,
        ground_texture,
        water_texture,
        wall_texture,
        soil_texture,
        wood_texture,
        icon_newbucks,
        icon_key,
        icon_health,
        icon_energy,
        icon_rads,
        icon_sun,
        icon_moon,
        item_meshes,
        plort_base_mesh,
        vacpack_mesh,
        world_scene_model,
        world_obj_mesh,
        sfx_vac_start,
        sfx_vac_capture,
        sfx_vac_shoot,
        sfx_jump,
        sfx_damage,
        sfx_plort_sell,
        sfx_airburst,
        sfx_slime_jump,
        sfx_slime_chomp,
        sfx_slime_plort,
        sfx_slime_voices,
        sfx_ui_click,
        sfx_ui_open,
        sfx_ui_buy,
        sfx_ui_error,
        bgm_ranch_day,
        bgm_ranch_night,
        bgm_tarr,
    }
}

pub fn play_sfx(commands: &mut Commands, clip: Option<&Handle<AudioSource>>, volume: f32) {
    if let Some(h) = clip {
        commands.spawn((
            AudioPlayer::new(h.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::new(volume)),
        ));
    }
}

/// Switches background music dynamically between Day, Night, and Tarr alert themes.
pub fn update_bgm_and_world_sfx_system(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<ClientAssets>,
    mut audio_state: ResMut<AudioState>,
    sim_res: Res<crate::world_sync::SimResource>,
) {
    let now = time.elapsed_secs();
    let sim = &sim_res.sim;

    // 1. Dynamic BGM switching (Tarr nearby > Night > Day)
    let player_pos = sim.model.player.position;
    let tarr_nearby = assets.bgm_tarr.is_some()
        && sim.model.actors.values().any(|a| {
            a.ident == IdentifiableId::TarrSlime && a.pos.distance_squared(player_pos) < 35.0 * 35.0
        });
    let hour = TimeDirector::curr_hour(&sim.model.world);
    let is_night = !(6.0..19.0).contains(&hour);

    let desired = if tarr_nearby {
        ActiveBgmTrack::Tarr
    } else if is_night && assets.bgm_ranch_night.is_some() {
        ActiveBgmTrack::Night
    } else if assets.bgm_ranch_day.is_some() {
        ActiveBgmTrack::Day
    } else {
        ActiveBgmTrack::None
    };

    if desired != audio_state.current_track {
        if let Some(old_ent) = audio_state.bgm_entity.take() {
            commands.entity(old_ent).despawn_recursive();
        }
        let clip = match desired {
            ActiveBgmTrack::Tarr => assets.bgm_tarr.as_ref(),
            ActiveBgmTrack::Night => assets.bgm_ranch_night.as_ref(),
            ActiveBgmTrack::Day => assets.bgm_ranch_day.as_ref(),
            ActiveBgmTrack::None => None,
        };
        if let Some(h) = clip {
            let vol = if desired == ActiveBgmTrack::Tarr { 0.34 } else { 0.26 };
            let ent = commands
                .spawn((
                    AudioPlayer::new(h.clone()),
                    PlaybackSettings::LOOP.with_volume(Volume::new(vol)),
                ))
                .id();
            audio_state.bgm_entity = Some(ent);
        }
        audio_state.current_track = desired;
    }

    // 2. Rate-limited simulation audio events (eating, plort drops, slime voices, damage)
    let events = &sim_res.last_events;
    if !events.ai.eat.eaten_pairs.is_empty() && now - audio_state.last_slime_chomp_t > 0.22 {
        audio_state.last_slime_chomp_t = now;
        play_sfx(&mut commands, assets.sfx_slime_chomp.as_ref(), 0.55);
    }
    if !events.ai.eat.spawned_plorts.is_empty() && now - audio_state.last_slime_plort_t > 0.25 {
        audio_state.last_slime_plort_t = now;
        play_sfx(&mut commands, assets.sfx_slime_plort.as_ref(), 0.50);
    }
    if (events.ai.player_damage > 0.0 || events.ai.player_fire_damage > 0.0)
        && now - audio_state.last_damage_t > 0.45
    {
        audio_state.last_damage_t = now;
        play_sfx(&mut commands, assets.sfx_damage.as_ref(), 0.70);
    }
    if assets.sfx_slime_jump.is_some()
        && now - audio_state.last_slime_jump_t > 1.4
        && sim.physics.bodies.values().any(|b| {
            b.vel.y > 4.2 && b.pos.distance_squared(player_pos) < 16.0 * 16.0
        })
    {
        audio_state.last_slime_jump_t = now;
        play_sfx(&mut commands, assets.sfx_slime_jump.as_ref(), 0.30);
    }
    if !assets.sfx_slime_voices.is_empty()
        && now - audio_state.last_slime_voice_t > 2.4
        && has_nearby_slimes(&sim.model, player_pos)
    {
        audio_state.last_slime_voice_t = now;
        let idx = audio_state.voice_cursor % assets.sfx_slime_voices.len();
        audio_state.voice_cursor += 1;
        play_sfx(&mut commands, Some(&assets.sfx_slime_voices[idx]), 0.35);
    }
}

fn has_nearby_slimes(model: &GameModel, player_pos: sr_math::Vec3) -> bool {
    model
        .actors
        .values()
        .any(|a| a.ident.is_slime() && a.pos.distance_squared(player_pos) < 18.0 * 18.0)
}
