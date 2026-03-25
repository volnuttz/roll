# roll

A command-line dice roller for tabletop RPGs. Supports standard dice notation, advantage/disadvantage, and probability estimation via Monte Carlo simulation.

## Usage

```
roll <expression> [--prob <target>] [--dist] [--sims <n>]
```

### Dice expressions

- `2d10+4` — roll 2 ten-sided dice and add 4
- `d20` — roll a single d20
- `adv d20+5` — roll d20 with advantage, add 5
- `dis d20-1` — roll d20 with disadvantage, subtract 1
- `2d6+1d4+3` — roll multiple dice groups with a flat bonus

### Probability estimation

Use `--prob` to estimate the chance of rolling at least a given value:

```
roll 2d10+4 --prob 15
# P(2d10+4 >= 15) = 42.00% (420000 / 1000000 sims)
```

The number of simulations defaults to 1,000,000 and can be changed with `--sims`.

### Distribution histogram

Use `--dist` to see the full probability distribution as an ASCII histogram:

```
roll 2d6 --dist
```

This shows every possible result value with its percentage and a bar chart. Cannot be combined with `--prob`.

## Building

```
cargo build --release
```

## License

MIT
