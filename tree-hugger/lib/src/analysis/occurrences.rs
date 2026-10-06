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

use super::scope::{BindingRule, BindingSite, BindingTarget, LanguageRules, Lookup, ScopeTree, ancestors};
use super::symbol_ref;
use crate::file::extract::{is_definition_like_reference, text_span_for_node};
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
}

/// Where one import record's name was captured, in import order.
struct ImportSite {
    name_start: usize,
    /// The whole import statement's byte range.
    statement: Option<(usize, usize)>,
    name: String,
}

/// [`ReferenceEvidence`] plus, for each occurrence (same order), the index of
/// the import it is bound to, which the summaries record as a dependency.
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
                    name_start: import.range.start_byte,
                    statement: import.statement_range.as_ref().map(|range| (range.start_byte, range.end_byte)),
                    name: import.name.clone(),
                })
                .collect(),
        }
    }

    /// The import (index into the `imports` given to [`Self::new`]) whose
    /// binding name starts at `name_start`: the import whose name node it
    /// is, else the import of that name whose statement holds it (an
    /// aliased import records the original name's node).
    fn import_at(&self, name_start: usize, name: &str) -> Option<usize> {
        self.imports.iter().position(|site| site.name_start == name_start).or_else(|| {
            self.imports.iter().position(|site| {
                site.name == name && site.statement.is_some_and(|(start, end)| start <= name_start && name_start < end)
            })
        })
    }

    /// Resolves every captured reference against `records` (one per
    /// definition, in the same order).
    pub(crate) fn resolve(self, records: &[SymbolRecord]) -> ResolvedOccurrences {
        debug_assert_eq!(records.len(), self.definitions.len());
        let rules = LanguageRules::for_language(self.language);
        let walk = Walk::run(self.root, self.language, rules, self.source);
        let mut regions = Regions::new(self.root, whole_file_span(self.source));

        for (idx, definition) in self.definitions.iter().enumerate() {
            if definition.kind.is_function()
                && let Some(node) = [definition.anchor, definition.declaration]
                    .into_iter()
                    .find(|node| node.id() != self.root.id() && node.child_by_field_name("body").is_some())
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

        let resolver = Resolver::new(&self, records, rules, &walk);
        let mut resolved = Vec::new();
        for capture in self.captures.iter().flatten() {
            if let Some(occurrence) = resolver.occurrence(*capture, &mut regions) {
                resolved.push(occurrence);
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
fn occurrence_order(occurrence: &Occurrence) -> (u32, u32, u32, u32, OccurrenceKind, &str, Option<&str>, Confidence) {
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
fn whole_file_span(source: &str) -> TextSpan {
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
}

impl<'tree> Walk<'tree> {
    /// Visits every node once, without recursion.
    fn run(root: Node<'tree>, language: ProgrammingLanguage, rules: Option<LanguageRules>, source: &str) -> Self {
        let mut walk = Self::default();
        let language_anonymous = anonymous_callable_kinds(language);
        let mut cursor = root.walk();
        loop {
            let node = cursor.node();
            if node.is_error() || node.is_missing() {
                walk.errors.push(node);
            }
            if node.is_named() {
                if language_anonymous.contains(&node.kind()) {
                    walk.anonymous.push(node);
                }
                if let Some(rules) = rules {
                    walk.visit(node, rules, source);
                }
            }
            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return walk;
                }
            }
        }
    }

    fn visit(&mut self, node: Node<'tree>, rules: LanguageRules, source: &str) {
        if rules.extra_scope(node) {
            self.extra_scopes.push(node);
        }
        rules.binding_sites(node, source, &mut self.sites);
        if rules.is_glob_import(node) {
            self.globs.push(node);
        }
        match rules {
            LanguageRules::Rust => self.visit_rust(node),
            LanguageRules::Ecma { .. } => self.visit_ecma(node, source),
        }
    }

    fn visit_rust(&mut self, node: Node<'tree>) {
        match node.kind() {
            "macro_invocation" => self.reasons.push((node, LimitReason::UnexpandedMacro)),
            "call_expression" => {
                let mut function = node.child_by_field_name("function");
                if let Some(generic) = function.filter(|function| function.kind() == "generic_function") {
                    function = generic.child_by_field_name("function");
                }
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

    fn visit_ecma(&mut self, node: Node<'tree>, source: &str) {
        match node.kind() {
            "call_expression" => match node.child_by_field_name("function") {
                Some(function) if function.kind() == "identifier" => {
                    if function.utf8_text(source.as_bytes()) == Ok("eval") {
                        self.reasons.push((node, LimitReason::UnsupportedBindingForm));
                        self.reasons.push((node, LimitReason::IndirectDispatch));
                    }
                }
                Some(function) if matches!(function.kind(), "super" | "import") => {}
                _ => self.reasons.push((node, LimitReason::IndirectDispatch)),
            },
            "new_expression" => {
                if node
                    .child_by_field_name("constructor")
                    .is_none_or(|constructor| constructor.kind() != "identifier")
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
    records: &'a [SymbolRecord],
    rules: Option<LanguageRules>,
    scopes: Option<ScopeTree<'tree>>,
    /// Name spans of binding sites; never occurrences.
    binding_names: HashSet<(usize, usize)>,
    /// Record indexes by name, in record order.
    by_name: HashMap<&'a str, Vec<usize>>,
}

impl<'a, 'tree> Resolver<'a, 'tree> {
    fn new(
        syntax: &'a OccurrenceSyntax<'tree>,
        records: &'a [SymbolRecord],
        rules: Option<LanguageRules>,
        walk: &Walk<'tree>,
    ) -> Self {
        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        for (idx, record) in records.iter().enumerate() {
            by_name.entry(record.identity.name.as_str()).or_default().push(idx);
        }
        let binding_names = walk
            .sites
            .iter()
            .map(|site| (site.name.start_byte(), site.name.end_byte()))
            .collect();
        let scopes = rules.map(|rules| {
            let scope_nodes = syntax.scope_nodes.iter().chain(&walk.extra_scopes).copied();
            let mut tree = ScopeTree::new(rules, syntax.root, scope_nodes);
            for &glob in &walk.globs {
                tree.add_glob(glob);
            }
            bind_all(&mut tree, syntax, rules, walk);
            tree
        });
        Self { syntax, records, rules, scopes, binding_names, by_name }
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
            || (self.syntax.excluded.contains(&range)
                && !self.rules.is_some_and(|rules| is_false_capture(rules, node)))
            || self.binding_names.contains(&range)
            // Supported languages know their binding names exactly; the
            // others fall back on the older syntactic filter.
            || (self.rules.is_none() && is_definition_like_reference(node, self.syntax.language))
            || ancestors(node).any(|ancestor| is_excluded_context(ancestor.kind()))
            || (self.rules == Some(LanguageRules::Rust) && name == "Self")
        {
            return None;
        }

        let kind = if is_call_target(node, self.syntax.language) {
            OccurrenceKind::Call
        } else {
            OccurrenceKind::Usage
        };
        let region = regions.around(node);
        let (confidence, id, candidates, import) = match (&self.scopes, self.rules) {
            (Some(scopes), Some(rules)) => {
                let resolution = self.resolve_supported(node, name, capture.is_member, scopes, rules);
                if kind == OccurrenceKind::Call && resolution.indirect {
                    regions.limit(region, LimitReason::IndirectDispatch);
                }
                let import = resolution.import.and_then(|name_start| self.syntax.import_at(name_start, name));
                (resolution.confidence, resolution.id, resolution.candidates, import)
            }
            _ => {
                let (confidence, id, candidates) = self.name_only(self.by_name.get(name).cloned().unwrap_or_default());
                (confidence, id, candidates, None)
            }
        };

        let occurrence = Occurrence {
            owner: regions.owner_ref(region, self.records),
            owner_span: regions.regions[region].span.clone(),
            target: SymbolRef {
                id,
                name: name.to_string(),
                qualified_name: written_path(node, self.syntax.source),
                source_span: None,
            },
            candidates,
            span: text_span_for_node(node),
            kind,
            confidence,
        };
        Some((occurrence, import))
    }

    /// `NameOnly` over `candidates` (record indexes), `Unresolved` when there
    /// are none.
    fn name_only(&self, mut candidates: Vec<usize>) -> (Confidence, Option<SymbolId>, Vec<SymbolRef>) {
        candidates.sort_by(|&left, &right| {
            let key = |idx: usize| {
                let record = &self.records[idx];
                (record.source.name_span.as_ref().map_or(0, |span| span.start_byte), &record.id.0)
            };
            key(left).cmp(&key(right))
        });
        candidates.dedup_by(|left, right| self.records[*left].id == self.records[*right].id);
        if candidates.is_empty() {
            (Confidence::Unresolved, None, Vec::new())
        } else {
            (
                Confidence::NameOnly,
                None,
                candidates.into_iter().map(|idx| symbol_ref(&self.records[idx])).collect(),
            )
        }
    }

    fn resolve_supported(
        &self,
        node: Node<'tree>,
        name: &str,
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
            return Resolution::uncertain(self.name_only(members), true);
        }
        if rules == LanguageRules::Rust
            && let Some(path) = node
                .parent()
                .filter(|parent| parent.kind() == "scoped_identifier")
                .filter(|parent| parent.child_by_field_name("name").is_some_and(|own| own.id() == node.id()))
                .and_then(|parent| parent.child_by_field_name("path"))
        {
            return self.resolve_rust_path(path, name, scopes);
        }
        if written_path(node, self.syntax.source).is_some() {
            // A qualified name whose qualifier no rule establishes.
            return Resolution::uncertain(self.name_only(self.item_records(name)), false);
        }

        match scopes.lookup(node, name, rules.reference_category(node)) {
            Lookup::Found(bindings) if bindings.len() == 1 => {
                let binding = scopes.binding(bindings[0]);
                Resolution {
                    confidence: Confidence::Scoped,
                    id: match binding.target {
                        BindingTarget::Record(idx) => Some(self.records[idx].id.clone()),
                        BindingTarget::Import | BindingTarget::Unreported => None,
                    },
                    candidates: Vec::new(),
                    indirect: !binding.rule.callable,
                    import: (binding.target == BindingTarget::Import).then_some(binding.name_start),
                }
            }
            Lookup::Found(bindings) => {
                let records = bindings
                    .iter()
                    .filter_map(|&idx| match scopes.binding(idx).target {
                        BindingTarget::Record(record) => Some(record),
                        _ => None,
                    })
                    .collect();
                Resolution::uncertain(self.name_only(records), false)
            }
            Lookup::Glob | Lookup::NotFound => Resolution::uncertain(self.name_only(self.item_records(name)), false),
        }
    }

    /// `Self::f`/`Type::f`: `Scoped` only when the path is `Self` in an
    /// inherent or trait `impl` of one in-file type, or a name that lookup
    /// establishes as one in-file type, and exactly one inherent `impl` of
    /// that type (or the enum itself, for a variant) declares `f`.
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
        if members.len() == 1 {
            let idx = members[0];
            return Resolution {
                confidence: Confidence::Scoped,
                id: Some(self.records[idx].id.clone()),
                candidates: Vec::new(),
                indirect: false,
                import: None,
            };
        }
        let candidates = if members.is_empty() {
            self.records_named(name, |idx| self.syntax.definitions[idx].scope != SymbolScope::Local)
        } else {
            members
        };
        Resolution::uncertain(self.name_only(candidates), false)
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

    /// Records called `name` that pass `keep`, in record order.
    fn records_named(&self, name: &str, keep: impl Fn(usize) -> bool) -> Vec<usize> {
        self.by_name
            .get(name)
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
    /// on a receiver, or whatever a variable holds.
    indirect: bool,
    /// The name start of the import binding a `Scoped` target.
    import: Option<usize>,
}

impl Resolution {
    fn uncertain(
        (confidence, id, candidates): (Confidence, Option<SymbolId>, Vec<SymbolRef>),
        indirect: bool,
    ) -> Self {
        Self { confidence, id, candidates, indirect, import: None }
    }
}

/// Places every binding: the walk's binding sites, then each record the
/// walk did not find (a capture-only declaration), by its kind.
fn bind_all<'tree>(
    tree: &mut ScopeTree<'tree>,
    syntax: &OccurrenceSyntax<'tree>,
    rules: LanguageRules,
    walk: &Walk<'tree>,
) {
    let records_by_name: HashMap<(usize, usize), usize> = syntax
        .definitions
        .iter()
        .enumerate()
        .map(|(idx, definition)| ((definition.name.start_byte(), definition.name.end_byte()), idx))
        .collect();
    let text = |node: Node<'_>| syntax.source.get(node.start_byte()..node.end_byte()).unwrap_or_default();
    let mut placed: HashSet<(usize, usize)> = HashSet::new();
    for site in &walk.sites {
        let range = (site.name.start_byte(), site.name.end_byte());
        if !placed.insert(range) {
            continue;
        }
        let target = if site.is_import {
            BindingTarget::Import
        } else {
            records_by_name.get(&range).map_or(BindingTarget::Unreported, |&idx| BindingTarget::Record(idx))
        };
        tree.bind(site.name, text(site.name), site.rule, target);
    }
    for (idx, definition) in syntax.definitions.iter().enumerate() {
        let range = (definition.name.start_byte(), definition.name.end_byte());
        if definition.scope == SymbolScope::Member
            || placed.contains(&range)
            || is_false_capture(rules, definition.name)
        {
            continue;
        }
        placed.insert(range);
        tree.bind(definition.name, text(definition.name), record_rule(rules, definition.kind), BindingTarget::Record(idx));
    }
}

/// A name the locals query captures as a declaration but that binds
/// nothing: the path of a Rust tuple-struct or struct pattern (`Some` in
/// `Some(x)`), and a TypeScript parameter's default value (`a` in
/// `b = a`), which is a reference.
fn is_false_capture(rules: LanguageRules, name: Node<'_>) -> bool {
    let Some(parent) = name.parent() else { return false };
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
        ordered: false,
        callable: kind.is_function() || kind == SymbolKind::Class,
    }
}

/// Code that holds no occurrences: imports (their names are import
/// records), attributes, and Rust lifetimes and loop labels (`'_`, `'outer`).
fn is_excluded_context(kind: &str) -> bool {
    matches!(
        kind,
        "use_declaration"
            | "extern_crate_declaration"
            | "attribute_item"
            | "inner_attribute_item"
            | "import_statement"
            | "lifetime"
            | "label"
    )
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
    "class_constant_access_expression",
    "dot_index_expression",
    "method_index_expression",
    "navigation_expression",
    "qualified_name",
];

/// The fields that hold a qualifier or receiver in [`QUALIFYING_KINDS`].
const QUALIFIER_FIELDS: &[&str] = &["path", "value", "object", "operand", "scope", "module", "expression", "table"];

/// The node `node` qualifies, when `node` is the written qualifier's name
/// part: the parent path or member expression.
fn qualified_parent(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if !QUALIFYING_KINDS.contains(&parent.kind()) {
        return None;
    }
    QUALIFIER_FIELDS
        .iter()
        .filter_map(|field| parent.child_by_field_name(field))
        .any(|qualifier| qualifier.id() != node.id() && qualifier.end_byte() <= node.start_byte())
        .then_some(parent)
}

/// The written path or receiver expression ending at `node` (`Self::new`,
/// `other.run`), when `node` is qualified.
fn written_path(node: Node<'_>, source: &str) -> Option<String> {
    let parent = qualified_parent(node)?;
    source.get(parent.start_byte()..node.end_byte()).map(str::to_string)
}

/// Whether `node` names the target of an invocation: a call's function or
/// a constructor, directly or as the name part of a path, receiver
/// expression or Rust turbofish.
fn is_call_target(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    let mut target = node;
    loop {
        let Some(parent) = target.parent() else { return false };
        let holds = |field: &str| parent.child_by_field_name(field).is_some_and(|child| child.id() == target.id());
        let invoked = match parent.kind() {
            "call_expression" | "call" | "invocation_expression" | "function_call_expression" => {
                holds("function")
                    || (parent.child_by_field_name("function").is_none()
                        && parent.named_child(0).is_some_and(|first| first.id() == target.id()))
            }
            "new_expression" => holds("constructor"),
            "object_creation_expression" => holds("type"),
            "method_invocation" | "member_call_expression" | "scoped_call_expression" | "function_call" => {
                holds("name")
            }
            _ => false,
        };
        if invoked {
            return true;
        }
        if language == ProgrammingLanguage::Rust && parent.kind() == "generic_function" && holds("function") {
            target = parent;
        } else if let Some(qualified) = qualified_parent(target) {
            target = qualified;
        } else {
            return false;
        }
    }
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
    /// captures in the order `arrange` leaves them.
    fn evidence_with(
        tree_file: &TreeFile,
        arrange: impl FnOnce(&mut Vec<tree_sitter::Node<'_>>, &mut Vec<crate::file::tree_file::ReferenceCapture<'_>>),
    ) -> ReferenceEvidence {
        let selection = MetadataSelection::for_kinds(Some(Vec::new()));
        let (definitions, mut scopes) = tree_file.definitions_with_scopes(&selection).expect("definitions");
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
        );
        let records = tree_file
            .records_from(definitions, &selection, RecordFacts::default())
            .expect("records");
        syntax.resolve(&records).evidence
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
