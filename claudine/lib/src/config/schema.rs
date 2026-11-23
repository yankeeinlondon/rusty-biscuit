//! The shipped configuration schema: the one authority for the shape and
//! defaults of `~/.claudine/config.yaml` and `{repo-root}/.claudine/config.yaml`.
//!
//! The text is `claudine/registration/claudine-config.yaml`, compiled in with
//! `include_str!`. Its `permissions` property imports Darkmatter's type
//! (`permissions@darkmatter:config`), so the contracts are prepared with
//! Darkmatter's shipped sources. [`super::load`] validates a file against
//! [`ConfigScope::config_type`] and nothing else.
//!
//! User-facing guide: `claudine/docs/topics/configuration.md`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use simplified_schema::{Prepared, SchemaError};

/// The virtual path the schema is prepared under, and the name its problems
/// cite.
pub const CONFIG_SCHEMA_PATH: &str = "claudine/registration/claudine-config.yaml";

/// The configuration schema's text, as shipped.
pub const CONFIG_SCHEMA_SOURCE: &str = include_str!("../../../registration/claudine-config.yaml");

/// The directory both configuration files live in, under the home directory
/// or the repository root.
pub const CONFIG_DIR: &str = ".claudine";

/// The file name of both configuration files.
pub const CONFIG_FILE_NAME: &str = "config.yaml";

/// The two configuration scopes. A repository file accepts only the
/// settings a repository may override, plus `permissions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigScope {
    /// `~/.claudine/config.yaml`.
    User,
    /// `{repo-root}/.claudine/config.yaml`.
    Repository,
}

impl ConfigScope {
    /// The named type a whole configuration file of this scope satisfies.
    #[must_use]
    pub const fn config_type(self) -> &'static str {
        match self {
            Self::User => "user-config",
            Self::Repository => "repository-config",
        }
    }

    /// The configuration file of this scope under `base` (the home directory
    /// or the repository root).
    #[must_use]
    pub fn file_under(base: &Path) -> PathBuf {
        base.join(CONFIG_DIR).join(CONFIG_FILE_NAME)
    }
}

/// The prepared contract of a whole configuration file of `scope`.
///
/// ## Panics
///
/// When the shipped schema does not prepare, which
/// `config::schema::tests` rules out.
pub(crate) fn contract(scope: ConfigScope) -> &'static Prepared {
    static USER: OnceLock<Prepared> = OnceLock::new();
    static REPOSITORY: OnceLock<Prepared> = OnceLock::new();
    let cell = match scope {
        ConfigScope::User => &USER,
        ConfigScope::Repository => &REPOSITORY,
    };
    cell.get_or_init(|| {
        prepare(scope.config_type())
            .unwrap_or_else(|error| panic!("the shipped `{}` type prepares: {error}", scope.config_type()))
    })
}

/// Prepares the object type `name` of the shipped schema as a root, with
/// Darkmatter resolving its `darkmatter:` imports.
fn prepare(name: &str) -> Result<Prepared, SchemaError> {
    darkmatter::config::schema::prepare_host_config_type(CONFIG_SCHEMA_SOURCE, CONFIG_SCHEMA_PATH, name)
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
