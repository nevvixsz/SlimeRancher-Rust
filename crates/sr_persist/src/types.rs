//! Binary dataset types used by `WorldV22`, `PlayerV14`, `RanchV07`, `ActorDataV09`, and `GameV12`.

use std::collections::HashMap;
use std::io::{Read, Write};

use sr_math::Vec3;

use crate::binary::{BinReader, BinWriter, PersistError, PersistResult, PersistedDataset};

/// Port of `Vector3V02` (`"SRV3"` v2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3V02 {
    pub value: Vec3,
}

impl From<Vec3> for Vector3V02 {
    fn from(value: Vec3) -> Self {
        Self { value }
    }
}

impl PersistedDataset for Vector3V02 {
    const IDENTIFIER: &'static str = "SRV3";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let x = reader.read_f32()?;
        let y = reader.read_f32()?;
        let z = reader.read_f32()?;
        Ok(Self {
            value: Vec3::new(x, y, z),
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f32(self.value.x)?;
        writer.write_f32(self.value.y)?;
        writer.write_f32(self.value.z)?;
        Ok(())
    }
}

/// Port of `GameSummaryV04` (`"SRGSUMM"` v4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameSummaryV04 {
    pub version: String,
    pub game_mode: i32,
    pub icon_id: i32,
    pub currency: i32,
    pub pedia_count: i32,
    pub world_time: f64,
    pub save_timestamp_ticks: i64,
    pub save_timestamp_offset_mins: f64,
    pub is_game_over: bool,
    pub save_number: u64,
}

impl PersistedDataset for GameSummaryV04 {
    const IDENTIFIER: &'static str = "SRGSUMM";
    const VERSION: u32 = 4;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            version: reader.read_string()?,
            game_mode: reader.read_i32()?,
            icon_id: reader.read_i32()?,
            currency: reader.read_i32()?,
            pedia_count: reader.read_i32()?,
            world_time: reader.read_f64()?,
            save_timestamp_ticks: reader.read_i64()?,
            save_timestamp_offset_mins: reader.read_f64()?,
            is_game_over: reader.read_bool()?,
            save_number: reader.read_u64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_string(&self.version)?;
        writer.write_i32(self.game_mode)?;
        writer.write_i32(self.icon_id)?;
        writer.write_i32(self.currency)?;
        writer.write_i32(self.pedia_count)?;
        writer.write_f64(self.world_time)?;
        writer.write_i64(self.save_timestamp_ticks)?;
        writer.write_f64(self.save_timestamp_offset_mins)?;
        writer.write_bool(self.is_game_over)?;
        writer.write_u64(self.save_number)?;
        Ok(())
    }
}

/// Port of `SlimeEmotionDataV02` (`"SRSED"` v2).
/// Note: in C# `SlimeEmotionDataV02`, dictionary entries do NOT include `ELEMENT_SEPARATOR`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SlimeEmotionDataV02 {
    pub emotion_data: HashMap<i32, f32>,
}

impl PersistedDataset for SlimeEmotionDataV02 {
    const IDENTIFIER: &'static str = "SRSED";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let count = reader.read_i32()?;
        if count < 0 {
            return Err(PersistError::InvalidCount(count));
        }
        let mut emotion_data = HashMap::with_capacity(count as usize);
        for _ in 0..count {
            let k = reader.read_i32()?;
            let v = reader.read_f32()?;
            emotion_data.insert(k, v);
        }
        Ok(Self { emotion_data })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.emotion_data.len() as i32)?;
        for (&k, &v) in &self.emotion_data {
            writer.write_i32(k)?;
            writer.write_f32(v)?;
        }
        Ok(())
    }
}

/// Port of `AmmoDataV02` (`"SRAD"` v2).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AmmoDataV02 {
    pub id: i32,
    pub count: i32,
    pub emotion_data: SlimeEmotionDataV02,
}

impl PersistedDataset for AmmoDataV02 {
    const IDENTIFIER: &'static str = "SRAD";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let id = reader.read_i32()?;
        let count = reader.read_i32()?;
        let emotion_data = SlimeEmotionDataV02::load(reader)?;
        Ok(Self {
            id,
            count,
            emotion_data,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.id)?;
        writer.write_i32(self.count)?;
        self.emotion_data.write(writer)?;
        Ok(())
    }
}

/// Port of `ResourceCycleDataV03` (`"SRRCD"` v3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResourceCycleDataV03 {
    pub state: i32,
    pub progress_time: f64,
}

impl PersistedDataset for ResourceCycleDataV03 {
    const IDENTIFIER: &'static str = "SRRCD";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            state: reader.read_i32()?,
            progress_time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.state)?;
        writer.write_f64(self.progress_time)?;
        Ok(())
    }
}

/// Port of `MailV02` (`"SRMAIL"` v2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MailV02 {
    pub mail_type: i32,
    pub message_key: String,
    pub is_read: bool,
}

impl PersistedDataset for MailV02 {
    const IDENTIFIER: &'static str = "SRMAIL";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            mail_type: reader.read_i32()?,
            message_key: reader.read_string()?,
            is_read: reader.read_bool()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.mail_type)?;
        writer.write_string(&self.message_key)?;
        writer.write_bool(self.is_read)?;
        Ok(())
    }
}

/// Port of `DecorizerSettingsV01` (`"SRDZRSETTINGS"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecorizerSettingsV01 {
    pub selected: i32,
}

impl PersistedDataset for DecorizerSettingsV01 {
    const IDENTIFIER: &'static str = "SRDZRSETTINGS";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            selected: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.selected)?;
        Ok(())
    }
}

/// Port of `DecorizerV01` (`"SRDZR"` v1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecorizerV01 {
    pub contents: HashMap<i32, i32>,
    pub settings: HashMap<String, Option<DecorizerSettingsV01>>,
}

impl PersistedDataset for DecorizerV01 {
    const IDENTIFIER: &'static str = "SRDZR";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let contents = reader.read_dict(|r| r.read_i32(), |r| r.read_i32())?;
        let settings = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(DecorizerSettingsV01::load),
        )?;
        Ok(Self { contents, settings })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_dict(&self.contents, |w, &k| w.write_i32(k), |w, &v| w.write_i32(v))?;
        writer.write_dict(
            &self.settings,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, s| s.write(wr)),
        )?;
        Ok(())
    }
}

/// Port of `ItemEntryV03` (`"SRIE"` v3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemEntryV03 {
    pub id: i32,
    pub non_ident_reward: i32,
    pub count: i32,
}

impl PersistedDataset for ItemEntryV03 {
    const IDENTIFIER: &'static str = "SRIE";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            id: reader.read_i32()?,
            non_ident_reward: reader.read_i32()?,
            count: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.id)?;
        writer.write_i32(self.non_ident_reward)?;
        writer.write_i32(self.count)?;
        Ok(())
    }
}

/// Port of `RequestedItemEntryV03` (`"SRRIE"` v3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestedItemEntryV03 {
    pub id: i32,
    pub non_ident_reward: i32,
    pub count: i32,
    pub progress: i32,
}

impl PersistedDataset for RequestedItemEntryV03 {
    const IDENTIFIER: &'static str = "SRRIE";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            id: reader.read_i32()?,
            non_ident_reward: reader.read_i32()?,
            count: reader.read_i32()?,
            progress: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.id)?;
        writer.write_i32(self.non_ident_reward)?;
        writer.write_i32(self.count)?;
        writer.write_i32(self.progress)?;
        Ok(())
    }
}

/// Port of `ExchangeOfferV04` (`"SREO"` v4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExchangeOfferV04 {
    pub has_offer: bool,
    pub rancher_id: String,
    pub offer_id: String,
    pub expire_time: f64,
    pub early_exchange_time: f64,
    pub requests: Vec<RequestedItemEntryV03>,
    pub rewards: Vec<ItemEntryV03>,
}

impl PersistedDataset for ExchangeOfferV04 {
    const IDENTIFIER: &'static str = "SREO";
    const VERSION: u32 = 4;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let has_offer = reader.read_bool()?;
        if !has_offer {
            return Ok(Self::default());
        }
        let rancher_id = reader.read_string()?;
        let offer_id = reader.read_string()?;
        let expire_time = reader.read_f64()?;
        let early_exchange_time = reader.read_f64()?;
        let requests = reader.read_list(RequestedItemEntryV03::load)?;
        let rewards = reader.read_list(ItemEntryV03::load)?;
        Ok(Self {
            has_offer,
            rancher_id,
            offer_id,
            expire_time,
            early_exchange_time,
            requests,
            rewards,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_bool(self.has_offer)?;
        if self.has_offer {
            writer.write_string(&self.rancher_id)?;
            writer.write_string(&self.offer_id)?;
            writer.write_f64(self.expire_time)?;
            writer.write_f64(self.early_exchange_time)?;
            writer.write_list(&self.requests, |w, r| r.write(w))?;
            writer.write_list(&self.rewards, |w, r| r.write(w))?;
        }
        Ok(())
    }
}

/// Port of `GordoV01` (`"SRG"` v1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GordoV01 {
    pub eaten_count: i32,
    pub fashions: Vec<i32>,
}

impl PersistedDataset for GordoV01 {
    const IDENTIFIER: &'static str = "SRG";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            eaten_count: reader.read_i32()?,
            fashions: reader.read_list(|r| r.read_i32())?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.eaten_count)?;
        writer.write_list(&self.fashions, |w, &f| w.write_i32(f))?;
        Ok(())
    }
}

/// Port of `ResourceWaterV03` (`"SRRW"` v3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResourceWaterV03 {
    pub spawn: f64,
    pub water: f32,
}

impl PersistedDataset for ResourceWaterV03 {
    const IDENTIFIER: &'static str = "SRRW";
    const VERSION: u32 = 3;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            spawn: reader.read_f64()?,
            water: reader.read_f32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.spawn)?;
        writer.write_f32(self.water)?;
        Ok(())
    }
}

/// Port of `DroneStationBatteryV01` (`"SRDRSTB"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DroneStationBatteryV01 {
    pub time: f64,
}

impl PersistedDataset for DroneStationBatteryV01 {
    const IDENTIFIER: &'static str = "SRDRSTB";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.time)?;
        Ok(())
    }
}

/// Port of `DroneStationV01` (`"SRDRST"` v1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DroneStationV01 {
    pub battery: Option<DroneStationBatteryV01>,
}

impl PersistedDataset for DroneStationV01 {
    const IDENTIFIER: &'static str = "SRDRST";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            battery: reader.read_persistable_opt(DroneStationBatteryV01::load)?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_persistable_opt(self.battery.as_ref(), |w, b| b.write(w))
    }
}

/// Port of `DroneProgramV01` (`"SRDRONEPROG"` v1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DroneProgramV01 {
    pub target: String,
    pub source: String,
    pub destination: String,
}

impl PersistedDataset for DroneProgramV01 {
    const IDENTIFIER: &'static str = "SRDRONEPROG";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            target: reader.read_string()?,
            source: reader.read_string()?,
            destination: reader.read_string()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_string(&self.target)?;
        writer.write_string(&self.source)?;
        writer.write_string(&self.destination)?;
        Ok(())
    }
}

/// Port of `DroneV05` (`"SRDRONE"` v5).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DroneV05 {
    pub position: Option<Vector3V02>,
    pub rotation: Option<Vector3V02>,
    pub ammo: Option<AmmoDataV02>,
    pub fashions: Vec<i32>,
    pub no_clip: bool,
}

impl PersistedDataset for DroneV05 {
    const IDENTIFIER: &'static str = "SRDRONE";
    const VERSION: u32 = 5;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            position: reader.read_persistable_opt(Vector3V02::load)?,
            rotation: reader.read_persistable_opt(Vector3V02::load)?,
            ammo: reader.read_persistable_opt(AmmoDataV02::load)?,
            fashions: reader.read_list(|r| r.read_i32())?,
            no_clip: reader.read_bool()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_persistable_opt(self.position.as_ref(), |w, p| p.write(w))?;
        writer.write_persistable_opt(self.rotation.as_ref(), |w, r| r.write(w))?;
        writer.write_persistable_opt(self.ammo.as_ref(), |w, a| a.write(w))?;
        writer.write_list(&self.fashions, |w, &f| w.write_i32(f))?;
        writer.write_bool(self.no_clip)?;
        Ok(())
    }
}

/// Port of `DroneGadgetV01` (`"SRDRGD"` v1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DroneGadgetV01 {
    pub drone: Option<DroneV05>,
    pub station: Option<DroneStationV01>,
    pub programs: Vec<DroneProgramV01>,
}

impl PersistedDataset for DroneGadgetV01 {
    const IDENTIFIER: &'static str = "SRDRGD";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            drone: reader.read_persistable_opt(DroneV05::load)?,
            station: reader.read_persistable_opt(DroneStationV01::load)?,
            programs: reader.read_list(DroneProgramV01::load)?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_persistable_opt(self.drone.as_ref(), |w, d| d.write(w))?;
        writer.write_persistable_opt(self.station.as_ref(), |w, s| s.write(w))?;
        writer.write_list(&self.programs, |w, p| p.write(w))?;
        Ok(())
    }
}

/// Port of `PlacedGadgetV08` (`"SRPG"` v8).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlacedGadgetV08 {
    pub gadget_id: i32,
    pub y_rotation: f32,
    pub is_primary_in_link: bool,
    pub ammo: Vec<AmmoDataV02>,
    pub extractor_cycles_remaining: i32,
    pub extractor_queued_to_produce: i32,
    pub extractor_cycle_end_time: f64,
    pub extractor_next_produce_time: f64,
    pub wait_for_chargeup_time: f64,
    pub last_spawn_time: f64,
    pub bait_type_id: i32,
    pub gordo_type_id: i32,
    pub gordo_eaten_count: i32,
    pub fashions: Vec<i32>,
    pub drone: Option<DroneGadgetV01>,
}

impl PersistedDataset for PlacedGadgetV08 {
    const IDENTIFIER: &'static str = "SRPG";
    const VERSION: u32 = 8;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            gadget_id: reader.read_i32()?,
            y_rotation: reader.read_f32()?,
            is_primary_in_link: reader.read_bool()?,
            ammo: reader.read_list(AmmoDataV02::load)?,
            extractor_cycles_remaining: reader.read_i32()?,
            extractor_queued_to_produce: reader.read_i32()?,
            extractor_cycle_end_time: reader.read_f64()?,
            extractor_next_produce_time: reader.read_f64()?,
            wait_for_chargeup_time: reader.read_f64()?,
            last_spawn_time: reader.read_f64()?,
            bait_type_id: reader.read_i32()?,
            gordo_type_id: reader.read_i32()?,
            gordo_eaten_count: reader.read_i32()?,
            fashions: reader.read_list(|r| r.read_i32())?,
            drone: reader.read_persistable_opt(DroneGadgetV01::load)?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.gadget_id)?;
        writer.write_f32(self.y_rotation)?;
        writer.write_bool(self.is_primary_in_link)?;
        writer.write_list(&self.ammo, |w, a| a.write(w))?;
        writer.write_i32(self.extractor_cycles_remaining)?;
        writer.write_i32(self.extractor_queued_to_produce)?;
        writer.write_f64(self.extractor_cycle_end_time)?;
        writer.write_f64(self.extractor_next_produce_time)?;
        writer.write_f64(self.wait_for_chargeup_time)?;
        writer.write_f64(self.last_spawn_time)?;
        writer.write_i32(self.bait_type_id)?;
        writer.write_i32(self.gordo_type_id)?;
        writer.write_i32(self.gordo_eaten_count)?;
        writer.write_list(&self.fashions, |w, &f| w.write_i32(f))?;
        writer.write_persistable_opt(self.drone.as_ref(), |w, d| d.write(w))?;
        Ok(())
    }
}

/// Port of `TreasurePodV01` (`"SRTP"` v1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreasurePodV01 {
    pub state: i32,
    pub spawn_queue: Vec<i32>,
}

impl PersistedDataset for TreasurePodV01 {
    const IDENTIFIER: &'static str = "SRTP";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            state: reader.read_i32()?,
            spawn_queue: reader.read_list(|r| r.read_i32())?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.state)?;
        writer.write_list(&self.spawn_queue, |w, &id| w.write_i32(id))?;
        Ok(())
    }
}

/// Port of `FirestormV01` (`"SRF"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FirestormV01 {
    pub end_storm_time: f64,
    pub storm_preparing: bool,
    pub next_storm_time: f64,
}

impl PersistedDataset for FirestormV01 {
    const IDENTIFIER: &'static str = "SRF";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            end_storm_time: reader.read_f64()?,
            storm_preparing: reader.read_bool()?,
            next_storm_time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.end_storm_time)?;
        writer.write_bool(self.storm_preparing)?;
        writer.write_f64(self.next_storm_time)?;
        Ok(())
    }
}

/// Port of `QuicksilverEnergyGeneratorV02` (`"SRQSEG"` v2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QuicksilverEnergyGeneratorV02 {
    pub state: i32,
    pub timer: Option<f64>,
}

impl PersistedDataset for QuicksilverEnergyGeneratorV02 {
    const IDENTIFIER: &'static str = "SRQSEG";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            state: reader.read_i32()?,
            timer: reader.read_nullable_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.state)?;
        writer.write_nullable_f64(self.timer)?;
        Ok(())
    }
}

/// Port of `EchoNoteGordoV01` (`"SRENG"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EchoNoteGordoV01 {
    pub state: i32,
}

impl PersistedDataset for EchoNoteGordoV01 {
    const IDENTIFIER: &'static str = "SRENG";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            state: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.state)?;
        Ok(())
    }
}

/// Port of `GlitchTeleportDestinationV01` (`"SRGLITCH_TPD"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlitchTeleportDestinationV01 {
    pub activation_time: Option<f64>,
}

impl PersistedDataset for GlitchTeleportDestinationV01 {
    const IDENTIFIER: &'static str = "SRGLITCH_TPD";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            activation_time: reader.read_nullable_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_nullable_f64(self.activation_time)
    }
}

/// Port of `GlitchTarrNodeV01` (`"SRGLITCH_TS"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlitchTarrNodeV01 {
    pub activation_time: f64,
}

impl PersistedDataset for GlitchTarrNodeV01 {
    const IDENTIFIER: &'static str = "SRGLITCH_TS";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            activation_time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f64(self.activation_time)
    }
}

/// Port of `GlitchSlimeDataV01` (`"SRAD_GS"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlitchSlimeDataV01 {
    pub exposure_chance: f32,
    pub death_time: f64,
}

impl PersistedDataset for GlitchSlimeDataV01 {
    const IDENTIFIER: &'static str = "SRAD_GS";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            exposure_chance: reader.read_f32()?,
            death_time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_f32(self.exposure_chance)?;
        writer.write_f64(self.death_time)?;
        Ok(())
    }
}

/// Port of `GlitchImpostoDirectorV01` (`"SRGLITCH_ID"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlitchImpostoDirectorV01 {
    pub hibernation_time: Option<f64>,
}

impl PersistedDataset for GlitchImpostoDirectorV01 {
    const IDENTIFIER: &'static str = "SRGLITCH_ID";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            hibernation_time: reader.read_nullable_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_nullable_f64(self.hibernation_time)
    }
}

/// Port of `GlitchImpostoV01` (`"SRGLITCH_IP"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlitchImpostoV01 {
    pub deactivate_time: Option<f64>,
    pub cooldown_time: f64,
}

impl PersistedDataset for GlitchImpostoV01 {
    const IDENTIFIER: &'static str = "SRGLITCH_IP";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            deactivate_time: reader.read_nullable_f64()?,
            cooldown_time: reader.read_f64()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_nullable_f64(self.deactivate_time)?;
        writer.write_f64(self.cooldown_time)?;
        Ok(())
    }
}

/// Port of `GlitchStorageV01` (`"SRGLITCH_ST"` v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlitchStorageV01 {
    pub id: i32,
    pub count: i32,
}

impl PersistedDataset for GlitchStorageV01 {
    const IDENTIFIER: &'static str = "SRGLITCH_ST";
    const VERSION: u32 = 1;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        Ok(Self {
            id: reader.read_i32()?,
            count: reader.read_i32()?,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_i32(self.id)?;
        writer.write_i32(self.count)?;
        Ok(())
    }
}

/// Port of `GlitchSlimulationV02` (`"SRGLITCH"` v2).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlitchSlimulationV02 {
    pub teleporters: HashMap<String, Option<GlitchTeleportDestinationV01>>,
    pub nodes: HashMap<String, Option<GlitchTarrNodeV01>>,
    pub slimes: HashMap<i64, Option<GlitchSlimeDataV01>>,
    pub imposto_directors: HashMap<String, Option<GlitchImpostoDirectorV01>>,
    pub impostos: HashMap<String, Option<GlitchImpostoV01>>,
    pub storage: HashMap<String, Option<GlitchStorageV01>>,
}

impl PersistedDataset for GlitchSlimulationV02 {
    const IDENTIFIER: &'static str = "SRGLITCH";
    const VERSION: u32 = 2;

    fn load_data<R: Read>(reader: &mut BinReader<R>) -> PersistResult<Self> {
        let teleporters = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(GlitchTeleportDestinationV01::load),
        )?;
        let nodes = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(GlitchTarrNodeV01::load),
        )?;
        let slimes = reader.read_dict(
            |r| r.read_i64(),
            |r| r.read_persistable_opt(GlitchSlimeDataV01::load),
        )?;
        let imposto_directors = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(GlitchImpostoDirectorV01::load),
        )?;
        let impostos = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(GlitchImpostoV01::load),
        )?;
        let storage = reader.read_dict(
            |r| r.read_string(),
            |r| r.read_persistable_opt(GlitchStorageV01::load),
        )?;
        Ok(Self {
            teleporters,
            nodes,
            slimes,
            imposto_directors,
            impostos,
            storage,
        })
    }

    fn write_data<W: Write>(&self, writer: &mut BinWriter<W>) -> PersistResult<()> {
        writer.write_dict(
            &self.teleporters,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        writer.write_dict(
            &self.nodes,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        writer.write_dict(
            &self.slimes,
            |w, &k| w.write_i64(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        writer.write_dict(
            &self.imposto_directors,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        writer.write_dict(
            &self.impostos,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        writer.write_dict(
            &self.storage,
            |w, k| w.write_string(k),
            |w, v| w.write_persistable_opt(v.as_ref(), |wr, x| x.write(wr)),
        )?;
        Ok(())
    }
}
