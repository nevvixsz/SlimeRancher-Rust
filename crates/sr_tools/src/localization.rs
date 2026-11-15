//! Parser and importer for Slime Rancher Java `.properties` localization files
//! exported from Unity `TextAsset` (`achieve`, `actor`, `exchange`, `global`, `mail`, `pedia`, `range`, `tutorial`, `ui`).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LOCALIZATION_BUNDLES: &[&str] = &[
    "achieve",
    "actor",
    "exchange",
    "global",
    "mail",
    "pedia",
    "range",
    "tutorial",
    "ui",
];

#[derive(Debug, Error)]
pub enum ToolsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("persist error: {0}")]
    Persist(#[from] sr_persist::PersistError),
    #[error("missing path: {0}")]
    MissingPath(String),
}

pub type ToolsResult<T> = Result<T, ToolsError>;

/// Full localization dictionary for one language (`en`, `ru`, ...), grouped by bundle name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizationSet {
    pub language: String,
    pub bundles: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(skip)]
    flat: HashMap<String, String>,
}

impl LocalizationSet {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            bundles: BTreeMap::new(),
            flat: HashMap::new(),
        }
    }

    pub fn rebuild_index(&mut self) {
        self.flat.clear();
        for (bundle, entries) in &self.bundles {
            for (k, v) in entries {
                self.flat.insert(format!("{bundle}.{k}"), v.clone());
                self.flat.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }

    pub fn total_entries(&self) -> usize {
        self.bundles.values().map(|m| m.len()).sum()
    }

    /// Looks up a key either as `"bundle.key"` (e.g. `"ui.t.pedia"`) or bare `"key"` (`"t.pedia"`).
    pub fn get(&self, key: &str) -> Option<&str> {
        if let Some(v) = self.flat.get(key) {
            return Some(v.as_str());
        }
        if let Some((bundle, subkey)) = key.split_once('.') {
            if let Some(v) = self.bundles.get(bundle).and_then(|b| b.get(subkey)) {
                return Some(v.as_str());
            }
        }
        None
    }

    /// Formats a localized string replacing `{0}`, `{1}`, ... placeholders.
    pub fn format(&self, key: &str, args: &[&str]) -> String {
        let mut out = self
            .get(key)
            .map(ToString::to_string)
            .unwrap_or_else(|| key.to_string());
        for (idx, arg) in args.iter().enumerate() {
            let needle = format!("{{{idx}}}");
            out = out.replace(&needle, arg);
        }
        out
    }

    /// Loads a `TextAsset/` directory containing files named `achieve`, `actor`, `ui`, etc.
    /// (with or without `.txt` / `.properties` suffix).
    pub fn load_from_text_asset_dir(
        language: impl Into<String>,
        dir: &Path,
    ) -> ToolsResult<Self> {
        let mut set = Self::new(language);
        if !dir.is_dir() {
            return Err(ToolsError::MissingPath(dir.display().to_string()));
        }
        for &bundle in LOCALIZATION_BUNDLES {
            let candidates = [
                dir.join(bundle),
                dir.join(format!("{bundle}.txt")),
                dir.join(format!("{bundle}.properties")),
            ];
            if let Some(path) = candidates.into_iter().find(|p| p.is_file()) {
                let raw = fs::read_to_string(&path)?;
                let parsed = parse_properties(&raw);
                set.bundles.insert(bundle.to_string(), parsed);
            }
        }
        set.rebuild_index();
        Ok(set)
    }

    pub fn save_json(&self, path: &Path) -> ToolsResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_json(path: &Path) -> ToolsResult<Self> {
        let raw = fs::read_to_string(path)?;
        let mut set: Self = serde_json::from_str(&raw)?;
        set.rebuild_index();
        Ok(set)
    }
}

/// Parses Java `.properties` text as exported by Slime Rancher (`MessageDirector`).
pub fn parse_properties(input: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let input = input.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0usize;

    while i < lines.len() {
        let mut line = lines[i].trim_start();
        i += 1;

        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        // Accumulate logical line across trailing backslash continuations (`\`).
        let mut logical = String::new();
        loop {
            let trimmed_end = line.trim_end_matches('\r');
            if let Some(prefix) = strip_trailing_continuation(trimmed_end) {
                logical.push_str(prefix);
                if i >= lines.len() {
                    break;
                }
                line = lines[i].trim_start();
                i += 1;
            } else {
                logical.push_str(trimmed_end);
                break;
            }
        }

        if let Some((raw_key, raw_val)) = split_kv(&logical) {
            let key = unescape_properties(raw_key.trim());
            let val = unescape_properties(raw_val.trim_start());
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }

    map
}

fn strip_trailing_continuation(s: &str) -> Option<&str> {
    let backslash_count = s.chars().rev().take_while(|&c| c == '\\').count();
    if backslash_count % 2 == 1 {
        Some(&s[..s.len() - 1])
    } else {
        None
    }
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '=' || ch == ':' {
            return Some((&line[..idx], &line[idx + 1..]));
        }
    }
    None
}

fn unescape_properties(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('=') => out.push('='),
            Some(':') => out.push(':'),
            Some('u') => {
                let mut hex = String::with_capacity(4);
                for _ in 0..4 {
                    if let Some(&h) = chars.peek() {
                        if h.is_ascii_hexdigit() {
                            hex.push(h);
                            chars.next();
                        }
                    }
                }
                if hex.len() == 4 {
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                            continue;
                        }
                    }
                }
                out.push_str("\\u");
                out.push_str(&hex);
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Imports an AssetStudio export directory (like `decompile/` containing `TextAsset/` and `en/TextAsset/`)
/// into `out_dir/localization_ru.json` and `out_dir/localization_en.json`.
pub fn import_decompile_localization(src_root: &Path, out_dir: &Path) -> ToolsResult<Vec<String>> {
    let mut written = Vec::new();
    let candidates = [
        ("ru", src_root.join("TextAsset")),
        ("en", src_root.join("en").join("TextAsset")),
        ("ru", src_root.join("decompile").join("TextAsset")),
        ("en", src_root.join("decompile").join("en").join("TextAsset")),
    ];

    let mut seen_langs = std::collections::HashSet::new();
    for (lang, dir) in candidates {
        if dir.is_dir() && seen_langs.insert(lang) {
            let set = LocalizationSet::load_from_text_asset_dir(lang, &dir)?;
            let out_path = out_dir.join(format!("localization_{lang}.json"));
            set.save_json(&out_path)?;
            written.push(format!(
                "{lang}: {} entries -> {}",
                set.total_entries(),
                out_path.display()
            ));
        }
    }

    if written.is_empty() {
        return Err(ToolsError::MissingPath(format!(
            "no TextAsset localization directory found under {}",
            src_root.display()
        )));
    }
    Ok(written)
}
