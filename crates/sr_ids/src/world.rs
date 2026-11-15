//! Core world, ranch, player, progression, and slime diet/emotion enum IDs.

/// Port of `ZoneDirector.Zone`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ZoneId {
    None = -1,
    Ranch = 0,
    Reef = 1,
    Quarry = 2,
    Moss = 3,
    Desert = 4,
    Sea = 5,
    Ruins = 7,
    RuinsTransition = 8,
    Wilds = 9,
    OgdenRanch = 10,
    Valley = 11,
    MochiRanch = 12,
    Slimulations = 13,
    ViktorLab = 14,
}

impl TryFrom<i32> for ZoneId {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::None),
            0 => Ok(Self::Ranch),
            1 => Ok(Self::Reef),
            2 => Ok(Self::Quarry),
            3 => Ok(Self::Moss),
            4 => Ok(Self::Desert),
            5 => Ok(Self::Sea),
            7 => Ok(Self::Ruins),
            8 => Ok(Self::RuinsTransition),
            9 => Ok(Self::Wilds),
            10 => Ok(Self::OgdenRanch),
            11 => Ok(Self::Valley),
            12 => Ok(Self::MochiRanch),
            13 => Ok(Self::Slimulations),
            14 => Ok(Self::ViktorLab),
            other => Err(crate::InvalidEnumId { enum_name: "ZoneId", value: other }),
        }
    }
}

impl ZoneId {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Ranch,
        Self::Reef,
        Self::Quarry,
        Self::Moss,
        Self::Desert,
        Self::Sea,
        Self::Ruins,
        Self::RuinsTransition,
        Self::Wilds,
        Self::OgdenRanch,
        Self::Valley,
        Self::MochiRanch,
        Self::Slimulations,
        Self::ViktorLab,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Ranch => "RANCH",
            Self::Reef => "REEF",
            Self::Quarry => "QUARRY",
            Self::Moss => "MOSS",
            Self::Desert => "DESERT",
            Self::Sea => "SEA",
            Self::Ruins => "RUINS",
            Self::RuinsTransition => "RUINS_TRANSITION",
            Self::Wilds => "WILDS",
            Self::OgdenRanch => "OGDEN_RANCH",
            Self::Valley => "VALLEY",
            Self::MochiRanch => "MOCHI_RANCH",
            Self::Slimulations => "SLIMULATIONS",
            Self::ViktorLab => "VIKTOR_LAB",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "NONE" => Some(Self::None),
            "RANCH" => Some(Self::Ranch),
            "REEF" => Some(Self::Reef),
            "QUARRY" => Some(Self::Quarry),
            "MOSS" => Some(Self::Moss),
            "DESERT" => Some(Self::Desert),
            "SEA" => Some(Self::Sea),
            "RUINS" => Some(Self::Ruins),
            "RUINSTRANSITION" => Some(Self::RuinsTransition),
            "WILDS" => Some(Self::Wilds),
            "OGDENRANCH" => Some(Self::OgdenRanch),
            "VALLEY" => Some(Self::Valley),
            "MOCHIRANCH" => Some(Self::MochiRanch),
            "SLIMULATIONS" => Some(Self::Slimulations),
            "VIKTORLAB" => Some(Self::ViktorLab),
            _ => None,
        }
    }
}

/// Port of `RegionRegistry.RegionSetId`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum RegionSetId {
    Unset = -1,
    Home = 0,
    Desert = 1,
    Valley = 2,
    ViktorLab = 3,
    Slimulations = 4,
}

impl TryFrom<i32> for RegionSetId {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::Unset),
            0 => Ok(Self::Home),
            1 => Ok(Self::Desert),
            2 => Ok(Self::Valley),
            3 => Ok(Self::ViktorLab),
            4 => Ok(Self::Slimulations),
            other => Err(crate::InvalidEnumId { enum_name: "RegionSetId", value: other }),
        }
    }
}

impl RegionSetId {
    pub const ALL: &'static [Self] = &[
        Self::Unset,
        Self::Home,
        Self::Desert,
        Self::Valley,
        Self::ViktorLab,
        Self::Slimulations,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Unset => "UNSET",
            Self::Home => "HOME",
            Self::Desert => "DESERT",
            Self::Valley => "VALLEY",
            Self::ViktorLab => "VIKTOR_LAB",
            Self::Slimulations => "SLIMULATIONS",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "UNSET" => Some(Self::Unset),
            "HOME" => Some(Self::Home),
            "DESERT" => Some(Self::Desert),
            "VALLEY" => Some(Self::Valley),
            "VIKTORLAB" => Some(Self::ViktorLab),
            "SLIMULATIONS" => Some(Self::Slimulations),
            _ => None,
        }
    }
}

/// Port of `LandPlot.Id`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum LandPlotId {
    None = 0,
    Empty = 1,
    Corral = 2,
    Coop = 3,
    Garden = 4,
    Silo = 5,
    Pond = 6,
    Incinerator = 7,
}

impl TryFrom<i32> for LandPlotId {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Empty),
            2 => Ok(Self::Corral),
            3 => Ok(Self::Coop),
            4 => Ok(Self::Garden),
            5 => Ok(Self::Silo),
            6 => Ok(Self::Pond),
            7 => Ok(Self::Incinerator),
            other => Err(crate::InvalidEnumId { enum_name: "LandPlotId", value: other }),
        }
    }
}

impl LandPlotId {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Empty,
        Self::Corral,
        Self::Coop,
        Self::Garden,
        Self::Silo,
        Self::Pond,
        Self::Incinerator,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Empty => "EMPTY",
            Self::Corral => "CORRAL",
            Self::Coop => "COOP",
            Self::Garden => "GARDEN",
            Self::Silo => "SILO",
            Self::Pond => "POND",
            Self::Incinerator => "INCINERATOR",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "NONE" => Some(Self::None),
            "EMPTY" => Some(Self::Empty),
            "CORRAL" => Some(Self::Corral),
            "COOP" => Some(Self::Coop),
            "GARDEN" => Some(Self::Garden),
            "SILO" => Some(Self::Silo),
            "POND" => Some(Self::Pond),
            "INCINERATOR" => Some(Self::Incinerator),
            _ => None,
        }
    }
}

/// Port of `LandPlot.Upgrade`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum LandPlotUpgrade {
    None = 0,
    Walls = 1,
    MusicBox = 2,
    Storage2 = 3,
    Storage3 = 4,
    Storage4 = 5,
    Soil = 6,
    Sprinkler = 7,
    Scareslime = 8,
    Feeder = 9,
    Vitamizer = 10,
    AirNet = 11,
    PlortCollector = 12,
    SolarShield = 13,
    AshTrough = 14,
    MiracleMix = 15,
    DeluxeGarden = 16,
    DeluxeCoop = 17,
}

impl TryFrom<i32> for LandPlotUpgrade {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Walls),
            2 => Ok(Self::MusicBox),
            3 => Ok(Self::Storage2),
            4 => Ok(Self::Storage3),
            5 => Ok(Self::Storage4),
            6 => Ok(Self::Soil),
            7 => Ok(Self::Sprinkler),
            8 => Ok(Self::Scareslime),
            9 => Ok(Self::Feeder),
            10 => Ok(Self::Vitamizer),
            11 => Ok(Self::AirNet),
            12 => Ok(Self::PlortCollector),
            13 => Ok(Self::SolarShield),
            14 => Ok(Self::AshTrough),
            15 => Ok(Self::MiracleMix),
            16 => Ok(Self::DeluxeGarden),
            17 => Ok(Self::DeluxeCoop),
            other => Err(crate::InvalidEnumId { enum_name: "LandPlotUpgrade", value: other }),
        }
    }
}

impl LandPlotUpgrade {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Walls,
        Self::MusicBox,
        Self::Storage2,
        Self::Storage3,
        Self::Storage4,
        Self::Soil,
        Self::Sprinkler,
        Self::Scareslime,
        Self::Feeder,
        Self::Vitamizer,
        Self::AirNet,
        Self::PlortCollector,
        Self::SolarShield,
        Self::AshTrough,
        Self::MiracleMix,
        Self::DeluxeGarden,
        Self::DeluxeCoop,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Walls => "WALLS",
            Self::MusicBox => "MUSIC_BOX",
            Self::Storage2 => "STORAGE2",
            Self::Storage3 => "STORAGE3",
            Self::Storage4 => "STORAGE4",
            Self::Soil => "SOIL",
            Self::Sprinkler => "SPRINKLER",
            Self::Scareslime => "SCARESLIME",
            Self::Feeder => "FEEDER",
            Self::Vitamizer => "VITAMIZER",
            Self::AirNet => "AIR_NET",
            Self::PlortCollector => "PLORT_COLLECTOR",
            Self::SolarShield => "SOLAR_SHIELD",
            Self::AshTrough => "ASH_TROUGH",
            Self::MiracleMix => "MIRACLE_MIX",
            Self::DeluxeGarden => "DELUXE_GARDEN",
            Self::DeluxeCoop => "DELUXE_COOP",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "NONE" => Some(Self::None),
            "WALLS" => Some(Self::Walls),
            "MUSICBOX" => Some(Self::MusicBox),
            "STORAGE2" => Some(Self::Storage2),
            "STORAGE3" => Some(Self::Storage3),
            "STORAGE4" => Some(Self::Storage4),
            "SOIL" => Some(Self::Soil),
            "SPRINKLER" => Some(Self::Sprinkler),
            "SCARESLIME" => Some(Self::Scareslime),
            "FEEDER" => Some(Self::Feeder),
            "VITAMIZER" => Some(Self::Vitamizer),
            "AIRNET" => Some(Self::AirNet),
            "PLORTCOLLECTOR" => Some(Self::PlortCollector),
            "SOLARSHIELD" => Some(Self::SolarShield),
            "ASHTROUGH" => Some(Self::AshTrough),
            "MIRACLEMIX" => Some(Self::MiracleMix),
            "DELUXEGARDEN" => Some(Self::DeluxeGarden),
            "DELUXECOOP" => Some(Self::DeluxeCoop),
            _ => None,
        }
    }
}

/// Port of `PlayerState.Upgrade`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum PlayerUpgrade {
    Health1 = 0,
    Health2 = 1,
    Health3 = 2,
    Energy1 = 3,
    Energy2 = 4,
    Energy3 = 5,
    Ammo1 = 6,
    Ammo2 = 7,
    Ammo3 = 8,
    Jetpack = 9,
    JetpackEfficiency = 10,
    AirBurst = 11,
    RunEfficiency = 12,
    LiquidSlot = 13,
    Ammo4 = 14,
    Health4 = 15,
    RunEfficiency2 = 16,
    GoldenSureshot = 17,
    SpareKey = 18,
    TreasureCracker1 = 100,
    TreasureCracker2 = 101,
    TreasureCracker3 = 102,
    TreasureCracker4 = 103,
}

impl TryFrom<i32> for PlayerUpgrade {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Health1),
            1 => Ok(Self::Health2),
            2 => Ok(Self::Health3),
            3 => Ok(Self::Energy1),
            4 => Ok(Self::Energy2),
            5 => Ok(Self::Energy3),
            6 => Ok(Self::Ammo1),
            7 => Ok(Self::Ammo2),
            8 => Ok(Self::Ammo3),
            9 => Ok(Self::Jetpack),
            10 => Ok(Self::JetpackEfficiency),
            11 => Ok(Self::AirBurst),
            12 => Ok(Self::RunEfficiency),
            13 => Ok(Self::LiquidSlot),
            14 => Ok(Self::Ammo4),
            15 => Ok(Self::Health4),
            16 => Ok(Self::RunEfficiency2),
            17 => Ok(Self::GoldenSureshot),
            18 => Ok(Self::SpareKey),
            100 => Ok(Self::TreasureCracker1),
            101 => Ok(Self::TreasureCracker2),
            102 => Ok(Self::TreasureCracker3),
            103 => Ok(Self::TreasureCracker4),
            other => Err(crate::InvalidEnumId { enum_name: "PlayerUpgrade", value: other }),
        }
    }
}

impl PlayerUpgrade {
    pub const ALL: &'static [Self] = &[
        Self::Health1,
        Self::Health2,
        Self::Health3,
        Self::Energy1,
        Self::Energy2,
        Self::Energy3,
        Self::Ammo1,
        Self::Ammo2,
        Self::Ammo3,
        Self::Jetpack,
        Self::JetpackEfficiency,
        Self::AirBurst,
        Self::RunEfficiency,
        Self::LiquidSlot,
        Self::Ammo4,
        Self::Health4,
        Self::RunEfficiency2,
        Self::GoldenSureshot,
        Self::SpareKey,
        Self::TreasureCracker1,
        Self::TreasureCracker2,
        Self::TreasureCracker3,
        Self::TreasureCracker4,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Health1 => "HEALTH_1",
            Self::Health2 => "HEALTH_2",
            Self::Health3 => "HEALTH_3",
            Self::Energy1 => "ENERGY_1",
            Self::Energy2 => "ENERGY_2",
            Self::Energy3 => "ENERGY_3",
            Self::Ammo1 => "AMMO_1",
            Self::Ammo2 => "AMMO_2",
            Self::Ammo3 => "AMMO_3",
            Self::Jetpack => "JETPACK",
            Self::JetpackEfficiency => "JETPACK_EFFICIENCY",
            Self::AirBurst => "AIR_BURST",
            Self::RunEfficiency => "RUN_EFFICIENCY",
            Self::LiquidSlot => "LIQUID_SLOT",
            Self::Ammo4 => "AMMO_4",
            Self::Health4 => "HEALTH_4",
            Self::RunEfficiency2 => "RUN_EFFICIENCY_2",
            Self::GoldenSureshot => "GOLDEN_SURESHOT",
            Self::SpareKey => "SPARE_KEY",
            Self::TreasureCracker1 => "TREASURE_CRACKER_1",
            Self::TreasureCracker2 => "TREASURE_CRACKER_2",
            Self::TreasureCracker3 => "TREASURE_CRACKER_3",
            Self::TreasureCracker4 => "TREASURE_CRACKER_4",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "HEALTH1" => Some(Self::Health1),
            "HEALTH2" => Some(Self::Health2),
            "HEALTH3" => Some(Self::Health3),
            "ENERGY1" => Some(Self::Energy1),
            "ENERGY2" => Some(Self::Energy2),
            "ENERGY3" => Some(Self::Energy3),
            "AMMO1" => Some(Self::Ammo1),
            "AMMO2" => Some(Self::Ammo2),
            "AMMO3" => Some(Self::Ammo3),
            "JETPACK" => Some(Self::Jetpack),
            "JETPACKEFFICIENCY" => Some(Self::JetpackEfficiency),
            "AIRBURST" => Some(Self::AirBurst),
            "RUNEFFICIENCY" => Some(Self::RunEfficiency),
            "LIQUIDSLOT" => Some(Self::LiquidSlot),
            "AMMO4" => Some(Self::Ammo4),
            "HEALTH4" => Some(Self::Health4),
            "RUNEFFICIENCY2" => Some(Self::RunEfficiency2),
            "GOLDENSURESHOT" => Some(Self::GoldenSureshot),
            "SPAREKEY" => Some(Self::SpareKey),
            "TREASURECRACKER1" => Some(Self::TreasureCracker1),
            "TREASURECRACKER2" => Some(Self::TreasureCracker2),
            "TREASURECRACKER3" => Some(Self::TreasureCracker3),
            "TREASURECRACKER4" => Some(Self::TreasureCracker4),
            _ => None,
        }
    }
}

/// Port of `PlayerState.GameMode`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    Classic = 0,
    TimeLimit = 1,
    Casual = 2,
    TimeLimitV2 = 3,
}

impl TryFrom<i32> for GameMode {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Classic),
            1 => Ok(Self::TimeLimit),
            2 => Ok(Self::Casual),
            3 => Ok(Self::TimeLimitV2),
            other => Err(crate::InvalidEnumId { enum_name: "GameMode", value: other }),
        }
    }
}

impl GameMode {
    pub const ALL: &'static [Self] = &[
        Self::Classic,
        Self::TimeLimit,
        Self::Casual,
        Self::TimeLimitV2,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Classic => "CLASSIC",
            Self::TimeLimit => "TIME_LIMIT",
            Self::Casual => "CASUAL",
            Self::TimeLimitV2 => "TIME_LIMIT_V2",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "CLASSIC" => Some(Self::Classic),
            "TIMELIMIT" => Some(Self::TimeLimit),
            "CASUAL" => Some(Self::Casual),
            "TIMELIMITV2" => Some(Self::TimeLimitV2),
            _ => None,
        }
    }
}

/// Port of `PlayerState.AmmoMode`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum AmmoMode {
    Default = 0,
    NimbleValley = 1,
}

impl TryFrom<i32> for AmmoMode {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Default),
            1 => Ok(Self::NimbleValley),
            other => Err(crate::InvalidEnumId { enum_name: "AmmoMode", value: other }),
        }
    }
}

impl AmmoMode {
    pub const ALL: &'static [Self] = &[
        Self::Default,
        Self::NimbleValley,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::NimbleValley => "NIMBLE_VALLEY",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "DEFAULT" => Some(Self::Default),
            "NIMBLEVALLEY" => Some(Self::NimbleValley),
            _ => None,
        }
    }
}

/// Port of `ProgressDirector.ProgressType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ProgressType {
    None = -1,
    UnlockQuarry = 0,
    UnlockMoss = 1,
    UnlockDesert = 2,
    UnlockLab = 3,
    UnlockRuins = 4,
    UnlockDocks = 5,
    UnlockGrotto = 6,
    UnlockOvergrowth = 7,
    UnlockValley = 8,
    UnlockLabDocksExtra = 9,
    UnlockOgdenMissions = 200,
    UnlockWilds = 201,
    UnlockMochiMissions = 202,
    UnlockViktorMissions = 300,
    UnlockSlimulations = 301,
    HobsonEnd = 990,
    HobsonEndUnlock = 991,
    SlimeDoors = 1000,
    PlortDoor = 1001,
    ExchangeThora = 2000,
    ExchangeViktor = 2001,
    ExchangeOgden = 2002,
    ExchangeMochi = 2003,
    ExchangeBob = 2004,
    CorporatePartner = 3000,
    CorporatePartnerUnlock = 3001,
    OgdenRewards = 3100,
    OgdenSeenFinalChat = 3101,
    MochiRewards = 3102,
    MochiSeenFinalChat = 3103,
    ViktorRewards = 3104,
    ViktorSeenFinalChat = 3105,
    EnterZoneOgdenRanch = 3200,
    EnterZoneMochiRanch = 3201,
    EnterZoneViktorLab = 3202,
    EnterZoneSlimulation = 3203,
}

impl TryFrom<i32> for ProgressType {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::None),
            0 => Ok(Self::UnlockQuarry),
            1 => Ok(Self::UnlockMoss),
            2 => Ok(Self::UnlockDesert),
            3 => Ok(Self::UnlockLab),
            4 => Ok(Self::UnlockRuins),
            5 => Ok(Self::UnlockDocks),
            6 => Ok(Self::UnlockGrotto),
            7 => Ok(Self::UnlockOvergrowth),
            8 => Ok(Self::UnlockValley),
            9 => Ok(Self::UnlockLabDocksExtra),
            200 => Ok(Self::UnlockOgdenMissions),
            201 => Ok(Self::UnlockWilds),
            202 => Ok(Self::UnlockMochiMissions),
            300 => Ok(Self::UnlockViktorMissions),
            301 => Ok(Self::UnlockSlimulations),
            990 => Ok(Self::HobsonEnd),
            991 => Ok(Self::HobsonEndUnlock),
            1000 => Ok(Self::SlimeDoors),
            1001 => Ok(Self::PlortDoor),
            2000 => Ok(Self::ExchangeThora),
            2001 => Ok(Self::ExchangeViktor),
            2002 => Ok(Self::ExchangeOgden),
            2003 => Ok(Self::ExchangeMochi),
            2004 => Ok(Self::ExchangeBob),
            3000 => Ok(Self::CorporatePartner),
            3001 => Ok(Self::CorporatePartnerUnlock),
            3100 => Ok(Self::OgdenRewards),
            3101 => Ok(Self::OgdenSeenFinalChat),
            3102 => Ok(Self::MochiRewards),
            3103 => Ok(Self::MochiSeenFinalChat),
            3104 => Ok(Self::ViktorRewards),
            3105 => Ok(Self::ViktorSeenFinalChat),
            3200 => Ok(Self::EnterZoneOgdenRanch),
            3201 => Ok(Self::EnterZoneMochiRanch),
            3202 => Ok(Self::EnterZoneViktorLab),
            3203 => Ok(Self::EnterZoneSlimulation),
            other => Err(crate::InvalidEnumId { enum_name: "ProgressType", value: other }),
        }
    }
}

impl ProgressType {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::UnlockQuarry,
        Self::UnlockMoss,
        Self::UnlockDesert,
        Self::UnlockLab,
        Self::UnlockRuins,
        Self::UnlockDocks,
        Self::UnlockGrotto,
        Self::UnlockOvergrowth,
        Self::UnlockValley,
        Self::UnlockLabDocksExtra,
        Self::UnlockOgdenMissions,
        Self::UnlockWilds,
        Self::UnlockMochiMissions,
        Self::UnlockViktorMissions,
        Self::UnlockSlimulations,
        Self::HobsonEnd,
        Self::HobsonEndUnlock,
        Self::SlimeDoors,
        Self::PlortDoor,
        Self::ExchangeThora,
        Self::ExchangeViktor,
        Self::ExchangeOgden,
        Self::ExchangeMochi,
        Self::ExchangeBob,
        Self::CorporatePartner,
        Self::CorporatePartnerUnlock,
        Self::OgdenRewards,
        Self::OgdenSeenFinalChat,
        Self::MochiRewards,
        Self::MochiSeenFinalChat,
        Self::ViktorRewards,
        Self::ViktorSeenFinalChat,
        Self::EnterZoneOgdenRanch,
        Self::EnterZoneMochiRanch,
        Self::EnterZoneViktorLab,
        Self::EnterZoneSlimulation,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::UnlockQuarry => "UNLOCK_QUARRY",
            Self::UnlockMoss => "UNLOCK_MOSS",
            Self::UnlockDesert => "UNLOCK_DESERT",
            Self::UnlockLab => "UNLOCK_LAB",
            Self::UnlockRuins => "UNLOCK_RUINS",
            Self::UnlockDocks => "UNLOCK_DOCKS",
            Self::UnlockGrotto => "UNLOCK_GROTTO",
            Self::UnlockOvergrowth => "UNLOCK_OVERGROWTH",
            Self::UnlockValley => "UNLOCK_VALLEY",
            Self::UnlockLabDocksExtra => "UNLOCK_LAB_DOCKS_EXTRA",
            Self::UnlockOgdenMissions => "UNLOCK_OGDEN_MISSIONS",
            Self::UnlockWilds => "UNLOCK_WILDS",
            Self::UnlockMochiMissions => "UNLOCK_MOCHI_MISSIONS",
            Self::UnlockViktorMissions => "UNLOCK_VIKTOR_MISSIONS",
            Self::UnlockSlimulations => "UNLOCK_SLIMULATIONS",
            Self::HobsonEnd => "HOBSON_END",
            Self::HobsonEndUnlock => "HOBSON_END_UNLOCK",
            Self::SlimeDoors => "SLIME_DOORS",
            Self::PlortDoor => "PLORT_DOOR",
            Self::ExchangeThora => "EXCHANGE_THORA",
            Self::ExchangeViktor => "EXCHANGE_VIKTOR",
            Self::ExchangeOgden => "EXCHANGE_OGDEN",
            Self::ExchangeMochi => "EXCHANGE_MOCHI",
            Self::ExchangeBob => "EXCHANGE_BOB",
            Self::CorporatePartner => "CORPORATE_PARTNER",
            Self::CorporatePartnerUnlock => "CORPORATE_PARTNER_UNLOCK",
            Self::OgdenRewards => "OGDEN_REWARDS",
            Self::OgdenSeenFinalChat => "OGDEN_SEEN_FINAL_CHAT",
            Self::MochiRewards => "MOCHI_REWARDS",
            Self::MochiSeenFinalChat => "MOCHI_SEEN_FINAL_CHAT",
            Self::ViktorRewards => "VIKTOR_REWARDS",
            Self::ViktorSeenFinalChat => "VIKTOR_SEEN_FINAL_CHAT",
            Self::EnterZoneOgdenRanch => "ENTER_ZONE_OGDEN_RANCH",
            Self::EnterZoneMochiRanch => "ENTER_ZONE_MOCHI_RANCH",
            Self::EnterZoneViktorLab => "ENTER_ZONE_VIKTOR_LAB",
            Self::EnterZoneSlimulation => "ENTER_ZONE_SLIMULATION",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "NONE" => Some(Self::None),
            "UNLOCKQUARRY" => Some(Self::UnlockQuarry),
            "UNLOCKMOSS" => Some(Self::UnlockMoss),
            "UNLOCKDESERT" => Some(Self::UnlockDesert),
            "UNLOCKLAB" => Some(Self::UnlockLab),
            "UNLOCKRUINS" => Some(Self::UnlockRuins),
            "UNLOCKDOCKS" => Some(Self::UnlockDocks),
            "UNLOCKGROTTO" => Some(Self::UnlockGrotto),
            "UNLOCKOVERGROWTH" => Some(Self::UnlockOvergrowth),
            "UNLOCKVALLEY" => Some(Self::UnlockValley),
            "UNLOCKLABDOCKSEXTRA" => Some(Self::UnlockLabDocksExtra),
            "UNLOCKOGDENMISSIONS" => Some(Self::UnlockOgdenMissions),
            "UNLOCKWILDS" => Some(Self::UnlockWilds),
            "UNLOCKMOCHIMISSIONS" => Some(Self::UnlockMochiMissions),
            "UNLOCKVIKTORMISSIONS" => Some(Self::UnlockViktorMissions),
            "UNLOCKSLIMULATIONS" => Some(Self::UnlockSlimulations),
            "HOBSONEND" => Some(Self::HobsonEnd),
            "HOBSONENDUNLOCK" => Some(Self::HobsonEndUnlock),
            "SLIMEDOORS" => Some(Self::SlimeDoors),
            "PLORTDOOR" => Some(Self::PlortDoor),
            "EXCHANGETHORA" => Some(Self::ExchangeThora),
            "EXCHANGEVIKTOR" => Some(Self::ExchangeViktor),
            "EXCHANGEOGDEN" => Some(Self::ExchangeOgden),
            "EXCHANGEMOCHI" => Some(Self::ExchangeMochi),
            "EXCHANGEBOB" => Some(Self::ExchangeBob),
            "CORPORATEPARTNER" => Some(Self::CorporatePartner),
            "CORPORATEPARTNERUNLOCK" => Some(Self::CorporatePartnerUnlock),
            "OGDENREWARDS" => Some(Self::OgdenRewards),
            "OGDENSEENFINALCHAT" => Some(Self::OgdenSeenFinalChat),
            "MOCHIREWARDS" => Some(Self::MochiRewards),
            "MOCHISEENFINALCHAT" => Some(Self::MochiSeenFinalChat),
            "VIKTORREWARDS" => Some(Self::ViktorRewards),
            "VIKTORSEENFINALCHAT" => Some(Self::ViktorSeenFinalChat),
            "ENTERZONEOGDENRANCH" => Some(Self::EnterZoneOgdenRanch),
            "ENTERZONEMOCHIRANCH" => Some(Self::EnterZoneMochiRanch),
            "ENTERZONEVIKTORLAB" => Some(Self::EnterZoneViktorLab),
            "ENTERZONESLIMULATION" => Some(Self::EnterZoneSlimulation),
            _ => None,
        }
    }
}

/// Port of `ProgressDirector.ProgressTrackerId`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ProgressTrackerId {
    Casey1 = 0,
    Casey2 = 1,
    Casey3 = 2,
    Casey4 = 3,
    Casey5 = 4,
    Casey6 = 5,
    Casey7 = 6,
    Casey8 = 7,
    Casey9 = 8,
    Casey10 = 9,
    Casey11 = 10,
    Hobson1 = 1000,
    TimeAfterLabDocks = 2000,
}

impl TryFrom<i32> for ProgressTrackerId {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Casey1),
            1 => Ok(Self::Casey2),
            2 => Ok(Self::Casey3),
            3 => Ok(Self::Casey4),
            4 => Ok(Self::Casey5),
            5 => Ok(Self::Casey6),
            6 => Ok(Self::Casey7),
            7 => Ok(Self::Casey8),
            8 => Ok(Self::Casey9),
            9 => Ok(Self::Casey10),
            10 => Ok(Self::Casey11),
            1000 => Ok(Self::Hobson1),
            2000 => Ok(Self::TimeAfterLabDocks),
            other => Err(crate::InvalidEnumId { enum_name: "ProgressTrackerId", value: other }),
        }
    }
}

impl ProgressTrackerId {
    pub const ALL: &'static [Self] = &[
        Self::Casey1,
        Self::Casey2,
        Self::Casey3,
        Self::Casey4,
        Self::Casey5,
        Self::Casey6,
        Self::Casey7,
        Self::Casey8,
        Self::Casey9,
        Self::Casey10,
        Self::Casey11,
        Self::Hobson1,
        Self::TimeAfterLabDocks,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Casey1 => "CASEY_1",
            Self::Casey2 => "CASEY_2",
            Self::Casey3 => "CASEY_3",
            Self::Casey4 => "CASEY_4",
            Self::Casey5 => "CASEY_5",
            Self::Casey6 => "CASEY_6",
            Self::Casey7 => "CASEY_7",
            Self::Casey8 => "CASEY_8",
            Self::Casey9 => "CASEY_9",
            Self::Casey10 => "CASEY_10",
            Self::Casey11 => "CASEY_11",
            Self::Hobson1 => "HOBSON_1",
            Self::TimeAfterLabDocks => "TIME_AFTER_LAB_DOCKS",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "CASEY1" => Some(Self::Casey1),
            "CASEY2" => Some(Self::Casey2),
            "CASEY3" => Some(Self::Casey3),
            "CASEY4" => Some(Self::Casey4),
            "CASEY5" => Some(Self::Casey5),
            "CASEY6" => Some(Self::Casey6),
            "CASEY7" => Some(Self::Casey7),
            "CASEY8" => Some(Self::Casey8),
            "CASEY9" => Some(Self::Casey9),
            "CASEY10" => Some(Self::Casey10),
            "CASEY11" => Some(Self::Casey11),
            "HOBSON1" => Some(Self::Hobson1),
            "TIMEAFTERLABDOCKS" => Some(Self::TimeAfterLabDocks),
            _ => None,
        }
    }
}

/// Port of `SlimeEmotions.Emotion`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum SlimeEmotion {
    Hunger = 0,
    Agitation = 1,
    Fear = 2,
    None = 3,
}

impl TryFrom<i32> for SlimeEmotion {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Hunger),
            1 => Ok(Self::Agitation),
            2 => Ok(Self::Fear),
            3 => Ok(Self::None),
            other => Err(crate::InvalidEnumId { enum_name: "SlimeEmotion", value: other }),
        }
    }
}

impl SlimeEmotion {
    pub const ALL: &'static [Self] = &[
        Self::Hunger,
        Self::Agitation,
        Self::Fear,
        Self::None,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Hunger => "HUNGER",
            Self::Agitation => "AGITATION",
            Self::Fear => "FEAR",
            Self::None => "NONE",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "HUNGER" => Some(Self::Hunger),
            "AGITATION" => Some(Self::Agitation),
            "FEAR" => Some(Self::Fear),
            "NONE" => Some(Self::None),
            _ => None,
        }
    }
}

/// Port of `SlimeEat.FoodGroup`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum FoodGroup {
    Fruit = 0,
    Veggies = 1,
    Meat = 2,
    NontarrgoldSlimes = 3,
    Plorts = 4,
    Ginger = 5,
}

impl TryFrom<i32> for FoodGroup {
    type Error = crate::InvalidEnumId;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Fruit),
            1 => Ok(Self::Veggies),
            2 => Ok(Self::Meat),
            3 => Ok(Self::NontarrgoldSlimes),
            4 => Ok(Self::Plorts),
            5 => Ok(Self::Ginger),
            other => Err(crate::InvalidEnumId { enum_name: "FoodGroup", value: other }),
        }
    }
}

impl FoodGroup {
    pub const ALL: &'static [Self] = &[
        Self::Fruit,
        Self::Veggies,
        Self::Meat,
        Self::NontarrgoldSlimes,
        Self::Plorts,
        Self::Ginger,
    ];

    #[inline]
    pub const fn as_i32(self) -> i32 { self as i32 }

    /// Exact C# enum member name (e.g. `"PINK_SLIME"`).
    pub const fn as_csharp_name(self) -> &'static str {
        match self {
            Self::Fruit => "FRUIT",
            Self::Veggies => "VEGGIES",
            Self::Meat => "MEAT",
            Self::NontarrgoldSlimes => "NONTARRGOLD_SLIMES",
            Self::Plorts => "PLORTS",
            Self::Ginger => "GINGER",
        }
    }

    /// Case-insensitive lookup by C# enum name or PascalCase name.
    pub fn from_name(name: &str) -> Option<Self> {
        let norm = crate::normalize_lookup_key(name);
        match norm.as_str() {
            "FRUIT" => Some(Self::Fruit),
            "VEGGIES" => Some(Self::Veggies),
            "MEAT" => Some(Self::Meat),
            "NONTARRGOLDSLIMES" => Some(Self::NontarrgoldSlimes),
            "PLORTS" => Some(Self::Plorts),
            "GINGER" => Some(Self::Ginger),
            _ => None,
        }
    }
}

impl ZoneId {
    /// Exact port of `ZoneDirector.GetRegionSetId(Zone)`.
    pub const fn region_set_id(self) -> Option<RegionSetId> {
        match self {
            Self::Ranch
            | Self::Reef
            | Self::Quarry
            | Self::Moss
            | Self::Sea
            | Self::Ruins
            | Self::RuinsTransition
            | Self::Wilds
            | Self::OgdenRanch => Some(RegionSetId::Home),
            Self::Desert => Some(RegionSetId::Desert),
            Self::Valley | Self::MochiRanch => Some(RegionSetId::Valley),
            Self::Slimulations => Some(RegionSetId::Slimulations),
            Self::ViktorLab => Some(RegionSetId::ViktorLab),
            Self::None => None,
        }
    }
}


