# CLAUDE.md

## Build & Run

```
cargo build              # debug build
cargo build --release    # release build
cargo run -- 2d10+4      # run with arguments
```

## Test

```
cargo test
```

Tests are in `src/lib.rs`. The project has no CI configuration.

## Lint

```
cargo clippy
cargo fmt -- --check
```

## Architecture

Rust CLI split into a library and a thin binary:
- `src/lib.rs` — core logic (parsing, rolling, probability, distribution)
- `src/main.rs` — CLI entry point using `clap` (derive) and `rand`

Key types:
- `DiceExpr` — parsed dice expression (modifier, dice groups, flat bonus)
- `Modifier` — None / Advantage / Disadvantage
- `DiceGroup` — count + sides (e.g. 2d10)

Key functions:
- `parse_expr()` — parse a dice expression string into `DiceExpr`
- `roll_verbose()` — single roll with human-readable breakdown
- `roll_value()` — single roll returning only the total
- `roll_once()` — single roll returning total and per-group detail
- `estimate_probability()` — Monte Carlo P(result >= target)
- `compute_distribution()` / `render_distribution()` — full result histogram

Flow: CLI args → `parse_expr()` → one of three paths:
1. Default — `roll_verbose()` for a single roll with breakdown
2. `--prob` — `estimate_probability()` via Monte Carlo
3. `--dist` — `compute_distribution()` + `render_distribution()` for a histogram
