use crownveil_content::load_world;
use crownveil_sim::World;

const WORLD_SOURCE: &str = include_str!("../../../content/world.ron");
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

    let mut world = match load_world(WORLD_SOURCE, seed) {
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
    println!();

    let months = years * MONTHS_PER_YEAR;
    for month in 0..months {
        world.tick_month();
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
    println!("== Relatório final ==");
    print_snapshot(&world);
}

fn print_snapshot(world: &World) {
    println!("== Ano {} ==", world.year);
    for faction in &world.factions {
        println!(
            "  {:<24} tesouro {:>9}  leg {:>3}  imposto {:>3}%  exército {:>6}",
            faction.name, faction.treasury, faction.legitimacy, faction.tax_rate, faction.army
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
                "    legitimidade {}, tesouro {}, imposto {}, comida {}, guerra {}",
                explanation.breakdown.legitimacy,
                explanation.breakdown.treasury,
                explanation.breakdown.tax,
                explanation.breakdown.food,
                explanation.breakdown.war
            );
        }
    }
    println!();
}
