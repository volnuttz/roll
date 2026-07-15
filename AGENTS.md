# AGENTS.md

## Project

`rollcli` is a Rust command-line dice roller. The published crate is named
`rollcli`, while the executable and library are named `roll`.

## Working agreement

- Keep changes focused and preserve the public dice-expression syntax and CLI
  flags unless the task explicitly changes them.
- Run `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test` before handing off Rust changes.
- Update `README.md` for user-facing commands and `docs/RELEASING.md` for any
  distribution-process change.
- Do not edit `Cargo.lock` by hand. Regenerate it through Cargo when dependency
  changes require it.
- Treat `~/.config/roll/presets.toml` as user data: code must tolerate a
  missing file and avoid destructive changes to it.

## Commands

```sh
cargo build
cargo build --release
cargo run -- 2d10+4
cargo test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```

## Code map

- `docs/ARCHITECTURE.md`: module boundaries and command flow.
- `docs/DICE-SYNTAX.md`: public dice-expression language reference.
- `.github/workflows/ci.yml`: pull-request and main-branch validation.
- `.github/workflows/release.yml`: builds and publishes binaries from version
  tags.
