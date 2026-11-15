//! Port of `AccessDoor.cs`, `SlimeGateActivator.cs`, `PuzzleSlot.cs`,
//! `PuzzleSlotLockable.cs`, and `TreasurePod.cs`.

use serde::{Deserialize, Serialize};
use sr_gameplay::{spawn_physical_actor, PlayerController};
use sr_ids::{GadgetId, IdentifiableId, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{AccessDoorModel, AccessDoorState, GameModel, PuzzleSlotModel, TreasurePodModel, TreasurePodState};
use sr_physics::PhysicsWorld;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PodTier {
    Green = 1,
    Blue = 2,
    Purple = 3,
}

impl PodTier {
    pub const fn required_cracker_level(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeGateConfig {
    pub gate_id: String,
    pub door_id: String,
    pub unlock_progress: Option<ProgressType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuzzleSlotConfig {
    pub slot_id: String,
    pub door_id: String,
    pub catch_id: IdentifiableId,
    pub pos: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuzzleDoorConfig {
    pub door_id: String,
    pub slot_ids: Vec<String>,
    pub unlock_progress: Option<ProgressType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreasurePodConfig {
    pub pod_id: String,
    pub tier: PodTier,
    pub pos: Vec3,
    pub region: RegionSetId,
    pub item_prizes: Vec<IdentifiableId>,
    pub blueprint_prizes: Vec<GadgetId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatePodError {
    NotFound,
    NoKeys,
    AlreadyOpen,
    InsufficientCrackerLevel { required: i32, current: i32 },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GatePodEvents {
    pub filled_puzzle_slots: Vec<String>,
    pub unlocked_doors: Vec<String>,
}

/// Ensures an `AccessDoorModel` entry exists in `GameModel::doors`.
pub fn ensure_door(model: &mut GameModel, door_id: impl Into<String>, default_state: AccessDoorState) {
    let id = door_id.into();
    model.doors.entry(id.clone()).or_insert(AccessDoorModel {
        id,
        state: default_state,
    });
}

/// Port of `SlimeGateActivator.Activate()`: spends 1 `SlimeKey`, opens the door,
/// increments `ProgressType::SlimeDoors`, and sets any zone unlock progress.
pub fn activate_slime_gate(
    model: &mut GameModel,
    gate: &SlimeGateConfig,
) -> Result<(), GatePodError> {
    let door_state = model
        .doors
        .get(&gate.door_id)
        .ok_or(GatePodError::NotFound)?
        .state;
    if door_state == AccessDoorState::Open {
        return Err(GatePodError::AlreadyOpen);
    }
    if model.player.keys <= 0 {
        return Err(GatePodError::NoKeys);
    }
    model.player.keys -= 1;
    if let Some(door) = model.doors.get_mut(&gate.door_id) {
        door.state = AccessDoorState::Open;
    }
    let doors_count = model.progress.get_progress(ProgressType::SlimeDoors);
    model
        .progress
        .set_progress(ProgressType::SlimeDoors, doors_count + 1);
    if let Some(p) = gate.unlock_progress {
        if model.progress.get_progress(p) == 0 {
            model.progress.set_progress(p, 1);
        }
    }
    Ok(())
}

/// Checks world actors overlapping unfilled `PuzzleSlot`s (Ruins / Glass Desert ancient gates),
/// consumes matching plorts, and opens any `PuzzleDoorConfig` whose slots are all filled.
pub fn step_puzzle_doors(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    slots: &[PuzzleSlotConfig],
    doors: &[PuzzleDoorConfig],
) -> GatePodEvents {
    let mut ev = GatePodEvents::default();

    for slot_cfg in slots {
        let filled = model
            .puzzle_slots
            .entry(slot_cfg.slot_id.clone())
            .or_insert_with(|| PuzzleSlotModel {
                id: slot_cfg.slot_id.clone(),
                filled: false,
            })
            .filled;
        if filled {
            continue;
        }
        let r2 = slot_cfg.radius * slot_cfg.radius;
        let matching_actor = model
            .actors
            .iter()
            .find(|(_, a)| a.ident == slot_cfg.catch_id && a.pos.distance_squared(slot_cfg.pos) <= r2)
            .map(|(id, _)| *id);

        if let Some(aid) = matching_actor {
            model.destroy_actor(aid);
            physics.remove_body(aid);
            if let Some(ps) = model.puzzle_slots.get_mut(&slot_cfg.slot_id) {
                ps.filled = true;
            }
            ev.filled_puzzle_slots.push(slot_cfg.slot_id.clone());
        }
    }

    for door_cfg in doors {
        ensure_door(model, door_cfg.door_id.clone(), AccessDoorState::Locked);
        let is_locked = model
            .doors
            .get(&door_cfg.door_id)
            .map_or(false, |d| d.state != AccessDoorState::Open);
        if !is_locked {
            continue;
        }
        let all_filled = door_cfg.slot_ids.iter().all(|sid| {
            model
                .puzzle_slots
                .get(sid)
                .map_or(false, |ps| ps.filled)
        });
        if all_filled {
            if let Some(d) = model.doors.get_mut(&door_cfg.door_id) {
                d.state = AccessDoorState::Open;
            }
            let plort_doors = model.progress.get_progress(ProgressType::PlortDoor);
            model
                .progress
                .set_progress(ProgressType::PlortDoor, plort_doors + 1);
            if let Some(p) = door_cfg.unlock_progress {
                if model.progress.get_progress(p) == 0 {
                    model.progress.set_progress(p, 1);
                }
            }
            ev.unlocked_doors.push(door_cfg.door_id.clone());
        }
    }

    ev
}

/// Port of `TreasurePod.Activate()`: verifies `TreasureCracker 1..4` level, opens the pod,
/// awards blueprints (`GadgetsModel::blueprints`), and spawns item prizes into the world.
pub fn open_treasure_pod(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    pod: &TreasurePodConfig,
    rng: &mut Randoms,
) -> Result<Vec<i64>, GatePodError> {
    let state = model
        .pods
        .entry(pod.pod_id.clone())
        .or_insert_with(|| TreasurePodModel {
            id: pod.pod_id.clone(),
            state: TreasurePodState::Locked,
            spawn_queue: Vec::new(),
        })
        .state;

    if state == TreasurePodState::Open {
        return Err(GatePodError::AlreadyOpen);
    }

    let req = pod.tier.required_cracker_level();
    let curr = PlayerController::treasure_cracker_level(&model.player);
    if curr < req {
        return Err(GatePodError::InsufficientCrackerLevel {
            required: req,
            current: curr,
        });
    }

    if let Some(pm) = model.pods.get_mut(&pod.pod_id) {
        pm.state = TreasurePodState::Open;
        pm.spawn_queue.clear();
    }

    for &bp in &pod.blueprint_prizes {
        if !model.gadgets.blueprints.contains(&bp) {
            model.gadgets.blueprints.push(bp);
        }
        model.gadgets.avail_blueprints.retain(|&b| b != bp);
    }

    let mut spawned = Vec::with_capacity(pod.item_prizes.len());
    for &item in &pod.item_prizes {
        let jitter = Vec3::new(
            rng.get_in_range_f32(-0.6, 0.6),
            0.7,
            rng.get_in_range_f32(-0.6, 0.6),
        );
        let aid = spawn_physical_actor(
            model,
            physics,
            item,
            pod.region,
            pod.pos + jitter,
            jitter * 3.0 + Vec3::Y * 3.0,
        );
        spawned.push(aid);
    }

    Ok(spawned)
}
