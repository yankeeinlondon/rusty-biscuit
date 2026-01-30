//! One-time migration script to populate research-inventory.json from existing topics.

use research_lib::metadata::migration_v2::scan_and_build_inventory;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get research directory
    let research_dir = std::env::var("RESEARCH_DIR")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{}/.research", h)))
        .map(PathBuf::from)?;

    println!("Scanning research directory: {:?}", research_dir);

    // Run migration
    let inventory = scan_and_build_inventory(&research_dir)?;
    println!("Found {} topics", inventory.len());

    // Save the inventory
    let inventory_path = research_dir.join("research-inventory.json");
    inventory.save_to(&inventory_path)?;
    println!("Saved inventory to {:?}", inventory_path);

    // Print summary
    for name in inventory.topic_names() {
        if let Some(topic) = inventory.get(name) {
            println!(
                "  - {} ({} documents)",
                name,
                topic.documents().len()
            );
        }
    }

    Ok(())
}
