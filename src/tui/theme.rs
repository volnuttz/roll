use ratatui::style::{Color, Modifier, Style};

// ── Brand colors ─────────────────────────────────────────────────────────────

pub const ACCENT: Color = Color::Cyan;
pub const ACCENT_DIM: Color = Color::DarkGray;
pub const SUCCESS: Color = Color::Green;
pub const DANGER: Color = Color::Red;
pub const WARNING: Color = Color::Yellow;

// ── Composite styles ─────────────────────────────────────────────────────────

pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn input() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn result_total() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn result_detail() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn error() -> Style {
    Style::default().fg(DANGER)
}

pub fn keybinding_key() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn keybinding_desc() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn history_expr() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn history_result() -> Style {
    Style::default().fg(Color::White)
}

pub fn bar_chart() -> Style {
    Style::default().fg(ACCENT)
}

pub fn bar_highlight() -> Style {
    Style::default().fg(WARNING)
}

pub fn stats() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn nat_max() -> Style {
    Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)
}

pub fn nat_min() -> Style {
    Style::default().fg(DANGER).add_modifier(Modifier::BOLD)
}

/// Flash style for a fresh roll result (bright background that fades).
pub fn flash_result(age_ms: u128) -> Style {
    if age_ms < 150 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if age_ms < 350 {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        result_total()
    }
}

/// Flash border style for the Result block.
pub fn flash_border(age_ms: u128) -> Style {
    if age_ms < 350 {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(ACCENT_DIM)
    }
}

pub fn selected() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

pub fn preset_name() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn preset_expr() -> Style {
    Style::default().fg(Color::Gray)
}
