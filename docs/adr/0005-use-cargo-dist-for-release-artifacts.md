# ADR-0005: Use cargo-dist for Release Artifacts

**Status:** Accepted
**Date:** 2026-05-28
**Deciders:** Maintainers

## Context

Ghost-Env should be installable as a native binary across common platforms. Manual release artifact generation is error-prone.

## Decision

Use cargo-dist for GitHub Release artifacts and installer generation.

## Options Considered

### cargo-dist

| Dimension | Assessment |
| --- | --- |
| Complexity | Medium |
| Rust fit | High |
| Release automation | High |
| Maintenance | Medium |

Pros:

- Built for Rust binary distribution.
- Supports GitHub Releases and installers.
- Reduces custom CI scripting.

Cons:

- Adds tool-specific release configuration.
- Package-manager publishing still needs credential setup and review.

### Custom GitHub Actions

| Dimension | Assessment |
| --- | --- |
| Complexity | High |
| Rust fit | Medium |
| Release automation | Medium |
| Maintenance | High |

Pros:

- Full control.
- No release framework dependency.

Cons:

- More custom code to maintain.
- Easier to miss platform-specific packaging details.

## Consequences

- Release workflow is tag-driven.
- Maintainers must configure required publishing secrets before public release.
