//! Port of `QuicksilverEnergyGenerator.cs`, `QuicksilverEnergyCheckpoint.cs`,
//! `QuicksilverAmmoReplacer.cs`, `QuicksilverSlimeSpawner.cs`, `QuicksilverElectricField.cs`,
//! and `QuicksilverPlortCollector.cs` (Mochi's Nimble Valley).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{AmmoMode, IdentifiableId, PediaId, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{GameModel, QuicksilverEnergyGeneratorModel, QuicksilverGeneratorState};
use sr_physics::PhysicsWorld;

use crate::actors::spawn_physical_actor;
use crate::ammo::AmmoController;
use crate::story::{PediaDirector, ProgressDirector, ProgressStepEvents};

pub const DEFAULT_GENERATOR_COUNTDOWN_MINS: f32 = 0.05;
pub const DEFAULT_GENERATOR_ACTIVE_MINS: f32 = 30.0;
pub const DEFAULT_GENERATOR_COOLDOWN_MINS: f32 = 15.0;
pub const DEFAULT_PLORT_COLLECT_MINS: f32 = 1.5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub id: String,
    pub countdown_minutes: f32,
    pub active_minutes: f32,
    pub max_active_minutes: f32,
    pub cooldown_minutes: f32,
    pub spawn_positions: Vec<Vec3>,
    pub target_slime_count: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            countdown_minutes: DEFAULT_GENERATOR_COUNTDOWN_MINS,
            active_minutes: DEFAULT_GENERATOR_ACTIVE_MINS,
            max_active_minutes: 60.0,
            cooldown_minutes: DEFAULT_GENERATOR_COOLDOWN_MINS,
            spawn_positions: Vec::new(),
            target_slime_count: 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmoReplacerState {
    pub id: String,
    pub ammo_id: IdentifiableId,
    pub cooldown_minutes: f32,
    pub next_available_time: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricFieldState {
    pub pos: Vec3,
    pub radius: f32,
    pub expire_time: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValleyStepEvents {
    pub activated_generators: Vec<String>,
    pub ended_generators: Vec<String>,
    pub spawned_slimes: Vec<i64>,
    pub produced_plorts: Vec<i64>,
    pub collected_plorts: i32,
    pub pedia_unlocked: Option<PediaId>,
}

/// Runtime manager for Mochi's Nimble Valley race loops.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValleySystem {
    pub generators: HashMap<String, GeneratorConfig>,
    pub generator_slimes: HashMap<String, Vec<i64>>,
    pub replacers: HashMap<String, AmmoReplacerState>,
    pub electric_fields: Vec<ElectricFieldState>,
    pub pending_plort_collect_at: HashMap<i64, f64>,
}

impl ValleySystem {
    pub fn register_generator(&mut self, model: &mut GameModel, cfg: GeneratorConfig) {
        let id = cfg.id.clone();
        model
            .quicksilver_generators
            .entry(id.clone())
            .or_insert_with(|| QuicksilverEnergyGeneratorModel {
                id: id.clone(),
                state: QuicksilverGeneratorState::Inactive,
                timer: None,
            });
        self.generators.insert(id, cfg);
    }

    pub fn register_replacer(
        &mut self,
        id: impl Into<String>,
        ammo_id: IdentifiableId,
        cooldown_minutes: f32,
    ) {
        let id = id.into();
        self.replacers.insert(
            id.clone(),
            AmmoReplacerState {
                id,
                ammo_id,
                cooldown_minutes,
                next_available_time: 0.0,
            },
        );
    }

    /// Checks whether the player can access Mochi's Manor / Nimble Valley.
    pub fn can_enter_valley(model: &GameModel, mode: &GameModeSettings) -> bool {
        mode.enable_mochi_missions
            && (model.progress.has_progress(ProgressType::UnlockMochiMissions)
                || (model.progress.has_progress(ProgressType::UnlockQuarry)
                    && model.progress.has_progress(ProgressType::UnlockGrotto)))
    }

    /// Enters Nimble Valley (`AmmoMode::NimbleValley`), recording `UnlockValley` and `EnterZoneMochiRanch`.
    pub fn enter_valley(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
        ammo: &mut AmmoController,
    ) -> Option<ProgressStepEvents> {
        if !Self::can_enter_valley(model, mode) {
            return None;
        }
        let mut ev = ProgressStepEvents::default();
        progress.set_unique_progress(model, mode, ProgressType::UnlockMochiMissions, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::UnlockValley, &mut ev);
        progress.set_unique_progress(model, mode, ProgressType::EnterZoneMochiRanch, &mut ev);
        model.player.ammo_mode = AmmoMode::NimbleValley;
        ammo.selected_idx = 0;
        Some(ev)
    }

    /// Exits Nimble Valley back to `AmmoMode::Default`, clearing transient race ammo (`ValleyAmmo1..4`).
    pub fn exit_valley(&self, model: &mut GameModel, ammo: &mut AmmoController) {
        if let Some(valley_ammo) = model.player.ammo_dict.get_mut(&AmmoMode::NimbleValley) {
            for slot_idx in [1usize, 2usize] {
                if let Some(slot) = valley_ammo.slots.get_mut(slot_idx) {
                    *slot = None;
                }
            }
        }
        model.player.ammo_mode = AmmoMode::Default;
        ammo.selected_idx = 0;
    }

    /// Activates an inactive `QuicksilverEnergyGenerator` (`Inactive -> Countdown` or `Active`).
    pub fn activate_generator(&self, model: &mut GameModel, generator_id: &str) -> bool {
        let Some(cfg) = self.generators.get(generator_id) else {
            return false;
        };
        let Some(gen_model) = model.quicksilver_generators.get_mut(generator_id) else {
            return false;
        };
        if gen_model.state != QuicksilverGeneratorState::Inactive {
            return false;
        }
        let wt = model.world.world_time;
        if cfg.countdown_minutes > 0.0 {
            gen_model.state = QuicksilverGeneratorState::Countdown;
            gen_model.timer = Some(wt + f64::from(cfg.countdown_minutes) * 60.0);
        } else {
            gen_model.state = QuicksilverGeneratorState::Active;
            gen_model.timer = Some(wt + f64::from(cfg.active_minutes) * 60.0);
        }
        true
    }

    /// Extends an active generator's remaining time (`QuicksilverEnergyCheckpoint`).
    pub fn trigger_checkpoint(
        &self,
        model: &mut GameModel,
        generator_id: &str,
        bonus_minutes: f32,
    ) -> bool {
        let Some(cfg) = self.generators.get(generator_id) else {
            return false;
        };
        let Some(gen_model) = model.quicksilver_generators.get_mut(generator_id) else {
            return false;
        };
        if gen_model.state != QuicksilverGeneratorState::Active {
            return false;
        }
        let wt = model.world.world_time;
        let max_end = wt + f64::from(cfg.max_active_minutes) * 60.0;
        let base_end = gen_model.timer.unwrap_or(wt);
        gen_model.timer = Some((base_end + f64::from(bonus_minutes) * 60.0).min(max_end));
        true
    }

    /// Port of `QuicksilverAmmoReplacer` — fills the appropriate Nimble Valley slot (`1` or `2`).
    pub fn trigger_ammo_replacer(
        &mut self,
        model: &mut GameModel,
        ammo: &mut AmmoController,
        replacer_id: &str,
    ) -> bool {
        if model.player.ammo_mode != AmmoMode::NimbleValley {
            return false;
        }
        let Some(rep) = self.replacers.get_mut(replacer_id) else {
            return false;
        };
        let wt = model.world.world_time;
        if wt < rep.next_available_time {
            return false;
        }
        let slot_idx = if rep.ammo_id == IdentifiableId::ValleyAmmo1 {
            1
        } else {
            2
        };
        let max_c = AmmoController::slot_max_count(&model.player, rep.ammo_id, slot_idx);
        let replaced = ammo.replace_slot(&mut model.player, slot_idx, rep.ammo_id, wt);
        if replaced && max_c > 1 {
            // Fill to stack limit if slot supports multiple charges (e.g. ValleyAmmo2..4 up to 3)
            if let Some(am) = model.player.ammo_dict.get_mut(&AmmoMode::NimbleValley) {
                if let Some(Some(slot)) = am.slots.get_mut(slot_idx) {
                    slot.count = max_c;
                }
            }
        }
        if replaced {
            rep.next_available_time = wt + f64::from(rep.cooldown_minutes) * 60.0;
        }
        replaced
    }

    /// Spawns a temporary `QuicksilverElectricField` (`ValleyAmmo2` / pulse shock).
    pub fn spawn_electric_field(&mut self, world_time: f64, pos: Vec3, radius: f32, active_minutes: f32) {
        self.electric_fields.push(ElectricFieldState {
            pos,
            radius,
            expire_time: world_time + f64::from(active_minutes) * 60.0,
        });
    }

    /// Electrifies nearby `QuicksilverSlime` actors around `pos`, producing `QuicksilverPlort`.
    pub fn shock_slimes_in_radius(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        pos: Vec3,
        radius: f32,
    ) -> Vec<i64> {
        let r2 = radius * radius;
        let targets: Vec<(i64, Vec3)> = model
            .actors_of_type(IdentifiableId::QuicksilverSlime)
            .filter(|a| a.pos.distance_squared(pos) <= r2)
            .map(|a| (a.actor_id, a.pos))
            .collect();

        let wt = model.world.world_time;
        let mut plorts = Vec::with_capacity(targets.len());
        for (_slime_id, slime_pos) in targets {
            let pid = spawn_physical_actor(
                model,
                physics,
                IdentifiableId::QuicksilverPlort,
                RegionSetId::Valley,
                slime_pos + Vec3::Y * 0.5,
                Vec3::new(0.0, 4.0, 0.0),
            );
            self.pending_plort_collect_at
                .insert(pid, wt + f64::from(DEFAULT_PLORT_COLLECT_MINS) * 60.0);
            plorts.push(pid);
        }
        plorts
    }

    pub fn step(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        physics: &mut PhysicsWorld,
        ammo: &mut AmmoController,
        pedia: &mut PediaDirector,
        rng: &mut Randoms,
    ) -> ValleyStepEvents {
        let mut ev = ValleyStepEvents::default();
        let wt = model.world.world_time;

        // 1. Advance QuicksilverEnergyGenerator state machine
        let gen_ids: Vec<String> = self.generators.keys().cloned().collect();
        for gid in gen_ids {
            let Some(cfg) = self.generators.get(&gid).cloned() else {
                continue;
            };
            let Some(gen_model) = model.quicksilver_generators.get_mut(&gid) else {
                continue;
            };
            match gen_model.state {
                QuicksilverGeneratorState::Countdown => {
                    if let Some(t) = gen_model.timer {
                        if wt >= t {
                            gen_model.state = QuicksilverGeneratorState::Active;
                            gen_model.timer = Some(wt + f64::from(cfg.active_minutes) * 60.0);
                            ev.activated_generators.push(gid.clone());
                        }
                    }
                }
                QuicksilverGeneratorState::Active => {
                    if let Some(t) = gen_model.timer {
                        if wt >= t {
                            gen_model.state = QuicksilverGeneratorState::Cooldown;
                            gen_model.timer = Some(wt + f64::from(cfg.cooldown_minutes) * 60.0);
                            ev.ended_generators.push(gid.clone());
                            // Despawn spawned quicksilver slimes when generator shuts off
                            if let Some(slimes) = self.generator_slimes.remove(&gid) {
                                for sid in slimes {
                                    model.destroy_actor(sid);
                                    physics.remove_body(sid);
                                }
                            }
                            continue;
                        }
                    }
                    // Maintain active QuicksilverSlime population
                    let list = self.generator_slimes.entry(gid.clone()).or_default();
                    list.retain(|sid| model.actors.contains_key(sid));
                    while list.len() < cfg.target_slime_count && !cfg.spawn_positions.is_empty() {
                        let idx = rng.get_in_range_i32(0, cfg.spawn_positions.len() as i32) as usize;
                        let pos = cfg.spawn_positions[idx.min(cfg.spawn_positions.len() - 1)];
                        let sid = spawn_physical_actor(
                            model,
                            physics,
                            IdentifiableId::QuicksilverSlime,
                            RegionSetId::Valley,
                            pos,
                            Vec3::ZERO,
                        );
                        list.push(sid);
                        ev.spawned_slimes.push(sid);
                    }
                }
                QuicksilverGeneratorState::Cooldown => {
                    if let Some(t) = gen_model.timer {
                        if wt >= t {
                            gen_model.state = QuicksilverGeneratorState::Inactive;
                            gen_model.timer = None;
                        }
                    }
                }
                QuicksilverGeneratorState::Inactive => {}
            }
        }

        // 2. Step active QuicksilverElectricField zones
        self.electric_fields.retain(|f| wt < f.expire_time);
        let fields = self.electric_fields.clone();
        for f in fields {
            let spawned = self.shock_slimes_in_radius(model, physics, f.pos, f.radius);
            ev.produced_plorts.extend(spawned);
        }

        // 3. Step QuicksilverPlortCollector timers (auto-adds plorts into NimbleValley slot 0)
        let ready_plorts: Vec<i64> = self
            .pending_plort_collect_at
            .iter()
            .filter(|(_, &collect_at)| wt >= collect_at)
            .map(|(&pid, _)| pid)
            .collect();
        for pid in ready_plorts {
            self.pending_plort_collect_at.remove(&pid);
            if !model.actors.contains_key(&pid) {
                continue;
            }
            let prev_mode = model.player.ammo_mode;
            model.player.ammo_mode = AmmoMode::NimbleValley;
            let added = ammo.maybe_add_to_slot(
                &mut model.player,
                IdentifiableId::QuicksilverPlort,
                None,
                1,
                wt,
            );
            model.player.ammo_mode = prev_mode;
            if added {
                model.destroy_actor(pid);
                physics.remove_body(pid);
                ev.collected_plorts += 1;
                if pedia.maybe_show_popup(model, mode, PediaId::QuicksilverSlime) {
                    ev.pedia_unlocked = Some(PediaId::QuicksilverSlime);
                }
            }
        }

        ev
    }
}
