//! Argument-type diagnostics for function calls: Darkmatter's shared call
//! diagnostics ([`expression_call_diagnostics`]) shown in the editor.
//!
//! Darkmatter decides every code, severity, message, and expected/actual
//! type ([`ExpressionDiagnostic`]); this module only ranges a record and maps
//! its severity to the protocol's. It holds no conversion, union, category,
//! or severity rule of its own, so the editor's errors are exactly the calls
//! composition rejects, reported with the same fields. The expected and
//! actual types travel in the LSP diagnostic's `data` as
//! `{"expected_type": …, "actual_type": …}`, `null` when unavailable.
//!
//! Every site's [`TypeScope`] is built over the declaration-only layer
//! ([`DeclaredTypes::for_schema`]) of the document's effective schema: a
//! frontmatter value is a runtime value (setters, caller inputs, and
//! interpolation can replace it), so it never narrows a design-time type.
//! Guards narrow it through the library alone: body `::block when`
//! conditions through `BodyGuards`, and lifecycle `when` items through
//! [`darkmatter::lifecycle::lifecycle_call_checks`].

use darkmatter::markdown::compose::expression::{
    CallCheck, DeclaredTypes, ExpressionDiagnostic, Severity, SpannedExpr, TypeScope, expression_call_diagnostics,
    incomplete_host_calls,
};
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};

use crate::providers::DocumentContext;

/// One shared call finding, ranged in the checked expression's text.
pub(crate) type CallDiagnostic = ExpressionDiagnostic;

/// The protocol severity of a library severity.
pub(crate) fn lsp_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    }
}

/// `found` as a protocol diagnostic at `range`, under `source`.
pub(crate) fn lsp_diagnostic(range: Range, source: &str, found: CallDiagnostic) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(lsp_severity(found.severity)),
        code: Some(NumberOrString::String(found.code.to_string())),
        source: Some(source.to_string()),
        message: found.message,
        data: Some(serde_json::json!({
            "expected_type": found.expected_type,
            "actual_type": found.actual_type,
        })),
        ..Default::default()
    }
}

/// The declaration-only type layer of every layer of `ctx`'s effective
/// schema ([`DeclaredTypes::for_schema`]); without one every variable is
/// `unknown`.
pub(crate) fn declared_types<'a>(ctx: &DocumentContext<'a>) -> DeclaredTypes<'a> {
    DeclaredTypes::for_schema(ctx.overlay.and_then(|overlay| overlay.bundle()).map(|bundle| &bundle.effective))
}

/// Every shown finding of every call in `expr`, in every branch, with each
/// argument typed at its own site under `site`, including the
/// incomplete-analysis warnings of [`incomplete_diagnostics`].
pub(crate) fn call_diagnostics(expr: &SpannedExpr, site: &TypeScope<'_>, strict: bool) -> Vec<CallDiagnostic> {
    expression_call_diagnostics(expr, site, strict)
}

/// One warning, under both strictness settings, per call in `expr` to a host
/// function whose analysis is incomplete in the entered host functions
/// ([`crate::providers::dsl::enter_host_functions`]), ranged on the whole
/// call. Such a call gets no category finding.
pub(crate) fn incomplete_diagnostics(expr: &SpannedExpr) -> Vec<CallDiagnostic> {
    incomplete_host_calls(expr).iter().map(|call| call.diagnostic()).collect()
}

/// The shown findings of checks Darkmatter already made, such as the
/// lifecycle surfaces' ([`darkmatter::lifecycle::lifecycle_call_checks`]).
pub(crate) fn check_diagnostics(checks: &[CallCheck], strict: bool) -> Vec<CallDiagnostic> {
    checks.iter().flat_map(|check| check.diagnostics(strict)).collect()
}

#[cfg(all(test, feature = "effects-instrumentation"))]
mod tests {
    use super::*;

    /// Both diagnostics producers over `text`, on this thread, so a
    /// thread-local test function and its entry counter are the ones the
    /// check sees.
    fn diagnostics(text: &str) -> Vec<lsp_types::Diagnostic> {
        use std::collections::BTreeMap;
        use std::path::{Path, PathBuf};
        use std::sync::Arc;

        use lsp_types::{InitializeParams, Uri};

        use crate::capabilities::ClientProfile;
        use crate::config::DmlsConfig;
        use crate::graph::WorkspaceGraph;
        use crate::overlay::OverlayState;
        use crate::source_map::{PositionEncoding, SourceMap};

        let path = Path::new("/w/doc.md");
        let uri: Uri = "file:///w/doc.md".parse().unwrap();
        let config = DmlsConfig::default();
        let roots = [PathBuf::from("/w")];
        let resolution = crate::context::test_support::resolution_for(path);
        let state = OverlayState::default();
        let overlay = state.for_document(&uri, text, path, &config, &roots, &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        let mut out = crate::diagnostics::frontmatter::diagnostics(&ctx);
        out.extend(crate::providers::dsl::diagnostics(&ctx));
        out
    }

    #[test]
    fn callbacks_reject_protected_patterns_and_nested_calls_without_execution() {
        let found = diagnostics("---\ntitle: ok\n---\n{{ map([], local => local) }}\n{{ map([], x => min(true, 5)) }}\n");
        let codes: Vec<_> = found.iter().filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) => Some(code.as_str()),
            _ => None,
        }).collect();
        assert!(codes.contains(&"dm.expression.invalid_callback"), "{found:#?}");
        assert!(codes.contains(&"dm.expression.incompatible_call"), "{found:#?}");
    }

    #[test]
    fn callback_analysis_never_runs_provider_handlers() {
        use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
        use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
        let registration = FunctionRegistration::builder("callback_effect", "function(parameters(void); returns(boolean))").build().unwrap();
        let _effect = hooks::register(registration, std::sync::Arc::new(|_| panic!("passive callback executed a provider")));
        hooks::reset_handler_entries();
        let found = diagnostics("{{ map([1], x => callback_effect()) }}\n{{ filter(null, x -> callback_effect()) }}\n");
        assert!(found.iter().all(|diagnostic| diagnostic.severity != Some(DiagnosticSeverity::ERROR)), "{found:#?}");
        assert_eq!(hooks::handler_entries("callback_effect"), 0);
    }

    /// Criterion 53, editor side: diagnostics never enter a handler, even for
    /// a test-registered effect in branches a run would skip, while the
    /// proven-bad call beside it is still reported.
    #[test]
    fn diagnostics_never_enter_a_handler() {
        use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
        use darkmatter::markdown::compose::expression::function_test_hooks as hooks;

        let registration = FunctionRegistration::builder("effect", "function(parameters(void); returns(boolean))")
            .build()
            .expect("the test signature compiles");
        let _effect = hooks::register(registration, std::sync::Arc::new(|_| Ok(serde_json::json!(true))));
        let text = concat!(
            "---\n",
            "$schema:\n  flag: boolean(required)\n",
            "flag: true\n",
            "title: \"{{ false ? effect() : 0 }}\"\n",
            "---\n\n",
            "{{ and(false, effect()) }} {{ or(true, effect(), min(flag, 5)) }}\n",
            "::block when=\"false && effect()\"\n{{ effect() }}\n::end-block\n",
        );
        hooks::reset_handler_entries();
        let found = diagnostics(text);
        assert_eq!(hooks::handler_entries("effect"), 0);
        assert_eq!(hooks::handler_entries("min"), 0);
        let codes: Vec<_> = found
            .iter()
            .filter_map(|diagnostic| match &diagnostic.code {
                Some(lsp_types::NumberOrString::String(code)) if code.ends_with("_call") => Some(code.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(codes, vec![crate::diagnostics::codes::code::EXPRESSION_INCOMPATIBLE_CALL], "{found:#?}");
    }
}
