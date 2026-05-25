use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedNote {
    pub file_path: String,
    pub file_name: String,
    pub meta: HashMap<String, Value>,
    pub body: String,
}

pub fn parse_file(path: &str) -> Result<ParsedNote> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Cannot read: {}", path))?;

    let (meta, body) = split_frontmatter(&raw);

    let file_name = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    Ok(ParsedNote {
        file_path: path.to_string(),
        file_name,
        meta,
        body: body.trim().to_string(),
    })
}

fn split_frontmatter(raw: &str) -> (HashMap<String, Value>, String) {
    if !raw.starts_with("---") {
        return (HashMap::new(), raw.to_string());
    }

    let after = &raw[3..];
    // Find closing ---
    let end = after.find("\n---");
    let Some(end_pos) = end else {
        return (HashMap::new(), raw.to_string());
    };

    let yaml_str = &after[..end_pos];
    let body = after[end_pos + 4..].to_string();

    let meta = parse_yaml_map(yaml_str);
    (meta, body)
}

/// Simple YAML key-value parser — handles:
///   key: value
///   key: "quoted value"
///   key:            (null)
///   tags:           (followed by - items on next lines)
fn parse_yaml_map(yaml: &str) -> HashMap<String, Value> {
    let mut map: HashMap<String, Value> = HashMap::new();
    let lines: Vec<&str> = yaml.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip comment and blank lines
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            i += 1;
            continue;
        }

        // Key: value line
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let raw_val = line[colon_pos + 1..].trim();

            // Check if next lines are a list (starts with "  - ")
            let mut list_items: Vec<Value> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j];
                if next.trim_start().starts_with("- ") {
                    let item = next.trim_start().trim_start_matches("- ").trim();
                    list_items.push(Value::String(unquote(item).to_string()));
                    j += 1;
                } else if next.trim().is_empty() {
                    j += 1;
                    break;
                } else {
                    break;
                }
            }

            if !list_items.is_empty() {
                map.insert(key, Value::Array(list_items));
                i = j;
                continue;
            }

            // Scalar value
            let val = parse_scalar(raw_val);
            if !key.is_empty() {
                map.insert(key, val);
            }
        }

        i += 1;
    }

    map
}

fn parse_scalar(s: &str) -> Value {
    if s.is_empty() {
        return Value::Null;
    }
    let s = unquote(s);
    match s {
        "true" | "yes" => Value::Bool(true),
        "false" | "no" => Value::Bool(false),
        "null" | "~" => Value::Null,
        _ => {
            // Try integer
            if let Ok(n) = s.parse::<i64>() {
                return Value::Number(n.into());
            }
            // Try float
            if let Ok(f) = s.parse::<f64>() {
                if let Some(n) = serde_json::Number::from_f64(f) {
                    return Value::Number(n);
                }
            }
            Value::String(s.to_string())
        }
    }
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}
