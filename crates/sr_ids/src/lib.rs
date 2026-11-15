//! All strongly-typed game IDs ported 1:1 from Slime Rancher's C# enums.
//!
//! Every enum uses `#[repr(i32)]` with exact C# numeric discriminants so binary
//! `.sav` files (`GameV12`) and simulation state match the original game.

pub mod gadget;
pub mod identifiable;
pub mod pedia;
pub mod world;

pub use gadget::GadgetId;
pub use identifiable::IdentifiableId;
pub use pedia::PediaId;
pub use world::{
    AmmoMode, FoodGroup, GameMode, LandPlotId, LandPlotUpgrade, PlayerUpgrade, ProgressTrackerId,
    ProgressType, RegionSetId, SlimeEmotion, ZoneId,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {enum_name} discriminant: {value}")]
pub struct InvalidEnumId {
    pub enum_name: &'static str,
    pub value: i32,
}

pub(crate) fn normalize_lookup_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| *c != '_' && *c != '-' && !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiable_counts_and_roundtrips() {
        assert_eq!(IdentifiableId::ALL.len(), 325);
        assert_eq!(GadgetId::ALL.len(), 136);
        assert_eq!(PediaId::ALL.len(), 126);

        for &id in IdentifiableId::ALL {
            assert_eq!(IdentifiableId::try_from(id.as_i32()), Ok(id));
            assert_eq!(IdentifiableId::from_name(id.as_csharp_name()), Some(id));
        }
        assert_eq!(IdentifiableId::from_name("PinkSlime"), Some(IdentifiableId::PinkSlime));
        assert_eq!(IdentifiableId::from_name("pink_slime"), Some(IdentifiableId::PinkSlime));
    }

    #[test]
    fn identifiable_classifiers_and_combine() {
        assert!(IdentifiableId::PinkSlime.is_slime());
        assert!(!IdentifiableId::PinkSlime.is_largo());
        assert!(IdentifiableId::PinkRockLargo.is_slime());
        assert!(IdentifiableId::PinkRockLargo.is_largo());
        assert!(IdentifiableId::PinkPlort.is_plort());
        assert!(IdentifiableId::CuberryFruit.is_fruit());
        assert!(IdentifiableId::CuberryFruit.is_food());
        assert!(IdentifiableId::Hen.is_meat());
        assert!(IdentifiableId::Hen.is_animal());
        assert!(IdentifiableId::WaterLiquid.is_water());
        assert!(!IdentifiableId::GlitchDebugSprayLiquid.is_water());
        assert!(IdentifiableId::TarrSlime.is_tarr());
        assert!(IdentifiableId::TabbySlime.is_boopable());

        assert_eq!(
            IdentifiableId::combine(&[IdentifiableId::PinkSlime, IdentifiableId::RockSlime]),
            Some(IdentifiableId::PinkRockLargo)
        );
        assert_eq!(
            IdentifiableId::combine(&[IdentifiableId::RockSlime, IdentifiableId::PinkSlime]),
            Some(IdentifiableId::PinkRockLargo)
        );
        assert_eq!(
            IdentifiableId::combine(&[IdentifiableId::PinkSlime, IdentifiableId::GoldSlime]),
            Some(IdentifiableId::TarrSlime)
        );
    }

    #[test]
    fn gadget_and_zone_classifiers() {
        assert!(GadgetId::ExtractorDrillNovice.is_extractor());
        assert!(GadgetId::TeleporterPink.is_teleporter());
        assert!(GadgetId::MarketLink.is_misc());
        assert!(GadgetId::EchoNet.is_echo_net());
        assert!(GadgetId::Drone.is_drone());
        assert!(GadgetId::DroneAdvanced.is_drone());
        assert!(GadgetId::TeleporterPink.is_link_destroyer_type());

        assert_eq!(ZoneId::Ranch.region_set_id(), Some(RegionSetId::Home));
        assert_eq!(ZoneId::Desert.region_set_id(), Some(RegionSetId::Desert));
        assert_eq!(ZoneId::Valley.region_set_id(), Some(RegionSetId::Valley));
        assert_eq!(ZoneId::None.region_set_id(), None);
    }
}

