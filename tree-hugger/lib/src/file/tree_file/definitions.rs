//! Definition capture for one file: the locals query runs once, duplicate
//! captures are removed, and only then is metadata extracted, for the
//! winning capture of each selected kind.

use std::collections::{BTreeSet, HashMap};

use tree_sitter::{QueryCursor, StreamingIterator};

use super::TreeFile;
use crate::analysis::counters::count_definition_capture;
use crate::analysis::agrees_with_owner_path;
use crate::error::TreeHuggerError;
use crate::file::extract::*;
use crate::queries::{QueryKind, query_for};
use crate::shared::{SymbolKindV2, SymbolScope};

/// The kinds whose declarations get metadata (visibility, `static` flags,
/// signature, type metadata, doc comment and the record's header texts).
/// Declarations of other kinds get none of it.
#[derive(Debug, Clone)]
pub(crate) struct MetadataSelection {
    /// `None` selects every kind.
    kinds: Option<BTreeSet<SymbolKindV2>>,
}

impl MetadataSelection {
    pub(crate) const ALL: Self = Self { kinds: None };

    /// Selects `kinds` (`None`: every kind), widened by the members whose
    /// access and `static` flags a selected type's field list copies
    /// ([`align_field_metadata`]) and whose doc comments a selected type's
    /// variant list copies ([`align_variant_metadata`]).
    pub(crate) fn for_kinds(kinds: Option<Vec<SymbolKindV2>>) -> Self {
        let Some(kinds) = kinds else {
            return Self::ALL;
        };
        let mut kinds: BTreeSet<SymbolKindV2> = kinds.into_iter().collect();
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
        Self { kinds: Some(kinds) }
    }

    pub(crate) fn wants(&self, kind: SymbolKind) -> bool {
        self.kinds
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
    ) -> Result<Vec<ExtractedSymbol<'_>>, TreeHuggerError> {
        count_definition_capture();
        let query = query_for(self.grammar_ref(), QueryKind::Locals)?;
        let mut cursor = QueryCursor::new();
        let root = self.tree.root_node();
        let capture_names = query.capture_names();
        let mut groups: HashMap<(usize, usize, String), CaptureGroup<'_>> = HashMap::new();
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
                .find(|capture| {
                    capture_names
                        .get(capture.index as usize)
                        .is_some_and(|name| name.ends_with(".context"))
                })
                .map(|capture| capture.node);

            for capture in query_match.captures {
                let capture_name = capture_names
                    .get(capture.index as usize)
                    .copied()
                    .unwrap_or_default();

                if capture_name == "local.definition.import" {
                    continue;
                }

                let Some(base_kind) = symbol_kind_from_capture(capture_name) else {
                    continue;
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
                let name = capture
                    .node
                    .utf8_text(self.source.as_bytes())
                    .map(str::to_string)
                    .unwrap_or_default();
                let candidate = Candidate {
                    name: capture.node,
                    kind,
                    declaration,
                    context,
                    priority: symbol_kind_priority(kind),
                };

                let key = (capture.node.start_byte(), capture.node.end_byte(), name);
                match groups.get_mut(&key) {
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
                        groups.insert(
                            key,
                            CaptureGroup {
                                winner: candidate,
                                later: Vec::new(),
                            },
                        );
                    }
                }
            }

            matches.advance();
        }

        let mut groups: Vec<((usize, usize, String), CaptureGroup<'_>)> =
            groups.into_iter().collect();
        groups.sort_by_key(|((start, end, _), _)| (*start, *end));

        let mut pending: Vec<PendingSymbol<'_>> = groups
            .into_iter()
            .map(|((_, _, name), group)| {
                self.materialize(name, group, selection, root, is_ecma_module)
            })
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
        let container_kinds: Vec<Option<SymbolKind>> = pending
            .iter()
            .map(|entry| {
                resolve_container_kind(
                    entry.container.as_ref(),
                    &entry.extracted.symbol.qualified_name,
                    &kinds_by_node,
                    &pending,
                )
            })
            .collect();

        Ok(pending
            .into_iter()
            .zip(container_kinds)
            .map(|(entry, container_kind)| {
                let mut extracted = entry.extracted;
                extracted.symbol.container_kind = container_kind;
                extracted
            })
            .collect())
    }

    /// Builds the symbol for a group's winner; metadata only when
    /// `selection` wants its kind.
    fn materialize<'tree>(
        &'tree self,
        name: String,
        group: CaptureGroup<'tree>,
        selection: &MetadataSelection,
        root: Node<'tree>,
        is_ecma_module: bool,
    ) -> PendingSymbol<'tree> {
        let winner = &group.winner;
        let container = match lua_table_of_name(winner.name) {
            Some((table, _)) if self.language == ProgrammingLanguage::Lua => {
                table.utf8_text(self.source.as_bytes()).ok().map(|table| ContainerRef {
                    name: table.to_string(),
                    lookup: ContainerLookup::SameFileTable,
                })
            }
            _ => find_symbol_container(winner.declaration, self.language, &self.source),
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
            self.fill_metadata(&mut symbol, &group, root, is_ecma_module);
        }

        PendingSymbol {
            extracted: ExtractedSymbol {
                symbol,
                anchor: winner.anchor(),
                declaration: winner.declaration,
                name: winner.name,
            },
            container,
        }
    }

    /// Reads metadata from the winner; a later equal-priority capture is
    /// read only for what is still missing. While no signature is known, a
    /// later capture's signature, visibility and `static` flags replace the
    /// current ones together.
    fn fill_metadata(
        &self,
        symbol: &mut SymbolInfo,
        group: &CaptureGroup<'_>,
        root: Node<'_>,
        is_ecma_module: bool,
    ) {
        let flags = |candidate: &Candidate<'_>| {
            let context = DeclarationContext {
                language: self.language,
                source: &self.source,
                name: candidate.name,
                root,
                is_ecma_module,
                kind: candidate.kind,
                scope: symbol_scope(candidate.declaration, candidate.name, candidate.kind, self.language),
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

        let (mut visibility, mut is_static, mut is_static_storage) = flags(&group.winner);
        let mut function_signature = signature(&group.winner, is_static);
        let mut metadata = type_metadata(&group.winner);
        let mut doc = doc_comment(&group.winner);
        for later in &group.later {
            if function_signature.is_none() {
                (visibility, is_static, is_static_storage) = flags(later);
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

    for entry in pending.iter_mut() {
        let symbol = &mut entry.extracted.symbol;
        if !completes_field_list(symbol.kind) || !selection.wants(symbol.kind) {
            continue;
        }
        let owners = [entry.extracted.declaration.id(), entry.extracted.anchor.id()];
        let own: Vec<&Member> = members
            .iter()
            .filter(|member| owners.contains(&member.owner))
            .collect();
        if own.is_empty() {
            continue;
        }
        let metadata = symbol.type_metadata.get_or_insert_with(TypeMetadata::new);

        let mut paired = vec![false; own.len()];
        let mut positioned: Vec<(usize, FieldInfo)> = Vec::new();
        let mut last_start = 0;
        for mut field in metadata.fields.drain(..) {
            if let Some(index) = (0..own.len()).find(|&index| !paired[index] && own[index].name == field.name) {
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
    let mut positioned: Vec<(usize, VariantInfo)> = Vec::new();
    let mut last_start = 0;
    for mut variant in listed {
        if let Some(index) =
            (0..standalone.len()).find(|&index| !paired[index] && standalone[index].1.name == variant.name)
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

/// The container's kind is the kind extracted for the container's own
/// declaration, so a container and its standalone symbol never disagree.
///
/// A container found by name (`SameFileType`, `SameFileTable`) takes its
/// kind only from same-named declarations whose qualified name agrees with
/// the member's explicit qualification (`agrees_with_owner_path`): the
/// members of `impl other::W` take no kind from an unrelated local `W`.
fn resolve_container_kind(
    container: Option<&ContainerRef<'_>>,
    qualified_name: &str,
    kinds_by_node: &HashMap<usize, SymbolKind>,
    symbols: &[PendingSymbol<'_>],
) -> Option<SymbolKind> {
    let container = container?;
    let agrees =
        |candidate: &SymbolInfo| agrees_with_owner_path(&candidate.qualified_name, qualified_name);
    match container.lookup {
        ContainerLookup::Node { node, fallback } => {
            Some(kinds_by_node.get(&node.id()).copied().unwrap_or(fallback))
        }
        // The table's kind when the file's item-scoped declarations of
        // that name agree on one.
        ContainerLookup::SameFileTable => {
            let mut kinds = symbols
                .iter()
                .map(|entry| &entry.extracted.symbol)
                .filter(|symbol| {
                    symbol.scope == SymbolScope::Item
                        && symbol.name == container.name
                        && agrees(symbol)
                })
                .map(|symbol| symbol.kind);
            let first = kinds.next()?;
            kinds.all(|kind| kind == first).then_some(first)
        }
        // Absent when the type is declared elsewhere, or when same-named
        // declarations in this file disagree on kind.
        ContainerLookup::SameFileType => {
            let mut kinds = symbols
                .iter()
                .map(|entry| &entry.extracted.symbol)
                .filter(|symbol| {
                    symbol.kind.is_type() && symbol.name == container.name && agrees(symbol)
                })
                .map(|symbol| symbol.kind);
            let first = kinds.next()?;
            kinds.all(|kind| kind == first).then_some(first)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
