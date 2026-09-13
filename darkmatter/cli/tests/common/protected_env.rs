//! The environment variables the `md` spawn contract owns, and what undoing
//! one after `build()` costs.
//!
//! Two consumers share this table so they cannot drift: the fixture builder
//! validates a declared override against it, and `spawn_site_guard.rs` flags a
//! post-`build()` override that was never declared. Neither list is a
//! duplicate of the other.
//!
//! The split is the whole point. A *containment* variable exists to keep the
//! developer's machine out of the child — undoing it re-contaminates the run,
//! and no test-specific claim can justify that, so there is no declaring
//! method for one. A *behavior input* is pinned to a deterministic default,
//! and a test whose subject is that behavior is making a legitimate claim; it
//! declares the claim on the builder, where it is visible at construction
//! beside `host_path`, `fake_only_path`, `ambient_context`, and
//! `inherit_no_env`.

#![allow(dead_code)]

/// Why the spawn contract owns a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedClass {
    /// Home, config, cache, and temp anchors, which the fixture points at its
    /// own workspace.
    HomeContainment,
    /// Git plumbing, which overrides cwd-based repository discovery and so
    /// defeats the pinned launch directory outright.
    GitContainment,
    /// Terminal capability and color inputs, pinned to a deterministic frame.
    RenderingInput,
    /// Darkmatter's own inputs — the `DARKMATTER_*`/`DM_*`/`MD_*` namespaces
    /// plus the names `md` reads directly.
    ApplicationInput,
}

impl ProtectedClass {
    /// Whether the class exists to keep host state out of the child, in which
    /// case no declaration can justify overriding it.
    pub fn is_containment(self) -> bool {
        matches!(self, Self::HomeContainment | Self::GitContainment)
    }

    /// The `MdCommandBuilder` method that declares an override, for the two
    /// classes that have one.
    pub fn declaring_method(self) -> Option<&'static str> {
        match self {
            Self::RenderingInput => Some("rendering_input"),
            Self::ApplicationInput => Some("application_input"),
            Self::HomeContainment | Self::GitContainment => None,
        }
    }

    /// How the class reads in a failure message.
    pub fn description(self) -> &'static str {
        match self {
            Self::HomeContainment => "home/config/cache/temp anchor",
            Self::GitContainment => "Git plumbing variable",
            Self::RenderingInput => "rendering input",
            Self::ApplicationInput => "darkmatter application input",
        }
    }
}

const HOME_CONTAINMENT: &[&str] = &[
    "HOME",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "TMPDIR",
    "TEMP",
    "TMP",
];

const GIT_CONTAINMENT: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

const RENDERING_INPUTS: &[&str] = &[
    "COLUMNS",
    "LINES",
    "TERM",
    "COLORTERM",
    "COLORFGBG",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "NO_COLOR",
    "DARK_MODE",
    "THEME",
    "CODE_THEME",
    "PREFER_ITALICS",
    "TERMINAL_IMAGES",
];

const APPLICATION_INPUTS: &[&str] = &[
    "AGENT",
    "MODEL",
    "RUST_LOG",
    "HASH_PROPERTY",
    "HASH_IGNORE_PROPERTIES",
    "BASELINE_SCHEMA",
];

const APPLICATION_PREFIXES: &[&str] = &["DARKMATTER_", "DM_", "MD_"];

/// Protected, but by its own vocabulary: the builder composes it
/// (`host_path`, `fake_only_path`) and the guard carries dedicated forms for a
/// post-`build()` set or remove. Classifying it here would open a second,
/// weaker route to the same escape.
pub const PATH_VARIABLE: &str = "PATH";

/// The class `key` belongs to, or `None` when the spawn contract leaves it to
/// the call site.
pub fn protected_class(key: &str) -> Option<ProtectedClass> {
    if key == PATH_VARIABLE {
        return None;
    }
    if HOME_CONTAINMENT.contains(&key) || key.starts_with("XDG_") {
        return Some(ProtectedClass::HomeContainment);
    }
    if GIT_CONTAINMENT.contains(&key) || key.starts_with("GIT_CONFIG_") {
        return Some(ProtectedClass::GitContainment);
    }
    if RENDERING_INPUTS.contains(&key) {
        return Some(ProtectedClass::RenderingInput);
    }
    if APPLICATION_INPUTS.contains(&key)
        || APPLICATION_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
    {
        return Some(ProtectedClass::ApplicationInput);
    }
    None
}
