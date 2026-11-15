//! Auxiliary state enums used across `sr_model` and `sr_persist`, matching C# discriminants.

use serde::{Deserialize, Serialize};

/// Port of `ResourceCycle.State`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ResourceCycleState {
    Unripe = 0,
    Ripe = 1,
    #[default]
    Edible = 2,
    Rotten = 3,
}

/// Port of `AccessDoor.State`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AccessDoorState {
    #[default]
    Locked = 0,
    Open = 1,
    Closed = 2,
}

/// Port of `TreasurePod.State`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TreasurePodState {
    #[default]
    Locked = 0,
    Open = 1,
}

/// Port of `SwitchHandler.State`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SwitchState {
    #[default]
    Up = 0,
    Down = 1,
}

/// Port of `SlimeFeeder.FeedSpeed`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FeedSpeed {
    #[default]
    Normal = 0,
    Slow = 1,
    Fast = 2,
}

/// Port of `SiloStorage.StorageType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StorageType {
    #[default]
    NonSlimes = 0,
    Plort = 1,
    Food = 2,
    Crafting = 3,
    Elder = 4,
}

/// Port of `MailDirector.Type`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MailType {
    #[default]
    Personal = 0,
    Upgrade = 1,
    Exchange = 2,
}

/// Port of `ExchangeDirector.OfferType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OfferType {
    #[default]
    General = 0,
    Ogden = 1,
    OgdenRecur = 2,
    Mochi = 3,
    MochiRecur = 4,
    Viktor = 5,
    ViktorRecur = 6,
}

/// Port of `AmbianceDirector.Weather`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Weather {
    #[default]
    None = 0,
    Rain = 1,
}

/// Port of `FirestormActivator.Mode`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FirestormMode {
    #[default]
    Idle = 0,
    Preparing = 1,
    Active = 2,
}

/// Port of `RanchDirector.PaletteType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaletteType {
    RanchTech = 0,
    House = 1,
    Vac = 2,
    OgdenTech = 3,
    OgdenHouse = 4,
    MochiTech = 5,
    MochiHouse = 6,
    Valley = 7,
    ViktorTech = 8,
    ViktorHouse = 9,
}

/// Port of `RanchDirector.Palette`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Palette {
    #[default]
    Default = 0,
    Palette01 = 1,
    Palette02 = 2,
    Palette03 = 3,
    Palette04 = 4,
    Palette05 = 5,
    Palette06 = 6,
    Palette07 = 7,
    Palette08 = 8,
    Palette09 = 9,
    Palette10 = 10,
    Palette11 = 11,
    Palette12 = 12,
    Palette13 = 13,
    Palette14 = 14,
    Palette15 = 15,
    Palette16 = 16,
    Palette17 = 17,
    Palette18 = 18,
    Palette19 = 19,
    Palette20 = 20,
    Palette21 = 21,
    Palette22 = 22,
    Palette23 = 23,
    Palette24 = 24,
    Palette25 = 25,
    Palette26 = 26,
    Palette27 = 27,
    Palette28 = 28,
    Palette29 = 29,
    Palette30 = 30,
    Ogden = 1000,
    Mochi = 1001,
    Viktor = 1002,
}

/// Port of `SpawnResource.Id`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SpawnResourceId {
    #[default]
    None = 0,
    CuberryTree = 1,
    MangoTree = 2,
    CarrotPatch = 3,
    OcaocaPatch = 4,
    PearTree = 5,
    PogoTree = 6,
    ParsnipPatch = 7,
    BeetPatch = 8,
    OnionPatch = 9,
    LemonTree = 10,
    GingerPatch = 11,
    CuberryTreeDlx = 12,
    MangoTreeDlx = 13,
    CarrotPatchDlx = 14,
    OcaocaPatchDlx = 15,
    PearTreeDlx = 16,
    PogoTreeDlx = 17,
    ParsnipPatchDlx = 18,
    BeetPatchDlx = 19,
    OnionPatchDlx = 20,
    LemonTreeDlx = 21,
    GingerPatchDlx = 22,
}

/// Port of `TutorialDirector.Id`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TutorialId {
    BasicMovement = 0,
    Jumping = 1,
    Vaccing = 2,
    Shooting = 3,
    Food = 4,
    Plort = 5,
    Market = 6,
    Death = 7,
    Garden = 8,
    Explore = 9,
    Largo = 10,
    ScienceBarn = 11,
    ScienceSummary = 12,
    ScienceRefinery = 13,
    ScienceBuilderShop = 14,
    ScienceFabricator = 15,
    ScienceGadgetMode = 16,
    ScienceSites = 17,
    SciencePlaceSite = 18,
    ScienceWrapup = 19,
    Map = 20,
    Water = 21,
    EnterZoneOgdenRanch = 22,
    EnterZoneMochiRanch = 23,
    RaceStart = 24,
    RaceGenerator = 25,
    RaceCheckpoint = 26,
    RacePulseshot = 27,
    RacePowerup = 28,
    RaceEnergyboost = 29,
    ModeTimeLimit = 30,
    EnterZoneViktorLab = 31,
    SlimulationsSlimepedia = 32,
    SlimulationsStart1 = 33,
    SlimulationsStart2 = 34,
    SlimulationsDebugSpray = 35,
    SlimulationsDamage = 36,
    SlimulationsExitAvailable = 37,
    WildsSlimepedia = 38,
    ValleySlimepedia = 39,
    AppearanceUi = 40,
}

/// Port of `SlimeAppearance.AppearanceSaveSet`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AppearanceSaveSet {
    #[default]
    Classic = 0,
    SecretStyle = 1,
}

/// Port of `QuicksilverEnergyGenerator.State`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum QuicksilverGeneratorState {
    #[default]
    Inactive = 0,
    Countdown = 1,
    Active = 2,
    Cooldown = 3,
}

/// Port of `InstrumentModel.Instrument`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Instrument {
    #[default]
    None = -1,
    Marimba = 0,
    Bells = 1,
    Cora = 2,
    Glock = 3,
    GlockLow = 4,
    MusicBox = 5,
    Organ = 6,
    Piano = 7,
    PianoElectric = 8,
    Plunky = 9,
    SingingLong = 10,
    SingingShort = 11,
    Vibe = 12,
    VibeLow = 13,
    Chiptune = 14,
    Violin = 15,
    Flute = 16,
}

/// Port of `ExchangeDirector.NonIdentReward`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum NonIdentReward {
    #[default]
    None = 0,
    OgdenMix = 100,
    OgdenGarden = 101,
    OgdenRanch = 102,
    MochiExtraMile = 200,
    MochiCoop = 201,
    MochiRanch = 202,
    ViktorChickenCloner = 300,
    ViktorDeluxeDrones = 301,
    ViktorRanch = 302,
    NewbucksSmall = 10000,
    NewbucksMedium = 10001,
    NewbucksLarge = 10002,
    NewbucksHuge = 10003,
    NewbucksMochi = 10004,
    TimeExtension12h = 20000,
}
