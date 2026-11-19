//! Batch balancing: runs many headless games (all factions on AI) and reports
//! how often events fire, how long modifiers last and where economies end up.
//!
//! Usage: crownveil-batch [--runs N] [--years N] [--seed N] [--csv arquivo]

use std::fmt::Write as _;
use std::thread;
use std::time::Instant;

use crownveil_content::{load_events, load_rites, load_world};
use crownveil_sim::{rules, EventScope, World};

const WORLD_SOURCE: &str = include_str!("../../../../content/world.ron");
const EVENTS_SOURCE: &str = include_str!("../../../../content/events.ron");
const RITES_SOURCE: &str = include_str!("../../../../content/rites.ron");
const MONTHS_PER_YEAR: usize = 12;
/// Fires per target per century above which an event is flagged as too frequent.
const FREQUENT_PER_TARGET_CENTURY: f64 = 4.0;
/// Share of province-months in famine above which famine is flagged.
const FAMINE_ALERT_SHARE: f64 = 0.15;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    runs: u64,
    years: usize,
    seed: u64,
    csv: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            runs: 100,
            years: 100,
            seed: 1,
            csv: None,
        }
    }
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{flag} precisa de um valor"))
        };
        match flag.as_str() {
            "--runs" => options.runs = parse_positive(value()?, flag)?,
            "--years" => options.years = parse_positive(value()?, flag)? as usize,
            "--seed" => {
                options.seed = value()?
                    .parse()
                    .map_err(|_| format!("{flag}: número inválido"))?
            }
            "--csv" => options.csv = Some(value()?.clone()),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("opção desconhecida: {other}")),
        }
    }
    Ok(options)
}

fn parse_positive(value: &str, flag: &str) -> Result<u64, String> {
    match value.parse::<u64>() {
        Ok(number) if number > 0 => Ok(number),
        _ => Err(format!("{flag}: use um número inteiro positivo")),
    }
}

/// Everything measured in one game.
#[derive(Debug, Clone, Default, PartialEq)]
struct RunStats {
    seed: u64,
    event_fires: Vec<u64>,
    /// Province-months (or faction-months) each modifier was active.
    modifier_months: Vec<u64>,
    famine_province_months: u64,
    province_months: u64,
    revolt_province_months: u64,
    final_population: i64,
    final_average_unrest: i64,
    peak_unrest: i64,
    factions: Vec<FactionEnd>,
    average_faith: Vec<i64>,
    average_mana: Vec<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FactionEnd {
    treasury: i64,
    legitimacy: i64,
    faith: i64,
    mana: i64,
    provinces: usize,
}

fn base_world(seed: u64) -> World {
    load_world(WORLD_SOURCE, seed)
        .and_then(|world| load_events(EVENTS_SOURCE, world))
        .and_then(|world| load_rites(RITES_SOURCE, world))
        .unwrap_or_else(|error| {
            eprintln!("erro ao carregar conteúdo: {error}");
            std::process::exit(1);
        })
        .with_player(None)
}

fn run_one(seed: u64, years: usize) -> RunStats {
    let mut world = base_world(seed);
    let months = years * MONTHS_PER_YEAR;
    let mut stats = RunStats {
        seed,
        event_fires: vec![0; world.events.len()],
        modifier_months: vec![0; world.modifiers.len()],
        average_faith: vec![0; world.factions.len()],
        average_mana: vec![0; world.factions.len()],
        ..RunStats::default()
    };
    let mut faith_sum = vec![0i64; world.factions.len()];
    let mut mana_sum = vec![0i64; world.factions.len()];

    for _ in 0..months {
        world.tick_month();

        // With no player there are no deferred choices, so everything that
        // fired this tick is logged with the current date.
        for record in world
            .event_log
            .iter()
            .rev()
            .take_while(|record| record.year == world.year && record.month == world.month)
        {
            stats.event_fires[record.event] += 1;
        }

        for province in &world.provinces {
            stats.province_months += 1;
            if rules::is_famine(province) {
                stats.famine_province_months += 1;
            }
            if province.unrest >= 70 {
                stats.revolt_province_months += 1;
            }
            stats.peak_unrest = stats.peak_unrest.max(province.unrest);
            for active in &province.modifiers {
                stats.modifier_months[active.modifier] += 1;
            }
        }
        for (index, faction) in world.factions.iter().enumerate() {
            faith_sum[index] += faction.faith;
            mana_sum[index] += faction.mana;
            for active in &faction.modifiers {
                stats.modifier_months[active.modifier] += 1;
            }
        }
    }

    let months = months as i64;
    stats.average_faith = faith_sum.iter().map(|sum| sum / months).collect();
    stats.average_mana = mana_sum.iter().map(|sum| sum / months).collect();
    stats.final_population = world.total_population();
    stats.final_average_unrest = world.average_unrest();
    stats.factions = world
        .factions
        .iter()
        .map(|faction| FactionEnd {
            treasury: faction.treasury,
            legitimacy: faction.legitimacy,
            faith: faction.faith,
            mana: faction.mana,
            provinces: world.province_count(faction.id),
        })
        .collect();
    stats
}

/// Runs are split across threads; results are put back in seed order, so the
/// report is identical no matter how many cores ran it.
fn run_all(options: &Options) -> Vec<RunStats> {
    let seeds: Vec<u64> = (0..options.runs)
        .map(|offset| options.seed + offset)
        .collect();
    let workers = thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(seeds.len());
    let chunk = seeds.len().div_ceil(workers);

    let mut results: Vec<RunStats> = thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .chunks(chunk)
            .map(|chunk| {
                let years = options.years;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|&seed| run_one(seed, years))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("uma partida entrou em pânico"))
            .collect()
    });
    results.sort_by_key(|stats| stats.seed);
    results
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Summary {
    mean: f64,
    p10: f64,
    median: f64,
    p90: f64,
}

fn summarize(values: impl IntoIterator<Item = f64>) -> Summary {
    let mut values: Vec<f64> = values.into_iter().collect();
    if values.is_empty() {
        return Summary {
            mean: 0.0,
            p10: 0.0,
            median: 0.0,
            p90: 0.0,
        };
    }
    values.sort_by(f64::total_cmp);
    let percentile = |p: f64| values[((values.len() - 1) as f64 * p).round() as usize];
    Summary {
        mean: values.iter().sum::<f64>() / values.len() as f64,
        p10: percentile(0.1),
        median: percentile(0.5),
        p90: percentile(0.9),
    }
}

fn summary_line(label: &str, summary: Summary) -> String {
    format!(
        "  {label:<34} média {:>9.1}   p10 {:>9.1}   mediana {:>9.1}   p90 {:>9.1}",
        summary.mean, summary.p10, summary.median, summary.p90
    )
}

fn report(world: &World, runs: &[RunStats], options: &Options) -> String {
    let mut out = String::new();
    let centuries = options.years as f64 / 100.0;
    let count = runs.len() as f64;

    let _ = writeln!(out, "== Mundo ==");
    let famine_share = runs
        .iter()
        .map(|run| run.famine_province_months as f64 / run.province_months.max(1) as f64)
        .sum::<f64>()
        / count;
    let revolt_share = runs
        .iter()
        .map(|run| run.revolt_province_months as f64 / run.province_months.max(1) as f64)
        .sum::<f64>()
        / count;
    let _ = writeln!(
        out,
        "{}",
        summary_line(
            "população final",
            summarize(runs.iter().map(|run| run.final_population as f64))
        )
    );
    let _ = writeln!(
        out,
        "{}",
        summary_line(
            "descontentamento médio final",
            summarize(runs.iter().map(|run| run.final_average_unrest as f64))
        )
    );
    let _ = writeln!(
        out,
        "{}",
        summary_line(
            "pico de descontentamento",
            summarize(runs.iter().map(|run| run.peak_unrest as f64))
        )
    );
    let _ = writeln!(
        out,
        "  províncias-mês em fome: {:.1}%   com descontentamento >= 70: {:.1}%",
        famine_share * 100.0,
        revolt_share * 100.0
    );

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "== Facções (fim da partida; fé/magia = média no tempo) =="
    );
    for (index, faction) in world.factions.iter().enumerate() {
        let pick = |f: &dyn Fn(&RunStats) -> i64| summarize(runs.iter().map(|run| f(run) as f64));
        let _ = writeln!(out, "  {}", faction.name);
        let _ = writeln!(
            out,
            "  {}",
            summary_line("tesouro", pick(&|run| run.factions[index].treasury))
        );
        let _ = writeln!(
            out,
            "  {}",
            summary_line("legitimidade", pick(&|run| run.factions[index].legitimacy))
        );
        let _ = writeln!(
            out,
            "  {}",
            summary_line("fé média", pick(&|run| run.average_faith[index]))
        );
        let _ = writeln!(
            out,
            "  {}",
            summary_line("magia média", pick(&|run| run.average_mana[index]))
        );
        let _ = writeln!(
            out,
            "  {}",
            summary_line(
                "províncias",
                pick(&|run| run.factions[index].provinces as i64)
            )
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "== Eventos (por partida; 'alvo/século' divide pelo nº de alvos possíveis) =="
    );
    let _ = writeln!(
        out,
        "  {:<34} {:>10} {:>12} {:>14}",
        "evento", "média", "alvo/século", "% partidas"
    );
    let mut alerts = Vec::new();
    for (id, event) in world.events.iter().enumerate() {
        let targets = match event.scope {
            EventScope::Province => world.provinces.len(),
            EventScope::Faction => world.factions.len(),
        }
        .max(1) as f64;
        let fires: Vec<f64> = runs.iter().map(|run| run.event_fires[id] as f64).collect();
        let mean = fires.iter().sum::<f64>() / count;
        let per_target_century = mean / targets / centuries;
        let present = fires.iter().filter(|&&value| value > 0.0).count() as f64 / count;
        let _ = writeln!(
            out,
            "  {:<34} {:>10.2} {:>12.2} {:>13.0}%",
            event.title,
            mean,
            per_target_century,
            present * 100.0
        );
        if mean == 0.0 {
            alerts.push(format!("'{}' nunca aconteceu", event.title));
        } else if per_target_century > FREQUENT_PER_TARGET_CENTURY {
            alerts.push(format!(
                "'{}' acontece {per_target_century:.1}x por alvo/século (limite {FREQUENT_PER_TARGET_CENTURY})",
                event.title
            ));
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "== Modificadores (alvo-meses ativos por partida) ==");
    let mut modifiers: Vec<(f64, &str)> = world
        .modifiers
        .iter()
        .enumerate()
        .map(|(id, modifier)| {
            let mean = runs
                .iter()
                .map(|run| run.modifier_months[id] as f64)
                .sum::<f64>()
                / count;
            (mean, modifier.name.as_str())
        })
        .collect();
    modifiers.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (mean, name) in &modifiers {
        let _ = writeln!(out, "  {name:<34} {mean:>10.1}");
    }

    if famine_share > FAMINE_ALERT_SHARE {
        alerts.push(format!(
            "fome em {:.1}% das províncias-mês (limite {:.0}%)",
            famine_share * 100.0,
            FAMINE_ALERT_SHARE * 100.0
        ));
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "== Alertas ==");
    if alerts.is_empty() {
        let _ = writeln!(out, "  nenhum");
    }
    for alert in alerts {
        let _ = writeln!(out, "  ! {alert}");
    }
    let _ = writeln!(
        out,
        "  (sem jogador: ritos e eventos que dependem de escolhas do jogador não aparecem)"
    );
    out
}

fn csv(world: &World, runs: &[RunStats]) -> String {
    let mut out = String::from(
        "seed,populacao_final,descontentamento_final,pico_descontentamento,fome_provincias_mes",
    );
    for faction in &world.factions {
        let _ = write!(
            out,
            ",{0} tesouro,{0} legitimidade,{0} fe_media,{0} magia_media",
            faction.name.replace(',', " ")
        );
    }
    for event in &world.events {
        let _ = write!(out, ",evento {}", event.key);
    }
    out.push('\n');
    for run in runs {
        let _ = write!(
            out,
            "{},{},{},{},{}",
            run.seed,
            run.final_population,
            run.final_average_unrest,
            run.peak_unrest,
            run.famine_province_months
        );
        for (index, faction) in run.factions.iter().enumerate() {
            let _ = write!(
                out,
                ",{},{},{},{}",
                faction.treasury,
                faction.legitimacy,
                run.average_faith[index],
                run.average_mana[index]
            );
        }
        for fires in &run.event_fires {
            let _ = write!(out, ",{fires}");
        }
        out.push('\n');
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            eprintln!("uso: crownveil-batch [--runs N] [--years N] [--seed N] [--csv arquivo]");
            std::process::exit(if message.is_empty() { 0 } else { 2 });
        }
    };

    let started = Instant::now();
    let runs = run_all(&options);
    let world = base_world(options.seed);

    println!(
        "Crownveil — balanceamento em lote: {} partidas x {} anos (seeds {}..={}), {:.1}s",
        options.runs,
        options.years,
        options.seed,
        options.seed + options.runs - 1,
        started.elapsed().as_secs_f64()
    );
    println!();
    print!("{}", report(&world, &runs, &options));

    if let Some(path) = &options.csv {
        match std::fs::write(path, csv(&world, &runs)) {
            Ok(()) => println!("\nCSV por partida gravado em {path}"),
            Err(error) => eprintln!("não foi possível gravar o CSV: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_flags_and_defaults() {
        assert_eq!(parse_args(&[]).unwrap(), Options::default());
        let options = parse_args(&args(&[
            "--runs", "5", "--years", "20", "--seed", "0", "--csv", "x.csv",
        ]))
        .unwrap();
        assert_eq!(
            options,
            Options {
                runs: 5,
                years: 20,
                seed: 0,
                csv: Some("x.csv".to_string()),
            }
        );
        assert!(parse_args(&args(&["--runs", "0"])).is_err());
        assert!(parse_args(&args(&["--years"])).is_err());
        assert!(parse_args(&args(&["--nada"])).is_err());
    }

    #[test]
    fn summary_percentiles() {
        let summary = summarize((1..=11).map(f64::from));
        assert_eq!(summary.mean, 6.0);
        assert_eq!(summary.median, 6.0);
        assert_eq!(summary.p10, 2.0);
        assert_eq!(summary.p90, 10.0);
    }

    #[test]
    fn batch_is_deterministic_and_counts_events() {
        let options = Options {
            runs: 3,
            years: 30,
            ..Options::default()
        };
        let first = run_all(&options);
        assert_eq!(first, run_all(&options));
        assert_eq!(
            first.iter().map(|run| run.seed).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(first
            .iter()
            .any(|run| run.event_fires.iter().sum::<u64>() > 0));
        assert_eq!(first[0], run_one(1, 30), "threads não mudam o resultado");
    }

    #[test]
    fn event_counts_match_the_log_when_it_is_not_truncated() {
        let stats = run_one(4, 20);
        let mut world = base_world(4);
        for _ in 0..20 * MONTHS_PER_YEAR {
            world.tick_month();
        }
        assert!(world.event_log.len() < crownveil_sim::event::EVENT_LOG_LIMIT);
        let mut expected = vec![0u64; world.events.len()];
        for record in &world.event_log {
            expected[record.event] += 1;
        }
        assert_eq!(stats.event_fires, expected);
    }

    #[test]
    fn report_and_csv_cover_every_event() {
        let options = Options {
            runs: 2,
            years: 10,
            ..Options::default()
        };
        let runs = run_all(&options);
        let world = base_world(1);
        let text = report(&world, &runs, &options);
        let table = csv(&world, &runs);
        for event in &world.events {
            assert!(text.contains(&event.title));
            assert!(table.contains(&event.key));
        }
        assert_eq!(table.lines().count(), 3);
    }
}
