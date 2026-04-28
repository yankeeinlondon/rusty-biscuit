//! Typed path templates for provider-owned filesystem locations.
//!
//! Phase 5 of the centralized providers refactor introduces strongly-typed
//! path data on each [`ProviderInfo`](super::ProviderInfo). Older fields
//! held free-form strings such as `"~/.claude/projects/<encoded-cwd>/"`; this
//! module replaces that prose with structured templates whose segments can
//! be resolved against a runtime [`PathContext`].
//!
//! Templates are always `'static` so they live in the binary's read-only
//! data segment with no heap allocation. A [`PathTemplate::Static`] entry
//! captures a literal path string; a [`PathTemplate::Templated`] entry
//! captures an ordered list of [`PathSegment`] values.

use std::path::{Path, PathBuf};

/// One element in a templated provider path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSegment {
    /// A literal path component, e.g. `"projects"`.
    Literal(&'static str),
    /// The user's home directory (typically `~`).
    HomeDir,
    /// The repository root for the current invocation.
    RepoRoot,
    /// An encoded form of the current working directory used by Claude
    /// (e.g. `/Users/me/proj` -> `-Users-me-proj`).
    EncodedCwd,
    /// The agent's session identifier.
    SessionId,
    /// The current date in `YYYYMMDD` format.
    DateYYYYMMDD,
    /// A short hash of the project root path.
    ProjectHash,
    /// A sanitized form of the current working directory.
    SanitizedCwd,
    /// A short hash of a directory path.
    DirHash,
    /// A glob wildcard component for diagnostics-only output.
    Glob(GlobKind),
}

/// Glob wildcard kind used inside templated paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobKind {
    /// Matches any single path component (`*`).
    Single,
    /// Matches any number of components recursively (`**`).
    Recursive,
}

/// A static or templated filesystem path tied to a provider.
#[derive(Debug, Clone, Copy)]
pub enum PathTemplate {
    /// A literal path. Useful for stable, fully-qualified locations
    /// such as `~/.claude/projects/`.
    Static(&'static str),
    /// A templated path resolved against a [`PathContext`] at runtime.
    Templated {
        /// Pretty-printed template form for diagnostics, e.g.
        /// `"~/.claude/projects/{encoded_cwd}/{session_id}.jsonl"`.
        raw: &'static str,
        /// Ordered list of segments composing the path.
        segments: &'static [PathSegment],
    },
}

/// Runtime context used to resolve [`PathTemplate::Templated`] values.
#[derive(Debug, Default, Clone, Copy)]
pub struct PathContext<'a> {
    /// User home directory (`$HOME`).
    pub home: Option<&'a Path>,
    /// Repository root for the active invocation.
    pub repo_root: Option<&'a Path>,
    /// Active session identifier.
    pub session_id: Option<&'a str>,
    /// Encoded current working directory (Claude's `<encoded-directory>`).
    pub encoded_cwd: Option<&'a str>,
    /// Calendar date as `YYYYMMDD`.
    pub date_yyyymmdd: Option<&'a str>,
    /// Short hash of the project root.
    pub project_hash: Option<&'a str>,
    /// Sanitized cwd identifier.
    pub sanitized_cwd: Option<&'a str>,
    /// Short hash of an arbitrary directory.
    pub dir_hash: Option<&'a str>,
}

impl PathTemplate {
    /// Resolves this template into a [`PathBuf`] using the provided
    /// [`PathContext`]. Returns `None` if any required dynamic segment is
    /// not present in the context.
    pub fn resolve(&self, ctx: &PathContext<'_>) -> Option<PathBuf> {
        match self {
            Self::Static(raw) => Some(expand_home_prefix(raw, ctx)),
            Self::Templated { segments, .. } => {
                let mut buf = PathBuf::new();
                for segment in *segments {
                    let component = match segment {
                        PathSegment::Literal(lit) => (*lit).to_string(),
                        PathSegment::HomeDir => ctx.home?.to_string_lossy().into_owned(),
                        PathSegment::RepoRoot => ctx.repo_root?.to_string_lossy().into_owned(),
                        PathSegment::EncodedCwd => ctx.encoded_cwd?.to_string(),
                        PathSegment::SessionId => ctx.session_id?.to_string(),
                        PathSegment::DateYYYYMMDD => ctx.date_yyyymmdd?.to_string(),
                        PathSegment::ProjectHash => ctx.project_hash?.to_string(),
                        PathSegment::SanitizedCwd => ctx.sanitized_cwd?.to_string(),
                        PathSegment::DirHash => ctx.dir_hash?.to_string(),
                        PathSegment::Glob(GlobKind::Single) => "*".to_string(),
                        PathSegment::Glob(GlobKind::Recursive) => "**".to_string(),
                    };
                    if component.is_empty() {
                        continue;
                    }
                    buf.push(component);
                }
                Some(buf)
            }
        }
    }

    /// Returns the raw template string for diagnostic display.
    pub fn raw(&self) -> &'static str {
        match self {
            Self::Static(raw) => raw,
            Self::Templated { raw, .. } => raw,
        }
    }
}

fn expand_home_prefix(raw: &'static str, ctx: &PathContext<'_>) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = ctx.home
    {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn static_template_round_trips_raw() {
        let template = PathTemplate::Static("~/.claude/projects/");
        assert_eq!(template.raw(), "~/.claude/projects/");
    }

    #[test]
    fn static_template_expands_home_when_present() {
        let home = Path::new("/Users/test");
        let ctx = PathContext {
            home: Some(home),
            ..Default::default()
        };
        let template = PathTemplate::Static("~/.claude/projects/");
        let resolved = template.resolve(&ctx).expect("resolve returns Some");
        assert_eq!(resolved, Path::new("/Users/test/.claude/projects/"));
    }

    #[test]
    fn static_template_falls_back_when_home_missing() {
        let ctx = PathContext::default();
        let template = PathTemplate::Static("/etc/codex/skills");
        let resolved = template.resolve(&ctx).expect("resolve returns Some");
        assert_eq!(resolved, Path::new("/etc/codex/skills"));
    }

    #[test]
    fn templated_path_substitutes_segments() {
        let home = Path::new("/Users/test");
        let ctx = PathContext {
            home: Some(home),
            session_id: Some("abc-123"),
            ..Default::default()
        };
        let template = PathTemplate::Templated {
            raw: "{home}/.claude/projects/{session_id}.jsonl",
            segments: &[
                PathSegment::HomeDir,
                PathSegment::Literal(".claude"),
                PathSegment::Literal("projects"),
                PathSegment::SessionId,
            ],
        };
        let resolved = template.resolve(&ctx).expect("resolve returns Some");
        assert_eq!(
            resolved,
            Path::new("/Users/test/.claude/projects/abc-123")
        );
    }

    #[test]
    fn templated_path_missing_context_returns_none() {
        let ctx = PathContext::default();
        let template = PathTemplate::Templated {
            raw: "{home}/.claude/{session_id}.jsonl",
            segments: &[PathSegment::HomeDir, PathSegment::SessionId],
        };
        assert!(template.resolve(&ctx).is_none());
    }

    #[test]
    fn glob_segment_renders_wildcards() {
        let home = Path::new("/h");
        let ctx = PathContext {
            home: Some(home),
            ..Default::default()
        };
        let template = PathTemplate::Templated {
            raw: "{home}/**/{*}",
            segments: &[
                PathSegment::HomeDir,
                PathSegment::Glob(GlobKind::Recursive),
                PathSegment::Glob(GlobKind::Single),
            ],
        };
        let resolved = template.resolve(&ctx).expect("resolves");
        assert_eq!(resolved, Path::new("/h/**/*"));
    }
}
