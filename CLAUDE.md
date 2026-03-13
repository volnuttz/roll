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

No tests exist yet. The project has no CI configuration.

## Lint

```
cargo clippy
cargo fmt -- --check
```

## Architecture

Single-file Rust CLI (`src/main.rs`) using `clap` (derive) for argument parsing and `rand` for dice rolls.

Key types:
- `DiceExpr` — parsed dice expression (modifier, dice groups, flat bonus)
- `Modifier` — None / Advantage / Disadvantage
- `DiceGroup` — count + sides (e.g. 2d10)

Flow: CLI args → `parse_expr()` → either `roll_verbose()` for a single roll or `roll_value()` in a loop for `--prob` Monte Carlo estimation.
