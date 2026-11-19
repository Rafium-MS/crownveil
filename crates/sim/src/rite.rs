use serde::{Deserialize, Serialize};

use crate::event::{self, absolute_month, Condition, Effect, EventScope, EventTarget};
use crate::world::{FactionId, World};

pub type RiteId = usize;

/// A player-triggered action paid with faith, magic and/or gold. Province rites
/// target a province the faction owns; faction rites target the faction itself.
/// Conditions and effects use the same vocabulary and scope rules as events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiteDef {
    pub key: String,
    pub name: String,
    pub description: String,
    pub scope: EventScope,
    pub faith_cost: i64,
    pub mana_cost: i64,
    pub gold_cost: i64,
    pub cooldown_months: i64,
    pub conditions: Vec<Condition>,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiteCooldown {
    pub rite: RiteId,
    pub target: EventTarget,
    pub until: i64,
}

/// Why a rite cannot be performed right now; lets the UI explain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiteBlock {
    UnknownRite,
    WrongScope,
    NotOwner,
    NotEnoughFaith,
    NotEnoughMana,
    NotEnoughGold,
    OnCooldown { months_left: i64 },
    ConditionsUnmet,
}

pub fn rite_target(faction: FactionId, province: Option<usize>) -> EventTarget {
    match province {
        Some(id) => EventTarget::Province(id),
        None => EventTarget::Faction(faction),
    }
}

pub fn check_rite(
    world: &World,
    faction: FactionId,
    rite: RiteId,
    target: EventTarget,
) -> Result<(), RiteBlock> {
    let def = world.rites.get(rite).ok_or(RiteBlock::UnknownRite)?;
    let Some(caster) = world.factions.get(faction) else {
        return Err(RiteBlock::NotOwner);
    };

    match (def.scope, target) {
        (EventScope::Province, EventTarget::Province(id)) => {
            let owner = world.provinces.get(id).map(|province| province.owner);
            if owner != Some(faction) {
                return Err(RiteBlock::NotOwner);
            }
        }
        (EventScope::Faction, EventTarget::Faction(id)) => {
            if id != faction {
                return Err(RiteBlock::NotOwner);
            }
        }
        _ => return Err(RiteBlock::WrongScope),
    }

    let now = absolute_month(world);
    if let Some(cooldown) = world
        .rite_cooldowns
        .iter()
        .find(|c| c.rite == rite && c.target == target && c.until > now)
    {
        return Err(RiteBlock::OnCooldown {
            months_left: cooldown.until - now,
        });
    }

    if caster.faith < def.faith_cost {
        return Err(RiteBlock::NotEnoughFaith);
    }
    if caster.mana < def.mana_cost {
        return Err(RiteBlock::NotEnoughMana);
    }
    if caster.treasury < def.gold_cost {
        return Err(RiteBlock::NotEnoughGold);
    }

    if !def
        .conditions
        .iter()
        .all(|condition| event::condition_holds(world, target, condition))
    {
        return Err(RiteBlock::ConditionsUnmet);
    }

    Ok(())
}

pub fn perform_rite(
    world: &mut World,
    faction: FactionId,
    rite: RiteId,
    target: EventTarget,
) -> Result<(), RiteBlock> {
    check_rite(world, faction, rite, target)?;
    let def = world.rites[rite].clone();

    let caster = &mut world.factions[faction];
    caster.faith -= def.faith_cost;
    caster.mana -= def.mana_cost;
    caster.treasury -= def.gold_cost;

    for effect in &def.effects {
        event::apply_effect(world, target, effect, None);
    }

    if def.cooldown_months > 0 {
        world.rite_cooldowns.push(RiteCooldown {
            rite,
            target,
            until: absolute_month(world) + def.cooldown_months,
        });
    }
    Ok(())
}

pub fn expire_cooldowns(world: &mut World) {
    let now = absolute_month(world);
    world.rite_cooldowns.retain(|cooldown| cooldown.until > now);
}
