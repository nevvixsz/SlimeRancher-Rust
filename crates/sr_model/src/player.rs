//! Port of `PlayerModel.cs` and `PlayerState.UpgradeLockData`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{AmmoMode, PlayerUpgrade, RegionSetId, ZoneId};
use sr_math::{Quat, Vec3};

use crate::ammo::AmmoModel;

pub const DEFAULT_MAX_HEALTH: i32 = 100;
pub const DEFAULT_MAX_ENERGY: i32 = 100;
pub const DEFAULT_MAX_RADS: i32 = 100;
pub const RAD_UNIT_DAMAGE: i32 = 10;
pub const DEFAULT_MAX_AMMO: [i32; 5] = [20, 30, 40, 50, 100];

pub const ENERGY_RECOVERED_PER_SECOND: f32 = 50.0;
pub const HEALTH_RECOVERED_PER_SECOND: f32 = 2.0;
pub const RADS_RECOVERED_PER_SECOND: f32 = 25.0;
pub const ENERGY_RECOVERY_DELAY: f64 = 5.0;
pub const FULL_RAD_RECOVERY_DELAY: f64 = 10.0;
pub const NONFULL_RAD_RECOVERY_DELAY: f64 = 5.0;
pub const HEALTH_BURST_AMOUNT: f32 = 10.0;
pub const HEALTH_BURST_RATE: f32 = 50.0;

/// Port of `PlayerState.UpgradeLockData`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UpgradeLockData {
    pub timed_lock: bool,
    pub locked_until: f64,
}

/// Port of `PlayerModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerModel {
    pub curr_region_set_id: RegionSetId,
    pub position: Vec3,
    pub rotation: Quat,

    pub ammo_mode: AmmoMode,
    pub ammo_dict: HashMap<AmmoMode, AmmoModel>,

    pub upgrades: Vec<PlayerUpgrade>,
    pub avail_upgrades: Vec<PlayerUpgrade>,
    pub upgrade_locks: HashMap<PlayerUpgrade, UpgradeLockData>,

    pub curr_energy: f32,
    pub curr_health: f32,
    pub curr_rads: f32,
    pub currency: i32,
    pub currency_ever_collected: i32,
    pub currency_display_override: Option<i32>,
    pub currency_display_delta: i32,
    pub keys: i32,

    pub rad_recover_after: f64,
    pub energy_recover_after: f64,
    pub health_burst_after: f64,
    pub curr_health_burst_remaining: f32,

    pub unlocked_zone_maps: HashSet<ZoneId>,

    pub max_health: i32,
    pub max_energy: i32,
    pub max_rads: i32,
    pub max_ammo: i32,
    pub end_game_time: Option<f64>,

    pub has_jetpack: bool,
    pub jetpack_efficiency: f32,
    pub run_efficiency: f32,
    pub has_air_burst: bool,
    pub run_energy_depletion_time: f64,
}

impl Default for PlayerModel {
    fn default() -> Self {
        let mut ammo_dict = HashMap::new();
        ammo_dict.insert(AmmoMode::Default, AmmoModel::new(5, 4));
        ammo_dict.insert(AmmoMode::NimbleValley, AmmoModel::new(3, 3));

        let mut unlocked_zone_maps = HashSet::new();
        unlocked_zone_maps.insert(ZoneId::Ranch);

        Self {
            curr_region_set_id: RegionSetId::Home,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            ammo_mode: AmmoMode::Default,
            ammo_dict,
            upgrades: Vec::new(),
            avail_upgrades: Vec::new(),
            upgrade_locks: HashMap::new(),
            curr_energy: DEFAULT_MAX_ENERGY as f32,
            curr_health: DEFAULT_MAX_HEALTH as f32,
            curr_rads: 0.0,
            currency: 0,
            currency_ever_collected: 0,
            currency_display_override: None,
            currency_display_delta: 0,
            keys: 0,
            rad_recover_after: f64::INFINITY,
            energy_recover_after: f64::INFINITY,
            health_burst_after: f64::INFINITY,
            curr_health_burst_remaining: 0.0,
            unlocked_zone_maps,
            max_health: DEFAULT_MAX_HEALTH,
            max_energy: DEFAULT_MAX_ENERGY,
            max_rads: DEFAULT_MAX_RADS,
            max_ammo: DEFAULT_MAX_AMMO[0],
            end_game_time: None,
            has_jetpack: false,
            jetpack_efficiency: 1.0,
            run_efficiency: 1.0,
            has_air_burst: false,
            run_energy_depletion_time: 0.0,
        }
    }
}

impl PlayerModel {
    pub fn reset(&mut self, settings: &GameModeSettings) {
        self.reset_for_game_mode(settings);
        self.max_energy = DEFAULT_MAX_ENERGY;
        self.max_health = DEFAULT_MAX_HEALTH;
        self.max_rads = DEFAULT_MAX_RADS;
        self.max_ammo = DEFAULT_MAX_AMMO[0];
        self.curr_energy = self.max_energy as f32;
        self.curr_health = self.max_health as f32;
        self.curr_rads = 0.0;
        self.upgrades.clear();
        self.keys = 0;
        self.has_jetpack = false;
        self.jetpack_efficiency = 1.0;
        self.run_efficiency = 1.0;
        self.has_air_burst = false;
        if let Some(a) = self.ammo_dict.get_mut(&AmmoMode::Default) {
            a.reset(5, 4);
        }
        if let Some(a) = self.ammo_dict.get_mut(&AmmoMode::NimbleValley) {
            a.reset(3, 3);
        }
    }

    pub fn reset_for_game_mode(&mut self, settings: &GameModeSettings) {
        self.currency = settings.init_currency;
        self.currency_ever_collected = settings.init_currency;
        self.end_game_time = settings.end_time();
    }

    pub fn set_energy(&mut self, energy: f32, world_time: f64) {
        self.curr_energy = energy.clamp(0.0, self.max_energy as f32);
        if self.curr_energy < self.max_energy as f32 {
            // C# uses 300.0 game-seconds (= 5s real time at 60x, ENERGY_RECOVERY_DELAY * 60)
            self.energy_recover_after = self.energy_recover_after.min(world_time + 300.0);
        }
    }

    /// Consumes energy and updates `energy_recover_after`.
    pub fn spend_energy(&mut self, amount: f32, world_time: f64) -> bool {
        if self.curr_energy <= 0.0 && amount > 0.0 {
            return false;
        }
        self.curr_energy = (self.curr_energy - amount).max(0.0);
        self.energy_recover_after = world_time + ENERGY_RECOVERY_DELAY * 60.0;
        true
    }

    pub fn set_health(&mut self, health: f32, world_time: f64) {
        self.curr_health = health.clamp(0.0, self.max_health as f32);
        if self.curr_health < self.max_health as f32 {
            self.health_burst_after = self.health_burst_after.min(world_time + 300.0);
        }
    }

    pub fn damage(&mut self, amount: f32, world_time: f64) {
        self.curr_health = (self.curr_health - amount).max(0.0);
        self.health_burst_after = world_time + 300.0;
    }

    /// Port of `PlayerModel.AddRads`. Returns integer health damage caused by rad overflow.
    pub fn add_rads(&mut self, rads: f32, world_time: f64) -> i32 {
        self.curr_rads += rads;
        let delay_mins = if self.curr_rads >= self.max_rads as f32 {
            FULL_RAD_RECOVERY_DELAY
        } else {
            NONFULL_RAD_RECOVERY_DELAY
        };
        self.rad_recover_after = world_time + 60.0 * delay_mins;

        if self.curr_rads > self.max_rads as f32 {
            let units = ((self.curr_rads - self.max_rads as f32) / RAD_UNIT_DAMAGE as f32).floor() as i32;
            if units > 0 {
                let dmg = RAD_UNIT_DAMAGE * units;
                self.curr_rads -= dmg as f32;
                return dmg;
            }
        }
        0
    }

    /// Port of `PlayerModel.Recover(dt)` using real-time `dt` and current `world_time`.
    pub fn recover(&mut self, dt: f32, world_time: f64) {
        self.maybe_recover_health(dt, world_time);
        if world_time >= self.energy_recover_after {
            self.curr_energy =
                (self.curr_energy + dt * ENERGY_RECOVERED_PER_SECOND).min(self.max_energy as f32);
        }
        if world_time >= self.rad_recover_after {
            self.curr_rads = (self.curr_rads - dt * RADS_RECOVERED_PER_SECOND).max(0.0);
        }
    }

    fn maybe_recover_health(&mut self, dt: f32, world_time: f64) {
        if self.curr_health < self.max_health as f32 {
            if self.curr_health_burst_remaining <= 0.0
                && self.curr_health.ceil() - self.curr_health < 0.001
            {
                self.curr_health_burst_remaining = self.curr_health.ceil() - self.curr_health;
            }
            if self.curr_health_burst_remaining > 0.0 {
                let step = self
                    .curr_health_burst_remaining
                    .min(dt * HEALTH_BURST_RATE);
                self.curr_health_burst_remaining =
                    (self.curr_health_burst_remaining - step).max(0.0);
                self.curr_health = (self.curr_health + step).min(self.max_health as f32);
                return;
            }
            if world_time >= self.health_burst_after {
                self.health_burst_after += 300.0;
                self.curr_health_burst_remaining = HEALTH_BURST_AMOUNT;
            }
        } else {
            self.health_burst_after = f64::INFINITY;
            self.curr_health_burst_remaining = 0.0;
        }
    }

    /// Port of `PlayerModel.ApplyUpgrade`.
    pub fn apply_upgrade(&mut self, upgrade: PlayerUpgrade, is_first_time: bool, world_time: f64) {
        match upgrade {
            PlayerUpgrade::Health1 => {
                self.max_health = self.max_health.max(150);
                if self.curr_health < self.max_health as f32 {
                    self.health_burst_after = self.health_burst_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Health2 => {
                self.max_health = self.max_health.max(200);
                if self.curr_health < self.max_health as f32 {
                    self.health_burst_after = self.health_burst_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Health3 => {
                self.max_health = self.max_health.max(250);
                if self.curr_health < self.max_health as f32 {
                    self.health_burst_after = self.health_burst_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Health4 => {
                self.max_health = self.max_health.max(350);
                if self.curr_health < self.max_health as f32 {
                    self.health_burst_after = self.health_burst_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Energy1 => {
                self.max_energy = self.max_energy.max(150);
                if self.curr_energy < self.max_energy as f32 {
                    self.energy_recover_after = self.energy_recover_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Energy2 => {
                self.max_energy = self.max_energy.max(200);
                if self.curr_energy < self.max_energy as f32 {
                    self.energy_recover_after = self.energy_recover_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Energy3 => {
                self.max_energy = self.max_energy.max(250);
                if self.curr_energy < self.max_energy as f32 {
                    self.energy_recover_after = self.energy_recover_after.min(world_time + 300.0);
                }
            }
            PlayerUpgrade::Ammo1 => self.max_ammo = DEFAULT_MAX_AMMO[1],
            PlayerUpgrade::Ammo2 => self.max_ammo = DEFAULT_MAX_AMMO[2],
            PlayerUpgrade::Ammo3 => self.max_ammo = DEFAULT_MAX_AMMO[3],
            PlayerUpgrade::Ammo4 => self.max_ammo = DEFAULT_MAX_AMMO[4],
            PlayerUpgrade::Jetpack => self.has_jetpack = true,
            PlayerUpgrade::JetpackEfficiency => {
                self.jetpack_efficiency = self.jetpack_efficiency.min(0.8);
            }
            PlayerUpgrade::AirBurst => self.has_air_burst = true,
            PlayerUpgrade::RunEfficiency => {
                self.run_efficiency = self.run_efficiency.min(0.667);
            }
            PlayerUpgrade::RunEfficiency2 => {
                self.run_efficiency = self.run_efficiency.min(0.5);
            }
            PlayerUpgrade::LiquidSlot => {
                if let Some(ammo) = self.ammo_dict.get_mut(&AmmoMode::Default) {
                    ammo.increase_usable_slots(5);
                }
            }
            PlayerUpgrade::SpareKey => {
                if is_first_time {
                    self.keys += 1;
                }
            }
            PlayerUpgrade::GoldenSureshot
            | PlayerUpgrade::TreasureCracker1
            | PlayerUpgrade::TreasureCracker2
            | PlayerUpgrade::TreasureCracker3
            | PlayerUpgrade::TreasureCracker4 => {}
        }
    }

    pub fn active_ammo(&self) -> Option<&AmmoModel> {
        self.ammo_dict.get(&self.ammo_mode)
    }

    pub fn active_ammo_mut(&mut self) -> Option<&mut AmmoModel> {
        self.ammo_dict.get_mut(&self.ammo_mode)
    }
}
