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
    pub license_title: Option<String>,
    pub license_url: Option<String>,
    pub total: Option<usize>,
}

/// A SQLite-backed cache of fetched Iconify icon bodies and set metadata.
#[derive(Clone)]
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
        let mut conn = Self::open_conn(&path)?;

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap_or(0);

        if version < 1 {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS icons (
                    prefix     TEXT NOT NULL,
                    name       TEXT NOT NULL,
                    body       TEXT NOT NULL,
                    left       INTEGER NOT NULL DEFAULT 0,
                    top        INTEGER NOT NULL DEFAULT 0,
                    width      INTEGER NOT NULL,
                    height     INTEGER NOT NULL,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (prefix, name)
                );
                CREATE INDEX IF NOT EXISTS idx_icons_name ON icons(name);

                CREATE TABLE IF NOT EXISTS sets (
                    prefix          TEXT PRIMARY KEY,
                    title           TEXT NOT NULL,
                    license         TEXT,
                    license_title   TEXT,
                    license_url     TEXT,
                    total           INTEGER,
                    fetched_at      TEXT NOT NULL
                );
                "#,
            )
            .map_err(map_sql)?;

            // Migrate from previous schema: inspect columns and adapt.
            let cols: Vec<String> = conn
                .prepare("PRAGMA table_info(icons)")
                .map_err(map_sql)?
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(map_sql)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_sql)?;

            let has_view_box = cols.contains(&"view_box".to_string());
            let has_left = cols.contains(&"left".to_string());
            let has_top = cols.contains(&"top".to_string());

            if !has_left {
                conn.execute("ALTER TABLE icons ADD COLUMN left INTEGER NOT NULL DEFAULT 0", [])
                    .map_err(map_sql)?;
            }
            if !has_top {
                conn.execute("ALTER TABLE icons ADD COLUMN top INTEGER NOT NULL DEFAULT 0", [])
                    .map_err(map_sql)?;
            }

            if has_view_box {
                let tx = conn.transaction().map_err(map_sql)?;
                {
                    let mut stmt = tx
                        .prepare("SELECT rowid, view_box FROM icons")
                        .map_err(map_sql)?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                        })
                        .map_err(map_sql)?;
                    let updates: Vec<(i64, i32, i32)> = rows
                        .map(|r| {
                            let (rowid, view_box) = r?;
                            let mut parts = view_box.split_whitespace();
                            let left = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                            let top = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                            Ok((rowid, left, top))
                        })
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(map_sql)?;
                    drop(stmt);
                    for (rowid, left, top) in updates {
                        tx.execute(
                            "UPDATE icons SET left = ?1, top = ?2 WHERE rowid = ?3",
                            params![left, top, rowid],
                        )
                        .map_err(map_sql)?;
                    }
                }
                tx.commit().map_err(map_sql)?;
                conn.execute("ALTER TABLE icons DROP COLUMN view_box", [])
                    .map_err(map_sql)?;
            }

            let set_cols: Vec<String> = conn
                .prepare("PRAGMA table_info(sets)")
                .map_err(map_sql)?
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(map_sql)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_sql)?;
            if !set_cols.contains(&"license_title".to_string()) {
                conn.execute("ALTER TABLE sets ADD COLUMN license_title TEXT", [])
                    .map_err(map_sql)?;
            }
            if !set_cols.contains(&"license_url".to_string()) {
                conn.execute("ALTER TABLE sets ADD COLUMN license_url TEXT", [])
                    .map_err(map_sql)?;
            }

            conn.execute_batch("PRAGMA user_version = 1;")
                .map_err(map_sql)?;
        }

        if version < 2 {
            let tx = conn.transaction().map_err(map_sql)?;

            let set_cols: Vec<String> = tx
                .prepare("PRAGMA table_info(sets)")
                .map_err(map_sql)?
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(map_sql)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_sql)?;

            if !set_cols.contains(&"total".to_string()) {
                tx.execute("ALTER TABLE sets ADD COLUMN total INTEGER", [])
                    .map_err(map_sql)?;
            }

            tx.execute_batch("PRAGMA user_version = 2;")
                .map_err(map_sql)?;
            tx.commit().map_err(map_sql)?;
        }

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

    fn open_conn(path: &Path) -> Result<Connection> {
        let conn = Connection::open(path).map_err(map_sql)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")
            .map_err(map_sql)?;
        Ok(conn)
    }

    fn conn(&self) -> Result<Connection> {
        Self::open_conn(&self.path)
    }

    /// Looks up a cached body by prefix and name.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn get(&self, prefix: &str, name: &str) -> Result<Option<IconBody>> {
        self.conn()?
            .query_row(
                "SELECT body, left, top, width, height FROM icons WHERE prefix = ?1 AND name = ?2",
                params![prefix, name],
                |row| {
                    let body = row.get::<_, String>(0)?;
                    let left: i32 = row.get(1)?;
                    let top: i32 = row.get(2)?;
                    let width: u32 = row.get(3)?;
                    let height: u32 = row.get(4)?;
                    Ok(IconBody::with_origin(body, width, height, left, top))
                },
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
                "INSERT OR REPLACE INTO icons (prefix, name, body, left, top, width, height, fetched_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![prefix, name, body.body, body.left, body.top, body.width, body.height],
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
                "INSERT OR REPLACE INTO sets (prefix, title, license, license_title, license_url, total, fetched_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![
                    info.prefix,
                    info.title,
                    info.license.as_deref(),
                    info.license_title.as_deref(),
                    info.license_url.as_deref(),
                    info.total.map(|v| v as i64)
                ],
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
                "SELECT prefix, title, license, license_title, license_url, total FROM sets \
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
                    license_title: row.get(3)?,
                    license_url: row.get(4)?,
                    total: row.get::<_, Option<i64>>(5)?.map(|v| v as usize),
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
            .prepare("SELECT prefix, title, license, license_title, license_url, total FROM sets ORDER BY prefix")
            .map_err(map_sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SetInfo {
                    prefix: row.get(0)?,
                    title: row.get(1)?,
                    license: row.get(2)?,
                    license_title: row.get(3)?,
                    license_url: row.get(4)?,
                    total: row.get::<_, Option<i64>>(5)?.map(|v| v as usize),
                })
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Returns a map of prefix → cached icon count for the given prefixes.
    ///
    /// Empty input returns an empty map without opening a query.
    /// Prefixes with zero cached rows are omitted; presentation treats a
    /// missing entry as zero.
    ///
    /// ## Errors
    /// [`IconError::Cache`] on query failure.
    pub fn cached_icon_counts(
        &self,
        prefixes: &[String],
    ) -> Result<std::collections::BTreeMap<String, usize>> {
        if prefixes.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }

        let placeholders = prefixes.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT prefix, COUNT(*) FROM icons WHERE prefix IN ({}) GROUP BY prefix",
            placeholders
        );

        let conn = self.conn()?;
        let mut stmt = conn.prepare(&sql).map_err(map_sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(prefixes.iter()), |row| {
                let prefix: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((prefix, count as usize))
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<std::collections::BTreeMap<_, _>>>()
            .map_err(map_sql)
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
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "Material Design".into(), license: Some("Apache-2.0".into()), license_title: Some("Apache License 2.0".into()), license_url: None, total: None }).unwrap();
        let hits = cache.search_sets("material").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].prefix, "mdi");
        assert_eq!(hits[0].title, "Material Design");
        assert_eq!(hits[0].license, Some("Apache-2.0".into()));
    }

    #[test]
    fn set_metadata_preserves_null_license() {
        let (_d, cache) = temp_cache();
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "Material Design".into(), license: None, license_title: None, license_url: None, total: None }).unwrap();
        let hits = cache.all_sets().unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].license, None);
    }

    #[test]
    fn clear_empties_the_cache() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "M".into(), license: None, license_title: None, license_url: None, total: None }).unwrap();
        cache.clear().unwrap();
        assert_eq!(cache.get("mdi", "home").unwrap(), None);
        assert!(cache.all_sets().unwrap().is_empty());
    }

    #[test]
    fn migration_from_old_schema_preserves_view_box_origin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icons.db");

        // Create a v0 schema database manually.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE icons (
                    prefix     TEXT NOT NULL,
                    name       TEXT NOT NULL,
                    body       TEXT NOT NULL,
                    view_box   TEXT NOT NULL,
                    width      INTEGER,
                    height     INTEGER,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (prefix, name)
                );
                CREATE TABLE sets (
                    prefix      TEXT PRIMARY KEY,
                    title       TEXT,
                    license     TEXT,
                    fetched_at  TEXT NOT NULL
                );
                PRAGMA user_version = 0;
                "#,
            )
            .unwrap();
            conn.execute(
                "INSERT INTO icons (prefix, name, body, view_box, width, height, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["mdi", "home", "<path/>", "10 20 32 32", 32, 32, "2024-01-01T00:00:00Z"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO icons (prefix, name, body, view_box, width, height, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["mdi", "alert", "<circle/>", "-5 -10 24 24", 24, 24, "2024-01-01T00:00:00Z"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sets (prefix, title, license, fetched_at) VALUES (?1, ?2, ?3, ?4)",
                params!["mdi", "Material Design", "Apache-2.0", "2024-01-01T00:00:00Z"],
            )
            .unwrap();
        }

        // Opening with the new code should migrate transparently.
        let cache = IconCache::open_at(&path).unwrap();

        // Non-zero origin must survive migration and appear in assembled SVG.
        let body = cache.get("mdi", "home").unwrap().expect("old row readable");
        assert_eq!(body.body, "<path/>");
        assert_eq!(body.width, 32);
        assert_eq!(body.height, 32);
        assert_eq!(body.left, 10);
        assert_eq!(body.top, 20);
        let icon = crate::Icon::from_network("mdi:home", body);
        let svg = icon.svg();
        assert!(svg.contains("viewBox=\"10 20 32 32\""), "non-zero viewBox lost in migration; got: {svg}");

        // Negative origin must also survive migration.
        let neg = cache.get("mdi", "alert").unwrap().expect("negative-origin row readable");
        assert_eq!(neg.left, -5);
        assert_eq!(neg.top, -10);
        let neg_icon = crate::Icon::from_network("mdi:alert", neg);
        assert!(neg_icon.svg().contains("viewBox=\"-5 -10 24 24\""));

        // Old set row should be readable with null license extras.
        let sets = cache.all_sets().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].prefix, "mdi");
        assert_eq!(sets[0].license, Some("Apache-2.0".into()));
        assert_eq!(sets[0].license_title, None);
        assert_eq!(sets[0].license_url, None);

        // Writes to the new columns should succeed.
        cache
            .put("custom", "logo", &IconBody::with_origin("<path/>", 32, 32, 10, 20))
            .unwrap();
        let new_body = cache.get("custom", "logo").unwrap().expect("new row readable");
        assert_eq!(new_body.left, 10);
        assert_eq!(new_body.top, 20);

        cache
            .put_set(&SetInfo {
                prefix: "lucide".into(),
                title: "Lucide".into(),
                license: Some("ISC".into()),
                license_title: Some("ISC License".into()),
                license_url: Some("https://opensource.org/licenses/ISC".into()),
                total: None,
            })
            .unwrap();
        let new_sets = cache.search_sets("lucide").unwrap();
        assert_eq!(new_sets[0].license_title, Some("ISC License".into()));
        assert_eq!(new_sets[0].license_url, Some("https://opensource.org/licenses/ISC".into()));
    }

    #[test]
    fn timestamp_is_rfc3339_utc() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<path/>", 24, 24)).unwrap();
        cache.put_set(&SetInfo { prefix: "mdi".into(), title: "M".into(), license: None, license_title: None, license_url: None, total: None }).unwrap();

        let conn = Connection::open(cache.path()).unwrap();
        let icon_ts: String = conn
            .query_row("SELECT fetched_at FROM icons WHERE prefix = 'mdi'", [], |row| row.get(0))
            .unwrap();
        let set_ts: String = conn
            .query_row("SELECT fetched_at FROM sets WHERE prefix = 'mdi'", [], |row| row.get(0))
            .unwrap();

        for ts in [icon_ts, set_ts] {
            assert!(ts.ends_with('Z'), "expected RFC 3339 UTC suffix; got: {ts}");
            assert!(ts.contains('T'), "expected RFC 3339 date/time separator; got: {ts}");
        }
    }

    #[test]
    fn fresh_db_is_user_version_2_with_total_column() {
        let (_d, cache) = temp_cache();
        let conn = Connection::open(cache.path()).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2, "fresh DB should be at user_version 2");

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(sets)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(cols.contains(&"total".to_string()), "sets table should have total column");
    }

    #[test]
    fn v0_db_migrates_to_v2_preserving_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icons.db");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE icons (
                    prefix     TEXT NOT NULL,
                    name       TEXT NOT NULL,
                    body       TEXT NOT NULL,
                    view_box   TEXT NOT NULL,
                    width      INTEGER,
                    height      INTEGER,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (prefix, name)
                );
                CREATE TABLE sets (
                    prefix      TEXT PRIMARY KEY,
                    title       TEXT,
                    license     TEXT,
                    fetched_at  TEXT NOT NULL
                );
                PRAGMA user_version = 0;
                "#,
            )
            .unwrap();
            conn.execute(
                "INSERT INTO icons (prefix, name, body, view_box, width, height, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["mdi", "home", "<path/>", "0 0 24 24", 24, 24, "2024-01-01T00:00:00Z"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sets (prefix, title, license, fetched_at) VALUES (?1, ?2, ?3, ?4)",
                params!["mdi", "Material Design", "Apache-2.0", "2024-01-01T00:00:00Z"],
            )
            .unwrap();
        }

        let cache = IconCache::open_at(&path).unwrap();

        let conn = Connection::open(cache.path()).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);

        let sets = cache.all_sets().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].prefix, "mdi");
        assert_eq!(sets[0].total, None, "existing v0 set total should be None after migration");

        let body = cache.get("mdi", "home").unwrap().expect("icon should survive migration");
        assert_eq!(body.body, "<path/>");
    }

    #[test]
    fn v1_db_migrates_to_v2() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icons.db");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE icons (
                    prefix     TEXT NOT NULL,
                    name       TEXT NOT NULL,
                    body       TEXT NOT NULL,
                    left       INTEGER NOT NULL DEFAULT 0,
                    top        INTEGER NOT NULL DEFAULT 0,
                    width      INTEGER NOT NULL,
                    height     INTEGER NOT NULL,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (prefix, name)
                );
                CREATE TABLE sets (
                    prefix          TEXT PRIMARY KEY,
                    title           TEXT NOT NULL,
                    license         TEXT,
                    license_title   TEXT,
                    license_url     TEXT,
                    fetched_at      TEXT NOT NULL
                );
                PRAGMA user_version = 1;
                "#,
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sets (prefix, title, license, license_title, license_url, fetched_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["mdi", "Material Design", "Apache-2.0", "Apache License 2.0", "https://example.com", "2024-01-01T00:00:00Z"],
            )
            .unwrap();
        }

        let cache = IconCache::open_at(&path).unwrap();

        let conn = Connection::open(cache.path()).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);

        let sets = cache.all_sets().unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].total, None, "existing v1 set total should be None after migration");
        assert_eq!(sets[0].license_title, Some("Apache License 2.0".into()));
    }

    #[test]
    fn idempotent_migration_when_total_column_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icons.db");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                r#"
                CREATE TABLE icons (
                    prefix     TEXT NOT NULL,
                    name       TEXT NOT NULL,
                    body       TEXT NOT NULL,
                    left       INTEGER NOT NULL DEFAULT 0,
                    top        INTEGER NOT NULL DEFAULT 0,
                    width      INTEGER NOT NULL,
                    height     INTEGER NOT NULL,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (prefix, name)
                );
                CREATE TABLE sets (
                    prefix          TEXT PRIMARY KEY,
                    title           TEXT NOT NULL,
                    license         TEXT,
                    license_title   TEXT,
                    license_url     TEXT,
                    total           INTEGER,
                    fetched_at      TEXT NOT NULL
                );
                PRAGMA user_version = 1;
                "#,
            )
            .unwrap();
        }

        // Opening should not fail with duplicate-column error.
        let cache = IconCache::open_at(&path).unwrap();

        let conn = Connection::open(cache.path()).unwrap();
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn set_info_total_round_trips() {
        let (_d, cache) = temp_cache();
        cache
            .put_set(&SetInfo {
                prefix: "mdi".into(),
                title: "Material Design".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: Some(7000),
            })
            .unwrap();

        let by_search = cache.search_sets("material").unwrap();
        assert_eq!(by_search.len(), 1);
        assert_eq!(by_search[0].total, Some(7000));

        let by_all = cache.all_sets().unwrap();
        assert_eq!(by_all.len(), 1);
        assert_eq!(by_all[0].total, Some(7000));

        cache
            .put_set(&SetInfo {
                prefix: "lucide".into(),
                title: "Lucide".into(),
                license: None,
                license_title: None,
                license_url: None,
                total: None,
            })
            .unwrap();

        let all = cache.all_sets().unwrap();
        let lucide = all.iter().find(|s| s.prefix == "lucide").unwrap();
        assert_eq!(lucide.total, None);
    }

    #[test]
    fn cached_icon_counts_groups_and_omits_zero() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put("mdi", "alert", &IconBody::new("<b/>", 24, 24)).unwrap();
        cache.put("ic", "baseline-apple", &IconBody::new("<c/>", 24, 24)).unwrap();

        let counts = cache.cached_icon_counts(&["mdi".into(), "ic".into(), "unknown".into()]).unwrap();
        assert_eq!(counts.get("mdi"), Some(&2usize));
        assert_eq!(counts.get("ic"), Some(&1usize));
        assert!(!counts.contains_key("unknown"), "zero-count prefix should be omitted");
    }

    #[test]
    fn cached_icon_counts_empty_input_returns_empty_map() {
        let (_d, cache) = temp_cache();
        let counts = cache.cached_icon_counts(&[]).unwrap();
        assert!(counts.is_empty());
    }

    #[test]
    fn cached_icon_counts_excludes_embedded_domain_icons() {
        // Embedded domain icons are not in the SQLite cache, so they are never counted.
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();

        let counts = cache.cached_icon_counts(&["ic".into()]).unwrap();
        assert!(counts.is_empty(), "embedded domain prefix should have no cached count");
    }
}
