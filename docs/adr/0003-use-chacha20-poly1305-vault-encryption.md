# ADR-0003: Use ChaCha20-Poly1305 Vault Encryption

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Maintainers

## Context

The vault needs authenticated encryption that works well in pure Rust and on machines without AES acceleration.

## Decision

Use ChaCha20-Poly1305 with fresh 96-bit nonces for encrypted vault records.

## Options Considered

### ChaCha20-Poly1305

| Dimension | Assessment |
| --- | --- |
| Complexity | Low |
| Security | High |
| Performance | High |
| Rust ecosystem fit | High |

Pros:

- Authenticated encryption.
- Strong Rust crate support.
- Good software performance across CPUs.

Cons:

- Nonce uniqueness must be preserved.

### AES-GCM

| Dimension | Assessment |
| --- | --- |
| Complexity | Low |
| Security | High |
| Performance | Hardware-dependent |
| Rust ecosystem fit | High |

Pros:

- Widely used authenticated encryption.
- Fast on CPUs with AES acceleration.

Cons:

- Software-only performance can be weaker.

## Consequences

- Vault writes must generate fresh nonces for every encrypted key and value.
- Tamper detection is part of normal vault decoding.
