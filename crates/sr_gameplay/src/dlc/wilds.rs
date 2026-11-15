//! Port of `KookadobaPatchNode.cs`, `ZoneDirector` Kookadoba spawning, and Ogden's Wilds progression.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{IdentifiableId, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{ActorKind, GameModel, ResourceCycleState};
use sr_physics::PhysicsWorld;

use crate::actors::spawn_physical_actor;
use crate::story::{ProgressDirector, ProgressStepEvents};

/// Delay in game hours between `KookadobaPatchNode` spawns (`ZoneDirector.KOOKADOBA_DELAY`).
pub const KOOKADOBA_SPAWN_DELAY_HOURS: f32 = 0.5;
pub const KOOKADOBA_SPAWN_DELAY_SECS: f64 = 1800.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KookadobaPatchNodeState {
    pub id: String,
    pub pos: Vec3,
    pub active_actor_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WildsStepEvents {
    pub grown_nodes: Vec<(String, i64)>,
}

/// Runtime manager for Ogden's Wilds (`KookadobaPatchNode` spawning and Wilds entry unlocks).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WildsSystem {
    pub nodes: HashMap<String, KookadobaPatchNodeState>,
    pub next_spawn_time: f64,
}

impl WildsSystem {
    pub fn register_node(&mut self, id: impl Into<String>, pos: Vec3) {
        let id = id.into();
        self.nodes.insert(
            id.clone(),
            KookadobaPatchNodeState {
                id,
                pos,
                active_actor_id: None,
            },
        );
    }

    /// Checks whether the player can enter Ogden's Wilds (`UNLOCK_MOSS` + `UNLOCK_OVERGROWTH` or `UNLOCK_OGDEN_MISSIONS`).
    pub fn can_enter_wilds(model: &GameModel, mode: &GameModeSettings) -> bool {
        mode.enable_ogden_missions
            && (model.progress.has_progress(ProgressType::UnlockOgdenMissions)
                || (model.progress.has_progress(ProgressType::UnlockMoss)
                    && model.progress.has_progress(ProgressType::UnlockOvergrowth)))
    }

    /// Transitions into Ogden's Ranch / Wilds, setting `EnterZoneOgdenRanch` & `UnlockWilds`.
    pub fn enter_wilds(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
    ) -> Option<ProgressStepEvents> {
        if !Self::can_enter_wilds(model, mode) {
            return None;
        }
        let mut ev = ProgressStepEvents::default();
        progress.set_unique_progress(model, mode, ProgressType::UnlockOgdenMissions, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::UnlockWilds, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::EnterZoneOgdenRanch, &mut ev);
        Some(ev)
    }

    /// Grows one unoccupied `KookadobaPatchNode` (`KookadobaPatchNode.Grow`).
    pub fn grow_node(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        node_id: &str,
    ) -> Option<i64> {
        let node = self.nodes.get_mut(node_id)?;
        if let Some(existing_id) = node.active_actor_id {
            if model.actors.contains_key(&existing_id) {
                return None;
            }
            node.active_actor_id = None;
        }
        let pos = node.pos;
        let aid = spawn_physical_actor(
            model,
            physics,
            IdentifiableId::KookadobaFruit,
            RegionSetId::Home,
            pos,
            Vec3::ZERO,
        );
        if let Some(actor) = model.actors.get_mut(&aid) {
            if let ActorKind::Produce(ref mut pm) = actor.kind {
                pm.state = ResourceCycleState::Edible;
            }
        }
        if let Some(n) = self.nodes.get_mut(node_id) {
            n.active_actor_id = Some(aid);
        }
        Some(aid)
    }

    /// Harvests the Kookadoba on `node_id` (`KookadobaPatchNode.Harvested`).
    pub fn harvest_node(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        node_id: &str,
    ) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };
        let Some(aid) = node.active_actor_id.take() else {
            return false;
        };
        model.destroy_actor(aid);
        physics.remove_body(aid);
        true
    }

    /// Cracks a `KookadobaBall` into `count` `KookadobaFruit` actors (default 3 in Wilds).
    pub fn crack_kookadoba_ball(
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        ball_actor_id: i64,
        count: usize,
        rng: &mut Randoms,
    ) -> Vec<i64> {
        let Some(ball) = model.actors.get(&ball_actor_id) else {
            return Vec::new();
        };
        if ball.ident != IdentifiableId::KookadobaBall {
            return Vec::new();
        }
        let pos = ball.pos;
        let region = ball.curr_region_set_id;
        model.destroy_actor(ball_actor_id);
        physics.remove_body(ball_actor_id);

        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let vel = Vec3::new(
                rng.get_in_range_f32(-3.0, 3.0),
                3.5,
                rng.get_in_range_f32(-3.0, 3.0),
            );
            out.push(spawn_physical_actor(
                model,
                physics,
                IdentifiableId::KookadobaFruit,
                region,
                pos + Vec3::Y * 0.4,
                vel,
            ));
        }
        out
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        rng: &mut Randoms,
    ) -> WildsStepEvents {
        let mut ev = WildsStepEvents::default();
        if self.nodes.is_empty() {
            return ev;
        }
        let wt = model.world.world_time;
        if self.next_spawn_time == 0.0 {
            self.next_spawn_time = wt + KOOKADOBA_SPAWN_DELAY_SECS;
            return ev;
        }

        // Clear references to already-harvested/destroyed Kookadobas
        for node in self.nodes.values_mut() {
            if let Some(aid) = node.active_actor_id {
                if !model.actors.contains_key(&aid) {
                    node.active_actor_id = None;
                }
            }
        }

        if wt >= self.next_spawn_time {
            self.next_spawn_time = wt + KOOKADOBA_SPAWN_DELAY_SECS;
            let mut candidates: Vec<String> = self
                .nodes
                .iter()
                .filter(|(_, n)| n.active_actor_id.is_none())
                .map(|(id, _)| id.clone())
                .collect();
            candidates.sort();
            if !candidates.is_empty() {
                let idx = rng.get_in_range_i32(0, candidates.len() as i32) as usize;
                let picked = candidates[idx.min(candidates.len() - 1)].clone();
                if let Some(aid) = self.grow_node(model, physics, &picked) {
                    ev.grown_nodes.push((picked, aid));
                }
            }
        }

        ev
    }
}
