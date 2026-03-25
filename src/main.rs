use clap::Parser;
use roll::{estimate_probability, parse_expr, roll_verbose};

#[derive(Parser)]
#[command(about = "Roll dice using TTRPG expressions like '2d10+4' or 'adv d20+5'")]
struct Cli {
    /// Dice expression, e.g. "2d10+4", "adv d20+5", "dis d20-1"
    expression: Vec<String>,

    /// Calculate probability of rolling at least this value (Monte Carlo)
    #[arg(long)]
    prob: Option<i64>,

    /// Number of Monte Carlo simulations
    #[arg(long, default_value_t = 1_000_000)]
    sims: u64,
}

fn main() {
    let cli = Cli::parse();
    let input = cli.expression.join(" ");

    let expr = match parse_expr(&input) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let mut rng = rand::thread_rng();

    if let Some(target) = cli.prob {
        let probability = estimate_probability(&expr, target, cli.sims, &mut rng);
        let hits = (probability * cli.sims as f64).round() as u64;
        println!(
            "P({} >= {}) = {:.2}% ({} / {} sims)",
            expr,
            target,
            probability * 100.0,
            hits,
            cli.sims
        );
    } else {
        let (result, detail) = roll_verbose(&expr, &mut rng);
        if expr.flat_bonus != 0 {
            println!(
                "{} => {} ({:+}) = {}",
                expr, detail, expr.flat_bonus, result
            );
        } else {
            println!("{} => {} = {}", expr, detail, result);
        }
    }
}
