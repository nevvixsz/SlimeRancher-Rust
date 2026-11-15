//! Player gameplay systems: `PlayerState`, `Ammo`, `WeaponVacuum`, and actor lifecycle rules.

pub mod actors;
pub mod ammo;
pub mod player;
pub mod vacuum;

pub use actors::{
    apply_water_spray, spawn_physical_actor, step_actor_lifecycle, ActorLifecycleEvents,
    BREAK_COLLISION_THRESHOLD,
};
pub use ammo::AmmoController;
pub use player::{DeathOutcome, DeathSource, PlayerController};
pub use vacuum::{
    VacEvents, VacInput, VacMode, VacuumableSize, WeaponVacuum, AIR_BURST_POWER, AIR_BURST_RADIUS,
    CAPTURE_DIST, EJECT_SPEED, MAX_VAC_DIST, SHOOT_COOLDOWN, STAMINA_PER_BURST,
};

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

    #[test]
    fn ammo_controller_enforces_liquid_slot_and_emotion_stack() {
        let mut model = PlayerModel::default();
        let mut ammo = AmmoController::default();

        // Slot 4 (liquid) is unusable until LiquidSlot upgrade
        assert!(!ammo.maybe_add_to_slot(
            &mut model,
            IdentifiableId::WaterLiquid,
            None,
            1,
            32400.0
        ));
        PlayerController::grant_upgrade(&mut model, PlayerUpgrade::LiquidSlot, 32400.0);
        assert!(ammo.maybe_add_to_slot(
            &mut model,
            IdentifiableId::WaterLiquid,
            None,
            1,
            32400.0
        ));
        assert_eq!(
            ammo.slot_id(&model, 4, 32400.0),
            IdentifiableId::WaterLiquid
        );

        let snap = sr_model::SlimeEmotionSnapshot {
            agitation: 0.2,
            fear: 0.0,
            hunger: 0.7,
        };
        assert!(ammo.maybe_add_to_slot(
            &mut model,
            IdentifiableId::PinkSlime,
            Some(snap),
            1,
            32400.0
        ));
        assert_eq!(ammo.slot_count(&model, 0), 1);
        let popped = ammo.decrement_selected(&mut model, 1, 32400.0);
        assert_eq!(popped, Some(snap));
        assert_eq!(ammo.slot_count(&model, 0), 0);
    }

    #[test]
    fn weapon_vacuum_captures_slime_holds_largo_shoots_and_airbursts() {
        use sr_ids::RegionSetId;
        use sr_math::{Quat, Vec3};
        use sr_model::GameModel;
        use sr_physics::{CollisionLayers, PhysicsWorld, RigidBody, StaticCollider, StaticShape};

        let mut gm = GameModel::default();
        let mut motor = PlayerMotor::default();
        let mut ammo = AmmoController::default();
        let mut vac = WeaponVacuum::default();
        let mut pw = PhysicsWorld::new();
        pw.add_static(StaticCollider {
            id: "floor".into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::GroundPlane { height: 0.0 },
        });
        PlayerController::grant_upgrade(&mut gm.player, PlayerUpgrade::AirBurst, 32400.0);

        // Place a PinkSlime right in front of the muzzle
        let muzzle = WeaponVacuum::muzzle_origin(&gm, &motor);
        let slime_pos = muzzle + motor.look_dir() * 0.6;
        let slime_id = gm.create_actor(
            IdentifiableId::PinkSlime,
            RegionSetId::Home,
            slime_pos,
            Quat::IDENTITY,
        );
        pw.insert_body(RigidBody {
            actor_id: slime_id,
            pos: slime_pos,
            radius: 0.45,
            use_gravity: false,
            ..Default::default()
        });

        let ev = vac.step(
            &mut gm,
            &motor,
            &mut ammo,
            &mut pw,
            VacInput {
                vac_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(ev.captured, vec![IdentifiableId::PinkSlime]);
        assert_eq!(ammo.slot_count(&gm.player, 0), 1);

        // Shoot it back out
        vac.elapsed_time += 0.5;
        let ev_shoot = vac.step(
            &mut gm,
            &motor,
            &mut ammo,
            &mut pw,
            VacInput {
                shoot_held: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(ev_shoot.expelled.len(), 1);
        let spawned_id = ev_shoot.expelled[0].0;
        assert!(pw.bodies[&spawned_id].vel.z > 20.0);

        // AirBurst pushes slime forward and consumes 20 energy
        let ev_burst = vac.step(
            &mut gm,
            &motor,
            &mut ammo,
            &mut pw,
            VacInput {
                burst_pressed: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        assert!(ev_burst.air_burst_fired);
        assert_eq!(gm.player.curr_energy, 80.0);
    }

    #[test]
    fn break_on_impact_spawns_crate_rewards_and_sea_kill_removes_actor() {
        use sr_ids::RegionSetId;
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::{CollisionLayers, PhysicsWorld, StaticCollider, StaticShape};

        let mut gm = GameModel::default();
        let mut pw = PhysicsWorld::new();
        pw.add_static(StaticCollider {
            id: "floor".into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::GroundPlane { height: 0.0 },
        });
        let crate_id = spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::CrateReef01,
            RegionSetId::Home,
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, -20.0, 0.0),
        );
        let mut rng = Randoms::from_seed(42);
        let mut broken = false;
        for _ in 0..30 {
            let (impacts, overlaps) = pw.step(1.0 / 60.0);
            let ev = step_actor_lifecycle(&mut gm, &mut pw, &impacts, &overlaps, &mut rng);
            if ev.broken_crates.contains(&crate_id) {
                broken = true;
                break;
            }
        }
        assert!(broken);
        assert!(gm.actor(crate_id).is_none());
        assert!(gm.actors.len() >= 2);
    }
}
