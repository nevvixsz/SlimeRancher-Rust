//! Pure game state models ported 1:1 from `MonomiPark.SlimeRancher.DataModel`.

pub mod actor;
pub mod enums;

pub use actor::{
    actor_ids, ActorKind, ActorModel, AnimalModel, EmotionState, LiquidSourceModel, PlortModel,
    ProduceModel, SlimeEmotionSnapshot, SlimeModel,
};
pub use enums::*;

#[cfg(test)]
mod tests {
    use super::*;
    use sr_ids::{IdentifiableId, RegionSetId, SlimeEmotion};
    use sr_math::{Quat, Vec3};

    #[test]
    fn actor_model_factory_matches_csharp_hierarchy() {
        let slime = ActorModel::new(
            100,
            IdentifiableId::PinkSlime,
            RegionSetId::Home,
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        assert!(matches!(slime.kind, ActorKind::Slime(_)));
        assert!(slime.is_edible());

        let mut produce = ActorModel::new(
            101,
            IdentifiableId::CuberryFruit,
            RegionSetId::Home,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(produce.is_edible());
        if let ActorKind::Produce(ref mut p) = produce.kind {
            p.state = ResourceCycleState::Unripe;
        }
        assert!(!produce.is_edible());

        let mut plort = ActorModel::new(
            102,
            IdentifiableId::PinkPlort,
            RegionSetId::Home,
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        if let ActorKind::Plort(ref mut p) = plort.kind {
            p.invulnerable = true;
        }
        assert!(!plort.is_edible());

        let mut em = EmotionState::new(SlimeEmotion::Hunger, 0.5, 1.0, 1.0, -0.08);
        em.set_enabled(false);
        assert_eq!(em.curr_val, 0.0);
        em.set_enabled(true);
        assert_eq!(em.curr_val, 1.0);
    }
}
