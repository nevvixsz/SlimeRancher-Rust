//! Slime AI, emotions, subbehaviour plexer, diet/largo/tarr lifecycle, unique abilities, and Gordos.

pub mod abilities_classic;
pub mod abilities_rare;
pub mod eat;
pub mod emotions;
pub mod gordo;
pub mod plexer;

pub use abilities_classic::{
    slime_has_trait, ClassicAbilityEvents, ClassicAbilityState, CrystalCluster, RadPhase,
    RockRollPhase, BOOM_EXPLODE_POWER, BOOM_EXPLODE_RADIUS, CRYSTAL_DAMAGE,
    PHOSPHOR_DAYLIGHT_BURN_SECS, PHOSPHOR_HOVER_HEIGHT, RAD_BASE_RADIUS, RAD_EXPAND_FACTOR,
    ROCK_ROLL_DAMAGE,
};
pub use abilities_rare::{
    step_whirlwinds, RareAbilityEvents, RareAbilityState, WhirlwindHazard,
    DERVISH_MAX_WHIRLWINDS, DERVISH_WHIRLWIND_CUTOFF, DERVISH_WHIRLWIND_RADIUS,
    TANGLE_GRAPPLE_COOLDOWN, TANGLE_GRAPPLE_RANGE,
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
pub use gordo::{
    step_gordos, AiStepEvents, GordoRewardKind, GordoStepEvents, SlimeAiCoordinator,
    SlimeRuntimeEntry, WorldGordo, GORDO_EAT_RADIUS,
};
pub use plexer::{
    gather_stimuli, ActiveSubbehaviour, PlexerOutput, SlimePlexerState, SlimeStimuli,
    ATTACK_PLAYER_COOLDOWN, ATTACK_PLAYER_DAMAGE, DEFAULT_ACTIVATION_DELAY, GROUNDED_CHECK_FREQ,
    MIN_RELEVANCY, RETHINK_TIME,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_data::{DataRegistry, GameModeSettings};
    use sr_ids::{GameMode, IdentifiableId, RegionSetId, SlimeEmotion};
    use sr_math::{Quat, Randoms, Vec3};
    use sr_model::{GameModel, SlimeModel};
    use sr_physics::{PhysicsWorld, RigidBody};

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

    #[test]
    fn slime_eat_favorite_produces_double_plorts_and_transforms_to_largo_and_tarr() {
        let registry = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut model = GameModel::new(GameMode::Classic);
        let mut physics = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(42);
        let mut ai = SlimeAiCoordinator::default();

        // Spawn Tabby slime + favorite Stony Hen near it
        let tabby_id = model.create_actor(
            IdentifiableId::TabbySlime,
            RegionSetId::Home,
            Vec3::new(0.0, 0.5, 0.0),
            Quat::IDENTITY,
        );
        physics.insert_body(RigidBody {
            actor_id: tabby_id,
            pos: Vec3::new(0.0, 0.5, 0.0),
            radius: 0.45,
            ..Default::default()
        });

        let hen_id = model.create_actor(
            IdentifiableId::StonyHen,
            RegionSetId::Home,
            Vec3::new(0.4, 0.5, 0.0),
            Quat::IDENTITY,
        );
        physics.insert_body(RigidBody {
            actor_id: hen_id,
            pos: Vec3::new(0.4, 0.5, 0.0),
            radius: 0.25,
            ..Default::default()
        });

        // Step 1: eats StonyHen
        let ev1 = ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            0.1,
            &mut rng,
        );
        assert_eq!(ev1.eat.eaten_pairs, vec![(tabby_id, IdentifiableId::StonyHen)]);

        // Step past DIGEST_DELAY_SECS (2.0s) -> 2 TabbyPlorts produced!
        let ev2 = ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            2.1,
            &mut rng,
        );
        assert_eq!(ev2.eat.spawned_plorts.len(), 2);
        assert!(ev2
            .eat
            .spawned_plorts
            .iter()
            .all(|(_, id)| *id == IdentifiableId::TabbyPlort));

        // Feed PinkPlort -> becomes PinkTabbyLargo!
        let p_plort = model.create_actor(
            IdentifiableId::PinkPlort,
            RegionSetId::Home,
            Vec3::new(0.2, 0.5, 0.0),
            Quat::IDENTITY,
        );
        physics.insert_body(RigidBody {
            actor_id: p_plort,
            pos: Vec3::new(0.2, 0.5, 0.0),
            radius: 0.25,
            ..Default::default()
        });
        ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            0.1,
            &mut rng,
        );
        let ev_largo = ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            2.1,
            &mut rng,
        );
        assert_eq!(
            ev_largo.eat.transformed_slimes,
            vec![(
                tabby_id,
                IdentifiableId::TabbySlime,
                IdentifiableId::PinkTabbyLargo
            )]
        );

        // Feed RockPlort (3rd plort type) -> becomes TarrSlime!
        let r_plort = model.create_actor(
            IdentifiableId::RockPlort,
            RegionSetId::Home,
            Vec3::new(0.2, 0.5, 0.0),
            Quat::IDENTITY,
        );
        physics.insert_body(RigidBody {
            actor_id: r_plort,
            pos: Vec3::new(0.2, 0.5, 0.0),
            radius: 0.25,
            ..Default::default()
        });
        ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            0.1,
            &mut rng,
        );
        let ev_tarr = ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            2.1,
            &mut rng,
        );
        assert_eq!(
            ev_tarr.eat.transformed_slimes,
            vec![(
                tabby_id,
                IdentifiableId::PinkTabbyLargo,
                IdentifiableId::TarrSlime
            )]
        );
    }

    #[test]
    fn gordo_eat_counts_favorites_double_and_pops_rewards() {
        let registry = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut model = GameModel::new(GameMode::Classic);
        let mut physics = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(7);
        let mut ai = SlimeAiCoordinator::default();

        ai.register_gordo(
            &mut model,
            WorldGordo {
                gordo_id: "gordo_rock_test".into(),
                slime_ident: IdentifiableId::RockSlime,
                pos: Vec3::new(10.0, 0.0, 0.0),
                region: RegionSetId::Home,
                rewards: vec![GordoRewardKind::SlimeKey],
                spawn_slime_count: 3,
            },
            4,
        );

        // Drop 2 HeartBeets (RockSlime favorite -> 2 * 2 = 4 count -> instant pop!)
        for offset in [0.2, -0.2] {
            let fid = model.create_actor(
                IdentifiableId::BeetVeggie,
                RegionSetId::Home,
                Vec3::new(10.0 + offset, 0.5, 0.0),
                Quat::IDENTITY,
            );
            physics.insert_body(RigidBody {
                actor_id: fid,
                pos: Vec3::new(10.0 + offset, 0.5, 0.0),
                radius: 0.25,
                ..Default::default()
            });
        }

        let ev = ai.step(
            &mut model,
            &mut physics,
            &registry,
            &mode,
            &[],
            None,
            true,
            12.0,
            0.1,
            &mut rng,
        );
        assert_eq!(ev.gordo.popped, vec!["gordo_rock_test".to_string()]);
        assert_eq!(ev.gordo.keys_awarded, 1);
        assert_eq!(model.player.keys, 1);
        assert_eq!(model.gordos["gordo_rock_test"].gordo_eaten_count, -1);
    }
}
