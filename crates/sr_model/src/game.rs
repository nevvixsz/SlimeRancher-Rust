//! Port of `GameModel.cs` — root mutable simulation state container.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{GameMode, IdentifiableId, RegionSetId};
use sr_math::{Quat, Vec3};

use crate::actor::{actor_ids, ActorModel, LiquidSourceModel};
use crate::gadgets::{GadgetSiteModel, GadgetsModel};
use crate::meta::{
    AchievesModel, AppearancesModel, HolidayModel, InstrumentModel, MailModel, PediaModel,
    ProgressModel, TutorialsModel,
};
use crate::objects::{
    AccessDoorModel, DecorizerModel, EchoNoteGordoModel, GlitchSlimulationModel, GordoModel,
    MasterSwitchModel, OasisModel, PuzzleSlotModel, QuicksilverEnergyGeneratorModel,
    TreasurePodModel,
};
use crate::player::PlayerModel;
use crate::ranch::{LandPlotModel, RanchModel};
use crate::world::WorldModel;

/// Port of `MonomiPark.SlimeRancher.DataModel.GameModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameModel {
    pub game_mode: GameMode,
    pub next_actor_id: i64,

    pub actors: HashMap<i64, ActorModel>,
    pub actors_by_ident: HashMap<IdentifiableId, Vec<i64>>,

    pub land_plots: HashMap<String, LandPlotModel>,
    pub doors: HashMap<String, AccessDoorModel>,
    pub gordos: HashMap<String, GordoModel>,
    pub echo_note_gordos: HashMap<String, EchoNoteGordoModel>,
    pub pods: HashMap<String, TreasurePodModel>,
    pub puzzle_slots: HashMap<String, PuzzleSlotModel>,
    pub switches: HashMap<String, MasterSwitchModel>,
    pub oases: HashMap<String, OasisModel>,
    pub liquid_sources: HashMap<String, LiquidSourceModel>,
    pub gadget_sites: HashMap<String, GadgetSiteModel>,
    pub quicksilver_generators: HashMap<String, QuicksilverEnergyGeneratorModel>,
    pub resource_spawners: HashMap<String, f64>,

    pub player: PlayerModel,
    pub ranch: RanchModel,
    pub world: WorldModel,
    pub gadgets: GadgetsModel,
    pub progress: ProgressModel,
    pub mail: MailModel,
    pub pedia: PediaModel,
    pub tutorials: TutorialsModel,
    pub holiday: HolidayModel,
    pub decorizer: DecorizerModel,
    pub appearances: AppearancesModel,
    pub instrument: InstrumentModel,
    pub game_achieves: AchievesModel,
    pub glitch: GlitchSlimulationModel,
}

impl Default for GameModel {
    fn default() -> Self {
        Self::new(GameMode::Classic)
    }
}

impl GameModel {
    pub fn new(game_mode: GameMode) -> Self {
        let settings = GameModeSettings::for_mode(game_mode);
        let mut player = PlayerModel::default();
        player.reset(&settings);

        let mut world = WorldModel::default();
        // World time begins at 09:00 on Day 1 (32400.0 s) matching TimeDirector.START_HOUR.
        world.world_time = 9.0 * 3600.0;

        Self {
            game_mode,
            next_actor_id: actor_ids::BEGIN_DYNAMIC,
            actors: HashMap::new(),
            actors_by_ident: HashMap::new(),
            land_plots: HashMap::new(),
            doors: HashMap::new(),
            gordos: HashMap::new(),
            echo_note_gordos: HashMap::new(),
            pods: HashMap::new(),
            puzzle_slots: HashMap::new(),
            switches: HashMap::new(),
            oases: HashMap::new(),
            liquid_sources: HashMap::new(),
            gadget_sites: HashMap::new(),
            quicksilver_generators: HashMap::new(),
            resource_spawners: HashMap::new(),
            player,
            ranch: RanchModel::default(),
            world,
            gadgets: GadgetsModel::new(),
            progress: ProgressModel::default(),
            mail: MailModel::default(),
            pedia: PediaModel::default(),
            tutorials: TutorialsModel::default(),
            holiday: HolidayModel::default(),
            decorizer: DecorizerModel::default(),
            appearances: AppearancesModel::default(),
            instrument: InstrumentModel::default(),
            game_achieves: AchievesModel::default(),
            glitch: GlitchSlimulationModel::default(),
        }
    }

    /// Allocates the next actor ID and inserts an `ActorModel` matching `GameModel.CreateActorModel`.
    pub fn create_actor(
        &mut self,
        ident: IdentifiableId,
        region_set_id: RegionSetId,
        pos: Vec3,
        rot: Quat,
    ) -> i64 {
        let actor_id = self.next_actor_id;
        self.next_actor_id += 1;
        let model = ActorModel::new(actor_id, ident, region_set_id, pos, rot);
        self.insert_actor_with_id(model);
        actor_id
    }

    /// Inserts an actor with a preassigned ID (used when loading save files).
    pub fn insert_actor_with_id(&mut self, model: ActorModel) {
        let actor_id = model.actor_id;
        let ident = model.ident;
        if actor_id >= self.next_actor_id {
            self.next_actor_id = actor_id + 1;
        }
        self.actors.insert(actor_id, model);
        let list = self.actors_by_ident.entry(ident).or_default();
        if !list.contains(&actor_id) {
            list.push(actor_id);
        }
    }

    /// Removes an actor from both `actors` and `actors_by_ident`.
    pub fn destroy_actor(&mut self, actor_id: i64) -> Option<ActorModel> {
        let removed = self.actors.remove(&actor_id)?;
        if let Some(list) = self.actors_by_ident.get_mut(&removed.ident) {
            list.retain(|&id| id != actor_id);
            if list.is_empty() {
                self.actors_by_ident.remove(&removed.ident);
            }
        }
        Some(removed)
    }

    pub fn actor(&self, actor_id: i64) -> Option<&ActorModel> {
        self.actors.get(&actor_id)
    }

    pub fn actor_mut(&mut self, actor_id: i64) -> Option<&mut ActorModel> {
        self.actors.get_mut(&actor_id)
    }

    pub fn actors_of_type(&self, ident: IdentifiableId) -> impl Iterator<Item = &ActorModel> {
        self.actors_by_ident
            .get(&ident)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.actors.get(id))
    }

    /// Port of `GameModel.IsGameOver()` (used in Rush / TimeLimit modes).
    pub fn is_game_over(&self) -> bool {
        match self.player.end_game_time {
            Some(end_time) => self.world.world_time >= end_time,
            None => false,
        }
    }
}
