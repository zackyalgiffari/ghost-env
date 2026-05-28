use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub const SERVICE: &str = "ghost-env";

pub trait KeyStore {
    fn get_key(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn set_key(&self, account: &str, key: &[u8]) -> Result<()>;
    fn delete_key(&self, account: &str) -> Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeyStore;

impl KeyStore for OsKeyStore {
    fn get_key(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let entry = keyring::Entry::new(SERVICE, account)
            .with_context(|| format!("failed to open keychain entry for {account}"))?;

        match entry.get_password() {
            Ok(encoded) => {
                let decoded = STANDARD
                    .decode(encoded.as_bytes())
                    .context("stored master key is not valid base64")?;
                if decoded.len() != 32 {
                    return Err(anyhow!("stored master key has invalid length"));
                }
                Ok(Some(Zeroizing::new(decoded)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err).context("failed to read master key from OS keychain"),
        }
    }

    fn set_key(&self, account: &str, key: &[u8]) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, account)
            .with_context(|| format!("failed to open keychain entry for {account}"))?;
        entry
            .set_password(&STANDARD.encode(key))
            .context("failed to write master key to OS keychain")
    }

    fn delete_key(&self, account: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, account)
            .with_context(|| format!("failed to open keychain entry for {account}"))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err).context("failed to delete master key from OS keychain"),
        }
    }
}

pub fn generate_master_key() -> Zeroizing<Vec<u8>> {
    let mut key = Zeroizing::new(vec![0u8; 32]);
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn account_for_vault(path: &Path) -> Result<String> {
    let stable_path = stable_absolute_path(path)?;
    let digest = Sha256::digest(stable_path.to_string_lossy().as_bytes());
    Ok(hex::encode(digest))
}

fn stable_absolute_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .context("failed to canonicalize vault path");
    }

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("vault path must include a file name"))?;
    let base = match parent {
        Some(parent) if parent.exists() => parent
            .canonicalize()
            .context("failed to canonicalize vault parent directory")?,
        Some(parent) if parent.is_absolute() => parent.to_path_buf(),
        Some(parent) => std::env::current_dir()
            .context("failed to read current directory")?
            .join(parent),
        None => std::env::current_dir().context("failed to read current directory")?,
    };
    Ok(base.join(file_name))
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use anyhow::Result;
    use zeroize::Zeroizing;

    use super::KeyStore;

    #[derive(Clone, Default)]
    pub struct MemoryKeyStore {
        keys: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl KeyStore for MemoryKeyStore {
        fn get_key(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
            Ok(self
                .keys
                .lock()
                .expect("memory key store poisoned")
                .get(account)
                .cloned()
                .map(Zeroizing::new))
        }

        fn set_key(&self, account: &str, key: &[u8]) -> Result<()> {
            self.keys
                .lock()
                .expect("memory key store poisoned")
                .insert(account.to_owned(), key.to_vec());
            Ok(())
        }

        fn delete_key(&self, account: &str) -> Result<()> {
            self.keys
                .lock()
                .expect("memory key store poisoned")
                .remove(account);
            Ok(())
        }
    }
}
