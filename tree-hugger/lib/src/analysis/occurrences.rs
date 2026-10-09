//! Reference occurrences and coverage for one file: every reference the
//! references query captures, its owning region, its kind and what it binds
//! to under the language's scope rules ([`super::scope`]).
//!
//! Everything here describes the whole file, whatever kinds a query selects.
//! Languages without binding rules report conservative occurrences (never
//! `Scoped`) and `Limited(unsupported_language)` regions.
//!
//! User contract: `tree-hugger/docs/references.md`.

use std::collections::{BTreeSet, HashMap, HashSet};

use tree_sitter::Node;

use super::scope::{
    BindingRule, BindingSite, BindingTarget, Dynamic, LanguageRules, Lookup, Refutability, ScopeTree, ancestors,
    is_direct_eval,
};
use super::declaration_names::{declaration_names, declared_name_tokens, scala_import_path_start};
use super::syntax_markers::{SyntaxMarkers, syntax_markers};
use super::symbol_ref;
use crate::file::parents::IndexedParent as _;
use crate::file::extract::{is_definition_like_reference, next_named_syntax_sibling, text_span_for_node};
use crate::file::tree_file::{ExtractedSymbol, ReferenceCapture};
use crate::shared::{
    Confidence, CoverageState, ImportSymbol, LimitReason, Occurrence, OccurrenceKind, ProgrammingLanguage,
    ReferenceCoverage, ReferenceEvidence, SymbolId, SymbolKind, SymbolKindV2, SymbolRecord, SymbolRef, SymbolScope,
    TextPoint, TextSpan,
};

/// The nodes of one definition, kept until its record exists.
struct DefinitionNodes<'tree> {
    name: Node<'tree>,
    declaration: Node<'tree>,
    anchor: Node<'tree>,
    kind: SymbolKind,
    scope: SymbolScope,
    /// The Rust `impl` this member is declared in.
    container_impl: Option<Node<'tree>>,
}

/// The syntax reference resolution reads, collected before the definitions
/// become records (whose order matches `definitions`).
pub(crate) struct OccurrenceSyntax<'tree> {
    language: ProgrammingLanguage,
    source: &'tree str,
    root: Node<'tree>,
    definitions: Vec<DefinitionNodes<'tree>>,
    scope_nodes: Vec<Node<'tree>>,
    /// `None`: the language's references query has no patterns.
    captures: Option<Vec<ReferenceCapture<'tree>>>,
    /// Definition and import name spans, never occurrences.
    excluded: HashSet<(usize, usize)>,
    imports: Vec<ImportSite>,
    /// The first import whose name starts at each byte.
    import_by_start: HashMap<usize, usize>,
    /// The definition pass's Rust pattern placement, finished by
    /// [`Self::resolve`] instead of walking again.
    patterns: Option<RustPatterns<'tree>>,
}

/// A Rust file's walk, scope tree and placed binding sites, as
/// [`rust_pattern_non_bindings`] left them for the definition pass. The
/// placement decides exactly what the bind pass's would: only the targets
/// of the sites differ (no record exists yet), and no decision reads them
/// ([`bare_pattern_meaning`]). [`OccurrenceSyntax::resolve`] sets them.
pub(crate) struct RustPatterns<'tree> {
    walk: Walk<'tree>,
    tree: ScopeTree<'tree>,
    placed: PlacedSites,
}

/// Where one import record's name was captured, in import order.
struct ImportSite {
    span: TextSpan,
    /// The whole import statement's byte range.
    statement: Option<(usize, usize)>,
    name: String,
}

/// [`ReferenceEvidence`] plus, for each occurrence (same order), the index of
/// the import it is bound to, which the summaries record as a dependency.
/// The evidence already carries the selected import's span before projection.
pub(crate) struct ResolvedOccurrences {
    pub(crate) evidence: ReferenceEvidence,
    pub(crate) bound_imports: Vec<Option<usize>>,
}

impl<'tree> OccurrenceSyntax<'tree> {
    pub(crate) fn new(
        language: ProgrammingLanguage,
        source: &'tree str,
        root: Node<'tree>,
        definitions: &[ExtractedSymbol<'tree>],
        scope_nodes: Vec<Node<'tree>>,
        imports: &[ImportSymbol],
        captures: Option<Vec<ReferenceCapture<'tree>>>,
    ) -> Self {
        Self {
            language,
            source,
            root,
            definitions: definitions
                .iter()
                .map(|extracted| DefinitionNodes {
                    name: extracted.name,
                    declaration: extracted.declaration,
                    anchor: extracted.anchor,
                    kind: extracted.symbol.kind,
                    scope: extracted.symbol.scope,
                    container_impl: extracted.container_impl,
                })
                .collect(),
            scope_nodes,
            captures,
            excluded: crate::file::tree_file::definition_ranges(definitions, imports),
            imports: imports
                .iter()
                .map(|import| ImportSite {
                    span: super::text_span(&import.range),
                    statement: import.statement_range.as_ref().map(|range| (range.start_byte, range.end_byte)),
                    name: import.name.clone(),
                })
                .collect(),
            import_by_start: imports.iter().enumerate().rev().map(|(idx, import)| (import.range.start_byte, idx)).collect(),
            patterns: None,
        }
    }

    /// Has [`Self::resolve`] finish the definition pass's Rust pattern
    /// placement of the same file instead of walking it again.
    pub(crate) fn with_rust_patterns(mut self, patterns: Option<RustPatterns<'tree>>) -> Self {
        self.patterns = patterns.filter(|_| self.language == ProgrammingLanguage::Rust);
        self
    }

    /// The import (index into the `imports` given to [`Self::new`]) whose
    /// binding name starts at `name_start`: the import whose name node it
    /// is, else the import of that name whose statement holds it.
    fn import_at(&self, name_start: usize, name: &str) -> Option<usize> {
        self.import_by_start.get(&name_start).copied().or_else(|| {
            self.imports.iter().position(|site| {
                super::name_identity::equal(self.language, &site.name, name) && site.statement.is_some_and(|(start, end)| start <= name_start && name_start < end)
            })
        })
    }

    /// Resolves every captured reference against `records` (one per
    /// definition, in the same order).
    pub(crate) fn resolve(mut self, records: &[SymbolRecord]) -> ResolvedOccurrences {
        debug_assert_eq!(records.len(), self.definitions.len());
        let rules = LanguageRules::for_language(self.language);
        let (walk, placed) = match self.patterns.take() {
            Some(RustPatterns { walk, tree, placed }) => (walk, Some((tree, placed))),
            None => (Walk::run(self.root, self.language, rules, self.source), None),
        };
        let mut regions = Regions::new(self.root, whole_file_span(self.source));

        // A named callable owns a region exactly when it has a body
        // (`body_span`, the extractor's one body reader): the node holding
        // its `body` field, else (a Python/Lua body read as a run of
        // children, a Lua body with no `block`) the declaration.
        for (idx, definition) in self.definitions.iter().enumerate() {
            if definition.kind.is_function()
                && records[idx].source.body_span.is_some()
                && let Some(node) = [definition.anchor, definition.declaration]
                    .into_iter()
                    .find(|node| node.child_by_field_name("body").is_some())
                    .or(Some(definition.declaration))
                    .filter(|node| node.id() != self.root.id())
            {
                regions.add(node, Some(idx));
            }
        }
        for &node in &walk.anonymous {
            regions.add(node, None);
        }
        for &(node, reason) in &walk.initializers {
            let region = regions.add(node, None);
            if let Some(reason) = reason {
                regions.limit(region, reason);
            }
        }
        for &(node, reason) in &walk.reasons {
            let region = regions.around(node);
            regions.limit(region, reason);
        }
        for &node in &walk.errors {
            let region = regions.around(node);
            regions.limit(region, LimitReason::ParseError);
        }
        if rules.is_none() {
            regions.limit_all(LimitReason::UnsupportedLanguage);
        }
        if self.captures.is_none() {
            regions.limit_all(LimitReason::EmptyReferenceQuery);
        }

        let resolver = Resolver::new(&self, records, rules, &walk, placed);
        let mut resolved = Vec::new();
        for capture in self.captures.iter().flatten() {
            if let Some((mut occurrence, import)) = resolver.occurrence(*capture, &mut regions) {
                occurrence.import_span = import.map(|idx| self.imports[idx].span.clone());
                resolved.push((occurrence, import));
            }
        }
        resolved.sort_by(|(left, _), (right, _)| occurrence_order(left).cmp(&occurrence_order(right)));
        let (occurrences, bound_imports) = resolved.into_iter().unzip();

        ResolvedOccurrences {
            evidence: ReferenceEvidence {
                occurrences,
                reference_coverage: regions.coverage(records),
            },
            bound_imports,
        }
    }
}

/// The canonical order: span, owner span, kind, written target, confidence.
pub(crate) fn occurrence_order(occurrence: &Occurrence) -> (u32, u32, u32, u32, OccurrenceKind, &str, Option<&str>, Confidence) {
    (
        occurrence.span.start_byte,
        occurrence.span.end_byte,
        occurrence.owner_span.start_byte,
        occurrence.owner_span.end_byte,
        occurrence.kind,
        occurrence.target.name.as_str(),
        occurrence.target.qualified_name.as_deref(),
        occurrence.confidence,
    )
}

/// The span of the whole source: the region of file-level code.
pub(crate) fn whole_file_span(source: &str) -> TextSpan {
    let line = source.matches('\n').count() as u32 + 1;
    let line_start = source.rfind('\n').map_or(0, |newline| newline + 1);
    TextSpan {
        start: TextPoint { line: 1, column: 1 },
        end: TextPoint {
            line,
            column: (source.len() - line_start) as u32 + 1,
        },
        start_byte: 0,
        end_byte: source.len() as u32,
    }
}

/// What one walk of the whole tree finds.
#[derive(Default)]
struct Walk<'tree> {
    anonymous: Vec<Node<'tree>>,
    /// Owner-absent regions (a Rust `const` inside a function, a class field
    /// initializer) and the limitation they carry.
    initializers: Vec<(Node<'tree>, Option<LimitReason>)>,
    /// A limitation of the region around the node.
    reasons: Vec<(Node<'tree>, LimitReason)>,
    errors: Vec<Node<'tree>>,
    extra_scopes: Vec<Node<'tree>>,
    sites: Vec<BindingSite<'tree>>,
    globs: Vec<Node<'tree>>,
    dynamics: Vec<Dynamic<'tree>>,
    /// Declaration-name tokens of a language without rules
    /// ([`declaration_names`]); never occurrences.
    declarations: Vec<Node<'tree>>,
    /// Contextual syntax markers of a language without rules
    /// ([`syntax_markers`]); never occurrences.
    markers: SyntaxMarkers<'tree>,
    /// The nodes below an excluded context ([`is_excluded_context`]), by id.
    in_excluded_context: HashSet<usize>,
    /// The TS/JS tokens that name a module export
    /// ([`is_ecma_export_name`]), by id.
    export_names: HashSet<usize>,
}

/// What [`Walk::run`] knows of the nodes on its path.
struct WalkFrame<'tree> {
    node: Node<'tree>,
    /// The node or one of its ancestors is an excluded context.
    excluding: bool,
    /// The nearest `export_statement` at or above the node, and whether it
    /// re-exports from a `source`.
    export: Option<(Node<'tree>, bool)>,
}

impl<'tree> Walk<'tree> {
    /// Visits every node once, without recursion.
    fn run(root: Node<'tree>, language: ProgrammingLanguage, rules: Option<LanguageRules>, source: &str) -> Self {
        let mut walk = Self::default();
        let language_anonymous = anonymous_callable_kinds(language);
        let ecma = matches!(language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript);
        let mut path: Vec<WalkFrame<'tree>> = Vec::new();
        let mut cursor = root.walk();
        loop {
            let node = cursor.node();
            let above = path.last();
            if above.is_some_and(|frame| frame.excluding) {
                walk.in_excluded_context.insert(node.id());
            }
            if ecma
                && let Some(frame) = above
                && let Some((statement, from_source)) = frame.export
                && names_module_export(node, frame.node, statement, from_source)
            {
                walk.export_names.insert(node.id());
            }
            if node.is_error() || node.is_missing() {
                walk.errors.push(node);
            }
            if node.is_named() {
                if language_anonymous.contains(&node.kind()) {
                    walk.anonymous.push(node);
                }
                match rules {
                    Some(rules) => walk.visit(node, rules, source),
                    None => {
                        declaration_names(node, language, source, &mut walk.declarations);
                        syntax_markers(node, language, source, &mut walk.markers);
                    }
                }
            }
            if cursor.goto_first_child() {
                let frame = WalkFrame {
                    node,
                    excluding: above.is_some_and(|frame| frame.excluding) || is_excluded_context(node.kind(), language),
                    export: if ecma && node.kind() == "export_statement" {
                        Some((node, node.child_by_field_name("source").is_some()))
                    } else {
                        above.and_then(|frame| frame.export)
                    },
                };
                path.push(frame);
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return walk;
                }
                path.pop();
            }
        }
    }

    fn visit(&mut self, node: Node<'tree>, rules: LanguageRules, source: &str) {
        if rules.extra_scope(node) {
            self.extra_scopes.push(node);
        }
        self.extra_scopes.extend(rules.extra_child_scope(node));
        rules.binding_sites(node, &mut self.sites);
        if rules.is_glob_import(node) {
            self.globs.push(node);
        }
        self.dynamics.extend(rules.dynamic(node, source));
        match rules {
            LanguageRules::Rust => self.visit_rust(node),
            LanguageRules::Ecma { .. } => self.visit_ecma(node, rules.language(), source),
        }
    }

    fn visit_rust(&mut self, node: Node<'tree>) {
        match node.kind() {
            "macro_invocation" => self.reasons.push((node, LimitReason::UnexpandedMacro)),
            "call_expression" => {
                let function = node
                    .child_by_field_name("function")
                    .map(|function| invoked_operand(function, ProgrammingLanguage::Rust));
                match function.map(|function| function.kind()) {
                    Some("identifier" | "scoped_identifier") => {}
                    // A method call, or a call of a computed value.
                    _ => self.reasons.push((node, LimitReason::IndirectDispatch)),
                }
            }
            "const_item" | "static_item"
                if ancestors(node).any(|ancestor| matches!(ancestor.kind(), "function_item" | "closure_expression")) =>
            {
                self.initializers.push((node, None));
            }
            _ => {}
        }
    }

    fn visit_ecma(&mut self, node: Node<'tree>, language: ProgrammingLanguage, source: &str) {
        match node.kind() {
            "call_expression" if is_direct_eval(node, language, source) => {
                self.reasons.push((node, LimitReason::UnsupportedBindingForm));
                self.reasons.push((node, LimitReason::IndirectDispatch));
            }
            "call_expression" => match node.child_by_field_name("function").map(|function| invoked_operand(function, language)) {
                // An optional `eval?.(…)` runs code that may call anything;
                // a tagged ``eval`…` `` evaluates nothing.
                Some(function)
                    if function.kind() == "identifier"
                        && function.utf8_text(source.as_bytes()).is_ok_and(|name| super::name_identity::equal(language, name, "eval"))
                        && node.child_by_field_name("arguments").is_some_and(|arguments| arguments.kind() == "arguments") =>
                {
                    self.reasons.push((node, LimitReason::IndirectDispatch));
                }
                Some(function) if function.kind() == "identifier" => {}
                Some(function) if matches!(function.kind(), "super" | "import") => {}
                _ => self.reasons.push((node, LimitReason::IndirectDispatch)),
            },
            "new_expression" => {
                if node
                    .child_by_field_name("constructor")
                    .is_none_or(|constructor| invoked_operand(constructor, language).kind() != "identifier")
                {
                    self.reasons.push((node, LimitReason::IndirectDispatch));
                }
            }
            "with_statement" => self.reasons.push((node, LimitReason::UnsupportedBindingForm)),
            "public_field_definition" | "field_definition" if node.child_by_field_name("value").is_some() => {
                self.initializers.push((node, Some(LimitReason::OwnershipUnknown)));
            }
            "class_static_block" => self.initializers.push((node, Some(LimitReason::OwnershipUnknown))),
            _ => {}
        }
    }
}

/// The node kinds of a language's anonymous callables (closures, lambdas,
/// function expressions).
fn anonymous_callable_kinds(language: ProgrammingLanguage) -> &'static [&'static str] {
    match language {
        ProgrammingLanguage::Rust => &["closure_expression"],
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            &["arrow_function", "function_expression", "function", "generator_function"]
        }
        ProgrammingLanguage::Go => &["func_literal"],
        ProgrammingLanguage::Python => &["lambda"],
        ProgrammingLanguage::Java
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::Scala => &["lambda_expression"],
        ProgrammingLanguage::CSharp => &["lambda_expression", "anonymous_method_expression"],
        ProgrammingLanguage::Swift => &["lambda_literal"],
        ProgrammingLanguage::Php => &["anonymous_function", "anonymous_function_creation_expression", "arrow_function"],
        ProgrammingLanguage::Lua => &["function_definition"],
        ProgrammingLanguage::Perl => &["anonymous_subroutine_expression"],
        ProgrammingLanguage::C | ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => &[],
    }
}

/// One owning region.
struct Region {
    /// The record index of a named callable.
    owner: Option<usize>,
    span: TextSpan,
    reasons: BTreeSet<LimitReason>,
}

/// The owning regions of a file: the file itself first.
struct Regions {
    regions: Vec<Region>,
    by_node: HashMap<usize, usize>,
}

impl Regions {
    fn new(root: Node<'_>, file_span: TextSpan) -> Self {
        Self {
            regions: vec![Region { owner: None, span: file_span, reasons: BTreeSet::new() }],
            by_node: HashMap::from([(root.id(), 0)]),
        }
    }

    /// Adds the region of `node` (the first claim of a node wins) and
    /// returns its index.
    fn add(&mut self, node: Node<'_>, owner: Option<usize>) -> usize {
        *self.by_node.entry(node.id()).or_insert_with(|| {
            self.regions.push(Region {
                owner,
                span: text_span_for_node(node),
                reasons: BTreeSet::new(),
            });
            self.regions.len() - 1
        })
    }

    /// The innermost region `node` lies in (itself included).
    fn around(&self, node: Node<'_>) -> usize {
        std::iter::once(node)
            .chain(ancestors(node))
            .find_map(|candidate| self.by_node.get(&candidate.id()).copied())
            .unwrap_or(0)
    }

    fn limit(&mut self, region: usize, reason: LimitReason) {
        self.regions[region].reasons.insert(reason);
    }

    fn limit_all(&mut self, reason: LimitReason) {
        for region in &mut self.regions {
            region.reasons.insert(reason);
        }
    }

    fn owner_ref(&self, region: usize, records: &[SymbolRecord]) -> Option<SymbolRef> {
        self.regions[region].owner.map(|idx| symbol_ref(&records[idx]))
    }

    /// Every region's coverage, outer regions first at one start.
    fn coverage(&self, records: &[SymbolRecord]) -> Vec<ReferenceCoverage> {
        let mut order: Vec<usize> = (0..self.regions.len()).collect();
        order.sort_by_key(|&idx| {
            let span = &self.regions[idx].span;
            (span.start_byte, std::cmp::Reverse(span.end_byte), idx)
        });
        order
            .into_iter()
            .map(|idx| {
                let region = &self.regions[idx];
                ReferenceCoverage {
                    owner: self.owner_ref(idx, records),
                    span: region.span.clone(),
                    state: if region.reasons.is_empty() {
                        CoverageState::Complete
                    } else {
                        CoverageState::Limited
                    },
                    reasons: region.reasons.iter().copied().collect(),
                }
            })
            .collect()
    }
}

/// Resolves captures against the scope tree and records.
struct Resolver<'a, 'tree> {
    syntax: &'a OccurrenceSyntax<'tree>,
    walk: &'a Walk<'tree>,
    records: &'a [SymbolRecord],
    rules: Option<LanguageRules>,
    scopes: Option<ScopeTree<'tree>>,
    /// Name spans of binding sites and of a rule-less language's
    /// declaration names and syntax markers; never occurrences.
    excluded_names: HashSet<(usize, usize)>,
    /// What each bare pattern name that binds nothing proved to be.
    pattern_reads: PatternReads,
    /// Record indexes by name, in candidate order ([`Self::candidate_key`]).
    /// The sort is stable, so any filtered subset is in candidate order too.
    by_name: HashMap<String, Vec<usize>>,
}

impl<'a, 'tree> Resolver<'a, 'tree> {
    fn new(
        syntax: &'a OccurrenceSyntax<'tree>,
        records: &'a [SymbolRecord],
        rules: Option<LanguageRules>,
        walk: &'a Walk<'tree>,
        placed: Option<(ScopeTree<'tree>, PlacedSites)>,
    ) -> Self {
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, record) in records.iter().enumerate() {
            by_name.entry(super::name_identity::key(syntax.language, &record.identity.name).into_owned()).or_default().push(idx);
        }
        for indexes in by_name.values_mut() {
            indexes.sort_by(|&left, &right| Self::candidate_key(records, left).cmp(&Self::candidate_key(records, right)));
        }
        let mut pattern_reads = PatternReads::default();
        let scopes = rules.map(|rules| {
            let (mut tree, placed) = match placed {
                Some(placed) => placed,
                None => {
                    let mut tree = scope_tree(rules, syntax.root, &syntax.scope_nodes, walk);
                    let placed = place_sites(&mut tree, walk, syntax.source, |_| BindingTarget::Unreported);
                    (tree, placed)
                }
            };
            pattern_reads = bind_all(&mut tree, syntax, rules, walk, placed);
            tree
        });
        let excluded_names = walk
            .sites
            .iter()
            .map(|site| site.name)
            .chain(walk.declarations.iter().flat_map(|name| declared_name_tokens(*name)))
            .chain(walk.markers.nodes())
            .map(|name| (name.start_byte(), name.end_byte()))
            .filter(|range| !pattern_reads.contains(range))
            .collect();
        Self { syntax, walk, records, rules, scopes, excluded_names, pattern_reads, by_name }
    }

    fn text(&self, node: Node<'_>) -> &'tree str {
        self.syntax.source.get(node.start_byte()..node.end_byte()).unwrap_or_default()
    }

    /// The occurrence `capture` is, with the import it is bound to, or
    /// `None` for a definition or import name, or anything inside an import
    /// or attribute.
    fn occurrence(&self, capture: ReferenceCapture<'tree>, regions: &mut Regions) -> Option<(Occurrence, Option<usize>)> {
        let node = capture.node;
        let range = (node.start_byte(), node.end_byte());
        let name = self.text(node);
        if name.is_empty()
            || self.syntax.excluded.contains(&range)
            || self.excluded_names.contains(&range)
            // Supported languages know their binding names exactly; the
            // others add the declaration-name sites to the older syntactic
            // filter.
            || (self.rules.is_none() && is_definition_like_reference(node, self.syntax.language))
            || self.walk.in_excluded_context.contains(&node.id())
            || self.walk.export_names.contains(&node.id())
            || (self.rules == Some(LanguageRules::Rust) && name == "Self")
            || is_wildcard(name, self.syntax.language)
        {
            return None;
        }

        let kind = if is_call_target(node, self.syntax.language, self.syntax.source) {
            OccurrenceKind::Call
        } else {
            OccurrenceKind::Usage
        };
        let region = regions.around(node);
        let path = written_path(node, self.syntax.source, self.syntax.language);
        let (confidence, id, candidates, import) = match (&self.scopes, self.rules) {
            (Some(scopes), Some(rules)) => {
                let resolution = self.resolve_supported(node, name, path.as_deref(), capture.is_member, scopes, rules);
                if kind == OccurrenceKind::Call && resolution.indirect {
                    regions.limit(region, LimitReason::IndirectDispatch);
                }
                let import = resolution.import.and_then(|name_start| self.syntax.import_at(name_start, name));
                (resolution.confidence, resolution.id, resolution.candidates, import)
            }
            _ => {
                let bucket = self.by_name.get(super::name_identity::key(self.syntax.language, name).as_ref());
                let (confidence, id, candidates) = self.name_only(bucket.map_or(&[], Vec::as_slice));
                (confidence, id, candidates, None)
            }
        };

        let occurrence = Occurrence {
            owner: regions.owner_ref(region, self.records),
            owner_span: regions.regions[region].span.clone(),
            target: SymbolRef {
                id,
                name: name.to_string(),
                qualified_name: path,
                source_span: None,
            },
            import_span: None,
            candidates,
            span: text_span_for_node(node),
            kind,
            confidence,
        };
        Some((occurrence, import))
    }

    /// An uncertain resolution over `candidates` (record indexes, in
    /// candidate order): `indirect` when the caller says so or when calling
    /// any candidate would invoke a value it holds ([`holds_a_value`]).
    fn uncertain(&self, candidates: &[usize], indirect: bool) -> Resolution {
        let indirect = indirect
            || candidates
                .iter()
                .any(|&idx| holds_a_value(self.syntax.definitions[idx].kind));
        let (confidence, id, candidates) = self.name_only(candidates);
        Resolution { confidence, id, candidates, indirect, import: None }
    }

    /// The order candidates are listed in: by name start, then ID.
    fn candidate_key(records: &[SymbolRecord], idx: usize) -> (u32, &str) {
        let record = &records[idx];
        (record.source.name_span.as_ref().map_or(0, |span| span.start_byte), &record.id.0)
    }

    /// `NameOnly` over `candidates` (record indexes, in candidate order),
    /// `Unresolved` when there are none. Of adjacent candidates with one ID,
    /// the first is kept.
    fn name_only(&self, candidates: &[usize]) -> (Confidence, Option<SymbolId>, Vec<SymbolRef>) {
        let mut refs: Vec<SymbolRef> = Vec::new();
        let mut last: Option<&SymbolId> = None;
        for &idx in candidates {
            let id = &self.records[idx].id;
            if last != Some(id) {
                refs.push(symbol_ref(&self.records[idx]));
                last = Some(id);
            }
        }
        let confidence = if refs.is_empty() { Confidence::Unresolved } else { Confidence::NameOnly };
        (confidence, None, refs)
    }

    /// Resolves `node`, written as `name` and, when qualified, as `path`
    /// ([`written_path`]).
    fn resolve_supported(
        &self,
        node: Node<'tree>,
        name: &str,
        path: Option<&str>,
        is_member: bool,
        scopes: &ScopeTree<'tree>,
        rules: LanguageRules,
    ) -> Resolution {
        if is_member {
            // `x.f()`: dispatch can select another implementation.
            let members = self.records_named(name, |idx| {
                let record = &self.records[idx];
                let definition = &self.syntax.definitions[idx];
                definition.scope == SymbolScope::Member
                    && (definition.kind.is_function()
                        || (rules != LanguageRules::Rust
                            && matches!(record.kind, SymbolKindV2::Field | SymbolKindV2::Property)))
            });
            return self.uncertain(&members, true);
        }
        if rules == LanguageRules::Rust
            && let Some(path) = node
                .indexed_parent()
                .filter(|parent| parent.kind() == "scoped_identifier")
                .filter(|parent| parent.child_by_field_name("name").is_some_and(|own| own.id() == node.id()))
                .and_then(|parent| parent.child_by_field_name("path"))
        {
            return self.resolve_rust_path(path, name, scopes);
        }
        if self.pattern_reads.unknown.contains(&(node.start_byte(), node.end_byte())) {
            // A bare pattern name that may bind or read a constant.
            return self.uncertain(&self.item_records(name), false);
        }
        if path.is_some() {
            // A qualified name whose qualifier no rule establishes, or a
            // rooted one (`::f`), which lexical lookup must never reach: the
            // root names no local, whatever shadows it.
            return self.uncertain(&self.item_records(name), false);
        }

        match scopes.lookup(node, name, rules.reference_category(node)) {
            Lookup::Found(bindings)
                if bindings.len() == 1 && scopes.binding(bindings[0]).target == BindingTarget::Unknown =>
            {
                // Either the bare pattern's own binding (whatever value it
                // holds) or the constant an import or glob supplies.
                self.uncertain(&self.item_records(name), true)
            }
            Lookup::Found(bindings) if bindings.len() == 1 => {
                let binding = scopes.binding(bindings[0]);
                Resolution {
                    confidence: Confidence::Scoped,
                    id: match binding.target {
                        BindingTarget::Record(idx) => Some(self.records[idx].id.clone()),
                        BindingTarget::Import | BindingTarget::Unreported | BindingTarget::Unknown => None,
                    },
                    candidates: Vec::new(),
                    indirect: !binding.rule.callable,
                    import: (binding.target == BindingTarget::Import).then_some(binding.name_start),
                }
            }
            Lookup::Found(bindings) => {
                let mut records: Vec<usize> = bindings
                    .iter()
                    .filter_map(|&idx| match scopes.binding(idx).target {
                        BindingTarget::Record(record) => Some(record),
                        _ => None,
                    })
                    .collect();
                // Bindings come in lookup order, not candidate order.
                records.sort_by(|&left, &right| {
                    Self::candidate_key(self.records, left).cmp(&Self::candidate_key(self.records, right))
                });
                self.uncertain(&records, false)
            }
            Lookup::Glob | Lookup::NotFound => self.uncertain(&self.item_records(name), false),
        }
    }

    /// `Self::f`/`Type::f`: `Scoped` only when the path is `Self` in an
    /// inherent or trait `impl` of one in-file type, or a name that lookup
    /// establishes as one in-file type, and exactly one inherent `impl` of
    /// that type (or the enum itself, for a variant) declares `f`. Calling a
    /// value member or candidate (`Self::C()` with `const C: fn()`) is
    /// indirect either way.
    fn resolve_rust_path(&self, path: Node<'tree>, name: &str, scopes: &ScopeTree<'tree>) -> Resolution {
        let owner = match path.kind() {
            "identifier" if self.text(path) == "Self" => ancestors(path)
                .find(|ancestor| ancestor.kind() == "impl_item")
                .and_then(|impl_item| self.rust_impl_type(impl_item, scopes)),
            "identifier" | "type_identifier" => self.rust_type_record(path, scopes),
            _ => None,
        };
        let members = owner
            .map(|owner| {
                self.records_named(name, |idx| {
                    let definition = &self.syntax.definitions[idx];
                    definition.scope == SymbolScope::Member && self.rust_member_of(definition, owner, scopes)
                })
            })
            .unwrap_or_default();
        if let [idx] = members[..] {
            return Resolution {
                confidence: Confidence::Scoped,
                id: Some(self.records[idx].id.clone()),
                candidates: Vec::new(),
                // `Self::C()` with `const C: fn()` binds `C` but calls
                // whatever `C` holds; the initializer is never followed.
                indirect: holds_a_value(self.syntax.definitions[idx].kind),
                import: None,
            };
        }
        let candidates = if members.is_empty() {
            self.records_named(name, |idx| self.syntax.definitions[idx].scope != SymbolScope::Local)
        } else {
            members
        };
        self.uncertain(&candidates, false)
    }

    /// The record of the one in-file type `path` establishes.
    fn rust_type_record(&self, path: Node<'tree>, scopes: &ScopeTree<'tree>) -> Option<usize> {
        let Lookup::Found(bindings) = scopes.lookup(path, self.text(path), super::scope::TYPE) else {
            return None;
        };
        let [binding] = bindings.as_slice() else { return None };
        match scopes.binding(*binding).target {
            BindingTarget::Record(idx) if matches!(self.syntax.definitions[idx].kind, SymbolKind::Type | SymbolKind::Enum) => {
                Some(idx)
            }
            _ => None,
        }
    }

    /// The type record an `impl`'s self type establishes, looked up around
    /// the `impl` (its own generic parameters included).
    fn rust_impl_type(&self, impl_item: Node<'tree>, scopes: &ScopeTree<'tree>) -> Option<usize> {
        let mut self_type = impl_item.child_by_field_name("type")?;
        if self_type.kind() == "generic_type" {
            self_type = self_type.child_by_field_name("type")?;
        }
        (self_type.kind() == "type_identifier").then_some(())?;
        let Lookup::Found(bindings) = scopes.lookup(self_type, self.text(self_type), super::scope::TYPE) else {
            return None;
        };
        let [binding] = bindings.as_slice() else { return None };
        match scopes.binding(*binding).target {
            BindingTarget::Record(idx) if matches!(self.syntax.definitions[idx].kind, SymbolKind::Type | SymbolKind::Enum) => {
                Some(idx)
            }
            _ => None,
        }
    }

    /// Whether `member` is declared by an inherent `impl` of the type record
    /// `owner`, or is a variant of the enum `owner`.
    fn rust_member_of(&self, member: &DefinitionNodes<'tree>, owner: usize, scopes: &ScopeTree<'tree>) -> bool {
        if let Some(impl_item) = member.container_impl {
            return impl_item.child_by_field_name("trait").is_none()
                && self.rust_impl_type(impl_item, scopes) == Some(owner);
        }
        let owner_node = self.syntax.definitions[owner].anchor;
        member.kind == SymbolKind::EnumVariant
            && ancestors(member.declaration).any(|ancestor| ancestor.id() == owner_node.id())
    }

    /// Records called `name` that pass `keep`, in candidate order.
    fn records_named(&self, name: &str, keep: impl Fn(usize) -> bool) -> Vec<usize> {
        self.by_name
            .get(super::name_identity::key(self.syntax.language, name).as_ref())
            .map(|indexes| indexes.iter().copied().filter(|&idx| keep(idx)).collect())
            .unwrap_or_default()
    }

    /// The item-level records called `name`: the candidates of a supported
    /// name that no visible binding establishes.
    fn item_records(&self, name: &str) -> Vec<usize> {
        self.records_named(name, |idx| self.syntax.definitions[idx].scope == SymbolScope::Item)
    }
}

/// A supported language's answer for one reference.
struct Resolution {
    confidence: Confidence,
    id: Option<SymbolId>,
    candidates: Vec<SymbolRef>,
    /// Calling the target does not invoke one known declaration: a method
    /// on a receiver, or whatever a variable, constant or other value
    /// holds, whether the value is established or only a candidate.
    indirect: bool,
    /// The name start of the import binding a `Scoped` target.
    import: Option<usize>,
}

/// Whether calling a declaration of `kind` invokes whatever value it holds
/// (a function pointer or closure in a constant, static, variable,
/// parameter, field or property) instead of one callable declaration.
///
/// Functions, methods and constructors are invoked themselves; an enum
/// variant or tuple struct constructs; any other type-like kind, an
/// associated `type` included, names a type, which holds no value to
/// invoke. `Unknown` is treated as a value, since nothing proves otherwise.
fn holds_a_value(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Constant
            | SymbolKind::Variable
            | SymbolKind::Parameter
            | SymbolKind::Field
            | SymbolKind::Property
            | SymbolKind::Unknown
    )
}

/// The bare pattern names ([`BindingSite::bare_pattern`]) that bind no
/// established name. Both kinds are occurrences, never declarations.
#[derive(Default)]
struct PatternReads {
    /// Names that read a constant, unit struct or `const` parameter in view
    /// (or the prelude's `None`): resolved by lookup like any reference.
    references: HashSet<(usize, usize)>,
    /// Names whose identity the file cannot establish (a refutable pattern
    /// an import or glob may supply a constant to): uncertain, and so is
    /// every reference that reaches their [`BindingTarget::Unknown`]
    /// binding.
    unknown: HashSet<(usize, usize)>,
}

impl PatternReads {
    fn contains(&self, range: &(usize, usize)) -> bool {
        self.references.contains(range) || self.unknown.contains(range)
    }
}

/// What a bare pattern name is.
enum PatternMeaning {
    Binds,
    Reads,
    Unknown,
}

/// The scope tree of a supported language: the locals query's scopes plus
/// the walk's extra scopes, globs and dynamic name sources, no bindings yet.
fn scope_tree<'tree>(
    rules: LanguageRules,
    root: Node<'tree>,
    scope_nodes: &[Node<'tree>],
    walk: &Walk<'tree>,
) -> ScopeTree<'tree> {
    let scope_nodes = scope_nodes.iter().chain(&walk.extra_scopes).copied();
    let mut tree = ScopeTree::new(rules, root, scope_nodes);
    for &glob in &walk.globs {
        tree.add_glob(glob);
    }
    for &dynamic in &walk.dynamics {
        tree.add_dynamic(dynamic);
    }
    tree
}

/// Finishes the binding of the walk's sites ([`place_sites`]): each site
/// binding a record's name targets that record. Then places each record the
/// walk did not find (a capture-only declaration), by its kind.
fn bind_all<'tree>(
    tree: &mut ScopeTree<'tree>,
    syntax: &OccurrenceSyntax<'tree>,
    rules: LanguageRules,
    walk: &Walk<'tree>,
    placed: PlacedSites,
) -> PatternReads {
    let records_by_name: HashMap<(usize, usize), usize> = syntax
        .definitions
        .iter()
        .enumerate()
        .map(|(idx, definition)| ((definition.name.start_byte(), definition.name.end_byte()), idx))
        .collect();
    let range = |node: Node<'_>| (node.start_byte(), node.end_byte());
    for (binding, name_range) in placed.targeted {
        if let Some(&idx) = records_by_name.get(&name_range) {
            tree.set_target(binding, BindingTarget::Record(idx));
        }
    }
    let reads = placed.reads;
    let mut placed: HashSet<(usize, usize)> = walk.sites.iter().map(|site| range(site.name)).collect();
    for (idx, definition) in syntax.definitions.iter().enumerate() {
        if definition.scope == SymbolScope::Member || !placed.insert(range(definition.name)) {
            continue;
        }
        let text = syntax.source.get(definition.name.start_byte()..definition.name.end_byte()).unwrap_or_default();
        tree.bind(definition.name, text, record_rule(rules, definition.kind), BindingTarget::Record(idx));
    }
    reads
}

/// What [`place_sites`] decided.
struct PlacedSites {
    reads: PatternReads,
    /// The bindings whose target `place_sites` asked for, each with its name
    /// span: binding index, span.
    targeted: Vec<(usize, (usize, usize))>,
}

/// Places the walk's binding sites: the established ones first, then the
/// bare pattern names, each decided by looking its name up where it is
/// written ([`bare_pattern_meaning`]), outer patterns first, so a nested
/// pattern sees the outer pattern's binding. `target` names what a binding
/// at a name span is; no decision here reads it, so [`bind_all`] may set
/// the targets afterwards. Declaration extraction runs the same placement
/// ([`rust_pattern_non_bindings`]), so a record and an occurrence never
/// disagree about a pattern name.
fn place_sites<'tree>(
    tree: &mut ScopeTree<'tree>,
    walk: &Walk<'tree>,
    source: &'tree str,
    target: impl Fn((usize, usize)) -> BindingTarget,
) -> PlacedSites {
    let text = |node: Node<'_>| source.get(node.start_byte()..node.end_byte()).unwrap_or_default();
    let range = |node: Node<'_>| (node.start_byte(), node.end_byte());
    let (bare, established): (Vec<&BindingSite<'tree>>, Vec<&BindingSite<'tree>>) =
        walk.sites.iter().partition(|site| site.bare_pattern.is_some());
    let mut placed: HashSet<(usize, usize)> = bare.iter().map(|site| range(site.name)).collect();
    let mut targeted = Vec::new();
    for site in established {
        if !placed.insert(range(site.name)) {
            continue;
        }
        let bound = if site.is_import {
            BindingTarget::Import
        } else {
            targeted.push((tree.binding_count(), range(site.name)));
            target(range(site.name))
        };
        tree.bind(site.name, text(site.name), site.rule, bound);
    }
    let mut reads = PatternReads::default();
    let mut decided: HashSet<(usize, usize)> = HashSet::new();
    for site in bare {
        let name_range = range(site.name);
        if !decided.insert(name_range) {
            continue;
        }
        let name = text(site.name);
        match bare_pattern_meaning(tree, *site, name) {
            PatternMeaning::Binds => {
                targeted.push((tree.binding_count(), name_range));
                tree.bind(site.name, name, site.rule, target(name_range));
            }
            PatternMeaning::Reads => {
                reads.references.insert(name_range);
            }
            PatternMeaning::Unknown => {
                reads.unknown.insert(name_range);
                tree.bind(site.name, name, site.rule, BindingTarget::Unknown);
            }
        }
    }
    PlacedSites { reads, targeted }
}

/// The name spans of the Rust pattern names that declare nothing: a bare
/// pattern name that reads a constant, unit struct or `const` parameter,
/// or whose identity the file cannot establish in a refutable pattern
/// ([`bare_pattern_meaning`]). The locals query captures some of them
/// (`C` in `Some(C)`), and declaration extraction drops those captures.
/// `scope_nodes` are the locals query's `@local.scope` captures. Also
/// returns the placement, which the bind pass of the same file finishes
/// ([`RustPatterns`]).
pub(crate) fn rust_pattern_non_bindings<'tree>(
    root: Node<'tree>,
    source: &'tree str,
    scope_nodes: &[Node<'tree>],
) -> (HashSet<(usize, usize)>, RustPatterns<'tree>) {
    let rules = LanguageRules::Rust;
    let walk = Walk::run(root, ProgrammingLanguage::Rust, Some(rules), source);
    let mut tree = scope_tree(rules, root, scope_nodes, &walk);
    let placed = place_sites(&mut tree, &walk, source, |_| BindingTarget::Unreported);
    let non_bindings = placed.reads.references.iter().chain(&placed.reads.unknown).copied().collect();
    (non_bindings, RustPatterns { walk, tree, placed })
}

/// rustc's reading of an identifier pattern written without `mut`, `ref`
/// or `@`: it names the constant, unit struct or `const` parameter the name
/// reaches in the value namespace where it is written (a local, a function
/// or nothing at all leaves it a new binding), whatever its spelling. An
/// import or glob reached first may supply a constant: unknown in a
/// refutable pattern, a binding in an irrefutable one ([`Refutability`]).
/// `Self` (a keyword) and the prelude's unit variant `None` never bind.
fn bare_pattern_meaning(tree: &ScopeTree<'_>, site: BindingSite<'_>, name: &str) -> PatternMeaning {
    let unestablished = match site.bare_pattern {
        Some(Refutability::Refutable) => PatternMeaning::Unknown,
        _ => PatternMeaning::Binds,
    };
    if name == "Self" {
        return PatternMeaning::Reads;
    }
    match tree.lookup(site.name, name, super::scope::VALUE) {
        Lookup::Found(bindings) => {
            let reads = |idx: &usize| tree.binding(*idx).rule.pattern_constant;
            if bindings.iter().all(reads) {
                PatternMeaning::Reads
            } else if bindings.iter().any(|idx| {
                reads(idx) || matches!(tree.binding(*idx).target, BindingTarget::Import | BindingTarget::Unknown)
            }) {
                unestablished
            } else {
                PatternMeaning::Binds
            }
        }
        Lookup::Glob => unestablished,
        Lookup::NotFound if name == "None" => PatternMeaning::Reads,
        Lookup::NotFound => PatternMeaning::Binds,
    }
}

/// A name the locals query captures as a declaration but that binds
/// nothing: the path of a Rust tuple-struct or struct pattern (`Some` in
/// `Some(x)`), and a TypeScript parameter's default value (`a` in
/// `b = a`), which is a reference. Declaration extraction drops these
/// captures, so no record and no binding exists for them.
pub(crate) fn is_false_capture(rules: LanguageRules, name: Node<'_>) -> bool {
    let Some(parent) = name.indexed_parent() else { return false };
    let field = match (rules, parent.kind()) {
        (LanguageRules::Rust, "tuple_struct_pattern" | "struct_pattern") => "type",
        (LanguageRules::Ecma { .. }, "required_parameter" | "optional_parameter") => "value",
        _ => return false,
    };
    parent.child_by_field_name(field).is_some_and(|child| child.id() == name.id())
}

/// The rule of a declaration known only from its record's kind.
fn record_rule(rules: LanguageRules, kind: SymbolKind) -> BindingRule {
    use super::scope::{MACRO, Placement, TYPE, VALUE};
    let categories = match (rules, kind) {
        (LanguageRules::Ecma { typed: false }, _) => VALUE,
        (_, SymbolKind::Macro) => MACRO,
        (LanguageRules::Rust, kind) if kind.is_type() || matches!(kind, SymbolKind::Module | SymbolKind::Namespace) => TYPE,
        (LanguageRules::Ecma { .. }, SymbolKind::Interface | SymbolKind::TypeAlias) => TYPE,
        (LanguageRules::Ecma { .. }, kind) if kind.is_type() || matches!(kind, SymbolKind::Module | SymbolKind::Namespace) => {
            TYPE | VALUE
        }
        _ => VALUE,
    };
    BindingRule {
        categories,
        placement: Placement::Innermost,
        visible_from: 0,
        visible_until: None,
        local: rules == LanguageRules::Rust && matches!(kind, SymbolKind::Variable | SymbolKind::Parameter),
        ordered: false,
        callable: kind.is_function() || kind == SymbolKind::Class,
        pattern_constant: rules == LanguageRules::Rust && kind == SymbolKind::Constant,
    }
}

/// Code that holds no occurrences: imports (their names are import
/// records), attributes, and Rust lifetimes and loop labels (`'_`, `'outer`).
/// `_` in a language where it is a wildcard keyword that no declaration can
/// bind (`_ = f()` discards), so it never refers to anything. Elsewhere
/// (Python, JS/TS, Lua, C/C++, C#, and Java before 9) `_` is an ordinary name a declaration can
/// bind, and a use of it stays an occurrence.
fn is_wildcard(name: &str, language: ProgrammingLanguage) -> bool {
    name == "_"
        && matches!(
            language,
            ProgrammingLanguage::Rust
                | ProgrammingLanguage::Go
                | ProgrammingLanguage::Swift
                | ProgrammingLanguage::Scala
        )
}

/// Whether nothing inside a node of `kind` is an occurrence: a Rust `use`
/// or `extern crate`, a TS/JS `import` statement, an attribute, a lifetime
/// or a label. Python shares the `import_statement` kind, but a Python
/// import's module path segments other than the bound name are reads.
fn is_excluded_context(kind: &str, language: ProgrammingLanguage) -> bool {
    match kind {
        "import_statement" => matches!(language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript),
        _ => matches!(
            kind,
            "use_declaration" | "extern_crate_declaration" | "attribute_item" | "inner_attribute_item" | "lifetime" | "label"
        ),
    }
}

/// Whether the TS/JS token `node` names a module export rather than reading
/// a binding: anything in a re-export from another module (`export { k as
/// j } from "m"`, `export * as ns from "m"`, `export { default } from "m"`,
/// `export type { T } from "m"`), which names that module's exports like an
/// `import` statement does; the exported name of a local export (`j` in
/// `export { k as j }`); and the global name of `export as namespace X`. A
/// local export's own name (`k` in `export { k }`, `export = k`) reads `k`.
pub(crate) fn is_ecma_export_name(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    if !matches!(language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript) {
        return false;
    }
    let Some(statement) = ancestors(node).find(|ancestor| ancestor.kind() == "export_statement") else {
        return false;
    };
    let Some(parent) = node.indexed_parent() else { return false };
    names_module_export(node, parent, statement, statement.child_by_field_name("source").is_some())
}

/// [`is_ecma_export_name`] for `node`, whose parent is `parent` and whose
/// nearest `export_statement` ancestor is `statement` (`from_source` when
/// it has a `source`). [`Walk::run`] decides it for every node from its
/// path; the occurrence and lint readers decide it the same way.
fn names_module_export(node: Node<'_>, parent: Node<'_>, statement: Node<'_>, from_source: bool) -> bool {
    if from_source {
        return true;
    }
    if parent.kind() == "export_specifier" {
        return parent.child_by_field_name("alias").is_some_and(|alias| alias.id() == node.id());
    }
    // `export as namespace X`: the grammar writes `as` and `namespace` as
    // anonymous tokens beside `X`.
    parent.id() == statement.id() && {
        let mut cursor = statement.walk();
        statement.children(&mut cursor).any(|child| child.kind() == "namespace")
    }
}

/// Parent kinds that write a qualifier or receiver before a name.
const QUALIFYING_KINDS: &[&str] = &[
    "scoped_identifier",
    "scoped_type_identifier",
    "field_expression",
    "member_expression",
    "nested_type_identifier",
    "nested_identifier",
    "attribute",
    "selector_expression",
    "qualified_identifier",
    "qualified_type",
    "member_access_expression",
    "field_access",
    "method_invocation",
    "scoped_call_expression",
    "member_call_expression",
    "nullsafe_member_call_expression",
    "nullsafe_member_access_expression",
    "class_constant_access_expression",
    "scoped_property_access_expression",
    "dot_index_expression",
    "method_index_expression",
    "navigation_expression",
    "qualified_name",
    "alias_qualified_name",
    "call_expression_with_bareword",
];

/// The fields that hold a qualifier or receiver in [`QUALIFYING_KINDS`].
const QUALIFIER_FIELDS: &[&str] = &[
    "path",
    "value",
    "object",
    "operand",
    "scope",
    "module",
    "expression",
    "table",
    "argument",
    "prefix",
    "qualifier",
    "alias",
    "package",
    "package_name",
    "object_return_value",
];

/// Path kinds whose segments carry no field: Java `a.b.C`, Scala `a.b.C`
/// (types and stable identifiers), Swift `A.B` types, PHP `X::K`, Python
/// module paths (`a.b` in `from a.b import c`) and class patterns. Every
/// segment after the first is qualified by the ones before it.
const SEGMENTED_KINDS: &[&str] = &[
    "dotted_name",
    "scoped_type_identifier",
    "stable_type_identifier",
    "stable_identifier",
    "user_type",
    "class_constant_access_expression",
];

/// The node `node` is the name part of, one step out: a path or member
/// access whose qualifier, receiver or root (`::f` in Rust and C++) is
/// written before `node`, the C# conditional access around the
/// `member_binding_expression` (`.F`) that holds `node` (`obj?.F`), or a
/// Java construction whose enclosing-instance receiver is written before
/// `new` (`outer.new C()`, [`construction_receiver`]) and whose type is
/// `node`.
fn qualified_parent(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.indexed_parent()?;
    let holds_name = || parent.child_by_field_name("name").is_some_and(|name| name.id() == node.id());
    match parent.kind() {
        "object_creation_expression" => parent
            .child_by_field_name("type")
            .filter(|constructed| constructed.id() == node.id())
            .and(construction_receiver(parent))
            .map(|_| parent),
        // Swift holds a member name in a `navigation_suffix` below its
        // `navigation_expression`.
        "navigation_suffix" => parent
            .child_by_field_name("suffix")
            .filter(|suffix| suffix.id() == node.id())
            .and(parent.indexed_parent())
            .filter(|navigation| navigation.kind() == "navigation_expression"),
        // PHP holds a static property's name in a `variable_name`
        // (`X::$f`).
        "variable_name" => parent.indexed_parent().filter(|access| {
            access.kind() == "scoped_property_access_expression"
                && access.child_by_field_name("name").is_some_and(|name| name.id() == parent.id())
        }),
        "member_binding_expression" if holds_name() => parent
            .indexed_parent()
            .filter(|access| access.kind() == "conditional_access_expression")
            .filter(|access| access.child_by_field_name("condition").is_some_and(|condition| condition.id() != parent.id())),
        kind if QUALIFYING_KINDS.contains(&kind) => {
            let mut cursor = parent.walk();
            let qualifiers: Vec<Node<'_>> = QUALIFIER_FIELDS
                .iter()
                .flat_map(|field| parent.children_by_field_name(field, &mut cursor).collect::<Vec<_>>())
                .collect();
            // A qualifier is never the name part: PHP writes each of `\A\B\f`'s
            // `\` and `A\B` as a `prefix`.
            let is_qualifier = qualifiers.iter().any(|qualifier| qualifier.id() == node.id());
            let qualified = qualifiers.iter().any(|qualifier| qualifier.end_byte() <= node.start_byte());
            let segmented = SEGMENTED_KINDS.contains(&kind) && field_less_segment_follows(parent, node);
            (!is_qualifier && (qualified || segmented || (holds_name() && is_rooted(parent, node)))).then_some(parent)
        }
        kind if SEGMENTED_KINDS.contains(&kind) => field_less_segment_follows(parent, node).then_some(parent),
        _ => None,
    }
}

/// The receiver a Java qualified construction writes before `new`
/// (`outer`, `(outer)`, `this`, `factory()`, `holder.outer` in
/// `….new C()`). The pinned grammar holds it as an unfielded child; Java,
/// C# and PHP constructions without one start with `new`.
fn construction_receiver(construction: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = construction.walk();
    construction
        .children(&mut cursor)
        .take_while(|child| child.kind() != "new")
        .find(|child| child.is_named() && !child.kind().contains("comment"))
}

/// Whether `node` follows another named segment of the field-less path
/// `path` ([`SEGMENTED_KINDS`]).
fn field_less_segment_follows(path: Node<'_>, node: Node<'_>) -> bool {
    first_operand(path).is_some_and(|first| first.id() != node.id() && first.end_byte() <= node.start_byte())
}

/// Whether `path` writes a root `::` before its name `name` and no
/// qualifier: Rust `::f`, C++ `::f`.
fn is_rooted(path: Node<'_>, name: Node<'_>) -> bool {
    let mut cursor = path.walk();
    path.children(&mut cursor)
        .take_while(|child| child.end_byte() <= name.start_byte())
        .any(|child| !child.is_named() && child.kind() == "::")
}

/// The written path or receiver expression ending at `node` (`Self::new`,
/// `other.run`, `::f`, `A::B::f`, `r4::f` in `r4::f<int>()`, `obj?.A?.Invoke`,
/// `R4->r4f`, `outer.new C`), when `node` is qualified: the source from the
/// outermost qualifier through `node`. Type arguments after `node` are
/// outside it; those inside a qualifier (`N::C<int>::member`) stay, and so
/// does Java's `new` between a receiver and the constructed type. A Swift
/// prefix operator applies to the whole member chain, so it is outside it
/// ([`qualifier_start`]).
///
/// The climb crosses generic instantiations of `node` ([`path_wrapper`]),
/// every path or access `node` is the name part of ([`qualified_parent`]),
/// and a C++ scope whose path is itself the name of an outer one
/// (`B` in `A::B::f`, written right-nested), so an outer namespace is never
/// lost.
fn written_path(node: Node<'_>, source: &str, language: ProgrammingLanguage) -> Option<String> {
    if let Some(start) = php_namespace_segment_start(node) {
        return source.get(start..node.end_byte()).map(str::to_string);
    }
    if language == ProgrammingLanguage::Scala
        && let Some(first) = scala_import_path_start(node)
    {
        return (first.id() != node.id()).then(|| source.get(first.start_byte()..node.end_byte()).map(str::to_string)).flatten();
    }
    let mut start = None;
    let mut current = node;
    loop {
        if let Some(wrapper) = path_wrapper(current, language) {
            current = wrapper;
        } else if let Some(qualified) = qualified_parent(current) {
            start = qualifier_start(qualified, language).or(start);
            current = qualified;
        } else if let Some(scoped) = right_nested_scope(current, language) {
            current = scoped;
        } else {
            break;
        }
    }
    source.get(start?..node.end_byte()).map(str::to_string)
}

/// Where the written path of a PHP namespace segment (`A`, `B` in
/// `\A\B\f()`) starts: at the root `\` when one is written, else at the
/// first segment, for every segment after it.
fn php_namespace_segment_start(node: Node<'_>) -> Option<usize> {
    let namespace = node.indexed_parent().filter(|parent| parent.kind() == "namespace_name")?;
    let rooted = namespace.indexed_parent().filter(|path| path.kind() == "qualified_name").filter(|path| {
        let mut cursor = path.walk();
        path.children(&mut cursor)
            .take_while(|child| child.end_byte() <= namespace.start_byte())
            .any(|child| child.kind() == "\\")
    });
    match rooted {
        Some(path) => Some(path.start_byte()),
        None => field_less_segment_follows(namespace, node).then_some(namespace.start_byte()),
    }
}

/// Where the path `qualified` writes its qualifier: its own start, except a
/// Perl bareword call, whose sigil (`&R4::r4f`) is not part of the path,
/// a Swift member chain the grammar starts with a prefix operator
/// ([`swift_chain_start`]) and a Python attribute chain it starts with an
/// unpacking star ([`python_chain_start`]). `None` when `qualified` writes no qualifier
/// after all: a Swift implicit member behind an operator (`some` in
/// `-.some`).
fn qualifier_start(qualified: Node<'_>, language: ProgrammingLanguage) -> Option<usize> {
    match (language, qualified.kind()) {
        (_, "call_expression_with_bareword") => Some(
            qualified
                .child_by_field_name("package_name")
                .map_or(qualified.start_byte(), |package| package.start_byte()),
        ),
        (ProgrammingLanguage::Swift, "navigation_expression") => swift_chain_start(qualified),
        (ProgrammingLanguage::Python, "attribute") => Some(python_chain_start(qualified)),
        _ => Some(qualified.start_byte()),
    }
}

/// Where the Python attribute chain `attribute` starts once a leading
/// unpacking star is set aside. In an assignment target the pinned grammar
/// writes `[*x.s] = r` as `(*x).s`, though the star unpacks the whole
/// target `x.s` (as it does in `f(*x.s)`, which the grammar nests
/// correctly), so the chain starts at the star's operand.
fn python_chain_start(attribute: Node<'_>) -> usize {
    let mut leftmost = attribute;
    loop {
        let next = match leftmost.kind() {
            "attribute" => leftmost.child_by_field_name("object"),
            "call" => leftmost.child_by_field_name("function"),
            "subscript" => leftmost.child_by_field_name("value"),
            "list_splat" | "dictionary_splat" => return first_operand(leftmost).map_or(attribute.start_byte(), |operand| operand.start_byte()),
            _ => None,
        };
        match next {
            Some(node) => leftmost = node,
            None => return attribute.start_byte(),
        }
    }
}

/// Where the Swift member chain `navigation` starts once a leading prefix
/// operator is set aside. The pinned grammar writes `!x.isEmpty()` as
/// `(!x).isEmpty()`, but a Swift prefix operator binds looser than member
/// access, calls, subscripts and postfix `!`/`?`, so it applies to the whole
/// chain and is no qualifier: the chain starts at the operator's operand
/// (`x`, also below `!-x`). Before an implicit member the grammar writes
/// the operator as the navigation's own anonymous `target` (`-.some.f()`):
/// the chain starts at that member's `.`, and the member itself has no
/// qualifier (`None`), as without the operator. Written parentheses
/// (`(!x).f`) are a `tuple_expression` and stay, as does an implicit
/// member's `.` (`.some.f`).
fn swift_chain_start(navigation: Node<'_>) -> Option<usize> {
    let is_operator = |node: Node<'_>| {
        node.kind() == "prefix_expression"
            && node.child_by_field_name("operation").is_some_and(|operation| operation.kind() != ".")
    };
    let mut leftmost = navigation;
    loop {
        let next = match leftmost.kind() {
            "navigation_expression" => {
                let target = leftmost.child_by_field_name("target");
                // `!` is the grammar's one named operator token.
                if let Some(operator) = target.filter(|target| !target.is_named() || target.kind() == "bang") {
                    return (leftmost.id() != navigation.id())
                        .then(|| operator.next_sibling())
                        .flatten()
                        .map(|member| member.start_byte());
                }
                target
            }
            "postfix_expression" => leftmost.child_by_field_name("target"),
            "call_expression" => first_operand(leftmost),
            _ if is_operator(leftmost) => {
                let mut operand = leftmost;
                while is_operator(operand) {
                    operand = operand.child_by_field_name("target")?;
                }
                return Some(operand.start_byte());
            }
            _ => None,
        };
        match next {
            Some(node) => leftmost = node,
            None => return Some(navigation.start_byte()),
        }
    }
}

/// The generic form `node` is the name of, for the written path: every
/// [`generic_instantiation`], types included (`C` in `r4::C<int> v`).
fn path_wrapper<'tree>(node: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let parent = node.indexed_parent()?;
    generic_instantiation(node, parent, language)
}

/// The C++ path `node` is the scope of, when that path is itself the name
/// of an outer path: the grammar writes `A::B::f` as `A::(B::f)`, so `B`'s
/// qualifier `A` sits one path further out.
fn right_nested_scope<'tree>(node: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    if language != ProgrammingLanguage::Cpp {
        return None;
    }
    let path = node.indexed_parent().filter(|parent| parent.kind() == "qualified_identifier")?;
    path.child_by_field_name("scope").filter(|scope| scope.id() == node.id())?;
    path.indexed_parent()
        .filter(|outer| outer.kind() == "qualified_identifier")
        .and_then(|outer| outer.child_by_field_name("name"))
        .filter(|name| name.id() == path.id())
        .map(|_| path)
}

/// Whether `node` names the target of an invocation: a call's function, a
/// method name, a constructed type (`new`, Swift `C<Int>()`, C++ braced
/// `C{}`) or an applied decorator, directly or as the name part of a path
/// (rooted ones included), receiver expression, conditional access
/// ([`qualified_parent`]) or generic instantiation of a function or a
/// constructed type ([`generic_instantiation`]), a Swift implicit member
/// or Scala mixin base ([`implied_operand`]), and through wrappers that
/// only restate their operand
/// ([`transparent_wrapper`]), or a PHP variable whose value is invoked
/// (`$cb()`, `new $C()`). A receiver, an argument, an operand of a
/// sequence or conditional, a dereference and a subscripted operand (Swift
/// `a[0]`, [`is_swift_subscript`]) are never the target, nor is the callee
/// of PHP callable creation (`f(...)`), which calls nothing.
fn is_call_target(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> bool {
    match language {
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => return is_shell_command_name(node),
        ProgrammingLanguage::Perl => return is_perl_invocation(node, source),
        _ => {}
    }
    let mut target = php_variable(node, language).unwrap_or(node);
    loop {
        while let Some(wrapper) = transparent_wrapper(target, language) {
            target = wrapper;
        }
        let Some(parent) = target.indexed_parent() else { return false };
        let holds = |field: &str| parent.child_by_field_name(field).is_some_and(|child| child.id() == target.id());
        let is_first = || first_operand(parent).is_some_and(|first| first.id() == target.id());
        let invoked = match parent.kind() {
            "call_expression" if language == ProgrammingLanguage::Swift && is_swift_subscript(parent) => false,
            "call_expression" | "call" | "invocation_expression" | "function_call_expression" => {
                holds("function") || (parent.child_by_field_name("function").is_none() && is_first())
            }
            // A C++ array `new C[n]` writes no constructor call, like Java
            // and C# array creation.
            "new_expression" => {
                holds("constructor") || (holds("type") && parent.child_by_field_name("declarator").is_none())
            }
            // Swift `C<Int>()`, `N.C<Int>()`; C++ braced construction
            // `Plain{}`, `N::C<int>{}` (a C compound literal constructs
            // nothing).
            "constructor_expression" => language == ProgrammingLanguage::Swift && holds("constructed_type"),
            "compound_literal_expression" => language == ProgrammingLanguage::Cpp && holds("type"),
            "object_creation_expression" => {
                holds("type") || (language == ProgrammingLanguage::Php && is_first())
            }
            "instance_expression" => language == ProgrammingLanguage::Scala && is_first(),
            // A PHP variable in the name position (`$o->$m()`) holds the
            // method's name, not a callable.
            "method_invocation"
            | "member_call_expression"
            | "nullsafe_member_call_expression"
            | "scoped_call_expression"
            | "function_call" => holds("name") && target.kind() != "variable_name",
            // `obj m 1` and `obj m` apply the method `m`.
            "infix_expression" => language == ProgrammingLanguage::Scala && holds("operator"),
            "postfix_expression" => {
                language == ProgrammingLanguage::Scala
                    && parent.named_child_count() > 1
                    && parent
                        .named_child(parent.named_child_count() as u32 - 1)
                        .is_some_and(|last| last.id() == target.id())
            }
            // `@f` calls `f` with the decorated declaration.
            "decorator" => {
                matches!(
                    language,
                    ProgrammingLanguage::Python | ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript
                ) && is_first()
            }
            _ => false,
        };
        if invoked {
            return !(language == ProgrammingLanguage::Php && is_php_callable_creation(parent));
        }
        if let Some(generic) = generic_instantiation(target, parent, language) {
            target = generic;
        } else if let Some(constructed) = implied_operand(target, parent, language) {
            target = constructed;
        } else if let Some(qualified) = qualified_parent(target) {
            target = qualified;
        } else {
            return false;
        }
    }
}

/// The form `node` is the invoked part of without being a path or an
/// instantiation of it, so it adds no written qualifier: a Swift implicit
/// member (`.f` in `.f()`, `.init()`, `.some(1)`), a Swift name an
/// operator is written before (`f` in `!f()`, `-f()`), and the class a
/// Scala mixin construction instantiates (`C` in `new C with T`).
fn implied_operand<'tree>(node: Node<'tree>, parent: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let holds = |field: &str| parent.child_by_field_name(field).is_some_and(|child| child.id() == node.id());
    let implied = match (language, parent.kind()) {
        // The pinned grammar writes `!f()` as `(!f)()`, but a Swift prefix
        // operator binds looser than a call, so it applies to the call's
        // result; written parentheses (`(-f)()`) call the operator's
        // result and are a `tuple_expression` in between.
        (ProgrammingLanguage::Swift, "prefix_expression") => {
            holds("target")
                && parent.child_by_field_name("operation").is_some_and(|operation| {
                    operation.kind() == "." || parent.indexed_parent().is_some_and(|call| call.kind() == "call_expression")
                })
        }
        (ProgrammingLanguage::Scala, "compound_type") => holds("base"),
        _ => false,
    };
    implied.then_some(parent)
}

/// Whether the Swift `call_expression` `call` is a subscript (`a[0]`,
/// `a?[0]`, `a[keyPath: k]`, `a[0] { … }`): tree-sitter-swift 0.7.1 writes
/// one with the same node as a call, its `value_arguments` bracketed.
/// A subscript invokes nothing, so its operand is never a call target.
fn is_swift_subscript(call: Node<'_>) -> bool {
    let mut cursor = call.walk();
    let Some(suffix) = call.named_children(&mut cursor).find(|child| child.kind() == "call_suffix") else {
        return false;
    };
    let mut cursor = suffix.walk();
    let Some(arguments) = suffix.named_children(&mut cursor).find(|child| child.kind() == "value_arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    arguments.children(&mut cursor).any(|token| token.kind() == "[")
}

/// The PHP variable `$cb` whose name token is `node`: a variable is the
/// callee of `$cb()` and `new $C()`, where its value is invoked.
fn php_variable<'tree>(node: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    (language == ProgrammingLanguage::Php)
        .then(|| node.indexed_parent())
        .flatten()
        .filter(|parent| parent.kind() == "variable_name")
}

/// Whether the PHP call-shaped `call` creates a closure instead of calling
/// (`f(...)`, `$o->m(...)`, `W::s(...)`, `$cb(...)`): its argument list is
/// the grammar's `variadic_placeholder`. Unpacking (`f(...$args)`) is an
/// `argument` and calls.
fn is_php_callable_creation(call: Node<'_>) -> bool {
    call.child_by_field_name("arguments").is_some_and(|arguments| {
        let mut cursor = arguments.walk();
        arguments.named_children(&mut cursor).any(|child| child.kind() == "variadic_placeholder")
    })
}

/// The first named child of `node` that is not a comment.
fn first_operand(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| !child.kind().contains("comment"))
}

/// The wrapper around `node` that evaluates to exactly `node`'s value, so
/// invoking the wrapper invokes `node`: parentheses holding nothing else
/// (Swift writes them as a one-element, unlabeled `tuple_expression`), the
/// TypeScript type-only forms `x as T`, `x satisfies T`, `<T>x` and `x!`,
/// the Swift force unwrap `x!` and the C# null-forgiving `x!`. Only the
/// exact `!` operator restates its operand: Swift optional chaining
/// (`x?()`), postfix `++`/`--` and the Swift infix operators `!!`/`?!`
/// the pinned grammar lexes from `cb!!()`/`cb?!()` are not wrappers.
///
/// PHP parentheses are transparent only around a variable (`($cb)()`,
/// `(($cb))()`): a bare name inside them is a constant, so `(f)()` calls the
/// constant's value, never function `f`, and `($o->m)()` calls a property's
/// value, never method `m`.
/// Casts, dereferences (`(*f)()`), sequences (`(a, f)()`) and conditionals
/// (`(c ? f : g)()`) compute a different value and are never wrappers.
fn transparent_wrapper<'tree>(node: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let parent = node.indexed_parent()?;
    wrapper_operand(parent, language).filter(|operand| operand.id() == node.id()).map(|_| parent)
}

/// The value `wrapper` restates, when it is one of
/// [`transparent_wrapper`]'s wrappers.
fn wrapper_operand<'tree>(wrapper: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let only_operand = || {
        let mut cursor = wrapper.walk();
        let mut operands = wrapper.named_children(&mut cursor).filter(|child| !child.kind().contains("comment"));
        let only = operands.next();
        if operands.next().is_some() { None } else { only }
    };
    match (language, wrapper.kind()) {
        // Only a variable's value passes through PHP parentheses: `($cb)()`.
        (ProgrammingLanguage::Php, "parenthesized_expression") => only_operand().filter(|operand| {
            operand.kind() == "variable_name"
                || (operand.kind() == "parenthesized_expression" && wrapper_operand(*operand, language).is_some())
        }),
        (_, "parenthesized_expression") => only_operand(),
        (ProgrammingLanguage::Swift, "tuple_expression") if wrapper.child_by_field_name("name").is_none() => only_operand(),
        (ProgrammingLanguage::TypeScript, "as_expression" | "satisfies_expression" | "non_null_expression") => {
            first_operand(wrapper)
        }
        (ProgrammingLanguage::TypeScript, "type_assertion") => wrapper
            .named_child((wrapper.named_child_count() as u32).saturating_sub(1))
            .filter(|operand| operand.kind() != "type_arguments"),
        // Swift force unwrap `cb!`: the grammar's one named operator token.
        // `?` in `cb?()` is a call's own token, and `++`/`--` are anonymous.
        (ProgrammingLanguage::Swift, "postfix_expression") => wrapper
            .child_by_field_name("operation")
            .filter(|operation| operation.kind() == "bang")
            .and_then(|_| wrapper.child_by_field_name("target")),
        // C# null-forgiving `cb!`, which shares its node with `i++`/`i--`.
        (ProgrammingLanguage::CSharp, "postfix_unary_expression") => {
            let mut cursor = wrapper.walk();
            let operator = wrapper.children(&mut cursor).filter(|child| !child.is_extra()).last();
            operator.filter(|operator| operator.kind() == "!").and_then(|_| only_operand())
        }
        _ => None,
    }
}

/// The expression a callee (or constructed operand) invokes: `callee`
/// with every transparent wrapper ([`wrapper_operand`]) and generic
/// instantiation ([`generic_operand`]) removed, at any depth. The inverse of
/// the climb [`is_call_target`] makes from a name, so a callee shape counts as
/// a direct call in coverage exactly when its name is a call there.
pub(crate) fn invoked_operand<'tree>(callee: Node<'tree>, language: ProgrammingLanguage) -> Node<'tree> {
    let mut operand = callee;
    while let Some(inner) = wrapper_operand(operand, language).or_else(|| generic_operand(operand, language)) {
        operand = inner;
    }
    operand
}

/// The member names invoked through a member access written inside
/// transparent wrappers or a generic instantiation ([`invoked_operand`]:
/// `((obj.m))()`, `(obj.m as T)()`, `(<T>obj.m)()`, `(obj.m<T>)()`,
/// Go `((pkg.F))()`, C++ `(obj.template m<T>)()`, `new ((ns.W))()`), which
/// no query pattern can reach at every depth. Direct member calls are
/// captured by the reference query; a duplicate span is merged by the
/// caller. Rust is absent: `(x.f)()` calls the field `f`, never a method.
pub(crate) fn wrapped_member_callees<'tree>(root: Node<'tree>, language: ProgrammingLanguage) -> Vec<Node<'tree>> {
    let callee_field: &[(&str, &str)] = match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            &[("call_expression", "function"), ("new_expression", "constructor")]
        }
        ProgrammingLanguage::Go | ProgrammingLanguage::C | ProgrammingLanguage::Cpp => &[("call_expression", "function")],
        _ => return Vec::new(),
    };
    let mut names = Vec::new();
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if let Some(&(_, field)) = callee_field.iter().find(|(kind, _)| *kind == node.kind())
            && let Some(callee) = node.child_by_field_name(field)
        {
            let operand = invoked_operand(callee, language);
            if operand.id() != callee.id() && let Some(name) = member_name(operand, language) {
                names.push(name);
            }
        }
        if cursor.goto_first_child() || cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                return names;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// The member name a member access `access` writes: TS/JS `obj.m`, Go
/// `x.m`, C/C++ `s.m`, `p->m`, `obj.template m<T>`.
fn member_name<'tree>(access: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let name = match (language, access.kind()) {
        (ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript, "member_expression") => {
            access.child_by_field_name("property")?
        }
        (ProgrammingLanguage::Go, "selector_expression") => access.child_by_field_name("field")?,
        (ProgrammingLanguage::C | ProgrammingLanguage::Cpp, "field_expression") => {
            let mut field = access.child_by_field_name("field")?;
            if field.kind() == "dependent_name" {
                field = first_operand(field)?;
            }
            if field.kind() == "template_method" {
                field = field.child_by_field_name("name")?;
            }
            field
        }
        _ => return None,
    };
    matches!(name.kind(), "property_identifier" | "private_property_identifier" | "field_identifier").then_some(name)
}

/// The generic instantiation `node` is the function or type part of: Rust
/// `f::<T>`, Scala `f[T]`, C++ `f<T>` (also `obj.template m<T>`), C# `f<T>`,
/// a TypeScript instantiation expression (`f<T>` or `obj.m<T>` written
/// apart from its call, as in `(f<T>)()`), and the generic types a
/// construction names: Java `C<String>` and `a.b.C<>`, Scala `C[Int]`,
/// C++ `C<int>`, Swift `C<Int>` (also with comments before the arguments; a one-segment `user_type`: in `N.C<Int>`
/// the path step reaches the same node). Go `f[T]` is not one: the grammar
/// writes it as an `index_expression`, the same node as `fs[0]`.
///
/// A generic type is only ever invoked where a construction holds it, so a
/// type annotation `C<T>` stays a usage.
fn generic_instantiation<'tree>(node: Node<'tree>, parent: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    generic_operand(parent, language).filter(|operand| operand.id() == node.id()).map(|_| parent)
}

/// The generic name `instantiation` instantiates, when it is one of
/// [`generic_instantiation`]'s instantiations.
fn generic_operand<'tree>(instantiation: Node<'tree>, language: ProgrammingLanguage) -> Option<Node<'tree>> {
    let field = match (language, instantiation.kind()) {
        (ProgrammingLanguage::Rust | ProgrammingLanguage::Scala, "generic_function") => "function",
        (ProgrammingLanguage::Cpp, "template_function" | "template_method" | "template_type") => "name",
        (ProgrammingLanguage::Scala, "generic_type") => "type",
        (ProgrammingLanguage::Java, "generic_type") => {
            return first_operand(instantiation).filter(|operand| operand.kind() != "type_arguments");
        }
        (ProgrammingLanguage::Swift, "user_type") => {
            let mut cursor = instantiation.walk();
            let mut segments = instantiation.named_children(&mut cursor).filter(|child| child.kind() == "type_identifier");
            let only = segments.next().filter(|_| segments.next().is_none())?;
            let generic = first_operand(instantiation).is_some_and(|first| first.id() == only.id())
                && next_named_syntax_sibling(only).is_some_and(|next| next.kind() == "type_arguments");
            return generic.then_some(only);
        }
        (ProgrammingLanguage::CSharp, "generic_name") | (ProgrammingLanguage::Cpp, "dependent_name") => {
            return first_operand(instantiation);
        }
        // The expression form has no field; the `typeof` query form writes
        // `function`.
        (ProgrammingLanguage::TypeScript, "instantiation_expression") => {
            return instantiation
                .child_by_field_name("function")
                .or_else(|| first_operand(instantiation))
                .filter(|operand| operand.kind() != "type_arguments");
        }
        _ => return None,
    };
    instantiation.child_by_field_name(field)
}

/// Whether `node` is a Bash/Zsh command's name (`f`, `X=1 f arg`, `$(f)`).
fn is_shell_command_name(node: Node<'_>) -> bool {
    let name = if node.kind() == "command_name" {
        Some(node)
    } else {
        node.indexed_parent().filter(|parent| parent.kind() == "command_name")
    };
    name.and_then(|name| name.indexed_parent().map(|command| (name, command)))
        .is_some_and(|(name, command)| {
            command.kind() == "command" && command.child_by_field_name("name").is_some_and(|held| held.id() == name.id())
        })
}

/// Whether `node` is the sub a Perl invocation calls: a bareword call in
/// any of its forms (`f()`, `f 1`, `f`, `&f()`, `&f`, `Pkg::f()`,
/// `goto &f`), a method name (`$obj->f()`, `Pkg->f()`), or the code
/// reference a variable call invokes (`$cb->()`, `$cb->(1)`, `&$cb()`,
/// `&{$cb}(1)`). A reference `\&f` / `\&$cb`, the existence tests
/// `defined &f` / `exists &f`, and the receiver of a named method
/// (`$cb` in `$cb->m()`) name the sub or value without calling it.
fn is_perl_invocation(node: Node<'_>, source: &str) -> bool {
    let Some(parent) = node.indexed_parent() else { return false };
    let holds = |field: &str| parent.child_by_field_name(field).is_some_and(|held| held.id() == node.id());
    match parent.kind() {
        // `$cb->(…)` is a `method_invocation` with no method name.
        "method_invocation" => {
            holds("function_name")
                || (holds("object_return_value")
                    && node.kind() == "scalar_variable"
                    && parent.child_by_field_name("function_name").is_none())
        }
        "call_expression_with_bareword" => holds("function_name") && !is_perl_sub_name_only(parent, source),
        "call_expression_with_variable" => node.kind() == "scalar_variable" && !is_perl_sub_name_only(parent, source),
        _ => false,
    }
}

/// Whether the bareword or variable call `call` only names its sub:
/// `\&f`, or `&f` as the argument of `defined` or `exists`.
fn is_perl_sub_name_only(call: Node<'_>, source: &str) -> bool {
    let Some(parent) = call.indexed_parent() else { return false };
    if parent.kind() == "to_reference" {
        return true;
    }
    let ampersand = source.get(call.start_byte()..).is_some_and(|text| text.starts_with('&'));
    ampersand
        && parent.kind() == "arguments"
        && parent
            .indexed_parent()
            .filter(|outer| outer.kind() == "call_expression_with_spaced_args")
            .and_then(|outer| outer.named_child(0))
            .and_then(|callee| callee.child_by_field_name("function_name"))
            .and_then(|callee| source.get(callee.start_byte()..callee.end_byte()))
            .is_some_and(|callee| matches!(callee, "defined" | "exists"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::OccurrenceSyntax;
    use crate::TreeFile;
    use crate::file::tree_file::{MetadataSelection, RecordFacts};
    use crate::shared::ReferenceEvidence;

    /// `tree_file`'s evidence with its scope captures and reference
    /// captures in the order `arrange` leaves them, walking the file again
    /// for the bind pass.
    fn evidence_with(
        tree_file: &TreeFile,
        arrange: impl FnOnce(&mut Vec<tree_sitter::Node<'_>>, &mut Vec<crate::file::tree_file::ReferenceCapture<'_>>),
    ) -> ReferenceEvidence {
        evidence_from(tree_file, false, arrange)
    }

    /// [`evidence_with`], finishing the definition pass's Rust pattern
    /// placement when `share_patterns`.
    fn evidence_from(
        tree_file: &TreeFile,
        share_patterns: bool,
        arrange: impl FnOnce(&mut Vec<tree_sitter::Node<'_>>, &mut Vec<crate::file::tree_file::ReferenceCapture<'_>>),
    ) -> ReferenceEvidence {
        let selection = MetadataSelection::for_kinds(Some(Vec::new()));
        let crate::file::tree_file::Definitions { symbols: definitions, mut scopes, rust_patterns, .. } =
            tree_file.definitions_with_scopes(&selection).expect("definitions");
        if share_patterns {
            assert!(rust_patterns.is_some(), "{}: no pattern placement to share", tree_file.file.display());
        }
        let imports = tree_file.imported_symbols().expect("imports");
        let mut captures = tree_file.reference_captures().expect("captures").expect("a reference query");
        arrange(&mut scopes, &mut captures);
        let syntax = OccurrenceSyntax::new(
            tree_file.language,
            tree_file.source(),
            tree_file.root_node(),
            &definitions,
            scopes,
            &imports,
            Some(captures),
        )
        .with_rust_patterns(rust_patterns.filter(|_| share_patterns));
        let records = tree_file
            .records_from(definitions, &selection, RecordFacts::default(), None)
            .expect("records");
        syntax.resolve(&records).evidence
    }

    /// The bind pass finishing the definition pass's pattern placement
    /// reports what a second walk reports: the same occurrences, targets
    /// (records, unknowns, imports) and coverage.
    #[test]
    fn shared_pattern_placement_gives_the_walked_evidence() {
        let dir = TempDir::new().expect("tempdir");
        for (name, source) in [
            (
                "patterns.rs",
                "use m::Imported;\nuse g::*;\nconst C: u8 = 1;\nstruct Unit;\n\
                 fn f<const N: usize>(v: Option<u8>, (a, b): (u8, u8)) -> u8 {\n\
                 \x20   let x = a + b;\n\
                 \x20   match v { Some(C) => C, Some(Imported) => x, Some(y) => y + x, None => N as u8 }\n\
                 }\n\
                 fn h(u: Unit) { let Unit = u; if let Some(z) = Some(1) { let _ = z; } for (i, j) in [(1, 2)] { let _ = i + j; } }\n\
                 mod inner { fn k(q: u8) -> u8 { match q { Glob => q, w => w } } }\n",
            ),
            ("closures.rs", "fn f() { let add = |p: u8, q: u8| p + q; let r = add(1, 2); let s = r; let _ = s; }\n"),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, source).expect("write source");
            let tree_file = TreeFile::new(&path).expect("parse");
            let walked = evidence_from(&tree_file, false, |_, _| {});
            let shared = evidence_from(&tree_file, true, |_, _| {});
            assert!(walked.occurrences.iter().any(|occurrence| occurrence.target.id.is_some()), "{name}");
            assert_eq!(
                serde_json::to_string(&shared).expect("json"),
                serde_json::to_string(&walked).expect("json"),
                "{name}"
            );
        }
    }

    #[test]
    fn capture_order_never_changes_the_evidence() {
        let dir = TempDir::new().expect("tempdir");
        for (name, source) in [
            (
                "sample.rs",
                "use a::b as c;\nfn f() -> u32 { let f = f(); g(); c(); f }\nfn g() { let h = || g(); h(); }\n\
                 struct W; impl W { fn m(&self) { self.m(); Self::m(self); } }\n",
            ),
            (
                "sample.ts",
                "import { x as y } from \"m\";\nfunction f() { { f(); let f = 1; } y(); obj.run(); }\n\
                 class C { a = f(); m() { this.m(); } }\n",
            ),
            ("sample.py", "def f(n):\n    return f(n) + g(n)\n\ndef g(n):\n    return n\n"),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, source).expect("write source");
            let tree_file = TreeFile::new(&path).expect("parse");

            let expected = evidence_with(&tree_file, |_, _| {});
            assert!(!expected.occurrences.is_empty(), "{name}");
            let reversed = evidence_with(&tree_file, |scopes, captures| {
                scopes.reverse();
                captures.reverse();
            });
            let rotated = evidence_with(&tree_file, |scopes, captures| {
                let middle = scopes.len() / 2;
                scopes.rotate_left(middle);
                // A repeated scope capture changes nothing either.
                scopes.extend(scopes.clone());
                let middle = captures.len() / 2;
                captures.rotate_left(middle);
            });
            assert_eq!(reversed, expected, "{name}: reversed captures");
            assert_eq!(rotated, expected, "{name}: rotated captures");
            assert_eq!(tree_file.reference_evidence().expect("evidence"), expected, "{name}");
        }
    }
}
