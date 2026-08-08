# Releasing rollcli

`rollcli` is distributed in two complementary ways:

- crates.io provides `cargo install rollcli`.
- GitHub Releases provides prebuilt `roll` binaries for users who do not have
  Rust installed.

## One-time repository setup

The `release.yml` workflow defaults to read-only access. Only its final GitHub
Release job receives `contents: write`, using the built-in `GITHUB_TOKEN`; no
personal access token is needed.

Keep crates.io publishing credentials outside the repository. A maintainer may
publish locally with Cargo credentials, or a future protected publishing
workflow can use a repository environment and trusted publishing.

## Release checklist

1. Create a release branch and update `version` in `Cargo.toml`, regenerate
   `Cargo.lock` through Cargo if needed, and update the Unreleased changelog.
2. Open a pull request into `main`; do not bypass its required quality,
   dependency-policy, and platform checks.
3. On the final release commit, run:

   ```sh
   cargo fmt -- --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo test --locked
   cargo package --locked
   cargo publish --dry-run --locked
   ```

4. Merge the approved pull request and verify the required workflows passed for
   the exact merged commit.
5. Publish the verified package from that commit:

   ```sh
   cargo publish --locked
   ```

6. Create and push the matching annotated tag. For version `0.3.5`, use:

   ```sh
   git tag -a v0.3.5 -m "rollcli v0.3.5"
   git push origin v0.3.5
   ```

7. Watch the **Release binaries** GitHub Actions workflow. It validates the
   tag/version match, tests the package, then builds native archives for:

   | Platform | Archive |
   | --- | --- |
   | Linux x86_64 | `roll-v<VERSION>-x86_64-unknown-linux-gnu.tar.gz` |
   | macOS Apple Silicon | `roll-v<VERSION>-aarch64-apple-darwin.tar.gz` |
   | Windows x86_64 | `roll-v<VERSION>-x86_64-pc-windows-msvc.zip` |

8. Confirm the generated GitHub Release has those three archives and
   `SHA256SUMS.txt`. Download the assets, verify archive contents and checksums,
   and run the appropriate binary before announcing the release.

Never move, delete, or recreate a published release tag. Re-run the workflow
for the same tag when repairing a release-only failure.

## Failure handling

If crates.io publishing succeeds but the binary workflow fails, fix the
workflow or release-only packaging issue and re-run the workflow for the same
tag. Do not republish the crate. If the tag does not match `Cargo.toml`, the
workflow deliberately stops before creating a release.
