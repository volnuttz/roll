# Changelog

All notable user-facing changes to `rollcli` are recorded here. The format is
inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- Added exact-value, at-most, and inclusive-range probability queries.
- Added machine-readable JSON output for rolls, statistics, probability
  queries, and distributions.

### Changed

- Renamed the at-least probability flag from `--prob` to `--prob-ge` for
  consistency with the other probability query flags.
- Pinned Rust 1.88 as the MSRV and added hardened quality, dependency-policy,
  coverage, and release automation.

## 0.3.5

### Changed

- GitHub binary releases no longer include macOS Intel binaries.

## 0.3.4

### Added

- GitHub Release archives for Linux, macOS, and Windows, generated from
  version tags.

## 0.3.3

Current package version. Earlier releases predate this changelog; consult the
Git history and GitHub Releases for their detailed history.
