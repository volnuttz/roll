use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
};

use super::app::{App, RollerFocus, Screen};
use super::theme;

/// Main draw dispatch.
pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Roller => draw_roller(f, app),
        Screen::History => draw_history_screen(f, app),
        Screen::Help => {
            // Draw the previous screen underneath, then overlay help
            match app.prev_screen {
                Screen::Roller => draw_roller(f, app),
                Screen::History => draw_history_screen(f, app),
                Screen::Help => draw_roller(f, app),
            }
            draw_help_overlay(f, app);
        }
    }
}

// ── Tab bar ─────────────────────────────────────────────────────────────────

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let roller_style = if app.screen == Screen::Roller {
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ratatui::style::Color::DarkGray)
    };
    let history_style = if app.screen == Screen::History {
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ratatui::style::Color::DarkGray)
    };

    let tabs = Line::from(vec![
        Span::styled(" [", Style::default().fg(ratatui::style::Color::DarkGray)),
        Span::styled("Roller", roller_style),
        Span::styled("] [", Style::default().fg(ratatui::style::Color::DarkGray)),
        Span::styled("History", history_style),
        Span::styled("]", Style::default().fg(ratatui::style::Color::DarkGray)),
        Span::styled(
            "  roll - dice roller",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ),
    ]);
    let paragraph = Paragraph::new(tabs);
    f.render_widget(paragraph, area);
}

// ── Roller screen ───────────────────────────────────────────────────────────

fn draw_roller(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Length(7), // input + presets row
            Constraint::Length(5), // result
            Constraint::Min(5),    // distribution
            Constraint::Length(1), // footer
        ])
        .split(area);

    draw_tab_bar(f, app, chunks[0]);
    draw_input_presets_row(f, app, chunks[1]);
    draw_result(f, app, chunks[2]);
    draw_inline_distribution(f, app, chunks[3]);
    draw_roller_footer(f, app, chunks[4]);
}

fn draw_input_presets_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // input
            Constraint::Percentage(40), // presets
        ])
        .split(area);

    draw_input(f, app, cols[0]);
    draw_presets_sidebar(f, app, cols[1]);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let display = if app.input.is_empty() {
        vec![
            Span::styled("> ", theme::input()),
            Span::styled(
                "type expression (e.g. 2d20+4)...",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
        ]
    } else {
        vec![
            Span::styled("> ", theme::input()),
            Span::styled(&app.input, theme::input()),
        ]
    };

    let focus_border = if app.roller_focus == RollerFocus::Input {
        Style::default().fg(theme::ACCENT)
    } else {
        Style::default().fg(theme::ACCENT_DIM)
    };

    let block = Block::default()
        .title(Span::styled(" Expression ", theme::title()))
        .borders(Borders::ALL)
        .border_style(focus_border);

    let paragraph = Paragraph::new(Line::from(display)).block(block);
    f.render_widget(paragraph, area);

    // Place cursor only when input is focused
    if app.roller_focus == RollerFocus::Input {
        let cursor_x = area.x + 2 + app.cursor_pos as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_presets_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let focus_border = if app.roller_focus == RollerFocus::Presets {
        Style::default().fg(theme::ACCENT)
    } else {
        Style::default().fg(theme::ACCENT_DIM)
    };

    let block = Block::default()
        .title(Span::styled(" Presets (F2) ", theme::title()))
        .borders(Borders::ALL)
        .border_style(focus_border);

    if app.presets.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            " No presets",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )))
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .presets
        .iter()
        .enumerate()
        .map(|(i, preset)| {
            let is_selected = app.roller_focus == RollerFocus::Presets && i == app.preset_selected;
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                theme::selected()
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::styled(marker, style),
                Span::styled(&preset.name, theme::preset_name()),
                Span::styled(" ", Style::default()),
                Span::styled(&preset.expression, theme::preset_expr()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_result(f: &mut Frame, app: &App, area: Rect) {
    let age_ms = app.latest_roll_age_ms().unwrap_or(1000);

    let border_style = if app.error_msg.is_some() {
        Style::default().fg(theme::DANGER)
    } else {
        theme::flash_border(age_ms)
    };

    let block = Block::default()
        .title(Span::styled(" Result ", theme::title()))
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(ref err) = app.error_msg {
        vec![Line::from(Span::styled(
            format!("Error: {err}"),
            theme::error(),
        ))]
    } else if let Some(entry) = app.latest_roll() {
        // Choose total style: nat max (green), nat min (red), flash, or normal
        let total_style = if entry.is_nat_max {
            theme::nat_max()
        } else if entry.is_nat_min {
            theme::nat_min()
        } else {
            theme::flash_result(age_ms)
        };

        // Build the total display with optional nat label
        let total_str = if entry.is_nat_max {
            format!("{} NAT MAX!", entry.total)
        } else if entry.is_nat_min {
            format!("{} NAT MIN", entry.total)
        } else {
            entry.total.to_string()
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(&entry.expression, theme::history_expr()),
            Span::styled(" => ", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(&entry.breakdown, theme::result_detail()),
            Span::styled(" = ", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(total_str, total_style),
        ])];
        lines.push(Line::from(Span::styled(
            format!(
                "min={}  max={}  mean={:.2}",
                entry.stats.min, entry.stats.max, entry.stats.mean
            ),
            theme::stats(),
        )));
        lines
    } else {
        vec![Line::from(Span::styled(
            "Press Enter to roll",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
}

fn draw_inline_distribution(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Distribution ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    let Some(ref dist) = app.dist else {
        let empty = Paragraph::new(Line::from(Span::styled(
            "Roll an expression to see distribution",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )))
        .block(block);
        f.render_widget(empty, area);
        return;
    };

    let total: u64 = dist.counts.values().sum();
    let bars: Vec<Bar> = dist
        .counts
        .iter()
        .map(|(&value, &count)| {
            let pct = if total > 0 {
                (count as f64 / total as f64 * 100.0) as u64
            } else {
                0
            };
            Bar::default()
                .value(pct)
                .label(Line::from(value.to_string()))
                .style(theme::bar_chart())
                .text_value(format!("{:.1}%", count as f64 / total as f64 * 100.0))
        })
        .collect();

    let bar_count = bars.len() as u16;
    let chart = BarChart::default()
        .block(block)
        .data(BarGroup::default().bars(&bars))
        .bar_width(((area.width.saturating_sub(2)) / bar_count.max(1)).clamp(1, 5))
        .bar_gap(1);

    f.render_widget(chart, area);
}

fn draw_roller_footer(f: &mut Frame, app: &App, area: Rect) {
    let keys = if app.roller_focus == RollerFocus::Presets {
        Line::from(vec![
            Span::styled("[Enter]", theme::keybinding_key()),
            Span::styled(" Roll  ", theme::keybinding_desc()),
            Span::styled("[j/k]", theme::keybinding_key()),
            Span::styled(" Navigate  ", theme::keybinding_desc()),
            Span::styled("[d]", theme::keybinding_key()),
            Span::styled(" Delete  ", theme::keybinding_desc()),
            Span::styled("[Esc/F2]", theme::keybinding_key()),
            Span::styled(" Back  ", theme::keybinding_desc()),
            Span::styled("[Tab]", theme::keybinding_key()),
            Span::styled(" History", theme::keybinding_desc()),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter]", theme::keybinding_key()),
            Span::styled(" Roll  ", theme::keybinding_desc()),
            Span::styled("[Tab]", theme::keybinding_key()),
            Span::styled(" History  ", theme::keybinding_desc()),
            Span::styled("[F2]", theme::keybinding_key()),
            Span::styled(" Presets  ", theme::keybinding_desc()),
            Span::styled("[F1]", theme::keybinding_key()),
            Span::styled(" Help  ", theme::keybinding_desc()),
            Span::styled("[Esc]", theme::keybinding_key()),
            Span::styled(" Quit", theme::keybinding_desc()),
        ])
    };
    let footer = Paragraph::new(keys).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

// ── History screen ──────────────────────────────────────────────────────────

fn draw_history_screen(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(5),    // history list
            Constraint::Length(1), // footer
        ])
        .split(area);

    draw_tab_bar(f, app, chunks[0]);
    draw_history_list(f, app, chunks[1]);
    draw_history_footer(f, chunks[2]);
}

fn draw_history_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            format!(" History ({}) ", app.roll_history.len()),
            theme::title(),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    if app.roll_history.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No rolls yet",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )))
        .block(block);
        f.render_widget(empty, area);
        return;
    }

    // Show history in reverse (newest first)
    let items: Vec<ListItem> = app
        .roll_history
        .iter()
        .rev()
        .skip(app.history_scroll)
        .map(|entry| {
            let total_style = if entry.is_nat_max {
                theme::nat_max()
            } else if entry.is_nat_min {
                theme::nat_min()
            } else {
                theme::history_result()
            };
            let line = Line::from(vec![
                Span::styled(format!("{:<16}", entry.expression), theme::history_expr()),
                Span::styled(" => ", Style::default().fg(ratatui::style::Color::DarkGray)),
                Span::styled(format!("{:<6}", entry.total), total_style),
                Span::styled(&entry.breakdown, theme::result_detail()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_history_footer(f: &mut Frame, area: Rect) {
    let keys = Line::from(vec![
        Span::styled("[Tab]", theme::keybinding_key()),
        Span::styled(" Roller  ", theme::keybinding_desc()),
        Span::styled("[PgUp/PgDn]", theme::keybinding_key()),
        Span::styled(" Scroll  ", theme::keybinding_desc()),
        Span::styled("[F1]", theme::keybinding_key()),
        Span::styled(" Help  ", theme::keybinding_desc()),
        Span::styled("[Esc]", theme::keybinding_key()),
        Span::styled(" Back", theme::keybinding_desc()),
    ]);
    let footer = Paragraph::new(keys).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

// ── Help overlay ────────────────────────────────────────────────────────────

fn draw_help_overlay(f: &mut Frame, _app: &App) {
    let area = f.area();

    // Center a box
    let width = 54.min(area.width.saturating_sub(4));
    let height = 26.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Roller Tab",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  Enter      ", theme::keybinding_key()),
            Span::styled("Roll the expression", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", theme::keybinding_key()),
            Span::styled("Switch to History tab", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down    ", theme::keybinding_key()),
            Span::styled("Browse input history", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  F2         ", theme::keybinding_key()),
            Span::styled("Focus presets sidebar", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+U     ", theme::keybinding_key()),
            Span::styled("Clear input", theme::keybinding_desc()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Presets (F2)",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  j/k        ", theme::keybinding_key()),
            Span::styled("Navigate presets", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", theme::keybinding_key()),
            Span::styled("Roll selected preset", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  d          ", theme::keybinding_key()),
            Span::styled("Delete selected preset", theme::keybinding_desc()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  History Tab",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn  ", theme::keybinding_key()),
            Span::styled("Scroll roll history", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Tab/Esc    ", theme::keybinding_key()),
            Span::styled("Back to Roller", theme::keybinding_desc()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Expressions",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  2d6+3      ", theme::keybinding_key()),
            Span::styled("Roll 2 six-sided dice + 3", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  adv d20+5  ", theme::keybinding_key()),
            Span::styled("Roll with advantage", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  4d6kh3     ", theme::keybinding_key()),
            Span::styled("Roll 4d6, keep highest 3", theme::keybinding_desc()),
        ]),
        Line::from(""),
    ];

    let block = Block::default()
        .title(Span::styled(" Help (F1 to close) ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup);
}
