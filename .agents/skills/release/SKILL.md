---
name: roll-release
description: Prepare and verify a rollcli crates.io and GitHub binary release.
---

# rollcli release procedure

Read `docs/RELEASING.md` before changing a release or distribution workflow.

Release invariants:

- `Cargo.toml` is the single version source; release tags must be `v<version>`.
- Run the complete Rust validation suite and `cargo publish --dry-run` before a
  real publish.
- Publish to crates.io from the final commit, then create and push its matching
  annotated tag. The tag triggers GitHub binary builds.
- Never expose a crates.io token in workflow files or logs.
- Verify the GitHub release contains all expected archives and
  `SHA256SUMS.txt` before announcing it.

The GitHub workflow uses native hosted runners for Linux, macOS (Intel and
Apple Silicon), and Windows. Keep that matrix aligned with the archives listed
in the release documentation.
