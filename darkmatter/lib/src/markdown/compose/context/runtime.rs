//! Runtime context (`ComposeContext`) captured once at compose start for
//! deterministic output across the transclusion graph.

use std::collections::HashMap;
use std::time::Duration;

/// Runtime context captured at compose start for deterministic output.
///
/// All date/time values are captured once when the context is created,
/// ensuring consistent values throughout the compose pipeline even
/// if the compose takes significant time.
///
/// Backed by `Arc` for cheap cloning across transclusion graphs.
/// The `values` map is the canonical backing store for all context variables.
/// Legacy public fields are kept for backward compatibility but are also
/// mirrored in the `values` map.
#[derive(Debug, Clone)]
pub struct ComposeContext {
    inner: std::sync::Arc<ComposeContextInner>,
}

/// Inner storage for ComposeContext, shared via Arc.
#[derive(Debug, Clone)]
struct ComposeContextInner {
    now: String,
    now_utc: String,
    today: String,
    yesterday: String,
    tomorrow: String,
    day: String,
    day_abbr: String,
    year: String,
    month: String,
    month_name: String,
    month_name_abbr: String,
    env: HashMap<String, String>,
    values: serde_json::Map<String, serde_json::Value>,
    /// Lazily-memoized `values` with the compose-time `AGENT`/`MODEL` env
    /// overrides applied. Computed once on first `get_effective`/`as_object`
    /// so a `ctx.*` lookup clones only the requested leaf rather than the
    /// whole context map. Reset by [`ComposeContext::env_mut`] when the env
    /// (which the overrides derive from) is mutated.
    overrides: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>>,
    capture_diagnostics: Vec<super::ContextMergeDiagnostic>,
    capture_timings: Vec<(String, Duration)>,
}

impl PartialEq for ComposeContext {
    fn eq(&self, other: &Self) -> bool {
        // Same Arc instance = equal; otherwise compare values
        std::sync::Arc::ptr_eq(&self.inner, &other.inner) || self.inner.values == other.inner.values
    }
}

impl Eq for ComposeContext {}

// Legacy public field accessors (backward compatible)
impl ComposeContext {
    /// ISO 8601 local datetime.
    pub fn now(&self) -> &str {
        &self.inner.now
    }
    /// ISO 8601 UTC datetime.
    pub fn now_utc(&self) -> &str {
        &self.inner.now_utc
    }
    /// Local date YYYY-MM-DD.
    pub fn today(&self) -> &str {
        &self.inner.today
    }
    /// Yesterday YYYY-MM-DD.
    pub fn yesterday(&self) -> &str {
        &self.inner.yesterday
    }
    /// Tomorrow YYYY-MM-DD.
    pub fn tomorrow(&self) -> &str {
        &self.inner.tomorrow
    }
    /// Full day of week name.
    pub fn day(&self) -> &str {
        &self.inner.day
    }
    /// Abbreviated day of week.
    pub fn day_abbr(&self) -> &str {
        &self.inner.day_abbr
    }
    /// Four-digit year.
    pub fn year(&self) -> &str {
        &self.inner.year
    }
    /// Two-digit month (01-12).
    pub fn month(&self) -> &str {
        &self.inner.month
    }
    /// Full month name.
    pub fn month_name(&self) -> &str {
        &self.inner.month_name
    }
    /// Abbreviated month name.
    pub fn month_name_abbr(&self) -> &str {
        &self.inner.month_name_abbr
    }
    /// Environment variables snapshot.
    pub fn env(&self) -> &HashMap<String, String> {
        &self.inner.env
    }
    /// Access the values map.
    pub fn values(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.inner.values
    }
}

impl ComposeContext {
    /// Captures the current runtime context using CWD as the base directory.
    ///
    /// This snapshots:
    /// - Current local and UTC time
    /// - Today, yesterday, and tomorrow dates
    /// - Day of week (full and abbreviated)
    /// - Year, month (numeric and named)
    /// - All environment variables
    /// - Repository, monorepo, OS, and hardware information (via sniff)
    pub fn capture() -> Self {
        match std::env::current_dir() {
            Ok(base_dir) => Self::capture_for_dir(&base_dir),
            Err(e) => {
                // CWD discovery failed: populate date/time/env but leave
                // sniff-derived fields null, and record the failure.
                let (mut values, diagnostics) = (
                    serde_json::Map::new(),
                    vec![
                        super::ContextMergeDiagnostic::PartialRuntimeCapture {
                            area: "cwd",
                            detail: format!("current_dir() failed: {e}"),
                        },
                    ],
                );
                super::capture::populate_datetime(&mut values);
                Self::from_values(
                    values,
                    diagnostics,
                    Vec::new(),
                    std::env::vars().collect(),
                )
            }
        }
    }

    /// Captures only the groups that cost no discovery: date/time, plus the
    /// environment snapshot every capture takes.
    ///
    /// Git, repository, file-change, language, document, OS, hardware, and GPU
    /// values are left uncaptured. Nothing becomes unresolvable: an expression
    /// naming one of those keys triggers
    /// [`resolve_ctx`](crate::markdown::compose::expression) to capture that
    /// single group on demand. Skipping them up front therefore drops work no
    /// document read, rather than narrowing what `ctx.*` can answer.
    ///
    /// Prefer [`capture_for_document`](Self::capture_for_document) when the
    /// document is already in hand — it captures exactly the groups the
    /// document names, which additionally keeps those groups inside the
    /// compose cache key (see `cache::hashing::context_hash`).
    pub fn capture_minimal() -> Self {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::capture_for_content(&base_dir, "")
    }

    /// Captures the runtime context using the given base directory.
    pub fn capture_for_dir(base_dir: &std::path::Path) -> Self {
        let (values, capture_diagnostics, timings, environment) =
            super::capture::capture_runtime_context(base_dir);
        Self::from_values(values, capture_diagnostics, timings, environment)
    }

    /// Demand-driven capture: scans `content` for `ctx.*` references and
    /// only captures the context groups actually needed.
    ///
    /// If the document uses no `ctx.*` variables, only date/time (zero I/O)
    /// is captured. This avoids git, repo, docs, OS, and hardware detection
    /// for documents that don't need them.
    ///
    /// **Note:** This scans only the provided string. If the document has
    /// frontmatter values containing `ctx.*` references, use
    /// [`capture_for_document`](Self::capture_for_document) instead.
    pub fn capture_for_content(base_dir: &std::path::Path, content: &str) -> Self {
        let (values, capture_diagnostics, timings, environment) =
            super::capture::capture_runtime_context_for_content(base_dir, content);
        Self::from_values(values, capture_diagnostics, timings, environment)
    }

    /// Demand-driven capture that scans both frontmatter values and body
    /// content for `ctx.*` references.
    ///
    /// This is the correct method when composing a full document, since
    /// frontmatter values may contain `ctx.*` references that are absent
    /// from the body.
    pub fn capture_for_document(
        base_dir: &std::path::Path,
        doc: &crate::markdown::Markdown,
    ) -> Self {
        let fm_json = serde_json::to_string(doc.frontmatter().as_map()).unwrap_or_default();
        let combined = format!("{}\n{}", fm_json, doc.content());
        Self::capture_for_content(base_dir, &combined)
    }

    /// Captures exactly `requirements` from request-owned evidence.
    ///
    /// Missing supplied facts produce partial-capture diagnostics. This entry
    /// point never discovers CWD, environment, Git, repository topology, OS,
    /// hardware, or GPU state.
    pub fn capture_with_evidence(
        base_dir: &std::path::Path,
        requirements: &super::capture::ContextRequirements,
        evidence: &super::capture::ContextCaptureEvidence,
    ) -> Self {
        let (values, diagnostics, timings, environment) =
            super::capture::capture_runtime_context_with_evidence(
                base_dir,
                requirements,
                evidence,
            );
        Self::from_values(values, diagnostics, timings, environment)
    }

    /// Demand-driven supplied capture for one content fragment.
    pub fn capture_for_content_with_evidence(
        base_dir: &std::path::Path,
        content: &str,
        evidence: &super::capture::ContextCaptureEvidence,
    ) -> Self {
        let requirements = super::capture::ContextRequirements::for_content(content);
        Self::capture_with_evidence(base_dir, &requirements, evidence)
    }

    /// Demand-driven supplied capture over authored frontmatter and body.
    pub fn capture_for_document_with_evidence(
        base_dir: &std::path::Path,
        document: &crate::markdown::Markdown,
        evidence: &super::capture::ContextCaptureEvidence,
    ) -> Self {
        let requirements = super::capture::ContextRequirements::for_document(document);
        Self::capture_with_evidence(base_dir, &requirements, evidence)
    }

    /// Build a `ComposeContext` from pre-computed values.
    fn from_values(
        values: serde_json::Map<String, serde_json::Value>,
        capture_diagnostics: Vec<super::ContextMergeDiagnostic>,
        capture_timings: Vec<(String, Duration)>,
        env: HashMap<String, String>,
    ) -> Self {
        let get_str = |key: &str| -> String {
            values
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Self {
            inner: std::sync::Arc::new(ComposeContextInner {
                now: get_str("now"),
                now_utc: get_str("now_utc"),
                today: get_str("today"),
                yesterday: get_str("yesterday"),
                tomorrow: get_str("tomorrow"),
                day: get_str("day"),
                day_abbr: get_str("day_abbr"),
                year: get_str("year"),
                month: get_str("month"),
                month_name: get_str("month_name"),
                month_name_abbr: get_str("month_name_abbr"),
                env,
                values,
                overrides: std::sync::OnceLock::new(),
                capture_diagnostics,
                capture_timings,
            }),
        }
    }

    /// Looks up a value from the backing store.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.inner.values.get(key)
    }

    /// Looks up a value after applying compose-time environment overrides.
    pub(crate) fn get_effective(&self, key: &str) -> Option<serde_json::Value> {
        self.effective_values().get(key).cloned()
    }

    /// Returns the full context as a JSON object.
    pub fn as_object(&self) -> serde_json::Value {
        serde_json::Value::Object(self.effective_values().clone())
    }

    /// Returns the `values` map with `AGENT`/`MODEL` env overrides applied,
    /// memoized once per context so repeated `ctx.*` lookups don't re-clone
    /// (and re-override) the entire map on every access.
    fn effective_values(&self) -> &serde_json::Map<String, serde_json::Value> {
        self.inner
            .overrides
            .get_or_init(|| self.values_with_env_agent_overrides())
    }

    /// Returns diagnostics from the capture phase.
    pub fn diagnostics(&self) -> &[super::ContextMergeDiagnostic] {
        &self.inner.capture_diagnostics
    }

    /// Returns per-group timings from the capture phase.
    ///
    /// Each entry is `(group_name, elapsed)`. Empty when no groups
    /// required I/O (e.g., DateTime-only capture).
    pub fn capture_timings(&self) -> &[(String, Duration)] {
        &self.inner.capture_timings
    }

    /// Iterates the exposed key names.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.inner.values.keys().map(String::as_str)
    }

    /// Returns a mutable reference to the inner env map.
    ///
    /// Clones the `Arc` on write if shared. Use this to inject
    /// environment overrides before passing the context to
    /// [`ComposeOptions::new_with_context`].
    pub fn env_mut(&mut self) -> &mut HashMap<String, String> {
        let inner = std::sync::Arc::make_mut(&mut self.inner);
        // Mutating the env can change the AGENT/MODEL-derived overrides, so
        // discard any memoized effective map; it is rebuilt on next access.
        inner.overrides = std::sync::OnceLock::new();
        &mut inner.env
    }

    fn values_with_env_agent_overrides(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut values = self.inner.values.clone();
        if let Some(agent) = normalized_env_value(&self.inner.env, "AGENT", "unknown") {
            values.insert(
                "agent".to_string(),
                serde_json::Value::String(agent),
            );
        }
        if let Some(model) = normalized_env_value(&self.inner.env, "MODEL", "default") {
            values.insert(
                "model".to_string(),
                serde_json::Value::String(model),
            );
        }
        values
    }

    /// Creates a context with fixed values for testing.
    ///
    /// Uses an empty environment to ensure deterministic test behavior.
    #[cfg(test)]
    pub fn fixed_for_testing() -> Self {
        let mut values = serde_json::Map::new();
        let fields = [
            ("now", "2024-06-15T10:30:00-07:00"),
            ("now_utc", "2024-06-15T17:30:00Z"),
            ("today", "2024-06-15"),
            ("yesterday", "2024-06-14"),
            ("tomorrow", "2024-06-16"),
            ("day", "Saturday"),
            ("day_abbr", "Sat"),
            ("year", "2024"),
            ("month", "06"),
            ("month_name", "June"),
            ("month_name_abbr", "Jun"),
        ];
        for (k, v) in &fields {
            values.insert((*k).to_string(), serde_json::Value::String(v.to_string()));
        }

        let get_str = |key: &str| -> String {
            values
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Self {
            inner: std::sync::Arc::new(ComposeContextInner {
                now: get_str("now"),
                now_utc: get_str("now_utc"),
                today: get_str("today"),
                yesterday: get_str("yesterday"),
                tomorrow: get_str("tomorrow"),
                day: get_str("day"),
                day_abbr: get_str("day_abbr"),
                year: get_str("year"),
                month: get_str("month"),
                month_name: get_str("month_name"),
                month_name_abbr: get_str("month_name_abbr"),
                env: HashMap::new(),
                values,
                overrides: std::sync::OnceLock::new(),
                capture_diagnostics: Vec::new(),
                capture_timings: Vec::new(),
            }),
        }
    }

    /// Returns a copy of [`fixed_for_testing`](Self::fixed_for_testing) with the
    /// given values inserted (or overwritten).
    ///
    /// Lets a test build two contexts that differ only in a single value — e.g.
    /// a volatile `timestamp` — to prove the reference-graph identity is
    /// complete rather than reusing the persistent-cache `context_hash`.
    #[cfg(test)]
    pub(crate) fn fixed_for_testing_with(
        extra: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
    ) -> Self {
        let mut ctx = Self::fixed_for_testing();
        let inner = std::sync::Arc::make_mut(&mut ctx.inner);
        for (key, value) in extra {
            inner.values.insert(key.to_string(), value);
        }
        inner.overrides = std::sync::OnceLock::new();
        ctx
    }
}

fn normalized_env_value(
    env: &HashMap<String, String>,
    key: &str,
    default: &str,
) -> Option<String> {
    env.get(key).map(|value| {
        let trimmed = value.trim_ascii();
        if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed.to_string()
        }
    })
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    use crate::markdown::compose::{ContextCaptureEvidence, ContextGroup, ContextRequirements};

    #[test]
    fn ambient_and_supplied_absence_populate_the_same_repository_values() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let content = "{{ ctx.repo_root }} {{ ctx.dirty_files }} {{ ctx.programming_language }}";
        let ambient = ComposeContext::capture_for_content(directory.path(), content);
        let supplied = ComposeContext::capture_for_content_with_evidence(
            directory.path(),
            content,
            &ContextCaptureEvidence::new(HashMap::new())
                .with_git(None)
                .with_repository(None, None)
                .with_file_changes(Vec::new())
                .with_languages(None),
        );

        for key in ["repo_root", "dirty_files", "programming_language"] {
            assert_eq!(ambient.get(key), supplied.get(key), "key: {key}");
        }
        assert!(supplied.diagnostics().is_empty());
    }

    #[test]
    fn missing_supplied_evidence_is_partial_and_never_falls_back() {
        let context = ComposeContext::capture_for_content_with_evidence(
            std::path::Path::new("does-not-need-to-exist"),
            "{{ ctx.os }}",
            &ContextCaptureEvidence::new(HashMap::new()),
        );

        assert_eq!(context.get("os"), Some(&serde_json::Value::Null));
        assert!(context.capture_timings().is_empty());
        assert!(context.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            super::super::ContextMergeDiagnostic::PartialRuntimeCapture { area: "os", .. }
        )));
    }

    #[test]
    fn unrequested_host_groups_need_no_host_evidence() {
        let requirements = ContextRequirements::for_content("ordinary markdown");
        assert!(requirements.contains(ContextGroup::DateTime));
        assert!(!requirements.contains(ContextGroup::Os));
        assert!(!requirements.contains(ContextGroup::Hardware));
        assert!(!requirements.contains(ContextGroup::Gpu));

        let context = ComposeContext::capture_with_evidence(
            std::path::Path::new("."),
            &requirements,
            &ContextCaptureEvidence::new(HashMap::new()),
        );
        assert!(!context.values().contains_key("os"));
        assert!(!context.values().contains_key("cpu_cores"));
        assert!(!context.values().contains_key("gpu"));
        assert!(context.diagnostics().is_empty());
    }

    #[test]
    #[serial_test::serial(supplied_environment)]
    fn supplied_environment_survives_process_environment_mutation() {
        let prior = std::env::var("AGENT").ok();
        let mut environment = HashMap::new();
        environment.insert("AGENT".to_string(), "captured-agent".to_string());
        let evidence = ContextCaptureEvidence::new(environment);

        unsafe { std::env::set_var("AGENT", "later-agent") };
        let context = ComposeContext::capture_for_content_with_evidence(
            std::path::Path::new("."),
            "{{ ctx.agent }}",
            &evidence,
        );
        unsafe {
            match prior {
                Some(value) => std::env::set_var("AGENT", value),
                None => std::env::remove_var("AGENT"),
            }
        }

        assert_eq!(context.env().get("AGENT").map(String::as_str), Some("captured-agent"));
        assert_eq!(
            context.get("agent"),
            Some(&serde_json::Value::String("captured-agent".to_string()))
        );
    }
}
