# ADR-0004: Use a Decoy Env Mask Plus Runtime Injection

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Maintainers

## Context

AI terminal agents often need project configuration context. Removing `.env` entirely can break assumptions or cause agents to ask for missing values. Keeping real `.env` files leaks secrets.

## Decision

Replace the real `.env` with a decoy mask and inject real values only when launching a child process through `ghost-env run`.

## Options Considered

### Decoy Mask and Runtime Injection

| Dimension | Assessment |
| --- | --- |
| Complexity | Medium |
| UX | High |
| Context-leak protection | High |
| Runtime protection | Limited |

Pros:

- AI agents see structurally valid fake values.
- Applications still receive real environment variables.
- Developers avoid manual shell exports.

Cons:

- Code run under `ghost-env run` can read real values.
- Mask generation needs provider-specific maintenance.

### Delete `.env` and Require Manual Export

| Dimension | Assessment |
| --- | --- |
| Complexity | Low |
| UX | Low |
| Context-leak protection | High |
| Runtime protection | Limited |

Pros:

- Simple.
- No decoy generation.

Cons:

- Breaks common workflows.
- Encourages unsafe shell history patterns.

## Consequences

- Documentation must clearly state that Ghost-Env is not a sandbox.
- Mask rules must avoid exposing real secrets by default.
