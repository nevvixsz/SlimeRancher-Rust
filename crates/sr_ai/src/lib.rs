//! Slime AI, emotions, subbehaviour plexer, diet/largo/tarr lifecycle, unique abilities, and Gordos.

pub mod abilities_classic;
pub mod eat;
pub mod emotions;
pub mod plexer;

pub use abilities_classic::{
    slime_has_trait, ClassicAbilityEvents, ClassicAbilityState, CrystalCluster, RadPhase,
    RockRollPhase, BOOM_EXPLODE_POWER, BOOM_EXPLODE_RADIUS, CRYSTAL_DAMAGE,
    PHOSPHOR_DAYLIGHT_BURN_SECS, PHOSPHOR_HOVER_HEIGHT, RAD_BASE_RADIUS, RAD_EXPAND_FACTOR,
    ROCK_ROLL_DAMAGE,
};

pub use eat::{
    step_slime_eat_and_tarr, EatStepEvents, SlimeEatState, DEFAULT_FERAL_LIFETIME_HOURS,
    DIGEST_DELAY_SECS, DYNAMIC_FERAL_AGITATION, EAT_AGITATION_REDUCTION, EAT_HUNGER_REDUCTION,
    SLIME_EAT_RADIUS, TARR_BITE_COOLDOWN, TARR_BITE_RADIUS, TARR_SUNLIGHT_BURN_RATE,
};
pub use emotions::{
    DriveCalculator, EmotionEnvironment, SlimeEmotionRuntime, ANGRY_CUTOFF, FAVORITE_TOY_FACTOR,
    HUNGRY_CUTOFF, MUSIC_BOX_FACTOR, NON_FAVORITE_TOY_FACTOR, POLLEN_SOURCE_AGITATION_PER_HOUR,
    STARVING_AGITATION_PER_HOUR, STARVING_CUTOFF, TERRIFIED_CUTOFF,
};
pub use plexer::{
    gather_stimuli, ActiveSubbehaviour, PlexerOutput, SlimePlexerState, SlimeStimuli,
    ATTACK_PLAYER_COOLDOWN, ATTACK_PLAYER_DAMAGE, DEFAULT_ACTIVATION_DELAY, GROUNDED_CHECK_FREQ,
    MIN_RELEVANCY, RETHINK_TIME,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_ids::{IdentifiableId, SlimeEmotion};
    use sr_model::SlimeModel;

    #[test]
    fn slime_emotions_hunger_and_starvation_agitation_match_csharp() {
        let mut sm = SlimeModel::default();
        SlimeEmotionRuntime::init_slime_emotions(&mut sm, IdentifiableId::PinkSlime);
        let mut rt = SlimeEmotionRuntime::new(32400.0);

        // Advance 2 game hours (7200 game sec): hunger rises from 0.5 -> 1.0 (0.5/hr * 2hr = +1.0 clamped)
        rt.update_to_time(&mut sm, 32400.0 + 7200.0);
        assert_eq!(sm.hunger.curr_val, 1.0);
        assert!(sm.hunger.curr_val >= STARVING_CUTOFF);

        // Now starving (>= 0.99): base_adj = -0.416667, recovery = 0.333 -> net -0.083667 -> agitation rises!
        rt.update_to_time(&mut sm, 32400.0 + 14400.0);
        assert!(sm.agitation.curr_val > 0.0);

        // Music box + favorite toy boosts recovery factor and calms agitation faster
        rt.env.music_box_count = 1;
        rt.env.nearby_favorite_toy_count = 1;
        let drive = DriveCalculator::new(SlimeEmotion::Hunger, 0.0, 0.0).drive(&sm);
        assert_eq!(drive, 1.0);
    }
}
