//! Default preparation: the one step that fills a property from its
//! `default(...)`.
//!
//! Preparing defaults and validating are separate steps. Preparation builds
//! the completed candidate; validation then judges that candidate and sets
//! nothing. [`DefaultPlan::compile`] reads a lowered schema (or a raw JSON
//! Schema) once, and [`prepare_defaults`] applies the plan to each value.
//! [`crate::Prepared`] and a host's own compiled schema (Darkmatter's
//! `EffectiveSchema`) hold a plan beside their validators.
//!
//! The rules, in the lowered JSON Schema's terms:
//!
//! - A property whose value is missing or `null` takes its declared
//!   `default`, whether or not it is `required`. A supplied non-null value
//!   always wins: `0`, `false`, `""`, `[]`, and `{}` are values.
//! - A `null` default fills nothing, except that a missing key of a
//!   `required` property is inserted as `null` (and reported); a written
//!   `null` already meets that contract.
//! - Defaults are collected per object location, across `$ref` targets,
//!   `allOf` members, and every `anyOf`/`oneOf` arm, without choosing an arm.
//!   Equal declarations count as one; different values for one property are
//!   [`SchemaError::ConflictingDefaults`] when the plan is compiled, used or
//!   not. Which arm the completed value then matches is validation's
//!   question.
//! - A key's value is described by every schema that selects it: its
//!   `properties` entries, every `patternProperties` schema whose pattern
//!   the key matches (JSON Schema semantics; SimplifiedSchema's literal-key
//!   precedence is already in the lowered patterns), and the
//!   `additionalProperties` of each node that neither names the key nor
//!   matches it with one of its own patterns. Together they are the key's
//!   one property definition: its default, `required`, default check, and
//!   children come from all of them at once. A literal name's definition is
//!   merged when the plan compiles; any other key's is merged from the key
//!   itself, at preparation, where two of its schemas declaring different
//!   defaults is [`SchemaError::ConflictingDefaults`]. Only a key that is
//!   there, or that some schema names, is prepared: a pattern or catch-all
//!   never creates a key.
//! - Conditional applicators (`if`/`then`/`else`, `dependentSchemas`), `not`,
//!   and annotation-dependent keywords (`unevaluatedProperties`,
//!   `unevaluatedItems`, `contains`) are not followed: whether they describe
//!   a value is a judgment of that value, which is validation's step.
//! - A child is filled only inside a parent that exists. A missing or `null`
//!   parent with no default of its own stays as it is. A parent filled from
//!   its own default is first checked against its complete definition
//!   (required children included), then descended into so its optional
//!   children are completed too.
//! - Array items and function-style tuple positions are never inserted;
//!   objects inside existing items are completed.
//! - Every default used is checked strictly, without I/O or conversion,
//!   before its children are completed, against the definitions of its
//!   property (the selectors of its key, and the `allOf` members, `$ref`
//!   targets and array item schemas of its location), judged from the arm
//!   of each definition declaring it. Every definition on that arm must
//!   accept the default. A definition on another member of a union the
//!   declaring arm is on is a sibling alternative and is not consulted. A
//!   union the declaring arm makes no choice in stays a group of
//!   alternatives: at least one of its members must accept the default,
//!   each member judged the same way (its own nested unions within it). A
//!   member defining nothing for the key accepts only if, read statically,
//!   it can hold an object with that key, since that object exists: a
//!   scalar, array, `null`-only or `false` member, or a closed object not
//!   naming the key, never does. (A `null`-only member beside a single
//!   other member is no alternative at all, so that member's definitions
//!   apply directly.) Each such union is required on its own.
//!   A violation is [`SchemaError::InvalidDefault`]; a default
//!   object is never repaired by its children's defaults. An unused default
//!   is never judged.
//! - Object properties are visited in declaration order and array items in
//!   ascending index order, depth first, so a parent always comes before its
//!   children; each changed path is reported once.
//! - A value at or below one of the caller's nested templates
//!   ([`NestedTemplates`]) is pending and is not filled.
//!
//! A `{{ … }}` slot in a `default(...)` never reaches this module: every
//! lowering entry refuses an unresolved slot, so a pending default stays
//! pending until the host resolves it.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use indexmap::IndexMap;
use jsonschema::Validator;
use serde_json::Value;

use crate::coerce::NestedTemplates;
use crate::coercion::TextPattern;
use crate::validate::{SchemaIdentity, ValidatorCache};
use crate::{JsonPointer, PropertyPath, SchemaError, SchemaOrigin, ValidationProblem, scope, simplified};

/// A default that preparation placed in the value.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedDefault {
    /// Where the default goes, from the document root.
    pub path: PropertyPath,
    /// The property's resolved default.
    pub value: Value,
    /// JSON Pointer of the `default` keyword in the lowered JSON Schema, as
    /// [`SchemaError::InvalidDefault`] names it (`/properties/doc/default`).
    /// Where several schemas select the property with one equal default,
    /// the first declaration's.
    pub pointer: String,
    /// Where the default is declared, when a host knows it: Darkmatter
    /// names the file a `Name@file` import read it from
    /// ([`crate::resolve::ResolvedSchema::default_files`]), else the layer
    /// that declared its top-level property. `None` from this crate, whose
    /// caller holds the whole schema's source.
    pub origin: Option<SchemaOrigin>,
}

impl AppliedDefault {
    /// Whether the value at `instance_path` is this default or lies inside
    /// it.
    pub fn holds(&self, instance_path: &JsonPointer) -> bool {
        instance_path.segments().starts_with(self.path.segments())
    }
}

/// Attributes each of `problems` about a value one of `defaults` inserted
/// to that default (the deepest one holding it): the problem records the
/// default in [`ValidationProblem::default`], its message names the
/// default's schema location (and schema file, when known), and any line
/// or column is cleared, since no authored text holds the value.
///
/// A problem already attributed is left alone, so a host may call this
/// again over the same problems.
///
/// Pass only the defaults that filled a missing key ([`filling_missing`]):
/// one that replaced a written `null` sits under a key the author wrote,
/// so a problem with it keeps that key's position.
pub fn attribute_problems(problems: &mut [ValidationProblem], defaults: &[AppliedDefault]) {
    if defaults.is_empty() {
        return;
    }
    for problem in problems.iter_mut().filter(|problem| problem.default.is_none()) {
        let Some(applied) = defaults
            .iter()
            .filter(|applied| applied.holds(&problem.instance_path))
            .max_by_key(|applied| applied.path.segments().len())
        else {
            continue;
        };
        let file = applied.origin.as_ref().and_then(SchemaOrigin::file);
        problem.message = match file {
            Some(file) => format!(
                "{} (the value is the schema default at schema location `{}` in `{}`)",
                problem.message,
                applied.pointer,
                file.display()
            ),
            None => format!("{} (the value is the schema default at schema location `{}`)", problem.message, applied.pointer),
        };
        problem.line = None;
        problem.column = None;
        problem.default = Some(applied.clone());
    }
}

/// The defaults of `defaults` whose key `written` (the value before any
/// default was applied) does not hold: the ones no authored text holds.
pub fn filling_missing(defaults: &[AppliedDefault], written: &Value) -> Vec<AppliedDefault> {
    defaults.iter().filter(|applied| applied.path.resolve(written).value().is_none()).cloned().collect()
}

/// The value to judge once missing and `null` properties hold their
/// defaults.
#[derive(Debug, Clone)]
pub struct DefaultPreparation<'input> {
    /// The input with every applied default in place; borrowed exactly when
    /// `defaults` is empty.
    pub candidate: Cow<'input, Value>,
    /// The defaults applied, in walk order (see the module docs).
    pub defaults: Vec<AppliedDefault>,
}

/// How many compiled plans [`DefaultPlan::for_identity`] keeps before it
/// starts over.
const PLAN_CACHE_CAPACITY: usize = 64;

static PLAN_CACHE: LazyLock<Mutex<HashMap<SchemaIdentity, Arc<DefaultPlan>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// How many compiled key patterns [`key_pattern`] keeps before it starts
/// over.
const PATTERN_CACHE_CAPACITY: usize = 1024;

/// Compiled `patternProperties` keys by source: a named type inlined at many
/// locations (the baseline's lifecycle stacks) repeats one long pattern, and
/// lowering checks declarations on every call.
static PATTERN_CACHE: LazyLock<Mutex<HashMap<String, Option<Arc<TextPattern>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// How many accepted documents [`check_declarations`] remembers before it
/// starts over.
const CHECKED_CAPACITY: usize = 256;

/// The [`written_digest`]s of documents [`check_declarations`] has
/// accepted: hosts lower one baseline again and again, and the check reads
/// the whole lowered document. A refusal is not remembered.
static CHECKED: LazyLock<Mutex<std::collections::HashSet<u64>>> = LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

pub(crate) fn reset_process_caches() {
    PLAN_CACHE.lock().expect("default plan cache poisoned").clear();
    CHECKED.lock().expect("checked declarations poisoned").clear();
    PATTERN_CACHE.lock().expect("key pattern cache poisoned").clear();
}

/// `source` compiled as a key pattern; `None` when it does not compile.
fn key_pattern(source: &str) -> Option<Arc<TextPattern>> {
    if let Some(known) = PATTERN_CACHE.lock().expect("key pattern cache poisoned").get(source) {
        return known.clone();
    }
    let compiled = TextPattern::new(source).ok().map(Arc::new);
    let mut patterns = PATTERN_CACHE.lock().expect("key pattern cache poisoned");
    if patterns.len() >= PATTERN_CACHE_CAPACITY {
        patterns.clear();
    }
    patterns.insert(source.to_string(), compiled.clone());
    compiled
}

/// The defaults one schema declares, per object location, compiled once
/// and applied to every value by [`prepare_defaults`].
pub struct DefaultPlan {
    schema: Arc<Value>,
    /// Compiles the structural validators that check a used default.
    cache: ValidatorCache,
    /// Object locations; `None` when the schema declares no structure.
    root: Option<usize>,
    scopes: Vec<Scope>,
    /// Whether any location declares a default.
    declares_defaults: bool,
}

impl std::fmt::Debug for DefaultPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPlan").field("scopes", &self.scopes.len()).finish_non_exhaustive()
    }
}

/// One object location: the merge of every schema node that can describe
/// the value there.
#[derive(Default)]
struct Scope {
    /// The first node's JSON Pointer, naming the location in an error.
    pointer: String,
    /// Every literal property name, each with every schema that selects it.
    properties: Vec<Property>,
    /// The names any node lists in `required`.
    required: Vec<String>,
    /// The `patternProperties` of every node, for a key no node names.
    patterns: Vec<KeyPattern>,
    /// The `additionalProperties` schema of every node that has one.
    catch_alls: Vec<CatchAll>,
    /// The location of each leading array item.
    prefix: Vec<Option<usize>>,
    /// The location of every array item after `prefix`.
    rest: Option<usize>,
}

/// What a set of schemas selecting one key says about its value.
struct Entry {
    child: Option<usize>,
    default: Option<Declared>,
    /// Every schema selecting the key, by which a used default is judged
    /// from its declaring arm (see [`Fill::accepts`]).
    definitions: Vec<Definition>,
}

/// The union members a schema node is reached through from the document
/// root: `(union pointer, member index)`, at most one per union, sorted.
/// Empty for a node that applies whichever arm a value matches.
type Arm = Vec<(String, usize)>;

/// One schema selecting a key, as a used default is checked against it.
struct Definition {
    pointer: String,
    arm: Arc<Arm>,
    /// Its structural validator, compiled on first use.
    validator: OnceLock<Arc<Validator>>,
}

/// Whether `context` chooses a member of `union`.
fn chosen(context: &[(&str, usize)], union: &str) -> bool {
    context.iter().any(|(other, _)| *other == union)
}

/// Whether a definition reached through `arm` is on a member of a union
/// other than the one `context` chooses: a sibling alternative, which a
/// default judged within `context` is not checked against.
fn conflicts(arm: &Arm, context: &[(&str, usize)]) -> bool {
    arm.iter().any(|(union, index)| context.iter().any(|(other, at)| other == union && at != index))
}

struct Property {
    name: String,
    entry: Entry,
}

/// One `patternProperties` entry of the node at index `node` among its
/// location's nodes.
struct KeyPattern {
    node: usize,
    pattern: Arc<TextPattern>,
    entry: Entry,
}

/// The `additionalProperties` of the node at index `node`: it selects a key
/// that node neither names nor matches with one of its own patterns.
struct CatchAll {
    node: usize,
    /// The node's own `properties` names.
    names: Vec<String>,
    entry: Entry,
}

impl Scope {
    /// Whether `catch_all` selects `key`.
    fn catches(&self, catch_all: &CatchAll, key: &str) -> bool {
        !catch_all.names.iter().any(|name| name == key)
            && !self.patterns.iter().any(|entry| entry.node == catch_all.node && entry.pattern.is_match(key))
    }
}

/// A property's one default.
struct Declared {
    value: Value,
    /// The JSON Pointer of the first `default` keyword declaring it.
    pointer: String,
    /// The arm of each definition declaring it: a used default must be
    /// accepted from each (see [`Fill::accepts`]).
    arms: Vec<Arc<Arm>>,
}

impl DefaultPlan {
    /// Reads every default `schema` declares, a lowered or raw JSON Schema,
    /// following local `$ref`s, `allOf`, `anyOf`, `oneOf`, properties,
    /// `patternProperties`, `additionalProperties`, and array items.
    /// Recursive references compile once. A used default is checked by
    /// structural validators compiled through `cache`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::ConflictingDefaults`] when one object location declares
    /// different defaults for one property (on different union arms, say);
    /// a key that only patterns or catch-alls select is checked later, by
    /// [`prepare_defaults`];
    /// [`SchemaError::RecursiveDefault`] when a `$ref` chain returns to itself
    /// without describing a child value.
    pub fn compile(schema: Arc<Value>, cache: &ValidatorCache) -> Result<Self, SchemaError> {
        let (root, scopes) = Compiler::run(&schema)?;
        let declares_defaults = scopes.iter().any(|scope| {
            scope.properties.iter().map(|property| &property.entry)
                .chain(scope.patterns.iter().map(|pattern| &pattern.entry))
                .chain(scope.catch_alls.iter().map(|catch_all| &catch_all.entry))
                .any(|entry| entry.default.is_some())
        });
        Ok(Self { schema, cache: cache.clone(), root, scopes, declares_defaults })
    }

    /// [`Self::compile`] for a schema whose identity the caller knows (the
    /// identity its validators are cached under), compiled once per identity
    /// for the process.
    ///
    /// ## Errors
    ///
    /// As [`Self::compile`]; an error is not remembered.
    pub fn for_identity(identity: SchemaIdentity, schema: &Arc<Value>, cache: &ValidatorCache) -> Result<Arc<Self>, SchemaError> {
        if let Some(plan) = PLAN_CACHE.lock().expect("default plan cache poisoned").get(&identity) {
            return Ok(Arc::clone(plan));
        }
        let plan = Arc::new(Self::compile(Arc::clone(schema), cache)?);
        let mut plans = PLAN_CACHE.lock().expect("default plan cache poisoned");
        if plans.len() >= PLAN_CACHE_CAPACITY {
            plans.clear();
        }
        plans.insert(identity, Arc::clone(&plan));
        Ok(plan)
    }

    /// Whether the schema declares any default at all, so preparation has
    /// nothing to do for any value.
    pub fn is_empty(&self) -> bool {
        !self.declares_defaults
    }
}

/// Fills the missing and `null` properties of `instance` from `plan`.
///
/// The candidate is not judged as a whole here; the caller validates it as
/// it validates any value. Nothing is read from the process or the file
/// system, and no expression is evaluated.
///
/// ## Errors
///
/// [`SchemaError::InvalidDefault`] when a default the value needs breaks its
/// own property's definition; [`SchemaError::RecursiveDefault`] when filling
/// a default keeps creating another defaulted parent;
/// [`SchemaError::ConflictingDefaults`] when a key no schema names is
/// selected by several `patternProperties` (or a pattern and a catch-all)
/// that default it, or one of its children, differently; [`SchemaError::BuildValidator`] when a default's
/// definition does not compile.
pub fn prepare_defaults<'input>(
    plan: &DefaultPlan,
    instance: Cow<'input, Value>,
    templates: &NestedTemplates,
) -> Result<DefaultPreparation<'input>, SchemaError> {
    let Some(root) = plan.root.filter(|_| !plan.is_empty()) else {
        return Ok(DefaultPreparation { candidate: instance, defaults: Vec::new() });
    };
    let mut fill = Fill { plan, templates, active: Vec::new(), out: Vec::new() };
    fill.visit(&[root], &instance, &mut Vec::new())?;
    Ok(finish(instance, fill.out))
}

// ── Compilation ───────────────────────────────────────────────────────────

/// A schema node reached through an arm: the ids [`Compiler`] interns its
/// JSON Pointer and its [`Arm`] under.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Reached {
    pointer: u32,
    arm: u32,
}

/// An interned JSON Pointer, with what compilation reads of its node.
struct Pointer<'s> {
    text: String,
    node: Option<&'s serde_json::Map<String, Value>>,
    /// Whether the node describes an object's or an array's members.
    structural: bool,
    /// The nodes it expands to, resolved on its first expansion.
    links: Option<Arc<Links<'s>>>,
    /// The schemas of its members, resolved when a location first holds it.
    members: Option<Arc<Members<'s>>>,
}

/// The schemas one structural node gives its object's and array's members.
struct Members<'s> {
    properties: Vec<(&'s str, u32)>,
    /// Each `patternProperties` entry whose pattern compiles.
    patterns: Vec<(Arc<TextPattern>, &'s str, u32)>,
    /// The `additionalProperties` schema, when it is one.
    additional: Option<u32>,
    /// The `items` schema, when it is one.
    items: Option<u32>,
    prefix: Vec<u32>,
}

/// The nodes one node expands to.
struct Links<'s> {
    /// The local `$ref` and its target.
    reference: Option<(&'s String, u32)>,
    /// Each `allOf`/`anyOf`/`oneOf`: the union, whether its members are
    /// alternatives, and the members.
    unions: Vec<(u32, bool, Vec<u32>)>,
}

/// An arm as interned: `(union pointer id, member index)`, sorted by id.
type ArmKey = Vec<(u32, usize)>;

/// The multiply-rotate hasher of Firefox and rustc, for the compiler's id
/// keys: in a debug build SipHash over them cost more than the rest of
/// compilation.
#[derive(Default)]
struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for chunk in chunks {
            self.write_u64(u64::from_le_bytes(*chunk));
        }
        for &byte in rest {
            self.write_u64(u64::from(byte));
        }
    }

    fn write_u32(&mut self, value: u32) {
        self.write_u64(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type IdMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<IdHasher>>;
type IdSet<K> = std::collections::HashSet<K, std::hash::BuildHasherDefault<IdHasher>>;

struct Compiler<'s> {
    root: &'s Value,
    scopes: Vec<Scope>,
    /// Each location's id, keyed by its sorted nodes.
    known: IdMap<Vec<Reached>, usize>,
    /// The location [`Self::scope`] returned for each list of nodes it was
    /// given, before expansion.
    entered: IdMap<Vec<Reached>, Option<usize>>,
    // Pointers and arms are interned: a union-heavy schema (the lifecycle
    // baseline) reaches one node through thousands of arms, and every
    // location is keyed by its nodes, so hashing and comparing them as
    // strings dominated compilation.
    pointers: Vec<Pointer<'s>>,
    pointer_ids: IdMap<String, u32>,
    arms: Vec<ArmKey>,
    arm_ids: IdMap<ArmKey, u32>,
    /// Each arm as a [`Definition`] carries it, built on first use.
    spelled: Vec<Option<Arc<Arm>>>,
    /// `(arm, union, member)` to the arm that chooses that member.
    choices: IdMap<(u32, u32, usize), u32>,
    /// [`Self::expand`] of one node on its own.
    expansions: IdMap<Reached, Arc<[Reached]>>,
    /// Whether union members record their choice in the arm.
    track_arms: bool,
}

impl<'s> Compiler<'s> {
    /// Compiles every location of `root`.
    fn run(root: &'s Value) -> Result<(Option<usize>, Vec<Scope>), SchemaError> {
        let mut compiler = Self::new(root, true);
        let root_scope = compiler.root_scope()?;
        Ok((root_scope, compiler.scopes))
    }

    /// A compiler for `root`. Without `arms` every node keeps the root's
    /// empty arm, so each location is compiled once rather than once per arm
    /// reaching it (see [`check_declarations`]).
    fn new(root: &'s Value, arms: bool) -> Self {
        Compiler {
            root,
            track_arms: arms,
            scopes: Vec::new(),
            known: IdMap::default(),
            entered: IdMap::default(),
            pointers: Vec::new(),
            pointer_ids: IdMap::default(),
            arms: Vec::new(),
            arm_ids: IdMap::default(),
            spelled: Vec::new(),
            choices: IdMap::default(),
            expansions: IdMap::default(),
        }
    }

    /// Compiles the root location and every location below it.
    fn root_scope(&mut self) -> Result<Option<usize>, SchemaError> {
        let pointer = self.pointer(String::new(), Some(self.root));
        let arm = self.arm(ArmKey::new());
        self.scope(&[Reached { pointer, arm }])
    }

    /// Whether a node expanded so far expands, through `$ref`s and
    /// `allOf`/`anyOf`/`oneOf` members, back to itself: the only way
    /// [`Self::expand`] fails.
    fn loops(&self) -> bool {
        let successors = |at: usize| -> Vec<usize> {
            let Some(links) = &self.pointers[at].links else { return Vec::new() };
            links
                .reference
                .iter()
                .map(|(_, target)| *target)
                .chain(links.unions.iter().flat_map(|(_, _, members)| members.iter().copied()))
                .map(|next| next as usize)
                .collect()
        };
        // 0: not yet met, 1: on the current path, 2: finished.
        let mut state = vec![0u8; self.pointers.len()];
        for start in 0..self.pointers.len() {
            if state[start] != 0 {
                continue;
            }
            state[start] = 1;
            let mut path: Vec<(usize, Vec<usize>)> = vec![(start, successors(start))];
            while let Some((at, next)) = path.last_mut() {
                let Some(next) = next.pop() else {
                    state[*at] = 2;
                    path.pop();
                    continue;
                };
                match state[next] {
                    0 => {
                        state[next] = 1;
                        path.push((next, successors(next)));
                    }
                    1 => return true,
                    _ => {}
                }
            }
        }
        false
    }

    /// The id of the pointer `text`, which names `value` (resolving
    /// `text` again would only find the same value).
    fn pointer(&mut self, text: String, value: Option<&'s Value>) -> u32 {
        if let Some(&id) = self.pointer_ids.get(&text) {
            return id;
        }
        let id = u32::try_from(self.pointers.len()).expect("fewer than 2^32 schema nodes");
        let node = value.and_then(Value::as_object);
        let structural = node.is_some_and(|map| {
            map.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "properties" | "required" | "patternProperties" | "additionalProperties" | "items" | "prefixItems"
                )
            })
        });
        self.pointer_ids.insert(text.clone(), id);
        self.pointers.push(Pointer { text, node, structural, links: None, members: None });
        id
    }

    fn arm(&mut self, arm: ArmKey) -> u32 {
        if let Some(&id) = self.arm_ids.get(&arm) {
            return id;
        }
        let id = u32::try_from(self.arms.len()).expect("fewer than 2^32 arms");
        self.arm_ids.insert(arm.clone(), id);
        self.arms.push(arm);
        self.spelled.push(None);
        id
    }

    /// The arm `arm` as a [`Definition`] carries it.
    fn spell(&mut self, arm: u32) -> Arc<Arm> {
        if let Some(spelled) = &self.spelled[arm as usize] {
            return Arc::clone(spelled);
        }
        let mut choices: Arm =
            self.arms[arm as usize].iter().map(|&(union, index)| (self.text(union).to_string(), index)).collect();
        choices.sort();
        let spelled = Arc::new(choices);
        self.spelled[arm as usize] = Some(Arc::clone(&spelled));
        spelled
    }

    /// `arm` choosing member `index` of the union at `union`, in place of
    /// any earlier choice of that union.
    fn choose(&mut self, arm: u32, union: u32, index: usize) -> u32 {
        if let Some(&id) = self.choices.get(&(arm, union, index)) {
            return id;
        }
        let mut chosen: ArmKey = self.arms[arm as usize].iter().copied().filter(|&(other, _)| other != union).collect();
        chosen.push((union, index));
        chosen.sort_unstable();
        let id = self.arm(chosen);
        self.choices.insert((arm, union, index), id);
        id
    }

    fn node(&self, pointer: u32) -> Option<&'s serde_json::Map<String, Value>> {
        self.pointers[pointer as usize].node
    }

    fn text(&self, pointer: u32) -> &str {
        &self.pointers[pointer as usize].text
    }

    /// The member schemas of the node at `pointer`, an object.
    fn members(&mut self, pointer: u32, map: &'s serde_json::Map<String, Value>) -> Arc<Members<'s>> {
        if let Some(members) = &self.pointers[pointer as usize].members {
            return Arc::clone(members);
        }
        let text = self.text(pointer).to_string();
        let mut properties = Vec::new();
        if let Some(Value::Object(entries)) = map.get("properties") {
            for (name, value) in entries {
                properties.push((name.as_str(), self.pointer(format!("{text}/properties/{}", escape(name)), Some(value))));
            }
        }
        let mut patterns = Vec::new();
        if let Some(Value::Object(entries)) = map.get("patternProperties") {
            for (source, value) in entries {
                // A pattern the validator's engine also refuses fails
                // validator compilation; it describes nothing here.
                if let Some(pattern) = key_pattern(source) {
                    let pointer = self.pointer(format!("{text}/patternProperties/{}", escape(source)), Some(value));
                    patterns.push((pattern, source.as_str(), pointer));
                }
            }
        }
        let additional = map
            .get("additionalProperties")
            .filter(|value| value.is_object())
            .map(|value| self.pointer(format!("{text}/additionalProperties"), Some(value)));
        let items = map.get("items").filter(|value| value.is_object()).map(|value| self.pointer(format!("{text}/items"), Some(value)));
        let prefix = match map.get("prefixItems") {
            Some(Value::Array(prefix)) => prefix
                .iter()
                .enumerate()
                .map(|(index, value)| self.pointer(format!("{text}/prefixItems/{index}"), Some(value)))
                .collect(),
            _ => Vec::new(),
        };
        let members = Arc::new(Members { properties, patterns, additional, items, prefix });
        self.pointers[pointer as usize].members = Some(Arc::clone(&members));
        members
    }

    /// The nodes the node at `pointer`, an object, expands to.
    fn links(&mut self, pointer: u32, map: &'s serde_json::Map<String, Value>) -> Arc<Links<'s>> {
        if let Some(links) = &self.pointers[pointer as usize].links {
            return Arc::clone(links);
        }
        let reference = match map.get("$ref") {
            Some(Value::String(reference)) => {
                reference.strip_prefix('#').map(|target| (reference, self.pointer(target.to_string(), self.root.pointer(target))))
            }
            _ => None,
        };
        let mut unions = Vec::new();
        for keyword in ["allOf", "anyOf", "oneOf"] {
            if let Some(Value::Array(members)) = map.get(keyword) {
                let union = format!("{}/{keyword}", self.text(pointer));
                let alternatives = keyword != "allOf" && members.iter().filter(|member| !admits_only_null(member)).count() > 1;
                let members =
                    members.iter().enumerate().map(|(index, member)| self.pointer(format!("{union}/{index}"), Some(member))).collect();
                unions.push((self.pointer(union, None), alternatives, members));
            }
        }
        let links = Arc::new(Links { reference, unions });
        self.pointers[pointer as usize].links = Some(Arc::clone(&links));
        links
    }

    /// Every schema node that applies wherever `at`'s node applies:
    /// itself, its local `$ref` target, and its `allOf`/`anyOf`/`oneOf`
    /// members, transitively, in that order. A `$ref` target and an `allOf`
    /// member keep the arm; an `anyOf`/`oneOf` member adds its own choice,
    /// replacing an earlier choice of the same union (a recursive schema
    /// meets it again one level down), unless it is the union's only member
    /// admitting a non-null value: a value these nodes describe exists, so
    /// that member is no alternative.
    fn expand(
        &mut self,
        at: Reached,
        chain: &mut Vec<u32>,
        seen: &mut IdSet<Reached>,
        out: &mut Vec<Reached>,
    ) -> Result<(), SchemaError> {
        if seen.contains(&at) {
            return Ok(());
        }
        let Some(map) = self.node(at.pointer) else {
            return Ok(());
        };
        seen.insert(at);
        out.push(at);
        chain.push(at.pointer);
        let links = self.links(at.pointer, map);
        if let Some((reference, target)) = links.reference {
            if chain.contains(&target) {
                return Err(SchemaError::RecursiveDefault { path: None, reference: reference.clone() });
            }
            self.expand(Reached { pointer: target, arm: at.arm }, chain, seen, out)?;
        }
        for (union, alternatives, members) in &links.unions {
            for (index, &member) in members.iter().enumerate() {
                let arm = if *alternatives && self.track_arms { self.choose(at.arm, *union, index) } else { at.arm };
                self.expand(Reached { pointer: member, arm }, chain, seen, out)?;
            }
        }
        chain.pop();
        Ok(())
    }

    fn expand_all(&mut self, reached: &[Reached]) -> Result<Vec<Reached>, SchemaError> {
        let mut out = Vec::new();
        let mut seen = IdSet::default();
        for node in reached {
            self.expand(*node, &mut Vec::new(), &mut seen, &mut out)?;
        }
        Ok(out)
    }

    /// [`Self::expand`] of `node` on its own.
    fn expansion(&mut self, node: Reached) -> Result<Arc<[Reached]>, SchemaError> {
        if let Some(expansion) = self.expansions.get(&node) {
            return Ok(Arc::clone(expansion));
        }
        let mut out = Vec::new();
        self.expand(node, &mut Vec::new(), &mut IdSet::default(), &mut out)?;
        let expansion: Arc<[Reached]> = out.into();
        self.expansions.insert(node, Arc::clone(&expansion));
        Ok(expansion)
    }

    /// The location described by the nodes `reached`; `None` when none of
    /// them describes an object or array.
    fn scope(&mut self, reached: &[Reached]) -> Result<Option<usize>, SchemaError> {
        if let Some(&known) = self.entered.get(reached) {
            return Ok(known);
        }
        let nodes: Vec<Reached> =
            self.expand_all(reached)?.into_iter().filter(|node| self.is_structural(node.pointer)).collect();
        if nodes.is_empty() {
            self.entered.insert(reached.to_vec(), None);
            return Ok(None);
        }
        let mut key = nodes.clone();
        key.sort();
        key.dedup();
        if let Some(&known) = self.known.get(&key) {
            self.entered.insert(reached.to_vec(), Some(known));
            return Ok(Some(known));
        }
        let id = self.scopes.len();
        self.scopes.push(Scope::default());
        self.known.insert(key, id);
        self.entered.insert(reached.to_vec(), Some(id));

        let mut declared: IndexMap<&'s str, Vec<Reached>> = IndexMap::new();
        let mut required: Vec<String> = Vec::new();
        // `(node, pattern, source, schema)` for every `patternProperties` entry.
        let mut patterns: Vec<(usize, Arc<TextPattern>, &'s str, Reached)> = Vec::new();
        // `(node, its own property names, schema)` for every `additionalProperties` schema.
        let mut catch_alls: Vec<(usize, Vec<String>, Reached)> = Vec::new();
        // Each node's leading item schemas and the schema of the items after them.
        let mut arrays: Vec<(Vec<Reached>, Option<Reached>)> = Vec::new();
        for (node, reached) in nodes.iter().enumerate() {
            let Some(map) = self.node(reached.pointer) else { continue };
            let members = self.members(reached.pointer, map);
            let at = |pointer: u32| Reached { pointer, arm: reached.arm };
            for &(name, pointer) in &members.properties {
                declared.entry(name).or_default().push(at(pointer));
            }
            if let Some(Value::Array(names)) = map.get("required") {
                required.extend(names.iter().filter_map(Value::as_str).map(str::to_string));
            }
            for (pattern, source, pointer) in &members.patterns {
                patterns.push((node, Arc::clone(pattern), source, at(*pointer)));
            }
            if let Some(pointer) = members.additional {
                let names = members.properties.iter().map(|(name, _)| (*name).to_string()).collect();
                catch_alls.push((node, names, at(pointer)));
            }
            let items = members.items.map(at);
            let prefix: Vec<Reached> = members.prefix.iter().copied().map(at).collect();
            if !prefix.is_empty() || items.is_some() {
                arrays.push((prefix, items));
            }
        }

        // A literal name is known now, so every schema selecting it (its
        // matching patterns, and each catch-all of a node that neither names
        // nor matches it) merges into its one definition here.
        for (name, definitions) in &mut declared {
            definitions.extend(patterns.iter().filter(|(_, pattern, ..)| pattern.is_match(name)).map(|(.., schema)| *schema));
            definitions.extend(
                catch_alls
                    .iter()
                    .filter(|(node, names, _)| {
                        !names.iter().any(|own| own == name)
                            && !patterns.iter().any(|(owner, pattern, ..)| owner == node && pattern.is_match(name))
                    })
                    .map(|(.., schema)| *schema),
            );
        }
        let location = self.text(nodes[0].pointer).to_string();
        let mut properties = Vec::with_capacity(declared.len());
        for (name, definitions) in declared {
            let entry = self.entry(&location, name, definitions)?;
            properties.push(Property { name: name.to_string(), entry });
        }
        let mut key_patterns = Vec::with_capacity(patterns.len());
        for (node, pattern, source, schema) in patterns {
            let entry = self.entry(&location, source, vec![schema])?;
            key_patterns.push(KeyPattern { node, pattern, entry });
        }
        let mut catches = Vec::with_capacity(catch_alls.len());
        for (node, names, schema) in catch_alls {
            let entry = self.entry(&location, "additionalProperties", vec![schema])?;
            catches.push(CatchAll { node, names, entry });
        }
        let longest = arrays.iter().map(|(prefix, _)| prefix.len()).max().unwrap_or(0);
        let mut prefix = Vec::with_capacity(longest);
        for index in 0..longest {
            let at: Vec<Reached> =
                arrays.iter().filter_map(|(prefix, items)| prefix.get(index).or(items.as_ref()).copied()).collect();
            prefix.push(if at.is_empty() { None } else { self.scope(&at)? });
        }
        let rest: Vec<Reached> = arrays.into_iter().filter_map(|(_, items)| items).collect();
        let rest = if rest.is_empty() { None } else { self.scope(&rest)? };
        self.scopes[id] =
            Scope { pointer: location, properties, required, patterns: key_patterns, catch_alls: catches, prefix, rest };
        Ok(Some(id))
    }

    /// The child location and the one default of the schemas at
    /// `definitions`, which all select the key `name` at `location`.
    fn entry(&mut self, location: &str, name: &str, definitions: Vec<Reached>) -> Result<Entry, SchemaError> {
        let child = self.scope(&definitions)?;
        let default = self.declared(location, name, &definitions)?;
        let definitions = definitions
            .into_iter()
            .map(|Reached { pointer, arm }| Definition {
                pointer: self.text(pointer).to_string(),
                arm: self.spell(arm),
                validator: OnceLock::new(),
            })
            .collect();
        Ok(Entry { child, default, definitions })
    }

    fn is_structural(&self, pointer: u32) -> bool {
        self.pointers[pointer as usize].structural
    }

    /// The one default the property `name` declares across `definitions`,
    /// its declarations at the location whose first node is `location`.
    fn declared(&mut self, location: &str, name: &str, definitions: &[Reached]) -> Result<Option<Declared>, SchemaError> {
        let mut found: Vec<(&'s Value, String)> = Vec::new();
        let mut arms: Vec<u32> = Vec::new();
        for definition in definitions {
            let expansion = self.expansion(*definition)?;
            let mut held = false;
            for node in expansion.iter() {
                if let Some(value) = self.node(node.pointer).and_then(|map| map.get("default")) {
                    held = true;
                    found.push((value, format!("{}/default", self.text(node.pointer))));
                }
            }
            if held && !arms.contains(&definition.arm) {
                arms.push(definition.arm);
            }
        }
        let Some((first, pointer)) = found.first().cloned() else {
            return Ok(None);
        };
        if found.iter().any(|(value, _)| *value != first) {
            return Err(SchemaError::ConflictingDefaults {
                pointer: location.to_string(),
                property: name.to_string(),
                declarations: found.into_iter().map(|(_, pointer)| pointer).collect(),
            });
        }
        let arms = arms.into_iter().map(|arm| self.spell(arm)).collect();
        Ok(Some(Declared { value: first.clone(), pointer, arms }))
    }
}

/// Checks that `schema` declares at most one default per property at each
/// object location: the compile-time half of [`DefaultPlan::compile`], for
/// lowering and export.
///
/// ## Errors
///
/// As [`DefaultPlan::compile`].
pub(crate) fn check_declarations(schema: &Value) -> Result<(), SchemaError> {
    let digest = written_digest(schema);
    if CHECKED.lock().expect("checked declarations poisoned").contains(&digest) {
        return Ok(());
    }
    // Arms only decide which definitions a used default is checked against,
    // which is preparation's step. Without them each location is compiled
    // once instead of once per arm reaching it (about 13 times fewer
    // locations for the lifecycle baseline), and every set of nodes compiled
    // with arms is still compiled, so every conflicting declaration is still
    // met. Expansion fails only on a reference loop, but whether it meets
    // one depends on how often it meets a node; a loop or any refusal is
    // therefore decided, and reported, by the compilation with arms.
    let mut compiler = Compiler::new(schema, false);
    if compiler.root_scope().is_err() || compiler.loops() {
        Compiler::run(schema)?;
    }
    let mut checked = CHECKED.lock().expect("checked declarations poisoned");
    if checked.len() >= CHECKED_CAPACITY {
        checked.clear();
    }
    checked.insert(digest);
    Ok(())
}

/// XXH64 of `schema`'s JSON text in its own key order, streamed into the
/// hasher: [`CHECKED`]'s key. A memo key needs no key-order-insensitive
/// identity, so it neither sorts keys nor builds the text as a canonical
/// rendering does; an equal schema written in another key order only misses
/// and is checked again. Every lowering pays this hash, a large registered
/// trigger payload included, even when the check itself runs.
fn written_digest(schema: &Value) -> u64 {
    use std::hash::Hasher as _;
    struct Feed(biscuit_hash::XxHasher);
    impl std::io::Write for Feed {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.write(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut feed = Feed(biscuit_hash::XxHasher::new());
    serde_json::to_writer(&mut feed, schema).expect("serializing a JSON value into a hasher cannot fail");
    feed.0.finish()
}

// ── Application ───────────────────────────────────────────────────────────

/// One step from a value to a child.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Key(String),
    Index(usize),
}

/// A default to place at `steps`.
#[derive(Debug, Clone)]
struct Insertion {
    steps: Vec<Step>,
    value: Value,
    /// The `default` keyword's pointer.
    pointer: String,
}

struct Fill<'a> {
    plan: &'a DefaultPlan,
    templates: &'a NestedTemplates,
    /// The `default` keywords being expanded: meeting one again inside its
    /// own expansion would never end.
    active: Vec<&'a str>,
    out: Vec<Insertion>,
}

/// Every schema that selects one key of an object, across all the
/// locations describing that object: the key's one property definition.
struct Selection<'a> {
    required: bool,
    entries: Vec<&'a Entry>,
}

impl Selection<'_> {
    /// The locations describing the key's value.
    fn children(&self) -> Vec<usize> {
        let mut children: Vec<usize> = Vec::with_capacity(self.entries.len());
        for child in self.entries.iter().filter_map(|entry| entry.child) {
            if !children.contains(&child) {
                children.push(child);
            }
        }
        children
    }
}

impl<'a> Fill<'a> {
    /// Completes `value`, described by every location in `locations` at
    /// once.
    fn visit(&mut self, locations: &[usize], value: &Value, steps: &mut Vec<Step>) -> Result<(), SchemaError> {
        if !self.templates.is_empty() && self.templates.holds(&instance_pointer(steps)) {
            return Ok(());
        }
        let plan: &'a DefaultPlan = self.plan;
        let scopes = || locations.iter().map(|location| &plan.scopes[*location]);
        match value {
            Value::Object(object) => {
                let mut named: Vec<&'a str> = Vec::new();
                // A name listed only in `required` is named too: the schemas
                // selecting it (a pattern, a catch-all) may default it.
                let names = scopes().flat_map(|scope| {
                    scope.properties.iter().map(|property| property.name.as_str()).chain(scope.required.iter().map(String::as_str))
                });
                for name in names {
                    if !named.contains(&name) {
                        named.push(name);
                    }
                }
                for name in &named {
                    self.key(locations, name, object.get(*name), steps)?;
                }
                // A dictionary selector describes only a key that is there.
                for (key, child) in object {
                    if !named.contains(&key.as_str()) {
                        self.key(locations, key, Some(child), steps)?;
                    }
                }
                Ok(())
            }
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    let mut at: Vec<usize> = Vec::with_capacity(locations.len());
                    let found = scopes().filter_map(|scope| match scope.prefix.get(index) {
                        Some(at) => *at,
                        None => scope.rest,
                    });
                    for location in found {
                        if !at.contains(&location) {
                            at.push(location);
                        }
                    }
                    if at.is_empty() {
                        continue;
                    }
                    steps.push(Step::Index(index));
                    let result = self.visit(&at, item, steps);
                    steps.pop();
                    result?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Every schema selecting `key` in an object described by `locations`:
    /// a location's literal property when it names the key (that already
    /// merges the location's matching patterns and catch-alls), otherwise
    /// its matching patterns and the catch-alls that select the key.
    fn select(&self, locations: &[usize], key: &str) -> Selection<'a> {
        let plan: &'a DefaultPlan = self.plan;
        let mut selection = Selection { required: false, entries: Vec::new() };
        for scope in locations.iter().map(|location| &plan.scopes[*location]) {
            selection.required |= scope.required.iter().any(|name| name == key);
            match scope.properties.iter().find(|property| property.name == key) {
                Some(property) => selection.entries.push(&property.entry),
                None => {
                    selection.entries.extend(scope.patterns.iter().filter(|entry| entry.pattern.is_match(key)).map(|entry| &entry.entry));
                    selection.entries.extend(
                        scope.catch_alls.iter().filter(|catch_all| scope.catches(catch_all, key)).map(|catch_all| &catch_all.entry),
                    );
                }
            }
        }
        selection
    }

    /// Prepares the value of `key`, which the object holds as `written`.
    fn key(&mut self, locations: &[usize], key: &str, written: Option<&Value>, steps: &mut Vec<Step>) -> Result<(), SchemaError> {
        let selection = self.select(locations, key);
        if selection.entries.is_empty() {
            return Ok(());
        }
        steps.push(Step::Key(key.to_string()));
        let result = match written {
            Some(value) if !value.is_null() => {
                let children = selection.children();
                if children.is_empty() { Ok(()) } else { self.visit(&children, value, steps) }
            }
            written => self.fill(locations, key, &selection, written.is_some(), steps),
        };
        steps.pop();
        result
    }

    /// Fills `key`, whose value at `steps` is missing or (when `written`)
    /// `null`, from its selection's one default, then completes the default's
    /// own children.
    fn fill(&mut self, locations: &[usize], key: &str, selection: &Selection<'a>, written: bool, steps: &mut Vec<Step>) -> Result<(), SchemaError> {
        let declared: Vec<&'a Declared> = selection.entries.iter().filter_map(|entry| entry.default.as_ref()).collect();
        let Some(&first) = declared.first() else {
            return Ok(());
        };
        // Schemas chosen by the key alone (patterns, catch-alls) meet here
        // first, so their disagreement is refused per key, never by order.
        if declared.iter().any(|other| other.value != first.value) {
            return Err(SchemaError::ConflictingDefaults {
                pointer: self.plan.scopes[locations[0]].pointer.clone(),
                property: key.to_string(),
                declarations: declared.iter().map(|other| other.pointer.clone()).collect(),
            });
        }
        if first.value.is_null() && (written || !selection.required) {
            return Ok(());
        }
        let Some(path) = property_path(steps) else {
            // A path no property path can spell (an empty key among several
            // segments) cannot be reported, so the property stays as it is.
            return Ok(());
        };
        if let Some(active) = declared.iter().find(|other| self.active.contains(&other.pointer.as_str())) {
            return Err(SchemaError::RecursiveDefault { path: Some(path), reference: active.pointer.clone() });
        }
        self.check(selection, &declared, key, path)?;
        self.out.push(Insertion { steps: steps.clone(), value: first.value.clone(), pointer: first.pointer.clone() });
        let children = selection.children();
        if !children.is_empty() {
            let depth = self.active.len();
            self.active.extend(declared.iter().map(|other| other.pointer.as_str()));
            let result = self.visit(&children, &first.value, steps);
            self.active.truncate(depth);
            result?;
        }
        Ok(())
    }

    /// Checks the one default of `declared` (equal values), about to stand
    /// as `key` at `path` with its children not yet completed, within the
    /// arm of each definition declaring it (see [`Fill::accepts`]).
    fn check(&self, selection: &Selection<'a>, declared: &[&'a Declared], key: &str, path: PropertyPath) -> Result<(), SchemaError> {
        let first = declared[0];
        let definitions: Vec<&'a Definition> = selection.entries.iter().flat_map(|entry| entry.definitions.iter()).collect();
        let mut verdicts: HashMap<&'a str, Vec<String>> = HashMap::new();
        let mut failed: Vec<&'a str> = Vec::new();
        for arm in declared.iter().flat_map(|default| default.arms.iter()) {
            let context: Vec<(&str, usize)> = arm.iter().map(|(union, index)| (union.as_str(), *index)).collect();
            let applicable: Vec<&'a Definition> =
                definitions.iter().copied().filter(|definition| !conflicts(&definition.arm, &context)).collect();
            self.accepts(&first.value, key, &context, applicable, &mut verdicts, &mut failed)?;
        }
        let mut reported: Vec<&str> = Vec::new();
        let mut messages: Vec<&str> = Vec::new();
        for pointer in failed {
            if !reported.contains(&pointer) {
                reported.push(pointer);
                messages.extend(verdicts[pointer].iter().map(String::as_str));
            }
        }
        if messages.is_empty() {
            Ok(())
        } else {
            Err(SchemaError::InvalidDefault { path, pointer: first.pointer.clone(), message: messages.join("; ") })
        }
    }

    /// Whether `value` satisfies `definitions` (none on a member of a union
    /// other than the one `context` chooses) given the union choices in
    /// `context`: every definition whose arm lies within `context`, and, for
    /// each union `context` makes no choice in, at least one member, judged
    /// the same way with that member chosen. Independent unions are each
    /// required; a member no definition reaches accepts only when it admits
    /// an object holding `key` (see [`admits_key`]): the object exists, so a
    /// member that cannot describe it, or denies the key, is no alternative.
    /// Pushes the pointer
    /// of every failing definition of a failing group to `failed`; each
    /// definition is validated once, into `verdicts`.
    fn accepts(
        &self,
        value: &Value,
        key: &str,
        context: &[(&str, usize)],
        definitions: Vec<&'a Definition>,
        verdicts: &mut HashMap<&'a str, Vec<String>>,
        failed: &mut Vec<&'a str>,
    ) -> Result<bool, SchemaError> {
        let (inside, mut rest): (Vec<&'a Definition>, Vec<&'a Definition>) =
            definitions.into_iter().partition(|definition| definition.arm.iter().all(|(union, _)| chosen(context, union)));
        let mut accepted = true;
        for definition in inside {
            if !self.verdict(value, definition, verdicts)?.is_empty() {
                accepted = false;
                failed.push(&definition.pointer);
            }
        }
        // Each definition left is on a member of a union `context` does not
        // choose; the first such union of the first one is judged as a
        // group, the definitions on no member of it afterwards.
        while let Some(union) =
            rest.first().and_then(|definition| definition.arm.iter().find(|(union, _)| !chosen(context, union))).map(|(union, _)| union.as_str())
        {
            let (group, others): (Vec<&'a Definition>, Vec<&'a Definition>) =
                rest.into_iter().partition(|definition| definition.arm.iter().any(|(other, _)| other == union));
            rest = others;
            let members = self.plan.schema.pointer(union).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
            let mut member_failures: Vec<&'a str> = Vec::new();
            let mut any = members.is_empty();
            for (index, member) in members.iter().enumerate() {
                let on: Vec<&'a Definition> = group
                    .iter()
                    .copied()
                    .filter(|definition| definition.arm.iter().any(|(other, at)| other == union && *at == index))
                    .collect();
                if on.is_empty() {
                    if !admits_key(self.plan.schema.as_ref(), member, key, &mut Vec::new()) {
                        continue;
                    }
                    any = true;
                    break;
                }
                let mut within_member = context.to_vec();
                within_member.push((union, index));
                if self.accepts(value, key, &within_member, on, verdicts, &mut member_failures)? {
                    any = true;
                    break;
                }
            }
            if !any {
                accepted = false;
                failed.extend(member_failures);
            }
        }
        Ok(accepted)
    }

    /// The structural validator's messages for `value` against `definition`,
    /// computed once per check.
    fn verdict<'v>(
        &self,
        value: &Value,
        definition: &'a Definition,
        verdicts: &'v mut HashMap<&'a str, Vec<String>>,
    ) -> Result<&'v [String], SchemaError> {
        if !verdicts.contains_key(definition.pointer.as_str()) {
            let messages = match self.plan.schema.pointer(&definition.pointer) {
                None => Vec::new(),
                Some(schema) => {
                    let validator = match definition.validator.get() {
                        Some(validator) => validator,
                        None => {
                            let built = self.structural(schema)?;
                            definition.validator.get_or_init(|| built)
                        }
                    };
                    validator.iter_errors(value).map(|error| error.to_string()).collect()
                }
            };
            verdicts.insert(&definition.pointer, messages);
        }
        Ok(&verdicts[definition.pointer.as_str()])
    }

    /// A structural validator for `schema` as a root of its own, the root's
    /// definitions in scope when it refers to them.
    fn structural(&self, schema: &Value) -> Result<Arc<Validator>, SchemaError> {
        let root = self.plan.schema.as_ref();
        let mut standalone = if refers(schema) { scope::with_root_definitions(schema, root) } else { schema.clone() };
        if let Some(map) = standalone.as_object_mut() {
            map.entry("$schema").or_insert_with(|| Value::String(simplified::DRAFT_2020_12.to_string()));
        }
        self.plan.cache.structural_validator_for(&standalone)
    }
}

/// `instance` with `insertions` applied (each names a different path) and
/// the defaults that were.
fn finish(instance: Cow<'_, Value>, insertions: Vec<Insertion>) -> DefaultPreparation<'_> {
    if insertions.is_empty() {
        return DefaultPreparation { candidate: instance, defaults: Vec::new() };
    }
    let candidate = apply(instance.into_owned(), &insertions);
    let defaults = insertions
        .into_iter()
        .filter_map(|insertion| {
            Some(AppliedDefault {
                path: property_path(&insertion.steps)?,
                value: insertion.value,
                pointer: insertion.pointer,
                origin: None,
            })
        })
        .collect();
    DefaultPreparation { candidate: Cow::Owned(candidate), defaults }
}

/// `value` with each insertion placed at its steps, replacing a missing or
/// `null` value; any other value is kept.
fn apply(mut value: Value, insertions: &[Insertion]) -> Value {
    for insertion in insertions {
        if let Some((Step::Key(last), parents)) = insertion.steps.split_last()
            && let Some(Value::Object(object)) = descend(&mut value, parents)
            && object.get(last).is_none_or(Value::is_null)
        {
            object.insert(last.clone(), insertion.value.clone());
        }
    }
    value
}

/// The value at `steps` below `value`, if every step exists.
fn descend<'v>(value: &'v mut Value, steps: &[Step]) -> Option<&'v mut Value> {
    steps.iter().try_fold(value, |holder, step| match (step, holder) {
        (Step::Key(key), Value::Object(object)) => object.get_mut(key),
        (Step::Index(index), Value::Array(items)) => items.get_mut(*index),
        _ => None,
    })
}

/// The property path of `steps`; `None` at the root or for a path no
/// property path can spell.
fn property_path(steps: &[Step]) -> Option<PropertyPath> {
    if steps.is_empty() {
        return None;
    }
    PropertyPath::from_segments(steps.iter().map(|step| match step {
        Step::Key(key) => key.clone(),
        Step::Index(index) => index.to_string(),
    }))
    .ok()
}

fn instance_pointer(steps: &[Step]) -> String {
    steps
        .iter()
        .map(|step| match step {
            Step::Key(key) => format!("/{}", escape(key)),
            Step::Index(index) => format!("/{index}"),
        })
        .collect()
}

/// `key` as one JSON Pointer reference token.
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Whether the union member `schema` admits nothing but `null`.
fn admits_only_null(schema: &Value) -> bool {
    schema.get("const").is_some_and(Value::is_null)
        || match schema.get("type") {
            Some(Value::String(kind)) => kind == "null",
            Some(Value::Array(kinds)) => !kinds.is_empty() && kinds.iter().all(|kind| kind == "null"),
            _ => false,
        }
}

/// Whether the union member `schema`, which defines nothing for `key`,
/// admits an object holding `key`, judged from its static form alone: its
/// `type`, `const` and `enum` allow an object, it is not `false`, and an
/// `additionalProperties: false` beside it does not deny a key that no
/// `properties` entry or pattern of its own names. Local `$ref` targets and
/// `allOf` members must each admit it; a nested `anyOf`/`oneOf` admits when
/// one member does. Anything else (an open object, an unresolvable
/// reference) admits: only what the schema states excludes a member.
fn admits_key(root: &Value, schema: &Value, key: &str, seen: &mut Vec<*const Value>) -> bool {
    let map = match schema {
        Value::Bool(admits) => return *admits,
        Value::Object(map) => map,
        _ => return true,
    };
    if seen.contains(&std::ptr::from_ref(schema)) {
        return true;
    }
    seen.push(std::ptr::from_ref(schema));
    let admitted = admits_key_here(root, map, key, seen);
    seen.pop();
    admitted
}

/// [`admits_key`] for the object schema `map`, already on the path `seen`.
fn admits_key_here(root: &Value, map: &serde_json::Map<String, Value>, key: &str, seen: &mut Vec<*const Value>) -> bool {
    let object_type = match map.get("type") {
        Some(Value::String(kind)) => kind == "object",
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| kind == "object"),
        _ => true,
    };
    let object_value = map.get("const").is_none_or(Value::is_object)
        && map.get("enum").and_then(Value::as_array).is_none_or(|values| values.iter().any(Value::is_object));
    let denied = map.get("additionalProperties") == Some(&Value::Bool(false))
        && !map.get("properties").and_then(Value::as_object).is_some_and(|names| names.contains_key(key))
        && !map
            .get("patternProperties")
            .and_then(Value::as_object)
            .is_some_and(|patterns| patterns.keys().any(|source| key_pattern(source).is_none_or(|pattern| pattern.is_match(key))));
    if !object_type || !object_value || denied {
        return false;
    }
    let target = match map.get("$ref") {
        Some(Value::String(reference)) => reference.strip_prefix('#').and_then(|pointer| root.pointer(pointer)),
        _ => None,
    };
    if target.is_some_and(|target| !admits_key(root, target, key, seen)) {
        return false;
    }
    let members = |keyword: &str| map.get(keyword).and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    members("allOf").iter().all(|member| admits_key(root, member, key, seen))
        && ["anyOf", "oneOf"].iter().all(|keyword| {
            let alternatives = members(keyword);
            alternatives.is_empty() || alternatives.iter().any(|member| admits_key(root, member, key, seen))
        })
}

/// Whether `schema` holds a `$ref` anywhere.
fn refers(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => map.contains_key("$ref") || map.values().any(refers),
        Value::Array(items) => items.iter().any(refers),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn prepare(schema: Value, instance: &Value) -> Result<DefaultPreparation<'_>, SchemaError> {
        let plan = DefaultPlan::compile(Arc::new(schema), &ValidatorCache::new())?;
        prepare_defaults(&plan, Cow::Borrowed(instance), &NestedTemplates::default())
    }

    fn pointers(prepared: &DefaultPreparation<'_>) -> Vec<String> {
        prepared.defaults.iter().map(|applied| applied.path.to_string()).collect()
    }

    #[test]
    fn a_reference_loop_without_descent_is_refused_when_compiled() {
        let schema = json!({
            "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } },
            "$ref": "#/$defs/a",
        });

        let error = DefaultPlan::compile(Arc::new(schema), &ValidatorCache::new()).unwrap_err();

        assert!(matches!(error, SchemaError::RecursiveDefault { path: None, .. }), "{error:?}");
    }

    #[test]
    fn a_recursive_reference_compiles_once_and_follows_the_value() {
        let node = json!({
            "type": "object",
            "properties": {
                "v": { "type": "number", "default": 1 },
                "next": { "$ref": "#/$defs/node" },
            },
        });
        let schema = json!({ "$defs": { "node": node }, "$ref": "#/$defs/node" });
        let input = json!({ "next": { "next": {} } });

        let prepared = prepare(schema, &input).unwrap();

        assert_eq!(pointers(&prepared), ["v", "next.v", "next.next.v"]);
    }

    #[test]
    fn a_default_that_recreates_its_own_parent_is_refused() {
        let node = json!({
            "type": "object",
            "properties": { "next": { "$ref": "#/$defs/node", "default": {} } },
        });
        let schema = json!({ "$defs": { "node": node }, "$ref": "#/$defs/node" });

        let error = prepare(schema, &json!({})).unwrap_err();

        match error {
            SchemaError::RecursiveDefault { path, reference } => {
                assert_eq!(path.map(|path| path.to_string()).as_deref(), Some("next.next"));
                assert_eq!(reference, "/$defs/node/properties/next/default");
            }
            other => panic!("expected RecursiveDefault, got {other:?}"),
        }
    }

    #[test]
    fn an_insertion_below_a_missing_parent_is_dropped() {
        let insertion = Insertion {
            steps: vec![Step::Key("gone".into()), Step::Key("a".into())],
            value: json!(1),
            pointer: "/properties/gone/properties/a/default".into(),
        };

        let value = apply(json!({ "kept": true }), &[insertion]);

        assert_eq!(value, json!({ "kept": true }));
    }

    #[test]
    fn tuple_positions_are_never_inserted_but_their_objects_are_completed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "pair": {
                    "type": "array",
                    "prefixItems": [
                        { "type": "number", "default": 0 },
                        { "type": "object", "properties": { "on": { "type": "boolean", "default": true } } },
                    ],
                },
            },
        });

        let short = json!({ "pair": [1] });
        assert!(prepare(schema.clone(), &short).unwrap().defaults.is_empty());
        let full = json!({ "pair": [null, {}] });
        let prepared = prepare(schema, &full).unwrap();
        assert_eq!(pointers(&prepared), ["pair.1.on"]);
        assert_eq!(prepared.candidate.into_owned(), json!({ "pair": [null, { "on": true }] }));
    }

    #[test]
    fn additional_property_values_are_completed() {
        let schema = json!({
            "type": "object",
            "properties": { "fixed": { "type": "number" } },
            "additionalProperties": { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } },
        });
        let input = json!({ "fixed": 1, "extra": {} });

        let prepared = prepare(schema, &input).unwrap();

        assert_eq!(pointers(&prepared), ["extra.n"]);
    }

    #[test]
    fn a_schema_without_defaults_borrows_every_value() {
        let schema = json!({ "type": "object", "properties": { "a": { "type": "number" } } });
        let input = json!({ "a": null });

        let prepared = prepare(schema, &input).unwrap();

        assert!(matches!(prepared.candidate, Cow::Borrowed(_)));
    }

    #[test]
    fn check_declarations_decides_reference_loops_as_compilation_does() {
        let schemas = [
            json!({ "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } }, "$ref": "#/$defs/a" }),
            // Entered at the loop's member, expansion returns to it as a
            // member, never meeting a reference to a node on its chain.
            json!({
                "$defs": { "p": { "allOf": [{ "$ref": "#/$defs/q" }] }, "q": { "$ref": "#/$defs/p" } },
                "type": "object",
                "properties": { "a": { "$ref": "#/$defs/p/allOf/0" } },
            }),
        ];

        for schema in schemas {
            let mut without_arms = Compiler::new(&schema, false);
            assert!(without_arms.root_scope().is_err() || without_arms.loops(), "{schema}");
            let compiled = DefaultPlan::compile(Arc::new(schema.clone()), &ValidatorCache::new()).map(|_| ());

            assert_eq!(format!("{:?}", check_declarations(&schema)), format!("{compiled:?}"), "{schema}");
        }
    }

    #[test]
    fn check_declarations_reports_conflicts_on_nested_unions() {
        let schema = json!({
            "type": "object",
            "properties": {
                "shape": {
                    "anyOf": [
                        { "type": "object", "properties": { "n": { "type": "number", "default": 1 } } },
                        { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } },
                    ],
                },
            },
        });

        let error = check_declarations(&schema).unwrap_err();

        match error {
            SchemaError::ConflictingDefaults { pointer, property, declarations } => {
                assert_eq!(pointer, "/properties/shape/anyOf/0");
                assert_eq!(property, "n");
                assert_eq!(
                    declarations,
                    ["/properties/shape/anyOf/0/properties/n/default", "/properties/shape/anyOf/1/properties/n/default"]
                );
            }
            other => panic!("expected ConflictingDefaults, got {other:?}"),
        }
    }
}
