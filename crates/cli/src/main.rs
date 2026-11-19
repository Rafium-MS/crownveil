use crownveil_content::{load_events, load_rites, load_world};
use crownveil_sim::World;

const WORLD_SOURCE: &str = include_str!("../../../content/world.ron");
const EVENTS_SOURCE: &str = include_str!("../../../content/events.ron");
const RITES_SOURCE: &str = include_str!("../../../content/rites.ron");
const MONTHS_PER_YEAR: i64 = 12;
const REPORT_EVERY_YEARS: i64 = 10;

fn main() {
    let mut args = std::env::args().skip(1);
    let years: i64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);
    let seed: u64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);

    if years <= 0 {
        eprintln!("o número de anos precisa ser positivo");
        std::process::exit(1);
    }

    let mut world = match load_world(WORLD_SOURCE, seed)
        .and_then(|world| load_events(EVENTS_SOURCE, world))
        .and_then(|world| load_rites(RITES_SOURCE, world))
    {
        Ok(world) => world,
        Err(error) => {
            eprintln!("erro ao carregar conteúdo: {error}");
            std::process::exit(1);
        }
    };

    println!("Crownveil — simulação headless");
    println!("seed: {seed}");
    println!("facções: {}", world.factions.len());
    println!("províncias: {}", world.provinces.len());
    println!("eventos: {}", world.events.len());
    println!();

    let months = years * MONTHS_PER_YEAR;
    let mut logged = 0;
    for month in 0..months {
        world.tick_month();
        print_new_events(&world, &mut logged);
        let elapsed_years = (month + 1) / MONTHS_PER_YEAR;
        if elapsed_years > 0
            && elapsed_years < years
            && elapsed_years % REPORT_EVERY_YEARS == 0
            && (month + 1) % MONTHS_PER_YEAR == 0
        {
            print_snapshot(&world);
        }
    }

    println!();
    println!("eventos ocorridos: {logged}");
    println!();
    println!("== Relatório final ==");
    print_snapshot(&world);
}

fn print_new_events(world: &World, logged: &mut usize) {
    // The log is bounded, so track by the latest entries instead of an index.
    let fresh = world
        .event_log
        .iter()
        .rev()
        .take_while(|record| record.year == world.year && record.month == world.month)
        .count();
    for record in &world.event_log[world.event_log.len() - fresh..] {
        let title = world
            .event_def(record.event)
            .map(|event| event.title.as_str())
            .unwrap_or("?");
        println!(
            "  [{:>2}/{}] {} — {}{}",
            record.month,
            record.year,
            title,
            world.target_name(record.target),
            world
                .choice_label(record)
                .map(|label| format!(" → {label}"))
                .unwrap_or_default()
        );
    }
    *logged += fresh;
}

fn print_snapshot(world: &World) {
    println!("== Ano {} ==", world.year);
    for faction in &world.factions {
        println!(
            "  {:<24} tesouro {:>9}  leg {:>3}  imposto {:>3}%  exército {:>6}  fé {:>4}  magia {:>4}",
            faction.name,
            faction.treasury,
            faction.legitimacy,
            faction.tax_rate,
            faction.army,
            faction.faith,
            faction.mana
        );
    }
    println!(
        "  população total: {}  descontentamento médio: {}",
        world.total_population(),
        world.average_unrest()
    );

    if world.diplomacy.wars.is_empty() {
        println!("  guerras: nenhuma");
    } else {
        for war in &world.diplomacy.wars {
            let attacker = world
                .faction(war.attacker)
                .map(|faction| faction.name.as_str())
                .unwrap_or("?");
            let defender = world
                .faction(war.defender)
                .map(|faction| faction.name.as_str())
                .unwrap_or("?");
            println!("  guerra: {attacker} vs {defender}");
        }
    }

    if let Some(province) = world
        .provinces
        .iter()
        .max_by_key(|province| province.unrest)
    {
        if let Some(explanation) = world.explain_unrest(province.id) {
            println!(
                "  maior descontentamento: {} ({}), alvo {}",
                province.name, explanation.current, explanation.target
            );
            println!(
                "    legitimidade {}, tesouro {}, imposto {}, comida {}, guerra {}, modificadores {}, fé {}, magia {}",
                explanation.breakdown.legitimacy,
                explanation.breakdown.treasury,
                explanation.breakdown.tax,
                explanation.breakdown.food,
                explanation.breakdown.war,
                explanation.breakdown.modifiers,
                explanation.breakdown.faith,
                explanation.breakdown.magic
            );
        }
    }
    println!();
}
