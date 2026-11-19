use crate::command::Command;
use crate::diplomacy::DEFAULT_TRUCE_MONTHS;
use crate::event;
use crate::rite;
use crate::rules::{self, MONTHS_PER_YEAR};
use crate::world::{FactionId, Province, ProvinceId, World};

const RNG_ROLL_MAX: i64 = 999;
const SHOCK_CHANCE: i64 = 10;

pub fn tick_month(world: &mut World) {
    apply_commands(world);
    advance_calendar(world);

    let province_count = world.provinces.len();
    let faction_count = world.factions.len();
    // Faith/mana gains are applied after the loop so that every province sees
    // the same faith and magic levels when computing unrest.
    let mut gains = vec![(0, 0); faction_count];

    for index in 0..province_count {
        let owner = world.provinces[index].owner;
        if owner >= faction_count {
            continue;
        }

        let income = rules::province_income(&world.provinces[index], &world.factions[owner]);
        let upkeep = rules::province_upkeep(&world.provinces[index]);
        world.factions[owner].treasury += income - upkeep;
        gains[owner].0 += rules::province_faith(&world.provinces[index]);
        gains[owner].1 += rules::province_mana(&world.provinces[index]);

        let at_war = world.diplomacy.war_count_for(owner) > 0;
        let breakdown =
            rules::unrest_breakdown(&world.provinces[index], &world.factions[owner], at_war);
        update_unrest(&mut world.provinces[index], breakdown.target());

        let treasury = world.factions[owner].treasury;
        grow_pops(&mut world.provinces[index], treasury);
        apply_famine(&mut world.provinces[index]);
    }

    apply_war_effects(world);
    settle_resources(world, &gains);
    pay_faction_costs(world);
    update_legitimacy(world);
    world.diplomacy.tick();
    apply_random_shocks(world);
    event::tick_all_modifiers(world);
    rite::expire_cooldowns(world);
    event::fire_events(world);
}

fn apply_commands(world: &mut World) {
    let commands = std::mem::take(&mut world.pending_commands);

    for command in commands {
        match command {
            Command::SetTax { faction: id, rate } => {
                if let Some(faction) = world.factions.get_mut(id) {
                    faction.tax_rate = rate.clamp(0, 100);
                }
            }
            Command::Recruit {
                faction: id,
                soldiers,
            } => {
                if soldiers <= 0 {
                    continue;
                }
                if let Some(faction) = world.factions.get_mut(id) {
                    let cost = soldiers.saturating_mul(2);
                    if faction.treasury >= cost {
                        faction.treasury -= cost;
                        faction.army = faction.army.saturating_add(soldiers);
                    }
                }
            }
            Command::Disband {
                faction: id,
                soldiers,
            } => {
                if soldiers <= 0 {
                    continue;
                }
                if let Some(faction) = world.factions.get_mut(id) {
                    faction.army = faction.army.saturating_sub(soldiers).max(0);
                }
            }
            Command::DeclareWar { attacker, defender } => {
                let year = world.year;
                let month = world.month;
                if world.diplomacy.declare_war(attacker, defender, year, month) {
                    if let Some(faction) = world.factions.get_mut(attacker) {
                        faction.legitimacy = (faction.legitimacy - 5).max(0);
                    }
                    if let Some(faction) = world.factions.get_mut(defender) {
                        faction.legitimacy = (faction.legitimacy - 3).max(0);
                    }
                }
            }
            Command::MakePeace { faction, other } => {
                world
                    .diplomacy
                    .make_peace(faction, other, DEFAULT_TRUCE_MONTHS);
            }
            Command::Attack { faction, province } => {
                resolve_attack(world, faction, province);
            }
            Command::SetPlayer { faction } => {
                world.player = faction.filter(|&id| id < world.factions.len());
            }
            Command::ChooseOption {
                faction,
                decision,
                option,
            } => {
                event::choose_option(world, faction, decision, option);
            }
            Command::PerformRite {
                faction,
                rite: id,
                province,
            } => {
                let _ =
                    rite::perform_rite(world, faction, id, rite::rite_target(faction, province));
            }
        }
    }
}

fn resolve_attack(world: &mut World, attacker: FactionId, province_id: ProvinceId) {
    let (defender, garrison) = match world.provinces.get(province_id) {
        Some(province) => (province.owner, rules::garrison(province)),
        None => return,
    };

    if defender == attacker || !world.diplomacy.is_at_war(attacker, defender) {
        return;
    }

    let Some(attacker_army) = world.factions.get(attacker).map(|faction| faction.army) else {
        return;
    };
    let Some(defender_army) = world.factions.get(defender).map(|faction| faction.army) else {
        return;
    };

    if attacker_army > defender_army + garrison {
        let losses = (garrison / 5).min(attacker_army);

        if let Some(faction) = world.factions.get_mut(attacker) {
            faction.army = faction.army.saturating_sub(losses);
            faction.legitimacy = (faction.legitimacy + 3).min(100);
        }
        if let Some(faction) = world.factions.get_mut(defender) {
            faction.legitimacy = (faction.legitimacy - 5).max(0);
        }
        if let Some(province) = world.provinces.get_mut(province_id) {
            province.owner = attacker;
            province.unrest = (province.unrest + 10).clamp(0, 100);
        }

        world.diplomacy.set_relation(attacker, defender, -100);
        end_wars_if_eliminated(world, defender);
    } else {
        let losses = (garrison / 10).max(1).min(attacker_army);

        if let Some(faction) = world.factions.get_mut(attacker) {
            faction.army = faction.army.saturating_sub(losses);
        }
        if let Some(province) = world.provinces.get_mut(province_id) {
            province.unrest = (province.unrest + 5).clamp(0, 100);
        }
    }
}

fn end_wars_if_eliminated(world: &mut World, faction: FactionId) {
    if world.province_count(faction) > 0 {
        return;
    }

    let opponents: Vec<FactionId> = world
        .diplomacy
        .wars
        .iter()
        .filter(|war| war.has(faction))
        .map(|war| {
            if war.attacker == faction {
                war.defender
            } else {
                war.attacker
            }
        })
        .collect();

    for opponent in opponents {
        world
            .diplomacy
            .make_peace(faction, opponent, DEFAULT_TRUCE_MONTHS);
    }
}

fn apply_war_effects(world: &mut World) {
    let wars = world.diplomacy.wars.clone();

    for war in &wars {
        apply_attrition(world, war.attacker);
        apply_attrition(world, war.defender);

        if let Some(faction) = world.factions.get_mut(war.attacker) {
            faction.legitimacy = (faction.legitimacy - 1).max(0);
        }
        if let Some(faction) = world.factions.get_mut(war.defender) {
            faction.legitimacy = (faction.legitimacy - 1).max(0);
        }

        world
            .diplomacy
            .set_relation(war.attacker, war.defender, -100);
    }
}

fn apply_attrition(world: &mut World, faction: FactionId) {
    if let Some(faction) = world.factions.get_mut(faction) {
        if faction.army > 0 {
            let losses = (faction.army / 100).max(1);
            faction.army = (faction.army - losses).max(0);
        }
    }
}

fn advance_calendar(world: &mut World) {
    world.month += 1;
    if world.month > MONTHS_PER_YEAR {
        world.month = 1;
        world.year += 1;
    }
}

fn update_unrest(province: &mut Province, target: i64) {
    let delta = (target - province.unrest) / 10;
    province.unrest = (province.unrest + delta).clamp(0, 100);
}

fn grow_pops(province: &mut Province, treasury: i64) {
    let capacity = rules::food_capacity(province);
    let total = province.pops.total();
    if capacity <= 0 || total >= capacity {
        return;
    }

    let room = ((capacity - total) * 100 / capacity).clamp(0, 100);
    let pressure = (100 - province.unrest).clamp(0, 100);

    let peasant_growth = (province.pops.peasant / 100) * pressure / 100 * room / 100;
    province.pops.peasant += peasant_growth;
    if province.unrest > 80 {
        province.pops.peasant -= province.pops.peasant / 200;
    }

    if treasury > 0 {
        province.pops.burgher += (province.pops.burgher / 200) * room / 100;
    }

    province.pops.noble += (province.pops.noble / 400) * room / 100;
    province.pops.clergy += (province.pops.clergy / 500) * room / 100;

    clamp_populations(province);
}

fn apply_famine(province: &mut Province) {
    let capacity = rules::food_capacity(province);
    let total = province.pops.total();
    if total <= capacity {
        return;
    }

    let excess = total - capacity;
    let deaths = (excess / 20).max(1).min(total);
    remove_proportional(province, deaths);
    province.unrest = (province.unrest + 3).clamp(0, 100);
}

fn remove_proportional(province: &mut Province, deaths: i64) {
    let total = province.pops.total();
    if total <= 0 {
        return;
    }

    let peasant_deaths = deaths * province.pops.peasant / total;
    let burgher_deaths = deaths * province.pops.burgher / total;
    let noble_deaths = deaths * province.pops.noble / total;
    let clergy_deaths = deaths - peasant_deaths - burgher_deaths - noble_deaths;

    province.pops.peasant = (province.pops.peasant - peasant_deaths).max(0);
    province.pops.burgher = (province.pops.burgher - burgher_deaths).max(0);
    province.pops.noble = (province.pops.noble - noble_deaths).max(0);
    province.pops.clergy = (province.pops.clergy - clergy_deaths).max(0);
}

fn clamp_populations(province: &mut Province) {
    province.pops.peasant = province.pops.peasant.max(0);
    province.pops.burgher = province.pops.burgher.max(0);
    province.pops.noble = province.pops.noble.max(0);
    province.pops.clergy = province.pops.clergy.max(0);
}

fn settle_resources(world: &mut World, gains: &[(i64, i64)]) {
    for (faction, &(faith, mana)) in world.factions.iter_mut().zip(gains) {
        let faith_decay = faction.faith / rules::RESOURCE_DECAY_DIVISOR;
        let mana_decay = faction.mana / rules::RESOURCE_DECAY_DIVISOR;
        faction.faith = (faction.faith - faith_decay + faith + rules::faction_faith_bonus(faction))
            .clamp(0, rules::MAX_RESOURCE);
        faction.mana = (faction.mana - mana_decay + mana + rules::faction_mana_bonus(faction))
            .clamp(0, rules::MAX_RESOURCE);
    }
}

fn pay_faction_costs(world: &mut World) {
    for faction in &mut world.factions {
        faction.treasury -= faction.court_upkeep + rules::army_upkeep(faction);
    }
}

fn update_legitimacy(world: &mut World) {
    for faction in &mut world.factions {
        if faction.legitimacy < 50 {
            faction.legitimacy += 1;
        } else if faction.legitimacy > 50 {
            faction.legitimacy -= 1;
        }

        if faction.treasury < -500 || faction.tax_rate > 50 {
            faction.legitimacy -= 1;
        }

        faction.legitimacy += rules::legitimacy_from_modifiers(faction);

        faction.legitimacy = faction.legitimacy.clamp(0, 100);
    }
}

fn apply_random_shocks(world: &mut World) {
    for index in 0..world.provinces.len() {
        if world.rng.range_i64(0, RNG_ROLL_MAX) < SHOCK_CHANCE {
            let shock = world.rng.range_i64(-5, 5);
            let province = &mut world.provinces[index];
            province.unrest = (province.unrest + shock).clamp(0, 100);
        }
    }
}
