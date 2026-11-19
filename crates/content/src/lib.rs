pub mod save;

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crownveil_sim::{
    Condition, Effect, EventDef, EventOption, EventScope, Faction, ModifierDef, ModifierEffects,
    Pops, Province, ProvinceId, RiteBlock, RiteDef, World,
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
    #[serde(default)]
    pub faith: i64,
    #[serde(default)]
    pub mana: i64,
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
    #[serde(default)]
    pub faith: i64,
    #[serde(default)]
    pub mana: i64,
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
    #[serde(default)]
    pub effects: Vec<EffectData>,
    #[serde(default)]
    pub options: Vec<OptionData>,
}

#[derive(Debug, Deserialize)]
pub struct RitesData {
    pub rites: Vec<RiteData>,
}

#[derive(Debug, Deserialize)]
pub struct RiteData {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub scope: EventScope,
    #[serde(default)]
    pub faith_cost: i64,
    #[serde(default)]
    pub mana_cost: i64,
    #[serde(default)]
    pub gold_cost: i64,
    #[serde(default)]
    pub cooldown_months: i64,
    #[serde(default)]
    pub conditions: Vec<ConditionData>,
    pub effects: Vec<EffectData>,
}

#[derive(Debug, Deserialize)]
pub struct OptionData {
    pub label: String,
    #[serde(default)]
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
    FaithAtLeast(i64),
    FaithBelow(i64),
    ManaAtLeast(i64),
    ManaBelow(i64),
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
    Faith(i64),
    Mana(i64),
    AddModifier(String, Option<i64>),
    RemoveModifier(String),
    /// `TriggerEvent("chave", meses)`: schedules a follow-up event.
    TriggerEvent(String, i64),
}

#[derive(Debug)]
pub enum LoadError {
    Parse(ron::error::SpannedError),
    UnknownFaction { province: String, faction: String },
    DuplicateKey { kind: &'static str, key: String },
    UnknownModifier { event: String, modifier: String },
    UnknownEvent { event: String, target: String },
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
                "'{event}' referencia modificador desconhecido '{modifier}'"
            ),
            LoadError::UnknownEvent { event, target } => write!(
                formatter,
                "'{event}' encadeia evento desconhecido '{target}'"
            ),
            LoadError::ProvinceConditionInFactionEvent { event } => write!(
                formatter,
                "'{event}' é de facção mas usa condição exclusiva de província"
            ),
            LoadError::InvalidValue { event, reason } => {
                write!(formatter, "'{event}': {reason}")
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
            .with_court_upkeep(faction.court_upkeep)
            .with_resources(faction.faith, faction.mana),
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
                faith: modifier.faith,
                mana: modifier.mana,
            },
        });
    }

    // Keys first, so an event can chain into one defined further down.
    let mut event_ids = EventIds::new();
    for (id, event) in data.events.iter().enumerate() {
        if event_ids
            .insert(event.key.clone(), (id, event.scope))
            .is_some()
        {
            return Err(LoadError::DuplicateKey {
                kind: "evento",
                key: event.key.clone(),
            });
        }
    }

    let mut events = Vec::with_capacity(data.events.len());
    for event in data.events {
        events.push(convert_event(event, &modifier_ids, &event_ids)?);
    }

    Ok(world.with_events(modifiers, events))
}

type EventIds = BTreeMap<String, (usize, EventScope)>;

fn convert_event(
    event: EventData,
    modifier_ids: &BTreeMap<String, usize>,
    event_ids: &EventIds,
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
    if event.effects.is_empty() && event.options.is_empty() {
        return Err(invalid("evento precisa de efeitos ou opções"));
    }
    if event.options.len() == 1 {
        return Err(invalid(
            "evento com escolha precisa de ao menos duas opções",
        ));
    }
    if event
        .options
        .iter()
        .any(|option| option.label.trim().is_empty())
    {
        return Err(invalid("toda opção precisa de um texto (label)"));
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

    let conditions = convert_conditions(&event.conditions, event.scope, &event.key, &modifier)?;
    let effects = convert_effects(
        &event.effects,
        &event.key,
        event.scope,
        &modifier,
        event_ids,
    )?;
    let mut options = Vec::with_capacity(event.options.len());
    for option in &event.options {
        options.push(EventOption {
            label: option.label.clone(),
            effects: convert_effects(
                &option.effects,
                &event.key,
                event.scope,
                &modifier,
                event_ids,
            )?,
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
        options,
    })
}

fn convert_conditions(
    conditions: &[ConditionData],
    scope: EventScope,
    key: &str,
    modifier: &impl Fn(&String) -> Result<usize, LoadError>,
) -> Result<Vec<Condition>, LoadError> {
    let mut converted = Vec::with_capacity(conditions.len());
    for condition in conditions {
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
            ConditionData::FaithAtLeast(v) => Condition::FaithAtLeast(*v),
            ConditionData::FaithBelow(v) => Condition::FaithBelow(*v),
            ConditionData::ManaAtLeast(v) => Condition::ManaAtLeast(*v),
            ConditionData::ManaBelow(v) => Condition::ManaBelow(*v),
            ConditionData::AtWar => Condition::AtWar,
            ConditionData::AtPeace => Condition::AtPeace,
            ConditionData::HasModifier(key) => Condition::HasModifier(modifier(key)?),
            ConditionData::LacksModifier(key) => Condition::LacksModifier(modifier(key)?),
        };
        if scope == EventScope::Faction && condition.is_province_only() {
            return Err(LoadError::ProvinceConditionInFactionEvent {
                event: key.to_string(),
            });
        }
        converted.push(condition);
    }
    Ok(converted)
}

/// Player-facing text for why a rite is unavailable. Lives here, next to the
/// other Portuguese content, so the REPL and the client say the same thing.
pub fn rite_block_text(block: RiteBlock) -> String {
    match block {
        RiteBlock::UnknownRite => "rito desconhecido".to_string(),
        RiteBlock::WrongScope => "alvo errado para este rito".to_string(),
        RiteBlock::NotOwner => "a província precisa ser sua".to_string(),
        RiteBlock::NotEnoughFaith => "fé insuficiente".to_string(),
        RiteBlock::NotEnoughMana => "magia insuficiente".to_string(),
        RiteBlock::NotEnoughGold => "ouro insuficiente".to_string(),
        RiteBlock::OnCooldown { months_left } => format!("disponível em {months_left} meses"),
        RiteBlock::ConditionsUnmet => "condições não atendidas".to_string(),
    }
}

/// Installs rites from `source` into `world`. Must run after `load_events`,
/// because rites reference modifiers by key.
pub fn load_rites(source: &str, world: World) -> Result<World, LoadError> {
    let data: RitesData = ron::from_str(source).map_err(LoadError::Parse)?;
    let modifier_ids: BTreeMap<String, usize> = world
        .modifiers
        .iter()
        .enumerate()
        .map(|(id, def)| (def.key.clone(), id))
        .collect();
    let event_ids: EventIds = world
        .events
        .iter()
        .enumerate()
        .map(|(id, def)| (def.key.clone(), (id, def.scope)))
        .collect();

    let mut keys = BTreeMap::new();
    let mut rites = Vec::with_capacity(data.rites.len());
    for rite in data.rites {
        if keys.insert(rite.key.clone(), ()).is_some() {
            return Err(LoadError::DuplicateKey {
                kind: "rito",
                key: rite.key,
            });
        }
        let invalid = |reason| LoadError::InvalidValue {
            event: rite.key.clone(),
            reason,
        };
        if rite.faith_cost < 0 || rite.mana_cost < 0 || rite.gold_cost < 0 {
            return Err(invalid("custos não podem ser negativos"));
        }
        if rite.cooldown_months < 0 {
            return Err(invalid("cooldown_months não pode ser negativo"));
        }
        if rite.effects.is_empty() {
            return Err(invalid("rito precisa de ao menos um efeito"));
        }

        let modifier = |key: &String| {
            modifier_ids
                .get(key)
                .copied()
                .ok_or_else(|| LoadError::UnknownModifier {
                    event: rite.key.clone(),
                    modifier: key.clone(),
                })
        };
        rites.push(RiteDef {
            conditions: convert_conditions(&rite.conditions, rite.scope, &rite.key, &modifier)?,
            effects: convert_effects(&rite.effects, &rite.key, rite.scope, &modifier, &event_ids)?,
            key: rite.key.clone(),
            name: rite.name.clone(),
            description: rite.description.clone(),
            scope: rite.scope,
            faith_cost: rite.faith_cost,
            mana_cost: rite.mana_cost,
            gold_cost: rite.gold_cost,
            cooldown_months: rite.cooldown_months,
        });
    }

    Ok(world.with_rites(rites))
}

fn convert_effects(
    effects: &[EffectData],
    event: &str,
    scope: EventScope,
    modifier: &impl Fn(&String) -> Result<usize, LoadError>,
    event_ids: &EventIds,
) -> Result<Vec<Effect>, LoadError> {
    let invalid = |reason| LoadError::InvalidValue {
        event: event.to_string(),
        reason,
    };

    let mut converted = Vec::with_capacity(effects.len());
    for effect in effects {
        converted.push(match effect {
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
            EffectData::Faith(v) => Effect::Faith(*v),
            EffectData::Mana(v) => Effect::Mana(*v),
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
            EffectData::TriggerEvent(key, months) => {
                let &(id, follow_up_scope) =
                    event_ids.get(key).ok_or_else(|| LoadError::UnknownEvent {
                        event: event.to_string(),
                        target: key.clone(),
                    })?;
                if *months < 1 {
                    return Err(invalid("TriggerEvent precisa de ao menos 1 mês"));
                }
                if scope == EventScope::Faction && follow_up_scope == EventScope::Province {
                    return Err(invalid(
                        "efeito de facção não pode encadear evento de província",
                    ));
                }
                Effect::TriggerEvent {
                    event: id,
                    months: *months,
                }
            }
        });
    }
    Ok(converted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD: &str = include_str!("../../../content/world.ron");
    const EVENTS: &str = include_str!("../../../content/events.ron");
    const RITES: &str = include_str!("../../../content/rites.ron");

    fn rites_world(rites: &str) -> Result<World, LoadError> {
        load_rites(rites, events_world(EVENTS).unwrap())
    }

    #[test]
    fn bundled_rites_load() {
        let world = rites_world(RITES).expect("os ritos embutidos devem carregar");
        assert!(world.rites.len() >= 6);
        assert!(world.rites.iter().any(|rite| rite.faith_cost > 0));
        assert!(world.rites.iter().any(|rite| rite.mana_cost > 0));
    }

    #[test]
    fn bundled_world_has_starting_faith() {
        let world = load_world(WORLD, 1).unwrap();
        assert!(world.factions.iter().any(|faction| faction.faith > 0));
    }

    #[test]
    fn rite_with_unknown_modifier_is_rejected() {
        let source = r#"(rites: [(key: "r", name: "R", scope: Province, faith_cost: 1,
                                 effects: [RemoveModifier("nada")])])"#;
        assert!(matches!(
            rites_world(source),
            Err(LoadError::UnknownModifier { .. })
        ));
    }

    #[test]
    fn rite_with_negative_cost_is_rejected() {
        let source = r#"(rites: [(key: "r", name: "R", scope: Faction, mana_cost: -5,
                                 effects: [Legitimacy(1)])])"#;
        assert!(matches!(
            rites_world(source),
            Err(LoadError::InvalidValue { .. })
        ));
    }

    #[test]
    fn faction_rite_with_province_condition_is_rejected() {
        let source = r#"(rites: [(key: "r", name: "R", scope: Faction,
                                 conditions: [Famine], effects: [Legitimacy(1)])])"#;
        assert!(matches!(
            rites_world(source),
            Err(LoadError::ProvinceConditionInFactionEvent { .. })
        ));
    }

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
    fn bundled_events_include_choices() {
        let world = events_world(EVENTS).unwrap();
        assert!(world.events.iter().any(|event| event.options.len() >= 2));
    }

    #[test]
    fn single_option_is_rejected() {
        let source = r#"(
            events: [(key: "e", title: "E", scope: Province, chance: 1,
                      options: [(label: "Só esta", effects: [Treasury(1)])])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::InvalidValue { .. })
        ));
    }

    #[test]
    fn unknown_modifier_inside_option_is_rejected() {
        let source = r#"(
            events: [(key: "e", title: "E", scope: Province, chance: 1, options: [
                (label: "A", effects: [Treasury(1)]),
                (label: "B", effects: [AddModifier("nada", None)]),
            ])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::UnknownModifier { .. })
        ));
    }

    #[test]
    fn bundled_events_include_chains() {
        let world = events_world(EVENTS).unwrap();
        let chains = world
            .events
            .iter()
            .flat_map(|event| {
                event
                    .effects
                    .iter()
                    .chain(event.options.iter().flat_map(|o| o.effects.iter()))
            })
            .filter(|effect| matches!(effect, Effect::TriggerEvent { .. }))
            .count();
        assert!(chains >= 4, "esperava crises encadeadas, achei {chains}");
    }

    #[test]
    fn chain_can_reference_later_event() {
        let source = r#"(
            events: [
                (key: "a", title: "A", scope: Province, chance: 1, effects: [TriggerEvent("b", 6)]),
                (key: "b", title: "B", scope: Faction, chance: 0, effects: [Treasury(1)]),
            ],
        )"#;
        let world = events_world(source).unwrap();
        assert_eq!(
            world.events[0].effects,
            vec![Effect::TriggerEvent {
                event: 1,
                months: 6
            }]
        );
    }

    #[test]
    fn chain_to_unknown_event_is_rejected() {
        let source = r#"(
            events: [(key: "a", title: "A", scope: Province, chance: 1,
                      effects: [TriggerEvent("nada", 6)])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::UnknownEvent { .. })
        ));
    }

    #[test]
    fn faction_event_cannot_chain_into_province_event() {
        let source = r#"(
            events: [
                (key: "a", title: "A", scope: Faction, chance: 1, effects: [TriggerEvent("b", 6)]),
                (key: "b", title: "B", scope: Province, chance: 0, effects: [Treasury(1)]),
            ],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::InvalidValue { .. })
        ));
    }

    #[test]
    fn chain_needs_positive_delay() {
        let source = r#"(
            events: [(key: "a", title: "A", scope: Province, chance: 1,
                      effects: [TriggerEvent("a", 0)])],
        )"#;
        assert!(matches!(
            events_world(source),
            Err(LoadError::InvalidValue { .. })
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
