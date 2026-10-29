//! One combined diagnostics pass discovers the lifecycle host once and shares
//! it with the frontmatter and DSL providers; a panic in that discovery drops
//! only those two providers' diagnostics.
//!
//! Discovery is counted by the reads of a host catalog that
//! [`DiscoveryProbe`] observes on this thread: every discovery reads each
//! visited catalog once, cached or not, and nothing else in a pass reads
//! through the probe.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::lifecycle::host_profile::{lifecycle_artifact, lifecycle_artifact_path};
use darkmatter::lifecycle::registry::EventRegistry;
use darkmatter::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy};
use darkmatter::markdown::schemas::catalog::discovery_probe::DiscoveryProbe;
use lsp_types::{Diagnostic, DiagnosticSeverity, InitializeParams, NumberOrString, Range, Uri};

use super::*;
use crate::config::DmlsConfig;
use crate::diagnostics::codes::code;
use crate::overlay::OverlayState;
use crate::source_map::PositionEncoding;
use crate::workspace::file_path_to_uri;

/// A lifecycle stack in Darkmatter's `finalize` event: under strict
/// expressions the call after the guard is narrowed and the one after the
/// write warns.
const DOCUMENT: &str = concat!(
    "---\n",
    "$schema:\n  count: number\n",
    "count: 1\n",
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

/// A repository whose `schemas/` root holds the host catalogs.
struct Repository {
    _dir: tempfile::TempDir,
    root: PathBuf,
    schemas: PathBuf,
}

impl Repository {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let schemas = root.join("schemas");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(&schemas).expect("schema root");
        Self { _dir: dir, root, schemas }
    }

    /// A host catalog named `name` declaring the `run` global, without a
    /// lifecycle artifact.
    fn host_catalog(&self, name: &str) -> PathBuf {
        let path = self.schemas.join(name);
        std::fs::write(&path, catalog_text(name)).expect("catalog");
        path
    }

    /// Writes the artifact of the `test` host, which declares no events,
    /// beside `catalog`.
    fn write_artifact(&self, catalog: &Path) {
        let name = catalog.file_name().expect("file name").to_string_lossy();
        let options = LifecycleParseOptions {
            is_known_effect: &|_| false,
            authored: None,
            unknown_verbs: UnknownVerbPolicy::Reject,
            registry: &EventRegistry::new("test"),
        };
        std::fs::write(lifecycle_artifact_path(catalog), lifecycle_artifact(&options, &[], &catalog_text(&name)))
            .expect("artifact");
    }

    /// Runs `f` with a strict-expressions context for [`DOCUMENT`] in the
    /// repository.
    fn with_ctx<R>(&self, f: impl FnOnce(&DocumentContext) -> R) -> R {
        let path = self.root.join("doc.md");
        let uri: Uri = file_path_to_uri(&path).expect("document uri");
        let mut config = DmlsConfig::default();
        config.expressions.strict = true;
        let roots = [self.root.clone()];
        let resolution = crate::context::test_support::resolution_in_repository(&self.root, &path);
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
        };
        f(&ctx)
    }
}

fn catalog_text(name: &str) -> String {
    format!("kind: schema\ndescription: {name}\n$schema:\n    run:\n        count: number\n")
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
/// registry order, each discovering for itself.
fn standalone(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let mut out = SubstrateProvider.diagnostics(ctx);
    out.extend(WikiProvider.diagnostics(ctx));
    out.extend(crate::diagnostics::frontmatter::diagnostics(ctx));
    out.extend(dsl::diagnostics(ctx));
    out
}

/// One pass under a catalog with no usable artifact discovers once and
/// reports exactly what the providers report on their own: the incomplete
/// warning, and the `md` host's narrowing of the `finalize` stack. With the
/// artifact repaired, the next pass discovers afresh and the warning is gone.
#[test]
fn one_lifecycle_discovery_serves_a_combined_pass_and_a_repair_shows_next_request() {
    let repository = Repository::new();
    let catalog = repository.host_catalog("host.yaml");
    let registry = ProviderRegistry::with_substrate();
    let catalog_reads = |probe: &DiscoveryProbe| probe.take_reads().iter().filter(|read| **read == catalog).count();

    repository.with_ctx(|ctx| {
        let probe = DiscoveryProbe::install();
        let combined = registry.diagnostics(ctx);
        assert_eq!(catalog_reads(&probe), 1, "one discovery per combined pass");

        let incomplete = with_code(&combined, code::LIFECYCLE_INCOMPLETE_ANALYSIS);
        assert_eq!(incomplete.len(), 1, "{combined:#?}");
        assert_eq!(incomplete[0].range, Range::default());
        assert_eq!(incomplete[0].severity, Some(DiagnosticSeverity::WARNING));
        let artifact = lifecycle_artifact_path(&catalog);
        assert!(incomplete[0].message.contains(&artifact.display().to_string()), "{}", incomplete[0].message);
        assert_eq!(
            lines_of(&combined, code::EXPRESSION_UNCERTAIN_CALL),
            [line_of("min(count, 6)")],
            "the md host narrows the guarded call: {combined:#?}"
        );
        assert_eq!(combined, standalone(ctx), "the shared host changes no provider's result");
        assert_eq!(catalog_reads(&probe), 2, "standalone frontmatter and DSL each discover");
    });

    repository.write_artifact(&catalog);
    repository.with_ctx(|ctx| {
        let probe = DiscoveryProbe::install();
        let combined = registry.diagnostics(ctx);
        assert_eq!(catalog_reads(&probe), 1, "one discovery per combined pass");
        assert!(with_code(&combined, code::LIFECYCLE_INCOMPLETE_ANALYSIS).is_empty(), "{combined:#?}");
        assert_eq!(
            lines_of(&combined, code::EXPRESSION_UNCERTAIN_CALL),
            [line_of("min(count, 6)")],
            "the `test` host's `finalize` narrows the guarded call: {combined:#?}"
        );
        assert_eq!(combined, standalone(ctx));
    });
}

/// A broken host catalog is reported by the DSL provider's own catalog
/// check in a combined pass, beside a usable host found once.
#[test]
fn a_broken_host_catalog_is_still_reported_in_a_combined_lifecycle_pass() {
    let repository = Repository::new();
    std::fs::write(
        repository.schemas.join("broken.yaml"),
        "kind: schema\ndescription: broken\n$schema:\n    seq: 42\n",
    )
    .expect("broken catalog");
    let catalog = repository.host_catalog("host.yaml");
    repository.write_artifact(&catalog);
    let registry = ProviderRegistry::with_substrate();

    repository.with_ctx(|ctx| {
        let probe = DiscoveryProbe::install();
        let combined = registry.diagnostics(ctx);
        assert_eq!(probe.take_reads().iter().filter(|read| **read == catalog).count(), 1);
        let reported = with_code(&combined, code::SCHEMA_HOST_CATALOG);
        assert_eq!(reported.len(), 1, "{combined:#?}");
        assert!(reported[0].message.contains("broken.yaml"), "{}", reported[0].message);
        assert!(with_code(&combined, code::LIFECYCLE_INCOMPLETE_ANALYSIS).is_empty(), "{combined:#?}");
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

/// A panic while discovering the pass's host is logged as a panic of the
/// frontmatter and DSL providers and drops only their diagnostics; the
/// providers around them still report, in order.
#[test]
#[tracing_test::traced_test]
fn a_lifecycle_discovery_panic_drops_only_the_frontmatter_and_dsl_diagnostics() {
    let repository = Repository::new();
    repository.host_catalog("host.yaml");
    let registry = ProviderRegistry {
        providers: vec![Box::new(Fixed("before")), Box::new(FrontmatterProvider), Box::new(DslProvider), Box::new(Fixed("after"))],
    };

    repository.with_ctx(|ctx| {
        assert!(!crate::diagnostics::frontmatter::diagnostics(ctx).is_empty() && !dsl::diagnostics(ctx).is_empty());
        let combined = registry.diagnostics_discovering(ctx, |_| panic!("injected lifecycle discovery panic"));
        let messages: Vec<&str> = combined.iter().map(|diagnostic| diagnostic.message.as_str()).collect();
        assert_eq!(messages, ["before", "after"]);
    });

    logs_assert(|lines: &[&str]| {
        let panics: Vec<&&str> = lines.iter().filter(|line| line.contains("provider panicked")).collect();
        let logged = |name: &str| {
            panics.iter().any(|line| {
                line.contains("ERROR") && line.contains(&format!("provider=\"{name}\"")) && line.contains("capability=\"diagnostics\"")
            })
        };
        match panics.len() {
            2 if logged("frontmatter") && logged("dsl") => Ok(()),
            _ => Err(format!("{panics:#?}")),
        }
    });
}
