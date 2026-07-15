# Releasing rollcli

`rollcli` is distributed in two complementary ways:

- crates.io provides `cargo install rollcli`.
- GitHub Releases provides prebuilt `roll` binaries for users who do not have
  Rust installed.

## One-time repository setup

The `release.yml` workflow requires the repository Actions setting **Workflow
permissions** to allow read and write access. It uses the built-in
`GITHUB_TOKEN` with `contents: write`; no personal access token is needed for
GitHub Releases.

Keep crates.io publishing credentials outside the repository. A maintainer may
publish locally with Cargo credentials, or a future protected publishing
workflow can use a repository environment and trusted publishing.

## Release checklist

1. Update `version` in `Cargo.toml`, `Cargo.lock` via Cargo if needed, and user
   documentation or changelog entries for the release.
2. On the final release commit, run:

   ```sh
   cargo fmt -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo package
   cargo publish --dry-run
   ```

3. Publish the verified package:

   ```sh
   cargo publish
   ```

4. Create and push the matching annotated tag. For version `0.3.3`, use:

   ```sh
   git tag -a v0.3.3 -m "rollcli v0.3.3"
   git push origin v0.3.3
   ```

5. Watch the **Release binaries** GitHub Actions workflow. It validates the
   tag/version match, tests the package, then builds native archives for:

   | Platform | Archive |
   | --- | --- |
   | Linux x86_64 | `roll-v<VERSION>-x86_64-unknown-linux-gnu.tar.gz` |
   | macOS Intel | `roll-v<VERSION>-x86_64-apple-darwin.tar.gz` |
   | macOS Apple Silicon | `roll-v<VERSION>-aarch64-apple-darwin.tar.gz` |
   | Windows x86_64 | `roll-v<VERSION>-x86_64-pc-windows-msvc.zip` |

6. Confirm the generated GitHub Release has those four archives and
   `SHA256SUMS.txt`; download and run the appropriate archive before announcing
   the release.

## Failure handling

If crates.io publishing succeeds but the binary workflow fails, fix the
workflow or release-only packaging issue and re-run the workflow for the same
tag. Do not republish the crate. If the tag does not match `Cargo.toml`, the
workflow deliberately stops before creating a release.
