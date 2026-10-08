//! Diagnostic prefixes are bounded independently of the parser's answer.
//! The raw reader observes bytes before BufReader decodes or strips delimiters.

use std::io::{self, Read};
use base64::Engine;
use std::path::Path;
use serde::Serialize;
use super::{RunScope, observation::Operation};

pub(crate) const INLINE_LIMIT: usize = 256 * 1024;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub(crate) struct RetainedData {
    pub(crate) response_text: Option<String>,
    pub(crate) response_complete: Option<bool>,
    pub(crate) raw_output: Option<serde_json::Value>,
    pub(crate) raw_output_complete: Option<bool>,
    pub(crate) raw_output_truncated: bool,
    pub(crate) raw_output_path: Option<String>,
}

pub(super) struct Retention {
    pub(super) closed: bool,
    answer: String,
    identified: bool,
    answer_clipped: bool,
    complete: Option<bool>,
    raw: Vec<u8>,
    raw_seen: bool,
    raw_clipped: bool,
    eof: bool,
    pub(super) capture: Option<CaptureFacts>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CaptureFacts {
    pub(crate) path: String,
    pub(crate) omitted: bool,
    pub(crate) failed: bool,
    pub(crate) flushed: bool,
    pub(crate) eof: bool,
}

impl Default for Retention {
    fn default() -> Self {
        Self { closed: false, answer: String::with_capacity(INLINE_LIMIT), identified: false, answer_clipped: false, complete: None,
            raw: Vec::with_capacity(INLINE_LIMIT), raw_seen: false, raw_clipped: false,
            eof: false, capture: None }
    }
}

impl Retention {
    pub(super) fn has_answer(&self) -> bool { self.identified }
    pub(super) fn answer(&mut self, text: &str, complete: bool, replace: bool) {
        if replace {
            self.answer.clear();
            self.answer_clipped = false;
        }
        self.identified = true;
        if self.answer_clipped { self.complete = Some(false); return; }
        let mut take = text.len().min(INLINE_LIMIT - self.answer.len());
        while !text.is_char_boundary(take) { take -= 1; }
        self.answer.push_str(&text[..take]);
        self.answer_clipped |= take != text.len();
        self.complete = Some(complete && !self.answer_clipped);
    }

    pub(super) fn raw(&mut self, bytes: &[u8]) {
        self.raw_seen = true;
        let take = bytes.len().min(INLINE_LIMIT - self.raw.len());
        self.raw.extend_from_slice(&bytes[..take]);
        self.raw_clipped |= take != bytes.len();
    }

    pub(super) fn eof(&mut self) { self.raw_seen = true; self.eof = true; }

    pub(super) fn closed() -> Self {
        Self { closed: true, answer: String::new(), identified: false, answer_clipped: false, complete: None,
            raw: Vec::new(), raw_seen: false, raw_clipped: false, eof: false, capture: None }
    }

    pub(super) fn freeze(mut self) -> RetainedData {
        self.closed = true;
        let raw = std::mem::take(&mut self.raw);
        let complete_artifact = self.capture.as_ref().is_some_and(|facts|
            facts.eof && facts.flushed && !facts.omitted && !facts.failed);
        RetainedData {
            response_text: self.identified.then(|| std::mem::take(&mut self.answer)), response_complete: self.complete,
            raw_output: self.raw_seen.then(|| match String::from_utf8(raw) {
                Ok(text) => serde_json::Value::String(text),
                Err(error) => serde_json::json!({ "encoding": "base64", "data": base64::engine::general_purpose::STANDARD.encode(error.as_bytes()) }),
            }),
            raw_output_complete: self.raw_seen.then_some((self.eof && !self.raw_clipped) || complete_artifact),
            raw_output_truncated: self.raw_clipped,
            raw_output_path: self.capture.take().map(|facts| facts.path),
        }
    }
}

pub(crate) struct RetainingReader<R> {
    reader: R,
    scope: RunScope,
    observe: bool,
}

impl<R> RetainingReader<R> {
    pub(crate) fn new(reader: R, scope: RunScope) -> Self { Self { reader, scope, observe: true } }
    pub(crate) fn capture_only(reader: R, scope: RunScope) -> Self { Self { reader, scope, observe: false } }
}

impl<R: Read> Read for RetainingReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.observe { self.scope.observe_stdout(Operation::PipeWait); }
        let count = self.reader.read(bytes)?;
        if count == 0 && !bytes.is_empty() {
            self.scope.raw_eof();
            if self.observe { self.scope.observe_stdout(Operation::Eof); }
        } else if count > 0 {
            self.scope.retain_raw(&bytes[..count]);
            if self.observe { self.scope.observe_stdout(Operation::BytesArrived); }
        }
        Ok(count)
    }
}

/// A last-message file identifies text, never a provider verdict. One extra
/// byte distinguishes a complete file from a clipped prefix.
pub(crate) fn last_message(path: &Path) -> io::Result<(String, bool)> {
    let mut bytes = Vec::with_capacity(INLINE_LIMIT + 1);
    std::fs::File::open(path)?.take((INLINE_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    let complete = bytes.len() <= INLINE_LIMIT;
    bytes.truncate(INLINE_LIMIT);
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) if !complete && error.utf8_error().error_len().is_none() => {
            let end = error.utf8_error().valid_up_to();
            String::from_utf8(error.into_bytes()[..end].to_vec()).expect("valid prefix")
        }
        Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidData, error)),
    };
    Ok((text, complete))
}
