//! Built-in baseline presets for early testing prior to exporting full Unity asset tables.

use sr_ids::{FoodGroup, IdentifiableId};

use crate::items::PlortBaseValue;
use crate::slime::{SlimeDefinition, SlimeDefinitions, SlimeDiet};

fn base_slime(
    name: &str,
    id: IdentifiableId,
    plort: IdentifiableId,
    groups: Vec<FoodGroup>,
    favorites: Vec<IdentifiableId>,
    additional: Vec<IdentifiableId>,
    toys: Vec<IdentifiableId>,
) -> SlimeDefinition {
    SlimeDefinition {
        name: name.to_string(),
        identifiable_id: id,
        base_slimes: Vec::new(),
        is_largo: false,
        can_largofy: id != IdentifiableId::PuddleSlime
            && id != IdentifiableId::FireSlime
            && id != IdentifiableId::GoldSlime
            && id != IdentifiableId::LuckySlime
            && id != IdentifiableId::QuicksilverSlime
            && id != IdentifiableId::GlitchSlime
            && !id.is_tarr(),
        prefab_scale: 1.0,
        diet: SlimeDiet {
            major_food_groups: groups,
            favorites,
            additional_foods: additional,
            produces: if plort == IdentifiableId::None {
                Vec::new()
            } else {
                vec![plort]
            },
            favorite_production_count: 2,
            eat_map: Vec::new(),
        },
        favorite_toys: toys,
    }
}

fn largo(name: &str, id: IdentifiableId, a: IdentifiableId, b: IdentifiableId) -> SlimeDefinition {
    SlimeDefinition {
        name: name.to_string(),
        identifiable_id: id,
        base_slimes: vec![a, b],
        is_largo: true,
        can_largofy: false,
        prefab_scale: 2.0,
        diet: SlimeDiet::default(),
        favorite_toys: Vec::new(),
    }
}

/// Creates a baseline `SlimeDefinitions` covering all canonical slime species and sample largos
/// so simulation tests & early sandbox can run before Unity asset extraction.
pub fn builtin_slime_definitions() -> SlimeDefinitions {
    use FoodGroup as FG;
    use IdentifiableId as Id;

    let mut defs = vec![
        base_slime(
            "Pink",
            Id::PinkSlime,
            Id::PinkPlort,
            vec![FG::Fruit, FG::Veggies, FG::Meat],
            vec![],
            vec![Id::SpicyTofu],
            vec![Id::BeachBallToy],
        ),
        base_slime(
            "Rock",
            Id::RockSlime,
            Id::RockPlort,
            vec![FG::Veggies],
            vec![Id::BeetVeggie],
            vec![Id::SpicyTofu],
            vec![Id::BigRockToy],
        ),
        base_slime(
            "Tabby",
            Id::TabbySlime,
            Id::TabbyPlort,
            vec![FG::Meat],
            vec![Id::StonyHen],
            vec![Id::SpicyTofu],
            vec![Id::YarnBallToy],
        ),
        base_slime(
            "Phosphor",
            Id::PhosphorSlime,
            Id::PhosphorPlort,
            vec![FG::Fruit],
            vec![Id::CuberryFruit],
            vec![Id::SpicyTofu],
            vec![Id::NightLightToy],
        ),
        base_slime(
            "Rad",
            Id::RadSlime,
            Id::RadPlort,
            vec![FG::Veggies],
            vec![Id::OcaocaVeggie],
            vec![Id::SpicyTofu],
            vec![Id::PowerCellToy],
        ),
        base_slime(
            "Boom",
            Id::BoomSlime,
            Id::BoomPlort,
            vec![FG::Meat],
            vec![Id::BriarHen],
            vec![Id::SpicyTofu],
            vec![Id::BombBallToy],
        ),
        base_slime(
            "Honey",
            Id::HoneySlime,
            Id::HoneyPlort,
            vec![FG::Fruit],
            vec![Id::MangoFruit],
            vec![Id::SpicyTofu],
            vec![Id::BuzzyBeeToy],
        ),
        base_slime(
            "Hunter",
            Id::HunterSlime,
            Id::HunterPlort,
            vec![FG::Meat],
            vec![Id::Rooster],
            vec![Id::SpicyTofu],
            vec![Id::StuffedChickenToy],
        ),
        base_slime(
            "Crystal",
            Id::CrystalSlime,
            Id::CrystalPlort,
            vec![FG::Veggies],
            vec![Id::OnionVeggie],
            vec![Id::SpicyTofu],
            vec![Id::CrystalBallToy],
        ),
        base_slime(
            "Quantum",
            Id::QuantumSlime,
            Id::QuantumPlort,
            vec![FG::Fruit],
            vec![Id::LemonFruit],
            vec![Id::SpicyTofu],
            vec![Id::PuzzleCubeToy],
        ),
        base_slime(
            "Dervish",
            Id::DervishSlime,
            Id::DervishPlort,
            vec![FG::Fruit],
            vec![Id::PearFruit],
            vec![Id::SpicyTofu],
            vec![Id::GyroTopToy],
        ),
        base_slime(
            "Mosaic",
            Id::MosaicSlime,
            Id::MosaicPlort,
            vec![FG::Veggies],
            vec![Id::ParsnipVeggie],
            vec![Id::SpicyTofu],
            vec![Id::DiscoBallToy],
        ),
        base_slime(
            "Tangle",
            Id::TangleSlime,
            Id::TanglePlort,
            vec![FG::Meat],
            vec![Id::PaintedHen],
            vec![Id::SpicyTofu],
            vec![Id::SolMateToy],
        ),
        base_slime(
            "Saber",
            Id::SaberSlime,
            Id::SaberPlort,
            vec![FG::Meat],
            vec![],
            vec![Id::SpicyTofu],
            vec![Id::StegoBuddyToy],
        ),
        base_slime(
            "Puddle",
            Id::PuddleSlime,
            Id::PuddlePlort,
            vec![],
            vec![],
            vec![],
            vec![Id::RubberDuckyToy],
        ),
        base_slime(
            "Fire",
            Id::FireSlime,
            Id::FirePlort,
            vec![],
            vec![],
            vec![],
            vec![Id::CharcoalBrickToy],
        ),
        base_slime(
            "Gold",
            Id::GoldSlime,
            Id::GoldPlort,
            vec![FG::Ginger],
            vec![Id::GingerVeggie],
            vec![],
            vec![],
        ),
        base_slime(
            "Lucky",
            Id::LuckySlime,
            Id::None,
            vec![FG::Meat],
            vec![],
            vec![],
            vec![],
        ),
        base_slime(
            "Tarr",
            Id::TarrSlime,
            Id::None,
            vec![FG::NontarrgoldSlimes, FG::Meat],
            vec![],
            vec![],
            vec![],
        ),
    ];

    // Add all valid Largo combinations automatically via IdentifiableId::combine.
    let base_ids: Vec<IdentifiableId> = defs
        .iter()
        .filter(|d| d.can_largofy)
        .map(|d| d.identifiable_id)
        .collect();

    for i in 0..base_ids.len() {
        for j in (i + 1)..base_ids.len() {
            if let Some(largo_id) = IdentifiableId::combine(&[base_ids[i], base_ids[j]]) {
                if largo_id.is_largo() {
                    defs.push(largo(
                        largo_id.as_csharp_name(),
                        largo_id,
                        base_ids[i],
                        base_ids[j],
                    ));
                }
            }
        }
    }

    SlimeDefinitions::from_vec(defs)
}

/// Canonical baseline plort market values (overridable via exported Unity config JSON).
pub fn builtin_plort_base_values() -> Vec<PlortBaseValue> {
    use IdentifiableId as Id;
    vec![
        PlortBaseValue { plort: Id::PinkPlort, base_value: 7.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::RockPlort, base_value: 15.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::TabbyPlort, base_value: 15.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::PhosphorPlort, base_value: 15.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::HoneyPlort, base_value: 30.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::RadPlort, base_value: 30.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::BoomPlort, base_value: 30.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::PuddlePlort, base_value: 30.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::CrystalPlort, base_value: 40.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::HunterPlort, base_value: 45.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::QuantumPlort, base_value: 45.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::FirePlort, base_value: 50.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::DervishPlort, base_value: 50.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::MosaicPlort, base_value: 55.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::TanglePlort, base_value: 55.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::SaberPlort, base_value: 55.0, full_saturation: 100.0 },
        PlortBaseValue { plort: Id::GoldPlort, base_value: 300.0, full_saturation: 50.0 },
        PlortBaseValue { plort: Id::QuicksilverPlort, base_value: 0.0, full_saturation: 100.0 },
    ]
}
