# Dice expression syntax

`roll` accepts a compact dice-expression language. Expressions are
case-insensitive. Whitespace is accepted around `+`/`-` operators and after an
`adv` or `dis` modifier; do not put whitespace inside a dice group.

## Grammar

```text
expression  = [modifier] term { ('+' | '-') term }
modifier    = 'adv' | 'dis'
dice-group  = [count] 'd' sides [keep]
term        = dice-group | integer
count       = positive integer (defaults to 1)
sides       = positive integer
keep        = 'kh' positive integer | 'kl' positive integer
integer     = whole number
```

An expression must contain at least one dice group. A flat modifier may appear
before, between, or after dice groups. Dice groups may be added, but they may
not be negated.

## Examples

| Expression | Meaning |
| --- | --- |
| `d20` | One twenty-sided die. |
| `2d10+4` | Two d10s, plus four. |
| `2d6+1d4+3` | Two d6s, one d4, plus three. |
| `d20-1` | One d20, minus one. |
| `adv d20+5` | Roll a d20 with advantage, then add five. |
| `dis d20-1` | Roll a d20 with disadvantage, then subtract one. |
| `4d6kh3` | Roll four d6s and keep the highest three. |
| `4d6kl1` | Roll four d6s and keep the lowest one. |

## Modifiers

`adv` and `dis` apply to the primary roll. They are intended for the familiar
d20-style advantage and disadvantage rules: two candidate results are rolled
and the higher (`adv`) or lower (`dis`) result is used.

## Keep rules

`khN` and `klN` follow a dice group directly:

- `khN` keeps the highest `N` dice from that group.
- `klN` keeps the lowest `N` dice from that group.

For example, `4d6kh3` is the common ability-score roll. Keep counts are parsed
as unsigned integers. A count of zero keeps no dice; a count larger than the
number rolled keeps every die in that group.

## Probability modes

`roll --prob <target>` calculates the chance of meeting or exceeding a target.
Simple expressions use an exact calculation. Expressions with advantage,
disadvantage, or keep rules use Monte Carlo estimation instead; control its
sample count with the `SIMS` environment variable.

## Invalid input

The following are invalid and result in a parse error:

- Missing sides, such as `2d`.
- Zero or negative dice counts/sides, such as `0d6` or `d0`.
- A malformed keep suffix, such as `4d6kh` or `4d6klmany`.
- A modifier without a following dice expression, such as `adv`.
- Unrecognised tokens, such as `2x6`.
