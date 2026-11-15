//! Port of `FirestormActivator.cs`, `FirestormController.cs`, `Oasis.cs`,
//! and `OasisWaterTrigger.cs` (Glass Desert weather & Oasis revival).

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_gameplay::spawn_physical_actor;
use sr_ids::{IdentifiableId, ProgressType, RegionSetId};
use sr_math::{Randoms, Vec3};
use sr_model::{FirestormMode, GameModel, OasisModel};
use sr_physics::PhysicsWorld;

pub const FIRESTORM_PREP_SECS: f64 = 0.5 * 3600.0;
pub const FIRESTORM_MIN_WAIT_HOURS: f32 = 8.0;
pub const FIRESTORM_MAX_WAIT_HOURS: f32 = 15.0;
pub const FIRESTORM_MIN_DURATION_HOURS: f32 = 1.5;
pub const FIRESTORM_MAX_DURATION_HOURS: f32 = 2.0;
pub const FIRECOLUMN_INTERVAL_SECS: f64 = 8.0 * 60.0; // 8 game minutes
pub const FIRECOLUMN_LIFETIME_SECS: f64 = 20.0 * 60.0; // 20 game minutes
pub const FIRECOLUMN_RADIUS: f32 = 3.5;
pub const FIRECOLUMN_PLAYER_DAMAGE: f32 = 15.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OasisConfig {
    pub oasis_id: String,
    pub pos: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveFireColumn {
    pub pos: Vec3,
    pub expires_at: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesertStepEvents {
    pub firestorm_mode: FirestormMode,
    pub spawned_slimes: Vec<(i64, IdentifiableId)>,
    pub player_fire_damage: f32,
}

/// Manages Glass Desert firestorms, fire columns, and Oases.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlassDesertSystem {
    pub oases: Vec<OasisConfig>,
    pub column_sites: Vec<Vec3>,
    pub active_columns: Vec<ActiveFireColumn>,
    pub desert_initialized: bool,
}

impl GlassDesertSystem {
    pub fn register_oasis(&mut self, model: &mut GameModel, oasis: OasisConfig, is_live: bool) {
        model.oases.entry(oasis.oasis_id.clone()).or_insert(OasisModel {
            id: oasis.oasis_id.clone(),
            is_live,
        });
        if !self.oases.iter().any(|o| o.oasis_id == oasis.oasis_id) {
            self.oases.push(oasis);
        }
    }

    pub fn is_pos_in_live_oasis(&self, model: &GameModel, pos: Vec3) -> bool {
        self.oases.iter().any(|o| {
            model.oases.get(&o.oasis_id).map_or(false, |m| m.is_live)
                && pos.distance_squared(o.pos) <= o.radius * o.radius
        })
    }

    /// Port of `OasisWaterTrigger`: `MagicWaterLiquid` revives a dormant Oasis
    /// and suppresses any active fire columns inside it.
    pub fn revive_oasis(&mut self, model: &mut GameModel, oasis_id: &str, liquid: IdentifiableId) -> bool {
        if liquid != IdentifiableId::MagicWaterLiquid {
            return false;
        }
        let Some(om) = model.oases.get_mut(oasis_id) else {
            return false;
        };
        if om.is_live {
            return false;
        }
        om.is_live = true;
        if let Some(cfg) = self.oases.iter().find(|o| o.oasis_id == oasis_id) {
            let r2 = cfg.radius * cfg.radius;
            self.active_columns
                .retain(|c| c.pos.distance_squared(cfg.pos) > r2);
        }
        true
    }

    /// Steps `FirestormActivator` state machine (`Idle -> Preparing -> Active -> Idle`)
    /// and spawns fire columns / Fire & Mosaic slimes / player fire damage when in Desert.
    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        mode: &GameModeSettings,
        player_in_desert: bool,
        dt: f32,
        rng: &mut Randoms,
    ) -> DesertStepEvents {
        let mut ev = DesertStepEvents::default();
        let wt = model.world.world_time;

        if model.progress.get_progress(ProgressType::UnlockDesert) <= 0 {
            ev.firestorm_mode = model.world.curr_firestorm_mode;
            return ev;
        }

        if !self.desert_initialized {
            self.desert_initialized = true;
            if model.world.next_firestorm_time <= wt
                || model.world.next_firestorm_time.is_infinite()
            {
                let wait = rng.get_in_range_f32(FIRESTORM_MIN_WAIT_HOURS, FIRESTORM_MAX_WAIT_HOURS)
                    * 0.5;
                model.world.next_firestorm_time = wt + (wait as f64) * 3600.0;
                model.world.end_firestorm_time = f64::INFINITY;
            }
        }

        // 1. End active storm
        if model.world.curr_firestorm_mode != FirestormMode::Idle
            && wt >= model.world.end_firestorm_time
        {
            model.world.curr_firestorm_mode = FirestormMode::Idle;
            model.world.next_firecolumn_time = f64::INFINITY;
            self.active_columns.clear();
        }

        // 2. Start preparing next storm
        if model.world.curr_firestorm_mode == FirestormMode::Idle
            && wt >= model.world.next_firestorm_time
        {
            model.world.curr_firestorm_mode = FirestormMode::Preparing;
            model.world.next_firecolumn_time = wt + FIRESTORM_PREP_SECS;
            let dur_hours =
                rng.get_in_range_f32(FIRESTORM_MIN_DURATION_HOURS, FIRESTORM_MAX_DURATION_HOURS);
            model.world.end_firestorm_time = wt + (dur_hours as f64) * 3600.0;
            model.world.end_firecolumns_time = model.world.end_firestorm_time - 1800.0;
            let next_wait =
                rng.get_in_range_f32(FIRESTORM_MIN_WAIT_HOURS, FIRESTORM_MAX_WAIT_HOURS);
            model.world.next_firestorm_time = wt + (next_wait as f64) * 3600.0;
        }

        // 3. Trigger fire columns during Active window
        if wt >= model.world.next_firecolumn_time && wt < model.world.end_firecolumns_time {
            model.world.curr_firestorm_mode = FirestormMode::Active;
            model.world.next_firecolumn_time = wt + FIRECOLUMN_INTERVAL_SECS;

            let candidates: Vec<Vec3> = if self.column_sites.is_empty() {
                vec![model.player.position + Vec3::new(6.0, 0.0, 0.0)]
            } else {
                self.column_sites
                    .iter()
                    .copied()
                    .filter(|p| !self.is_pos_in_live_oasis(model, *p))
                    .collect()
            };

            if !candidates.is_empty() {
                let idx = rng.get_in_range_i32(0, candidates.len() as i32) as usize;
                let cpos = candidates[idx];
                self.active_columns.push(ActiveFireColumn {
                    pos: cpos,
                    expires_at: wt + FIRECOLUMN_LIFETIME_SECS,
                });

                // Spawn Fire or Mosaic slime from firestorm column
                let slime_id = if rng.get_probability(0.65) {
                    IdentifiableId::FireSlime
                } else if !mode.prevent_hostiles || rng.get_probability(0.5) {
                    IdentifiableId::MosaicSlime
                } else {
                    IdentifiableId::FireSlime
                };
                let sid = spawn_physical_actor(
                    model,
                    physics,
                    slime_id,
                    RegionSetId::Desert,
                    cpos + Vec3::Y * 1.5,
                    Vec3::new(0.0, 2.0, 0.0),
                );
                ev.spawned_slimes.push((sid, slime_id));
            }
        }

        self.active_columns.retain(|c| wt < c.expires_at);

        // 4. Player fire damage if inside active column & not inside a live Oasis
        if player_in_desert && !self.is_pos_in_live_oasis(model, model.player.position) {
            let ppos = model.player.position;
            let hit = self
                .active_columns
                .iter()
                .any(|c| c.pos.distance_squared(ppos) <= FIRECOLUMN_RADIUS * FIRECOLUMN_RADIUS);
            if hit {
                ev.player_fire_damage = FIRECOLUMN_PLAYER_DAMAGE * dt;
            }
        }

        ev.firestorm_mode = model.world.curr_firestorm_mode;
        ev
    }
}
