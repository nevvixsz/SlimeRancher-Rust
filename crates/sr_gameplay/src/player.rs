//! Port of `PlayerState.cs`, `PlayerRadAbsorber.cs`, and `PlayerDeathHandler.cs`.

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{AmmoMode, PlayerUpgrade, RegionSetId};
use sr_math::{Quat, Vec3};
use sr_model::PlayerModel;
use sr_physics::PlayerMotor;

/// Port of `DeathHandler.Source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeathSource {
    SlimeAttack,
    SlimeRad,
    TarrBite,
    Water,
    CliffFall,
    FeralBite,
    Fire,
    EmergencyReturn,
    Other,
}

/// Summary produced when `PlayerController::on_death` triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeathOutcome {
    pub source: DeathSource,
    pub currency_lost: i32,
    pub wake_up_world_time: f64,
    pub wake_up_pos: Vec3,
}

/// Stateful helper operating on `PlayerModel` + `PlayerMotor` (`PlayerState`, `PlayerRadAbsorber`, `PlayerDeathHandler`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerController {
    pub in_gadget_mode: bool,
    pub absorbing_rads_this_frame: bool,
    pub wake_up_pos: Vec3,
    pub wake_up_region: RegionSetId,
    pub death_in_progress: bool,
    pub death_count: i32,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            in_gadget_mode: false,
            absorbing_rads_this_frame: false,
            wake_up_pos: Vec3::new(0.0, 0.0, 0.0),
            wake_up_region: RegionSetId::Home,
            death_in_progress: false,
            death_count: 0,
        }
    }
}

impl PlayerController {
    pub fn add_currency(model: &mut PlayerModel, adjust: i32) {
        model.currency += adjust;
        if adjust > 0 {
            model.currency_ever_collected += adjust;
        }
    }

    pub fn spend_currency(model: &mut PlayerModel, amount: i32) -> bool {
        if amount < 0 || model.currency < amount {
            return false;
        }
        model.currency -= amount;
        true
    }

    /// Port of `PlayerState.Damage`. Returns `true` if health hit 0.
    pub fn damage(
        model: &mut PlayerModel,
        settings: &GameModeSettings,
        raw_loss: i32,
        world_time: f64,
    ) -> bool {
        let scaled = ((raw_loss as f32) * settings.player_damage_multiplier).round() as i32;
        if scaled <= 0 {
            return false;
        }
        model.damage(scaled as f32, world_time);
        if model.curr_health <= 0.0 {
            model.curr_health = 0.0;
            model.health_burst_after = f64::INFINITY;
            true
        } else {
            false
        }
    }

    pub fn heal(model: &mut PlayerModel, gain: i32, world_time: f64) {
        let target = (model.curr_health + gain as f32).min(model.max_health as f32);
        model.set_health(target, world_time);
    }

    /// Port of `PlayerRadAbsorber.Absorb`. Returns `true` if rad overflow killed the player.
    pub fn absorb_rads(
        &mut self,
        model: &mut PlayerModel,
        settings: &GameModeSettings,
        rads: f32,
        world_time: f64,
    ) -> bool {
        self.absorbing_rads_this_frame = true;
        let overflow_dmg = model.add_rads(rads, world_time);
        if overflow_dmg > 0 {
            Self::damage(model, settings, overflow_dmg, world_time)
        } else {
            false
        }
    }

    /// Grants an upgrade if not already owned, applying all stat/slot effects.
    pub fn grant_upgrade(model: &mut PlayerModel, upgrade: PlayerUpgrade, world_time: f64) -> bool {
        if model.upgrades.contains(&upgrade) {
            return false;
        }
        model.upgrades.push(upgrade);
        model.avail_upgrades.retain(|&u| u != upgrade);
        model.apply_upgrade(upgrade, true, world_time);
        true
    }

    /// Returns the Treasure Cracker tier (`0..=4`) currently unlocked by the player.
    pub fn treasure_cracker_level(model: &PlayerModel) -> i32 {
        if model.upgrades.contains(&PlayerUpgrade::TreasureCracker4) {
            4
        } else if model.upgrades.contains(&PlayerUpgrade::TreasureCracker3) {
            3
        } else if model.upgrades.contains(&PlayerUpgrade::TreasureCracker2) {
            2
        } else if model.upgrades.contains(&PlayerUpgrade::TreasureCracker1) {
            1
        } else {
            0
        }
    }

    /// Port of `PlayerDeathHandler.OnDeath` + `ResetPlayer`.
    /// Computes exact wake-up world_time (`GetNextDawnAfterNextDusk` or `+hours_lost_on_death`),
    /// deducts currency penalty, clears vacpack ammo, and restores player to ranch wake-up point.
    pub fn handle_death(
        &mut self,
        model: &mut PlayerModel,
        motor: &mut PlayerMotor,
        settings: &GameModeSettings,
        source: DeathSource,
        world_time: &mut f64,
    ) -> DeathOutcome {
        self.death_count += 1;

        let target_time = if settings.hours_til_dawn_on_death {
            next_dawn_after_next_dusk(*world_time)
        } else {
            *world_time + (settings.hours_lost_on_death as f64) * 3600.0
        };
        let wake_up_world_time = target_time.max(*world_time + 390.0);

        let mut currency_lost = 0;
        if settings.pct_currency_lost_on_death > 0.0 {
            currency_lost = ((model.currency as f32) * settings.pct_currency_lost_on_death).floor() as i32;
            Self::spend_currency(model, currency_lost);
        }

        // Reset vitals, position, velocity, and clear vacpack ammo
        *world_time = wake_up_world_time;
        model.curr_health = model.max_health as f32;
        model.curr_rads = 0.0;
        model.curr_energy = model.max_energy as f32;
        model.rad_recover_after = f64::INFINITY;
        model.energy_recover_after = f64::INFINITY;
        model.health_burst_after = f64::INFINITY;
        model.curr_health_burst_remaining = 0.0;
        model.ammo_mode = AmmoMode::Default;
        model.position = self.wake_up_pos;
        model.rotation = Quat::IDENTITY;
        model.curr_region_set_id = self.wake_up_region;

        for ammo in model.ammo_dict.values_mut() {
            for slot in &mut ammo.slots {
                *slot = None;
            }
        }
        motor.stop();
        motor.grounded = true;
        motor.in_sea_kill_zone = false;

        DeathOutcome {
            source,
            currency_lost,
            wake_up_world_time,
            wake_up_pos: self.wake_up_pos,
        }
    }
}

fn next_dawn_after_next_dusk(from_time: f64) -> f64 {
    const SECS_PER_DAY: f64 = 86400.0;
    let frac = ((from_time % SECS_PER_DAY) / SECS_PER_DAY) as f32;
    let hour = frac * 24.0;
    let full_days = if (6.0..=18.0).contains(&hour) { 0 } else { 1 };
    let target_frac = 6.0f32 / 24.0;
    let delta_days = if frac < target_frac {
        target_frac - frac + full_days as f32
    } else {
        target_frac - frac + full_days as f32 + 1.0
    };
    from_time + (delta_days as f64) * SECS_PER_DAY
}
