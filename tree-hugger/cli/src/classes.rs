//! The `classes` presentation step: containers with their members split into
//! static and instance sections.

use std::collections::HashMap;

use tree_hugger::{
    Capability, FieldData, Modifier, SymbolKindData, SymbolKindV2, SymbolQuery, SymbolRecord,
};

use crate::order::{SortOrder, sort_file_records};
use crate::selection::Selection;

/// The kinds `classes` summarizes.
pub(crate) const CONTAINER_KINDS: [SymbolKindV2; 4] = [
    SymbolKindV2::Class,
    SymbolKindV2::Type,
    SymbolKindV2::Interface,
    SymbolKindV2::Trait,
];

/// The member kinds `classes` lists under a container.
pub(crate) const MEMBER_KINDS: [SymbolKindV2; 2] =
    [SymbolKindV2::Method, SymbolKindV2::Constructor];

/// Which members a `classes` run shows.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MemberFilter {
    pub(crate) static_only: bool,
    pub(crate) instance_only: bool,
}

/// How a `classes` run applies the shared record predicates: kind, scope,
/// name, visibility, export, test, docs and deprecation predicates select
/// containers, `--in` matches a container's own name, and the
/// signature-shape predicates and `--static-only`/`--instance-only` select
/// the displayed members. A selected container may have no member left.
#[derive(Debug, Clone)]
pub(crate) struct ClassSelection {
    /// The container kinds kept and the container predicates.
    containers: SymbolQuery,
    /// `--in`, as names of the container itself.
    within: SymbolQuery,
    /// The signature-shape predicates.
    members: SymbolQuery,
    /// Whether a shape predicate is given: fields, which have no signature,
    /// are then never displayed.
    shaped: bool,
    member_filter: MemberFilter,
}

impl ClassSelection {
    pub(crate) fn new(selection: &Selection, member_filter: MemberFilter) -> Self {
        let kinds = selection
            .kinds_within(Some(&CONTAINER_KINDS))
            .unwrap_or_else(|| CONTAINER_KINDS.to_vec());
        let mut containers = selection.identity_predicates(SymbolQuery::new().kinds(kinds));
        if let Some(scopes) = selection.scopes_within(None) {
            containers = containers.scopes(scopes);
        }
        let within = selection
            .containers
            .iter()
            .fold(SymbolQuery::new(), |query, pattern| query.name(pattern.clone()));
        Self {
            containers,
            within,
            members: selection.shape_predicates(SymbolQuery::new().kinds(MEMBER_KINDS)),
            shaped: selection.has_shape(),
            member_filter,
        }
    }

    /// The analysis query: the selected container kinds and every member
    /// kind, with containment and what the predicates read.
    pub(crate) fn query(&self, capabilities: impl IntoIterator<Item = Capability>) -> SymbolQuery {
        let kinds: Vec<SymbolKindV2> = self
            .containers
            .selected_kinds()
            .unwrap_or_default()
            .into_iter()
            .chain(MEMBER_KINDS)
            .collect();
        let capabilities: Vec<Capability> = capabilities
            .into_iter()
            .chain([Capability::Containment])
            .chain(self.containers.required_capabilities())
            .chain(self.members.required_capabilities())
            .collect();
        SymbolQuery::new().kinds(kinds).capabilities(capabilities)
    }

    /// Whether `container` is selected.
    pub(crate) fn selects(&self, container: &SymbolRecord) -> bool {
        self.containers.selects(container.kind)
            && self.containers.matches(container)
            && self.within.matches(container)
    }
}

/// A container and the members `classes` displays for it.
#[derive(Debug, Clone)]
pub(crate) struct ClassSummary {
    pub(crate) container: SymbolRecord,
    pub(crate) static_methods: Vec<SymbolRecord>,
    pub(crate) instance_methods: Vec<SymbolRecord>,
    /// From the container's own type metadata, whose entries carry each
    /// field's access and `static` flag even when Field, Property and
    /// Constant records were not selected.
    pub(crate) static_fields: Vec<FieldData>,
    pub(crate) instance_fields: Vec<FieldData>,
}

impl ClassSummary {
    /// The displayed methods and constructors, in source order.
    pub(crate) fn displayed_members(&self) -> Vec<SymbolRecord> {
        let mut members: Vec<SymbolRecord> = self
            .static_methods
            .iter()
            .chain(&self.instance_methods)
            .cloned()
            .collect();
        sort_file_records(&mut members, "", SortOrder::File);
        members
    }
}

/// Summarizes the containers among `records` (one file's result of the
/// `classes` query, containment included) that `selection` selects, with the
/// members it displays. `--static-only` empties the instance sections and
/// `--instance-only` the static ones.
pub(crate) fn summarize(records: &[SymbolRecord], selection: &ClassSelection) -> Vec<ClassSummary> {
    let by_id: HashMap<_, _> = records.iter().map(|record| (&record.id, record)).collect();
    let filter = selection.member_filter;
    let mut summaries = Vec::new();

    // Interfaces and traits own static members too: a Java interface constant
    // is implicitly `static`, and a Rust trait declares associated functions.
    for container in records
        .iter()
        .filter(|record| CONTAINER_KINDS.contains(&record.kind) && selection.selects(record))
    {
        let mut static_methods = Vec::new();
        let mut instance_methods = Vec::new();
        for member in &container.relations.members {
            let Some(record) = member.id.as_ref().and_then(|id| by_id.get(id)) else {
                continue;
            };
            if !MEMBER_KINDS.contains(&record.kind) || !selection.members.matches(record) {
                continue;
            }
            if is_static(record) {
                static_methods.push((*record).clone());
            } else {
                instance_methods.push((*record).clone());
            }
        }

        let (mut static_fields, mut instance_fields): (Vec<FieldData>, Vec<FieldData>) =
            match &container.kind_data {
                SymbolKindData::Type(data) => {
                    data.fields.iter().cloned().partition(|field| field.is_static)
                }
                _ => (Vec::new(), Vec::new()),
            };

        // Members keep source order whatever `--sort` says, with the shared
        // tie-breakers rather than the order containment linked them in.
        sort_file_records(&mut static_methods, "", SortOrder::File);
        sort_file_records(&mut instance_methods, "", SortOrder::File);

        if selection.shaped {
            static_fields.clear();
            instance_fields.clear();
        }
        if filter.static_only {
            instance_methods.clear();
            instance_fields.clear();
        }
        if filter.instance_only {
            static_methods.clear();
            static_fields.clear();
        }

        summaries.push(ClassSummary {
            container: container.clone(),
            static_methods,
            instance_methods,
            static_fields,
            instance_fields,
        });
    }
    summaries
}

/// A member without an instance (`SymbolInfo::is_static` in the v1 view).
fn is_static(record: &SymbolRecord) -> bool {
    record
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, Modifier::Static))
}

#[cfg(test)]
mod tests {
    use tree_hugger::{Capability, SourceFile, SymbolQuery};

    use super::*;

    /// Containment links members in extraction order; the sections and the
    /// JSON members list them by source position instead.
    #[test]
    fn members_are_listed_in_source_order_not_link_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("Shapes.java");
        std::fs::write(
            &path,
            "class Shapes {\n  void beta() {}\n  static void alpha() {}\n  void gamma() {}\n  static void delta() {}\n}\n",
        )
        .expect("write");
        let query = SymbolQuery::new()
            .kinds(CONTAINER_KINDS.into_iter().chain(MEMBER_KINDS))
            .capabilities([Capability::Containment]);
        let mut records = SourceFile::read(&path).expect("read").query(&query).expect("query").index.symbols;
        let container = records
            .iter_mut()
            .find(|record| record.kind == SymbolKindV2::Class)
            .expect("the class");
        assert_eq!(container.relations.members.len(), 4, "four linked members");
        container.relations.members.reverse();

        let selection = ClassSelection::new(&Selection::default(), MemberFilter::default());
        let summaries = summarize(&records, &selection);
        let names = |members: &[SymbolRecord]| -> Vec<String> {
            members.iter().map(|member| member.identity.name.clone()).collect()
        };
        assert_eq!(names(&summaries[0].static_methods), ["alpha", "delta"]);
        assert_eq!(names(&summaries[0].instance_methods), ["beta", "gamma"]);
        assert_eq!(
            names(&summaries[0].displayed_members()),
            ["beta", "alpha", "gamma", "delta"]
        );
    }
}
