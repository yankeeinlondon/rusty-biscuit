use std::path::PathBuf;

use clipboard_rs::common::RustImage;
use clipboard_rs::Clipboard;

use crate::content::ImageSnapshot;
use crate::error::ClipboardError;

pub trait ClipboardBackend {
    fn get_text(&self) -> Result<Option<String>, ClipboardError>;
    fn get_html(&self) -> Result<Option<String>, ClipboardError>;
    fn get_rtf(&self) -> Result<Option<String>, ClipboardError>;
    fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError>;
    fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError>;
    fn set_text(&self, text: &str) -> Result<(), ClipboardError>;
    fn is_concealed(&self) -> Result<bool, ClipboardError>;
}

pub struct SystemClipboard {
    ctx: clipboard_rs::ClipboardContext,
}

impl SystemClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        let ctx = clipboard_rs::ClipboardContext::new()
            .map_err(|e| ClipboardError::Backend(e.to_string()))?;
        Ok(Self { ctx })
    }
}

impl ClipboardBackend for SystemClipboard {
    fn get_text(&self) -> Result<Option<String>, ClipboardError> {
        if self.ctx.has(clipboard_rs::ContentFormat::Text) {
            self.ctx
                .get_text()
                .map(Some)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_html(&self) -> Result<Option<String>, ClipboardError> {
        if self.ctx.has(clipboard_rs::ContentFormat::Html) {
            self.ctx
                .get_html()
                .map(Some)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
        if self.ctx.has(clipboard_rs::ContentFormat::Rtf) {
            self.ctx
                .get_rich_text()
                .map(Some)
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError> {
        if self.ctx.has(clipboard_rs::ContentFormat::Image) {
            match self.ctx.get_image() {
                Ok(img) => {
                    let (width, height) = img.get_size();
                    let data = img
                        .to_png()
                        .map_err(|e| ClipboardError::Backend(e.to_string()))?
                        .get_bytes()
                        .to_vec();
                    Ok(Some(ImageSnapshot::Inline {
                        data,
                        width,
                        height,
                    }))
                }
                Err(e) => Err(ClipboardError::Backend(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        if self.ctx.has(clipboard_rs::ContentFormat::Files) {
            self.ctx
                .get_files()
                .map(|files| Some(files.into_iter().map(PathBuf::from).collect()))
                .map_err(|e| ClipboardError::Backend(e.to_string()))
        } else {
            Ok(None)
        }
    }

    fn set_text(&self, text: &str) -> Result<(), ClipboardError> {
        self.ctx
            .set_text(text.to_string())
            .map_err(|e| ClipboardError::Backend(e.to_string()))
    }

    fn is_concealed(&self) -> Result<bool, ClipboardError> {
        #[cfg(target_os = "macos")]
        {
            let result = self
                .ctx
                .get_buffer("org.nspasteboard.ConcealedType")
                .is_ok();
            Ok(result)
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(false)
        }
    }
}

#[derive(Default)]
pub struct MockClipboard {
    pub text: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub image: Option<ImageSnapshot>,
    pub files: Option<Vec<PathBuf>>,
    pub concealed: bool,
}

impl ClipboardBackend for MockClipboard {
    fn get_text(&self) -> Result<Option<String>, ClipboardError> {
        Ok(self.text.clone())
    }

    fn get_html(&self) -> Result<Option<String>, ClipboardError> {
        Ok(self.html.clone())
    }

    fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
        Ok(self.rtf.clone())
    }

    fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError> {
        Ok(self.image.clone())
    }

    fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        Ok(self.files.clone())
    }

    fn set_text(&self, text: &str) -> Result<(), ClipboardError> {
        let _ = text;
        Ok(())
    }

    fn is_concealed(&self) -> Result<bool, ClipboardError> {
        Ok(self.concealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_clipboard_defaults() {
        let mock = MockClipboard::default();
        assert!(mock.get_text().unwrap().is_none());
        assert!(mock.get_html().unwrap().is_none());
        assert!(!mock.is_concealed().unwrap());
    }

    #[test]
    fn test_mock_clipboard_with_text() {
        let mock = MockClipboard {
            text: Some("hello".to_string()),
            ..Default::default()
        };
        assert_eq!(mock.get_text().unwrap(), Some("hello".to_string()));
    }

    #[test]
    fn test_mock_clipboard_concealed() {
        let mock = MockClipboard {
            concealed: true,
            ..Default::default()
        };
        assert!(mock.is_concealed().unwrap());
    }

    #[test]
    fn test_mock_clipboard_set_text() {
        let mock = MockClipboard::default();
        mock.set_text("test").unwrap();
    }
}
