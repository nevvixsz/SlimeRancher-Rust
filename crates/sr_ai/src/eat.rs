//! Port of `SlimeEat.cs`, `SlimeFeral.cs`, `TarrBite.cs`, and basic chicken lifecycle.

use serde::{Deserialize, Serialize};
use sr_data::{DataRegistry, GameModeSettings};
use sr_ids::{IdentifiableId, RegionSetId, SlimeEmotion};
use sr_math::{Quat, Randoms, Vec3};
use sr_model::{ActorKind, GameModel, PlortModel, SlimeModel};
use sr_physics::{PhysicsWorld, RigidBody};

use crate::emotions::SlimeEmotionRuntime;

pub const DIGEST_DELAY_SECS: f64 = 2.0;
pub const EAT_HUNGER_REDUCTION: f32 = 0.33333334;
pub const EAT_AGITATION_REDUCTION: f32 = 0.15;
pub const DYNAMIC_FERAL_AGITATION: f32 = 0.999;
pub const DEFAULT_FERAL_LIFETIME_HOURS: f32 = 12.0;
pub const SLIME_EAT_RADIUS: f32 = 1.35;
pub const TARR_BITE_RADIUS: f32 = 1.85;
pub const TARR_BITE_COOLDOWN: f64 = 2.5;
pub const TARR_SUNLIGHT_BURN_RATE: f32 = 0.25; // ~4s exposed to full sunlight

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EatStepEvents {
    pub eaten_pairs: Vec<(i64, IdentifiableId)>,
    pub spawned_plorts: Vec<(i64, IdentifiableId)>,
    pub transformed_slimes: Vec<(i64, IdentifiableId, IdentifiableId)>,
    pub became_feral: Vec<i64>,
    pub calmed_feral: Vec<i64>,
    pub tarr_converted: Vec<i64>,
    pub tarr_destroyed: Vec<i64>,
}

/// Per-slime eating / digesting / feral runtime state.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SlimeEatState {
    pub digest_ready_at: f64,
    pub pending_produce_id: Option<IdentifiableId>,
    pub pending_produce_count: u32,
    pub pending_becomes_id: Option<IdentifiableId>,
    pub feral_expires_at: Option<f64>,
    pub next_tarr_bite_time: f64,
    pub sunlight_damage: f32,
}

impl SlimeEatState {
    /// Port of `SlimeFeral` dynamic transitions.
    pub fn update_feral(
        &mut self,
        actor_id: i64,
        ident: IdentifiableId,
        slime: &mut SlimeModel,
        world_time: f64,
        in_wilds_or_ranch: bool,
        events: &mut EatStepEvents,
    ) {
        if ident.is_tarr() || matches!(ident, IdentifiableId::GoldSlime | IdentifiableId::LuckySlime) {
            return;
        }
        // Dynamic to feral when agitation >= 0.999 (primarily largos / normal wild slimes)
        if !slime.is_feral && slime.agitation.curr_val >= DYNAMIC_FERAL_AGITATION && ident.is_largo() {
            slime.is_feral = true;
            if !in_wilds_or_ranch {
                self.feral_expires_at =
                    Some(world_time + (DEFAULT_FERAL_LIFETIME_HOURS as f64) * 3600.0);
            }
            events.became_feral.push(actor_id);
        }
        if slime.is_feral {
            if let Some(exp) = self.feral_expires_at {
                if world_time >= exp && !in_wilds_or_ranch {
                    slime.is_feral = false;
                    slime.agitation.curr_val = 0.0;
                    self.feral_expires_at = None;
                    events.calmed_feral.push(actor_id);
                }
            }
        }
    }
}

/// Processes slime chomping/eating, pending digestion -> plorts/largo/tarr, and Tarr bites/sunlight burn.
pub fn step_slime_eat_and_tarr(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    registry: &DataRegistry,
    mode: &GameModeSettings,
    eat_states: &mut std::collections::BTreeMap<i64, SlimeEatState>,
    held_actor_id: Option<i64>,
    world_time: f64,
    elapsed_time: f64,
    dt: f32,
    hour_of_day: f32,
    rng: &mut Randoms,
) -> EatStepEvents {
    let mut events = EatStepEvents::default();

    let slime_ids: Vec<i64> = model
        .actors
        .iter()
        .filter(|(id, a)| a.ident.is_slime() && held_actor_id != Some(**id))
        .map(|(id, _)| *id)
        .collect();

    // 1. Complete pending digestions (2.0s after eating)
    for &slime_id in &slime_ids {
        let Some(state) = eat_states.get_mut(&slime_id) else {
            continue;
        };
        if state.pending_produce_id.is_some() || state.pending_becomes_id.is_some() {
            if elapsed_time >= state.digest_ready_at {
                let prod = state.pending_produce_id.take();
                let count = state.pending_produce_count;
                let becomes = state.pending_becomes_id.take();

                let (pos, region, old_ident, old_slime) = match model.actors.get(&slime_id) {
                    Some(a) => {
                        let sm = match &a.kind {
                            ActorKind::Slime(s) => s.clone(),
                            _ => SlimeModel::default(),
                        };
                        (a.pos, a.curr_region_set_id, a.ident, sm)
                    }
                    None => continue,
                };

                // Spawn plorts
                if let Some(plort_id) = prod {
                    for _ in 0..count {
                        if plort_id == IdentifiableId::None {
                            continue;
                        }
                        let jitter = Vec3::new(
                            rng.get_in_range_f32(-0.4, 0.4),
                            0.5,
                            rng.get_in_range_f32(-0.4, 0.4),
                        );
                        let p_pos = pos + jitter;
                        let pid = model.create_actor(plort_id, region, p_pos, Quat::IDENTITY);
                        if let Some(pa) = model.actors.get_mut(&pid) {
                            if let ActorKind::Plort(ref mut pm) = pa.kind {
                                pm.invulnerable = false;
                                pm.destroy_time = world_time + 24.0 * 3600.0;
                            }
                        }
                        physics.insert_body(RigidBody {
                            actor_id: pid,
                            pos: p_pos,
                            vel: jitter * 3.0 + Vec3::Y * 2.5,
                            radius: 0.25,
                            ..Default::default()
                        });
                        events.spawned_plorts.push((pid, plort_id));
                    }
                }

                // Transform into Largo or Tarr
                if let Some(new_id) = becomes {
                    if new_id != IdentifiableId::None
                        && !(new_id.is_tarr() && mode.prevent_hostiles)
                    {
                        transform_slime(
                            model,
                            physics,
                            slime_id,
                            old_ident,
                            new_id,
                            pos,
                            region,
                            old_slime,
                            &mut events,
                        );
                    }
                }
            }
        }
    }

    // 2. Scan for nearby edible items / plorts & Tarr bites
    let mut consumed_targets: Vec<i64> = Vec::new();
    let mut tarr_targets: Vec<i64> = Vec::new();
    let mut tarr_sunlight_deaths: Vec<i64> = Vec::new();

    for &slime_id in &slime_ids {
        let Some(slime_actor) = model.actors.get(&slime_id) else {
            continue;
        };
        let slime_ident = slime_actor.ident;
        let slime_pos = slime_actor.pos;
        let state = eat_states.entry(slime_id).or_default();

        // Tarr sunlight check (06:00 to 18:00 burns in open daylight; no cave system in flat sandbox yet)
        if slime_ident.is_tarr() {
            if hour_of_day >= 6.0 && hour_of_day <= 18.0 {
                state.sunlight_damage += TARR_SUNLIGHT_BURN_RATE * dt;
                if state.sunlight_damage >= 1.0 {
                    tarr_sunlight_deaths.push(slime_id);
                    continue;
                }
            } else {
                state.sunlight_damage = (state.sunlight_damage - 0.1 * dt).max(0.0);
            }
        }

        // Skip if already digesting
        if state.pending_produce_id.is_some() || state.pending_becomes_id.is_some() {
            continue;
        }

        let Some(def) = registry.slimes.get(slime_ident) else {
            continue;
        };

        // Find closest overlapping edible actor
        let eat_r2 = SLIME_EAT_RADIUS * SLIME_EAT_RADIUS;
        let mut chosen: Option<(i64, IdentifiableId, bool, u32, IdentifiableId, IdentifiableId)> =
            None;
        let mut best_d2 = eat_r2;

        for (&other_id, other) in &model.actors {
            if other_id == slime_id
                || held_actor_id == Some(other_id)
                || consumed_targets.contains(&other_id)
            {
                continue;
            }
            if matches!(&other.kind, ActorKind::Plort(pm) if pm.invulnerable) {
                continue;
            }
            let d2 = slime_pos.distance_squared(other.pos);
            if d2 <= best_d2 {
                if let Some(entry) = def.diet.eat_map.iter().find(|e| e.eats == other.ident) {
                    // Prevent peaceful Tarr creation if game mode forbids hostiles
                    if entry.becomes_id.is_tarr() && mode.prevent_hostiles {
                        continue;
                    }
                    best_d2 = d2;
                    chosen = Some((
                        other_id,
                        other.ident,
                        entry.is_favorite,
                        entry.num_to_produce(),
                        entry.produces_id,
                        entry.becomes_id,
                    ));
                }
            }
        }

        if let Some((food_id, food_ident, _is_fav, prod_count, prod_id, becomes_id)) = chosen {
            consumed_targets.push(food_id);
            events.eaten_pairs.push((slime_id, food_ident));

            if let Some(sa) = model.actors.get_mut(&slime_id) {
                if let ActorKind::Slime(ref mut sm) = sa.kind {
                    SlimeEmotionRuntime::adjust(sm, SlimeEmotion::Hunger, -EAT_HUNGER_REDUCTION);
                    SlimeEmotionRuntime::adjust(
                        sm,
                        SlimeEmotion::Agitation,
                        -EAT_AGITATION_REDUCTION,
                    );
                    if sm.is_feral && !slime_ident.is_tarr() {
                        sm.is_feral = false;
                        sm.agitation.curr_val = 0.0;
                        state.feral_expires_at = None;
                        events.calmed_feral.push(slime_id);
                    }
                }
            }

            if prod_id != IdentifiableId::None {
                state.pending_produce_id = Some(prod_id);
                state.pending_produce_count = prod_count.max(1);
            }
            if becomes_id != IdentifiableId::None {
                state.pending_becomes_id = Some(becomes_id);
            }
            state.digest_ready_at = elapsed_time + DIGEST_DELAY_SECS;
        }

        // Tarr bite: turns nearby non-Tarr slimes into Tarr
        if slime_ident.is_tarr() && !mode.prevent_hostiles && elapsed_time >= state.next_tarr_bite_time
        {
            let bite_r2 = TARR_BITE_RADIUS * TARR_BITE_RADIUS;
            for (&other_id, other) in &model.actors {
                if other_id == slime_id
                    || !other.ident.is_slime()
                    || other.ident.is_tarr()
                    || tarr_targets.contains(&other_id)
                {
                    continue;
                }
                if slime_pos.distance_squared(other.pos) <= bite_r2 {
                    tarr_targets.push(other_id);
                    state.next_tarr_bite_time = elapsed_time + TARR_BITE_COOLDOWN;
                    break;
                }
            }
        }
    }

    // Apply consumed foods
    for fid in consumed_targets {
        model.destroy_actor(fid);
        physics.remove_body(fid);
    }

    // Apply Tarr conversions
    for tid in tarr_targets {
        if let Some(a) = model.actors.get(&tid).cloned() {
            let old_sm = match a.kind {
                ActorKind::Slime(s) => s,
                _ => SlimeModel::default(),
            };
            transform_slime(
                model,
                physics,
                tid,
                a.ident,
                IdentifiableId::TarrSlime,
                a.pos,
                a.curr_region_set_id,
                old_sm,
                &mut events,
            );
            events.tarr_converted.push(tid);
        }
    }

    // Apply Tarr sunlight deaths
    for tid in tarr_sunlight_deaths {
        model.destroy_actor(tid);
        physics.remove_body(tid);
        eat_states.remove(&tid);
        events.tarr_destroyed.push(tid);
    }

    events
}

fn transform_slime(
    model: &mut GameModel,
    physics: &mut PhysicsWorld,
    slime_id: i64,
    old_ident: IdentifiableId,
    new_ident: IdentifiableId,
    pos: Vec3,
    region: RegionSetId,
    mut old_slime: SlimeModel,
    events: &mut EatStepEvents,
) {
    if new_ident.is_tarr() {
        SlimeEmotionRuntime::init_slime_emotions(&mut old_slime, new_ident);
        old_slime.is_feral = false;
    }
    if let Some(actor) = model.actors.get_mut(&slime_id) {
        actor.ident = new_ident;
        actor.curr_region_set_id = region;
        actor.kind = ActorKind::Slime(old_slime);
    }
    if let Some(body) = physics.bodies.get_mut(&slime_id) {
        body.radius = if new_ident.is_largo() || new_ident.is_tarr() {
            0.75
        } else {
            0.45
        };
        body.pos = pos;
    }
    events
        .transformed_slimes
        .push((slime_id, old_ident, new_ident));
}
