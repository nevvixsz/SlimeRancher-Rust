//! Actor lifecycle systems: `BreakOnImpact`, `DestroyOutsideBounds`, water calming/sterilization,
//! and plort expiration.

use sr_ids::{IdentifiableId, RegionSetId, SlimeEmotion};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel};
use sr_physics::{CollisionLayers, ImpactEvent, PhysicsWorld, RigidBody, TriggerOverlap};

use crate::vacuum::WeaponVacuum;

pub const BREAK_COLLISION_THRESHOLD: f32 = 14.0;
pub const WATER_AGITATION_REDUCTION: f32 = 0.1;
pub const WATER_CALMED_HOURS: f32 = 0.3333;
pub const OUT_OF_BOUNDS_MIN_Y: f32 = -50.0;

/// Events emitted while processing actor impacts, water contacts, and bounds checks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorLifecycleEvents {
    pub broken_crates: Vec<i64>,
    pub destroyed_actors: Vec<i64>,
    pub calmed_slimes: Vec<i64>,
}

/// Processes `BreakOnImpact` (`COLLISION_THRESHOLD = 14.0`), `DestroyOutsideBounds`,
/// `DestroyOnWater`, and `PlortModel` expiration.
pub fn step_actor_lifecycle(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    impacts: &[ImpactEvent],
    overlaps: &[TriggerOverlap],
    rng: &mut Randoms,
) -> ActorLifecycleEvents {
    let mut events = ActorLifecycleEvents::default();
    let world_time = model.world.world_time;

    // 1. Break crates whose impact normal_speed exceeds 14.0
    let mut to_break = Vec::new();
    for imp in impacts {
        if imp.normal_speed > BREAK_COLLISION_THRESHOLD {
            if let Some(actor) = model.actors.get(&imp.actor_id) {
                if actor.ident.is_standard_crate()
                    || matches!(
                        actor.ident,
                        IdentifiableId::DailyExchangeCrate | IdentifiableId::SpecialExchangeCrate
                    )
                {
                    if !to_break.contains(&imp.actor_id) {
                        to_break.push(imp.actor_id);
                    }
                }
            }
        }
    }

    for crate_id in to_break {
        if let Some(crate_actor) = model.destroy_actor(crate_id) {
            physics.remove_body(crate_id);
            events.broken_crates.push(crate_id);
            let rewards = crate_reward_pool(crate_actor.ident);
            let count = rng.get_in_range_i32(2, 5);
            for _ in 0..count {
                let idx = rng.get_in_range_i32(0, rewards.len() as i32) as usize;
                let reward_id = rewards[idx];
                let jitter = Vec3::new(
                    rng.get_in_range_f32(-0.6, 0.6),
                    rng.get_in_range_f32(0.2, 0.8),
                    rng.get_in_range_f32(-0.6, 0.6),
                );
                let pos = crate_actor.pos + jitter;
                let spawned_id = model.create_actor(
                    reward_id,
                    crate_actor.curr_region_set_id,
                    pos,
                    Quat::IDENTITY,
                );
                physics.insert_body(RigidBody {
                    actor_id: spawned_id,
                    pos,
                    vel: jitter * 3.0 + Vec3::Y * 2.5,
                    radius: WeaponVacuum::radius_of(reward_id),
                    ..Default::default()
                });
            }
        }
    }

    // 2. Water / Sea kill triggers on actors
    let mut to_destroy = Vec::new();
    for ov in overlaps {
        if ov.layers.contains(CollisionLayers::SEA_KILL) {
            if !to_destroy.contains(&ov.actor_id) {
                to_destroy.push(ov.actor_id);
            }
        } else if ov.layers.contains(CollisionLayers::LIQUID) {
            if let Some(actor) = model.actors.get_mut(&ov.actor_id) {
                if actor.ident.is_water() {
                    if !to_destroy.contains(&ov.actor_id) {
                        to_destroy.push(ov.actor_id);
                    }
                } else if let ActorKind::Slime(slime) = &mut actor.kind {
                    if actor.ident.is_tarr() {
                        // Water damages/sterilizes Tarr
                        if !to_destroy.contains(&ov.actor_id) {
                            to_destroy.push(ov.actor_id);
                        }
                    } else if !events.calmed_slimes.contains(&ov.actor_id) {
                        if let Some(ag) = slime.emotion_mut(SlimeEmotion::Agitation) {
                            ag.curr_val = (ag.curr_val - WATER_AGITATION_REDUCTION * 0.1).max(0.0);
                        }
                        events.calmed_slimes.push(ov.actor_id);
                    }
                }
            }
        }
    }

    // 3. Out-of-bounds Y and expired plorts
    let actor_ids: Vec<i64> = model.actors.keys().copied().collect();
    for id in actor_ids {
        let Some(actor) = model.actors.get(&id) else {
            continue;
        };
        if let Some(body) = physics.bodies.get(&id) {
            if body.pos.y < OUT_OF_BOUNDS_MIN_Y {
                if !to_destroy.contains(&id) {
                    to_destroy.push(id);
                }
                continue;
            }
        }
        if let ActorKind::Plort(plort) = &actor.kind {
            if world_time >= plort.destroy_time {
                if !to_destroy.contains(&id) {
                    to_destroy.push(id);
                }
            }
        }
    }

    for id in to_destroy {
        if model.destroy_actor(id).is_some() {
            physics.remove_body(id);
            events.destroyed_actors.push(id);
        }
    }

    // 4. Sync rigidbody positions back into GameModel::actors
    for (id, body) in &physics.bodies {
        if let Some(actor) = model.actors.get_mut(id) {
            actor.pos = body.pos;
        }
    }

    events
}

/// Applies water spray impact (from player shooting `WaterLiquid` or `MagicWaterLiquid`).
pub fn apply_water_spray(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    target_id: i64,
    liquid_id: IdentifiableId,
) -> bool {
    let Some(actor) = model.actors.get_mut(&target_id) else {
        return false;
    };
    if actor.ident.is_tarr() {
        model.destroy_actor(target_id);
        physics.remove_body(target_id);
        return true;
    }
    let reduction = if liquid_id == IdentifiableId::MagicWaterLiquid {
        WATER_AGITATION_REDUCTION * 10.0
    } else {
        WATER_AGITATION_REDUCTION
    };
    if let Some(slime) = actor.as_slime_mut() {
        if let Some(ag) = slime.emotion_mut(SlimeEmotion::Agitation) {
            ag.curr_val = (ag.curr_val - reduction).max(0.0);
        }
        slime.is_feral = false;
        return true;
    }
    false
}

fn crate_reward_pool(crate_id: IdentifiableId) -> &'static [IdentifiableId] {
    match crate_id {
        IdentifiableId::CrateQuarry01 => &[
            IdentifiableId::BeetVeggie,
            IdentifiableId::OcaocaVeggie,
            IdentifiableId::Hen,
            IdentifiableId::RockPlort,
        ],
        IdentifiableId::CrateMoss01 => &[
            IdentifiableId::MangoFruit,
            IdentifiableId::BriarHen,
            IdentifiableId::HoneyPlort,
        ],
        IdentifiableId::CrateDesert01 => &[
            IdentifiableId::PearFruit,
            IdentifiableId::PaintedHen,
            IdentifiableId::DervishPlort,
        ],
        _ => &[
            IdentifiableId::CarrotVeggie,
            IdentifiableId::PogoFruit,
            IdentifiableId::CuberryFruit,
            IdentifiableId::Hen,
        ],
    }
}

/// Helper to spawn a physical actor into both `GameModel` and `PhysicsWorld`.
pub fn spawn_physical_actor(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    ident: IdentifiableId,
    region: RegionSetId,
    pos: Vec3,
    vel: Vec3,
) -> i64 {
    let actor_id = model.create_actor(ident, region, pos, Quat::IDENTITY);
    physics.insert_body(RigidBody {
        actor_id,
        pos,
        vel,
        radius: WeaponVacuum::radius_of(ident),
        ..Default::default()
    });
    actor_id
}
