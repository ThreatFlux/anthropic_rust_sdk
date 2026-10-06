# Fix crates.io Publish Target

## Goal

Publish this Anthropic Rust SDK under a ThreatFlux-owned crates.io target and make the release workflow actually upload the intended crate.

## Status

Implemented with `threatflux-anthropic-sdk` as the crates.io package name and `threatflux_anthropic_sdk` as the Rust import path. Releases are managed by Release Please, which opens version-bump PRs from conventional commits and creates the GitHub Release/tag after the release PR is merged. The release workflow then publishes crates.io, publishes a GHCR image to GitHub Packages, and uploads crate/checksum assets to the GitHub Release.

crates.io publishing now uses [trusted publishing](https://crates.io/docs/trusted-publishing) instead of a stored registry token. The `publish` job in `.github/workflows/release.yml` runs in the `crates-io` environment, and `rust-lang/crates-io-auth-action` exchanges the job's GitHub OIDC token for a short-lived crates.io token, which it revokes when the job ends. No registry token secret is stored or read. 0.4.1 was the first version published this way. `CONTRIBUTING.md` documents the current release process.

The rest of this file is the original June 2026 plan, kept for history. Where it mentions a registry token, trusted publishing replaces it.

## Scope

In scope:
- Correct crate identity and metadata in `Cargo.toml`.
- Align README badges, install snippets, imports, and docs links with the chosen crate name.
- Fix the GitHub release workflow so tagged releases can publish to crates.io.
- Remove or repair release binary packaging that currently references a non-existent `anthropic_rust_sdk` binary.
- Add pre-publish verification commands to CI.

Out of scope:
- Changing SDK API behavior.
- Running real Anthropic API tests unless credentials are explicitly supplied.
- Publishing any crate before the final name is confirmed.

## Original Findings (June 2026)

- `cargo publish --dry-run --allow-dirty` passes for the current package.
- The current package name is `threatflux`, not `anthropic_rust_sdk`.
- `anthropic_rust_sdk` normalizes to `anthropic-rust-sdk`, which already exists on crates.io and is not owned by this project.
- `Cargo.toml` points repository/docs metadata at `wyattroersma/threatflux`, not `ThreatFlux/anthropic_rust_sdk`.
- `.github/workflows/release.yml` still runs `cargo publish --dry-run`, so releases do not publish.
- The release workflow tries to package `anthropic_rust_sdk` binaries that this crate does not build.

## Recommended Publish Target

Use:

```toml
name = "threatflux-anthropic-sdk"
```

Rationale:
- It is clearly owned/namespaced by ThreatFlux.
- It describes the project better than the broad `threatflux` name.
- It avoids the already-published `anthropic-rust-sdk` crate.
- `cargo info threatflux-anthropic-sdk` did not find an existing crate on June 23, 2026.

Acceptable alternatives if preferred:
- `threatflux-anthropic-rust-sdk`
- `threatflux-anthropic`
- Keep `threatflux`, but this is less descriptive and should only be used if the project intentionally wants that broad crate name.

## Files To Modify

- `Cargo.toml`
  - Set `package.name` to the final ThreatFlux-owned crate name.
  - Set `repository = "https://github.com/ThreatFlux/anthropic_rust_sdk"`.
  - Set `documentation = "https://docs.rs/<crate-name>"`.
  - Consider adding `homepage = "https://github.com/ThreatFlux/anthropic_rust_sdk"`.
  - Consider trimming generated/assistant author entries before first public publish.

- `README.md`
  - Update crates.io badge, docs.rs badge, install snippet, imports, and support links.
  - Replace mixed `anthropic_rust_sdk` and `threatflux` examples with the final crate name.
  - Add a short note if the Rust import path differs from the package name because hyphenated package names import with underscores.

- `src/lib.rs`
  - Update doc examples from `threatflux::...` to the final Rust crate import path.

- `.github/workflows/release.yml`
  - Replace `cargo publish --dry-run` with real `cargo publish` only after separate package verification passes.
  - Verify the package name and version against `Cargo.toml` before publish.
  - Verify publication using the final crate name, not `anthropic_rust_sdk`.
  - Remove the binary release job or change it to package actual binaries (`check_my_usage`, `test_api`) only if binary artifacts are truly wanted.

- `.github/workflows/ci.yml`
  - Add or confirm `cargo package --locked` / `cargo publish --dry-run --locked` coverage before release.

## Implementation Steps

1. Finalize the crate name.
   - Recommended: `threatflux-anthropic-sdk`.
   - Re-check crates.io immediately before editing/publishing because name availability can change.

2. Update package metadata.
   - Change `Cargo.toml` package name and metadata URLs.
   - Keep version `0.1.0` only if this crate has never been published under the chosen name.

3. Align docs and examples.
   - Update README dependency snippet to:

     ```toml
     threatflux-anthropic-sdk = "0.1.0"
     ```

   - Update Rust imports to use the normalized crate import path:

     ```rust
     use threatflux_anthropic_sdk::{Client, builders::MessageBuilder};
     ```

4. Fix release workflow.
   - Split release into:
     - verify: `cargo test --all-targets --no-run`, `cargo doc --no-deps`, `cargo package --locked`
     - publish: `cargo publish --locked`
   - Ensure publish depends only on successful verification, not on broken binary artifact packaging.
   - Authenticate with crates.io trusted publishing: give the publish job `id-token: write` and the `crates-io` environment, and get a short-lived token from `rust-lang/crates-io-auth-action`. Do not store a long-lived registry token.

5. Decide binary release policy.
   - Preferred: remove `build-release` for this SDK library release.
   - Alternative: package the existing binary names, not `anthropic_rust_sdk`.

6. Verify locally before merge.
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-targets --no-run`
   - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
   - `cargo package --locked`
   - `cargo publish --dry-run --locked`

7. Publish.
   - Confirm the crate's crates.io trusted publisher matches the workflow: owner `ThreatFlux`, repository `anthropic_rust_sdk`, workflow `release.yml`, environment `crates-io`.
   - Publish once from CI or locally, not both.
   - Add additional crates.io owners/team members after the first publish if needed.

## Testing Strategy

- Compile-only validation is enough for publish mechanics:
  - `cargo test --all-targets --no-run`
  - `cargo package --locked`
  - `cargo publish --dry-run --locked`
- Docs validation:
  - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- Optional behavior validation:
  - Run normal unit/integration tests without `real_api_tests`.
  - Run real API tests only with `ANTHROPIC_API_KEY` and `RUN_REAL_API_TESTS=true`.

## Risks

- crates.io names are permanent after first publish; choose carefully.
- Name availability can change between planning and publishing.
- Renaming the crate changes the public dependency/import path and requires README/doc updates.
- Publishing with stale metadata cannot be fully corrected for already-published versions; later versions can fix metadata but the original upload remains in history.
- The current GitHub release workflow can create a GitHub release before later publish steps fail.

## Decisions

- Crate name: `threatflux-anthropic-sdk`.
- Auto-versioning: Release Please with `release-type = "rust"` and manifest version tracking.
- crates.io authentication: trusted publishing (GitHub OIDC) from `release.yml` in the `crates-io` environment; no stored registry token.
- GitHub Packages target: GHCR image `ghcr.io/threatflux/anthropic-rust-sdk`, because GitHub Packages does not provide a native Cargo registry.
- GitHub releases publish the library crate and attach `.crate` plus checksum assets.
- `plans/` is excluded from the crates.io package archive.
