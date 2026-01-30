//! SQLite-backed research inventory.
//!
//! This module provides [`ResearchInventoryDb`], an async database-backed inventory
//! that replaces the JSON file-based [`ResearchInventory`].
//!
//! ## Usage
//!
//! ```no_run
//! use research_lib::metadata::db::ResearchInventoryDb;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to database
//! let inventory = ResearchInventoryDb::connect(Path::new("research.db")).await?;
//!
//! // Query topics
//! if let Some(topic) = inventory.get("clap").await? {
//!     println!("Found: {}", topic.name());
//! }
//!
//! // List all topics
//! let topics = inventory.list_all().await?;
//! println!("Total topics: {}", topics.len());
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use crate::metadata::{Document, KindCategory, Topic};

use super::{
    queries::{
        count_topics, fetch_topic_with_children, get_all_topic_names, topic_exists,
    },
    rows::{
        content_type_to_text, kind_category_to_discriminator,
        licenses_to_json, package_manager_to_text,
    },
    init_memory_pool, init_pool, run_migrations, u64_to_i64, DbError, DbPool, DbResult,
};

/// SQLite-backed research inventory.
///
/// Provides async CRUD operations for research topics backed by a SQLite database.
/// Unlike the JSON-based `ResearchInventory`, this is a pure database facade
/// with no in-memory caching.
///
/// ## Thread Safety
///
/// `ResearchInventoryDb` uses a connection pool internally and is safe to share
/// across threads (implements `Send + Sync`).
pub struct ResearchInventoryDb {
    pool: DbPool,
}

impl ResearchInventoryDb {
    /// Connect to a SQLite database at the given path.
    ///
    /// Creates the database file if it doesn't exist and runs migrations.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The database connection cannot be established
    /// - Migrations fail to apply
    pub async fn connect(path: &Path) -> DbResult<Self> {
        let pool = init_pool(path).await?;
        run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Create an in-memory database for testing.
    ///
    /// The database is initialized with the schema and ready to use.
    pub async fn in_memory() -> DbResult<Self> {
        let pool = init_memory_pool().await?;
        run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Get access to the underlying connection pool.
    ///
    /// Useful for advanced queries or testing.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Get a topic by name, including all children recursively.
    ///
    /// Returns `None` if the topic doesn't exist.
    pub async fn get(&self, name: &str) -> DbResult<Option<Topic>> {
        fetch_topic_with_children(&self.pool, name).await
    }

    /// Check if a topic exists.
    pub async fn contains(&self, name: &str) -> DbResult<bool> {
        topic_exists(&self.pool, name).await
    }

    /// Get the total count of topics.
    pub async fn count(&self) -> DbResult<usize> {
        count_topics(&self.pool).await
    }

    /// List all topic names.
    pub async fn list_names(&self) -> DbResult<Vec<String>> {
        get_all_topic_names(&self.pool).await
    }

    /// List all topics with their full data (expensive for large inventories).
    ///
    /// Consider using `list_names()` and `get()` for selective loading.
    pub async fn list_all(&self) -> DbResult<Vec<Topic>> {
        let names = get_all_topic_names(&self.pool).await?;
        let mut topics = Vec::with_capacity(names.len());

        for name in names {
            if let Some(topic) = fetch_topic_with_children(&self.pool, &name).await? {
                topics.push(topic);
            }
        }

        Ok(topics)
    }

    /// Insert a new topic into the database.
    ///
    /// ## Arguments
    ///
    /// * `topic` - The topic to insert
    /// * `parent_name` - Optional parent topic name (for nested topics)
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - A topic with the same name already exists
    /// - The parent topic doesn't exist
    /// - Database operations fail
    pub async fn insert(&self, topic: &Topic, parent_name: Option<&str>) -> DbResult<()> {
        // Start a transaction for atomicity
        let mut tx = self.pool.begin().await.map_err(DbError::QueryFailed)?;

        // Insert the topic
        self.insert_topic_with_tx(&mut tx, topic, parent_name).await?;

        // Commit
        tx.commit().await.map_err(DbError::QueryFailed)?;
        Ok(())
    }

    /// Insert a topic within a transaction.
    async fn insert_topic_with_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        topic: &Topic,
        parent_name: Option<&str>,
    ) -> DbResult<()> {
        let kind_str = kind_category_to_discriminator(topic.kind());
        let created = topic.created().to_rfc3339();
        let last_updated = topic.last_updated().to_rfc3339();

        // Insert main topic row
        sqlx::query(
            r#"
            INSERT INTO topics (name, kind, parent_topic_name, created, last_updated, brief, summary, when_to_use)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(topic.name())
        .bind(kind_str)
        .bind(parent_name)
        .bind(&created)
        .bind(&last_updated)
        .bind(topic.brief())
        .bind(topic.summary())
        .bind(topic.when_to_use())
        .execute(&mut **tx)
        .await
        .map_err(DbError::QueryFailed)?;

        // Insert kind-specific details
        match topic.kind() {
            KindCategory::Library(lib) => {
                let features_json: Option<String> = None; // TODO: Need getter on Library
                let licenses_json = licenses_to_json(&[])?; // TODO: Need getter on Library

                sqlx::query(
                    r#"
                    INSERT INTO library_details (topic_name, package_manager, package_name, features, language, url, repo, docs, licenses)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                )
                .bind(topic.name())
                .bind(package_manager_to_text(lib.package_manager()))
                .bind(lib.package_name())
                .bind(features_json)
                .bind(lib.language())
                .bind(lib.url())
                .bind::<Option<&str>>(None) // repo TODO: Need getter
                .bind::<Option<&str>>(None) // docs TODO: Need getter
                .bind(&licenses_json)
                .execute(&mut **tx)
                .await
                .map_err(DbError::QueryFailed)?;
            }
            KindCategory::Software(sw) => {
                sqlx::query(
                    r#"
                    INSERT INTO software_details (topic_name, name, company)
                    VALUES (?1, ?2, ?3)
                    "#,
                )
                .bind(topic.name())
                .bind(sw.name())
                .bind(sw.company())
                .execute(&mut **tx)
                .await
                .map_err(DbError::QueryFailed)?;
            }
            // Person, SolutionArea, ProgrammingLanguage have no detail tables
            KindCategory::Person | KindCategory::SolutionArea | KindCategory::ProgrammingLanguage => {}
        }

        // Insert documents
        for doc in topic.documents() {
            self.insert_document_with_tx(tx, topic.name(), doc).await?;
        }

        // Recursively insert children
        for child in topic.children() {
            Box::pin(self.insert_topic_with_tx(tx, child, Some(topic.name()))).await?;
        }

        Ok(())
    }

    /// Insert a document within a transaction.
    async fn insert_document_with_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        topic_name: &str,
        doc: &Document,
    ) -> DbResult<()> {
        let content_type_str = content_type_to_text(doc.content_type());
        // TODO: Need getters for these fields on Document
        let created = chrono::Utc::now().to_rfc3339();
        let last_updated = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO documents (topic_name, filename, content_type, prompt, flow, created, last_updated, model, model_capability, content_hash, interpolated_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(topic_name)
        .bind(doc.filename())
        .bind(content_type_str)
        .bind::<Option<&str>>(None) // prompt TODO: Need getter
        .bind::<Option<&str>>(None) // flow TODO: Need getter
        .bind(&created)
        .bind(&last_updated)
        .bind::<Option<&str>>(None) // model TODO: Need getter
        .bind::<Option<&str>>(None) // model_capability TODO: Need getter
        .bind(u64_to_i64(doc.content_hash()))
        .bind(0i64) // interpolated_hash TODO: Need getter
        .execute(&mut **tx)
        .await
        .map_err(DbError::QueryFailed)?;

        Ok(())
    }

    /// Update an existing topic.
    ///
    /// This replaces the topic entirely (delete + insert).
    pub async fn update(&self, topic: &Topic) -> DbResult<()> {
        let mut tx = self.pool.begin().await.map_err(DbError::QueryFailed)?;

        // Get the parent name before deleting
        let parent_name: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT parent_topic_name FROM topics WHERE name = ?1",
        )
        .bind(topic.name())
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::QueryFailed)?;

        let parent = parent_name.and_then(|(p,)| p);

        // Delete old topic (cascades to details and documents)
        sqlx::query("DELETE FROM topics WHERE name = ?1")
            .bind(topic.name())
            .execute(&mut *tx)
            .await
            .map_err(DbError::QueryFailed)?;

        // Insert updated topic
        self.insert_topic_with_tx(&mut tx, topic, parent.as_deref()).await?;

        tx.commit().await.map_err(DbError::QueryFailed)?;
        Ok(())
    }

    /// Remove a topic by name.
    ///
    /// Returns the removed topic if it existed, `None` otherwise.
    /// Cascades to children, documents, and details tables.
    pub async fn remove(&self, name: &str) -> DbResult<Option<Topic>> {
        // Fetch the topic first (to return it)
        let topic = self.get(name).await?;

        if topic.is_some() {
            // DELETE cascades to all related tables
            sqlx::query("DELETE FROM topics WHERE name = ?1")
                .bind(name)
                .execute(&self.pool)
                .await
                .map_err(DbError::QueryFailed)?;
        }

        Ok(topic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{Software, KindCategory};

    fn create_test_topic(name: &str) -> Topic {
        Topic::new(
            name.to_string(),
            KindCategory::Software(Software::new(name.to_string())),
        )
    }

    #[tokio::test]
    async fn test_in_memory_creation() {
        let db = ResearchInventoryDb::in_memory().await;
        assert!(db.is_ok());
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let topic = create_test_topic("test-topic");

        db.insert(&topic, None).await.unwrap();

        let retrieved = db.get("test-topic").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test-topic");
    }

    #[tokio::test]
    async fn test_contains() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();

        assert!(!db.contains("missing").await.unwrap());

        let topic = create_test_topic("exists");
        db.insert(&topic, None).await.unwrap();

        assert!(db.contains("exists").await.unwrap());
    }

    #[tokio::test]
    async fn test_count() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();

        assert_eq!(db.count().await.unwrap(), 0);

        db.insert(&create_test_topic("topic1"), None).await.unwrap();
        assert_eq!(db.count().await.unwrap(), 1);

        db.insert(&create_test_topic("topic2"), None).await.unwrap();
        assert_eq!(db.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_list_names() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();

        db.insert(&create_test_topic("a"), None).await.unwrap();
        db.insert(&create_test_topic("b"), None).await.unwrap();
        db.insert(&create_test_topic("c"), None).await.unwrap();

        let names = db.list_names().await.unwrap();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
        assert!(names.contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_remove() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let topic = create_test_topic("to-remove");

        db.insert(&topic, None).await.unwrap();
        assert!(db.contains("to-remove").await.unwrap());

        let removed = db.remove("to-remove").await.unwrap();
        assert!(removed.is_some());
        assert!(!db.contains("to-remove").await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let removed = db.remove("nonexistent").await.unwrap();
        assert!(removed.is_none());
    }

    #[tokio::test]
    async fn test_insert_person_kind() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let topic = Topic::new("john-doe".to_string(), KindCategory::Person);

        db.insert(&topic, None).await.unwrap();

        let retrieved = db.get("john-doe").await.unwrap().unwrap();
        assert!(matches!(retrieved.kind(), KindCategory::Person));
    }

    #[tokio::test]
    async fn test_insert_solution_area_kind() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let topic = Topic::new("authentication".to_string(), KindCategory::SolutionArea);

        db.insert(&topic, None).await.unwrap();

        let retrieved = db.get("authentication").await.unwrap().unwrap();
        assert!(matches!(retrieved.kind(), KindCategory::SolutionArea));
    }

    #[tokio::test]
    async fn test_insert_programming_language_kind() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();
        let topic = Topic::new("rust".to_string(), KindCategory::ProgrammingLanguage);

        db.insert(&topic, None).await.unwrap();

        let retrieved = db.get("rust").await.unwrap().unwrap();
        assert!(matches!(retrieved.kind(), KindCategory::ProgrammingLanguage));
    }

    #[tokio::test]
    async fn test_insert_with_parent() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();

        let parent = create_test_topic("parent");
        db.insert(&parent, None).await.unwrap();

        let child = create_test_topic("child");
        db.insert(&child, Some("parent")).await.unwrap();

        // Verify parent has the child
        let retrieved_parent = db.get("parent").await.unwrap().unwrap();
        assert_eq!(retrieved_parent.children().len(), 1);
        assert_eq!(retrieved_parent.children()[0].name(), "child");
    }

    #[tokio::test]
    async fn test_concurrent_inserts() {
        use std::sync::Arc;

        let db = Arc::new(ResearchInventoryDb::in_memory().await.unwrap());

        // Spawn multiple tasks to insert topics concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = tokio::spawn(async move {
                let topic = Topic::new(
                    format!("concurrent-topic-{}", i),
                    KindCategory::Person,
                );
                db_clone.insert(&topic, None).await
            });
            handles.push(handle);
        }

        // Wait for all inserts to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all topics were inserted
        assert_eq!(db.count().await.unwrap(), 10);

        // Verify we can retrieve each topic
        for i in 0..10 {
            let name = format!("concurrent-topic-{}", i);
            assert!(db.contains(&name).await.unwrap(), "Topic {} not found", name);
        }
    }

    #[tokio::test]
    async fn test_all_kind_variants_roundtrip() {
        use sniff_lib::package::LanguagePackageManager;
        use crate::metadata::Library;

        let db = ResearchInventoryDb::in_memory().await.unwrap();

        // Test Library kind
        let lib = Library::new(
            LanguagePackageManager::Cargo,
            "test-lib".to_string(),
            "Rust".to_string(),
            "https://crates.io/crates/test-lib".to_string(),
        );
        let library_topic = Topic::new("library-topic".to_string(), KindCategory::Library(lib));
        db.insert(&library_topic, None).await.unwrap();

        // Test Software kind
        let software_topic = create_test_topic("software-topic");
        db.insert(&software_topic, None).await.unwrap();

        // Test Person kind
        let person_topic = Topic::new("person-topic".to_string(), KindCategory::Person);
        db.insert(&person_topic, None).await.unwrap();

        // Test SolutionArea kind
        let solution_topic = Topic::new("solution-topic".to_string(), KindCategory::SolutionArea);
        db.insert(&solution_topic, None).await.unwrap();

        // Test ProgrammingLanguage kind
        let lang_topic = Topic::new("lang-topic".to_string(), KindCategory::ProgrammingLanguage);
        db.insert(&lang_topic, None).await.unwrap();

        // Verify all can be retrieved with correct kinds
        let lib_retrieved = db.get("library-topic").await.unwrap().unwrap();
        assert!(matches!(lib_retrieved.kind(), KindCategory::Library(_)));

        let sw_retrieved = db.get("software-topic").await.unwrap().unwrap();
        assert!(matches!(sw_retrieved.kind(), KindCategory::Software(_)));

        let person_retrieved = db.get("person-topic").await.unwrap().unwrap();
        assert!(matches!(person_retrieved.kind(), KindCategory::Person));

        let solution_retrieved = db.get("solution-topic").await.unwrap().unwrap();
        assert!(matches!(solution_retrieved.kind(), KindCategory::SolutionArea));

        let lang_retrieved = db.get("lang-topic").await.unwrap().unwrap();
        assert!(matches!(lang_retrieved.kind(), KindCategory::ProgrammingLanguage));
    }

    #[tokio::test]
    async fn test_update_topic() {
        let db = ResearchInventoryDb::in_memory().await.unwrap();

        // Insert initial topic
        let mut topic = Topic::new("update-test".to_string(), KindCategory::Person);
        topic.set_brief("Original brief".to_string());
        db.insert(&topic, None).await.unwrap();

        // Verify original
        let retrieved = db.get("update-test").await.unwrap().unwrap();
        assert_eq!(retrieved.brief(), "Original brief");

        // Update topic
        topic.set_brief("Updated brief".to_string());
        db.update(&topic).await.unwrap();

        // Verify update
        let retrieved = db.get("update-test").await.unwrap().unwrap();
        assert_eq!(retrieved.brief(), "Updated brief");
    }
}
