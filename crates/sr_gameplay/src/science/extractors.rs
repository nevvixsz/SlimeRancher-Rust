//! Port of `Extractor.cs` (Drill, Pump, Apiary across Novice/Advanced/Master/Titan tiers
//! and zone-restricted resource tables).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_ids::{GadgetId, IdentifiableId, RegionSetId, ZoneId};
use sr_math::{Randoms, Vec3};
use sr_model::{GadgetKind, GameModel};
use sr_physics::PhysicsWorld;

use crate::actors::spawn_physical_actor;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractorProduceEntry {
    pub id: IdentifiableId,
    pub weight: f32,
    pub restrict_zone: Option<ZoneId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractorSpec {
    pub cycles: i32,
    pub infinite_cycles: bool,
    pub produce_min: i32,
    pub produce_max: i32,
    pub hours_per_cycle: f32,
    pub produces: Vec<ExtractorProduceEntry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractorStepEvents {
    pub ready_sites: Vec<String>,
    pub spawned_items: Vec<(String, i64, IdentifiableId)>,
    pub expired_sites: Vec<String>,
}

fn region_for_zone(z: ZoneId) -> RegionSetId {
    match z {
        ZoneId::Desert => RegionSetId::Desert,
        ZoneId::Valley | ZoneId::MochiRanch => RegionSetId::Valley,
        ZoneId::Slimulations => RegionSetId::Slimulations,
        ZoneId::ViktorLab => RegionSetId::ViktorLab,
        _ => RegionSetId::Home,
    }
}

pub fn spec_for_extractor(id: GadgetId) -> Option<ExtractorSpec> {
    use GadgetId as G;
    use IdentifiableId as Id;

    let drill_produces = vec![
        ExtractorProduceEntry { id: Id::PrimordyOilCraft, weight: 3.0, restrict_zone: None },
        ExtractorProduceEntry { id: Id::JellystoneCraft, weight: 3.0, restrict_zone: None },
        ExtractorProduceEntry { id: Id::SlimeFossilCraft, weight: 2.0, restrict_zone: None },
        ExtractorProduceEntry { id: Id::StrangeDiamondCraft, weight: 0.25, restrict_zone: None },
        ExtractorProduceEntry { id: Id::IndigoniumCraft, weight: 3.0, restrict_zone: Some(ZoneId::Quarry) },
        ExtractorProduceEntry { id: Id::LavaDustCraft, weight: 2.5, restrict_zone: Some(ZoneId::Desert) },
        ExtractorProduceEntry { id: Id::GlassShardCraft, weight: 2.5, restrict_zone: Some(ZoneId::Desert) },
    ];
    let pump_produces = vec![
        ExtractorProduceEntry { id: Id::PrimordyOilCraft, weight: 3.0, restrict_zone: None },
        ExtractorProduceEntry { id: Id::SpiralSteamCraft, weight: 3.0, restrict_zone: None },
        ExtractorProduceEntry { id: Id::StrangeDiamondCraft, weight: 0.2, restrict_zone: None },
        ExtractorProduceEntry { id: Id::DeepBrineCraft, weight: 3.5, restrict_zone: Some(ZoneId::Reef) },
        ExtractorProduceEntry { id: Id::SilkySandCraft, weight: 3.0, restrict_zone: Some(ZoneId::Desert) },
    ];
    let apiary_produces = vec![
        ExtractorProduceEntry { id: Id::BuzzWaxCraft, weight: 3.5, restrict_zone: None },
        ExtractorProduceEntry { id: Id::HexacombCraft, weight: 2.5, restrict_zone: None },
        ExtractorProduceEntry { id: Id::RoyalJellyCraft, weight: 0.5, restrict_zone: None },
        ExtractorProduceEntry { id: Id::WildHoneyCraft, weight: 3.5, restrict_zone: Some(ZoneId::Moss) },
        ExtractorProduceEntry { id: Id::PepperJamCraft, weight: 3.0, restrict_zone: Some(ZoneId::Desert) },
    ];

    match id {
        G::ExtractorDrillNovice => Some(ExtractorSpec { cycles: 1, infinite_cycles: false, produce_min: 4, produce_max: 8, hours_per_cycle: 12.0, produces: drill_produces }),
        G::ExtractorDrillAdvanced => Some(ExtractorSpec { cycles: 6, infinite_cycles: false, produce_min: 6, produce_max: 10, hours_per_cycle: 12.0, produces: drill_produces }),
        G::ExtractorDrillMaster => Some(ExtractorSpec { cycles: 12, infinite_cycles: false, produce_min: 8, produce_max: 12, hours_per_cycle: 12.0, produces: drill_produces }),
        G::ExtractorDrillTitan => Some(ExtractorSpec { cycles: 20, infinite_cycles: false, produce_min: 10, produce_max: 15, hours_per_cycle: 12.0, produces: drill_produces }),
        G::ExtractorPumpNovice => Some(ExtractorSpec { cycles: 1, infinite_cycles: false, produce_min: 4, produce_max: 8, hours_per_cycle: 12.0, produces: pump_produces }),
        G::ExtractorPumpAdvanced => Some(ExtractorSpec { cycles: 6, infinite_cycles: false, produce_min: 6, produce_max: 10, hours_per_cycle: 12.0, produces: pump_produces }),
        G::ExtractorPumpMaster => Some(ExtractorSpec { cycles: 12, infinite_cycles: false, produce_min: 8, produce_max: 12, hours_per_cycle: 12.0, produces: pump_produces }),
        G::ExtractorPumpAbyssal => Some(ExtractorSpec { cycles: 20, infinite_cycles: false, produce_min: 10, produce_max: 15, hours_per_cycle: 12.0, produces: pump_produces }),
        G::ExtractorApiaryNovice => Some(ExtractorSpec { cycles: 1, infinite_cycles: false, produce_min: 4, produce_max: 8, hours_per_cycle: 12.0, produces: apiary_produces }),
        G::ExtractorApiaryAdvanced => Some(ExtractorSpec { cycles: 6, infinite_cycles: false, produce_min: 6, produce_max: 10, hours_per_cycle: 12.0, produces: apiary_produces }),
        G::ExtractorApiaryMaster => Some(ExtractorSpec { cycles: 12, infinite_cycles: false, produce_min: 8, produce_max: 12, hours_per_cycle: 12.0, produces: apiary_produces }),
        G::ExtractorApiaryRoyal => Some(ExtractorSpec { cycles: 20, infinite_cycles: false, produce_min: 10, produce_max: 15, hours_per_cycle: 12.0, produces: apiary_produces }),
        _ => None,
    }
}

/// Runtime state and simulation logic for placed Drill/Pump/Apiary extractors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractorSystem {
    pub cycle_end_times: HashMap<String, f64>,
    pub site_positions: HashMap<String, Vec3>,
}

impl ExtractorSystem {
    pub fn register_site_pos(&mut self, site_id: impl Into<String>, pos: Vec3) {
        self.site_positions.insert(site_id.into(), pos);
    }

    pub fn is_ready(&self, model: &GameModel, site_id: &str) -> bool {
        let Some(end_t) = self.cycle_end_times.get(site_id) else {
            return false;
        };
        model.world.world_time >= *end_t
    }

    /// Port of `Extractor.OnInteract()`: queues `produceMin..=produceMax` items for ejection.
    pub fn interact_extractor(
        &mut self,
        model: &mut GameModel,
        site_id: &str,
        rng: &mut Randoms,
    ) -> bool {
        let wt = model.world.world_time;
        let Some(&end_t) = self.cycle_end_times.get(site_id) else {
            return false;
        };
        if wt < end_t {
            return false;
        }
        let Some(site) = model.gadget_sites.get_mut(site_id) else {
            return false;
        };
        let Some(ref mut gadget) = site.attached else {
            return false;
        };
        let Some(spec) = spec_for_extractor(gadget.ident) else {
            return false;
        };
        let GadgetKind::Extractor(ref mut ex) = gadget.kind else {
            return false;
        };
        ex.queued_to_produce = rng.get_in_range_i32(spec.produce_min, spec.produce_max + 1);
        ex.next_produce_time = wt;
        self.cycle_end_times.insert(site_id.to_string(), f64::INFINITY);
        true
    }

    /// Port of `Extractor.Update()`: initializes fresh extractors, ejects queued items,
    /// starts subsequent cycles, and destroys spent extractors (`cycles_remaining <= 0`).
    pub fn step(
        &mut self,
        model: &mut GameModel,
        physics: &mut PhysicsWorld,
        site_zones: &HashMap<String, ZoneId>,
        rng: &mut Randoms,
    ) -> ExtractorStepEvents {
        let mut ev = ExtractorStepEvents::default();
        let wt = model.world.world_time;

        let site_ids: Vec<String> = model.gadget_sites.keys().cloned().collect();
        let mut to_destroy = Vec::new();

        for sid in site_ids {
            let zone = site_zones.get(&sid).copied().unwrap_or(ZoneId::Ranch);
            let spawn_pos = self
                .site_positions
                .get(&sid)
                .copied()
                .unwrap_or(Vec3::new(0.0, 1.0, 0.0));

            let mut pending_spawn: Option<(IdentifiableId, Vec3)> = None;

            {
                let Some(site) = model.gadget_sites.get_mut(&sid) else {
                    continue;
                };
                let Some(ref mut gadget) = site.attached else {
                    self.cycle_end_times.remove(&sid);
                    continue;
                };
                let Some(spec) = spec_for_extractor(gadget.ident) else {
                    continue;
                };
                let GadgetKind::Extractor(ref mut ex) = gadget.kind else {
                    continue;
                };

                // Initialize newly placed extractor
                if !self.cycle_end_times.contains_key(&sid) {
                    if ex.cycles_remaining == 0 && ex.queued_to_produce == 0 {
                        ex.cycles_remaining = if spec.infinite_cycles {
                            spec.cycles
                        } else {
                            (spec.cycles - 1).max(0)
                        };
                    }
                    self.cycle_end_times
                        .insert(sid.clone(), wt + (spec.hours_per_cycle as f64) * 3600.0);
                    continue;
                }

                if ex.queued_to_produce > 0 {
                    if wt >= ex.next_produce_time {
                        // Pick weighted zone-valid craft resource
                        let eligible: Vec<&ExtractorProduceEntry> = spec
                            .produces
                            .iter()
                            .filter(|p| p.restrict_zone.map_or(true, |rz| rz == zone))
                            .collect();
                        let total_w: f32 = eligible.iter().map(|e| e.weight.max(0.0)).sum();
                        if total_w > 0.0 {
                            let mut roll = rng.get_in_range_f32(0.0, total_w);
                            let mut chosen = eligible[0].id;
                            for e in &eligible {
                                roll -= e.weight.max(0.0);
                                if roll <= 0.0 {
                                    chosen = e.id;
                                    break;
                                }
                            }
                            let vel = Vec3::new(
                                rng.get_in_range_f32(-1.5, 1.5),
                                3.0,
                                rng.get_in_range_f32(-1.5, 1.5),
                            );
                            pending_spawn = Some((chosen, vel));
                        }

                        ex.queued_to_produce -= 1;
                        if ex.queued_to_produce <= 0 {
                            ex.next_produce_time = 0.0;
                            if ex.cycles_remaining <= 0 && !spec.infinite_cycles {
                                to_destroy.push((sid.clone(), gadget.ident));
                            } else {
                                if !spec.infinite_cycles {
                                    ex.cycles_remaining -= 1;
                                }
                                self.cycle_end_times.insert(
                                    sid.clone(),
                                    wt + (spec.hours_per_cycle as f64) * 3600.0,
                                );
                            }
                        } else {
                            ex.next_produce_time = wt + 0.004166667 * 3600.0;
                        }
                    }
                } else if let Some(&end_t) = self.cycle_end_times.get(&sid) {
                    if wt >= end_t {
                        ev.ready_sites.push(sid.clone());
                    }
                }
            }

            if let Some((chosen, vel)) = pending_spawn {
                let aid = spawn_physical_actor(
                    model,
                    physics,
                    chosen,
                    region_for_zone(zone),
                    spawn_pos + Vec3::Y * 1.2,
                    vel,
                );
                ev.spawned_items.push((sid.clone(), aid, chosen));
            }
        }

        for (sid, gid) in to_destroy {
            if let Some(site) = model.gadget_sites.get_mut(&sid) {
                site.attached = None;
            }
            if let Some(c) = model.gadgets.placed_gadget_counts.get_mut(&gid) {
                *c = (*c - 1).max(0);
            }
            self.cycle_end_times.remove(&sid);
            ev.expired_sites.push(sid);
        }

        ev
    }
}
