//! Port of `GordoEat.cs`, `GordoRewards.cs`, and the unified `SlimeAiCoordinator`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sr_data::{DataRegistry, GameModeSettings};
use sr_ids::{IdentifiableId, RegionSetId};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, GordoModel};
use sr_physics::{ImpactEvent, PhysicsWorld, RigidBody};

use crate::abilities_classic::{
    ClassicAbilityState, CrystalCluster, CRYSTAL_DAMAGE,
};
use crate::abilities_rare::{
    step_whirlwinds, RareAbilityState, WhirlwindHazard,
};
use crate::eat::{step_slime_eat_and_tarr, EatStepEvents, SlimeEatState};
use crate::emotions::SlimeEmotionRuntime;
use crate::plexer::{gather_stimuli, SlimePlexerState};

pub const GORDO_EAT_RADIUS: f32 = 2.6;

/// Reward granted when a Gordo pops (`GordoRewards`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GordoRewardKind {
    SlimeKey,
    TeleporterUnlock { door_or_teleporter_id: String },
    CratesAndSlimes { crate_id: IdentifiableId, crate_count: u32 },
}

/// Runtime placement & configuration of a world Gordo linked to `GameModel::gordos`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldGordo {
    pub gordo_id: String,
    pub slime_ident: IdentifiableId,
    pub pos: Vec3,
    pub region: RegionSetId,
    pub rewards: Vec<GordoRewardKind>,
    pub spawn_slime_count: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GordoStepEvents {
    pub fed: Vec<(String, IdentifiableId, i32, i32)>,
    pub popped: Vec<String>,
    pub keys_awarded: i32,
}

/// Feeds nearby edible food into unpopped Gordos (`num_to_produce()` / 2x favorite bonus!)
/// and triggers pop rewards when `gordo_eaten_count >= target_count`.
pub fn step_gordos(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    registry: &DataRegistry,
    world_gordos: &[WorldGordo],
    held_actor_id: Option<i64>,
    rng: &mut Randoms,
) -> GordoStepEvents {
    let mut events = GordoStepEvents::default();
    let r2 = GORDO_EAT_RADIUS * GORDO_EAT_RADIUS;

    for g in world_gordos {
        let Some(gm) = model.gordos.get(&g.gordo_id) else {
            continue;
        };
        if gm.gordo_eaten_count == -1 || gm.gordo_eaten_count >= gm.target_count {
            continue;
        }
        let Some(def) = registry.slimes.get(g.slime_ident) else {
            continue;
        };

        let mut eaten_foods: Vec<(i64, IdentifiableId, i32)> = Vec::new();
        for (&actor_id, actor) in &model.actors {
            if held_actor_id == Some(actor_id) {
                continue;
            }
            if g.pos.distance_squared(actor.pos) <= r2 {
                if let Some(entry) = def.diet.eat_map.iter().find(|e| e.eats == actor.ident) {
                    // Gordos don't eat plorts
                    if !actor.ident.is_plort() {
                        let count = entry.num_to_produce() as i32;
                        eaten_foods.push((actor_id, actor.ident, count));
                    }
                }
            }
        }

        for (fid, fident, count) in eaten_foods {
            let (should_pop, new_eaten, target) = {
                let Some(gm) = model.gordos.get_mut(&g.gordo_id) else {
                    break;
                };
                if gm.gordo_eaten_count == -1 {
                    break;
                }
                gm.gordo_eaten_count = (gm.gordo_eaten_count + count).min(gm.target_count);
                let popped = gm.gordo_eaten_count >= gm.target_count;
                (popped, gm.gordo_eaten_count, gm.target_count)
            };

            model.destroy_actor(fid);
            physics.remove_body(fid);
            events.fed.push((g.gordo_id.clone(), fident, new_eaten, target));

            if should_pop {
                if let Some(gm) = model.gordos.get_mut(&g.gordo_id) {
                    gm.gordo_eaten_count = -1;
                }
                events.popped.push(g.gordo_id.clone());

                // Grant GordoRewards
                for r in &g.rewards {
                    match r {
                        GordoRewardKind::SlimeKey => {
                            model.player.keys += 1;
                            events.keys_awarded += 1;
                        }
                        GordoRewardKind::TeleporterUnlock { door_or_teleporter_id } => {
                            if let Some(door) = model.doors.get_mut(door_or_teleporter_id) {
                                door.state = sr_model::AccessDoorState::Open;
                            }
                        }
                        GordoRewardKind::CratesAndSlimes { crate_id, crate_count } => {
                            for _ in 0..*crate_count {
                                let jitter = Vec3::new(
                                    rng.get_in_range_f32(-1.6, 1.6),
                                    0.6,
                                    rng.get_in_range_f32(-1.6, 1.6),
                                );
                                let pos = g.pos + jitter;
                                let cid = model.create_actor(*crate_id, g.region, pos, Quat::IDENTITY);
                                physics.insert_body(RigidBody {
                                    actor_id: cid,
                                    pos,
                                    vel: jitter * 2.5 + Vec3::Y * 3.5,
                                    radius: 0.6,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }

                // Burst out child slimes
                for _ in 0..g.spawn_slime_count {
                    let jitter = Vec3::new(
                        rng.get_in_range_f32(-1.4, 1.4),
                        0.7,
                        rng.get_in_range_f32(-1.4, 1.4),
                    );
                    let pos = g.pos + jitter;
                    let sid = model.create_actor(g.slime_ident, g.region, pos, Quat::IDENTITY);
                    physics.insert_body(RigidBody {
                        actor_id: sid,
                        pos,
                        vel: jitter * 3.5 + Vec3::Y * 4.0,
                        radius: 0.45,
                        ..Default::default()
                    });
                }
                break;
            }
        }
    }

    events
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeRuntimeEntry {
    pub emotions: SlimeEmotionRuntime,
    pub plexer: SlimePlexerState,
    pub eat: SlimeEatState,
    pub classic: ClassicAbilityState,
    pub rare: RareAbilityState,
}

impl SlimeRuntimeEntry {
    pub fn new(world_time: f64, elapsed: f64) -> Self {
        Self {
            emotions: SlimeEmotionRuntime::new(world_time),
            plexer: SlimePlexerState::new(elapsed),
            eat: SlimeEatState::default(),
            classic: ClassicAbilityState::default(),
            rare: RareAbilityState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiStepEvents {
    pub eat: EatStepEvents,
    pub gordo: GordoStepEvents,
    pub player_damage: f32,
    pub player_rad_delta: f32,
    pub player_fire_damage: f32,
    pub exploded_slimes: Vec<(i64, Vec3)>,
}

/// Top-level Step 3 coordinator owning runtime AI states, hazards, and world Gordos.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlimeAiCoordinator {
    pub elapsed_time: f64,
    pub slimes: BTreeMap<i64, SlimeRuntimeEntry>,
    pub crystal_clusters: Vec<CrystalCluster>,
    pub whirlwinds: Vec<WhirlwindHazard>,
    pub world_gordos: Vec<WorldGordo>,
}

impl SlimeAiCoordinator {
    pub fn register_gordo(&mut self, model: &mut GameModel, gordo: WorldGordo, target_count: i32) {
        model.gordos.entry(gordo.gordo_id.clone()).or_insert(GordoModel {
            id: gordo.gordo_id.clone(),
            gordo_eaten_count: 0,
            target_count,
            fashions: Vec::new(),
        });
        if !self.world_gordos.iter().any(|g| g.gordo_id == gordo.gordo_id) {
            self.world_gordos.push(gordo);
        }
    }

    /// Runs emotions, plexer subbehaviours, unique abilities, eating/largo/tarr, and gordos for 1 tick.
    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        registry: &DataRegistry,
        mode: &GameModeSettings,
        impacts: &[ImpactEvent],
        held_actor_id: Option<i64>,
        player_alive: bool,
        hour_of_day: f32,
        dt: f32,
        rng: &mut Randoms,
    ) -> AiStepEvents {
        self.elapsed_time += dt as f64;
        let world_time = model.world.world_time;
        let player_pos = model.player.position;
        let mut out = AiStepEvents::default();

        // Prune destroyed slimes
        self.slimes.retain(|id, _| model.actors.contains_key(id));

        let slime_ids: Vec<i64> = model
            .actors
            .iter()
            .filter(|(_, a)| a.ident.is_slime())
            .map(|(id, _)| *id)
            .collect();

        // Update emotions, plexer, feral checks, and classic/rare abilities
        let mut eat_states: BTreeMap<i64, SlimeEatState> = BTreeMap::new();
        let mut to_despawn: Vec<i64> = Vec::new();

        for &sid in &slime_ids {
            let entry = self
                .slimes
                .entry(sid)
                .or_insert_with(|| SlimeRuntimeEntry::new(world_time, self.elapsed_time));

            let (ident, pos, in_ranch_or_wilds) = match model.actors.get(&sid) {
                Some(a) => (
                    a.ident,
                    a.pos,
                    matches!(a.curr_region_set_id, RegionSetId::Home),
                ),
                None => continue,
            };

            let body = match physics.bodies.get(&sid) {
                Some(b) => b.clone(),
                None => continue,
            };

            let stimuli = gather_stimuli(
                sid,
                ident,
                pos,
                &body,
                model,
                physics,
                registry,
                held_actor_id,
                player_pos,
                player_alive,
            );

            // Step emotion update + feral + plexer
            let slime_clone = {
                let Some(actor) = model.actors.get_mut(&sid) else {
                    continue;
                };
                let ActorKind::Slime(ref mut sm) = actor.kind else {
                    continue;
                };
                entry.emotions.update_to_time(sm, world_time);
                entry.eat.update_feral(
                    sid,
                    ident,
                    sm,
                    world_time,
                    in_ranch_or_wilds,
                    &mut out.eat,
                );
                let plex = entry.plexer.step(
                    self.elapsed_time,
                    pos,
                    body.vel,
                    ident,
                    sm,
                    &stimuli,
                    rng,
                );
                if plex.impulse.length_squared() > 0.0 {
                    physics.add_force(sid, plex.impulse);
                }
                if let Some(q) = plex.target_facing {
                    actor.rot = q;
                }
                out.player_damage += plex.player_damage;
                sm.clone()
            };

            // Classic abilities (Rock, Boom, Rad, Crystal, Phosphor)
            let c_ev = entry.classic.step(
                sid,
                ident,
                pos,
                player_pos,
                false,
                hour_of_day,
                self.elapsed_time,
                dt,
                registry,
                physics,
                &mut self.crystal_clusters,
                rng,
            );
            out.player_damage += c_ev.player_damage;
            out.player_rad_delta += c_ev.player_rad_delta;
            out.exploded_slimes.extend(c_ev.exploded_slimes);
            if c_ev.phosphor_burned_out {
                to_despawn.push(sid);
            }

            // Rare abilities (Hunter, Quantum, Dervish, Mosaic, Tangle, Puddle, Fire, Gold, Lucky)
            let r_ev = entry.rare.step(
                sid,
                ident,
                pos,
                &slime_clone,
                player_pos,
                world_time,
                self.elapsed_time,
                dt,
                registry,
                model,
                physics,
                &mut self.whirlwinds,
                impacts,
                rng,
            );
            out.player_fire_damage += r_ev.player_fire_damage;
            to_despawn.extend(r_ev.despawned_slimes);

            eat_states.insert(sid, entry.eat.clone());
        }

        // Step active hazards: crystal clusters & whirlwinds
        self.crystal_clusters
            .retain(|c| self.elapsed_time < c.expires_at);
        for c in &self.crystal_clusters {
            if player_alive && player_pos.distance(c.pos) <= c.radius {
                out.player_damage += CRYSTAL_DAMAGE * dt;
            }
        }
        step_whirlwinds(&mut self.whirlwinds, physics, self.elapsed_time, dt);

        // Despawn burned-out phosphors / fled gold & lucky slimes
        for did in to_despawn {
            model.destroy_actor(did);
            physics.remove_body(did);
            self.slimes.remove(&did);
            eat_states.remove(&did);
        }

        // Step SlimeEat & Tarr bites/sunlight
        let eat_ev = step_slime_eat_and_tarr(
            model,
            physics,
            registry,
            mode,
            &mut eat_states,
            held_actor_id,
            world_time,
            self.elapsed_time,
            dt,
            hour_of_day,
            rng,
        );
        for (sid, est) in eat_states {
            if let Some(entry) = self.slimes.get_mut(&sid) {
                entry.eat = est;
            }
        }
        out.eat.eaten_pairs.extend(eat_ev.eaten_pairs);
        out.eat.spawned_plorts.extend(eat_ev.spawned_plorts);
        out.eat.transformed_slimes.extend(eat_ev.transformed_slimes);
        out.eat.tarr_converted.extend(eat_ev.tarr_converted);
        out.eat.tarr_destroyed.extend(eat_ev.tarr_destroyed);

        // Step Gordos
        out.gordo = step_gordos(
            model,
            physics,
            registry,
            &self.world_gordos,
            held_actor_id,
            rng,
        );

        out
    }
}
