mod tui;

use clap::Parser;
use roll::{
    ProbabilityQuery, compute_distribution, estimate_query_probability, exact_query_probability,
    parse_expr, render_distribution, roll_stats, roll_verbose,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::Write as _;

#[derive(Parser)]
#[command(about = "Roll dice using TTRPG expressions like '2d10+4' or 'adv d20+5'")]
struct Cli {
    /// Dice expression, e.g. "2d10+4", "adv d20+5", "4d6kh3", or a saved preset name
    expression: Vec<String>,

    /// Show full probability distribution as a histogram
    #[arg(long, conflicts_with_all = ["prob_ge", "prob_eq", "prob_le", "prob_range"])]
    dist: bool,

    /// Calculate probability of rolling at least this value
    #[arg(long, conflicts_with_all = ["prob_eq", "prob_le", "prob_range"])]
    prob_ge: Option<i64>,

    /// Calculate probability of rolling exactly this value
    #[arg(long, conflicts_with_all = ["prob_ge", "prob_le", "prob_range"])]
    prob_eq: Option<i64>,

    /// Calculate probability of rolling at most this value
    #[arg(long, conflicts_with_all = ["prob_ge", "prob_eq", "prob_range"])]
    prob_le: Option<i64>,

    /// Calculate probability of rolling within an inclusive MIN..MAX range
    #[arg(long, value_name = "MIN..MAX", value_parser = parse_probability_range, conflicts_with_all = ["prob_ge", "prob_eq", "prob_le"])]
    prob_range: Option<(i64, i64)>,

    /// Roll the expression this many times
    #[arg(long, short = 'n', default_value_t = 1)]
    times: u32,

    /// Print theoretical min, max, and mean for the expression
    #[arg(long)]
    stats: bool,

    /// Emit machine-readable JSON
    #[arg(long)]
    json: bool,

    /// Enter interactive REPL mode (type expressions, 'quit' to exit)
    #[arg(long)]
    repl: bool,

    /// Save the expression as a named preset
    #[arg(long, value_name = "NAME")]
    save: Option<String>,

    /// Delete a named preset
    #[arg(long, value_name = "NAME")]
    delete: Option<String>,

    /// List all saved presets
    #[arg(long)]
    list: bool,

    /// Launch interactive TUI mode
    #[arg(long, short = 't')]
    tui: bool,
}

fn parse_probability_range(value: &str) -> Result<(i64, i64), String> {
    let (min, max) = value
        .split_once("..")
        .ok_or_else(|| "range must use MIN..MAX syntax".to_string())?;
    let min = min
        .parse::<i64>()
        .map_err(|_| format!("invalid range minimum: '{min}'"))?;
    let max = max
        .parse::<i64>()
        .map_err(|_| format!("invalid range maximum: '{max}'"))?;
    if min > max {
        return Err("range minimum must not exceed maximum".to_string());
    }
    Ok((min, max))
}

// ── Presets ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct Presets {
    #[serde(default)]
    presets: HashMap<String, String>,
}

fn presets_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("roll")
        .join("presets.toml")
}

fn load_presets() -> Presets {
    let path = presets_path();
    if !path.exists() {
        return Presets::default();
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

fn save_presets(presets: &Presets) -> Result<(), String> {
    let path = presets_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = toml::to_string(presets).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Roll handling ─────────────────────────────────────────────────────────────

struct HandleOptions {
    dist: bool,
    query: Option<ProbabilityQuery>,
    sims: u64,
    times: u32,
    show_stats: bool,
    json_output: bool,
}

fn handle_expression(input: &str, options: HandleOptions, rng: &mut impl rand::Rng) {
    let HandleOptions {
        dist,
        query,
        sims,
        times,
        show_stats,
        json_output,
    } = options;
    let expr = match parse_expr(input) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            return;
        }
    };

    if dist {
        let counts = compute_distribution(&expr, sims, rng);
        if json_output {
            println!(
                "{}",
                json!({
                    "expression": expr.to_string(),
                    "mode": "distribution",
                    "simulations": sims,
                    "counts": counts,
                    "stats": show_stats.then(|| roll_stats(&expr)),
                })
            );
        } else {
            print!("{}", render_distribution(&expr, &counts, sims));
        }
    } else if let Some(query) = query {
        let exact = exact_query_probability(&expr, query);
        let probability =
            exact.unwrap_or_else(|| estimate_query_probability(&expr, query, sims, rng));
        let (operator, bounds) = query_description(query);
        if json_output {
            println!(
                "{}",
                json!({
                    "expression": expr.to_string(),
                    "mode": "probability",
                    "query": { "operator": operator, "bounds": bounds },
                    "probability": probability,
                    "percent": probability * 100.0,
                    "exact": exact.is_some(),
                    "simulations": exact.is_none().then_some(sims),
                    "stats": show_stats.then(|| roll_stats(&expr)),
                })
            );
        } else if exact.is_some() {
            println!(
                "P({expr} {operator} {}) = {:.4}% (exact)",
                format_bounds(&bounds),
                probability * 100.0
            );
        } else {
            let hits = (probability * sims as f64).round() as u64;
            println!(
                "P({expr} {operator} {}) = {:.2}% ({hits} / {sims} sims)",
                format_bounds(&bounds),
                probability * 100.0,
            );
        }
    } else {
        let mut rolls = Vec::with_capacity(times as usize);
        for i in 1..=times {
            let (result, detail) = roll_verbose(&expr, rng);
            if json_output {
                rolls.push(json!({ "number": i, "total": result, "breakdown": detail }));
            } else {
                if times > 1 {
                    print!("#{i}: ");
                }
                if expr.flat_bonus != 0 {
                    println!("{expr} => {detail} ({:+}) = {result}", expr.flat_bonus);
                } else {
                    println!("{expr} => {detail} = {result}");
                }
            }
        }
        if json_output {
            println!(
                "{}",
                json!({
                    "expression": expr.to_string(),
                    "mode": "roll",
                    "flat_bonus": expr.flat_bonus,
                    "rolls": rolls,
                    "stats": show_stats.then(|| roll_stats(&expr)),
                })
            );
        }
    }

    if show_stats && !json_output {
        let s = roll_stats(&expr);
        println!("  [min={}, max={}, mean={:.2}]", s.min, s.max, s.mean);
    }
}

fn query_description(query: ProbabilityQuery) -> (&'static str, Vec<i64>) {
    match query {
        ProbabilityQuery::AtLeast(target) => (">=", vec![target]),
        ProbabilityQuery::AtMost(target) => ("<=", vec![target]),
        ProbabilityQuery::Equal(target) => ("==", vec![target]),
        ProbabilityQuery::InclusiveRange(min, max) => ("in", vec![min, max]),
    }
}

fn format_bounds(bounds: &[i64]) -> String {
    match bounds {
        [value] => value.to_string(),
        [min, max] => format!("{min}..{max}"),
        _ => String::new(),
    }
}

// ── REPL ──────────────────────────────────────────────────────────────────────

fn run_repl(rng: &mut impl rand::Rng) {
    println!("Roll REPL — type a dice expression or 'quit' to exit.");
    let stdin = std::io::stdin();
    loop {
        print!("> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "quit" || line == "exit" {
            break;
        }
        handle_expression(
            line,
            HandleOptions {
                dist: false,
                query: None,
                sims: sims_from_env(),
                times: 1,
                show_stats: false,
                json_output: false,
            },
            rng,
        );
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn sims_from_env() -> u64 {
    std::env::var("SIMS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000)
}

fn main() {
    let cli = Cli::parse();
    let mut rng = rand::rng();
    let sims = sims_from_env();
    let query = cli
        .prob_ge
        .map(ProbabilityQuery::AtLeast)
        .or_else(|| cli.prob_eq.map(ProbabilityQuery::Equal))
        .or_else(|| cli.prob_le.map(ProbabilityQuery::AtMost))
        .or_else(|| {
            cli.prob_range
                .map(|(min, max)| ProbabilityQuery::InclusiveRange(min, max))
        });

    // -- Preset management (no expression needed) --

    if cli.list {
        let presets = load_presets();
        if presets.presets.is_empty() {
            println!("No presets saved.");
        } else {
            println!("Saved presets:");
            let mut sorted: Vec<_> = presets.presets.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());
            for (name, expr) in sorted {
                println!("  {name} = {expr}");
            }
        }
        return;
    }

    if let Some(ref name) = cli.delete {
        let mut presets = load_presets();
        if presets.presets.remove(name).is_some() {
            if let Err(e) = save_presets(&presets) {
                eprintln!("Error saving presets: {e}");
                std::process::exit(1);
            }
            println!("Deleted preset '{name}'.");
        } else {
            eprintln!("No preset named '{name}'.");
            std::process::exit(1);
        }
        return;
    }

    // -- REPL mode --

    if cli.repl {
        run_repl(&mut rng);
        return;
    }

    let input = cli.expression.join(" ");

    // -- TUI mode --

    if cli.tui {
        let expr = if input.is_empty() {
            None
        } else {
            Some(input.as_str())
        };
        if let Err(e) = tui::run(expr, sims) {
            eprintln!("TUI error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // -- Save preset --

    if let Some(ref name) = cli.save {
        if input.is_empty() {
            eprintln!("Provide a dice expression to save.");
            std::process::exit(1);
        }
        if let Err(e) = parse_expr(&input) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        let mut presets = load_presets();
        presets.presets.insert(name.clone(), input.clone());
        if let Err(e) = save_presets(&presets) {
            eprintln!("Error saving presets: {e}");
            std::process::exit(1);
        }
        println!("Saved preset '{name}' = '{input}'.");
        return;
    }

    if input.is_empty() {
        eprintln!("Provide a dice expression or preset name. Use --help for usage.");
        std::process::exit(1);
    }

    // Resolve preset names (case-insensitive)
    let resolved = {
        let presets = load_presets();
        presets
            .presets
            .get(&input.to_lowercase())
            .cloned()
            .unwrap_or_else(|| input.clone())
    };

    handle_expression(
        &resolved,
        HandleOptions {
            dist: cli.dist,
            query,
            sims,
            times: cli.times,
            show_stats: cli.stats,
            json_output: cli.json,
        },
        &mut rng,
    );
}
