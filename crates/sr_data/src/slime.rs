//! Slime definitions, diets, largo synthesis, and eat-map generation
//! ported from `SlimeDefinition.cs`, `SlimeDiet.cs`, and `SlimeDefinitions.cs`.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sr_ids::{FoodGroup, IdentifiableId, SlimeEmotion};

/// Port of `SlimeDiet.EatMapEntry`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EatMapEntry {
    pub eats: IdentifiableId,
    pub is_favorite: bool,
    pub favorite_production_count: u32,
    pub produces_id: IdentifiableId,
    pub becomes_id: IdentifiableId,
    pub driver: SlimeEmotion,
    pub extra_drive: f32,
    pub min_drive: f32,
}

impl EatMapEntry {
    /// Port of `EatMapEntry.NumToProduce()`.
    #[inline]
    pub fn num_to_produce(&self) -> u32 {
        if self.is_favorite {
            self.favorite_production_count
        } else {
            1
        }
    }
}

/// Port of `SlimeDiet`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeDiet {
    #[serde(default)]
    pub major_food_groups: Vec<FoodGroup>,
    #[serde(default)]
    pub favorites: Vec<IdentifiableId>,
    #[serde(default)]
    pub additional_foods: Vec<IdentifiableId>,
    #[serde(default)]
    pub produces: Vec<IdentifiableId>,
    #[serde(default = "default_favorite_count")]
    pub favorite_production_count: u32,
    #[serde(default)]
    pub eat_map: Vec<EatMapEntry>,
}

fn default_favorite_count() -> u32 {
    2
}

impl Default for SlimeDiet {
    fn default() -> Self {
        Self {
            major_food_groups: Vec::new(),
            favorites: Vec::new(),
            additional_foods: Vec::new(),
            produces: Vec::new(),
            favorite_production_count: 2,
            eat_map: Vec::new(),
        }
    }
}

impl SlimeDiet {
    /// Port of `SlimeDiet.Combine(diet1, diet2)`.
    pub fn combine(diet1: &Self, diet2: &Self) -> Self {
        let mut major_food_groups = diet1.major_food_groups.clone();
        for &g in &diet2.major_food_groups {
            if !major_food_groups.contains(&g) {
                major_food_groups.push(g);
            }
        }
        let mut favorites = diet1.favorites.clone();
        for &f in &diet2.favorites {
            if !favorites.contains(&f) {
                favorites.push(f);
            }
        }
        let mut additional_foods = diet1.additional_foods.clone();
        for &a in &diet2.additional_foods {
            if !additional_foods.contains(&a) {
                additional_foods.push(a);
            }
        }
        let mut produces = diet1.produces.clone();
        for &p in &diet2.produces {
            if !produces.contains(&p) {
                produces.push(p);
            }
        }
        Self {
            major_food_groups,
            favorites,
            additional_foods,
            produces,
            favorite_production_count: diet1.favorite_production_count,
            eat_map: Vec::new(),
        }
    }

    /// Port of `SlimeDiet.GetDietIdentifiableIds()`.
    pub fn diet_identifiable_ids(&self) -> Vec<IdentifiableId> {
        let mut set = BTreeSet::new();
        for &group in &self.major_food_groups {
            for id in IdentifiableId::ids_in_food_group(group) {
                set.insert(id);
            }
        }
        for &id in &self.additional_foods {
            set.insert(id);
        }
        set.into_iter().collect()
    }

    fn produces_plorts(&self) -> bool {
        self.produces.iter().any(|id| id.is_plort())
    }

    /// Port of `SlimeDiet.RefreshEatMap(SlimeDefinitions, SlimeDefinition)`.
    pub fn refresh_eat_map(
        &mut self,
        is_largo: bool,
        can_largofy: bool,
        largo_lookup: &HashMap<(IdentifiableId, IdentifiableId), IdentifiableId>,
    ) {
        self.eat_map.clear();

        for food_id in self.diet_identifiable_ids() {
            let driver = if food_id != IdentifiableId::SpicyTofu {
                SlimeEmotion::Hunger
            } else {
                SlimeEmotion::None
            };
            for &produces_id in &self.produces {
                self.eat_map.push(EatMapEntry {
                    eats: food_id,
                    produces_id,
                    is_favorite: self.favorites.contains(&food_id),
                    favorite_production_count: self.favorite_production_count,
                    driver,
                    extra_drive: 0.0,
                    min_drive: 0.0,
                    becomes_id: IdentifiableId::None,
                });
            }
        }

        if self.produces_plorts() && (is_largo || can_largofy) {
            for plort_id in IdentifiableId::ids_in_food_group(FoodGroup::Plorts) {
                if plort_id == IdentifiableId::QuicksilverPlort || self.produces.contains(&plort_id)
                {
                    continue;
                }
                let becomes_id = if is_largo {
                    IdentifiableId::TarrSlime
                } else {
                    let key = sorted_pair(plort_id, self.produces[0]);
                    match largo_lookup.get(&key) {
                        Some(&largo_id) => largo_id,
                        None => continue,
                    }
                };
                self.eat_map.push(EatMapEntry {
                    eats: plort_id,
                    produces_id: IdentifiableId::None,
                    is_favorite: false,
                    favorite_production_count: self.favorite_production_count,
                    driver: SlimeEmotion::Agitation,
                    extra_drive: 0.0,
                    min_drive: 0.5,
                    becomes_id,
                });
            }
        }
    }

    pub fn add_eat_map_entries(&self, id: IdentifiableId, target: &mut Vec<EatMapEntry>) {
        for entry in &self.eat_map {
            if entry.eats == id {
                target.push(entry.clone());
            }
        }
    }
}

/// Port of `SlimeDefinition`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeDefinition {
    pub name: String,
    pub identifiable_id: IdentifiableId,
    #[serde(default)]
    pub base_slimes: Vec<IdentifiableId>,
    pub is_largo: bool,
    pub can_largofy: bool,
    #[serde(default = "default_scale")]
    pub prefab_scale: f32,
    pub diet: SlimeDiet,
    #[serde(default)]
    pub favorite_toys: Vec<IdentifiableId>,
}

fn default_scale() -> f32 {
    1.0
}

/// Port of `SlimeDefinitions`: holds all slime definitions and precomputes largo/eat-map indexes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlimeDefinitions {
    pub slimes: HashMap<IdentifiableId, SlimeDefinition>,
    #[serde(skip)]
    largo_by_base_plorts: HashMap<(IdentifiableId, IdentifiableId), IdentifiableId>,
    #[serde(skip)]
    largo_by_base_slimes: HashMap<(IdentifiableId, IdentifiableId), IdentifiableId>,
}

impl SlimeDefinitions {
    pub fn from_vec(defs: Vec<SlimeDefinition>) -> Self {
        let mut slimes = HashMap::with_capacity(defs.len());
        for def in defs {
            slimes.insert(def.identifiable_id, def);
        }
        let mut instance = Self {
            slimes,
            largo_by_base_plorts: HashMap::new(),
            largo_by_base_slimes: HashMap::new(),
        };
        instance.refresh_all();
        instance
    }

    pub fn refresh_all(&mut self) {
        self.largo_by_base_plorts.clear();
        self.largo_by_base_slimes.clear();

        // First synthesize largo diets/toys from base slimes when empty.
        let ids: Vec<IdentifiableId> = self.slimes.keys().copied().collect();
        for id in &ids {
            let (is_largo, bases) = {
                let def = &self.slimes[id];
                (def.is_largo, def.base_slimes.clone())
            };
            if is_largo && bases.len() == 2 {
                if let (Some(b0), Some(b1)) =
                    (self.slimes.get(&bases[0]).cloned(), self.slimes.get(&bases[1]).cloned())
                {
                    if let Some(def) = self.slimes.get_mut(id) {
                        if def.diet.produces.is_empty() {
                            def.diet = SlimeDiet::combine(&b0.diet, &b1.diet);
                        }
                        if def.favorite_toys.is_empty() {
                            let mut toys = b0.favorite_toys.clone();
                            for &t in &b1.favorite_toys {
                                if !toys.contains(&t) {
                                    toys.push(t);
                                }
                            }
                            def.favorite_toys = toys;
                        }
                    }
                    if let (Some(&p0), Some(&p1)) = (b0.diet.produces.first(), b1.diet.produces.first()) {
                        self.largo_by_base_plorts.insert(sorted_pair(p0, p1), *id);
                    }
                    self.largo_by_base_slimes
                        .insert(sorted_pair(bases[0], bases[1]), *id);
                }
            }
        }

        let largo_lookup = self.largo_by_base_plorts.clone();
        for def in self.slimes.values_mut() {
            def.diet
                .refresh_eat_map(def.is_largo, def.can_largofy, &largo_lookup);
        }
    }

    pub fn get(&self, id: IdentifiableId) -> Option<&SlimeDefinition> {
        self.slimes.get(&id)
    }

    pub fn get_largo_by_plorts(
        &self,
        plort1: IdentifiableId,
        plort2: IdentifiableId,
    ) -> Option<&SlimeDefinition> {
        self.largo_by_base_plorts
            .get(&sorted_pair(plort1, plort2))
            .and_then(|id| self.slimes.get(id))
    }

    pub fn get_largo_by_base_slimes(
        &self,
        slime1: IdentifiableId,
        slime2: IdentifiableId,
    ) -> Option<&SlimeDefinition> {
        self.largo_by_base_slimes
            .get(&sorted_pair(slime1, slime2))
            .and_then(|id| self.slimes.get(id))
    }
}

#[inline]
fn sorted_pair(a: IdentifiableId, b: IdentifiableId) -> (IdentifiableId, IdentifiableId) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
