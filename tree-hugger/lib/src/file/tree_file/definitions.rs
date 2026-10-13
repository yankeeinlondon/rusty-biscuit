//! Definition capture for one file: the locals query runs once, duplicate
//! captures are removed, and only then is metadata extracted, for the
//! winning capture of each selected kind.

use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap, VecDeque};

use tree_sitter::{Node, QueryCursor, StreamingIterator};

use super::TreeFile;
use super::relations::LexicalWalk;
use crate::analysis::occurrences::RustPatterns;
use crate::analysis::counters::{count_definition_capture, count_shared_context_steps};
use crate::analysis::{owner_path, qualifying_prefixes, segment_suffixes};
use crate::error::TreeHuggerError;
use crate::file::extract::*;
use crate::queries::{QueryKind, query_for};
use crate::shared::{SymbolKindV2, SymbolScope};

/// The kinds whose declarations get metadata (visibility, `static` flags,
/// signature, type metadata and doc comment), and, of those, the kinds asked
/// for, whose records also get header texts and the record facets
/// ([`Self::selects`]). Declarations of other kinds get none of it.
#[derive(Debug, Clone)]
pub(crate) struct MetadataSelection {
    /// `None` selects every kind.
    kinds: Option<BTreeSet<SymbolKindV2>>,
    /// The kinds asked for, before widening; `None` selects every kind.
    selected: Option<BTreeSet<SymbolKindV2>>,
}

impl MetadataSelection {
    pub(crate) const ALL: Self = Self { kinds: None, selected: None };

    /// Selects `kinds` (`None`: every kind), widened by the members whose
    /// access, `static` flags and doc comments a selected type's field list
    /// copies ([`align_field_metadata`]), and by the enum variants a selected
    /// enum's variant list is completed from ([`align_variant_metadata`]).
    pub(crate) fn for_kinds(kinds: Option<Vec<SymbolKindV2>>) -> Self {
        let Some(kinds) = kinds else {
            return Self::ALL;
        };
        let selected: BTreeSet<SymbolKindV2> = kinds.into_iter().collect();
        let mut kinds = selected.clone();
        if kinds
            .iter()
            .any(|kind| completes_field_list(SymbolKind::from(*kind)))
        {
            kinds.extend([
                SymbolKindV2::Field,
                SymbolKindV2::Property,
                SymbolKindV2::Constant,
            ]);
        }
        if kinds
            .iter()
            .any(|kind| completes_variant_list(SymbolKind::from(*kind)))
        {
            kinds.insert(SymbolKindV2::EnumVariant);
        }
        Self { kinds: Some(kinds), selected: Some(selected) }
    }

    pub(crate) fn wants(&self, kind: SymbolKind) -> bool {
        self.kinds
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&SymbolKindV2::from(kind)))
    }

    /// Whether `kind` was asked for, not only added by widening. A member
    /// added by widening is read only for what completes its owner's lists
    /// (access, `static` flags, doc comment) and is dropped before the
    /// result is returned, so its record gets no header texts, attributes,
    /// modifiers, flags or metrics.
    pub(crate) fn selects(&self, kind: SymbolKind) -> bool {
        self.selected
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&SymbolKindV2::from(kind)))
    }
}

/// The capability-computed facts [`TreeFile::records_from`] adds to the
/// records of wanted kinds; without them those records keep `false` test and
/// deprecation flags and no `metrics` facet.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RecordFacts {
    /// `Capability::Flags`.
    pub(crate) flags: bool,
    /// `Capability::Metrics`.
    pub(crate) metrics: bool,
    /// `Capability::Containment`: Rust impl members arrive linked to their
    /// impl's self type and Swift extension members to their extension's
    /// owner, which only the syntax tree can resolve.
    pub(crate) containment: bool,
}

/// A symbol with the nodes it was extracted from.
pub(crate) struct ExtractedSymbol<'tree> {
    pub(crate) symbol: SymbolInfo,
    /// The query's context capture (for a C/C++ function captured without
    /// one, [`implied_c_function_context`]), else the name: the record's
    /// declaration span.
    pub(crate) anchor: Node<'tree>,
    /// The declaration the name belongs to: the source of visibility, static
    /// flags, signature and declaration text.
    pub(crate) declaration: Node<'tree>,
    /// The captured name node.
    pub(crate) name: Node<'tree>,
    /// The Rust `impl` whose self type names this symbol's container.
    pub(crate) container_impl: Option<Node<'tree>>,
    /// The Swift extension whose body declares this symbol: its owner, as
    /// the relation resolver decides it, is the symbol's container.
    pub(crate) container_extension: Option<Node<'tree>>,
    /// [`is_exported_definition`] of the finished symbol, decided once here
    /// for every reader of exports and records.
    pub(crate) is_exported: bool,
}

/// One definition capture, before any metadata is read from it.
struct Candidate<'tree> {
    name: Node<'tree>,
    kind: SymbolKind,
    declaration: Node<'tree>,
    /// The match's context capture, or the implied C/C++ function context.
    context: Option<Node<'tree>>,
    priority: u8,
}

impl<'tree> Candidate<'tree> {
    fn anchor(&self) -> Node<'tree> {
        self.context.unwrap_or(self.name)
    }
}

/// The captures of one name at one span: the highest-priority capture (the
/// first of equal priority) and the equal-priority captures after it, in
/// match order, which fill only the metadata the winner lacks.
struct CaptureGroup<'tree> {
    winner: Candidate<'tree>,
    later: Vec<Candidate<'tree>>,
}

/// What a capture of the definition query is, by its capture name.
#[derive(Clone, Copy)]
enum CaptureRole {
    /// `@local.definition.import`.
    Import,
    /// `@local.scope`.
    Scope,
    /// A declaration name of this base kind.
    Definition(SymbolKind),
    Ignored,
}

/// Facts about the file that its declarations share, each derived once per
/// file (lazily, where only some declarations need it) instead of re-read
/// from the tree for every declaration.
struct FileFacts<'tree> {
    root: Node<'tree>,
    /// JavaScript/TypeScript: the file is a module ([`is_ecma_module_file`]).
    is_ecma_module: bool,
    scopes: SiblingScopes<'tree>,
    visibility: VisibilityMemo<'tree>,
}

/// What one run of the definition (locals) query found.
pub(crate) struct Definitions<'tree> {
    /// One symbol per declared name and span, in source order.
    pub(crate) symbols: Vec<ExtractedSymbol<'tree>>,
    /// The `@local.definition.import` captures in match order (repeats
    /// included), which [`TreeFile::imports_from_nodes`] reads exactly as
    /// [`TreeFile::imported_symbols`] reads its own run of the same query.
    pub(crate) import_nodes: Vec<Node<'tree>>,
    /// The `@local.scope` captures in match order (repeats included); empty
    /// unless asked for by [`TreeFile::definitions_with_scopes`].
    pub(crate) scopes: Vec<Node<'tree>>,
    /// A Rust file's relation walk, built to decide impl members' containers;
    /// [`TreeFile::records_from`] hands it to the relation linker.
    pub(crate) rust_walk: Option<LexicalWalk<'tree>>,
    /// A Rust file's pattern placement ([`RustPatterns`]); kept only with
    /// `scopes`, for reference resolution.
    pub(crate) rust_patterns: Option<RustPatterns<'tree>>,
}

struct PendingSymbol<'tree> {
    extracted: ExtractedSymbol<'tree>,
    container: Option<ContainerRef<'tree>>,
}

impl TreeFile {
    /// Runs the definition (locals) query once and returns one symbol per
    /// declared name and span, in source order, with metadata for the kinds
    /// `selection` wants.
    pub(crate) fn definitions(
        &self,
        selection: &MetadataSelection,
    ) -> Result<Definitions<'_>, TreeHuggerError> {
        self.capture_definitions(selection, false)
    }

    /// [`Self::definitions`], also keeping the query's `@local.scope` nodes,
    /// which only reference resolution reads.
    pub(crate) fn definitions_with_scopes(
        &self,
        selection: &MetadataSelection,
    ) -> Result<Definitions<'_>, TreeHuggerError> {
        self.capture_definitions(selection, true)
    }

    /// Drops the captures that declare nothing, decided exactly as
    /// reference occurrences decide them (`analysis::occurrences`): the path
    /// of a Rust tuple-struct or struct pattern (`Some` in `Some(x)`), a
    /// TypeScript/JavaScript parameter default (`a` in `b = a`), and a Rust
    /// pattern name that reads a constant, unit struct or `const` parameter,
    /// or whose identity is unknown in a refutable pattern (an import or
    /// glob may supply a constant: no declaration is proved, so none is
    /// reported). The Rust walk runs only when a variable or parameter was
    /// captured; its placement is returned for the bind pass to finish.
    fn drop_non_declarations<'tree>(
        &'tree self,
        groups: &mut Vec<((usize, usize, String), CaptureGroup<'tree>)>,
        local_scopes: &[Node<'tree>],
    ) -> Option<RustPatterns<'tree>> {
        use crate::analysis::occurrences::{is_false_capture, rust_pattern_non_bindings};
        let rules = crate::analysis::scope::LanguageRules::for_language(self.language)?;
        groups.retain(|(_, group)| !is_false_capture(rules, group.winner.name));
        let pattern_capture = |group: &CaptureGroup<'_>| matches!(group.winner.kind, SymbolKind::Variable | SymbolKind::Parameter);
        if self.language != ProgrammingLanguage::Rust || !groups.iter().any(|(_, group)| pattern_capture(group)) {
            return None;
        }
        let (non_bindings, patterns) = rust_pattern_non_bindings(self.tree.root_node(), &self.source, local_scopes);
        if !non_bindings.is_empty() {
            groups.retain(|((start, end, _), _)| !non_bindings.contains(&(*start, *end)));
        }
        Some(patterns)
    }

    fn capture_definitions<'tree>(
        &'tree self,
        selection: &MetadataSelection,
        keep_scopes: bool,
    ) -> Result<Definitions<'tree>, TreeHuggerError> {
        count_definition_capture();
        let query = query_for(self.grammar_ref(), QueryKind::Locals)?;
        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        // Each capture's role, decided once per capture name.
        let is_context: Vec<bool> = capture_names.iter().map(|name| name.ends_with(".context")).collect();
        let roles: Vec<CaptureRole> = capture_names
            .iter()
            .map(|&name| match name {
                "local.definition.import" => CaptureRole::Import,
                "local.scope" => CaptureRole::Scope,
                _ => symbol_kind_from_capture(name).map_or(CaptureRole::Ignored, CaptureRole::Definition),
            })
            .collect();
        // Keyed by the name's span, which decides its text.
        let mut groups: HashMap<(usize, usize), (String, CaptureGroup<'_>)> = HashMap::new();
        let mut import_nodes = Vec::new();
        let mut scopes = Vec::new();
        // Rust pattern names are decided by lookup through these scopes.
        let mut local_scopes: Vec<Node<'tree>> = Vec::new();
        let is_ecma_module = matches!(
            self.language,
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript
        ) && is_ecma_module_file(&self.file, root, &self.source);

        let mut matches = cursor.matches(query.as_ref(), root, self.source.as_bytes());
        matches.advance();

        while let Some(query_match) = matches.get() {
            let context_capture = query_match
                .captures
                .iter()
                .find(|capture| is_context.get(capture.index as usize).copied().unwrap_or(false))
                .map(|capture| capture.node);

            for capture in query_match.captures {
                let base_kind = match roles.get(capture.index as usize).copied().unwrap_or(CaptureRole::Ignored) {
                    CaptureRole::Import => {
                        import_nodes.push(capture.node);
                        continue;
                    }
                    CaptureRole::Scope => {
                        if keep_scopes {
                            scopes.push(capture.node);
                        }
                        if self.language == ProgrammingLanguage::Rust {
                            local_scopes.push(capture.node);
                        }
                        continue;
                    }
                    CaptureRole::Definition(base_kind) => base_kind,
                    CaptureRole::Ignored => continue,
                };
                let context_capture = context_capture
                    .map(|context| context_for_name(context, capture.node, self.language));
                let declaration = context_capture.map_or_else(
                    || resolve_declaration_node(capture.node),
                    declaration_for_context,
                );
                let kind = refine_symbol_kind(
                    base_kind,
                    capture.node,
                    declaration,
                    self.language,
                    &self.source,
                );
                let context = context_capture.or_else(|| {
                    implied_c_function_context(self.language, kind, capture.node, declaration)
                });
                let candidate = Candidate {
                    name: capture.node,
                    kind,
                    declaration,
                    context,
                    priority: symbol_kind_priority(kind),
                };

                let key = (capture.node.start_byte(), capture.node.end_byte());
                match groups.get_mut(&key).map(|(_, group)| group) {
                    Some(group) if candidate.priority > group.winner.priority => {
                        *group = CaptureGroup {
                            winner: candidate,
                            later: Vec::new(),
                        };
                    }
                    Some(group) if candidate.priority == group.winner.priority => {
                        group.later.push(candidate);
                    }
                    Some(_) => {}
                    None => {
                        let name = capture
                            .node
                            .utf8_text(self.source.as_bytes())
                            .map(str::to_string)
                            .unwrap_or_default();
                        groups.insert(
                            key,
                            (
                                name,
                                CaptureGroup {
                                    winner: candidate,
                                    later: Vec::new(),
                                },
                            ),
                        );
                    }
                }
            }

            matches.advance();
        }

        let mut groups: Vec<((usize, usize, String), CaptureGroup<'_>)> =
            groups.into_iter().map(|((start, end), (name, group))| ((start, end, name), group)).collect();
        groups.sort_by_key(|((start, end, _), _)| (*start, *end));
        // Kept only for the bind pass, which reads the same scopes.
        let rust_patterns = self.drop_non_declarations(&mut groups, &local_scopes).filter(|_| keep_scopes);

        let facts = FileFacts {
            root,
            is_ecma_module,
            scopes: SiblingScopes::default(),
            visibility: VisibilityMemo::default(),
        };
        let mut pending: Vec<PendingSymbol<'_>> = groups
            .into_iter()
            .map(|((_, _, name), group)| self.materialize(name, group, selection, &facts))
            .collect();

        align_field_metadata(&mut pending, selection);
        align_variant_metadata(&mut pending, selection, &self.source);

        let inputs: Vec<NamingInput<'_>> = pending
            .iter()
            .map(|entry| NamingInput {
                name: &entry.extracted.symbol.name,
                kind: entry.extracted.symbol.kind,
                name_node: entry.extracted.name,
                declaration: entry.extracted.declaration,
                anchor: entry.extracted.anchor,
            })
            .collect();
        let qualified = qualified_names(&inputs, self.language, &self.source);
        for (entry, name) in pending.iter_mut().zip(qualified) {
            entry.extracted.symbol.qualified_name = name.text;
            entry.extracted.symbol.is_namespace_rooted = name.is_namespace_rooted;
        }

        let mut kinds_by_node: HashMap<usize, SymbolKind> = HashMap::new();
        for entry in &pending {
            for node in [entry.extracted.declaration, entry.extracted.anchor] {
                kinds_by_node
                    .entry(node.id())
                    .or_insert(entry.extracted.symbol.kind);
            }
        }
        let owner_kinds = OwnerKinds::default();
        let (impl_owners, rust_walk) = rust_impl_owners(&pending, root, &self.source);
        let container_kinds: Vec<Option<SymbolKind>> = pending
            .iter()
            .map(|entry| {
                resolve_container_kind(
                    entry.container.as_ref(),
                    &entry.extracted.symbol.qualified_name,
                    &kinds_by_node,
                    &pending,
                    &owner_kinds,
                    &impl_owners,
                )
            })
            .collect();

        let symbols = pending
            .into_iter()
            .zip(container_kinds)
            .map(|(entry, container_kind)| {
                let mut extracted = entry.extracted;
                extracted.symbol.container_kind = container_kind;
                extracted.is_exported = is_exported_definition(&extracted.symbol, extracted.anchor, root);
                match entry.container.map(|container| container.lookup) {
                    Some(ContainerLookup::SameFileType { impl_item }) => extracted.container_impl = impl_item,
                    Some(ContainerLookup::Node { node, .. })
                        if self.language == ProgrammingLanguage::Swift
                            && declaration_keyword(node, self.language).as_deref() == Some("extension") =>
                    {
                        extracted.container_extension = Some(node);
                    }
                    _ => {}
                }
                extracted
            })
            .collect();
        Ok(Definitions { symbols, import_nodes, scopes, rust_walk, rust_patterns })
    }

    /// Builds the symbol for a group's winner; metadata only when
    /// `selection` wants its kind.
    fn materialize<'tree>(
        &'tree self,
        name: String,
        group: CaptureGroup<'tree>,
        selection: &MetadataSelection,
        facts: &FileFacts<'tree>,
    ) -> PendingSymbol<'tree> {
        let winner = &group.winner;
        let container = match lua_table_of_name(winner.name) {
            Some((table, _)) if self.language == ProgrammingLanguage::Lua => {
                table.utf8_text(self.source.as_bytes()).ok().map(|table| ContainerRef {
                    name: table.to_string(),
                    lookup: ContainerLookup::SameFileTable,
                })
            }
            _ => find_symbol_container(
                winner.declaration,
                self.language,
                &self.source,
                &facts.scopes,
            ),
        };
        let mut symbol = SymbolInfo {
            name,
            kind: winner.kind,
            scope: symbol_scope(winner.declaration, winner.name, winner.kind, self.language),
            // Built once every symbol is known (`qualified_names`).
            qualified_name: String::new(),
            is_namespace_rooted: false,
            range: range_for_node(winner.name),
            language: self.language,
            file: self.file.clone(),
            container_name: container.as_ref().map(|container| container.name.clone()),
            // Resolved by the caller, once every symbol's kind is known.
            container_kind: None,
            doc_comment: None,
            visibility: None,
            is_static: false,
            is_static_storage: false,
            is_async: is_async_declaration(winner.declaration, winner.name, self.language),
            signature: None,
            type_metadata: None,
        };
        if selection.wants(winner.kind) {
            self.fill_metadata(&mut symbol, &group, facts);
        }

        PendingSymbol {
            extracted: ExtractedSymbol {
                symbol,
                anchor: winner.anchor(),
                declaration: winner.declaration,
                name: winner.name,
                // Read from `container` once every symbol is known.
                container_impl: None,
                container_extension: None,
                // Decided once the symbol is finished.
                is_exported: false,
            },
            container,
        }
    }

    /// Reads metadata from the winner; a later equal-priority capture is
    /// read only for what is still missing. While no signature is known, a
    /// later capture's signature, visibility and `static` flags replace the
    /// current ones together.
    fn fill_metadata<'tree>(
        &'tree self,
        symbol: &mut SymbolInfo,
        group: &CaptureGroup<'tree>,
        facts: &FileFacts<'tree>,
    ) {
        let flags = |candidate: &Candidate<'tree>, scope: SymbolScope| {
            let context = DeclarationContext {
                language: self.language,
                source: &self.source,
                name: candidate.name,
                root: facts.root,
                is_ecma_module: facts.is_ecma_module,
                kind: candidate.kind,
                scope,
                memo: &facts.visibility,
            };
            (
                extract_visibility(candidate.declaration, &context),
                extract_is_static(candidate.declaration, &context),
                extract_is_static_storage(candidate.declaration, &context),
            )
        };
        let signature = |candidate: &Candidate<'_>, is_static: bool| {
            candidate.kind.is_function().then(|| {
                extract_signature(
                    candidate.declaration,
                    candidate.name,
                    self.language,
                    &self.source,
                    is_static,
                )
            })
        };
        let type_metadata = |candidate: &Candidate<'_>| {
            let context = candidate.context?;
            (candidate.kind.is_type() || candidate.kind == SymbolKind::Class)
                .then(|| extract_type_metadata(context, self.language, &self.source))
                .flatten()
        };
        let scope = symbol.scope;
        let doc_comment = |candidate: &Candidate<'_>| match candidate.context {
            // A C/C++ function's context is its declarator; the doc comment
            // precedes the whole definition.
            Some(context) => extract_doc_comment(
                declaration_for_context(context),
                self.language,
                &self.source,
            ),
            // A name captured without a context (a JS/TS `const f = () => …`,
            // a field, a typedef name behind a declarator) is documented
            // before its declaration; a local is never documented that way.
            None if scope != SymbolScope::Local => {
                extract_doc_comment(candidate.declaration, self.language, &self.source)
            }
            None => None,
        };

        // `symbol.scope` is the winner's own scope; a later candidate's is
        // read from its own declaration.
        let (mut visibility, mut is_static, mut is_static_storage) = flags(&group.winner, scope);
        let mut function_signature = signature(&group.winner, is_static);
        let mut metadata = type_metadata(&group.winner);
        let mut doc = doc_comment(&group.winner);
        for later in &group.later {
            if function_signature.is_none() {
                let later_scope = symbol_scope(later.declaration, later.name, later.kind, self.language);
                (visibility, is_static, is_static_storage) = flags(later, later_scope);
                function_signature = signature(later, is_static);
            }
            if metadata.is_none() {
                metadata = type_metadata(later);
            }
            if doc.is_none() {
                doc = doc_comment(later);
            }
        }

        symbol.visibility = visibility;
        symbol.is_static = is_static;
        symbol.is_static_storage = is_static_storage;
        symbol.signature = function_signature;
        symbol.type_metadata = metadata;
        symbol.doc_comment = doc;
    }
}

/// Whether a kind's type metadata lists fields that [`align_field_metadata`]
/// completes from standalone members (not enums: they list variants).
fn completes_field_list(kind: SymbolKind) -> bool {
    (kind.is_type() || kind == SymbolKind::Class) && kind != SymbolKind::Enum
}

/// A type's metadata field list is completed from, and agrees with, the
/// standalone `Field`, `Property` and `Constant` symbols the type directly
/// contains, so `hug classes` (metadata) and `hug symbols` (standalone
/// symbols) list the same members with the same flags:
///
/// - an entry with a standalone symbol of the same name copies its access and
///   member `static`, and its doc comment when the reader read none (the k-th
///   entry of a name pairs with the k-th symbol of that name);
/// - a standalone member no metadata reader listed is added, with its doc
///   comment and no type annotation;
/// - entries with no standalone symbol (record components, Rust tuple
///   positions, Scala class parameters) keep what the reader read.
///
/// The result is in source order: an entry without a standalone symbol stays
/// after the entry the reader listed before it. Enum owners are not
/// completed: they list variants, and a Rust struct-variant field names the
/// enum as its container. Owners `selection` does not want have no metadata
/// and are skipped; [`MetadataSelection::for_kinds`] keeps the members of the
/// owners it wants.
fn align_field_metadata(pending: &mut [PendingSymbol<'_>], selection: &MetadataSelection) {
    struct Member {
        owner: usize,
        start: usize,
        name: String,
        visibility: Option<Visibility>,
        is_static: bool,
        doc_comment: Option<String>,
    }

    let members: Vec<Member> = pending
        .iter()
        .filter_map(|entry| {
            let ContainerLookup::Node { node, .. } = entry.container.as_ref()?.lookup else {
                return None;
            };
            let symbol = &entry.extracted.symbol;
            matches!(
                symbol.kind,
                SymbolKind::Field | SymbolKind::Property | SymbolKind::Constant
            )
            .then(|| Member {
                owner: node.id(),
                start: symbol.range.start_byte,
                name: symbol.name.clone(),
                visibility: symbol.visibility,
                is_static: symbol.is_static,
                doc_comment: symbol.doc_comment.clone(),
            })
        })
        .collect();

    // Each owner's members, in source order, so an owner never scans the
    // members of every other type.
    let mut by_owner: HashMap<usize, Vec<usize>> = HashMap::new();
    for (index, member) in members.iter().enumerate() {
        by_owner.entry(member.owner).or_default().push(index);
    }

    for entry in pending.iter_mut() {
        let symbol = &mut entry.extracted.symbol;
        if !completes_field_list(symbol.kind) || !selection.wants(symbol.kind) {
            continue;
        }
        let owners = [entry.extracted.declaration.id(), entry.extracted.anchor.id()];
        let mut indices: Vec<usize> = owners
            .into_iter()
            .collect::<BTreeSet<usize>>()
            .into_iter()
            .filter_map(|owner| by_owner.get(&owner))
            .flatten()
            .copied()
            .collect();
        count_shared_context_steps(indices.len());
        if indices.is_empty() {
            continue;
        }
        indices.sort_unstable();
        let own: Vec<&Member> = indices.into_iter().map(|index| &members[index]).collect();
        let metadata = symbol.type_metadata.get_or_insert_with(TypeMetadata::new);

        let mut paired = vec![false; own.len()];
        let mut unpaired = unpaired_by_name(own.iter().map(|member| member.name.as_str()));
        let mut positioned: Vec<(usize, FieldInfo)> = Vec::new();
        let mut last_start = 0;
        for mut field in metadata.fields.drain(..) {
            if let Some(index) = unpaired.get_mut(field.name.as_str()).and_then(VecDeque::pop_front) {
                paired[index] = true;
                field.visibility = own[index].visibility;
                field.is_static = own[index].is_static;
                if field.doc_comment.is_none() {
                    field.doc_comment = own[index].doc_comment.clone();
                }
                last_start = own[index].start;
            }
            positioned.push((last_start, field));
        }
        for (member, _) in own.iter().zip(&paired).filter(|(_, paired)| !**paired) {
            positioned.push((
                member.start,
                FieldInfo {
                    name: member.name.clone(),
                    type_annotation: None,
                    doc_comment: member.doc_comment.clone(),
                    visibility: member.visibility,
                    is_static: member.is_static,
                },
            ));
        }
        positioned.sort_by_key(|(start, _)| *start);
        metadata.fields = positioned.into_iter().map(|(_, field)| field).collect();
    }
}

/// The indexes of `names` by name, in order: popping a name's front gives
/// its first member not yet paired, so the k-th listed entry of a name pairs
/// with the k-th member of that name.
fn unpaired_by_name<'n>(names: impl Iterator<Item = &'n str>) -> HashMap<&'n str, VecDeque<usize>> {
    let mut by_name: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for (index, name) in names.enumerate() {
        by_name.entry(name).or_default().push_back(index);
    }
    by_name
}

/// Whether a kind's type metadata lists variants that
/// [`align_variant_metadata`] completes. Only an enum's record carries a
/// variant list; a Swift enum is extracted as `Type`, whose record has none,
/// so completing its metadata would make the two paths disagree.
fn completes_variant_list(kind: SymbolKind) -> bool {
    kind == SymbolKind::Enum
}

/// A type's variant list is completed from, and agrees with, the standalone
/// `EnumVariant` symbols the type directly contains, so the enum's variant
/// list and `hug symbols` name the same variants ([`reconcile_variants`]).
/// Variants never enter the field list, and an owner with no standalone
/// variant keeps what its reader listed. Owners `selection` does not want
/// are skipped; [`MetadataSelection::for_kinds`] keeps the variants of the
/// owners it wants.
fn align_variant_metadata(pending: &mut [PendingSymbol<'_>], selection: &MetadataSelection, source: &str) {
    let mut by_owner: HashMap<usize, Vec<(usize, VariantInfo)>> = HashMap::new();
    for entry in pending.iter() {
        let symbol = &entry.extracted.symbol;
        if symbol.kind != SymbolKind::EnumVariant {
            continue;
        }
        let Some(ContainerLookup::Node { node, .. }) =
            entry.container.as_ref().map(|container| container.lookup)
        else {
            continue;
        };
        let mut variant = VariantInfo::unit(symbol.name.clone());
        variant.discriminant = variant_discriminant(entry.extracted.declaration, source);
        variant.doc_comment = symbol.doc_comment.clone();
        by_owner
            .entry(node.id())
            .or_default()
            .push((symbol.range.start_byte, variant));
    }
    if by_owner.is_empty() {
        return;
    }

    for entry in pending.iter_mut() {
        let symbol = &mut entry.extracted.symbol;
        if !completes_variant_list(symbol.kind) || !selection.wants(symbol.kind) {
            continue;
        }
        let standalone: Vec<(usize, VariantInfo)> = [entry.extracted.declaration.id(), entry.extracted.anchor.id()]
            .into_iter()
            .collect::<BTreeSet<usize>>()
            .into_iter()
            .filter_map(|owner| by_owner.remove(&owner))
            .flatten()
            .collect();
        if standalone.is_empty() {
            continue;
        }
        let metadata = symbol.type_metadata.get_or_insert_with(TypeMetadata::new);
        let listed = std::mem::take(&mut metadata.variants);
        metadata.variants = reconcile_variants(listed, standalone);
    }
}

/// Merges a reader's variant list with the standalone variants of the same
/// type (`(start byte, variant)`):
///
/// - the k-th listed variant of a name pairs with the k-th standalone variant
///   of that name, keeping the reader's payload and taking a discriminant or
///   doc comment only where the reader has none;
/// - a standalone variant the reader did not list is added at its source
///   position;
/// - a listed variant with no standalone symbol stays after the variant the
///   reader listed before it.
fn reconcile_variants(listed: Vec<VariantInfo>, mut standalone: Vec<(usize, VariantInfo)>) -> Vec<VariantInfo> {
    standalone.sort_by_key(|(start, _)| *start);
    let mut paired = vec![false; standalone.len()];
    let mut unpaired = unpaired_by_name(standalone.iter().map(|(_, variant)| variant.name.as_str()));
    let mut positioned: Vec<(usize, VariantInfo)> = Vec::new();
    let mut last_start = 0;
    for mut variant in listed {
        if let Some(index) = unpaired.get_mut(variant.name.as_str()).and_then(VecDeque::pop_front)
        {
            paired[index] = true;
            let (start, own) = &standalone[index];
            variant.discriminant = variant.discriminant.or_else(|| own.discriminant.clone());
            variant.doc_comment = variant.doc_comment.or_else(|| own.doc_comment.clone());
            last_start = *start;
        }
        positioned.push((last_start, variant));
    }
    for ((start, variant), _) in standalone.into_iter().zip(paired).filter(|(_, paired)| !paired) {
        positioned.push((start, variant));
    }
    positioned.sort_by_key(|(start, _)| *start);
    positioned.into_iter().map(|(_, variant)| variant).collect()
}

/// For each Rust `impl` of the file (by node id), the index in `pending` of
/// the declaration its self type resolves to, by the relation resolver's
/// decision (`relations::impl_self_types`), and the walk that decision
/// built; empty, with no walk, when no symbol sits in an impl.
fn rust_impl_owners<'tree>(
    pending: &[PendingSymbol<'tree>],
    root: Node<'tree>,
    source: &'tree str,
) -> (HashMap<usize, Option<usize>>, Option<LexicalWalk<'tree>>) {
    let in_impl = |entry: &PendingSymbol<'_>| {
        entry.container.as_ref().is_some_and(|container| {
            matches!(container.lookup, ContainerLookup::SameFileType { impl_item: Some(_) })
        })
    };
    if !pending.iter().any(in_impl) {
        return (HashMap::new(), None);
    }
    let entries = pending
        .iter()
        .map(|entry| super::relations::Declared {
            qualified_name: entry.extracted.symbol.qualified_name.as_str(),
            kind: SymbolKindV2::from(entry.extracted.symbol.kind),
            declaration: entry.extracted.declaration,
            anchor: entry.extracted.anchor,
            name: entry.extracted.name,
        })
        .collect();
    let (owners, walk) = super::relations::impl_self_types(entries, root, source);
    (owners, Some(walk))
}

/// The container's kind is the kind extracted for the container's own
/// declaration, so a container and its standalone symbol never disagree.
///
/// A Rust impl member's container is the declaration the impl's self type
/// resolves to (`impl_owners`, the relation resolver's decision); it has no
/// kind when the self type does not resolve (`impl ::ext::B`, an enclosing
/// generic `T`), however a same-named local declaration is called. Another
/// container found by name (`SameFileType`, `SameFileTable`) takes its kind
/// only from same-named declarations whose qualified name agrees with the
/// member's explicit qualification (`agrees_with_owner_path`). Absent when
/// no such declaration exists or when they disagree on kind.
fn resolve_container_kind<'a>(
    container: Option<&ContainerRef<'_>>,
    qualified_name: &str,
    kinds_by_node: &HashMap<usize, SymbolKind>,
    symbols: &'a [PendingSymbol<'_>],
    owner_kinds: &OwnerKinds<'a>,
    impl_owners: &HashMap<usize, Option<usize>>,
) -> Option<SymbolKind> {
    let container = container?;
    if let ContainerLookup::SameFileType { impl_item: Some(impl_item) } = container.lookup {
        let owner = (*impl_owners.get(&impl_item.id())?)?;
        return Some(symbols[owner].extracted.symbol.kind);
    }
    match container.lookup {
        ContainerLookup::Node { node, fallback } => {
            Some(kinds_by_node.get(&node.id()).copied().unwrap_or(fallback))
        }
        // The table's kind when the file's item-scoped declarations of
        // that name agree on one.
        ContainerLookup::SameFileTable => owner_kinds
            .tables
            .get_or_init(|| KindAgreement::of(symbols, |symbol| symbol.scope == SymbolScope::Item))
            .agreed(&container.name, qualified_name),
        ContainerLookup::SameFileType { .. } => owner_kinds
            .types
            .get_or_init(|| KindAgreement::of(symbols, |symbol| symbol.kind.is_type()))
            .agreed(&container.name, qualified_name),
    }
}

/// The per-file indexes behind [`resolve_container_kind`], each built on
/// the first member that needs it.
#[derive(Default)]
struct OwnerKinds<'a> {
    /// Over the item-scoped declarations (`SameFileTable`).
    tables: OnceCell<KindAgreement<'a>>,
    /// Over the type declarations (`SameFileType`).
    types: OnceCell<KindAgreement<'a>>,
}

/// What a set of same-named declarations says about their kind.
#[derive(Clone, Copy, PartialEq)]
enum Agreement {
    Kind(SymbolKind),
    Disagree,
}

impl Agreement {
    fn with(current: Option<Self>, next: Option<Self>) -> Option<Self> {
        match (current, next) {
            (None, next) => next,
            (current, None) => current,
            (Some(current), Some(next)) if current == next => Some(current),
            _ => Some(Self::Disagree),
        }
    }
}

/// The kind agreement of a file's eligible declarations, by name, so that a
/// member reads the decision its candidates share instead of scanning every
/// same-named declaration: a candidate agrees with a member's qualified name
/// when it qualifies it (one of [`qualifying_prefixes`]) or ends with its
/// owner path (one of the candidate's [`segment_suffixes`]), and with every
/// member when the member records no owner path.
struct KindAgreement<'a> {
    by_name: HashMap<&'a str, Agreement>,
    by_qualified: HashMap<(&'a str, &'a str), Agreement>,
    by_suffix: HashMap<(&'a str, &'a str), Agreement>,
}

impl<'a> KindAgreement<'a> {
    fn of(symbols: &'a [PendingSymbol<'_>], eligible: impl Fn(&SymbolInfo) -> bool) -> Self {
        fn add<K: std::hash::Hash + Eq>(map: &mut HashMap<K, Agreement>, key: K, kind: SymbolKind) {
            map.entry(key)
                .and_modify(|agreement| {
                    if *agreement != Agreement::Kind(kind) {
                        *agreement = Agreement::Disagree;
                    }
                })
                .or_insert(Agreement::Kind(kind));
        }
        let mut agreement =
            Self { by_name: HashMap::new(), by_qualified: HashMap::new(), by_suffix: HashMap::new() };
        let mut steps = 0;
        for symbol in symbols.iter().map(|entry| &entry.extracted.symbol) {
            steps += 1;
            if !eligible(symbol) {
                continue;
            }
            let name = symbol.name.as_str();
            let qualified = symbol.qualified_name.as_str();
            add(&mut agreement.by_name, name, symbol.kind);
            add(&mut agreement.by_qualified, (name, qualified), symbol.kind);
            for tail in segment_suffixes(qualified) {
                add(&mut agreement.by_suffix, (name, tail), symbol.kind);
                steps += 1;
            }
        }
        count_shared_context_steps(steps);
        agreement
    }

    /// The kind every declaration called `name` that agrees with `member`
    /// has, if they share one.
    fn agreed(&self, name: &str, member: &str) -> Option<SymbolKind> {
        count_shared_context_steps(1);
        let agreement = match owner_path(member) {
            None => self.by_name.get(name).copied(),
            Some(path) => qualifying_prefixes(member).fold(
                self.by_suffix.get(&(name, path)).copied(),
                |agreement, prefix| {
                    count_shared_context_steps(1);
                    Agreement::with(agreement, self.by_qualified.get(&(name, prefix)).copied())
                },
            ),
        };
        match agreement? {
            Agreement::Kind(kind) => Some(kind),
            Agreement::Disagree => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProgrammingLanguage;

    use crate::language_cases::language_tests;
    language_tests!(imports_from_the_definition_run_equal_imported_symbols => check_definition_imports);

    /// The imports read from the definition run's captures, with the same
    /// forwarding flags, are exactly the imports of `imported_symbols`' own
    /// run of the query, in every shipped fixture and in grouped and repeated
    /// captures.
    fn same_imports(tree_file: &TreeFile) -> usize {
        let definitions = tree_file.definitions(&MetadataSelection::ALL).expect("definitions");
        let from_definitions = tree_file.forwarded_imports_from_nodes(&definitions.import_nodes);
        let own_run = tree_file.imported_symbols().expect("imports");
        assert_eq!(format!("{from_definitions:?}"), format!("{own_run:?}"), "{}", tree_file.file.display());
        from_definitions.len()
    }

    fn check_definition_imports(language: ProgrammingLanguage) {
        let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures");
        let mut pending = vec![fixtures];
        let (mut files, mut imports) = (0, 0);
        while let Some(path) = pending.pop() {
            if path.is_dir() {
                pending.extend(std::fs::read_dir(&path).expect("fixture dir").map(|entry| entry.expect("entry").path()));
            } else if ProgrammingLanguage::from_path(&path) == Some(language)
                && let Ok(tree_file) = TreeFile::new(&path)
            {
                files += 1;
                imports += same_imports(&tree_file);
            }
        }

        let minimum = crate::language_cases::corpus_minimums(language);
        assert!(files >= minimum.files && imports >= minimum.imports, "{language}: {files} fixtures, {imports} imports");
    }

    #[test]
    fn grouped_and_repeated_imports_equal_the_definition_run() {
        let dir = tempfile::TempDir::new().expect("temp");
        for (file, source) in [
            ("grouped.rs", "use a::{self, *};\nuse b::{c as d, e};\n"),
            ("grouped.scala", "import a.{_, given}\nimport b.{C => D}\n"),
            ("aliased.go", "package p\nimport (\n\tf \"fmt\"\n\t\"os/exec\"\n)\n"),
            ("selective.swift", "import struct Foundation.Date\nimport UIKit\n"),
            ("mixed.ts", "import x, { y as z, w } from \"m\";\nimport * as ns from \"n\";\nexport { y } from \"m\";\n"),
            ("grouped.py", "from a import (b, c as d)\nimport e.f as g\n"),
        ] {
            let path = dir.path().join(file);
            std::fs::write(&path, source).expect("source");
            assert!(same_imports(&TreeFile::new(&path).expect("parse")) > 0, "{file}");
        }
    }

    fn variant(name: &str, discriminant: Option<&str>, doc: Option<&str>) -> VariantInfo {
        let mut variant = VariantInfo::unit(name);
        variant.discriminant = discriminant.map(str::to_string);
        variant.doc_comment = doc.map(str::to_string);
        variant
    }

    fn summary(variants: &[VariantInfo]) -> Vec<(&str, Option<&str>, Option<&str>)> {
        variants
            .iter()
            .map(|variant| {
                (
                    variant.name.as_str(),
                    variant.discriminant.as_deref(),
                    variant.doc_comment.as_deref(),
                )
            })
            .collect()
    }

    /// Paired variants keep the reader's data and take only what it lacks;
    /// a standalone variant the reader missed is added at its position; a
    /// listed variant with no standalone symbol is kept; no name appears
    /// twice unless declared twice.
    #[test]
    fn variant_lists_merge_standalone_variants_without_duplicates() {
        let mut listed_payload = variant("Tuple", None, Some("reader doc"));
        listed_payload.tuple_fields = vec!["i32".to_string()];
        let listed = vec![
            variant("A", Some("1"), None),
            listed_payload,
            variant("OnlyListed", None, None),
        ];
        let standalone = vec![
            (40, variant("Missed", Some("7"), Some("missed doc"))),
            (10, variant("A", Some("ignored"), Some("a doc"))),
            (20, variant("Tuple", Some("2"), Some("standalone doc"))),
        ];

        let merged = reconcile_variants(listed, standalone);
        assert_eq!(
            summary(&merged),
            [
                ("A", Some("1"), Some("a doc")),
                ("Tuple", Some("2"), Some("reader doc")),
                ("OnlyListed", None, None),
                ("Missed", Some("7"), Some("missed doc")),
            ]
        );
        assert_eq!(merged[1].tuple_fields, ["i32"]);
    }

    /// A reader that lists nothing (or reads no metadata) yields the
    /// standalone variants in source order; repeated names pair in order.
    #[test]
    fn variant_lists_are_built_from_standalone_variants_alone() {
        let standalone = vec![
            (30, variant("Value", None, None)),
            (10, variant("Red", None, None)),
            (20, variant("Blue", Some("2"), None)),
        ];
        assert_eq!(
            summary(&reconcile_variants(Vec::new(), standalone)),
            [("Red", None, None), ("Blue", Some("2"), None), ("Value", None, None)]
        );

        let repeated = reconcile_variants(
            vec![variant("Same", Some("1"), None), variant("Same", None, None)],
            vec![(5, variant("Same", None, None)), (9, variant("Same", Some("2"), None))],
        );
        assert_eq!(summary(&repeated), [("Same", Some("1"), None), ("Same", Some("2"), None)]);
        assert!(reconcile_variants(Vec::new(), Vec::new()).is_empty());
    }
}
