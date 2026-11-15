//! Binary `.sav` persistence layer compatible with Slime Rancher `GameV12`.
//!
//! Implements .NET `BinaryReader`/`BinaryWriter` framing (`PersistedDataSet`),
//! all `GameV12` nested datasets, bidirectional `GameV12 <-> GameModel` mapping,
//! atomic `FileStorageProvider`, and `AutoSaveDirector`.

pub mod binary;
pub mod game_v12;
pub mod mapper;
pub mod storage;
pub mod types;

pub use binary::{
    BinReader, BinWriter, PersistError, PersistResult, PersistedDataset, DATA_PAYLOAD_BEGIN,
    DATA_PAYLOAD_END, ELEMENT_SEPARATOR, SECTION_SEPARATOR,
};
pub use game_v12::{
    ActorDataV09, AppearancesV01, GameAchieveV03, GameV12, HolidayDirectorV02, InstrumentV01,
    LandPlotV08, LoadedSummary, LockDataV01, PediaV03, PlayerV14, RanchV07, WorldV22,
};
pub use mapper::SAVE_FORMAT_APP_VERSION;
pub use storage::{
    AutoSaveDirector, FileStorageProvider, SaveSlotInfo, StorageProvider,
    DEFAULT_AUTOSAVE_INTERVAL_SECS, DEFAULT_MAX_BACKUPS, SAVE_EXTENSION,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sr_ids::{
        AmmoMode, GadgetId, GameMode, IdentifiableId, LandPlotId, LandPlotUpgrade, PediaId,
        PlayerUpgrade, ProgressType, RegionSetId, ZoneId,
    };
    use sr_math::{Quat, Vec3};
    use sr_model::{
        ActorKind, AmmoSlot, AppearanceSaveSet, DroneProgramData, EchoNoteGordoModel,
        FirestormMode, GadgetKind, GadgetModel, GadgetSiteModel, GameModel, GlitchImpostoModel,
        GlitchTarrNodeModel, Instrument, LandPlotModel, MailEntry, MailType, Palette, PaletteType,
        QuicksilverEnergyGeneratorModel, QuicksilverGeneratorState, ResourceCycleState,
        SlimeEmotionSnapshot, StorageType, TutorialId, UpgradeLockData, Weather,
    };

    use super::*;

    #[test]
    fn step10_binary_primitives_leb128_strings_and_markers() {
        let mut bytes = Vec::new();
        let long_str = "S".repeat(300) + "Ω_slime";
        {
            let mut w = BinWriter::new(&mut bytes);
            w.write_string("SRTEST").unwrap();
            w.write_u32(12).unwrap();
            w.write_i16(DATA_PAYLOAD_BEGIN).unwrap();
            w.write_string(&long_str).unwrap();
            w.write_list(&[10i32, 20, 30], |w, v| w.write_i32(*v)).unwrap();
            let mut pairs = std::collections::HashMap::new();
            pairs.insert("alpha".to_string(), 1.5f32);
            pairs.insert("beta".to_string(), -2.25f32);
            w.write_dict(&pairs, |w, k| w.write_string(k), |w, v| w.write_f32(*v))
                .unwrap();
            w.write_i16(DATA_PAYLOAD_END).unwrap();
        }

        let mut r = BinReader::new(std::io::Cursor::new(&bytes));
        assert_eq!(r.read_string().unwrap(), "SRTEST");
        assert_eq!(r.read_u32().unwrap(), 12);
        r.read_marker(DATA_PAYLOAD_BEGIN).unwrap();
        assert_eq!(r.read_string().unwrap(), long_str);
        let list = r.read_list(|r| r.read_i32()).unwrap();
        assert_eq!(list, vec![10, 20, 30]);
        let dict = r
            .read_dict(|r| r.read_string(), |r| r.read_f32())
            .unwrap();
        assert_eq!(dict.get("alpha"), Some(&1.5));
        assert_eq!(dict.get("beta"), Some(&-2.25));
        r.read_marker(DATA_PAYLOAD_END).unwrap();
        assert_eq!(r.stream_pos().unwrap(), bytes.len() as u64);
    }

    #[test]
    fn step10_game_v12_binary_roundtrip_and_summary_only() {
        let mut model = GameModel::new(GameMode::Classic);
        model.world.world_time = 123_456.75;
        model.world.econ_seed = 42_001.0;
        model.world.curr_weather = Weather::Rain;
        model.world.curr_firestorm_mode = FirestormMode::Preparing;
        model.world.next_firestorm_time = 200_000.0;
        model.world.market_saturation.insert(IdentifiableId::PinkPlort, 1.35);

        // Player state + emotions in ammo slot
        model.player.currency = 9876;
        model.player.currency_ever_collected = 12_345;
        model.player.keys = 3;
        model.player.curr_health = 180.0;
        model.player.curr_energy = 140.0;
        model.player.position = Vec3::new(12.5, 4.25, -33.0);
        model.player.rotation = Quat::from_euler(glam::EulerRot::YXZ, 0.9, 0.25, 0.0);
        model.player.curr_region_set_id = RegionSetId::Valley;
        model.player.upgrades.push(PlayerUpgrade::Jetpack);
        model.player.upgrade_locks.insert(
            PlayerUpgrade::Health3,
            UpgradeLockData {
                timed_lock: true,
                locked_until: 5000.0,
            },
        );
        model.player.unlocked_zone_maps.insert(ZoneId::Reef);
        model.player.unlocked_zone_maps.insert(ZoneId::Moss);
        if let Some(ammo) = model.player.ammo_dict.get_mut(&AmmoMode::Default) {
            ammo.slots[0] = Some(AmmoSlot {
                id: IdentifiableId::TabbySlime,
                count: 5,
                emotions: vec![SlimeEmotionSnapshot {
                    agitation: 0.4,
                    fear: 0.1,
                    hunger: 0.75,
                }],
            });
        }

        // Mail & progress
        model.mail.all_mail.push(MailEntry {
            mail_type: MailType::Personal,
            key: "casey_1".to_string(),
            read: true,
        });
        model
            .progress
            .progress_dict
            .insert(ProgressType::UnlockMoss, 1);

        // Ranch plot + Silo + Palette
        let mut corral = LandPlotModel::new("plot_01", LandPlotId::Corral);
        corral.upgrades.insert(LandPlotUpgrade::Walls);
        corral.upgrades.insert(LandPlotUpgrade::MusicBox);
        corral.next_feeding_time = 130_000.0;
        corral.remaining_feed_operations = 2;
        corral.silo_storage_indices = [1, 2, 0, 0];
        let mut plort_silo = sr_model::AmmoModel::new(4, 4);
        plort_silo.slots[0] = Some(AmmoSlot {
            id: IdentifiableId::MosaicPlort,
            count: 42,
            emotions: Vec::new(),
        });
        corral.silo_ammo.insert(StorageType::Plort, plort_silo);
        model.land_plots.insert("plot_01".to_string(), corral);
        model
            .ranch
            .palettes
            .insert(PaletteType::House, Palette::Palette07);

        // Actors: Slime + Produce + Plort
        let slime_id = model.create_actor(
            IdentifiableId::HunterSlime,
            RegionSetId::Home,
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
        );
        if let Some(a) = model.actor_mut(slime_id) {
            if let ActorKind::Slime(s) = &mut a.kind {
                s.is_feral = true;
                s.hunger.curr_val = 0.65;
                s.fashions.push(IdentifiableId::HandlebarFashion);
            }
        }
        let fruit_id = model.create_actor(
            IdentifiableId::LemonFruit,
            RegionSetId::Desert,
            Vec3::new(-4.0, 1.5, 8.0),
            Quat::IDENTITY,
        );
        if let Some(a) = model.actor_mut(fruit_id) {
            if let ActorKind::Produce(p) = &mut a.kind {
                p.state = ResourceCycleState::Ripe;
                p.progress_time = 99_999.0;
            }
        }

        // Gadgets: Drone + program
        let mut drone = GadgetModel::new(GadgetId::Drone, "site_ranch_01", 90.0);
        if let GadgetKind::Drone(d) = &mut drone.kind {
            d.battery_deplete_time = 150_000.0;
            d.ammo_id = IdentifiableId::PinkPlort;
            d.ammo_count = 25;
            d.programs.push(DroneProgramData {
                target: IdentifiableId::PinkPlort,
                source: "CORRAL".to_string(),
                destination: "MARKET".to_string(),
            });
        }
        model.gadget_sites.insert(
            "site_ranch_01".to_string(),
            GadgetSiteModel {
                id: "site_ranch_01".to_string(),
                attached: Some(drone),
            },
        );

        // Pedia, Tutorials, Achievements, DLC
        model.pedia.unlocked.insert(PediaId::PinkSlime);
        model.pedia.unlocked.insert(PediaId::GlitchSlime);
        model.pedia.completed_tuts.insert(TutorialId::Vaccing);
        model.tutorials.popup_queue.push(TutorialId::Plort);
        model
            .game_achieves
            .game_int_stats
            .insert("DEATHS".to_string(), 2);
        model.quicksilver_generators.insert(
            "qseg_01".to_string(),
            QuicksilverEnergyGeneratorModel {
                id: "qseg_01".to_string(),
                state: QuicksilverGeneratorState::Active,
                timer: Some(45.5),
            },
        );
        model.echo_note_gordos.insert(
            "eng_01".to_string(),
            EchoNoteGordoModel {
                id: "eng_01".to_string(),
                has_been_triggered: true,
            },
        );
        model.glitch.nodes.insert(
            "gnode_01".to_string(),
            GlitchTarrNodeModel {
                id: "gnode_01".to_string(),
                is_active: true,
            },
        );
        model.glitch.impostos.insert(
            "gimp_01".to_string(),
            GlitchImpostoModel {
                id: "gimp_01".to_string(),
                is_active: true,
            },
        );
        model.holiday.event_gordos.insert("twinkle_01".to_string());
        let mut app_set = HashSet::new();
        app_set.insert(AppearanceSaveSet::SecretStyle);
        model
            .appearances
            .unlocks
            .insert(IdentifiableId::PinkSlime, app_set);
        model
            .appearances
            .selections
            .insert(IdentifiableId::PinkSlime, AppearanceSaveSet::SecretStyle);
        model.instrument.unlocked.insert(Instrument::Marimba);
        model.instrument.selection = Instrument::Marimba;

        // Serialize to GameV12 binary
        let v12 = GameV12::from_game_model(&model, "slot_0", "My Ranch", 7, 638_000_000_000);
        let bytes = v12.to_bytes().expect("GameV12::to_bytes");

        // 1. Summary-only read
        let sum = GameV12::load_summary_from_bytes(&bytes).expect("load_summary_from_bytes");
        assert_eq!(sum.game_name, "slot_0");
        assert_eq!(sum.display_name, "My Ranch");
        assert_eq!(sum.summary.currency, 9876);
        assert_eq!(sum.summary.pedia_count, 2);
        assert_eq!(sum.summary.save_number, 7);

        // 2. Full binary roundtrip -> GameModel
        let decoded_v12 = GameV12::from_bytes(&bytes).expect("GameV12::from_bytes");
        let restored = decoded_v12.into_game_model().expect("into_game_model");

        assert_eq!(restored.game_mode, GameMode::Classic);
        assert_eq!(restored.world.world_time, 123_456.75);
        assert_eq!(restored.world.econ_seed, 42_001.0);
        assert_eq!(restored.world.curr_weather, Weather::Rain);
        assert_eq!(restored.world.curr_firestorm_mode, FirestormMode::Preparing);
        assert_eq!(restored.player.currency, 9876);
        assert_eq!(restored.player.keys, 3);
        assert_eq!(restored.player.curr_region_set_id, RegionSetId::Valley);
        assert!((restored.player.position - Vec3::new(12.5, 4.25, -33.0)).length() < 1e-4);
        let slot0 = restored.player.ammo_dict[&AmmoMode::Default].slots[0]
            .as_ref()
            .unwrap();
        assert_eq!(slot0.id, IdentifiableId::TabbySlime);
        assert_eq!(slot0.count, 5);
        assert!((slot0.emotions[0].hunger - 0.75).abs() < 1e-5);

        let restored_plot = &restored.land_plots["plot_01"];
        assert_eq!(restored_plot.type_id, LandPlotId::Corral);
        assert!(restored_plot.upgrades.contains(&LandPlotUpgrade::MusicBox));
        assert_eq!(
            restored_plot.silo_ammo[&StorageType::Plort].slots[0]
                .as_ref()
                .unwrap()
                .count,
            42
        );
        assert_eq!(restored.ranch.palettes[&PaletteType::House], Palette::Palette07);

        assert_eq!(restored.actors.len(), 2);
        let restored_slime = restored.actor(slime_id).unwrap();
        if let ActorKind::Slime(s) = &restored_slime.kind {
            assert!(s.is_feral);
            assert!((s.hunger.curr_val - 0.65).abs() < 1e-5);
            assert_eq!(s.fashions, vec![IdentifiableId::HandlebarFashion]);
        } else {
            panic!("expected Slime");
        }

        let restored_drone = restored.gadget_sites["site_ranch_01"]
            .attached
            .as_ref()
            .unwrap();
        if let GadgetKind::Drone(d) = &restored_drone.kind {
            assert_eq!(d.ammo_id, IdentifiableId::PinkPlort);
            assert_eq!(d.ammo_count, 25);
            assert_eq!(d.programs.len(), 1);
            assert_eq!(d.programs[0].destination, "MARKET");
        } else {
            panic!("expected Drone");
        }

        assert!(restored.pedia.unlocked.contains(&PediaId::GlitchSlime));
        assert!(restored.pedia.completed_tuts.contains(&TutorialId::Vaccing));
        assert_eq!(restored.tutorials.popup_queue, vec![TutorialId::Plort]);
        assert_eq!(restored.game_achieves.game_int_stats["DEATHS"], 2);
        assert_eq!(
            restored.quicksilver_generators["qseg_01"].state,
            QuicksilverGeneratorState::Active
        );
        assert!(restored.echo_note_gordos["eng_01"].has_been_triggered);
        assert!(restored.glitch.nodes["gnode_01"].is_active);
        assert!(restored.glitch.impostos["gimp_01"].is_active);
        assert_eq!(restored.instrument.selection, Instrument::Marimba);
        assert_eq!(
            restored.appearances.selections[&IdentifiableId::PinkSlime],
            AppearanceSaveSet::SecretStyle
        );
    }

    #[test]
    fn step10_file_storage_atomic_write_backups_and_autosave_director() {
        let dir = std::env::temp_dir().join(format!(
            "sr_persist_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let provider = FileStorageProvider::new(&dir).with_max_backups(3);
        let mut director = AutoSaveDirector::new(provider);

        let mut model = GameModel::new(GameMode::Casual);
        model.player.currency = 500;
        let s1 = director
            .save_game("ranch_a", "Ranch A", &model, 100)
            .expect("save 1");
        assert_eq!(s1, 1);
        assert!(director.storage.has_game_data("ranch_a"));

        // Save 2 & 3 rotate backups (.bak1, .bak2) and delete .tmp
        model.player.currency = 900;
        let s2 = director
            .save_game("ranch_a", "Ranch A", &model, 200)
            .expect("save 2");
        assert_eq!(s2, 2);
        assert!(dir.join("ranch_a.sav.bak1").is_file());
        assert!(!dir.join("ranch_a.sav.tmp").exists());

        // Autosave 1440s countdown
        assert!(!director.tick(1000.0));
        assert!(director.tick(440.0));
        assert!((director.time_until_next_autosave - DEFAULT_AUTOSAVE_INTERVAL_SECS).abs() < 1e-9);

        // Summary listing
        let summaries = director
            .storage
            .list_saves_with_summaries()
            .expect("list_saves_with_summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].summary.summary.currency, 900);

        // Load back
        let (v12, loaded) = director.load_game("ranch_a").expect("load_game");
        assert_eq!(v12.summary.save_number, 2);
        assert_eq!(loaded.player.currency, 900);
        assert_eq!(director.next_save_number, 3);

        // Cleanup
        director.storage.delete_game_data("ranch_a").expect("delete");
        assert!(!director.storage.has_game_data("ranch_a"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
