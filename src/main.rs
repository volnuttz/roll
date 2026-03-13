use clap::Parser;
use rand::Rng;
use std::fmt;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    None,
    Advantage,
    Disadvantage,
}

#[derive(Debug, Clone)]
struct DiceGroup {
    count: u32,
    sides: u32,
}

#[derive(Debug, Clone)]
struct DiceExpr {
    modifier: Modifier,
    groups: Vec<DiceGroup>,
    flat_bonus: i64,
}

impl fmt::Display for DiceExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.modifier {
            Modifier::Advantage => write!(f, "adv ")?,
            Modifier::Disadvantage => write!(f, "dis ")?,
            Modifier::None => {}
        }
        for (i, g) in self.groups.iter().enumerate() {
            if i > 0 {
                write!(f, "+")?;
            }
            write!(f, "{}d{}", g.count, g.sides)?;
        }
        if self.flat_bonus > 0 {
            write!(f, "+{}", self.flat_bonus)?;
        } else if self.flat_bonus < 0 {
            write!(f, "{}", self.flat_bonus)?;
        }
        Ok(())
    }
}

fn parse_expr(input: &str) -> Result<DiceExpr, String> {
    let input = input.trim().to_lowercase();
    let (modifier, rest) = if let Some(r) = input.strip_prefix("adv") {
        (Modifier::Advantage, r.trim_start())
    } else if let Some(r) = input.strip_prefix("dis") {
        (Modifier::Disadvantage, r.trim_start())
    } else {
        (Modifier::None, input.as_str())
    };

    let mut groups = Vec::new();
    let mut flat_bonus: i64 = 0;

    let mut tokens: Vec<(i64, &str)> = Vec::new();
    let mut start = 0;
    let mut sign: i64 = 1;
    let bytes = rest.as_bytes();

    while start < bytes.len() && bytes[start] == b' ' {
        start += 1;
    }

    let mut i = start;
    while i <= bytes.len() {
        if i == bytes.len() || bytes[i] == b'+' || bytes[i] == b'-' {
            if i > start {
                let token = rest[start..i].trim();
                if !token.is_empty() {
                    tokens.push((sign, token));
                }
            }
            if i < bytes.len() {
                sign = if bytes[i] == b'-' { -1 } else { 1 };
            }
            start = i + 1;
        }
        i += 1;
    }

    for (s, token) in &tokens {
        if let Some(d_pos) = token.find('d') {
            let count_str = &token[..d_pos];
            let sides_str = &token[d_pos + 1..];
            let count: u32 = if count_str.is_empty() {
                1
            } else {
                count_str
                    .parse()
                    .map_err(|_| format!("invalid dice count: '{count_str}'"))?
            };
            let sides: u32 = sides_str
                .parse()
                .map_err(|_| format!("invalid sides: '{sides_str}'"))?;
            if *s == -1 {
                return Err("negative dice groups not supported".into());
            }
            groups.push(DiceGroup { count, sides });
        } else {
            let val: i64 = token
                .parse()
                .map_err(|_| format!("invalid token: '{token}'"))?;
            flat_bonus += s * val;
        }
    }

    if groups.is_empty() {
        return Err("no dice found in expression".into());
    }

    Ok(DiceExpr {
        modifier,
        groups,
        flat_bonus,
    })
}

fn roll_once(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, Vec<Vec<u32>>) {
    let mut total: i64 = 0;
    let mut all_rolls = Vec::new();
    for g in &expr.groups {
        let mut rolls = Vec::new();
        for _ in 0..g.count {
            let r = rng.gen_range(1..=g.sides);
            rolls.push(r);
            total += r as i64;
        }
        all_rolls.push(rolls);
    }
    total += expr.flat_bonus;
    (total, all_rolls)
}

fn roll_value(expr: &DiceExpr, rng: &mut impl Rng) -> i64 {
    match expr.modifier {
        Modifier::None => roll_once(expr, rng).0,
        Modifier::Advantage => {
            let a = roll_once(expr, rng).0;
            let b = roll_once(expr, rng).0;
            a.max(b)
        }
        Modifier::Disadvantage => {
            let a = roll_once(expr, rng).0;
            let b = roll_once(expr, rng).0;
            a.min(b)
        }
    }
}

fn roll_verbose(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, String) {
    match expr.modifier {
        Modifier::None => {
            let (total, rolls) = roll_once(expr, rng);
            (total, format_rolls(&rolls))
        }
        Modifier::Advantage => {
            let (a, rolls_a) = roll_once(expr, rng);
            let (b, rolls_b) = roll_once(expr, rng);
            if a >= b {
                (a, format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)))
            } else {
                (b, format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)))
            }
        }
        Modifier::Disadvantage => {
            let (a, rolls_a) = roll_once(expr, rng);
            let (b, rolls_b) = roll_once(expr, rng);
            if a <= b {
                (a, format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)))
            } else {
                (b, format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)))
            }
        }
    }
}

fn format_rolls(rolls: &[Vec<u32>]) -> String {
    rolls
        .iter()
        .map(|group| {
            let inner: Vec<String> = group.iter().map(|r| r.to_string()).collect();
            format!("[{}]", inner.join(", "))
        })
        .collect::<Vec<_>>()
        .join(" + ")
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
        let hits = (0..cli.sims)
            .filter(|_| roll_value(&expr, &mut rng) >= target)
            .count();
        let probability = hits as f64 / cli.sims as f64;
        println!(
            "P({} >= {}) = {:.2}% ({} / {} sims)",
            expr, target, probability * 100.0, hits, cli.sims
        );
    } else {
        let (result, detail) = roll_verbose(&expr, &mut rng);
        if expr.flat_bonus != 0 {
            println!("{} => {} ({:+}) = {}", expr, detail, expr.flat_bonus, result);
        } else {
            println!("{} => {} = {}", expr, detail, result);
        }
    }
}
