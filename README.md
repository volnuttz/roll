# roll

A command-line dice roller for tabletop RPGs. Supports standard dice notation,
advantage/disadvantage, keep-highest/lowest, exact probability computation, and
named presets.

## Usage

```
roll <expression> [options]
roll --tui [expression]
roll --repl
roll --save <name> <expression>
roll --list
roll --delete <name>
```

### Dice expressions

| Expression | Meaning |
|---|---|
| `2d10+4` | Roll 2d10, add 4 |
| `d20` | Roll a single d20 |
| `adv d20+5` | Roll d20 with advantage (take higher), add 5 |
| `dis d20-1` | Roll d20 with disadvantage (take lower), subtract 1 |
| `2d6+1d4+3` | Multiple dice groups with a flat bonus |
| `4d6kh3` | Roll 4d6, keep the highest 3 (D&D ability scores) |
| `4d6kl1` | Roll 4d6, keep the lowest 1 |

### Rolling multiple times

Use `-n` / `--times` to roll the same expression several times at once:

```
roll 2d6+3 -n 5
# #1: 2d6+3 => [4, 2] (+3) = 9
# #2: 2d6+3 => [6, 1] (+3) = 10
# ...
```

### Theoretical statistics

Use `--stats` to print the theoretical min, max, and mean alongside any output:

```
roll 2d6+3 --stats
# 2d6+3 => [5, 3] (+3) = 11
#   [min=5, max=15, mean=10.00]
```

### Probability estimation

Use `--prob` to calculate the chance of rolling at least a given value.
For simple expressions (no advantage/disadvantage, no keep), the result is
computed **exactly** via polynomial convolution; otherwise it falls back to
Monte Carlo simulation:

```
roll d20 --prob 15
# P(1d20 >= 15) = 30.0000% (exact)

roll adv d20 --prob 15
# P(adv 1d20 >= 15) = 50.97% (509700 / 1000000 sims)
```

The number of simulations defaults to 1,000,000 and can be changed with `--sims`.

### Distribution histogram

Use `--dist` to see the full probability distribution as an ASCII histogram.
Cannot be combined with `--prob`.

```
roll 2d6 --dist
# Distribution for 2d6 (1000000 simulations):
#  2 |  2.8% ███
#  3 |  5.5% ██████
#  ...
```

The simulation count defaults to 1,000,000 and can be changed with `--sims`.

### Interactive REPL

Use `--repl` to drop into an interactive session where you can type expressions
one per line without re-invoking the binary:

```
roll --repl
# Roll REPL — type a dice expression or 'quit' to exit.
# > adv d20+5
# adv 1d20+5 => [18] vs [7] (+5) = 23
# > 4d6kh3
# 4d6kh3 => [6, 5, 3] = 14
# > quit
```

### Named presets

Save frequently-used expressions as named presets stored in
`~/.config/roll/presets.toml`:

```
roll --save attack "adv d20+7"
# Saved preset 'attack' = 'adv d20+7'.

roll --list
# Saved presets:
#   attack = adv d20+7

roll attack
# adv 1d20+7 => [17] vs [9] (+7) = 24

roll --delete attack
# Deleted preset 'attack'.
```

Preset names are resolved case-insensitively before the input is parsed as a
dice expression. Any roll option (`--times`, `--stats`, `--prob`, `--dist`)
works normally with presets.

### Interactive TUI

Use `--tui` / `-t` to launch a full-screen terminal interface:

```
roll --tui
roll adv d20+5 --tui   # opens TUI with expression pre-filled
```

The TUI has two tabs and a help overlay:

- **Roller** — type a dice expression and press Enter to roll. Results show
  individual dice with natural-max/natural-min highlighting, plus an
  auto-generated distribution chart. A presets sidebar (F2) lets you browse
  and roll saved presets.
- **History** — scrollable log of all rolls from the current session.

#### Key bindings

| Key | Action |
|---|---|
| `Enter` | Roll the current expression |
| `Tab` | Switch between Roller and History tabs |
| `F1` | Toggle help overlay |
| `F2` | Toggle presets sidebar |
| `Up` / `Down` | Navigate input history (Roller) |
| `j` / `k` | Navigate presets list (when sidebar focused) |
| `d` | Delete selected preset (press twice to confirm) |
| `PageUp` / `PageDown` | Scroll history (History tab) |
| `Ctrl+A` / `Ctrl+E` | Move cursor to start / end of input |
| `Ctrl+U` | Clear input line |
| `Esc` | Dismiss error, close sidebar/overlay, or quit |
| `Ctrl+C` | Quit immediately |

## Building

```
cargo build --release
```

## License

MIT
