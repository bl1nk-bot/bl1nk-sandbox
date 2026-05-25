mod config;
mod frontmatter;
mod notion;
mod sync;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ons", about = "Obsidian → Notion prompt sync CLI")]
struct Cli {
    #[arg(short, long, default_value = "settings.json")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync all .md files in vault to Notion
    Sync {
        #[arg(long)]
        dry_run: bool,
    },
    /// Sync a single file
    Push {
        file: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Show parsed frontmatter of a file (JSON)
    Inspect {
        file: String,
    },
    /// Validate settings.json
    Check,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config)?;

    match cli.command {
        Commands::Sync { dry_run } => {
            sync::sync_all(&cfg, dry_run)?;
        }
        Commands::Push { file, dry_run } => {
            sync::sync_file(&cfg, &file, dry_run)?;
        }
        Commands::Inspect { file } => {
            let fm = frontmatter::parse_file(&file)?;
            println!("{}", serde_json::to_string_pretty(&fm)?);
        }
        Commands::Check => {
            println!("✓ settings.json OK");
            println!("{:#?}", cfg);
        }
    }

    Ok(())
}
