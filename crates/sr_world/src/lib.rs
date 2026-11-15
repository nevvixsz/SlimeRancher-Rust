//! World streaming, regions, zone/cell spawners, doors/gates/treasure pods, and Glass Desert weather.

pub mod desert;
pub mod gates_pods;
pub mod region;
pub mod spawners;

pub use desert::{
    ActiveFireColumn, DesertStepEvents, GlassDesertSystem, OasisConfig, FIRECOLUMN_INTERVAL_SECS,
    FIRECOLUMN_LIFETIME_SECS, FIRECOLUMN_PLAYER_DAMAGE, FIRECOLUMN_RADIUS,
    FIRESTORM_MAX_DURATION_HOURS, FIRESTORM_MAX_WAIT_HOURS, FIRESTORM_MIN_DURATION_HOURS,
    FIRESTORM_MIN_WAIT_HOURS, FIRESTORM_PREP_SECS,
};
pub use gates_pods::{
    activate_slime_gate, ensure_door, open_treasure_pod, step_puzzle_doors, GatePodError,
    GatePodEvents, PodTier, PuzzleDoorConfig, PuzzleSlotConfig, SlimeGateConfig,
    TreasurePodConfig,
};
pub use region::{region_set_for_zone, Region, RegionRegistry, RegionStepEvents};
pub use spawners::{
    ActorSpawner, AuxItemEntry, CellDirector, GingerPatchNode, SpawnConstraint, SpawnMember,
    SpawnerStepEvents, WorldSpawnerSystem, ZoneDirector, DAILY_RESET_HOUR,
};

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::RegionSetId;
use sr_math::Randoms;
use sr_model::GameModel;
use sr_physics::PhysicsWorld;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldStepEvents {
    pub regions: RegionStepEvents,
    pub spawners: SpawnerStepEvents,
    pub gates: GatePodEvents,
    pub desert: DesertStepEvents,
}

/// Unified Step 5 runtime coordinating `RegionRegistry`, spawners, puzzle doors, and Glass Desert.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldRuntime {
    pub regions: RegionRegistry,
    pub spawners: WorldSpawnerSystem,
    pub puzzle_slots: Vec<PuzzleSlotConfig>,
    pub puzzle_doors: Vec<PuzzleDoorConfig>,
    pub desert: GlassDesertSystem,
}

impl WorldRuntime {
    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        mode: &GameModeSettings,
        curr_hour: f32,
        dt: f32,
        rng: &mut Randoms,
    ) -> WorldStepEvents {
        let wt = model.world.world_time;
        let last_wt = model.world.last_world_time;
        let player_set = model.player.curr_region_set_id;
        let player_pos = model.player.position;

        let regions = self.regions.update_player_proximity(player_pos, player_set, wt);
        let spawners = self.spawners.step(
            model,
            physics,
            &self.regions,
            mode,
            wt,
            last_wt,
            curr_hour,
            rng,
        );
        let gates = step_puzzle_doors(model, physics, &self.puzzle_slots, &self.puzzle_doors);
        let desert = self.desert.step(
            model,
            physics,
            mode,
            player_set == RegionSetId::Desert,
            dt,
            rng,
        );

        WorldStepEvents {
            regions,
            spawners,
            gates,
            desert,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sr_data::GameModeSettings;
    use sr_gameplay::{spawn_physical_actor, PlayerController};
    use sr_ids::{
        GadgetId, GameMode, IdentifiableId, PlayerUpgrade, ProgressType, RegionSetId, ZoneId,
    };
    use sr_math::{Randoms, Vec3};
    use sr_model::{AccessDoorState, FirestormMode, GameModel, TreasurePodState};
    use sr_physics::PhysicsWorld;

    #[test]
    fn region_streaming_hibernates_far_cells_and_wakes_with_elapsed_time() {
        let mut reg = RegionRegistry::default();
        reg.active_radius = 50.0;
        reg.proxy_radius = 100.0;
        reg.register_region(Region::new(
            "cell_reef_0",
            ZoneId::Reef,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(20.0, 20.0, 20.0),
        ));
        reg.register_region(Region::new(
            "cell_moss_0",
            ZoneId::Moss,
            Vec3::new(500.0, 0.0, 0.0),
            Vec3::new(20.0, 20.0, 20.0),
        ));

        let ev1 = reg.update_player_proximity(Vec3::ZERO, RegionSetId::Home, 1000.0);
        assert_eq!(ev1.woke_regions, vec![("cell_reef_0".to_string(), 0.0)]);
        assert!(!reg.regions["cell_reef_0"].is_hibernated());
        assert!(reg.regions["cell_moss_0"].is_hibernated());

        // Move far away to Moss at t=5000
        let ev2 = reg.update_player_proximity(Vec3::new(500.0, 0.0, 0.0), RegionSetId::Home, 5000.0);
        assert_eq!(ev2.hibernated_regions, vec!["cell_reef_0".to_string()]);
        assert_eq!(ev2.woke_regions, vec![("cell_moss_0".to_string(), 0.0)]);

        // Return to Reef at t=12200 -> slept 7200s
        let ev3 = reg.update_player_proximity(Vec3::ZERO, RegionSetId::Home, 12200.0);
        assert_eq!(ev3.woke_regions, vec![("cell_reef_0".to_string(), 7200.0)]);
    }

    #[test]
    fn spawners_and_daily_5am_crate_ginger_reset() {
        let mut model = GameModel::new(GameMode::Classic);
        let mut physics = PhysicsWorld::new();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut rng = Randoms::from_seed(11);
        let mut world = WorldRuntime::default();

        world.regions.register_region(Region::new(
            "cell_desert_0",
            ZoneId::Desert,
            Vec3::ZERO,
            Vec3::new(40.0, 20.0, 40.0),
        ));
        model.player.curr_region_set_id = RegionSetId::Desert;

        let mut cell = CellDirector::new("cell_desert_0", ZoneId::Desert);
        cell.slime_spawners.push(ActorSpawner {
            pos: Vec3::new(2.0, 0.0, 2.0),
            constraints: vec![SpawnConstraint::always(vec![SpawnMember {
                ident: IdentifiableId::DervishSlime,
                weight: 1.0,
            }])],
            ..Default::default()
        });
        world.spawners.register_cell(cell);

        let mut zd = ZoneDirector::new(ZoneId::Desert, IdentifiableId::CrateDesert01);
        zd.crate_spawners.push(Vec3::new(5.0, 0.0, 5.0));
        zd.ginger_patches.push(GingerPatchNode {
            id: "gp_1".into(),
            pos: Vec3::new(-5.0, 0.0, 5.0),
            active_actor_id: None,
        });
        world.spawners.register_zone(zd);

        // Cross 05:00 AM
        model.world.last_world_time = Some(4.9 * 3600.0);
        model.world.world_time = 5.1 * 3600.0;
        let ev = world.step(&mut model, &mut physics, &mode, 5.1, 0.1, &mut rng);
        assert!(!ev.spawners.spawned_slimes.is_empty());
        assert_eq!(ev.spawners.spawned_crates.len(), 1);
        assert_eq!(ev.spawners.spawned_gingers.len(), 1);
    }

    #[test]
    fn slime_gates_puzzle_doors_and_treasure_pods() {
        let mut model = GameModel::new(GameMode::Classic);
        let mut physics = PhysicsWorld::new();
        let mut rng = Randoms::from_seed(3);

        // 1. Slime Gate
        ensure_door(&mut model, "gate_moss", AccessDoorState::Locked);
        let gate = SlimeGateConfig {
            gate_id: "sg_moss".into(),
            door_id: "gate_moss".into(),
            unlock_progress: Some(ProgressType::UnlockMoss),
        };
        assert_eq!(activate_slime_gate(&mut model, &gate), Err(GatePodError::NoKeys));
        model.player.keys = 1;
        assert_eq!(activate_slime_gate(&mut model, &gate), Ok(()));
        assert_eq!(model.player.keys, 0);
        assert_eq!(model.doors["gate_moss"].state, AccessDoorState::Open);
        assert_eq!(model.progress.get_progress(ProgressType::SlimeDoors), 1);
        assert_eq!(model.progress.get_progress(ProgressType::UnlockMoss), 1);

        // 2. Puzzle Door
        let slots = vec![PuzzleSlotConfig {
            slot_id: "slot_rad".into(),
            door_id: "door_ruins".into(),
            catch_id: IdentifiableId::RadPlort,
            pos: Vec3::new(10.0, 0.0, 0.0),
            radius: 2.0,
        }];
        let doors = vec![PuzzleDoorConfig {
            door_id: "door_ruins".into(),
            slot_ids: vec!["slot_rad".into()],
            unlock_progress: Some(ProgressType::UnlockRuins),
        }];
        spawn_physical_actor(
            &mut model,
            &mut physics,
            IdentifiableId::RadPlort,
            RegionSetId::Home,
            Vec3::new(10.2, 0.0, 0.0),
            Vec3::ZERO,
        );
        let gev = step_puzzle_doors(&mut model, &mut physics, &slots, &doors);
        assert_eq!(gev.filled_puzzle_slots, vec!["slot_rad".to_string()]);
        assert_eq!(gev.unlocked_doors, vec!["door_ruins".to_string()]);
        assert_eq!(model.doors["door_ruins"].state, AccessDoorState::Open);

        // 3. Treasure Pod requiring TreasureCracker2 (Blue)
        let pod = TreasurePodConfig {
            pod_id: "pod_blue_1".into(),
            tier: PodTier::Blue,
            pos: Vec3::ZERO,
            region: RegionSetId::Home,
            item_prizes: vec![IdentifiableId::GoldPlort],
            blueprint_prizes: vec![GadgetId::HydroShower],
        };
        assert_eq!(
            open_treasure_pod(&mut model, &mut physics, &pod, &mut rng),
            Err(GatePodError::InsufficientCrackerLevel {
                required: 2,
                current: 0
            })
        );
        PlayerController::grant_upgrade(&mut model.player, PlayerUpgrade::TreasureCracker1, 0.0);
        PlayerController::grant_upgrade(&mut model.player, PlayerUpgrade::TreasureCracker2, 0.0);
        let spawned = open_treasure_pod(&mut model, &mut physics, &pod, &mut rng).unwrap();
        assert_eq!(spawned.len(), 1);
        assert_eq!(model.pods["pod_blue_1"].state, TreasurePodState::Open);
        assert!(model.gadgets.blueprints.contains(&GadgetId::HydroShower));
    }

    #[test]
    fn desert_firestorm_cycle_and_oasis_revival() {
        let mut model = GameModel::new(GameMode::Classic);
        let mut physics = PhysicsWorld::new();
        let mode = GameModeSettings::for_mode(GameMode::Classic);
        let mut rng = Randoms::from_seed(5);
        let mut desert = GlassDesertSystem::default();

        desert.register_oasis(
            &mut model,
            OasisConfig {
                oasis_id: "oasis_1".into(),
                pos: Vec3::new(0.0, 0.0, 0.0),
                radius: 10.0,
            },
            false,
        );
        desert.column_sites.push(Vec3::new(2.0, 0.0, 0.0));
        model.progress.set_progress(ProgressType::UnlockDesert, 1);

        // Initialize storm schedule
        model.world.world_time = 32400.0;
        let _ = desert.step(&mut model, &mut physics, &mode, true, 0.1, &mut rng);
        assert!(model.world.next_firestorm_time > 32400.0);

        // Jump to storm start -> Preparing
        model.world.world_time = model.world.next_firestorm_time + 1.0;
        let ev_prep = desert.step(&mut model, &mut physics, &mode, true, 0.1, &mut rng);
        assert_eq!(ev_prep.firestorm_mode, FirestormMode::Preparing);

        // Jump past prep -> Active, spawns column & Fire/Mosaic slime
        model.world.world_time = model.world.next_firecolumn_time + 1.0;
        let ev_act = desert.step(&mut model, &mut physics, &mode, true, 0.1, &mut rng);
        assert_eq!(ev_act.firestorm_mode, FirestormMode::Active);
        assert!(!desert.active_columns.is_empty());
        assert!(!ev_act.spawned_slimes.is_empty());

        // Revive Oasis with MagicWaterLiquid -> suppresses fire columns inside oasis
        assert!(!desert.revive_oasis(&mut model, "oasis_1", IdentifiableId::WaterLiquid));
        assert!(desert.revive_oasis(&mut model, "oasis_1", IdentifiableId::MagicWaterLiquid));
        assert!(desert.active_columns.is_empty());
    }
}
