# Security Policy

Ghost-Env is security-sensitive software. Please report vulnerabilities privately.

## Reporting a Vulnerability

Use GitHub private vulnerability reporting if it is enabled for this repository:

https://github.com/zackyalgiffari/ghost-env/security/advisories/new

If private vulnerability reporting is unavailable, open a minimal public issue asking for a private contact path. Do not include exploit details, real secrets, vault files, or key material in public issues.

## What To Include

- Affected version or commit
- Operating system and shell
- Reproduction steps using fake secrets only
- Impact assessment
- Any suggested fix or mitigation

## Scope

In scope:

- Vault confidentiality or integrity failures
- Keychain account/key handling issues
- Accidental plaintext secret persistence
- Mask generation behavior that exposes real values unexpectedly
- `run` command behavior that leaks secrets outside the child process environment

Out of scope:

- Malicious code intentionally run under `ghost-env run`
- Compromised operating systems or root/admin-level memory inspection
- Social engineering or physical access attacks
- Vulnerabilities in unrelated third-party applications launched by Ghost-Env

## Supported Versions

Until the first stable release, security fixes target `main` and the latest published prerelease.
