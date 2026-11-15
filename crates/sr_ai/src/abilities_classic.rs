//! Classic slime unique abilities:
//! - `RockSlimeRoll`
//! - `BoomSlimeExplode`
//! - `RadSlimeExpand`
//! - `CrystalSlimeLaunch` (and active crystal spike hazards)
//! - `Phosphor` (`SlimeHover` + `DestroyOutsideHoursOfDay` sunlight fade)

use serde::{Deserialize, Serialize};
use sr_data::DataRegistry;
use sr_ids::IdentifiableId;
use sr_math::{Randoms, Vec3};
use sr_physics::PhysicsWorld;

pub const ROCK_SPIN_TIME: f32 = 1.0;
pub const ROCK_ROLL_TIME: f32 = 2.0;
pub const ROCK_COOLDOWN: f32 = 5.0;
pub const ROCK_ROLL_DAMAGE: f32 = 10.0;

pub const BOOM_MIN_DELAY: f32 = 8.0;
pub const BOOM_MAX_DELAY: f32 = 18.0;
pub const BOOM_EXPLODE_POWER: f32 = 600.0;
pub const BOOM_EXPLODE_RADIUS: f32 = 7.0;
pub const BOOM_MIN_DAMAGE: f32 = 15.0;
pub const BOOM_MAX_DAMAGE: f32 = 45.0;

pub const RAD_MIN_DELAY: f32 = 30.0;
pub const RAD_MAX_DELAY: f32 = 180.0;
pub const RAD_EXPANDING_TIME: f32 = 3.0;
pub const RAD_EXPANDED_TIME: f32 = 10.0;
pub const RAD_CALMING_TIME: f32 = 0.2;
pub const RAD_EXPAND_FACTOR: f32 = 1.5;
pub const RAD_BASE_RADIUS: f32 = 3.5;

pub const CRYSTAL_MIN_DELAY: f32 = 8.0;
pub const CRYSTAL_MAX_DELAY: f32 = 22.0;
pub const CRYSTAL_SPIKES_SPREAD: f32 = 1.5;
pub const CRYSTAL_CLUSTER_LIFETIME: f32 = 25.0;
pub const CRYSTAL_DAMAGE: f32 = 20.0;

pub const PHOSPHOR_HOVER_MIN_DELAY: f32 = 10.0;
pub const PHOSPHOR_HOVER_MAX_DELAY: f32 = 25.0;
pub const PHOSPHOR_HOVER_TIME: f32 = 6.0;
pub const PHOSPHOR_HOVER_HEIGHT: f32 = 5.0;
pub const PHOSPHOR_DAYLIGHT_BURN_SECS: f32 = 8.0;

/// Checks whether `ident` is `base` or a Largo whose `base_slimes` includes `base`.
pub fn slime_has_trait(
    registry: &DataRegistry,
    ident: IdentifiableId,
    base: IdentifiableId,
) -> bool {
    if ident == base {
        return true;
    }
    registry
        .slimes
        .get(ident)
        .map_or(false, |d| d.base_slimes.contains(&base))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RockRollPhase {
    #[default]
    Idle,
    Spinning,
    Rolling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RadPhase {
    #[default]
    Idle,
    Expanding,
    Expanded,
    Calming,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrystalCluster {
    pub pos: Vec3,
    pub radius: f32,
    pub expires_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassicAbilityState {
    // Rock
    pub rock_phase: RockRollPhase,
    pub rock_timer: f32,
    pub rock_dir: Vec3,
    // Boom
    pub boom_next_time: f64,
    // Rad
    pub rad_phase: RadPhase,
    pub rad_timer: f32,
    pub rad_next_time: f64,
    pub rad_factor: f32,
    // Crystal
    pub crystal_next_time: f64,
    // Phosphor
    pub hover_remaining: f32,
    pub hover_next_time: f64,
    pub sunlight_exposure: f32,
}

impl Default for ClassicAbilityState {
    fn default() -> Self {
        Self {
            rock_phase: RockRollPhase::Idle,
            rock_timer: 0.0,
            rock_dir: Vec3::Z,
            boom_next_time: 10.0,
            rad_phase: RadPhase::Idle,
            rad_timer: 0.0,
            rad_next_time: 15.0,
            rad_factor: 1.0,
            crystal_next_time: 10.0,
            hover_remaining: 0.0,
            hover_next_time: 12.0,
            sunlight_exposure: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClassicAbilityEvents {
    pub exploded_slimes: Vec<(i64, Vec3)>,
    pub player_damage: f32,
    pub player_rad_delta: f32,
    pub crystal_clusters_spawned: usize,
    pub phosphor_burned_out: bool,
}

impl ClassicAbilityState {
    pub fn calm_rad(&mut self) {
        if self.rad_phase != RadPhase::Idle {
            self.rad_phase = RadPhase::Calming;
            self.rad_timer = RAD_CALMING_TIME;
        }
    }

    pub fn step(
        &mut self,
        actor_id: i64,
        ident: IdentifiableId,
        slime_pos: Vec3,
        player_pos: Vec3,
        in_cave: bool,
        hour_of_day: f32,
        elapsed: f64,
        dt: f32,
        registry: &DataRegistry,
        physics: &mut PhysicsWorld,
        crystal_clusters: &mut Vec<CrystalCluster>,
        rng: &mut Randoms,
    ) -> ClassicAbilityEvents {
        let mut ev = ClassicAbilityEvents::default();
        let dist_player = slime_pos.distance(player_pos);

        // 1. RockSlimeRoll
        if slime_has_trait(registry, ident, IdentifiableId::RockSlime) {
            match self.rock_phase {
                RockRollPhase::Idle => {
                    self.rock_timer -= dt;
                    if self.rock_timer <= 0.0 && dist_player < 14.0 {
                        let flat = Vec3::new(
                            player_pos.x - slime_pos.x,
                            0.0,
                            player_pos.z - slime_pos.z,
                        );
                        self.rock_dir = if flat.length_squared() > 1e-4 {
                            flat.normalize()
                        } else {
                            Vec3::Z
                        };
                        self.rock_phase = RockRollPhase::Spinning;
                        self.rock_timer = ROCK_SPIN_TIME;
                    }
                }
                RockRollPhase::Spinning => {
                    self.rock_timer -= dt;
                    if self.rock_timer <= 0.0 {
                        self.rock_phase = RockRollPhase::Rolling;
                        self.rock_timer = ROCK_ROLL_TIME;
                    }
                }
                RockRollPhase::Rolling => {
                    self.rock_timer -= dt;
                    physics.add_force(actor_id, self.rock_dir * 18.0 * dt);
                    if dist_player <= 1.4 {
                        ev.player_damage += ROCK_ROLL_DAMAGE * dt * 2.0;
                    }
                    if self.rock_timer <= 0.0 {
                        self.rock_phase = RockRollPhase::Idle;
                        self.rock_timer = ROCK_COOLDOWN;
                    }
                }
            }
        }

        // 2. BoomSlimeExplode
        if slime_has_trait(registry, ident, IdentifiableId::BoomSlime) && elapsed >= self.boom_next_time
        {
            self.boom_next_time =
                elapsed + rng.get_in_range_f32(BOOM_MIN_DELAY, BOOM_MAX_DELAY) as f64;
            physics.soft_explosion_force(
                BOOM_EXPLODE_POWER,
                slime_pos,
                BOOM_EXPLODE_RADIUS,
                0.08,
                |id| id != actor_id,
            );
            if dist_player <= BOOM_EXPLODE_RADIUS {
                let falloff = (1.0 - dist_player / BOOM_EXPLODE_RADIUS).clamp(0.0, 1.0);
                ev.player_damage +=
                    BOOM_MIN_DAMAGE + (BOOM_MAX_DAMAGE - BOOM_MIN_DAMAGE) * falloff;
            }
            ev.exploded_slimes.push((actor_id, slime_pos));
        }

        // 3. RadSlimeExpand
        if slime_has_trait(registry, ident, IdentifiableId::RadSlime) {
            match self.rad_phase {
                RadPhase::Idle => {
                    self.rad_factor = 1.0;
                    if elapsed >= self.rad_next_time {
                        self.rad_phase = RadPhase::Expanding;
                        self.rad_timer = RAD_EXPANDING_TIME;
                    }
                }
                RadPhase::Expanding => {
                    self.rad_timer -= dt;
                    let t = 1.0 - (self.rad_timer / RAD_EXPANDING_TIME).clamp(0.0, 1.0);
                    self.rad_factor = 1.0 + (RAD_EXPAND_FACTOR - 1.0) * t;
                    if self.rad_timer <= 0.0 {
                        self.rad_phase = RadPhase::Expanded;
                        self.rad_timer = RAD_EXPANDED_TIME;
                    }
                }
                RadPhase::Expanded => {
                    self.rad_factor = RAD_EXPAND_FACTOR;
                    self.rad_timer -= dt;
                    if self.rad_timer <= 0.0 {
                        self.rad_phase = RadPhase::Calming;
                        self.rad_timer = RAD_CALMING_TIME;
                    }
                }
                RadPhase::Calming => {
                    self.rad_timer -= dt;
                    let t = (self.rad_timer / RAD_CALMING_TIME).clamp(0.0, 1.0);
                    self.rad_factor = 1.0 + (RAD_EXPAND_FACTOR - 1.0) * t;
                    if self.rad_timer <= 0.0 {
                        self.rad_phase = RadPhase::Idle;
                        self.rad_factor = 1.0;
                        self.rad_next_time =
                            elapsed + rng.get_in_range_f32(RAD_MIN_DELAY, RAD_MAX_DELAY) as f64;
                    }
                }
            }
            let rad_radius = RAD_BASE_RADIUS * self.rad_factor;
            if dist_player <= rad_radius {
                let closeness = (1.0 - dist_player / rad_radius).clamp(0.1, 1.0);
                ev.player_rad_delta += closeness * 0.25 * dt;
            }
        }

        // 4. CrystalSlimeLaunch
        if slime_has_trait(registry, ident, IdentifiableId::CrystalSlime)
            && elapsed >= self.crystal_next_time
        {
            self.crystal_next_time =
                elapsed + rng.get_in_range_f32(CRYSTAL_MIN_DELAY, CRYSTAL_MAX_DELAY) as f64;
            for _ in 0..3 {
                let angle = rng.get_in_range_f32(0.0, std::f32::consts::TAU);
                let offset = Vec3::new(angle.cos(), 0.0, angle.sin())
                    * rng.get_in_range_f32(0.8, CRYSTAL_SPIKES_SPREAD * 2.0);
                crystal_clusters.push(CrystalCluster {
                    pos: Vec3::new(slime_pos.x + offset.x, 0.0, slime_pos.z + offset.z),
                    radius: 0.9,
                    expires_at: elapsed + CRYSTAL_CLUSTER_LIFETIME as f64,
                });
                ev.crystal_clusters_spawned += 1;
            }
        }

        // 5. PhosphorSlime: Hover + Sunlight burn
        if slime_has_trait(registry, ident, IdentifiableId::PhosphorSlime) {
            if self.hover_remaining > 0.0 {
                self.hover_remaining -= dt;
                if slime_pos.y < PHOSPHOR_HOVER_HEIGHT {
                    physics.add_force(actor_id, Vec3::Y * 14.0 * dt);
                }
            } else if elapsed >= self.hover_next_time {
                self.hover_remaining = PHOSPHOR_HOVER_TIME;
                self.hover_next_time = elapsed
                    + PHOSPHOR_HOVER_TIME as f64
                    + rng.get_in_range_f32(PHOSPHOR_HOVER_MIN_DELAY, PHOSPHOR_HOVER_MAX_DELAY)
                        as f64;
            }

            if !in_cave && hour_of_day >= 6.0 && hour_of_day <= 18.0 {
                self.sunlight_exposure += dt;
                if self.sunlight_exposure >= PHOSPHOR_DAYLIGHT_BURN_SECS {
                    ev.phosphor_burned_out = true;
                }
            } else {
                self.sunlight_exposure = (self.sunlight_exposure - dt * 0.5).max(0.0);
            }
        }

        ev
    }
}
