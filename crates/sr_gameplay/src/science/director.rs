//! Port of `GadgetDirector.cs` (Refinery, Blueprints, Fabricator, GadgetSite placement/removal).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::{CraftCost, DataRegistry, GameModeSettings, GadgetDefinition};
use sr_ids::{GadgetId, IdentifiableId, PediaId, ProgressType, ZoneId};
use sr_model::{BlueprintLockData, GadgetKind, GadgetModel, GadgetSiteModel, GameModel};

use crate::player::PlayerController;

pub const REFINERY_MAX: i32 = 999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GadgetError {
    BlueprintsDisabled,
    BlueprintNotAvailable,
    BlueprintAlreadyOwned,
    BlueprintNotOwned,
    InsufficientCurrency { required: i32, current: i32 },
    InsufficientMaterials,
    BuyLimitReached,
    GadgetNotInInventory,
    SiteNotFound,
    SiteOccupied,
    SiteEmpty,
    PlacementLimitReached,
    DroneNotOnRanch,
    DroneNetworkLimitReached,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlueprintUnlockRule {
    pub id: GadgetId,
    pub wait_for_blueprints: Vec<GadgetId>,
    pub wait_for_progress: Vec<ProgressType>,
    pub min_viktor_rewards: i32,
    pub delay_hours: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GadgetDirectorStepEvents {
    pub newly_available_blueprints: Vec<GadgetId>,
}

/// Port of `GadgetDirector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GadgetDirector {
    pub definitions: HashMap<GadgetId, GadgetDefinition>,
    pub unlock_rules: Vec<BlueprintUnlockRule>,
    pub site_zones: HashMap<String, ZoneId>,
}

impl Default for GadgetDirector {
    fn default() -> Self {
        Self {
            definitions: builtin_gadget_definitions(),
            unlock_rules: builtin_blueprint_unlock_rules(),
            site_zones: HashMap::new(),
        }
    }
}

impl GadgetDirector {
    pub fn from_registry(data: &DataRegistry) -> Self {
        let mut defs = builtin_gadget_definitions();
        for (k, v) in &data.gadgets {
            defs.insert(*k, v.clone());
        }
        Self {
            definitions: defs,
            unlock_rules: builtin_blueprint_unlock_rules(),
            site_zones: HashMap::new(),
        }
    }

    pub fn init_model(&self, model: &mut GameModel, mode: &GameModeSettings) {
        model.gadgets.init_initial_blueprints();
        model.gadgets.blueprint_locks.clear();
        if mode.blueprints_enabled {
            for rule in &self.unlock_rules {
                if !model.gadgets.avail_blueprints.contains(&rule.id)
                    && !model.gadgets.blueprints.contains(&rule.id)
                {
                    model.gadgets.blueprint_locks.insert(
                        rule.id,
                        BlueprintLockData {
                            timed_lock: false,
                            locked_until: f64::INFINITY,
                        },
                    );
                }
            }
        }
    }

    pub fn ensure_site(&mut self, model: &mut GameModel, site_id: impl Into<String>, zone: ZoneId) {
        let sid = site_id.into();
        self.site_zones.insert(sid.clone(), zone);
        model
            .gadget_sites
            .entry(sid.clone())
            .or_insert_with(|| GadgetSiteModel {
                id: sid,
                attached: None,
            });
    }

    /// Port of `GadgetDirector.IsRefineryResource`.
    pub fn is_refinery_resource(id: IdentifiableId) -> bool {
        id.is_plort() || id.is_craft()
    }

    pub fn get_refinery_count(model: &GameModel, id: IdentifiableId) -> i32 {
        model.gadgets.get_refinery_count(id)
    }

    pub fn get_refinery_space_available(model: &GameModel, id: IdentifiableId) -> i32 {
        if !Self::is_refinery_resource(id) {
            return 0;
        }
        (REFINERY_MAX - Self::get_refinery_count(model, id)).max(0)
    }

    /// Port of `GadgetDirector.AddToRefinery`.
    pub fn add_to_refinery(model: &mut GameModel, id: IdentifiableId, count: i32) -> i32 {
        if count <= 0 || !Self::is_refinery_resource(id) {
            return 0;
        }
        let avail = Self::get_refinery_space_available(model, id);
        let added = count.min(avail);
        if added > 0 {
            model.gadgets.add_refinery_item(id, added);
        }
        added
    }

    pub fn has_in_refinery(model: &GameModel, costs: &[CraftCost]) -> bool {
        costs
            .iter()
            .all(|c| Self::get_refinery_count(model, c.id) >= c.amount)
    }

    pub fn try_spend_from_refinery(model: &mut GameModel, costs: &[CraftCost]) -> bool {
        if !Self::has_in_refinery(model, costs) {
            return false;
        }
        for c in costs {
            if c.amount > 0 {
                if let Some(v) = model.gadgets.craft_mat_counts.get_mut(&c.id) {
                    *v -= c.amount;
                }
            }
        }
        true
    }

    /// Checks blueprint lock conditions & unlocks timed blueprints in `GadgetsModel.avail_blueprints`.
    pub fn step_blueprint_locks(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
    ) -> GadgetDirectorStepEvents {
        let mut ev = GadgetDirectorStepEvents::default();
        if !mode.blueprints_enabled {
            return ev;
        }
        let wt = model.world.world_time;
        let mut to_unlock = Vec::new();

        for rule in &self.unlock_rules {
            if model.gadgets.blueprints.contains(&rule.id)
                || model.gadgets.avail_blueprints.contains(&rule.id)
            {
                continue;
            }
            let cond_ok = rule
                .wait_for_blueprints
                .iter()
                .all(|b| model.gadgets.blueprints.contains(b))
                && rule
                    .wait_for_progress
                    .iter()
                    .all(|p| model.progress.get_progress(*p) > 0)
                && (rule.min_viktor_rewards <= 0
                    || model.progress.get_progress(ProgressType::ViktorRewards)
                        >= rule.min_viktor_rewards);

            let entry = model
                .gadgets
                .blueprint_locks
                .entry(rule.id)
                .or_insert(BlueprintLockData {
                    timed_lock: false,
                    locked_until: f64::INFINITY,
                });

            if cond_ok && !entry.timed_lock {
                entry.timed_lock = true;
                entry.locked_until = wt + (rule.delay_hours as f64) * 3600.0;
            }

            if entry.timed_lock && wt >= entry.locked_until {
                to_unlock.push(rule.id);
            }
        }

        for id in to_unlock {
            model.gadgets.blueprint_locks.remove(&id);
            if !model.gadgets.avail_blueprints.contains(&id) {
                model.gadgets.avail_blueprints.push(id);
                ev.newly_available_blueprints.push(id);
            }
        }

        ev
    }

    /// Port of Builder's Shop blueprint purchase.
    pub fn buy_blueprint(
        &self,
        model: &mut GameModel,
        mode: &GameModeSettings,
        id: GadgetId,
    ) -> Result<i32, GadgetError> {
        if !mode.blueprints_enabled {
            return Err(GadgetError::BlueprintsDisabled);
        }
        if model.gadgets.blueprints.contains(&id) {
            return Err(GadgetError::BlueprintAlreadyOwned);
        }
        if !model.gadgets.avail_blueprints.contains(&id)
            && !model.gadgets.registered_blueprints.contains(&id)
        {
            return Err(GadgetError::BlueprintNotAvailable);
        }
        let cost = self
            .definitions
            .get(&id)
            .map(|d| d.blueprint_cost)
            .unwrap_or(500);
        if !PlayerController::spend_currency(&mut model.player, cost) {
            return Err(GadgetError::InsufficientCurrency {
                required: cost,
                current: model.player.currency,
            });
        }
        model.gadgets.avail_blueprints.retain(|&b| b != id);
        model.gadgets.blueprints.push(id);
        Ok(cost)
    }

    pub fn get_gadget_count(model: &GameModel, id: GadgetId) -> i32 {
        model.gadgets.gadgets.get(&id).copied().unwrap_or(0)
    }

    pub fn get_placed_count(model: &GameModel, ids: &[GadgetId]) -> i32 {
        ids.iter()
            .map(|id| model.gadgets.placed_gadget_counts.get(id).copied().unwrap_or(0))
            .sum()
    }

    /// Port of Fabricator crafting (`GadgetDirector.AddGadget` + `TryToSpendFromRefinery`).
    pub fn fabricate_gadget(
        &self,
        model: &mut GameModel,
        id: GadgetId,
    ) -> Result<i32, GadgetError> {
        if !model.gadgets.blueprints.contains(&id) {
            return Err(GadgetError::BlueprintNotOwned);
        }
        let def = self.definitions.get(&id);
        if let Some(d) = def {
            if d.buy_count_limit > 0 {
                let total = Self::get_gadget_count(model, id) + Self::get_placed_count(model, &[id]);
                if total >= d.buy_count_limit {
                    return Err(GadgetError::BuyLimitReached);
                }
            }
            if !Self::try_spend_from_refinery(model, &d.craft_costs) {
                return Err(GadgetError::InsufficientMaterials);
            }
        }
        let add_amount = if def.map_or(false, |d| d.buy_in_pairs) {
            2
        } else {
            1
        };
        *model.gadgets.gadgets.entry(id).or_insert(0) += add_amount;
        Ok(add_amount)
    }

    /// Places a crafted gadget on a `GadgetSiteModel`, checking placement/network limits.
    pub fn place_gadget(
        &self,
        model: &mut GameModel,
        site_id: &str,
        id: GadgetId,
        y_rotation: f32,
    ) -> Result<(), GadgetError> {
        if Self::get_gadget_count(model, id) <= 0 {
            return Err(GadgetError::GadgetNotInInventory);
        }
        {
            let site = model
                .gadget_sites
                .get(site_id)
                .ok_or(GadgetError::SiteNotFound)?;
            if site.attached.is_some() {
                return Err(GadgetError::SiteOccupied);
            }
        }

        if let Some(def) = self.definitions.get(&id) {
            if id.is_drone() {
                let zone = self
                    .site_zones
                    .get(site_id)
                    .copied()
                    .unwrap_or(ZoneId::Ranch);
                if zone != ZoneId::Ranch
                    && zone != ZoneId::OgdenRanch
                    && zone != ZoneId::MochiRanch
                    && zone != ZoneId::ViktorLab
                {
                    return Err(GadgetError::DroneNotOnRanch);
                }
                let drones_in_zone = model
                    .gadget_sites
                    .values()
                    .filter(|s| {
                        s.attached.as_ref().map_or(false, |g| g.ident.is_drone())
                            && self.site_zones.get(&s.id).copied() == Some(zone)
                    })
                    .count() as i32;
                if def.count_limit > 0 && drones_in_zone >= def.count_limit {
                    return Err(GadgetError::DroneNetworkLimitReached);
                }
            } else if def.count_limit > 0 {
                let ids = def.gadgets_to_count_ids();
                if Self::get_placed_count(model, &ids) >= def.count_limit {
                    return Err(GadgetError::PlacementLimitReached);
                }
            }
        }

        // Spend from inventory
        if let Some(c) = model.gadgets.gadgets.get_mut(&id) {
            *c -= 1;
            if *c <= 0 {
                model.gadgets.gadgets.remove(&id);
            }
        }

        let mut gmodel = GadgetModel::new(id, site_id, y_rotation);
        if let GadgetKind::Drone(ref mut dm) = gmodel.kind {
            let count = if id == GadgetId::DroneAdvanced { 2 } else { 1 };
            dm.programs = vec![Default::default(); count];
            dm.battery_deplete_time = model.world.world_time + 28.0 * 3600.0;
        }

        if let Some(site) = model.gadget_sites.get_mut(site_id) {
            site.attached = Some(gmodel);
        }
        *model.gadgets.placed_gadget_counts.entry(id).or_insert(0) += 1;
        Ok(())
    }

    /// Removes a gadget from a `GadgetSiteModel`.
    /// Respects `destroy_on_removal` and paired `Teleporter` demolition.
    pub fn remove_gadget(&self, model: &mut GameModel, site_id: &str) -> Result<GadgetId, GadgetError> {
        let removed_id = {
            let site = model
                .gadget_sites
                .get_mut(site_id)
                .ok_or(GadgetError::SiteNotFound)?;
            let Some(g) = site.attached.take() else {
                return Err(GadgetError::SiteEmpty);
            };
            g.ident
        };

        if let Some(c) = model.gadgets.placed_gadget_counts.get_mut(&removed_id) {
            *c = (*c - 1).max(0);
        }

        let destroy = self
            .definitions
            .get(&removed_id)
            .map_or(false, |d| d.destroy_on_removal);
        if !destroy {
            *model.gadgets.gadgets.entry(removed_id).or_insert(0) += 1;
        }

        // Teleporters destroy their linked partner on removal (TeleporterGadget.ShouldDestroyPair)
        if removed_id.is_teleporter() {
            let partner_site = model
                .gadget_sites
                .iter()
                .find(|(sid, s)| {
                    sid.as_str() != site_id
                        && s.attached
                            .as_ref()
                            .map_or(false, |g| g.ident == removed_id)
                })
                .map(|(sid, _)| sid.clone());
            if let Some(psid) = partner_site {
                if let Some(ps) = model.gadget_sites.get_mut(&psid) {
                    ps.attached = None;
                }
                if let Some(c) = model.gadgets.placed_gadget_counts.get_mut(&removed_id) {
                    *c = (*c - 1).max(0);
                }
            }
        }

        Ok(removed_id)
    }
}

fn def(
    id: GadgetId,
    blueprint_cost: i32,
    craft_costs: Vec<(IdentifiableId, i32)>,
    buy_count_limit: i32,
    count_limit: i32,
    destroy_on_removal: bool,
    buy_in_pairs: bool,
) -> GadgetDefinition {
    GadgetDefinition {
        id,
        pedia_link: PediaId::Science,
        blueprint_cost,
        craft_costs: craft_costs
            .into_iter()
            .map(|(id, amount)| CraftCost { id, amount })
            .collect(),
        buy_count_limit,
        count_limit,
        count_other_ids: Vec::new(),
        destroy_on_removal,
        buy_in_pairs,
    }
}

/// Canonical gadget blueprint/craft recipes for core Slime Science items.
pub fn builtin_gadget_definitions() -> HashMap<GadgetId, GadgetDefinition> {
    use GadgetId as G;
    use IdentifiableId as Id;
    let mut map = HashMap::new();

    let drill_ids = vec![
        G::ExtractorDrillNovice,
        G::ExtractorDrillAdvanced,
        G::ExtractorDrillMaster,
        G::ExtractorDrillTitan,
    ];
    let pump_ids = vec![
        G::ExtractorPumpNovice,
        G::ExtractorPumpAdvanced,
        G::ExtractorPumpMaster,
        G::ExtractorPumpAbyssal,
    ];
    let apiary_ids = vec![
        G::ExtractorApiaryNovice,
        G::ExtractorApiaryAdvanced,
        G::ExtractorApiaryMaster,
        G::ExtractorApiaryRoyal,
    ];

    for (id, cost, mats) in [
        (G::ExtractorDrillNovice, 500, vec![(Id::PinkPlort, 10), (Id::RockPlort, 6), (Id::RadPlort, 3)]),
        (G::ExtractorDrillAdvanced, 1500, vec![(Id::RockPlort, 12), (Id::CrystalPlort, 4), (Id::PrimordyOilCraft, 4)]),
        (G::ExtractorDrillMaster, 3500, vec![(Id::RadPlort, 15), (Id::BoomPlort, 10), (Id::StrangeDiamondCraft, 1)]),
        (G::ExtractorDrillTitan, 0, vec![(Id::QuantumPlort, 12), (Id::MosaicPlort, 8), (Id::LavaDustCraft, 6)]),
    ] {
        let mut d = def(id, cost, mats, -1, 6, true, false);
        d.count_other_ids = drill_ids.iter().copied().filter(|&x| x != id).collect();
        map.insert(id, d);
    }

    for (id, cost, mats) in [
        (G::ExtractorPumpNovice, 500, vec![(Id::PinkPlort, 10), (Id::PhosphorPlort, 6), (Id::TabbyPlort, 3)]),
        (G::ExtractorPumpAdvanced, 1500, vec![(Id::PhosphorPlort, 12), (Id::PuddlePlort, 4), (Id::DeepBrineCraft, 4)]),
        (G::ExtractorPumpMaster, 3500, vec![(Id::PuddlePlort, 12), (Id::HunterPlort, 8), (Id::StrangeDiamondCraft, 1)]),
        (G::ExtractorPumpAbyssal, 0, vec![(Id::DervishPlort, 12), (Id::PuddlePlort, 10), (Id::SilkySandCraft, 6)]),
    ] {
        let mut d = def(id, cost, mats, -1, 6, true, false);
        d.count_other_ids = pump_ids.iter().copied().filter(|&x| x != id).collect();
        map.insert(id, d);
    }

    for (id, cost, mats) in [
        (G::ExtractorApiaryNovice, 500, vec![(Id::PinkPlort, 10), (Id::HoneyPlort, 6), (Id::PhosphorPlort, 3)]),
        (G::ExtractorApiaryAdvanced, 1500, vec![(Id::HoneyPlort, 12), (Id::BoomPlort, 4), (Id::BuzzWaxCraft, 4)]),
        (G::ExtractorApiaryMaster, 3500, vec![(Id::HoneyPlort, 15), (Id::TanglePlort, 6), (Id::StrangeDiamondCraft, 1)]),
        (G::ExtractorApiaryRoyal, 0, vec![(Id::TanglePlort, 12), (Id::HoneyPlort, 12), (Id::RoyalJellyCraft, 2)]),
    ] {
        let mut d = def(id, cost, mats, -1, 6, true, false);
        d.count_other_ids = apiary_ids.iter().copied().filter(|&x| x != id).collect();
        map.insert(id, d);
    }

    for tp in [
        G::TeleporterPink,
        G::TeleporterBlue,
        G::TeleporterViolet,
        G::TeleporterGrey,
        G::TeleporterGreen,
        G::TeleporterRed,
        G::TeleporterAmber,
        G::TeleporterGold,
        G::TeleporterBerry,
        G::TeleporterCocoa,
        G::TeleporterButterscotch,
    ] {
        map.insert(
            tp,
            def(tp, 1250, vec![(Id::PinkPlort, 15), (Id::StrangeDiamondCraft, 1)], 2, 2, false, true),
        );
    }

    for wd in [
        G::WarpDepotPink,
        G::WarpDepotBlue,
        G::WarpDepotViolet,
        G::WarpDepotGrey,
        G::WarpDepotGreen,
        G::WarpDepotRed,
        G::WarpDepotAmber,
        G::WarpDepotGold,
        G::WarpDepotBerry,
        G::WarpDepotCocoa,
        G::WarpDepotButterscotch,
    ] {
        map.insert(
            wd,
            def(wd, 750, vec![(Id::PinkPlort, 10), (Id::DeepBrineCraft, 4)], 2, 2, false, true),
        );
    }

    map.insert(G::MarketLink, def(G::MarketLink, 0, vec![(Id::PinkPlort, 10), (Id::IndigoniumCraft, 4), (Id::DeepBrineCraft, 4)], -1, -1, false, false));
    map.insert(G::RefineryLink, def(G::RefineryLink, 1000, vec![(Id::PinkPlort, 10), (Id::PrimordyOilCraft, 4)], -1, -1, false, false));
    map.insert(G::GordoSnareNovice, def(G::GordoSnareNovice, 450, vec![(Id::PinkPlort, 10), (Id::BuzzWaxCraft, 3)], -1, -1, true, false));
    map.insert(G::GordoSnareAdvanced, def(G::GordoSnareAdvanced, 1200, vec![(Id::HunterPlort, 6), (Id::WildHoneyCraft, 3)], -1, -1, true, false));
    map.insert(G::GordoSnareMaster, def(G::GordoSnareMaster, 0, vec![(Id::HunterPlort, 10), (Id::RoyalJellyCraft, 2)], -1, -1, true, false));
    map.insert(G::EchoNet, def(G::EchoNet, 1000, vec![(Id::PhosphorPlort, 12), (Id::SpiralSteamCraft, 4)], -1, -1, false, false));
    map.insert(G::MedStation, def(G::MedStation, 500, vec![(Id::PinkPlort, 10), (Id::DeepBrineCraft, 3)], -1, -1, false, false));
    map.insert(G::HydroTurret, def(G::HydroTurret, 750, vec![(Id::RockPlort, 10), (Id::DeepBrineCraft, 4)], -1, -1, false, false));
    map.insert(G::TamingBell, def(G::TamingBell, 750, vec![(Id::HoneyPlort, 10), (Id::BuzzWaxCraft, 4)], -1, -1, false, false));
    map.insert(G::DashPad, def(G::DashPad, 750, vec![(Id::TabbyPlort, 10), (Id::PrimordyOilCraft, 4)], -1, -1, false, false));
    map.insert(G::ChickenCloner, def(G::ChickenCloner, 1500, vec![(Id::TabbyPlort, 10), (Id::ManifoldCubeCraft, 2)], -1, -1, false, false));
    map.insert(G::PortableWaterTap, def(G::PortableWaterTap, 750, vec![(Id::PuddlePlort, 8), (Id::DeepBrineCraft, 4)], -1, -1, false, false));
    map.insert(G::PortableSlimeBaitFruit, def(G::PortableSlimeBaitFruit, 500, vec![(Id::PhosphorPlort, 8)], -1, -1, true, false));
    map.insert(G::PortableSlimeBaitVeggie, def(G::PortableSlimeBaitVeggie, 500, vec![(Id::RockPlort, 8)], -1, -1, true, false));
    map.insert(G::PortableSlimeBaitMeat, def(G::PortableSlimeBaitMeat, 500, vec![(Id::TabbyPlort, 8)], -1, -1, true, false));
    map.insert(G::Drone, def(G::Drone, 1500, vec![(Id::HoneyPlort, 12), (Id::WildHoneyCraft, 4), (Id::HexacombCraft, 2)], -1, 2, false, false));
    map.insert(G::DroneAdvanced, def(G::DroneAdvanced, 2500, vec![(Id::QuantumPlort, 10), (Id::ManifoldCubeCraft, 2), (Id::RoyalJellyCraft, 2)], -1, 2, false, false));

    map
}

pub fn builtin_blueprint_unlock_rules() -> Vec<BlueprintUnlockRule> {
    use GadgetId as G;
    use ProgressType as P;
    vec![
        BlueprintUnlockRule { id: G::ExtractorDrillAdvanced, wait_for_blueprints: vec![], wait_for_progress: vec![P::UnlockLab], min_viktor_rewards: 0, delay_hours: 126.0 },
        BlueprintUnlockRule { id: G::ExtractorPumpAdvanced, wait_for_blueprints: vec![G::ExtractorPumpNovice], wait_for_progress: vec![], min_viktor_rewards: 0, delay_hours: 120.0 },
        BlueprintUnlockRule { id: G::ExtractorApiaryAdvanced, wait_for_blueprints: vec![G::ExtractorApiaryNovice], wait_for_progress: vec![], min_viktor_rewards: 0, delay_hours: 120.0 },
        BlueprintUnlockRule { id: G::RefineryLink, wait_for_blueprints: vec![G::ExtractorDrillAdvanced, G::ExtractorPumpAdvanced, G::ExtractorApiaryAdvanced], wait_for_progress: vec![], min_viktor_rewards: 0, delay_hours: 24.0 },
        BlueprintUnlockRule { id: G::TeleporterBlue, wait_for_blueprints: vec![G::TeleporterPink], wait_for_progress: vec![P::UnlockQuarry], min_viktor_rewards: 0, delay_hours: 48.0 },
        BlueprintUnlockRule { id: G::TeleporterGrey, wait_for_blueprints: vec![G::TeleporterPink], wait_for_progress: vec![P::UnlockMoss], min_viktor_rewards: 0, delay_hours: 48.0 },
        BlueprintUnlockRule { id: G::TeleporterButterscotch, wait_for_blueprints: vec![G::TeleporterPink], wait_for_progress: vec![P::UnlockDesert], min_viktor_rewards: 0, delay_hours: 0.5 },
        BlueprintUnlockRule { id: G::WarpDepotBlue, wait_for_blueprints: vec![G::WarpDepotPink], wait_for_progress: vec![P::UnlockQuarry], min_viktor_rewards: 0, delay_hours: 48.0 },
        BlueprintUnlockRule { id: G::WarpDepotGrey, wait_for_blueprints: vec![G::WarpDepotPink], wait_for_progress: vec![P::UnlockMoss], min_viktor_rewards: 0, delay_hours: 48.0 },
        BlueprintUnlockRule { id: G::WarpDepotButterscotch, wait_for_blueprints: vec![G::WarpDepotPink], wait_for_progress: vec![P::UnlockDesert], min_viktor_rewards: 0, delay_hours: 0.5 },
        BlueprintUnlockRule { id: G::ChickenCloner, wait_for_blueprints: vec![], wait_for_progress: vec![], min_viktor_rewards: 1, delay_hours: 0.0 },
        BlueprintUnlockRule { id: G::DroneAdvanced, wait_for_blueprints: vec![], wait_for_progress: vec![], min_viktor_rewards: 2, delay_hours: 0.0 },
    ]
}
