//! The environment one document lifecycle reads.
//!
//! Each document — the launch document, a sequence task, a group leg, a
//! `proxy` target — captures one map before its `initialize` and keeps it for
//! its whole lifecycle; transclusion reads its including document's map. The
//! library never reads the process environment for it: the CLI boundary
//! supplies the launch map and installs the [`EnvironmentSource`] later
//! documents capture from.

use std::collections::HashMap;
use std::sync::Arc;

/// One document's captured environment, immutable for its lifecycle.
///
/// Cloning shares the map; two documents hold equal values only when their
/// captures agreed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentEnvironment {
    vars: Arc<HashMap<String, String>>,
}

impl DocumentEnvironment {
    /// A document environment holding exactly `vars`.
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self { vars: Arc::new(vars) }
    }

    /// Every captured variable.
    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// One captured value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// [`Self::get`] in the owned shape the provider/model selectors take.
    pub fn lookup(&self, name: &str) -> Option<String> {
        self.get(name).map(str::to_string)
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for DocumentEnvironment {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::new(iter.into_iter().map(|(key, value)| (key.into(), value.into())).collect())
    }
}

/// Where a new document's environment is captured from.
///
/// The CLI installs one that reads its own process environment
/// (`biscuit_file::capture_env`); a test installs one that returns supplied
/// maps. Without an installed source, every later document re-enters the
/// launch map, so the library itself never reads the process.
#[derive(Clone)]
pub struct EnvironmentSource {
    capture: Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>,
}

impl EnvironmentSource {
    /// A source that calls `capture` once per document entry.
    pub fn new(capture: impl Fn() -> HashMap<String, String> + Send + Sync + 'static) -> Self {
        Self {
            capture: Arc::new(capture),
        }
    }

    /// A source that returns `environment` for every document.
    pub fn fixed(environment: DocumentEnvironment) -> Self {
        Self::new(move || environment.vars().clone())
    }

    /// Capture one document's environment.
    pub fn capture(&self) -> DocumentEnvironment {
        DocumentEnvironment::new((self.capture)())
    }
}

impl std::fmt::Debug for EnvironmentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentSource").finish_non_exhaustive()
    }
}
