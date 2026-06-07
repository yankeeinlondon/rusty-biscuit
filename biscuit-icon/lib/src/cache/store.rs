use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::body::IconBody;
use crate::error::{IconError, Result};

/// Metadata about an Iconify icon set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetInfo {
    pub prefix: String,
    pub title: String,
    pub license: Option<String>,
}

/// A SQLite-backed cache of fetched Iconify icon bodies and set metadata.
pub struct IconCache {
    path: PathBuf,
}

fn map_sql<E: std::fmt::Display>(e: E) -> IconError {
    IconError::Cache(e.to_string())
}

impl IconCache {
    /// Opens (creating if needed) the cache at the default user location
    /// `~/.cache/biscuit-icon/icons.db`.
    ///
    /// ## Errors
    /// [`IconError::Cache`] if the directory or database cannot be created.
    pub fn open_default() -> Result<Self> {
        let dir = Self::default_dir()?;
        std::fs::create_dir_all(&dir).map_err(map_sql)?;
        Self::open_at(dir.join("icons.db"))
    }

    /// Opens (creating if needed) a cache at an explicit path (used in tests).
    ///
    /// ## Errors
    /// [`IconError::Cache`] on connection or schema failure.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(&path).map_err(map_sql)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS icons (
                prefix     TEXT NOT NULL,
                name       TEXT NOT NULL,
                body       TEXT NOT NULL,
                width      INTEGER NOT NULL,
                height     INTEGER NOT NULL,
                fetched_at TEXT NOT NULL,
                PRIMARY KEY (prefix, name)
            );
            CREATE INDEX IF NOT EXISTS idx_icons_name ON icons(name);

            CREATE TABLE IF NOT EXISTS sets (
                prefix     TEXT PRIMARY KEY,
                title      TEXT NOT NULL,
                license    TEXT,
                fetched_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(map_sql)?;
        drop(conn);
        Ok(Self { path })
    }

    /// The filesystem path of this cache database.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn default_dir() -> Result<PathBuf> {
        let base = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
        } else {
            dirs::home_dir().ok_or_else(|| IconError::Cache("no home dir".into()))?
        };
        Ok(base.join(".cache").join("biscuit-icon"))
    }

    fn conn(&self) -> Result<Connection> {
        Connection::open(&self.path).map_err(map_sql)
    }

    /// Looks up a cached body by prefix and name.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn get(&self, prefix: &str, name: &str) -> Result<Option<IconBody>> {
        self.conn()?
            .query_row(
                "SELECT body, width, height FROM icons WHERE prefix = ?1 AND name = ?2",
                params![prefix, name],
                |row| Ok(IconBody::new(row.get::<_, String>(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(map_sql)
    }

    /// Inserts or replaces a cached body.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on write failure.
    pub fn put(&self, prefix: &str, name: &str, body: &IconBody) -> Result<()> {
        self.conn()?
            .execute(
                "INSERT OR REPLACE INTO icons (prefix, name, body, width, height, fetched_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![prefix, name, body.body, body.width, body.height],
            )
            .map(|_| ())
            .map_err(map_sql)
    }

    /// Returns cached `prefix:name` ids whose name contains `needle` (for completions).
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn search_names(&self, needle: &str) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT prefix, name FROM icons WHERE name LIKE ?1 ORDER BY prefix, name")
            .map_err(map_sql)?;
        let like = format!("%{needle}%");
        let rows = stmt
            .query_map(params![like], |row| {
                Ok(format!("{}:{}", row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Returns every distinct cached prefix.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn cached_prefixes(&self) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT DISTINCT prefix FROM icons ORDER BY prefix").map_err(map_sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Inserts or replaces set metadata.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on write failure.
    pub fn put_set(&self, info: &SetInfo) -> Result<()> {
        self.conn()?
            .execute(
                "INSERT OR REPLACE INTO sets (prefix, title, license, fetched_at) \
                 VALUES (?1, ?2, ?3, datetime('now'))",
                params![info.prefix, info.title, info.license.as_deref().unwrap_or_default()],
            )
            .map(|_| ())
            .map_err(map_sql)
    }

    /// Returns cached set metadata whose prefix or title contains `needle`.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn search_sets(&self, needle: &str) -> Result<Vec<SetInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT prefix, title, license FROM sets \
                 WHERE prefix LIKE ?1 OR title LIKE ?1 ORDER BY prefix",
            )
            .map_err(map_sql)?;
        let like = format!("%{needle}%");
        let rows = stmt
            .query_map(params![like], |row| {
                Ok(SetInfo {
                    prefix: row.get(0)?,
                    title: row.get(1)?,
                    license: row.get(2)?,
                })
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Returns all cached set metadata.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn all_sets(&self) -> Result<Vec<SetInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT prefix, title, license FROM sets ORDER BY prefix")
            .map_err(map_sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SetInfo {
                    prefix: row.get(0)?,
                    title: row.get(1)?,
                    license: row.get(2)?,
                })
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Deletes all cached rows.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on delete failure.
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM icons", []).map_err(map_sql)?;
        conn.execute("DELETE FROM sets", []).map_err(map_sql)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache() -> (tempfile::TempDir, IconCache) {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        (dir, cache)
    }

    #[test]
    fn put_then_get_round_trips() {
        let (_d, cache) = temp_cache();
        let body = IconBody::new("<path/>", 24, 24);
        cache.put("mdi", "home", &body).unwrap();
        assert_eq!(cache.get("mdi", "home").unwrap(), Some(body));
    }

    #[test]
    fn get_miss_is_none() {
        let (_d, cache) = temp_cache();
        assert_eq!(cache.get("mdi", "ghost").unwrap(), None);
    }

    #[test]
    fn search_names_matches_substring() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put("mdi", "home-outline", &IconBody::new("<b/>", 24, 24)).unwrap();
        cache.put("mdi", "alert", &IconBody::new("<c/>", 24, 24)).unwrap();
        let hits = cache.search_names("home").unwrap();
        assert_eq!(hits, vec!["mdi:home", "mdi:home-outline"]);
    }

    #[test]
    fn set_metadata_round_trips() {
        let (_d, cache) = temp_cache();
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "Material Design".into(), license: Some("Apache-2.0".into()) }).unwrap();
        let hits = cache.search_sets("material").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].prefix, "mdi");
        assert_eq!(hits[0].title, "Material Design");
    }

    #[test]
    fn clear_empties_the_cache() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "M".into(), license: None }).unwrap();
        cache.clear().unwrap();
        assert_eq!(cache.get("mdi", "home").unwrap(), None);
        assert!(cache.all_sets().unwrap().is_empty());
    }
}
