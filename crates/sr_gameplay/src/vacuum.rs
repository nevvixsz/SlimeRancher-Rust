//! Port of `WeaponVacuum.cs` and `Vacuumable.cs` — vaccone capture, hold, expel, liquid intake, and AirBurst.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::IdentifiableId;
use sr_math::{Quat, Vec3};
use sr_model::GameModel;
use sr_physics::{CollisionLayers, PhysicsWorld, PlayerMotor, RayQuery, RigidBody};

use crate::ammo::AmmoController;

pub const EJECT_SPEED: f32 = 25.0;
pub const VALLEY_EJECT_SPEED_FACTOR: f32 = 3.0;
pub const SHOOT_COOLDOWN: f32 = 0.24;
pub const MAX_VAC_DIST: f32 = 20.0;
pub const CAPTURE_DIST: f32 = 1.0;
pub const MIN_JOINT_SPEED: f32 = 3.0;
pub const MAX_JOINT_SPEED: f32 = 6.0;
pub const VAC_CONE_COS_HALF_ANGLE: f32 = 0.86;
pub const AIR_BURST_POWER: f32 = 1200.0;
pub const AIR_BURST_RADIUS: f32 = 20.0;
pub const STAMINA_PER_BURST: f32 = 20.0;
pub const LIQUID_UNIT_TIME: f32 = 0.25;
pub const PREVENT_CAPTURE_ON_RELEASE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VacMode {
    #[default]
    None,
    Shoot,
    Vac,
    Gadget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VacuumableSize {
    Normal,
    Large,
    Giant,
}

/// Input state for the vacpack each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VacInput {
    pub vac_held: bool,
    pub shoot_held: bool,
    pub burst_pressed: bool,
    pub gadget_toggle_pressed: bool,
    pub next_slot: bool,
    pub prev_slot: bool,
    pub select_slot: Option<usize>,
}

/// Events produced by `WeaponVacuum::step`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VacEvents {
    pub captured: Vec<IdentifiableId>,
    pub expelled: Vec<(i64, IdentifiableId)>,
    pub held_actor: Option<i64>,
    pub air_burst_fired: bool,
    pub air_burst_no_energy: bool,
    pub liquid_captured: Option<IdentifiableId>,
}

/// Port of `WeaponVacuum`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponVacuum {
    pub vac_mode: VacMode,
    pub held: Option<i64>,
    pub held_start_time: f64,
    pub held_rad: f32,
    pub launched_held: bool,
    pub next_shot_time: f32,
    pub elapsed_time: f32,
    pub liquid_tick_timers: HashMap<String, f32>,
    /// Per-actor capture cooldown timestamp (`elapsed_time < until` -> cannot capture).
    pub prevent_capture_until: HashMap<i64, f32>,
    pub total_captured: u64,
    pub total_expelled: u64,
}

impl Default for WeaponVacuum {
    fn default() -> Self {
        Self {
            vac_mode: VacMode::None,
            held: None,
            held_start_time: f64::NAN,
            held_rad: 0.0,
            launched_held: false,
            next_shot_time: 0.0,
            elapsed_time: 0.0,
            liquid_tick_timers: HashMap::new(),
            prevent_capture_until: HashMap::new(),
            total_captured: 0,
            total_expelled: 0,
        }
    }
}

impl WeaponVacuum {
    /// Classifies an actor's vacuumable size (`NORMAL`, `LARGE`, or `GIANT`).
    pub fn size_of(id: IdentifiableId) -> VacuumableSize {
        if id.is_gordo() || id == IdentifiableId::Player {
            VacuumableSize::Giant
        } else if id.is_largo()
            || id.is_tarr()
            || id.is_toy()
            || id.is_standard_crate()
            || matches!(
                id,
                IdentifiableId::DailyExchangeCrate | IdentifiableId::SpecialExchangeCrate
            )
        {
            VacuumableSize::Large
        } else {
            VacuumableSize::Normal
        }
    }

    pub fn radius_of(id: IdentifiableId) -> f32 {
        match Self::size_of(id) {
            VacuumableSize::Giant => 1.5,
            VacuumableSize::Large => 0.75,
            VacuumableSize::Normal => {
                if id.is_plort() {
                    0.25
                } else if id.is_slime() || id.is_animal() {
                    0.45
                } else {
                    0.32
                }
            }
        }
    }

    pub fn muzzle_origin(model: &GameModel, motor: &PlayerMotor) -> Vec3 {
        let eye = model.player.position + Vec3::Y * 1.45;
        eye + motor.look_dir() * 0.75 + motor.right_xz() * 0.25
    }

    pub fn drop_all_vacced(&mut self, physics: &mut PhysicsWorld) {
        if let Some(actor_id) = self.held.take() {
            if let Some(body) = physics.bodies.get_mut(&actor_id) {
                body.is_kinematic = false;
            }
            self.prevent_capture_until
                .insert(actor_id, self.elapsed_time + PREVENT_CAPTURE_ON_RELEASE);
            self.held_rad = 0.0;
            self.held_start_time = f64::NAN;
        }
        self.liquid_tick_timers.clear();
    }

    /// Executes one frame of `WeaponVacuum` logic (vac, hold, expel, liquid intake, airburst).
    pub fn step(
        &mut self,
        model: &mut GameModel,
        motor: &PlayerMotor,
        ammo: &mut AmmoController,
        physics: &mut PhysicsWorld,
        input: VacInput,
        dt: f32,
    ) -> VacEvents {
        self.elapsed_time += dt;
        let mut events = VacEvents::default();

        // Clean expired capture cooldowns
        let now = self.elapsed_time;
        self.prevent_capture_until.retain(|_, &mut t| t > now);

        // Slot selection
        if let Some(slot_idx) = input.select_slot {
            ammo.select_slot(&model.player, slot_idx);
        }
        if input.next_slot {
            ammo.next_slot(&model.player);
        }
        if input.prev_slot {
            ammo.prev_slot(&model.player);
        }

        // Mode determination
        if input.gadget_toggle_pressed {
            self.vac_mode = if self.vac_mode == VacMode::Gadget {
                VacMode::None
            } else {
                VacMode::Gadget
            };
        } else if self.vac_mode != VacMode::Gadget {
            if input.shoot_held {
                self.vac_mode = VacMode::Shoot;
            } else if input.vac_held {
                self.vac_mode = VacMode::Vac;
            } else {
                self.vac_mode = VacMode::None;
                self.launched_held = false;
            }
        }

        if !input.shoot_held && !input.vac_held {
            self.launched_held = false;
        }

        // AirBurst (`WeaponVacuum.AirBurst`)
        if input.burst_pressed && model.player.has_air_burst {
            self.air_burst(model, motor, physics, &mut events);
        }

        // Shoot / Expel
        if self.vac_mode == VacMode::Shoot
            && !self.launched_held
            && self.elapsed_time >= self.next_shot_time
        {
            self.expel(model, motor, ammo, physics, &mut events);
            self.next_shot_time = self.elapsed_time + SHOOT_COOLDOWN;
        }

        // Vac / Hold
        if self.vac_mode == VacMode::Vac && !self.launched_held {
            if let Some(held_id) = self.held {
                self.update_held(model, motor, physics, held_id);
                events.held_actor = Some(held_id);
            } else {
                self.consume(model, motor, ammo, physics, dt, &mut events);
            }
        } else if self.held.is_some() {
            self.drop_all_vacced(physics);
        } else {
            self.liquid_tick_timers.clear();
        }

        events
    }

    fn air_burst(
        &mut self,
        model: &mut GameModel,
        motor: &PlayerMotor,
        physics: &mut PhysicsWorld,
        events: &mut VacEvents,
    ) {
        let world_time = model.world.world_time;
        if model.player.curr_energy < STAMINA_PER_BURST {
            events.air_burst_no_energy = true;
            return;
        }
        let origin = Self::muzzle_origin(model, motor);
        let look = motor.look_dir();
        model.player.spend_energy(STAMINA_PER_BURST, world_time);
        let slime_ids: std::collections::HashSet<i64> = model
            .actors
            .iter()
            .filter_map(|(&id, a)| {
                if a.ident.is_slime() && look.dot(a.pos - origin) > 0.0 {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        physics.soft_explosion_force(AIR_BURST_POWER, origin, AIR_BURST_RADIUS, 1.0 / 60.0, |id| {
            slime_ids.contains(&id)
        });
        events.air_burst_fired = true;
    }

    fn update_held(
        &mut self,
        model: &mut GameModel,
        motor: &PlayerMotor,
        physics: &mut PhysicsWorld,
        held_id: i64,
    ) {
        if !model.actors.contains_key(&held_id) {
            self.held = None;
            return;
        }
        let target_pos =
            Self::muzzle_origin(model, motor) + motor.look_dir() * (self.held_rad + 0.85);
        if let Some(body) = physics.bodies.get_mut(&held_id) {
            body.is_kinematic = true;
            body.vel = Vec3::ZERO;
            body.pos = target_pos;
        }
        if let Some(actor) = model.actors.get_mut(&held_id) {
            actor.pos = target_pos;
        }
    }

    fn expel(
        &mut self,
        model: &mut GameModel,
        motor: &PlayerMotor,
        ammo: &mut AmmoController,
        physics: &mut PhysicsWorld,
        events: &mut VacEvents,
    ) {
        let origin = Self::muzzle_origin(model, motor);
        let look = motor.look_dir();

        if let Some(held_id) = self.held.take() {
            let mut pos = origin + look * (self.held_rad + 0.5);
            let mut vel = look * EJECT_SPEED + motor.velocity;
            pos = Self::ensure_not_shooting_into_rock(physics, origin, look, pos, self.held_rad, &mut vel);
            if let Some(body) = physics.bodies.get_mut(&held_id) {
                body.is_kinematic = false;
                body.pos = pos;
                body.vel = vel;
            }
            if let Some(actor) = model.actors.get_mut(&held_id) {
                actor.pos = pos;
                events.expelled.push((held_id, actor.ident));
            }
            self.prevent_capture_until
                .insert(held_id, self.elapsed_time + PREVENT_CAPTURE_ON_RELEASE);
            self.held_rad = 0.0;
            self.held_start_time = f64::NAN;
            self.launched_held = true;
            self.total_expelled += 1;
            return;
        }

        if !ammo.has_selected_ammo(&model.player) {
            return;
        }

        let world_time = model.world.world_time;
        let id = ammo.selected_id(&model.player, world_time);
        if id == IdentifiableId::None {
            return;
        }
        let emotions = ammo.decrement_selected(&mut model.player, 1, world_time);

        let rad = Self::radius_of(id);
        let down_offset = if look.y >= 0.0 { 0.0 } else { -0.5 * look.y };
        let mut pos = origin + look * (rad * 0.2 + down_offset + 0.45);
        let speed = match id {
            IdentifiableId::ValleyAmmo1
            | IdentifiableId::ValleyAmmo2
            | IdentifiableId::ValleyAmmo3
            | IdentifiableId::ValleyAmmo4 => EJECT_SPEED * VALLEY_EJECT_SPEED_FACTOR,
            _ => EJECT_SPEED,
        };
        let mut vel = look * speed + motor.velocity;
        pos = Self::ensure_not_shooting_into_rock(physics, origin, look, pos, rad, &mut vel);

        let region = model.player.curr_region_set_id;
        let actor_id = model.create_actor(id, region, pos, Quat::IDENTITY);
        if let Some(snap) = emotions {
            if let Some(actor) = model.actor_mut(actor_id) {
                if let Some(slime) = actor.as_slime_mut() {
                    slime.apply_snapshot(snap);
                }
            }
        }
        physics.insert_body(RigidBody {
            actor_id,
            pos,
            vel,
            radius: rad,
            ..Default::default()
        });
        self.prevent_capture_until
            .insert(actor_id, self.elapsed_time + 0.25);
        self.total_expelled += 1;
        events.expelled.push((actor_id, id));
    }

    fn ensure_not_shooting_into_rock(
        physics: &PhysicsWorld,
        origin: Vec3,
        dir: Vec3,
        mut start_pos: Vec3,
        obj_rad: f32,
        vel: &mut Vec3,
    ) -> Vec3 {
        let back = 0.5;
        let ray_origin = origin - dir * back;
        let max_dist = (start_pos - ray_origin).length();
        if let Some(hit) = physics.raycast(RayQuery {
            origin: ray_origin,
            dir,
            max_dist,
            mask: CollisionLayers::WORLD,
            include_triggers: false,
        }) {
            start_pos = hit.point - dir * obj_rad;
            let vn = vel.dot(hit.normal);
            *vel -= hit.normal * vn;
        }
        start_pos
    }

    fn consume(
        &mut self,
        model: &mut GameModel,
        motor: &PlayerMotor,
        ammo: &mut AmmoController,
        physics: &mut PhysicsWorld,
        dt: f32,
        events: &mut VacEvents,
    ) {
        let origin = Self::muzzle_origin(model, motor);
        let look = motor.look_dir();
        let candidates = physics.overlap_cone(origin, look, MAX_VAC_DIST, VAC_CONE_COS_HALF_ANGLE);

        for actor_id in candidates {
            if self.prevent_capture_until.contains_key(&actor_id) {
                continue;
            }
            let Some(actor) = model.actors.get(&actor_id) else {
                continue;
            };
            let ident = actor.ident;
            let size = Self::size_of(ident);
            if size == VacuumableSize::Giant {
                continue;
            }
            let Some(body) = physics.bodies.get(&actor_id) else {
                continue;
            };
            if body.is_kinematic {
                continue;
            }
            let to_actor = body.pos - origin;
            let dist = to_actor.length();

            // Check world occlusion
            if dist > 0.1 {
                if let Some(hit) = physics.raycast(RayQuery {
                    origin,
                    dir: to_actor / dist,
                    max_dist: dist,
                    mask: CollisionLayers::WORLD,
                    include_triggers: false,
                }) {
                    if hit.distance + 0.25 < dist {
                        continue;
                    }
                }
            }

            if dist <= CAPTURE_DIST + body.radius * 0.5 {
                match size {
                    VacuumableSize::Normal => {
                        let snap = actor.as_slime().map(|s| s.snapshot());
                        let world_time = model.world.world_time;
                        if ammo.maybe_add_to_slot(&mut model.player, ident, snap, 1, world_time) {
                            model.destroy_actor(actor_id);
                            physics.remove_body(actor_id);
                            self.total_captured += 1;
                            events.captured.push(ident);
                        }
                    }
                    VacuumableSize::Large => {
                        if self.held.is_none() {
                            self.held = Some(actor_id);
                            self.held_rad = body.radius;
                            self.held_start_time = model.world.world_time;
                            self.update_held(model, motor, physics, actor_id);
                            events.held_actor = Some(actor_id);
                            break;
                        }
                    }
                    VacuumableSize::Giant => {}
                }
            } else if dist > 1e-4 {
                // Spring-pull toward muzzle (`ConsumeExistingJointed` pull speed interpolation)
                let t = (dist / MAX_VAC_DIST).clamp(0.0, 1.0);
                let pull_speed = MAX_JOINT_SPEED + (MIN_JOINT_SPEED - MAX_JOINT_SPEED) * t;
                let pull_dir = -to_actor / dist;
                if let Some(b) = physics.bodies.get_mut(&actor_id) {
                    // Blend velocity cleanly toward muzzle while overcoming gravity
                    let desired_vel = pull_dir * (pull_speed * 2.5);
                    b.vel = b.vel.lerp(desired_vel, (dt * 10.0).min(1.0));
                }
            }
        }

        // Liquid intake raycast (`ConsumeLiquids`)
        if let Some(hit) = physics.raycast(RayQuery {
            origin,
            dir: look,
            max_dist: MAX_VAC_DIST,
            mask: CollisionLayers::LIQUID | CollisionLayers::WORLD,
            include_triggers: true,
        }) {
            if hit.is_trigger {
                if let Some(src_id) = hit.static_id {
                    let timer = self.liquid_tick_timers.entry(src_id).or_insert(LIQUID_UNIT_TIME);
                    *timer -= dt;
                    if *timer <= 0.0 {
                        *timer += LIQUID_UNIT_TIME;
                        let world_time = model.world.world_time;
                        if ammo.maybe_add_to_slot(
                            &mut model.player,
                            IdentifiableId::WaterLiquid,
                            None,
                            1,
                            world_time,
                        ) {
                            events.liquid_captured = Some(IdentifiableId::WaterLiquid);
                        }
                    }
                }
            }
        }
    }
}
