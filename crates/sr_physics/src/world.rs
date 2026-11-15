//! 3D dynamic rigidbody simulation, raycast batching, and collision event reporting.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_math::Vec3;

use crate::shapes::{
    intersect_ray_aabb, intersect_ray_sphere, CollisionLayers, RayHit, StaticCollider, StaticShape,
};

pub const DEFAULT_GRAVITY: f32 = -19.62;

/// Rigid-body parameters & state for an actor in 3D space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigidBody {
    pub actor_id: i64,
    pub pos: Vec3,
    pub vel: Vec3,
    pub radius: f32,
    pub mass: f32,
    pub restitution: f32,
    pub linear_damping: f32,
    pub use_gravity: bool,
    pub is_kinematic: bool,
    pub grounded: bool,
    pub in_liquid: bool,
    pub layers: CollisionLayers,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            actor_id: 0,
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            radius: 0.45,
            mass: 1.0,
            restitution: 0.35,
            linear_damping: 0.05,
            use_gravity: true,
            is_kinematic: false,
            grounded: false,
            in_liquid: false,
            layers: CollisionLayers::ACTOR,
        }
    }
}

/// Collision event generated during `PhysicsWorld::step` (used by `BreakOnImpact` etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactEvent {
    pub actor_id: i64,
    pub other_actor_id: Option<i64>,
    pub static_id: Option<String>,
    pub normal: Vec3,
    /// Relative velocity projected onto contact normal (compared against `COLLISION_THRESHOLD = 14.0`).
    pub normal_speed: f32,
}

/// Trigger overlap event (e.g., sea kill zone, liquid pool).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerOverlap {
    pub actor_id: i64,
    pub static_id: String,
    pub layers: CollisionLayers,
}

/// Batched raycast request matching `RaycastBatcher`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayQuery {
    pub origin: Vec3,
    pub dir: Vec3,
    pub max_dist: f32,
    pub mask: CollisionLayers,
    pub include_triggers: bool,
}

/// Self-contained deterministic 3D physics world.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicsWorld {
    pub gravity: f32,
    pub bodies: HashMap<i64, RigidBody>,
    pub statics: Vec<StaticCollider>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            bodies: HashMap::new(),
            statics: Vec::new(),
        }
    }

    pub fn add_static(&mut self, collider: StaticCollider) {
        self.statics.push(collider);
    }

    pub fn insert_body(&mut self, body: RigidBody) {
        self.bodies.insert(body.actor_id, body);
    }

    pub fn remove_body(&mut self, actor_id: i64) -> Option<RigidBody> {
        self.bodies.remove(&actor_id)
    }

    pub fn add_force(&mut self, actor_id: i64, impulse: Vec3) {
        if let Some(b) = self.bodies.get_mut(&actor_id) {
            if !b.is_kinematic && b.mass > 0.0 {
                b.vel += impulse / b.mass;
            }
        }
    }

    /// Port of `PhysicsUtil.SoftExplosionForce`.
    pub fn soft_explosion_force(
        &mut self,
        power: f32,
        origin: Vec3,
        radius: f32,
        dt: f32,
        filter: impl Fn(i64) -> bool,
    ) {
        if radius <= 0.0 {
            return;
        }
        let ids: Vec<i64> = self.bodies.keys().copied().collect();
        for id in ids {
            if !filter(id) {
                continue;
            }
            let Some(b) = self.bodies.get_mut(&id) else {
                continue;
            };
            if b.is_kinematic {
                continue;
            }
            let delta = b.pos - origin;
            let dist = delta.length();
            if dist <= radius {
                let falloff = (1.0 - dist / radius).clamp(0.0, 1.0);
                let dir = if dist > 1e-4 {
                    (delta / dist + Vec3::Y * 0.25).normalize()
                } else {
                    Vec3::Y
                };
                // Scale force cleanly to impulse
                let impulse = dir * (power * falloff * dt);
                b.vel += impulse / b.mass.max(0.1);
            }
        }
    }

    /// Steps all rigid bodies, resolves static & dynamic sphere collisions, and emits events.
    pub fn step(&mut self, dt: f32) -> (Vec<ImpactEvent>, Vec<TriggerOverlap>) {
        let mut impacts = Vec::new();
        let mut overlaps = Vec::new();
        if dt <= 0.0 {
            return (impacts, overlaps);
        }

        let mut ids: Vec<i64> = self.bodies.keys().copied().collect();
        ids.sort_unstable();

        // 1. Integrate velocities & positions, resolve static collisions + triggers
        for &id in &ids {
            let Some(body) = self.bodies.get_mut(&id) else {
                continue;
            };
            body.grounded = false;
            body.in_liquid = false;
            if body.is_kinematic {
                continue;
            }
            if body.use_gravity {
                body.vel.y += self.gravity * dt;
            }
            let damp = (1.0 - body.linear_damping * dt).clamp(0.0, 1.0);
            body.vel *= damp;
            body.pos += body.vel * dt;

            for stat in &self.statics {
                if stat.is_trigger {
                    let hit = match &stat.shape {
                        StaticShape::GroundPlane { height } => body.pos.y - body.radius <= *height,
                        StaticShape::Aabb {
                            center,
                            half_extents,
                        } => {
                            let d = (body.pos - *center).abs();
                            d.x <= half_extents.x + body.radius
                                && d.y <= half_extents.y + body.radius
                                && d.z <= half_extents.z + body.radius
                        }
                    };
                    if hit {
                        if stat.layers.contains(CollisionLayers::LIQUID) {
                            body.in_liquid = true;
                        }
                        overlaps.push(TriggerOverlap {
                            actor_id: id,
                            static_id: stat.id.clone(),
                            layers: stat.layers,
                        });
                    }
                    continue;
                }

                if !stat.layers.intersects(CollisionLayers::ALL_SOLID) {
                    continue;
                }

                match &stat.shape {
                    StaticShape::GroundPlane { height } => {
                        let min_y = *height + body.radius;
                        if body.pos.y < min_y {
                            let vn = body.vel.y;
                            let normal_speed = (-vn).max(0.0);
                            if normal_speed > 1.0 {
                                impacts.push(ImpactEvent {
                                    actor_id: id,
                                    other_actor_id: None,
                                    static_id: Some(stat.id.clone()),
                                    normal: Vec3::Y,
                                    normal_speed,
                                });
                            }
                            body.pos.y = min_y;
                            if vn < 0.0 {
                                body.vel.y = if -vn > 1.2 { -vn * body.restitution } else { 0.0 };
                            }
                            body.vel.x *= 0.92;
                            body.vel.z *= 0.92;
                            body.grounded = true;
                        }
                    }
                    StaticShape::Aabb {
                        center,
                        half_extents,
                    } => {
                        let min = *center - *half_extents;
                        let max = *center + *half_extents;
                        let closest = body.pos.clamp(min, max);
                        let delta = body.pos - closest;
                        let dist_sq = delta.length_squared();
                        if dist_sq < body.radius * body.radius {
                            let dist = dist_sq.sqrt();
                            let normal = if dist > 1e-5 {
                                delta / dist
                            } else {
                                Vec3::Y
                            };
                            let penetration = body.radius - dist;
                            body.pos += normal * penetration;
                            let vn = body.vel.dot(normal);
                            if vn < 0.0 {
                                let normal_speed = -vn;
                                if normal_speed > 1.0 {
                                    impacts.push(ImpactEvent {
                                        actor_id: id,
                                        other_actor_id: None,
                                        static_id: Some(stat.id.clone()),
                                        normal,
                                        normal_speed,
                                    });
                                }
                                let bounce = if normal_speed > 1.2 {
                                    body.restitution
                                } else {
                                    0.0
                                };
                                body.vel -= normal * vn * (1.0 + bounce);
                            }
                            if normal.y > 0.6 {
                                body.grounded = true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Pairwise dynamic sphere-sphere resolution
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let id_a = ids[i];
                let id_b = ids[j];
                let (pos_a, rad_a, vel_a, kin_a, mass_a, rest_a) = {
                    let b = &self.bodies[&id_a];
                    (b.pos, b.radius, b.vel, b.is_kinematic, b.mass, b.restitution)
                };
                let (pos_b, rad_b, vel_b, kin_b, mass_b, rest_b) = {
                    let b = &self.bodies[&id_b];
                    (b.pos, b.radius, b.vel, b.is_kinematic, b.mass, b.restitution)
                };
                if kin_a && kin_b {
                    continue;
                }
                let delta = pos_b - pos_a;
                let min_dist = rad_a + rad_b;
                let dist_sq = delta.length_squared();
                if dist_sq < min_dist * min_dist {
                    let dist = dist_sq.sqrt();
                    let normal = if dist > 1e-5 {
                        delta / dist
                    } else {
                        Vec3::X
                    };
                    let penetration = min_dist - dist;
                    let inv_a = if kin_a { 0.0 } else { 1.0 / mass_a.max(0.1) };
                    let inv_b = if kin_b { 0.0 } else { 1.0 / mass_b.max(0.1) };
                    let inv_sum = inv_a + inv_b;
                    if inv_sum <= 0.0 {
                        continue;
                    }
                    let rel_vel = vel_b - vel_a;
                    let vn = rel_vel.dot(normal);

                    if !kin_a {
                        if let Some(a) = self.bodies.get_mut(&id_a) {
                            a.pos -= normal * penetration * (inv_a / inv_sum);
                        }
                    }
                    if !kin_b {
                        if let Some(b) = self.bodies.get_mut(&id_b) {
                            b.pos += normal * penetration * (inv_b / inv_sum);
                        }
                    }

                    if vn < 0.0 {
                        let e = rest_a.min(rest_b);
                        let impulse_mag = -(1.0 + e) * vn / inv_sum;
                        let impulse = normal * impulse_mag;
                        if (-vn) > 1.0 {
                            impacts.push(ImpactEvent {
                                actor_id: id_a,
                                other_actor_id: Some(id_b),
                                static_id: None,
                                normal: -normal,
                                normal_speed: -vn,
                            });
                            impacts.push(ImpactEvent {
                                actor_id: id_b,
                                other_actor_id: Some(id_a),
                                static_id: None,
                                normal,
                                normal_speed: -vn,
                            });
                        }
                        if !kin_a {
                            if let Some(a) = self.bodies.get_mut(&id_a) {
                                a.vel -= impulse * inv_a;
                            }
                        }
                        if !kin_b {
                            if let Some(b) = self.bodies.get_mut(&id_b) {
                                b.vel += impulse * inv_b;
                            }
                        }
                    }
                }
            }
        }

        (impacts, overlaps)
    }

    /// Performs a single raycast against static colliders and actor spheres.
    pub fn raycast(&self, query: RayQuery) -> Option<RayHit> {
        let dir = query.dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let mut best: Option<RayHit> = None;
        let mut best_dist = query.max_dist;

        if query.mask.intersects(CollisionLayers::WORLD | CollisionLayers::LIQUID) {
            for stat in &self.statics {
                if stat.is_trigger && !query.include_triggers {
                    continue;
                }
                if !stat.layers.intersects(query.mask) {
                    continue;
                }
                let hit = match &stat.shape {
                    StaticShape::GroundPlane { height } => {
                        if dir.y.abs() > 1e-6 {
                            let t = (*height - query.origin.y) / dir.y;
                            if t >= 0.0 && t <= best_dist {
                                Some((t, Vec3::Y))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    StaticShape::Aabb {
                        center,
                        half_extents,
                    } => intersect_ray_aabb(query.origin, dir, *center, *half_extents, best_dist),
                };
                if let Some((t, normal)) = hit {
                    if t < best_dist {
                        best_dist = t;
                        best = Some(RayHit {
                            distance: t,
                            point: query.origin + dir * t,
                            normal,
                            actor_id: None,
                            static_id: Some(stat.id.clone()),
                            is_trigger: stat.is_trigger,
                        });
                    }
                }
            }
        }

        if query.mask.intersects(CollisionLayers::ACTOR) {
            for (&id, body) in &self.bodies {
                if !body.layers.intersects(query.mask) {
                    continue;
                }
                if let Some(t) =
                    intersect_ray_sphere(query.origin, dir, body.pos, body.radius, best_dist)
                {
                    if t < best_dist {
                        best_dist = t;
                        let point = query.origin + dir * t;
                        best = Some(RayHit {
                            distance: t,
                            point,
                            normal: (point - body.pos).normalize_or_zero(),
                            actor_id: Some(id),
                            static_id: None,
                            is_trigger: false,
                        });
                    }
                }
            }
        }

        best
    }

    /// Port of `RaycastBatcher`: executes a batch of ray queries.
    pub fn raycast_batch(&self, queries: &[RayQuery]) -> Vec<Option<RayHit>> {
        queries.iter().map(|&q| self.raycast(q)).collect()
    }

    /// Returns all actor IDs within a cone `(origin, forward, max_dist, cos_half_angle)`.
    pub fn overlap_cone(
        &self,
        origin: Vec3,
        forward: Vec3,
        max_dist: f32,
        cos_half_angle: f32,
    ) -> Vec<i64> {
        let fwd = forward.normalize_or_zero();
        let mut out = Vec::new();
        for (&id, body) in &self.bodies {
            let to_obj = body.pos - origin;
            let dist = to_obj.length();
            if dist <= max_dist + body.radius {
                if dist <= body.radius * 1.25 {
                    out.push(id);
                } else {
                    let cos_ang = (to_obj / dist).dot(fwd);
                    if cos_ang >= cos_half_angle {
                        out.push(id);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }
}
