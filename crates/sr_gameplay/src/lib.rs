//! Player gameplay systems: `PlayerState`, `Ammo`, `WeaponVacuum`, and actor lifecycle rules.

pub mod actors;
pub mod ammo;
pub mod dlc;
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
pub use dlc::*;
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
    AchievementId, AchievementTier, AchievementsDirector, DelayedTrackerRule, EndGamePlortEntry,
    EndGameSummary, JournalEntryDef, MailDirector, MailReadOutcome, MailStepEvents, PediaDirector,
    ProgressDirector, ProgressStepEvents, StoryRuntime, StoryStepEvents, TutorialDirector,
    CASEY_MAIL_FINAL, CASEY_MAIL_PREFIX, HOBSON_MAIL_KEY, MOCHI_MAIL_KEY, OGDEN_MAIL_KEY,
    PARTNER_MAIL_KEY, PARTNER_UNLOCK_TIME, PEDIA_CATEGORY_HEADERS, PEDIA_HIDDEN_ENTRIES,
    PEDIA_INIT_UNLOCKED, SECRET_STYLE_MAIL_DELAY, SECRET_STYLE_MAIL_KEY, VIKTOR_MAIL_KEY,
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

    #[test]
    fn story_progress_casey_chain_and_hobson_end() {
        use sr_ids::{GadgetId, PlayerUpgrade, ProgressType};
        use sr_model::{GameModel, MailType, ProgressTrackerId};

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut rt = StoryRuntime::default();
        rt.init_model(&mut gm, &mode);

        // Welcome mail present at start
        assert!(gm
            .mail
            .all_mail
            .iter()
            .any(|m| m.mail_type == MailType::Personal && m.key == "welcome"));

        // casey_1 unlocks 3h after start with SLIME_DOORS >= 0
        let ev0 = rt.step(&mut gm, &mode);
        assert!(ev0.progress.sent_mail_keys.is_empty());
        gm.world.world_time += 3.1 * 3600.0;
        let ev1 = rt.step(&mut gm, &mode);
        assert!(ev1.progress.sent_mail_keys.contains(&"casey_1".to_string()));

        // Reading casey_1 queues wistful music & arms casey_2 (48h)
        let r1 = MailDirector::select_and_read_mail(
            &mut gm,
            &mode,
            &mut rt.progress,
            MailType::Personal,
            "casey_1",
        );
        assert!(r1.was_unread);
        assert!(r1.wistful_music_queued);
        assert_eq!(rt.on_returned_to_ranch(), (false, true));

        gm.world.world_time += 48.1 * 3600.0;
        let ev2 = rt.step(&mut gm, &mode);
        assert!(ev2.progress.sent_mail_keys.contains(&"casey_2".to_string()));

        // Fast-forward story flags up to HOBSON_END + reading casey_10 -> casey_11 -> hobson_1
        let mut dummy_ev = ProgressStepEvents::default();
        rt.progress.set_progress(&mut gm, &mode, ProgressType::SlimeDoors, 5, &mut dummy_ev);
        rt.progress.set_progress(&mut gm, &mode, ProgressType::PlortDoor, 1, &mut dummy_ev);
        rt.progress.set_progress(&mut gm, &mode, ProgressType::UnlockDesert, 1, &mut dummy_ev);
        rt.progress.set_progress(&mut gm, &mode, ProgressType::HobsonEnd, 1, &mut dummy_ev);

        for k in ["casey_2", "casey_3", "casey_4", "casey_5", "casey_6", "casey_7", "casey_8", "casey_9", "casey_10"] {
            gm.mail.send_mail(MailType::Personal, k);
            gm.mail.mark_read(MailType::Personal, k);
        }
        rt.progress.unlocked_trackers.insert(ProgressTrackerId::Casey1);
        rt.progress.unlocked_trackers.insert(ProgressTrackerId::Casey2);
        for t in [
            ProgressTrackerId::Casey3,
            ProgressTrackerId::Casey4,
            ProgressTrackerId::Casey5,
            ProgressTrackerId::Casey6,
            ProgressTrackerId::Casey7,
            ProgressTrackerId::Casey8,
            ProgressTrackerId::Casey9,
            ProgressTrackerId::Casey10,
        ] {
            rt.progress.unlocked_trackers.insert(t);
        }

        // Arm casey_11 (6h delay)
        let _ = rt.step(&mut gm, &mode);
        gm.world.world_time += 6.1 * 3600.0;
        let ev11 = rt.step(&mut gm, &mode);
        assert!(ev11.progress.sent_mail_keys.contains(&"casey_11".to_string()));

        // Reading casey_11 queues end credits & arms hobson_1 (0.167h)
        let r11 = MailDirector::select_and_read_mail(
            &mut gm,
            &mode,
            &mut rt.progress,
            MailType::Personal,
            "casey_11",
        );
        assert!(r11.credits_queued);
        assert_eq!(rt.on_returned_to_ranch(), (true, false));

        gm.world.world_time += 0.2 * 3600.0;
        let ev_h = rt.step(&mut gm, &mode);
        assert!(ev_h.progress.sent_mail_keys.contains(&"hobson_1".to_string()));

        // Reading hobson_1 unlocks HOBSON_END_UNLOCK
        MailDirector::select_and_read_mail(
            &mut gm,
            &mode,
            &mut rt.progress,
            MailType::Personal,
            "hobson_1",
        );
        assert!(gm.progress.has_progress(ProgressType::HobsonEndUnlock));

        // 7Zee CorporatePartner progress unlocks MarketLink (11), Ammo4 (20), TeleporterGold (28)
        rt.progress.set_progress(&mut gm, &mode, ProgressType::CorporatePartner, 28, &mut dummy_ev);
        assert!(gm.player.upgrades.contains(&PlayerUpgrade::Ammo4));
        assert!(gm.player.upgrades.contains(&PlayerUpgrade::GoldenSureshot));
        assert!(gm.gadgets.blueprints.contains(&GadgetId::MarketLink));
        assert!(gm.gadgets.blueprints.contains(&GadgetId::TeleporterGold));
    }

    #[test]
    fn story_mail_partner_secret_style_and_mission_invites() {
        use sr_ids::{PediaId, PlayerUpgrade, ProgressType};
        use sr_model::{GameModel, MailType, TutorialId};

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut rt = StoryRuntime::default();
        rt.init_model(&mut gm, &mode);
        rt.mail.secret_style_dlc_enabled = true;

        // First step schedules secret_styles (+600s); advancing past PARTNER_UNLOCK_TIME sends both
        let _ = rt.step(&mut gm, &mode);
        gm.world.world_time = PARTNER_UNLOCK_TIME + 1.0;
        let ev = rt.step(&mut gm, &mode);
        assert!(ev.mail.sent_mail.iter().any(|(_, k)| k == PARTNER_MAIL_KEY));
        assert!(ev.mail.sent_mail.iter().any(|(_, k)| k == SECRET_STYLE_MAIL_KEY));

        let read_p = MailDirector::select_and_read_mail(
            &mut gm,
            &mode,
            &mut rt.progress,
            MailType::Personal,
            PARTNER_MAIL_KEY,
        );
        assert_eq!(read_p.unlocked_pedia, Some(PediaId::Partner));
        assert!(gm.progress.has_progress(ProgressType::CorporatePartnerUnlock));

        // Mission unlock conditions: Lab + Ruins + TreasureCracker2 -> viktor_invite
        let mut pev = ProgressStepEvents::default();
        rt.progress.set_progress(&mut gm, &mode, ProgressType::UnlockLab, 1, &mut pev);
        rt.progress.set_progress(&mut gm, &mode, ProgressType::UnlockRuins, 1, &mut pev);
        PlayerController::grant_upgrade(&mut gm.player, PlayerUpgrade::TreasureCracker2, 100.0);
        let ev_v = rt.step(&mut gm, &mode);
        assert!(gm.progress.has_progress(ProgressType::UnlockViktorMissions));
        assert!(ev_v.progress.sent_mail_keys.contains(&VIKTOR_MAIL_KEY.to_string()));

        let read_v = MailDirector::select_and_read_mail(
            &mut gm,
            &mode,
            &mut rt.progress,
            MailType::Personal,
            VIKTOR_MAIL_KEY,
        );
        assert_eq!(read_v.triggered_tutorial, Some(TutorialId::EnterZoneViktorLab));
    }

    #[test]
    fn story_journal_slimepedia_and_tutorial_dependencies() {
        use sr_ids::{PediaId, ProgressType};
        use sr_model::{GameModel, TutorialId};

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut rt = StoryRuntime::default();
        rt.init_model(&mut gm, &mode);

        // Hobson End journal sets HobsonEnd progress
        let (activated, _) = rt.pedia.activate_journal_entry(
            &mut gm,
            &mode,
            &mut rt.progress,
            &JournalEntryDef::hobson_end(),
        );
        assert!(activated);
        assert!(gm.progress.has_progress(ProgressType::HobsonEnd));

        // Slimepedia unlocks by IdentifiableId and tracks completion excluding HIDDEN_ENTRIES
        assert_eq!(
            rt.pedia.maybe_show_popup_for_ident(&mut gm, &mode, IdentifiableId::MosaicSlime),
            Some(PediaId::MosaicSlime)
        );
        assert!(!PediaDirector::check_completed_slimepedia(&mut gm));

        for &pid in PediaId::ALL {
            if !PEDIA_CATEGORY_HEADERS.contains(&pid) && !PEDIA_HIDDEN_ENTRIES.contains(&pid) {
                PediaDirector::unlock_without_popup(&mut gm, pid);
            }
        }
        assert!(PediaDirector::check_completed_slimepedia(&mut gm));
        assert_eq!(gm.game_achieves.counts["COMPLETED_SLIMEPEDIA"], 1);

        // Tutorial dependencies: Market waits for Plort to complete
        rt.tutorials.set_in_market_area(&mut gm, &mode, true);
        assert!(!gm.tutorials.popup_queue.contains(&TutorialId::Market));
        rt.tutorials.on_vac(&mut gm, &mode, IdentifiableId::PinkPlort);
        assert!(gm.tutorials.popup_queue.contains(&TutorialId::Plort));
        let unblocked = rt.tutorials.complete_tutorial(&mut gm, &mode, TutorialId::Plort);
        assert_eq!(unblocked, vec![TutorialId::Market]);
        assert!(gm.tutorials.popup_queue.contains(&TutorialId::Market));
    }

    #[test]
    fn achievements_and_end_game_summary_including_rush_v2() {
        use sr_data::DataRegistry;
        use sr_ids::LandPlotId;
        use sr_math::Randoms;
        use sr_model::GameModel;

        let data = DataRegistry::builtin();
        let classic_mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        gm.player.currency = 100_000;

        // Ranch storage achievement
        RanchDirector::ensure_plot(&mut gm, "c", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "c", LandPlotId::Corral).unwrap();
        for u in [
            sr_ids::LandPlotUpgrade::Walls,
            sr_ids::LandPlotUpgrade::MusicBox,
            sr_ids::LandPlotUpgrade::AirNet,
            sr_ids::LandPlotUpgrade::SolarShield,
            sr_ids::LandPlotUpgrade::PlortCollector,
            sr_ids::LandPlotUpgrade::Feeder,
        ] {
            gm.land_plots.get_mut("c").unwrap().upgrades.insert(u);
        }
        RanchDirector::ensure_plot(&mut gm, "k", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "k", LandPlotId::Coop).unwrap();
        for u in [
            sr_ids::LandPlotUpgrade::Walls,
            sr_ids::LandPlotUpgrade::Feeder,
            sr_ids::LandPlotUpgrade::Vitamizer,
            sr_ids::LandPlotUpgrade::DeluxeCoop,
        ] {
            gm.land_plots.get_mut("k").unwrap().upgrades.insert(u);
        }
        RanchDirector::ensure_plot(&mut gm, "s", LandPlotId::Empty);
        RanchDirector::buy_plot(&mut gm, "s", LandPlotId::Silo).unwrap();
        for u in [
            sr_ids::LandPlotUpgrade::Storage2,
            sr_ids::LandPlotUpgrade::Storage3,
            sr_ids::LandPlotUpgrade::Storage4,
        ] {
            gm.land_plots.get_mut("s").unwrap().upgrades.insert(u);
        }

        AchievementsDirector::add_int_stat(&mut gm, "PLORTS_SOLD", 5000);
        AchievementsDirector::add_int_stat(&mut gm, "CURRENCY", 100_000);
        AchievementsDirector::add_game_int_stat(&mut gm, "FABRICATED_GADGETS", 35);
        let earned = AchievementsDirector::evaluate_all(&mut gm);
        assert!(earned.contains(&AchievementId::SellPlortsA));
        assert!(earned.contains(&AchievementId::SellPlortsE));
        assert!(earned.contains(&AchievementId::EarnCurrencyC));
        assert!(earned.contains(&AchievementId::RanchUpgradedStorage));
        assert!(earned.contains(&AchievementId::FabricateGadgetsB));
        assert!(AchievementsDirector::calculate_game_progress(&gm) > 0.0);

        // Rush Mode V2 EndGameV2UI score multiplier: 25 QuantumPlort (+8%) + 25 PinkPlort (+5%) = 1.13x
        let rush_mode = GameModeSettings::for_mode(GameMode::TimeLimitV2);
        let mut rush_gm = GameModel::new(GameMode::TimeLimitV2);
        rush_gm.player.currency = 10_000;
        let mut econ = EconomyDirector::default();
        let mut rng = Randoms::from_seed(5);
        econ.init_model(&mut rush_gm.world, &data, &rush_mode, &mut rng);

        AchievementsDirector::add_game_plort_sold(&mut rush_gm, IdentifiableId::QuantumPlort, 25);
        AchievementsDirector::add_game_plort_sold(&mut rush_gm, IdentifiableId::PinkPlort, 25);
        let summary = AchievementsDirector::build_end_game_summary(&mut rush_gm, &econ);
        assert!((summary.rush_score_multiplier - 1.13).abs() < 1e-4);
        assert_eq!(summary.rush_final_score, 11300);

        let rush_earned = AchievementsDirector::evaluate_all(&mut rush_gm);
        assert!(rush_earned.contains(&AchievementId::TimeLimitCurrencyA));
        assert!(!rush_earned.contains(&AchievementId::SellPlortsA)); // blocked in TimeLimitV2
        let _ = classic_mode;
    }

    #[test]
    fn step9_ogdens_wilds_kookadoba_nodes_and_crack() {
        use sr_data::GameModeSettings;
        use sr_ids::{GameMode, IdentifiableId, ProgressType, RegionSetId};
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(91);
        let mut prog = ProgressDirector::default();
        let mut wilds = WildsSystem::default();

        assert!(!WildsSystem::can_enter_wilds(&gm, &mode));
        gm.progress.set_unique_progress(ProgressType::UnlockMoss);
        gm.progress
            .set_unique_progress(ProgressType::UnlockOvergrowth);
        assert!(WildsSystem::can_enter_wilds(&gm, &mode));

        let _entry_ev = wilds.enter_wilds(&mut gm, &mode, &mut prog).unwrap();
        assert!(gm.progress.has_progress(ProgressType::EnterZoneOgdenRanch));
        assert!(gm.progress.has_progress(ProgressType::UnlockWilds));

        wilds.register_node("node_a", Vec3::new(10.0, 0.0, 10.0));
        wilds.register_node("node_b", Vec3::new(15.0, 0.0, 15.0));

        // First step initializes next_spawn_time
        wilds.step(&mut gm, &mut pw, &mut rng);
        gm.world.world_time += KOOKADOBA_SPAWN_DELAY_SECS + 1.0;
        let ev = wilds.step(&mut gm, &mut pw, &mut rng);
        assert_eq!(ev.grown_nodes.len(), 1);
        let (node_id, fruit_id) = &ev.grown_nodes[0];
        assert_eq!(gm.actors[fruit_id].ident, IdentifiableId::KookadobaFruit);
        assert!(wilds.harvest_node(&mut gm, &mut pw, node_id));
        assert!(!gm.actors.contains_key(fruit_id));

        // Cracking a KookadobaBall spawns 3 KookadobaFruit actors
        let ball_id = spawn_physical_actor(
            &mut gm,
            &mut pw,
            IdentifiableId::KookadobaBall,
            RegionSetId::Home,
            Vec3::ZERO,
            Vec3::ZERO,
        );
        let fruits = WildsSystem::crack_kookadoba_ball(&mut gm, &mut pw, ball_id, 3, &mut rng);
        assert_eq!(fruits.len(), 3);
        assert!(!gm.actors.contains_key(&ball_id));
    }

    #[test]
    fn step9_mochi_nimble_valley_generators_checkpoints_and_plorts() {
        use sr_data::GameModeSettings;
        use sr_ids::{AmmoMode, GameMode, IdentifiableId, PediaId, ProgressType};
        use sr_math::{Randoms, Vec3};
        use sr_model::{GameModel, QuicksilverGeneratorState};
        use sr_physics::PhysicsWorld;

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(92);
        let mut ammo = AmmoController::default();
        let mut prog = ProgressDirector::default();
        let mut pedia = PediaDirector::default();
        let mut valley = ValleySystem::default();

        gm.progress.set_unique_progress(ProgressType::UnlockQuarry);
        gm.progress.set_unique_progress(ProgressType::UnlockGrotto);
        let _entry = valley
            .enter_valley(&mut gm, &mode, &mut prog, &mut ammo)
            .unwrap();
        assert!(gm.progress.has_progress(ProgressType::UnlockValley));
        assert_eq!(gm.player.ammo_mode, AmmoMode::NimbleValley);

        valley.register_generator(
            &mut gm,
            GeneratorConfig {
                id: "gen_1".to_string(),
                countdown_minutes: 0.05,
                active_minutes: 10.0,
                max_active_minutes: 30.0,
                cooldown_minutes: 5.0,
                spawn_positions: vec![Vec3::new(0.0, 0.0, 0.0)],
                target_slime_count: 3,
            },
        );
        valley.register_replacer("rep_pulse", IdentifiableId::ValleyAmmo1, 1.0);
        valley.register_replacer("rep_field", IdentifiableId::ValleyAmmo2, 1.0);
        assert!(valley.trigger_ammo_replacer(&mut gm, &mut ammo, "rep_pulse"));
        assert!(valley.trigger_ammo_replacer(&mut gm, &mut ammo, "rep_field"));
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::NimbleValley].get_slot_id(1),
            IdentifiableId::ValleyAmmo1
        );
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::NimbleValley].get_slot_count(2),
            3
        );

        assert!(valley.activate_generator(&mut gm, "gen_1"));
        assert_eq!(
            gm.quicksilver_generators["gen_1"].state,
            QuicksilverGeneratorState::Countdown
        );

        // Advance past countdown -> Active & spawn 3 QuicksilverSlimes
        gm.world.world_time += 10.0;
        let ev1 = valley.step(&mut gm, &mode, &mut pw, &mut ammo, &mut pedia, &mut rng);
        assert_eq!(ev1.activated_generators, vec!["gen_1".to_string()]);
        assert_eq!(ev1.spawned_slimes.len(), 3);

        // Checkpoint extends active window
        let before = gm.quicksilver_generators["gen_1"].timer.unwrap();
        assert!(valley.trigger_checkpoint(&mut gm, "gen_1", 5.0));
        assert!(gm.quicksilver_generators["gen_1"].timer.unwrap() > before);

        // Electric shock produces plorts, collector auto-adds to NimbleValley slot 0
        let plorts = valley.shock_slimes_in_radius(&mut gm, &mut pw, Vec3::ZERO, 10.0);
        assert_eq!(plorts.len(), 3);
        gm.world.world_time += f64::from(DEFAULT_PLORT_COLLECT_MINS) * 60.0 + 1.0;
        let ev2 = valley.step(&mut gm, &mode, &mut pw, &mut ammo, &mut pedia, &mut rng);
        assert_eq!(ev2.collected_plorts, 3);
        assert_eq!(ev2.pedia_unlocked, Some(PediaId::QuicksilverSlime));
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::NimbleValley].get_slot_count(0),
            3
        );

        // Exit valley restores AmmoMode::Default and clears transient ValleyAmmo slots
        valley.exit_valley(&mut gm, &mut ammo);
        assert_eq!(gm.player.ammo_mode, AmmoMode::Default);
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::NimbleValley].get_slot_id(1),
            IdentifiableId::None
        );
    }

    #[test]
    fn step9_viktor_slimulation_impostos_tarr_and_bug_reports() {
        use sr_data::GameModeSettings;
        use sr_ids::{AmmoMode, GameMode, IdentifiableId, PlayerUpgrade, ProgressType};
        use sr_math::{Randoms, Vec3};
        use sr_model::GameModel;
        use sr_physics::PhysicsWorld;

        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut pw = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(93);
        let mut ammo = AmmoController::default();
        let mut prog = ProgressDirector::default();
        let mut tut = TutorialDirector::default();
        let mut slim = SlimulationsSystem::default();

        gm.progress.set_unique_progress(ProgressType::UnlockLab);
        gm.progress.set_unique_progress(ProgressType::UnlockRuins);
        gm.player.upgrades.push(PlayerUpgrade::TreasureCracker2);
        assert!(SlimulationsSystem::can_enter_viktor_lab(&gm, &mode));

        slim.register_imposto(
            &mut gm,
            ImpostoConfig {
                id: "imp_1".to_string(),
                pos: Vec3::ZERO,
                spawn_pos: Vec3::new(1.0, 0.0, 1.0),
                weight: 1.0,
            },
        );
        slim.register_tarr_node(
            &mut gm,
            TarrNodeConfig {
                id: "node_1".to_string(),
                pos: Vec3::new(20.0, 0.0, 20.0),
                dist_sq_from_entrance: 800.0,
                spawner_positions: vec![Vec3::new(20.0, 0.0, 20.0)],
            },
        );
        slim.register_debug_spray_source(&mut gm, "spray_1", Vec3::new(2.0, 0.0, 2.0));

        let _entry = slim
            .enter_slimulation(&mut gm, &mode, &mut prog, &mut tut)
            .unwrap();
        assert!(gm.progress.has_progress(ProgressType::EnterZoneSlimulation));
        assert!(slim.consume_debug_spray_source(&mut gm, &mode, &mut ammo, &mut tut, "spray_1"));
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::Default].get_slot_id(4),
            IdentifiableId::GlitchDebugSprayLiquid
        );
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::Default].get_slot_count(4),
            crate::ammo::GLITCH_DEBUG_SPRAY_MAX_AMMO
        );

        let glitched = slim.expose_imposto(&mut gm, &mut pw, "imp_1", &mut rng);
        assert!((1..=3).contains(&glitched.len()));
        assert!(!gm.glitch.impostos["imp_1"].is_active);

        // Add GlitchSlimes to vacpack, take tarr damage, then exit -> GlitchBugReport
        assert!(ammo.maybe_add_to_slot(
            &mut gm.player,
            IdentifiableId::GlitchSlime,
            None,
            10,
            gm.world.world_time
        ));
        let lost = slim.apply_tarr_damage_exposure(&mut gm, &mode, &mut tut, &mut rng);
        assert!((1..=3).contains(&lost));
        let remaining_expected = 10 - lost;
        let converted = slim.exit_slimulation(&mut gm);
        assert_eq!(converted, remaining_expected);
        assert_eq!(
            gm.player.ammo_dict[&AmmoMode::Default].get_slot_id(0),
            IdentifiableId::GlitchBugReport
        );
        assert!(slim.store_glitch_item(IdentifiableId::GlitchBugReport));
        assert_eq!(slim.storage_count, 1);

        // Sterilizing a GlitchTarrSlime prevents timed split
        let tarr_id = slim.spawn_glitch_tarr(&mut gm, &mut pw, Vec3::ZERO, 1.0, &mut rng);
        assert!(slim.sterilize_tarr(&mut gm, &mut pw, tarr_id));
        assert!(!gm.actors.contains_key(&tarr_id));
    }

    #[test]
    fn step9_holiday_gordos_and_secret_style_largo_propagation() {
        use sr_data::{DataRegistry, GameModeSettings};
        use sr_ids::{GameMode, IdentifiableId, PediaId};
        use sr_math::Randoms;
        use sr_model::{AppearanceSaveSet, GameModel};

        let data = DataRegistry::builtin();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut gm = GameModel::new(GameMode::Classic);
        let mut rng = Randoms::from_seed(94);
        let mut pedia = PediaDirector::default();
        let mut holiday = HolidayDirector::default();

        // Weekend of week 51 (Snowflake / Star ornaments) + Dec 25 (Wiggly Wonderland Echo Note Gordo)
        let xmas_weekend = HolidayDate::new(2026, 12, 25, 5, 51);
        holiday.sync_holidays(&mut gm, xmas_weekend);
        assert!(!gm.holiday.event_gordos.is_empty());
        assert!(!gm.holiday.event_echo_note_gordos.is_empty());
        assert!(gm.gordos.contains_key("gordoPinkParty13"));
        assert!(gm.echo_note_gordos.contains_key("gordoEchoNote11"));

        let party_def = HolidayDirector::find_live_party_gordo(xmas_weekend).unwrap();
        let ornament = HolidayDirector::roll_party_ornament(party_def, &mut rng);
        assert!(ornament.is_ornament());

        let echo_drop =
            HolidayDirector::trigger_echo_note_gordo(&mut gm, &mode, &mut pedia, "gordoEchoNote11");
        assert_eq!(echo_drop, Some(IdentifiableId::EchoNote11));
        assert!(gm.pedia.unlocked.contains(&PediaId::EchoNoteGordoSlime));

        // Secret Style unlocks & dependent Largo appearance propagation
        let app = SlimeAppearanceDirector::default();
        SlimeAppearanceDirector::init_defaults(&mut gm, &data.slimes);
        assert_eq!(
            SlimeAppearanceDirector::selected_appearance(&gm, IdentifiableId::PinkRockLargo),
            AppearanceSaveSet::Classic
        );
        app.unlock_appearance(&mut gm, IdentifiableId::PinkSlime, AppearanceSaveSet::SecretStyle);
        app.unlock_appearance(&mut gm, IdentifiableId::RockSlime, AppearanceSaveSet::SecretStyle);
        app.select_appearance(
            &mut gm,
            &data.slimes,
            IdentifiableId::PinkSlime,
            AppearanceSaveSet::SecretStyle,
        );
        // Only Pink is SecretStyle -> PinkRockLargo stays Classic
        assert_eq!(
            SlimeAppearanceDirector::selected_appearance(&gm, IdentifiableId::PinkRockLargo),
            AppearanceSaveSet::Classic
        );
        // Once Rock is also SecretStyle -> PinkRockLargo propagates to SecretStyle
        app.select_appearance(
            &mut gm,
            &data.slimes,
            IdentifiableId::RockSlime,
            AppearanceSaveSet::SecretStyle,
        );
        assert_eq!(
            SlimeAppearanceDirector::selected_appearance(&gm, IdentifiableId::PinkRockLargo),
            AppearanceSaveSet::SecretStyle
        );
    }
}
