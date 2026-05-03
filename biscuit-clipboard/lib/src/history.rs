use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use crate::content::ClipboardFormat;
use crate::entry::{ClipboardEntry, EntryId};

const DEFAULT_TTL: Duration = Duration::from_secs(3600);
const DEFAULT_MIN_ENTRIES: usize = 2;
const DEFAULT_MAX_ENTRIES: usize = 100;

pub struct History {
    entries: Vec<ClipboardEntry>,
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
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
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

    pub fn insert(&mut self, formats: Vec<ClipboardFormat>) -> Option<&ClipboardEntry> {
        let new_hash = compute_content_hash(&formats);

        if self.entries.first().is_some_and(|e| e.content_hash == new_hash) {
            return None;
        }

        let entry = ClipboardEntry::new(formats);

        self.entries.retain(|e| e.id != entry.id);

        self.entries.insert(0, entry);
        self.evict();
        self.entries.first()
    }

    pub fn get(&self, id: &EntryId) -> Option<&ClipboardEntry> {
        self.entries.iter().find(|e| e.id == *id)
    }

    pub fn get_mut(&mut self, id: &EntryId) -> Option<&mut ClipboardEntry> {
        self.entries.iter_mut().find(|e| e.id == *id)
    }

    pub fn latest(&self) -> Option<&ClipboardEntry> {
        self.entries.first()
    }

    pub fn all(&self) -> &[ClipboardEntry] {
        &self.entries
    }

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

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn evict(&mut self) {
        let now = Utc::now();
        let ttl_delta = TimeDelta::from_std(self.ttl).unwrap_or(TimeDelta::seconds(3600));
        let cutoff = now - ttl_delta;
        let min_entries = self.min_entries;

        if self.entries.len() > min_entries {
            let mut i = min_entries;
            while i < self.entries.len() {
                if self.entries[i].timestamp <= cutoff {
                    self.entries.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        while self.entries.len() > self.max_entries {
            self.entries.pop();
        }
    }
}

fn compute_content_hash(formats: &[ClipboardFormat]) -> u64 {
    use std::io::Write;

    let mut hasher = Vec::new();
    for fmt in formats {
        match fmt {
            ClipboardFormat::Text(s) => {
                let _ = hasher.write_all(b"text:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Html(s) => {
                let _ = hasher.write_all(b"html:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Rtf(s) => {
                let _ = hasher.write_all(b"rtf:");
                let _ = hasher.write_all(s.as_bytes());
            }
            ClipboardFormat::Image(img) => {
                let _ = hasher.write_all(b"image:");
                let _ = hasher.write_all(img.data());
            }
            ClipboardFormat::Files(paths) => {
                let _ = hasher.write_all(b"files:");
                for p in paths {
                    let _ = hasher.write_all(p.to_string_lossy().as_bytes());
                    let _ = hasher.write_all(b"\0");
                }
            }
        }
    }
    biscuit_hash::xx_hash_bytes(&hasher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ImageSnapshot;
    use std::path::PathBuf;

    #[test]
    fn test_insert_and_latest() {
        let mut history = History::new();
        let entry = history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        assert!(entry.is_some());
        assert_eq!(history.latest().unwrap().find_text().unwrap(), "hello");
    }

    #[test]
    fn test_deduplication() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
        let second = history.insert(vec![ClipboardFormat::Text("hello".to_string())]);
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
        assert!(history.get(&"nonexistent".to_string()).is_none());
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
        let mut history = History::new().with_min_entries(2).with_ttl(Duration::from_secs(0));

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
    fn test_since() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("before".to_string())]);
        let cutoff = history.latest().unwrap().timestamp;
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
    fn test_reinsert_moves_to_front() {
        let mut history = History::new();
        history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        history.insert(vec![ClipboardFormat::Text("second".to_string())]);
        history.insert(vec![ClipboardFormat::Text("first".to_string())]);
        assert_eq!(history.len(), 2);
        assert_eq!(history.latest().unwrap().find_text().unwrap(), "first");
    }
}
