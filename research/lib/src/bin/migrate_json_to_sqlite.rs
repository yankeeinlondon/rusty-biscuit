//! Migration script to convert research-inventory.json to SQLite database.
//!
//! ## Usage
//!
//! ```bash
//! # Preview what would be migrated (dry run)
//! cargo run -p research-lib --bin migrate_json_to_sqlite -- --dry-run
//!
//! # Perform the migration
//! cargo run -p research-lib --bin migrate_json_to_sqlite
//!
//! # Verify migration
//! cargo run -p research-lib --bin migrate_json_to_sqlite -- --verify
//!
//! # Export database back to JSON (rollback support)
//! cargo run -p research-lib --bin migrate_json_to_sqlite -- --export backup.json
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use research_lib::metadata::{ResearchInventory, ResearchInventoryDb, Topic};

#[derive(Parser, Debug)]
#[command(name = "migrate_json_to_sqlite")]
#[command(about = "Migrate research inventory from JSON to SQLite")]
struct Args {
    /// Path to JSON inventory file (default: auto-detect from RESEARCH_DIR or HOME)
    #[arg(long)]
    json_path: Option<PathBuf>,

    /// Path to SQLite database (default: same directory as JSON, with .db extension)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Dry run - validate without writing
    #[arg(long)]
    dry_run: bool,

    /// Verify migration by comparing counts
    #[arg(long)]
    verify: bool,

    /// Export database back to JSON (rollback support)
    #[arg(long)]
    export: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Determine paths
    let json_path = match args.json_path {
        Some(p) => p,
        None => ResearchInventory::default_path()?,
    };

    let db_path = match args.db_path {
        Some(p) => p,
        None => json_path.with_extension("db"),
    };

    println!("JSON path: {}", json_path.display());
    println!("DB path: {}", db_path.display());
    println!();

    // Handle export mode
    if let Some(export_path) = args.export {
        return export_db_to_json(&db_path, &export_path).await;
    }

    // Handle verify mode
    if args.verify {
        return verify_migration(&json_path, &db_path).await;
    }

    // Load JSON inventory
    println!("Loading JSON inventory...");
    let inventory = ResearchInventory::load_from(&json_path)?;
    let topic_count = inventory.len();
    println!("  Found {} topics", topic_count);

    if topic_count == 0 {
        println!("No topics to migrate.");
        return Ok(());
    }

    // Count documents
    let doc_count: usize = inventory.iter().map(|(_, t)| count_documents(t)).sum();
    println!("  Found {} documents total", doc_count);

    // Dry run mode
    if args.dry_run {
        println!();
        println!("=== DRY RUN ===");
        println!("Would migrate:");
        for (name, topic) in inventory.iter() {
            let docs = topic.documents().len();
            let children = count_all_children(topic);
            println!("  - {} ({} docs, {} children)", name, docs, children);
        }
        println!();
        println!("No changes made.");
        return Ok(());
    }

    // Check if DB already exists
    if db_path.exists() {
        println!();
        println!("Warning: Database already exists at {}", db_path.display());
        println!("  Backing up to {}.bak", db_path.display());
        fs::copy(&db_path, db_path.with_extension("db.bak"))?;
        fs::remove_file(&db_path)?;
    }

    // Create and migrate to database
    println!();
    println!("Creating SQLite database...");
    let db = ResearchInventoryDb::connect(&db_path).await?;

    println!("Migrating topics...");
    let mut migrated = 0;
    let mut errors = 0;

    for (name, topic) in inventory.iter() {
        match db.insert(topic, None).await {
            Ok(()) => {
                migrated += 1;
                if migrated % 10 == 0 {
                    println!("  Migrated {} topics...", migrated);
                }
            }
            Err(e) => {
                eprintln!("  Error migrating '{}': {}", name, e);
                errors += 1;
            }
        }
    }

    println!();
    println!("=== Migration Complete ===");
    println!("  Topics migrated: {}", migrated);
    println!("  Errors: {}", errors);

    // Verify counts
    let db_count = db.count().await?;
    if db_count != topic_count {
        eprintln!("Warning: Topic count mismatch! JSON: {}, DB: {}", topic_count, db_count);
    } else {
        println!("  Verified: {} topics in database", db_count);
    }

    // Backup JSON if migration succeeded
    if errors == 0 {
        let backup_path = json_path.with_extension("json.bak");
        println!();
        println!("Creating JSON backup at {}", backup_path.display());
        fs::copy(&json_path, &backup_path)?;
    }

    Ok(())
}

/// Count all documents in a topic including children.
fn count_documents(topic: &Topic) -> usize {
    let mut count = topic.documents().len();
    for child in topic.children() {
        count += count_documents(child);
    }
    count
}

/// Count all children recursively.
fn count_all_children(topic: &Topic) -> usize {
    let mut count = topic.children().len();
    for child in topic.children() {
        count += count_all_children(child);
    }
    count
}

/// Verify migration by comparing JSON and DB counts.
async fn verify_migration(json_path: &Path, db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Verification ===");

    // Load JSON
    let inventory = ResearchInventory::load_from(json_path)?;
    let json_count = inventory.len();
    println!("JSON topics: {}", json_count);

    // Load DB
    if !db_path.exists() {
        eprintln!("Database not found: {}", db_path.display());
        return Ok(());
    }

    let db = ResearchInventoryDb::connect(db_path).await?;
    let db_count = db.count().await?;
    println!("DB topics: {}", db_count);

    // Compare
    if json_count == db_count {
        println!();
        println!("PASS: Counts match!");
    } else {
        println!();
        println!("FAIL: Count mismatch!");

        // Find missing topics
        let db_names: std::collections::HashSet<_> = db.list_names().await?.into_iter().collect();

        println!();
        println!("Topics in JSON but not in DB:");
        for (name, _) in inventory.iter() {
            if !db_names.contains(name) {
                println!("  - {}", name);
            }
        }

        println!();
        println!("Topics in DB but not in JSON:");
        for name in &db_names {
            if !inventory.contains(name) {
                println!("  - {}", name);
            }
        }
    }

    Ok(())
}

/// Export database back to JSON for rollback.
async fn export_db_to_json(db_path: &Path, export_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Export to JSON ===");

    if !db_path.exists() {
        eprintln!("Database not found: {}", db_path.display());
        return Ok(());
    }

    let db = ResearchInventoryDb::connect(db_path).await?;

    // Load all topics
    println!("Loading topics from database...");
    let names = db.list_names().await?;
    println!("  Found {} topics", names.len());

    // Create inventory
    let mut inventory = ResearchInventory::new();
    for name in &names {
        if let Some(topic) = db.get(name).await? {
            // Only insert root topics (children are nested)
            // Check if this topic has a parent by checking if any other topic contains it as a child
            // For simplicity, we'll insert all and rely on the JSON structure
            inventory.insert(name.clone(), topic);
        }
    }

    // Save
    println!("Saving to {}...", export_path.display());
    inventory.save_to(export_path)?;
    println!("Export complete!");

    Ok(())
}
