use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use rand::distributions::{Alphanumeric, DistString};
use rand::{Rng, RngCore};
use regex::Regex;
use uuid::Uuid;

use crate::vault::SecretMap;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    UseReal,
    Format(MaskFormat),
    Literal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaskFormat {
    Uuid,
    Jwt,
    Url,
    DatabaseUrl,
    Hex32,
}

pub fn generate_mask(secrets: &SecretMap, rules_path: &Path) -> Result<BTreeMap<String, String>> {
    let rules = load_rules(rules_path)?;
    let mut out = BTreeMap::new();
    for (key, value) in secrets {
        let fake = match rules.get(key) {
            Some(Rule::UseReal) => value.to_string(),
            Some(Rule::Format(format)) => fake_for_format(*format),
            Some(Rule::Literal(literal)) => literal.clone(),
            None => fake_for_secret(key, value),
        };
        out.insert(key.clone(), fake);
    }
    Ok(out)
}

fn load_rules(path: &Path) -> Result<BTreeMap<String, Rule>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read mask rules {}", path.display()))?;
    parse_rules(&content)
}

fn parse_rules(content: &str) -> Result<BTreeMap<String, Rule>> {
    let mut rules = BTreeMap::new();
    for (idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid .ghostignore line {}: missing '='", idx + 1))?;
        let key = key.trim();
        crate::envfile::validate_key(key)?;
        let value = value
            .split_once(" #")
            .map_or(value, |(before_comment, _)| before_comment)
            .trim();
        let rule = if value == "use_real" {
            Rule::UseReal
        } else if let Some(format) = value.strip_prefix("format:") {
            Rule::Format(match format.trim() {
                "uuid" => MaskFormat::Uuid,
                "jwt" => MaskFormat::Jwt,
                "url" => MaskFormat::Url,
                "database_url" | "dsn" => MaskFormat::DatabaseUrl,
                "hex32" | "secret" => MaskFormat::Hex32,
                other => return Err(anyhow!("unknown .ghostignore format '{other}'")),
            })
        } else {
            Rule::Literal(strip_quotes(value).to_owned())
        };
        rules.insert(key.to_owned(), rule);
    }
    Ok(rules)
}

fn fake_for_secret(key: &str, value: &str) -> String {
    let upper = key.to_ascii_uppercase();
    if upper == "OPENAI_API_KEY" {
        return format!("sk-proj-{}", alnum(48));
    }
    if upper.starts_with("STRIPE_") && upper.ends_with("_KEY") {
        return format!("sk_live_{}", alnum(24));
    }
    if upper == "AWS_ACCESS_KEY_ID" {
        return format!("AKIA{}", upper_alnum(16));
    }
    if upper == "AWS_SECRET_ACCESS_KEY" {
        return alnum(40);
    }
    if upper == "DATABASE_URL" || upper.ends_with("_DATABASE_URL") {
        return fake_database_url();
    }
    if upper.ends_with("_URL") || upper.ends_with("_URI") {
        return fake_url();
    }
    if upper == "PORT" {
        return "3000".to_owned();
    }
    if looks_like_uuid(value) && upper.ends_with("_ID") {
        return Uuid::new_v4().to_string();
    }
    if looks_like_jwt(value) {
        return fake_jwt();
    }
    if looks_like_database_url(value) {
        return fake_database_url();
    }
    if looks_like_url(value) {
        return fake_url();
    }
    if looks_like_uuid(value) {
        return Uuid::new_v4().to_string();
    }
    if upper.ends_with("_SECRET")
        || upper.ends_with("_KEY")
        || upper.ends_with("_TOKEN")
        || upper.contains("SECRET")
        || upper.contains("TOKEN")
    {
        return hex_string(32);
    }

    let len = value.chars().count().max(8);
    alnum(len)
}

fn fake_for_format(format: MaskFormat) -> String {
    match format {
        MaskFormat::Uuid => Uuid::new_v4().to_string(),
        MaskFormat::Jwt => fake_jwt(),
        MaskFormat::Url => fake_url(),
        MaskFormat::DatabaseUrl => fake_database_url(),
        MaskFormat::Hex32 => hex_string(32),
    }
}

fn fake_database_url() -> String {
    "postgres://user:password@localhost:5432/dbname".to_owned()
}

fn fake_url() -> String {
    "https://api.example.com/v1".to_owned()
}

fn fake_jwt() -> String {
    format!(
        "{}.{}.{}",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        "eyJzdWIiOiJnaG9zdC1lbnYiLCJpYXQiOjE3MDAwMDAwMDB9",
        alnum(43)
    )
}

fn looks_like_url(value: &str) -> bool {
    Regex::new(r"^https?://[^\s]+$")
        .expect("valid regex")
        .is_match(value)
}

fn looks_like_database_url(value: &str) -> bool {
    Regex::new(r"^(postgres|postgresql|mysql|mariadb|sqlite|sqlserver)://[^\s]+$")
        .expect("valid regex")
        .is_match(value)
}

fn looks_like_uuid(value: &str) -> bool {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$")
        .expect("valid regex")
        .is_match(value)
}

fn looks_like_jwt(value: &str) -> bool {
    Regex::new(r"^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$")
        .expect("valid regex")
        .is_match(value)
}

fn alnum(len: usize) -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

fn upper_alnum(len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect()
}

fn hex_string(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes / 2];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

fn strip_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::*;

    #[test]
    fn applies_builtin_masks() {
        assert!(fake_for_secret("OPENAI_API_KEY", "real").starts_with("sk-proj-"));
        assert!(fake_for_secret("STRIPE_SECRET_KEY", "real").starts_with("sk_live_"));
        assert_eq!(fake_for_secret("PORT", "8080"), "3000");
        assert_eq!(
            fake_for_secret("DATABASE_URL", "postgres://prod"),
            "postgres://user:password@localhost:5432/dbname"
        );
    }

    #[test]
    fn parses_rules() {
        let rules = parse_rules(
            "INTERNAL_FLAG = use_real\nMY_CUSTOM_KEY = format:uuid\nWEBHOOK_URL = https://example.test\n",
        )
        .unwrap();

        assert_eq!(rules.get("INTERNAL_FLAG"), Some(&Rule::UseReal));
        assert_eq!(
            rules.get("MY_CUSTOM_KEY"),
            Some(&Rule::Format(MaskFormat::Uuid))
        );
        assert_eq!(
            rules.get("WEBHOOK_URL"),
            Some(&Rule::Literal("https://example.test".into()))
        );
    }

    #[test]
    fn generates_mask_with_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let rules_path = tmp.path().join(".ghostignore");
        fs::write(&rules_path, "FLAG=use_real\nID=format:uuid\n").unwrap();
        let mut secrets = SecretMap::new();
        secrets.insert("FLAG".into(), Zeroizing::new("true".into()));
        secrets.insert("ID".into(), Zeroizing::new("real".into()));

        let mask = generate_mask(&secrets, &rules_path).unwrap();
        assert_eq!(mask["FLAG"], "true");
        assert!(looks_like_uuid(&mask["ID"]));
    }
}
