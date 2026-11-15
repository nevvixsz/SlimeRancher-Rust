//! Local asset discovery (`local_assets/`), embedded RU/EN localization, and audio/texture hooks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::audio::Volume;
use bevy::prelude::*;
use sr_ids::IdentifiableId;
use sr_tools::{AssetManifest, LocalizationSet};

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

#[derive(Resource)]
pub struct ClientAssets {
    pub local_root: Option<PathBuf>,
    pub manifest: AssetManifest,
    pub language: Language,
    pub loc_ru: LocalizationSet,
    pub loc_en: LocalizationSet,
    pub ui_font: Handle<Font>,
    pub item_textures: HashMap<IdentifiableId, Handle<Image>>,
    pub ground_texture: Option<Handle<Image>>,
    pub water_texture: Option<Handle<Image>>,
    pub wall_texture: Option<Handle<Image>>,
    pub sfx_vac_capture: Option<Handle<AudioSource>>,
    pub sfx_vac_shoot: Option<Handle<AudioSource>>,
    pub sfx_jump: Option<Handle<AudioSource>>,
    pub sfx_plort_sell: Option<Handle<AudioSource>>,
    pub sfx_airburst: Option<Handle<AudioSource>>,
    pub bgm_ranch: Option<Handle<AudioSource>>,
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
        // Fallback: prettified C# name
        id.as_csharp_name().replace('_', " ")
    }

    pub fn ui_text<'a>(&'a self, key: &'a str, fallback: &'a str) -> &'a str {
        self.active_loc().get(key).unwrap_or(fallback)
    }
}

pub fn find_local_assets_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("local_assets"),
        PathBuf::from("../local_assets"),
        PathBuf::from("../../local_assets"),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

fn resolve_manifest_path(root: &Path, rel: &str) -> PathBuf {
    let p = root.join(rel);
    std::fs::canonicalize(&p).unwrap_or(p)
}

fn load_font(asset_server: &AssetServer, local_root: Option<&Path>) -> Handle<Font> {
    let mut font_candidates: Vec<PathBuf> = Vec::new();
    if let Some(root) = local_root {
        font_candidates.push(root.join("fonts/ui.ttf"));
        font_candidates.push(root.join("fonts/SegoeUI.ttf"));
    }
    font_candidates.push(PathBuf::from("assets/fonts/ui.ttf"));
    #[cfg(target_os = "windows")]
    {
        font_candidates.push(PathBuf::from(r"C:\Windows\Fonts\segoeui.ttf"));
        font_candidates.push(PathBuf::from(r"C:\Windows\Fonts\arial.ttf"));
        font_candidates.push(PathBuf::from(r"C:\Windows\Fonts\tahoma.ttf"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        font_candidates.push(PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"));
        font_candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
    }

    for path in font_candidates {
        if path.is_file() {
            let abs = std::fs::canonicalize(&path).unwrap_or(path);
            info!("Loaded UI font: {}", abs.display());
            return asset_server.load(abs);
        }
    }
    Handle::default()
}

pub fn load_client_assets(
    asset_server: &AssetServer,
    initial_lang: Language,
) -> ClientAssets {
    let local_root = find_local_assets_dir();
    let manifest = if let Some(ref root) = local_root {
        let m_path = root.join("manifest.json");
        if m_path.is_file() {
            AssetManifest::load_json(&m_path).unwrap_or_default()
        } else {
            let scanned = AssetManifest::scan(root).unwrap_or_default();
            info!(
                "Scanned {}: {} textures, {} audio, {} models",
                root.display(),
                scanned.textures.len(),
                scanned.audio.len(),
                scanned.models.len()
            );
            scanned
        }
    } else {
        AssetManifest::default()
    };

    let loc_ru =
        LocalizationSet::load_with_override("ru", local_root.as_deref()).unwrap_or_default();
    let loc_en =
        LocalizationSet::load_with_override("en", local_root.as_deref()).unwrap_or_default();

    let ui_font = load_font(asset_server, local_root.as_deref());

    let mut ground_texture = None;
    let mut water_texture = None;
    let mut wall_texture = None;
    let mut item_textures = HashMap::new();

    let mut sfx_vac_capture = None;
    let mut sfx_vac_shoot = None;
    let mut sfx_jump = None;
    let mut sfx_plort_sell = None;
    let mut sfx_airburst = None;
    let mut bgm_ranch = None;

    if let Some(ref root) = local_root {
        let load_tex = |candidates: &[&str]| -> Option<Handle<Image>> {
            manifest
                .find_texture(candidates)
                .map(|rel| asset_server.load(resolve_manifest_path(root, rel)))
        };
        let load_aud = |candidates: &[&str]| -> Option<Handle<AudioSource>> {
            manifest
                .find_audio(candidates)
                .map(|rel| asset_server.load(resolve_manifest_path(root, rel)))
        };

        ground_texture = load_tex(&["grass", "terrain_grass", "ranch_ground", "dirt"]);
        water_texture = load_tex(&["water", "pond_water", "ocean"]);
        wall_texture = load_tex(&["stone", "rock_wall", "corral", "concrete"]);

        for &id in IdentifiableId::ALL {
            let csharp = id.as_csharp_name().to_ascii_lowercase();
            let compact = csharp.replace('_', "");
            let icon_prefixed = format!("icon{compact}");
            if let Some(h) = load_tex(&[&icon_prefixed, &csharp, &compact]) {
                item_textures.insert(id, h);
            }
        }

        sfx_vac_capture = load_aud(&["vac_capture", "vaccapture", "vac_suck", "vacin"]);
        sfx_vac_shoot = load_aud(&["vac_shoot", "vacshoot", "vacout", "shoot"]);
        sfx_jump = load_aud(&["jump", "player_jump", "boing"]);
        sfx_plort_sell = load_aud(&["plort_sell", "newbucks", "cash", "coin", "market"]);
        sfx_airburst = load_aud(&["airburst", "pulse", "shockwave"]);
        bgm_ranch = load_aud(&["ranch_day", "ranchmusic", "ranch", "day_01"]);
    }

    ClientAssets {
        local_root,
        manifest,
        language: initial_lang,
        loc_ru,
        loc_en,
        ui_font,
        item_textures,
        ground_texture,
        water_texture,
        wall_texture,
        sfx_vac_capture,
        sfx_vac_shoot,
        sfx_jump,
        sfx_plort_sell,
        sfx_airburst,
        bgm_ranch,
    }
}

/// Starts background ranch music if an audio clip was found in `local_assets/`.
pub fn start_bgm_system(mut commands: Commands, assets: Res<ClientAssets>) {
    if let Some(ref bgm) = assets.bgm_ranch {
        commands.spawn((
            AudioPlayer::new(bgm.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::new(0.28)),
        ));
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
