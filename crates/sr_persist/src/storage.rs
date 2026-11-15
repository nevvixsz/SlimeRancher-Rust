//! File storage provider and AutoSaveDirector (matching C# `FileStorageProvider` & `AutoSaveDirector`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sr_model::GameModel;

use crate::binary::{PersistError, PersistResult};
use crate::game_v12::{GameV12, LoadedSummary};

pub const SAVE_EXTENSION: &str = ".sav";
pub const TEMP_SUFFIX: &str = ".tmp";
pub const DEFAULT_AUTOSAVE_INTERVAL_SECS: f64 = 1440.0; // 24 game minutes / 24 real mins in C#
pub const DEFAULT_MAX_BACKUPS: usize = 5;

/// Metadata for a discovered save file on disk.
#[derive(Debug, Clone)]
pub struct SaveSlotInfo {
    pub game_name: String,
    pub path: PathBuf,
    pub summary: LoadedSummary,
}

/// Abstract save storage backend (`StorageProvider.cs`).
pub trait StorageProvider {
    fn has_game_data(&self, game_name: &str) -> bool;
    fn store_game_data(&self, game_name: &str, bytes: &[u8]) -> PersistResult<()>;
    fn load_game_data(&self, game_name: &str) -> PersistResult<Vec<u8>>;
    fn delete_game_data(&self, game_name: &str) -> PersistResult<()>;
    fn get_available_games(&self) -> PersistResult<Vec<String>>;
}

/// Disk storage provider storing `<game_name>.sav` atomically via `<game_name>.sav.tmp`
/// and keeping optional rotating backups (`<game_name>.sav.bak1..N`).
#[derive(Debug, Clone)]
pub struct FileStorageProvider {
    save_dir: PathBuf,
    max_backups: usize,
}

impl FileStorageProvider {
    pub fn new(save_dir: impl Into<PathBuf>) -> Self {
        Self {
            save_dir: save_dir.into(),
            max_backups: DEFAULT_MAX_BACKUPS,
        }
    }

    pub fn with_max_backups(mut self, max_backups: usize) -> Self {
        self.max_backups = max_backups;
        self
    }

    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    pub fn get_game_file(&self, game_name: &str) -> PathBuf {
        self.save_dir.join(format!("{game_name}{SAVE_EXTENSION}"))
    }

    fn get_temp_file(&self, game_name: &str) -> PathBuf {
        self.save_dir
            .join(format!("{game_name}{SAVE_EXTENSION}{TEMP_SUFFIX}"))
    }

    fn get_backup_file(&self, game_name: &str, index: usize) -> PathBuf {
        self.save_dir
            .join(format!("{game_name}{SAVE_EXTENSION}.bak{index}"))
    }

    fn rotate_backups(&self, game_name: &str) -> io::Result<()> {
        if self.max_backups == 0 {
            return Ok(());
        }
        let target = self.get_game_file(game_name);
        if !target.exists() {
            return Ok(());
        }

        // Shift bak(N-1) -> bak(N)
        for i in (1..self.max_backups).rev() {
            let src = self.get_backup_file(game_name, i);
            let dst = self.get_backup_file(game_name, i + 1);
            if src.exists() {
                fs::copy(&src, &dst)?;
            }
        }
        let bak1 = self.get_backup_file(game_name, 1);
        fs::copy(&target, &bak1)?;
        Ok(())
    }

    /// Reads only the summary header of a save without parsing the world/actors body.
    pub fn load_summary(&self, game_name: &str) -> PersistResult<SaveSlotInfo> {
        let path = self.get_game_file(game_name);
        let bytes = fs::read(&path)?;
        let summary = GameV12::load_summary_from_bytes(&bytes)?;
        Ok(SaveSlotInfo {
            game_name: game_name.to_string(),
            path,
            summary,
        })
    }

    /// Lists all valid `.sav` files in `save_dir` with their parsed summaries.
    pub fn list_saves_with_summaries(&self) -> PersistResult<Vec<SaveSlotInfo>> {
        let names = self.get_available_games()?;
        let mut out = Vec::new();
        for name in names {
            if let Ok(info) = self.load_summary(&name) {
                out.push(info);
            }
        }
        out.sort_by(|a, b| {
            b.summary
                .summary
                .save_timestamp_ticks
                .cmp(&a.summary.summary.save_timestamp_ticks)
        });
        Ok(out)
    }

    /// Standard Slime Rancher save directory (`%USERPROFILE%\AppData\LocalLow\Monomi Park\Slime Rancher` on Windows).
    pub fn default_slime_rancher_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("USERPROFILE").map(|home| {
                PathBuf::from(home)
                    .join("AppData")
                    .join("LocalLow")
                    .join("Monomi Park")
                    .join("Slime Rancher")
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home)
                    .join(".config")
                    .join("unity3d")
                    .join("Monomi Park")
                    .join("Slime Rancher")
            })
        }
    }
}

impl StorageProvider for FileStorageProvider {
    fn has_game_data(&self, game_name: &str) -> bool {
        self.get_game_file(game_name).is_file()
    }

    fn store_game_data(&self, game_name: &str, bytes: &[u8]) -> PersistResult<()> {
        fs::create_dir_all(&self.save_dir)?;
        self.rotate_backups(game_name)?;

        let tmp = self.get_temp_file(game_name);
        let target = self.get_game_file(game_name);

        fs::write(&tmp, bytes)?;
        fs::copy(&tmp, &target)?;
        let _ = fs::remove_file(&tmp);
        Ok(())
    }

    fn load_game_data(&self, game_name: &str) -> PersistResult<Vec<u8>> {
        let path = self.get_game_file(game_name);
        if !path.is_file() {
            return Err(PersistError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("save file not found: {}", path.display()),
            )));
        }
        Ok(fs::read(path)?)
    }

    fn delete_game_data(&self, game_name: &str) -> PersistResult<()> {
        let path = self.get_game_file(game_name);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn get_available_games(&self) -> PersistResult<Vec<String>> {
        if !self.save_dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.save_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if let Some(stem) = name.strip_suffix(SAVE_EXTENSION) {
                        out.push(stem.to_string());
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

/// Manages autosave timer, monotonic save numbers, and save/load orchestration (`AutoSaveDirector.cs`).
#[derive(Debug, Clone)]
pub struct AutoSaveDirector<S: StorageProvider> {
    pub storage: S,
    pub autosave_interval_secs: f64,
    pub time_until_next_autosave: f64,
    pub next_save_number: u64,
}

impl<S: StorageProvider> AutoSaveDirector<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            autosave_interval_secs: DEFAULT_AUTOSAVE_INTERVAL_SECS,
            time_until_next_autosave: DEFAULT_AUTOSAVE_INTERVAL_SECS,
            next_save_number: 1,
        }
    }

    /// Advances the autosave countdown (`dt` in real seconds). Returns `true` when an autosave should trigger.
    pub fn tick(&mut self, dt_secs: f64) -> bool {
        if dt_secs <= 0.0 || self.autosave_interval_secs <= 0.0 {
            return false;
        }
        self.time_until_next_autosave -= dt_secs;
        if self.time_until_next_autosave <= 0.0 {
            self.time_until_next_autosave = self.autosave_interval_secs;
            true
        } else {
            false
        }
    }

    pub fn reset_timer(&mut self) {
        self.time_until_next_autosave = self.autosave_interval_secs;
    }

    /// Saves `GameModel` as a `GameV12` `.sav` file and resets the autosave timer.
    pub fn save_game(
        &mut self,
        game_name: &str,
        display_name: &str,
        model: &GameModel,
        save_timestamp_ticks: i64,
    ) -> PersistResult<u64> {
        let save_num = self.next_save_number;
        let v12 = GameV12::from_game_model(
            model,
            game_name,
            display_name,
            save_num,
            save_timestamp_ticks,
        );
        let bytes = v12.to_bytes()?;
        self.storage.store_game_data(game_name, &bytes)?;
        self.next_save_number = save_num + 1;
        self.reset_timer();
        Ok(save_num)
    }

    /// Loads a `GameV12` `.sav` file into a fresh `GameModel`.
    pub fn load_game(&mut self, game_name: &str) -> PersistResult<(GameV12, GameModel)> {
        let bytes = self.storage.load_game_data(game_name)?;
        let v12 = GameV12::from_bytes(&bytes)?;
        if v12.summary.save_number >= self.next_save_number {
            self.next_save_number = v12.summary.save_number + 1;
        }
        let model = v12.into_game_model()?;
        self.reset_timer();
        Ok((v12, model))
    }
}
