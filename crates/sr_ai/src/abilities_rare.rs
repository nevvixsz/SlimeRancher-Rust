//! Rare & special slime abilities:
//! - `Hunter`: `SlimeStealth`
//! - `Quantum`: `QuantumSlimeSuperposition`
//! - `Dervish`: `DervishSlimeSpin` (whirlwinds)
//! - `Mosaic`: `MosaicAttractor` / solar glint pops
//! - `Tangle`: `TentacleGrapple`
//! - `Puddle`: `PuddleSlimeScoot` + shyness
//! - `Fire`: `FireSlimeIgnition`
//! - `Gold` & `Lucky`: plort/newbuck drops on collision/feed + timed flee vanish

use serde::{Deserialize, Serialize};
use sr_data::DataRegistry;
use sr_ids::{IdentifiableId, RegionSetId, UpgradeId};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, SlimeModel};
use sr_physics::{ImpactEvent, PhysicsWorld, RigidBody};

use crate::abilities_classic::slime_has_trait;

pub const HUNTER_STEALTH_INIT_TIME: f32 = 5.0;
pub const HUNTER_OPACITY_RATE: f32 = 2.0;

pub const QUANTUM_MIN_DELAY: f32 = 5.0;
pub const QUANTUM_MAX_DELAY: f32 = 30.0;

pub const DERVISH_WHIRLWIND_CUTOFF: f32 = 0.95;
pub const DERVISH_MAX_WHIRLWINDS: usize = 6;
pub const DERVISH_WHIRLWIND_RADIUS: f32 = 4.5;

pub const TANGLE_GRAPPLE_COOLDOWN: f32 = 2.0;
pub const TANGLE_GRAPPLE_RANGE: f32 = 12.0;

pub const PUDDLE_SHY_RADIUS: f32 = 8.0;
pub const PUDDLE_SHY_THRESHOLD: usize = 3;

pub const FIRE_EXTINGUISH_SECS: f64 = 0.5 * 3600.0;
pub const GOLD_HIT_MIN_SPEED: f32 = 0.02;
pub const GOLD_LUCKY_DESPAWN_SECS: f64 = 12.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhirlwindHazard {
    pub pos: Vec3,
    pub vel: Vec3,
    pub expires_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RareAbilityState {
    // Hunter
    pub opacity: f32,
    pub stealth_timer: f32,
    // Quantum
    pub phantom_pos: Option<Vec3>,
    pub quantum_next_time: f64,
    // Dervish
    pub whirlwind_cooldown: f32,
    // Mosaic
    pub mosaic_next_glint: f64,
    // Tangle
    pub tangle_next_grapple: f64,
    // Puddle
    pub puddle_shy: bool,
    // Fire
    pub fire_reignite_at: f64,
    // Gold / Lucky
    pub flee_vanish_at: Option<f64>,
    pub gold_plorts_produced: u32,
}

impl Default for RareAbilityState {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            stealth_timer: HUNTER_STEALTH_INIT_TIME,
            phantom_pos: None,
            quantum_next_time: 12.0,
            whirlwind_cooldown: 0.0,
            mosaic_next_glint: 10.0,
            tangle_next_grapple: 2.0,
            puddle_shy: false,
            fire_reignite_at: 0.0,
            flee_vanish_at: None,
            gold_plorts_produced: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RareAbilityEvents {
    pub quantum_teleports: Vec<(i64, Vec3)>,
    pub tangle_grabbings: Vec<(i64, i64)>,
    pub gold_plorts_dropped: Vec<(i64, u32)>,
    pub lucky_newbucks_awarded: i32,
    pub despawned_slimes: Vec<i64>,
    pub player_fire_damage: f32,
}

impl RareAbilityState {
    pub fn extinguish_fire(&mut self, world_time: f64) {
        self.fire_reignite_at = world_time + FIRE_EXTINGUISH_SECS;
    }

    pub fn is_fire_lit(&self, world_time: f64) -> bool {
        world_time >= self.fire_reignite_at
    }

    pub fn clear_quantum_phantom(&mut self) {
        self.phantom_pos = None;
    }

    pub fn step(
        &mut self,
        actor_id: i64,
        ident: IdentifiableId,
        slime_pos: Vec3,
        slime: &SlimeModel,
        player_pos: Vec3,
        world_time: f64,
        elapsed: f64,
        dt: f32,
        registry: &DataRegistry,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        whirlwinds: &mut Vec<WhirlwindHazard>,
        impacts: &[ImpactEvent],
        rng: &mut Randoms,
    ) -> RareAbilityEvents {
        let mut ev = RareAbilityEvents::default();

        // 1. Hunter Stealth
        if slime_has_trait(registry, ident, IdentifiableId::HunterSlime) {
            let target_opacity = if slime.agitation.curr_val >= 0.7 || slime.is_feral {
                self.stealth_timer = HUNTER_STEALTH_INIT_TIME;
                1.0
            } else {
                self.stealth_timer = (self.stealth_timer - dt).max(0.0);
                if self.stealth_timer <= 0.0 {
                    0.0
                } else {
                    1.0
                }
            };
            if (self.opacity - target_opacity).abs() > 0.001 {
                if self.opacity < target_opacity {
                    self.opacity = (self.opacity + HUNTER_OPACITY_RATE * dt).min(target_opacity);
                } else {
                    self.opacity = (self.opacity - HUNTER_OPACITY_RATE * dt).max(target_opacity);
                }
            }
        }

        // 2. Quantum Superposition
        if slime_has_trait(registry, ident, IdentifiableId::QuantumSlime) {
            if elapsed >= self.quantum_next_time {
                self.quantum_next_time =
                    elapsed + rng.get_in_range_f32(QUANTUM_MIN_DELAY, QUANTUM_MAX_DELAY) as f64;
                if let Some(p_pos) = self.phantom_pos.take() {
                    if slime.agitation.curr_val >= 0.5 {
                        if let Some(a) = model.actors.get_mut(&actor_id) {
                            a.pos = p_pos;
                        }
                        if let Some(b) = physics.bodies.get_mut(&actor_id) {
                            b.pos = p_pos;
                            b.vel = Vec3::ZERO;
                        }
                        ev.quantum_teleports.push((actor_id, p_pos));
                    }
                } else if slime.agitation.curr_val >= 0.4 {
                    let ang = rng.get_in_range_f32(0.0, std::f32::consts::TAU);
                    let dist = rng.get_in_range_f32(3.5, 8.5);
                    self.phantom_pos = Some(Vec3::new(
                        slime_pos.x + ang.cos() * dist,
                        slime_pos.y + 0.5,
                        slime_pos.z + ang.sin() * dist,
                    ));
                }
            }
        }

        // 3. Dervish Spin / Whirlwind
        if slime_has_trait(registry, ident, IdentifiableId::DervishSlime) {
            self.whirlwind_cooldown = (self.whirlwind_cooldown - dt).max(0.0);
            if slime.agitation.curr_val >= DERVISH_WHIRLWIND_CUTOFF
                && self.whirlwind_cooldown <= 0.0
                && whirlwinds.len() < DERVISH_MAX_WHIRLWINDS
            {
                self.whirlwind_cooldown = 4.0;
                let ang = rng.get_in_range_f32(0.0, std::f32::consts::TAU);
                whirlwinds.push(WhirlwindHazard {
                    pos: slime_pos,
                    vel: Vec3::new(ang.cos(), 0.0, ang.sin()) * 3.5,
                    expires_at: elapsed + 5.0,
                });
            }
        }

        // 4. Mosaic Solar Glint
        if slime_has_trait(registry, ident, IdentifiableId::MosaicSlime)
            && elapsed >= self.mosaic_next_glint
        {
            self.mosaic_next_glint = elapsed + rng.get_in_range_f32(8.0, 16.0) as f64;
            if slime_pos.distance(player_pos) <= 4.0 && slime.agitation.curr_val >= 0.6 {
                ev.player_fire_damage += 10.0;
            }
        }

        // 5. Tangle Tentacle Grapple
        if slime_has_trait(registry, ident, IdentifiableId::TangleSlime)
            && elapsed >= self.tangle_next_grapple
        {
            self.tangle_next_grapple = elapsed + TANGLE_GRAPPLE_COOLDOWN as f64;
            if let Some(def) = registry.slimes.get(ident) {
                let r2 = TANGLE_GRAPPLE_RANGE * TANGLE_GRAPPLE_RANGE;
                let mut target: Option<(i64, Vec3, f32)> = None;
                for (&other_id, other) in &model.actors {
                    if other_id == actor_id {
                        continue;
                    }
                    let d2 = slime_pos.distance_squared(other.pos);
                    if d2 > 2.25 && d2 <= r2 {
                        let edible = def.diet.eat_map.iter().any(|e| e.eats == other.ident);
                        if edible && target.map_or(true, |(_, _, bd2)| d2 < bd2) {
                            target = Some((other_id, other.pos, d2));
                        }
                    }
                }
                if let Some((tid, tpos, _)) = target {
                    let pull = (slime_pos - tpos).normalize_or_zero() * 9.0 + Vec3::Y * 3.0;
                    physics.add_force(tid, pull);
                    ev.tangle_grabbings.push((actor_id, tid));
                }
            }
        }

        // 6. Puddle Shyness
        if slime_has_trait(registry, ident, IdentifiableId::PuddleSlime) {
            let r2 = PUDDLE_SHY_RADIUS * PUDDLE_SHY_RADIUS;
            let nearby_slimes = model
                .actors
                .iter()
                .filter(|(id, a)| {
                    **id != actor_id && a.ident.is_slime() && slime_pos.distance_squared(a.pos) <= r2
                })
                .count();
            self.puddle_shy = nearby_slimes > PUDDLE_SHY_THRESHOLD;
        }

        // 7. Fire Slime Ignition
        if slime_has_trait(registry, ident, IdentifiableId::FireSlime)
            && self.is_fire_lit(world_time)
            && slime_pos.distance(player_pos) <= 1.25
        {
            ev.player_fire_damage += 15.0 * dt;
        }

        // 8. Gold Slime Plort Drops + Flee Despawn
        if ident == IdentifiableId::GoldSlime {
            let gold_hit = impacts.iter().any(|imp| {
                imp.actor_id == actor_id && imp.normal_speed > GOLD_HIT_MIN_SPEED
            });
            // Also check nearby fast moving non-gold food/plort items
            if gold_hit {
                let mult = if model.player.upgrades.contains(&UpgradeId::GoldenSureshot) {
                    3
                } else {
                    1
                };
                for _ in 0..mult {
                    let jitter = Vec3::new(
                        rng.get_in_range_f32(-0.5, 0.5),
                        0.7,
                        rng.get_in_range_f32(-0.5, 0.5),
                    );
                    let pid = model.create_actor(
                        IdentifiableId::GoldPlort,
                        RegionSetId::Unset,
                        slime_pos + jitter,
                        Quat::IDENTITY,
                    );
                    if let Some(pa) = model.actors.get_mut(&pid) {
                        if let ActorKind::Plort(ref mut pm) = pa.kind {
                            pm.invulnerable = true;
                        }
                    }
                    physics.insert_body(RigidBody {
                        actor_id: pid,
                        pos: slime_pos + jitter,
                        vel: jitter * 4.0 + Vec3::Y * 3.0,
                        radius: 0.25,
                        ..Default::default()
                    });
                }
                self.gold_plorts_produced += mult;
                ev.gold_plorts_dropped.push((actor_id, mult));
                if self.flee_vanish_at.is_none() {
                    self.flee_vanish_at = Some(elapsed + GOLD_LUCKY_DESPAWN_SECS);
                }
            }
        }

        // 9. Lucky Slime Flee Despawn + proximity trigger
        if matches!(ident, IdentifiableId::GoldSlime | IdentifiableId::LuckySlime) {
            if self.flee_vanish_at.is_none() && slime_pos.distance(player_pos) < 8.0 {
                self.flee_vanish_at = Some(elapsed + GOLD_LUCKY_DESPAWN_SECS);
            }
            if let Some(v_at) = self.flee_vanish_at {
                if elapsed >= v_at {
                    ev.despawned_slimes.push(actor_id);
                }
            }
        }

        ev
    }
}

/// Steps active Dervish whirlwinds, pushing nearby bodies & expiring timed whirlwinds.
pub fn step_whirlwinds(
    whirlwinds: &mut Vec<WhirlwindHazard>,
    physics: &mut PhysicsWorld,
    elapsed: f64,
    dt: f32,
) {
    whirlwinds.retain(|w| elapsed < w.expires_at);
    for w in whirlwinds.iter_mut() {
        w.pos += w.vel * dt;
        physics.soft_explosion_force(140.0, w.pos, DERVISH_WHIRLWIND_RADIUS, dt, |_| true);
    }
}
