use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Text,
    Html,
    Rtf,
    Image,
    Files,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClipboardFormat {
    #[serde(rename = "text")]
    Text(String),

    #[serde(rename = "html")]
    Html(String),

    #[serde(rename = "rtf")]
    Rtf(String),

    #[serde(rename = "image")]
    Image(ImageSnapshot),

    #[serde(rename = "files")]
    Files(Vec<PathBuf>),
}

impl ClipboardFormat {
    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Text(_) => ContentType::Text,
            Self::Html(_) => ContentType::Html,
            Self::Rtf(_) => ContentType::Rtf,
            Self::Image(_) => ContentType::Image,
            Self::Files(_) => ContentType::Files,
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Html(s) => s.len(),
            Self::Rtf(s) => s.len(),
            Self::Image(img) => img.size_bytes(),
            Self::Files(paths) => paths.iter().map(|p| p.as_os_str().len()).sum(),
        }
    }

    pub fn preview(&self) -> String {
        match self {
            Self::Text(s) | Self::Html(s) | Self::Rtf(s) => {
                if s.len() > 80 {
                    format!("{}...", &s[..s.ceil_char_boundary(80)])
                } else {
                    s.clone()
                }
            }
            Self::Image(img) => format!("Image {}x{}", img.width(), img.height()),
            Self::Files(paths) => {
                if paths.len() == 1 {
                    paths[0]
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "1 file".to_string())
                } else {
                    format!("{} files", paths.len())
                }
            }
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_html(&self) -> Option<&str> {
        match self {
            Self::Html(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_image(&self) -> Option<&ImageSnapshot> {
        match self {
            Self::Image(img) => Some(img),
            _ => None,
        }
    }

    pub fn as_files(&self) -> Option<&Vec<PathBuf>> {
        match self {
            Self::Files(paths) => Some(paths),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImageSnapshot {
    Inline {
        data: Vec<u8>,
        width: u32,
        height: u32,
    },
    Spilled {
        path: PathBuf,
        width: u32,
        height: u32,
        size_bytes: u64,
    },
}

impl ImageSnapshot {
    pub fn width(&self) -> u32 {
        match self {
            Self::Inline { width, .. } => *width,
            Self::Spilled { width, .. } => *width,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Inline { height, .. } => *height,
            Self::Spilled { height, .. } => *height,
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Inline { data, .. } => data.len(),
            Self::Spilled { size_bytes, .. } => *size_bytes as usize,
        }
    }

    pub fn data(&self) -> &[u8] {
        match self {
            Self::Inline { data, .. } => data,
            Self::Spilled { .. } => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_serialization() {
        let ct = ContentType::Text;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"text\"");

        let ct = ContentType::Image;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"image\"");
    }

    #[test]
    fn test_clipboard_format_text() {
        let fmt = ClipboardFormat::Text("hello".to_string());
        assert_eq!(fmt.content_type(), ContentType::Text);
        assert_eq!(fmt.size_bytes(), 5);
        assert_eq!(fmt.preview(), "hello");
    }

    #[test]
    fn test_clipboard_format_text_preview_truncation() {
        let long_text = "a".repeat(100);
        let fmt = ClipboardFormat::Text(long_text);
        let preview = fmt.preview();
        assert!(preview.ends_with("..."));
        assert_eq!(preview.len(), 83);
    }

    #[test]
    fn test_clipboard_format_image_preview() {
        let img = ImageSnapshot::Inline {
            data: vec![0u8; 100],
            width: 1920,
            height: 1080,
        };
        let fmt = ClipboardFormat::Image(img);
        assert_eq!(fmt.preview(), "Image 1920x1080");
    }

    #[test]
    fn test_clipboard_format_files_preview_single() {
        let fmt = ClipboardFormat::Files(vec![PathBuf::from("/tmp/test.txt")]);
        assert_eq!(fmt.preview(), "test.txt");
    }

    #[test]
    fn test_clipboard_format_files_preview_multiple() {
        let fmt = ClipboardFormat::Files(vec![
            PathBuf::from("/tmp/a.txt"),
            PathBuf::from("/tmp/b.txt"),
            PathBuf::from("/tmp/c.txt"),
        ]);
        assert_eq!(fmt.preview(), "3 files");
    }

    #[test]
    fn test_image_snapshot_inline() {
        let img = ImageSnapshot::Inline {
            data: vec![1, 2, 3],
            width: 10,
            height: 20,
        };
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 20);
        assert_eq!(img.size_bytes(), 3);
    }

    #[test]
    fn test_image_snapshot_spilled() {
        let img = ImageSnapshot::Spilled {
            path: PathBuf::from("/tmp/test.dat"),
            width: 100,
            height: 200,
            size_bytes: 1024,
        };
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 200);
        assert_eq!(img.size_bytes(), 1024);
    }

    #[test]
    fn test_clipboard_format_serialization_roundtrip() {
        let fmt = ClipboardFormat::Text("hello world".to_string());
        let json = serde_json::to_string(&fmt).unwrap();
        let deserialized: ClipboardFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, deserialized);
    }
}
