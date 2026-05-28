# Ghost-Env

[![CI](https://github.com/zackyalgiffari/ghost-env/actions/workflows/ci.yml/badge.svg)](https://github.com/zackyalgiffari/ghost-env/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Protect project secrets from AI terminal context leaks. Ghost-Env replaces a real `.env` file with a structurally valid decoy, stores real values in an encrypted `.env.ghost` vault, and injects the real values only into child process environments.

## Project Status

Ghost-Env is early software. The core CLI is implemented and tested, but public releases and package-manager distribution are still being prepared.

## Why

AI terminal agents often read project files for context. If a real `.env` file is present, secrets can be copied into a remote model prompt. Ghost-Env keeps the file shape agents expect while removing real secrets from plaintext project files.

## Install

From source:

```bash
cargo install --path .
```

After crates.io publishing:

```bash
cargo install ghost-env
```

Release automation is configured for GitHub Releases, shell and PowerShell installers, and the Homebrew tap `zackyalgiffari/homebrew-tap`.

## Quick Start

Protect an existing `.env` file:

```bash
ghost-env init
ghost-env protect .env
ghost-env run npm run dev
```

During local development before installing:

```bash
cargo run -- init
cargo run -- protect .env
cargo run -- run npm run dev
```

`protect` imports the real `.env`, writes encrypted values to `.env.ghost`, and replaces `.env` with generated fake values.

## Commands

```text
ghost-env init
ghost-env set KEY=value
ghost-env get KEY
ghost-env unset KEY
ghost-env list
ghost-env export
ghost-env status
ghost-env protect .env
ghost-env mask
ghost-env run <command...>
```

Important behavior:

- `get` and `export` intentionally print real secrets.
- `list` prints key names only.
- `run` injects real secrets into the child process environment.
- `mask` regenerates the fake `.env` file from vault keys.

## Mask Rules

Add `.ghostignore` to customize generated fake values:

```ini
INTERNAL_FLAG = use_real
MY_CUSTOM_KEY = format:uuid
WEBHOOK_URL = https://webhook.site/fake-uuid-here
```

Supported forced formats:

```text
uuid
jwt
url
database_url
dsn
hex32
secret
```

## Security Model

Ghost-Env mitigates accidental context leaking when AI terminal agents read project files such as `.env`.

- Real secrets are stored in `.env.ghost`, encrypted with ChaCha20-Poly1305.
- The 32-byte master key is stored in the OS keychain.
- Vault keys and values are encrypted independently with fresh 96-bit nonces.
- The `.env` file contains generated fake values only.

Ghost-Env is not a sandbox. If malicious code runs under `ghost-env run`, that code can read the injected environment variables at runtime. Review generated scripts before running them with real secrets.

More detail:

- [Architecture](docs/architecture.md)
- [Threat model](docs/threat-model.md)
- [Release process](docs/release.md)
- [Roadmap](docs/roadmap.md)
- [Architecture decisions](docs/adr/)

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Package verification:

```bash
cargo package --allow-dirty --offline
```

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), and [SECURITY.md](SECURITY.md) before contributing.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
