//! Pure game state models ported 1:1 from `MonomiPark.SlimeRancher.DataModel`.

pub mod actor;
pub mod ammo;
pub mod enums;
pub mod gadgets;
pub mod game;
pub mod meta;
pub mod objects;
pub mod player;
pub mod ranch;
pub mod world;

pub use actor::{
    actor_ids, ActorKind, ActorModel, AnimalModel, EmotionState, LiquidSourceModel, PlortModel,
    ProduceModel, SlimeEmotionSnapshot, SlimeModel,
};
pub use ammo::{AmmoModel, AmmoSlot};
pub use enums::*;
pub use gadgets::{
    BlueprintLockData, DroneModel, DroneProgramData, ExtractorModel, GadgetKind, GadgetModel,
    GadgetSiteModel, GadgetsModel, SnareModel, WarpDepotModel,
};
pub use game::GameModel;
pub use meta::{
    AchievesModel, AppearancesModel, HolidayModel, InstrumentModel, MailEntry, MailModel,
    PediaModel, ProgressModel, TutorialsModel,
};
pub use objects::{
    AccessDoorModel, DecorizerModel, EchoNoteGordoModel, GlitchImpostoModel,
    GlitchSlimulationModel, GlitchTarrNodeModel, GordoModel, MasterSwitchModel, OasisModel,
    PuzzleSlotModel, QuicksilverEnergyGeneratorModel, TreasurePodModel,
};
pub use player::{PlayerModel, UpgradeLockData};
pub use ranch::{LandPlotModel, RanchCellModel, RanchModel};
pub use world::{ExchangeItemEntry, ExchangeOffer, ExchangeRequestedItemEntry, WorldModel};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_data::GameModeSettings;
    use sr_ids::{
        AmmoMode, GameMode, GadgetId, IdentifiableId, LandPlotId, LandPlotUpgrade, PlayerUpgrade,
        RegionSetId, SlimeEmotion,
    };
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

    #[test]
    fn player_upgrades_rads_and_recovery_match_csharp() {
        let mut p = PlayerModel::default();
        let classic = GameModeSettings::for_mode(GameMode::Classic);
        p.reset(&classic);
        assert_eq!(p.currency, 250);
        assert!(p.end_game_time.is_none());

        let rush = GameModeSettings::for_mode(GameMode::TimeLimitV2);
        p.reset(&rush);
        assert_eq!(p.currency, 1000);
        assert!(p.end_game_time.is_some());

        p.apply_upgrade(PlayerUpgrade::Health2, true, 32400.0);
        assert_eq!(p.max_health, 200);
        p.apply_upgrade(PlayerUpgrade::LiquidSlot, true, 32400.0);
        assert_eq!(p.ammo_dict[&AmmoMode::Default].usable_slots, 5);
        p.apply_upgrade(PlayerUpgrade::SpareKey, true, 32400.0);
        assert_eq!(p.keys, 1);

        let dmg = p.add_rads(125.0, 32400.0);
        assert_eq!(dmg, 20);
        assert_eq!(p.curr_rads, 105.0);
    }

    #[test]
    fn ranch_fully_upgraded_and_gadgets_defaults() {
        let mut plots = std::collections::HashMap::new();
        let mut corral = LandPlotModel::new("plot1", LandPlotId::Corral);
        for u in [
            LandPlotUpgrade::Walls,
            LandPlotUpgrade::MusicBox,
            LandPlotUpgrade::AirNet,
            LandPlotUpgrade::SolarShield,
            LandPlotUpgrade::PlortCollector,
            LandPlotUpgrade::Feeder,
        ] {
            corral.upgrades.insert(u);
        }
        let mut coop = LandPlotModel::new("plot2", LandPlotId::Coop);
        for u in [
            LandPlotUpgrade::Walls,
            LandPlotUpgrade::Feeder,
            LandPlotUpgrade::Vitamizer,
        ] {
            coop.upgrades.insert(u);
        }
        let mut silo = LandPlotModel::new("plot3", LandPlotId::Silo);
        for u in [
            LandPlotUpgrade::Storage2,
            LandPlotUpgrade::Storage3,
            LandPlotUpgrade::Storage4,
        ] {
            silo.upgrades.insert(u);
        }
        plots.insert("plot1".into(), corral);
        plots.insert("plot2".into(), coop);
        plots.insert("plot3".into(), silo);
        assert!(RanchModel::includes_fully_upgraded_corral_coop_and_silo(&plots));

        let g = GadgetsModel::new();
        assert_eq!(g.blueprints, vec![GadgetId::ExtractorDrillNovice]);
        assert!(!g.avail_blueprints.contains(&GadgetId::HydroShower));
    }

    #[test]
    fn game_model_actor_lifecycle_and_rush_game_over() {
        let mut gm = GameModel::new(GameMode::TimeLimitV2);
        let id1 = gm.create_actor(
            IdentifiableId::PinkSlime,
            RegionSetId::Home,
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
        );
        let id2 = gm.create_actor(
            IdentifiableId::RockSlime,
            RegionSetId::Home,
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        assert_eq!(id1, 100);
        assert_eq!(id2, 101);
        assert_eq!(gm.actors_of_type(IdentifiableId::PinkSlime).count(), 1);
        assert!(gm.destroy_actor(id1).is_some());
        assert_eq!(gm.actors_of_type(IdentifiableId::PinkSlime).count(), 0);

        assert!(!gm.is_game_over());
        let end = gm.player.end_game_time.unwrap();
        gm.world.world_time = end;
        assert!(gm.is_game_over());
    }
}
