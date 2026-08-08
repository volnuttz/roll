# Contributing

Thanks for improving `roll`. Small, focused pull requests are easiest to
review.

## Local setup

The repository pins its minimum supported Rust version (MSRV) in
`rust-toolchain.toml`. Rustup selects it automatically after cloning:

```sh
cargo build
cargo test
```

Run a command during development with:

```sh
cargo run -- 2d10+4
```

For interactive work, use `cargo run -- --tui` in a real terminal.

## Before opening a pull request

Run the same checks enforced by CI:

```sh
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo package --locked
```

CI also runs tests on Linux, macOS, Windows, and the current stable Rust
release. Coverage is measured with `cargo-llvm-cov`; the 33% line floor is a
regression guard based on the measured whole-application baseline, not a
coverage target.

## Guidelines

- Keep dice syntax and CLI flags backward-compatible unless the issue and PR
  clearly describe a breaking change.
- Add focused tests in `src/lib.rs` for parser, roll, probability, or
  distribution changes.
- Keep terminal state/event logic separate from rendering when editing the TUI.
- Document user-visible behavior in `README.md` and use
  `docs/DICE-SYNTAX.md` for expression-language changes.
- Avoid committing user preset files, build output, credentials, or generated
  release archives.

## Issues and pull requests

Describe the user-facing behavior, include representative dice expressions or
terminal output where helpful, and call out any effect on probability mode or
saved presets. See `AGENTS.md` and `.agents/skills/rust-cli/SKILL.md` for the
repository’s agent-oriented implementation guidance.
