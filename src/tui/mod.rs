pub mod app;
pub mod event;
pub mod theme;
pub mod ui;

use crossterm::ExecutableCommand as _;
use crossterm::event::Event;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, stdout};
use std::time::Duration;

use app::{App, PresetEntry};

// ── Preset helpers (bridge to main.rs preset system) ─────────────────────────

/// Load presets as a sorted Vec for the TUI.
pub fn load_presets_list() -> Vec<PresetEntry> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("roll")
        .join("presets.toml");

    if !path.exists() {
        return Vec::new();
    }

    let content = std::fs::read_to_string(&path).unwrap_or_default();

    #[derive(serde::Deserialize, Default)]
    struct Presets {
        #[serde(default)]
        presets: std::collections::HashMap<String, String>,
    }

    let loaded: Presets = toml::from_str(&content).unwrap_or_default();
    let mut entries: Vec<PresetEntry> = loaded
        .presets
        .into_iter()
        .map(|(name, expression)| PresetEntry { name, expression })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Delete a preset by name.
pub fn delete_preset(name: &str) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("roll")
        .join("presets.toml");

    if !path.exists() {
        return;
    }

    let content = std::fs::read_to_string(&path).unwrap_or_default();

    #[derive(serde::Serialize, serde::Deserialize, Default)]
    struct Presets {
        #[serde(default)]
        presets: std::collections::HashMap<String, String>,
    }

    let mut loaded: Presets = toml::from_str(&content).unwrap_or_default();
    loaded.presets.remove(name);

    if let Ok(serialized) = toml::to_string(&loaded) {
        let _ = std::fs::write(&path, serialized);
    }
}

// ── Terminal wrapper with drop-based cleanup ─────────────────────────────────

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

pub fn run(initial_expr: Option<&str>, sims: u64) -> io::Result<()> {
    let mut guard = TerminalGuard::new()?;
    let mut app = App::new(initial_expr, sims);

    // Load presets on startup
    app.reload_presets();

    // If initial expression was provided, roll it immediately
    if initial_expr.is_some() {
        app.submit_roll();
    }

    loop {
        guard.terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(Event::Key(key)) = event::poll_event(Duration::from_millis(50))? {
            event::handle_key(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
