//! Port of `JournalEntry.cs`, `PediaDirector.cs`, `PediaAreaTrigger.cs`, and `TutorialDirector.cs`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{IdentifiableId, PediaId, ProgressType, ZoneId};
use sr_model::{GameModel, MailType, TutorialId};

use crate::story::progress::{ProgressDirector, ProgressStepEvents};

/// Entries excluded from the `COMPLETED_SLIMEPEDIA` achievement requirement.
pub const PEDIA_HIDDEN_ENTRIES: &[PediaId] = &[
    PediaId::PartyGordoSlime,
    PediaId::EchoNoteGordoSlime,
    PediaId::EchoNotes,
];

/// Category/meta entries that are UI tabs rather than unlockable articles.
pub const PEDIA_CATEGORY_HEADERS: &[PediaId] = &[
    PediaId::Tutorials,
    PediaId::Slimes,
    PediaId::Resources,
    PediaId::Ranch,
    PediaId::World,
    PediaId::Science,
    PediaId::Locked,
];

/// Entries unlocked at the start of a new game (`PediaDirector.initUnlocked`).
pub const PEDIA_INIT_UNLOCKED: &[PediaId] = &[
    PediaId::Splash,
    PediaId::Basics,
    PediaId::Vacing,
    PediaId::Capturetanks,
    PediaId::Energy,
    PediaId::Corralling,
    PediaId::Feeding,
    PediaId::Plorts,
    PediaId::TheRanch,
    PediaId::Sea,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntryDef {
    pub entry_key: String,
    pub ensure_progress: Vec<ProgressType>,
}

impl JournalEntryDef {
    pub fn hobson_end() -> Self {
        Self {
            entry_key: "m.journal.desert_end".to_string(),
            ensure_progress: vec![ProgressType::HobsonEnd],
        }
    }
}

/// Port of `PediaDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PediaDirector {
    pub popup_queue: Vec<PediaId>,
    pub read_journals: HashSet<String>,
}

impl PediaDirector {
    pub fn init_model(model: &mut GameModel) {
        model.pedia.unlocked.clear();
        for &id in PEDIA_INIT_UNLOCKED {
            model.pedia.unlocked.insert(id);
        }
    }

    pub fn pedia_id_for_identifiable(id: IdentifiableId) -> Option<PediaId> {
        use IdentifiableId as Id;
        use PediaId as P;
        Some(match id {
            Id::PinkSlime => P::PinkSlime,
            Id::RockSlime => P::RockSlime,
            Id::PhosphorSlime => P::PhosphorSlime,
            Id::TabbySlime => P::TabbySlime,
            Id::RadSlime => P::RadSlime,
            Id::BoomSlime => P::BoomSlime,
            Id::HunterSlime => P::HunterSlime,
            Id::HoneySlime => P::HoneySlime,
            Id::PuddleSlime => P::PuddleSlime,
            Id::CrystalSlime => P::CrystalSlime,
            Id::TarrSlime => P::TarrSlime,
            Id::GoldSlime => P::GoldSlime,
            Id::LuckySlime => P::LuckySlime,
            Id::QuantumSlime => P::QuantumSlime,
            Id::FireSlime => P::FireSlime,
            Id::MosaicSlime => P::MosaicSlime,
            Id::DervishSlime => P::DervishSlime,
            Id::TangleSlime => P::TangleSlime,
            Id::SaberSlime => P::SaberSlime,
            Id::QuicksilverSlime => P::QuicksilverSlime,
            Id::GlitchSlime => P::GlitchSlime,
            Id::PartyGordo => P::PartyGordoSlime,
            Id::CarrotVeggie => P::Carrot,
            Id::OcaocaVeggie => P::Ocaoca,
            Id::BeetVeggie => P::Beet,
            Id::ParsnipVeggie => P::Parsnip,
            Id::PogoFruit => P::Pogo,
            Id::MangoFruit => P::Mango,
            Id::CuberryFruit => P::Cuberry,
            Id::PearFruit => P::Pear,
            Id::Hen => P::Henhen,
            Id::BriarHen => P::BriarHen,
            Id::StonyHen => P::StonyHen,
            Id::Rooster => P::Roostro,
            Id::Chick => P::Chickadoo,
            Id::BriarChick => P::BriarChickadoo,
            Id::StonyChick => P::StonyChickadoo,
            Id::ElderHen => P::ElderHen,
            Id::ElderRooster => P::ElderRoostro,
            Id::OnionVeggie => P::Onion,
            Id::LemonFruit => P::Lemon,
            Id::PaintedHen => P::PaintedHen,
            Id::PaintedChick => P::PaintedChickadoo,
            Id::GingerVeggie => P::Ginger,
            Id::KookadobaFruit => P::Kookadoba,
            Id::SpicyTofu => P::SpicyTofu,
            Id::PrimordyOilCraft => P::PrimordyOilCraft,
            Id::DeepBrineCraft => P::DeepBrineCraft,
            Id::SpiralSteamCraft => P::SpiralSteamCraft,
            Id::LavaDustCraft => P::LavaDustCraft,
            Id::BuzzWaxCraft => P::BuzzWaxCraft,
            Id::WildHoneyCraft => P::WildHoneyCraft,
            Id::HexacombCraft => P::HexacombCraft,
            Id::RoyalJellyCraft => P::RoyalJellyCraft,
            Id::JellystoneCraft => P::JellystoneCraft,
            Id::IndigoniumCraft => P::IndigoniumCraft,
            Id::SlimeFossilCraft => P::SlimeFossilCraft,
            Id::StrangeDiamondCraft => P::StrangeDiamondCraft,
            Id::SilkySandCraft => P::SilkySandCraft,
            Id::PepperJamCraft => P::PepperJamCraft,
            Id::GlassShardCraft => P::GlassShardCraft,
            Id::ManifoldCubeCraft => P::ManifoldCubeCraft,
            _ if id.is_echo() => P::Echoes,
            _ if id.is_echo_note() => P::EchoNotes,
            _ if id.is_ornament() => P::Ornaments,
            _ if id.is_toy() => P::SlimeToys,
            _ if id.is_largo() => P::LargoSlime,
            _ if id.is_gordo() => P::GordoSlime,
            _ => return None,
        })
    }

    pub fn pedia_id_for_zone(zone: ZoneId) -> Option<PediaId> {
        Some(match zone {
            ZoneId::Ranch => PediaId::TheRanch,
            ZoneId::Reef => PediaId::Reef,
            ZoneId::Quarry => PediaId::Quarry,
            ZoneId::Moss => PediaId::Moss,
            ZoneId::Desert => PediaId::Desert,
            ZoneId::Ruins | ZoneId::RuinsTransition => PediaId::Ruins,
            ZoneId::Sea => PediaId::Sea,
            ZoneId::OgdenRanch => PediaId::OgdenRetreat,
            ZoneId::MochiRanch => PediaId::MochiManor,
            ZoneId::Valley => PediaId::Valley,
            ZoneId::ViktorLab => PediaId::ViktorLab,
            ZoneId::Slimulations => PediaId::SlimulationsWorld,
            ZoneId::Wilds => PediaId::Wilds,
            _ => return None,
        })
    }

    pub fn unlock_science(&mut self, model: &mut GameModel) -> Vec<PediaId> {
        let ids = [
            PediaId::Refinery,
            PediaId::Fabricator,
            PediaId::Blueprints,
            PediaId::Extractors,
            PediaId::Utilities,
            PediaId::WarpTech,
            PediaId::Decorations,
            PediaId::Curios,
            PediaId::Drones,
            PediaId::Ssbasics,
            PediaId::Gadgetmode,
        ];
        let mut newly = Vec::new();
        for id in ids {
            if model.pedia.unlock(id) {
                newly.push(id);
            }
        }
        Self::check_completed_slimepedia(model);
        newly
    }

    pub fn unlock_without_popup(model: &mut GameModel, id: PediaId) -> bool {
        let added = model.pedia.unlock(id);
        Self::check_completed_slimepedia(model);
        added
    }

    pub fn maybe_show_popup(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        id: PediaId,
    ) -> bool {
        if model.pedia.unlocked.contains(&id) {
            return false;
        }
        model.pedia.unlocked.insert(id);
        Self::check_completed_slimepedia(model);
        if !mode.assume_experienced_user && !self.popup_queue.contains(&id) {
            self.popup_queue.push(id);
        }
        true
    }

    pub fn maybe_show_popup_for_ident(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ident: IdentifiableId,
    ) -> Option<PediaId> {
        let pid = Self::pedia_id_for_identifiable(ident)?;
        if self.maybe_show_popup(model, mode, pid) {
            Some(pid)
        } else {
            None
        }
    }

    pub fn check_completed_slimepedia(model: &mut GameModel) -> bool {
        let complete = PediaId::ALL.iter().all(|id| {
            PEDIA_CATEGORY_HEADERS.contains(id)
                || PEDIA_HIDDEN_ENTRIES.contains(id)
                || model.pedia.unlocked.contains(id)
        });
        if complete {
            model
                .game_achieves
                .counts
                .insert("COMPLETED_SLIMEPEDIA".to_string(), 1);
        }
        complete
    }

    /// Port of `JournalEntry.Activate`. Returns `(activated, progress_events)`.
    pub fn activate_journal_entry(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        progress: &mut ProgressDirector,
        entry: &JournalEntryDef,
    ) -> (bool, ProgressStepEvents) {
        let mut ev = ProgressStepEvents::default();
        if mode.suppress_story {
            return (false, ev);
        }
        self.read_journals.insert(entry.entry_key.clone());
        for &ptype in &entry.ensure_progress {
            progress.set_progress(model, mode, ptype, 1, &mut ev);
        }
        (true, ev)
    }
}

/// Port of `TutorialDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TutorialDirector {
    pub hidden: HashSet<TutorialId>,
    pub in_market_area: bool,
    pub in_barn_area: bool,
    pub suppressors: i32,
}

impl TutorialDirector {
    pub fn dependencies(id: TutorialId) -> &'static [TutorialId] {
        use TutorialId as T;
        match id {
            T::Market => &[T::Plort],
            T::ScienceSummary
            | T::ScienceRefinery
            | T::ScienceBuilderShop
            | T::ScienceFabricator
            | T::ScienceGadgetMode => &[T::ScienceBarn],
            T::ScienceSites | T::SciencePlaceSite | T::ScienceWrapup => &[T::ScienceGadgetMode],
            _ => &[],
        }
    }

    pub fn maybe_show_popup(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        id: TutorialId,
    ) -> bool {
        if mode.assume_experienced_user || self.suppressors > 0 {
            return false;
        }
        if model.tutorials.completed_ids.contains(&id)
            || model.tutorials.popup_queue.contains(&id)
        {
            return false;
        }
        let deps_ok = Self::dependencies(id)
            .iter()
            .all(|d| model.tutorials.completed_ids.contains(d));
        if !deps_ok {
            self.hidden.insert(id);
            return false;
        }
        model.tutorials.popup_queue.push(id);
        true
    }

    pub fn complete_tutorial(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        id: TutorialId,
    ) -> Vec<TutorialId> {
        model.tutorials.completed_ids.insert(id);
        model.tutorials.popup_queue.retain(|&x| x != id);
        let mut unblocked = Vec::new();
        let hidden_list: Vec<TutorialId> = self.hidden.iter().copied().collect();
        for h in hidden_list {
            if Self::dependencies(h)
                .iter()
                .all(|d| model.tutorials.completed_ids.contains(d))
            {
                self.hidden.remove(&h);
                if self.maybe_show_popup(model, mode, h) {
                    unblocked.push(h);
                }
            }
        }
        unblocked
    }

    pub fn on_vac(&mut self, model: &mut GameModel, mode: &GameModeSettings, id: IdentifiableId) {
        if id.is_slime() {
            self.maybe_show_popup(model, mode, TutorialId::Shooting);
        } else if id.is_food() {
            self.maybe_show_popup(model, mode, TutorialId::Food);
        } else if id.is_plort() {
            self.maybe_show_popup(model, mode, TutorialId::Plort);
        }
    }

    pub fn on_progress(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        ptype: ProgressType,
    ) {
        if ptype == ProgressType::UnlockLab {
            self.maybe_show_popup(model, mode, TutorialId::ScienceBarn);
        }
    }

    pub fn set_in_market_area(&mut self, model: &mut GameModel, mode: &GameModeSettings, in_area: bool) {
        self.in_market_area = in_area;
        if in_area {
            self.maybe_show_popup(model, mode, TutorialId::Market);
        }
    }

    pub fn set_in_barn_area(&mut self, model: &mut GameModel, mode: &GameModeSettings, in_area: bool) {
        self.in_barn_area = in_area;
        if in_area {
            for id in [
                TutorialId::ScienceSummary,
                TutorialId::ScienceRefinery,
                TutorialId::ScienceBuilderShop,
                TutorialId::ScienceFabricator,
                TutorialId::ScienceGadgetMode,
            ] {
                self.maybe_show_popup(model, mode, id);
            }
        }
    }

    pub fn on_gadget_mode_activated(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        self.maybe_show_popup(model, mode, TutorialId::ScienceSites);
        self.maybe_show_popup(model, mode, TutorialId::SciencePlaceSite);
    }

    pub fn on_liquid_slot_gained(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        self.maybe_show_popup(model, mode, TutorialId::Water);
    }

    pub fn on_map_data_gained(&mut self, model: &mut GameModel, mode: &GameModeSettings) {
        self.maybe_show_popup(model, mode, TutorialId::Map);
    }

    pub fn on_mail_read(
        &mut self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        mail_type: MailType,
        key: &str,
    ) {
        if mail_type != MailType::Personal {
            return;
        }
        match key {
            "ogden_invite" => {
                self.maybe_show_popup(model, mode, TutorialId::EnterZoneOgdenRanch);
            }
            "mochi_invite" => {
                self.maybe_show_popup(model, mode, TutorialId::EnterZoneMochiRanch);
            }
            "viktor_invite" => {
                self.maybe_show_popup(model, mode, TutorialId::EnterZoneViktorLab);
            }
            _ => {}
        }
    }
}
