use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crownveil_sim::{
    Condition, Effect, EventDef, EventScope, Faction, ModifierDef, ModifierEffects, Pops, Province,
    ProvinceId, World,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorldData {
    pub factions: Vec<FactionData>,
    pub provinces: Vec<ProvinceData>,
}

#[derive(Debug, Deserialize)]
pub struct FactionData {
    pub name: String,
    pub treasury: i64,
    pub legitimacy: i64,
    #[serde(default = "default_tax_rate")]
    pub tax_rate: i64,
    #[serde(default)]
    pub army: i64,
    #[serde(default = "default_court_upkeep")]
    pub court_upkeep: i64,
}

fn default_tax_rate() -> i64 {
    20
}

fn default_court_upkeep() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
pub struct ProvinceData {
    pub name: String,
    pub owner: String,
    pub development: i64,
    pub unrest: i64,
    pub peasant: i64,
    pub burgher: i64,
    pub noble: i64,
    pub clergy: i64,
    #[serde(default)]
    pub map: Option<MapPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct MapPosition {
    pub col: i32,
    pub row: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLayout {
    positions: Vec<MapPosition>,
}

impl MapLayout {
    pub fn position(&self, id: ProvinceId) -> Option<MapPosition> {
        self.positions.get(id).copied()
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn grid(count: usize) -> Self {
        Self {
            positions: (0..count).map(default_map_position).collect(),
        }
    }
}

fn default_map_position(index: usize) -> MapPosition {
    MapPosition {
        col: (index % 4) as i32,
        row: (index / 4) as i32,
    }
}

#[derive(Debug, Deserialize)]
pub struct EventsData {
    #[serde(default)]
    pub modifiers: Vec<ModifierData>,
    #[serde(default)]
    pub events: Vec<EventData>,
}

#[derive(Debug, Deserialize)]
pub struct ModifierData {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub unrest: i64,
    #[serde(default)]
    pub income_pct: i64,
    #[serde(default)]
    pub food_pct: i64,
    #[serde(default)]
    pub legitimacy: i64,
}

#[derive(Debug, Deserialize)]
pub struct EventData {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub scope: EventScope,
    pub chance: i64,
    #[serde(default = "default_cooldown_months")]
    pub cooldown_months: i64,
    #[serde(default)]
    pub conditions: Vec<ConditionData>,
    pub effects: Vec<EffectData>,
}

fn default_cooldown_months() -> i64 {
    120
}

/// Mirrors `sim::Condition`, but references modifiers by key.
#[derive(Debug, Deserialize)]
pub enum ConditionData {
    UnrestAtLeast(i64),
    UnrestBelow(i64),
    DevelopmentAtLeast(i64),
    DevelopmentBelow(i64),
    ClergyAtLeast(i64),
    Famine,
    NoFamine,
    LegitimacyAtLeast(i64),
    LegitimacyBelow(i64),
    TreasuryAtLeast(i64),
    TreasuryBelow(i64),
    TaxAtLeast(i64),
    AtWar,
    AtPeace,
    HasModifier(String),
    LacksModifier(String),
}

/// Mirrors `sim::Effect`, but references modifiers by key.
#[derive(Debug, Deserialize)]
pub enum EffectData {
    Unrest(i64),
    Development(i64),
    Population(i64),
    Legitimacy(i64),
    Treasury(i64),
    Army(i64),
    AddModifier(String, Option<i64>),
    RemoveModifier(String),
}

#[derive(Debug)]
pub enum LoadError {
    Parse(ron::error::SpannedError),
    UnknownFaction { province: String, faction: String },
    DuplicateKey { kind: &'static str, key: String },
    UnknownModifier { event: String, modifier: String },
    ProvinceConditionInFactionEvent { event: String },
    InvalidValue { event: String, reason: &'static str },
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Parse(error) => write!(formatter, "erro ao ler RON: {error}"),
            LoadError::UnknownFaction { province, faction } => write!(
                formatter,
                "província '{province}' referencia facção desconhecida '{faction}'"
            ),
            LoadError::DuplicateKey { kind, key } => {
                write!(formatter, "{kind} com chave duplicada '{key}'")
            }
            LoadError::UnknownModifier { event, modifier } => write!(
                formatter,
                "evento '{event}' referencia modificador desconhecido '{modifier}'"
            ),
            LoadError::ProvinceConditionInFactionEvent { event } => write!(
                formatter,
                "evento '{event}' é de facção mas usa condição exclusiva de província"
            ),
            LoadError::InvalidValue { event, reason } => {
                write!(formatter, "evento '{event}': {reason}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

pub fn load_world(source: &str, seed: u64) -> Result<World, LoadError> {
    load_world_with_map(source, seed).map(|(world, _)| world)
}

pub fn load_world_with_map(source: &str, seed: u64) -> Result<(World, MapLayout), LoadError> {
    let data: WorldData = ron::from_str(source).map_err(LoadError::Parse)?;

    let mut faction_ids = HashMap::new();
    let mut factions = Vec::with_capacity(data.factions.len());
    for (id, faction) in data.factions.iter().enumerate() {
        faction_ids.insert(faction.name.as_str(), id);
        factions.push(
            Faction::new(
                id,
                faction.name.clone(),
                faction.treasury,
                faction.legitimacy,
            )
            .with_tax(faction.tax_rate)
            .with_army(faction.army)
            .with_court_upkeep(faction.court_upkeep),
        );
    }

    let mut provinces = Vec::with_capacity(data.provinces.len());
    let mut positions = Vec::with_capacity(data.provinces.len());
    for (id, province) in data.provinces.iter().enumerate() {
        let owner =
            *faction_ids
                .get(province.owner.as_str())
                .ok_or_else(|| LoadError::UnknownFaction {
                    province: province.name.clone(),
                    faction: province.owner.clone(),
                })?;

        positions.push(province.map.unwrap_or_else(|| default_map_position(id)));
        provinces.push(Province::new(
            id,
            province.name.clone(),
            owner,
            province.development,
            province.unrest,
            Pops::new(
                province.peasant,
                province.burgher,
                province.noble,
                province.clergy,
            ),
        ));
    }

    Ok((
        World::new(seed, factions, provinces),
        MapLayout { positions },
    ))
}

/// Installs modifiers and events from `source` into `world`, validating keys,
/// references, scopes and value ranges.
pub fn load_events(source: &str, world: World) -> Result<World, LoadError> {
    let data: EventsData = ron::from_str(source).map_err(LoadError::Parse)?;

    let mut modifier_ids = BTreeMap::new();
    let mut modifiers = Vec::with_capacity(data.modifiers.len());
    for (id, modifier) in data.modifiers.into_iter().enumerate() {
        if modifier_ids.insert(modifier.key.clone(), id).is_some() {
            return Err(LoadError::DuplicateKey {
                kind: "modificador",
                key: modifier.key,
            });
        }
        modifiers.push(ModifierDef {
            key: modifier.key,
            name: modifier.name,
            effects: ModifierEffects {
                unrest: modifier.unrest,
                income_pct: modifier.income_pct,
                food_pct: modifier.food_pct,
                legitimacy: modifier.legitimacy,
            },
        });
    }

    let mut event_keys = BTreeMap::new();
    let mut events = Vec::with_capacity(data.events.len());
    for event in data.events {
        if event_keys.insert(event.key.clone(), ()).is_some() {
            return Err(LoadError::DuplicateKey {
                kind: "evento",
                key: event.key,
            });
        }
        events.push(convert_event(event, &modifier_ids)?);
    }

    Ok(world.with_events(modifiers, events))
}

fn convert_event(
    event: EventData,
    modifier_ids: &BTreeMap<String, usize>,
) -> Result<EventDef, LoadError> {
    let invalid = |reason| LoadError::InvalidValue {
        event: event.key.clone(),
        reason,
    };
    if !(0..=crownveil_sim::event::CHANCE_SCALE).contains(&event.chance) {
        return Err(invalid("chance deve estar entre 0 e 1000"));
    }
    if event.cooldown_months < 0 {
        return Err(invalid("cooldown_months não pode ser negativo"));
    }
    if event.effects.is_empty() {
        return Err(invalid("evento precisa de ao menos um efeito"));
    }

    let modifier = |key: &String| {
        modifier_ids
            .get(key)
            .copied()
            .ok_or_else(|| LoadError::UnknownModifier {
                event: event.key.clone(),
                modifier: key.clone(),
            })
    };

    let mut conditions = Vec::with_capacity(event.conditions.len());
    for condition in &event.conditions {
        let condition = match condition {
            ConditionData::UnrestAtLeast(v) => Condition::UnrestAtLeast(*v),
            ConditionData::UnrestBelow(v) => Condition::UnrestBelow(*v),
            ConditionData::DevelopmentAtLeast(v) => Condition::DevelopmentAtLeast(*v),
            ConditionData::DevelopmentBelow(v) => Condition::DevelopmentBelow(*v),
            ConditionData::ClergyAtLeast(v) => Condition::ClergyAtLeast(*v),
            ConditionData::Famine => Condition::Famine,
            ConditionData::NoFamine => Condition::NoFamine,
            ConditionData::LegitimacyAtLeast(v) => Condition::LegitimacyAtLeast(*v),
            ConditionData::LegitimacyBelow(v) => Condition::LegitimacyBelow(*v),
            ConditionData::TreasuryAtLeast(v) => Condition::TreasuryAtLeast(*v),
            ConditionData::TreasuryBelow(v) => Condition::TreasuryBelow(*v),
            ConditionData::TaxAtLeast(v) => Condition::TaxAtLeast(*v),
            ConditionData::AtWar => Condition::AtWar,
            ConditionData::AtPeace => Condition::AtPeace,
            ConditionData::HasModifier(key) => Condition::HasModifier(modifier(key)?),
            ConditionData::LacksModifier(key) => Condition::LacksModifier(modifier(key)?),
        };
        if event.scope == EventScope::Faction && condition.is_province_only() {
            return Err(LoadError::ProvinceConditionInFactionEvent {
                event: event.key.clone(),
            });
        }
        conditions.push(condition);
    }

    let mut effects = Vec::with_capacity(event.effects.len());
    for effect in &event.effects {
        effects.push(match effect {
            EffectData::Unrest(v) => Effect::Unrest(*v),
            EffectData::Development(v) => Effect::Development(*v),
            EffectData::Population(v) => {
                if *v < -100 {
                    return Err(invalid("Population não pode passar de -100%"));
                }
                Effect::Population(*v)
            }
            EffectData::Legitimacy(v) => Effect::Legitimacy(*v),
            EffectData::Treasury(v) => Effect::Treasury(*v),
            EffectData::Army(v) => Effect::Army(*v),
            EffectData::AddModifier(key, months) => {
                if months.is_some_and(|months| months <= 0) {
                    return Err(invalid("duração de modificador deve ser positiva"));
                }
                Effect::AddModifier {
                    modifier: modifier(key)?,
                    months: *months,
                }
            }
            EffectData::RemoveModifier(key) => Effect::RemoveModifier(modifier(key)?),
        });
    }

    Ok(EventDef {
        key: event.key.clone(),
        title: event.title.clone(),
        description: event.description.clone(),
        scope: event.scope,
        chance: event.chance,
        cooldown_months: event.cooldown_months,
        conditions,
        effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: &str = include_str!("../../../content/world.ron");
    const EVENTS: &str = include_str!("../../../content/events.ron");

    fn events_world(events: &str) -> Result<World, LoadError> {
        load_events(events, load_world(WORLD, 1).unwrap())
    }

    #[test]
    fn bundled_events_load() {
        let world = events_world(EVENTS).expect("os eventos embutidos devem carregar");
        assert!(!world.modifiers.is_empty());
        assert!(!world.events.is_empty());
    }

    #[test]
    fn bundled_events_fire_deterministically_over_a_century() {
        let run = || {
            let mut world = events_world(EVENTS).unwrap();
            for _ in 0..1_200 {
                world.tick_month();
            }
            world
        };
        let world = run();
        assert!(
            !world.event_log.is_empty(),
            "algum evento deve ocorrer em 100 anos"
        );
        assert_eq!(world, run());
    }

    #[test]
    fn unknown_modifier_is_rejected() {
        let source = r#"(
            events: [(key: "e", title: "E", scope: Province, chance: 1,
                      effects: [AddModifier("nada", Some(12))])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::UnknownModifier { .. })
        ));
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let source = r#"(
            modifiers: [(key: "m", name: "M"), (key: "m", name: "N")],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::DuplicateKey { .. })
        ));
    }

    #[test]
    fn province_condition_in_faction_event_is_rejected() {
        let source = r#"(
            events: [(key: "e", title: "E", scope: Faction, chance: 1,
                      conditions: [Famine], effects: [Treasury(1)])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::ProvinceConditionInFactionEvent { .. })
        ));
    }

    #[test]
    fn out_of_range_chance_is_rejected() {
        let source = r#"(
            events: [(key: "e", title: "E", scope: Province, chance: 1001,
                      effects: [Treasury(1)])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::InvalidValue { .. })
        ));
    }

    #[test]
    fn bundled_world_loads() {
        let world = load_world(WORLD, 1).expect("o mundo embutido deve carregar");
        assert_eq!(world.factions.len(), 3);
        assert_eq!(world.provinces.len(), 9);
        assert!(world
            .provinces
            .iter()
            .all(|p| p.owner < world.factions.len()));
    }

    #[test]
    fn map_layout_matches_provinces() {
        let (world, layout) =
            load_world_with_map(WORLD, 1).expect("o mundo embutido deve carregar");
        assert_eq!(layout.len(), world.provinces.len());
        assert_eq!(layout.position(0), Some(MapPosition { col: 0, row: 0 }));
        assert_eq!(layout.position(8), Some(MapPosition { col: 3, row: 2 }));
    }

    #[test]
    fn missing_map_falls_back_to_grid() {
        let source = r#"(
            factions: [(name: "A", treasury: 0, legitimacy: 50)],
            provinces: [
                (name: "P", owner: "A", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0),
                (name: "Q", owner: "A", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0),
            ],
        )"#;
        let (_, layout) = load_world_with_map(source, 1).unwrap();
        assert_eq!(layout.position(0), Some(MapPosition { col: 0, row: 0 }));
        assert_eq!(layout.position(1), Some(MapPosition { col: 1, row: 0 }));
    }

    #[test]
    fn unknown_owner_is_rejected() {
        let source = r#"(
            factions: [(name: "A", treasury: 0, legitimacy: 50)],
            provinces: [(name: "P", owner: "B", development: 50, unrest: 0, peasant: 1, burgher: 0, noble: 0, clergy: 0)],
        )"#;
        assert!(matches!(
            load_world(source, 1),
            Err(LoadError::UnknownFaction { .. })
        ));
    }
}
