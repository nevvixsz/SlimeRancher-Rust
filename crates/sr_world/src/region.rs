//! Port of `RegionRegistry.cs`, `Region.cs`, `RegionMember.cs`, and `ZoneDirector.GetRegionSetId`.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_ids::{RegionSetId, ZoneId};
use sr_math::Vec3;

/// Port of `ZoneDirector.GetRegionSetId(Zone)`.
pub fn region_set_for_zone(zone: ZoneId) -> RegionSetId {
    match zone {
        ZoneId::Ranch
        | ZoneId::Reef
        | ZoneId::Quarry
        | ZoneId::Moss
        | ZoneId::Sea
        | ZoneId::Ruins
        | ZoneId::RuinsTransition
        | ZoneId::Wilds
        | ZoneId::OgdenRanch => RegionSetId::Home,
        ZoneId::Desert => RegionSetId::Desert,
        ZoneId::Valley | ZoneId::MochiRanch => RegionSetId::Valley,
        ZoneId::Slimulations => RegionSetId::Slimulations,
        ZoneId::ViktorLab => RegionSetId::ViktorLab,
        ZoneId::None => RegionSetId::Unset,
    }
}

/// Axis-aligned region cell volume matching `Region.cs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub set_id: RegionSetId,
    pub zone: ZoneId,
    pub center: Vec3,
    pub half_extents: Vec3,
    pub non_proxy_ref_count: i32,
    pub non_hibernate_ref_count: i32,
    pub last_hibernated_at: Option<f64>,
    pub members: HashSet<i64>,
}

impl Region {
    pub fn new(
        id: impl Into<String>,
        zone: ZoneId,
        center: Vec3,
        half_extents: Vec3,
    ) -> Self {
        Self {
            id: id.into(),
            set_id: region_set_for_zone(zone),
            zone,
            center,
            half_extents,
            non_proxy_ref_count: 0,
            non_hibernate_ref_count: 0,
            last_hibernated_at: None,
            members: HashSet::new(),
        }
    }

    #[inline]
    pub fn is_proxied(&self) -> bool {
        self.non_proxy_ref_count <= 0
    }

    #[inline]
    pub fn is_hibernated(&self) -> bool {
        self.non_hibernate_ref_count <= 0
    }

    pub fn contains(&self, pos: Vec3) -> bool {
        (pos.x - self.center.x).abs() <= self.half_extents.x
            && (pos.y - self.center.y).abs() <= self.half_extents.y
            && (pos.z - self.center.z).abs() <= self.half_extents.z
    }

    pub fn distance_sq(&self, pos: Vec3) -> f32 {
        let clamped = Vec3::new(
            pos.x.clamp(self.center.x - self.half_extents.x, self.center.x + self.half_extents.x),
            pos.y.clamp(self.center.y - self.half_extents.y, self.center.y + self.half_extents.y),
            pos.z.clamp(self.center.z - self.half_extents.z, self.center.z + self.half_extents.z),
        );
        pos.distance_squared(clamped)
    }

    pub fn add_non_proxied_ref(&mut self) -> bool {
        let was_proxied = self.is_proxied();
        self.non_proxy_ref_count += 1;
        was_proxied && !self.is_proxied()
    }

    pub fn remove_non_proxied_ref(&mut self) -> bool {
        let was_proxied = self.is_proxied();
        self.non_proxy_ref_count = (self.non_proxy_ref_count - 1).max(0);
        !was_proxied && self.is_proxied()
    }

    pub fn add_non_hibernate_ref(&mut self) -> bool {
        let was_hib = self.is_hibernated();
        self.non_hibernate_ref_count += 1;
        was_hib && !self.is_hibernated()
    }

    pub fn remove_non_hibernate_ref(&mut self, world_time: f64) -> bool {
        let was_hib = self.is_hibernated();
        self.non_hibernate_ref_count = (self.non_hibernate_ref_count - 1).max(0);
        if !was_hib && self.is_hibernated() {
            self.last_hibernated_at = Some(world_time);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegionStepEvents {
    /// `(region_id, elapsed_hibernated_secs)` when a region wakes up from hibernation.
    pub woke_regions: Vec<(String, f64)>,
    pub hibernated_regions: Vec<String>,
}

/// Port of `RegionRegistry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionRegistry {
    pub curr_region_set_id: RegionSetId,
    pub regions: BTreeMap<String, Region>,
    /// Radius within which regions stay un-hibernated (`nonHibernateRefCount > 0`).
    pub active_radius: f32,
    /// Radius within which regions stay un-proxied (`nonProxyRefCount > 0`).
    pub proxy_radius: f32,
    pub loaded_player_regions: HashSet<String>,
    pub proxied_player_regions: HashSet<String>,
}

impl Default for RegionRegistry {
    fn default() -> Self {
        Self {
            curr_region_set_id: RegionSetId::Home,
            regions: BTreeMap::new(),
            active_radius: 90.0,
            proxy_radius: 180.0,
            loaded_player_regions: HashSet::new(),
            proxied_player_regions: HashSet::new(),
        }
    }
}

impl RegionRegistry {
    pub fn register_region(&mut self, region: Region) {
        self.regions.insert(region.id.clone(), region);
    }

    pub fn set_region_set(&mut self, set_id: RegionSetId, world_time: f64) -> RegionStepEvents {
        if self.curr_region_set_id == set_id {
            return RegionStepEvents::default();
        }
        self.curr_region_set_id = set_id;
        let mut ev = RegionStepEvents::default();
        let active_ids: Vec<String> = self.loaded_player_regions.drain().collect();
        for rid in active_ids {
            if let Some(r) = self.regions.get_mut(&rid) {
                if r.set_id != set_id && r.remove_non_hibernate_ref(world_time) {
                    ev.hibernated_regions.push(rid);
                }
            }
        }
        let proxy_ids: Vec<String> = self.proxied_player_regions.drain().collect();
        for rid in proxy_ids {
            if let Some(r) = self.regions.get_mut(&rid) {
                if r.set_id != set_id {
                    r.remove_non_proxied_ref();
                }
            }
        }
        ev
    }

    /// Updates player loader proximity: wakes nearby regions inside `curr_region_set_id`
    /// and hibernates far ones (`RegionLoader` semantics).
    pub fn update_player_proximity(
        &mut self,
        player_pos: Vec3,
        player_set: RegionSetId,
        world_time: f64,
    ) -> RegionStepEvents {
        let mut ev = self.set_region_set(player_set, world_time);
        let active_r2 = self.active_radius * self.active_radius;
        let proxy_r2 = self.proxy_radius * self.proxy_radius;

        let region_ids: Vec<String> = self.regions.keys().cloned().collect();
        for rid in region_ids {
            let Some(region) = self.regions.get_mut(&rid) else {
                continue;
            };
            let in_set = region.set_id == player_set;
            let d2 = region.distance_sq(player_pos);

            // Non-proxy streaming
            let should_unproxy = in_set && d2 <= proxy_r2;
            let was_unproxied = self.proxied_player_regions.contains(&rid);
            if should_unproxy && !was_unproxied {
                self.proxied_player_regions.insert(rid.clone());
                region.add_non_proxied_ref();
            } else if !should_unproxy && was_unproxied {
                self.proxied_player_regions.remove(&rid);
                region.remove_non_proxied_ref();
            }

            // Active / Hibernate
            let should_wake = in_set && d2 <= active_r2;
            let was_awake = self.loaded_player_regions.contains(&rid);
            if should_wake && !was_awake {
                self.loaded_player_regions.insert(rid.clone());
                if region.add_non_hibernate_ref() {
                    let slept = region
                        .last_hibernated_at
                        .take()
                        .map_or(0.0, |t| (world_time - t).max(0.0));
                    ev.woke_regions.push((rid, slept));
                }
            } else if !should_wake && was_awake {
                self.loaded_player_regions.remove(&rid);
                if region.remove_non_hibernate_ref(world_time) {
                    ev.hibernated_regions.push(rid);
                }
            }
        }

        ev
    }

    /// Assigns an actor ID to the region containing `pos` (for `RegionMember` tracking).
    pub fn assign_actor_member(&mut self, actor_id: i64, set_id: RegionSetId, pos: Vec3) {
        for r in self.regions.values_mut() {
            if r.set_id == set_id && r.contains(pos) {
                r.members.insert(actor_id);
            } else {
                r.members.remove(&actor_id);
            }
        }
    }

    pub fn remove_actor_member(&mut self, actor_id: i64) {
        for r in self.regions.values_mut() {
            r.members.remove(&actor_id);
        }
    }

    pub fn active_regions_in_zone(&self, zone: ZoneId) -> Vec<&Region> {
        self.regions
            .values()
            .filter(|r| r.zone == zone && !r.is_hibernated())
            .collect()
    }
}
