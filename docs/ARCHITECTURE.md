# Architecture

`rollcli` is a Rust package with a reusable core library and a thin executable.
The published package is `rollcli`; both the binary and library are named
`roll`.

## Module map

| Path | Responsibility |
| --- | --- |
| `src/lib.rs` | Dice-expression parsing, random rolling, analytical statistics, exact and simulated probability, distribution rendering, and unit tests. |
| `src/main.rs` | Clap argument parsing, command routing, REPL, and named-preset persistence. |
| `src/tui/mod.rs` | Terminal setup/teardown and public TUI entry point. |
| `src/tui/app.rs` | TUI state, roll history, presets, and distribution data. |
| `src/tui/event.rs` | Keyboard-event handling and state transitions. |
| `src/tui/ui.rs` | Ratatui layout and widget rendering. |
| `src/tui/theme.rs` | Shared terminal colour palette. |

## Command flow

```text
CLI arguments
  -> preset resolution (when applicable)
  -> parse_expr()
  -> normal roll | probability | distribution | REPL | preset management | TUI
```

Normal rolls use `roll_verbose()` for output or `roll_value()` where only a
total is needed. `roll_stats()` supplies analytical minimum, maximum, and mean.
Probability requests use `exact_probability()` when supported, otherwise
`estimate_probability()`; full histograms use `compute_distribution()` and
`render_distribution()`.

## Core types

- `DiceExpr`: parsed expression containing modifier, dice groups, and flat
  bonus.
- `Modifier`: no modifier, advantage, or disadvantage.
- `DiceGroup`: dice count, sides, and optional keep rule.
- `Keep`: keep all, the highest `N`, or the lowest `N` dice.
- `ParseError`: typed parser failure that implements `Display` and `Error`.

## Persistence boundary

Named presets are stored outside the repository in
`~/.config/roll/presets.toml`. Application code must treat that file as user
data: it may not exist, may be malformed, and must not be overwritten except
through explicit preset-management commands.

## Change boundaries

Parser changes affect the public expression language and should include tests
in `src/lib.rs` and examples in `docs/DICE-SYNTAX.md`. TUI changes should keep
state transitions in `app.rs`/`event.rs` separate from rendering in `ui.rs`.
