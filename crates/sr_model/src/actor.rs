//! Actor models ported from `ActorModel.cs`, `SlimeModel.cs`, `GlitchSlimeModel.cs`,
//! `AnimalModel.cs`, `ProduceModel.cs`, `PlortModel.cs`, `ResourceModel.cs`,
//! `ScienceMatModel.cs`, `EchoModel.cs`, `BasicActorModel.cs`, and `LiquidSourceModel.cs`.

use serde::{Deserialize, Serialize};
use sr_ids::{IdentifiableId, RegionSetId, SlimeEmotion};
use sr_math::{Quat, Vec3};

use crate::enums::ResourceCycleState;

/// Reserved & starting actor IDs matching `ActorModel.Id`.
pub mod actor_ids {
    pub const PLAYER: i64 = 1;
    pub const BEGIN_DYNAMIC: i64 = 100;
}

/// Port of `SlimeEmotions.EmotionState`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EmotionState {
    pub emotion: SlimeEmotion,
    pub curr_val: f32,
    pub def_val: f32,
    pub sensitivity: f32,
    pub recovery_per_game_hour: f32,
    pub enabled: bool,
}

impl EmotionState {
    pub fn new(
        emotion: SlimeEmotion,
        curr_val: f32,
        def_val: f32,
        sensitivity: f32,
        recovery_per_game_hour: f32,
    ) -> Self {
        Self {
            emotion,
            curr_val,
            def_val,
            sensitivity,
            recovery_per_game_hour,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.curr_val = if enabled { self.def_val } else { 0.0 };
        }
    }
}

/// Port of `SlimeEmotionData` stored on captured slimes in vacpack ammo slots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SlimeEmotionSnapshot {
    pub agitation: f32,
    pub fear: f32,
    pub hunger: f32,
}

impl Default for SlimeEmotionSnapshot {
    fn default() -> Self {
        Self {
            agitation: 0.0,
            fear: 0.0,
            hunger: 0.0,
        }
    }
}

/// Port of `SlimeModel` (+ optional `GlitchSlimeModel` fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeModel {
    pub agitation: EmotionState,
    pub fear: EmotionState,
    pub hunger: EmotionState,
    pub disabled_at_time: Option<f64>,
    pub is_feral: bool,
    pub is_glitch: bool,
    pub fashions: Vec<IdentifiableId>,
    /// Populated when `ident == IdentifiableId::GlitchSlime` (`GlitchSlimeModel`).
    pub glitch_exposure_chance: f32,
    pub glitch_death_time: f64,
}

impl Default for SlimeModel {
    fn default() -> Self {
        Self {
            agitation: EmotionState::new(SlimeEmotion::Agitation, 0.0, 0.0, 1.0, 0.1),
            fear: EmotionState::new(SlimeEmotion::Fear, 0.0, 0.0, 1.0, 0.25),
            hunger: EmotionState::new(SlimeEmotion::Hunger, 0.5, 1.0, 1.0, -0.083333336),
            disabled_at_time: None,
            is_feral: false,
            is_glitch: false,
            fashions: Vec::new(),
            glitch_exposure_chance: 0.0,
            glitch_death_time: 0.0,
        }
    }
}

impl SlimeModel {
    pub fn snapshot(&self) -> SlimeEmotionSnapshot {
        SlimeEmotionSnapshot {
            agitation: self.agitation.curr_val,
            fear: self.fear.curr_val,
            hunger: self.hunger.curr_val,
        }
    }

    pub fn apply_snapshot(&mut self, snap: SlimeEmotionSnapshot) {
        self.agitation.curr_val = snap.agitation.clamp(0.0, 1.0);
        self.fear.curr_val = snap.fear.clamp(0.0, 1.0);
        self.hunger.curr_val = snap.hunger.clamp(0.0, 1.0);
    }

    pub fn emotion_mut(&mut self, emotion: SlimeEmotion) -> Option<&mut EmotionState> {
        match emotion {
            SlimeEmotion::Hunger => Some(&mut self.hunger),
            SlimeEmotion::Agitation => Some(&mut self.agitation),
            SlimeEmotion::Fear => Some(&mut self.fear),
            SlimeEmotion::None => None,
        }
    }

    pub fn emotion_curr(&self, emotion: SlimeEmotion) -> f32 {
        match emotion {
            SlimeEmotion::Hunger => self.hunger.curr_val,
            SlimeEmotion::Agitation => self.agitation.curr_val,
            SlimeEmotion::Fear => self.fear.curr_val,
            SlimeEmotion::None => 0.0,
        }
    }
}

/// Port of `AnimalModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimalModel {
    pub transform_time: f64,
    pub next_reproduce_time: f64,
    pub fashions: Vec<IdentifiableId>,
}

/// Port of `ProduceModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProduceModel {
    pub state: ResourceCycleState,
    pub progress_time: f64,
}

impl Default for ProduceModel {
    fn default() -> Self {
        Self {
            state: ResourceCycleState::Edible,
            progress_time: f64::NAN,
        }
    }
}

impl ProduceModel {
    #[inline]
    pub fn is_edible(&self) -> bool {
        self.state == ResourceCycleState::Edible
    }
}

/// Port of `PlortModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlortModel {
    pub destroy_time: f64,
    /// Mirrors `PlortInvulnerability.enabled` (newly launched plorts briefly immune to eating).
    pub invulnerable: bool,
}

impl Default for PlortModel {
    fn default() -> Self {
        Self {
            destroy_time: f64::INFINITY,
            invulnerable: false,
        }
    }
}

/// Type-specific payload corresponding to the `ActorModel` class hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActorKind {
    Slime(SlimeModel),
    Animal(AnimalModel),
    Produce(ProduceModel),
    Plort(PlortModel),
    ScienceMat,
    Echo,
    Basic,
}

/// Port of `ActorModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorModel {
    pub actor_id: i64,
    pub ident: IdentifiableId,
    pub curr_region_set_id: RegionSetId,
    pub pos: Vec3,
    pub rot: Quat,
    pub kind: ActorKind,
}

impl ActorModel {
    /// Creates an `ActorModel` matching `GameModel.CreateActorModel`.
    pub fn new(
        actor_id: i64,
        ident: IdentifiableId,
        region_set_id: RegionSetId,
        pos: Vec3,
        rot: Quat,
    ) -> Self {
        let kind = if ident == IdentifiableId::GlitchSlime {
            let mut s = SlimeModel::default();
            s.is_glitch = true;
            ActorKind::Slime(s)
        } else if ident.is_slime() {
            ActorKind::Slime(SlimeModel::default())
        } else if ident.is_animal() {
            ActorKind::Animal(AnimalModel::default())
        } else if ident.is_food() || ident == IdentifiableId::LemonPhase {
            ActorKind::Produce(ProduceModel::default())
        } else if ident.is_plort() {
            ActorKind::Plort(PlortModel::default())
        } else if ident.is_craft() {
            ActorKind::ScienceMat
        } else if ident.is_echo() {
            ActorKind::Echo
        } else {
            ActorKind::Basic
        };

        Self {
            actor_id,
            ident,
            curr_region_set_id: region_set_id,
            pos,
            rot,
            kind,
        }
    }

    /// Port of `ActorModel.IsEdible()`.
    pub fn is_edible(&self) -> bool {
        match &self.kind {
            ActorKind::Produce(p) => p.is_edible(),
            ActorKind::Plort(p) => !p.invulnerable,
            _ => true,
        }
    }

    pub fn as_slime(&self) -> Option<&SlimeModel> {
        match &self.kind {
            ActorKind::Slime(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_slime_mut(&mut self) -> Option<&mut SlimeModel> {
        match &mut self.kind {
            ActorKind::Slime(s) => Some(s),
            _ => None,
        }
    }
}

/// Port of `LiquidSourceModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiquidSourceModel {
    pub pos: Vec3,
    pub is_scaling: bool,
    pub units_filled: f32,
}
