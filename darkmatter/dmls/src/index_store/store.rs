//! [`IndexStore`]: one root's SQLite store of [`FileFacts`].
//!
//! The store is opened in WAL mode with `synchronous = NORMAL` and a 5 s busy
//! timeout, so several processes can share it. Every table is `STRICT`: a
//! wrongly typed value cannot be written, and a value that does not decode on
//! read is [`StoreFailure::Corrupt`], never an empty result. Each holder keeps
//! the shared liveness lock (see [`super::lock`]) for the connection's
//! lifetime.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params};

use crate::clock::Clock;
use crate::graph::{HeadingFact, LinkFact, TransclusionFact, WikiLinkFact, classify_link_target};

use super::facts::FileFacts;
use super::failure::StoreFailure;
use super::keys::target_keys;
use super::location::{lock_path, touch_mtime};
use super::lock::{SHARED_LOCK_WAIT, StoreLock};
use super::{FACTS_VERSION, STORE_FORMAT};

/// Files written per `BEGIN IMMEDIATE` transaction.
pub const BATCH_SIZE: usize = 200;

/// SQLite's busy timeout for every store connection.
pub const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// How long open retries a `journal_mode=WAL` switch that reports busy. On a
/// new file SQLite answers busy at once, without honoring the busy timeout.
pub const WAL_SWITCH_RETRY: Duration = Duration::from_secs(2);

/// The minimum interval between liveness touches after open.
pub const TOUCH_INTERVAL: Duration = Duration::from_secs(60 * 60);

const WAL_RETRY_SLEEP: Duration = Duration::from_millis(5);

/// Every table of the layout; a store missing any is corrupt.
const TABLES: [&str; 7] = ["meta", "files", "headings", "links", "wiki", "transclusions", "cand"];

const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
CREATE TABLE files (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, hash INTEGER NOT NULL) STRICT;
CREATE TABLE headings (
    file_id INTEGER NOT NULL, ord INTEGER NOT NULL, level INTEGER NOT NULL,
    title TEXT NOT NULL, slug TEXT NOT NULL,
    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL,
    title_start INTEGER NOT NULL, title_end INTEGER NOT NULL, line INTEGER NOT NULL,
    PRIMARY KEY (file_id, ord)
) STRICT;
CREATE TABLE links (
    file_id INTEGER NOT NULL, ord INTEGER NOT NULL, raw_target TEXT NOT NULL,
    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL, line INTEGER NOT NULL,
    PRIMARY KEY (file_id, ord)
) STRICT;
CREATE TABLE wiki (
    file_id INTEGER NOT NULL, ord INTEGER NOT NULL, target TEXT NOT NULL,
    heading TEXT, alias TEXT,
    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL,
    target_start INTEGER NOT NULL, target_end INTEGER NOT NULL,
    heading_start INTEGER, heading_end INTEGER,
    line INTEGER NOT NULL, unsupported INTEGER NOT NULL,
    PRIMARY KEY (file_id, ord)
) STRICT;
CREATE TABLE transclusions (
    file_id INTEGER NOT NULL, ord INTEGER NOT NULL, raw_target TEXT NOT NULL, path TEXT NOT NULL,
    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL, line INTEGER NOT NULL,
    PRIMARY KEY (file_id, ord)
) STRICT;
CREATE TABLE cand (file_id INTEGER NOT NULL, key TEXT NOT NULL, PRIMARY KEY (file_id, key)) STRICT;
CREATE INDEX cand_key ON cand (key);
";

/// Open-time knobs; [`OpenSettings::default`] is production.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OpenSettings {
    pub wal_retry: Duration,
    pub lock_wait: Duration,
    pub busy_timeout: Duration,
}

impl Default for OpenSettings {
    fn default() -> Self {
        Self { wal_retry: WAL_SWITCH_RETRY, lock_wait: SHARED_LOCK_WAIT, busy_timeout: BUSY_TIMEOUT }
    }
}

/// What opening found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenReport {
    /// The store file was new (or empty) and its tables were created.
    pub created: bool,
    /// Files with a row.
    pub rows: usize,
}

/// What one [`IndexStore::upsert_batch`] call did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpsertReport {
    /// Rows inserted or replaced.
    pub written: usize,
    /// Rows left alone because the stored hash already matched.
    pub skipped: usize,
}

/// One root's store.
#[derive(Debug)]
pub struct IndexStore {
    conn: Connection,
    path: PathBuf,
    clock: Arc<dyn Clock>,
    last_touch: Cell<Instant>,
    /// Held for the connection's lifetime; dropped after `conn`.
    _lock: StoreLock,
}

impl IndexStore {
    /// Opens (creating when missing) the store at `path` for `root`, the
    /// effective workspace root as the file list spells it.
    ///
    /// ## Errors
    ///
    /// - [`StoreFailure::Corrupt`]: not a database, malformed, a missing
    ///   table, or a format stamp other than this build's
    /// - [`StoreFailure::RootMismatch`]: the store records another root
    /// - [`StoreFailure::Busy`]: the lock, the busy timeout, or the WAL switch
    ///   stayed busy
    /// - [`StoreFailure::Unavailable`]: the file cannot be created, opened,
    ///   or written
    pub fn open(path: &Path, root: &Path, clock: Arc<dyn Clock>) -> Result<(Self, OpenReport), StoreFailure> {
        Self::open_with(path, root, clock, OpenSettings::default())
    }

    pub(crate) fn open_with(
        path: &Path,
        root: &Path,
        clock: Arc<dyn Clock>,
        settings: OpenSettings,
    ) -> Result<(Self, OpenReport), StoreFailure> {
        let root_text = path_text(root)?;
        let lock = StoreLock::shared(&lock_path(path), settings.lock_wait).map_err(|error| match error.kind() {
            io::ErrorKind::WouldBlock => StoreFailure::Busy("store lock held exclusively".to_string()),
            _ => StoreFailure::from(error),
        })?;
        create_private_file(path)?;
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(settings.busy_timeout)?;
        switch_to_wal(&conn, settings.wal_retry)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let created = initialize(&mut conn, &root_text)?;
        let rows: usize = conn.query_row("SELECT count(*) FROM files", [], |row| row.get(0))?;
        let last_touch = clock.now();
        let store = Self { conn, path: path.to_path_buf(), clock, last_touch: Cell::new(last_touch), _lock: lock };
        store.touch(SystemTime::now())?;
        Ok((store, OpenReport { created, rows }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `facts` in transactions of [`BATCH_SIZE`] files, replacing a
    /// file's row by path and skipping it when the stored hash matches.
    pub fn upsert_batch(&mut self, facts: &[FileFacts]) -> Result<UpsertReport, StoreFailure> {
        let mut report = UpsertReport::default();
        for chunk in facts.chunks(BATCH_SIZE) {
            let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            for file in chunk {
                if write_file(&tx, file)? {
                    report.written += 1;
                } else {
                    report.skipped += 1;
                }
            }
            tx.commit()?;
        }
        self.touch_if_due()?;
        Ok(report)
    }

    /// Deletes the rows of files missing on disk; never of a file merely
    /// absent from this process's file list.
    ///
    /// ## Returns
    ///
    /// The number of rows deleted.
    pub fn remove_missing(&mut self) -> Result<usize, StoreFailure> {
        let mut missing: Vec<i64> = Vec::new();
        {
            let mut statement = self.conn.prepare_cached("SELECT id, path FROM files")?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                let path = decode_path(row, 1)?;
                if is_missing(&path) {
                    missing.push(row.get(0)?);
                }
            }
        }
        if missing.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        for id in &missing {
            delete_children(&tx, *id)?;
            tx.execute("DELETE FROM files WHERE id = ?1", [id])?;
        }
        tx.commit()?;
        self.touch_if_due()?;
        Ok(missing.len())
    }

    /// Every stored file and its row hash.
    pub fn hashes(&self) -> Result<HashMap<PathBuf, u64>, StoreFailure> {
        self.touch_if_due()?;
        let mut statement = self.conn.prepare_cached("SELECT path, hash FROM files")?;
        let mut rows = statement.query([])?;
        let mut hashes = HashMap::new();
        while let Some(row) = rows.next()? {
            hashes.insert(decode_path(row, 0)?, decode_hash(row, 1)?);
        }
        Ok(hashes)
    }

    /// The facts of every path in `paths` that has a row, in `paths` order.
    pub fn load(&self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure> {
        self.touch_if_due()?;
        let mut out = Vec::with_capacity(paths.len());
        for path in paths {
            // A path that is not UTF-8 can never have been written.
            let Ok(text) = path_text(path) else { continue };
            let found = self
                .conn
                .prepare_cached("SELECT id, hash FROM files WHERE path = ?1")?
                .query_row([&text], |row| Ok((row.get::<_, i64>(0)?, decode_hash(row, 1)?)))
                .optional()?;
            if let Some((id, hash)) = found {
                out.push(self.read_file(id, path.clone(), hash)?);
            }
        }
        Ok(out)
    }

    /// Every stored file holding the path key or the file-name key of
    /// `target`, sorted by path. `target` itself is included only when it
    /// links to a file of its own name.
    pub fn candidates_for(&self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure> {
        self.touch_if_due()?;
        let [path_key, name_key] = target_keys(target);
        let mut statement = self.conn.prepare_cached(
            "SELECT DISTINCT f.path FROM cand c JOIN files f ON f.id = c.file_id
             WHERE c.key IN (?1, ?2) ORDER BY f.path",
        )?;
        let mut rows = statement.query(params![path_key, name_key])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(decode_path(row, 0)?);
        }
        Ok(out)
    }

    /// Every stored file with its headings in document order, sorted by
    /// path; a file without headings has an empty list.
    pub fn scan_headings(&self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure> {
        self.touch_if_due()?;
        let mut by_id: BTreeMap<i64, (PathBuf, Vec<HeadingFact>)> = BTreeMap::new();
        {
            let mut statement = self.conn.prepare_cached("SELECT id, path FROM files")?;
            let mut rows = statement.query([])?;
            while let Some(row) = rows.next()? {
                by_id.insert(row.get(0)?, (decode_path(row, 1)?, Vec::new()));
            }
        }
        let mut statement = self.conn.prepare_cached(&format!("SELECT file_id, {HEADING_COLUMNS} FROM headings ORDER BY file_id, ord"))?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let heading = decode_heading(row, 1)?;
            match by_id.get_mut(&id) {
                Some((_, headings)) => headings.push(heading),
                None => return Err(StoreFailure::Corrupt(format!("heading row for missing file {id}"))),
            }
        }
        let mut out: Vec<_> = by_id.into_values().collect();
        out.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(out)
    }

    /// Records `now` as `meta.last_opened` and the store file's mtime, so
    /// the sweep sees the store as live. Open touches; every later read or
    /// write touches again once [`TOUCH_INTERVAL`] has passed on the store's
    /// clock.
    pub fn touch(&self, now: SystemTime) -> Result<(), StoreFailure> {
        let seconds = now.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES ('last_opened', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [seconds.to_string()],
        )?;
        touch_mtime(&self.path, now)?;
        self.last_touch.set(self.clock.now());
        Ok(())
    }

    fn touch_if_due(&self) -> Result<(), StoreFailure> {
        if self.clock.now().duration_since(self.last_touch.get()) >= TOUCH_INTERVAL {
            self.touch(SystemTime::now())?;
        }
        Ok(())
    }

    fn read_file(&self, id: i64, path: PathBuf, hash: u64) -> Result<FileFacts, StoreFailure> {
        let headings = self.rows(&format!("SELECT {HEADING_COLUMNS} FROM headings WHERE file_id = ?1 ORDER BY ord"), id, |row| {
            decode_heading(row, 0)
        })?;
        let links = self.rows(
            "SELECT raw_target, span_start, span_end, line FROM links WHERE file_id = ?1 ORDER BY ord",
            id,
            decode_link,
        )?;
        let wiki_links = self.rows(
            "SELECT target, heading, alias, span_start, span_end, target_start, target_end,
                    heading_start, heading_end, line, unsupported
             FROM wiki WHERE file_id = ?1 ORDER BY ord",
            id,
            decode_wiki,
        )?;
        let transclusions = self.rows(
            "SELECT raw_target, path, span_start, span_end, line FROM transclusions WHERE file_id = ?1 ORDER BY ord",
            id,
            decode_transclusion,
        )?;
        let candidate_keys =
            self.rows("SELECT key FROM cand WHERE file_id = ?1", id, |row| row.get::<_, String>(0))?.into_iter().collect();
        Ok(FileFacts { path, hash, headings, links, wiki_links, transclusions, candidate_keys })
    }

    /// Decodes every row of `sql` (bound to `file_id`) through `decode`;
    /// the first row that fails to decode fails the whole read.
    fn rows<T>(&self, sql: &str, file_id: i64, decode: impl Fn(&Row<'_>) -> rusqlite::Result<T>) -> Result<Vec<T>, StoreFailure> {
        let mut statement = self.conn.prepare_cached(sql)?;
        let mut rows = statement.query([file_id])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(decode(row)?);
        }
        Ok(out)
    }
}

/// Creates `path` empty and private (`0600` on Unix) when missing, so SQLite
/// and its `-wal`/`-shm` files (which copy the database file's mode) never
/// start world-readable.
fn create_private_file(path: &Path) -> Result<(), StoreFailure> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn switch_to_wal(conn: &Connection, retry: Duration) -> Result<(), StoreFailure> {
    let deadline = Instant::now() + retry;
    loop {
        match conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0)) {
            Ok(mode) if mode.eq_ignore_ascii_case("wal") => return Ok(()),
            Ok(mode) => return Err(StoreFailure::Unavailable(format!("journal mode stayed {mode}"))),
            Err(error) => match StoreFailure::from(error) {
                StoreFailure::Busy(message) if Instant::now() < deadline => {
                    tracing::trace!(%message, "index store WAL switch busy; retrying");
                    std::thread::sleep(WAL_RETRY_SLEEP);
                }
                StoreFailure::Busy(message) => {
                    return Err(StoreFailure::Busy(format!("WAL switch still busy after {retry:?}: {message}")));
                }
                other => return Err(other),
            },
        }
    }
}

/// Creates the layout in an empty database or validates an existing one.
///
/// ## Returns
///
/// Whether the layout was created.
fn initialize(conn: &mut Connection, root: &str) -> Result<bool, StoreFailure> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let tables: Vec<String> = {
        let mut statement = tx.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
        let names = statement.query_map([], |row| row.get(0))?;
        names.collect::<rusqlite::Result<_>>()?
    };
    let created = tables.is_empty();
    if created {
        tx.execute_batch(SCHEMA)?;
        let mut insert = tx.prepare("INSERT INTO meta (key, value) VALUES (?1, ?2)")?;
        insert.execute(["format", &STORE_FORMAT.to_string()])?;
        insert.execute(["facts_version", &FACTS_VERSION.to_string()])?;
        insert.execute(["root", root])?;
    } else if let Some(missing) = TABLES.iter().find(|table| !tables.iter().any(|name| name == *table)) {
        return Err(StoreFailure::Corrupt(format!("missing table {missing}")));
    }
    let meta: HashMap<String, String> = {
        let mut statement = tx.prepare("SELECT key, value FROM meta")?;
        let pairs = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        pairs.collect::<rusqlite::Result<_>>()?
    };
    for (key, expected) in [("format", STORE_FORMAT), ("facts_version", FACTS_VERSION)] {
        let stored = meta.get(key).map(String::as_str);
        if stored != Some(expected.to_string().as_str()) {
            return Err(StoreFailure::Corrupt(format!("{key} stamp {stored:?} is not {expected}")));
        }
    }
    match meta.get("root") {
        Some(stored) if stored == root => {}
        Some(stored) => return Err(StoreFailure::RootMismatch { stored: stored.clone() }),
        None => return Err(StoreFailure::Corrupt("missing root stamp".to_string())),
    }
    tx.commit()?;
    Ok(created)
}

/// Writes one file's row unless its stored hash already matches.
///
/// ## Returns
///
/// Whether the row was written.
fn write_file(tx: &Transaction<'_>, file: &FileFacts) -> Result<bool, StoreFailure> {
    let path = path_text(&file.path)?;
    // SQLite integers are signed; the hash is stored bit for bit.
    let hash = file.hash as i64;
    let existing = tx
        .prepare_cached("SELECT id, hash FROM files WHERE path = ?1")?
        .query_row([&path], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .optional()?;
    let id = match existing {
        Some((_, stored)) if stored == hash => return Ok(false),
        Some((id, _)) => {
            delete_children(tx, id)?;
            tx.prepare_cached("UPDATE files SET hash = ?1 WHERE id = ?2")?.execute(params![hash, id])?;
            id
        }
        None => {
            tx.prepare_cached("INSERT INTO files (path, hash) VALUES (?1, ?2)")?.execute(params![path, hash])?;
            tx.last_insert_rowid()
        }
    };

    let mut insert = tx.prepare_cached(
        "INSERT INTO headings (file_id, ord, level, title, slug, span_start, span_end, title_start, title_end, line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for (ord, heading) in file.headings.iter().enumerate() {
        insert.execute(params![
            id,
            ord,
            heading.level,
            heading.title,
            heading.slug,
            heading.span.start,
            heading.span.end,
            heading.title_span.start,
            heading.title_span.end,
            heading.line
        ])?;
    }
    let mut insert = tx.prepare_cached(
        "INSERT INTO links (file_id, ord, raw_target, span_start, span_end, line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (ord, link) in file.links.iter().enumerate() {
        insert.execute(params![id, ord, link.raw_target, link.span.start, link.span.end, link.line])?;
    }
    let mut insert = tx.prepare_cached(
        "INSERT INTO wiki (file_id, ord, target, heading, alias, span_start, span_end, target_start, target_end,
                           heading_start, heading_end, line, unsupported)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    for (ord, wiki) in file.wiki_links.iter().enumerate() {
        insert.execute(params![
            id,
            ord,
            wiki.target,
            wiki.heading,
            wiki.alias,
            wiki.span.start,
            wiki.span.end,
            wiki.target_span.start,
            wiki.target_span.end,
            wiki.heading_span.as_ref().map(|span| span.start),
            wiki.heading_span.as_ref().map(|span| span.end),
            wiki.line,
            i64::from(wiki.unsupported)
        ])?;
    }
    let mut insert = tx.prepare_cached(
        "INSERT INTO transclusions (file_id, ord, raw_target, path, span_start, span_end, line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (ord, transclusion) in file.transclusions.iter().enumerate() {
        insert.execute(params![
            id,
            ord,
            transclusion.raw_target,
            transclusion.path,
            transclusion.span.start,
            transclusion.span.end,
            transclusion.line
        ])?;
    }
    let mut insert = tx.prepare_cached("INSERT INTO cand (file_id, key) VALUES (?1, ?2)")?;
    for key in &file.candidate_keys {
        insert.execute(params![id, key])?;
    }
    Ok(true)
}

fn delete_children(tx: &Transaction<'_>, id: i64) -> Result<(), StoreFailure> {
    for table in &TABLES[2..] {
        tx.prepare_cached(&format!("DELETE FROM {table} WHERE file_id = ?1"))?.execute([id])?;
    }
    Ok(())
}

/// The stored spelling of `path`: re-joined from its components, so two
/// spellings `PathBuf` treats as equal (`a\b/c` and `a\b\c` on Windows, a
/// doubled `/`) name one row.
fn path_text(path: &Path) -> Result<String, StoreFailure> {
    path.components()
        .collect::<PathBuf>()
        .into_os_string()
        .into_string()
        .map_err(|_| StoreFailure::Unavailable(format!("path is not UTF-8: {}", path.display())))
}

/// Whether `path` is gone from disk. Any other error (permissions, a
/// transient failure) keeps the row.
pub(super) fn is_missing(path: &Path) -> bool {
    matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

const HEADING_COLUMNS: &str = "level, title, slug, span_start, span_end, title_start, title_end, line";

// The row decoders: the one path every stored value takes back into a fact.
// A value that does not decode is an error, which `StoreFailure::from` turns
// into corruption.

fn invalid(column: usize, message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, message.to_string().into())
}

fn decode_path(row: &Row<'_>, column: usize) -> rusqlite::Result<PathBuf> {
    Ok(PathBuf::from(row.get::<_, String>(column)?))
}

fn decode_hash(row: &Row<'_>, column: usize) -> rusqlite::Result<u64> {
    Ok(row.get::<_, i64>(column)? as u64)
}

fn decode_span(row: &Row<'_>, start: usize) -> rusqlite::Result<Range<usize>> {
    let (from, to): (usize, usize) = (row.get(start)?, row.get(start + 1)?);
    if from > to {
        return Err(invalid(start, "inverted span"));
    }
    Ok(from..to)
}

fn decode_heading(row: &Row<'_>, first: usize) -> rusqlite::Result<HeadingFact> {
    let level: u8 = row.get(first)?;
    if !(1..=6).contains(&level) {
        return Err(invalid(first, "heading level outside 1-6"));
    }
    Ok(HeadingFact {
        level,
        title: row.get(first + 1)?,
        slug: row.get(first + 2)?,
        span: decode_span(row, first + 3)?,
        title_span: decode_span(row, first + 5)?,
        line: row.get(first + 7)?,
    })
}

fn decode_link(row: &Row<'_>) -> rusqlite::Result<LinkFact> {
    let raw_target: String = row.get(0)?;
    Ok(LinkFact { target: classify_link_target(&raw_target), raw_target, span: decode_span(row, 1)?, line: row.get(3)? })
}

fn decode_wiki(row: &Row<'_>) -> rusqlite::Result<WikiLinkFact> {
    let heading_span = match (row.get::<_, Option<usize>>(7)?, row.get::<_, Option<usize>>(8)?) {
        (None, None) => None,
        (Some(_), Some(_)) => Some(decode_span(row, 7)?),
        _ => return Err(invalid(7, "half a heading span")),
    };
    let unsupported = match row.get::<_, i64>(10)? {
        0 => false,
        1 => true,
        _ => return Err(invalid(10, "unsupported flag is not 0 or 1")),
    };
    Ok(WikiLinkFact {
        target: row.get(0)?,
        heading: row.get(1)?,
        alias: row.get(2)?,
        span: decode_span(row, 3)?,
        target_span: decode_span(row, 5)?,
        heading_span,
        line: row.get(9)?,
        unsupported,
    })
}

fn decode_transclusion(row: &Row<'_>) -> rusqlite::Result<TransclusionFact> {
    Ok(TransclusionFact { raw_target: row.get(0)?, path: row.get(1)?, span: decode_span(row, 2)?, line: row.get(4)? })
}
