//! Which [`BindingView`] an expression is classified against, and under which
//! host's lifecycle.
//!
//! A document's lifecycle stacks are analyzed under one [`LifecycleHost`]:
//! the host lifecycle profile recorded beside a host catalog in the
//! document's schema roots
//! ([`darkmatter::markdown::schemas::catalog::host_lifecycle_profiles`]), or
//! the `md` host ([`md_host::registry`]) when no catalog records one. Its
//! registry decides which root frontmatter keys are stacks, and its parse
//! options which verbs carry expression operands. An expression inside the
//! value of a root key the registry resolves to one of its events reads that
//! event's view, at any depth of the stack: `when` guards, action operands,
//! and `set`/`with` data alike. Every other expression reads Darkmatter's
//! baseline view. Event names come from the registry's descriptors, never
//! from a list kept here.
//!
//! Passive: views are built from descriptors, catalogs, and artifacts only;
//! DMLS never runs host code and does not depend on any host.

use std::path::Path;
use std::sync::Arc;

use darkmatter::lifecycle::host_profile::HostLifecycleProfile;
use darkmatter::lifecycle::md_host;
use darkmatter::lifecycle::registry::{EventId, EventRegistry, KeyResolution};
use darkmatter::lifecycle::LifecycleConfig;
use darkmatter::lifecycle::error::LifecycleError;
use darkmatter::markdown::compose::expression::{
    Availability, BindingView, ParseError, SpannedExpr, is_reserved_namespace,
};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, HostLifecycleDiscovery};
use darkmatter::markdown::span::SourceSpan;

use crate::overlay::expressions::{self, ScalarProjection};
use crate::overlay::{FmEntry, FmPathSegment, FmValueKind, FrontmatterAst};

/// The host whose lifecycle a document's stacks are analyzed under.
#[derive(Debug, Clone)]
pub struct LifecycleHost {
    registry: Arc<EventRegistry>,
    profile: Option<Arc<HostLifecycleProfile>>,
    incomplete: Vec<String>,
}

impl LifecycleHost {
    /// The `md` host, with nothing reported.
    pub fn md() -> Self {
        Self { registry: md_host::registry().clone(), profile: None, incomplete: Vec::new() }
    }

    /// The first host lifecycle profile in the document's schema roots, in
    /// [`host_catalogs`](darkmatter::markdown::schemas::catalog::host_catalogs)
    /// order. Without one, the `md` host, plus each catalog's reason its
    /// artifact could not be trusted ([`Self::incomplete`]); without a
    /// resolution context, the `md` host alone.
    ///
    /// Discovery is lazy: no catalog or artifact after the first usable
    /// profile is read.
    pub fn for_document(ctx: &crate::providers::DocumentContext) -> Self {
        let Ok(context) = ctx.file_context() else {
            return Self::md();
        };
        Self::first_usable(darkmatter::markdown::schemas::catalog::host_lifecycle_profiles(context))
    }

    /// The first usable profile `discoveries` yields, taking nothing after
    /// it; without one, the `md` host with every reason in order.
    fn first_usable(discoveries: impl IntoIterator<Item = HostLifecycleDiscovery>) -> Self {
        let mut incomplete = Vec::new();
        for discovery in discoveries {
            match discovery.profile {
                Ok(profile) => {
                    return Self { registry: profile.registry().clone(), profile: Some(profile), incomplete: Vec::new() };
                }
                Err(reason) => incomplete.push(reason),
            }
        }
        Self { incomplete, ..Self::md() }
    }

    /// The host's event registry.
    pub fn registry(&self) -> &Arc<EventRegistry> {
        &self.registry
    }

    /// Why each host catalog's lifecycle artifact was not used, when none
    /// was: the document's host lifecycle is then analyzed as the `md` host's,
    /// which may not be the host's.
    pub fn incomplete(&self) -> &[String] {
        &self.incomplete
    }

    /// Parses and statically validates the document's lifecycle under this
    /// host's parse options, as the host does before anything runs.
    ///
    /// ## Errors
    ///
    /// The parser's [`LifecycleError`].
    pub fn parse_lifecycle(&self, frontmatter: &serde_json::Value, path: &Path) -> Result<LifecycleConfig, LifecycleError> {
        let Some(profile) = &self.profile else {
            return md_host::parse_lifecycle(frontmatter, path, None);
        };
        let config = darkmatter::lifecycle::parse_lifecycle_config_with(frontmatter, path, &profile.parse_options(None))?;
        darkmatter::lifecycle::validate_no_nested_spans_in_literals_in(frontmatter, &config, path, &self.registry)?;
        Ok(config)
    }

    /// The event whose stack `path` (rooted at the frontmatter) lies in.
    pub fn stack_event(&self, path: &[FmPathSegment<'_>]) -> Option<StackEvent> {
        match path.first()? {
            FmPathSegment::Key(key) => self.event_of_key(key),
            FmPathSegment::Index(_) => None,
        }
    }

    /// The event a root frontmatter key holds the stack of.
    pub fn event_of_key(&self, key: &str) -> Option<StackEvent> {
        match self.registry.resolve_key(key) {
            KeyResolution::Event(id) => Some(StackEvent { registry: self.registry.clone(), id }),
            KeyResolution::Renamed { .. } | KeyResolution::Data => None,
        }
    }
}

/// One event of a [`LifecycleHost`], with the registry it belongs to.
#[derive(Debug, Clone)]
pub struct StackEvent {
    registry: Arc<EventRegistry>,
    id: EventId,
}

impl StackEvent {
    /// The event's identity in its registry.
    pub fn id(&self) -> EventId {
        self.id
    }

    /// The event's authored name.
    pub fn name(&self) -> &str {
        self.registry.name(self.id)
    }

    /// The registry's own view of the event, without host-catalog globals.
    pub fn globals(&self) -> &BindingView {
        self.registry.globals(self.id)
    }
}

/// The view DMLS classifies an expression's roots against: the registry view
/// of `event`, or Darkmatter's baseline outside a lifecycle stack, plus every
/// global a host catalog declares.
///
/// This is the one place a host-catalog global's availability is decided.
/// DMLS has no host runtime context: it cannot know whether an expression runs
/// inside a loop or a group, and it does not depend on any host. So a
/// host-catalog global is declared `ExecutionDependent`, inside a stack or
/// not, which is the availability a host gives when membership is unknown
/// (Claudine's `binding_view_in`). Passive validation defers such a global, so
/// it is never undeclared and never unavailable. Without a catalog the root
/// stays a document property. A registry declaration of the same
/// root wins over a catalog's, and a catalog name that cannot be a global
/// (a reserved namespace or a non-identifier) is ignored.
pub fn validation_view(event: Option<&StackEvent>, catalogs: &[GlobalCatalog]) -> BindingView {
    let baseline = BindingView::baseline();
    let base: &BindingView = match event {
        Some(event) => event.globals(),
        None => &baseline,
    };
    let mut builder = BindingView::builder(base.scope().clone());
    let mut declared: Vec<&str> = Vec::new();
    for global in base.globals() {
        builder = builder.declare(global.root(), global.description(), global.availability().clone());
        declared.push(global.root());
    }
    for global in catalogs.iter().flat_map(GlobalCatalog::globals) {
        let name = global.name();
        if declared.contains(&name) || is_reserved_namespace(name) || !is_declarable(name) {
            continue;
        }
        builder = builder.declare(
            name,
            global.description().unwrap_or("host global"),
            Availability::ExecutionDependent,
        );
        declared.push(name);
    }
    builder.build().unwrap_or_else(|_| base.clone())
}

/// Whether `name` is a root a view accepts, asked of Darkmatter's builder.
fn is_declarable(name: &str) -> bool {
    BindingView::builder(BindingView::baseline().scope().clone())
        .declare(name, "", Availability::ExecutionDependent)
        .build()
        .is_ok()
}

/// The grammar an expression is parsed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// A `{{ … }}` span: `||` is fallback.
    Value,
    /// A `when` guard's condition text: `||` is logical OR.
    Condition,
}

impl Dialect {
    /// The parser for this dialect.
    pub fn parser(self) -> fn(&str) -> Result<SpannedExpr, ParseError> {
        match self {
            Self::Value => expressions::parse,
            Self::Condition => expressions::parse_condition,
        }
    }
}

/// One expression inside a lifecycle stack value.
#[derive(Debug, Clone)]
pub struct StackExpression<'a> {
    /// The event whose stack holds it.
    pub event: StackEvent,
    /// The scalar entry holding it.
    pub entry: &'a FmEntry,
    /// The entry's arena index.
    pub index: usize,
    /// The expression text the parser sees.
    pub text: String,
    pub dialect: Dialect,
    /// Where `text` starts in the scalar's analyzed text.
    start: usize,
    /// The authored range a cursor must be in: the `{{ … }}` span, or the
    /// whole guard scalar.
    reach: SourceSpan,
    projection: ScalarProjection,
}

impl StackExpression<'_> {
    /// Projects a range of [`text`](Self::text) to authored bytes, or `None`
    /// without an exact map.
    pub fn project(&self, range: std::ops::Range<usize>) -> Option<SourceSpan> {
        self.projection.project(self.start + range.start..self.start + range.end)
    }

    /// The exact authored range of a range of the text, else the whole scalar.
    pub fn range(&self, range: std::ops::Range<usize>) -> SourceSpan {
        self.project(range).unwrap_or_else(|| self.projection.scalar_span())
    }

    /// The exact authored range of a range of the whole scalar's analyzed
    /// text (not of [`text`](Self::text)), else the whole scalar.
    pub fn scalar_range(&self, range: std::ops::Range<usize>) -> SourceSpan {
        self.projection.project(range).unwrap_or_else(|| self.projection.scalar_span())
    }

    /// Whether the projection is exact.
    pub fn is_exact(&self) -> bool {
        self.projection.is_exact()
    }

    /// The authored range a hover or completion cursor must fall in.
    pub fn reach(&self) -> &SourceSpan {
        &self.reach
    }

    /// The offset into [`text`](Self::text) of a document cursor, when the
    /// projection is exact and the cursor is inside the text.
    pub fn text_offset(&self, document_offset: usize) -> Option<usize> {
        let analyzed = self.projection.analyzed_offset(document_offset)?;
        let offset = analyzed.checked_sub(self.start)?;
        (offset <= self.text.len()).then_some(offset)
    }

    pub fn parse(&self) -> Result<SpannedExpr, ParseError> {
        (self.dialect.parser())(&self.text)
    }
}

/// Every expression inside a lifecycle stack value, in document order: the
/// `{{ … }}` spans of any scalar beneath the stack, and each item's `when`
/// guard that holds no span. `skip` names entries another surface already
/// analyzes (an Expression-typed value), which contribute nothing here.
///
/// This walks the whole document; a request about one cursor uses
/// [`scalar_stack_expressions`] instead, which yields the same expressions for
/// its one scalar.
pub fn stack_expressions<'a>(
    host: &LifecycleHost,
    document: &str,
    ast: &'a FrontmatterAst,
    skip: impl Fn(usize) -> bool,
) -> Vec<StackExpression<'a>> {
    crate::work_counters::record_stack_walk();
    let mut out = Vec::new();
    for index in 0..ast.entries().len() {
        if !skip(index) {
            out.extend(scalar_stack_expressions(host, document, ast, index));
        }
    }
    out.sort_by_key(|expression| expression.reach.start);
    out
}

/// The lifecycle expressions inside the one scalar entry `index`, in order:
/// its `{{ … }}` spans, or, for a `when` guard holding no span, the guard
/// itself. Empty for a non-scalar entry or one outside every stack.
pub fn scalar_stack_expressions<'a>(
    host: &LifecycleHost,
    document: &str,
    ast: &'a FrontmatterAst,
    index: usize,
) -> Vec<StackExpression<'a>> {
    let Some(entry) = ast.entries().get(index).filter(|entry| entry.kind == FmValueKind::Scalar) else {
        return Vec::new();
    };
    let path = ast.path_at(index);
    let Some(event) = host.stack_event(&path) else {
        return Vec::new();
    };
    let spans: Vec<StackExpression<'a>> = expressions::scalar_interpolations(document, entry, index)
        .into_iter()
        .map(|interpolation| StackExpression {
            event: event.clone(),
            entry,
            index,
            start: interpolation.analyzed_start(),
            reach: interpolation.outer_span(),
            projection: interpolation.projection().clone(),
            text: interpolation.text,
            dialect: Dialect::Value,
        })
        .collect();
    if !spans.is_empty() || !crate::diagnostics::nested_span::lifecycle::is_stack_predicate(&path[1..]) {
        return spans;
    }
    let Some((projection, analyzed)) = ScalarProjection::for_value(entry, document) else {
        return spans;
    };
    // A guard holding a `{{ … }}` span is analyzed through that span.
    if analyzed.contains("{{") || analyzed.trim().is_empty() {
        return spans;
    }
    vec![StackExpression {
        event,
        entry,
        index,
        start: 0,
        reach: entry.value_span.clone(),
        projection,
        text: analyzed,
        dialect: Dialect::Condition,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast(text: &str) -> FrontmatterAst {
        FrontmatterAst::parse(text).and_then(|parse| parse.ast).expect("frontmatter parses")
    }

    #[test]
    fn every_registry_event_key_is_a_stack_and_nothing_else_is() {
        let host = LifecycleHost::md();
        for key in md_host::registry().authored_event_keys() {
            let event = host.event_of_key(key).unwrap_or_else(|| panic!("{key}"));
            assert_eq!(event.name(), key);
        }
        for key in ["start", "success", "failure", "loop", "title", "err"] {
            assert!(host.event_of_key(key).is_none(), "{key}");
        }
    }

    #[test]
    fn a_host_catalog_global_is_execution_dependent_in_every_view() {
        let catalog = GlobalCatalog::from_documents(
            "virtual/globals.yaml",
            &[("virtual/globals.yaml", "kind: schema\ndescription: host\n$schema:\n    seq:\n        index: \"number -> The position.\"\n")],
            &[darkmatter::markdown::schemas::catalog::GlobalRequirement::shape("seq")],
        )
        .expect("the test catalog loads");
        let catalogs = [catalog];
        let host = LifecycleHost::md();
        for event in std::iter::once(None).chain(
            md_host::registry().authored_event_keys().into_iter().map(|key| host.event_of_key(key)),
        ) {
            let view = validation_view(event.as_ref(), &catalogs);
            assert_eq!(
                view.global("seq").map(|global| global.availability()),
                Some(&Availability::ExecutionDependent),
                "{event:?}"
            );
            assert!(!view.names_document_property("seq"));
        }
        assert!(validation_view(None, &[]).names_document_property("seq"));
    }

    #[test]
    fn guards_and_spans_beneath_an_event_key_are_stack_expressions() {
        let text = "---\ncomposed:\n  - when: err.message != ''\n    then:\n      - info: \"{{ err.message }}\"\nnotes:\n  - info: \"{{ err }}\"\n---\n";
        let ast = ast(text);
        let found = stack_expressions(&LifecycleHost::md(), text, &ast, |_| false);
        let texts: Vec<(&str, Dialect)> = found.iter().map(|e| (e.text.as_str(), e.dialect)).collect();
        assert_eq!(texts, [("err.message != ''", Dialect::Condition), ("err.message", Dialect::Value)]);
        assert!(found.iter().all(|e| e.event.name() == "composed"));
    }

    /// `(index, text, dialect, reach)` of each expression, comparable.
    fn summary(found: &[StackExpression<'_>]) -> Vec<(usize, String, Dialect, SourceSpan)> {
        found.iter().map(|e| (e.index, e.text.clone(), e.dialect, e.reach().clone())).collect()
    }

    #[test]
    fn one_scalar_yields_exactly_its_share_of_the_document_walk() {
        let text = concat!(
            "---\n",
            "title: \"{{ outside }}\"\n",
            "composed:\n",
            "  - when: plain != ''\n",
            "    then:\n",
            "      - info: '{{ first }} and {{ second }}'\n",
            "      - info: \"{{ quoted }}\"\n",
            "      - info: |\n",
            "          {{ block }}\n",
            "      - info: !!str \"{{ tagged }}\"\n",
            "  - when: \"{{ wrapped }}\"\n",
            "    then: [stop]\n",
            "  - when: typed == 1\n",
            "    then: [stop]\n",
            "---\n",
        );
        let ast = ast(text);
        let host = LifecycleHost::md();
        let typed = ast.entries().iter().position(|entry| entry.scalar.as_deref() == Some("typed == 1")).unwrap();
        let skip = |index: usize| index == typed;

        let walked = stack_expressions(&host, text, &ast, skip);
        let mut per_scalar: Vec<StackExpression<'_>> = (0..ast.entries().len())
            .filter(|&index| !skip(index))
            .flat_map(|index| scalar_stack_expressions(&host, text, &ast, index))
            .collect();
        per_scalar.sort_by_key(|expression| expression.reach.start);
        assert_eq!(summary(&walked), summary(&per_scalar));

        let texts: Vec<(&str, Dialect)> = walked.iter().map(|e| (e.text.as_str(), e.dialect)).collect();
        assert_eq!(
            texts,
            [
                ("plain != ''", Dialect::Condition),
                ("first", Dialect::Value),
                ("second", Dialect::Value),
                ("quoted", Dialect::Value),
                ("block", Dialect::Value),
                ("tagged", Dialect::Value),
                ("wrapped", Dialect::Value),
            ],
            "a span-holding guard is analyzed through its span; the typed guard is skipped"
        );
        // Two spans of one scalar come from that scalar alone, in order.
        let pair = walked.iter().find(|e| e.text == "first").unwrap().index;
        let own: Vec<String> =
            scalar_stack_expressions(&host, text, &ast, pair).into_iter().map(|e| e.text).collect();
        assert_eq!(own, ["first", "second"]);
        // A value outside every stack, and a container, yield nothing.
        let outside = ast.entries().iter().position(|entry| entry.key == "title").unwrap();
        assert!(scalar_stack_expressions(&host, text, &ast, outside).is_empty());
        let container = ast.entries().iter().position(|entry| entry.key == "composed").unwrap();
        assert!(scalar_stack_expressions(&host, text, &ast, container).is_empty());
        assert!(scalar_stack_expressions(&host, text, &ast, ast.entries().len()).is_empty());
        // The typed guard, asked for directly, is still a guard: skipping is
        // the caller's decision.
        let guard = scalar_stack_expressions(&host, text, &ast, typed);
        assert_eq!(summary(&guard).len(), 1);
        assert_eq!(guard[0].dialect, Dialect::Condition);
    }

    #[test]
    fn only_the_document_walk_is_counted_as_one() {
        let text = "---\ncomposed:\n  - when: a\n    then: [stop]\n---\n";
        let ast = ast(text);
        let before = crate::work_counters::work_counts();
        for index in 0..ast.entries().len() {
            let _ = scalar_stack_expressions(&LifecycleHost::md(), text, &ast, index);
        }
        assert_eq!(crate::work_counters::work_counts().since(before).stack_walks, 0);
        let _ = stack_expressions(&LifecycleHost::md(), text, &ast, |_| false);
        assert_eq!(crate::work_counters::work_counts().since(before).stack_walks, 1);
    }

    /// A host catalog in `dir` named `name`, with an artifact when `fresh`.
    fn host_catalog(dir: &Path, name: &str, fresh: bool) -> std::path::PathBuf {
        use darkmatter::lifecycle::host_profile::{lifecycle_artifact, lifecycle_artifact_path};
        use darkmatter::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy};

        let text = format!("kind: schema\ndescription: {name}\n$schema:\n    run:\n        count: number\n");
        let path = dir.join(name);
        std::fs::write(&path, &text).expect("catalog");
        if fresh {
            let options = LifecycleParseOptions {
                is_known_effect: &|_| false,
                authored: None,
                unknown_verbs: UnknownVerbPolicy::Reject,
                registry: &EventRegistry::new("test"),
            };
            std::fs::write(lifecycle_artifact_path(&path), lifecycle_artifact(&options, &[], &text)).expect("artifact");
        }
        path
    }

    /// The host is the first usable profile, and nothing of a later
    /// candidate is read; with none usable, every reason is kept in order.
    #[test]
    fn the_host_is_the_first_usable_profile_and_nothing_after_it_is_read() {
        use darkmatter::lifecycle::host_profile::lifecycle_artifact_path;
        use darkmatter::markdown::schemas::catalog::discovery_probe::DiscoveryProbe;
        use darkmatter::markdown::schemas::catalog::host_lifecycle_profiles;

        let dir = tempfile::TempDir::new().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let schemas = root.join("schemas");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(&schemas).expect("schema root");
        let context =
            biscuit_file::FileResolutionContext::from_snapshot(&root, None, Default::default()).with_repository_root(&root);
        let unusable = host_catalog(&schemas, "a.yaml", false);
        let winner = host_catalog(&schemas, "b.yaml", true);
        let later = host_catalog(&schemas, "c.yaml", true);
        let probe = DiscoveryProbe::install();

        let host = LifecycleHost::first_usable(host_lifecycle_profiles(&context));
        assert_eq!(host.registry().host(), "test");
        assert!(host.incomplete().is_empty());
        let reads = probe.take_reads();
        assert!(reads.contains(&unusable) && reads.contains(&winner), "{reads:?}");
        assert!(!reads.contains(&later) && !reads.contains(&lifecycle_artifact_path(&later)), "{reads:?}");

        std::fs::remove_file(lifecycle_artifact_path(&winner)).expect("remove artifact");
        std::fs::remove_file(lifecycle_artifact_path(&later)).expect("remove artifact");
        let host = LifecycleHost::first_usable(host_lifecycle_profiles(&context));
        assert_eq!(host.registry().host(), "md");
        let artifacts: Vec<_> = [&unusable, &winner, &later]
            .into_iter()
            .map(|catalog| format!("the lifecycle artifact `{}` is absent", lifecycle_artifact_path(catalog).display()))
            .collect();
        assert_eq!(host.incomplete(), artifacts);
    }
}
