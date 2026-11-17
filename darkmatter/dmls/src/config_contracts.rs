//! The configuration files DMLS edits by their location, and the contract
//! each one is judged against.
//!
//! A configuration file has no mandatory `kind`, so kind selection cannot
//! find its schema. Four exact paths are configuration files instead, judged
//! by a contract chosen before any `kind` or `$schema` is consulted:
//!
//! | Path | Contract |
//! |---|---|
//! | `<home>/.darkmatter.yaml` | Darkmatter's `user-config` |
//! | `<repo-root>/.darkmatter.yaml` | Darkmatter's `repository-config` |
//! | `<home>/.claudine/config.yaml` | Claudine's `user-config` |
//! | `<repo-root>/.claudine/config.yaml` | Claudine's `repository-config` |
//!
//! The home and the repository root are the document's captured
//! [`FileResolutionContext`]'s, and paths compare by [`PathIdentity`]. A file
//! of the same name anywhere else is an ordinary YAML document.
//!
//! Claudine's schema is compiled in from its one authoritative text
//! ([`CLAUDINE_CONFIG_SCHEMA_SOURCE`]): DMLS does not depend on the Claudine
//! crate, and a brand-new Claudine configuration is editable before Claudine
//! is installed or registered. Its `permissions` import
//! (`permissions@darkmatter:config`) resolves among Darkmatter's shipped
//! documents. Each contract's schema is prepared once per process.
//!
//! User-facing contract: `darkmatter/docs/topics/configuration.md`.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use biscuit_file::{FileResolutionContext, PathIdentity};
use darkmatter::config::schema::{
    CONFIG_FILE_NAME, CONFIG_SCHEMA_PATH, CONFIG_SCHEMA_SOURCE, ConfigScope, resolve_host_config_type,
};
use simplified_schema::SchemaError;
use simplified_schema::resolve::ResolvedSchema;

/// The virtual path Claudine's configuration schema is cited under; the
/// same one Claudine prepares it under.
pub const CLAUDINE_CONFIG_SCHEMA_PATH: &str = "claudine/schemas/claudine-config.yaml";

/// Claudine's configuration schema, compiled in from Claudine's own source
/// file, so the two never differ.
pub const CLAUDINE_CONFIG_SCHEMA_SOURCE: &str = include_str!("../../../claudine/schemas/claudine-config.yaml");

/// The program a configuration file configures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigHost {
    /// `.darkmatter.yaml`.
    Darkmatter,
    /// `.claudine/config.yaml`.
    Claudine,
}

impl ConfigHost {
    /// The configuration file of this host under `base` (the home directory
    /// or the repository root).
    #[must_use]
    pub fn file_under(self, base: &Path) -> std::path::PathBuf {
        match self {
            Self::Darkmatter => base.join(CONFIG_FILE_NAME),
            Self::Claudine => base.join(".claudine").join("config.yaml"),
        }
    }

    fn schema(self) -> (&'static str, &'static str) {
        match self {
            Self::Darkmatter => (CONFIG_SCHEMA_SOURCE, CONFIG_SCHEMA_PATH),
            Self::Claudine => (CLAUDINE_CONFIG_SCHEMA_SOURCE, CLAUDINE_CONFIG_SCHEMA_PATH),
        }
    }
}

/// One configuration file: whose it is and at which scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfigurationFile {
    /// The program it configures.
    pub host: ConfigHost,
    /// The user file (under the home) or the repository file (under the
    /// repository root).
    pub scope: ConfigScope,
}

impl ConfigurationFile {
    /// The configuration file at `path`, when `path` is exactly one of the
    /// four configuration paths of `context`; `None` otherwise. Reads
    /// nothing. When the home is the repository root, its files are the
    /// user's, as the user scope is checked first.
    #[must_use]
    pub fn select(path: &Path, context: &FileResolutionContext) -> Option<Self> {
        let target = PathIdentity::new(path);
        [(context.home_dir(), ConfigScope::User), (context.repository_root(), ConfigScope::Repository)]
            .into_iter()
            .filter_map(|(base, scope)| base.map(|base| (base, scope)))
            .find_map(|(base, scope)| {
                [ConfigHost::Darkmatter, ConfigHost::Claudine]
                    .into_iter()
                    .find(|host| PathIdentity::new(&host.file_under(base)) == target)
                    .map(|host| Self { host, scope })
            })
    }

    /// How reports name the contract: its type and the schema it is in.
    #[must_use]
    pub fn definition(self) -> String {
        format!("`{}` in {}", self.scope.config_type(), self.host.schema().1)
    }

    /// The contract's schema, with its descriptions and defaults, prepared
    /// on first use and shared by every later document.
    ///
    /// ## Errors
    ///
    /// The error preparing a compiled-in schema raised; the shipped texts are
    /// proven to prepare by this crate's tests.
    pub fn resolved(self) -> Result<&'static ResolvedSchema, Arc<SchemaError>> {
        static CELLS: [OnceLock<Result<ResolvedSchema, Arc<SchemaError>>>; 4] =
            [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];
        let index = match (self.host, self.scope) {
            (ConfigHost::Darkmatter, ConfigScope::User) => 0,
            (ConfigHost::Darkmatter, ConfigScope::Repository) => 1,
            (ConfigHost::Claudine, ConfigScope::User) => 2,
            (ConfigHost::Claudine, ConfigScope::Repository) => 3,
        };
        let (source, virtual_path) = self.host.schema();
        CELLS[index]
            .get_or_init(|| resolve_host_config_type(source, virtual_path, self.scope.config_type()).map_err(Arc::new))
            .as_ref()
            .map_err(Arc::clone)
    }
}
