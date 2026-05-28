use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueHint};
use zeroize::Zeroizing;

use crate::envfile::{EnvEntry, parse_assignment, parse_env_file, render_env};
use crate::keychain::{KeyStore, OsKeyStore, account_for_vault, generate_master_key};
use crate::mask::generate_mask;
use crate::vault::{SecretMap, inspect_vault, read_vault, write_vault};

const VAULT_FILE: &str = ".env.ghost";
const MASK_FILE: &str = ".env";
const RULES_FILE: &str = ".ghostignore";

#[derive(Debug, Parser)]
#[command(name = "ghost-env")]
#[command(version, about = "Protect secrets from AI terminal context leaks.")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a project vault and store its master key in the OS keychain.
    Init,
    /// Add or update one secret.
    Set {
        /// Secret assignment in KEY=value form.
        assignment: String,
    },
    /// Print one decrypted secret value.
    Get {
        /// Secret key to read.
        key: String,
    },
    /// Remove one secret.
    Unset {
        /// Secret key to remove.
        key: String,
    },
    /// List secret names without values.
    List,
    /// Export decrypted secrets as KEY=value lines.
    Export,
    /// Check vault and keychain status.
    Status,
    /// Import a real .env file into the vault and replace it with a mask.
    Protect {
        /// Path to the .env file to protect.
        #[arg(default_value = MASK_FILE)]
        path: PathBuf,
    },
    /// Regenerate the fake .env mask from vault keys.
    Mask,
    /// Run a command with real secrets injected into its environment.
    Run {
        /// Command and arguments to execute.
        #[arg(
            required = true,
            trailing_var_arg = true,
            num_args = 1..,
            allow_hyphen_values = true,
            value_hint = ValueHint::CommandWithArguments
        )]
        command: Vec<OsString>,
    },
}

pub fn run() -> Result<()> {
    let code = run_with_store(OsKeyStore)?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

pub fn run_with_store(store: impl KeyStore) -> Result<i32> {
    let cli = Cli::parse();
    execute(
        cli,
        &store,
        &std::env::current_dir().context("failed to read current directory")?,
    )
}

#[cfg(test)]
pub fn execute_args_with_store<I, T>(args: I, store: &impl KeyStore, cwd: &Path) -> Result<i32>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    execute(Cli::parse_from(args), store, cwd)
}

fn execute(cli: Cli, store: &impl KeyStore, cwd: &Path) -> Result<i32> {
    let vault_path = cwd.join(VAULT_FILE);
    let rules_path = cwd.join(RULES_FILE);

    match cli.command {
        Commands::Init => {
            init(store, &vault_path)?;
            Ok(0)
        }
        Commands::Set { assignment } => {
            let entry = parse_assignment(&assignment)?;
            let key = load_or_create_master_key(store, &vault_path)?;
            let mut secrets = load_or_empty(&vault_path, &key)?;
            secrets.insert(entry.key, Zeroizing::new(entry.value));
            write_vault(&vault_path, &key, &secrets)?;
            println!("secret saved");
            Ok(0)
        }
        Commands::Get { key } => {
            let key = checked_key_name(&key)?;
            let master_key = load_master_key(store, &vault_path)?;
            let secrets = read_vault(&vault_path, &master_key)?;
            let value = secrets
                .get(key)
                .ok_or_else(|| anyhow!("secret '{key}' not found"))?;
            println!("{}", value.as_str());
            Ok(0)
        }
        Commands::Unset { key } => {
            let key = checked_key_name(&key)?;
            let master_key = load_master_key(store, &vault_path)?;
            let mut secrets = read_vault(&vault_path, &master_key)?;
            if secrets.remove(key).is_none() {
                bail!("secret '{key}' not found");
            }
            write_vault(&vault_path, &master_key, &secrets)?;
            println!("secret removed");
            Ok(0)
        }
        Commands::List => {
            let master_key = load_master_key(store, &vault_path)?;
            let secrets = read_vault(&vault_path, &master_key)?;
            for key in secrets.keys() {
                println!("{key}");
            }
            Ok(0)
        }
        Commands::Export => {
            let master_key = load_master_key(store, &vault_path)?;
            let secrets = read_vault(&vault_path, &master_key)?;
            for (key, value) in secrets {
                println!("{}={}", key, value.as_str());
            }
            Ok(0)
        }
        Commands::Status => status(store, &vault_path),
        Commands::Protect { path } => protect(store, cwd, &vault_path, &rules_path, &path),
        Commands::Mask => regenerate_mask(store, cwd, &vault_path, &rules_path),
        Commands::Run { command } => run_child(store, &vault_path, command),
    }
}

fn init(store: &impl KeyStore, vault_path: &Path) -> Result<()> {
    let account = account_for_vault(vault_path)?;
    let master_key = match store.get_key(&account)? {
        Some(key) => key,
        None => {
            let key = generate_master_key();
            store.set_key(&account, &key)?;
            key
        }
    };

    if !vault_path.exists() {
        write_vault(vault_path, &master_key, &SecretMap::new())?;
    }

    println!("initialized {}", vault_path.display());
    Ok(())
}

fn status(store: &impl KeyStore, vault_path: &Path) -> Result<i32> {
    let account = account_for_vault(vault_path)?;
    let Some(master_key) = store.get_key(&account)? else {
        println!("keychain: missing");
        println!(
            "vault: {}",
            if vault_path.exists() {
                "present"
            } else {
                "missing"
            }
        );
        return Ok(0);
    };

    let status = inspect_vault(vault_path, &master_key)?;
    println!("keychain: ok");
    println!("vault: {}", if status.exists { "ok" } else { "missing" });
    println!("entries: {}", status.entry_count);
    Ok(0)
}

fn protect(
    store: &impl KeyStore,
    cwd: &Path,
    vault_path: &Path,
    rules_path: &Path,
    env_path: &Path,
) -> Result<i32> {
    let env_path = resolve_under_cwd(cwd, env_path);
    let entries = parse_env_file(&env_path)?;
    let master_key = load_or_create_master_key(store, vault_path)?;
    let mut secrets = load_or_empty(vault_path, &master_key)?;
    for EnvEntry { key, value } in entries {
        secrets.insert(key, Zeroizing::new(value));
    }
    write_vault(vault_path, &master_key, &secrets)?;
    write_mask(&env_path, rules_path, &secrets)?;
    println!("protected {}", env_path.display());
    Ok(0)
}

fn regenerate_mask(
    store: &impl KeyStore,
    cwd: &Path,
    vault_path: &Path,
    rules_path: &Path,
) -> Result<i32> {
    let master_key = load_master_key(store, vault_path)?;
    let secrets = read_vault(vault_path, &master_key)?;
    write_mask(&cwd.join(MASK_FILE), rules_path, &secrets)?;
    println!("mask regenerated");
    Ok(0)
}

fn write_mask(path: &Path, rules_path: &Path, secrets: &SecretMap) -> Result<()> {
    let mask = generate_mask(secrets, rules_path)?;
    let rendered = render_env(&mask);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temporary mask beside {}", path.display()))?;
    std::io::Write::write_all(&mut tmp, rendered.as_bytes()).context("failed to write mask")?;
    tmp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

fn run_child(store: &impl KeyStore, vault_path: &Path, command: Vec<OsString>) -> Result<i32> {
    let master_key = load_master_key(store, vault_path)?;
    let secrets = read_vault(vault_path, &master_key)?;
    let (program, args) = command
        .split_first()
        .ok_or_else(|| anyhow!("missing command to run"))?;

    eprintln!(
        "Ghost-Env protects against context leaking, not malicious runtime code. Review scripts before running them with real secrets."
    );

    let mut child = Command::new(program);
    child.args(args);
    for (key, value) in &secrets {
        child.env(key, value.as_str());
    }
    let status = child
        .status()
        .with_context(|| format!("failed to run {:?}", program))?;
    if let Some(code) = status.code() {
        return Ok(code);
    }
    bail!("child process terminated by signal");
}

fn load_or_empty(path: &Path, master_key: &[u8]) -> Result<SecretMap> {
    if path.exists() {
        read_vault(path, master_key)
    } else {
        Ok(SecretMap::new())
    }
}

fn load_or_create_master_key(
    store: &impl KeyStore,
    vault_path: &Path,
) -> Result<Zeroizing<Vec<u8>>> {
    let account = account_for_vault(vault_path)?;
    match store.get_key(&account)? {
        Some(key) => Ok(key),
        None => {
            let key = generate_master_key();
            store.set_key(&account, &key)?;
            Ok(key)
        }
    }
}

fn load_master_key(store: &impl KeyStore, vault_path: &Path) -> Result<Zeroizing<Vec<u8>>> {
    let account = account_for_vault(vault_path)?;
    store
        .get_key(&account)?
        .ok_or_else(|| anyhow!("no master key found; run `ghost-env init` first"))
}

fn checked_key_name(key: &str) -> Result<&str> {
    crate::envfile::validate_key(key)?;
    Ok(key)
}

fn resolve_under_cwd(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

#[allow(dead_code)]
fn remove_key_for_tests(store: &impl KeyStore, vault_path: &Path) -> Result<()> {
    store.delete_key(&account_for_vault(vault_path)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use assert_cmd::Command as AssertCommand;
    use predicates::prelude::*;

    use crate::keychain::tests::MemoryKeyStore;

    use super::*;

    #[test]
    fn binary_help_mentions_commands() {
        let mut cmd = AssertCommand::cargo_bin("ghost-env").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("protect"))
            .stdout(predicate::str::contains("run"));
    }

    #[test]
    fn protects_and_regenerates_mask_with_memory_key_store() {
        let tmp = tempfile::tempdir().unwrap();
        let store = MemoryKeyStore::default();
        fs::write(
            tmp.path().join(".env"),
            "OPENAI_API_KEY=sk-real\nDATABASE_URL=postgres://prod\nFLAG=true\n",
        )
        .unwrap();
        fs::write(tmp.path().join(".ghostignore"), "FLAG=use_real\n").unwrap();

        execute_args_with_store(["ghost-env", "protect", ".env"], &store, tmp.path()).unwrap();

        let mask = fs::read_to_string(tmp.path().join(".env")).unwrap();
        assert!(mask.contains("OPENAI_API_KEY=sk-proj-"));
        assert!(mask.contains("DATABASE_URL=postgres://user:password@localhost:5432/dbname"));
        assert!(mask.contains("FLAG=true"));
        assert!(!mask.contains("sk-real"));
        assert!(tmp.path().join(".env.ghost").exists());

        execute_args_with_store(["ghost-env", "mask"], &store, tmp.path()).unwrap();
        let regenerated = fs::read_to_string(tmp.path().join(".env")).unwrap();
        assert!(regenerated.contains("OPENAI_API_KEY=sk-proj-"));
    }

    #[test]
    fn set_and_unset_update_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let store = MemoryKeyStore::default();

        execute_args_with_store(["ghost-env", "init"], &store, tmp.path()).unwrap();
        execute_args_with_store(["ghost-env", "set", "TOKEN=real-token"], &store, tmp.path())
            .unwrap();

        let key = load_master_key(&store, &tmp.path().join(".env.ghost")).unwrap();
        let secrets = read_vault(&tmp.path().join(".env.ghost"), &key).unwrap();
        assert_eq!(secrets.get("TOKEN").unwrap().as_str(), "real-token");

        execute_args_with_store(["ghost-env", "unset", "TOKEN"], &store, tmp.path()).unwrap();
        let secrets = read_vault(&tmp.path().join(".env.ghost"), &key).unwrap();
        assert!(!secrets.contains_key("TOKEN"));
    }

    #[cfg(unix)]
    #[test]
    fn run_injects_secret_into_child_environment() {
        let tmp = tempfile::tempdir().unwrap();
        let store = MemoryKeyStore::default();

        execute_args_with_store(
            ["ghost-env", "set", "GHOST_ENV_TEST=visible"],
            &store,
            tmp.path(),
        )
        .unwrap();
        let code = execute_args_with_store(
            [
                "ghost-env",
                "run",
                "sh",
                "-c",
                "test \"$GHOST_ENV_TEST\" = visible",
            ],
            &store,
            tmp.path(),
        )
        .unwrap();

        assert_eq!(code, 0);
    }
}
