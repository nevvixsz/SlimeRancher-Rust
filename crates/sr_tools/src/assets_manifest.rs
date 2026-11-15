//! Scanner and manifest index for user-supplied local assets (`local_assets/`):
//! original Unity Texture2D (`.png`), AudioClip (`.wav`/`.ogg`), and Mesh (`.glb`/`.gltf`/`.fbx`).

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

impl AssetManifest {
    /// Recursively scans `root` (`local_assets/`) and indexes supported files by lowercase stem.
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
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                match ext.as_str() {
                    "png" | "jpg" | "jpeg" | "tga" | "webp" => {
                        manifest.textures.insert(stem, rel);
                    }
                    "wav" | "ogg" | "mp3" | "flac" => {
                        manifest.audio.insert(stem, rel);
                    }
                    "glb" | "gltf" | "fbx" | "obj" => {
                        manifest.models.insert(stem, rel);
                    }
                    "json" if stem.starts_with("localization_") => {
                        let lang = stem.trim_start_matches("localization_").to_string();
                        manifest.localization.insert(lang, rel);
                    }
                    _ => {}
                }
            }
        }
        Ok(manifest)
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
