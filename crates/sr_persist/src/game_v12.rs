//! Binary `GameV12` dataset and its top-level sub-datasets (`WorldV22`, `PlayerV14`,
//! `RanchV07`, `ActorDataV09`, `PediaV03`, `GameAchieveV03`, `HolidayDirectorV02`,
//! `AppearancesV01`, `InstrumentV01`).

use std::collections::HashMap;
use std::io::{Read, Write};

use crate::binary::{
    BinReader, BinWriter, PersistError, PersistResult, PersistedDataset, DATA_PAYLOAD_BEGIN,
    DATA_PAYLOAD_END, ELEMENT_SEPARATOR, SECTION_SEPARATOR,
};
use crate::types::{
    AmmoDataV02, DecorizerV01, EchoNoteGordoV01, ExchangeOfferV04, FirestormV01,
    GameSummaryV04, GlitchSlimulationV02, GordoV01, MailV02, PlacedGadgetV08,
    QuicksilverEnergyGeneratorV02, ResourceCycleDataV03, ResourceWaterV03, SlimeEmotionDataV02,
    TreasurePodV01, Vector3V02,
};

/// Helper for dictionaries keyed by `Vector3V02` (where `f32` does not implement `Eq + Hash`).
fn read_vec3_dict<R: Read, V, F>(
    reader: &mut BinReader<R>,
    mut val_fn: F,
) -> PersistResult<Vec<(Vector3V02, V)>>
where
    F: FnMut(&mut BinReader<R>) -> PersistResult<V>,
{
    let count = reader.read_i32()?;
    if count < 0 {
        return Err(PersistError::InvalidCount(count));
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let k = Vector3V02::load(reader)?;
        let v = val_fn(reader)?;
        reader.read_marker(ELEMENT_SEPARATOR)?;
        out.push((k, v));
    }
    Ok(out)
}

fn write_vec3_dict<W: Write, V, F>(
    writer: &mut BinWriter<W>,
    entries: &[(Vector3V02, V)],
    mut val_fn: F,
) -> PersistResult<()>
where
    F: FnMut(&mut BinWriter<W>, &V) -> PersistResult<()>,
{
    writer.write_i32(entries.len() as i32)?;
    for (k, v) in entries {
        k.write(writer)?;
        val_fn(writer, v)?;
        writer.write_i16(ELEMENT_SEPARATOR)?;
    }
    Ok(())
}

/// Port of `WorldV22` (`"SRW"` v22).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldV22 {
    pub world_time: f64,
    pub econ_seed: f32,
    pub daily_offer_create_time: f64,
    pub last_offer_rancher_ids: Vec<String>,
    pub pending_offer_rancher_ids: Vec<String>,
    pub weather_until: f64,
    pub weather: i32,
    pub offers: HashMap<i32, ExchangeOfferV04>,
    pub econ_saturations: HashMap<i32, f32>,
    pub teleport_node_activations: HashMap<String, bool>,
    pub animal_spawner_times: Vec<(Vector3V02, f64)>,
    pub liquid_source_units: HashMap<String, f32>,
    pub spawner_trigger_times: Vec<(Vector3V02, f64)>,
    pub gordos: HashMap<String, GordoV01>,
    pub resource_spawner_water: Vec<(Vector3V02, ResourceWaterV03)>,
    pub placed_gadgets: HashMap<String, PlacedGadgetV08>,
    pub treasure_pods: HashMap<String, TreasurePodV01>,
    pub switches: HashMap<String, i32>,
    pub puzzle_slots_filled: HashMap<String, bool>,
    pub occupied_phase_sites: HashMap<String, bool>,
    pub firestorm: FirestormV01,
    pub oasis_states: HashMap<String, bool>,
    pub active_ginger_patches: Vec<String>,
    pub quicksilver_energy_generators: HashMap<String, QuicksilverEnergyGeneratorV02>,
    pub echo_note_gordos: HashMap<String, Option<EchoNoteGordoV01>>,
    pub glitch: Option<GlitchSlimulationV02>,
}

impl PersistedDataset for WorldV22 {
    const IDENTIFIER: &'static str = "SRW";
    const VERSION: u32 = 22;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let world_time = reader.read_f64()?;
        let econ_seed = reader.read_f32()?;
        let daily_offer_create_time = reader.read_f64()?;
        let last_offer_rancher_ids = reader.read_list(|r| r.read_string())?;
        let pending_offer_rancher_ids = reader.read_list(|r| r.read_string())?;
        let weather_until = reader.read_f64()?;
        let weather = reader.read_i32()?;
        let offers = reader.read_dict(|r| r.read_i32(), ExchangeOfferV04::load)?;
        let econ_saturations = reader.read_dict(|r| r.read_i32(), |r| r.read_f32())?;
        let teleport_node_activations =
            reader.read_dict(|r| r.read_string(), |r| r.read_bool())?;
        let animal_spawner_times = read_vec3_dict(reader, |r| r.read_f64())?;
        let liquid_source_units = reader.read_dict(|r| r.read_string(), |r| r.read_f32())?;
        let spawner_trigger_times = read_vec3_dict(reader, |r| r.read_f64())?;
        let gordos = reader.read_dict(|r| r.read_string(), GordoV01::load)?;
        let resource_spawner_water = read_vec3_dict(reader, ResourceWaterV03::load)?;
        let placed_gadgets =
            reader.read_dict(|r| r.read_string(), PlacedGadgetV08::load)?;
        let treasure_pods =
            reader.read_dict(|r| r.read_string(), TreasurePodV01::load)?;
        let switches = reader.read_dict(|r| r.read_string(), |r| r.read_i32())?;
        let puzzle_slots_filled = reader.read_dict(|r| r.read_string(), |r| r.read_bool())?;
        let occupied_phase_sites = reader.read_dict(|r| r.read_string(), |r| r.read_bool())?;
        let firestorm = FirestormV01::load(reader)?;
        let oasis_states = reader.read_dict(|r| r.read_string(), |r| r.read_bool())?;
        let active_ginger_patches = reader.read_list(|r| r.read_string())?;
        let quicksilver_energy_generators =
            reader.read_dict(|r| r.read_string(), QuicksilverEnergyGeneratorV02::load)?;
        let echo_note_gordos = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(EchoNoteGordoV01::load),
        )?;
        let glitch = reader.read_persistable_opt(GlitchSlimulationV02::load)?;

        Ok(Self {
            world_time,
            econ_seed,
            daily_offer_create_time,
            last_offer_rancher_ids,
            pending_offer_rancher_ids,
            weather_until,
            weather,
            offers,
            econ_saturations,
            teleport_node_activations,
            animal_spawner_times,
            liquid_source_units,
            spawner_trigger_times,
            gordos,
            resource_spawner_water,
            placed_gadgets,
            treasure_pods,
            switches,
            puzzle_slots_filled,
            occupied_phase_sites,
            firestorm,
            oasis_states,
            active_ginger_patches,
            quicksilver_energy_generators,
            echo_note_gordos,
            glitch,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.world_time)?;
        writer.write_f32(self.econ_seed)?;
        writer.write_f64(self.daily_offer_create_time)?;
        writer.write_list(&self.last_offer_rancher_ids, |w, s| w.write_string(s))?;
        writer.write_list(&self.pending_offer_rancher_ids, |w, s| w.write_string(s))?;
        writer.write_f64(self.weather_until)?;
        writer.write_i32(self.weather)?;
        writer.write_dict(&self.offers, |w, &k| w.write_i32(k), |w, v| v.write(w))?;
        writer.write_dict(
            &self.econ_saturations,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_f32(v),
        )?;
        writer.write_dict(
            &self.teleport_node_activations,
            |w, k| w.write_string(k),
            |w, &v| w.write_bool(v),
        )?;
        write_vec3_dict(writer, &self.animal_spawner_times, |w, &v| w.write_f64(v))?;
        writer.write_dict(
            &self.liquid_source_units,
            |w, k| w.write_string(k),
            |w, &v| w.write_f32(v),
        )?;
        write_vec3_dict(writer, &self.spawner_trigger_times, |w, &v| w.write_f64(v))?;
        writer.write_dict(&self.gordos, |w, k| w.write_string(k), |w, v| v.write(w))?;
        write_vec3_dict(writer, &self.resource_spawner_water, |w, v| v.write(w))?;
        writer.write_dict(
            &self.placed_gadgets,
            |w, k| w.write_string(k),
            |w, v| v.write(w),
        )?;
        writer.write_dict(
            &self.treasure_pods,
            |w, k| w.write_string(k),
            |w, v| v.write(w),
        )?;
        writer.write_dict(&self.switches, |w, k| w.write_string(k), |w, &v| w.write_i32(v))?;
        writer.write_dict(
            &self.puzzle_slots_filled,
            |w, k| w.write_string(k),
            |w, &v| w.write_bool(v),
        )?;
        writer.write_dict(
            &self.occupied_phase_sites,
            |w, k| w.write_string(k),
            |w, &v| w.write_bool(v),
        )?;
        self.firestorm.write(writer)?;
        writer.write_dict(
            &self.oasis_states,
            |w, k| w.write_string(k),
            |w, &v| w.write_bool(v),
        )?;
        writer.write_list(&self.active_ginger_patches, |w, s| w.write_string(s))?;
        writer.write_dict(
            &self.quicksilver_energy_generators,
            |w, k| w.write_string(k),
            |w, v| v.write(w),
        )?;
        writer.write_dict(
            &self.echo_note_gordos,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, g| g.write(wr)),
        )?;
        writer.write_persistable_opt(self.glitch.as_ref(), |w, g| g.write(w))?;
        Ok(())
    }
}

/// Port of `PlayerState.UpgradeLockData` / `GadgetDirector.BlueprintLockData`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LockDataV01 {
    pub timed_lock: bool,
    pub locked_until: f64,
}

/// Port of `PlayerV14` (`"SRPL"` v14).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerV14 {
    pub health: i32,
    pub energy: i32,
    pub radiation: i32,
    pub currency: i32,
    pub keys: i32,
    pub currency_ever_collected: i32,
    pub version: String,
    pub game_mode: i32,
    pub game_icon_id: i32,
    pub player_pos: Vector3V02,
    pub player_rot_euler: Vector3V02,
    pub upgrades: Vec<i32>,
    pub ammo: HashMap<i32, Vec<AmmoDataV02>>,
    pub mail: Vec<MailV02>,
    pub avail_upgrades: Vec<i32>,
    pub upgrade_locks: HashMap<i32, LockDataV01>,
    pub progress: HashMap<i32, i32>,
    pub delayed_progress: HashMap<i32, f64>,
    pub blueprints: Vec<i32>,
    pub avail_blueprints: Vec<i32>,
    pub blueprint_locks: HashMap<i32, LockDataV01>,
    pub gadgets: HashMap<i32, i32>,
    pub craft_mat_counts: HashMap<i32, i32>,
    pub region_set_id: i32,
    pub unlocked_zone_maps: Vec<i32>,
    pub end_game_time: Option<f64>,
    pub decorizer: Option<DecorizerV01>,
}

impl PersistedDataset for PlayerV14 {
    const IDENTIFIER: &'static str = "SRPL";
    const VERSION: u32 = 14;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let health = reader.read_i32()?;
        let energy = reader.read_i32()?;
        let radiation = reader.read_i32()?;
        let currency = reader.read_i32()?;
        let keys = reader.read_i32()?;
        let currency_ever_collected = reader.read_i32()?;
        let version = reader.read_string()?;
        let game_mode = reader.read_i32()?;
        let game_icon_id = reader.read_i32()?;
        let player_pos = Vector3V02::load(reader)?;
        let player_rot_euler = Vector3V02::load(reader)?;
        let upgrades = reader.read_list(|r| r.read_i32())?;
        let ammo = reader.read_dict(
            |r| r.read_i32(),
            |r| r.read_list(AmmoDataV02::load),
        )?;
        let mail = reader.read_list(MailV02::load)?;
        let avail_upgrades = reader.read_list(|r| r.read_i32())?;
        let upgrade_locks = reader.read_dict(
            |r| r.read_i32(),
            |r| {
                Ok(LockDataV01 {
                    timed_lock: r.read_bool()?,
                    locked_until: r.read_f64()?,
                })
            },
        )?;
        let progress = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let delayed_progress = reader.read_dict(|r| r.read_i32(), |r| r.read_f64())?;
        let blueprints = reader.read_list(|r| r.read_i32())?;
        let avail_blueprints = reader.read_list(|r| r.read_i32())?;
        let blueprint_locks = reader.read_dict(
            |r| r.read_i32(),
            |r| {
                Ok(LockDataV01 {
                    timed_lock: r.read_bool()?,
                    locked_until: r.read_f64()?,
                })
            },
        )?;
        let gadgets = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let craft_mat_counts = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let region_set_id = reader.read_i32()?;
        let unlocked_zone_maps = reader.read_list(|r| r.read_i32())?;
        let end_game_time = reader.read_nullable_f64()?;
        let decorizer = reader.read_persistable_opt(DecorizerV01::load)?;

        Ok(Self {
            health,
            energy,
            radiation,
            currency,
            keys,
            currency_ever_collected,
            version,
            game_mode,
            game_icon_id,
            player_pos,
            player_rot_euler,
            upgrades,
            ammo,
            mail,
            avail_upgrades,
            upgrade_locks,
            progress,
            delayed_progress,
            blueprints,
            avail_blueprints,
            blueprint_locks,
            gadgets,
            craft_mat_counts,
            region_set_id,
            unlocked_zone_maps,
            end_game_time,
            decorizer,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.health)?;
        writer.write_i32(self.energy)?;
        writer.write_i32(self.radiation)?;
        writer.write_i32(self.currency)?;
        writer.write_i32(self.keys)?;
        writer.write_i32(self.currency_ever_collected)?;
        writer.write_string(&self.version)?;
        writer.write_i32(self.game_mode)?;
        writer.write_i32(self.game_icon_id)?;
        self.player_pos.write(writer)?;
        self.player_rot_euler.write(writer)?;
        writer.write_list(&self.upgrades, |w, &u| w.write_i32(u))?;
        writer.write_dict(
            &self.ammo,
            |w, &k| w.write_i32(k),
            |w, list| w.write_list(list, |wr, a| a.write(wr)),
        )?;
        writer.write_list(&self.mail, |w, m| m.write(w))?;
        writer.write_list(&self.avail_upgrades, |w, &u| w.write_i32(u))?;
        writer.write_dict(
            &self.upgrade_locks,
            |w, &k| w.write_i32(k),
            |w, v| {
                w.write_bool(v.timed_lock)?;
                w.write_f64(v.locked_until)
            },
        )?;
        writer.write_dict(&self.progress, |w, &k| w.write_i32(k), |w, &v| w.write_i32(v))?;
        writer.write_dict(
            &self.delayed_progress,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_f64(v),
        )?;
        writer.write_list(&self.blueprints, |w, &b| w.write_i32(b))?;
        writer.write_list(&self.avail_blueprints, |w, &b| w.write_i32(b))?;
        writer.write_dict(
            &self.blueprint_locks,
            |w, &k| w.write_i32(k),
            |w, v| {
                w.write_bool(v.timed_lock)?;
                w.write_f64(v.locked_until)
            },
        )?;
        writer.write_dict(&self.gadgets, |w, &k| w.write_i32(k), |w, &v| w.write_i32(v))?;
        writer.write_dict(
            &self.craft_mat_counts,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_i32(v),
        )?;
        writer.write_i32(self.region_set_id)?;
        writer.write_list(&self.unlocked_zone_maps, |w, &z| w.write_i32(z))?;
        writer.write_nullable_f64(self.end_game_time)?;
        writer.write_persistable_opt(self.decorizer.as_ref(), |w, d| d.write(w))?;
        Ok(())
    }
}

/// Port of `LandPlotV08` (`"SRLP"` v8).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LandPlotV08 {
    pub feeder_next_time: f64,
    pub feeder_pending_count: i32,
    pub feeder_cycle_speed: i32,
    pub collector_next_time: f64,
    pub attached_death_time: f64,
    pub type_id: i32,
    pub attached_id: i32,
    pub id: String,
    pub upgrades: Vec<i32>,
    pub silo_ammo: HashMap<i32, Vec<AmmoDataV02>>,
    pub silo_activator_indices: Vec<i32>,
    pub ash_units: f32,
}

impl PersistedDataset for LandPlotV08 {
    const IDENTIFIER: &'static str = "SRLP";
    const VERSION: u32 = 8;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let feeder_next_time = reader.read_f64()?;
        let feeder_pending_count = reader.read_i32()?;
        let feeder_cycle_speed = reader.read_i32()?;
        let collector_next_time = reader.read_f64()?;
        let attached_death_time = reader.read_f64()?;
        let type_id = reader.read_i32()?;
        let attached_id = reader.read_i32()?;
        let id = reader.read_string()?;
        let upgrades = reader.read_list(|r| r.read_i32())?;
        let silo_ammo = reader.read_dict(
            |r| r.read_i32(),
            |r| r.read_list(AmmoDataV02::load),
        )?;
        let silo_activator_indices = reader.read_list(|r| r.read_i32())?;
        let ash_units = reader.read_f32()?;
        Ok(Self {
            feeder_next_time,
            feeder_pending_count,
            feeder_cycle_speed,
            collector_next_time,
            attached_death_time,
            type_id,
            attached_id,
            id,
            upgrades,
            silo_ammo,
            silo_activator_indices,
            ash_units,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.feeder_next_time)?;
        writer.write_i32(self.feeder_pending_count)?;
        writer.write_i32(self.feeder_cycle_speed)?;
        writer.write_f64(self.collector_next_time)?;
        writer.write_f64(self.attached_death_time)?;
        writer.write_i32(self.type_id)?;
        writer.write_i32(self.attached_id)?;
        writer.write_string(&self.id)?;
        writer.write_list(&self.upgrades, |w, &u| w.write_i32(u))?;
        writer.write_dict(
            &self.silo_ammo,
            |w, &k| w.write_i32(k),
            |w, list| w.write_list(list, |wr, a| a.write(wr)),
        )?;
        writer.write_list(&self.silo_activator_indices, |w, &i| w.write_i32(i))?;
        writer.write_f32(self.ash_units)?;
        Ok(())
    }
}

/// Port of `RanchV07` (`"SRRANCH"` v7).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RanchV07 {
    pub plots: Vec<LandPlotV08>,
    pub access_door_states: HashMap<String, i32>,
    pub palettes: HashMap<i32, i32>,
    pub ranch_fast_forward: HashMap<String, f64>,
}

impl PersistedDataset for RanchV07 {
    const IDENTIFIER: &'static str = "SRRANCH";
    const VERSION: u32 = 7;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let plots = reader.read_list(LandPlotV08::load)?;
        let access_door_states = reader.read_dict(|r| r.read_string(), |r| r.read_i32())?;
        let palettes = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let ranch_fast_forward = reader.read_dict(|r| r.read_string(), |r| r.read_f64())?;
        Ok(Self {
            plots,
            access_door_states,
            palettes,
            ranch_fast_forward,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_list(&self.plots, |w, p| p.write(w))?;
        writer.write_dict(
            &self.access_door_states,
            |w, k| w.write_string(k),
            |w, &v| w.write_i32(v),
        )?;
        writer.write_dict(&self.palettes, |w, &k| w.write_i32(k), |w, &v| w.write_i32(v))?;
        writer.write_dict(
            &self.ranch_fast_forward,
            |w, k| w.write_string(k),
            |w, &v| w.write_f64(v),
        )?;
        Ok(())
    }
}

/// Port of `ActorDataV09` (`"SRAD"` v9).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorDataV09 {
    pub pos: Vector3V02,
    pub rot: Vector3V02,
    pub actor_id: i64,
    pub type_id: i32,
    pub emotions: SlimeEmotionDataV02,
    pub transform_time: f64,
    pub reproduce_time: f64,
    pub destroy_time: f64,
    pub cycle_data: ResourceCycleDataV03,
    pub disabled_at_time: Option<f64>,
    pub is_feral: bool,
    pub fashions: Vec<i32>,
    pub is_glitch: bool,
    pub region_set_id: i32,
}

impl PersistedDataset for ActorDataV09 {
    const IDENTIFIER: &'static str = "SRAD";
    const VERSION: u32 = 9;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let pos = Vector3V02::load(reader)?;
        let rot = Vector3V02::load(reader)?;
        let actor_id = reader.read_i64()?;
        let type_id = reader.read_i32()?;
        let emotions = SlimeEmotionDataV02::load(reader)?;
        let transform_time = reader.read_f64()?;
        let reproduce_time = reader.read_f64()?;
        let destroy_time = reader.read_f64()?;
        let cycle_data = ResourceCycleDataV03::load(reader)?;
        let disabled_at_time = reader.read_nullable_f64()?;
        let is_feral = reader.read_bool()?;
        let fashions = reader.read_list(|r| r.read_i32())?;
        let is_glitch = reader.read_bool()?;
        let region_set_id = reader.read_i32()?;
        Ok(Self {
            pos,
            rot,
            actor_id,
            type_id,
            emotions,
            transform_time,
            reproduce_time,
            destroy_time,
            cycle_data,
            disabled_at_time,
            is_feral,
            fashions,
            is_glitch,
            region_set_id,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        self.pos.write(writer)?;
        self.rot.write(writer)?;
        writer.write_i64(self.actor_id)?;
        writer.write_i32(self.type_id)?;
        self.emotions.write(writer)?;
        writer.write_f64(self.transform_time)?;
        writer.write_f64(self.reproduce_time)?;
        writer.write_f64(self.destroy_time)?;
        self.cycle_data.write(writer)?;
        writer.write_nullable_f64(self.disabled_at_time)?;
        writer.write_bool(self.is_feral)?;
        writer.write_list(&self.fashions, |w, &f| w.write_i32(f))?;
        writer.write_bool(self.is_glitch)?;
        writer.write_i32(self.region_set_id)?;
        Ok(())
    }
}

/// Port of `PediaV03` (`"SRPED"` v3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PediaV03 {
    pub progress_given_for_pedia_count: i32,
    pub unlocked_ids: Vec<String>,
    pub completed_tuts: Vec<String>,
    pub popup_queue: Vec<String>,
}

impl PersistedDataset for PediaV03 {
    const IDENTIFIER: &'static str = "SRPED";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            progress_given_for_pedia_count: reader.read_i32()?,
            unlocked_ids: reader.read_list(|r| r.read_string())?,
            completed_tuts: reader.read_list(|r| r.read_string())?,
            popup_queue: reader.read_list(|r| r.read_string())?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.progress_given_for_pedia_count)?;
        writer.write_list(&self.unlocked_ids, |w, s| w.write_string(s))?;
        writer.write_list(&self.completed_tuts, |w, s| w.write_string(s))?;
        writer.write_list(&self.popup_queue, |w, s| w.write_string(s))?;
        Ok(())
    }
}

/// Port of `GameAchieveV03` (`"SRGA"` v3).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameAchieveV03 {
    pub game_float_stat_dict: HashMap<i32, f32>,
    pub game_double_stat_dict: HashMap<i32, f64>,
    pub game_int_stat_dict: HashMap<i32, i32>,
    pub game_id_dict_stat_dict: HashMap<i32, HashMap<i32, i32>>,
}

impl PersistedDataset for GameAchieveV03 {
    const IDENTIFIER: &'static str = "SRGA";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let game_float_stat_dict = reader.read_dict(|r| r.read_i32(), |r| r.read_f32())?;
        let game_double_stat_dict = reader.read_dict(|r| r.read_i32(), |r| r.read_f64())?;
        let game_int_stat_dict = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let game_id_dict_stat_dict = reader.read_dict(
            |r| r.read_i32(),
            |r| r.read_dict(|r2| r2.read_i32(), |r2| r2.read_i32()),
        )?;
        Ok(Self {
            game_float_stat_dict,
            game_double_stat_dict,
            game_int_stat_dict,
            game_id_dict_stat_dict,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_dict(
            &self.game_float_stat_dict,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_f32(v),
        )?;
        writer.write_dict(
            &self.game_double_stat_dict,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_f64(v),
        )?;
        writer.write_dict(
            &self.game_int_stat_dict,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_i32(v),
        )?;
        writer.write_dict(
            &self.game_id_dict_stat_dict,
            |w, &k| w.write_i32(k),
            |w, inner| w.write_dict(inner, |w2, &ik| w2.write_i32(ik), |w2, &iv| w2.write_i32(iv)),
        )?;
        Ok(())
    }
}

/// Port of `HolidayDirectorV02` (`"SRHD"` v2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HolidayDirectorV02 {
    pub event_gordos: Vec<String>,
    pub event_echo_note_gordos: Vec<String>,
}

impl PersistedDataset for HolidayDirectorV02 {
    const IDENTIFIER: &'static str = "SRHD";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            event_gordos: reader.read_list(|r| r.read_string())?,
            event_echo_note_gordos: reader.read_list(|r| r.read_string())?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_list(&self.event_gordos, |w, s| w.write_string(s))?;
        writer.write_list(&self.event_echo_note_gordos, |w, s| w.write_string(s))?;
        Ok(())
    }
}

/// Port of `AppearancesV01` (`"SRAPP"` v1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppearancesV01 {
    pub unlocks: HashMap<i32, Vec<i32>>,
    pub selections: HashMap<i32, i32>,
}

impl PersistedDataset for AppearancesV01 {
    const IDENTIFIER: &'static str = "SRAPP";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let unlocks = reader.read_dict(|r| r.read_i32(), |r| r.read_list(|r2| r2.read_i32()))?;
        let selections = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        Ok(Self {
            unlocks,
            selections,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_dict(
            &self.unlocks,
            |w, &k| w.write_i32(k),
            |w, list| w.write_list(list, |w2, &v| w2.write_i32(v)),
        )?;
        writer.write_dict(
            &self.selections,
            |w, &k| w.write_i32(k),
            |w, &v| w.write_i32(v),
        )?;
        Ok(())
    }
}

/// Port of `InstrumentV01` (`"SRINSTR"` v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentV01 {
    pub unlocks: Vec<i32>,
    pub selection: i32,
}

impl Default for InstrumentV01 {
    fn default() -> Self {
        Self {
            unlocks: Vec::new(),
            selection: -1,
        }
    }
}

impl PersistedDataset for InstrumentV01 {
    const IDENTIFIER: &'static str = "SRINSTR";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            unlocks: reader.read_list(|r| r.read_i32())?,
            selection: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_list(&self.unlocks, |w, &u| w.write_i32(u))?;
        writer.write_i32(self.selection)?;
        Ok(())
    }
}

/// Port of `GameV12` (`"SRGAME"` v12) — top-level save file format.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameV12 {
    pub game_name: String,
    pub display_name: String,
    pub summary: GameSummaryV04,
    pub world: WorldV22,
    pub player: PlayerV14,
    pub ranch: RanchV07,
    pub actors: Vec<ActorDataV09>,
    pub pedia: PediaV03,
    pub achieve: GameAchieveV03,
    pub holiday: HolidayDirectorV02,
    pub appearances: AppearancesV01,
    pub instrument: InstrumentV01,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadedSummary {
    pub game_name: String,
    pub display_name: String,
    pub summary: GameSummaryV04,
}

impl GameV12 {
    /// Reads only the header and summary section (`GamePersistedDataSet.LoadSummary`),
    /// without parsing the full world/actor payload.
    pub fn load_summary_only<R: Read>(reader: &mut BinReader<R>) -> PersistResult<LoadedSummary> {
        reader.read_header(Self::IDENTIFIER, Self::VERSION)?;
        reader.read_marker(DATA_PAYLOAD_BEGIN)?;
        let game_name = reader.read_string()?;
        let display_name = reader.read_string()?;
        let summary = GameSummaryV04::load(reader)?;
        Ok(LoadedSummary {
            game_name,
            display_name,
            summary,
        })
    }
}

impl PersistedDataset for GameV12 {
    const IDENTIFIER: &'static str = "SRGAME";
    const VERSION: u32 = 12;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let game_name = reader.read_string()?;
        let display_name = reader.read_string()?;
        let summary = GameSummaryV04::load(reader)?;
        let world = WorldV22::load(reader)?;
        let player = PlayerV14::load(reader)?;
        let ranch = RanchV07::load(reader)?;
        reader.read_marker(SECTION_SEPARATOR)?;
        let actors = reader.read_list(ActorDataV09::load)?;
        reader.read_marker(SECTION_SEPARATOR)?;
        let pedia = PediaV03::load(reader)?;
        let achieve = GameAchieveV03::load(reader)?;
        let holiday = HolidayDirectorV02::load(reader)?;
        let appearances = AppearancesV01::load(reader)?;
        let instrument = InstrumentV01::load(reader)?;
        Ok(Self {
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
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_string(&self.game_name)?;
        writer.write_string(&self.display_name)?;
        self.summary.write(writer)?;
        self.world.write(writer)?;
        self.player.write(writer)?;
        self.ranch.write(writer)?;
        writer.write_i16(SECTION_SEPARATOR)?;
        writer.write_list(&self.actors, |w, a| a.write(w))?;
        writer.write_i16(SECTION_SEPARATOR)?;
        self.pedia.write(writer)?;
        self.achieve.write(writer)?;
        self.holiday.write(writer)?;
        self.appearances.write(writer)?;
        self.instrument.write(writer)?;
        Ok(())
    }
}

impl GameV12 {
    pub fn to_bytes(&self) -> PersistResult<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let mut w = BinWriter::new(&mut buf);
            self.write(&mut w)?;
        }
        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> PersistResult<Self> {
        let mut r = BinReader::new(std::io::Cursor::new(bytes));
        Self::load(&mut r)
    }
}
