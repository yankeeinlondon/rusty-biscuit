//! Prepared records hosts share: the named-type tables of a reserved
//! shipped namespace ([`crate::embedded::ShippedSources`]).
//!
//! A host whose artifact holds its shipped partials' named types attaches a
//! [`PreparedSourcesDescriptor`] to its `ShippedSources`
//! ([`ShippedSources::with_prepared`](crate::embedded::ShippedSources::with_prepared)).
//! The resolver then reads a partial's table from the accepted artifact
//! instead of parsing and admitting its text, but only when the documents
//! the `ShippedSources` carries are exactly the documents the tables were
//! prepared from. A caller supplying other text under the same paths gets
//! ordinary preparation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Acceptance, PreparationMode, RecordError, SourceFingerprint};
use crate::embedded::EmbeddedDocument;
use crate::errors::SchemaError;
use crate::resolve::NamedTypes;
use crate::simplified::{PropertyDef, SchemaSourcePath};

/// The unexpanded named types of one shipped document, as the resolver
/// parses them from its text: the `types` table, or the payload mapping
/// when there is none. Entries keep authored order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedTypeTableRecord {
    /// The document's virtual path.
    pub identity: String,
    /// Where the entries sit in the document (`[types]` or the root).
    pub root: SchemaSourcePath,
    pub properties: IndexMap<String, PropertyDef>,
}

impl NamedTypeTableRecord {
    /// Parses and admits `document`'s named types in `mode`.
    ///
    /// ## Errors
    ///
    /// The resolver's error for a document that supplies no named types.
    pub fn prepare(document: &EmbeddedDocument<'_>, mode: PreparationMode<'_>) -> Result<Self, SchemaError> {
        let table = crate::resolve::named_types_from_text_in(
            document.text,
            std::path::Path::new(document.path),
            mode,
        )?;
        Ok(Self { identity: document.path.to_string(), root: table.root, properties: table.properties })
    }
}

/// The accepted named-type tables of a host's shipped documents.
///
/// Only constructed inside artifact acceptance
/// ([`Self::from_records`] takes an [`Acceptance`]), so a table served from
/// here was prepared from the fingerprinted text.
pub struct PreparedNamedTypes {
    /// The documents the tables were prepared from, sorted by identity.
    sources: Vec<SourceFingerprint>,
    tables: HashMap<String, Arc<NamedTypes>>,
    /// Whether a supplied document table (by address and length) matched
    /// `sources`, so each table is hashed once.
    matched: Mutex<Vec<(usize, usize, bool)>>,
}

impl std::fmt::Debug for PreparedNamedTypes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedNamedTypes")
            .field("sources", &self.sources)
            .field("tables", &self.tables.len())
            .finish()
    }
}

impl PreparedNamedTypes {
    /// The tables of `records`, prepared from the documents `inventory`
    /// names (each a source the accepted header fingerprints).
    ///
    /// ## Errors
    ///
    /// A [`RecordError`] at `pointer` when an inventory identity is not a
    /// header source or is repeated, or a record names a document outside
    /// the inventory or repeats one.
    pub fn from_records(
        acceptance: &Acceptance<'_>,
        pointer: &str,
        inventory: &[String],
        records: Vec<NamedTypeTableRecord>,
    ) -> Result<Self, RecordError> {
        let header = acceptance.header();
        let mut sources = Vec::with_capacity(inventory.len());
        for (index, identity) in inventory.iter().enumerate() {
            let source = header
                .sources
                .iter()
                .find(|source| &source.identity == identity)
                .ok_or_else(|| RecordError::new(format!("{pointer}/inventory/{index}"), format!("`{identity}` is not a source of this artifact")))?;
            sources.push(source.clone());
        }
        sources.sort_by(|left, right| left.identity.cmp(&right.identity));
        if let Some(pair) = sources.windows(2).find(|pair| pair[0].identity == pair[1].identity) {
            return Err(RecordError::new(format!("{pointer}/inventory"), format!("`{}` is listed twice", pair[0].identity)));
        }
        let mut tables = HashMap::with_capacity(records.len());
        for (index, record) in records.into_iter().enumerate() {
            let at = format!("{pointer}/tables/{index}");
            if !inventory.contains(&record.identity) {
                return Err(RecordError::new(at, format!("`{}` is not in the inventory", record.identity)));
            }
            let identity = record.identity.clone();
            let table = NamedTypes { properties: record.properties, root: record.root };
            if tables.insert(identity.clone(), Arc::new(table)).is_some() {
                return Err(RecordError::new(at, format!("`{identity}` has two tables")));
            }
        }
        Ok(Self { sources, tables, matched: Mutex::new(Vec::new()) })
    }

    /// The table of the document at `path`, when `documents` are exactly the
    /// documents the tables were prepared from.
    pub(crate) fn table(&self, documents: &[EmbeddedDocument<'_>], path: &str) -> Option<Arc<NamedTypes>> {
        if !self.matches(documents) {
            return None;
        }
        self.tables.get(path).map(Arc::clone)
    }

    fn matches(&self, documents: &[EmbeddedDocument<'_>]) -> bool {
        let key = (documents.as_ptr() as usize, documents.len());
        let mut matched = self.matched.lock().expect("prepared named-type match memo poisoned");
        if let Some(&(_, _, result)) = matched.iter().find(|(address, length, _)| (*address, *length) == key) {
            return result;
        }
        let mut supplied: Vec<SourceFingerprint> = documents.iter().map(SourceFingerprint::of).collect();
        supplied.sort_by(|left, right| left.identity.cmp(&right.identity));
        let result = supplied == self.sources;
        matched.push((key.0, key.1, result));
        result
    }
}

/// A host's prepared named-type tables for its reserved namespace, found
/// through `provider` only when the resolver looks a shipped partial up.
#[derive(Clone, Copy, Debug)]
pub struct PreparedSourcesDescriptor {
    /// The owning crate, for diagnostics.
    pub owner: &'static str,
    /// The accepted tables, or `None` when the host's artifact was refused.
    pub provider: fn() -> Option<&'static PreparedNamedTypes>,
}
