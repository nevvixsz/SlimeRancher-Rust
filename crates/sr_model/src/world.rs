//! Port of `WorldModel.cs` and `ExchangeDirector.Offer`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sr_ids::IdentifiableId;

use crate::enums::{FirestormMode, NonIdentReward, OfferType, Weather};

/// Port of `ExchangeDirector.ItemEntry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeItemEntry {
    pub id: IdentifiableId,
    pub count: i32,
    pub spec_reward: NonIdentReward,
}

/// Port of `ExchangeDirector.RequestedItemEntry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeRequestedItemEntry {
    pub id: IdentifiableId,
    pub count: i32,
    pub progress: i32,
    pub spec_reward: NonIdentReward,
}

impl ExchangeRequestedItemEntry {
    pub fn is_complete(&self) -> bool {
        self.progress >= self.count
    }
}

/// Port of `ExchangeDirector.Offer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeOffer {
    pub offer_id: String,
    pub rancher_id: String,
    pub expire_time: f64,
    pub early_exchange_time: f64,
    pub requests: Vec<ExchangeRequestedItemEntry>,
    pub rewards: Vec<ExchangeItemEntry>,
}

impl ExchangeOffer {
    pub fn is_complete(&self) -> bool {
        self.requests.iter().all(|r| r.is_complete())
    }
}

/// Port of `WorldModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldModel {
    pub econ_seed: f32,
    pub market_saturation: HashMap<IdentifiableId, f32>,
    pub world_time: f64,
    pub fast_forward_until: Option<f64>,
    pub pause_world_time: bool,
    pub last_world_time: Option<f64>,

    pub curr_offers: HashMap<OfferType, ExchangeOffer>,
    pub next_daily_offer_create_time: f64,
    pub last_offer_rancher_ids: Vec<String>,
    pub pending_offer_rancher_ids: Vec<String>,

    pub curr_weather: Weather,
    pub weather_until: f64,

    pub curr_firestorm_mode: FirestormMode,
    pub curr_firestorm_ambiance_mode: FirestormMode,
    pub next_firestorm_time: f64,
    pub end_firestorm_time: f64,
    pub end_firecolumns_time: f64,
    pub next_firecolumn_time: f64,

    pub current_ginger_patch_ids: Vec<String>,
    pub occupied_phase_sites: HashSet<String>,
}

impl Default for WorldModel {
    fn default() -> Self {
        Self {
            econ_seed: 0.0,
            market_saturation: HashMap::new(),
            world_time: 0.0,
            fast_forward_until: None,
            pause_world_time: false,
            last_world_time: None,
            curr_offers: HashMap::new(),
            next_daily_offer_create_time: f64::INFINITY,
            last_offer_rancher_ids: Vec::new(),
            pending_offer_rancher_ids: Vec::new(),
            curr_weather: Weather::None,
            weather_until: 0.0,
            curr_firestorm_mode: FirestormMode::Idle,
            curr_firestorm_ambiance_mode: FirestormMode::Idle,
            next_firestorm_time: 0.0,
            end_firestorm_time: 0.0,
            end_firecolumns_time: 0.0,
            next_firecolumn_time: f64::INFINITY,
            current_ginger_patch_ids: Vec::new(),
            occupied_phase_sites: HashSet::new(),
        }
    }
}

impl WorldModel {
    #[inline]
    pub fn has_reached_time(&self, time: f64) -> bool {
        self.world_time >= time
    }
}
