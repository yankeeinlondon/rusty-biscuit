//! Argument-type diagnostics for function calls: Darkmatter's design-time
//! call check ([`check_expression_calls`]) shown as editor severities.
//!
//! Darkmatter decides every category; this module only maps a category to a
//! severity under `expressions.strict` (see [`severity`]). It holds no
//! conversion, union, or category rule of its own, so the editor's errors are
//! exactly the calls composition rejects.
//!
//! Every site's [`TypeScope`] is built over the declaration-only layer
//! ([`DeclaredTypes::new`]) of the document's `EffectiveSchema.simplified`:
//! a frontmatter value is a runtime value (setters, caller inputs, and
//! interpolation can replace it), so it never narrows a design-time type.

use darkmatter::markdown::compose::expression::{
    ArgumentCategory, DeclaredTypes, SpannedExpr, TypeScope, check_expression_calls,
};
use darkmatter::markdown::span::SourceSpan;
use lsp_types::DiagnosticSeverity;

use crate::diagnostics::codes::code;
use crate::providers::DocumentContext;

/// One call finding, ready to range and publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallDiagnostic {
    /// The argument's span, or the whole call's for an arity or overload
    /// problem, relative to the checked expression text.
    pub(crate) span: SourceSpan,
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

/// The editor severity of a category: 5 is always an error, 3 always a
/// warning, 2 and 4 a warning only in strict mode, and 1 never shown.
pub(crate) fn severity(category: ArgumentCategory, strict: bool) -> Option<DiagnosticSeverity> {
    match category {
        ArgumentCategory::Incompatible => Some(DiagnosticSeverity::ERROR),
        ArgumentCategory::Uncertain => Some(DiagnosticSeverity::WARNING),
        ArgumentCategory::NullableMatch | ArgumentCategory::Unknown if strict => {
            Some(DiagnosticSeverity::WARNING)
        }
        ArgumentCategory::Compatible | ArgumentCategory::NullableMatch | ArgumentCategory::Unknown => None,
    }
}

/// The declaration-only type layer of `ctx`'s authored schema; without a
/// typed single-shape schema every variable is `unknown`.
pub(crate) fn declared_types<'a>(ctx: &DocumentContext<'a>) -> DeclaredTypes<'a> {
    let schema = ctx
        .overlay
        .and_then(|overlay| overlay.bundle())
        .and_then(|bundle| bundle.effective.simplified.as_ref());
    DeclaredTypes::new(schema)
}

/// Every shown finding of every call in `expr`, in every branch, with
/// each argument typed at its own site under `site`.
pub(crate) fn call_diagnostics(expr: &SpannedExpr, site: &TypeScope<'_>, strict: bool) -> Vec<CallDiagnostic> {
    check_expression_calls(expr, site)
        .iter()
        .flat_map(|check| check.findings())
        .filter_map(|finding| {
            let severity = severity(finding.category, strict)?;
            let code = if finding.category == ArgumentCategory::Incompatible {
                code::EXPRESSION_INCOMPATIBLE_CALL
            } else {
                code::EXPRESSION_UNCERTAIN_CALL
            };
            Some(CallDiagnostic { span: finding.span, severity, code, message: finding.message })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use ArgumentCategory::{Compatible, Incompatible, NullableMatch, Uncertain, Unknown};

    /// Both diagnostics producers over `text`, on this thread, so a
    /// thread-local test function and its entry counter are the ones the
    /// check sees.
    #[cfg(feature = "effects-instrumentation")]
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

    /// Criterion 53, editor side: diagnostics never enter a handler, even for
    /// a test-registered effect in branches a run would skip, while the
    /// proven-bad call beside it is still reported.
    #[cfg(feature = "effects-instrumentation")]
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
        assert_eq!(codes, vec![code::EXPRESSION_INCOMPATIBLE_CALL], "{found:#?}");
    }

    #[test]
    fn the_severity_ladder_follows_the_category_table() {
        let warning = Some(DiagnosticSeverity::WARNING);
        let error = Some(DiagnosticSeverity::ERROR);
        for (category, default, strict) in [
            (Compatible, None, None),
            (NullableMatch, None, warning),
            (Uncertain, warning, warning),
            (Unknown, None, warning),
            (Incompatible, error, error),
        ] {
            assert_eq!(severity(category, false), default, "{category:?} default");
            assert_eq!(severity(category, true), strict, "{category:?} strict");
        }
    }
}
