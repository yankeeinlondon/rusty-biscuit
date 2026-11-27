//! The single-source registry of locked diagnostic codes.
//!
//! This is the ratified code catalog from
//! `claudine/features/_completed/2026-06-28-real-errors/error-catalog.md` §3,
//! encoded as data: every dotted `code`, its
//! [`Category`]/[`Disposition`]/[`Origin`] facets, an optional severity
//! override, and the field names its `detail` payload carries. It is the contract that `claudine errors` introspects and
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
    /// Whether a check of the passive dry-run report (`claudine … --dry-run`)
    /// reports this code. Each such code has a fixture pair (a finding and a
    /// control) and plain and JSON report snapshots, which the CLI's
    /// `dry_run_check::codes` guard requires of every `true` entry. A code
    /// the report only projects from an error that stopped a check (most of
    /// the catalog, and the `internal.bug` fallback no authored input
    /// reaches) is `false`.
    pub dry_run_report: bool,
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
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "reason"],
    },
    CodeSpec {
        code: "auth.permission",
        category: Category::Auth,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "action"],
    },
    // --- cap — usage / account limits ---
    CodeSpec {
        code: "cap.rate_limit",
        category: Category::Cap,
        disposition: Disposition::Throttled,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "reset_at", "retry_after_ms", "scope"],
    },
    CodeSpec {
        code: "cap.plan_limit",
        category: Category::Cap,
        disposition: Disposition::Throttled,
        origin: Origin::Provider,
        dry_run_report: false,
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
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "reason"],
    },
    // --- timeout ---
    CodeSpec {
        code: "timeout.step_silence",
        category: Category::Timeout,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["elapsed_ms", "limit_ms"],
    },
    CodeSpec {
        code: "timeout.wall_clock",
        category: Category::Timeout,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["elapsed_ms", "limit_ms"],
    },
    // --- provider — the agent run (platform + behavior) ---
    CodeSpec {
        code: "provider.unavailable",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "path"],
    },
    CodeSpec {
        code: "provider.launch_failed",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider"],
    },
    CodeSpec {
        code: "provider.exited",
        category: Category::Provider,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "exit_code", "signal"],
    },
    CodeSpec {
        code: "provider.interrupted",
        category: Category::Provider,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        detail: &["signal"],
    },
    CodeSpec {
        code: "provider.stream_error",
        category: Category::Provider,
        disposition: Disposition::Transient,
        origin: Origin::Provider,
        dry_run_report: false,
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
        dry_run_report: false,
        severity_override: Some(Severity::Warning),
        detail: &["provider"],
    },
    CodeSpec {
        code: "provider.incomplete_subagents",
        category: Category::Provider,
        // Fail-fast by ratified contract: the provider abandoned sub-agent work
        // and then exited cleanly, so nothing generic resolves it. `transient`
        // would invite a retry handler to loop a run that will abandon the same
        // work again, and `correctable` would promise an action the author or
        // operator does not have. Only an explicit `failure` stack recovers it.
        disposition: Disposition::Unrecoverable,
        // The agent run abandoned its own work — not the author's document, not
        // the host, not a Claudine bug.
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        // The synthesized-label path knows the code but no per-instance values,
        // so both project `null` today; the complete per-task list lives in
        // `session_end.extra.subagent_outcomes` regardless.
        detail: &["provider", "incomplete_count"],
    },
    CodeSpec {
        code: "provider.overlay_unsupported",
        category: Category::Provider,
        // Nothing generic resolves it: the provider has no verified mechanism
        // for the requested reason, so a retry would refuse identically. Only
        // dropping the mode (`--repo`, `--mcp`) changes the outcome, and that
        // is the caller's decision, not a handler's.
        disposition: Disposition::Unrecoverable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        // `selector` is a variable *name*. Its value is never projected —
        // shadow-home spec Invariant 8.
        detail: &["provider", "reason", "selector", "next_action"],
    },
    CodeSpec {
        code: "provider.overlay_failed",
        category: Category::Provider,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["provider", "reason", "stage", "message"],
    },
    // --- composition — author's prompt document (Darkmatter) ---
    CodeSpec {
        code: "composition.invalid_file_reference",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
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
        // authored reference kind; `effective_kind` names the anchoring after
        // interpolation. The `CompositionError` wrapper supplies
        // `source_path`/`property`/`event`; the shared `biscuit-file`/harness
        // resolver supplies `failure` on both arms and additionally projects
        // `kind`/`effective_kind`/`repository_root`/`candidates` when a probe
        // ran. The pre-probe form of `FileReferenceUnresolvable` and the legacy
        // `FileReferenceDiagnostic` path leave those fields (and `failure` on
        // the legacy path) as `null`. Values come from typed data, never
        // back-derived from `Display` (spec §D3).
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
            "effective_kind",
        ],
    },
    CodeSpec {
        code: "composition.unknown_function",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["name", "suggestions"],
    },
    CodeSpec {
        code: "composition.expression_invalid",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["expression", "message"],
    },
    CodeSpec {
        code: "composition.schema_load",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "message"],
    },
    CodeSpec {
        code: "composition.schema_slot_failed",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "slot_code", "message"],
    },
    CodeSpec {
        code: "composition.schema_parse",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "property", "message"],
    },
    CodeSpec {
        code: "composition.schema_validation",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "problems", "pointer_paths"],
    },
    CodeSpec {
        code: "composition.missing_properties",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &["missing", "pointer_paths"],
    },
    CodeSpec {
        code: "composition.frontmatter_parse",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path"],
    },
    CodeSpec {
        code: "composition.lifecycle_invalid",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["property", "message"],
    },
    CodeSpec {
        code: "composition.shell_expansion",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["command"],
    },
    CodeSpec {
        code: "composition.shell_approval",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        // A shell command the author wrote could not be approved. Every reason
        // — the user declined, the catalog blacklists it, no handler was
        // available, `--dry-run` cannot prompt — resolves the same way: change
        // the document or the approval configuration. That shared remediation
        // is what keeps one code's `disposition` stable while `reason`
        // discriminates (error-catalog §7.5).
        detail: &["command", "source_path", "line", "property", "reason"],
    },
    CodeSpec {
        code: "composition.body_unchanged",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        // The agent was asked to write the document and did not, so the run's
        // producer is what has to change — not the author's document.
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "reason"],
    },
    CodeSpec {
        code: "composition.completion_schema",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        // `properties` is the ordered per-property failure list — one object
        // per problem carrying `property`, `message`, and `kind` — so a
        // `failure.stack` can branch on which property was not satisfied
        // without parsing the rendered status block.
        detail: &["source_path", "properties"],
    },
    CodeSpec {
        code: "composition.cancelled",
        category: Category::Composition,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: Some(Severity::Warning),
        detail: &["group", "leg", "cause"],
    },
    CodeSpec {
        // A lifecycle `edit` could not open its file: no usable editor, no
        // terminal or display to lend it, a refused or unusable target, or
        // the editor itself failed. `reason` is the typed cause's text.
        code: "composition.lifecycle_edit_failed",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path", "property", "target", "reason"],
    },
    // Reported by the passive dry-run check (`claudine … --dry-run`); a real
    // run reports the same conditions under the error that stops it.
    // A step-level `agent:`/`model:` a task reads as data.
    CodeSpec {
        code: "composition.selection_key_as_data",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A task with no executable over a document with no body.
    CodeSpec {
        code: "composition.no_executable",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A task naming more than one executable.
    CodeSpec {
        code: "composition.multiple_executables",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A `sequence` value or item of the wrong shape.
    CodeSpec {
        code: "composition.sequence_shape",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // The file run as a sequence declares no `sequence` list.
    CodeSpec {
        code: "composition.sequence_undefined",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // No source of the selection chain chose a provider.
    CodeSpec {
        code: "composition.provider_unresolved",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A provider name no provider answers to.
    CodeSpec {
        code: "composition.provider_unknown",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A chosen provider whose binary is not on `PATH`.
    CodeSpec {
        code: "composition.provider_not_installed",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A provider that needs a model for this launch, with none found.
    CodeSpec {
        code: "composition.model_required",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // Command-line positionals that do not parse.
    CodeSpec {
        code: "composition.invalid_arguments",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        dry_run_report: true,
        severity_override: None,
        detail: &[],
    },
    // A bare-name document or a supplied `file` input that names no file
    // but matches files a real run on this terminal would ask the caller to
    // choose from.
    CodeSpec {
        code: "composition.file_choice_pending",
        category: Category::Composition,
        disposition: Disposition::NeedsInput,
        origin: Origin::Caller,
        dry_run_report: true,
        severity_override: Some(Severity::Warning),
        detail: &[],
    },
    CodeSpec {
        code: "composition.failed",
        category: Category::Composition,
        disposition: Disposition::Correctable,
        origin: Origin::Author,
        dry_run_report: false,
        severity_override: None,
        detail: &["source_path"],
    },
    // --- document — agent's output postconditions ---
    CodeSpec {
        code: "document.missing_frontmatter",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["doc", "property"],
    },
    CodeSpec {
        code: "document.invalid_frontmatter",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["doc", "property", "problems"],
    },
    CodeSpec {
        code: "document.empty",
        category: Category::Document,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["doc"],
    },
    // --- vcs — git-state postconditions ---
    CodeSpec {
        code: "vcs.unexpected_dirty_files",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["scope", "files"],
    },
    CodeSpec {
        code: "vcs.unexpected_commits",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["commits"],
    },
    CodeSpec {
        code: "vcs.missing_dirty_files",
        category: Category::Vcs,
        disposition: Disposition::Correctable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["scope"],
    },
    // --- io — filesystem / network plumbing ---
    CodeSpec {
        code: "io.read_failed",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.write_failed",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.permission_denied",
        category: Category::Io,
        disposition: Disposition::Correctable,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["path"],
    },
    CodeSpec {
        code: "io.network",
        category: Category::Io,
        disposition: Disposition::Transient,
        origin: Origin::Environment,
        dry_run_report: false,
        severity_override: None,
        detail: &["url", "message"],
    },
    // --- config — Claudine / user configuration ---
    CodeSpec {
        code: "config.invalid",
        category: Category::Config,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        detail: &["field", "message"],
    },
    CodeSpec {
        code: "config.mcp_invalid",
        category: Category::Config,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        detail: &["server", "message"],
    },
    // --- usage — Rust API misuse ---
    CodeSpec {
        code: "usage.invalid_argument",
        category: Category::Usage,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        detail: &["argument", "expected"],
    },
    CodeSpec {
        code: "usage.unsupported",
        category: Category::Usage,
        disposition: Disposition::Correctable,
        origin: Origin::Caller,
        dry_run_report: false,
        severity_override: None,
        detail: &["operation", "provider"],
    },
    // --- runaway — content-guard trips (always unrecoverable) ---
    CodeSpec {
        code: "runaway.volume",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["measure", "cap"],
    },
    CodeSpec {
        code: "runaway.repetition",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["cycles"],
    },
    CodeSpec {
        code: "runaway.exit_expression",
        category: Category::Runaway,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Provider,
        dry_run_report: false,
        severity_override: None,
        detail: &["expression"],
    },
    // --- internal — our bugs ---
    CodeSpec {
        code: "internal.bug",
        category: Category::Internal,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Internal,
        dry_run_report: false,
        severity_override: None,
        detail: &["message"],
    },
    CodeSpec {
        code: "internal.serialization",
        category: Category::Internal,
        disposition: Disposition::Unrecoverable,
        origin: Origin::Internal,
        dry_run_report: false,
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
            "effective_kind",
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
    fn incomplete_subagents_is_unrecoverable_provider_work() {
        // The ratified fail-fast contract: a run whose sub-agent work was
        // abandoned must never be picked up by a generic retry handler.
        let spec = code_spec("provider.incomplete_subagents").unwrap();
        assert_eq!(spec.category, Category::Provider);
        assert_eq!(spec.disposition, Disposition::Unrecoverable);
        assert_eq!(spec.origin, Origin::Provider);
        assert_eq!(spec.severity(), Severity::Error);
        assert!(spec.detail.contains(&"incomplete_count"));
    }

    #[test]
    fn interrupted_origin_is_caller() {
        // error-catalog §7.7: the human pressed Ctrl-C — the caller.
        assert_eq!(code_spec("provider.interrupted").unwrap().origin, Origin::Caller);
    }

    #[test]
    fn dry_run_report_codes_are_the_checks_own_and_the_reused_missing_input() {
        let scoped: Vec<&str> = CODES.iter().filter(|spec| spec.dry_run_report).map(|spec| spec.code).collect();
        assert!(scoped.iter().all(|code| code.starts_with("composition.")), "{scoped:?}");
        assert!(scoped.contains(&"composition.missing_properties"), "{scoped:?}");
        // A launch-only refusal: the dry run reports a slot by its own code.
        assert!(!code_spec("composition.schema_slot_failed").unwrap().dry_run_report);
        assert!(!code_spec("internal.bug").unwrap().dry_run_report);
        assert_eq!(scoped.len(), 12, "{scoped:?}");
    }

    #[test]
    fn catalog_covers_the_ratified_count() {
        // 12 categories; the faithful transcription of §3 landed at 42, plus the
        // additive `composition.schema_parse` (finding #6) → 43, plus the
        // additive `composition.shell_approval` (error-propagation §D-14) → 44,
        // plus the additive `provider.incomplete_subagents`
        // (silent-success-and-startup-stall §4) → 45, plus the completion
        // verdict's `composition.body_unchanged` and
        // `composition.completion_schema` (inline-flow-and-validations §D5) → 47,
        // plus the pre-spawn provider-overlay pair
        // `provider.overlay_unsupported` / `provider.overlay_failed`
        // (shadow-home fix → Fail closed before spawn) → 49, plus structured
        // sibling cancellation (`composition.cancelled`) → 50, plus the
        // lifecycle `edit` action's `composition.lifecycle_edit_failed` → 51,
        // plus the passive dry-run check's nine codes → 60, plus the launch
        // check's `composition.schema_slot_failed` → 61, plus the dry run's
        // pending file choice (`composition.file_choice_pending`) → 62, plus
        // a sequence run of a file with no `sequence`
        // (`composition.sequence_undefined`) → 63.
        // This pins the count so an accidental drop or duplicate is caught.
        assert_eq!(CODES.len(), 63);
    }
}
