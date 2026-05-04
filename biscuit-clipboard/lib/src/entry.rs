use biscuit_hash::xx_hash_bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::{ClipboardFormat, ContentType};

pub type EntryId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: EntryId,
    pub timestamp: DateTime<Utc>,
    pub content_hash: u64,
    pub formats: Vec<ClipboardFormat>,
}

impl ClipboardEntry {
    pub fn new(formats: Vec<ClipboardFormat>) -> Self {
        let content_hash = compute_hash(&formats);
        let id = format!("{:016x}", content_hash);
        let timestamp = Utc::now();

        Self {
            id,
            timestamp,
            content_hash,
            formats,
        }
    }

    pub fn primary_content_type(&self) -> ContentType {
        let priority = [ContentType::Text, ContentType::Html, ContentType::Rtf, ContentType::Image, ContentType::Files];

        let available: Vec<ContentType> = self.formats.iter().map(|f| f.content_type()).collect();

        for ct in &priority {
            if available.contains(ct) {
                return *ct;
            }
        }

        ContentType::Text
    }

    pub fn preview(&self) -> String {
        let priority = [ContentType::Text, ContentType::Html, ContentType::Rtf, ContentType::Image, ContentType::Files];

        for ct in &priority {
            if let Some(fmt) = self.formats.iter().find(|f| f.content_type() == *ct) {
                return fmt.preview();
            }
        }

        String::new()
    }

    pub fn total_size_bytes(&self) -> usize {
        self.formats.iter().map(|f| f.size_bytes()).sum()
    }

    pub fn find_format(&self, content_type: ContentType) -> Option<&ClipboardFormat> {
        self.formats.iter().find(|f| f.content_type() == content_type)
    }

    pub fn find_text(&self) -> Option<&str> {
        self.formats
            .iter()
            .find(|f| f.content_type() == ContentType::Text)
            .and_then(|f| f.as_text())
    }

    pub fn find_image(&self) -> Option<&crate::content::ImageSnapshot> {
        self.formats
            .iter()
            .find(|f| f.content_type() == ContentType::Image)
            .and_then(|f| f.as_image())
    }
}

fn compute_hash(formats: &[ClipboardFormat]) -> u64 {
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
                match img {
                    crate::content::ImageSnapshot::Inline { data, .. } => {
                        let _ = hasher.write_all(data);
                    }
                    crate::content::ImageSnapshot::Spilled { size_bytes, .. } => {
                        let _ = hasher.write_all(&size_bytes.to_le_bytes());
                    }
                }
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
    xx_hash_bytes(&hasher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ImageSnapshot;
    use std::path::PathBuf;

    #[test]
    fn test_entry_id_is_hex_string() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        assert_eq!(entry.id.len(), 16);
        assert!(entry.id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_entry_id_deterministic() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        assert_eq!(entry1.id, entry2.id);
    }

    #[test]
    fn test_entry_id_different_content() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Text("hello".to_string())]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Text("world".to_string())]);
        assert_ne!(entry1.id, entry2.id);
    }

    #[test]
    fn test_primary_content_type_text() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hi".to_string())]);
        assert_eq!(entry.primary_content_type(), ContentType::Text);
    }

    #[test]
    fn test_primary_content_type_priority() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Html("<b>hi</b>".to_string()),
            ClipboardFormat::Text("hi".to_string()),
        ]);
        assert_eq!(entry.primary_content_type(), ContentType::Text);
    }

    #[test]
    fn test_preview() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hello world".to_string())]);
        assert_eq!(entry.preview(), "hello world");
    }

    #[test]
    fn test_total_size_bytes() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Html("<b>hello</b>".to_string()),
        ]);
        assert_eq!(entry.total_size_bytes(), 5 + 12);
    }

    #[test]
    fn test_find_format() {
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("hi".to_string())]);
        assert!(entry.find_format(ContentType::Text).is_some());
        assert!(entry.find_format(ContentType::Html).is_none());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let entry = ClipboardEntry::new(vec![
            ClipboardFormat::Text("hello".to_string()),
            ClipboardFormat::Files(vec![PathBuf::from("/tmp/test.txt")]),
        ]);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ClipboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.content_hash, deserialized.content_hash);
    }

    #[test]
    fn test_image_entry_hash_uses_data() {
        let entry1 = ClipboardEntry::new(vec![ClipboardFormat::Image(ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        })]);
        let entry2 = ClipboardEntry::new(vec![ClipboardFormat::Image(ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 10,
        })]);
        assert_eq!(entry1.id, entry2.id);
    }

    #[test]
    fn test_timestamp_is_recent() {
        let before = Utc::now();
        let entry = ClipboardEntry::new(vec![ClipboardFormat::Text("test".to_string())]);
        let after = Utc::now();
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);
    }
}
