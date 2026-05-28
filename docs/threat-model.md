# Threat Model

Ghost-Env is designed to reduce accidental secret exposure caused by AI terminal agents reading project files for context.

## Protected Assets

- API keys
- Database URLs
- Cloud provider credentials
- Webhook secrets
- Tokens and signing keys stored in `.env`

## Primary Threat

An AI terminal agent reads `.env` through ordinary filesystem access and sends the contents to a remote inference service as context.

Ghost-Env mitigates this by replacing `.env` with fake values and moving real secrets into an encrypted vault.

## Mitigated Risks

| Risk | Mitigation |
| --- | --- |
| AI reads `.env` | `.env` contains fake mask values |
| AI reads `.env.ghost` | Vault is encrypted |
| Accidental commit of `.env` | Committed file should contain fake values |
| Shell history from `export SECRET=...` | `ghost-env run` avoids manual shell exports |

## Partial Mitigations

| Risk | Notes |
| --- | --- |
| App logs environment values | Ghost-Env cannot prevent application logging |
| CI or shell wrappers capture command output | Real secrets are only exposed if launched code prints them |
| `get` or `export` misuse | These commands intentionally print real values |

## Non-Goals

Ghost-Env does not protect against:

- Malicious code intentionally executed under `ghost-env run`
- Root/admin memory scraping
- Compromised operating systems
- Compromised OS keychain
- Secrets already committed to git history
- Network exfiltration performed by the launched child process

## User Safety Guidance

- Review generated scripts before running them with `ghost-env run`.
- Treat `ghost-env get` and `ghost-env export` as sensitive commands.
- Do not upload `.env.ghost` and keychain data together.
- Rotate any secret that was previously present in a real committed `.env`.

## Security Reporting

Report vulnerabilities through [SECURITY.md](../SECURITY.md).
