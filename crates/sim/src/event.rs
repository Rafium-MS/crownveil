use serde::{Deserialize, Serialize};

use crate::modifier::{self, ModifierId};
use crate::rules;
use crate::world::{FactionId, ProvinceId, World};

pub type EventId = usize;

pub const CHANCE_SCALE: i64 = 1000;
pub const EVENT_LOG_LIMIT: usize = 200;
/// Months the player has to answer before the default (first) option applies.
pub const DECISION_DEADLINE_MONTHS: i64 = 12;

pub type DecisionId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventScope {
    Province,
    Faction,
}

/// In province scope, faction conditions refer to the province's owner.
/// Province-only conditions are always false in faction scope (the content
/// loader rejects them there).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
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
    HasModifier(ModifierId),
    LacksModifier(ModifierId),
}

impl Condition {
    pub fn is_province_only(&self) -> bool {
        matches!(
            self,
            Condition::UnrestAtLeast(_)
                | Condition::UnrestBelow(_)
                | Condition::DevelopmentAtLeast(_)
                | Condition::DevelopmentBelow(_)
                | Condition::ClergyAtLeast(_)
                | Condition::Famine
                | Condition::NoFamine
        )
    }
}

/// In province scope, faction effects (legitimacy, treasury, army) hit the owner.
/// In faction scope, province effects hit every province the faction owns.
/// `AddModifier`/`RemoveModifier` target the event's own scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Unrest(i64),
    Development(i64),
    /// Percentage change to every population class.
    Population(i64),
    Legitimacy(i64),
    Treasury(i64),
    Army(i64),
    AddModifier {
        modifier: ModifierId,
        months: Option<i64>,
    },
    RemoveModifier(ModifierId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDef {
    pub key: String,
    pub title: String,
    pub description: String,
    pub scope: EventScope,
    /// Monthly chance per target, out of `CHANCE_SCALE`.
    pub chance: i64,
    pub cooldown_months: i64,
    pub conditions: Vec<Condition>,
    /// Applied as soon as the event fires, before any choice.
    pub effects: Vec<Effect>,
    /// Empty means the event resolves on its own. Otherwise the deciding faction
    /// picks one; the first option is the AI's pick and the timeout default.
    #[serde(default)]
    pub options: Vec<EventOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventOption {
    pub label: String,
    pub effects: Vec<Effect>,
}

/// A choice waiting for the player. The decider is fixed when the event fires,
/// so losing the province afterwards does not hand the choice to someone else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDecision {
    pub id: DecisionId,
    pub event: EventId,
    pub target: EventTarget,
    pub faction: FactionId,
    pub deadline: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventTarget {
    Province(ProvinceId),
    Faction(FactionId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    pub year: i64,
    pub month: u32,
    pub event: EventId,
    pub target: EventTarget,
    #[serde(default)]
    pub choice: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCooldown {
    pub event: EventId,
    pub target: EventTarget,
    pub until: i64,
}

pub fn absolute_month(world: &World) -> i64 {
    world.year * rules::MONTHS_PER_YEAR as i64 + world.month as i64
}

pub fn tick_all_modifiers(world: &mut World) {
    for province in &mut world.provinces {
        modifier::tick_modifiers(&mut province.modifiers);
    }
    for faction in &mut world.factions {
        modifier::tick_modifiers(&mut faction.modifiers);
    }
}

/// Events and targets are visited in index order and the RNG is only consumed
/// for targets that pass their conditions and cooldown, so the outcome depends
/// only on the seed and the world state.
pub fn fire_events(world: &mut World) {
    let now = absolute_month(world);
    world
        .event_cooldowns
        .retain(|cooldown| cooldown.until > now);
    expire_decisions(world, now);

    for event_id in 0..world.events.len() {
        let targets: Vec<EventTarget> = match world.events[event_id].scope {
            EventScope::Province => (0..world.provinces.len())
                .map(EventTarget::Province)
                .collect(),
            EventScope::Faction => (0..world.factions.len())
                .filter(|&faction| world.province_count(faction) > 0)
                .map(EventTarget::Faction)
                .collect(),
        };

        for target in targets {
            if on_cooldown(world, event_id, target) {
                continue;
            }
            let event = &world.events[event_id];
            if !event
                .conditions
                .iter()
                .all(|condition| condition_holds(world, target, condition))
            {
                continue;
            }
            let chance = event.chance;
            if world.rng.range_i64(0, CHANCE_SCALE - 1) >= chance {
                continue;
            }
            trigger_event(world, event_id, target);
        }
    }
}

/// Applies an event immediately, bypassing chance and conditions. Used by
/// `fire_events` and handy for tests and debugging tools.
pub fn trigger_event(world: &mut World, event_id: EventId, target: EventTarget) {
    let Some(event) = world.events.get(event_id) else {
        return;
    };
    let effects = event.effects.clone();
    let cooldown = event.cooldown_months;
    let has_options = !event.options.is_empty();

    for effect in &effects {
        apply_effect(world, target, effect);
    }

    let now = absolute_month(world);
    if cooldown > 0 {
        world.event_cooldowns.push(EventCooldown {
            event: event_id,
            target,
            until: now + cooldown,
        });
    }

    if !has_options {
        log_event(world, event_id, target, None);
        return;
    }

    let Some(decider) = target_faction(world, target) else {
        return;
    };
    if world.player == Some(decider) {
        let id = world.next_decision_id;
        world.next_decision_id += 1;
        world.pending_decisions.push(PendingDecision {
            id,
            event: event_id,
            target,
            faction: decider,
            deadline: now + DECISION_DEADLINE_MONTHS,
        });
    } else {
        resolve_option(world, event_id, target, 0);
    }
}

/// Resolves a pending decision for `faction`. Returns `false` (and changes
/// nothing) if the decision does not exist, belongs to another faction or the
/// option is out of range.
pub fn choose_option(
    world: &mut World,
    faction: FactionId,
    decision: DecisionId,
    option: usize,
) -> bool {
    let Some(index) = world
        .pending_decisions
        .iter()
        .position(|pending| pending.id == decision && pending.faction == faction)
    else {
        return false;
    };
    let pending = &world.pending_decisions[index];
    let option_count = world
        .events
        .get(pending.event)
        .map_or(0, |event| event.options.len());
    if option >= option_count {
        return false;
    }

    let pending = world.pending_decisions.remove(index);
    resolve_option(world, pending.event, pending.target, option);
    true
}

fn expire_decisions(world: &mut World, now: i64) {
    let (expired, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut world.pending_decisions)
        .into_iter()
        .partition(|pending| pending.deadline <= now);
    world.pending_decisions = waiting;
    for pending in expired {
        resolve_option(world, pending.event, pending.target, 0);
    }
}

fn resolve_option(world: &mut World, event_id: EventId, target: EventTarget, option: usize) {
    let Some(effects) = world
        .events
        .get(event_id)
        .and_then(|event| event.options.get(option))
        .map(|option| option.effects.clone())
    else {
        return;
    };
    for effect in &effects {
        apply_effect(world, target, effect);
    }
    log_event(world, event_id, target, Some(option));
}

fn log_event(world: &mut World, event: EventId, target: EventTarget, choice: Option<usize>) {
    world.event_log.push(EventRecord {
        year: world.year,
        month: world.month,
        event,
        target,
        choice,
    });
    if world.event_log.len() > EVENT_LOG_LIMIT {
        let excess = world.event_log.len() - EVENT_LOG_LIMIT;
        world.event_log.drain(..excess);
    }
}

fn on_cooldown(world: &World, event: EventId, target: EventTarget) -> bool {
    world
        .event_cooldowns
        .iter()
        .any(|cooldown| cooldown.event == event && cooldown.target == target)
}

fn target_faction(world: &World, target: EventTarget) -> Option<FactionId> {
    match target {
        EventTarget::Province(id) => world.provinces.get(id).map(|province| province.owner),
        EventTarget::Faction(id) => Some(id),
    }
}

pub fn condition_holds(world: &World, target: EventTarget, condition: &Condition) -> bool {
    let province = match target {
        EventTarget::Province(id) => world.provinces.get(id),
        EventTarget::Faction(_) => None,
    };
    let faction_id = target_faction(world, target);
    let faction = faction_id.and_then(|id| world.factions.get(id));

    match condition {
        Condition::UnrestAtLeast(value) => province.is_some_and(|p| p.unrest >= *value),
        Condition::UnrestBelow(value) => province.is_some_and(|p| p.unrest < *value),
        Condition::DevelopmentAtLeast(value) => province.is_some_and(|p| p.development >= *value),
        Condition::DevelopmentBelow(value) => province.is_some_and(|p| p.development < *value),
        Condition::ClergyAtLeast(value) => province.is_some_and(|p| p.pops.clergy >= *value),
        Condition::Famine => province.is_some_and(rules::is_famine),
        Condition::NoFamine => province.is_some_and(|p| !rules::is_famine(p)),
        Condition::LegitimacyAtLeast(value) => faction.is_some_and(|f| f.legitimacy >= *value),
        Condition::LegitimacyBelow(value) => faction.is_some_and(|f| f.legitimacy < *value),
        Condition::TreasuryAtLeast(value) => faction.is_some_and(|f| f.treasury >= *value),
        Condition::TreasuryBelow(value) => faction.is_some_and(|f| f.treasury < *value),
        Condition::TaxAtLeast(value) => faction.is_some_and(|f| f.tax_rate >= *value),
        Condition::AtWar => faction_id.is_some_and(|id| world.is_at_war(id)),
        Condition::AtPeace => faction_id.is_some_and(|id| !world.is_at_war(id)),
        Condition::HasModifier(modifier) => {
            target_modifiers(world, target).is_some_and(|m| modifier::has_modifier(m, *modifier))
        }
        Condition::LacksModifier(modifier) => {
            target_modifiers(world, target).is_some_and(|m| !modifier::has_modifier(m, *modifier))
        }
    }
}

fn target_modifiers(world: &World, target: EventTarget) -> Option<&[modifier::ActiveModifier]> {
    match target {
        EventTarget::Province(id) => world.provinces.get(id).map(|p| p.modifiers.as_slice()),
        EventTarget::Faction(id) => world.factions.get(id).map(|f| f.modifiers.as_slice()),
    }
}

fn affected_provinces(world: &World, target: EventTarget) -> Vec<ProvinceId> {
    match target {
        EventTarget::Province(id) => vec![id],
        EventTarget::Faction(faction) => world
            .provinces
            .iter()
            .filter(|province| province.owner == faction)
            .map(|province| province.id)
            .collect(),
    }
}

fn apply_effect(world: &mut World, target: EventTarget, effect: &Effect) {
    match effect {
        Effect::Unrest(delta) => {
            for id in affected_provinces(world, target) {
                if let Some(province) = world.provinces.get_mut(id) {
                    province.unrest = (province.unrest + delta).clamp(0, 100);
                }
            }
        }
        Effect::Development(delta) => {
            for id in affected_provinces(world, target) {
                if let Some(province) = world.provinces.get_mut(id) {
                    province.development = (province.development + delta).max(1);
                }
            }
        }
        Effect::Population(pct) => {
            for id in affected_provinces(world, target) {
                if let Some(province) = world.provinces.get_mut(id) {
                    let pops = &mut province.pops;
                    pops.peasant = modifier::apply_pct(pops.peasant, *pct);
                    pops.burgher = modifier::apply_pct(pops.burgher, *pct);
                    pops.noble = modifier::apply_pct(pops.noble, *pct);
                    pops.clergy = modifier::apply_pct(pops.clergy, *pct);
                }
            }
        }
        Effect::Legitimacy(delta) => {
            if let Some(faction) = faction_mut(world, target) {
                faction.legitimacy = (faction.legitimacy + delta).clamp(0, 100);
            }
        }
        Effect::Treasury(delta) => {
            if let Some(faction) = faction_mut(world, target) {
                faction.treasury += delta;
            }
        }
        Effect::Army(delta) => {
            if let Some(faction) = faction_mut(world, target) {
                faction.army = (faction.army + delta).max(0);
            }
        }
        Effect::AddModifier { modifier, months } => {
            let Some(def) = world.modifiers.get(*modifier).cloned() else {
                return;
            };
            match target {
                EventTarget::Province(id) => {
                    if let Some(province) = world.provinces.get_mut(id) {
                        modifier::add_modifier(&mut province.modifiers, &def, *modifier, *months);
                    }
                }
                EventTarget::Faction(id) => {
                    if let Some(faction) = world.factions.get_mut(id) {
                        modifier::add_modifier(&mut faction.modifiers, &def, *modifier, *months);
                    }
                }
            }
        }
        Effect::RemoveModifier(modifier) => match target {
            EventTarget::Province(id) => {
                if let Some(province) = world.provinces.get_mut(id) {
                    modifier::remove_modifier(&mut province.modifiers, *modifier);
                }
            }
            EventTarget::Faction(id) => {
                if let Some(faction) = world.factions.get_mut(id) {
                    modifier::remove_modifier(&mut faction.modifiers, *modifier);
                }
            }
        },
    }
}

fn faction_mut(world: &mut World, target: EventTarget) -> Option<&mut crate::world::Faction> {
    let id = target_faction(world, target)?;
    world.factions.get_mut(id)
}
