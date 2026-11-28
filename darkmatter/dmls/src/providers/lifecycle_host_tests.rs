//! One combined diagnostics pass reads the lifecycle host from the document's
//! selected host view and shares it with the frontmatter provider; a panic
//! while reading it drops only that provider's diagnostics.
//!
//! The host comes from an explicitly registered format 3 bundle
//! ([`crate::host_fixtures`]); a lifecycle artifact beside a catalog in a schema
//! root supplies none.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use lsp_types::{Diagnostic, InitializeParams, NumberOrString, Uri};

use super::*;
use crate::config::DmlsConfig;
use crate::context::DocumentResolution;
use crate::diagnostics::codes::code;
use crate::overlay::OverlayState;
use crate::source_map::PositionEncoding;
use crate::workspace::file_path_to_uri;

/// A lifecycle stack in Darkmatter's `finalize` event: under strict
/// expressions the call after the guard is narrowed and the one after the
/// write warns. Its `widget` matches the `acme` host's trigger.
const DOCUMENT: &str = concat!(
    "---\n",
    "$schema:\n  count: number\n",
    "count: 1\n",
    "widget:\n  size: 2\n",
    "finalize:\n",
    "  - when: count\n",
    "    then:\n",
    "      - stdout: \"{{ min(count, 5) }}\"\n",
    "      - set:\n",
    "          count: 7\n",
    "      - stdout: \"{{ min(count, 6) }}\"\n",
    "---\n",
    "Body\n",
);

/// A temporary home and working directory with the `acme` bundle installed
/// under `host/`, registered by the user configuration when `registered`.
struct Workspace {
    fixture: crate::host_fixtures::Fixture,
}

impl Workspace {
    fn new(registered: bool) -> Self {
        let fixture = crate::host_fixtures::Fixture::new();
        if registered {
            fixture.write("home/.darkmatter.yaml", &format!("register:\n  - ../host/{}\n", crate::host_fixtures::MANIFEST_NAME));
        } else {
            fixture.write("home/.darkmatter.yaml", "{}\n");
        }
        Self { fixture }
    }

    /// Runs `f` with a strict-expressions context for [`DOCUMENT`] at
    /// `work/doc.md`, its overlay prepared (registrations included).
    fn with_ctx<R>(&self, f: impl FnOnce(&DocumentContext) -> R) -> R {
        let path = self.fixture.path("work/doc.md");
        let uri: Uri = file_path_to_uri(&path).expect("document uri");
        let mut config = DmlsConfig::default();
        config.expressions.strict = true;
        let roots: [PathBuf; 1] = [self.fixture.path("work")];
        let resolution = DocumentResolution::from_context(self.fixture.context().for_source(&path));
        let overlay = OverlayState::default().for_document(&uri, DOCUMENT, &path, &config, &roots, &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(DOCUMENT));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path: &path,
            text: DOCUMENT,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
            host: None,
        };
        f(&ctx)
    }
}

fn with_code<'d>(diagnostics: &'d [Diagnostic], wanted: &str) -> Vec<&'d Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| matches!(&diagnostic.code, Some(NumberOrString::String(found)) if found == wanted))
        .collect()
}

/// The 0-based lines of `wanted` diagnostics.
fn lines_of(diagnostics: &[Diagnostic], wanted: &str) -> Vec<u32> {
    with_code(diagnostics, wanted).into_iter().map(|diagnostic| diagnostic.range.start.line).collect()
}

fn line_of(needle: &str) -> u32 {
    let offset = DOCUMENT.find(needle).expect("needle in document");
    u32::try_from(DOCUMENT[..offset].matches('\n').count()).expect("line fits")
}

/// What each provider of the default registry reports on its own, in
/// registry order.
fn standalone(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let mut out = SubstrateProvider.diagnostics(ctx);
    out.extend(WikiProvider.diagnostics(ctx));
    out.extend(crate::diagnostics::frontmatter::diagnostics(ctx));
    out.extend(dsl::diagnostics(ctx));
    out
}

/// A document the registered `acme` host identifies is analyzed under the
/// host's lifecycle, and one combined pass reports exactly what the
/// providers report on their own: the host's `finalize` narrows the guarded
/// call and the call after the write warns.
#[test]
fn a_registered_lifecycle_host_serves_a_combined_pass() {
    let workspace = Workspace::new(true);
    let registry = ProviderRegistry::with_substrate();

    workspace.with_ctx(|ctx| {
        assert_eq!(ctx.host_view().lifecycle().registry().host(), "acme", "{:?}", ctx.host_view());
        let combined = registry.diagnostics(ctx);
        assert_eq!(
            lines_of(&combined, code::EXPRESSION_UNCERTAIN_CALL),
            [line_of("min(count, 6)")],
            "the host's `finalize` narrows the guarded call: {combined:#?}"
        );
        assert_eq!(combined, standalone(ctx), "the shared host changes no provider's result");
    });
}

/// A catalog with a lifecycle artifact beside it in the document's schema
/// root, which a host installed before registration, supplies no host: the
/// document is analyzed under `md`, and the pass still agrees with the
/// providers on their own.
#[test]
fn a_root_local_catalog_and_artifact_supply_no_lifecycle_host() {
    let workspace = Workspace::new(false);
    workspace.fixture.root_catalog("work/schemas", "acme");
    let registry = ProviderRegistry::with_substrate();

    workspace.with_ctx(|ctx| {
        assert_eq!(ctx.host_view().lifecycle().registry().host(), "md", "{:?}", ctx.host_view());
        assert!(ctx.host_view().catalogs().is_empty());
        let combined = registry.diagnostics(ctx);
        assert_eq!(lines_of(&combined, code::EXPRESSION_UNCERTAIN_CALL), [line_of("min(count, 6)")], "{combined:#?}");
        assert_eq!(combined, standalone(ctx));
    });
}

/// A provider that reports one fixed diagnostic and never reads the host.
struct Fixed(&'static str);

impl Provider for Fixed {
    fn name(&self) -> &'static str {
        self.0
    }

    fn diagnostics(&self, _: &DocumentContext) -> Vec<Diagnostic> {
        vec![Diagnostic { message: self.0.to_string(), ..Diagnostic::default() }]
    }
}

/// A panic while reading the pass's host is logged as a panic of the
/// frontmatter provider and drops only its diagnostics; the providers around
/// it, the DSL provider included, still report, in order.
#[test]
#[tracing_test::traced_test]
fn a_lifecycle_host_panic_drops_only_the_frontmatter_diagnostics() {
    let workspace = Workspace::new(true);
    let registry = ProviderRegistry {
        providers: vec![Box::new(Fixed("before")), Box::new(FrontmatterProvider), Box::new(DslProvider), Box::new(Fixed("after"))],
    };

    workspace.with_ctx(|ctx| {
        assert!(!crate::diagnostics::frontmatter::diagnostics(ctx).is_empty());
        let dsl = dsl::diagnostics(ctx);
        let combined = registry.diagnostics_discovering(ctx, |_| panic!("injected lifecycle host panic"));
        let messages: Vec<&str> = combined.iter().map(|diagnostic| diagnostic.message.as_str()).collect();
        let expected: Vec<&str> =
            std::iter::once("before").chain(dsl.iter().map(|d| d.message.as_str())).chain(["after"]).collect();
        assert_eq!(messages, expected);
    });

    logs_assert(|lines: &[&str]| {
        let panics: Vec<&&str> = lines.iter().filter(|line| line.contains("provider panicked")).collect();
        let logged = |name: &str| {
            panics.iter().any(|line| {
                line.contains("ERROR") && line.contains(&format!("provider=\"{name}\"")) && line.contains("capability=\"diagnostics\"")
            })
        };
        match panics.len() {
            1 if logged("frontmatter") => Ok(()),
            _ => Err(format!("{panics:#?}")),
        }
    });
}
