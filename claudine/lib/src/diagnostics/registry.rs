//! The single-source registry of locked diagnostic codes.
//!
//! This is the ratified code catalog from
//! `claudine/features/2026-06-28-real-errors/error-catalog.md` §3, encoded as
//! data: every dotted `code`, its [`Category`]/[`Disposition`]/[`Origin`]
//! facets, an optional severity override, and the field names its `detail`
//! payload carries. It is the contract that `claudine errors` introspects and
//! that a `Diagnostic` implementation must agree with.
//!
//! Evolution is **additive-only**: adding a new [`CodeSpec`] or a new
//! `detail` field name is non-breaking; renaming or removing one breaks any
//! author `when:` clause matching it (error-catalog §2.8 / §7.8).

use super::facets::{Category, Disposition, Origin, Severity};

/// One row of the locked code catalog.
#[derive(Debug, Clone, Copy)]
pub struct CodeSpec {
    /// The stable dotted code, always prefixed by its category slug
    /// (`cap.plan_limit`). This is the public API contract.
    pub code: &'static str,
    /// Coarse domain. Always equals the prefix of `code`.
    pub category: Category,
    /// Generic-strategy facet. Stable per code (the `code → disposition`
    /// invariant handlers rely on — error-catalog §7.5).
    pub disposition: Disposition,
    /// Who remediates.
    pub origin: Origin,
    /// Severity override; `None` derives from
    /// [`Disposition::default_severity`].
    pub severity_override: Option<Severity>,
    /// Field names this code's `detail` payload carries, projected to
    /// `err.detail.<field>`. The shared field vocabulary (error-catalog §2.5)
    /// keeps the same concept under the same name across codes.
    pub detail: &'static [&'static str],
}

impl CodeSpec {
    /// The effective operator-facing severity (override, else disposition default).
    pub fn severity(&self) -> Severity {
        self.severity_override
            .unwrap_or_else(|| self.disposition.default_severity())
    }
}

/// The locked code catalog (error-catalog §3), grouped by category.
pub const CODES: &[CodeSpec] = &[
    // --- auth — identity / authorization ---
    CodeSpec {
        code: "auth.invalid",
        category: Category::Auth,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "reason"],
    },
    CodeSpec {
        code: "auth.permission",
        category: Category::Auth,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "action"],
    },
    // --- cap — usage / account limits ---
    CodeSpec {
        code: "cap.rate_limit",
        category: Category::Cap,
        disposition: Disposition::Throttled,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "reset_at", "retry_after_ms", "scope"],
    },
    CodeSpec {
        code: "cap.plan_limit",
        category: Category::Cap,
        disposition: Disposition::Throttled,
        origin: Origin::Provider,
        severity_override: None,
        detail: &[
            "provider",
            "reset_at",
            "retry_after_ms",
            "limit_kind",
            "scope",
        ],
    },
    CodeSpec {
        code: "cap.billing",
        category: Category::Cap,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "reason"],
    },
    // --- timeout ---
    CodeSpec {
        code: "timeout.step_silence",
        category: Category::Timeout,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["elapsed_ms", "limit_ms"],
    },
    CodeSpec {
        code: "timeout.wall_clock",
        category: Category::Timeout,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["elapsed_ms", "limit_ms"],
    },
    // --- provider — the agent run (platform + behavior) ---
    CodeSpec {
        code: "provider.unavailable",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["provider", "path"],
    },
    CodeSpec {
        code: "provider.launch_failed",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["provider"],
    },
    CodeSpec {
        code: "provider.exited",
        category: Category::Provider,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "exit_code", "signal"],
    },
    CodeSpec {
        code: "provider.interrupted",
        category: Category::Provider,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Caller,
        severity_override: None,
        detail: &["signal"],
    },
    CodeSpec {
        code: "provider.stream_error",
        category: Category::Provider,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["provider", "raw"],
    },
    CodeSpec {
        code: "provider.context_pressure",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        // Override to warning: context pressure is a heads-up, not a hard
        // failure (error-catalog §3).
        severity_override: Some(Severity::Warning),
        detail: &["provider"],
    },
    // --- composition — author's prompt document (Darkmatter) ---
    CodeSpec {
        code: "composition.invalid_file_reference",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        // One identity for the same authoring mistake across proxying,
        // expressions, schemas, and transclusion — so the payload has to carry
        // enough context to tell those surfaces apart (`source_path`,
        // `property`, `event`) without a surface-specific code.
        //
        // `base_dir` and `fallback_dir` are **compatibility projections**: the
        // two anchors the pre-`candidates` payload exposed, retained so an
        // existing `when:` clause keeps matching. `candidates` supersedes them
        // with the ordered, provenance-carrying record sequence.
        //
        // `failure` is the stable snake_case failure classification —
        // `invalid_syntax`, `missing_context`, `no_match`, `permission_io`, or
        // `unsupported_remote`. It is distinct from `kind`, which names the
        // reference kind. The current resolver cannot supply `failure`,
        // `source_path`, `property`, `event`, `repository_root`, or
        // `candidates`; they project as `null` until the file-resolution
        // feature replaces them with typed values (spec §D3).
        detail: &[
            "reference",
            "kind",
            "base_dir",
            "suggestions",
            "fallback_dir",
            "source_path",
            "property",
            "event",
            "repository_root",
            "candidates",
            "failure",
        ],
    },
    CodeSpec {
        code: "composition.unknown_function",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["name", "suggestions"],
    },
    CodeSpec {
        code: "composition.expression_invalid",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["expression", "message"],
    },
    CodeSpec {
        code: "composition.schema_load",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["source_path", "message"],
    },
    CodeSpec {
        code: "composition.schema_parse",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["source_path", "property", "message"],
    },
    CodeSpec {
        code: "composition.schema_validation",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["source_path", "problems", "pointer_paths"],
    },
    CodeSpec {
        code: "composition.missing_properties",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["missing", "pointer_paths"],
    },
    CodeSpec {
        code: "composition.frontmatter_parse",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["source_path"],
    },
    CodeSpec {
        code: "composition.lifecycle_invalid",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["property", "message"],
    },
    CodeSpec {
        code: "composition.shell_expansion",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["command"],
    },
    CodeSpec {
        code: "composition.failed",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        severity_override: None,
        detail: &["source_path"],
    },
    // --- document — agent's output postconditions ---
    CodeSpec {
        code: "document.missing_frontmatter",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["doc", "property"],
    },
    CodeSpec {
        code: "document.invalid_frontmatter",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["doc", "property", "problems"],
    },
    CodeSpec {
        code: "document.empty",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["doc"],
    },
    // --- vcs — git-state postconditions ---
    CodeSpec {
        code: "vcs.unexpected_dirty_files",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["scope", "files"],
    },
    CodeSpec {
        code: "vcs.unexpected_commits",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["commits"],
    },
    CodeSpec {
        code: "vcs.missing_dirty_files",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["scope"],
    },
    // --- io — filesystem / network plumbing ---
    CodeSpec {
        code: "io.read_failed",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.write_failed",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.permission_denied",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.network",
        category: Category::Io,
        disposition: Disposition::Transient,
        origin: Origin::Environment,
        severity_override: None,
        detail: &["url", "message"],
    },
    // --- config — Claudine / user configuration ---
    CodeSpec {
        code: "config.invalid",
        category: Category::Config,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        severity_override: None,
        detail: &["field", "message"],
    },
    CodeSpec {
        code: "config.mcp_invalid",
        category: Category::Config,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        severity_override: None,
        detail: &["server", "message"],
    },
    // --- usage — Rust API misuse ---
    CodeSpec {
        code: "usage.invalid_argument",
        category: Category::Usage,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        severity_override: None,
        detail: &["argument", "expected"],
    },
    CodeSpec {
        code: "usage.unsupported",
        category: Category::Usage,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        severity_override: None,
        detail: &["operation", "provider"],
    },
    // --- runaway — content-guard trips (always unrecoverable) ---
    CodeSpec {
        code: "runaway.volume",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["measure", "cap"],
    },
    CodeSpec {
        code: "runaway.repetition",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["cycles"],
    },
    CodeSpec {
        code: "runaway.exit_expression",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        severity_override: None,
        detail: &["expression"],
    },
    // --- internal — our bugs ---
    CodeSpec {
        code: "internal.bug",
        category: Category::Internal,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Internal,
        severity_override: None,
        detail: &["message"],
    },
    CodeSpec {
        code: "internal.serialization",
        category: Category::Internal,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Internal,
        severity_override: None,
        detail: &["message"],
    },
];

/// Look up the [`CodeSpec`] for a dotted code, if it is in the locked catalog.
pub fn code_spec(code: &str) -> Option<&'static CodeSpec> {
    CODES.iter().find(|spec| spec.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_prefixed_by_its_category() {
        for spec in CODES {
            let prefix = format!("{}.", spec.category.as_str());
            assert!(
                spec.code.starts_with(&prefix),
                "code `{}` is not prefixed by its category `{}`",
                spec.code,
                spec.category.as_str()
            );
        }
    }

    #[test]
    fn codes_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for spec in CODES {
            assert!(seen.insert(spec.code), "duplicate code `{}`", spec.code);
        }
    }

    #[test]
    fn every_category_has_at_least_one_code() {
        for &cat in Category::ALL {
            assert!(
                CODES.iter().any(|spec| spec.category == cat),
                "category `{}` has no codes",
                cat.as_str()
            );
        }
    }

    #[test]
    fn lookup_finds_known_codes_and_misses_unknown() {
        assert_eq!(code_spec("cap.plan_limit").unwrap().code, "cap.plan_limit");
        assert!(code_spec("cap.does_not_exist").is_none());
    }

    #[test]
    fn plan_limit_carries_throttle_timing_detail() {
        let spec = code_spec("cap.plan_limit").unwrap();
        assert_eq!(spec.disposition, Disposition::Throttled);
        assert!(spec.detail.contains(&"reset_at"));
        assert!(spec.detail.contains(&"retry_after_ms"));
        assert!(spec.detail.contains(&"limit_kind"));
    }

    #[test]
    fn invalid_file_reference_projects_file_reference_diagnostic_fields() {
        let spec = code_spec("composition.invalid_file_reference").unwrap();
        assert_eq!(spec.origin, Origin::Author);
        for field in ["reference", "kind", "base_dir", "suggestions", "fallback_dir"] {
            assert!(
                spec.detail.contains(&field),
                "missing FileReferenceDiagnostic detail field `{field}`"
            );
        }
    }

    #[test]
    fn invalid_file_reference_carries_the_semantic_wrapper_context_fields() {
        // The wrapper owns this code for every surface (proxy, expression,
        // schema, transclusion), so the payload — not a second code — is what
        // distinguishes them.
        let spec = code_spec("composition.invalid_file_reference").unwrap();
        for field in [
            "source_path",
            "property",
            "event",
            "repository_root",
            "candidates",
            "failure",
        ] {
            assert!(
                spec.detail.contains(&field),
                "missing semantic-wrapper detail field `{field}`"
            );
        }
    }

    #[test]
    fn invalid_file_reference_extension_removed_no_field() {
        // The catalog evolves additively: the pre-extension field set must
        // still be declared, in its original order, ahead of the additions.
        let spec = code_spec("composition.invalid_file_reference").unwrap();
        assert_eq!(
            &spec.detail[..5],
            &["reference", "kind", "base_dir", "suggestions", "fallback_dir"]
        );
    }

    #[test]
    fn every_code_declares_unique_detail_fields() {
        // A duplicated field name would make `null_detail_for`'s key count
        // disagree with the declared list without failing any other test.
        for spec in CODES {
            let mut seen = std::collections::HashSet::new();
            for field in spec.detail {
                assert!(
                    seen.insert(field),
                    "code `{}` declares `{field}` twice",
                    spec.code
                );
            }
        }
    }

    #[test]
    fn runaway_codes_are_unrecoverable() {
        for spec in CODES.iter().filter(|s| s.category == Category::Runaway) {
            assert_eq!(
                spec.disposition,
                Disposition::Unrecoverable,
                "runaway code `{}` must be unrecoverable so a generic retry handler never loops it",
                spec.code
            );
        }
    }

    #[test]
    fn context_pressure_severity_override_is_warning() {
        let spec = code_spec("provider.context_pressure").unwrap();
        assert_eq!(spec.severity(), Severity::Warning);
        // ...even though its disposition would default to error.
        assert_eq!(spec.disposition.default_severity(), Severity::Error);
    }

    #[test]
    fn interrupted_origin_is_caller() {
        // error-catalog §7.7: the human pressed Ctrl-C — the caller.
        assert_eq!(code_spec("provider.interrupted").unwrap().origin, Origin::Caller);
    }

    #[test]
    fn catalog_covers_the_ratified_count() {
        // 12 categories; the faithful transcription of §3 landed at 42, plus the
        // additive `composition.schema_parse` (finding #6) → 43. This pins the
        // count so an accidental drop or duplicate is caught.
        assert_eq!(CODES.len(), 43);
    }
}
