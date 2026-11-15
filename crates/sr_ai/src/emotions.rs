//! Port of `SlimeEmotions.cs` and `DriveCalculator.cs`.

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, SlimeEmotion};
use sr_model::{EmotionState, SlimeModel};

pub const HUNGRY_CUTOFF: f32 = 0.666;
pub const STARVING_CUTOFF: f32 = 0.99;
pub const ANGRY_CUTOFF: f32 = 0.9;
pub const TERRIFIED_CUTOFF: f32 = 0.99;

pub const STARVING_AGITATION_PER_HOUR: f32 = 0.416667;
pub const POLLEN_SOURCE_AGITATION_PER_HOUR: f32 = 0.416667;
pub const FAVORITE_TOY_FACTOR: f32 = 0.5;
pub const NON_FAVORITE_TOY_FACTOR: f32 = 0.25;
pub const MUSIC_BOX_FACTOR: f32 = 1.0;

/// Port of `DriveCalculator`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriveCalculator {
    pub emotion: SlimeEmotion,
    pub extra_drive: f32,
    pub min_drive: f32,
}

impl DriveCalculator {
    pub const fn new(emotion: SlimeEmotion, extra_drive: f32, min_drive: f32) -> Self {
        Self {
            emotion,
            extra_drive,
            min_drive,
        }
    }

    pub fn drive(&self, slime: &SlimeModel) -> f32 {
        let curr = match self.emotion {
            SlimeEmotion::None => 1.0,
            e => slime.emotion_curr(e),
        };
        (self.min_drive.max(curr) + self.extra_drive).max(0.0)
    }
}

/// Environmental modifiers influencing `SlimeEmotions.UpdateToTime`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EmotionEnvironment {
    pub music_box_count: usize,
    pub nearby_favorite_toy_count: usize,
    pub nearby_toy_count: usize,
    pub pollen_source_count: usize,
    pub mod_hunger_factor: f32,
}

impl EmotionEnvironment {
    pub fn with_defaults() -> Self {
        Self {
            mod_hunger_factor: 1.0,
            ..Default::default()
        }
    }
}

/// Runtime emotion updater mirroring `SlimeEmotions.UpdateToTime`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeEmotionRuntime {
    pub last_update_time: f64,
    pub env: EmotionEnvironment,
}

impl SlimeEmotionRuntime {
    pub fn new(world_time: f64) -> Self {
        Self {
            last_update_time: world_time.max(32400.0),
            env: EmotionEnvironment::with_defaults(),
        }
    }

    /// Port of `SlimeEmotions.UpdateToTime(worldTime)`.
    pub fn update_to_time(&mut self, model: &mut SlimeModel, world_time: f64) {
        let hours = ((world_time - self.last_update_time) * 0.00027777778450399637) as f32;
        if hours <= 0.0 {
            return;
        }

        let hunger_curr = model.hunger.curr_val;
        let fear_curr = model.fear.curr_val;

        let base_adj_agitation = {
            let mut adj = 0.0f32;
            if hunger_curr >= STARVING_CUTOFF || fear_curr >= TERRIFIED_CUTOFF {
                adj -= STARVING_AGITATION_PER_HOUR;
            }
            if self.env.pollen_source_count > 0 {
                adj -= POLLEN_SOURCE_AGITATION_PER_HOUR;
            }
            adj
        };

        let agitation_factor = {
            let mut f = 1.0f32;
            if self.env.music_box_count > 0 {
                f += MUSIC_BOX_FACTOR;
            }
            if self.env.nearby_favorite_toy_count > 0 {
                f += FAVORITE_TOY_FACTOR;
            } else if self.env.nearby_toy_count > 0 {
                f += NON_FAVORITE_TOY_FACTOR;
            }
            f
        };

        let hunger_factor = self.env.mod_hunger_factor.max(0.01);

        Self::step_emotion_state(&mut model.hunger, 0.0, hunger_factor, hours);
        Self::step_emotion_state(&mut model.agitation, base_adj_agitation, agitation_factor, hours);
        Self::step_emotion_state(&mut model.fear, 0.0, 1.0, hours);

        self.last_update_time = world_time;
    }

    fn step_emotion_state(
        state: &mut EmotionState,
        base_adj: f32,
        recovery_factor: f32,
        hours: f32,
    ) {
        if !state.enabled {
            return;
        }
        let num2 = state.recovery_per_game_hour + base_adj;
        let rate = if num2 < 0.0 {
            num2 / recovery_factor
        } else {
            num2 * recovery_factor
        };
        if state.curr_val > state.def_val {
            state.curr_val = (state.curr_val - rate * hours).max(state.def_val);
        } else if state.curr_val < state.def_val {
            state.curr_val = (state.curr_val + rate * hours).min(state.def_val);
        } else if state.def_val == 0.0 && rate < 0.0 {
            state.curr_val = (state.curr_val - rate * hours).max(state.def_val);
        } else if state.def_val == 1.0 && rate < 0.0 {
            state.curr_val = (state.curr_val + rate * hours).min(state.def_val);
        }
        state.curr_val = state.curr_val.clamp(0.0, 1.0);
    }

    /// Port of `SlimeEmotions.Adjust`.
    pub fn adjust(model: &mut SlimeModel, emotion: SlimeEmotion, delta: f32) -> bool {
        if let Some(st) = model.emotion_mut(emotion) {
            if st.enabled {
                st.curr_val = (st.curr_val + delta).clamp(0.0, 1.0);
                return true;
            }
        }
        false
    }

    /// Computes initial default `SlimeModel` emotion settings matching `SlimeEmotions.cs`
    /// (`initHunger = (0.5, 1, 1, 0.5)`, `initAgitation = (0, 0, 1, 0.333)`, `initFear = (0, 0, 1, 5)`).
    pub fn init_slime_emotions(model: &mut SlimeModel, ident: IdentifiableId) {
        model.hunger = EmotionState::new(SlimeEmotion::Hunger, 0.5, 1.0, 1.0, 0.5);
        model.agitation = EmotionState::new(SlimeEmotion::Agitation, 0.0, 0.0, 1.0, 0.333);
        model.fear = EmotionState::new(SlimeEmotion::Fear, 0.0, 0.0, 1.0, 5.0);
        if ident.is_tarr() {
            model.agitation.curr_val = 1.0;
        }
    }
}
