//! In-memory ring buffer of recent clipboard entries.
//!
//! [`History`] is the daemon's working set: every successful watcher
//! capture and every `POST /set` lands here. It is bounded by both a
//! TTL (default 1 hour) and a hard cap (default 100 entries), with a
//! minimum-entries floor so the most recent items are never evicted by
//! the TTL pass alone.
//!
//! ## Examples
//!
//! ```
//! use biscuit_clipboard::{History, content::ClipboardFormat};
//!
//! let mut history = History::new();
//! history.insert(vec![ClipboardFormat::Text("hello".into())]);
//! assert_eq!(history.len(), 1);
//! assert_eq!(history.latest().unwrap().find_text(), Some("hello"));
//! ```
//!
//! ## Notes
//!
//! Storage is a [`VecDeque`] so insertion at the front and eviction
//! from the back are both O(1) — see review-1 Code Quality #10/#16.

use std::collections::VecDeque;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use crate::content::ClipboardFormat;
use crate::entry::{self, ClipboardEntry, EntryId};

const DEFAULT_TTL: Duration = Duration::from_secs(3600);
const DEFAULT_MIN_ENTRIES: usize = 2;
const DEFAULT_MAX_ENTRIES: usize = 100;

/// Bounded ring buffer of recent [`ClipboardEntry`] values.
///
/// Newest entries are at the front (`latest()` / index `0`); the back
/// is the oldest. Eviction policy:
///
/// 1. After every insert, drop entries past the TTL **except** for the
///    most recent `min_entries` (default 2).
/// 2. Then truncate from the back until `len() <= max_entries` (default
///    100).
pub struct History {
    entries: VecDeque<ClipboardEntry>,
    ttl: Duration,
    min_entries: usize,
    max_entries: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// Build a [`History`] with the spec defaults (1h TTL, 2-entry
    /// floor, 100-entry cap).
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            ttl: DEFAULT_TTL,
            min_entries: DEFAULT_MIN_ENTRIES,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn with_min_entries(mut self, min: usize) -> Self {
        self.min_entries = min;
        self
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Insert a new entry built from `formats`. Returns a tuple:
    ///
    /// 1. `Some(&ClipboardEntry)` — the inserted entry, or `None` if it
    ///    duplicates the head (most-recent) entry.
    /// 2. `Vec<ClipboardEntry>` — every entry removed from history by
    ///    eviction (TTL expiry or max-capacity truncation).
    ///
    /// ## Notes
    ///
    /// The dedup probe uses [`entry::content_hash_of`] — the same
    /// function that derives the entry id — so a re-insert of the same
    /// content always either deduplicates or matches the existing id.
    pub fn insert(
        &mut self,
        formats: Vec<ClipboardFormat>,
    ) -> (Option<&ClipboardEntry>, Vec<ClipboardEntry>) {
        let new_hash = entry::content_hash_of(&formats);

        if self
            .entries
            .front()
            .is_some_and(|e| e.content_hash == new_hash)
        {
            return (None, Vec::new());
        }

        let entry = ClipboardEntry::with_hash(formats, new_hash);

        self.entries.retain(|e| e.id != entry.id);

        self.entries.push_front(entry);
        let evicted = self.evict();
        (self.entries.front(), evicted)
    }

    /// Look up an entry by id.
    pub fn get(&self, id: &EntryId) -> Option<&ClipboardEntry> {
        self.entries.iter().find(|e| e.id == *id)
    }

    /// Look up an entry by id (mutable).
    pub fn get_mut(&mut self, id: &EntryId) -> Option<&mut ClipboardEntry> {
        self.entries.iter_mut().find(|e| e.id == *id)
    }

    pub fn latest(&self) -> Option<&ClipboardEntry> {
        self.entries.front()
    }

    /// Borrow every entry, newest first. Allocates a `Vec<&_>` only at
    /// the call site if you collect.
    pub fn all(&self) -> Vec<&ClipboardEntry> {
        self.entries.iter().collect()
    }

    /// Return entries strictly newer than `timestamp`, newest first.
    pub fn since(&self, timestamp: DateTime<Utc>) -> Vec<&ClipboardEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp > timestamp)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove every entry from the buffer and return them.
    pub fn clear(&mut self) -> Vec<ClipboardEntry> {
        std::mem::take(&mut self.entries).into_iter().collect()
    }

    fn evict(&mut self) -> Vec<ClipboardEntry> {
        let mut evicted = Vec::new();
        let now = Utc::now();
        let ttl_delta = TimeDelta::from_std(self.ttl).unwrap_or(TimeDelta::seconds(3600));
        let cutoff = now - ttl_delta;
        let min_entries = self.min_entries;

        if self.entries.len() > min_entries {
            // Drop expired entries from the back, but keep at least
            // `min_entries` regardless of age.
            let mut i = min_entries;
            while i < self.entries.len() {
                if self.entries[i].timestamp <= cutoff {
                    if let Some(entry) = self.entries.remove(i) {
                        evicted.push(entry);
                    }
                } else {
                    i += 1;
                }
            }
        }

        // Hard cap: pop from the back (oldest first).
        while self.entries.len() > self.max_entries {
            if let Some(entry) = self.entries.pop_back() {
                evicted.push(entry);
            }
        }

        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ImageSnapshot;
    use std::path::PathBuf;

    #[test]
    fn test_insert_and_latest() {
        let mut history = History::new();
        let (entry, _) = history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        assert!(entry.is_some());
        assert_eq!(history.latest().unwrap().find_text().unwrap(), "hello");
    }

    #[test]
    fn test_deduplication() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        let (second, _) = history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        assert!(second.is_none());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_different_content_not_deduped() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        history.insert(vec![ClipboardFormat::Text("world".to_string())]);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_get_by_id() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        let id = history.latest().unwrap().id.clone();
        assert!(history.get(&id).is_some());
        let missing: EntryId = "0000000000000000".parse().unwrap();
        assert!(history.get(&missing).is_none());
    }

    #[test]
    fn test_clear() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_all_returns_ordered() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        history.insert(vec![ClipboardFormat::Text("second".to_string())]);
        let all = history.all();
        assert_eq!(all[0].find_text().unwrap(), "second");
        assert_eq!(all[1].find_text().unwrap(), "first");
    }

    #[test]
    fn test_min_entries_floor() {
        let mut history = History::new()
            .with_min_entries(2)
            .with_ttl(Duration::from_secs(0));

        history.insert(vec![ClipboardFormat::Text("old1".to_string())]);
        history.insert(vec![ClipboardFormat::Text("old2".to_string())]);
        history.insert(vec![ClipboardFormat::Text("new".to_string())]);

        assert!(history.len() >= 2);
    }

    #[test]
    fn test_max_entries_cap() {
        let mut history = History::new().with_max_entries(3);
        for i in 0..5 {
            history.insert(vec![ClipboardFormat::Text(format!("entry-{i}"))]);
        }
        assert_eq!(history.len(), 3);
        assert_eq!(history.latest().unwrap().find_text().unwrap(), "entry-4");
    }

    #[test]
    fn test_max_entries_drops_oldest_from_back() {
        // VecDeque invariant: when the cap is exceeded, the oldest
        // entry (at the back) is dropped, the newest stays at the
        // front. Code Quality #10/#16.
        let mut history = History::new().with_max_entries(2);
        history.insert(vec![ClipboardFormat::Text("oldest".to_string())]);
        history.insert(vec![ClipboardFormat::Text("middle".to_string())]);
        history.insert(vec![ClipboardFormat::Text("newest".to_string())]);

        assert_eq!(history.len(), 2);
        let all = history.all();
        assert_eq!(all[0].find_text().unwrap(), "newest");
        assert_eq!(all[1].find_text().unwrap(), "middle");
        // The oldest entry's content is gone entirely.
        assert!(
            !all.iter().any(|e| e.find_text() == Some("oldest")),
            "oldest must have been evicted from the back"
        );
    }

    #[test]
    fn test_since() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("before".to_string())]);
        let cutoff = history.latest().unwrap().timestamp;
        // Sleep just enough that the next entry's timestamp is
        // strictly newer than `cutoff` even on fast clocks.
        std::thread::sleep(Duration::from_millis(2));
        history.insert(vec![ClipboardFormat::Text("after".to_string())]);
        let since = history.since(cutoff);
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].find_text().unwrap(), "after");
    }

    #[test]
    fn test_insert_with_multiple_formats() {
        let mut history = History::new();
        history.insert(vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Html("<b>hello</b>".to_string()),
        ]);
        let entry = history.latest().unwrap();
        assert_eq!(entry.formats.len(), 2);
    }

    #[test]
    fn test_insert_image() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Image(ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 20,
        })]);
        assert_eq!(history.len(), 1);
        assert!(history.latest().unwrap().find_image().is_some());
    }

    #[test]
    fn test_insert_files() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Files(vec![
            PathBuf::from("/tmp/a.txt"),
            PathBuf::from("/tmp/b.txt"),
        ])]);
        assert_eq!(history.len(), 1);
        let entry = history.latest().unwrap();
        assert_eq!(entry.formats.len(), 1);
    }

    #[test]
    fn test_dedup_probe_agrees_with_entry_id_for_spilled_image() {
        // Regression test for review-1 #12: the dedup probe and entry
        // id derivation must compute the same hash for a spilled image
        // so re-inserting the same payload deduplicates correctly.
        let bytes = vec![9u8; 128];
        let payload_hash = biscuit_hash::xx_hash_bytes(&bytes);
        let spill_path = PathBuf::from(format!("/tmp/cache/{payload_hash:016x}.dat"));
        let formats = vec![ClipboardFormat::Image(ImageSnapshot::Spilled {
            path: spill_path,
            width: 16,
            height: 16,
            size_bytes: bytes.len() as u64,
        })];

        let mut history = History::new();
        history.insert(formats.clone());
        let id = history.latest().unwrap().id.clone();
        let dedup_hash = entry::content_hash_of(&formats);
        assert_eq!(EntryId::from(dedup_hash), id);

        // Re-inserting the same spilled image should be deduped.
        let (second, _) = history.insert(formats);
        assert!(second.is_none());
    }

    #[test]
    fn test_reinsert_moves_to_front() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        history.insert(vec![ClipboardFormat::Text("second".to_string())]);
        history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        assert_eq!(history.len(), 2);
        assert_eq!(history.latest().unwrap().find_text().unwrap(), "first");
    }

    #[test]
    fn test_insert_returns_evicted_entries() {
        let mut history = History::new().with_max_entries(2);
        let (_, evicted1) = history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        assert!(evicted1.is_empty());

        let (_, evicted2) = history.insert(vec![ClipboardFormat::Text("second".to_string())]);
        assert!(evicted2.is_empty());

        let (_, evicted3) = history.insert(vec![ClipboardFormat::Text("third".to_string())]);
        assert_eq!(evicted3.len(), 1);
        assert_eq!(evicted3[0].find_text().unwrap(), "first");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_clear_returns_all_entries() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("a".to_string())]);
        history.insert(vec![ClipboardFormat::Text("b".to_string())]);
        history.insert(vec![ClipboardFormat::Text("c".to_string())]);

        let removed = history.clear();
        assert_eq!(removed.len(), 3);
        assert!(history.is_empty());
    }
}
