//! Binary `.sav` persistence (`GameV12` compatible) and `AutoSaveDirector`.

pub mod binary;

pub use binary::{
    BinReader, BinWriter, PersistError, PersistResult, PersistedDataset, DATA_PAYLOAD_BEGIN,
    DATA_PAYLOAD_END, ELEMENT_SEPARATOR, SECTION_SEPARATOR,
};
