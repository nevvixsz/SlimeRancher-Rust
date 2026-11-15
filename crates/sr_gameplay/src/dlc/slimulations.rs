//! Port of `GlitchCellDirector.cs`, `GlitchImpostoDirector.cs`, `GlitchImposto.cs`,
//! `GlitchLiquidSource.cs`, `GlitchTarrNode.cs`, `GlitchTarrSterilizeOnWater.cs`,
//! `GlitchStorage.cs`, and `GlitchMetadata.cs` (Viktor's Slimulation).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{AmmoMode, IdentifiableId, PlayerUpgrade, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{
    GameModel, GlitchImpostoModel, GlitchTarrNodeModel, LiquidSourceModel, TutorialId,
};
use sr_physics::PhysicsWorld;

use crate::actors::spawn_physical_actor;
use crate::ammo::{AmmoController, GLITCH_DEBUG_SPRAY_MAX_AMMO};
use crate::story::{ProgressDirector, ProgressStepEvents, TutorialDirector};

pub const DEFAULT_IMPOSTO_COOLDOWN_HOURS: f32 = 2.0;
pub const DEFAULT_TARR_LIFETIME_MINS: f32 = 20.0;
pub const DEFAULT_TARR_BASE_MULTIPLY_CHANCE: f32 = 0.5;
pub const DEFAULT_TARR_MULTIPLY_DEGRADATION: f32 = 0.25;
pub const DEFAULT_TARR_SPAWNER_THROTTLE_MINS: f32 = 1.0;
pub const DEFAULT_TARR_NODE_ACTIVATION_DELAY_HOURS: f32 = 0.5;
pub const DEFAULT_DAMAGE_LOSS_COOLDOWN_MINS: f32 = 0.25;
pub const DEFAULT_EXIT_TELEPORTER_DELAY_HOURS: f32 = 0.5;
pub const GLITCH_STORAGE_MAX_COUNT: i32 = 300;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpostoConfig {
    pub id: String,
    pub pos: Vec3,
    pub spawn_pos: Vec3,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TarrNodeConfig {
    pub id: String,
    pub pos: Vec3,
    pub dist_sq_from_entrance: f32,
    pub spawner_positions: Vec<Vec3>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlitchTarrRuntime {
    pub actor_id: i64,
    pub death_time: f64,
    pub multiply_chance: f32,
    pub sterilized: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SlimulationsStepEvents {
    pub activated_nodes: Vec<String>,
    pub spawned_tarrs: Vec<i64>,
    pub split_tarrs: Vec<i64>,
    pub exit_activated: bool,
}

/// Runtime manager for Viktor's Slimulation (`GlitchCellDirector`, `GlitchImpostoDirector`,
/// `GlitchTarrNode`, `GlitchLiquidSource`, and `GlitchStorage`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlimulationsSystem {
    pub impostos: HashMap<String, ImpostoConfig>,
    pub imposto_cooldown_until: HashMap<String, f64>,
    pub tarr_nodes: HashMap<String, TarrNodeConfig>,
    pub active_tarrs: HashMap<i64, GlitchTarrRuntime>,
    pub next_tarr_spawn_time: f64,
    pub next_node_activation_time: f64,
    pub target_tarr_count: usize,
    pub in_slimulation: bool,
    pub exit_teleporter_active: bool,
    pub exit_activation_time: Option<f64>,
    pub next_ammo_loss_damage_time: f64,
    pub storage_id: Option<IdentifiableId>,
    pub storage_count: i32,
}

impl SlimulationsSystem {
    pub fn register_imposto(&mut self, model: &mut GameModel, cfg: ImpostoConfig) {
        let id = cfg.id.clone();
        model
            .glitch
            .impostos
            .entry(id.clone())
            .or_insert_with(|| GlitchImpostoModel {
                id: id.clone(),
                is_active: true,
            });
        self.impostos.insert(id, cfg);
    }

    pub fn register_tarr_node(&mut self, model: &mut GameModel, cfg: TarrNodeConfig) {
        let id = cfg.id.clone();
        model
            .glitch
            .nodes
            .entry(id.clone())
            .or_insert_with(|| GlitchTarrNodeModel {
                id: id.clone(),
                is_active: false,
            });
        self.tarr_nodes.insert(id, cfg);
    }

    pub fn register_debug_spray_source(
        &self,
        model: &mut GameModel,
        source_id: impl Into<String>,
        pos: Vec3,
    ) {
        model.liquid_sources.insert(
            source_id.into(),
            LiquidSourceModel {
                pos,
                is_scaling: false,
                units_filled: 1.0,
            },
        );
    }

    /// Checks whether the player can access Viktor's Lab / Slimulation (`UnlockLab` + `UnlockRuins` + `TreasureCracker2`).
    pub fn can_enter_viktor_lab(model: &GameModel, mode: &GameModeSettings) -> bool {
        mode.enable_viktor_missions
            && (model.progress.has_progress(ProgressType::UnlockViktorMissions)
                || (model.progress.has_progress(ProgressType::UnlockLab)
                    && model.progress.has_progress(ProgressType::UnlockRuins)
                    && model
                        .player
                        .upgrades
                        .contains(&PlayerUpgrade::TreasureCracker2)))
    }

    /// Enters Viktor's Lab (`EnterZoneViktorLab`).
    pub fn enter_viktor_lab(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
    ) -> Option<ProgressStepEvents> {
        if !Self::can_enter_viktor_lab(model, mode) {
            return None;
        }
        let mut ev = ProgressStepEvents::default();
        progress.set_unique_progress(model, mode, ProgressType::UnlockViktorMissions, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::EnterZoneViktorLab, &mut ev);
        Some(ev)
    }

    /// Enters the Slimulation (`GlitchCellDirector` resets cells, schedules exit teleporter & tarr node activation).
    pub fn enter_slimulation(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
        tutorials: &mut TutorialDirector,
    ) -> Option<ProgressStepEvents> {
        if !Self::can_enter_viktor_lab(model, mode) {
            return None;
        }
        let mut ev = ProgressStepEvents::default();
        progress.set_unique_progress(model, mode, ProgressType::UnlockSlimulations, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::EnterZoneSlimulation, &mut ev);

        let wt = model.world.world_time;
        self.in_slimulation = true;
        self.exit_teleporter_active = false;
        self.exit_activation_time = Some(wt + f64::from(DEFAULT_EXIT_TELEPORTER_DELAY_HOURS) * 3600.0);
        self.next_node_activation_time =
            wt + f64::from(DEFAULT_TARR_NODE_ACTIVATION_DELAY_HOURS) * 3600.0;
        self.next_tarr_spawn_time = wt + f64::from(DEFAULT_TARR_SPAWNER_THROTTLE_MINS) * 60.0;
        if self.target_tarr_count == 0 {
            self.target_tarr_count = 6;
        }
        // Reset debug spray liquid sources & impostos on entry
        for src in model.liquid_sources.values_mut() {
            src.units_filled = 1.0;
        }
        self.reset_impostos(model);
        tutorials.maybe_show_popup(model, mode, TutorialId::SlimulationsStart1);
        Some(ev)
    }

    /// Exits the Slimulation back to Viktor's Lab, converting captured `GlitchSlime` into `GlitchBugReport`.
    pub fn exit_slimulation(&mut self, model: &mut GameModel) -> i32 {
        self.in_slimulation = false;
        self.exit_teleporter_active = false;
        self.exit_activation_time = None;
        let mut converted = 0;
        if let Some(ammo) = model.player.ammo_dict.get_mut(&AmmoMode::Default) {
            for slot_opt in ammo.slots.iter_mut() {
                if let Some(slot) = slot_opt.as_mut() {
                    if slot.id == IdentifiableId::GlitchSlime {
                        converted += slot.count;
                        slot.id = IdentifiableId::GlitchBugReport;
                        slot.emotions.clear();
                    }
                }
            }
        }
        converted
    }

    /// Port of `GlitchImpostoDirector.ResetImpostos` — reactivates off-cooldown impostos.
    pub fn reset_impostos(&mut self, model: &mut GameModel) {
        let wt = model.world.world_time;
        for (id, imp_model) in model.glitch.impostos.iter_mut() {
            let ready = self
                .imposto_cooldown_until
                .get(id)
                .map(|&t| wt >= t)
                .unwrap_or(true);
            imp_model.is_active = ready;
        }
    }

    /// Consumes a `GlitchLiquidSource` station, refilling `GlitchDebugSprayLiquid` in the vacpack liquid slot.
    pub fn consume_debug_spray_source(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ammo: &mut AmmoController,
        tutorials: &mut TutorialDirector,
        source_id: &str,
    ) -> bool {
        let Some(src) = model.liquid_sources.get_mut(source_id) else {
            return false;
        };
        if src.units_filled <= 0.0 {
            return false;
        }
        let wt = model.world.world_time;
        let replaced = ammo.replace_slot(
            &mut model.player,
            4,
            IdentifiableId::GlitchDebugSprayLiquid,
            wt,
        );
        if !replaced {
            return false;
        }
        if let Some(am) = model.player.ammo_dict.get_mut(&AmmoMode::Default) {
            if let Some(Some(slot)) = am.slots.get_mut(4) {
                slot.count = GLITCH_DEBUG_SPRAY_MAX_AMMO;
            }
        }
        src.units_filled = 0.0;
        tutorials.maybe_show_popup(model, mode, TutorialId::SlimulationsDebugSpray);
        true
    }

    /// Exposes a `GlitchImposto` with `GlitchDebugSprayLiquid`, spawning `GlitchSlime` actors.
    pub fn expose_imposto(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        imposto_id: &str,
        rng: &mut Randoms,
    ) -> Vec<i64> {
        let Some(imp_model) = model.glitch.impostos.get_mut(imposto_id) else {
            return Vec::new();
        };
        if !imp_model.is_active {
            return Vec::new();
        }
        let Some(cfg) = self.impostos.get(imposto_id).cloned() else {
            return Vec::new();
        };
        imp_model.is_active = false;
        let wt = model.world.world_time;
        self.imposto_cooldown_until.insert(
            imposto_id.to_string(),
            wt + f64::from(DEFAULT_IMPOSTO_COOLDOWN_HOURS) * 3600.0,
        );

        let count = rng.get_in_range_i32(1, 4) as usize;
        let mut spawned = Vec::with_capacity(count);
        for _ in 0..count {
            let vel = Vec3::new(
                rng.get_in_range_f32(-3.5, 3.5),
                4.0,
                rng.get_in_range_f32(-3.5, 3.5),
            );
            spawned.push(spawn_physical_actor(
                model,
                physics,
                IdentifiableId::GlitchSlime,
                RegionSetId::Slimulations,
                cfg.spawn_pos + Vec3::Y * 0.4,
                vel,
            ));
        }
        spawned
    }

    /// Sterilizes a `GlitchTarrSlime` with water or debug spray, preventing splitting (`GlitchTarrSterilizeOnWater`).
    pub fn sterilize_tarr(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        tarr_actor_id: i64,
    ) -> bool {
        if let Some(rt) = self.active_tarrs.get_mut(&tarr_actor_id) {
            rt.sterilized = true;
            self.active_tarrs.remove(&tarr_actor_id);
            model.destroy_actor(tarr_actor_id);
            physics.remove_body(tarr_actor_id);
            return true;
        }
        false
    }

    /// Port of `GlitchMetadata.MaybeDamageExposure` — player loses some `GlitchSlime` ammo when hit.
    pub fn apply_tarr_damage_exposure(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        tutorials: &mut TutorialDirector,
        rng: &mut Randoms,
    ) -> i32 {
        let wt = model.world.world_time;
        if wt < self.next_ammo_loss_damage_time {
            return 0;
        }
        let loss_target = rng.get_in_range_i32(1, 4);
        let mut lost = 0;
        if let Some(am) = model.player.ammo_dict.get_mut(&AmmoMode::Default) {
            for slot_opt in am.slots.iter_mut() {
                if let Some(slot) = slot_opt.as_mut() {
                    if slot.id == IdentifiableId::GlitchSlime && slot.count > 0 {
                        let take = loss_target.min(slot.count);
                        slot.count -= take;
                        lost = take;
                        if slot.count == 0 {
                            *slot_opt = None;
                        }
                        break;
                    }
                }
            }
        }
        if lost > 0 {
            self.next_ammo_loss_damage_time =
                wt + f64::from(DEFAULT_DAMAGE_LOSS_COOLDOWN_MINS) * 60.0;
            tutorials.maybe_show_popup(model, mode, TutorialId::SlimulationsDamage);
        }
        lost
    }

    /// Deposits `GlitchBugReport` into Viktor's `GlitchStorage` (capacity 300).
    pub fn store_glitch_item(&mut self, id: IdentifiableId) -> bool {
        if id == IdentifiableId::None || id.is_slime() {
            return false;
        }
        if self.storage_count >= GLITCH_STORAGE_MAX_COUNT {
            return false;
        }
        if self.storage_count > 0 && self.storage_id != Some(id) {
            return false;
        }
        self.storage_id = Some(id);
        self.storage_count += 1;
        true
    }

    pub fn spawn_glitch_tarr(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        pos: Vec3,
        multiply_chance: f32,
        rng: &mut Randoms,
    ) -> i64 {
        let aid = spawn_physical_actor(
            model,
            physics,
            IdentifiableId::GlitchTarrSlime,
            RegionSetId::Slimulations,
            pos,
            Vec3::ZERO,
        );
        let lifetime = DEFAULT_TARR_LIFETIME_MINS + rng.get_in_range_f32(-3.0, 3.0);
        self.active_tarrs.insert(
            aid,
            GlitchTarrRuntime {
                actor_id: aid,
                death_time: model.world.world_time + f64::from(lifetime) * 60.0,
                multiply_chance,
                sterilized: false,
            },
        );
        aid
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        physics: &mut PhysicsWorld,
        tutorials: &mut TutorialDirector,
        rng: &mut Randoms,
    ) -> SlimulationsStepEvents {
        let mut ev = SlimulationsStepEvents::default();
        if !self.in_slimulation {
            return ev;
        }
        let wt = model.world.world_time;

        // 1. Activate exit teleporter after delay
        if !self.exit_teleporter_active {
            if let Some(t) = self.exit_activation_time {
                if wt >= t {
                    self.exit_teleporter_active = true;
                    ev.exit_activated = true;
                    tutorials.maybe_show_popup(model, mode, TutorialId::SlimulationsExitAvailable);
                }
            }
        }

        // 2. Periodically activate next inactive GlitchTarrNode
        if wt >= self.next_node_activation_time {
            self.next_node_activation_time =
                wt + f64::from(DEFAULT_TARR_NODE_ACTIVATION_DELAY_HOURS) * 3600.0;
            let mut inactive: Vec<String> = model
                .glitch
                .nodes
                .iter()
                .filter(|(_, n)| !n.is_active)
                .map(|(id, _)| id.clone())
                .collect();
            inactive.sort();
            if let Some(picked) = inactive.first() {
                if let Some(n) = model.glitch.nodes.get_mut(picked) {
                    n.is_active = true;
                    ev.activated_nodes.push(picked.clone());
                }
            }
        }

        // 3. Tarr lifecycle & multiplication on timed death
        self.active_tarrs
            .retain(|aid, _| model.actors.contains_key(aid));
        let expired: Vec<GlitchTarrRuntime> = self
            .active_tarrs
            .values()
            .filter(|t| wt >= t.death_time)
            .cloned()
            .collect();
        for rt in expired {
            self.active_tarrs.remove(&rt.actor_id);
            let pos = model
                .actors
                .get(&rt.actor_id)
                .map(|a| a.pos)
                .unwrap_or(Vec3::ZERO);
            model.destroy_actor(rt.actor_id);
            physics.remove_body(rt.actor_id);
            if !rt.sterilized && rng.get_probability(rt.multiply_chance) {
                let child_chance = rt.multiply_chance * (1.0 - DEFAULT_TARR_MULTIPLY_DEGRADATION);
                for _ in 0..2 {
                    let jitter = Vec3::new(
                        rng.get_in_range_f32(-1.5, 1.5),
                        0.5,
                        rng.get_in_range_f32(-1.5, 1.5),
                    );
                    let child = self.spawn_glitch_tarr(
                        model,
                        physics,
                        pos + jitter,
                        child_chance,
                        rng,
                    );
                    ev.split_tarrs.push(child);
                }
            }
        }

        // 4. Spawn new GlitchTarrSlime from active nodes up to target_tarr_count
        if wt >= self.next_tarr_spawn_time && self.active_tarrs.len() < self.target_tarr_count {
            self.next_tarr_spawn_time = wt + f64::from(DEFAULT_TARR_SPAWNER_THROTTLE_MINS) * 60.0;
            let mut active_spawners: Vec<Vec3> = Vec::new();
            for (nid, n_model) in &model.glitch.nodes {
                if n_model.is_active {
                    if let Some(cfg) = self.tarr_nodes.get(nid) {
                        active_spawners.extend_from_slice(&cfg.spawner_positions);
                    }
                }
            }
            if !active_spawners.is_empty() {
                let idx = rng.get_in_range_i32(0, active_spawners.len() as i32) as usize;
                let pos = active_spawners[idx.min(active_spawners.len() - 1)];
                let tid = self.spawn_glitch_tarr(
                    model,
                    physics,
                    pos,
                    DEFAULT_TARR_BASE_MULTIPLY_CHANCE,
                    rng,
                );
                ev.spawned_tarrs.push(tid);
            }
        }

        ev
    }
}
