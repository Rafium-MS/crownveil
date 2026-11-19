use std::io::{self, BufRead, Write};

use crownveil_content::{load_events, load_rites, load_world, rite_block_text};
use crownveil_sim::event::absolute_month;
use crownveil_sim::{
    check_rite, food_capacity, rite_target, ActiveModifier, Command, EventScope, FactionId, World,
};

const WORLD_SOURCE: &str = include_str!("../../../../content/world.ron");
const EVENTS_SOURCE: &str = include_str!("../../../../content/events.ron");
const RITES_SOURCE: &str = include_str!("../../../../content/rites.ron");
const DEFAULT_EVENT_COUNT: usize = 10;
const DEFAULT_PLAYER: FactionId = 0;

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);

    let mut world = match load_world(WORLD_SOURCE, seed)
        .and_then(|world| load_events(EVENTS_SOURCE, world))
        .and_then(|world| load_rites(RITES_SOURCE, world))
    {
        Ok(world) => world.with_player(Some(DEFAULT_PLAYER)),
        Err(error) => {
            eprintln!("erro ao carregar conteúdo: {error}");
            std::process::exit(1);
        }
    };

    println!("Crownveil REPL — seed {seed}, jogando com a facção {DEFAULT_PLAYER}");
    print_help();

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let line = match lines.next() {
            Some(Ok(line)) => line,
            _ => break,
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(command) = parts.first().copied() else {
            continue;
        };

        match command {
            "help" => print_help(),
            "status" => print_status(&world),
            "provinces" => print_provinces(&world),
            "tax" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse().ok()),
            ) {
                (Some(faction), Some(rate)) => {
                    world.queue_command(Command::SetTax { faction, rate });
                    println!(
                        "imposto da facção {faction} será ajustado para {rate} no próximo tick"
                    );
                }
                _ => println!("uso: tax <facção> <0-100>"),
            },
            "recruit" => {
                match (
                    parts.get(1).and_then(|value| value.parse().ok()),
                    parts.get(2).and_then(|value| value.parse().ok()),
                ) {
                    (Some(faction), Some(soldiers)) => {
                        world.queue_command(Command::Recruit { faction, soldiers });
                        println!("recrutamento de {soldiers} soldados enfileirado para a facção {faction}");
                    }
                    _ => println!("uso: recruit <facção> <soldados>"),
                }
            }
            "disband" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse().ok()),
            ) {
                (Some(faction), Some(soldiers)) => {
                    world.queue_command(Command::Disband { faction, soldiers });
                    println!("dispensa de {soldiers} soldados enfileirada para a facção {faction}");
                }
                _ => println!("uso: disband <facção> <soldados>"),
            },
            "war" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse().ok()),
            ) {
                (Some(attacker), Some(defender)) => {
                    world.queue_command(Command::DeclareWar { attacker, defender });
                    println!("guerra entre facções {attacker} e {defender} enfileirada");
                }
                _ => println!("uso: war <atacante> <defensor>"),
            },
            "peace" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse().ok()),
            ) {
                (Some(faction), Some(other)) => {
                    world.queue_command(Command::MakePeace { faction, other });
                    println!("paz entre facções {faction} e {other} enfileirada");
                }
                _ => println!("uso: peace <facção> <outra>"),
            },
            "attack" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse().ok()),
            ) {
                (Some(faction), Some(province)) => {
                    world.queue_command(Command::Attack { faction, province });
                    println!("ataque da facção {faction} à província {province} enfileirado");
                }
                _ => println!("uso: attack <facção> <província>"),
            },
            "wars" => print_wars(&world),
            "rites" => print_rites(&world, parts.get(1).and_then(|value| value.parse().ok())),
            "rite" => match (
                world.player,
                parts.get(1).and_then(|value| value.parse::<usize>().ok()),
            ) {
                (None, _) => println!("nenhuma facção está sob controle do jogador"),
                (Some(faction), Some(rite)) => {
                    let province = parts.get(2).and_then(|value| value.parse().ok());
                    let target = rite_target(faction, province);
                    match check_rite(&world, faction, rite, target) {
                        Ok(()) => {
                            world.queue_command(Command::PerformRite {
                                faction,
                                rite,
                                province,
                            });
                            let name = world.rite_def(rite).map_or("?", |def| def.name.as_str());
                            println!("rito '{name}' enfileirado para o próximo tick");
                        }
                        Err(block) => println!("não é possível: {}", rite_block_text(block)),
                    }
                }
                _ => println!("uso: rite <rito> [província]"),
            },
            "events" => {
                let count = parts
                    .get(1)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_EVENT_COUNT);
                print_events(&world, count);
            }
            "step" => {
                let months: usize = parts
                    .get(1)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1);
                let mut advanced = 0;
                let decisions_before = world.next_decision_id;
                for _ in 0..months {
                    world.tick_month();
                    advanced += 1;
                    if world.next_decision_id != decisions_before {
                        break;
                    }
                }
                println!("avançou {advanced} mês(es)");
                print_status(&world);
                print_decisions(&world);
            }
            "decisions" => print_decisions(&world),
            "choose" => match (
                parts.get(1).and_then(|value| value.parse().ok()),
                parts.get(2).and_then(|value| value.parse::<usize>().ok()),
            ) {
                (Some(decision), Some(option)) if option >= 1 => match world.player {
                    Some(faction) => {
                        world.queue_command(Command::ChooseOption {
                            faction,
                            decision,
                            option: option - 1,
                        });
                        println!("escolha {option} da decisão {decision} enfileirada");
                    }
                    None => println!("nenhuma facção está sob controle do jogador"),
                },
                _ => println!("uso: choose <decisão> <opção (1, 2, ...)>"),
            },
            "player" => match parts.get(1).copied() {
                Some("none") => {
                    world.queue_command(Command::SetPlayer { faction: None });
                    println!("todas as facções passarão para a IA no próximo tick");
                }
                Some(value) => match value.parse() {
                    Ok(faction) => {
                        world.queue_command(Command::SetPlayer {
                            faction: Some(faction),
                        });
                        println!("controle da facção {faction} enfileirado");
                    }
                    Err(_) => println!("uso: player <facção|none>"),
                },
                None => match world.player {
                    Some(faction) => println!("jogando com a facção {faction}"),
                    None => println!("todas as facções estão com a IA"),
                },
            },
            "explain" => match parts.get(1).and_then(|value| value.parse().ok()) {
                Some(province) => print_explanation(&world, province),
                None => println!("uso: explain <província>"),
            },
            "quit" | "exit" => break,
            other => println!("comando desconhecido: {other} (use 'help')"),
        }
    }
}

fn print_help() {
    println!("Comandos:");
    println!("  status                 mostra o estado das facções");
    println!("  provinces              lista províncias, população e capacidade");
    println!("  tax <facção> <0-100>   enfileira ajuste de imposto");
    println!("  recruit <facção> <n>   enfileira recrutamento (2 de ouro por soldado)");
    println!("  disband <facção> <n>   enfileira dispensa de soldados");
    println!("  war <atacante> <alvo>  declara guerra entre facções");
    println!("  peace <facção> <outra> encerra a guerra e inicia trégua");
    println!("  attack <facção> <prov> ataca uma província inimiga");
    println!("  wars                   lista guerras ativas");
    println!("  step <meses>           avança a simulação (padrão: 1); para em nova decisão");
    println!("  explain <província>    explica o descontentamento e mostra modificadores");
    println!("  events [n]             mostra os últimos n eventos (padrão: 10)");
    println!("  decisions              lista decisões pendentes do jogador");
    println!("  rites [província]      lista ritos e se estão disponíveis");
    println!("  rite <rito> [prov]     realiza um rito (província só para ritos locais)");
    println!("  choose <dec> <opção>   escolhe uma opção (aplicada no próximo tick)");
    println!("  player [facção|none]   mostra ou troca a facção do jogador");
    println!("  help                   mostra esta ajuda");
    println!("  quit                   sai");
}

fn print_status(world: &World) {
    println!("Ano {} / mês {}", world.year, world.month);
    for faction in &world.factions {
        println!(
            "  [{}] {:<24} tesouro {:>8}  leg {:>3}  imposto {:>3}%  exército {:>5}  fé {:>4}  magia {:>4}  guerras {}",
            faction.id,
            faction.name,
            faction.treasury,
            faction.legitimacy,
            faction.tax_rate,
            faction.army,
            faction.faith,
            faction.mana,
            world.diplomacy.war_count_for(faction.id)
        );
    }
    println!(
        "  população total: {}  descontentamento médio: {}",
        world.total_population(),
        world.average_unrest()
    );
    if !world.pending_commands.is_empty() {
        println!("  comandos pendentes: {}", world.pending_commands.len());
    }
}

fn print_wars(world: &World) {
    if world.diplomacy.wars.is_empty() {
        println!("sem guerras ativas");
        return;
    }

    for war in &world.diplomacy.wars {
        let attacker = world
            .faction(war.attacker)
            .map(|faction| faction.name.as_str())
            .unwrap_or("?");
        let defender = world
            .faction(war.defender)
            .map(|faction| faction.name.as_str())
            .unwrap_or("?");
        println!(
            "  {} vs {} (desde {}/{})",
            attacker, defender, war.started_month, war.started_year
        );
    }
}

fn print_provinces(world: &World) {
    for province in &world.provinces {
        let capacity = food_capacity(province);
        let owner = world
            .faction(province.owner)
            .map(|faction| faction.name.as_str())
            .unwrap_or("?");
        println!(
            "  [{}] {:<18} dono {:<22} pop {:>7}/{:>7}  descontentamento {:>3}",
            province.id,
            province.name,
            owner,
            province.pops.total(),
            capacity,
            province.unrest
        );
    }
}

fn print_explanation(world: &World, province_id: usize) {
    let Some(explanation) = world.explain_unrest(province_id) else {
        println!("província {province_id} não existe");
        return;
    };
    let province = &world.provinces[province_id];

    println!(
        "{}: descontentamento {} → alvo {} (move 10% da distância por mês)",
        province.name, explanation.current, explanation.target
    );
    println!(
        "  legitimidade +{}, tesouro +{}, imposto +{}, comida +{}, guerra +{}",
        explanation.breakdown.legitimacy,
        explanation.breakdown.treasury,
        explanation.breakdown.tax,
        explanation.breakdown.food,
        explanation.breakdown.war
    );
    println!(
        "  modificadores {:+}, fé {:+}, magia {:+}",
        explanation.breakdown.modifiers, explanation.breakdown.faith, explanation.breakdown.magic
    );
    print_modifiers(world, "  província", &province.modifiers);
    if let Some(owner) = world.faction(province.owner) {
        print_modifiers(world, "  facção", &owner.modifiers);
    }
}

fn print_modifiers(world: &World, label: &str, modifiers: &[ActiveModifier]) {
    for active in modifiers {
        let name = world
            .modifier_def(active.modifier)
            .map(|def| def.name.as_str())
            .unwrap_or("?");
        let duration = match active.months_left {
            Some(months) => format!("{months} meses"),
            None => "permanente".to_string(),
        };
        let effects = active.effects;
        println!(
            "{label}: {name} ({duration}) — descontentamento {:+}, receita {:+}%, comida {:+}%, legitimidade {:+}/mês",
            effects.unrest, effects.income_pct, effects.food_pct, effects.legitimacy
        );
    }
}

fn print_rites(world: &World, province: Option<usize>) {
    let Some(faction) = world.player else {
        println!("nenhuma facção está sob controle do jogador");
        return;
    };
    for (id, rite) in world.rites.iter().enumerate() {
        let (scope, target) = match rite.scope {
            EventScope::Faction => ("reino", Some(rite_target(faction, None))),
            EventScope::Province => ("província", province.map(|p| rite_target(faction, Some(p)))),
        };
        let status = match target.map(|target| check_rite(world, faction, id, target)) {
            Some(Ok(())) => "disponível".to_string(),
            Some(Err(block)) => rite_block_text(block),
            None => "informe a província: rites <província>".to_string(),
        };
        println!(
            "  [{id}] {} ({scope}) — fé {}, magia {}, ouro {} — {status}",
            rite.name, rite.faith_cost, rite.mana_cost, rite.gold_cost
        );
    }
}

fn print_events(world: &World, count: usize) {
    if world.event_log.is_empty() {
        println!("nenhum evento ainda");
        return;
    }

    let start = world.event_log.len().saturating_sub(count);
    for record in &world.event_log[start..] {
        let Some(event) = world.event_def(record.event) else {
            continue;
        };
        println!(
            "  [{}/{}] {} — {}",
            record.month,
            record.year,
            event.title,
            world.target_name(record.target)
        );
        if !event.description.is_empty() {
            println!("      {}", event.description);
        }
        if let Some(label) = world.choice_label(record) {
            println!("      escolha: {label}");
        }
    }
}

fn print_decisions(world: &World) {
    let Some(player) = world.player else {
        return;
    };
    for pending in world.decisions_for(player) {
        let Some(event) = world.event_def(pending.event) else {
            continue;
        };
        let now = absolute_month(world);
        println!();
        println!(
            "  ** Decisão {}: {} — {} (prazo: {} meses)",
            pending.id,
            event.title,
            world.target_name(pending.target),
            pending.deadline - now
        );
        if !event.description.is_empty() {
            println!("     {}", event.description);
        }
        for (index, option) in event.options.iter().enumerate() {
            println!("     {}. {}", index + 1, option.label);
        }
        println!("     use: choose {} <opção>", pending.id);
    }
}
