//! Bidirectional mapping between binary `GameV12` persistence datasets and `sr_model::GameModel`.

use std::collections::{HashMap, HashSet};

use glam::EulerRot;
use sr_ids::{
    AmmoMode, GadgetId, GameMode, IdentifiableId, LandPlotId, LandPlotUpgrade, PediaId,
    PlayerUpgrade, ProgressType, RegionSetId, SlimeEmotion, ZoneId,
};
use sr_math::{Quat, Vec3};
use sr_model::{
    AccessDoorModel, AccessDoorState, ActorKind, ActorModel, AmmoModel, AmmoSlot, AnimalModel,
    AppearanceSaveSet, BlueprintLockData, DroneGadgetModel, DroneModel, DroneProgramData,
    EchoNoteGordoModel, ExchangeItemEntry, ExchangeOffer, ExchangeRequestedItemEntry,
    ExtractorModel, FeedSpeed, FirestormMode, GadgetKind, GadgetModel, GadgetSiteModel, GameModel,
    GlitchImpostoModel, GlitchTarrNodeModel, GordoModel, Instrument, LandPlotModel,
    LiquidSourceModel, MailEntry, MailType, MasterSwitchModel, NonIdentReward, OasisModel,
    OfferType, Palette, PaletteType, PlortModel, ProduceModel, ProgressTrackerId, PuzzleSlotModel,
    QuicksilverEnergyGeneratorModel, QuicksilverGeneratorState, ResourceCycleState, SlimeEmotionSnapshot,
    SlimeModel, SnareModel, SpawnResourceId, StorageType, SwitchState, TreasurePodModel,
    TreasurePodState, TutorialId, UpgradeLockData, WarpDepotModel, Weather,
};

use crate::binary::{PersistError, PersistResult};
use crate::game_v12::{
    ActorDataV09, AppearancesV01, GameAchieveV03, GameV12, HolidayDirectorV02, InstrumentV01,
    LandPlotV08, LockDataV01, PediaV03, PlayerV14, RanchV07, WorldV22,
};
use crate::types::{
    AmmoDataV02, DecorizerSettingsV01, DecorizerV01, DroneGadgetV01, DroneProgramV01,
    DroneStationBatteryV01, DroneStationV01, DroneV05, EchoNoteGordoV01, ExchangeOfferV04,
    FirestormV01, GameSummaryV04, GlitchImpostoV01, GlitchSlimeDataV01, GlitchSlimulationV02,
    GlitchStorageV01, GlitchTarrNodeV01, GordoV01, ItemEntryV03, MailV02, PlacedGadgetV08,
    QuicksilverEnergyGeneratorV02, RequestedItemEntryV03, ResourceCycleDataV03, SlimeEmotionDataV02,
    TreasurePodV01, Vector3V02,
};

pub const SAVE_FORMAT_APP_VERSION: &str = "1.4.4";

impl GameV12 {
    /// Serializes a live `GameModel` into a `GameV12` persistence tree (`SavedGame.Pull`).
    pub fn from_game_model(
        model: &GameModel,
        game_name: impl Into<String>,
        display_name: impl Into<String>,
        save_number: u64,
        save_timestamp_ticks: i64,
    ) -> Self {
        let game_name = game_name.into();
        let display_name = display_name.into();

        let summary = GameSummaryV04 {
            version: SAVE_FORMAT_APP_VERSION.to_string(),
            game_mode: model.game_mode as i32,
            icon_id: IdentifiableId::PinkSlime as i32,
            currency: model.player.currency,
            pedia_count: model.pedia.unlocked.len() as i32,
            world_time: model.world.world_time,
            save_timestamp_ticks,
            save_timestamp_offset_mins: 0.0,
            is_game_over: model.is_game_over(),
            save_number,
        };

        let world = pull_world(model);
        let player = pull_player(model);
        let ranch = pull_ranch(model);
        let actors = pull_actors(model);
        let pedia = pull_pedia(model);
        let achieve = pull_achieve(model);
        let holiday = HolidayDirectorV02 {
            event_gordos: model.holiday.event_gordos.iter().cloned().collect(),
            event_echo_note_gordos: model
                .holiday
                .event_echo_note_gordos
                .iter()
                .cloned()
                .collect(),
        };
        let appearances = AppearancesV01 {
            unlocks: model
                .appearances
                .unlocks
                .iter()
                .map(|(&id, sets)| (id as i32, sets.iter().map(|&s| s as i32).collect()))
                .collect(),
            selections: model
                .appearances
                .selections
                .iter()
                .map(|(&id, &s)| (id as i32, s as i32))
                .collect(),
        };
        let instrument = InstrumentV01 {
            unlocks: model.instrument.unlocked.iter().map(|&i| i as i32).collect(),
            selection: model.instrument.selection as i32,
        };

        Self {
            game_name,
            display_name,
            summary,
            world,
            player,
            ranch,
            actors,
            pedia,
            achieve,
            holiday,
            appearances,
            instrument,
        }
    }

    /// Reconstructs a fresh `GameModel` from this `GameV12` (`SavedGame.Push`).
    pub fn into_game_model(&self) -> PersistResult<GameModel> {
        let gm_enum = GameMode::try_from(self.player.game_mode).map_err(|_| {
            PersistError::InvalidEnum {
                enum_name: "GameMode",
                value: self.player.game_mode,
            }
        })?;
        let mut model = GameModel::new(gm_enum);
        self.apply_to_game_model(&mut model)?;
        Ok(model)
    }

    /// Applies all datasets in `self` onto an existing `GameModel`.
    pub fn apply_to_game_model(&self, model: &mut GameModel) -> PersistResult<()> {
        model.game_mode = GameMode::try_from(self.player.game_mode).map_err(|_| {
            PersistError::InvalidEnum {
                enum_name: "GameMode",
                value: self.player.game_mode,
            }
        })?;

        push_world(model, &self.world)?;
        push_player(model, &self.player)?;
        push_ranch(model, &self.ranch)?;
        push_actors(model, &self.actors)?;
        push_pedia(model, &self.pedia)?;
        push_achieve(model, &self.achieve)?;

        model.holiday.event_gordos = self.holiday.event_gordos.iter().cloned().collect();
        model.holiday.event_echo_note_gordos =
            self.holiday.event_echo_note_gordos.iter().cloned().collect();

        model.appearances.unlocks.clear();
        for (&id_raw, sets_raw) in &self.appearances.unlocks {
            if let Ok(id) = IdentifiableId::try_from(id_raw) {
                let mut set = HashSet::new();
                for &s_raw in sets_raw {
                    set.insert(appearance_from_i32(s_raw)?);
                }
                model.appearances.unlocks.insert(id, set);
            }
        }
        model.appearances.selections.clear();
        for (&id_raw, &s_raw) in &self.appearances.selections {
            if let Ok(id) = IdentifiableId::try_from(id_raw) {
                model
                    .appearances
                    .selections
                    .insert(id, appearance_from_i32(s_raw)?);
            }
        }

        model.instrument.unlocked.clear();
        for &i_raw in &self.instrument.unlocks {
            model
                .instrument
                .unlocked
                .insert(instrument_from_i32(i_raw)?);
        }
        model.instrument.selection = instrument_from_i32(self.instrument.selection)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WorldV22 <-> GameModel
// ---------------------------------------------------------------------------

fn pull_world(model: &GameModel) -> WorldV22 {
    let mut offers = HashMap::new();
    for (&otype, offer) in &model.world.curr_offers {
        offers.insert(
            otype as i32,
            ExchangeOfferV04 {
                has_offer: true,
                rancher_id: offer.rancher_id.clone(),
                offer_id: offer.offer_id.clone(),
                expire_time: offer.expire_time,
                early_exchange_time: offer.early_exchange_time,
                requests: offer
                    .requests
                    .iter()
                    .map(|r| RequestedItemEntryV03 {
                        id: r.id as i32,
                        non_ident_reward: r.spec_reward as i32,
                        count: r.count,
                        progress: r.progress,
                    })
                    .collect(),
                rewards: offer
                    .rewards
                    .iter()
                    .map(|r| ItemEntryV03 {
                        id: r.id as i32,
                        non_ident_reward: r.spec_reward as i32,
                        count: r.count,
                    })
                    .collect(),
            },
        );
    }

    let econ_saturations = model
        .world
        .market_saturation
        .iter()
        .map(|(&id, &sat)| (id as i32, sat))
        .collect();

    let liquid_source_units = model
        .liquid_sources
        .iter()
        .map(|(k, s)| (k.clone(), s.units_filled))
        .collect();

    let gordos = model
        .gordos
        .iter()
        .map(|(k, g)| {
            (
                k.clone(),
                GordoV01 {
                    eaten_count: g.gordo_eaten_count,
                    fashions: g.fashions.iter().map(|&f| f as i32).collect(),
                },
            )
        })
        .collect();

    let mut placed_gadgets = HashMap::new();
    for (site_id, site) in &model.gadget_sites {
        if let Some(g) = &site.attached {
            placed_gadgets.insert(site_id.clone(), pull_placed_gadget(g));
        }
    }

    let treasure_pods = model
        .pods
        .iter()
        .map(|(k, p)| {
            (
                k.clone(),
                TreasurePodV01 {
                    state: p.state as i32,
                    spawn_queue: p.spawn_queue.iter().map(|&id| id as i32).collect(),
                },
            )
        })
        .collect();

    let switches = model
        .switches
        .iter()
        .map(|(k, s)| (k.clone(), s.state as i32))
        .collect();

    let puzzle_slots_filled = model
        .puzzle_slots
        .iter()
        .map(|(k, p)| (k.clone(), p.filled))
        .collect();

    let occupied_phase_sites = model
        .world
        .occupied_phase_sites
        .iter()
        .map(|k| (k.clone(), true))
        .collect();

    let oasis_states = model
        .oases
        .iter()
        .map(|(k, o)| (k.clone(), o.is_live))
        .collect();

    let quicksilver_energy_generators = model
        .quicksilver_generators
        .iter()
        .map(|(k, q)| {
            (
                k.clone(),
                QuicksilverEnergyGeneratorV02 {
                    state: q.state as i32,
                    timer: q.timer,
                },
            )
        })
        .collect();

    let echo_note_gordos = model
        .echo_note_gordos
        .iter()
        .map(|(k, g)| {
            (
                k.clone(),
                Some(EchoNoteGordoV01 {
                    state: i32::from(g.has_been_triggered),
                }),
            )
        })
        .collect();

    let glitch = GlitchSlimulationV02 {
        teleporters: HashMap::new(),
        nodes: model
            .glitch
            .nodes
            .iter()
            .map(|(k, n)| {
                (
                    k.clone(),
                    Some(GlitchTarrNodeV01 {
                        activation_time: if n.is_active { 1.0 } else { 0.0 },
                    }),
                )
            })
            .collect(),
        slimes: model
            .actors
            .iter()
            .filter_map(|(&aid, a)| {
                a.as_slime().filter(|s| s.is_glitch).map(|s| {
                    (
                        aid,
                        Some(GlitchSlimeDataV01 {
                            exposure_chance: s.glitch_exposure_chance,
                            death_time: s.glitch_death_time,
                        }),
                    )
                })
            })
            .collect(),
        imposto_directors: HashMap::new(),
        impostos: model
            .glitch
            .impostos
            .iter()
            .map(|(k, i)| {
                (
                    k.clone(),
                    Some(GlitchImpostoV01 {
                        deactivate_time: if i.is_active { None } else { Some(0.0) },
                        cooldown_time: 0.0,
                    }),
                )
            })
            .collect(),
        storage: HashMap::<String, Option<GlitchStorageV01>>::new(),
    };

    WorldV22 {
        world_time: model.world.world_time,
        econ_seed: model.world.econ_seed,
        daily_offer_create_time: model.world.next_daily_offer_create_time,
        last_offer_rancher_ids: model.world.last_offer_rancher_ids.clone(),
        pending_offer_rancher_ids: model.world.pending_offer_rancher_ids.clone(),
        weather_until: model.world.weather_until,
        weather: model.world.curr_weather as i32,
        offers,
        econ_saturations,
        teleport_node_activations: HashMap::new(),
        animal_spawner_times: Vec::new(),
        liquid_source_units,
        spawner_trigger_times: Vec::new(),
        gordos,
        resource_spawner_water: Vec::new(),
        placed_gadgets,
        treasure_pods,
        switches,
        puzzle_slots_filled,
        occupied_phase_sites,
        firestorm: FirestormV01 {
            end_storm_time: model.world.end_firestorm_time,
            storm_preparing: model.world.curr_firestorm_mode == FirestormMode::Preparing,
            next_storm_time: model.world.next_firestorm_time,
        },
        oasis_states,
        active_ginger_patches: model.world.current_ginger_patch_ids.clone(),
        quicksilver_energy_generators,
        echo_note_gordos,
        glitch: Some(glitch),
    }
}

fn push_world(model: &mut GameModel, w: &WorldV22) -> PersistResult<()> {
    model.world.world_time = w.world_time;
    model.world.econ_seed = w.econ_seed;
    model.world.next_daily_offer_create_time = w.daily_offer_create_time;
    model.world.last_offer_rancher_ids = w.last_offer_rancher_ids.clone();
    model.world.pending_offer_rancher_ids = w.pending_offer_rancher_ids.clone();
    model.world.weather_until = w.weather_until;
    model.world.curr_weather = weather_from_i32(w.weather)?;

    model.world.curr_offers.clear();
    for (&otype_raw, off) in &w.offers {
        if !off.has_offer {
            continue;
        }
        let otype = offer_type_from_i32(otype_raw)?;
        let mut requests = Vec::with_capacity(off.requests.len());
        for r in &off.requests {
            requests.push(ExchangeRequestedItemEntry {
                id: ident_from_i32(r.id)?,
                count: r.count,
                progress: r.progress,
                spec_reward: non_ident_from_i32(r.non_ident_reward)?,
            });
        }
        let mut rewards = Vec::with_capacity(off.rewards.len());
        for r in &off.rewards {
            rewards.push(ExchangeItemEntry {
                id: ident_from_i32(r.id)?,
                count: r.count,
                spec_reward: non_ident_from_i32(r.non_ident_reward)?,
            });
        }
        model.world.curr_offers.insert(
            otype,
            ExchangeOffer {
                offer_id: off.offer_id.clone(),
                rancher_id: off.rancher_id.clone(),
                expire_time: off.expire_time,
                early_exchange_time: off.early_exchange_time,
                requests,
                rewards,
            },
        );
    }

    model.world.market_saturation.clear();
    for (&id_raw, &sat) in &w.econ_saturations {
        if let Ok(id) = IdentifiableId::try_from(id_raw) {
            model.world.market_saturation.insert(id, sat);
        }
    }

    for (id, &units) in &w.liquid_source_units {
        model
            .liquid_sources
            .entry(id.clone())
            .or_insert_with(LiquidSourceModel::default)
            .units_filled = units;
    }

    model.gordos.clear();
    for (id, g) in &w.gordos {
        let mut fashions = Vec::with_capacity(g.fashions.len());
        for &f in &g.fashions {
            fashions.push(ident_from_i32(f)?);
        }
        model.gordos.insert(
            id.clone(),
            GordoModel {
                id: id.clone(),
                gordo_eaten_count: g.eaten_count,
                target_count: 50,
                fashions,
            },
        );
    }

    model.gadget_sites.clear();
    for (site_id, pg) in &w.placed_gadgets {
        let gadget = push_placed_gadget(site_id, pg)?;
        model.gadget_sites.insert(
            site_id.clone(),
            GadgetSiteModel {
                id: site_id.clone(),
                attached: Some(gadget),
            },
        );
    }

    model.pods.clear();
    for (id, tp) in &w.treasure_pods {
        let mut spawn_queue = Vec::with_capacity(tp.spawn_queue.len());
        for &sid in &tp.spawn_queue {
            spawn_queue.push(ident_from_i32(sid)?);
        }
        model.pods.insert(
            id.clone(),
            TreasurePodModel {
                id: id.clone(),
                state: treasure_pod_state_from_i32(tp.state)?,
                spawn_queue,
            },
        );
    }

    model.switches.clear();
    for (id, &st) in &w.switches {
        model.switches.insert(
            id.clone(),
            MasterSwitchModel {
                id: id.clone(),
                state: switch_state_from_i32(st)?,
            },
        );
    }

    model.puzzle_slots.clear();
    for (id, &filled) in &w.puzzle_slots_filled {
        model.puzzle_slots.insert(
            id.clone(),
            PuzzleSlotModel {
                id: id.clone(),
                filled,
            },
        );
    }

    model.world.occupied_phase_sites = w
        .occupied_phase_sites
        .iter()
        .filter(|(_, &occupied)| occupied)
        .map(|(k, _)| k.clone())
        .collect();

    model.world.end_firestorm_time = w.firestorm.end_storm_time;
    model.world.next_firestorm_time = w.firestorm.next_storm_time;
    model.world.curr_firestorm_mode = if w.firestorm.storm_preparing {
        FirestormMode::Preparing
    } else if w.firestorm.end_storm_time > w.world_time {
        FirestormMode::Active
    } else {
        FirestormMode::Idle
    };

    model.oases.clear();
    for (id, &is_live) in &w.oasis_states {
        model.oases.insert(
            id.clone(),
            OasisModel {
                id: id.clone(),
                is_live,
            },
        );
    }

    model.world.current_ginger_patch_ids = w.active_ginger_patches.clone();

    model.quicksilver_generators.clear();
    for (id, q) in &w.quicksilver_energy_generators {
        model.quicksilver_generators.insert(
            id.clone(),
            QuicksilverEnergyGeneratorModel {
                id: id.clone(),
                state: quicksilver_state_from_i32(q.state)?,
                timer: q.timer,
            },
        );
    }

    model.echo_note_gordos.clear();
    for (id, opt_g) in &w.echo_note_gordos {
        if let Some(g) = opt_g {
            model.echo_note_gordos.insert(
                id.clone(),
                EchoNoteGordoModel {
                    id: id.clone(),
                    has_been_triggered: g.state != 0,
                },
            );
        }
    }

    model.glitch.nodes.clear();
    model.glitch.impostos.clear();
    if let Some(glitch) = &w.glitch {
        for (id, opt_node) in &glitch.nodes {
            if let Some(n) = opt_node {
                model.glitch.nodes.insert(
                    id.clone(),
                    GlitchTarrNodeModel {
                        id: id.clone(),
                        is_active: n.activation_time > 0.0,
                    },
                );
            }
        }
        for (id, opt_imp) in &glitch.impostos {
            if let Some(i) = opt_imp {
                model.glitch.impostos.insert(
                    id.clone(),
                    GlitchImpostoModel {
                        id: id.clone(),
                        is_active: i.deactivate_time.is_none(),
                    },
                );
            }
        }
    }

    Ok(())
}

fn pull_placed_gadget(g: &GadgetModel) -> PlacedGadgetV08 {
    let mut out = PlacedGadgetV08 {
        gadget_id: g.ident as i32,
        y_rotation: g.y_rotation,
        wait_for_chargeup_time: g.wait_for_chargeup_time,
        ..Default::default()
    };
    match &g.kind {
        GadgetKind::Basic => {}
        GadgetKind::Extractor(e) => {
            out.extractor_cycles_remaining = e.cycles_remaining;
            out.extractor_next_produce_time = e.next_produce_time;
            out.extractor_queued_to_produce = e.queued_to_produce;
        }
        GadgetKind::WarpDepot(w) => {
            out.is_primary_in_link = w.is_linked;
            out.ammo.push(AmmoDataV02 {
                id: w.ammo_id as i32,
                count: w.ammo_count,
                emotion_data: SlimeEmotionDataV02::default(),
            });
        }
        GadgetKind::Snare(s) => {
            out.bait_type_id = s.bait_type_id as i32;
            out.gordo_type_id = s.gordo_type_id as i32;
            out.gordo_eaten_count = s.gordo_eaten_count;
            out.fashions = s.fashions.iter().map(|&f| f as i32).collect();
        }
        GadgetKind::Drone(d) => {
            out.drone = Some(DroneGadgetV01 {
                drone: Some(DroneV05 {
                    position: Some(Vector3V02::from(d.no_clip_position)),
                    rotation: Some(Vector3V02::default()),
                    ammo: Some(AmmoDataV02 {
                        id: d.ammo_id as i32,
                        count: d.ammo_count,
                        emotion_data: SlimeEmotionDataV02::default(),
                    }),
                    fashions: d.fashions.iter().map(|&f| f as i32).collect(),
                    no_clip: d.no_clip_position != Vec3::ZERO,
                }),
                station: Some(DroneStationV01 {
                    battery: Some(DroneStationBatteryV01 {
                        time: d.battery_deplete_time,
                    }),
                }),
                programs: d
                    .programs
                    .iter()
                    .map(|p| DroneProgramV01 {
                        target: p.target.as_csharp_name().to_string(),
                        source: p.source.clone(),
                        destination: p.destination.clone(),
                    })
                    .collect(),
            });
        }
    }
    out
}

fn push_placed_gadget(site_id: &str, pg: &PlacedGadgetV08) -> PersistResult<GadgetModel> {
    let ident = GadgetId::try_from(pg.gadget_id).map_err(|_| PersistError::InvalidEnum {
        enum_name: "GadgetId",
        value: pg.gadget_id,
    })?;
    let mut g = GadgetModel::new(ident, site_id, pg.y_rotation);
    g.wait_for_chargeup_time = pg.wait_for_chargeup_time;
    match &mut g.kind {
        GadgetKind::Basic => {}
        GadgetKind::Extractor(e) => {
            e.cycles_remaining = pg.extractor_cycles_remaining;
            e.next_produce_time = pg.extractor_next_produce_time;
            e.queued_to_produce = pg.extractor_queued_to_produce;
        }
        GadgetKind::WarpDepot(w) => {
            w.is_linked = pg.is_primary_in_link;
            if let Some(first) = pg.ammo.first() {
                w.ammo_id = ident_from_i32(first.id)?;
                w.ammo_count = first.count;
            }
        }
        GadgetKind::Snare(s) => {
            s.bait_type_id = ident_from_i32(pg.bait_type_id)?;
            s.gordo_type_id = ident_from_i32(pg.gordo_type_id)?;
            s.gordo_eaten_count = pg.gordo_eaten_count;
            s.fashions.clear();
            for &f in &pg.fashions {
                s.fashions.push(ident_from_i32(f)?);
            }
        }
        GadgetKind::Drone(d) => {
            if let Some(dg) = &pg.drone {
                if let Some(st) = &dg.station {
                    if let Some(bat) = &st.battery {
                        d.battery_deplete_time = bat.time;
                    }
                }
                if let Some(dr) = &dg.drone {
                    if let Some(pos) = dr.position {
                        d.no_clip_position = pos.value;
                    }
                    if let Some(am) = &dr.ammo {
                        d.ammo_id = ident_from_i32(am.id)?;
                        d.ammo_count = am.count;
                    }
                    d.fashions.clear();
                    for &f in &dr.fashions {
                        d.fashions.push(ident_from_i32(f)?);
                    }
                }
                d.programs.clear();
                for p in &dg.programs {
                    let target = IdentifiableId::from_name(&p.target).unwrap_or(IdentifiableId::None);
                    d.programs.push(DroneProgramData {
                        target,
                        source: p.source.clone(),
                        destination: p.destination.clone(),
                    });
                }
            }
        }
    }
    Ok(g)
}

// ---------------------------------------------------------------------------
// PlayerV14 <-> GameModel
// ---------------------------------------------------------------------------

fn pull_ammo_list(ammo: &AmmoModel) -> Vec<AmmoDataV02> {
    ammo.slots
        .iter()
        .map(|slot_opt| match slot_opt {
            Some(slot) if !slot.is_empty() => {
                let mut emotion_data = HashMap::new();
                if let Some(last) = slot.emotions.last() {
                    emotion_data.insert(SlimeEmotion::Agitation as i32, last.agitation);
                    emotion_data.insert(SlimeEmotion::Fear as i32, last.fear);
                    emotion_data.insert(SlimeEmotion::Hunger as i32, last.hunger);
                }
                AmmoDataV02 {
                    id: slot.id as i32,
                    count: slot.count,
                    emotion_data: SlimeEmotionDataV02 { emotion_data },
                }
            }
            _ => AmmoDataV02::default(),
        })
        .collect()
}

fn push_ammo_list(list: &[AmmoDataV02], usable_slots: usize) -> PersistResult<AmmoModel> {
    let mut slots = Vec::with_capacity(list.len());
    for item in list {
        let id = ident_from_i32(item.id)?;
        if id == IdentifiableId::None || item.count <= 0 {
            slots.push(None);
        } else {
            let mut emotions = Vec::new();
            if !item.emotion_data.emotion_data.is_empty() {
                emotions.push(SlimeEmotionSnapshot {
                    agitation: item
                        .emotion_data
                        .emotion_data
                        .get(&(SlimeEmotion::Agitation as i32))
                        .copied()
                        .unwrap_or(0.0),
                    fear: item
                        .emotion_data
                        .emotion_data
                        .get(&(SlimeEmotion::Fear as i32))
                        .copied()
                        .unwrap_or(0.0),
                    hunger: item
                        .emotion_data
                        .emotion_data
                        .get(&(SlimeEmotion::Hunger as i32))
                        .copied()
                        .unwrap_or(0.0),
                });
            }
            slots.push(Some(AmmoSlot {
                id,
                count: item.count,
                emotions,
            }));
        }
    }
    Ok(AmmoModel {
        usable_slots: usable_slots.min(slots.len()),
        slots,
    })
}

fn quat_to_unity_euler_deg(q: Quat) -> Vec3 {
    let (y, x, z) = q.to_euler(EulerRot::YXZ);
    Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
}

fn unity_euler_deg_to_quat(deg: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        deg.y.to_radians(),
        deg.x.to_radians(),
        deg.z.to_radians(),
    )
}

fn pull_player(model: &GameModel) -> PlayerV14 {
    let p = &model.player;
    let ammo = p
        .ammo_dict
        .iter()
        .map(|(&mode, am)| (mode as i32, pull_ammo_list(am)))
        .collect();

    let mail = model
        .mail
        .all_mail
        .iter()
        .map(|m| MailV02 {
            mail_type: m.mail_type as i32,
            message_key: m.key.clone(),
            is_read: m.read,
        })
        .collect();

    let decorizer = DecorizerV01 {
        contents: model
            .decorizer
            .contents
            .iter()
            .map(|(&id, &cnt)| (id as i32, cnt))
            .collect(),
        settings: model
            .decorizer
            .settings
            .iter()
            .map(|(k, &id)| {
                (
                    k.clone(),
                    Some(DecorizerSettingsV01 {
                        selected: id as i32,
                    }),
                )
            })
            .collect(),
    };

    PlayerV14 {
        health: p.curr_health.round() as i32,
        energy: p.curr_energy.round() as i32,
        radiation: p.curr_rads.round() as i32,
        currency: p.currency,
        keys: p.keys,
        currency_ever_collected: p.currency_ever_collected,
        version: SAVE_FORMAT_APP_VERSION.to_string(),
        game_mode: model.game_mode as i32,
        game_icon_id: IdentifiableId::PinkSlime as i32,
        player_pos: Vector3V02::from(p.position),
        player_rot_euler: Vector3V02::from(quat_to_unity_euler_deg(p.rotation)),
        upgrades: p.upgrades.iter().map(|&u| u as i32).collect(),
        ammo,
        mail,
        avail_upgrades: p.avail_upgrades.iter().map(|&u| u as i32).collect(),
        upgrade_locks: p
            .upgrade_locks
            .iter()
            .map(|(&u, l)| {
                (
                    u as i32,
                    LockDataV01 {
                        timed_lock: l.timed_lock,
                        locked_until: l.locked_until,
                    },
                )
            })
            .collect(),
        progress: model
            .progress
            .progress_dict
            .iter()
            .map(|(&k, &v)| (k as i32, v))
            .collect(),
        delayed_progress: model
            .progress
            .delayed_trackers
            .iter()
            .map(|(&k, &v)| (k as i32, v))
            .collect(),
        blueprints: model.gadgets.blueprints.iter().map(|&b| b as i32).collect(),
        avail_blueprints: model
            .gadgets
            .avail_blueprints
            .iter()
            .map(|&b| b as i32)
            .collect(),
        blueprint_locks: model
            .gadgets
            .blueprint_locks
            .iter()
            .map(|(&b, l)| {
                (
                    b as i32,
                    LockDataV01 {
                        timed_lock: l.timed_lock,
                        locked_until: l.locked_until,
                    },
                )
            })
            .collect(),
        gadgets: model
            .gadgets
            .gadgets
            .iter()
            .map(|(&g, &c)| (g as i32, c))
            .collect(),
        craft_mat_counts: model
            .gadgets
            .craft_mat_counts
            .iter()
            .map(|(&id, &c)| (id as i32, c))
            .collect(),
        region_set_id: p.curr_region_set_id as i32,
        unlocked_zone_maps: p.unlocked_zone_maps.iter().map(|&z| z as i32).collect(),
        end_game_time: p.end_game_time,
        decorizer: Some(decorizer),
    }
}

fn push_player(model: &mut GameModel, p: &PlayerV14) -> PersistResult<()> {
    model.player.curr_health = p.health as f32;
    model.player.curr_energy = p.energy as f32;
    model.player.curr_rads = p.radiation as f32;
    model.player.currency = p.currency;
    model.player.keys = p.keys;
    model.player.currency_ever_collected = p.currency_ever_collected;
    model.player.position = p.player_pos.value;
    model.player.rotation = unity_euler_deg_to_quat(p.player_rot_euler.value);
    model.player.curr_region_set_id =
        RegionSetId::try_from(p.region_set_id).map_err(|_| PersistError::InvalidEnum {
            enum_name: "RegionSetId",
            value: p.region_set_id,
        })?;
    model.player.end_game_time = p.end_game_time;

    model.player.upgrades.clear();
    for &u_raw in &p.upgrades {
        if let Ok(u) = PlayerUpgrade::try_from(u_raw) {
            model.player.upgrades.push(u);
        }
    }
    model.player.avail_upgrades.clear();
    for &u_raw in &p.avail_upgrades {
        if let Ok(u) = PlayerUpgrade::try_from(u_raw) {
            model.player.avail_upgrades.push(u);
        }
    }
    model.player.upgrade_locks.clear();
    for (&u_raw, l) in &p.upgrade_locks {
        if let Ok(u) = PlayerUpgrade::try_from(u_raw) {
            model.player.upgrade_locks.insert(
                u,
                UpgradeLockData {
                    timed_lock: l.timed_lock,
                    locked_until: l.locked_until,
                },
            );
        }
    }

    for (&mode_raw, list) in &p.ammo {
        if let Ok(mode) = AmmoMode::try_from(mode_raw) {
            let default_usable = if mode == AmmoMode::NimbleValley { 3 } else { 4 };
            let am = push_ammo_list(list, default_usable.max(list.len()))?;
            model.player.ammo_dict.insert(mode, am);
        }
    }

    model.player.unlocked_zone_maps.clear();
    for &z_raw in &p.unlocked_zone_maps {
        if let Ok(z) = ZoneId::try_from(z_raw) {
            model.player.unlocked_zone_maps.insert(z);
        }
    }

    model.mail.all_mail.clear();
    for m in &p.mail {
        model.mail.all_mail.push(MailEntry {
            mail_type: mail_type_from_i32(m.mail_type)?,
            key: m.message_key.clone(),
            read: m.is_read,
        });
    }
    model.mail.has_new_mail = model.mail.all_mail.iter().any(|m| !m.read);

    model.progress.progress_dict.clear();
    for (&k_raw, &v) in &p.progress {
        if let Ok(pt) = ProgressType::try_from(k_raw) {
            model.progress.progress_dict.insert(pt, v);
        }
    }
    model.progress.delayed_trackers.clear();
    for (&k_raw, &t) in &p.delayed_progress {
        if let Some(tid) = progress_tracker_from_i32(k_raw) {
            model.progress.delayed_trackers.insert(tid, t);
        }
    }

    model.gadgets.blueprints.clear();
    for &b_raw in &p.blueprints {
        if let Ok(b) = GadgetId::try_from(b_raw) {
            model.gadgets.blueprints.push(b);
        }
    }
    model.gadgets.avail_blueprints.clear();
    for &b_raw in &p.avail_blueprints {
        if let Ok(b) = GadgetId::try_from(b_raw) {
            model.gadgets.avail_blueprints.push(b);
        }
    }
    model.gadgets.blueprint_locks.clear();
    for (&b_raw, l) in &p.blueprint_locks {
        if let Ok(b) = GadgetId::try_from(b_raw) {
            model.gadgets.blueprint_locks.insert(
                b,
                BlueprintLockData {
                    timed_lock: l.timed_lock,
                    locked_until: l.locked_until,
                },
            );
        }
    }
    model.gadgets.gadgets.clear();
    for (&g_raw, &cnt) in &p.gadgets {
        if let Ok(g) = GadgetId::try_from(g_raw) {
            model.gadgets.gadgets.insert(g, cnt);
        }
    }
    model.gadgets.craft_mat_counts.clear();
    for (&id_raw, &cnt) in &p.craft_mat_counts {
        if let Ok(id) = IdentifiableId::try_from(id_raw) {
            model.gadgets.craft_mat_counts.insert(id, cnt);
        }
    }

    model.decorizer.contents.clear();
    model.decorizer.settings.clear();
    if let Some(dec) = &p.decorizer {
        for (&id_raw, &cnt) in &dec.contents {
            if let Ok(id) = IdentifiableId::try_from(id_raw) {
                model.decorizer.contents.insert(id, cnt);
            }
        }
        for (k, opt_s) in &dec.settings {
            if let Some(s) = opt_s {
                model
                    .decorizer
                    .settings
                    .insert(k.clone(), ident_from_i32(s.selected)?);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// RanchV07 <-> GameModel
// ---------------------------------------------------------------------------

fn pull_ranch(model: &GameModel) -> RanchV07 {
    let plots = model
        .land_plots
        .values()
        .map(|lp| LandPlotV08 {
            feeder_next_time: lp.next_feeding_time,
            feeder_pending_count: lp.remaining_feed_operations,
            feeder_cycle_speed: lp.feeder_cycle_speed as i32,
            collector_next_time: lp.collector_next_time,
            attached_death_time: lp.attached_death_time,
            type_id: lp.type_id as i32,
            attached_id: lp.attached_id as i32,
            id: lp.id.clone(),
            upgrades: lp.upgrades.iter().map(|&u| u as i32).collect(),
            silo_ammo: lp
                .silo_ammo
                .iter()
                .map(|(&st, am)| (st as i32, pull_ammo_list(am)))
                .collect(),
            silo_activator_indices: lp.silo_storage_indices.to_vec(),
            ash_units: lp.ash_units,
        })
        .collect();

    let access_door_states = model
        .doors
        .iter()
        .map(|(k, d)| (k.clone(), d.state as i32))
        .collect();

    let palettes = model
        .ranch
        .palettes
        .iter()
        .map(|(&pt, &pal)| (pt as i32, pal as i32))
        .collect();

    let mut ranch_fast_forward = HashMap::new();
    if model.ranch.ranch_fast_forward_amount != 0.0 {
        ranch_fast_forward.insert(
            "ranch".to_string(),
            model.ranch.ranch_fast_forward_amount,
        );
    }

    RanchV07 {
        plots,
        access_door_states,
        palettes,
        ranch_fast_forward,
    }
}

fn push_ranch(model: &mut GameModel, r: &RanchV07) -> PersistResult<()> {
    model.land_plots.clear();
    for lp in &r.plots {
        let type_id =
            LandPlotId::try_from(lp.type_id).map_err(|_| PersistError::InvalidEnum {
                enum_name: "LandPlotId",
                value: lp.type_id,
            })?;
        let mut upgrades = HashSet::new();
        for &u_raw in &lp.upgrades {
            if let Ok(u) = LandPlotUpgrade::try_from(u_raw) {
                upgrades.insert(u);
            }
        }
        let mut silo_ammo = HashMap::new();
        for (&st_raw, list) in &lp.silo_ammo {
            let st = storage_type_from_i32(st_raw)?;
            silo_ammo.insert(st, push_ammo_list(list, list.len())?);
        }
        let mut silo_storage_indices = [0i32; 4];
        for (i, &val) in lp.silo_activator_indices.iter().take(4).enumerate() {
            silo_storage_indices[i] = val;
        }
        model.land_plots.insert(
            lp.id.clone(),
            LandPlotModel {
                id: lp.id.clone(),
                type_id,
                upgrades,
                next_feeding_time: lp.feeder_next_time,
                remaining_feed_operations: lp.feeder_pending_count,
                feeder_cycle_speed: feed_speed_from_i32(lp.feeder_cycle_speed)?,
                collector_next_time: lp.collector_next_time,
                attached_death_time: lp.attached_death_time,
                attached_id: spawn_resource_from_i32(lp.attached_id)?,
                attached_resource_id: IdentifiableId::None,
                silo_ammo,
                silo_storage_indices,
                ash_units: lp.ash_units,
            },
        );
    }

    model.doors.clear();
    for (id, &st_raw) in &r.access_door_states {
        model.doors.insert(
            id.clone(),
            AccessDoorModel {
                id: id.clone(),
                state: access_door_state_from_i32(st_raw)?,
            },
        );
    }

    model.ranch.palettes.clear();
    for (&pt_raw, &pal_raw) in &r.palettes {
        model
            .ranch
            .palettes
            .insert(palette_type_from_i32(pt_raw)?, palette_from_i32(pal_raw)?);
    }
    model.ranch.ranch_fast_forward_amount = r.ranch_fast_forward.values().copied().sum();
    Ok(())
}

// ---------------------------------------------------------------------------
// ActorDataV09 <-> GameModel
// ---------------------------------------------------------------------------

fn pull_actors(model: &GameModel) -> Vec<ActorDataV09> {
    let mut actors: Vec<&ActorModel> = model.actors.values().collect();
    actors.sort_by_key(|a| a.actor_id);
    actors
        .into_iter()
        .map(|a| {
            let mut d = ActorDataV09 {
                pos: Vector3V02::from(a.pos),
                rot: Vector3V02::from(quat_to_unity_euler_deg(a.rot)),
                actor_id: a.actor_id,
                type_id: a.ident as i32,
                region_set_id: a.curr_region_set_id as i32,
                ..Default::default()
            };
            match &a.kind {
                ActorKind::Slime(s) => {
                    d.emotions.emotion_data.insert(
                        SlimeEmotion::Agitation as i32,
                        s.agitation.curr_val,
                    );
                    d.emotions
                        .emotion_data
                        .insert(SlimeEmotion::Fear as i32, s.fear.curr_val);
                    d.emotions
                        .emotion_data
                        .insert(SlimeEmotion::Hunger as i32, s.hunger.curr_val);
                    d.disabled_at_time = s.disabled_at_time;
                    d.is_feral = s.is_feral;
                    d.is_glitch = s.is_glitch;
                    d.fashions = s.fashions.iter().map(|&f| f as i32).collect();
                }
                ActorKind::Animal(an) => {
                    d.transform_time = an.transform_time;
                    d.reproduce_time = an.next_reproduce_time;
                    d.fashions = an.fashions.iter().map(|&f| f as i32).collect();
                }
                ActorKind::Produce(p) => {
                    d.cycle_data = ResourceCycleDataV03 {
                        state: p.state as i32,
                        progress_time: p.progress_time,
                    };
                }
                ActorKind::Plort(pl) => {
                    d.destroy_time = pl.destroy_time;
                }
                ActorKind::ScienceMat | ActorKind::Echo | ActorKind::Basic => {}
            }
            d
        })
        .collect()
}

fn push_actors(model: &mut GameModel, list: &[ActorDataV09]) -> PersistResult<()> {
    model.actors.clear();
    model.actors_by_ident.clear();
    let mut max_id = model.next_actor_id;
    for d in list {
        let ident = ident_from_i32(d.type_id)?;
        let region = RegionSetId::try_from(d.region_set_id).map_err(|_| {
            PersistError::InvalidEnum {
                enum_name: "RegionSetId",
                value: d.region_set_id,
            }
        })?;
        let mut actor = ActorModel::new(
            d.actor_id,
            ident,
            region,
            d.pos.value,
            unity_euler_deg_to_quat(d.rot.value),
        );
        match &mut actor.kind {
            ActorKind::Slime(s) => {
                if let Some(&v) = d.emotions.emotion_data.get(&(SlimeEmotion::Agitation as i32)) {
                    s.agitation.curr_val = v;
                }
                if let Some(&v) = d.emotions.emotion_data.get(&(SlimeEmotion::Fear as i32)) {
                    s.fear.curr_val = v;
                }
                if let Some(&v) = d.emotions.emotion_data.get(&(SlimeEmotion::Hunger as i32)) {
                    s.hunger.curr_val = v;
                }
                s.disabled_at_time = d.disabled_at_time;
                s.is_feral = d.is_feral;
                s.is_glitch = d.is_glitch;
                s.fashions.clear();
                for &f in &d.fashions {
                    s.fashions.push(ident_from_i32(f)?);
                }
            }
            ActorKind::Animal(an) => {
                an.transform_time = d.transform_time;
                an.next_reproduce_time = d.reproduce_time;
                an.fashions.clear();
                for &f in &d.fashions {
                    an.fashions.push(ident_from_i32(f)?);
                }
            }
            ActorKind::Produce(p) => {
                p.state = resource_cycle_from_i32(d.cycle_data.state)?;
                p.progress_time = d.cycle_data.progress_time;
            }
            ActorKind::Plort(pl) => {
                pl.destroy_time = d.destroy_time;
            }
            ActorKind::ScienceMat | ActorKind::Echo | ActorKind::Basic => {}
        }
        if d.actor_id >= max_id {
            max_id = d.actor_id + 1;
        }
        model.insert_actor_with_id(actor);
    }
    model.next_actor_id = max_id;
    Ok(())
}

// ---------------------------------------------------------------------------
// PediaV03 & GameAchieveV03 <-> GameModel
// ---------------------------------------------------------------------------

fn pull_pedia(model: &GameModel) -> PediaV03 {
    PediaV03 {
        progress_given_for_pedia_count: model.pedia.progress_count,
        unlocked_ids: model
            .pedia
            .unlocked
            .iter()
            .map(|id| id.as_csharp_name().to_string())
            .collect(),
        completed_tuts: model
            .pedia
            .completed_tuts
            .iter()
            .map(|&t| tutorial_csharp_name(t).to_string())
            .collect(),
        popup_queue: model
            .tutorials
            .popup_queue
            .iter()
            .map(|&t| tutorial_csharp_name(t).to_string())
            .collect(),
    }
}

fn push_pedia(model: &mut GameModel, p: &PediaV03) -> PersistResult<()> {
    model.pedia.progress_count = p.progress_given_for_pedia_count;
    model.pedia.unlocked.clear();
    for name in &p.unlocked_ids {
        if let Some(pid) = PediaId::from_name(name) {
            model.pedia.unlocked.insert(pid);
        }
    }
    model.pedia.completed_tuts.clear();
    model.tutorials.completed_ids.clear();
    for name in &p.completed_tuts {
        if let Some(tid) = tutorial_from_name(name) {
            model.pedia.completed_tuts.insert(tid);
            model.tutorials.completed_ids.insert(tid);
        }
    }
    model.tutorials.popup_queue.clear();
    for name in &p.popup_queue {
        if let Some(tid) = tutorial_from_name(name) {
            model.tutorials.popup_queue.push(tid);
        }
    }
    Ok(())
}

fn pull_achieve(model: &GameModel) -> GameAchieveV03 {
    let mut game_double_stat_dict = HashMap::new();
    for (idx, key) in [
        (0, "LAST_LEFT_RANCH"),
        (1, "LAST_ENTERED_RANCH"),
        (2, "LAST_SLEPT"),
        (3, "LAST_AWOKE"),
    ] {
        if let Some(&v) = model.game_achieves.doubles.get(key) {
            game_double_stat_dict.insert(idx, v);
        }
    }

    let mut game_int_stat_dict = HashMap::new();
    for (idx, key) in [
        (0, "DEATHS"),
        (1, "UPGRADES_PURCHASED"),
        (2, "CURRENCY_SPENT"),
        (3, "FABRICATED_GADGETS"),
    ] {
        if let Some(&v) = model.game_achieves.game_int_stats.get(key) {
            game_int_stat_dict.insert(idx, v);
        }
    }

    let mut game_id_dict_stat_dict = HashMap::new();
    if let Some(plorts) = model.game_achieves.game_id_dicts.get("PLORT_TYPES_SOLD") {
        let mut inner = HashMap::new();
        for (&id, &cnt) in plorts {
            inner.insert(id as i32, cnt);
        }
        game_id_dict_stat_dict.insert(0, inner);
    }

    GameAchieveV03 {
        game_float_stat_dict: HashMap::new(),
        game_double_stat_dict,
        game_int_stat_dict,
        game_id_dict_stat_dict,
    }
}

fn push_achieve(model: &mut GameModel, a: &GameAchieveV03) -> PersistResult<()> {
    model.game_achieves.doubles.clear();
    for (&idx, &v) in &a.game_double_stat_dict {
        let key = match idx {
            0 => "LAST_LEFT_RANCH",
            1 => "LAST_ENTERED_RANCH",
            2 => "LAST_SLEPT",
            3 => "LAST_AWOKE",
            _ => continue,
        };
        model.game_achieves.doubles.insert(key.to_string(), v);
    }

    model.game_achieves.game_int_stats.clear();
    for (&idx, &v) in &a.game_int_stat_dict {
        let key = match idx {
            0 => "DEATHS",
            1 => "UPGRADES_PURCHASED",
            2 => "CURRENCY_SPENT",
            3 => "FABRICATED_GADGETS",
            _ => continue,
        };
        model.game_achieves.game_int_stats.insert(key.to_string(), v);
    }

    model.game_achieves.game_id_dicts.clear();
    if let Some(inner) = a.game_id_dict_stat_dict.get(&0) {
        let mut map = HashMap::new();
        for (&id_raw, &cnt) in inner {
            if let Ok(id) = IdentifiableId::try_from(id_raw) {
                map.insert(id, cnt);
            }
        }
        model
            .game_achieves
            .game_id_dicts
            .insert("PLORT_TYPES_SOLD".to_string(), map);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Small i32 -> Enum helpers
// ---------------------------------------------------------------------------

fn ident_from_i32(v: i32) -> PersistResult<IdentifiableId> {
    IdentifiableId::try_from(v).map_err(|_| PersistError::InvalidEnum {
        enum_name: "IdentifiableId",
        value: v,
    })
}

fn resource_cycle_from_i32(v: i32) -> PersistResult<ResourceCycleState> {
    match v {
        0 => Ok(ResourceCycleState::Unripe),
        1 => Ok(ResourceCycleState::Ripe),
        2 => Ok(ResourceCycleState::Edible),
        3 => Ok(ResourceCycleState::Rotten),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "ResourceCycleState",
            value: v,
        }),
    }
}

fn access_door_state_from_i32(v: i32) -> PersistResult<AccessDoorState> {
    match v {
        0 => Ok(AccessDoorState::Locked),
        1 => Ok(AccessDoorState::Open),
        2 => Ok(AccessDoorState::Closed),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "AccessDoorState",
            value: v,
        }),
    }
}

fn treasure_pod_state_from_i32(v: i32) -> PersistResult<TreasurePodState> {
    match v {
        0 => Ok(TreasurePodState::Locked),
        1 => Ok(TreasurePodState::Open),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "TreasurePodState",
            value: v,
        }),
    }
}

fn switch_state_from_i32(v: i32) -> PersistResult<SwitchState> {
    match v {
        0 => Ok(SwitchState::Up),
        1 => Ok(SwitchState::Down),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "SwitchState",
            value: v,
        }),
    }
}

fn feed_speed_from_i32(v: i32) -> PersistResult<FeedSpeed> {
    match v {
        0 => Ok(FeedSpeed::Normal),
        1 => Ok(FeedSpeed::Slow),
        2 => Ok(FeedSpeed::Fast),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "FeedSpeed",
            value: v,
        }),
    }
}

fn storage_type_from_i32(v: i32) -> PersistResult<StorageType> {
    match v {
        0 => Ok(StorageType::NonSlimes),
        1 => Ok(StorageType::Plort),
        2 => Ok(StorageType::Food),
        3 => Ok(StorageType::Crafting),
        4 => Ok(StorageType::Elder),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "StorageType",
            value: v,
        }),
    }
}

fn mail_type_from_i32(v: i32) -> PersistResult<MailType> {
    match v {
        0 => Ok(MailType::Personal),
        1 => Ok(MailType::Upgrade),
        2 => Ok(MailType::Exchange),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "MailType",
            value: v,
        }),
    }
}

fn offer_type_from_i32(v: i32) -> PersistResult<OfferType> {
    match v {
        0 => Ok(OfferType::General),
        1 => Ok(OfferType::Ogden),
        2 => Ok(OfferType::OgdenRecur),
        3 => Ok(OfferType::Mochi),
        4 => Ok(OfferType::MochiRecur),
        5 => Ok(OfferType::Viktor),
        6 => Ok(OfferType::ViktorRecur),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "OfferType",
            value: v,
        }),
    }
}

fn non_ident_from_i32(v: i32) -> PersistResult<NonIdentReward> {
    match v {
        0 => Ok(NonIdentReward::None),
        100 => Ok(NonIdentReward::OgdenMix),
        101 => Ok(NonIdentReward::OgdenGarden),
        102 => Ok(NonIdentReward::OgdenRanch),
        200 => Ok(NonIdentReward::MochiExtraMile),
        201 => Ok(NonIdentReward::MochiCoop),
        202 => Ok(NonIdentReward::MochiRanch),
        300 => Ok(NonIdentReward::ViktorChickenCloner),
        301 => Ok(NonIdentReward::ViktorDeluxeDrones),
        302 => Ok(NonIdentReward::ViktorRanch),
        10000 => Ok(NonIdentReward::NewbucksSmall),
        10001 => Ok(NonIdentReward::NewbucksMedium),
        10002 => Ok(NonIdentReward::NewbucksLarge),
        10003 => Ok(NonIdentReward::NewbucksHuge),
        10004 => Ok(NonIdentReward::NewbucksMochi),
        20000 => Ok(NonIdentReward::TimeExtension12h),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "NonIdentReward",
            value: v,
        }),
    }
}

fn weather_from_i32(v: i32) -> PersistResult<Weather> {
    match v {
        0 => Ok(Weather::None),
        1 => Ok(Weather::Rain),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "Weather",
            value: v,
        }),
    }
}

fn palette_type_from_i32(v: i32) -> PersistResult<PaletteType> {
    match v {
        0 => Ok(PaletteType::RanchTech),
        1 => Ok(PaletteType::House),
        2 => Ok(PaletteType::Vac),
        3 => Ok(PaletteType::OgdenTech),
        4 => Ok(PaletteType::OgdenHouse),
        5 => Ok(PaletteType::MochiTech),
        6 => Ok(PaletteType::MochiHouse),
        7 => Ok(PaletteType::Valley),
        8 => Ok(PaletteType::ViktorTech),
        9 => Ok(PaletteType::ViktorHouse),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "PaletteType",
            value: v,
        }),
    }
}

fn palette_from_i32(v: i32) -> PersistResult<Palette> {
    match v {
        0 => Ok(Palette::Default),
        1 => Ok(Palette::Palette01),
        2 => Ok(Palette::Palette02),
        3 => Ok(Palette::Palette03),
        4 => Ok(Palette::Palette04),
        5 => Ok(Palette::Palette05),
        6 => Ok(Palette::Palette06),
        7 => Ok(Palette::Palette07),
        8 => Ok(Palette::Palette08),
        9 => Ok(Palette::Palette09),
        10 => Ok(Palette::Palette10),
        11 => Ok(Palette::Palette11),
        12 => Ok(Palette::Palette12),
        13 => Ok(Palette::Palette13),
        14 => Ok(Palette::Palette14),
        15 => Ok(Palette::Palette15),
        16 => Ok(Palette::Palette16),
        17 => Ok(Palette::Palette17),
        18 => Ok(Palette::Palette18),
        19 => Ok(Palette::Palette19),
        20 => Ok(Palette::Palette20),
        21 => Ok(Palette::Palette21),
        22 => Ok(Palette::Palette22),
        23 => Ok(Palette::Palette23),
        24 => Ok(Palette::Palette24),
        25 => Ok(Palette::Palette25),
        26 => Ok(Palette::Palette26),
        27 => Ok(Palette::Palette27),
        28 => Ok(Palette::Palette28),
        29 => Ok(Palette::Palette29),
        30 => Ok(Palette::Palette30),
        1000 => Ok(Palette::Ogden),
        1001 => Ok(Palette::Mochi),
        1002 => Ok(Palette::Viktor),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "Palette",
            value: v,
        }),
    }
}

fn spawn_resource_from_i32(v: i32) -> PersistResult<SpawnResourceId> {
    match v {
        0 => Ok(SpawnResourceId::None),
        1 => Ok(SpawnResourceId::CuberryTree),
        2 => Ok(SpawnResourceId::MangoTree),
        3 => Ok(SpawnResourceId::CarrotPatch),
        4 => Ok(SpawnResourceId::OcaocaPatch),
        5 => Ok(SpawnResourceId::PearTree),
        6 => Ok(SpawnResourceId::PogoTree),
        7 => Ok(SpawnResourceId::ParsnipPatch),
        8 => Ok(SpawnResourceId::BeetPatch),
        9 => Ok(SpawnResourceId::OnionPatch),
        10 => Ok(SpawnResourceId::LemonTree),
        11 => Ok(SpawnResourceId::GingerPatch),
        12 => Ok(SpawnResourceId::CuberryTreeDlx),
        13 => Ok(SpawnResourceId::MangoTreeDlx),
        14 => Ok(SpawnResourceId::CarrotPatchDlx),
        15 => Ok(SpawnResourceId::OcaocaPatchDlx),
        16 => Ok(SpawnResourceId::PearTreeDlx),
        17 => Ok(SpawnResourceId::PogoTreeDlx),
        18 => Ok(SpawnResourceId::ParsnipPatchDlx),
        19 => Ok(SpawnResourceId::BeetPatchDlx),
        20 => Ok(SpawnResourceId::OnionPatchDlx),
        21 => Ok(SpawnResourceId::LemonTreeDlx),
        22 => Ok(SpawnResourceId::GingerPatchDlx),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "SpawnResourceId",
            value: v,
        }),
    }
}

fn appearance_from_i32(v: i32) -> PersistResult<AppearanceSaveSet> {
    match v {
        0 => Ok(AppearanceSaveSet::Classic),
        1 => Ok(AppearanceSaveSet::SecretStyle),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "AppearanceSaveSet",
            value: v,
        }),
    }
}

fn quicksilver_state_from_i32(v: i32) -> PersistResult<QuicksilverGeneratorState> {
    match v {
        0 => Ok(QuicksilverGeneratorState::Inactive),
        1 => Ok(QuicksilverGeneratorState::Countdown),
        2 => Ok(QuicksilverGeneratorState::Active),
        3 => Ok(QuicksilverGeneratorState::Cooldown),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "QuicksilverGeneratorState",
            value: v,
        }),
    }
}

fn instrument_from_i32(v: i32) -> PersistResult<Instrument> {
    match v {
        -1 => Ok(Instrument::None),
        0 => Ok(Instrument::Marimba),
        1 => Ok(Instrument::Bells),
        2 => Ok(Instrument::Cora),
        3 => Ok(Instrument::Glock),
        4 => Ok(Instrument::GlockLow),
        5 => Ok(Instrument::MusicBox),
        6 => Ok(Instrument::Organ),
        7 => Ok(Instrument::Piano),
        8 => Ok(Instrument::PianoElectric),
        9 => Ok(Instrument::Plunky),
        10 => Ok(Instrument::SingingLong),
        _ => Err(PersistError::InvalidEnum {
            enum_name: "Instrument",
            value: v,
        }),
    }
}

fn progress_tracker_from_i32(v: i32) -> Option<ProgressTrackerId> {
    match v {
        0 => Some(ProgressTrackerId::Casey1),
        1 => Some(ProgressTrackerId::Casey2),
        2 => Some(ProgressTrackerId::Casey3),
        3 => Some(ProgressTrackerId::Casey4),
        4 => Some(ProgressTrackerId::Casey5),
        5 => Some(ProgressTrackerId::Casey6),
        6 => Some(ProgressTrackerId::Casey7),
        7 => Some(ProgressTrackerId::Casey8),
        8 => Some(ProgressTrackerId::Casey9),
        9 => Some(ProgressTrackerId::Casey10),
        10 => Some(ProgressTrackerId::Casey11),
        1000 => Some(ProgressTrackerId::Hobson1),
        2000 => Some(ProgressTrackerId::TimeAfterLabDocks),
        _ => None,
    }
}

const TUTORIAL_PAIRS: &[(TutorialId, &str)] = &[
    (TutorialId::BasicMovement, "BASIC_MOVEMENT"),
    (TutorialId::Jumping, "JUMPING"),
    (TutorialId::Vaccing, "VACCING"),
    (TutorialId::Shooting, "SHOOTING"),
    (TutorialId::Food, "FOOD"),
    (TutorialId::Plort, "PLORT"),
    (TutorialId::Market, "MARKET"),
    (TutorialId::Death, "DEATH"),
    (TutorialId::Garden, "GARDEN"),
    (TutorialId::Explore, "EXPLORE"),
    (TutorialId::Largo, "LARGO"),
    (TutorialId::ScienceBarn, "SCIENCE_BARN"),
    (TutorialId::ScienceSummary, "SCIENCE_SUMMARY"),
    (TutorialId::ScienceRefinery, "SCIENCE_REFINERY"),
    (TutorialId::ScienceBuilderShop, "SCIENCE_BUILDER_SHOP"),
    (TutorialId::ScienceFabricator, "SCIENCE_FABRICATOR"),
    (TutorialId::ScienceGadgetMode, "SCIENCE_GADGET_MODE"),
    (TutorialId::ScienceSites, "SCIENCE_SITES"),
    (TutorialId::SciencePlaceSite, "SCIENCE_PLACE_SITE"),
    (TutorialId::ScienceWrapup, "SCIENCE_WRAPUP"),
    (TutorialId::Map, "MAP"),
    (TutorialId::Water, "WATER"),
    (TutorialId::EnterZoneOgdenRanch, "ENTER_ZONE_OGDEN_RANCH"),
    (TutorialId::EnterZoneMochiRanch, "ENTER_ZONE_MOCHI_RANCH"),
    (TutorialId::RaceStart, "RACE_START"),
    (TutorialId::RaceGenerator, "RACE_GENERATOR"),
    (TutorialId::RaceCheckpoint, "RACE_CHECKPOINT"),
    (TutorialId::RacePulseshot, "RACE_PULSESHOT"),
    (TutorialId::RacePowerup, "RACE_POWERUP"),
    (TutorialId::RaceEnergyboost, "RACE_ENERGYBOOST"),
    (TutorialId::ModeTimeLimit, "MODE_TIME_LIMIT"),
    (TutorialId::EnterZoneViktorLab, "ENTER_ZONE_VIKTOR_LAB"),
    (TutorialId::SlimulationsSlimepedia, "SLIMULATIONS_SLIMEPEDIA"),
    (TutorialId::SlimulationsStart1, "SLIMULATIONS_START_1"),
    (TutorialId::SlimulationsStart2, "SLIMULATIONS_START_2"),
    (TutorialId::SlimulationsDebugSpray, "SLIMULATIONS_DEBUG_SPRAY"),
    (TutorialId::SlimulationsDamage, "SLIMULATIONS_DAMAGE"),
    (
        TutorialId::SlimulationsExitAvailable,
        "SLIMULATIONS_EXIT_AVAILABLE",
    ),
    (TutorialId::WildsSlimepedia, "WILDS_SLIMEPEDIA"),
    (TutorialId::ValleySlimepedia, "VALLEY_SLIMEPEDIA"),
    (TutorialId::AppearanceUi, "APPEARANCE_UI"),
];

fn tutorial_csharp_name(t: TutorialId) -> &'static str {
    TUTORIAL_PAIRS
        .iter()
        .find(|(id, _)| *id == t)
        .map(|(_, name)| *name)
        .unwrap_or("BASIC_MOVEMENT")
}

fn tutorial_from_name(s: &str) -> Option<TutorialId> {
    TUTORIAL_PAIRS
        .iter()
        .find(|(_, name)| name.eq_ignore_ascii_case(s))
        .map(|(id, _)| *id)
}
