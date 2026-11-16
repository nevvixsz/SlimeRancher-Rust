//! Local asset discovery (`local_assets/`), cached AssetStudio indexing,
//! exact 1:1 original textures/icons/meshes/SFX/BGM loading, food map+mask+ramp
//! composition, and embedded RU/EN localization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::audio::Volume;
use bevy::gltf::GltfAssetLabel;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sr_ids::IdentifiableId;
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

    pub fn full_face_tokens(self) -> &'static [&'static str] {
        match self {
            Self::Happy => &["happy"],
            Self::Blink => &["blink"],
            Self::Elated => &["elated", "singing"],
            Self::Hungry => &["hungry"],
            Self::Starving => &["starving", "hnnng"],
            Self::Angry => &["feral", "alarm"],
            Self::Feral => &["feral"],
            Self::Scared => &["scared", "alarm", "awe"],
            Self::ChompOpen => &["chomp", "stuffed"],
        }
    }

    pub fn part_tokens(self) -> &'static [&'static str] {
        match self {
            Self::Happy => &["idle", "happy", "default"],
            Self::Blink => &["blink", "closed"],
            Self::Elated => &["smile", "elated", "joy"],
            Self::Hungry => &["hungry"],
            Self::Starving => &["starving", "grimace"],
            Self::Angry => &["angry", "mad", "ravenous"],
            Self::Feral => &["feral", "ravenous"],
            Self::Scared => &["scared", "alarm", "fear"],
            Self::ChompOpen => &["chomp", "open"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoodCompositeSpec {
    pub id: IdentifiableId,
    pub map_path: PathBuf,
    pub mask_path: Option<PathBuf>,
    pub ramp_r_path: Option<PathBuf>,
    pub ramp_g_path: Option<PathBuf>,
    pub ramp_b_path: Option<PathBuf>,
    pub fallback_rgb: [u8; 3],
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
    pub food_composites: Vec<FoodCompositeSpec>,
    pub face_full_textures: HashMap<SlimeExpr, Handle<Image>>,
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
    pub icon_reticle: Option<Handle<Image>>,

    // Original 3D meshes (.obj normalized for actors/items, or raw paths for slime alignment)
    pub item_meshes: HashMap<IdentifiableId, Handle<Mesh>>,
    pub slime_obj_paths: HashMap<String, PathBuf>,
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

    /// Picks the best available UI icon for an identifiable.
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
    let mut keys = vec![csharp.clone(), compact.clone()];
    let parts: Vec<&str> = csharp.split('_').collect();
    if parts.len() == 2 {
        keys.push(format!("{}_{}", parts[1], parts[0]));
        keys.push(format!("{}{}", parts[1], parts[0]));
    }
    for suffix in ["_veggie", "_fruit"] {
        if let Some(base) = csharp.strip_suffix(suffix) {
            keys.push(base.to_string());
        }
    }
    keys
}

fn exact_model_name(id: IdentifiableId) -> Option<&'static str> {
    match id {
        IdentifiableId::CarrotVeggie => Some("model_carrot"),
        IdentifiableId::OcaocaVeggie => Some("model_ocaoca"),
        IdentifiableId::BeetVeggie => Some("model_heartbeet"),
        IdentifiableId::ParsnipVeggie => Some("model_parsnip"),
        IdentifiableId::OnionVeggie => Some("model_oddonion"),
        IdentifiableId::GingerVeggie => Some("model_ginger"),
        IdentifiableId::PogoFruit => Some("model_pogofruit"),
        IdentifiableId::CuberryFruit => Some("model_cuberry"),
        IdentifiableId::MangoFruit => Some("model_mintmango"),
        IdentifiableId::PearFruit => Some("model_pricklepear"),
        IdentifiableId::LemonFruit | IdentifiableId::LemonPhase => Some("model_phaselemon"),
        IdentifiableId::Hen
        | IdentifiableId::BriarHen
        | IdentifiableId::StonyHen
        | IdentifiableId::ElderHen
        | IdentifiableId::PaintedHen
        | IdentifiableId::Rooster
        | IdentifiableId::ElderRooster => Some("birdhen"),
        IdentifiableId::Chick
        | IdentifiableId::BriarChick
        | IdentifiableId::StonyChick
        | IdentifiableId::PaintedChick => Some("birdchick"),
        _ if id.is_plort() => Some("plort"),
        _ if id.is_standard_crate() => Some("objcrate"),
        _ => None,
    }
}

fn exact_texture_name(id: IdentifiableId) -> Option<&'static str> {
    match id {
        IdentifiableId::Hen
        | IdentifiableId::ElderHen
        | IdentifiableId::Rooster
        | IdentifiableId::ElderRooster => Some("birdhen"),
        IdentifiableId::BriarHen | IdentifiableId::PaintedHen => Some("birdhenbriar"),
        IdentifiableId::StonyHen => Some("birdhenstony"),
        IdentifiableId::Chick | IdentifiableId::PaintedChick => Some("birdchick"),
        IdentifiableId::BriarChick => Some("birdchickbriar"),
        IdentifiableId::StonyChick => Some("birdchickstony"),
        _ => None,
    }
}

/// Checks if a PNG decal has a transparent corner (`alpha < 25`), so opaque RGB face exports
/// won't accidentally draw black boxes over slimes.
fn png_has_transparent_bg(path: &Path) -> bool {
    let Ok(img) = image::open(path) else {
        return false;
    };
    let rgba = img.to_rgba8();
    if rgba.width() == 0 || rgba.height() == 0 {
        return false;
    }
    rgba.get_pixel(0, 0)[3] < 25
}

fn sample_ramp_center(path: &Path) -> Option<[u8; 3]> {
    let img = image::open(path).ok()?.to_rgb8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let p = img.get_pixel(w / 2, h / 2);
    Some([p[0], p[1], p[2]])
}

/// Composes original `*_map.png` + `*_mask.png` + `*_ramp_*.png` into a vivid RGBA Bevy `Image`.
pub fn build_food_composite_image(spec: &FoodCompositeSpec) -> Option<Image> {
    let map_img = image::open(&spec.map_path).ok()?.to_rgb8();
    let (w, h) = map_img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }

    let mask_img = spec
        .mask_path
        .as_deref()
        .and_then(|p| image::open(p).ok())
        .map(|i| i.resize_exact(w, h, image::imageops::FilterType::Triangle).to_rgb8());

    let c_r = spec
        .ramp_r_path
        .as_deref()
        .and_then(sample_ramp_center)
        .unwrap_or(spec.fallback_rgb);
    let c_g = spec
        .ramp_g_path
        .as_deref()
        .and_then(sample_ramp_center)
        .unwrap_or([72, 188, 72]);
    let c_b = spec
        .ramp_b_path
        .as_deref()
        .and_then(sample_ramp_center)
        .unwrap_or(spec.fallback_rgb);

    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mp = map_img.get_pixel(x, y);
            // Convert map texture to crisp AO/detail shade
            let luma = (0.299 * mp[0] as f32 + 0.587 * mp[1] as f32 + 0.114 * mp[2] as f32) / 255.0;
            let shade = 0.42 + 0.58 * luma;

            let tint = if let Some(ref mk) = mask_img {
                let m = mk.get_pixel(x, y);
                let wr = m[0] as f32 / 255.0;
                let wg = m[1] as f32 / 255.0;
                let wb = m[2] as f32 / 255.0;
                let wsum = wr + wg + wb;
                if wsum > 0.05 {
                    let inv = 1.0 / wsum.max(1.0);
                    let base_w = (1.0 - wsum).max(0.0);
                    [
                        (c_r[0] as f32 * wr + c_g[0] as f32 * wg + c_b[0] as f32 * wb) * inv
                            + spec.fallback_rgb[0] as f32 * base_w,
                        (c_r[1] as f32 * wr + c_g[1] as f32 * wg + c_b[1] as f32 * wb) * inv
                            + spec.fallback_rgb[1] as f32 * base_w,
                        (c_r[2] as f32 * wr + c_g[2] as f32 * wg + c_b[2] as f32 * wb) * inv
                            + spec.fallback_rgb[2] as f32 * base_w,
                    ]
                } else {
                    [
                        spec.fallback_rgb[0] as f32,
                        spec.fallback_rgb[1] as f32,
                        spec.fallback_rgb[2] as f32,
                    ]
                }
            } else {
                [
                    spec.fallback_rgb[0] as f32,
                    spec.fallback_rgb[1] as f32,
                    spec.fallback_rgb[2] as f32,
                ]
            };

            out.push((tint[0] * shade).round().clamp(0.0, 255.0) as u8);
            out.push((tint[1] * shade).round().clamp(0.0, 255.0) as u8);
            out.push((tint[2] * shade).round().clamp(0.0, 255.0) as u8);
            out.push(255);
        }
    }

    Some(Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        out,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

pub fn load_client_assets(asset_server: &AssetServer, initial_lang: Language) -> ClientAssets {
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
    let mut icon_reticle = None;
    let mut item_icons = HashMap::new();
    let mut item_textures = HashMap::new();
    let mut food_composites = Vec::new();
    let mut face_full_textures = HashMap::new();
    let mut face_eyes_textures = HashMap::new();
    let mut face_mouth_textures = HashMap::new();
    let mut item_meshes = HashMap::new();
    let mut slime_obj_paths = HashMap::new();
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

    if let Some(ref root) = local_root {
        let load_repeat_exact_or_scored = |exact: &[&str], fallback: &[&str]| -> Option<Handle<Image>> {
            let rel = exact
                .iter()
                .find_map(|k| manifest.find_exact_texture(k))
                .or_else(|| manifest.find_diffuse_texture(fallback))?;
            Some(asset_server.load_with_settings(
                asset_rel_path(rel),
                |s: &mut ImageLoaderSettings| {
                    s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        ..default()
                    });
                },
            ))
        };
        let load_tex_exact_or_icon = |exact: &[&str], fallback: &[&str]| -> Option<Handle<Image>> {
            let rel = exact
                .iter()
                .find_map(|k| manifest.find_exact_texture(k))
                .or_else(|| manifest.find_icon_texture(fallback))?;
            Some(asset_server.load(asset_rel_path(rel)))
        };
        let load_aud_exact_or_scored =
            |exact: &[&str], fallback: &[&str]| -> Option<Handle<AudioSource>> {
                let rel = exact
                    .iter()
                    .find_map(|k| manifest.find_exact_audio(k))
                    .or_else(|| manifest.find_audio(fallback))?;
                Some(asset_server.load(asset_rel_path(rel)))
            };
        let load_obj_unit = |exact: Option<&str>, fallback: &[&str]| -> Option<Handle<Mesh>> {
            let rel = exact
                .and_then(|k| manifest.find_exact_model(k))
                .or_else(|| manifest.find_model(fallback))?;
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
        };

        // Environment tiling textures (exact matches from AssetStudio dump)
        ground_texture = load_repeat_exact_or_scored(
            &["envgrass01", "envgrassquar01"],
            &["ranch_grass", "grass_01", "grass"],
        );
        water_texture = load_repeat_exact_or_scored(
            &["water_01", "dpt_waterwaves"],
            &["water", "pond_water"],
        );
        wall_texture = load_repeat_exact_or_scored(
            &["dpt_objrockwall01", "envrocky01"],
            &["stone_wall", "rock_wall", "stone"],
        );
        soil_texture = load_repeat_exact_or_scored(
            &["envsoil01"],
            &["garden_soil", "soil", "dirt"],
        );
        wood_texture = load_repeat_exact_or_scored(
            &["wood_housekit", "objwoodkit01"],
            &["wood", "plank"],
        );

        // HUD UI icons (exact 1:1 AssetStudio keys)
        icon_newbucks = load_tex_exact_or_icon(&["iconnewbuck"], &["iconnewbucks", "newbucks"]);
        icon_key = load_tex_exact_or_icon(&["map_counter_slimekey"], &["iconslimekey", "slimekey"]);
        icon_health = load_tex_exact_or_icon(&["health_heart"], &["iconhealth", "health", "heart"]);
        icon_energy = load_tex_exact_or_icon(&["energy_bolt"], &["iconenergy", "energy", "bolt"]);
        icon_rads = load_tex_exact_or_icon(&["rad_symbol"], &["iconrad", "rad"]);
        icon_sun = load_tex_exact_or_icon(&["notif_day"], &["iconsun", "sun"]);
        icon_moon = load_tex_exact_or_icon(&["notif_moon"], &["iconmoon", "moon"]);
        icon_reticle = load_tex_exact_or_icon(&["reticlecircle"], &["reticle"]);

        // Original combined slime faces (`face_default_*.png`) validated for transparent alpha
        for &expr in SlimeExpr::ALL {
            if let Some(rel) = manifest.find_full_face_texture("face_default_", expr.full_face_tokens()) {
                let full_path = root.join(rel);
                if png_has_transparent_bg(&full_path) {
                    face_full_textures.insert(expr, asset_server.load(asset_rel_path(rel)));
                }
            }
            if let Some(rel) = manifest.find_face_texture(true, expr.part_tokens()) {
                let full_path = root.join(rel);
                if png_has_transparent_bg(&full_path) {
                    face_eyes_textures.insert(expr, asset_server.load(asset_rel_path(rel)));
                }
            }
            if let Some(rel) = manifest.find_face_texture(false, expr.part_tokens()) {
                let full_path = root.join(rel);
                if png_has_transparent_bg(&full_path) {
                    face_mouth_textures.insert(expr, asset_server.load(asset_rel_path(rel)));
                }
            }
        }

        // Collect raw `.obj` paths for aligned slime body & accessory loading
        for model_key in [
            "slime_default",
            "slime_puddle_lod0",
            "slime_gordo",
            "slime_tarr",
            "slime_saber",
            "slime_gold",
            "slime_quicksilver",
            "slime_face_lod0",
            "ears_n_tail_lod0",
            "hunter_ears_n_tail_lod0",
            "rocks_spine_lod0",
            "crystals_spine_lod0",
            "wing_l",
            "wing_r",
            "dervishring_lod0",
            "tangleflower_lod0",
            "luckycat_coin_lod0",
            "quicksilvercrest",
            "mosaic_lod0",
        ] {
            if let Some(rel) = manifest.find_exact_model(model_key) {
                slime_obj_paths.insert(model_key.to_string(), root.join(rel));
            }
        }

        // Food composite map+mask+ramp definitions
        let food_defs: &[(IdentifiableId, &str, &str, &str, &str, &str, [u8; 3])] = &[
            (
                IdentifiableId::CarrotVeggie,
                "veggiecarrot_map",
                "veggiecarrot_mask",
                "veggiecarrot_ramp_orange",
                "veggiecarrot_ramp_green",
                "veggiecarrot_ramp_dirt",
                [245, 130, 32],
            ),
            (
                IdentifiableId::OcaocaVeggie,
                "veggieocaoca_map",
                "veggieocaoca_mask",
                "veggieocaoca_ramp_red",
                "veggieocaoca_ramp_yellow",
                "veggieocaoca_ramp_brown",
                [202, 122, 74],
            ),
            (
                IdentifiableId::BeetVeggie,
                "veggieheartbeet_map",
                "veggieheartbeet_mask",
                "veggieheartbeet_ramp_red",
                "veggieheartbeet_ramp_green",
                "veggieheartbeet_ramp_blue",
                [198, 42, 88],
            ),
            (
                IdentifiableId::ParsnipVeggie,
                "veggieparsnip_map",
                "veggieparsnip_mask",
                "veggieparsnip_ramp_silver",
                "veggieparsnip_ramp_purple",
                "veggieparsnip_ramp_blue",
                [238, 224, 168],
            ),
            (
                IdentifiableId::PogoFruit,
                "fruitpogo_map",
                "fruitpogo_mask",
                "fruitpogo_ramp_orange",
                "fruitpogo_ramp_green",
                "fruitpogo_ramp_red",
                [248, 162, 68],
            ),
            (
                IdentifiableId::CuberryFruit,
                "fruitcuberry_map",
                "fruitcuberry_mask",
                "fruitcuberry_ramp_red",
                "fruitcuberry_ramp_stem",
                "fruitcuberry_ramp_red",
                [235, 68, 102],
            ),
            (
                IdentifiableId::MangoFruit,
                "fruitmintmango_map",
                "fruitmintmango_mask",
                "fruitmintmango_ramp_lgreen",
                "fruitmintmango_ramp_dgreen",
                "fruitmintmango_ramp_white",
                [142, 224, 122],
            ),
            (
                IdentifiableId::PearFruit,
                "fruitpricklepear_map",
                "fruitpricklepear_mask",
                "fruitpricklepear_ramp_purple",
                "fruitpricklepear_ramp_green",
                "fruitpricklepear_ramp_purple",
                [182, 222, 72],
            ),
            (
                IdentifiableId::LemonFruit,
                "fruitphaselemon_map",
                "fruitphaselemon_mask",
                "fruitphaselemon_ramp_yellow",
                "fruitphaselemon_ramp_cyan",
                "fruitphaselemon_ramp_yellow",
                [250, 236, 58],
            ),
        ];
        for &(id, map_k, mask_k, rr_k, rg_k, rb_k, fallback_rgb) in food_defs {
            if let Some(map_rel) = manifest.find_exact_texture(map_k) {
                food_composites.push(FoodCompositeSpec {
                    id,
                    map_path: root.join(map_rel),
                    mask_path: manifest.find_exact_texture(mask_k).map(|r| root.join(r)),
                    ramp_r_path: manifest.find_exact_texture(rr_k).map(|r| root.join(r)),
                    ramp_g_path: manifest.find_exact_texture(rg_k).map(|r| root.join(r)),
                    ramp_b_path: manifest.find_exact_texture(rb_k).map(|r| root.join(r)),
                    fallback_rgb,
                });
            }
        }

        // Per-item UI icons, 3D diffuse textures (never for slimes/plorts!), and 3D `.obj` meshes
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

            if !id.is_slime() && !id.is_plort() {
                let tex_refs: Vec<&str> = base_keys.iter().map(String::as_str).collect();
                if let Some(exact_tex) = exact_texture_name(id).and_then(|k| manifest.find_exact_texture(k)) {
                    item_textures.insert(id, asset_server.load(asset_rel_path(exact_tex)));
                } else if let Some(rel) = manifest.find_diffuse_texture(&tex_refs) {
                    item_textures.insert(id, asset_server.load(asset_rel_path(rel)));
                }
                if let Some(mesh_h) = load_obj_unit(exact_model_name(id), &tex_refs) {
                    item_meshes.insert(id, mesh_h);
                }
            } else if id.is_plort() {
                if let Some(mesh_h) = load_obj_unit(Some("plort"), &["plort"]) {
                    item_meshes.insert(id, mesh_h);
                }
            }
        }

        plort_base_mesh = load_obj_unit(Some("plort"), &["plort"]);
        vacpack_mesh = load_obj_unit(Some("vac"), &["vac", "vacpac"]);

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

        // Exact 1:1 SFX & Music from `audio/AudioClip/*.wav`
        sfx_vac_start = load_aud_exact_or_scored(&["vac_ammo_slowfield_shoot_05"], &["vac_start", "vac"]);
        sfx_vac_capture = load_aud_exact_or_scored(&["vac_end_01"], &["vac_end", "vac_capture"]);
        sfx_vac_shoot = load_aud_exact_or_scored(&["vac_ammo_slowfield_shoot_05"], &["vac_shoot"]);
        sfx_jump = load_aud_exact_or_scored(&["saber_wiggle_02"], &["jump"]);
        sfx_damage = load_aud_exact_or_scored(&["tarr_fear_02"], &["damage", "fear"]);
        sfx_plort_sell = load_aud_exact_or_scored(&["plort_deposit_08"], &["plort_deposit", "newbucks"]);
        sfx_airburst = load_aud_exact_or_scored(&["quicksilver_hit_success_01"], &["airburst", "hit_success"]);

        sfx_slime_jump = load_aud_exact_or_scored(&["saber_wiggle_02"], &["slime_jump", "wiggle"]);
        sfx_slime_chomp = load_aud_exact_or_scored(&["gordo_gulp_01", "slime_glitchtarr_chomp_02"], &["gulp", "chomp"]);
        sfx_slime_plort = load_aud_exact_or_scored(&["hit_toy_09"], &["plort", "toy"]);
        for exact in [
            "vo_fireslime_happymix_03",
            "vo_waterslime_happy_mix_03",
            "vo_slime_alarm_01",
            "tarr_fun_01",
            "hen_05",
        ] {
            if let Some(rel) = manifest.find_exact_audio(exact) {
                sfx_slime_voices.push(asset_server.load(asset_rel_path(rel)));
            }
        }

        sfx_ui_click = load_aud_exact_or_scored(&["ui_drone_activate"], &["ui_click", "drone_activate"]);
        sfx_ui_open = load_aud_exact_or_scored(&["viklab_computer_teleport_in_01"], &["ui_open", "teleport_in"]);
        sfx_ui_buy = load_aud_exact_or_scored(&["gadget_cloner_success_01"], &["ui_buy", "success"]);
        sfx_ui_error = load_aud_exact_or_scored(&["gadget_cloner_fail_04"], &["ui_error", "fail"]);

        bgm_ranch_day = load_aud_exact_or_scored(&["musicranchwistful"], &["musicranch", "ranch_day"]);
        bgm_ranch_night = load_aud_exact_or_scored(&["musicquarryrelax", "musicradiosunrise"], &["musicquarry", "ranch_night"]);
        bgm_tarr = load_aud_exact_or_scored(&["musicviktorvrquarry", "tarrslime_idle_04"], &["tarr", "viktor"]);

        info!(
            "Matched original assets -> icons: {}, 3D textures: {}, food composites: {}, faces(full/eyes/mouth): {}/{}/{}, slime objs: {}, 3D meshes: {}, slime voices: {}, BGM: {}/{}/{}",
            item_icons.len(),
            item_textures.len(),
            food_composites.len(),
            face_full_textures.len(),
            face_eyes_textures.len(),
            face_mouth_textures.len(),
            slime_obj_paths.len(),
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
        food_composites,
        face_full_textures,
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
        icon_reticle,
        item_meshes,
        slime_obj_paths,
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
    sim_res: Res<crate::world_sync::SimResource>,
    assets: Res<ClientAssets>,
    mut audio_state: ResMut<AudioState>,
) {
    let t = time.elapsed_secs();
    let model = &sim_res.sim.model;
    let has_tarr = model
        .actors
        .values()
        .any(|a| a.ident == IdentifiableId::TarrSlime);

    let desired = if has_tarr && assets.bgm_tarr.is_some() {
        ActiveBgmTrack::Tarr
    } else {
        let hour = TimeDirector::hour_of_day(model.world.world_time);
        if !(6.0..19.5).contains(&hour) && assets.bgm_ranch_night.is_some() {
            ActiveBgmTrack::Night
        } else if assets.bgm_ranch_day.is_some() {
            ActiveBgmTrack::Day
        } else {
            ActiveBgmTrack::None
        }
    };

    if desired != audio_state.current_track {
        if let Some(old_ent) = audio_state.bgm_entity.take() {
            commands.entity(old_ent).despawn_recursive();
        }
        let next_handle = match desired {
            ActiveBgmTrack::Day => assets.bgm_ranch_day.clone(),
            ActiveBgmTrack::Night => assets.bgm_ranch_night.clone(),
            ActiveBgmTrack::Tarr => assets.bgm_tarr.clone(),
            ActiveBgmTrack::None => None,
        };
        if let Some(h) = next_handle {
            let ent = commands
                .spawn((
                    AudioPlayer::new(h),
                    PlaybackSettings::LOOP.with_volume(Volume::new(0.28)),
                ))
                .id();
            audio_state.bgm_entity = Some(ent);
        }
        audio_state.current_track = desired;
    }

    let ev = &sim_res.last_events;
    if !ev.ai.eat.eaten_pairs.is_empty() && (t - audio_state.last_slime_chomp_t) > 0.25 {
        audio_state.last_slime_chomp_t = t;
        play_sfx(&mut commands, assets.sfx_slime_chomp.as_ref(), 0.45);
    }
    if !ev.ai.eat.spawned_plorts.is_empty() && (t - audio_state.last_slime_plort_t) > 0.25 {
        audio_state.last_slime_plort_t = t;
        play_sfx(&mut commands, assets.sfx_slime_plort.as_ref(), 0.40);
    }
    if !ev.ai.slime_jumps.is_empty() && (t - audio_state.last_slime_jump_t) > 0.20 {
        audio_state.last_slime_jump_t = t;
        play_sfx(&mut commands, assets.sfx_slime_jump.as_ref(), 0.30);
    }
    if (ev.ai.player_damage > 0 || ev.ai.player_fire_damage > 0)
        && (t - audio_state.last_damage_t) > 0.45
    {
        audio_state.last_damage_t = t;
        play_sfx(&mut commands, assets.sfx_damage.as_ref(), 0.55);
    }
    if !assets.sfx_slime_voices.is_empty()
        && !model.actors.is_empty()
        && (t - audio_state.last_slime_voice_t) > 4.2
    {
        audio_state.last_slime_voice_t = t;
        let idx = audio_state.voice_cursor % assets.sfx_slime_voices.len();
        audio_state.voice_cursor = audio_state.voice_cursor.wrapping_add(1);
        play_sfx(&mut commands, Some(&assets.sfx_slime_voices[idx]), 0.22);
    }
}
