---
name: roll-rust-cli
description: Implement and validate changes to the rollcli Rust command-line dice roller.
---

# rollcli Rust development

Use this skill for parser, rolling, probability, CLI, preset, or TUI work.

1. Identify the affected layer: `src/lib.rs` for core behavior, `src/main.rs`
   for command-line and preset behavior, or `src/tui/` for terminal UI state,
   events, and rendering.
2. Preserve deterministic tests where possible. Random behavior should be
   tested through injected or seeded RNG paths when available.
3. Add or update unit tests in `src/lib.rs` for parser and probability changes.
4. Validate with:

   ```sh
   cargo fmt -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

5. Smoke-test a changed command with `cargo run -- <expression>` when it is
   safe and non-interactive. Do not attempt a full TUI smoke test in a
   non-interactive terminal.

Compatibility notes:

- The crate package is `rollcli`; the installed executable is `roll`.
- Dice expressions support advantage/disadvantage, multiple groups, and keep
  rules. Changes to their parsing are public API changes.
- Exact probability intentionally falls back to Monte Carlo for unsupported
  expressions such as advantage/disadvantage and keep rules.
