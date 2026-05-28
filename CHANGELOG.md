# Changelog

All notable changes to Ghost-Env will be documented in this file.

This project uses semantic versioning once stable releases begin.

## Unreleased

### Added

- Initial Rust CLI implementation.
- Encrypted `.env.ghost` vault using ChaCha20-Poly1305.
- OS keychain-backed per-project master key storage.
- Decoy `.env` mask generation with `.ghostignore` overrides.
- Runtime environment injection through `ghost-env run`.
- CI, release scaffolding, and open-source project documentation.
