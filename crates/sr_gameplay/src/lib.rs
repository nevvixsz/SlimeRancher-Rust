//! Player gameplay systems: `PlayerState`, `Ammo`, `WeaponVacuum`, and actor lifecycle rules.

pub mod actors;
pub mod ammo;
pub mod economy;
pub mod player;
pub mod ranch;
pub mod vacuum;

pub use actors::{
    apply_water_spray, spawn_physical_actor, step_actor_lifecycle, ActorLifecycleEvents,
    BREAK_COLLISION_THRESHOLD,
};
pub use ammo::AmmoController;
pub use economy::{
    build_market_listings, current_seven_zee_rank, deposit_plorts, get_plort_sold_count,
    next_seven_zee_rank, plort_sold_stat_key, purchase_next_seven_zee_rank, step_market_deposits,
    unlock_seven_zee, AcceptOutcome, CoinsType, CurrValueEntry, DepositResponse, EconomyDirector,
    EconomyRuntime, EconomyStepEvents, ExchangeDirector, ExchangeStepEvents, MarketDepositZone,
    MarketListing, PriceTrend, ProgressRewardLevel, RancherProfile, SevenZeeError,
    SevenZeePurchaseResult, SevenZeeRank, BONUS_PROB, CHROMA_UNLOCK_RANK,
    CORPORATE_PARTNER_MAX_RANK, DAILY_OFFER_HOUR, DEFAULT_DAILY_SHUTDOWN_MINS,
    DEFAULT_SATURATION_RECOVERY, EARLY_EXCHANGE_HOURS, MAX_OFFER_VAL, MIN_OFFER_VAL,
    MOCHI_BONUS_CHANCE, MOCHI_BONUS_FACTOR, MOCHI_RECUR_AMOUNT, OGDEN_RECUR_AMOUNT, RANCHER_IDS,
    RARE_PROB, REWARD_BONUS_MULT, REWARD_VAL_MULT, RUSH_PLORT_BONUS_COUNT, SEVEN_ZEE_RANKS,
    SLIME_TOYS_UNLOCK_RANK, STAT_PLORTS_SOLD, VIKTOR_RECUR_AMOUNT,
};
pub use player::{DeathOutcome, DeathSource, PlayerController};
pub use ranch::{
    clear_garden_crop, crop_for_item, deposit_feeder_food, feeder_cycle_hours, plant_garden_crop,
    plot_purchase_cost, step_corrals_and_coops, step_feed_speed, step_gardens_ponds_incinerators,
    upgrade_purchase_cost, upgrade_to_deluxe_crop, water_garden, withdraw_collector_plorts,
    AirNetState, CorralCoopEvents, GardenPondIncineratorEvents, GardenRuntimeState, PlotBounds,
    PlotOpError, RanchDirector, RanchExpansion, RanchRuntime, RanchStepEvents, SiloController,
    ToyDirector, BASE_TOYS, CORPORATE_LEVEL_UNLOCK, FASTFORWARD_CHUNK_SECS, FASTFORWARD_MIN_SECS,
    UPGRADED_TOYS,
};
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
        use sr_math::Quat;
        use sr_model::GameModel;
        use sr_physics::{CollisionLayers, PhysicsWorld, RigidBody, StaticCollider, StaticShape};

        let mut gm = GameModel::default();
        let motor = PlayerMotor::default();
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

    #[test]
    fn ranch_plot_purchase_upgrades_and_silo_capacity() {
        use sr_ids::{LandPlotId, LandPlotUpgrade};
        use sr_model::GameModel;

        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 5000;
        RanchDirector::ensure_plot(&mut gm, "plot_1", LandPlotId::Empty);

        // Build Silo (450 NB)
        assert_eq!(RanchDirector::buy_plot(&mut gm, "plot_1", LandPlotId::Silo), Ok(()));
        assert_eq!(gm.player.currency, 5000 - 450);

        // Cannot skip Storage2 straight to Storage3
        assert_eq!(
            RanchDirector::buy_upgrade(&mut gm, "plot_1", LandPlotUpgrade::Storage3),
            Err(PlotOpError::PrerequisiteUpgradeMissing)
        );
        assert_eq!(
            RanchDirector::buy_upgrade(&mut gm, "plot_1", LandPlotUpgrade::Storage2),
            Ok(())
        );
        assert_eq!(
            RanchDirector::buy_upgrade(&mut gm, "plot_1", LandPlotUpgrade::Storage3),
            Ok(())
        );
        assert_eq!(
            RanchDirector::buy_upgrade(&mut gm, "plot_1", LandPlotUpgrade::Storage4),
            Ok(())
        );

        let plot = gm.land_plots.get_mut("plot_1").unwrap();
        assert_eq!(SiloController::usable_slot_count(plot), 12);
        let stored = SiloController::deposit_to_slot(plot, 0, IdentifiableId::PinkPlort, 350);
        assert_eq!(stored, 300);
        assert_eq!(
            SiloController::withdraw_from_slot(plot, 0, 120),
            Some((IdentifiableId::PinkPlort, 120))
        );
    }

    #[test]
    fn corral_feeder_collector_airnet_and_garden_harvest() {
        use sr_ids::{LandPlotId, LandPlotUpgrade, RegionSetId};
        use sr_math::{Randoms, Vec3};
        use sr_model::{GameModel, SpawnResourceId};
        use sr_physics::PhysicsWorld;

        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 10000;
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(9);
        let mut ranch = RanchRuntime::default();

        RanchDirector::ensure_plot(&mut gm, "corral_1", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "corral_1", LandPlotId::Corral).unwrap();
        RanchDirector::buy_upgrade(&mut gm, "corral_1", LandPlotUpgrade::Feeder).unwrap();
        RanchDirector::buy_upgrade(&mut gm, "corral_1", LandPlotUpgrade::PlortCollector).unwrap();
        RanchDirector::buy_upgrade(&mut gm, "corral_1", LandPlotUpgrade::AirNet).unwrap();
        ranch.register_bounds(PlotBounds {
            plot_id: "corral_1".into(),
            center: Vec3::ZERO,
            half_extents: Vec3::new(5.0, 3.0, 5.0),
            region: RegionSetId::Home,
        });

        // Deposit carrots into feeder and drop a plort inside the corral
        {
            let p = gm.land_plots.get_mut("corral_1").unwrap();
            assert_eq!(deposit_feeder_food(p, IdentifiableId::CarrotVeggie, 10), 10);
        }
        spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::PinkPlort,
            RegionSetId::Home,
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::ZERO,
        );

        // Advance world_time past feeder + collector trigger
        let wt = gm.world.world_time + 7.0 * 3600.0;
        gm.world.world_time = wt;
        let ev = ranch.step(&mut gm, &mut pw, wt, 3600.0, 0.1, &mut rng);
        assert!(!ev.corral_coop.feeder_ejected.is_empty());
        assert_eq!(
            ev.corral_coop.collector_stored,
            vec![("corral_1".to_string(), IdentifiableId::PinkPlort)]
        );
        let withdrawn = withdraw_collector_plorts(gm.land_plots.get_mut("corral_1").unwrap(), 0, 5);
        assert_eq!(withdrawn, Some((IdentifiableId::PinkPlort, 1)));

        // Garden with DeluxeGarden auto-upgrades CuberryTree -> CuberryTreeDlx
        RanchDirector::ensure_plot(&mut gm, "garden_1", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "garden_1", LandPlotId::Garden).unwrap();
        RanchDirector::buy_upgrade(&mut gm, "garden_1", LandPlotUpgrade::DeluxeGarden).unwrap();
        let mut gs = GardenRuntimeState::default();
        assert!(plant_garden_crop(
            gm.land_plots.get_mut("garden_1").unwrap(),
            &mut gs,
            IdentifiableId::CuberryFruit,
            wt
        ));
        assert_eq!(gm.land_plots["garden_1"].attached_id, SpawnResourceId::CuberryTreeDlx);
    }

    #[test]
    fn toy_director_unlocks_upgraded_toys_at_corporate_rank_10() {
        use sr_ids::ProgressType;
        use sr_math::Vec3;
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 1000;
        let mut pw = PhysicsWorld::new();
        let toys = ToyDirector::default();

        assert!(!toys.purchaseable_toys(&gm).contains(&IdentifiableId::PuzzleCubeToy));
        assert!(toys.buy_toy(&mut gm, &mut pw, IdentifiableId::PuzzleCubeToy, Vec3::ZERO).is_err());

        gm.progress.set_progress(ProgressType::CorporatePartner, 10);
        assert!(toys.purchaseable_toys(&gm).contains(&IdentifiableId::PuzzleCubeToy));
        assert!(toys.buy_toy(&mut gm, &mut pw, IdentifiableId::PuzzleCubeToy, Vec3::ZERO).is_ok());
    }

    #[test]
    fn economy_director_pricing_saturation_recovery_and_shutdown() {
        use sr_data::DataRegistry;
        use sr_math::Randoms;
        use sr_model::GameModel;

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut rng = Randoms::from_seed(42);

        let mut econ1 = EconomyDirector::default();
        econ1.init_model(&mut gm.world, &data, &mode, &mut rng);
        let day0_pink = econ1.get_curr_value(IdentifiableId::PinkPlort).unwrap();
        assert!(day0_pink > 0);

        // Midnight shutdown window 00:00 .. 00:05
        assert!(econ1.is_market_shutdown(&mode, 0.02));
        assert!(!econ1.is_market_shutdown(&mode, 0.25));

        // Selling 100 Pink Plorts raises saturation, lowering Day 1 price
        econ1.register_sold(&mut gm.world, IdentifiableId::PinkPlort, 100);
        let sat_after_sale = gm.world.market_saturation[&IdentifiableId::PinkPlort];
        gm.world.world_time = 86400.0;
        assert!(econ1.step(&mut gm.world, &mode, 1));
        let sat_after_midnight = gm.world.market_saturation[&IdentifiableId::PinkPlort];
        assert!((sat_after_midnight - sat_after_sale * 0.75).abs() < 1e-3);
        assert!(econ1.get_change_in_value(IdentifiableId::PinkPlort).is_some());
    }

    #[test]
    fn plort_market_deposit_mochi_bonus_market_link_and_rush_listings() {
        use sr_data::DataRegistry;
        use sr_ids::{ProgressType, RegionSetId};
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::TimeLimitV2);
        let mut gm = GameModel::new(GameMode::TimeLimitV2);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(7);

        let mut rt = EconomyRuntime::default();
        rt.ensure_initialized(&mut gm, &data, &mode, &mut rng);
        // Rush mode disables dynamic market & shutdown and uses 1.5x base value
        assert!(!rt.market.is_market_shutdown(&mode, 0.01));
        assert_eq!(rt.market.get_curr_value(IdentifiableId::PinkPlort), Some(11)); // round(7 * 1.5) = 11

        rt.deposit_zones.push(MarketDepositZone {
            id: "market_link_1".into(),
            pos: Vec3::new(10.0, 0.0, 0.0),
            radius: 2.0,
            is_market_link: true,
        });
        spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::PinkPlort,
            RegionSetId::Home,
            Vec3::new(10.5, 0.0, 0.0),
            Vec3::ZERO,
        );
        let ev = rt.step(&mut gm, &mut pw, &data, &mode, 1, 9.0, &mut rng);
        assert_eq!(ev.market_deposits.len(), 1);
        assert_eq!(gm.player.currency, 1011);

        // Deposit 24 more to hit Rush Mode 25-plort bonus
        gm.progress.set_progress(ProgressType::MochiRewards, 1);
        let resp = deposit_plorts(
            &rt.market,
            &mut gm,
            &mode,
            IdentifiableId::PinkPlort,
            24,
            9.0,
            1.0,
            false,
            &mut rng,
        );
        assert_eq!(resp.deposits, 24);
        assert!(resp.currency >= 24 * 11);
        let listings = build_market_listings(&rt.market, &gm, &mode);
        let pink = listings
            .iter()
            .find(|l| l.id == IdentifiableId::PinkPlort)
            .unwrap();
        assert_eq!(pink.sold_count, 25);
        assert!(pink.rush_bonus_complete);
        assert_eq!(pink.trend, PriceTrend::BonusComplete);
    }

    #[test]
    fn seven_zee_28_ranks_upgrades_and_blueprints() {
        use sr_ids::{GadgetId, PediaId, ProgressType};
        use sr_model::GameModel;

        let mut gm = GameModel::new(GameMode::Classic);
        assert_eq!(
            purchase_next_seven_zee_rank(&mut gm, true),
            Err(SevenZeeError::Locked)
        );
        unlock_seven_zee(&mut gm);
        gm.player.currency = 5_000_000;

        for expected_rank in 1..=CORPORATE_PARTNER_MAX_RANK {
            let res = purchase_next_seven_zee_rank(&mut gm, true).unwrap();
            assert_eq!(res.new_rank, expected_rank);
        }
        assert_eq!(
            purchase_next_seven_zee_rank(&mut gm, true),
            Err(SevenZeeError::MaxRankReached)
        );
        assert_eq!(gm.progress.get_progress(ProgressType::CorporatePartner), 28);
        assert!(gm.pedia.unlocked.contains(&PediaId::Chroma));
        assert!(gm.pedia.unlocked.contains(&PediaId::SlimeToys));
        assert!(gm.player.upgrades.contains(&PlayerUpgrade::GoldenSureshot));
        assert!(gm.gadgets.blueprints.contains(&GadgetId::MarketLink));
        assert!(gm.gadgets.blueprints.contains(&GadgetId::TeleporterGold));
        assert_eq!(gm.player.max_ammo, 100);
        assert_eq!(gm.player.max_health, 350);
    }

    #[test]
    fn range_exchange_daily_offer_early_bonus_and_ogden_progress() {
        use sr_ids::{ProgressType, RegionSetId};
        use sr_math::{Randoms, Vec3};
        use sr_model::{GameModel, NonIdentReward, OfferType};
        use sr_physics::PhysicsWorld;

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(17);
        let ex = ExchangeDirector::default();
        ExchangeDirector::init_model(&mut gm.world, &mode);

        // Day 2 at 12:05 PM triggers pending rancher ("thora" first)
        gm.world.world_time = 86400.0 + (DAILY_OFFER_HOUR as f64) * 3600.0 + 1.0;
        let ev = ex.step(&mut gm, &mode, 2, &mut rng);
        assert_eq!(ev.created_pending_ranchers, vec!["thora".to_string()]);

        let offer = ex
            .select_daily_offer(&mut gm, &mode, 2, "thora", true, &mut rng)
            .unwrap();
        assert_eq!(gm.progress.get_progress(ProgressType::ExchangeThora), 1);

        // Fulfill every requested item immediately -> triggers early bonus + rewards
        let mut completed = false;
        for req in &offer.requests {
            for _ in 0..req.count {
                let out = ex.try_accept_item(
                    &mut gm,
                    &mut pw,
                    OfferType::General,
                    req.id,
                    Vec3::ZERO,
                    RegionSetId::Home,
                    &mut rng,
                );
                assert!(out.accepted);
                if out.completed {
                    completed = true;
                    assert!(out.early_bonus);
                    assert!(out.newbucks_awarded >= 100);
                }
            }
        }
        assert!(completed);
        assert!(!gm.world.curr_offers.contains_key(&OfferType::General));

        // Ogden special progress offer awards OgdenMix -> OgdenRewards = 1 + OgdenRecur unlocked
        assert!(ex.maybe_start_progress_offer(&mut gm, OfferType::Ogden));
        let og = gm.world.curr_offers[&OfferType::Ogden].clone();
        assert_eq!(og.rewards[0].spec_reward, NonIdentReward::OgdenMix);
        for _ in 0..og.requests[0].count {
            ex.try_accept_item(
                &mut gm,
                &mut pw,
                OfferType::Ogden,
                og.requests[0].id,
                Vec3::ZERO,
                RegionSetId::Home,
                &mut rng,
            );
        }
        assert_eq!(gm.progress.get_progress(ProgressType::OgdenRewards), 1);
    }
}
