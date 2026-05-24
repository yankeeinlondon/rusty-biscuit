//! Trait abstraction over the host clipboard.
//!
//! The [`ClipboardBackend`] trait isolates the rest of the crate from
//! `clipboard-rs`. The production impl ([`SystemClipboard`]) wraps a
//! real `ClipboardContext`; the test impl ([`MockClipboard`]) returns
//! whatever the test set up.
//!
//! Only [`SystemClipboard`] holds OS-level state — no other code in
//! this crate touches `clipboard_rs::ClipboardContext` directly.

use std::path::PathBuf;

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, RustImageData};

use crate::content::ImageSnapshot;
use crate::error::ClipboardError;

/// Read/write access to a clipboard.
///
/// All methods return `Result<Option<_>, ClipboardError>` for read
/// operations: `Ok(None)` means "no payload of that format is
/// currently on the clipboard", `Err(_)` means the read itself
/// failed.
///
/// ## Errors
///
/// Implementations should return [`ClipboardError::Backend`] for
/// platform-level failures, and [`ClipboardError::Io`] for I/O.
pub trait ClipboardBackend {
    fn get_text(&self) -> Result<Option<String>, ClipboardError>;
    fn get_html(&self) -> Result<Option<String>, ClipboardError>;
    fn get_rtf(&self) -> Result<Option<String>, ClipboardError>;
    fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError>;
    fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError>;
    fn set_text(&self, text: &str) -> Result<(), ClipboardError>;
    fn set_html(&self, html: &str) -> Result<(), ClipboardError>;
    fn set_rtf(&self, rtf: &str) -> Result<(), ClipboardError>;
    fn set_image(&self, data: &[u8], width: u32, height: u32) -> Result<(), ClipboardError>;
    fn set_files(&self, files: &[PathBuf]) -> Result<(), ClipboardError>;
    /// Whether the OS marks the current clipboard contents as concealed
    /// (e.g. a password manager pasteboard with
    /// `org.nspasteboard.ConcealedType` on macOS).
    fn is_concealed(&self) -> Result<bool, ClipboardError>;
}

/// Production [`ClipboardBackend`] backed by `clipboard_rs`.
///
/// Construct via [`SystemClipboard::new`]. The constructor wraps the
/// real OS clipboard; subsequent methods are blocking and must not be
/// called from the Tokio runtime hot path (use
/// `tokio::task::spawn_blocking`).
pub struct SystemClipboard {
    ctx: clipboard_rs::ClipboardContext,
}

impl SystemClipboard {
    /// Initialize the OS clipboard backend.
    ///
    /// ## Errors
    ///
    /// Returns [`ClipboardError::Backend`] if `clipboard_rs` cannot
    /// open a context (e.g. no running window server on Linux).
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

    fn set_html(&self, html: &str) -> Result<(), ClipboardError> {
        self.ctx
            .set_html(html.to_string())
            .map_err(|e| ClipboardError::Backend(e.to_string()))
    }

    fn set_rtf(&self, rtf: &str) -> Result<(), ClipboardError> {
        self.ctx
            .set_rich_text(rtf.to_string())
            .map_err(|e| ClipboardError::Backend(e.to_string()))
    }

    fn set_image(&self, data: &[u8], _width: u32, _height: u32) -> Result<(), ClipboardError> {
        let image =
            RustImageData::from_bytes(data).map_err(|e| ClipboardError::Backend(e.to_string()))?;
        self.ctx
            .set_image(image)
            .map_err(|e| ClipboardError::Backend(e.to_string()))
    }

    fn set_files(&self, files: &[PathBuf]) -> Result<(), ClipboardError> {
        let files = files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        self.ctx
            .set_files(files)
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

/// Test-only [`ClipboardBackend`] that returns whatever was set on
/// the struct fields.
///
/// ## Examples
///
/// ```
/// use biscuit_clipboard::backend::{ClipboardBackend, MockClipboard};
///
/// let mock = MockClipboard {
///     text: Some("hi".into()),
///     ..Default::default()
/// };
/// assert_eq!(mock.get_text().unwrap(), Some("hi".into()));
/// mock.set_text("changed").unwrap();
/// assert_eq!(mock.set_text_log(), vec!["changed".to_string()]);
/// assert_eq!(mock.get_text().unwrap(), Some("changed".to_string()));
/// ```
#[derive(Default)]
pub struct MockClipboard {
    pub text: Option<String>,
    pub html: Option<String>,
    pub rtf: Option<String>,
    pub image: Option<ImageSnapshot>,
    pub files: Option<Vec<PathBuf>>,
    pub concealed: bool,
    /// Captures every `set_text` call so tests can assert backend
    /// invocations (e.g. `POST /clear` calls `set_text("")`).
    pub set_text_calls: std::sync::Mutex<Vec<String>>,
    pub set_html_calls: std::sync::Mutex<Vec<String>>,
    pub set_rtf_calls: std::sync::Mutex<Vec<String>>,
    pub set_image_calls: std::sync::Mutex<Vec<(Vec<u8>, u32, u32)>>,
    pub set_files_calls: std::sync::Mutex<Vec<Vec<PathBuf>>>,
}

impl MockClipboard {
    /// Snapshot of every `set_text` argument observed so far.
    pub fn set_text_log(&self) -> Vec<String> {
        self.set_text_calls.lock().unwrap().clone()
    }

    pub fn set_html_log(&self) -> Vec<String> {
        self.set_html_calls.lock().unwrap().clone()
    }

    pub fn set_rtf_log(&self) -> Vec<String> {
        self.set_rtf_calls.lock().unwrap().clone()
    }

    pub fn set_image_log(&self) -> Vec<(Vec<u8>, u32, u32)> {
        self.set_image_calls.lock().unwrap().clone()
    }

    pub fn set_files_log(&self) -> Vec<Vec<PathBuf>> {
        self.set_files_calls.lock().unwrap().clone()
    }
}

impl ClipboardBackend for MockClipboard {
    fn get_text(&self) -> Result<Option<String>, ClipboardError> {
        if let Some(text) = self.set_text_calls.lock().unwrap().last() {
            return Ok(Some(text.clone()));
        }
        Ok(self.text.clone())
    }

    fn get_html(&self) -> Result<Option<String>, ClipboardError> {
        if let Some(html) = self.set_html_calls.lock().unwrap().last() {
            return Ok(Some(html.clone()));
        }
        Ok(self.html.clone())
    }

    fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
        if let Some(rtf) = self.set_rtf_calls.lock().unwrap().last() {
            return Ok(Some(rtf.clone()));
        }
        Ok(self.rtf.clone())
    }

    fn get_image(&self) -> Result<Option<ImageSnapshot>, ClipboardError> {
        if let Some((data, width, height)) = self.set_image_calls.lock().unwrap().last() {
            return Ok(Some(ImageSnapshot::Inline {
                data: data.clone(),
                width: *width,
                height: *height,
            }));
        }
        Ok(self.image.clone())
    }

    fn get_files(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        if let Some(files) = self.set_files_calls.lock().unwrap().last() {
            return Ok(Some(files.clone()));
        }
        Ok(self.files.clone())
    }

    fn set_text(&self, text: &str) -> Result<(), ClipboardError> {
        self.set_text_calls.lock().unwrap().push(text.to_string());
        Ok(())
    }

    fn set_html(&self, html: &str) -> Result<(), ClipboardError> {
        self.set_html_calls.lock().unwrap().push(html.to_string());
        Ok(())
    }

    fn set_rtf(&self, rtf: &str) -> Result<(), ClipboardError> {
        self.set_rtf_calls.lock().unwrap().push(rtf.to_string());
        Ok(())
    }

    fn set_image(&self, data: &[u8], width: u32, height: u32) -> Result<(), ClipboardError> {
        self.set_image_calls
            .lock()
            .unwrap()
            .push((data.to_vec(), width, height));
        Ok(())
    }

    fn set_files(&self, files: &[PathBuf]) -> Result<(), ClipboardError> {
        self.set_files_calls.lock().unwrap().push(files.to_vec());
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
        assert_eq!(mock.set_text_log(), vec!["test".to_string()]);
        assert_eq!(mock.get_text().unwrap(), Some("test".to_string()));
    }
}
