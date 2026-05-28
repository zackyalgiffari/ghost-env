# Architecture

Ghost-Env is a local Rust CLI that removes real secrets from plaintext project files while preserving the developer workflow of running applications with environment variables.

## Goals

- Keep real secrets out of `.env` files that AI terminal agents may read.
- Store secrets encrypted on disk.
- Store the vault master key outside the repository in the OS keychain.
- Inject real values only into the child process launched by `ghost-env run`.
- Keep the CLI understandable and auditable.

## Components

| Component | Module | Responsibility |
| --- | --- | --- |
| CLI | `src/cli.rs` | Command parsing, command dispatch, process spawning |
| Vault | `src/vault.rs` | Binary vault encoding, encryption, decryption, atomic writes |
| Keychain | `src/keychain.rs` | Per-project master key storage and lookup |
| Env parser | `src/envfile.rs` | `.env` parsing, assignment parsing, mask rendering |
| Masking | `src/mask.rs` | Fake value generation and `.ghostignore` rules |

## Data Flow

### Protect

```text
real .env
  -> parse key/value entries
  -> load or create OS keychain master key
  -> encrypt entries into .env.ghost
  -> generate fake values
  -> replace .env with mask
```

The vault write happens before the mask replacement. If vault writing fails, the original `.env` is not intentionally replaced by Ghost-Env.

### Run

```text
ghost-env run <command...>
  -> fetch master key from OS keychain
  -> decrypt .env.ghost in memory
  -> spawn child process with decrypted env vars
  -> wait for child exit
  -> drop zeroizing secret buffers
```

The child process receives real environment variables. The parent shell does not receive exported values.

## Vault Format

The v1 vault is a binary file named `.env.ghost`.

```text
magic:       "GHOSTENV" (8 bytes)
version:     u32
entry_count: u32

entries:
  key_nonce:      12 bytes
  value_nonce:    12 bytes
  key_len:        u32
  value_len:      u32
  key_ciphertext: bytes, includes AEAD tag
  value_ciphertext: bytes, includes AEAD tag
```

Keys and values are encrypted independently with ChaCha20-Poly1305. Value encryption uses the decrypted key name as associated data so encrypted values cannot be silently moved between keys.

## Keychain Model

The OS keychain entry uses:

- Service: `ghost-env`
- Account: `sha256(canonical .env.ghost path)`

This allows multiple projects on the same machine to have separate vault master keys.

## Masking Pipeline

Mask generation uses this priority:

1. Exact `.ghostignore` rule for the key
2. Built-in key-name patterns
3. Original value shape inference
4. Same-length random alphanumeric fallback

Supported `.ghostignore` actions are:

- `use_real`
- `format:uuid`
- `format:jwt`
- `format:url`
- `format:database_url`
- `format:dsn`
- `format:hex32`
- literal fake values

## Trust Boundaries

Trusted:

- OS keychain
- Ghost-Env process memory
- Child process launched intentionally by the developer

Untrusted:

- Plaintext `.env` mask
- AI terminal agent file reads
- Repository contents visible to tools

Out of scope:

- Malicious code intentionally run under `ghost-env run`
- Root/admin memory inspection
- Compromised OS keychain

## Architecture Decisions

Accepted decisions are recorded in [docs/adr](adr/).
