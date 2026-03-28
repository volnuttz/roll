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
- `src/main.rs` — CLI entry point using `clap` (derive), `rand`, `serde`, and `toml`

Key types:
- `DiceExpr` — parsed dice expression (modifier, dice groups, flat bonus)
- `Modifier` — None / Advantage / Disadvantage
- `DiceGroup` — count + sides + keep rule (e.g. `4d6kh3`)
- `Keep` — All / Highest(n) / Lowest(n)
- `ParseError` — typed parse failure enum (implements `Display` + `std::error::Error`)
- `RollStats` — theoretical min, max, mean for an expression

Key functions:
- `parse_expr()` — parse a dice expression string into `DiceExpr`; returns `Result<DiceExpr, ParseError>`
- `roll_verbose()` — single roll with human-readable breakdown
- `roll_value()` — single roll returning only the total
- `roll_once()` — single roll returning total and kept dice per group
- `roll_stats()` — analytical min/max/mean (ignores adv/dis)
- `exact_probability()` — exact P(result >= target) via polynomial convolution; returns `None` for adv/dis/keep
- `estimate_probability()` — Monte Carlo P(result >= target)
- `compute_distribution()` / `render_distribution()` — full result histogram

Flow: CLI args → preset resolution → `parse_expr()` → one of these paths:
1. Default — `roll_verbose()` repeated `--times` N times; optionally print `roll_stats()` via `--stats`
2. `--prob` — `exact_probability()` if supported, else `estimate_probability()` via Monte Carlo
3. `--dist` — `compute_distribution()` + `render_distribution()` for a histogram
4. `--repl` — read expressions from stdin in a loop
5. `--save` / `--list` / `--delete` — manage named presets in `~/.config/roll/presets.toml`

## Dependencies

- `clap` — CLI argument parsing (derive feature)
- `rand 0.9` — dice rolling RNG (`rand::rng()` for thread-local RNG)
- `serde` + `toml` — preset file serialisation/deserialisation
