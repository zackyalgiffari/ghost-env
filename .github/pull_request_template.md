## Summary

Describe the change and why it is needed.

## Security Impact

Does this affect vault encryption, keychain access, masking, process spawning, file replacement, `get`, or `export`?

## Testing

Paste the checks you ran:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Checklist

- [ ] I used fake secrets in tests and examples.
- [ ] I updated docs for user-facing changes.
- [ ] I added or updated tests for behavior changes.
- [ ] I considered the threat model for security-sensitive changes.
