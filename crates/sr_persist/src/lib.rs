//! Binary `.sav` persistence (`GameV12` compatible) and `AutoSaveDirector`.

pub mod binary;
pub mod game_v12;
pub mod mapper;
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
pub use types::*;
