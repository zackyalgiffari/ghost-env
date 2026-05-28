# Release Process

Ghost-Env uses tag-based releases.

## Release Coordinates

- Repository: `zackyalgiffari/ghost-env`
- Crate: `ghost-env`
- Binary: `ghost-env`
- Homebrew tap: `zackyalgiffari/homebrew-tap`
- Winget package ID: `zackyalgiffari.ghost-env`

## Required Secrets

Configure these in GitHub Actions before publishing:

- `CARGO_REGISTRY_TOKEN` for crates.io
- GitHub token permissions for release artifact publishing
- Homebrew tap access for `zackyalgiffari/homebrew-tap`
- Winget publishing token or a manual Winget PR process

## Preflight

Run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo package --allow-dirty --offline
```

Review:

- `CHANGELOG.md`
- `README.md`
- `Cargo.toml` version
- Security-sensitive diffs

## Publish

1. Update `Cargo.toml` version.
2. Update `CHANGELOG.md`.
3. Commit release changes.
4. Create and push a tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. Monitor the Release workflow.
6. Verify generated artifacts and installers.
7. Publish or submit the Winget manifest if it is not fully automated.

## Rollback

If a release is broken:

- Mark the GitHub Release as prerelease or remove broken assets.
- Yank the crate version from crates.io if needed.
- Revoke affected installer manifests.
- Publish a patch release with the fix.
