use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::diplomacy::Diplomacy;
use crate::event::{
    EventCooldown, EventDef, EventId, EventRecord, EventTarget, PendingDecision, ScheduledEvent,
};
use crate::modifier::{ActiveModifier, ModifierDef, ModifierId};
use crate::rite::{RiteCooldown, RiteDef, RiteId};
use crate::rng::Rng;
use crate::rules::{self, UnrestExplanation};

pub type ProvinceId = usize;
pub type FactionId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PopClass {
    Peasant,
    Burgher,
    Noble,
    Clergy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pops {
    pub peasant: i64,
    pub burgher: i64,
    pub noble: i64,
    pub clergy: i64,
}

impl Pops {
    pub fn new(peasant: i64, burgher: i64, noble: i64, clergy: i64) -> Self {
        Self {
            peasant,
            burgher,
            noble,
            clergy,
        }
    }

    pub fn total(&self) -> i64 {
        self.peasant + self.burgher + self.noble + self.clergy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Province {
    pub id: ProvinceId,
    pub name: String,
    pub owner: FactionId,
    pub development: i64,
    pub unrest: i64,
    pub pops: Pops,
    #[serde(default)]
    pub modifiers: Vec<ActiveModifier>,
}

impl Province {
    pub fn new(
        id: ProvinceId,
        name: String,
        owner: FactionId,
        development: i64,
        unrest: i64,
        pops: Pops,
    ) -> Self {
        Self {
            id,
            name,
            owner,
            development,
            unrest,
            pops,
            modifiers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faction {
    pub id: FactionId,
    pub name: String,
    pub treasury: i64,
    pub legitimacy: i64,
    pub tax_rate: i64,
    pub army: i64,
    pub court_upkeep: i64,
    #[serde(default)]
    pub faith: i64,
    #[serde(default)]
    pub mana: i64,
    #[serde(default)]
    pub modifiers: Vec<ActiveModifier>,
}

impl Faction {
    pub fn new(id: FactionId, name: String, treasury: i64, legitimacy: i64) -> Self {
        Self {
            id,
            name,
            treasury,
            legitimacy,
            tax_rate: 20,
            army: 0,
            court_upkeep: 20,
            faith: 0,
            mana: 0,
            modifiers: Vec::new(),
        }
    }

    pub fn with_tax(mut self, tax_rate: i64) -> Self {
        self.tax_rate = tax_rate.clamp(0, 100);
        self
    }

    pub fn with_army(mut self, army: i64) -> Self {
        self.army = army.max(0);
        self
    }

    pub fn with_resources(mut self, faith: i64, mana: i64) -> Self {
        self.faith = faith.clamp(0, crate::rules::MAX_RESOURCE);
        self.mana = mana.clamp(0, crate::rules::MAX_RESOURCE);
        self
    }

    pub fn with_court_upkeep(mut self, court_upkeep: i64) -> Self {
        self.court_upkeep = court_upkeep.max(0);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub year: i64,
    pub month: u32,
    pub provinces: Vec<Province>,
    pub factions: Vec<Faction>,
    pub rng: Rng,
    #[serde(default)]
    pub pending_commands: Vec<Command>,
    pub diplomacy: Diplomacy,
    #[serde(default)]
    pub modifiers: Vec<ModifierDef>,
    #[serde(default)]
    pub events: Vec<EventDef>,
    #[serde(default)]
    pub event_log: Vec<EventRecord>,
    #[serde(default)]
    pub event_cooldowns: Vec<EventCooldown>,
    /// Faction whose event choices wait for input; everyone else is AI.
    #[serde(default)]
    pub player: Option<FactionId>,
    #[serde(default)]
    pub pending_decisions: Vec<PendingDecision>,
    #[serde(default)]
    pub next_decision_id: u64,
    #[serde(default)]
    pub scheduled_events: Vec<ScheduledEvent>,
    #[serde(default)]
    pub rites: Vec<RiteDef>,
    #[serde(default)]
    pub rite_cooldowns: Vec<RiteCooldown>,
}

impl World {
    pub fn new(seed: u64, factions: Vec<Faction>, provinces: Vec<Province>) -> Self {
        let diplomacy = Diplomacy::new(factions.len());
        Self {
            seed,
            year: 1,
            month: 1,
            provinces,
            factions,
            rng: Rng::new(seed),
            pending_commands: Vec::new(),
            diplomacy,
            modifiers: Vec::new(),
            events: Vec::new(),
            event_log: Vec::new(),
            event_cooldowns: Vec::new(),
            player: None,
            pending_decisions: Vec::new(),
            next_decision_id: 0,
            scheduled_events: Vec::new(),
            rites: Vec::new(),
            rite_cooldowns: Vec::new(),
        }
    }

    pub fn with_events(mut self, modifiers: Vec<ModifierDef>, events: Vec<EventDef>) -> Self {
        self.modifiers = modifiers;
        self.events = events;
        self
    }

    pub fn with_rites(mut self, rites: Vec<RiteDef>) -> Self {
        self.rites = rites;
        self
    }

    pub fn rite_def(&self, id: RiteId) -> Option<&RiteDef> {
        self.rites.get(id)
    }

    pub fn with_player(mut self, player: Option<FactionId>) -> Self {
        self.player = player.filter(|&id| id < self.factions.len());
        self
    }

    /// Follow-ups that will hit `faction` or one of its provinces.
    pub fn looming_for(&self, faction: FactionId) -> impl Iterator<Item = &ScheduledEvent> {
        self.scheduled_events
            .iter()
            .filter(move |scheduled| match scheduled.target {
                EventTarget::Province(id) => {
                    self.provinces.get(id).map(|p| p.owner) == Some(faction)
                }
                EventTarget::Faction(id) => id == faction,
            })
    }

    pub fn decisions_for(&self, faction: FactionId) -> impl Iterator<Item = &PendingDecision> {
        self.pending_decisions
            .iter()
            .filter(move |pending| pending.faction == faction)
    }

    pub fn modifier_def(&self, id: ModifierId) -> Option<&ModifierDef> {
        self.modifiers.get(id)
    }

    pub fn event_def(&self, id: EventId) -> Option<&EventDef> {
        self.events.get(id)
    }

    pub fn choice_label(&self, record: &EventRecord) -> Option<&str> {
        let choice = record.choice?;
        let option = self.event_def(record.event)?.options.get(choice)?;
        Some(option.label.as_str())
    }

    pub fn target_name(&self, target: EventTarget) -> &str {
        let name = match target {
            EventTarget::Province(id) => self.provinces.get(id).map(|p| p.name.as_str()),
            EventTarget::Faction(id) => self.factions.get(id).map(|f| f.name.as_str()),
        };
        name.unwrap_or("?")
    }

    pub fn tick_month(&mut self) {
        crate::tick::tick_month(self);
    }

    pub fn queue_command(&mut self, command: Command) {
        self.pending_commands.push(command);
    }

    pub fn faction(&self, id: FactionId) -> Option<&Faction> {
        self.factions.get(id)
    }

    pub fn explain_unrest(&self, province_id: ProvinceId) -> Option<UnrestExplanation> {
        let province = self.provinces.get(province_id)?;
        let faction = self.factions.get(province.owner)?;
        let at_war = self.diplomacy.war_count_for(province.owner) > 0;
        let breakdown = rules::unrest_breakdown(province, faction, at_war);
        Some(UnrestExplanation {
            current: province.unrest,
            target: breakdown.target(),
            breakdown,
        })
    }

    pub fn province_count(&self, faction: FactionId) -> usize {
        self.provinces
            .iter()
            .filter(|province| province.owner == faction)
            .count()
    }

    pub fn is_at_war(&self, faction: FactionId) -> bool {
        self.diplomacy.war_count_for(faction) > 0
    }

    pub fn total_population(&self) -> i64 {
        self.provinces
            .iter()
            .map(|province| province.pops.total())
            .sum()
    }

    pub fn average_unrest(&self) -> i64 {
        if self.provinces.is_empty() {
            return 0;
        }
        let total: i64 = self.provinces.iter().map(|province| province.unrest).sum();
        total / self.provinces.len() as i64
    }

    pub fn demo(seed: u64) -> Self {
        let factions = vec![
            Faction::new(0, "Reino de Aeldrath".to_string(), 1000, 60)
                .with_tax(25)
                .with_court_upkeep(20),
            Faction::new(1, "Corte de Vhalen".to_string(), 800, 45)
                .with_tax(35)
                .with_army(300)
                .with_court_upkeep(15),
            Faction::new(2, "Sacro Dominio de Orem".to_string(), 1200, 70)
                .with_tax(15)
                .with_court_upkeep(25),
        ];

        let provinces = vec![
            Province::new(
                0,
                "Pedra Alta".to_string(),
                0,
                55,
                20,
                Pops::new(12_000, 2_500, 300, 400),
            ),
            Province::new(
                1,
                "Vale Sereno".to_string(),
                0,
                45,
                15,
                Pops::new(9_000, 1_200, 150, 250),
            ),
            Province::new(
                2,
                "Porto Cinéreo".to_string(),
                0,
                60,
                30,
                Pops::new(14_000, 4_000, 400, 350),
            ),
            Province::new(
                3,
                "Muralha de Vidro".to_string(),
                1,
                50,
                40,
                Pops::new(10_000, 1_800, 250, 200),
            ),
            Province::new(
                4,
                "Charneca Rubra".to_string(),
                1,
                35,
                55,
                Pops::new(7_000, 700, 120, 150),
            ),
            Province::new(
                5,
                "Bosque das Véus".to_string(),
                1,
                40,
                25,
                Pops::new(8_500, 900, 180, 600),
            ),
            Province::new(
                6,
                "Cidade Sagrada".to_string(),
                2,
                65,
                10,
                Pops::new(11_000, 3_200, 500, 1_500),
            ),
            Province::new(
                7,
                "Planície do Sol".to_string(),
                2,
                48,
                20,
                Pops::new(13_000, 1_500, 200, 450),
            ),
            Province::new(
                8,
                "Foz de Prata".to_string(),
                2,
                58,
                15,
                Pops::new(10_500, 2_800, 350, 500),
            ),
        ];

        World::new(seed, factions, provinces)
    }
}
