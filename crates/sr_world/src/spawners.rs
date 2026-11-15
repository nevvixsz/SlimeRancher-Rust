//! Port of `ZoneDirector.cs`, `CellDirector.cs`, `DirectedActorSpawner.cs`,
//! `DirectedSlimeSpawner.cs`, `DirectedAnimalSpawner.cs`, `DirectedAuxItemSpawner.cs`,
//! `DirectedCrateSpawner.cs`, and `GingerPatchNode.cs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_gameplay::spawn_physical_actor;
use sr_ids::{IdentifiableId, SlimeEmotion, ZoneId};
use sr_math::{Randoms, Vec3};
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

use crate::region::{region_set_for_zone, RegionRegistry};

pub const DAILY_RESET_HOUR: f32 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnMember {
    pub ident: IdentifiableId,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnConstraint {
    /// Start hour (`0.0..24.0`). If `start_hour == end_hour`, always eligible.
    pub start_hour: f32,
    /// End hour (`0.0..24.0`). Supports wraparound (e.g., `18.0..6.0` for night).
    pub end_hour: f32,
    pub weight: f32,
    pub members: Vec<SpawnMember>,
    pub feral: bool,
    pub max_agitation: bool,
}

impl SpawnConstraint {
    pub fn always(members: Vec<SpawnMember>) -> Self {
        Self {
            start_hour: 0.0,
            end_hour: 24.0,
            weight: 1.0,
            members,
            feral: false,
            max_agitation: false,
        }
    }

    pub fn in_window(&self, hour: f32) -> bool {
        if (self.end_hour - self.start_hour).abs() < 1e-4 || self.end_hour - self.start_hour >= 24.0
        {
            return true;
        }
        if self.start_hour < self.end_hour {
            hour >= self.start_hour && hour <= self.end_hour
        } else {
            hour >= self.start_hour || hour <= self.end_hour
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorSpawner {
    pub pos: Vec3,
    pub radius: f32,
    pub weight: f32,
    pub constraints: Vec<SpawnConstraint>,
    pub spawn_locs: Vec<Vec3>,
}

impl Default for ActorSpawner {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            radius: 3.0,
            weight: 1.0,
            constraints: Vec::new(),
            spawn_locs: Vec::new(),
        }
    }
}

impl ActorSpawner {
    pub fn can_spawn(&self, hour: f32) -> bool {
        self.constraints
            .iter()
            .any(|c| c.in_window(hour) && !c.members.is_empty() && c.weight > 0.0)
    }

    fn pick_constraint(&self, hour: f32, rng: &mut Randoms) -> Option<&SpawnConstraint> {
        let eligible: Vec<&SpawnConstraint> = self
            .constraints
            .iter()
            .filter(|c| c.in_window(hour) && !c.members.is_empty() && c.weight > 0.0)
            .collect();
        if eligible.is_empty() {
            return None;
        }
        let total: f32 = eligible.iter().map(|c| c.weight).sum();
        let mut roll = rng.get_in_range_f32(0.0, total);
        for c in eligible {
            roll -= c.weight;
            if roll <= 0.0 {
                return Some(c);
            }
        }
        self.constraints.first()
    }

    fn pick_member(constraint: &SpawnConstraint, rng: &mut Randoms) -> Option<IdentifiableId> {
        let total: f32 = constraint.members.iter().map(|m| m.weight.max(0.0)).sum();
        if total <= 0.0 {
            return constraint.members.first().map(|m| m.ident);
        }
        let mut roll = rng.get_in_range_f32(0.0, total);
        for m in &constraint.members {
            roll -= m.weight.max(0.0);
            if roll <= 0.0 {
                return Some(m.ident);
            }
        }
        constraint.members.last().map(|m| m.ident)
    }

    fn sample_pos(&self, rng: &mut Randoms) -> Vec3 {
        if !self.spawn_locs.is_empty() {
            let idx = rng.get_in_range_i32(0, self.spawn_locs.len() as i32) as usize;
            return self.spawn_locs[idx];
        }
        let ang = rng.get_in_range_f32(0.0, std::f32::consts::TAU);
        let dist = rng.get_in_range_f32(0.0, self.radius);
        self.pos + Vec3::new(ang.cos() * dist, 0.5, ang.sin() * dist)
    }
}

/// Port of `CellDirector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellDirector {
    pub region_id: String,
    pub zone: ZoneId,
    pub is_ranch: bool,
    pub target_slime_count: usize,
    pub cull_slimes_limit: usize,
    pub min_per_spawn: i32,
    pub max_per_spawn: i32,
    pub target_animal_count: usize,
    pub cull_animals_limit: usize,
    pub min_animal_per_spawn: i32,
    pub max_animal_per_spawn: i32,
    pub avg_spawn_time_hours: f32,
    pub next_spawn_time: f64,
    pub slime_spawners: Vec<ActorSpawner>,
    pub animal_spawners: Vec<ActorSpawner>,
}

impl CellDirector {
    pub fn new(region_id: impl Into<String>, zone: ZoneId) -> Self {
        Self {
            region_id: region_id.into(),
            zone,
            is_ranch: zone == ZoneId::Ranch,
            target_slime_count: 10,
            cull_slimes_limit: 20,
            min_per_spawn: 2,
            max_per_spawn: 5,
            target_animal_count: 6,
            cull_animals_limit: 12,
            min_animal_per_spawn: 1,
            max_animal_per_spawn: 3,
            avg_spawn_time_hours: 1.5,
            next_spawn_time: 0.0,
            slime_spawners: Vec::new(),
            animal_spawners: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuxItemEntry {
    pub ident: IdentifiableId,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GingerPatchNode {
    pub id: String,
    pub pos: Vec3,
    pub active_actor_id: Option<i64>,
}

/// Port of `ZoneDirector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneDirector {
    pub zone: ZoneId,
    pub crate_ident: IdentifiableId,
    pub crate_spawners: Vec<Vec3>,
    pub aux_item_spawners: Vec<Vec3>,
    pub aux_items: Vec<AuxItemEntry>,
    pub ginger_patches: Vec<GingerPatchNode>,
    pub max_active_gingers: usize,
    pub active_crate_ids: Vec<i64>,
    pub active_aux_ids: Vec<i64>,
}

impl ZoneDirector {
    pub fn new(zone: ZoneId, crate_ident: IdentifiableId) -> Self {
        Self {
            zone,
            crate_ident,
            crate_spawners: Vec::new(),
            aux_item_spawners: Vec::new(),
            aux_items: Vec::new(),
            ginger_patches: Vec::new(),
            max_active_gingers: 2,
            active_crate_ids: Vec::new(),
            active_aux_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpawnerStepEvents {
    pub spawned_slimes: Vec<(i64, IdentifiableId)>,
    pub spawned_animals: Vec<(i64, IdentifiableId)>,
    pub spawned_crates: Vec<i64>,
    pub spawned_aux_items: Vec<(i64, IdentifiableId)>,
    pub spawned_gingers: Vec<i64>,
    pub culled_actors: Vec<i64>,
}

/// Coordinates all `CellDirector` and `ZoneDirector` spawning & daily 05:00 resets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldSpawnerSystem {
    pub cells: BTreeMap<String, CellDirector>,
    pub zones: BTreeMap<ZoneId, ZoneDirector>,
}

impl WorldSpawnerSystem {
    pub fn register_cell(&mut self, cell: CellDirector) {
        self.cells.insert(cell.region_id.clone(), cell);
    }

    pub fn register_zone(&mut self, zone: ZoneDirector) {
        self.zones.insert(zone.zone, zone);
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        regions: &RegionRegistry,
        mode: &GameModeSettings,
        world_time: f64,
        last_world_time: Option<f64>,
        curr_hour: f32,
        rng: &mut Randoms,
    ) -> SpawnerStepEvents {
        let mut ev = SpawnerStepEvents::default();

        // 1. Daily 05:00 reset for crates, aux items, and Desert Ginger patches
        let passed_5am = match last_world_time {
            Some(prev) if world_time > prev => {
                let prev_day_idx = ((prev - (DAILY_RESET_HOUR as f64) * 3600.0) / 86400.0).floor();
                let curr_day_idx =
                    ((world_time - (DAILY_RESET_HOUR as f64) * 3600.0) / 86400.0).floor();
                curr_day_idx > prev_day_idx
            }
            _ => false,
        };

        if passed_5am {
            self.reset_all_zones(model, physics, rng, &mut ev);
        }

        // 2. Per-cell slime & animal spawning (skipping hibernated regions)
        for cell in self.cells.values_mut() {
            let Some(reg) = regions.regions.get(&cell.region_id) else {
                continue;
            };
            if reg.is_hibernated() {
                continue;
            }

            let mut cell_slimes = Vec::new();
            let mut cell_animals = Vec::new();
            for &aid in &reg.members {
                if let Some(a) = model.actors.get(&aid) {
                    if a.ident.is_slime() {
                        cell_slimes.push(aid);
                    } else if a.ident.is_meat() {
                        cell_animals.push(aid);
                    }
                }
            }

            // Cull excess
            if !cell.is_ranch && cell_slimes.len() > cell.cull_slimes_limit {
                let excess = cell_slimes.len() - cell.cull_slimes_limit;
                for &aid in cell_slimes.iter().take(excess) {
                    model.destroy_actor(aid);
                    physics.remove_body(aid);
                    ev.culled_actors.push(aid);
                }
            }
            if !cell.is_ranch && cell_animals.len() > cell.cull_animals_limit {
                let excess = cell_animals.len() - cell.cull_animals_limit;
                for &aid in cell_animals.iter().take(excess) {
                    model.destroy_actor(aid);
                    physics.remove_body(aid);
                    ev.culled_actors.push(aid);
                }
            }

            if world_time < cell.next_spawn_time {
                continue;
            }

            let set_id = region_set_for_zone(cell.zone);

            // Slime spawn
            if cell_slimes.len() < cell.target_slime_count && !cell.slime_spawners.is_empty() {
                let eligible: Vec<&ActorSpawner> = cell
                    .slime_spawners
                    .iter()
                    .filter(|s| s.can_spawn(curr_hour))
                    .collect();
                if !eligible.is_empty() {
                    let idx = rng.get_in_range_i32(0, eligible.len() as i32) as usize;
                    let spawner = eligible[idx];
                    if let Some(constraint) = spawner.pick_constraint(curr_hour, rng) {
                        let count = rng.get_in_range_i32(cell.min_per_spawn, cell.max_per_spawn + 1);
                        for _ in 0..count {
                            if let Some(ident) = ActorSpawner::pick_member(constraint, rng) {
                                if ident.is_tarr() && mode.prevent_hostiles {
                                    continue;
                                }
                                let pos = spawner.sample_pos(rng);
                                let sid = spawn_physical_actor(
                                    model,
                                    physics,
                                    ident,
                                    set_id,
                                    pos,
                                    Vec3::ZERO,
                                );
                                if let Some(a) = model.actors.get_mut(&sid) {
                                    if let Some(sm) = a.as_slime_mut() {
                                        if constraint.feral && !mode.prevent_hostiles && ident.is_largo() {
                                            sm.is_feral = true;
                                        }
                                        if constraint.max_agitation {
                                            if let Some(ag) = sm.emotion_mut(SlimeEmotion::Agitation) {
                                                ag.curr_val = 1.0;
                                            }
                                        }
                                    }
                                }
                                ev.spawned_slimes.push((sid, ident));
                            }
                        }
                    }
                }
            }

            // Animal spawn
            if cell_animals.len() < cell.target_animal_count && !cell.animal_spawners.is_empty() {
                let eligible: Vec<&ActorSpawner> = cell
                    .animal_spawners
                    .iter()
                    .filter(|s| s.can_spawn(curr_hour))
                    .collect();
                if !eligible.is_empty() {
                    let idx = rng.get_in_range_i32(0, eligible.len() as i32) as usize;
                    let spawner = eligible[idx];
                    if let Some(constraint) = spawner.pick_constraint(curr_hour, rng) {
                        let count = rng.get_in_range_i32(
                            cell.min_animal_per_spawn,
                            cell.max_animal_per_spawn + 1,
                        );
                        for _ in 0..count {
                            if let Some(ident) = ActorSpawner::pick_member(constraint, rng) {
                                let pos = spawner.sample_pos(rng);
                                let aid = spawn_physical_actor(
                                    model,
                                    physics,
                                    ident,
                                    set_id,
                                    pos,
                                    Vec3::ZERO,
                                );
                                ev.spawned_animals.push((aid, ident));
                            }
                        }
                    }
                }
            }

            cell.next_spawn_time = world_time
                + (cell.avg_spawn_time_hours * 3600.0 * rng.get_in_range_f32(0.5, 1.5)) as f64;
        }

        ev
    }

    pub fn reset_all_zones(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        rng: &mut Randoms,
        ev: &mut SpawnerStepEvents,
    ) {
        for z in self.zones.values_mut() {
            let set_id = region_set_for_zone(z.zone);

            // Reset Crates
            z.active_crate_ids.retain(|id| model.actors.contains_key(id));
            if z.crate_ident != IdentifiableId::None {
                for &pos in &z.crate_spawners {
                    let nearby = z.active_crate_ids.iter().any(|cid| {
                        model
                            .actors
                            .get(cid)
                            .map_or(false, |a| a.pos.distance_squared(pos) < 4.0)
                    });
                    if !nearby {
                        let cid = spawn_physical_actor(
                            model,
                            physics,
                            z.crate_ident,
                            set_id,
                            pos,
                            Vec3::ZERO,
                        );
                        z.active_crate_ids.push(cid);
                        ev.spawned_crates.push(cid);
                    }
                }
            }

            // Reset Aux items (fruits/veggies)
            z.active_aux_ids.retain(|id| model.actors.contains_key(id));
            if !z.aux_items.is_empty() {
                for &pos in &z.aux_item_spawners {
                    let nearby = z.active_aux_ids.iter().any(|aid| {
                        model
                            .actors
                            .get(aid)
                            .map_or(false, |a| a.pos.distance_squared(pos) < 4.0)
                    });
                    if !nearby {
                        let total: f32 = z.aux_items.iter().map(|e| e.weight.max(0.0)).sum();
                        let mut roll = rng.get_in_range_f32(0.0, total.max(0.01));
                        let mut chosen = z.aux_items[0].ident;
                        for e in &z.aux_items {
                            roll -= e.weight.max(0.0);
                            if roll <= 0.0 {
                                chosen = e.ident;
                                break;
                            }
                        }
                        let aid = spawn_physical_actor(model, physics, chosen, set_id, pos, Vec3::ZERO);
                        z.active_aux_ids.push(aid);
                        ev.spawned_aux_items.push((aid, chosen));
                    }
                }
            }

            // Reset GingerPatchNodes (Glass Desert)
            if !z.ginger_patches.is_empty() {
                for node in &mut z.ginger_patches {
                    if let Some(old_id) = node.active_actor_id.take() {
                        model.destroy_actor(old_id);
                        physics.remove_body(old_id);
                    }
                }
                let count = z.max_active_gingers.min(z.ginger_patches.len());
                let mut indices: Vec<usize> = (0..z.ginger_patches.len()).collect();
                for _ in 0..count {
                    if indices.is_empty() {
                        break;
                    }
                    let pick = rng.get_in_range_i32(0, indices.len() as i32) as usize;
                    let idx = indices.swap_remove(pick);
                    let pos = z.ginger_patches[idx].pos;
                    let gid = spawn_physical_actor(
                        model,
                        physics,
                        IdentifiableId::GingerVeggie,
                        set_id,
                        pos,
                        Vec3::ZERO,
                    );
                    z.ginger_patches[idx].active_actor_id = Some(gid);
                    ev.spawned_gingers.push(gid);
                }
            }
        }
    }
}
