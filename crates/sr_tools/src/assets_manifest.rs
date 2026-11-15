//! Scanner and manifest index for user-supplied local assets (`local_assets/`):
//! original Unity Texture2D (`.png`), AudioClip (`.wav`/`.ogg`), and Mesh (`.glb`/`.gltf`/`.fbx`).
//! Supports both AssetStudio default subfolders (`local_assets/textures/Texture2D/...`,
//! `local_assets/Texture2D/...`, `local_assets/AudioClip/...`) and flat layouts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::localization::ToolsResult;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Normalized lowercase stem -> relative path inside `local_assets/`.
    #[serde(default)]
    pub textures: BTreeMap<String, String>,
    /// Normalized lowercase stem -> relative path inside `local_assets/`.
    #[serde(default)]
    pub audio: BTreeMap<String, String>,
    /// Normalized lowercase stem -> relative path inside `local_assets/`.
    #[serde(default)]
    pub models: BTreeMap<String, String>,
    /// Language code -> relative path to `localization_<lang>.json`.
    #[serde(default)]
    pub localization: BTreeMap<String, String>,
}

fn strip_asset_studio_suffix(stem: &str) -> &str {
    match stem.find(" #") {
        Some(idx) => &stem[..idx],
        None => stem,
    }
}

fn insert_asset_keys(map: &mut BTreeMap<String, String>, stem: &str, rel: String) {
    let lower = stem.to_ascii_lowercase();
    let clean = strip_asset_studio_suffix(&lower).trim().to_string();
    map.insert(lower.clone(), rel.clone());
    if !clean.is_empty() && clean != lower {
        map.entry(clean).or_insert(rel);
    }
}

impl AssetManifest {
    /// Recursively scans `root` (`local_assets/`) and indexes supported files by lowercase stem.
    /// Works identically whether files are in `local_assets/textures/Texture2D/`,
    /// `local_assets/Texture2D/`, `local_assets/audio/AudioClip/`, or `local_assets/AudioClip/`.
    pub fn scan(root: &Path) -> ToolsResult<Self> {
        let mut manifest = Self::default();
        if !root.exists() {
            return Ok(manifest);
        }
        let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !path.is_file() {
                    continue;
                }
                let rel = match path.strip_prefix(root) {
                    Ok(p) => p.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                match ext.as_str() {
                    "png" | "jpg" | "jpeg" | "tga" | "webp" => {
                        insert_asset_keys(&mut manifest.textures, stem, rel);
                    }
                    "wav" | "ogg" | "mp3" | "flac" => {
                        insert_asset_keys(&mut manifest.audio, stem, rel);
                    }
                    "glb" | "gltf" | "fbx" | "obj" => {
                        insert_asset_keys(&mut manifest.models, stem, rel);
                    }
                    "json" => {
                        let lower = stem.to_ascii_lowercase();
                        if lower.starts_with("localization_") {
                            let lang = lower.trim_start_matches("localization_").to_string();
                            manifest.localization.insert(lang, rel);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(manifest)
    }

    pub fn find_texture(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map(&self.textures, candidates)
    }

    pub fn find_audio(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map(&self.audio, candidates)
    }

    pub fn find_model(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map(&self.models, candidates)
    }

    pub fn save_json(&self, path: &Path) -> ToolsResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn load_json(path: &Path) -> ToolsResult<Self> {
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }
}

fn find_in_map<'a>(map: &'a BTreeMap<String, String>, candidates: &[&str]) -> Option<&'a str> {
    for &cand in candidates {
        let key = cand.to_ascii_lowercase();
        if let Some(v) = map.get(&key) {
            return Some(v.as_str());
        }
    }
    for &cand in candidates {
        let needle = cand.to_ascii_lowercase();
        if let Some((_, v)) = map.iter().find(|(k, _)| k.contains(&needle)) {
            return Some(v.as_str());
        }
    }
    None
}
