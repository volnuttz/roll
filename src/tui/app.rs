use roll::{
    DiceExpr, RollStats, compute_distribution, estimate_probability, exact_probability, parse_expr,
    roll_stats, roll_verbose,
};
use std::collections::BTreeMap;
use std::time::Instant;

// ── App mode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Roller,
    Distribution,
    Presets,
    Help,
}

// ── Roll history entry ───────────────────────────────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
pub struct RollEntry {
    pub expression: String,
    pub total: i64,
    pub breakdown: String,
    pub stats: RollStats,
    pub timestamp: Instant,
    /// The kept dice per group, for nat/min/max coloring.
    pub kept_dice: Vec<Vec<u32>>,
    /// The sides of each dice group (parallel to kept_dice).
    pub group_sides: Vec<u32>,
    /// Whether this was a "natural" max (all dice rolled their maximum).
    pub is_nat_max: bool,
    /// Whether this was a "natural" min (all dice rolled 1).
    pub is_nat_min: bool,
}

// ── Distribution data ────────────────────────────────────────────────────────

pub struct DistData {
    pub expr_str: String,
    pub counts: BTreeMap<i64, u64>,
    pub sims: u64,
    pub stats: RollStats,
    pub target: Option<i64>,
    pub target_prob: Option<f64>,
}

// ── Preset entry ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PresetEntry {
    pub name: String,
    pub expression: String,
}

// ── Application state ────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub prev_screen: Screen,
    pub input: String,
    pub cursor_pos: usize,
    pub roll_history: Vec<RollEntry>,
    pub input_history: Vec<String>,
    pub input_history_idx: Option<usize>,
    pub error_msg: Option<String>,
    pub should_quit: bool,

    // Distribution view
    pub dist: Option<DistData>,

    // Preset view
    pub presets: Vec<PresetEntry>,
    pub preset_selected: usize,
    pub preset_confirm_delete: bool,

    // History scroll
    pub history_scroll: usize,

    // Config
    pub sims: u64,
}

const MAX_HISTORY: usize = 50;

impl App {
    pub fn new(initial_expr: Option<&str>, sims: u64) -> Self {
        let mut app = Self {
            screen: Screen::Roller,
            prev_screen: Screen::Roller,
            input: String::new(),
            cursor_pos: 0,
            roll_history: Vec::new(),
            input_history: Vec::new(),
            input_history_idx: None,
            error_msg: None,
            should_quit: false,
            dist: None,
            presets: Vec::new(),
            preset_selected: 0,
            preset_confirm_delete: false,
            history_scroll: 0,
            sims,
        };
        if let Some(expr) = initial_expr {
            app.input = expr.to_string();
            app.cursor_pos = expr.len();
        }
        app
    }

    // ── Input editing ────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, c: char) {
        self.error_msg = None;
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    // ── Input history navigation ─────────────────────────────────────────────

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let idx = match self.input_history_idx {
            None => self.input_history.len() - 1,
            Some(0) => return,
            Some(i) => i - 1,
        };
        self.input_history_idx = Some(idx);
        self.input = self.input_history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    pub fn history_down(&mut self) {
        let Some(idx) = self.input_history_idx else {
            return;
        };
        if idx + 1 >= self.input_history.len() {
            self.input_history_idx = None;
            self.input.clear();
            self.cursor_pos = 0;
        } else {
            self.input_history_idx = Some(idx + 1);
            self.input = self.input_history[idx + 1].clone();
            self.cursor_pos = self.input.len();
        }
    }

    // ── Rolling ──────────────────────────────────────────────────────────────

    pub fn submit_roll(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Resolve preset names
        let resolved = self
            .presets
            .iter()
            .find(|p| p.name.to_lowercase() == input.to_lowercase())
            .map(|p| p.expression.clone())
            .unwrap_or_else(|| input.clone());

        let expr = match parse_expr(&resolved) {
            Ok(e) => e,
            Err(e) => {
                self.error_msg = Some(format!("{e}"));
                return;
            }
        };

        let mut rng = rand::rng();
        let (total, breakdown) = roll_verbose(&expr, &mut rng);
        let stats = roll_stats(&expr);

        // For nat detection, do an extra roll_once to get kept dice
        // (roll_verbose doesn't expose them). We use the breakdown from roll_verbose
        // for display, and parse the kept dice from a separate roll for coloring
        // of the *displayed* result. Since roll_verbose already consumed the roll,
        // we parse the kept dice from the breakdown string instead.
        let (kept_dice, group_sides) = parse_kept_from_breakdown(&breakdown, &expr);
        let is_nat_max = is_natural_max(&kept_dice, &group_sides);
        let is_nat_min = is_natural_min(&kept_dice);

        let entry = RollEntry {
            expression: resolved,
            total,
            breakdown,
            stats,
            timestamp: Instant::now(),
            kept_dice,
            group_sides,
            is_nat_max,
            is_nat_min,
        };

        self.roll_history.push(entry);
        if self.roll_history.len() > MAX_HISTORY {
            self.roll_history.remove(0);
        }

        // Push to input history (deduplicate consecutive)
        if self.input_history.last().map(|s| s.as_str()) != Some(&input) {
            self.input_history.push(input);
        }
        self.input_history_idx = None;

        self.input.clear();
        self.cursor_pos = 0;
        self.error_msg = None;
        self.history_scroll = 0;
    }

    // ── Distribution ─────────────────────────────────────────────────────────

    pub fn open_distribution(&mut self) {
        let input = self.input.trim().to_string();

        // Use last rolled expression if input is empty
        let expr_str = if input.is_empty() {
            match self.roll_history.last() {
                Some(entry) => entry.expression.clone(),
                None => {
                    self.error_msg = Some("Type an expression first".to_string());
                    return;
                }
            }
        } else {
            // Resolve preset
            self.presets
                .iter()
                .find(|p| p.name.to_lowercase() == input.to_lowercase())
                .map(|p| p.expression.clone())
                .unwrap_or(input)
        };

        let expr = match parse_expr(&expr_str) {
            Ok(e) => e,
            Err(e) => {
                self.error_msg = Some(format!("{e}"));
                return;
            }
        };

        let mut rng = rand::rng();
        let counts = compute_distribution(&expr, self.sims, &mut rng);
        let stats = roll_stats(&expr);

        self.dist = Some(DistData {
            expr_str,
            counts,
            sims: self.sims,
            stats,
            target: None,
            target_prob: None,
        });
        self.prev_screen = self.screen;
        self.screen = Screen::Distribution;
        self.error_msg = None;
    }

    pub fn dist_set_target(&mut self, target: i64) {
        if let Some(ref mut dist) = self.dist {
            let expr = parse_expr(&dist.expr_str).unwrap();
            let prob = exact_probability(&expr, target).unwrap_or_else(|| {
                let mut rng = rand::rng();
                estimate_probability(&expr, target, dist.sims, &mut rng)
            });
            dist.target = Some(target);
            dist.target_prob = Some(prob);
        }
    }

    pub fn dist_move_target(&mut self, delta: i64) {
        if let Some(ref dist) = self.dist {
            let current = dist
                .target
                .unwrap_or_else(|| dist.stats.min + (dist.stats.max - dist.stats.min) / 2);
            let new_target = (current + delta).clamp(dist.stats.min, dist.stats.max);
            self.dist_set_target(new_target);
        }
    }

    // ── Presets ──────────────────────────────────────────────────────────────

    pub fn open_presets(&mut self) {
        self.reload_presets();
        self.prev_screen = self.screen;
        self.screen = Screen::Presets;
        self.preset_confirm_delete = false;
        if self.preset_selected >= self.presets.len() {
            self.preset_selected = 0;
        }
    }

    pub fn reload_presets(&mut self) {
        let loaded = super::load_presets_list();
        self.presets = loaded;
    }

    pub fn preset_select_up(&mut self) {
        if self.preset_selected > 0 {
            self.preset_selected -= 1;
            self.preset_confirm_delete = false;
        }
    }

    pub fn preset_select_down(&mut self) {
        if !self.presets.is_empty() && self.preset_selected < self.presets.len() - 1 {
            self.preset_selected += 1;
            self.preset_confirm_delete = false;
        }
    }

    pub fn preset_roll_selected(&mut self) {
        if let Some(preset) = self.presets.get(self.preset_selected) {
            self.input = preset.expression.clone();
            self.cursor_pos = self.input.len();
            self.screen = Screen::Roller;
            self.submit_roll();
        }
    }

    pub fn preset_delete_selected(&mut self) {
        if self.preset_confirm_delete {
            if let Some(preset) = self.presets.get(self.preset_selected) {
                super::delete_preset(&preset.name);
                self.reload_presets();
                if self.preset_selected >= self.presets.len() && self.preset_selected > 0 {
                    self.preset_selected -= 1;
                }
            }
            self.preset_confirm_delete = false;
        } else {
            self.preset_confirm_delete = true;
        }
    }

    // ── Screen navigation ────────────────────────────────────────────────────

    pub fn go_back(&mut self) {
        self.screen = self.prev_screen;
        self.prev_screen = Screen::Roller;
        self.error_msg = None;
    }

    pub fn toggle_help(&mut self) {
        if self.screen == Screen::Help {
            self.go_back();
        } else {
            self.prev_screen = self.screen;
            self.screen = Screen::Help;
        }
    }

    // ── History scroll ───────────────────────────────────────────────────────

    pub fn scroll_history_up(&mut self) {
        if self.history_scroll + 1 < self.roll_history.len() {
            self.history_scroll += 1;
        }
    }

    pub fn scroll_history_down(&mut self) {
        if self.history_scroll > 0 {
            self.history_scroll -= 1;
        }
    }

    pub fn latest_roll(&self) -> Option<&RollEntry> {
        self.roll_history.last()
    }

    /// Returns how many milliseconds ago the latest roll was made (for flash animation).
    pub fn latest_roll_age_ms(&self) -> Option<u128> {
        self.roll_history
            .last()
            .map(|e| e.timestamp.elapsed().as_millis())
    }

    // ── Distribution sims control ────────────────────────────────────────────

    pub fn dist_increase_sims(&mut self) {
        if let Some(ref mut dist) = self.dist {
            dist.sims = next_sims_up(dist.sims);
            self.recompute_distribution();
        }
    }

    pub fn dist_decrease_sims(&mut self) {
        if let Some(ref mut dist) = self.dist {
            dist.sims = next_sims_down(dist.sims);
            self.recompute_distribution();
        }
    }

    fn recompute_distribution(&mut self) {
        if let Some(ref mut dist) = self.dist {
            let expr = match parse_expr(&dist.expr_str) {
                Ok(e) => e,
                Err(_) => return,
            };
            let mut rng = rand::rng();
            dist.counts = compute_distribution(&expr, dist.sims, &mut rng);
            // Recompute target probability if one was set
            if let Some(target) = dist.target {
                let prob = exact_probability(&expr, target)
                    .unwrap_or_else(|| estimate_probability(&expr, target, dist.sims, &mut rng));
                dist.target_prob = Some(prob);
            }
        }
    }
}

// ── Sims stepping ────────────────────────────────────────────────────────────

const SIMS_STEPS: &[u64] = &[
    10_000, 50_000, 100_000, 500_000, 1_000_000, 5_000_000, 10_000_000,
];

fn next_sims_up(current: u64) -> u64 {
    SIMS_STEPS
        .iter()
        .find(|&&s| s > current)
        .copied()
        .unwrap_or(*SIMS_STEPS.last().unwrap())
}

fn next_sims_down(current: u64) -> u64 {
    SIMS_STEPS
        .iter()
        .rev()
        .find(|&&s| s < current)
        .copied()
        .unwrap_or(SIMS_STEPS[0])
}

// ── Nat detection helpers ────────────────────────────────────────────────────

/// Parse kept dice values from the breakdown string produced by roll_verbose.
/// Breakdown looks like "[3, 5] + [2]" or "[17 vs 12]" for adv/dis.
fn parse_kept_from_breakdown(breakdown: &str, expr: &DiceExpr) -> (Vec<Vec<u32>>, Vec<u32>) {
    // For advantage/disadvantage with "vs", just parse the first set
    let working = if breakdown.contains(" vs ") {
        breakdown.split(" vs ").next().unwrap_or(breakdown)
    } else {
        breakdown
    };

    let mut kept_dice = Vec::new();
    let group_sides: Vec<u32> = expr.groups.iter().map(|g| g.sides).collect();

    // Split on " + " to get each group's "[x, y, z]"
    for part in working.split(" + ") {
        let trimmed = part.trim().trim_start_matches('[').trim_end_matches(']');
        let dice: Vec<u32> = trimmed
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        kept_dice.push(dice);
    }

    (kept_dice, group_sides)
}

fn is_natural_max(kept_dice: &[Vec<u32>], group_sides: &[u32]) -> bool {
    if kept_dice.is_empty() {
        return false;
    }
    kept_dice
        .iter()
        .zip(group_sides.iter())
        .all(|(dice, &sides)| !dice.is_empty() && dice.iter().all(|&d| d == sides))
}

fn is_natural_min(kept_dice: &[Vec<u32>]) -> bool {
    if kept_dice.is_empty() {
        return false;
    }
    kept_dice
        .iter()
        .all(|dice| !dice.is_empty() && dice.iter().all(|&d| d == 1))
}
