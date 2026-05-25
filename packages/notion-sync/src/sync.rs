use anyhow::Result;
use walkdir::WalkDir;

use crate::config::Config;
use crate::frontmatter;
use crate::notion::NotionClient;

pub fn sync_all(cfg: &Config, dry_run: bool) -> Result<()> {
    let scan_path = match &cfg.vault_subfolder {
        Some(sub) => format!("{}/{}", cfg.vault_path, sub),
        None => cfg.vault_path.clone(),
    };

    println!("→ Scanning: {}", scan_path);

    let files: Vec<String> = WalkDir::new(&scan_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    println!("→ Found {} .md files\n", files.len());

    let (mut created, mut updated, mut skipped, mut errors) = (0, 0, 0, 0);

    for file in &files {
        match sync_file(cfg, file, dry_run) {
            Ok(action) => match action.as_str() {
                "created" => created += 1,
                "updated" => updated += 1,
                _ => skipped += 1,
            },
            Err(e) => {
                eprintln!("✗ {} — {}", file, e);
                errors += 1;
            }
        }
    }

    println!("\n{}", "─".repeat(40));
    println!(
        "Created: {}  Updated: {}  Skipped: {}  Errors: {}",
        created, updated, skipped, errors
    );

    Ok(())
}

pub fn sync_file(cfg: &Config, file_path: &str, dry_run: bool) -> Result<String> {
    let note = frontmatter::parse_file(file_path)?;

    if note.meta.is_empty() {
        println!("– {} (no frontmatter)", note.file_name);
        return Ok("skipped".to_string());
    }

    if dry_run {
        println!("[dry-run] {}", note.file_name);
        println!("  fields: {:?}", note.meta.keys().collect::<Vec<_>>());
        return Ok("skipped".to_string());
    }

    let client = NotionClient::new(cfg);

    match client.find_by_filename(&note.file_name)? {
        None => {
            let page_id = client.create_page(&note)?;
            println!("✓ Created  {} ({})", note.file_name, &page_id[..8.min(page_id.len())]);
            Ok("created".to_string())
        }
        Some(page_id) => {
            client.update_page(&page_id, &note)?;
            println!("↑ Updated  {} ({})", note.file_name, &page_id[..8.min(page_id.len())]);
            Ok("updated".to_string())
        }
    }
}
