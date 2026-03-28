use clap::Parser;
use roll::{
    compute_distribution, estimate_probability, exact_probability, parse_expr, render_distribution,
    roll_stats, roll_verbose,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write as _;

#[derive(Parser)]
#[command(about = "Roll dice using TTRPG expressions like '2d10+4' or 'adv d20+5'")]
struct Cli {
    /// Dice expression, e.g. "2d10+4", "adv d20+5", "4d6kh3", or a saved preset name
    expression: Vec<String>,

    /// Show full probability distribution as a histogram
    #[arg(long, conflicts_with = "prob")]
    dist: bool,

    /// Calculate probability of rolling at least this value
    #[arg(long)]
    prob: Option<i64>,

    /// Roll the expression this many times
    #[arg(long, short = 'n', default_value_t = 1)]
    times: u32,

    /// Print theoretical min, max, and mean for the expression
    #[arg(long)]
    stats: bool,

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

fn handle_expression(
    input: &str,
    dist: bool,
    prob: Option<i64>,
    sims: u64,
    times: u32,
    show_stats: bool,
    rng: &mut impl rand::Rng,
) {
    let expr = match parse_expr(input) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            return;
        }
    };

    if dist {
        let counts = compute_distribution(&expr, sims, rng);
        let output = render_distribution(&expr, &counts, sims);
        print!("{output}");
    } else if let Some(target) = prob {
        if let Some(p) = exact_probability(&expr, target) {
            println!("P({expr} >= {target}) = {:.4}% (exact)", p * 100.0);
        } else {
            let probability = estimate_probability(&expr, target, sims, rng);
            let hits = (probability * sims as f64).round() as u64;
            println!(
                "P({expr} >= {target}) = {:.2}% ({hits} / {sims} sims)",
                probability * 100.0,
            );
        }
    } else {
        for i in 1..=times {
            let (result, detail) = roll_verbose(&expr, rng);
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

    if show_stats {
        let s = roll_stats(&expr);
        println!("  [min={}, max={}, mean={:.2}]", s.min, s.max, s.mean);
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
        handle_expression(line, false, None, sims_from_env(), 1, false, rng);
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
        cli.dist,
        cli.prob,
        sims,
        cli.times,
        cli.stats,
        &mut rng,
    );
}
