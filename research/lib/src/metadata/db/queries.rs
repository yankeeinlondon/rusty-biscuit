//! Database query functions including recursive CTEs for topic hierarchies.
//!
//! This module provides query functions that use recursive Common Table Expressions
//! (CTEs) to efficiently load topic trees from the database.

use super::{
    rows::{DocumentRow, LibraryDetailsRow, SoftwareDetailsRow, TopicRow},
    DbError, DbPool, DbResult,
};
use crate::metadata::{Document, KindCategory, Topic};

/// Maximum recursion depth for topic tree queries.
/// Prevents infinite loops if data is corrupted.
const MAX_RECURSION_DEPTH: i32 = 10;

/// Get all descendant topic names for a given parent topic.
///
/// Uses a recursive CTE to traverse the topic tree starting from `parent_name`.
///
/// ## Arguments
///
/// * `pool` - Database connection pool
/// * `parent_name` - Name of the parent topic to start from
///
/// ## Returns
///
/// Vector of topic names that are descendants of the given parent,
/// including the parent itself.
pub async fn get_all_descendant_names(pool: &DbPool, parent_name: &str) -> DbResult<Vec<String>> {
    let query = r#"
        WITH RECURSIVE topic_tree AS (
            SELECT name, parent_topic_name, 0 as depth
            FROM topics
            WHERE name = ?1
            UNION ALL
            SELECT t.name, t.parent_topic_name, tt.depth + 1
            FROM topics t
            JOIN topic_tree tt ON t.parent_topic_name = tt.name
            WHERE tt.depth < ?2
        )
        SELECT name FROM topic_tree
    "#;

    let rows: Vec<(String,)> = sqlx::query_as(query)
        .bind(parent_name)
        .bind(MAX_RECURSION_DEPTH)
        .fetch_all(pool)
        .await
        .map_err(DbError::QueryFailed)?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Get direct children topic names for a given parent topic.
pub async fn get_child_topic_names(pool: &DbPool, parent_name: &str) -> DbResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM topics WHERE parent_topic_name = ?1 ORDER BY name",
    )
    .bind(parent_name)
    .fetch_all(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Get root topics (topics with no parent).
pub async fn get_root_topic_names(pool: &DbPool) -> DbResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM topics WHERE parent_topic_name IS NULL ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Fetch a single topic row by name.
pub async fn fetch_topic_row(pool: &DbPool, name: &str) -> DbResult<Option<TopicRow>> {
    let row: Option<TopicRow> = sqlx::query_as(
        "SELECT name, kind, parent_topic_name, created, last_updated, brief, summary, when_to_use FROM topics WHERE name = ?1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(row)
}

/// Fetch library details for a topic.
pub async fn fetch_library_details(pool: &DbPool, topic_name: &str) -> DbResult<Option<LibraryDetailsRow>> {
    let row: Option<LibraryDetailsRow> = sqlx::query_as(
        "SELECT topic_name, package_manager, package_name, features, language, url, repo, docs, licenses FROM library_details WHERE topic_name = ?1",
    )
    .bind(topic_name)
    .fetch_optional(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(row)
}

/// Fetch software details for a topic.
pub async fn fetch_software_details(pool: &DbPool, topic_name: &str) -> DbResult<Option<SoftwareDetailsRow>> {
    let row: Option<SoftwareDetailsRow> = sqlx::query_as(
        "SELECT topic_name, name, company FROM software_details WHERE topic_name = ?1",
    )
    .bind(topic_name)
    .fetch_optional(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(row)
}

/// Fetch all documents for a topic.
pub async fn fetch_documents(pool: &DbPool, topic_name: &str) -> DbResult<Vec<DocumentRow>> {
    let rows: Vec<DocumentRow> = sqlx::query_as(
        "SELECT topic_name, filename, content_type, prompt, flow, created, last_updated, model, model_capability, content_hash, interpolated_hash FROM documents WHERE topic_name = ?1 ORDER BY filename",
    )
    .bind(topic_name)
    .fetch_all(pool)
    .await
    .map_err(DbError::QueryFailed)?;

    Ok(rows)
}

/// Recursively fetch a topic with all its children.
///
/// This is the main entry point for loading a complete topic tree.
/// Uses `Box::pin` for recursive async calls.
pub async fn fetch_topic_with_children(pool: &DbPool, name: &str) -> DbResult<Option<Topic>> {
    // Fetch the topic row
    let topic_row = match fetch_topic_row(pool, name).await? {
        Some(row) => row,
        None => return Ok(None),
    };

    // Determine the kind based on discriminator and fetch details
    let kind = match topic_row.kind.as_str() {
        "Library" => {
            let details = fetch_library_details(pool, name)
                .await?
                .ok_or_else(|| DbError::InvalidEnumValue {
                    type_name: "LibraryDetails".to_string(),
                    value: format!("Missing library_details for topic {name}"),
                })?;
            KindCategory::Library(details.into_library()?)
        }
        "Software" => {
            let details = fetch_software_details(pool, name)
                .await?
                .ok_or_else(|| DbError::InvalidEnumValue {
                    type_name: "SoftwareDetails".to_string(),
                    value: format!("Missing software_details for topic {name}"),
                })?;
            KindCategory::Software(details.into_software())
        }
        "Person" => KindCategory::Person,
        "SolutionArea" => KindCategory::SolutionArea,
        "ProgrammingLanguage" => KindCategory::ProgrammingLanguage,
        other => {
            return Err(DbError::InvalidEnumValue {
                type_name: "KindCategory".to_string(),
                value: other.to_string(),
            })
        }
    };

    // Fetch documents
    let doc_rows = fetch_documents(pool, name).await?;
    let documents: Vec<Document> = doc_rows
        .into_iter()
        .map(|row| row.into_document())
        .collect::<DbResult<Vec<_>>>()?;

    // Fetch children recursively
    let child_names = get_child_topic_names(pool, name).await?;
    let mut children = Vec::with_capacity(child_names.len());

    for child_name in child_names {
        // Use Box::pin for recursive async
        if let Some(child) = Box::pin(fetch_topic_with_children(pool, &child_name)).await? {
            children.push(child);
        }
    }

    // Convert row to Topic
    topic_row
        .into_topic(kind, documents, children)
        .map(Some)
}

/// Fetch all root topics with their children.
pub async fn fetch_all_root_topics(pool: &DbPool) -> DbResult<Vec<Topic>> {
    let root_names = get_root_topic_names(pool).await?;
    let mut topics = Vec::with_capacity(root_names.len());

    for name in root_names {
        if let Some(topic) = fetch_topic_with_children(pool, &name).await? {
            topics.push(topic);
        }
    }

    Ok(topics)
}

/// Count total number of topics.
pub async fn count_topics(pool: &DbPool) -> DbResult<usize> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM topics")
        .fetch_one(pool)
        .await
        .map_err(DbError::QueryFailed)?;

    Ok(count as usize)
}

/// Check if a topic exists by name.
pub async fn topic_exists(pool: &DbPool, name: &str) -> DbResult<bool> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM topics WHERE name = ?1")
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(DbError::QueryFailed)?;

    Ok(count > 0)
}

/// Get all topic names (flat list, no hierarchy).
pub async fn get_all_topic_names(pool: &DbPool) -> DbResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM topics ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(DbError::QueryFailed)?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::db::{init_memory_pool, run_migrations};

    async fn setup_test_db() -> DbPool {
        let pool = init_memory_pool().await.expect("Failed to create pool");
        run_migrations(&pool).await.expect("Migrations failed");
        pool
    }

    async fn insert_test_topic(pool: &DbPool, name: &str, parent: Option<&str>, kind: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO topics (name, kind, parent_topic_name, created, last_updated, brief, summary, when_to_use) VALUES (?1, ?2, ?3, ?4, ?5, '', '', '')",
        )
        .bind(name)
        .bind(kind)
        .bind(parent)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .expect("Failed to insert topic");
    }

    #[tokio::test]
    async fn test_get_root_topic_names() {
        let pool = setup_test_db().await;

        insert_test_topic(&pool, "root1", None, "Person").await;
        insert_test_topic(&pool, "root2", None, "SolutionArea").await;
        insert_test_topic(&pool, "child1", Some("root1"), "Person").await;

        let roots = get_root_topic_names(&pool).await.unwrap();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&"root1".to_string()));
        assert!(roots.contains(&"root2".to_string()));
        assert!(!roots.contains(&"child1".to_string()));
    }

    #[tokio::test]
    async fn test_get_child_topic_names() {
        let pool = setup_test_db().await;

        insert_test_topic(&pool, "parent", None, "Person").await;
        insert_test_topic(&pool, "child1", Some("parent"), "Person").await;
        insert_test_topic(&pool, "child2", Some("parent"), "Person").await;
        insert_test_topic(&pool, "other", None, "Person").await;

        let children = get_child_topic_names(&pool, "parent").await.unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"child1".to_string()));
        assert!(children.contains(&"child2".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_descendant_names_3_levels() {
        let pool = setup_test_db().await;

        // Create 3-level hierarchy: root -> level1 -> level2
        insert_test_topic(&pool, "root", None, "Person").await;
        insert_test_topic(&pool, "level1a", Some("root"), "Person").await;
        insert_test_topic(&pool, "level1b", Some("root"), "Person").await;
        insert_test_topic(&pool, "level2a", Some("level1a"), "Person").await;
        insert_test_topic(&pool, "level2b", Some("level1a"), "Person").await;

        let descendants = get_all_descendant_names(&pool, "root").await.unwrap();
        assert_eq!(descendants.len(), 5); // root + 2 level1 + 2 level2
        assert!(descendants.contains(&"root".to_string()));
        assert!(descendants.contains(&"level1a".to_string()));
        assert!(descendants.contains(&"level1b".to_string()));
        assert!(descendants.contains(&"level2a".to_string()));
        assert!(descendants.contains(&"level2b".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_descendant_names_leaf_node() {
        let pool = setup_test_db().await;

        insert_test_topic(&pool, "leaf", None, "Person").await;

        let descendants = get_all_descendant_names(&pool, "leaf").await.unwrap();
        assert_eq!(descendants.len(), 1);
        assert!(descendants.contains(&"leaf".to_string()));
    }

    #[tokio::test]
    async fn test_count_topics() {
        let pool = setup_test_db().await;

        assert_eq!(count_topics(&pool).await.unwrap(), 0);

        insert_test_topic(&pool, "topic1", None, "Person").await;
        assert_eq!(count_topics(&pool).await.unwrap(), 1);

        insert_test_topic(&pool, "topic2", None, "Person").await;
        assert_eq!(count_topics(&pool).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_topic_exists() {
        let pool = setup_test_db().await;

        assert!(!topic_exists(&pool, "missing").await.unwrap());

        insert_test_topic(&pool, "exists", None, "Person").await;
        assert!(topic_exists(&pool, "exists").await.unwrap());
        assert!(!topic_exists(&pool, "still-missing").await.unwrap());
    }

    #[tokio::test]
    async fn test_fetch_topic_with_children_simple() {
        let pool = setup_test_db().await;

        insert_test_topic(&pool, "simple", None, "Person").await;

        let topic = fetch_topic_with_children(&pool, "simple").await.unwrap();
        assert!(topic.is_some());

        let topic = topic.unwrap();
        assert_eq!(topic.name(), "simple");
        assert!(matches!(topic.kind(), KindCategory::Person));
        assert!(topic.children().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_topic_with_children_hierarchy() {
        let pool = setup_test_db().await;

        insert_test_topic(&pool, "parent", None, "SolutionArea").await;
        insert_test_topic(&pool, "child1", Some("parent"), "Person").await;
        insert_test_topic(&pool, "child2", Some("parent"), "Person").await;
        insert_test_topic(&pool, "grandchild", Some("child1"), "Person").await;

        let topic = fetch_topic_with_children(&pool, "parent").await.unwrap().unwrap();
        assert_eq!(topic.name(), "parent");
        assert_eq!(topic.children().len(), 2);

        // Find child1 and check its grandchild
        let child1 = topic.children().iter().find(|c| c.name() == "child1").unwrap();
        assert_eq!(child1.children().len(), 1);
        assert_eq!(child1.children()[0].name(), "grandchild");

        // child2 has no children
        let child2 = topic.children().iter().find(|c| c.name() == "child2").unwrap();
        assert!(child2.children().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_topic_not_found() {
        let pool = setup_test_db().await;

        let topic = fetch_topic_with_children(&pool, "nonexistent").await.unwrap();
        assert!(topic.is_none());
    }
}
