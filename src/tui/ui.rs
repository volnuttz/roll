use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
};

use super::app::{App, Screen};
use super::theme;

/// Main draw dispatch.
pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Roller => draw_roller(f, app),
        Screen::Distribution => draw_distribution(f, app),
        Screen::Presets => draw_presets(f, app),
        Screen::Help => {
            // Draw the previous screen underneath, then overlay help
            match app.prev_screen {
                Screen::Roller => draw_roller(f, app),
                Screen::Distribution => draw_distribution(f, app),
                Screen::Presets => draw_presets(f, app),
                Screen::Help => draw_roller(f, app),
            }
            draw_help_overlay(f, app);
        }
    }
}

// ── Roller screen ────────────────────────────────────────────────────────────

fn draw_roller(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // input
            Constraint::Length(5), // result
            Constraint::Min(5),    // history
            Constraint::Length(1), // footer
        ])
        .split(area);

    draw_header(f, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_result(f, app, chunks[2]);
    draw_history(f, app, chunks[3]);
    draw_roller_footer(f, chunks[4]);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" roll ", theme::title()),
            Span::styled(
                "- dice roller ",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));
    f.render_widget(block, area);
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

    let block = Block::default()
        .title(Span::styled(" Expression ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    let paragraph = Paragraph::new(Line::from(display)).block(block);
    f.render_widget(paragraph, area);

    // Place cursor
    if !app.input.is_empty() || app.screen == Screen::Roller {
        let cursor_x = area.x + 2 + app.cursor_pos as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
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

fn draw_history(f: &mut Frame, app: &App, area: Rect) {
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

    // Show history in reverse (newest first), skip the latest since it's in Result
    let items: Vec<ListItem> = app
        .roll_history
        .iter()
        .rev()
        .skip(1) // skip latest, it's shown in Result area
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

fn draw_roller_footer(f: &mut Frame, area: Rect) {
    let keys = Line::from(vec![
        Span::styled("[Enter]", theme::keybinding_key()),
        Span::styled(" Roll  ", theme::keybinding_desc()),
        Span::styled("[Tab]", theme::keybinding_key()),
        Span::styled(" Distribution  ", theme::keybinding_desc()),
        Span::styled("[F2]", theme::keybinding_key()),
        Span::styled(" Presets  ", theme::keybinding_desc()),
        Span::styled("[F1]", theme::keybinding_key()),
        Span::styled(" Help  ", theme::keybinding_desc()),
        Span::styled("[Esc]", theme::keybinding_key()),
        Span::styled(" Quit", theme::keybinding_desc()),
    ]);
    let footer = Paragraph::new(keys).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

// ── Distribution screen ──────────────────────────────────────────────────────

fn draw_distribution(f: &mut Frame, app: &App) {
    let area = f.area();

    let Some(ref dist) = app.dist else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(8),    // bar chart
            Constraint::Length(3), // stats
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let header = Block::default()
        .title(Line::from(vec![
            Span::styled(" Distribution: ", theme::title()),
            Span::styled(&dist.expr_str, theme::history_expr()),
            Span::styled(
                format!("  ({} sims) ", format_sims(dist.sims)),
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));
    f.render_widget(header, chunks[0]);

    // Bar chart
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
            let style = if dist.target == Some(value) {
                theme::bar_highlight()
            } else {
                theme::bar_chart()
            };
            Bar::default()
                .value(pct)
                .label(Line::from(value.to_string()))
                .style(style)
                .text_value(format!("{:.1}%", count as f64 / total as f64 * 100.0))
        })
        .collect();

    let chart_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    let chart = BarChart::default()
        .block(chart_block)
        .data(BarGroup::default().bars(&bars))
        .bar_width(
            // Calculate appropriate bar width based on available space
            ((chunks[1].width.saturating_sub(2)) / (bars.len() as u16).max(1)).clamp(1, 5),
        )
        .bar_gap(1);

    f.render_widget(chart, chunks[1]);

    // Stats
    let stats_block = Block::default()
        .title(Span::styled(" Stats ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    let mut stats_spans = vec![Span::styled(
        format!(
            "  min={}  max={}  mean={:.2}",
            dist.stats.min, dist.stats.max, dist.stats.mean
        ),
        theme::stats(),
    )];

    if let (Some(target), Some(prob)) = (dist.target, dist.target_prob) {
        stats_spans.push(Span::styled(
            format!("    P(>= {target}) = {:.2}%", prob * 100.0),
            Style::default()
                .fg(theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let stats_para = Paragraph::new(Line::from(stats_spans)).block(stats_block);
    f.render_widget(stats_para, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[Left/Right]", theme::keybinding_key()),
        Span::styled(" Move target  ", theme::keybinding_desc()),
        Span::styled("[+/-]", theme::keybinding_key()),
        Span::styled(" Sims  ", theme::keybinding_desc()),
        Span::styled("[Esc/Tab]", theme::keybinding_key()),
        Span::styled(" Back", theme::keybinding_desc()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[3]);
}

fn format_sims(sims: u64) -> String {
    if sims >= 1_000_000 {
        format!("{}M", sims / 1_000_000)
    } else if sims >= 1_000 {
        format!("{}K", sims / 1_000)
    } else {
        sims.to_string()
    }
}

// ── Presets screen ───────────────────────────────────────────────────────────

fn draw_presets(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // list
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let header = Block::default()
        .title(Span::styled(" Presets ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));
    f.render_widget(header, chunks[0]);

    // List
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_DIM));

    if app.presets.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No presets saved. Use --save <name> from CLI to add presets.",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )))
        .block(list_block);
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .presets
            .iter()
            .enumerate()
            .map(|(i, preset)| {
                let marker = if i == app.preset_selected {
                    " > "
                } else {
                    "   "
                };
                let style = if i == app.preset_selected {
                    theme::selected()
                } else {
                    Style::default()
                };
                let line = Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(format!("{:<16}", preset.name), theme::preset_name()),
                    Span::styled(&preset.expression, theme::preset_expr()),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(list_block);
        f.render_widget(list, chunks[1]);
    }

    // Footer
    let footer_spans = if app.preset_confirm_delete {
        vec![
            Span::styled("[d]", theme::keybinding_key()),
            Span::styled(" Confirm delete  ", Style::default().fg(theme::DANGER)),
            Span::styled("[Esc]", theme::keybinding_key()),
            Span::styled(" Cancel", theme::keybinding_desc()),
        ]
    } else {
        vec![
            Span::styled("[Enter]", theme::keybinding_key()),
            Span::styled(" Roll  ", theme::keybinding_desc()),
            Span::styled("[d]", theme::keybinding_key()),
            Span::styled(" Delete  ", theme::keybinding_desc()),
            Span::styled("[j/k]", theme::keybinding_key()),
            Span::styled(" Navigate  ", theme::keybinding_desc()),
            Span::styled("[Esc]", theme::keybinding_key()),
            Span::styled(" Back", theme::keybinding_desc()),
        ]
    };
    let footer = Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

// ── Help overlay ─────────────────────────────────────────────────────────────

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
            "  Roller Screen",
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
            Span::styled("Show distribution chart", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down    ", theme::keybinding_key()),
            Span::styled("Browse input history", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/PgDn  ", theme::keybinding_key()),
            Span::styled("Scroll roll history", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  F2         ", theme::keybinding_key()),
            Span::styled("Open presets", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+U     ", theme::keybinding_key()),
            Span::styled("Clear input", theme::keybinding_desc()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Distribution",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  Left/Right ", theme::keybinding_key()),
            Span::styled("Move probability target", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  +/-        ", theme::keybinding_key()),
            Span::styled("Increase/decrease simulations", theme::keybinding_desc()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Colors",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme::ACCENT),
        )]),
        Line::from(vec![
            Span::styled("  NAT MAX!   ", theme::nat_max()),
            Span::styled("All dice rolled maximum", theme::keybinding_desc()),
        ]),
        Line::from(vec![
            Span::styled("  NAT MIN    ", theme::nat_min()),
            Span::styled("All dice rolled 1", theme::keybinding_desc()),
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
