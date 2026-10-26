//! The shared frontmatter analysis of one document snapshot.
//!
//! [`OverlayState`](super::OverlayState) keeps one current
//! [`FrontmatterAnalysis`] per open document and hands the same `Arc` to every
//! [`DocumentOverlay`](super::DocumentOverlay) built for that snapshot, so
//! schema work done for one request (the effective root shape, merged views of
//! union arms, synthesized array-item definitions) is reused by the next. A
//! request builds a fresh overlay, so state kept on the overlay itself would
//! not survive to the next request.
//!
//! A snapshot is identified by everything its contents depend on (see
//! [`AnalysisIdentity`]). Any change replaces it; closing the document drops
//! it. A request that still holds an old snapshot keeps it alive until the
//! request finishes, but the old one never becomes current again.
//!
//! Every schema node a walk reaches lives in an owner this snapshot keeps
//! alive and never mutates: the effective root, the bundle's definitions, a
//! shape this snapshot built, or an item definition it holds from the
//! cross-snapshot store (below). A node's address is therefore a stable identity
//! for the life of the snapshot, and the caches key on it. A merged view
//! copies the properties of its arms; a copy contributed by a single arm keeps
//! its source's identity, so a recursive definition reached again below a
//! merged view returns to the same cache entries at every depth instead of
//! starting a new chain. Built values are stored in an append-only arena, so a
//! borrow of one lasts as long as the borrow of the snapshot, without unsafe
//! code.
//!
//! The snapshot also keeps what the schema says about each scalar entry
//! ([`ScalarTyping`]), keyed by the entry's index in the snapshot's own tree.
//! There are two ways in, and they share one store:
//!
//! - [`FrontmatterAnalysis::scalar`] classifies one entry, for a request about
//!   one cursor (hover, definition). It resolves only that entry's path and
//!   never starts the document walk.
//! - [`FrontmatterAnalysis::inventory`] classifies every scalar entry, once per
//!   snapshot, on the first request that needs the whole document
//!   (diagnostics, document links). It reuses any entry a point request
//!   already classified, and every later caller reads the finished inventory.
//!
//! Every outcome is stored, including an entry no definition reaches and an
//! expression whose text cannot be decoded, so a miss is never retried.
//! Evaluated values and resolved file targets are not stored: they depend on
//! more than the snapshot.
//!
//! Building a snapshot does no I/O, evaluates nothing, and mutates no
//! document; it only reuses the inputs overlay preparation already produced.
//!
//! Two kinds of schema work outlive a snapshot ([`CrossSnapshotSchema`]):
//! synthesized array-item definitions, and the prepared union-alternative
//! matchers that tell which alternative a sequence item matches. Both copy or
//! lower whole alternatives, which for a recursive lifecycle type is megabytes
//! of work, and an edit re-assembles the schema even when it is unchanged. So
//! both are shared by every snapshot of one
//! [`OverlayState`](super::OverlayState) and keyed by the content they are
//! built from, never by a node of one snapshot. An item definition's key
//! covers only the atoms it is copied from, so taking one opens no definition
//! beyond those its synthesis opens.
//! A snapshot that uses a shared item definition holds its own `Arc` to it, so
//! the definition's address is still a stable identity for that snapshot.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use darkmatter::markdown::schemas::completion::{
    AlternativeMatcher, DefinitionLookup, alternative_fingerprint,
};
use darkmatter::markdown::schemas::{PropertyAtom, PropertyDef, SchemaShape, effective_property_shape};
use std::fmt::Write as _;
use std::hash::Hasher as _;
use serde_json::Value;

use super::expressions::ScalarProjection;
use super::{FmValueKind, FrontmatterAst, NamedTypeCatalog, SchemaBundle};
use crate::work_counters;

/// The identity of a schema node within one snapshot: the address of a
/// [`PropertyDef`] inside an owner the snapshot keeps alive.
type NodeId = usize;

fn address(def: &PropertyDef) -> NodeId {
    std::ptr::from_ref(def) as usize
}

/// Which of a property's inline-object arms one descent step may enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArmAdmission {
    /// Every inline-object arm, array or not (transparent array crossing).
    Any,
    /// Only array arms (an explicit step through an index).
    Items,
    /// Only non-array arms (an explicit key step).
    Values,
}

/// What a snapshot depends on. Reuse requires every field to match.
///
/// The bundle and named-type catalog are compared by `Arc` identity: overlay
/// preparation returns the same `Arc` only while the document text, the
/// configuration, the trigger registry, the document's context, and every
/// imported schema file are unchanged, so their identity carries those inputs.
/// The snapshot retains both `Arc`s, so an address cannot be reused by a
/// different value while it is compared.
#[derive(Clone)]
pub(crate) struct AnalysisIdentity {
    /// The client's buffer version; `None` for a caller with no versioned
    /// buffer (a test or a benchmark), where the text alone identifies it.
    pub revision: Option<i32>,
    /// The hash of the current buffer text.
    pub text_hash: u64,
    /// Whether the tree is the last-good tree of an earlier buffer.
    pub stale: bool,
    /// The hash of the text the tree was parsed from (the current text when
    /// the tree is fresh).
    pub tree_text_hash: u64,
    /// The schema-cache key overlay preparation computed (text,
    /// configuration, trigger-registry version, and context identity), or
    /// `None` when assembly was skipped.
    pub schema_key: Option<u64>,
    pub bundle: Option<Arc<SchemaBundle>>,
    pub named_types: Arc<NamedTypeCatalog>,
}

impl AnalysisIdentity {
    fn matches(&self, other: &Self) -> bool {
        self.revision == other.revision
            && self.text_hash == other.text_hash
            && self.stale == other.stale
            && self.tree_text_hash == other.tree_text_hash
            && self.schema_key == other.schema_key
            && match (&self.bundle, &other.bundle) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && Arc::ptr_eq(&self.named_types, &other.named_types)
    }
}

/// What the schema says about one scalar entry of a snapshot. An entry no
/// definition reaches is stored as [`ScalarTyping::UNRESOLVED`], neither a
/// file nor an expression, so the miss is not classified again.
#[derive(Debug)]
pub(crate) struct ScalarTyping {
    /// Whether any arm of the definition is a `file` type.
    pub(crate) file: bool,
    pub(crate) expression: ExpressionTyping,
}

/// Whether a scalar is an Expression-typed value, and its decoded text.
#[derive(Debug)]
pub(crate) enum ExpressionTyping {
    /// No arm of the definition is an `expression` type.
    No,
    /// Expression-typed, but the scalar has no decoded text to analyze.
    Undecodable,
    /// Expression-typed: the text the compose parser sees, and its projection
    /// back to authored bytes.
    Decoded { text: String, projection: ScalarProjection },
}

impl ScalarTyping {
    /// The outcome for an entry no definition reaches.
    pub(crate) const UNRESOLVED: Self = Self { file: false, expression: ExpressionTyping::No };
}

/// Schema work shared by every snapshot of one overlay state and keyed by
/// content: prepared union-alternative matchers and synthesized array-item
/// definitions. Equal keys build equal values, whichever assembly the source
/// came from.
///
/// Each kind is bounded: when [`CROSS_SNAPSHOT_LIMIT`] distinct values are
/// held, the next new one empties that kind first, so a schema that keeps
/// changing cannot grow it without limit. A snapshot keeps the item
/// definitions it took, so emptying never invalidates one in use.
#[derive(Default)]
pub(crate) struct CrossSnapshotSchema {
    matchers: Mutex<HashMap<u64, AlternativeMatcher>>,
    items: Mutex<HashMap<u64, Arc<Option<PropertyDef>>>>,
}

/// The most values of each kind one [`CrossSnapshotSchema`] holds.
const CROSS_SNAPSHOT_LIMIT: usize = 256;

impl CrossSnapshotSchema {
    /// Whether `value` is a valid instance of `atom`, with references
    /// resolved through `definitions`
    /// ([`alternative_accepts_in`](darkmatter::markdown::schemas::completion::alternative_accepts_in)),
    /// preparing the alternative only the first time its content is seen.
    pub(crate) fn accepts(&self, atom: &PropertyAtom, definitions: &dyn DefinitionLookup, value: &Value) -> bool {
        let Some(fingerprint) = alternative_fingerprint(atom, definitions) else {
            return false;
        };
        // Prepared outside the lock: compiling a validator takes
        // milliseconds, and two requests preparing the same alternative
        // build equal matchers.
        shared(&self.matchers, fingerprint, || AlternativeMatcher::prepare(atom, definitions)).accepts(value)
    }

    /// The item definition `build` synthesizes from an array's item `atoms`,
    /// shared with every earlier synthesis from equal atoms.
    fn item_definition(
        &self,
        atoms: &[&PropertyAtom],
        build: impl FnOnce(&[&PropertyAtom]) -> Option<PropertyDef>,
    ) -> Arc<Option<PropertyDef>> {
        let mut hasher = biscuit_hash::XxHasher::new();
        for atom in atoms {
            // Infallible: the sink never fails.
            let _ = write!(hasher, "\u{0}{atom:?}");
        }
        shared(&self.items, hasher.finish(), || {
            work_counters::record_item_synthesis();
            Arc::new(build(atoms))
        })
    }
}

/// The value `map` holds under `key`, built by `build` (without the lock) and
/// stored when there is none.
fn shared<V: Clone>(map: &Mutex<HashMap<u64, V>>, key: u64, build: impl FnOnce() -> V) -> V {
    let lock = || map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(held) = lock().get(&key) {
        return held.clone();
    }
    let value = build();
    let mut held = lock();
    if held.len() >= CROSS_SNAPSHOT_LIMIT {
        held.clear();
    }
    held.insert(key, value.clone());
    value
}

/// The shared analysis of one document snapshot.
pub struct FrontmatterAnalysis {
    identity: Option<AnalysisIdentity>,
    ast: Option<Arc<FrontmatterAst>>,
    root: OnceLock<SchemaShape>,
    /// The source identity of each merged-view property copied from a single
    /// arm.
    sources: Mutex<HashMap<NodeId, NodeId>>,
    /// The discriminant JSON of each arm, per property and admission.
    discriminants: SharedStore<(NodeId, ArmAdmission), Vec<Value>>,
    /// Merged views of several admissible arms, per property and admission;
    /// `None` when no arm is admissible.
    merged: SharedStore<(NodeId, ArmAdmission), Option<SchemaShape>>,
    /// Array-item definitions, per array property, each shared with the
    /// overlay state's other snapshots.
    items: SharedStore<NodeId, Arc<Option<PropertyDef>>>,
    /// The typing of each scalar entry classified so far, by entry index.
    scalars: SharedStore<usize, ScalarTyping>,
    /// Every scalar entry's index and slot in `scalars`, in document order,
    /// once a whole-document request has classified them all.
    inventory: OnceLock<Box<[(usize, usize)]>>,
    /// The overlay state's schema work that outlives this snapshot.
    shared: Arc<CrossSnapshotSchema>,
}

impl FrontmatterAnalysis {
    /// A snapshot for `identity` over `ast`, sharing the overlay state's
    /// cross-snapshot schema work.
    pub(crate) fn new(
        identity: AnalysisIdentity,
        ast: Option<Arc<FrontmatterAst>>,
        shared: Arc<CrossSnapshotSchema>,
    ) -> Self {
        Self {
            identity: Some(identity),
            ast,
            root: OnceLock::new(),
            sources: Mutex::default(),
            discriminants: SharedStore::default(),
            merged: SharedStore::default(),
            items: SharedStore::default(),
            scalars: SharedStore::default(),
            inventory: OnceLock::new(),
            shared,
        }
    }

    /// A snapshot for a document with no frontmatter tree (a standalone or
    /// trigger schema file). The overlay cache never keeps it.
    pub(crate) fn detached() -> Self {
        Self {
            identity: None,
            ast: None,
            root: OnceLock::new(),
            sources: Mutex::default(),
            discriminants: SharedStore::default(),
            merged: SharedStore::default(),
            items: SharedStore::default(),
            scalars: SharedStore::default(),
            inventory: OnceLock::new(),
            shared: Arc::default(),
        }
    }

    /// Whether this snapshot was built from exactly `identity`'s inputs.
    pub(crate) fn matches(&self, identity: &AnalysisIdentity) -> bool {
        self.identity.as_ref().is_some_and(|own| own.matches(identity))
    }

    /// The cross-snapshot schema work this snapshot shares.
    pub(crate) fn shared(&self) -> &CrossSnapshotSchema {
        &self.shared
    }

    /// The frontmatter tree this snapshot describes.
    pub(crate) fn ast(&self) -> Option<&Arc<FrontmatterAst>> {
        self.ast.as_ref()
    }

    fn bundle(&self) -> Option<&SchemaBundle> {
        self.identity.as_ref()?.bundle.as_deref()
    }

    /// The effective root shape, assembled on first use: the Darkmatter base
    /// properties, then each matched extension baseline, the matching trigger
    /// payloads, and the document's own `$schema`
    /// ([`EffectiveSchema::property_shape`](darkmatter::markdown::schemas::EffectiveSchema::property_shape)).
    pub(crate) fn root(&self) -> &SchemaShape {
        match self.bundle() {
            Some(bundle) => self.root.get_or_init(|| {
                work_counters::record_root_assembly();
                bundle
                    .effective
                    .property_shape(&bundle.extension_shapes, &bundle.frontmatter_json)
            }),
            None => baseline_root(),
        }
    }

    /// The identity `def` is cached under: its source's, when `def` is a
    /// merged view's copy of a single arm's property.
    fn node(&self, def: &PropertyDef) -> NodeId {
        let own = address(def);
        let sources = self.sources.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        sources.get(&own).copied().unwrap_or(own)
    }

    /// The discriminant JSON of `def`'s arms under `admission`, built once.
    pub(crate) fn discriminants(
        &self,
        def: &PropertyDef,
        admission: ArmAdmission,
        build: impl FnOnce() -> Vec<Value>,
    ) -> &[Value] {
        self.discriminants.get_or_insert_with((self.node(def), admission), build).0
    }

    /// The merged view of `def`'s admissible `arms` under `admission`, built
    /// by `build` once per snapshot.
    pub(crate) fn merged_shape(
        &self,
        def: &PropertyDef,
        admission: ArmAdmission,
        arms: &[Option<&SchemaShape>],
        build: impl FnOnce() -> Option<SchemaShape>,
    ) -> Option<&SchemaShape> {
        let (merged, built) = self.merged.get_or_insert_with((self.node(def), admission), || {
            work_counters::record_merged_shape();
            build()
        });
        let merged = merged.as_ref()?;
        if built {
            self.record_sources(merged, arms);
        }
        Some(merged)
    }

    /// Records the source of each of `merged`'s properties that exactly one
    /// of `arms` declares.
    fn record_sources(&self, merged: &SchemaShape, arms: &[Option<&SchemaShape>]) {
        let copies: Vec<(NodeId, NodeId)> = merged
            .properties
            .iter()
            .filter_map(|(name, copy)| {
                let mut declaring = arms.iter().flatten().filter_map(|arm| arm.properties.get(name));
                match (declaring.next(), declaring.next()) {
                    (Some(source), None) => Some((address(copy), self.node(source))),
                    _ => None,
                }
            })
            .collect();
        self.sources.lock().unwrap_or_else(std::sync::PoisonError::into_inner).extend(copies);
    }

    /// The item definition of the array property `def`, taken once per
    /// snapshot from the cross-snapshot store: `atoms` lists the item atoms of
    /// `def`'s array arms, and `build` synthesizes the definition from them
    /// only for atoms the store has not seen.
    pub(crate) fn item_definition<'a>(
        &self,
        def: &PropertyDef,
        atoms: impl FnOnce() -> Vec<&'a PropertyAtom>,
        build: impl FnOnce(&[&PropertyAtom]) -> Option<PropertyDef>,
    ) -> Option<&PropertyDef> {
        self.items
            .get_or_insert_with(self.node(def), || self.shared.item_definition(&atoms(), build))
            .0
            .as_ref()
            .as_ref()
    }

    /// The typing of the scalar entry `index`, classified by `classify` the
    /// first time any request asks. Never starts the document walk.
    pub(crate) fn scalar(&self, index: usize, classify: impl FnOnce() -> ScalarTyping) -> &ScalarTyping {
        self.scalars.get_or_insert_with(index, || {
            work_counters::record_scalar_classification();
            classify()
        })
        .0
    }

    /// Every scalar entry of the snapshot's tree with its typing, in document
    /// order. The first call walks the tree once, classifying with `classify`
    /// each entry no earlier request classified; every later call, from any
    /// request on this snapshot, reads the stored result. `classify` must not
    /// ask for the inventory itself.
    pub(crate) fn inventory<'s>(
        &'s self,
        classify: &mut dyn FnMut(usize) -> ScalarTyping,
    ) -> impl Iterator<Item = (usize, &'s ScalarTyping)> + use<'s> {
        let slots = self.inventory.get_or_init(|| {
            work_counters::record_typing_walk();
            let Some(ast) = &self.ast else {
                return Box::default();
            };
            ast.entries()
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.kind == FmValueKind::Scalar)
                .map(|(index, _)| {
                    let slot = self.scalars.slot_or_insert_with(index, || {
                        work_counters::record_scalar_classification();
                        classify(index)
                    });
                    (index, slot)
                })
                .collect()
        });
        slots.iter().map(|&(index, slot)| (index, self.scalars.value(slot)))
    }
}

/// The effective root of a document with no schema bundle: the Darkmatter
/// base properties alone. Its inputs are compiled in, so it is a constant
/// built once per process rather than a cache.
pub(crate) fn baseline_root() -> &'static SchemaShape {
    static BASELINE: LazyLock<SchemaShape> = LazyLock::new(|| {
        work_counters::record_root_assembly();
        effective_property_shape(&[], None, &Value::Null)
    });
    &BASELINE
}

/// A map from key to a value built at most once, whose values are borrowed for
/// as long as the store is.
struct SharedStore<K, V> {
    /// Each key's slot in `values`, and the number of slots used.
    index: Mutex<(HashMap<K, usize>, usize)>,
    values: AppendOnly<V>,
}

impl<K, V> Default for SharedStore<K, V> {
    fn default() -> Self {
        Self { index: Mutex::new((HashMap::new(), 0)), values: AppendOnly::default() }
    }
}

impl<K: Eq + Hash, V> SharedStore<K, V> {
    /// The value stored under `key`, building it first when there is none,
    /// and whether this call stored it.
    fn get_or_insert_with(&self, key: K, build: impl FnOnce() -> V) -> (&V, bool) {
        let (slot, built) = self.slot_with(key, build);
        (self.values.get(slot), built)
    }

    /// The slot of the value stored under `key`, building it first when
    /// there is none; read it back with [`Self::value`].
    fn slot_or_insert_with(&self, key: K, build: impl FnOnce() -> V) -> usize {
        self.slot_with(key, build).0
    }

    /// The value in a slot [`Self::slot_or_insert_with`] returned.
    fn value(&self, slot: usize) -> &V {
        self.values.get(slot)
    }

    fn slot_with(&self, key: K, build: impl FnOnce() -> V) -> (usize, bool) {
        if let Some(&slot) = self.lock().0.get(&key) {
            return (slot, false);
        }
        // Built without the lock, so a builder may use the snapshot's other
        // stores. A concurrent request that built the same key first wins.
        let value = build();
        let mut index = self.lock();
        if let Some(&slot) = index.0.get(&key) {
            return (slot, false);
        }
        let slot = index.1;
        self.values.set(slot, value);
        index.1 += 1;
        index.0.insert(key, slot);
        (slot, true)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, (HashMap<K, usize>, usize)> {
        self.index.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The first chunk's length; chunk `n` holds `FIRST_CHUNK << n` slots.
const FIRST_CHUNK: usize = 16;
const CHUNKS: usize = 32;

/// Write-once slots whose values never move, grown a chunk at a time.
struct AppendOnly<V> {
    chunks: [OnceLock<Box<[OnceLock<V>]>>; CHUNKS],
}

impl<V> Default for AppendOnly<V> {
    fn default() -> Self {
        Self { chunks: std::array::from_fn(|_| OnceLock::new()) }
    }
}

impl<V> AppendOnly<V> {
    /// The chunk and offset of `slot`.
    fn locate(slot: usize) -> (usize, usize) {
        let chunk = (slot / FIRST_CHUNK + 1).ilog2() as usize;
        (chunk, slot - FIRST_CHUNK * ((1 << chunk) - 1))
    }

    /// Fills `slot`, which must be unused; the caller serializes slot
    /// assignment.
    fn set(&self, slot: usize, value: V) {
        let (chunk, offset) = Self::locate(slot);
        let slots = self.chunks[chunk]
            .get_or_init(|| (0..FIRST_CHUNK << chunk).map(|_| OnceLock::new()).collect());
        assert!(slots[offset].set(value).is_ok(), "slot {slot} is filled once");
    }

    fn get(&self, slot: usize) -> &V {
        let (chunk, offset) = Self::locate(slot);
        self.chunks[chunk]
            .get()
            .and_then(|slots| slots[offset].get())
            .expect("a slot is read only after it is filled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_only_slots_keep_their_values_across_chunks() {
        let store: SharedStore<usize, usize> = SharedStore::default();
        let (first, built) = store.get_or_insert_with(0, || 100);
        assert!(built);
        let first: *const usize = first;
        for key in 1..1_000 {
            assert_eq!(*store.get_or_insert_with(key, || key * 2).0, key * 2);
        }
        assert_eq!(store.get_or_insert_with(0, || unreachable!("built once")), (&100, false));
        assert!(std::ptr::eq(first, store.get_or_insert_with(0, || 0).0), "a value never moves");
        assert_eq!(AppendOnly::<()>::locate(15), (0, 15));
        assert_eq!(AppendOnly::<()>::locate(16), (1, 0));
        assert_eq!(AppendOnly::<()>::locate(47), (1, 31));
        assert_eq!(AppendOnly::<()>::locate(48), (2, 0));
    }
}
