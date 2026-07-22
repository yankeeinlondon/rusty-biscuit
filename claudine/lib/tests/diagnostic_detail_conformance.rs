//! Executable conformance guard: every `Diagnostic::detail()` projection must
//! satisfy the locked registry schema for the code it maps to.
//!
//! The error catalog makes the `detail` payload part of the public, handleable
//! contract: every field a code lists in
//! [`code_spec`](claudine::diagnostics::code_spec) must be a *present* key in
//! `err.detail()`, with an absent OPTIONAL value projecting as JSON `null`
//! (the key still present), not by omitting the shape
//! (`error-catalog.md` §2.5). This test exercises a representative
//! `Diagnostic` value for every code family the three Claudine error surfaces
//! (`ClaudineError`, `CompositionError`, `HarnessError`) emit and asserts the
//! key set covers the registry's declared field set.
//!
//! ## Notes
//!
//! The check is *key coverage*, not value population — an unavailable optional
//! is allowed to be `null`. Required-vs-optional is not encoded in the registry
//! (the field list is flat), so the guard verifies the locked invariant that
//! actually holds across every code: *the key is present*.

use std::path::PathBuf;

use biscuit_terminal::errors::SourceContext;
use claudine::hook_adapters::AdapterError;
use claudine::composition::{CompositionError, MissingProperty};
use claudine::diagnostics::{Diagnostic, code_spec};
use claudine::error::ClaudineError;
use claudine::harness::error::HarnessError;
use claudine::provider::Provider;
use darkmatter::markdown::compose::{ShellCommandOrigin, ShellExpansionError};
use serde_json::Value;

/// Assert that `err.detail()` is a JSON object that carries every field the
/// locked catalog declares for `err.code()`.
fn assert_detail_covers_registry(err: &dyn Diagnostic) {
    let code = err.code();
    let spec = code_spec(code)
        .unwrap_or_else(|| panic!("`{code}` is not a locked catalog code"));

    let detail = err.detail();
    let object = detail.as_object().unwrap_or_else(|| {
        panic!("`{code}` detail() must be a JSON object, got: {detail}")
    });

    for &field in spec.detail {
        assert!(
            object.contains_key(field),
            "`{code}` detail() is missing the registry-declared key `{field}`; \
             present keys: {:?}",
            object.keys().collect::<Vec<_>>()
        );
    }
}

/// One representative `Diagnostic` per `ClaudineError`-emitted code family.
fn claudine_representatives() -> Vec<ClaudineError> {
    vec![
        // provider.unavailable
        ClaudineError::ProviderNotAvailable("codex".to_string()),
        // provider.stream_error
        ClaudineError::Adapter(AdapterError::UnknownEvent("boom".to_string())),
        // io.permission_denied
        ClaudineError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        )),
        // io.read_failed (generic IO)
        ClaudineError::Io(std::io::Error::other("boom")),
        // io.read_failed (system prompt file)
        ClaudineError::SystemPromptFileNotFound("/x/y.md".to_string()),
        // io.read_failed (sqlite)
        ClaudineError::Sqlite(rusqlite::Error::QueryReturnedNoRows),
        // io.write_failed (lock)
        ClaudineError::LockError {
            path: PathBuf::from("/x/lock"),
        },
        // io.write_failed (linking)
        ClaudineError::LinkingError("link failed".to_string()),
        // io.network
        ClaudineError::UrlError(url::ParseError::EmptyHost),
        // config.invalid (string)
        ClaudineError::ConfigValidation("bad field".to_string()),
        // config.invalid (typed parse source)
        ClaudineError::ConfigNotFound(PathBuf::from("/x/config.toml")),
        // config.invalid (policy native parse, carries field)
        ClaudineError::PolicyNativeParse {
            source_id: "src".to_string(),
            message: "bad".to_string(),
            source: None,
        },
        // config.mcp_invalid (server known)
        ClaudineError::McpServerNotFound {
            id: "srv".to_string(),
        },
        // config.mcp_invalid (catalog not found)
        ClaudineError::McpCatalogNotFound,
        // usage.unsupported (mcp)
        ClaudineError::McpProviderNotSupported {
            provider: Provider::Goose,
            reason: "no MCP".to_string(),
        },
        // usage.unsupported (config creation)
        ClaudineError::ConfigCreationNotSupported {
            provider: Provider::Goose,
        },
        // internal.bug (message)
        ClaudineError::TemplateError("boom".to_string()),
        // internal.bug (date range)
        ClaudineError::InvalidReportingDateRange {
            from: "2026-06-02".to_string(),
            to: "2026-06-01".to_string(),
        },
    ]
}

/// One representative `CompositionError` per emitted code family.
fn composition_representatives() -> Vec<CompositionError> {
    vec![
        // composition.invalid_file_reference (resolved not found)
        CompositionError::FileNotFound("missing.md".to_string()),
        // composition.invalid_file_reference (typed source)
        CompositionError::InvalidReference {
            reference: "@bad".to_string(),
            source: biscuit_file::FileReferenceError::InvalidSyntax("@bad".to_string()),
        },
        // composition.schema_load
        CompositionError::SchemaLoad {
            source_path: PathBuf::from("p.md"),
            message: "load failed".to_string(),
        },
        // composition.schema_parse
        CompositionError::SchemaParse {
            source_path: PathBuf::from("p.md"),
            property: Some("spec".to_string()),
            message: "parse failed".to_string(),
            span: None,
        },
        // composition.schema_validation
        CompositionError::SchemaValidation {
            source_path: PathBuf::from("p.md"),
            message: "validation failed".to_string(),
            problems: vec!["/spec".to_string()],
        },
        // composition.missing_properties
        CompositionError::MissingProperties {
            source_path: PathBuf::from("p.md"),
            missing: vec![MissingProperty {
                name: "spec".to_string(),
                type_label: None,
                description: None,
                interactive_shape: None,
            }],
            frontmatter_description: None,
            pointer_paths: vec![],
        },
        // composition.shell_expansion (command-carrying variant)
        CompositionError::ShellExpansionFailed {
            source_path: PathBuf::from("p.md"),
            error: Box::new(ShellExpansionError::Denied {
                ctx: Box::new(SourceContext::new(
                    PathBuf::from("/repo/p.md"),
                    PathBuf::from("p.md"),
                    "---\ntitle: Test\n---\n# Body\n\n::shell \"rm -rf /\"\n",
                )),
                command: "rm -rf /".to_string(),
                origin: ShellCommandOrigin::Body { line: 6 },
            }),
        },
        // io.write_failed (atomic write)
        CompositionError::AtomicWriteFailed {
            path: PathBuf::from("p.md"),
            source: std::io::Error::other("x"),
        },
        // composition.lifecycle_invalid
        CompositionError::LifecycleInvalid {
            property: "success".to_string(),
            message: "bad".to_string(),
            source_file: PathBuf::from("p.md"),
            unknown_field: None,
            expected_fields: vec![],
        },
        // composition.lifecycle_invalid (cardinality: more than one control action).
        // Per-variant representative — these message-less lifecycle variants were
        // previously unwired from code()/detail() and fell through to the
        // catch-all; cover them explicitly, not just one family representative.
        CompositionError::LifecycleMultipleLifecycleActions {
            source_path: PathBuf::from("p.md"),
            property: "start".to_string(),
        },
        // composition.lifecycle_invalid (cardinality: control action not last)
        CompositionError::LifecycleActionOrder {
            source_path: PathBuf::from("p.md"),
            property: "start".to_string(),
        },
        // composition.failed (catch-all)
        CompositionError::NoRunnableProviders,
    ]
}

/// One representative `HarnessError` per emitted code family.
fn harness_representatives() -> Vec<HarnessError> {
    vec![
        // composition.lifecycle_invalid
        HarnessError::InvalidFrontmatter {
            source_path: PathBuf::from("p.md"),
            property: "timeout".to_string(),
            detail: "must be a string".to_string(),
        },
        // composition.shell_expansion
        HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        },
        // io.read_failed
        HarnessError::RepoRootRequired {
            path: "@/x".to_string(),
        },
    ]
}

#[test]
fn claudine_error_detail_covers_registry_schema() {
    for err in claudine_representatives() {
        assert_detail_covers_registry(&err);
    }
}

#[test]
fn composition_error_detail_covers_registry_schema() {
    for err in composition_representatives() {
        assert_detail_covers_registry(&err);
    }
}

#[test]
fn harness_error_detail_covers_registry_schema() {
    for err in harness_representatives() {
        assert_detail_covers_registry(&err);
    }
}

#[test]
fn absent_optional_projects_as_json_null_not_omission() {
    // provider.unavailable declares `provider` AND `path`; the variant only
    // knows the provider, so `path` must be a present `null` key — the exact
    // contract the catalog locks (key present, value null), not an omission.
    let err = ClaudineError::ProviderNotAvailable("codex".to_string());
    let detail = err.detail();
    let object = detail.as_object().expect("detail is an object");
    assert_eq!(object.get("provider"), Some(&Value::String("codex".into())));
    assert_eq!(
        object.get("path"),
        Some(&Value::Null),
        "absent optional `path` must project as a present JSON null, got: {detail}"
    );
}
