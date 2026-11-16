//! Scanner, fast cache, and smart manifest index for user-supplied local assets (`local_assets/`):
//! original Unity `Texture2D` (`.png`/`.tga`/`.webp`), `AudioClip` (`.wav`/`.ogg`/`.mp3`/`.flac`),
//! and `Mesh` (`.obj`/`.glb`/`.gltf`/`.fbx`).
//! Supports AssetStudio default subfolders (`local_assets/Texture2D/`, `local_assets/AudioClip/`,
//! `local_assets/Mesh/`, `local_assets/textures/...`) as well as flat layouts.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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
    /// Cached token -> keys index (rebuilt lazily in memory, not serialized).
    #[serde(skip)]
    texture_tokens: HashMap<String, Vec<String>>,
    #[serde(skip)]
    audio_tokens: HashMap<String, Vec<String>>,
    #[serde(skip)]
    model_tokens: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CachedManifestFile {
    fingerprint: String,
    manifest: AssetManifest,
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

/// Computes a fast directory fingerprint from `root` and immediate subdirectories
/// (`Texture2D`, `AudioClip`, `Mesh`, etc.) so `scan_cached` avoids re-walking 5,000+ files
/// when nothing changed on disk.
fn compute_dir_fingerprint(root: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let add_meta = |p: &Path, label: &str, out: &mut Vec<String>| {
        if let Ok(meta) = fs::metadata(p) {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push(format!("{label}:{mtime}"));
        }
    };
    add_meta(root, "root", &mut parts);
    if let Ok(entries) = fs::read_dir(root) {
        let mut subdirs: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.is_dir() {
                    let name = p.file_name()?.to_string_lossy().into_owned();
                    Some((name, p))
                } else {
                    None
                }
            })
            .collect();
        subdirs.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, p) in subdirs {
            add_meta(&p, &name, &mut parts);
        }
    }
    parts.join("|")
}

fn tokenize_key(key: &str) -> Vec<String> {
    let clean = strip_asset_studio_suffix(key);
    let mut tokens = Vec::new();
    let mut cur = String::new();
    for ch in clean.chars() {
        if ch.is_ascii_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            tokens.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

fn build_token_index(map: &BTreeMap<String, String>) -> HashMap<String, Vec<String>> {
    let mut idx: HashMap<String, Vec<String>> = HashMap::with_capacity(map.len() * 2);
    for key in map.keys() {
        for tok in tokenize_key(key) {
            if tok.len() >= 2 {
                idx.entry(tok).or_default().push(key.clone());
            }
        }
    }
    idx
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
                    "glb" | "gltf" | "obj" | "fbx" => {
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
        manifest.rebuild_indices();
        Ok(manifest)
    }

    /// Loads manifest from `local_assets/.sr_manifest_cache.json` if directory mtimes match,
    /// otherwise runs `scan(root)` and writes the cache for near-instant future launches.
    pub fn scan_cached(root: &Path) -> ToolsResult<Self> {
        let cache_path = root.join(".sr_manifest_cache.json");
        let fp = compute_dir_fingerprint(root);
        if cache_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&cache_path) {
                if let Ok(mut cached) = serde_json::from_str::<CachedManifestFile>(&raw) {
                    if cached.fingerprint == fp && !fp.is_empty() {
                        cached.manifest.rebuild_indices();
                        return Ok(cached.manifest);
                    }
                }
            }
        }
        let manifest = Self::scan(root)?;
        let cache = CachedManifestFile {
            fingerprint: fp,
            manifest: manifest.clone(),
        };
        if let Ok(json) = serde_json::to_string(&cache) {
            let _ = fs::write(&cache_path, json);
        }
        Ok(manifest)
    }

    pub fn rebuild_indices(&mut self) {
        self.texture_tokens = build_token_index(&self.textures);
        self.audio_tokens = build_token_index(&self.audio);
        self.model_tokens = build_token_index(&self.models);
    }

    pub fn find_texture(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map_scored(
            &self.textures,
            &self.texture_tokens,
            candidates,
            &["_normal", "_nrm", "_bump", "_mask", "_rough", "_spec", "_ao"],
            &[],
            false,
        )
    }

    /// Prefers UI icon sprites (`icon*`, `*_icon*`) and excludes PBR maps.
    pub fn find_icon_texture(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map_scored(
            &self.textures,
            &self.texture_tokens,
            candidates,
            &["_normal", "_nrm", "_bump", "_mask", "_rough", "_spec", "_ao", "_lut"],
            &["icon"],
            false,
        )
    }

    /// Prefers 3D albedo/diffuse textures (`_diffuse`, `_albedo`, `_color`, bare stem)
    /// and strictly excludes UI icons, sprites, face decals, and PBR maps.
    pub fn find_diffuse_texture(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map_scored_ex(
            &self.textures,
            &self.texture_tokens,
            candidates,
            &[
                "icon", "pedia", "sprite", "hud_", "ui_", "_normal", "_nrm", "_bump",
                "_mask", "_rough", "_spec", "_ao", "_lut", "_ramp", "_emissive", "_shadow",
                "eyes", "mouth", "face",
            ],
            &[],
            &["diffuse", "albedo", "color", "tex"],
            false,
        )
    }

    /// Finds original slime face textures (`eyes_happy`, `mouth_happy`, `slime_eyes_*`, etc.).
    pub fn find_face_texture(&self, is_eyes: bool, expr_tokens: &[&str]) -> Option<&str> {
        let part_tokens: &[&str] = if is_eyes {
            &["eye"]
        } else {
            &["mouth"]
        };
        let mut best: Option<(i32, &str)> = None;
        for (key, rel) in &self.textures {
            let clean = strip_asset_studio_suffix(key).trim();
            if clean.contains("icon") || clean.contains("normal") || clean.contains("mask") {
                continue;
            }
            if !part_tokens.iter().any(|p| clean.contains(p)) {
                continue;
            }
            for (idx, &expr) in expr_tokens.iter().enumerate() {
                let e = expr.to_ascii_lowercase();
                if clean.contains(&e) {
                    let mut score = 500 - (idx as i32) * 30 - (clean.len() as i32);
                    if clean.contains("slime") {
                        score += 80;
                    }
                    if best.map_or(true, |(bs, _)| score > bs) {
                        best = Some((score, rel.as_str()));
                    }
                }
            }
        }
        best.map(|(_, r)| r)
    }

    pub fn find_audio(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map_scored(
            &self.audio,
            &self.audio_tokens,
            candidates,
            &[],
            &[],
            false,
        )
    }

    /// Finds up to `limit` distinct matching audio paths (ordered by relevance score).
    pub fn find_audio_many(&self, candidates: &[&str], limit: usize) -> Vec<&str> {
        find_many_scored(&self.audio, &self.audio_tokens, candidates, &[], &[], limit)
    }

    /// Finds a loadable 3D model (`.glb`, `.gltf`, `.obj`, `.fbx`), preferring `.glb`/`.gltf`/`.obj`
    /// over `.fbx` and excluding `_lod` / `_collider` / `_shadow` / sub-part meshes.
    pub fn find_model(&self, candidates: &[&str]) -> Option<&str> {
        find_in_map_scored_ex(
            &self.models,
            &self.model_tokens,
            candidates,
            &[
                "_lod", "lod1", "lod2", "collider", "_shadow", "collision", "petal",
                "gordo", "fashion", "ornament",
            ],
            &[],
            &[],
            true,
        )
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
        let mut m: Self = serde_json::from_str(&raw)?;
        m.rebuild_indices();
        Ok(m)
    }
}

fn score_key_match(
    key: &str,
    rel: &str,
    candidate_lower: &str,
    penalize: &[&str],
    boost: &[&str],
    prefer_bevy_mesh_ext: bool,
) -> Option<i32> {
    let clean = strip_asset_studio_suffix(key).trim();
    let mut score = if clean == candidate_lower || key == candidate_lower {
        1000
    } else if clean.starts_with(candidate_lower) {
        750 - (clean.len() as i32 - candidate_lower.len() as i32).min(200)
    } else if clean.contains(candidate_lower) {
        500 - (clean.len() as i32 - candidate_lower.len() as i32).min(200)
    } else {
        // Multi-token match: e.g. "pink_slime" matches "slime_pink_diffuse"
        let cand_tokens = tokenize_key(candidate_lower);
        if cand_tokens.len() > 1 && cand_tokens.iter().all(|t| clean.contains(t.as_str())) {
            420 - (clean.len() as i32).min(150)
        } else {
            return None;
        }
    };

    for &bad in penalize {
        if clean.contains(bad) && !candidate_lower.contains(bad) {
            score -= 450;
        }
    }
    for &good in boost {
        if clean.contains(good) {
            score += 120;
        }
    }
    if prefer_bevy_mesh_ext {
        let r = rel.to_ascii_lowercase();
        if r.ends_with(".glb") || r.ends_with(".gltf") {
            score += 200;
        } else if r.ends_with(".obj") {
            score += 160;
        } else if r.ends_with(".fbx") {
            score -= 300;
        }
    }
    Some(score)
}

fn gather_candidates<'a>(
    map: &'a BTreeMap<String, String>,
    token_index: &HashMap<String, Vec<String>>,
    candidates: &[&str],
) -> Vec<&'a String> {
    let mut seen: Vec<&'a String> = Vec::new();
    for &cand in candidates {
        let lower = cand.to_ascii_lowercase();
        if let Some((k, _)) = map.get_key_value(&lower) {
            seen.push(k);
        }
        for tok in tokenize_key(&lower) {
            if let Some(list) = token_index.get(&tok) {
                for k in list {
                    if let Some((mk, _)) = map.get_key_value(k) {
                        seen.push(mk);
                    }
                }
            }
        }
    }
    if seen.is_empty() {
        // Fallback: full scan for partial substring matches (e.g., `iconSlimePink`)
        seen.extend(map.keys());
    }
    seen
}

fn find_in_map_scored<'a>(
    map: &'a BTreeMap<String, String>,
    token_index: &HashMap<String, Vec<String>>,
    candidates: &[&str],
    penalize: &[&str],
    boost: &[&str],
    prefer_bevy_mesh_ext: bool,
) -> Option<&'a str> {
    find_in_map_scored_ex(
        map,
        token_index,
        candidates,
        &[],
        penalize,
        boost,
        prefer_bevy_mesh_ext,
    )
}

fn find_in_map_scored_ex<'a>(
    map: &'a BTreeMap<String, String>,
    token_index: &HashMap<String, Vec<String>>,
    candidates: &[&str],
    exclude: &[&str],
    penalize: &[&str],
    boost: &[&str],
    prefer_bevy_mesh_ext: bool,
) -> Option<&'a str> {
    if map.is_empty() || candidates.is_empty() {
        return None;
    }

    let is_excluded = |key: &str, cand_lower: &str| -> bool {
        let clean = strip_asset_studio_suffix(key).trim();
        exclude
            .iter()
            .any(|&ex| clean.contains(ex) && !cand_lower.contains(ex))
    };

    // Fast exact lookup first
    for &cand in candidates {
        let key = cand.to_ascii_lowercase();
        if let Some(v) = map.get(&key) {
            if !is_excluded(&key, &key)
                && (!prefer_bevy_mesh_ext || !v.to_ascii_lowercase().ends_with(".fbx"))
            {
                return Some(v.as_str());
            }
        }
    }

    let pool = gather_candidates(map, token_index, candidates);
    let mut best: Option<(i32, &str)> = None;

    for (priority_idx, &cand) in candidates.iter().enumerate() {
        let cand_lower = cand.to_ascii_lowercase();
        let priority_bonus = ((candidates.len() - priority_idx) as i32) * 20;
        for &key in &pool {
            if is_excluded(key, &cand_lower) {
                continue;
            }
            if let Some(rel) = map.get(key) {
                if let Some(mut s) = score_key_match(
                    key,
                    rel,
                    &cand_lower,
                    penalize,
                    boost,
                    prefer_bevy_mesh_ext,
                ) {
                    s += priority_bonus;
                    if best.map_or(true, |(bs, _)| s > bs) {
                        best = Some((s, rel.as_str()));
                    }
                }
            }
        }
    }

    // If token index didn't find anything (e.g. concatenated names like `iconSlimePink`),
    // do a single substring pass across the map.
    if best.is_none() && !token_index.is_empty() {
        for (priority_idx, &cand) in candidates.iter().enumerate() {
            let cand_lower = cand.to_ascii_lowercase();
            let priority_bonus = ((candidates.len() - priority_idx) as i32) * 20;
            for (key, rel) in map {
                if is_excluded(key, &cand_lower) {
                    continue;
                }
                if let Some(mut s) = score_key_match(
                    key,
                    rel,
                    &cand_lower,
                    penalize,
                    boost,
                    prefer_bevy_mesh_ext,
                ) {
                    s += priority_bonus;
                    if best.map_or(true, |(bs, _)| s > bs) {
                        best = Some((s, rel.as_str()));
                    }
                }
            }
        }
    }

    best.map(|(_, rel)| rel)
}

fn find_many_scored<'a>(
    map: &'a BTreeMap<String, String>,
    token_index: &HashMap<String, Vec<String>>,
    candidates: &[&str],
    penalize: &[&str],
    boost: &[&str],
    limit: usize,
) -> Vec<&'a str> {
    if map.is_empty() || candidates.is_empty() || limit == 0 {
        return Vec::new();
    }
    let pool = gather_candidates(map, token_index, candidates);
    let mut scored: Vec<(i32, &str)> = Vec::new();

    for (priority_idx, &cand) in candidates.iter().enumerate() {
        let cand_lower = cand.to_ascii_lowercase();
        let priority_bonus = ((candidates.len() - priority_idx) as i32) * 20;
        for &key in &pool {
            if let Some(rel) = map.get(key) {
                if let Some(s) = score_key_match(key, rel, &cand_lower, penalize, boost, false) {
                    scored.push((s + priority_bonus, rel.as_str()));
                }
            }
        }
    }
    if scored.is_empty() && !token_index.is_empty() {
        for (priority_idx, &cand) in candidates.iter().enumerate() {
            let cand_lower = cand.to_ascii_lowercase();
            let priority_bonus = ((candidates.len() - priority_idx) as i32) * 20;
            for (key, rel) in map {
                if let Some(s) = score_key_match(key, rel, &cand_lower, penalize, boost, false) {
                    scored.push((s + priority_bonus, rel.as_str()));
                }
            }
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let mut out: Vec<&'a str> = Vec::with_capacity(limit);
    for (_, rel) in scored {
        if !out.contains(&rel) {
            out.push(rel);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}
