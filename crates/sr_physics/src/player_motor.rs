//! Port of `vp_FPController` + `EnergyJetpack` player movement and hover physics.

use serde::{Deserialize, Serialize};
use sr_math::Vec3;
use sr_model::PlayerModel;

use crate::shapes::{CollisionLayers, StaticShape};
use crate::world::PhysicsWorld;

// Exact constants from `EnergyJetpack.cs`
pub const JETPACK_ENERGY_PER_SECOND: f32 = 33.3;
pub const JETPACK_INIT_ENERGY_USED: f32 = 10.0;
pub const JETPACK_START_THRESHOLD_SECS: f32 = 0.45;
pub const JETPACK_VEL_THRESHOLD: f32 = 0.5;
pub const JETPACK_JUMP_PAUSE_WORLD_SECS: f64 = 18.0;
pub const JETPACK_HOVER_HEIGHT: f32 = 7.0;
pub const JETPACK_HOVER_SOFTNESS_FUDGE: f32 = 2.2;
pub const JETPACK_GRAV_PER_Y: f32 = 0.1;
pub const JETPACK_FALL_COMPENSATION_PER_FRAME: f32 = 0.003;

// Player capsule & movement constants
pub const PLAYER_RADIUS: f32 = 0.45;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const WALK_SPEED: f32 = 7.2;
pub const SPRINT_SPEED: f32 = 11.4;
pub const SPRINT_ENERGY_PER_SECOND: f32 = 10.0;
pub const BACKWARD_FACTOR: f32 = 0.65;
pub const AIR_CONTROL_FACTOR: f32 = 0.35;
pub const JUMP_VELOCITY: f32 = 8.4;
pub const PLAYER_GRAVITY: f32 = -19.62;

/// Input frame for player movement & camera orientation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PlayerMoveInput {
    /// Strafe (x in [-1, 1]) and forward/backward (y in [-1, 1]).
    pub move_axes: [f32; 2],
    /// Camera yaw in radians (0 faces +Z).
    pub yaw: f32,
    /// Camera pitch in radians (positive looks up).
    pub pitch: f32,
    pub sprint: bool,
    pub jump_held: bool,
    pub jump_pressed: bool,
}

/// Port of `vp_FPController` + `EnergyJetpack` runtime state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerMotor {
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub jumping: bool,
    pub sprinting: bool,
    pub jetpack_active: bool,
    pub hover_y: f32,
    pub can_kick_in_jetpack_time: f64,
    pub in_sea_kill_zone: bool,
}

impl Default for PlayerMotor {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            grounded: true,
            jumping: false,
            sprinting: false,
            jetpack_active: false,
            hover_y: f32::INFINITY,
            can_kick_in_jetpack_time: 0.0,
            in_sea_kill_zone: false,
        }
    }
}

impl PlayerMotor {
    pub fn forward_xz(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos()).normalize()
    }

    pub fn right_xz(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin()).normalize()
    }

    pub fn look_dir(&self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(
            self.yaw.sin() * cp,
            self.pitch.sin(),
            self.yaw.cos() * cp,
        )
        .normalize()
    }

    pub fn stop(&mut self) {
        self.velocity = Vec3::ZERO;
        self.jumping = false;
        self.sprinting = false;
        self.jetpack_active = false;
    }

    /// Port of `EnergyJetpack.DownwardExtraGravity`.
    pub fn downward_extra_gravity(&self, y: f32, y_vel: f32) -> f32 {
        let delta = y - self.hover_y;
        if delta <= 0.0 {
            0.0
        } else {
            delta * JETPACK_GRAV_PER_Y * y_vel.max(0.5)
        }
    }

    /// Steps player movement, sprinting, jump, jetpack, and world collisions.
    pub fn step(
        &mut self,
        player: &mut PlayerModel,
        world: &PhysicsWorld,
        input: PlayerMoveInput,
        dt: f32,
        world_time: f64,
    ) {
        if dt <= 0.0 {
            return;
        }
        self.yaw = input.yaw;
        self.pitch = input.pitch.clamp(-1.48, 1.48);

        // Sprint energy check
        let wants_move = input.move_axes[0].abs() > 0.05 || input.move_axes[1] > 0.05;
        if input.sprint && wants_move && player.curr_energy > 0.0 {
            let cost = dt * SPRINT_ENERGY_PER_SECOND * player.run_efficiency;
            self.sprinting = player.spend_energy(cost, world_time);
        } else {
            self.sprinting = false;
        }

        // Horizontal velocity
        let fwd = self.forward_xz();
        let right = self.right_xz();
        let fwd_input = if input.move_axes[1] >= 0.0 {
            input.move_axes[1]
        } else {
            input.move_axes[1] * BACKWARD_FACTOR
        };
        let mut wish = fwd * fwd_input + right * input.move_axes[0];
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let target_speed = if self.sprinting {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };
        let control = if self.grounded { 1.0 } else { AIR_CONTROL_FACTOR };
        let target_horiz = wish * target_speed;
        let current_horiz = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
        let blended_horiz = current_horiz.lerp(target_horiz, (control * 12.0 * dt).min(1.0));
        self.velocity.x = blended_horiz.x;
        self.velocity.z = blended_horiz.z;

        // Jump initiation
        if input.jump_pressed && self.grounded {
            self.velocity.y = JUMP_VELOCITY;
            self.grounded = false;
            self.jumping = true;
            self.can_kick_in_jetpack_time = world_time + JETPACK_JUMP_PAUSE_WORLD_SECS;
            self.hover_y = self
                .hover_y
                .min(player.position.y + JETPACK_HOVER_HEIGHT - JETPACK_HOVER_SOFTNESS_FUDGE);
        }
        if !input.jump_held {
            self.jumping = false;
            if self.jetpack_active {
                self.jetpack_active = false;
                self.can_kick_in_jetpack_time = world_time + JETPACK_JUMP_PAUSE_WORLD_SECS;
            }
        }

        // Jetpack activation (`EnergyJetpack.FixedUpdate`)
        let jetpack_threshold = (JETPACK_START_THRESHOLD_SECS * JETPACK_ENERGY_PER_SECOND
            + JETPACK_INIT_ENERGY_USED)
            * player.jetpack_efficiency;
        if input.jump_held
            && !self.grounded
            && !self.jetpack_active
            && player.has_jetpack
            && self.velocity.y <= JETPACK_VEL_THRESHOLD
            && world_time >= self.can_kick_in_jetpack_time
            && player.curr_energy >= jetpack_threshold
        {
            self.jetpack_active = true;
            self.hover_y = self
                .hover_y
                .min(player.position.y + JETPACK_HOVER_HEIGHT - JETPACK_HOVER_SOFTNESS_FUDGE);
            player.spend_energy(JETPACK_INIT_ENERGY_USED * player.jetpack_efficiency, world_time);
        }

        if self.jetpack_active {
            player.spend_energy(
                dt * JETPACK_ENERGY_PER_SECOND * player.jetpack_efficiency,
                world_time,
            );
            if player.curr_energy <= 0.0 {
                self.jetpack_active = false;
            }
        }

        if self.grounded && !input.jump_held && !self.jetpack_active {
            self.hover_y = f32::INFINITY;
        }

        // Vertical acceleration / hover thrust
        if self.jetpack_active {
            let extra_grav = self.downward_extra_gravity(player.position.y, self.velocity.y);
            // Upward thrust counteracts gravity when below hover_y
            let net_accel = if extra_grav > 0.0 {
                -extra_grav * 8.0
            } else {
                8.5
            };
            self.velocity.y += net_accel * dt;
            if self.velocity.y < 0.0 && extra_grav <= 0.0 {
                self.velocity.y += -self.velocity.y * JETPACK_FALL_COMPENSATION_PER_FRAME * 60.0 * dt;
            }
            self.velocity.y = self.velocity.y.clamp(-12.0, 7.5);
        } else {
            self.velocity.y += PLAYER_GRAVITY * dt;
        }

        // Integrate position & resolve static world collisions
        player.position += self.velocity * dt;
        self.grounded = false;
        self.in_sea_kill_zone = false;

        for stat in &world.statics {
            if stat.is_trigger {
                if stat.layers.contains(CollisionLayers::SEA_KILL) {
                    if let StaticShape::GroundPlane { height } = stat.shape {
                        if player.position.y <= height {
                            self.in_sea_kill_zone = true;
                        }
                    }
                }
                continue;
            }
            if !stat.layers.intersects(CollisionLayers::ALL_SOLID) {
                continue;
            }
            match &stat.shape {
                StaticShape::GroundPlane { height } => {
                    if player.position.y <= *height {
                        player.position.y = *height;
                        if self.velocity.y < 0.0 {
                            self.velocity.y = 0.0;
                        }
                        self.grounded = true;
                    }
                }
                StaticShape::Aabb {
                    center,
                    half_extents,
                } => {
                    // Check player capsule center against expanded AABB
                    let body_center = player.position + Vec3::Y * (PLAYER_HEIGHT * 0.5);
                    let min = *center - *half_extents;
                    let max = *center + *half_extents;
                    let closest = body_center.clamp(min, max);
                    let delta = body_center - closest;
                    let half_h = PLAYER_HEIGHT * 0.5;
                    let horiz = Vec3::new(delta.x, 0.0, delta.z);
                    let horiz_dist = horiz.length();
                    if delta.y.abs() <= half_h && horiz_dist < PLAYER_RADIUS {
                        if delta.y > half_h * 0.65 && self.velocity.y <= 0.0 {
                            player.position.y = max.y;
                            self.velocity.y = 0.0;
                            self.grounded = true;
                        } else if horiz_dist > 1e-5 {
                            let push = (horiz / horiz_dist) * (PLAYER_RADIUS - horiz_dist);
                            player.position += push;
                            let vn = self.velocity.dot(horiz / horiz_dist);
                            if vn < 0.0 {
                                self.velocity -= (horiz / horiz_dist) * vn;
                            }
                        }
                    }
                }
            }
        }
    }
}
