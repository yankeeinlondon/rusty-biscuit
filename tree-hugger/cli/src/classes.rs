//! The `classes` presentation step: containers with their members split into
//! static and instance sections.

use std::collections::HashMap;

use tree_hugger::{FieldData, Modifier, SymbolKindData, SymbolKindV2, SymbolRecord};

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
        members.sort_by_key(|member| member.source.declaration_span.start_byte);
        members
    }
}

/// Summarizes the containers among `records` (one file's result of the
/// `classes` query, containment included). `--static-only` empties the
/// instance sections and `--instance-only` the static ones; passing both
/// leaves containers with no members.
pub(crate) fn summarize(records: &[SymbolRecord], filter: MemberFilter) -> Vec<ClassSummary> {
    let by_id: HashMap<_, _> = records.iter().map(|record| (&record.id, record)).collect();
    let mut summaries = Vec::new();

    // Interfaces and traits own static members too: a Java interface constant
    // is implicitly `static`, and a Rust trait declares associated functions.
    for container in records.iter().filter(|record| CONTAINER_KINDS.contains(&record.kind)) {
        let mut static_methods = Vec::new();
        let mut instance_methods = Vec::new();
        for member in &container.relations.members {
            let Some(record) = member.id.as_ref().and_then(|id| by_id.get(id)) else {
                continue;
            };
            if !MEMBER_KINDS.contains(&record.kind) {
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
