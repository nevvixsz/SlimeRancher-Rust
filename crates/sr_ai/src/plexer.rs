//! Port of `SlimeSubbehaviourPlexer.cs` and core `SlimeSubbehaviour` modules:
//! `SlimeRandomMove`, `GotoConsumable`, `SlimeFlee`, `AttackPlayer`, and `PlayWithToys`.

use serde::{Deserialize, Serialize};
use sr_data::DataRegistry;
use sr_ids::{IdentifiableId, SlimeEmotion};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, SlimeModel};
use sr_physics::{PhysicsWorld, RigidBody};

use crate::emotions::{DriveCalculator, ANGRY_CUTOFF};

pub const DEFAULT_ACTIVATION_DELAY: f32 = 3.0;
pub const GROUNDED_CHECK_FREQ: f32 = 0.25;
pub const RETHINK_TIME: f32 = 1.0;
pub const MIN_RELEVANCY: f32 = 0.0001;

pub const RANDOM_MOVE_JUMP_INTERVAL: f32 = 1.0;
pub const RANDOM_MOVE_MAX_JUMP: f32 = 6.0;
pub const GOTO_CONSUMABLE_MAX_JUMP: f32 = 12.0;
pub const GOTO_CONSUMABLE_ATTEMPT_TIME: f32 = 10.0;
pub const GOTO_CONSUMABLE_GIVE_UP_TIME: f32 = 10.0;
pub const GOTO_CONSUMABLE_AGITATION_PER_GIVE_UP: f32 = 0.1;
pub const PLAY_WITH_TOYS_POUNCE_DIST: f32 = 8.0;
pub const PLAY_WITH_TOYS_RESET_TIME: f32 = 5.0;
pub const ATTACK_PLAYER_DAMAGE: f32 = 20.0;
pub const ATTACK_PLAYER_COOLDOWN: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActiveSubbehaviour {
    #[default]
    None,
    RandomMove,
    GotoConsumable,
    Flee,
    AttackPlayer,
    PlayWithToys,
}

/// Snapshot of nearby world stimuli used by a slime's plexer tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlimeStimuli {
    pub nearest_consumable: Option<(i64, Vec3, IdentifiableId)>,
    pub nearest_toy: Option<(i64, Vec3, IdentifiableId)>,
    pub nearest_threat_pos: Option<Vec3>,
    pub player_pos: Vec3,
    pub player_alive: bool,
    pub is_grounded: bool,
    pub is_captive_or_held: bool,
}

/// Output actions emitted by the plexer for the physics / gameplay layer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlexerOutput {
    pub active: ActiveSubbehaviour,
    pub impulse: Vec3,
    pub target_facing: Option<Quat>,
    pub player_damage: f32,
    pub gave_up_consumable: bool,
}

/// Per-slime subbehaviour state matching `SlimeSubbehaviourPlexer` + core subbehaviours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimePlexerState {
    pub active: ActiveSubbehaviour,
    pub behavior_blockers: i32,
    pub next_activate_time: f64,
    pub next_grounded_check_time: f64,
    pub next_rethink_time: f64,
    pub cached_grounded: bool,
    pub next_jump_time: f64,
    pub attempt_consumable_id: Option<i64>,
    pub attempt_until_time: f64,
    pub give_up_until_time: f64,
    pub next_attack_time: f64,
    pub next_toy_play_time: f64,
}

impl Default for SlimePlexerState {
    fn default() -> Self {
        Self {
            active: ActiveSubbehaviour::None,
            behavior_blockers: 0,
            next_activate_time: DEFAULT_ACTIVATION_DELAY as f64,
            next_grounded_check_time: 0.0,
            next_rethink_time: 0.0,
            cached_grounded: false,
            next_jump_time: 0.0,
            attempt_consumable_id: None,
            attempt_until_time: 0.0,
            give_up_until_time: 0.0,
            next_attack_time: 0.0,
            next_toy_play_time: 0.0,
        }
    }
}

impl SlimePlexerState {
    pub fn new(now_elapsed: f64) -> Self {
        Self {
            next_activate_time: now_elapsed + DEFAULT_ACTIVATION_DELAY as f64,
            ..Default::default()
        }
    }

    pub fn is_blocked(&self, is_captive_or_held: bool) -> bool {
        self.behavior_blockers > 0 || is_captive_or_held
    }

    /// Evaluates relevancy across subbehaviours, selects the best active behaviour,
    /// and computes a movement/attack impulse for this frame.
    pub fn step(
        &mut self,
        now: f64,
        slime_pos: Vec3,
        slime_vel: Vec3,
        ident: IdentifiableId,
        slime: &mut SlimeModel,
        stimuli: &SlimeStimuli,
        rng: &mut Randoms,
    ) -> PlexerOutput {
        let mut out = PlexerOutput::default();

        if self.is_blocked(stimuli.is_captive_or_held) || now < self.next_activate_time {
            self.active = ActiveSubbehaviour::None;
            return out;
        }

        if now >= self.next_grounded_check_time {
            self.next_grounded_check_time = now + GROUNDED_CHECK_FREQ as f64;
            self.cached_grounded = stimuli.is_grounded;
        }

        if now >= self.next_rethink_time {
            self.next_rethink_time = now + RETHINK_TIME as f64;

            let mut best = ActiveSubbehaviour::None;
            let mut best_rel = MIN_RELEVANCY;

            // 1. RandomMove baseline
            let random_rel = 0.15f32;
            if random_rel > best_rel {
                best_rel = random_rel;
                best = ActiveSubbehaviour::RandomMove;
            }

            // 2. PlayWithToys
            if let Some((_, toy_pos, toy_id)) = stimuli.nearest_toy {
                if now >= self.next_toy_play_time
                    && slime_pos.distance_squared(toy_pos) <= PLAY_WITH_TOYS_POUNCE_DIST * PLAY_WITH_TOYS_POUNCE_DIST
                {
                    let is_fav = toy_favorite_matches(ident, toy_id);
                    let rel = if is_fav { 0.45 } else { 0.25 };
                    if rel > best_rel {
                        best_rel = rel;
                        best = ActiveSubbehaviour::PlayWithToys;
                    }
                }
            }

            // 3. GotoConsumable (hunger-driven)
            if let Some((cid, cpos, _)) = stimuli.nearest_consumable {
                if now >= self.give_up_until_time {
                    let dist = slime_pos.distance(cpos).max(1.0);
                    let hunger_drive = DriveCalculator::new(SlimeEmotion::Hunger, 0.0, 0.0).drive(slime);
                    let rel = hunger_drive * (18.0 / (dist + 4.0)).min(1.0);
                    if rel > best_rel {
                        best_rel = rel;
                        best = ActiveSubbehaviour::GotoConsumable;
                        if self.attempt_consumable_id != Some(cid) {
                            self.attempt_consumable_id = Some(cid);
                            self.attempt_until_time = now + GOTO_CONSUMABLE_ATTEMPT_TIME as f64;
                        }
                    }
                }
            }

            // 4. AttackPlayer (Tarr, Feral, or very angry hunter/boom/rock)
            if stimuli.player_alive {
                let dist = slime_pos.distance(stimuli.player_pos);
                let is_hostile = ident.is_tarr()
                    || slime.is_feral
                    || (slime.agitation.curr_val >= ANGRY_CUTOFF
                        && matches!(
                            ident,
                            IdentifiableId::HunterSlime | IdentifiableId::BoomSlime | IdentifiableId::RockSlime
                        ));
                if is_hostile && dist < 24.0 {
                    let rel = if ident.is_tarr() {
                        0.92
                    } else if slime.is_feral {
                        0.85
                    } else {
                        0.68
                    };
                    if rel > best_rel {
                        best_rel = rel;
                        best = ActiveSubbehaviour::AttackPlayer;
                    }
                }
            }

            // 5. SlimeFlee (Fear-driven or Gold/Lucky)
            if stimuli.nearest_threat_pos.is_some() {
                let fear_drive = DriveCalculator::new(SlimeEmotion::Fear, 0.0, 0.0).drive(slime);
                let rel = if matches!(ident, IdentifiableId::GoldSlime | IdentifiableId::LuckySlime) {
                    0.98
                } else {
                    fear_drive
                };
                if rel > best_rel {
                    best = ActiveSubbehaviour::Flee;
                }
            }

            // Give-up timer check on GotoConsumable
            if self.active == ActiveSubbehaviour::GotoConsumable
                && now >= self.attempt_until_time
                && self.attempt_consumable_id.is_some()
            {
                self.attempt_consumable_id = None;
                self.give_up_until_time = now + GOTO_CONSUMABLE_GIVE_UP_TIME as f64;
                slime.agitation.curr_val =
                    (slime.agitation.curr_val + GOTO_CONSUMABLE_AGITATION_PER_GIVE_UP).clamp(0.0, 1.0);
                out.gave_up_consumable = true;
            }

            self.active = best;
        }

        out.active = self.active;

        // Execute active subbehaviour action
        match self.active {
            ActiveSubbehaviour::None => {}
            ActiveSubbehaviour::RandomMove => {
                if self.cached_grounded && now >= self.next_jump_time && slime_vel.length_squared() <= 25.0 {
                    self.next_jump_time = now + RANDOM_MOVE_JUMP_INTERVAL as f64;
                    let yaw = rng.get_in_range_f32(0.0, std::f32::consts::TAU);
                    let dir = Vec3::new(yaw.cos(), 0.0, yaw.sin());
                    let hop = rng.get_in_range_f32(3.5, RANDOM_MOVE_MAX_JUMP);
                    out.impulse = dir * 4.5 + Vec3::Y * hop;
                    out.target_facing = Some(Quat::from_rotation_y(-yaw + std::f32::consts::FRAC_PI_2));
                }
            }
            ActiveSubbehaviour::GotoConsumable => {
                if let Some((_, cpos, _)) = stimuli.nearest_consumable {
                    let delta = Vec3::new(cpos.x - slime_pos.x, 0.0, cpos.z - slime_pos.z);
                    let len = delta.length();
                    if len > 0.05 && self.cached_grounded && now >= self.next_jump_time {
                        self.next_jump_time = now + 0.65;
                        let dir = delta / len;
                        let hop = (len * 0.85).clamp(4.0, GOTO_CONSUMABLE_MAX_JUMP * 0.75);
                        out.impulse = dir * 6.5 + Vec3::Y * hop;
                        out.target_facing = Some(facing_from_dir(dir));
                    }
                }
            }
            ActiveSubbehaviour::Flee => {
                if let Some(threat) = stimuli.nearest_threat_pos {
                    let away = Vec3::new(slime_pos.x - threat.x, 0.0, slime_pos.z - threat.z);
                    let len = away.length();
                    if len > 0.05 && self.cached_grounded && now >= self.next_jump_time {
                        self.next_jump_time = now + 0.55;
                        let dir = away / len;
                        out.impulse = dir * 8.5 + Vec3::Y * 5.8;
                        out.target_facing = Some(facing_from_dir(dir));
                    }
                }
            }
            ActiveSubbehaviour::AttackPlayer => {
                let to_player = Vec3::new(
                    stimuli.player_pos.x - slime_pos.x,
                    0.0,
                    stimuli.player_pos.z - slime_pos.z,
                );
                let dist = slime_pos.distance(stimuli.player_pos);
                let flat_len = to_player.length();
                if flat_len > 0.05 {
                    let dir = to_player / flat_len;
                    out.target_facing = Some(facing_from_dir(dir));
                    if self.cached_grounded && now >= self.next_jump_time {
                        self.next_jump_time = now + 0.75;
                        out.impulse = dir * 7.5 + Vec3::Y * 5.2;
                    }
                }
                if dist <= 1.8 && now >= self.next_attack_time {
                    self.next_attack_time = now + ATTACK_PLAYER_COOLDOWN as f64;
                    out.player_damage = ATTACK_PLAYER_DAMAGE;
                }
            }
            ActiveSubbehaviour::PlayWithToys => {
                if let Some((_, toy_pos, _)) = stimuli.nearest_toy {
                    let to_toy = Vec3::new(toy_pos.x - slime_pos.x, 0.0, toy_pos.z - slime_pos.z);
                    let len = to_toy.length();
                    if len > 0.1 && self.cached_grounded && now >= self.next_jump_time {
                        self.next_jump_time = now + 0.85;
                        self.next_toy_play_time = now + PLAY_WITH_TOYS_RESET_TIME as f64;
                        let dir = to_toy / len;
                        out.impulse = dir * 5.0 + Vec3::Y * 4.5;
                        out.target_facing = Some(facing_from_dir(dir));
                    }
                }
            }
        }

        out
    }
}

fn facing_from_dir(dir: Vec3) -> Quat {
    let yaw = dir.x.atan2(dir.z);
    Quat::from_rotation_y(yaw)
}

fn toy_favorite_matches(slime: IdentifiableId, toy: IdentifiableId) -> bool {
    matches!(
        (slime, toy),
        (IdentifiableId::PinkSlime, IdentifiableId::BeachBallToy)
            | (IdentifiableId::RockSlime, IdentifiableId::BigRockToy)
            | (IdentifiableId::TabbySlime, IdentifiableId::YarnBallToy)
            | (IdentifiableId::PhosphorSlime, IdentifiableId::NightLightToy)
            | (IdentifiableId::RadSlime, IdentifiableId::PowerCellToy)
            | (IdentifiableId::BoomSlime, IdentifiableId::BombBallToy)
            | (IdentifiableId::HoneySlime, IdentifiableId::BumbleBallToy)
            | (IdentifiableId::PuddleSlime, IdentifiableId::RubberDuckyToy)
            | (IdentifiableId::CrystalSlime, IdentifiableId::CrystalBallToy)
            | (IdentifiableId::HunterSlime, IdentifiableId::StuffedChickenToy)
            | (IdentifiableId::QuantumSlime, IdentifiableId::PuzzleCubeToy)
            | (IdentifiableId::TangleSlime, IdentifiableId::SolMateToy)
            | (IdentifiableId::DervishSlime, IdentifiableId::GyroTopToy)
            | (IdentifiableId::MosaicSlime, IdentifiableId::DiscoBallToy)
            | (IdentifiableId::FireSlime, IdentifiableId::CharcoalBrickToy)
    )
}

/// Scans `GameModel` + `PhysicsWorld` to build `SlimeStimuli` for a specific slime.
pub fn gather_stimuli(
    actor_id: i64,
    slime_ident: IdentifiableId,
    slime_pos: Vec3,
    body: &RigidBody,
    model: &GameModel,
    _physics: &PhysicsWorld,
    registry: &DataRegistry,
    held_actor_id: Option<i64>,
    player_pos: Vec3,
    player_alive: bool,
) -> SlimeStimuli {
    let diet = registry.slimes.get(slime_ident);

    let mut nearest_consumable: Option<(i64, Vec3, IdentifiableId, f32)> = None;
    let mut nearest_toy: Option<(i64, Vec3, IdentifiableId, f32)> = None;
    let mut nearest_threat_pos: Option<(Vec3, f32)> = None;

    let search_radius_sq = 26.0 * 26.0;
    for (&other_id, other) in &model.actors {
        if other_id == actor_id || held_actor_id == Some(other_id) {
            continue;
        }
        let d2 = slime_pos.distance_squared(other.pos);
        if d2 > search_radius_sq {
            continue;
        }

        if other.ident.is_toy() {
            if nearest_toy.map_or(true, |(_, _, _, best)| d2 < best) {
                nearest_toy = Some((other_id, other.pos, other.ident, d2));
            }
        }

        // Tarr threatens non-Tarr slimes
        if other.ident.is_tarr() && !slime_ident.is_tarr() {
            if nearest_threat_pos.map_or(true, |(_, best)| d2 < best) {
                nearest_threat_pos = Some((other.pos, d2));
            }
        }

        // Check diet edibility (skip invulnerable plorts)
        let invuln = matches!(&other.kind, ActorKind::Plort(pm) if pm.invulnerable);
        if !invuln {
            let edible = diet.map_or(false, |def| {
                def.diet.eat_map.iter().any(|e| e.eats == other.ident)
            });
            if edible && nearest_consumable.map_or(true, |(_, _, _, best)| d2 < best) {
                nearest_consumable = Some((other_id, other.pos, other.ident, d2));
            }
        }
    }

    // Player threatens Gold/Lucky slimes or any slime in close proximity if timid
    if player_alive && matches!(slime_ident, IdentifiableId::GoldSlime | IdentifiableId::LuckySlime) {
        let d2 = slime_pos.distance_squared(player_pos);
        if d2 < search_radius_sq && nearest_threat_pos.map_or(true, |(_, best)| d2 < best) {
            nearest_threat_pos = Some((player_pos, d2));
        }
    }

    let is_grounded = body.grounded;

    SlimeStimuli {
        nearest_consumable: nearest_consumable.map(|(id, p, i, _)| (id, p, i)),
        nearest_toy: nearest_toy.map(|(id, p, i, _)| (id, p, i)),
        nearest_threat_pos: nearest_threat_pos.map(|(p, _)| p),
        player_pos,
        player_alive,
        is_grounded,
        is_captive_or_held: held_actor_id == Some(actor_id),
    }
}
