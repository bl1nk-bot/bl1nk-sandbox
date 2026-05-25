use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Command;

use crate::config::Config;
use crate::frontmatter::ParsedNote;

pub struct NotionClient {
    token: String,
    version: String,
    database_id: String,
}

impl NotionClient {
    pub fn new(cfg: &Config) -> Self {
        Self {
            token: cfg.notion_token.clone(),
            version: cfg.notion_version.clone(),
            database_id: cfg.notion_database_id.clone(),
        }
    }

    fn curl_post(&self, url: &str, body: &Value) -> Result<Value> {
        let body_str = serde_json::to_string(body)?;
        let out = Command::new("curl")
            .args([
                "-s", "-X", "POST", url,
                "-H", &format!("Authorization: Bearer {}", self.token),
                "-H", &format!("Notion-Version: {}", self.version),
                "-H", "Content-Type: application/json",
                "-d", &body_str,
            ])
            .output()?;
        let val: Value = serde_json::from_slice(&out.stdout)?;
        Ok(val)
    }

    fn curl_patch(&self, url: &str, body: &Value) -> Result<Value> {
        let body_str = serde_json::to_string(body)?;
        let out = Command::new("curl")
            .args([
                "-s", "-X", "PATCH", url,
                "-H", &format!("Authorization: Bearer {}", self.token),
                "-H", &format!("Notion-Version: {}", self.version),
                "-H", "Content-Type: application/json",
                "-d", &body_str,
            ])
            .output()?;
        let val: Value = serde_json::from_slice(&out.stdout)?;
        Ok(val)
    }

    fn curl_get(&self, url: &str) -> Result<Value> {
        let out = Command::new("curl")
            .args([
                "-s", "-X", "GET", url,
                "-H", &format!("Authorization: Bearer {}", self.token),
                "-H", &format!("Notion-Version: {}", self.version),
            ])
            .output()?;
        let val: Value = serde_json::from_slice(&out.stdout)?;
        Ok(val)
    }

    fn curl_delete(&self, url: &str) -> Result<()> {
        Command::new("curl")
            .args([
                "-s", "-X", "DELETE", url,
                "-H", &format!("Authorization: Bearer {}", self.token),
                "-H", &format!("Notion-Version: {}", self.version),
            ])
            .output()?;
        Ok(())
    }

    /// Query by File_name → return page_id if found
    pub fn find_by_filename(&self, file_name: &str) -> Result<Option<String>> {
        let url = format!(
            "https://api.notion.com/v1/databases/{}/query",
            self.database_id
        );
        let body = json!({
            "filter": {
                "property": "File_name",
                "rich_text": { "equals": file_name }
            }
        });
        let res = self.curl_post(&url, &body)?;
        if let Some(pages) = res["results"].as_array() {
            if let Some(page) = pages.first() {
                let id = page["id"].as_str().unwrap_or("").to_string();
                if !id.is_empty() {
                    return Ok(Some(id));
                }
            }
        }
        Ok(None)
    }

    pub fn create_page(&self, note: &ParsedNote) -> Result<String> {
        let properties = build_properties(&note.meta, &note.file_name);
        let children = body_to_blocks(&note.body);
        let body = json!({
            "parent": { "database_id": &self.database_id },
            "properties": properties,
            "children": children
        });
        let res = self.curl_post("https://api.notion.com/v1/pages", &body)?;
        if res.get("code").is_some() {
            bail!("Notion error: {}", res["message"].as_str().unwrap_or("unknown"));
        }
        Ok(res["id"].as_str().unwrap_or("").to_string())
    }

    pub fn update_page(&self, page_id: &str, note: &ParsedNote) -> Result<()> {
        let properties = build_properties(&note.meta, &note.file_name);
        let body = json!({ "properties": properties });
        let res = self.curl_patch(
            &format!("https://api.notion.com/v1/pages/{}", page_id),
            &body,
        )?;
        if res.get("code").is_some() {
            bail!("Notion error: {}", res["message"].as_str().unwrap_or("unknown"));
        }

        self.clear_blocks(page_id)?;

        let children = body_to_blocks(&note.body);
        if !children.is_empty() {
            let append = json!({ "children": children });
            self.curl_patch(
                &format!("https://api.notion.com/v1/blocks/{}/children", page_id),
                &append,
            )?;
        }
        Ok(())
    }

    fn clear_blocks(&self, page_id: &str) -> Result<()> {
        let res = self.curl_get(&format!(
            "https://api.notion.com/v1/blocks/{}/children",
            page_id
        ))?;
        if let Some(blocks) = res["results"].as_array() {
            for block in blocks {
                if let Some(id) = block["id"].as_str() {
                    let _ = self.curl_delete(&format!("https://api.notion.com/v1/blocks/{}", id));
                }
            }
        }
        Ok(())
    }
}

fn build_properties(meta: &HashMap<String, Value>, file_name: &str) -> Value {
    let mut props = serde_json::Map::new();

    // Title: frontmatter "title" หรือ fallback เป็น file_name
    let title_val = meta
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(file_name);
    props.insert(
        "Title".to_string(),
        json!({ "title": [{ "text": { "content": title_val } }] }),
    );

    // File_name: set จาก filename เสมอ
    props.insert(
        "File_name".to_string(),
        json!({ "rich_text": [{ "text": { "content": file_name } }] }),
    );

    for (key, value) in meta {
        if key == "title" {
            continue;
        }
        let notion_key = normalize_key(key);
        if let Some(p) = infer_property(key, value) {
            props.insert(notion_key, p);
        }
    }

    Value::Object(props)
}

fn normalize_key(key: &str) -> String {
    let known = [
        "ID", "Description", "Status", "Created", "Tags",
        "Updated", "File_name", "Used_In_Post", "Scheduled_Post", "Source",
    ];
    for k in &known {
        if k.to_lowercase() == key.to_lowercase() {
            return k.to_string();
        }
    }
    key.to_string()
}

fn infer_property(key: &str, value: &Value) -> Option<Value> {
    if value.is_null() {
        return None;
    }
    let k = key.to_lowercase();

    if k.contains("created") || k.contains("updated") || k.contains("scheduled") {
        if let Some(s) = value.as_str() {
            return Some(json!({ "date": { "start": s } }));
        }
    }
    if k.contains("url") || k == "used_in_post" {
        if let Some(s) = value.as_str() {
            return Some(json!({ "url": s }));
        }
    }
    if k == "status" || k == "source" {
        if let Some(s) = value.as_str() {
            return Some(json!({ "status": { "name": s } }));
        }
    }
    if k == "tags" {
        if let Some(arr) = value.as_array() {
            let opts: Vec<Value> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| json!({ "name": s }))
                .collect();
            return Some(json!({ "multi_select": opts }));
        }
    }

    // Fallback: rich_text
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    };
    Some(json!({ "rich_text": [{ "text": { "content": text } }] }))
}

fn body_to_blocks(body: &str) -> Vec<Value> {
    body.lines()
        .map(|line| {
            if line.starts_with("### ") {
                json!({ "object": "block", "type": "heading_3",
                    "heading_3": { "rich_text": [{ "text": { "content": &line[4..] } }] } })
            } else if line.starts_with("## ") {
                json!({ "object": "block", "type": "heading_2",
                    "heading_2": { "rich_text": [{ "text": { "content": &line[3..] } }] } })
            } else if line.starts_with("# ") {
                json!({ "object": "block", "type": "heading_1",
                    "heading_1": { "rich_text": [{ "text": { "content": &line[2..] } }] } })
            } else {
                json!({ "object": "block", "type": "paragraph",
                    "paragraph": { "rich_text": [{ "text": { "content": line } }] } })
            }
        })
        .collect()
}
