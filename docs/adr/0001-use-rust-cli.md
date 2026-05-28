# ADR-0001: Use a Rust CLI

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Maintainers

## Context

Ghost-Env needs to run locally, handle secrets carefully, integrate with OS keychains, encrypt data, and spawn child processes with predictable behavior.

## Decision

Build Ghost-Env as a Rust command-line application.

## Options Considered

### Rust CLI

| Dimension | Assessment |
| --- | --- |
| Complexity | Medium |
| Runtime dependency | Low |
| Security fit | High |
| Cross-platform fit | High |

Pros:

- Native binary distribution.
- Strong ecosystem for CLI parsing, encryption, keychain access, and zeroization.
- Good process spawning APIs.

Cons:

- More compile-time complexity than scripting languages.
- Cross-platform keychain behavior needs testing.

### Node.js CLI

| Dimension | Assessment |
| --- | --- |
| Complexity | Low |
| Runtime dependency | High |
| Security fit | Medium |
| Cross-platform fit | High |

Pros:

- Easy installation for JavaScript users.
- Fast iteration.

Cons:

- Requires a runtime.
- Larger dependency surface for a security-sensitive local tool.

## Consequences

- Releases must produce native binaries.
- CI must test multiple operating systems.
- Contributors need Rust tooling.
