# ADR-0002: Store the Master Key in the OS Keychain

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Maintainers

## Context

The encrypted vault needs a master key. Writing that key next to the vault would weaken the design because AI agents and accidental commits can access project files.

## Decision

Store the 32-byte master key in the OS keychain. The keychain account is derived from the canonical `.env.ghost` path hash.

## Options Considered

### OS Keychain

| Dimension | Assessment |
| --- | --- |
| Complexity | Medium |
| Security | High |
| UX | Good |
| Portability | Medium |

Pros:

- Keeps key material outside the repository.
- Uses platform-native secret storage.
- Supports one key per project vault.

Cons:

- Headless Linux keychain availability can vary.
- Manual testing is needed across platforms.

### Passphrase Prompt

| Dimension | Assessment |
| --- | --- |
| Complexity | Medium |
| Security | Medium |
| UX | Lower |
| Portability | High |

Pros:

- No platform keychain dependency.
- Works in more minimal environments.

Cons:

- Users must remember or store the passphrase.
- More opportunity for shell history or prompt capture mistakes.

## Consequences

- CI tests use an in-memory key store abstraction.
- User-facing errors must explain missing or unavailable keychain backends.
