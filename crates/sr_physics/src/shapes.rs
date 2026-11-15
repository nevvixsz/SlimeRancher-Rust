//! Collision shapes, raycasts, and collision layers matching Unity/Slime Rancher physics.

use serde::{Deserialize, Serialize};
use sr_math::Vec3;

/// Collision layer flags mirroring Slime Rancher's physics layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct CollisionLayers(pub u32);

impl CollisionLayers {
    pub const WORLD: Self = Self(1 << 0);
    pub const ACTOR: Self = Self(1 << 1);
    pub const PLAYER: Self = Self(1 << 2);
    pub const LIQUID: Self = Self(1 << 3);
    pub const SEA_KILL: Self = Self(1 << 4);
    pub const ALL_SOLID: Self = Self(Self::WORLD.0 | Self::ACTOR.0 | Self::PLAYER.0);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl std::ops::BitOr for CollisionLayers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for CollisionLayers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for CollisionLayers {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for CollisionLayers {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// Static collider geometry in the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticShape {
    /// Axis-aligned box centered at `center` with half-extents `half_extents`.
    Aabb { center: Vec3, half_extents: Vec3 },
    /// Horizontal plane `y <= height` (ground floor / sea kill plane).
    GroundPlane { height: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticCollider {
    pub id: String,
    pub layers: CollisionLayers,
    pub is_trigger: bool,
    pub shape: StaticShape,
}

/// Raycast result.
#[derive(Debug, Clone, PartialEq)]
pub struct RayHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub actor_id: Option<i64>,
    pub static_id: Option<String>,
    pub is_trigger: bool,
}

/// Ray-AABB slab intersection returning `(t_enter, normal)` if hit in `[0, max_dist]`.
pub fn intersect_ray_aabb(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    half_extents: Vec3,
    max_dist: f32,
) -> Option<(f32, Vec3)> {
    let min = center - half_extents;
    let max = center + half_extents;

    let mut tmin = 0.0f32;
    let mut tmax = max_dist;
    let mut hit_normal = Vec3::ZERO;

    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        if d.abs() < 1e-6 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
        } else {
            let inv = 1.0 / d;
            let mut t1 = (min[axis] - o) * inv;
            let mut t2 = (max[axis] - o) * inv;
            let mut n_sign = -1.0f32;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                n_sign = 1.0;
            }
            if t1 > tmin {
                tmin = t1;
                hit_normal = Vec3::ZERO;
                hit_normal[axis] = n_sign;
            }
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }

    if tmin >= 0.0 && tmin <= max_dist {
        Some((tmin, hit_normal))
    } else {
        None
    }
}

/// Ray-sphere intersection returning entry distance in `[0, max_dist]`.
pub fn intersect_ray_sphere(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    radius: f32,
    max_dist: f32,
) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.length_squared() - radius * radius;
    if c <= 0.0 {
        return Some(0.0);
    }
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 && t <= max_dist {
        Some(t)
    } else {
        None
    }
}
