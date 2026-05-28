use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use tempfile::NamedTempFile;
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"GHOSTENV";
const VERSION: u32 = 1;
const NONCE_LEN: usize = 12;

pub type SecretMap = BTreeMap<String, Zeroizing<String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultStatus {
    pub exists: bool,
    pub entry_count: usize,
}

pub fn empty_status() -> VaultStatus {
    VaultStatus {
        exists: false,
        entry_count: 0,
    }
}

pub fn read_vault(path: &Path, master_key: &[u8]) -> Result<SecretMap> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    decode_vault(&bytes, master_key)
}

pub fn write_vault(path: &Path, master_key: &[u8], entries: &SecretMap) -> Result<()> {
    let bytes = encode_vault(master_key, entries)?;
    atomic_write(path, &bytes)
}

pub fn inspect_vault(path: &Path, master_key: &[u8]) -> Result<VaultStatus> {
    if !path.exists() {
        return Ok(empty_status());
    }
    let entries = read_vault(path, master_key)?;
    Ok(VaultStatus {
        exists: true,
        entry_count: entries.len(),
    })
}

fn encode_vault(master_key: &[u8], entries: &SecretMap) -> Result<Vec<u8>> {
    let cipher = cipher(master_key)?;
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_be_bytes());

    for (key, value) in entries {
        let key_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let key_ct = cipher
            .encrypt(
                &key_nonce,
                Payload {
                    msg: key.as_bytes(),
                    aad: b"ghost-env:v1:key",
                },
            )
            .context("failed to encrypt key")?;

        let value_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let value_ct = cipher
            .encrypt(
                &value_nonce,
                Payload {
                    msg: value.as_bytes(),
                    aad: key.as_bytes(),
                },
            )
            .context("failed to encrypt value")?;

        out.extend_from_slice(key_nonce.as_slice());
        out.extend_from_slice(value_nonce.as_slice());
        write_len(&mut out, key_ct.len())?;
        write_len(&mut out, value_ct.len())?;
        out.extend_from_slice(&key_ct);
        out.extend_from_slice(&value_ct);
    }

    Ok(out)
}

fn decode_vault(bytes: &[u8], master_key: &[u8]) -> Result<SecretMap> {
    let cipher = cipher(master_key)?;
    let mut cursor = Cursor::new(bytes);
    let mut magic = [0u8; 8];
    cursor
        .read_exact(&mut magic)
        .context("vault file is too short")?;
    if &magic != MAGIC {
        bail!("invalid vault magic");
    }
    let version = read_u32(&mut cursor)?;
    if version != VERSION {
        bail!("unsupported vault version {version}");
    }
    let count = read_u32(&mut cursor)?;
    let mut entries = SecretMap::new();

    for _ in 0..count {
        let mut key_nonce = [0u8; NONCE_LEN];
        let mut value_nonce = [0u8; NONCE_LEN];
        cursor
            .read_exact(&mut key_nonce)
            .context("truncated key nonce")?;
        cursor
            .read_exact(&mut value_nonce)
            .context("truncated value nonce")?;
        let key_len = read_u32(&mut cursor)? as usize;
        let value_len = read_u32(&mut cursor)? as usize;
        let key_ct = read_vec(&mut cursor, key_len)?;
        let value_ct = read_vec(&mut cursor, value_len)?;

        let key_bytes = cipher
            .decrypt(
                Nonce::from_slice(&key_nonce),
                Payload {
                    msg: &key_ct,
                    aad: b"ghost-env:v1:key",
                },
            )
            .context("failed to decrypt vault key")?;
        let key = String::from_utf8(key_bytes).context("vault key is not valid UTF-8")?;
        let value_bytes = cipher
            .decrypt(
                Nonce::from_slice(&value_nonce),
                Payload {
                    msg: &value_ct,
                    aad: key.as_bytes(),
                },
            )
            .context("failed to decrypt vault value")?;
        let value = String::from_utf8(value_bytes).context("vault value is not valid UTF-8")?;
        entries.insert(key, Zeroizing::new(value));
    }

    if cursor.position() != bytes.len() as u64 {
        bail!("vault has trailing bytes");
    }

    Ok(entries)
}

fn cipher(master_key: &[u8]) -> Result<ChaCha20Poly1305> {
    if master_key.len() != 32 {
        bail!("master key must be 32 bytes");
    }
    Ok(ChaCha20Poly1305::new_from_slice(master_key).expect("checked key length"))
}

fn write_len(out: &mut Vec<u8>, len: usize) -> Result<()> {
    let len = u32::try_from(len).map_err(|_| anyhow!("vault entry is too large"))?;
    out.extend_from_slice(&len.to_be_bytes());
    Ok(())
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf).context("truncated u32")?;
    Ok(u32::from_be_bytes(buf))
}

fn read_vec(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    cursor
        .read_exact(&mut buf)
        .context("truncated ciphertext")?;
    Ok(buf)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let mut temp =
        NamedTempFile::new_in(parent).context("failed to create temporary vault file")?;
    std::io::Write::write_all(&mut temp, bytes).context("failed to write vault file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .context("failed to set vault file permissions")?;
    }

    temp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Zeroizing<Vec<u8>> {
        Zeroizing::new(vec![7u8; 32])
    }

    #[test]
    fn vault_roundtrips_entries() {
        let mut entries = SecretMap::new();
        entries.insert(
            "DATABASE_URL".into(),
            Zeroizing::new("postgres://real".into()),
        );
        entries.insert("OPENAI_API_KEY".into(), Zeroizing::new("sk-real".into()));

        let encoded = encode_vault(&key(), &entries).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("sk-real"));
        let decoded = decode_vault(&encoded, &key()).unwrap();

        assert_eq!(decoded.get("OPENAI_API_KEY").unwrap().as_str(), "sk-real");
        assert_eq!(
            decoded.get("DATABASE_URL").unwrap().as_str(),
            "postgres://real"
        );
    }

    #[test]
    fn tampering_fails() {
        let mut entries = SecretMap::new();
        entries.insert("A".into(), Zeroizing::new("B".into()));
        let mut encoded = encode_vault(&key(), &entries).unwrap();
        let last = encoded.last_mut().unwrap();
        *last ^= 0x80;

        assert!(decode_vault(&encoded, &key()).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let mut entries = SecretMap::new();
        entries.insert("A".into(), Zeroizing::new("B".into()));
        let encoded = encode_vault(&key(), &entries).unwrap();
        let wrong = Zeroizing::new(vec![8u8; 32]);

        assert!(decode_vault(&encoded, &wrong).is_err());
    }
}
