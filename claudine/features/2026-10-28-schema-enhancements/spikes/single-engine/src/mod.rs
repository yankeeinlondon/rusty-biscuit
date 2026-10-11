//! THROWAWAY SPIKE — single coercion engine (feature 2026-09-21-schema-enhancements).
//!
//! `core` is the one rule list; `from_json_schema` and `from_grammar` are the
//! two translators; `tests` runs today's coerce.rs / L1 cases through both
//! and writes the classified diff. Never merged.

pub mod core;
pub mod from_grammar;
pub mod from_json_schema;
pub mod numberlike_format;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use jsonschema::Validator;
use serde_json::json;

use crate::markdown::schemas::validate::build_validator;

/// Format checks delegated to the existing format registry, one cached
/// single-keyword validator per format name (not per union arm).
pub struct FormatChecks {
    base_dir: Option<PathBuf>,
    cache: RefCell<HashMap<String, Option<Arc<Validator>>>>,
}

impl FormatChecks {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        Self {
            base_dir,
            cache: RefCell::new(HashMap::new()),
        }
    }
}

impl core::Checks for FormatChecks {
    fn format(&self, format: &str, text: &str) -> bool {
        let mut cache = self.cache.borrow_mut();
        let v = cache.entry(format.to_string()).or_insert_with(|| {
            build_validator(&json!({ "format": format }), self.base_dir.as_deref(), None)
                .ok()
                .map(Arc::new)
        });
        v.as_ref()
            .is_none_or(|val| val.is_valid(&serde_json::Value::String(text.to_string())))
    }
}
