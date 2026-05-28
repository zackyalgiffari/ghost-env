# Contributing

Thanks for helping improve Ghost-Env. This project handles developer secrets, so changes should be small, reviewable, and explicit about security impact.

## Development Setup

Prerequisites:

- Rust stable toolchain
- `cargo`, `rustfmt`, and `clippy`
- A platform keychain for manual end-to-end tests

Run the standard checks:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Pull Requests

Before opening a PR:

- Add or update tests for behavior changes.
- Update README or docs for user-facing changes.
- Keep security-sensitive changes easy to audit.
- Avoid printing real secret values except in commands that explicitly do so, such as `get` and `export`.

## Security-Sensitive Code

Changes to vault encryption, keychain access, masking, process spawning, or file replacement need extra care. Document the threat model impact in the PR body and add tests for failure cases such as corrupt vaults or missing keychain entries.

## Licensing

Unless explicitly stated otherwise, contributions are accepted under the project license: `MIT OR Apache-2.0`.
