use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
}

pub fn parse_env_file(path: &Path) -> Result<Vec<EnvEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read env file {}", path.display()))?;
    parse_env(&content)
}

pub fn parse_env(content: &str) -> Result<Vec<EnvEntry>> {
    let mut entries = Vec::new();
    for (idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);
        let (key, raw_value) = line
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid .env line {}: missing '='", idx + 1))?;
        let key = key.trim();
        validate_key(key)?;
        entries.push(EnvEntry {
            key: key.to_owned(),
            value: parse_value(raw_value.trim())?,
        });
    }
    Ok(entries)
}

pub fn parse_assignment(input: &str) -> Result<EnvEntry> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| anyhow!("expected KEY=value"))?;
    let key = key.trim();
    validate_key(key)?;
    Ok(EnvEntry {
        key: key.to_owned(),
        value: parse_value(value.trim())?,
    })
}

pub fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(anyhow!("environment key cannot be empty"));
    }
    let mut chars = key.chars();
    let first = chars.next().expect("checked non-empty key");
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(anyhow!("invalid environment key '{key}'"));
    }
    if !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        return Err(anyhow!("invalid environment key '{key}'"));
    }
    Ok(())
}

pub fn render_env(entries: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (key, value) in entries {
        out.push_str(key);
        out.push('=');
        out.push_str(&quote_if_needed(value));
        out.push('\n');
    }
    out
}

fn parse_value(value: &str) -> Result<String> {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return Ok(unescape_quoted(
                &value[1..value.len() - 1],
                bytes[0] == b'"',
            ));
        }
    }

    let without_comment = value
        .split_once(" #")
        .map_or(value, |(before_comment, _)| before_comment)
        .trim_end();
    Ok(without_comment.to_owned())
}

fn unescape_quoted(value: &str, double_quoted: bool) -> String {
    if !double_quoted {
        return value.to_owned();
    }
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn quote_if_needed(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch == '#' || ch == '\'' || ch == '"')
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_env_content() {
        let parsed = parse_env("A=1\nexport B=\"two words\"\n# skip\nC='raw # value'\n").unwrap();
        assert_eq!(
            parsed,
            vec![
                EnvEntry {
                    key: "A".into(),
                    value: "1".into()
                },
                EnvEntry {
                    key: "B".into(),
                    value: "two words".into()
                },
                EnvEntry {
                    key: "C".into(),
                    value: "raw # value".into()
                }
            ]
        );
    }

    #[test]
    fn rejects_invalid_keys() {
        assert!(parse_assignment("1BAD=value").is_err());
        assert!(parse_assignment("BAD-KEY=value").is_err());
    }
}
