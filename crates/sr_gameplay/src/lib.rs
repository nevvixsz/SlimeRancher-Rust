//! Player gameplay systems: `PlayerState`, `Ammo`, `WeaponVacuum`, and actor lifecycle rules.

pub mod player;

pub use player::{DeathOutcome, DeathSource, PlayerController};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_data::GameModeSettings;
    use sr_ids::{AmmoMode, GameMode, IdentifiableId, PlayerUpgrade};
    use sr_model::{AmmoSlot, PlayerModel};
    use sr_physics::PlayerMotor;

    #[test]
    fn death_clears_ammo_restores_vitals_and_advances_to_dawn() {
        let mut model = PlayerModel::default();
        let mut motor = PlayerMotor::default();
        let mut ctrl = PlayerController::default();
        let classic = GameModeSettings::for_mode(GameMode::Classic);
        model.reset(&classic);

        model.ammo_dict.get_mut(&AmmoMode::Default).unwrap().slots[0] = Some(AmmoSlot {
            id: IdentifiableId::PinkSlime,
            count: 5,
            emotions: vec![],
        });
        let mut world_time = 10.0 * 3600.0; // 10:00 Day 1
        assert!(ctrl.absorb_rads(&mut model, &classic, 1100.0, world_time));
        let outcome = ctrl.handle_death(&mut model, &mut motor, &classic, DeathSource::SlimeRad, &mut world_time);
        assert_eq!(outcome.currency_lost, 0);
        assert!(world_time >= 86400.0 + 6.0 * 3600.0 - 1.0);
        assert!(model.ammo_dict[&AmmoMode::Default].slots[0].is_none());
        assert_eq!(model.curr_health, 100.0);
        assert_eq!(model.curr_rads, 0.0);
    }

    #[test]
    fn upgrade_grants_and_treasure_cracker_tier() {
        let mut model = PlayerModel::default();
        assert_eq!(PlayerController::treasure_cracker_level(&model), 0);
        assert!(PlayerController::grant_upgrade(&mut model, PlayerUpgrade::TreasureCracker1, 32400.0));
        assert!(!PlayerController::grant_upgrade(&mut model, PlayerUpgrade::TreasureCracker1, 32400.0));
        assert_eq!(PlayerController::treasure_cracker_level(&model), 1);
        PlayerController::grant_upgrade(&mut model, PlayerUpgrade::Ammo4, 32400.0);
        assert_eq!(model.max_ammo, 100);
    }
}
