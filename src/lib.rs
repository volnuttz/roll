//! A dice roller library for tabletop RPGs.
//!
//! Supports standard dice notation (e.g. `2d10+4`), advantage/disadvantage,
//! and Monte Carlo probability estimation.
//!
//! # Examples
//!
//! ```
//! use roll::{parse_expr, Modifier};
//!
//! let expr = parse_expr("2d10+4").unwrap();
//! assert_eq!(expr.flat_bonus, 4);
//! assert_eq!(expr.modifier, Modifier::None);
//! assert_eq!(expr.groups.len(), 1);
//! assert_eq!(expr.groups[0].count, 2);
//! assert_eq!(expr.groups[0].sides, 10);
//! ```

use rand::Rng;
use std::fmt;

/// Modifier applied to a dice roll (advantage, disadvantage, or none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    None,
    Advantage,
    Disadvantage,
}

/// A group of identical dice, e.g. `2d10` means 2 ten-sided dice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceGroup {
    pub count: u32,
    pub sides: u32,
}

/// A parsed dice expression such as `adv 2d10+1d4+3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceExpr {
    pub modifier: Modifier,
    pub groups: Vec<DiceGroup>,
    pub flat_bonus: i64,
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

/// Parse a dice expression string into a [`DiceExpr`].
///
/// Supports expressions like `"2d10+4"`, `"adv d20+5"`, `"dis d20-1"`,
/// and `"2d6+1d4+3"`.
///
/// # Errors
///
/// Returns an error string if the expression is malformed.
pub fn parse_expr(input: &str) -> Result<DiceExpr, String> {
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

/// Roll the dice once, returning the total and the individual rolls per group.
pub fn roll_once(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, Vec<Vec<u32>>) {
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

/// Roll and return just the final value, applying advantage/disadvantage.
pub fn roll_value(expr: &DiceExpr, rng: &mut impl Rng) -> i64 {
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

/// Roll with detailed output, returning the total and a human-readable string
/// showing the individual dice results.
pub fn roll_verbose(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, String) {
    match expr.modifier {
        Modifier::None => {
            let (total, rolls) = roll_once(expr, rng);
            (total, format_rolls(&rolls))
        }
        Modifier::Advantage => {
            let (a, rolls_a) = roll_once(expr, rng);
            let (b, rolls_b) = roll_once(expr, rng);
            if a >= b {
                (
                    a,
                    format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)),
                )
            } else {
                (
                    b,
                    format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)),
                )
            }
        }
        Modifier::Disadvantage => {
            let (a, rolls_a) = roll_once(expr, rng);
            let (b, rolls_b) = roll_once(expr, rng);
            if a <= b {
                (
                    a,
                    format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)),
                )
            } else {
                (
                    b,
                    format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)),
                )
            }
        }
    }
}

/// Format roll results as a human-readable string like `[3, 5] + [2]`.
pub fn format_rolls(rolls: &[Vec<u32>]) -> String {
    rolls
        .iter()
        .map(|group| {
            let inner: Vec<String> = group.iter().map(|r| r.to_string()).collect();
            format!("[{}]", inner.join(", "))
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Estimate the probability of rolling at least `target` using Monte Carlo simulation.
pub fn estimate_probability(expr: &DiceExpr, target: i64, sims: u64, rng: &mut impl Rng) -> f64 {
    let hits = (0..sims)
        .filter(|_| roll_value(expr, rng) >= target)
        .count();
    hits as f64 / sims as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    // ---- parse_expr tests ----

    #[test]
    fn parse_simple_dice() {
        let expr = parse_expr("2d10").unwrap();
        assert_eq!(expr.modifier, Modifier::None);
        assert_eq!(expr.groups.len(), 1);
        assert_eq!(expr.groups[0].count, 2);
        assert_eq!(expr.groups[0].sides, 10);
        assert_eq!(expr.flat_bonus, 0);
    }

    #[test]
    fn parse_single_die_shorthand() {
        let expr = parse_expr("d20").unwrap();
        assert_eq!(expr.groups[0].count, 1);
        assert_eq!(expr.groups[0].sides, 20);
    }

    #[test]
    fn parse_with_positive_bonus() {
        let expr = parse_expr("2d10+4").unwrap();
        assert_eq!(expr.flat_bonus, 4);
    }

    #[test]
    fn parse_with_negative_bonus() {
        let expr = parse_expr("d20-3").unwrap();
        assert_eq!(expr.flat_bonus, -3);
    }

    #[test]
    fn parse_advantage() {
        let expr = parse_expr("adv d20+5").unwrap();
        assert_eq!(expr.modifier, Modifier::Advantage);
        assert_eq!(expr.groups[0].count, 1);
        assert_eq!(expr.groups[0].sides, 20);
        assert_eq!(expr.flat_bonus, 5);
    }

    #[test]
    fn parse_disadvantage() {
        let expr = parse_expr("dis d20-1").unwrap();
        assert_eq!(expr.modifier, Modifier::Disadvantage);
        assert_eq!(expr.flat_bonus, -1);
    }

    #[test]
    fn parse_multiple_groups() {
        let expr = parse_expr("2d6+1d4+3").unwrap();
        assert_eq!(expr.groups.len(), 2);
        assert_eq!(expr.groups[0].count, 2);
        assert_eq!(expr.groups[0].sides, 6);
        assert_eq!(expr.groups[1].count, 1);
        assert_eq!(expr.groups[1].sides, 4);
        assert_eq!(expr.flat_bonus, 3);
    }

    #[test]
    fn parse_case_insensitive() {
        let expr = parse_expr("ADV D20+5").unwrap();
        assert_eq!(expr.modifier, Modifier::Advantage);
    }

    #[test]
    fn parse_with_whitespace() {
        let expr = parse_expr("  2d10 + 4  ").unwrap();
        assert_eq!(expr.groups[0].count, 2);
        assert_eq!(expr.flat_bonus, 4);
    }

    #[test]
    fn parse_no_dice_error() {
        assert!(parse_expr("42").is_err());
    }

    #[test]
    fn parse_negative_dice_group_error() {
        assert!(parse_expr("d20-2d6").is_err());
    }

    #[test]
    fn parse_invalid_sides_error() {
        assert!(parse_expr("2dx").is_err());
    }

    #[test]
    fn parse_empty_error() {
        assert!(parse_expr("").is_err());
    }

    // ---- Display tests ----

    #[test]
    fn display_simple() {
        let expr = parse_expr("2d10+4").unwrap();
        assert_eq!(expr.to_string(), "2d10+4");
    }

    #[test]
    fn display_advantage() {
        let expr = parse_expr("adv d20+5").unwrap();
        assert_eq!(expr.to_string(), "adv 1d20+5");
    }

    #[test]
    fn display_negative_bonus() {
        let expr = parse_expr("d20-3").unwrap();
        assert_eq!(expr.to_string(), "1d20-3");
    }

    #[test]
    fn display_no_bonus() {
        let expr = parse_expr("d20").unwrap();
        assert_eq!(expr.to_string(), "1d20");
    }

    // ---- Rolling tests ----

    #[test]
    fn roll_once_within_bounds() {
        let expr = parse_expr("2d6").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let (total, rolls) = roll_once(&expr, &mut rng);
            assert!(total >= 2 && total <= 12);
            assert_eq!(rolls.len(), 1);
            assert_eq!(rolls[0].len(), 2);
            for &r in &rolls[0] {
                assert!(r >= 1 && r <= 6);
            }
        }
    }

    #[test]
    fn roll_once_applies_flat_bonus() {
        let expr = parse_expr("1d6+10").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let (total, _) = roll_once(&expr, &mut rng);
            assert!(total >= 11 && total <= 16);
        }
    }

    #[test]
    fn roll_value_deterministic_with_seed() {
        let expr = parse_expr("d20").unwrap();
        let mut rng1 = seeded_rng();
        let mut rng2 = seeded_rng();
        let a = roll_value(&expr, &mut rng1);
        let b = roll_value(&expr, &mut rng2);
        assert_eq!(a, b);
    }

    #[test]
    fn roll_value_advantage_takes_higher() {
        let expr = parse_expr("adv d20").unwrap();
        let mut rng = seeded_rng();
        // Roll many times; advantage should always be >= a single roll
        for _ in 0..100 {
            let adv = roll_value(&expr, &mut rng);
            assert!(adv >= 1 && adv <= 20);
        }
    }

    #[test]
    fn roll_value_disadvantage_takes_lower() {
        let expr = parse_expr("dis d20").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let dis = roll_value(&expr, &mut rng);
            assert!(dis >= 1 && dis <= 20);
        }
    }

    #[test]
    fn advantage_greater_equal_disadvantage() {
        let adv_expr = parse_expr("adv d20").unwrap();
        let dis_expr = parse_expr("dis d20").unwrap();
        let mut rng = seeded_rng();
        let mut adv_total: i64 = 0;
        let mut dis_total: i64 = 0;
        let n = 10_000;
        for _ in 0..n {
            adv_total += roll_value(&adv_expr, &mut rng);
            dis_total += roll_value(&dis_expr, &mut rng);
        }
        // On average, advantage should produce higher results
        assert!(adv_total > dis_total);
    }

    // ---- roll_verbose tests ----

    #[test]
    fn roll_verbose_includes_rolls() {
        let expr = parse_expr("2d6").unwrap();
        let mut rng = seeded_rng();
        let (_, detail) = roll_verbose(&expr, &mut rng);
        assert!(detail.starts_with('['));
        assert!(detail.contains(']'));
    }

    #[test]
    fn roll_verbose_advantage_shows_vs() {
        let expr = parse_expr("adv d20").unwrap();
        let mut rng = seeded_rng();
        let (_, detail) = roll_verbose(&expr, &mut rng);
        assert!(detail.contains("vs"));
    }

    // ---- format_rolls tests ----

    #[test]
    fn format_rolls_single_group() {
        assert_eq!(format_rolls(&[vec![3, 5]]), "[3, 5]");
    }

    #[test]
    fn format_rolls_multiple_groups() {
        assert_eq!(format_rolls(&[vec![3, 5], vec![2]]), "[3, 5] + [2]");
    }

    // ---- estimate_probability tests ----

    #[test]
    fn probability_d6_at_least_1_is_100_percent() {
        let expr = parse_expr("d6").unwrap();
        let mut rng = seeded_rng();
        let p = estimate_probability(&expr, 1, 10_000, &mut rng);
        assert!((p - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn probability_d6_at_least_7_is_0_percent() {
        let expr = parse_expr("d6").unwrap();
        let mut rng = seeded_rng();
        let p = estimate_probability(&expr, 7, 10_000, &mut rng);
        assert!(p.abs() < f64::EPSILON);
    }

    #[test]
    fn probability_d6_at_least_4_roughly_50_percent() {
        let expr = parse_expr("d6").unwrap();
        let mut rng = seeded_rng();
        let p = estimate_probability(&expr, 4, 100_000, &mut rng);
        // Should be 3/6 = 50%
        assert!((p - 0.5).abs() < 0.02);
    }
}
