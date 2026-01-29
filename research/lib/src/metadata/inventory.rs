//! Centralized research inventory management.
//!
//! This module provides the [`ResearchInventory`] type for managing a centralized
//! registry of all research topics. The inventory is stored as a JSON file at
//! `$RESEARCH_DIR/.research/research-inventory.json`.
//!
//! ## Migration
//!
//! The inventory system supports lazy migration from the legacy per-topic
//! `metadata.json` files. When loading an inventory that doesn't exist,
//! the system can scan the filesystem and build an inventory from existing
//! research topics.
//!
//! ## Examples
//!
//! ```no_run
//! use research_lib::metadata::inventory::ResearchInventory;
//!
//! // Load or create inventory
//! let inventory = ResearchInventory::load().unwrap();
//!
//! // Get a topic
//! if let Some(topic) = inventory.get("clap") {
//!     println!("Found topic: {}", topic.name());
//! }
//! ```

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Topic;

/// Errors that can occur when working with the research inventory.
#[derive(Debug, Error)]
pub enum InventoryError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse the inventory JSON.
    #[error("Failed to parse inventory: {0}")]
    Parse(#[from] serde_json::Error),

    /// The requested topic was not found.
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    /// The RESEARCH_DIR and HOME environment variables are not set.
    #[error("Neither RESEARCH_DIR nor HOME environment variable is set")]
    NoResearchDir,
}

/// Result type for inventory operations.
pub type Result<T> = std::result::Result<T, InventoryError>;

/// The centralized research inventory.
///
/// This stores all research topics in a single JSON file, replacing the
/// per-topic `metadata.json` approach. The inventory uses topic names
/// as keys for O(1) lookup.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchInventory {
    /// Schema version for future migrations.
    #[serde(default = "default_schema_version")]
    schema_version: u32,

    /// Map of topic name to topic metadata.
    #[serde(default)]
    topics: HashMap<String, Topic>,
}

fn default_schema_version() -> u32 {
    2
}

impl ResearchInventory {
    /// Create a new empty inventory.
    pub fn new() -> Self {
        Self {
            schema_version: 2,
            topics: HashMap::new(),
        }
    }

    /// Get the default inventory path.
    ///
    /// Returns `$RESEARCH_DIR/.research/research-inventory.json` if `RESEARCH_DIR`
    /// is set, otherwise `$HOME/.research/research-inventory.json`.
    pub fn default_path() -> Result<PathBuf> {
        let base = std::env::var("RESEARCH_DIR").unwrap_or_else(|_| {
            std::env::var("HOME").unwrap_or_else(|_| String::new())
        });

        if base.is_empty() {
            return Err(InventoryError::NoResearchDir);
        }

        Ok(PathBuf::from(base)
            .join(".research")
            .join("research-inventory.json"))
    }

    /// Load the inventory from the default path.
    ///
    /// If the inventory file doesn't exist, returns an empty inventory.
    /// The caller should then trigger migration if needed.
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_path()?)
    }

    /// Load the inventory from a specific path.
    ///
    /// If the file doesn't exist, returns an empty inventory.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let inventory = serde_json::from_reader(reader)?;
        Ok(inventory)
    }

    /// Save the inventory to the default path.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption.
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_path()?)
    }

    /// Save the inventory to a specific path.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temp file first
        let temp_path = path.with_extension("json.tmp");
        {
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, self)?;
            writer.flush()?;
        }

        // Atomic rename
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Get a topic by name.
    pub fn get(&self, name: &str) -> Option<&Topic> {
        self.topics.get(name)
    }

    /// Get a mutable reference to a topic by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Topic> {
        self.topics.get_mut(name)
    }

    /// Insert or update a topic (upsert behavior).
    ///
    /// If a topic with the same name already exists, it is replaced.
    /// Returns the previous topic if one existed.
    pub fn insert(&mut self, name: String, topic: Topic) -> Option<Topic> {
        self.topics.insert(name, topic)
    }

    /// Remove a topic by name.
    ///
    /// Returns the removed topic if it existed.
    pub fn remove(&mut self, name: &str) -> Option<Topic> {
        self.topics.remove(name)
    }

    /// Check if a topic exists.
    pub fn contains(&self, name: &str) -> bool {
        self.topics.contains_key(name)
    }

    /// Get the number of topics in the inventory.
    pub fn len(&self) -> usize {
        self.topics.len()
    }

    /// Check if the inventory is empty.
    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
    }

    /// Iterate over all topics.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Topic)> {
        self.topics.iter()
    }

    /// Get all topic names.
    pub fn topic_names(&self) -> impl Iterator<Item = &String> {
        self.topics.keys()
    }

    /// Get the schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{KindCategory, Software};
    use tempfile::TempDir;

    fn create_test_topic(name: &str) -> Topic {
        Topic::new(
            name.to_string(),
            KindCategory::Software(Software::new(name.to_string())),
        )
    }

    #[test]
    fn test_new_inventory_is_empty() {
        let inventory = ResearchInventory::new();
        assert!(inventory.is_empty());
        assert_eq!(inventory.len(), 0);
        assert_eq!(inventory.schema_version(), 2);
    }

    #[test]
    fn test_insert_and_get() {
        let mut inventory = ResearchInventory::new();
        let topic = create_test_topic("test-topic");

        let previous = inventory.insert("test-topic".to_string(), topic);
        assert!(previous.is_none());

        let retrieved = inventory.get("test-topic");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test-topic");
    }

    #[test]
    fn test_insert_upsert_behavior() {
        let mut inventory = ResearchInventory::new();

        let topic1 = create_test_topic("topic");
        inventory.insert("topic".to_string(), topic1);

        let topic2 = create_test_topic("topic-updated");
        let previous = inventory.insert("topic".to_string(), topic2);

        assert!(previous.is_some());
        assert_eq!(previous.unwrap().name(), "topic");

        // Verify the new topic is stored
        let current = inventory.get("topic").unwrap();
        assert_eq!(current.name(), "topic-updated");
    }

    #[test]
    fn test_remove() {
        let mut inventory = ResearchInventory::new();
        let topic = create_test_topic("to-remove");

        inventory.insert("to-remove".to_string(), topic);
        assert!(inventory.contains("to-remove"));

        let removed = inventory.remove("to-remove");
        assert!(removed.is_some());
        assert!(!inventory.contains("to-remove"));
    }

    #[test]
    fn test_contains() {
        let mut inventory = ResearchInventory::new();
        let topic = create_test_topic("exists");

        assert!(!inventory.contains("exists"));
        inventory.insert("exists".to_string(), topic);
        assert!(inventory.contains("exists"));
    }

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("inventory.json");

        // Create and save
        let mut inventory = ResearchInventory::new();
        inventory.insert("topic1".to_string(), create_test_topic("topic1"));
        inventory.insert("topic2".to_string(), create_test_topic("topic2"));

        inventory.save_to(&path).unwrap();
        assert!(path.exists());

        // Load and verify
        let loaded = ResearchInventory::load_from(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains("topic1"));
        assert!(loaded.contains("topic2"));
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("does-not-exist.json");

        let loaded = ResearchInventory::load_from(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let mut inventory = ResearchInventory::new();
        inventory.insert("clap".to_string(), create_test_topic("clap"));
        inventory.insert("serde".to_string(), create_test_topic("serde"));

        // Serialize
        let json = serde_json::to_string_pretty(&inventory).unwrap();

        // Deserialize
        let loaded: ResearchInventory = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.len(), inventory.len());
        assert_eq!(loaded.schema_version(), inventory.schema_version());
        assert!(loaded.contains("clap"));
        assert!(loaded.contains("serde"));
    }

    #[test]
    fn test_iter() {
        let mut inventory = ResearchInventory::new();
        inventory.insert("a".to_string(), create_test_topic("a"));
        inventory.insert("b".to_string(), create_test_topic("b"));
        inventory.insert("c".to_string(), create_test_topic("c"));

        let names: Vec<_> = inventory.topic_names().collect();
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested").join("dir").join("inventory.json");

        let inventory = ResearchInventory::new();
        inventory.save_to(&path).unwrap();

        assert!(path.exists());
    }
}
