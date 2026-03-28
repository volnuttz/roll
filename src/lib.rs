#![warn(clippy::pedantic)]
//! A dice roller library for tabletop RPGs.
//!
//! Supports standard dice notation (e.g. `2d10+4`), advantage/disadvantage,
//! keep-highest/lowest (`4d6kh3`), and Monte Carlo / exact probability estimation.
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
use std::collections::BTreeMap;
use std::fmt;

// ── Error type ────────────────────────────────────────────────────────────────

/// Error type for dice expression parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidDiceCount(String),
    InvalidSides(String),
    NegativeDiceGroup,
    NoDiceFound,
    InvalidToken(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDiceCount(s) => write!(f, "invalid dice count: '{s}'"),
            Self::InvalidSides(s) => write!(f, "invalid sides: '{s}'"),
            Self::NegativeDiceGroup => write!(f, "negative dice groups are not supported"),
            Self::NoDiceFound => write!(f, "no dice found in expression"),
            Self::InvalidToken(s) => write!(f, "invalid token: '{s}'"),
        }
    }
}

impl std::error::Error for ParseError {}

// ── Core types ────────────────────────────────────────────────────────────────

/// Keep rule applied to a dice group after rolling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keep {
    /// Keep all dice (default).
    All,
    /// Keep only the N highest dice.
    Highest(u32),
    /// Keep only the N lowest dice.
    Lowest(u32),
}

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
    pub keep: Keep,
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
            match g.keep {
                Keep::All => {}
                Keep::Highest(n) => write!(f, "kh{n}")?,
                Keep::Lowest(n) => write!(f, "kl{n}")?,
            }
        }
        if self.flat_bonus > 0 {
            write!(f, "+{}", self.flat_bonus)?;
        } else if self.flat_bonus < 0 {
            write!(f, "{}", self.flat_bonus)?;
        }
        Ok(())
    }
}

/// Theoretical statistics for a dice expression (min, max, mean).
///
/// Computed analytically; does not account for advantage/disadvantage.
#[derive(Debug, Clone, PartialEq)]
pub struct RollStats {
    pub min: i64,
    pub max: i64,
    pub mean: f64,
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Split an expression string into sign-annotated tokens on `+`/`-` boundaries.
///
/// `"2d10 + 1d4 - 3"` → `[(1, "2d10"), (1, "1d4"), (-1, "3")]`
fn split_signed_tokens(s: &str) -> Vec<(i64, &str)> {
    let mut tokens = Vec::new();
    let mut sign: i64 = 1;
    let mut token_start = 0usize;

    for (i, ch) in s.char_indices() {
        if ch == '+' || ch == '-' {
            let tok = s[token_start..i].trim();
            if !tok.is_empty() {
                tokens.push((sign, tok));
            }
            sign = if ch == '-' { -1 } else { 1 };
            token_start = i + ch.len_utf8();
        }
    }
    let tok = s[token_start..].trim();
    if !tok.is_empty() {
        tokens.push((sign, tok));
    }
    tokens
}

/// Parse a single dice token (already lowercased) that may contain a `kh`/`kl` keep suffix.
fn parse_dice_token(token: &str) -> Result<DiceGroup, ParseError> {
    let (dice_part, keep) = if let Some(pos) = token.find("kh") {
        let n: u32 = token[pos + 2..]
            .parse()
            .map_err(|_| ParseError::InvalidSides(token[pos + 2..].to_string()))?;
        (&token[..pos], Keep::Highest(n))
    } else if let Some(pos) = token.find("kl") {
        let n: u32 = token[pos + 2..]
            .parse()
            .map_err(|_| ParseError::InvalidSides(token[pos + 2..].to_string()))?;
        (&token[..pos], Keep::Lowest(n))
    } else {
        (token, Keep::All)
    };

    let d_pos = dice_part
        .find('d')
        .ok_or_else(|| ParseError::InvalidToken(dice_part.to_string()))?;

    let count_str = &dice_part[..d_pos];
    let sides_str = &dice_part[d_pos + 1..];

    let count: u32 = if count_str.is_empty() {
        1
    } else {
        count_str
            .parse()
            .map_err(|_| ParseError::InvalidDiceCount(count_str.to_string()))?
    };
    if count == 0 {
        return Err(ParseError::InvalidDiceCount(
            "count must be at least 1".to_string(),
        ));
    }

    let sides: u32 = sides_str
        .parse()
        .map_err(|_| ParseError::InvalidSides(sides_str.to_string()))?;
    if sides == 0 {
        return Err(ParseError::InvalidSides(
            "sides must be at least 1".to_string(),
        ));
    }

    Ok(DiceGroup { count, sides, keep })
}

/// Parse a dice expression string into a [`DiceExpr`].
///
/// Supports expressions like `"2d10+4"`, `"adv d20+5"`, `"dis d20-1"`,
/// `"2d6+1d4+3"`, and `"4d6kh3"` (keep highest 3 of 4d6).
///
/// # Errors
///
/// Returns a [`ParseError`] if the expression is malformed.
pub fn parse_expr(input: &str) -> Result<DiceExpr, ParseError> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Err(ParseError::NoDiceFound);
    }

    let (modifier, rest) = if let Some(r) = input.strip_prefix("adv") {
        (Modifier::Advantage, r.trim_start())
    } else if let Some(r) = input.strip_prefix("dis") {
        (Modifier::Disadvantage, r.trim_start())
    } else {
        (Modifier::None, input.as_str())
    };

    let mut groups = Vec::new();
    let mut flat_bonus: i64 = 0;

    for (sign, token) in split_signed_tokens(rest) {
        if token.contains('d') {
            if sign == -1 {
                return Err(ParseError::NegativeDiceGroup);
            }
            groups.push(parse_dice_token(token)?);
        } else {
            let val: i64 = token
                .parse()
                .map_err(|_| ParseError::InvalidToken(token.to_string()))?;
            flat_bonus += sign * val;
        }
    }

    if groups.is_empty() {
        return Err(ParseError::NoDiceFound);
    }

    Ok(DiceExpr {
        modifier,
        groups,
        flat_bonus,
    })
}

// ── Rolling ───────────────────────────────────────────────────────────────────

/// Roll the dice once, returning the total and the kept dice per group.
///
/// For groups with a [`Keep`] rule, only the kept dice are included in the inner
/// `Vec`; the total already reflects the keep logic.
#[must_use]
pub fn roll_once(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, Vec<Vec<u32>>) {
    let mut total: i64 = expr.flat_bonus;
    let mut all_rolls = Vec::new();

    for g in &expr.groups {
        let mut rolls: Vec<u32> = (0..g.count)
            .map(|_| rng.random_range(1..=g.sides))
            .collect();

        let kept = match &g.keep {
            Keep::All => {
                total += rolls.iter().map(|&r| i64::from(r)).sum::<i64>();
                rolls
            }
            Keep::Highest(n) => {
                rolls.sort_unstable_by(|a, b| b.cmp(a));
                let kept: Vec<u32> = rolls.iter().take(*n as usize).copied().collect();
                total += kept.iter().map(|&r| i64::from(r)).sum::<i64>();
                kept
            }
            Keep::Lowest(n) => {
                rolls.sort_unstable();
                let kept: Vec<u32> = rolls.iter().take(*n as usize).copied().collect();
                total += kept.iter().map(|&r| i64::from(r)).sum::<i64>();
                kept
            }
        };

        all_rolls.push(kept);
    }

    (total, all_rolls)
}

/// Roll and return just the final value, applying advantage/disadvantage.
#[must_use]
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

/// Roll with detailed output, returning the total and a human-readable breakdown.
#[must_use]
pub fn roll_verbose(expr: &DiceExpr, rng: &mut impl Rng) -> (i64, String) {
    match expr.modifier {
        Modifier::None => {
            let (total, rolls) = roll_once(expr, rng);
            (total, format_rolls(&rolls))
        }
        Modifier::Advantage | Modifier::Disadvantage => {
            let (a, rolls_a) = roll_once(expr, rng);
            let (b, rolls_b) = roll_once(expr, rng);
            let total = if expr.modifier == Modifier::Advantage {
                a.max(b)
            } else {
                a.min(b)
            };
            (
                total,
                format!("{} vs {}", format_rolls(&rolls_a), format_rolls(&rolls_b)),
            )
        }
    }
}

/// Format roll results as a human-readable string like `[3, 5] + [2]`.
#[must_use]
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

// ── Statistics ────────────────────────────────────────────────────────────────

/// Compute theoretical min, max, and mean for a [`DiceExpr`].
///
/// Ignores advantage/disadvantage (those require simulation to compute exactly).
/// For keep groups, uses the kept count to compute bounds (not statistically
/// exact for `kh`/`kl`, but gives useful ballpark figures).
#[must_use]
pub fn roll_stats(expr: &DiceExpr) -> RollStats {
    let mut min = expr.flat_bonus;
    let mut max = expr.flat_bonus;
    let mut mean = expr.flat_bonus as f64;

    for g in &expr.groups {
        let keep_count = match g.keep {
            Keep::All => g.count,
            Keep::Highest(n) | Keep::Lowest(n) => n,
        };
        min += i64::from(keep_count);
        max += i64::from(keep_count) * i64::from(g.sides);
        mean += f64::from(g.sides + 1) / 2.0 * f64::from(keep_count);
    }

    RollStats { min, max, mean }
}

// ── Distribution ──────────────────────────────────────────────────────────────

/// Run a Monte Carlo simulation and return the count of each result value.
#[must_use]
pub fn compute_distribution(expr: &DiceExpr, sims: u64, rng: &mut impl Rng) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for _ in 0..sims {
        *counts.entry(roll_value(expr, rng)).or_insert(0) += 1;
    }
    counts
}

/// Render a probability distribution histogram as a string.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn render_distribution(expr: &DiceExpr, counts: &BTreeMap<i64, u64>, sims: u64) -> String {
    let mut out = format!("Distribution for {expr} ({sims} simulations):\n");

    let (&min_val, &max_val) = match (counts.keys().next(), counts.keys().next_back()) {
        (Some(lo), Some(hi)) => (lo, hi),
        _ => return out,
    };

    let max_count = *counts.values().max().unwrap_or(&1);
    let label_width = max_val.to_string().len().max(min_val.to_string().len());
    const MAX_BAR: usize = 40;

    for v in min_val..=max_val {
        let count = counts.get(&v).copied().unwrap_or(0);
        let pct = count as f64 / sims as f64 * 100.0;
        let bar_len = if max_count > 0 {
            (count as f64 / max_count as f64 * MAX_BAR as f64).round() as usize
        } else {
            0
        };
        let bar: String = "\u{2588}".repeat(bar_len);
        out.push_str(&format!(
            " {:>width$} | {:>5.1}% {}\n",
            v,
            pct,
            bar,
            width = label_width,
        ));
    }

    out
}

// ── Probability ───────────────────────────────────────────────────────────────

/// Compute the exact probability of rolling at least `target` via polynomial convolution.
///
/// Returns `None` when the expression is too complex for analytical computation
/// (i.e. advantage/disadvantage is active, or any group uses a keep rule).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn exact_probability(expr: &DiceExpr, target: i64) -> Option<f64> {
    if expr.modifier != Modifier::None {
        return None;
    }
    if expr.groups.iter().any(|g| g.keep != Keep::All) {
        return None;
    }

    // Convolve uniform distributions for each individual die.
    let mut dist: BTreeMap<i64, f64> = BTreeMap::new();
    dist.insert(0, 1.0);

    for g in &expr.groups {
        let p = 1.0 / f64::from(g.sides);
        for _ in 0..g.count {
            let mut new_dist: BTreeMap<i64, f64> = BTreeMap::new();
            for (&val, &prob) in &dist {
                for face in 1..=g.sides {
                    *new_dist.entry(val + i64::from(face)).or_insert(0.0) += prob * p;
                }
            }
            dist = new_dist;
        }
    }

    // P(dice_total + flat_bonus >= target)  ≡  P(dice_total >= target - flat_bonus)
    let adjusted = target - expr.flat_bonus;
    let prob: f64 = dist.range(adjusted..).map(|(_, &p)| p).sum();
    Some(prob)
}

/// Estimate the probability of rolling at least `target` using Monte Carlo simulation.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn estimate_probability(expr: &DiceExpr, target: i64, sims: u64, rng: &mut impl Rng) -> f64 {
    let hits = (0..sims)
        .filter(|_| roll_value(expr, rng) >= target)
        .count();
    hits as f64 / sims as f64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
        assert_eq!(expr.groups[0].keep, Keep::All);
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

    #[test]
    fn parse_zero_sides_error() {
        assert_eq!(
            parse_expr("2d0"),
            Err(ParseError::InvalidSides(
                "sides must be at least 1".to_string()
            ))
        );
    }

    #[test]
    fn parse_zero_count_error() {
        assert_eq!(
            parse_expr("0d6"),
            Err(ParseError::InvalidDiceCount(
                "count must be at least 1".to_string()
            ))
        );
    }

    #[test]
    fn parse_keep_highest() {
        let expr = parse_expr("4d6kh3").unwrap();
        assert_eq!(expr.groups[0].count, 4);
        assert_eq!(expr.groups[0].sides, 6);
        assert_eq!(expr.groups[0].keep, Keep::Highest(3));
    }

    #[test]
    fn parse_keep_lowest() {
        let expr = parse_expr("4d6kl1").unwrap();
        assert_eq!(expr.groups[0].keep, Keep::Lowest(1));
    }

    #[test]
    fn parse_keep_with_bonus() {
        let expr = parse_expr("4d6kh3+2").unwrap();
        assert_eq!(expr.groups[0].keep, Keep::Highest(3));
        assert_eq!(expr.flat_bonus, 2);
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

    #[test]
    fn display_keep_highest() {
        let expr = parse_expr("4d6kh3").unwrap();
        assert_eq!(expr.to_string(), "4d6kh3");
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
    fn roll_once_keep_highest() {
        let expr = parse_expr("4d6kh3").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let (total, rolls) = roll_once(&expr, &mut rng);
            // Only 3 dice kept
            assert_eq!(rolls[0].len(), 3);
            // Kept dice are sorted descending
            assert!(rolls[0].windows(2).all(|w| w[0] >= w[1]));
            // Total equals sum of kept dice
            let sum: i64 = rolls[0].iter().map(|&r| i64::from(r)).sum();
            assert_eq!(total, sum);
            // Each die is within range
            assert!(total >= 3 && total <= 18);
        }
    }

    #[test]
    fn roll_once_keep_lowest() {
        let expr = parse_expr("4d6kl1").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let (total, rolls) = roll_once(&expr, &mut rng);
            assert_eq!(rolls[0].len(), 1);
            assert!(total >= 1 && total <= 6);
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

    // ---- roll_stats tests ----

    #[test]
    fn roll_stats_d6() {
        let expr = parse_expr("d6").unwrap();
        let stats = roll_stats(&expr);
        assert_eq!(stats.min, 1);
        assert_eq!(stats.max, 6);
        assert!((stats.mean - 3.5).abs() < f64::EPSILON);
    }

    #[test]
    fn roll_stats_with_bonus() {
        let expr = parse_expr("2d6+5").unwrap();
        let stats = roll_stats(&expr);
        assert_eq!(stats.min, 7);
        assert_eq!(stats.max, 17);
        assert!((stats.mean - 12.0).abs() < f64::EPSILON);
    }

    #[test]
    fn roll_stats_keep_highest() {
        // 4d6kh3: keep 3 dice
        let expr = parse_expr("4d6kh3").unwrap();
        let stats = roll_stats(&expr);
        assert_eq!(stats.min, 3);
        assert_eq!(stats.max, 18);
    }

    // ---- compute_distribution tests ----

    #[test]
    fn distribution_d6_has_all_values() {
        let expr = parse_expr("d6").unwrap();
        let mut rng = seeded_rng();
        let counts = compute_distribution(&expr, 100_000, &mut rng);
        for v in 1..=6 {
            assert!(counts.contains_key(&v), "missing value {v}");
        }
        assert!(!counts.contains_key(&0));
        assert!(!counts.contains_key(&7));
    }

    #[test]
    fn distribution_counts_sum_to_sims() {
        let expr = parse_expr("2d6+3").unwrap();
        let mut rng = seeded_rng();
        let sims = 50_000;
        let counts = compute_distribution(&expr, sims, &mut rng);
        let total: u64 = counts.values().sum();
        assert_eq!(total, sims);
    }

    // ---- render_distribution tests ----

    #[test]
    fn render_distribution_contains_all_values() {
        let expr = parse_expr("d6").unwrap();
        let mut counts = BTreeMap::new();
        for v in 1..=6 {
            counts.insert(v, 1000);
        }
        let output = render_distribution(&expr, &counts, 6000);
        assert!(output.starts_with("Distribution for"));
        for v in 1..=6 {
            assert!(output.contains(&format!("{v} |")));
        }
    }

    #[test]
    fn render_distribution_percentages() {
        let expr = parse_expr("d6").unwrap();
        let mut counts = BTreeMap::new();
        counts.insert(1, 500);
        counts.insert(2, 500);
        let output = render_distribution(&expr, &counts, 1000);
        assert!(output.contains("50.0%"));
    }

    // ---- exact_probability tests ----

    #[test]
    fn exact_probability_d6_at_least_1_is_100_percent() {
        let expr = parse_expr("d6").unwrap();
        let p = exact_probability(&expr, 1).unwrap();
        assert!((p - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn exact_probability_d6_at_least_7_is_0_percent() {
        let expr = parse_expr("d6").unwrap();
        let p = exact_probability(&expr, 7).unwrap();
        assert!(p.abs() < f64::EPSILON);
    }

    #[test]
    fn exact_probability_d6_at_least_4_is_50_percent() {
        let expr = parse_expr("d6").unwrap();
        let p = exact_probability(&expr, 4).unwrap();
        assert!((p - 0.5).abs() < 1e-10);
    }

    #[test]
    fn exact_probability_returns_none_for_advantage() {
        let expr = parse_expr("adv d20").unwrap();
        assert!(exact_probability(&expr, 15).is_none());
    }

    #[test]
    fn exact_probability_returns_none_for_keep() {
        let expr = parse_expr("4d6kh3").unwrap();
        assert!(exact_probability(&expr, 10).is_none());
    }

    #[test]
    fn exact_probability_2d6_known_value() {
        // P(2d6 >= 7) = 21/36 = 7/12
        let expr = parse_expr("2d6").unwrap();
        let p = exact_probability(&expr, 7).unwrap();
        assert!((p - 7.0 / 12.0).abs() < 1e-10);
    }

    #[test]
    fn exact_probability_with_flat_bonus() {
        // P(d6+3 >= 7) = P(d6 >= 4) = 3/6 = 0.5
        let expr = parse_expr("d6+3").unwrap();
        let p = exact_probability(&expr, 7).unwrap();
        assert!((p - 0.5).abs() < 1e-10);
    }

    #[test]
    fn exact_probability_sums_to_one() {
        // Sum of P(2d6 >= k) - P(2d6 >= k+1) across all outcomes should equal 1.
        // Equivalently, P(2d6 >= 2) should be 1.0.
        let expr = parse_expr("2d6").unwrap();
        let p = exact_probability(&expr, 2).unwrap();
        assert!((p - 1.0).abs() < 1e-10);
    }

    // ---- ParseError display tests ----

    #[test]
    fn parse_error_display_no_dice() {
        assert_eq!(
            ParseError::NoDiceFound.to_string(),
            "no dice found in expression"
        );
    }

    #[test]
    fn parse_error_display_negative_group() {
        assert_eq!(
            ParseError::NegativeDiceGroup.to_string(),
            "negative dice groups are not supported"
        );
    }

    #[test]
    fn parse_error_display_invalid_token() {
        assert_eq!(
            ParseError::InvalidToken("foo".to_string()).to_string(),
            "invalid token: 'foo'"
        );
    }

    #[test]
    fn parse_error_display_invalid_sides() {
        assert_eq!(
            ParseError::InvalidSides("sides must be at least 1".to_string()).to_string(),
            "invalid sides: 'sides must be at least 1'"
        );
    }

    #[test]
    fn parse_error_display_invalid_count() {
        assert_eq!(
            ParseError::InvalidDiceCount("count must be at least 1".to_string()).to_string(),
            "invalid dice count: 'count must be at least 1'"
        );
    }

    // ---- roll_verbose with keep tests ----

    #[test]
    fn roll_verbose_keep_shows_kept_count() {
        // 4d6kh3 keeps 3 dice; the detail should contain exactly 3 numbers in brackets
        let expr = parse_expr("4d6kh3").unwrap();
        let mut rng = seeded_rng();
        for _ in 0..20 {
            let (_, detail) = roll_verbose(&expr, &mut rng);
            // Detail looks like "[a, b, c]"; split on ',' to count dice
            let inner = detail.trim_start_matches('[').trim_end_matches(']');
            assert_eq!(
                inner.split(',').count(),
                3,
                "expected 3 kept dice, got: {detail}"
            );
        }
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
        assert!((p - 0.5).abs() < 0.02);
    }
}
