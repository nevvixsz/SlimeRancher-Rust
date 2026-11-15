//! Player gameplay systems: `PlayerState`, `Ammo`, `WeaponVacuum`, and actor lifecycle rules.

pub mod actors;
pub mod ammo;
pub mod economy;
pub mod player;
pub mod ranch;
pub mod science;
pub mod story;
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
pub use science::{
    bait_gordo_snare, builtin_blueprint_unlock_rules, builtin_gadget_definitions,
    check_gordo_snares, deposit_warp_depot, drone_battery_fraction, drone_has_battery,
    drone_program_count, feed_snared_gordo, recharge_drone_battery, set_drone_program,
    spec_for_extractor, step_utility_gadgets, trigger_chicken_cloner, use_teleporter,
    withdraw_warp_depot, BlueprintUnlockRule, DroneAiSystem, DroneDestKind, DroneNetworkScope,
    DronePhase, DroneSourceKind, DroneStepEvents, DroneTargetCategory, ExtractorProduceEntry,
    ExtractorSpec, ExtractorStepEvents, ExtractorSystem, GadgetDirector, GadgetDirectorStepEvents,
    GadgetError, ScienceRuntime, ScienceStepEvents, UtilityGadgetEvents, DRONE_AMMO_CAPACITY,
    DRONE_BATTERY_DURATION_HOURS, DRONE_BATTERY_DURATION_SECS, DRONE_FASTFORWARD_CYCLE_SECS,
    DRONE_FLY_SPEED, HYDRO_TURRET_RANGE, MED_STATION_HEAL_PER_SEC, REFINERY_MAX,
    SNARED_GORDO_TARGET_COUNT, TAMING_BELL_RANGE, WARP_DEPOT_MAX_STACK,
};
pub use story::{
    DelayedTrackerRule, MailDirector, MailReadOutcome, MailStepEvents, ProgressDirector,
    ProgressStepEvents, CASEY_MAIL_FINAL, CASEY_MAIL_PREFIX, HOBSON_MAIL_KEY, MOCHI_MAIL_KEY,
    OGDEN_MAIL_KEY, PARTNER_MAIL_KEY, PARTNER_UNLOCK_TIME, SECRET_STYLE_MAIL_DELAY,
    SECRET_STYLE_MAIL_KEY, VIKTOR_MAIL_KEY,
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

    #[test]
    fn slime_science_refinery_blueprints_fabricator_and_paired_teleporters() {
        use sr_ids::{GadgetId, RegionSetId, ZoneId};
        use sr_math::Vec3;
        use sr_model::GameModel;
        use sr_physics::PlayerMotor;

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut motor = PlayerMotor::default();
        gm.player.currency = 5000;

        let mut rt = ScienceRuntime::default();
        rt.director.init_model(&mut gm, &mode);
        rt.director.ensure_site(&mut gm, "site_ranch", ZoneId::Ranch);
        rt.director.ensure_site(&mut gm, "site_desert", ZoneId::Desert);
        rt.extractors.register_site_pos("site_ranch", Vec3::new(0.0, 0.0, 0.0));
        rt.extractors.register_site_pos("site_desert", Vec3::new(100.0, 0.0, 100.0));

        // Refinery max 999 & rejects non-craft/non-plort food
        assert_eq!(GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::CarrotVeggie, 5), 0);
        assert_eq!(GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::PinkPlort, 1500), 999);
        assert_eq!(GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::StrangeDiamondCraft, 5), 5);

        // Buy TeleporterPink blueprint and fabricate -> buy_in_pairs yields 2 teleporters
        rt.director.buy_blueprint(&mut gm, &mode, GadgetId::TeleporterPink).unwrap();
        assert_eq!(rt.director.fabricate_gadget(&mut gm, GadgetId::TeleporterPink).unwrap(), 2);

        rt.director.place_gadget(&mut gm, "site_ranch", GadgetId::TeleporterPink, 0.0).unwrap();
        rt.director.place_gadget(&mut gm, "site_desert", GadgetId::TeleporterPink, 0.0).unwrap();

        let tp = use_teleporter(
            &mut gm,
            &mut motor,
            &rt.extractors.site_positions,
            &rt.director.site_zones,
            "site_ranch",
        )
        .unwrap();
        assert_eq!(tp.0, "site_desert");
        assert_eq!(tp.2, RegionSetId::Desert);
        assert!((gm.player.position - Vec3::new(100.0, 1.2, 100.0)).length() < 0.01);

        // Removing one teleporter destroys its linked partner
        rt.director.remove_gadget(&mut gm, "site_ranch").unwrap();
        assert!(gm.gadget_sites["site_desert"].attached.is_none());
    }

    #[test]
    fn extractors_zone_drops_interaction_and_cycle_expiry() {
        use sr_data::DataRegistry;
        use sr_ids::{GadgetId, ZoneId};
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(13);
        let econ = EconomyDirector::default();
        let ranch = RanchRuntime::default();

        let mut rt = ScienceRuntime::default();
        rt.director.init_model(&mut gm, &mode);
        rt.director.ensure_site(&mut gm, "site_quarry", ZoneId::Quarry);
        rt.extractors.register_site_pos("site_quarry", Vec3::new(10.0, 0.0, 10.0));

        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::PinkPlort, 20);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::RockPlort, 20);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::RadPlort, 10);
        assert_eq!(rt.director.fabricate_gadget(&mut gm, GadgetId::ExtractorDrillNovice).unwrap(), 1);
        rt.director.place_gadget(&mut gm, "site_quarry", GadgetId::ExtractorDrillNovice, 0.0).unwrap();

        // Initialize extractor cycle (12h)
        let _ = rt.step(&mut gm, &mut pw, &data, &mode, &ranch, &econ, false, 9.0, 0.1, &mut rng);
        assert!(!rt.extractors.is_ready(&gm, "site_quarry"));

        // Advance past 12h -> ready & interact
        gm.world.world_time += 12.1 * 3600.0;
        let ev_ready = rt.step(&mut gm, &mut pw, &data, &mode, &ranch, &econ, false, 21.1, 0.1, &mut rng);
        assert!(ev_ready.extractors.ready_sites.contains(&"site_quarry".to_string()));
        assert!(rt.extractors.interact_extractor(&mut gm, "site_quarry", &mut rng));

        // Step ejection ticks until all items spawn and Novice Drill (1 cycle) expires
        let mut spawned_total = 0usize;
        let mut expired = false;
        for _ in 0..20 {
            gm.world.world_time += 20.0;
            let ev = rt.step(&mut gm, &mut pw, &data, &mode, &ranch, &econ, false, 21.2, 0.1, &mut rng);
            spawned_total += ev.extractors.spawned_items.len();
            if ev.extractors.expired_sites.contains(&"site_quarry".to_string()) {
                expired = true;
                break;
            }
        }
        assert!((4..=8).contains(&spawned_total));
        assert!(expired);
        assert!(gm.gadget_sites["site_quarry"].attached.is_none());
    }

    #[test]
    fn warp_depot_sync_gordo_snare_and_chicken_cloner() {
        use sr_data::DataRegistry;
        use sr_ids::{GadgetId, RegionSetId, ZoneId};
        use sr_math::{Randoms, Vec3};
        use sr_model::{GadgetKind, GameModel};
        use sr_physics::PhysicsWorld;

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 10_000;
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(19);

        let mut rt = ScienceRuntime::default();
        rt.director.init_model(&mut gm, &mode);
        rt.director.ensure_site(&mut gm, "wd_a", ZoneId::Ranch);
        rt.director.ensure_site(&mut gm, "wd_b", ZoneId::Moss);
        rt.director.ensure_site(&mut gm, "snare_1", ZoneId::Ranch);

        // Paired WarpDepots share inventory across zones
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::PinkPlort, 30);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::DeepBrineCraft, 10);
        rt.director.buy_blueprint(&mut gm, &mode, GadgetId::WarpDepotPink).unwrap();
        rt.director.fabricate_gadget(&mut gm, GadgetId::WarpDepotPink).unwrap();
        rt.director.place_gadget(&mut gm, "wd_a", GadgetId::WarpDepotPink, 0.0).unwrap();
        rt.director.place_gadget(&mut gm, "wd_b", GadgetId::WarpDepotPink, 0.0).unwrap();

        assert_eq!(deposit_warp_depot(&mut gm, "wd_a", IdentifiableId::GoldPlort, 5), 5);
        assert_eq!(withdraw_warp_depot(&mut gm, "wd_b", 3), Some((IdentifiableId::GoldPlort, 3)));
        assert_eq!(withdraw_warp_depot(&mut gm, "wd_a", 5), Some((IdentifiableId::GoldPlort, 2)));

        // Master Gordo Snare baited with Rooster -> guaranteed Hunter Gordo at 05:00
        gm.gadgets.blueprints.push(GadgetId::GordoSnareMaster);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::HunterPlort, 15);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::RoyalJellyCraft, 5);
        rt.director.fabricate_gadget(&mut gm, GadgetId::GordoSnareMaster).unwrap();
        rt.director.place_gadget(&mut gm, "snare_1", GadgetId::GordoSnareMaster, 0.0).unwrap();
        assert!(bait_gordo_snare(&mut gm, "snare_1", IdentifiableId::Rooster));
        let snared = check_gordo_snares(&mut gm, &data, &mut rng);
        assert_eq!(snared, vec![("snare_1".to_string(), IdentifiableId::HunterGordo)]);

        let GadgetKind::Snare(ref sn) = gm.gadget_sites["snare_1"].attached.as_ref().unwrap().kind else {
            panic!("expected snare");
        };
        assert_eq!(sn.gordo_type_id, IdentifiableId::HunterGordo);

        // Feed snared Hunter Gordo 25 Roosters (favorite = 2x each -> 50) to pop it
        for i in 0..25 {
            let (ate, popped) = feed_snared_gordo(
                &mut gm,
                &mut pw,
                &data,
                "snare_1",
                IdentifiableId::Rooster,
                Vec3::ZERO,
                RegionSetId::Home,
                &mut rng,
            );
            assert!(ate);
            assert_eq!(popped, i == 24);
        }

        // Chicken cloner accepts meat
        let cid = spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::Hen,
            RegionSetId::Home,
            Vec3::ZERO,
            Vec3::ZERO,
        );
        let cloned = trigger_chicken_cloner(&mut gm, &mut pw, cid, Vec3::ZERO, RegionSetId::Home, &mut rng);
        assert!(cloned.is_some());
    }

    #[test]
    fn drone_battery_programs_ai_delivery_to_market_and_fast_forward() {
        use sr_data::DataRegistry;
        use sr_ids::{GadgetId, LandPlotId, LandPlotUpgrade, RegionSetId, ZoneId};
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 10_000;
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(29);

        let mut econ = EconomyDirector::default();
        econ.init_model(&mut gm.world, &data, &mode, &mut rng);

        let mut ranch = RanchRuntime::default();
        RanchDirector::ensure_plot(&mut gm, "corral_1", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "corral_1", LandPlotId::Corral).unwrap();
        RanchDirector::buy_upgrade(&mut gm, "corral_1", LandPlotUpgrade::PlortCollector).unwrap();
        ranch.register_bounds(PlotBounds {
            plot_id: "corral_1".into(),
            center: Vec3::ZERO,
            half_extents: Vec3::new(5.0, 3.0, 5.0),
            region: RegionSetId::Home,
        });

        // Drop a Pink Plort inside corral and collect it into PlortCollector
        spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::PinkPlort,
            RegionSetId::Home,
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::ZERO,
        );
        let wt = gm.world.world_time + 1.5 * 3600.0;
        gm.world.world_time = wt;
        ranch.step(&mut gm, &mut pw, wt, 3600.0, 0.1, &mut rng);

        let mut rt = ScienceRuntime::default();
        rt.director.init_model(&mut gm, &mode);
        rt.director.ensure_site(&mut gm, "drone_site_1", ZoneId::Ranch);
        rt.extractors.register_site_pos("drone_site_1", Vec3::new(8.0, 0.0, 0.0));
        gm.gadgets.blueprints.push(GadgetId::DroneAdvanced);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::QuantumPlort, 20);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::ManifoldCubeCraft, 5);
        GadgetDirector::add_to_refinery(&mut gm, IdentifiableId::RoyalJellyCraft, 5);
        rt.director.fabricate_gadget(&mut gm, GadgetId::DroneAdvanced).unwrap();
        rt.director.place_gadget(&mut gm, "drone_site_1", GadgetId::DroneAdvanced, 0.0).unwrap();

        assert!(set_drone_program(
            &mut gm,
            "drone_site_1",
            0,
            IdentifiableId::PinkPlort,
            DroneSourceKind::PlortCollector,
            DroneDestKind::PlortMarket,
        ));
        assert!(recharge_drone_battery(&mut gm, "drone_site_1", IdentifiableId::WaterLiquid));

        let cash_before = gm.player.currency;
        // Tick 1 gathers from PlortCollector; Tick 2 delivers to PlortMarket
        let ev1 = rt.step(&mut gm, &mut pw, &data, &mode, &ranch, &econ, false, 10.5, 0.1, &mut rng);
        assert_eq!(ev1.drones.gathered.len(), 1);
        let ev2 = rt.step(&mut gm, &mut pw, &data, &mode, &ranch, &econ, false, 10.6, 0.1, &mut rng);
        assert_eq!(ev2.drones.delivered.len(), 1);
        assert!(gm.player.currency > cash_before);

        // Fast-forward drone over 6h with ground plorts in corral
        spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::RockPlort,
            RegionSetId::Home,
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::ZERO,
        );
        set_drone_program(
            &mut gm,
            "drone_site_1",
            1,
            IdentifiableId::RockPlort,
            DroneSourceKind::Corral,
            DroneDestKind::Refinery,
        );
        let ff_ev = rt.drones.fast_forward_drones(
            &mut gm,
            &mut pw,
            &ranch,
            &econ,
            &mode,
            16.5,
            wt,
            wt + 6.0 * 3600.0,
            &mut rng,
        );
        assert_eq!(ff_ev.delivered.len(), 1);
        assert_eq!(GadgetDirector::get_refinery_count(&gm, IdentifiableId::RockPlort), 1);
    }
}
