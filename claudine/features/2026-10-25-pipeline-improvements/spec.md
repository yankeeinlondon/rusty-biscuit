---
area: claudine
status: review-findings
created: 2026-10-25
---

# Pipeline Improvements

This document collects mini-review findings from the completed pipeline. Each
section records the implementation examined, the affected packages, and
alternative improvements. Proposed improvements are findings for review, not
claims that their implementation has been authorized or completed.

## 2026-10-11-schema-loader-yaml-spellings

### Preserve grammar-failure provenance instead of reconstructing it

**Importance:** medium. **Packages impacted:** `darkmatter`, `dmls`.

The library correctly shares the YAML scalar decoder between strict schema
loading and tolerant frontmatter lookup. However, the source-aware property
parser returns immediately when semantic parsing fails, before building its
source map (`simplified/source.rs::parse_property_definition_with_source`).
DMLS must therefore reconstruct the failing scalar in
`diagnostics/frontmatter.rs::grammar_token_span`: locate the standalone payload
again, collect every string scalar under the definition into a vector, allocate
a property name for each scalar, and reparse candidate strings until one
reproduces the grammar error's span and message. The surrounding diagnostic
path also reparses the document's YAML and builds a frontmatter AST. This is
concrete repeated work on an interactive error path, although its effect on
editor latency has not been measured. Matching the rendered grammar message
also couples source recovery to error wording and traversal order rather than
an explicit identity of the failed node.

One approach is a library helper that returns a structured, located parse
failure while keeping the existing public `SchemaError` wrapper intact. For
example, a failure sidecar could retain the definition's key path and projected
authored token range; DMLS would format the same error and consume the range,
without searching for a matching failure. This gives other schema consumers
the same recovery behavior but requires a deliberate library API addition.

A smaller approach keeps recovery in DMLS and replaces the collecting walk
with a short-circuit visitor, passing borrowed property names and the existing
located payload. The first matching scalar ends traversal, so an error in the
first member of a large definition does not allocate metadata for all its
siblings. This preserves current error contracts and is easier to introduce,
but it retains reparsing and message-based matching. In either approach,
measure malformed large schemas before claiming an end-to-end speedup.

### Share continuation ownership between loaded and partially typed schemas

**Importance:** low. **Packages impacted:** `darkmatter`, `dmls`.

Completion uses the shared semantic source map for a successfully loaded
standalone schema, but falls back to DMLS's own backward line scanner
(`providers/frontmatter.rs::continued_scalar_holder`) when a definition is
incomplete. That scanner independently encodes indentation, sequence-item,
comment, and scalar-opening rules. In particular, it rejects a holder value
beginning with an anchor (`&name`), while the library's
`decode_scalar_node` explicitly supports anchors before scalars and the strict
schema locator preserves supported mapping-value anchors. Thus the two paths
have different presentation coverage, and maintaining continuation support
requires reasoning about both implementations. An anchored continued type
expression is a useful regression example when extending partial completion.

One approach is a narrowly scoped library query that identifies the scalar
holding an authored cursor, using the existing locator and scalar decoder with
an explicit partial-input policy. DMLS would retain responsibility for deciding
which completion category belongs there, while the library would own YAML
presentation rules. This avoids adding a separate locator.

Alternatively, retain the DMLS backward search but use the shared decoder to
validate a candidate holder and its consumed source range, rather than keeping
a separate list of forbidden prefixes. Add paired tests for loaded and
partially typed anchored continuations, compact mapping items, comments, and
indentless lists. This is a smaller change, though indentation ownership still
has two implementations.

## 2026-10-12-shared-type-tables

The resolver's immutable table sharing is sound in the integrated implementation:
the first file lookup, its cache entry, later lookups, and `@this` retain one
allocation; modifiers are applied to independent expanded definitions. The
opportunities below concern the loaders that now supply those shared tables.

### Remove the redundant YAML parse before standalone classification

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

`resolve.rs::named_types_from_text` first parses the complete document with
`serde_yaml_ng::from_str::<YamlValue>` to produce an explicit YAML syntax error.
It then calls `parse_standalone_schema_document`, whose first operation is
`numeric::load_admitted_yaml`, another complete YAML parse. The standalone
classifier now returns `SchemaError::SchemaDocument` for invalid YAML itself,
including the same `not valid YAML` prefix for syntax errors. The preliminary
pass therefore repeats decoding and allocating the YAML tree for every cold
named-table load, including disk, shipped, and host-embedded documents. Warm
parse-memo hits avoid this cost. No end-to-end speedup has been measured.

The smallest solution is to remove the preliminary parse and let the existing
classifier remain the only authority. Keep regression assertions for invalid
YAML, numeric-admission errors, ordinary YAML, and both schema envelopes so
diagnostic behavior stays intentional. For example, importing an unterminated
`types: [` document should still report a typed schema-document syntax error.

If another caller genuinely needs the YAML value before classification, a
second approach is an internal classifier entry point accepting the admitted
YAML value together with the original source and path. The source remains
necessary for authored ranges. Callers could decode once and classify that
same value, rather than introducing another parser or duplicating envelope
rules. This API change is larger and should be justified by actual callers.

### Retain hot named tables when the revision memo reaches capacity

**Importance:** low. **Packages impacted:** `darkmatter`; long-lived consumers
include `dmls`, `claudine`, and `claudine-cli`.

`resolve.rs::FILE_NAMED_TYPES` keys its process-wide memo by `(path,
content_hash)`. Every successful edited revision occupies another entry,
including revisions no longer current. At 256 entries, `load_named_types`
clears the entire memo before inserting the next revision. This is bounded and
correct, but editing one frequently used schema can discard unrelated hot
tables and make their next resolutions repeat parsing. A workspace using more
than 256 distinct imported files can also repeatedly empty the cache. These
are concrete eviction characteristics, not evidence of measured editor latency.

One solution is a bounded least-recently-used memo that evicts one cold
revision when inserting a new one. Keep the existing read-and-hash before
lookup, so edits and deletions remain immediately visible. For example, adding
revision 257 would retain a frequently accessed lifecycle table while evicting
the least recently used old revision. The nearby schema `ValidatorCache`
already provides a policy to compare with before adding another cache design.

Another solution keys the memo by path and stores only its latest
`(content_hash, Arc<NamedTypes>)`, with a bounded policy for distinct paths.
An edit replaces that path's memo entry; already running expansions retain
their old immutable allocation through `Arc`. This prevents revision buildup
and preserves in-flight independence, at the cost of reparsing if callers
alternate between old and new content. Either option should be checked with a
small edit-and-reuse workload before choosing a more elaborate cache.

## 2026-10-13-named-type-presence-and-local-names

The integrated resolver preserves inherited presence, resolves bare local names
in their defining namespaces, and keeps repeated file-match groups separate.
The review followed constraint merging and lowering, typed conflict reporting,
editor related locations, and Claudine's grouped candidate walk.

### Show the values that make inherited constraints contradictory

**Importance:** medium. **Packages impacted:** `darkmatter`, `darkmatter-cli`,
`dmls`; Claudine surfaces the same library errors through `claudine` and
`claudine-cli`.

`resolve.rs::merge_constraints` knows both conflicting constraints, but
`MergeSite::conflict` retains only their origins. `SchemaConstraintOrigin`
stores the keyword, source, structural path, occurrence, and optional span;
it does not store the constraint's value. Consequently the plain error,
terminal status block, and DMLS related-location message can identify `min`
and `max` without saying that the inherited minimum is 10 and the use-site
maximum is 5. Repeated defaults and disjoint scheme lists have the same
limitation. A developer using a several-hop named type must open the related
declarations to understand a conflict the resolver has already diagnosed.

One solution is a typed conflict participant containing the surviving
`Constraint` alongside its `SchemaConstraintOrigin`. Every renderer can then
describe `min(10)` versus `max(5)` and link to the same authored declarations.
This preserves meaning when the source file is unavailable and avoids
recovering semantic values by parsing diagnostic text. Keep formatting of a
constraint shared with the existing canonical serializer where possible.

A smaller solution stores a canonical display string for each participant
when the conflict is constructed, while retaining the existing typed origin
fields. For example, DMLS could show “`max(5)` is declared here” and the CLI
could show both operands in one message. This is easier to introduce but
offers downstream consumers less structured information than retaining the
constraint itself. Both approaches should cover multi-hop bounds, unequal
defaults, and disjoint schemes rather than only the direct two-file example.

### Share default-value normalization with schema lowering

**Importance:** low. **Packages impacted:** `darkmatter`; consumers of its
public schema model include `darkmatter-cli`, `dmls`, and `claudine`.

**Status:** addressed during this audit. The resolver now calls the converter's
shared `normalize_json_number` helper. Regression tests first demonstrated
incorrect equal-default rejection and unequal-default acceptance at 2^63,
then passed with the shared implementation. The account and alternatives
below retain the review's original reasoning.

`resolve.rs::defaults_equal` recursively clones both values through its own
`normalized_default`, which describes itself as mirroring the converter.
`simplified/convert.rs::normalize_json_number` now delegates numeric policy to
`numeric::json_number`, but the resolver still carries an older signed-integer
range and cast. The two implementations therefore disagree for some
programmatically constructed float defaults at and above 2^63; an inclusive
rounded `i64::MAX` bound can also saturate a 2^63 float to the wrong signed
integer. Authored numeric default arguments are already canonicalized by
`grammar.rs::arg_to_json`, so this is a narrow public-model consistency issue,
not evidence that ordinary parsed defaults currently fail. The duplicate
implementations still make a future numeric-policy change easy to apply to
only one consumer.

One solution is one internal recursive JSON-number normalization helper used
by lowering and equality. It can take ownership when lowering already owns a
value and accept a cloned value at the merge boundary. Regression cases
should compare normalized integer and float values around the signed and
unsigned boundaries, as well as nested arrays and objects accepted by the
public model.

Another solution compares borrowed values structurally, using the shared
`numeric::json_number` only for number leaves. This avoids cloning two complete
default trees for every equality check. Keep the converter's owned rewrite,
but give both paths the same leaf policy and parity tests; for example,
`3` and `3.0` compare equal while the string `"3"` remains distinct.

### Avoid repeating a directory walk for each file alternative

**Importance:** low. **Packages impacted:** `claudine-cli`; the alternative
model is supplied by `darkmatter` and `claudine`.

`completion/schema_completion/candidates.rs::match_alternative_files` calls
`match_glob_files` separately for every file alternative. Each call prepares
its groups, walks its first group's roots, examines entries, and sorts its
matches. The outer `seen` set removes duplicate output paths only after that
work. Thus alternatives such as `file(match('*.md'))` and
`file(match('guide*'))` can traverse the same directory tree twice. A schema
with more alternatives repeats the traversal, even though the resolution
context and completion visibility filters are identical. No interactive
latency measurement was made, so the ranking reflects a bounded optimization
opportunity rather than a measured regression.

One solution prepares all alternatives first and walks each distinct root
once. Judge an entry against the alternatives that can still admit that
subtree, retain separate result buckets, and fold those buckets in authored
alternative order. This preserves both conjunction across an alternative's groups and
the current first-alternative ordering while reducing repeated filesystem
work.

A smaller solution adds an invocation-local directory-entry inventory shared
by the alternative walks. Cache only the entry facts and common visibility
decisions, then apply each alternative's own prepared predicates and ordering.
This retains the existing walk organization, although subtree pruning needs
care so one alternative cannot hide entries another would admit. Compare
one-, two-, and many-alternative completion over the same temporary tree
before choosing between these approaches; preserve hidden-file, ignore,
symlink, and root-precedence behavior.

## 2026-10-14-union-type-constraints

The review followed named-reference postfix resolution, separate array/item
lowering, union acceptance and diagnostics, conservative cleanup, eager-file
rewriting, and typed editor queries. The implementation retains one
array-of-union representation and keeps explanation heuristics separate from
authorization to mutate data. Later recursive-reference support is accounted
for in the opportunities below.

### Prepare eager-file union arms once per rewrite invocation

**Importance:** medium. **Packages impacted:** `darkmatter`; composition
consumers include `darkmatter-cli`, `claudine`, and `claudine-cli`.

`rewrite.rs::rewrite_array_items` calls `rewrite_property` for each item. When the
item schema is a union, `rewrite_property_union` searches every arm for an
eager-file marker, copies the root definitions into each eligible arm's
standalone schema, and calls `validate::build_validator_in_context` before
testing that item's value. That builder compiles a validator directly; it does
not use `ValidatorCache`. A list of 100 items and two eligible object arms can
therefore compile the same two validators 100 times and repeatedly traverse
the same referenced definitions to find their eager markers. The instance
checks must remain per item, but the fragment preparation depends only on the
schema and captured resolution context. No end-to-end timing was measured, so
this finding identifies redundant work rather than a quantified regression.

One solution is a rewrite-invocation memo keyed by the borrowed arm's identity.
Store whether the fragment contains an eager marker and, when eligible, its
compiled validator. Every item then applies the prepared validator to its own
value. Keep the memo inside the one rewrite invocation, where the root schema,
base directory, and request context are fixed; never reuse a file-validation
verdict between values or requests. For example, both arms of a mixed list
would compile once, while each of the 100 values would still be tested against
both to preserve the existing ambiguity rule.

Another solution builds a small prepared rewrite plan before walking the
instance. A union node stores its eligible prepared arms, and an array node
reuses its item plan for every element. This can also share eager-marker
discovery across recursive references, but needs an explicit cycle guard and
is larger than a local memo. In either approach, retain the current
context-aware format validators and require exactly one accepting eligible arm
before a rewrite. Measure large mixed lists and nested recursive object arms;
do not substitute the passive structural matcher used for editor assistance.

### Retain prepared alternative matchers for repeated editor queries

**Importance:** low. **Packages impacted:** `darkmatter`, `dmls`.

DMLS's `providers/frontmatter.rs::union_item` calls
`completion::alternative_accepts_in` for every alternative whenever it builds
an item's hover. The library correctly caches the compiled structural
validator, but each call first collects and clones the reachable definition
table, clones the atom and adds `required`, builds a one-property schema,
lowers it to JSON Schema, and hashes that result to find the cache entry. A
second hover on an unchanged item repeats this preparation for every arm even
when all validator lookups hit. Recursive definitions increase the amount of
schema data copied; the cached validator removes compilation but not this
schema construction. Interactive latency has not been measured.

One solution is a library preparation API returning a passive alternative
matcher, built once from an atom and its resolved definition table. It can
retain the structural validator and expose `accepts(&Value)`. DMLS stores
these matchers alongside its document-version schema bundle and discards them
with that bundle, so dependency edits invalidate both the typed alternatives
and their matchers. For example, repeated hover on a policy list would test
only the current item values after its schema was prepared.

A smaller solution leaves the public single-value helper intact and adds an
internal prepared-schema memo at the resolved-document boundary. Its key must
identify the atom together with its definition revision, rather than the
atom's address alone; changing an imported recursive body can change
acceptance without changing the referring atom. Reuse the existing validator
cache after this memo supplies the lowered schema and digest. Compare cold
and repeated hover over unions with shallow and recursive arms before
choosing an additional cache. Both approaches must remain passive and must
never provide authorization for coercion or filesystem rewriting.

## 2026-10-15-diagnostic-node-positions

The review followed source projection, semantic-structure checks, original-byte
line indexing, category-aware problem lookup, fallback selection, complete-file
offsets, CLI rendering, and DMLS's independently ranged diagnostics. The
implementation builds one located tree per validation call and shares it across
all problems and root-union arms. It preserves the distinctions between a key,
a value, and a mapping with a missing field; no correctness defect was found.

### Defer source-position preparation until validation needs it

**Importance:** low. **Packages impacted:** `darkmatter`; callers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

Both `DarkmatterSchemas::validate` and `validate_for_phase` call
`positions_for(source)` before `effective_for(source)`. Position preparation
extracts the frontmatter block, constructs its located tree, checks its mapping
structure, and builds a line-start index. Programmatically constructed Markdown
also serializes its frontmatter to obtain a source view. All this work is
discarded when no schema applies, and also when schema resolution fails before
instance validation. A valid instance needs no problem coordinates either,
although avoiding preparation in that case needs a larger change. This is
demonstrable redundant work, not a measured end-to-end performance regression.

The smallest solution resolves the effective schema first and builds positions
only in its `Some(effective)` branch. For example, validating a schema-free
document with no baseline would return its existing empty successful report
without projecting every frontmatter node. Keep phase-aware and unphased entry
points aligned, and preserve schema-resolution errors and their timing records.

A second solution gives the internal problem collector a lazily initialized
position provider. The first actual validation error constructs the map, and
all later errors and union arms borrow that same map. Public APIs accepting an
already-built `PositionMap` can remain unchanged. A valid large document then
pays no projection cost, while an invalid document still projects once. This
requires careful integration with root-union selection and is justified only
if measurements show source preparation matters; it must not construct a new
map separately for every failed arm.

### Avoid repeated sibling scans when locating many problems

**Importance:** low. **Packages impacted:** `darkmatter`; validation consumers
include `darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

`positions.rs::locate_in` walks each decoded pointer from the root. At every
mapping it searches the ordered `entries` vector with `iter().find`; unknown-key
lookup can search the final mapping again for its key span. A wide mapping with
many independently invalid properties therefore repeats the same sibling
searches for every problem. For example, locating one error for each of 1,000
top-level properties can perform roughly half a million key comparisons even
though source projection already visited those entries once. Shared ancestors
are searched again across union arms too. The current implementation is simple
and likely appropriate for ordinary small frontmatter; no latency measurement
was made, so this is a bounded optimization opportunity.

One solution builds a private key-to-child-index table alongside each located
mapping, retaining the ordered vector as the authority for authored order and
spans. Pointer traversal then finds each child directly, while sequence indices,
scalar descendants, and nearest-authored-ancestor fallback keep their existing
rules. A size threshold can avoid allocating a hash table for small mappings.
The index should be private to `PositionMap`, rather than changing the shared
locator's representation for consumers that do not need repeated lookup.

Another solution indexes complete decoded pointers to their key and value
starts during the trusted projection walk. Exact errors use direct lookups;
an unauthored path searches its pointer prefixes from deepest to shallowest,
excluding the root as the current fallback contract requires. Scalar ancestors
must still anchor descendants, and a missing top-level property must still
have no position. This trades additional pointer storage for simpler repeated
lookups. Compare both approaches on narrow nested documents and wide invalid
mappings before choosing; preserve empty, escaped, numeric, and Unicode keys
and all existing fallback and offset tests.

## 2026-10-16-core-schema-types

The review followed the four types through parsing, option canonicalization,
lowering, default and suggestion checks, coercion, effective presence, subtype
comparison, descriptors, and editor presentation. The shared option model and
passive shell format are sound. The first two findings retain public-API gaps
identified by the original review and still visible in the integrated source;
the third concerns repeated work during schema preparation.

### Preserve unresolved option assertions through serialization

**Importance:** medium. **Packages impacted:** `darkmatter`; callers of its
public schema serializer include `darkmatter-cli` and `dmls`. Ordinary resolved
schema output is unaffected.

**Status:** addressed during this audit. Serialization now retains explicit
option assertions on unresolved references. A regression covers 15 forms and
requires both the original and serialized declarations to retain their option
conflict. It failed before the repair and passed afterward. The account and
alternatives below preserve the finding's original reasoning.

`serialize.rs::render_constraints` canonicalizes every `Constraint::Options`
list and removes it when the canonical set is empty, including on an unresolved
named reference. On a primitive this preserves meaning, but a named reference's
explicit options are an assertion that its target has exactly that canonical
option set. For example, if `slug` denotes `alphanumeric(allow-dashes)`,
`slug(block-leading-dash)` asserts the empty canonical set and must conflict
with the target. Serializing it as `slug` removes the assertion and allows the
declaration to load. The same loss occurs inside arrays and inline objects.
The serializer's public parseable-round-trip contract therefore changes a
rejected declaration into an accepted one, even though ordinary consumers
mostly serialize primitive or already resolved atoms.

One solution distinguishes primitive options from unresolved assertions in
the serializer. Keep primitive output canonical, but preserve an unresolved
reference's authored option keywords in a deterministic order until resolution
has supplied its identity. Thus the example remains
`slug(block-leading-dash)` and still conflicts; matching assertions continue
to resolve as before. Test this through parse, serialize, parse, and resolve,
including an imported name, a local name, and nested array/object forms.

A second solution makes the distinction explicit in the model: an option-set
assertion on a reference retains its authored witness separately from the
canonical set used for identity. The parser records both, resolution checks
the canonical set, and serialization uses the witness when an empty set would
otherwise erase the assertion. This makes the invariant harder for future
consumers to miss but requires a larger model change. Neither solution needs
new schema syntax or should retain redundant options on resolved primitives.

### Compare structured identity with the same semantic rules as primitives

**Importance:** medium. **Packages impacted:** `darkmatter`; consumers of its
public subtype API include shell-template approval in `darkmatter`, reached
through `darkmatter-cli`, `claudine`, and `claudine-cli`. That approval path
currently limits comparisons to string-like scalars, so this finding concerns
the broader public API rather than an observed approval failure.

**Status:** addressed during this audit. Structured identity now applies the
constraint-independent identity rules recursively to objects, unions, tuples,
and functions, while preserving shape, type, container, positional, option,
enum, and literal boundaries and rejecting unresolved children. Two new
regressions failed before the repair and passed afterward. Together with the
serializer regression, the repair passed 178 focused tests; the slowest took
1.643 seconds. The account and alternatives below retain the original review.

`subtype.rs::Identity::Primitive` ignores ordinary constraints and canonicalizes
alphanumeric options and enum members. `Identity::Structured`, however, compares
the raw `TypeExpr` trees. Consequently `{ p: string(min(1)) }` and
`{ p: string }` do not compare as identical underlying types; neither do
equivalent child alphanumeric options written in different orders. The same
issue affects arrays of these objects. This is conservative rather than a
widening, but the public contract says ordinary constraints are ignored and
identical underlying types compare equal. Later union and recursive-reference
support makes a consistent definition of identity more useful.

One solution introduces a private semantic-identity comparison that recursively
compares the same structure while applying the existing primitive identity
rules to its leaves. Object keys and structure remain significant, ordinary
constraints and descriptions do not, option sets are canonical, and references
retain definition identity. A visited-pair guard preserves termination for
recursive definitions. This repairs identity without promising general object
subtyping; objects with genuinely different child types remain incomparable.

Another solution constructs a normalized identity view once when resolved
definitions are prepared. Subtype queries compare that view rather than the
authored tree, while the original tree retains constraints, descriptions, and
source information for validation and diagnostics. This can also avoid
repeated enum-set allocation but adds storage and invalidation obligations.
Whichever approach is chosen, include nested option-order, redundant-option,
constraint-only, enum-order, array, and recursive-reference controls, and keep
the approval consumer's separate runtime checks and quoting requirements.

### Reuse validators for repeated alphanumeric annotation checks

**Importance:** low. **Packages impacted:** `darkmatter`; schema-preparation
consumers include `darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

`convert.rs::check_alphanumeric_values` correctly validates defaults and
suggestions against the complete lowered string fragment, including authored
patterns. It calls `build_structural_validator` directly on every lowering
occurrence that has such values. Repeated imports of the same nonrecursive
named alphanumeric type therefore compile equivalent validators repeatedly,
even though `ValidatorCache::structural_validator_for` already provides
content-based reuse for passive checks. Root-union enforcing passes can also
lower the same fragment again. The implementation does not perform redundant
compilation per candidate within one occurrence; the opportunity is across
equivalent occurrences. No end-to-end speedup has been measured.

One solution carries a small lowering-invocation cache and uses the existing
structural-validator cache for the fragment after annotation removal. Keep
the check at each use site, because inherited `min`, `max`, or `pattern`
constraints can make its valid values differ; only equal lowered targets
share compilation. For example, ten imports of one unchanged suggested slug
would compile once, while a narrowed import would compile its distinct target.

A smaller solution memoizes successful annotation-check results by the full
target fragment and candidate values within one resolution. This avoids both
compilation and rechecking for identical occurrences while preserving
use-site-specific error attribution for failures. Do not cache by the option
set alone: `alphanumeric(min(3); suggest(ab))` must still fail despite a prior
successful check of `alphanumeric(suggest(ab))`. Measure repeated named imports
before adding a new cache, and keep all validation passive and free of file
resolution or command execution.

## 2026-10-17-content-policy-base-schema

The review followed the exported Content Policy artifact through the embedded
document table, shared import engine, cached runtime baseline, CLI selection,
and validation reporting. Embedded imports remain isolated from the disk and
network, the low-level schema constructor remains unconfigured, and the CLI's
selection order preserves explicit overrides. Later pipeline work has replaced
the original baseline file with the global catalog's resolved `fm` definition;
the findings below concern the integrated implementation.

### Retain schema-presence information from the validation preparation

**Importance:** low. **Packages impacted:** `darkmatter`, `darkmatter-cli`.

`commands/schema/validate.rs::validate_one` calls `api.validate(&md)` and then
`api.effective_for(&md)?.is_none()` solely to decide whether pretty output
should say that no effective schema exists. The library's `validate` already
calls `effective_for`, so every successfully processed file prepares its
effective schema twice after assignments. Compiled validators are cached, but
that cache does not remove the second layer selection, resolution, merge,
lowering, and preparation of the `EffectiveSchema` value. A document schema
referencing a file can also be read again. The earlier preparation before
assignments is necessary because assignments can change trigger selection;
this finding concerns only the two preparations of the same final instance.
The default baseline makes this path common even for files without `$schema`.
No end-to-end speedup has been measured.

One solution adds a library operation returning the validation report with a
small preparation summary, such as `has_effective_schema`. It determines that
flag from the same `Option<EffectiveSchema>` used for validation, so the CLI
can retain its existing output and error handling without resolving again.
The ordinary `validate` method can delegate to that operation and discard the
summary. Keep source positions, pending-template handling, advisories, and
validation timing attribution in the shared library implementation.

Another solution lets the CLI prepare the final effective schema once and
passes it to a library helper that validates a `Markdown` document against
that prepared value. The CLI derives the display flag from the option it
already holds. This makes preparation explicit and can help other callers
that also need type information, but the helper should own frontmatter
conversion and source-position preparation rather than duplicating the
library's `validate` implementation in the CLI. In either approach, retain a
separate post-assignment preparation and test baseline-only, trigger-only,
document-only, and genuinely schema-free outcomes.

### Reduce independent maintenance of policy grammar in editor patterns

**Importance:** low. **Packages impacted:** `content-policy`, `darkmatter`,
`darkmatter-cli`, and `dmls` as consumers of the shipped artifact.

`content-policy/schemas/content-policy.yaml` repeats duration alternatives for
`d`, `wk`, `mo`, and `yr` in three `ValidFor` variants, alongside repeated date
and property-reference fragments. `content-policy/lib/src/grammar.rs` defines
the same lexical choices independently in `parse_duration`, `parse_reference`,
and rule dispatch. This repetition is understandable: the schema's escaped
patterns work within SimplifiedSchema's constraint syntax, and the editor
intentionally checks less than the policy parser. However, changing a unit or
reference spelling requires finding every regex occurrence as well as the
parser. The shared complete-declaration tests cover important examples but do
not make those repeated spellings one source of truth. There is no observed
current disagreement beyond the documented limitations.

One solution generates the editor artifact within Content Policy from a small
local set of lexical fragments and rule examples. For example, one duration
unit roster can produce the three expanded `ValidFor` patterns while the
checked-in YAML remains the exported `EDITOR_SCHEMA` artifact. Keep calendar
validity, overflow, path-boundary checks, and cross-entry semantics in the
parser; a generator does not need to reproduce them or introduce a dependency
from Content Policy to Darkmatter. A deterministic regeneration check catches
artifact drift and lets authors review the generated text.

A smaller solution retains the authored YAML and adds a compact, exhaustive
lexical parity matrix derived from one rule-variant and unit roster. Exercise
every unit in each of the default-baseline, property-baseline, and inline-date
forms, plus adjacent invalid spellings and reference forms, through both
`Policy::from_declaration` and the embedded editor schema. This does not remove
the regex repetition, but it makes missed updates fail automatically without
adding a generator or broadening schema grammar. Keep explicit expected
differences for impossible dates, overflow, paths, and whole-list null rather
than accepting arbitrary disagreement.

## 2026-10-18-fm-namespace

The review followed reserved-root classification, shared frontmatter lookups,
literal-index dependency collection, hook dispatch, and the per-document
environment handoff. Dotted `fm` helpers borrow through the effective mapping
and clone only the selected value; bare `fm` intentionally creates a snapshot.
The document view replaces its environment consumers together and preserves
launch-owned anchors. No correctness defect was found in those contracts.

### Resolve dotted hook reads without materializing the whole payload

**Importance:** medium. **Packages impacted:** `claudine`, `claudine-cli`.

`dispatch/expression.rs::EventMetaExpressionLookup::get` handles every
`hook.<path>` by calling `hook_payload_object` and then `nested_pointer`.
Building that object copies every present top-level field, including complete
`tool_input`, `tool_response`, and `extra` values, and reconstructs the
`os`, `hardware`, `git`, and `project` groups. Thus a condition reading only
`hook.tool_name` or `hook.extra.flag` copies unrelated tool payloads before
cloning its selected result. Multiple hook reads in templates and matchers
repeat the work. The unprefixed lookup already resolves those same leaves
without constructing the full event object. This is an evidenced allocation
opportunity, not a measured dispatch-latency regression.

One approach adds a private hook-member resolver that dispatches the first
path segment to the existing top-level, extra, nested-tool, or environment-group
helpers. Bare `hook` keeps using `hook_payload_object`; dotted reads clone only
the selected subtree. For example, `hook.tool_input.command` borrows into
`meta.tool_input`, and `hook.os` builds only the OS group. Keep this resolver
closed: `hook.env.HOME` must remain absent, and missing members must never
fall through to an unprefixed field or the process environment. Paired tests
against the whole-object projection should preserve omitted versus null fields.

A second approach prepares the whole hook object lazily once per event-dispatch
context and shares it across that event's templates, conditions, and matchers.
This avoids repeated reconstruction when a template reads many fields or bare
`hook`, but a single leaf read still pays for the whole payload. It also needs
a deliberate shared context because the current lookup is a lightweight,
copyable borrow. Compare both approaches with a large tool response and several
small leaf reads before adding state; neither should cache across distinct
events or alter the existing live hook `env.*` contract.

### Avoid copying environment maps that document entry immediately replaces

**Importance:** low. **Packages impacted:** `claudine`, `darkmatter`,
`biscuit-file`; their CLI consumers inherit any allocation reduction.

`DocumentEnvironment` correctly shares its immutable map through `Arc`, but
`InvocationContext::enter_document` clones that map into a local `vars` and
clones it again for `RequestSnapshot`. It also clones the launch snapshot and
`FileResolutionContext`, both of which own environment `HashMap`s, before
`with_env` immediately discards their cloned launch maps. The entered document
therefore holds several full copies of one captured string map and pays for
copies of an old map it never reads. `EnvironmentSource::fixed` additionally
rebuilds a map on each capture even though its input is already immutable and
shared. These costs recur per document, rather than on retries or individual
expression reads; no end-to-end speedup has been measured.

A small approach adds environment-replacement derivation methods to
`RequestSnapshot` and `FileResolutionContext` that copy the retained fields
without first cloning the environment being replaced. Keep the existing
owned-map APIs for callers. For example, entering a later sequence task copies
its new map into each required owner but does not copy the launch map merely
to overwrite it. The file-context method must perform the same tree reselection
as `with_env`, including a changed `VAULT`, and preserve all request and launch
anchors.

A broader approach lets these read-only consumers accept a shared immutable
environment handle, with owned-map constructors wrapping it for compatibility.
The environment source can return `DocumentEnvironment` directly, so a fixed
source shares its allocation while a process source still captures a new map
once per document. Preserve the separate raw `OsString` child baseline and
its home-variable protections; sharing the UTF-8 view must not replace that
record or skip per-document capture accounting. Measure many small sequence
tasks with a large supplied environment to decide whether the additional
cross-package API work is justified.

## 2026-10-19-speak-prepare

The review followed preparation, resolved settings, cache identity, ordinary
playback, detached handoff, and default-voice snapshots across all five
file-producing providers. Preparation and playback share provider-specific
resolution, retained settings remain separate from the caller's configuration,
and cache misses preserve detached queue ordering. The unresolved macOS system
rate remains an intentional safe no-op rather than an invented default.

### Keep durable cloud-cache publication off async executor workers

**Importance:** medium. **Packages impacted:** `biscuit-speaks`; async callers
include `biscuit-speaks-cli`, `claudine`, and `claudine-cli`.

`ElevenLabsProvider::select_and_record` synchronously calls `snapshot::write`
inside its async operation. That routine serializes a record and calls
`audio_cache::write_atomic`, which creates directories, writes a file, calls
`sync_all`, and renames it using blocking filesystem APIs. A cache miss then
calls the same writer on the complete synthesized audio buffer in
`generate_to_cache`. Even a preparation whose audio is already cached can
therefore block its executor worker while durably refreshing the required
voice-selection record. Large audio buffers or slow storage extend that
blocking section. Claudine runs pre-warm in the background, but a background
async task still occupies its runtime worker during synchronous I/O. The
blocking calls are evidenced; their effect on application latency has not
been measured.

One solution adds an async wrapper that moves the complete publication
operation into `tokio::task::spawn_blocking`. Move the owned destination and
audio buffer into the closure rather than copying the buffer, and await its
result before reporting successful preparation. Snapshot publication should
use the same boundary. Keep the existing refresh rule, atomic publication,
and error policy: an audio write failure fails preparation, while a failed
voice-record write only loses the later detached default-voice cache hit.
Dropping the awaiting task does not cancel an already running blocking
closure, so that closure must own staging cleanup and may finish publishing
only complete data.

A second solution implements async publication with `tokio::fs` operations
and an owned temporary-file guard. Await the write, `sync_all`, and final
rename explicitly, keeping staging data on the destination filesystem and
cleaning it up on cancellation or error. This exposes cancellation points
throughout the operation and avoids a long monolithic blocking closure, but
requires careful guard ownership across awaits. Either approach preserves the
current durability rather than removing `sync_all` to improve timing. Measure
executor responsiveness during parallel preparation and unrelated work
before claiming an end-to-end speedup.

### Share temporary-file ownership across cache publication paths

**Importance:** low. **Packages impacted:** `biscuit-speaks`; speech consumers
in `biscuit-speaks-cli`, `claudine`, and `claudine-cli` inherit the behavior.

**Status:** addressed during this audit. `write_atomic` now uses an owned
same-directory temporary file, retaining `sync_all` and atomic persistence.
The rename-failure regression reproduced the leak before the fix and passes
with no abandoned staging file afterward. The original evidence and
alternatives below describe the reviewed implementation before that repair.

Subprocess providers use `prepared::staging_file` and `publish`, whose
`TempPath` owns partial output and deletes it on failure or cancellation.
ElevenLabs audio and its default-voice snapshots instead use
`audio_cache::write_atomic`, which constructs a temporary filename manually.
Its `write_all`, `sync_all`, and rename operations return through `?` without
removing that path. For example, a complete write followed by a rename failure
leaves `.biscuit-speaks-tmp-*` behind. The final cache entry remains atomic;
this is cleanup drift between two implementations, not evidence of a partial
entry being read. Repeated failures can accumulate files, and future cleanup
changes must otherwise be maintained in both paths.

One solution gives `write_atomic` a same-directory `NamedTempFile` or
`TempPath`, retaining its current write-and-sync behavior before persistence.
The existing public helper can keep its signature, while ownership guarantees
that every error path removes unfinished output. A regression can make the
destination an existing directory, force the final replacement to fail, and
assert that no temporary file remains; successful replacement and concurrent
complete-file publication remain separate controls.

Another solution introduces one private staging/publication utility used by
both byte-producing and subprocess providers. The byte writer writes and
syncs into the owned staging file; a subprocess receives its path; both hand
ownership to the same final persistence operation. Keep their different
durability requirements explicit rather than adding a sync to every
subprocess output merely for uniformity. This removes more duplication but
needs a deliberate error mapping so provider errors and the public cache
error contract stay useful. Neither approach needs cache eviction, a new key
format, synthesis deduplication, or a cross-process lock.

## 2026-10-20-lifecycle-grammar

The review followed the final Darkmatter-owned parser, typed node model,
authored source map, expression checks, and serial executor, together with
Claudine's preparation adapter. Dictionary expansion fixes action order before
execution; guards select only one body at event time, while preparation sees
both branches. Source paths survive expansion, and event-ending directives
propagate out of nested bodies. Later pipeline changes, including returning
`prep` and cancellation stacks, remain part of the integrated behavior. No
correctness defect was found in these contracts.

### Borrow grammar views instead of cloning excluded subtrees

**Importance:** low. **Packages impacted:** `darkmatter`; parser consumers
include `claudine`, `claudine-cli`, `darkmatter-cli`, and `dmls`.

`lifecycle/parse.rs::parse_loop_concerns` creates an owned
`without_iteration` map to reject removed keys, then creates another owned
`concerns` map to separate iteration controls from lifecycle actions. Both
copy their values recursively. An explicit `gate:` list, including every
nested branch and operand, is therefore copied twice before its borrowed
value reaches `parse_stack`. The compact guarded-item path similarly clones
the whole item only to remove `when` before parsing its sibling verbs.
`LifecycleSourceMap::from_frontmatter_named` makes another owned filtered map
for loose loop actions. These copies are avoidable preparation work; the
effect on command or editor latency has not been measured.

One approach keeps these operations on borrowed mappings. Let removed-key
checking accept an exclusion predicate, inspect `gate` directly in the
original loop mapping, and let bundle parsing and source recording consume
an iterator of borrowed entries. For example, a guarded bundle filters out
`when` without copying a large `set` mapping. Preserve the current diagnostic
precedence: removed loop keys are rejected before iteration controls are
filtered, and a loose verb beside `gate` still fails at its authored path.
The parser must continue expanding actions into its owned execution model;
only the temporary views become borrowed.

A smaller approach adds focused helpers for loop and guarded bundles that
collect only `(&str, &Value)` pairs, then reuse the existing bundle logic over
that borrowed collection. This retains a small allocation but avoids copying
operand data and makes the excluded keys explicit at each caller. Share the
same loop-key classification with source recording rather than adding a
second grammar. Compare large nested gates and bundles with large mutation
values before choosing a broader parser-interface refactor.

### Reuse authored surface mapping across lifecycle validation passes

**Importance:** low. **Packages impacted:** `darkmatter`, `claudine`, and
`dmls`; their CLI consumers inherit the preparation work.

`lifecycle/validate.rs` constructs a complete `LifecycleSourceMap` separately
in nested-span validation and `lifecycle_call_checks`. In Claudine,
`prepare.rs::effective_surface` runs the first check, then its caller runs
`reject_incompatible_calls` against the same effective frontmatter and parsed
config. Each construction walks both branches, clones scalar text and paths,
and reconstructs named-operand ordering through signature selection in
`source_map.rs::record_long_form`. The parser already selected that ordering
when creating the action. DMLS likewise parses the profile and subsequently
checks calls. The repeated mapping is concrete, although no end-to-end
speedup has been measured; reconstructing the parser's choices also adds a
maintenance obligation when an action shape changes.

One solution adds internal validation entry points that accept a borrowed
`LifecycleSourceMap`, while keeping the existing public wrappers for callers
that need only one check. A preparation or editor operation builds the map
once and passes it through its nested-span and call checks. Keep the map
scoped to that exact frontmatter revision and host registry: a retry, reread,
or proxy target must create a new one. This is a bounded reuse opportunity,
not a process-wide cache, and requires no change to action execution.

Another solution has parsing produce an optional authored-surface sidecar
alongside the config. When the parser chooses a long-form overload, it records
the selected argument's authored key and text at the same time; validation
then reads those records rather than reproducing overload selection. For
example, an omitted optional named parameter retains the parser's exact
argument index without a second lookup. Preserve authored strings separately
from `Expr`: nested-span diagnostics must distinguish authored quoted
literals from synthesized literals. Neither approach should evaluate operands
early, skip an unselected branch's syntax checks, or reuse evaluated values
across mutations.

## 2026-10-21-claudine-schemas

The review followed the shipped bundle, disk shadowing, embedded import
isolation, winning-property provenance, launch cleanup, and editor property
projection. The final implementation preserves disk versus embedded origins,
keeps invalid Claudine controls out of optional cleanup, and shares the
effective property shape with editor consumers. Later recursive types and
generated operand typing remain part of the integrated behavior. No
correctness defect was found in these contracts.

### Reuse schema preparation across the optional scrub and launch check

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`;
`darkmatter` supplies the existing registry-reuse API.

`schema/mod.rs::pre_validate_verdict` first calls
`drop_invalid_optionals_with_origin`. That helper builds the effective
instance, calls `load_effective_schema`, and validates it to identify values
eligible for cleanup. Even when validation succeeds and nothing changes, the
caller builds that same instance and calls `load_effective_schema` again.
Each call constructs a new `DarkmatterSchemas` and eagerly discovers triggers
from the document's schema roots. Parsed payload and validator memos reduce
later work, but do not eliminate repeated discovery, source reads, matching,
and effective-schema bookkeeping. This is the preflight scrub/check pair,
separate from the `md schema validate` display duplication recorded earlier.
No end-to-end speedup has been measured.

One approach returns a private preparation sidecar from the scrub helper:
the effective schema and whether the instance changed. If nothing was
removed, pre-validation can retain that schema and apply its required
phase-specific verdict without rediscovering it. A direct compose check may
also reuse an equivalent unphased report, but an inline launch check must
preserve launch projection, pending-value handling, and file-array
normalization. If cleanup removes an identifying property, rebuild and
rematch triggers against the cleaned instance; for example, dropping an
optional application discriminator can change the applicable payload.

Another approach creates one document-scoped schema preparation context and
shares its trigger registry between scrub and launch checks using
`DarkmatterSchemas::with_trigger_registry`. Each check still calls
`effective_for` with its actual instance and retains its own validation
phase. This saves discovery while allowing trigger selection to change after
cleanup, and can serve a later composition stage only while its source and
schema inputs remain the same. Keep the scope local to this preparation;
retries, proxy targets, and fresh document reads must observe changed disk
schemas and their existing failure behavior.

### Declare shared document metadata types once

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`;
`darkmatter-cli` and `dmls` consume the authored schema artifacts.

`schemas/inline-compose.yaml` repeats the complete `prompt`, `hash`, and
`last_updated` definitions from `schemas/claudine.yaml`, including the
longhand hash object's enum, default, payload, ignored-key list, and
descriptions. The existing parity test checks that the inline payload's
lines are a subset of the main payload's lines, so current definitions agree.
The maintenance cost remains: changing the hash representation requires
editing two nested declarations and their descriptions. Other shared types,
such as linking properties and lifecycle stacks, already use named imports.
The inline trigger itself remains useful for independently shadowable schema
selection; this finding concerns only the repeated value definitions.

One approach puts these three value types in a regular schema library and
has both trigger payloads import them. For example, `hash` can reference a
single named type holding its shorthand and longhand alternatives. Retain
the current descriptions, optional presence, and open document behavior.
Choose the library's ownership deliberately rather than moving unrelated
document metadata into `linking.yaml` solely to avoid another file. Update
the embedded bundle inventory and complete-disk-bundle documentation if a
new library is added, and replace textual subset checking with resolved-type
parity across both trigger paths.

A smaller approach generates just the shared payload declarations from one
checked-in metadata catalog, retaining the trigger files as readable YAML
artifacts and their independent matching rules. A deterministic regeneration
check verifies both outputs. This preserves the current file inventory and
can share description text as well as value types, but adds generation
machinery for only three properties. Compare that maintenance cost with a
small type-library extraction before adopting a broader schema generator.

## 2026-10-22-tts-prewarm

The review followed collection, conservative expression classification,
invocation-wide de-duplication, worker ownership, playback configuration,
sequence preflight, and cancellation. The dedicated worker thread keeps
provider discovery and synthesis off the launch runtime, and candidate keys
retain each speech path's playback configuration. The collector correctly
visits both conditional bodies without evaluating their guards. The findings
below concern final integrated container coverage and repeated preparation
work; no synthesis-latency improvement is claimed without measurement.

### Include container-owned lifecycle speech in pre-warm discovery

**Importance:** medium. **Packages impacted:** `claudine`, `claudine-cli`.

**Status:** addressed during this audit. The existing preflight collector now
visits root and nested container lifecycles with their lexical frontmatter,
and container preparation resubmits against captured state through the same
invocation handle. The regression queued one phrase instead of five before
the repair; the new unit and CLI tests pass with the fix. All 218 distinct
scoped L1 tests passed, with a maximum of 2.450 seconds. The original evidence
and alternatives below describe the implementation before that repair.

The sequence pre-warm walk predates container-owned events. In
`prewarm/sequence.rs`, `Walk::task` descends into a group's tasks or a nested
sequence's steps but does not collect either container's own `lifecycle`.
`submit_sequence_preflight` also reads the root document's lifecycle only
when the graph contains a body step, rather than collecting
`PreflightGraph::root_lifecycle` independently. A root sequence consisting
entirely of explicit tasks can therefore skip its own literal `success`
speech, while a nested container's literal terminal or loop-gate speech is
also absent from the preflight queue. The root and nested container runners
do not currently call `collect_and_submit` when they establish or refresh
their event state. Event-time speech remains correct, but these paths miss
the preparation opportunity promised by the invocation-wide feature. Existing
preflight coverage includes a root body step, which does not expose the
task-only root case.

One solution extends the existing preflight traversal to collect the root and
each container's already-parsed lifecycle before visiting its children. Carry
the correct lexical frontmatter through the walk: a referenced sequence's
own frontmatter replaces its parent's base, while inline containers inherit
their enclosing state. Keep `seq`, `group`, `loop`, injected setters, and
unestablished `ctx`/`env` reads late. The same shared collector should run
when a container's state is established after initialization and when a reread
changes it. A task-only root and nested group/sequence, each with distinct
literal phrases, provide small recorder tests; a referenced sequence with a
different title checks lexical ownership.

Another solution adds one container-preparation adapter that receives its
parsed lifecycle, effective state, captured context, and invocation handle.
Static preflight calls it with pending context, and the container runners
call it with their established state. The adapter can reuse
`collect_and_submit` and its existing de-duplication instead of creating a
second speech walker. This places lifecycle submission beside container
preparation and makes later re-entry sites explicit, at the cost of carrying
the handle through the library container execution boundary. Neither approach
should synthesize during traversal, prepare prompt bodies early, or make
event-time playback wait for warming.

### Reuse one evaluation snapshot across a submitted phrase batch

**Importance:** low. **Packages impacted:** `claudine`; callers in
`claudine-cli` inherit the preparation cost, and `darkmatter` supplies the
evaluation-state APIs.

`PrewarmHandle::queue` invokes `classify` for every phrase. Each eligible
phrase's `resolve` clones the complete frontmatter map into a new `HashMap`,
clones the captured `ComposeContext`, and builds a new `EffectiveState`.
`early_reads` also constructs the scope's runtime bindings for each phrase,
including literal text with no reads. These operations run synchronously on
the submitting thread before invocation-wide text/config de-duplication.
Thus many speech actions over one large document repeatedly copy identical
state, even when a bootstrap/reread discovers already-queued phrases. This is
distinct from the document-entry environment copies recorded earlier: it
repeats within one submitted snapshot, once per phrase. Its end-to-end impact
has not been measured.

One approach introduces a submission-local classifier session. Build the
immutable effective state once from `PrewarmInputs`, reuse it across the
batch, and prepare binding views by lifecycle scope. Keep expression
eligibility analysis per phrase and preserve speculative-failure skips. The
session ends with that submission, so an initialization write, retry reread,
or proxy overlay still receives a fresh snapshot. For example, twenty
`success` phrases over the same frontmatter need one state construction,
rather than twenty complete copies.

A smaller approach adds a literal fast path and lazily constructs the shared
state only when the first eligible phrase actually reads data. Plain literal
text can retain the same empty-text and authored shell-syntax checks without
building frontmatter or runtime bindings. A batch-scoped serialized TTS
configuration can also be reused when constructing keys. This avoids a wider
classifier API change but leaves some repeated work for expression-heavy
batches. Measure many phrases with a large frontmatter/context snapshot,
including a duplicate reread, before choosing between the approaches.

## 2026-10-23-shell-template-approval

The review followed typed placeholder projection, shape matching, current-value
filling, saved policy loading, and Claudine's structured lifecycle and sequence
dispatch. Exact and template approval keys remain distinct, argument values
are not reparsed as command syntax, and current blacklists are checked before
dispatch. Later guard refinements and recursive declarations remain part of
the integrated implementation. No correctness defect was found in the inspected
contracts; the findings concern repeated preparation work.

### Retain parsed placeholder expressions across analysis passes

**Importance:** low. **Packages impacted:** `darkmatter`; template consumers
include `darkmatter-cli`, `claudine`, and `claudine-cli`.

A `Placeholder` retains its expression text and authored byte span but no
parsed expression. `grammar.rs::take_hole` calls `project_hole` with an empty
type scope, which parses the expression. `CommandTemplate::project_types`
parses it again under the owning document's scope. During candidate discovery,
`DeferredShellRoots::late_roots_of`, the guard lookup, and the presence check
each parse the same text independently. At execution, `LookupEvaluator`
parses it again before evaluating it against current state. Retyping and
reevaluating are necessary when state or guards change; reconstructing an
unchanged authored expression tree is separate work. Commands with many holes
or repeated sequence preparation multiply these parser calls, although their
effect on end-to-end runtime has not been measured.

One approach retains an immutable parsed-expression sidecar with each
placeholder. Add internal helpers accepting `&Expr` for type projection,
late-root collection, guard lookup, and evaluation. Keep the original text
and span for diagnostics and saved-rule display. For example, a placeholder
reading `last(seq.outputs)` can reuse its syntax tree while its value is
evaluated afresh at the step barrier. A reread with changed authored text
creates a new tree, and a changed schema still retypes the tree; cached syntax
must never retain an old value, refinement, or approval decision.

A smaller approach makes candidate construction parse each distinct expression
once into a local map and passes borrowed trees through its analysis helpers.
The public string-based `HoleEvaluator` remains unchanged, and execution still
parses through that interface. This removes repeated discovery work without
changing the public placeholder representation or callback contract, but
retains parsing across discovery and dispatch. Measure preparation of many
typed holes before choosing whether the broader sidecar is worthwhile.

### Load only the policy rules a runtime blacklist check needs

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`;
`darkmatter` owns the shared rule loader.

`harness/shell.rs::user_blacklist` constructs a complete
`ShellApprovalOptions` and calls `SessionPolicy::load`, then returns only
`policy.blacklist`. The loader reads and parses both the whitelist and the
blacklist through `store::load_ruleset`; the whitelist result is discarded.
Both structured lifecycle dispatch and sequence primary-template preparation
call this helper to check a filled command before dispatch. A large whitelist
therefore adds unrelated file reads and template-rule parsing to every such
runtime check. Current blacklist freshness is required; this finding does not
propose replacing that check with a previously cached approval. The extra
reads and parsing are evidenced, but no latency improvement has been measured.

One approach extracts a narrow blacklist loader taking the existing policy
root. It derives the same policy paths and reads only the blacklist, keeping
the current missing/unreadable-file behavior and built-in checks. The full
`SessionPolicy` loader remains for approval decisions that need both files.
For example, running a previously approved `build {{ pkg }}` still reads the
current blacklist immediately before dispatch, without reparsing thousands of
unrelated whitelist entries. Share path derivation and error policy with the
full loader rather than introducing a second interpretation of policy roots.

Another approach introduces a read-only policy-file loader that retains parsed
rules by exact file contents, with each caller selecting the files it needs.
It must read current contents on each required check, replace the parsed entry
when those contents change, and invalidate or replace an entry after policy
persistence. This can also reduce parsing during repeated approval checks,
but adds cache ownership and bounds that the narrow loader avoids. Keep the
cache independent of authorization membership: newly added blacklist rules
must still refuse a command whose shape was approved earlier.

## 2026-10-24-sequence-containers

### Allocate private group state when a leg is admitted

**Importance:** medium. **Packages impacted:** `claudine`, `claudine-cli`.

The group scheduler correctly limits worker threads through `max_parallel`,
but `sequence/task/group.rs::run_parallel` constructs a private
`RuntimeState::from_snapshot(base.clone())` for every declared leg before
starting those workers. It also copies the complete live frontmatter into
one cell per leg. Both snapshots contain owned JSON trees. A group with
1,000 legs and `max_parallel: 4` therefore creates 1,000 copies of its
group-start state even though only four legs can execute at once, and even
if the first failure prevents most legs from being admitted. This is a
memory-scaling opportunity rather than a violation of the documented
concurrency limit; peak-memory savings have not been measured. Validation
and context capture for every leg before effects remain intentional.

One approach keeps the immutable group-start snapshots shared and creates a
leg's private runtime and live-frontmatter cell inside the worker immediately
after declaration-order admission. Keep the base snapshot fixed for the
whole group, retain each finished leg's mutation delta and result in its
indexed slot, and drop its private cells when that leg settles. With a cap
of four, at most four active copies are then needed; the final fold still
uses declaration order. Eager task validation, captured per-leg contexts,
cancellation opt-ins, and never-admitted cancellation outcomes stay as they
are.

Another approach gives runtime branches an immutable shared baseline plus
owned deltas, with copy-on-write live frontmatter. A leg reading an untouched
property would borrow its group's baseline, while its own `set` writes would
land in its private delta. This can also benefit uncapped groups, but it
changes more of runtime-state ownership and needs explicit tests that a
sibling's mutation never becomes visible, retries start from the required
checkpoint, and atomic failed groups publish no deltas. Measure a capped
large group before choosing the broader representation change.

### Project the runtime fields a container event actually needs

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`.

`RuntimeState::snapshot` deep-clones the entire `RuntimeSnapshot`: mutations,
committed outputs, the lexical `seq` value, and the proxy chain. Container
drivers frequently request only one of those fields. For example,
`RootRun::event` uses `snapshot().mutations` to update live frontmatter, and
the nested sequence driver takes separate complete snapshots for its event
outputs, event mutations, and parent live-cell update. Its child loop also
takes separate snapshots for the position's outputs and the child's
mutations. As result history grows, a small mutation-only read repeatedly
copies unrelated provider responses and nested container results. The
unnecessary copies are visible in the implementation; their runtime cost has
not been benchmarked.

One approach adds narrow owned projections, such as `mutations_snapshot()`
and an output snapshot, to the existing mutex-backed runtime API. When an
event needs several fields, obtain those fields together under one lock and
reuse that owned view for the event. This preserves the important rule that
no lock or borrow remains held while lifecycle actions run. A root
`finalize` that needs to refresh one changed frontmatter property would no
longer copy every earlier response solely for that refresh.

Alternatively, retain the full snapshot API but store immutable result
history and lexical bindings behind shared ownership, replacing them only
when a step commits or scope changes. Full snapshots would then share the
unchanged history while retaining independent mutation data. This reaches
more callers, including group branching, but needs a deliberate boundary
between immutable committed results and mutable attempt state. Preserve
retry clearing, lexical restoration, declaration-order publication, and
current-output visibility rather than caching a stale rendered `seq` value.

## 2026-10-25-loops-and-flow-control

The review followed the shared loop frame and post-pass decision path,
container checkpoints, nearest-loop break routing, returning helper dispatch,
task guards, and dry-run admission. The shared frame keeps condition-before-cap
classification and action staging in one place. Existing implementation notes
also identify three unresolved `prep` scope departures: standalone group calls,
advance approval inventory, and an invocation-wide remaining deadline. Current
calls inherit per-child timeout settings and approve their targets at entry;
this review does not treat those departures as implemented guarantees or
silently decide their product scope.

### Make helper target admission match the callable primitive surface

**Importance:** medium. **Packages impacted:** `claudine`, `claudine-cli`.

`commands/prep_target.rs::target_kind` classifies a Markdown `kind: group`
document as `TargetKind::Sequence`, then dispatches it through
`run_sequence_target`. That entry point calls `reject_non_sequence_kind`,
which explicitly rejects groups. YAML targets are also classified as sequences
before their declared kind is considered. Consequently, a valid group that
works as a sequence step cannot be called directly with `prep`, despite the
uniform helper surface specified for documents, sequences, and groups. The
failure arrives when the helper is entered, potentially after its caller has
already modified files. The implementation log records this limitation; the
misleading internal classification also makes it harder to see which target
kinds are intentionally callable.

One approach adds an explicit group case to a shared in-process primitive
dispatcher. Adapt a standalone group into an invocation-owned task without
pretending its source file is a sequence that references itself. Reuse the
existing group preflight, lifecycle, cancellation, and result machinery while
keeping the helper's runtime roots isolated. For example,
`prep: '@checks.md'` could call the same parallel checks already accepted by
`group: '@checks.md'`, then return without appending the group result to the
caller's outputs. Preserve the combined call/handoff cycle identity and the
caller-relative overlay origin.

A narrower approach makes target classification explicit about unsupported
callable kinds and rejects a statically known group helper during the caller's
passive preflight, with an example of the supported sequence wrapper. The
classification and preview paths should share that decision, so a dry run
cannot imply that a known unsupported target is runnable. This improves the
current error timing without adding group execution, but it remains a scope
reduction from the specified uniform helper interface and requires the
outstanding author decision. It does not by itself implement transitive shell
approval or a total invocation deadline.

### Project scalar loop members without cloning the previous result

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`.

`looping/expression.rs::LoopExpressionLookup::get` resolves a named loop member
by building `ambient.to_value()` and then selecting the member. Building that
object clones `loop.last_output` even for a small read such as `loop.count`
or `loop.is_last`. For a looping sequence or group, the previous result may
contain large nested arrays or dictionaries; repeated scalar predicates copy
that unrelated data. Reading `loop.last_output` itself additionally clones the
selected value from the newly built temporary object. `LoopFrame::ambient`
already copies the result at the frame boundary, so this is extra work within
one evaluation. It is distinct from the preceding full `RuntimeState`
snapshot finding, and its end-to-end cost has not been measured.

One approach exposes a direct member projection on `LoopAmbient`, sharing the
existing `AmbientVariable` member catalog. A scalar lookup returns the scalar
directly; only a whole-root or output read clones the corresponding result.
For example, `loop.count < 100` would not copy a prior multi-megabyte group
result. Keep unknown-member behavior and `fm.loop` separation unchanged, and
exercise the projection through both condition and action evaluation.

Another approach introduces a borrowed loop-value view used by expression and
lifecycle bindings, materializing an owned JSON value only at the existing
`EvaluationLookup` return boundary. This can remove duplicated namespace
projection rules across consumers and permit shared immutable result storage,
but changes more ownership interfaces. Preserve the previous-versus-current
pass timing, retry restoration, and isolated break-slot identity; do not cache
a value beyond the pass that owns it. Measure scalar-heavy conditions over a
large prior result before choosing the broader representation change.

### Retain parsed loop conditions while evaluating their values afresh

**Importance:** low. **Packages impacted:** `claudine`, `claudine-cli`;
`darkmatter` supplies the shared expression parser and evaluator.

The loop configuration retains condition source as a string. Configuration
parsing validates that string, control-variable extraction parses it again,
and `looping/expression.rs::evaluate_condition` calls `parse_condition` and
checks retired-name reads on every completed pass. `LoopFrame` freezes the
condition configuration for the invocation, so reparsing its unchanged syntax
is separate from the required fresh value and file reads. The shared frame
makes this a bounded optimization for all three primitives, particularly
loops of inexpensive shell or side-effect tasks. No runtime improvement has
been measured, and the existing expression engine should remain the only
parser.

One approach gives the frame an immutable prepared-condition sidecar containing
the parsed expression and its original source. Reuse that tree for
control-variable analysis and each post-pass evaluation, but construct a fresh
lookup and perform every file function again. Keep the authored condition and
iteration in diagnostic wrapping. For example, an `until` expression reading
a marker file still probes the file after each pass; only its unchanged syntax
is retained. A new invocation or changed authored condition gets a new tree.

A smaller approach adds an internal evaluation entry point accepting an
already parsed expression and lets the document/container loop driver own it.
The public string-based helper can retain its current behavior for standalone
callers. This avoids changing the serialized `LoopConfig` model, but leaves
some repeated analysis during initial preparation. Preserve error timing and
the condition-before-exhaustion rule, and keep cached syntax independent of
runtime truth values, shell approval, and prepared document state.

## 2026-10-26-task-inputs-and-providers

The review followed task-input provenance, passive provider prediction, task
identity and review selection, composition-time identity selection, and the
fresh-read launch rebuild. The hard rename and schema-selected file anchoring
use their existing shared mechanisms. The findings concern distinguishing a
task's own model from an inherited decision, and preparing only the state that
passive selection actually needs.

### Preserve model provenance when a task's fresh document drops its hint

**Importance:** medium. **Packages impacted:** `claudine-cli`; `claudine`
supplies the selection types and shared model resolver.

**Status:** addressed during this audit. Model selection now distinguishes a
task's own document hint from a genuine fallback, and harness reprepare uses
the canonical identity selector before rendering the body. Five new CLI
regressions cover retry and pre-launch document rewrites, with positive
controls for `with`, captured environment, and enclosing fallbacks. The final
479 L1 and five review-screen L2 tests passed; prompt text, argv, and child
environment agree. The original evidence and alternatives below are retained.

Provider selection now predicts a prompt task's own model, but the refresh
path still treats every sequence target's planned model as a lasting fallback.
`target_launch.rs::planned_fallback_model` retains even
`FrontmatterSingle`/`FrontmatterList` for a sequence task, and
`task_run.rs::launch_target` returns the planned target unchanged when the
fresh hints omit `model` and keep its provider. These rules do not distinguish
a model read from this task's document from a true enclosing-document or
review-screen fallback. The task also projects its selected `MODEL` into its
composition environment, so removing just the fallback field can leave the
old value looking like higher-precedence environment input. For example, a
task that starts with `model: opening-model` and removes that line before a
retry can retain `opening-model` rather than returning to genuine caller
environment, an eligible inherited choice, or the provider default. The
implementation log identified the retry case as unverified. During this
audit, a real CLI provider-recording regression reproduced the retained
`MODEL`; the test agent is repairing the boundary and adding fallback controls.

One approach carries explicit model ownership with the task selection: task
document, task input, enclosing document, review choice, caller environment,
or explicit flag. On each fresh read, resolve the current task hints against
the captured caller environment, then apply only a still-eligible fallback
owned elsewhere and matched to the selected provider. Project the resulting
identity into the prompt and child environment after selection, without
promoting that projection into caller intent. This preserves an explicit
`--model` or review choice while allowing a removed document hint to disappear.
Tests should check both a retry and an earlier task rewriting a later task,
including prompt text, argv, and child `MODEL`.

A narrower approach separates the document-derived model from the fallback
model in the existing launch intent. Drop document-derived fallback values
and remove the previous identity projection before model resolution, restoring
the captured value when one existed. Keep enclosing and reviewed fallback
values in a separate channel rather than trying to infer ownership from
`ModelResolutionReason` alone: an enclosing document and the task itself can
both produce `FrontmatterSingle`. This changes fewer public types but needs
the same explicit distinction at the task-entry and retry boundaries, and
must preserve provider-specific environment precedence and model-list rules.

### Prepare prediction state lazily for absent or literal selection hints

**Importance:** low. **Packages impacted:** `claudine`; `claudine-cli` pays
the cost during sequence preflight.

`preflight/providers.rs::predict` eagerly clones and converts the complete
prompt frontmatter into `document_state` before it knows whether `agent` or
`model` needs evaluation. For each document-owned hint, `predict_key` then
clones a complete raw layered state and builds late-key metadata before its
literal fast path. An ordinary `agent: codex` with no model therefore copies
unrelated application data even though the hint requires no expression
evaluation; an entirely hintless task still builds the first full map.
Sequences of many documents with large frontmatter multiply this work. The
extra preparation is visible in the implementation, but its effect on total
preflight time has not been measured. This is distinct from event-time
runtime snapshots and phrase-batch preparation recorded earlier.

One approach classifies the two winning raw hint values first, allocating a
shared evaluation map only when a value contains an expression or needs
transitive dependency analysis. Absent hints and plain literal scalars/lists
can return their current typed states directly, retaining empty-list checks,
source locations, and the later whole-graph test for writes to `agent` or
`model` themselves. Shell substitutions and identity-dependent expressions
must still take their existing deferred or circular paths; the fast path
cannot mistake a literal-looking template for a static value.

Another approach introduces one borrowed layered-state view per task and
shares it across `agent` and `model` analysis. Materialize an owned evaluation
state only at the existing evaluator boundary, and compute common deferred
metadata once. This also reduces duplicate work for expression-heavy hints
but touches more lookup interfaces. Preserve atomic `with` evaluation,
caller-setter precedence, source ownership, transitive runtime-write
deferral, and ordinary-expression failures rather than caching a prediction
past the fresh preparation boundary. Measure many literal tasks and a second
case with dependent hint expressions before choosing the wider refactor.

## 2026-10-27-darkmatter-lifecycle-ownership

The review followed the shared registry, parser and policy boundaries, the
`md` request/reply driver, Claudine's adapter registration and ownership
guards, and the passive lifecycle profile used by DMLS. The inspected runtime
paths keep routing decisions in Darkmatter and do not introduce a second
Claudine policy implementation. The findings concern lifetime and discovery
costs at the editor's host-profile boundary.

### Give dynamically loaded lifecycle profiles a reclaimable lifetime

**Importance:** medium. **Packages impacted:** `darkmatter`, `dmls`; host
authors such as `claudine` supply the catalogs and companion artifacts.

`schemas/catalog.rs::host_lifecycle_profiles` retains a process-wide map keyed
by catalog path, catalog content hash, and the complete artifact text. The map
has no eviction or replacement policy, so editing a catalog or regenerating
its artifact adds another retained revision. Every new key also leaks a
`GlobalCatalog` before checking the lifecycle artifact, including when that
artifact is absent, stale, or invalid. `HostLifecycleProfile::from_artifact`
leaks dynamic host-global names and an `ignore_unprovided` verb list to meet
the registry's static-reference interfaces. A long-lived DMLS process thus
retains obsolete artifact text, catalog schemas, and profile data after
documents stop using them. Evicting just the map entries would not reclaim
the leaked allocations. The growth follows distinct loaded revisions; no
resident-memory measurement has been made.

One approach makes dynamic registry data owned and shared: store an
`Arc<GlobalCatalog>` and owned or shared strings in the registry and parse
policy, while keeping static constructors ergonomic for compiled hosts.
Retain the latest revision per path, or use a bounded revision cache, and let
active document analyses hold their own `Arc<HostLifecycleProfile>`. For
example, saving `globals.lifecycle.json` repeatedly would replace the cache's
old revision while an already-started analysis could finish with its original
profile. Preserve exact catalog/artifact validation and cache failures without
retaining every historic failure forever.

Another approach gives a workspace analysis session an explicit profile arena
and makes dynamically rebuilt registries borrow that arena through a lifetime
or a separate borrowed profile representation. A catalog/artifact update
creates the next immutable analysis generation; dropping the last document
snapshot from the previous generation releases the entire arena. Compiled
host registrations can keep their process-lifetime representation. This
avoids per-field shared ownership but changes more lifetime-facing APIs and
requires a clear boundary preventing a registry from outliving its arena.
In either approach, test repeated valid and invalid artifact revisions and
document closure, including held older snapshots; a bounded map alone is
insufficient while `Box::leak` remains on the dynamic load path.

### Reuse lifecycle-host discovery within an editor analysis request

**Importance:** low. **Packages impacted:** `darkmatter`, `dmls`.

The profile cache avoids rebuilding a known registry, but each
`LifecycleHost::for_document` still calls `host_lifecycle_profiles`, which
enumerates every schema root, sorts candidate files, reads and classifies
their YAML, and reads companion artifacts before consulting that cache.
Frontmatter diagnostics and DSL incomplete-profile diagnostics each request
the host independently; completion and the two frontmatter hover paths do so
as well. The discovery function also builds a complete vector before the
editor chooses its first valid profile, so later candidates are read even
when an earlier valid host already decides the result. This repeats passive
filesystem and decoding work on interactive requests. The existing cache
does not remove those operations, but their latency has not been measured.

One approach adds a lazy request-scoped lifecycle-host observation to
`DocumentContext`, alongside the context's other analysis inputs. Diagnostic,
completion, and hover helpers borrow that one observation rather than each
rediscovering files. A subsequent request or watched catalog/artifact update
must produce a fresh observation, so repairing an invalid artifact immediately
removes its warning. Share the captured catalog candidates with ordinary host
catalog analysis where possible; retain precedence, malformed-catalog
reporting, and the passive no-host-code rule.

A smaller approach exposes discovery as a fallible iterator or a callback
walk and lets the editor stop once it obtains the first valid profile. Keep
the reasons from preceding unusable candidates for the existing fallback
case, and preserve the rule that incomplete warnings are reported only when
no profile can be selected. An optional discovery object can then be passed
explicitly between the frontmatter and DSL diagnostic passes without changing
the document-context model. This removes work after the winning candidate
and duplicate work in that diagnostic request, while leaving separate editor
requests fresh. Measure filesystem reads and YAML decodes on a schema root
containing many unrelated files before choosing the larger shared-observation
design.

## 2026-10-28-schema-enhancements

### Materialize rejected-candidate diagnostics only when the call fails

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

The shared engine and binder retain the required order-independent selection
rules, but successful bindings can pay for diagnostics they discard.
`coercion/union.rs::Engine::union` collects an owned `OptionFailure` for each
failed conversion candidate even when another candidate ultimately succeeds.
`engine.rs::fail` clones the received JSON value, builds an owned path, and
formats the expected shape. At the function level,
`call_binding/bind.rs::bind_each` additionally formats every rejected
signature through `candidate`, then drops those failures whenever any overload
binds. For a large object accepted by one alternative but rejected by several
others, these are copies and strings on a successful path. All candidates
must still be considered to preserve ambiguity and exact-match precedence;
the opportunity concerns diagnostic ownership, not skipping that judgment.
The cost has not been benchmarked.

One approach introduces internal borrowed failure records that retain a
reference to the received value and expected shape, plus the necessary path
and reason. Keep those records until union or overload selection finishes,
then materialize the existing owned public diagnostics only if no candidate
binds. For example, a rejected scalar arm need not copy an entire supplied
object if an object arm later accepts it. Public error types, every failing
path, numeric recovery's retained subject error, and caller-owned values keep
their current contracts. Deferral predicates must still be consulted exactly
as required during the original walk; this should not become a second
evaluation pass.

A smaller approach leaves engine failures owned but postpones display work
and wrapper construction. `bind_each` can retain each rejected overload's
index and `SignatureProblem`, constructing its `CandidateFailure` and
signature string only when the final result needs them. The union layer can
likewise derive an option's display text when assembling `NoOptionAccepts`.
This avoids a failure-lifetime redesign and repeated signature rendering,
although it retains the engine's value clones. Measure successful bindings
with several rejected alternatives and compare allocation counts before
choosing the broader representation change.

### Reuse the bounded structural validator cache for refined call types

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`dmls`, `darkmatter-cli`, `claudine`, and `claudine-cli`.

`call_binding/translate.rs::format_assertion` owns a process-wide
`Mutex<HashMap<String, Arc<Validator>>>`, keyed by serialized schema fragments.
It builds validators through `build_structural_validator` and never evicts an
entry. Refined signatures with different constraints therefore remain cached
after their registrations are dropped. Meanwhile `schemas/validate.rs`
already provides `ValidatorCache::structural_validator_for`, with bounded
capacity, structural-context identity, and shared validator ownership. The
second memo independently implements lookup, compilation, locking, and
retention policy. This is distinct from the lifecycle-profile leaks above:
these allocations are ordinary owned values and are reclaimable if their
cache entries are evicted. No process-memory growth measurement has been made.

One approach gives the signature translator a shared `ValidatorCache` and
requests its structural slot for each fragment. The returned `Arc` remains
captured by the assertion, so evicting an unused cache entry cannot invalidate
an active compiled signature. For example, repeatedly rebuilding an editor
extension's constrained URL signatures would retain only the bounded memo
plus validators still owned by active registrations. Preserve syntax-only
file checks, custom format registration, and the separation from validators
bound to a document's file-resolution context.

Alternatively, let a catalog or registration-compilation session own a
fragment memo and pass it explicitly through translation. Signatures compiled
in that session share validators; dropping the session and its registrations
releases their obsolete fragments. This makes lifetime and retained memory
follow the caller's catalog rather than a process-wide setting, at the cost
of threading compilation context through more constructors and losing reuse
between unrelated sessions. Test repeated unique signatures with an older
registration still alive, and verify that eviction or session replacement
does not change either registration's binding result.

## 2026-10-29-recursive-schema-types

### Reuse completed recursive conversion targets across sibling references

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

The JSON conversion translator correctly closes a recursive path with a weak
back reference and retains the completed target through an owning `ShapeRef`.
However, `schemas/json_shape.rs::Translator::reference` finds reusable targets
only in its active `references` stack. It pops a target after translating that
body and keeps no completed-target map. If three sibling properties reference
the same self-recursive `item` definition, each sibling translates `item`
again and creates a separate owning slot, even though the emitted JSON Schema
stores that definition once. The existing `ConversionPlan` memo avoids this
work on later validations of the same schema; it does not remove the repeated
work inside the first translation of a schema revision. No cold-translation
speedup has been measured.

One approach adds a translation-local memo for completed, self-contained
recursive targets. For example, after translating a self-recursive `item`,
subsequent sibling uses can clone its owning `ShapeRef` rather than lower its
body again. A target that contains weak links to an enclosing target cannot be
reused by pointer alone: either include that construction context in the memo
identity or decline to memoize it. Keep the current reference-depth and work
budgets explicit, preserve the resource's resolved URI identity, and retain
per-use constraining siblings outside the shared target.

Alternatively, index the reachable reference graph into recursive components
before translating those components. Allocate their slots once, lower each
body once, and give the resulting shape graph a deliberate ownership scheme
that keeps internal back references weak. This handles mutually recursive
families rather than only self-contained targets, but changes more of the
translator. Use a work-count probe for repeated sibling references and a
lifetime test for mutually recursive targets; verify identical conversion,
assertion fallback, and release behavior before claiming an improvement.

### Project definition bodies without cloning the body being replaced

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

Phase projection calls `DefinitionTable::map_bodies` from
`schemas/phase.rs::project_definitions`. Its projection closure clones each
body and then changes that clone for the requested phase. The table builder
constructs `Definition { body: project(&definition.body),
..definition.clone() }`, which also expresses a clone of the original
`Definition`, including the body that the explicit `body` field replaces.
Only `origin` and `json_key` are needed from that base value. This adds an
avoidable full-body clone at the source level on a path introduced to keep
recursive definitions shared; compiler elimination and end-to-end cost have
not been measured. It is distinct from caching whole phase projections or
changing their keys and lookup semantics.

The smallest approach constructs the replacement definition explicitly:
`Definition { body: project(&definition.body), origin:
definition.origin.clone(), json_key: definition.json_key.clone() }`.
The projection closure remains the one owner of the body copy. Allocation
order, `DefKey` identity, source origins, and the reverse JSON-key map stay
unchanged, so existing phase and merge tests can validate the result.

Alternatively, add a consuming projection operation for callers that already
own an unshared definition table. It can move keys and metadata and transform
each body in place, while retaining the borrowing projection for shared
baseline and trigger tables. This removes more copying when ownership is
available, but introduces a second API that should be justified by actual
callers. For either approach, compare a definition with a large nested body
under Launch and Completion and retain the current mixed-use timing and
stable-reference tests; a clone-count or allocation probe would establish
whether the smaller source expression produces measurable savings.

## 2026-10-30-claudine-recursive-schemas

### Share repeated acyclic action helpers as well as recursive families

**Importance:** medium. **Packages impacted:** `darkmatter`, `claudine`;
consumers include `darkmatter-cli`, `dmls`, and `claudine-cli`.

The recursive stack and container families correctly become shared definitions,
but their large acyclic helpers still expand at every reference. In
`claudine/schemas/lifecycle.yaml`, each ordinary `stack` and `item` references
the nonrecursive `entry`, which in turn references action categories and the
short-form bundle. Initialization and cancellation have corresponding helpers;
`loop-single` and `loop-bundle` repeat the same generated verb references.
`resolve.rs::ImportEngine::expand_import` shares a `TypeExpr::Ref` only for a
member of `self.cyclic`; an acyclic cached expansion is cloned and receives its
use-site postfix. The cache saves expansion work but does not share the output
body. Consequently, recursive nesting is compact while the action alternatives
around it remain large. The implementation log records about 1.83 MB for the
combined effective payload, and
`embedded_trigger_payloads_resolve_within_two_megabytes` imposes a 2 MB ceiling.
That is a passing budget, but adding an ordinary helper reference can consume
the remaining headroom disproportionately. No new timing or allocation
measurement was made during this review.

One approach extends definition sharing to selected repeated acyclic named
types. For example, the resolved `entry` and each action category could have
one definition per source `DefKey`, with references at stack and item sites.
Keep use-site descriptions, constraints, required markers, and provenance
outside the shared base where necessary; a constrained use must not mutate the
other uses. Preserve lexical imports, merged-trigger rekeying, and editor
traversal. This belongs in the shared resolver rather than introducing
hand-written raw JSON Schema or a Claudine-specific lowering path. Compare
resolved bytes, cold resolution, validator compilation, and the existing
deep-profile diagnostic corpus before broadening which types are shared.

A smaller approach adds explicit sharing for a narrow set of exported helper
types, leaving ordinary acyclic inlining as the default. A resolver option or
schema-level sharing declaration could retain `entry`, the action categories,
and generated operand types without changing all named-type presentation at
once. This requires a clear ownership and serialization contract and should
not be simulated by inventing cycles in otherwise acyclic schemas. It offers
a bounded experiment for the existing payload budget, at the cost of another
authoring choice. In either approach, retain ordinary, initialization, and
cancellation profiles independently; sharing must not widen their permissions.

### Index operand sites and reuse immutable marker analysis

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

Passive operand binding correctly walks the instance beside the recursive
schema, then uses the runtime registration for each authored call. Its
discovery bookkeeping can nevertheless repeat work for large action lists.
In `schemas/bound_operands.rs::Walker::walk`, every marked action searches
`self.sites.iter_mut().find` by its complete path. For N distinct action sites,
the inserts alone inspect a growing prefix of previously recorded sites,
even though the new path usually has no match. `marks` also recursively
collects the same verb/form metadata from immutable schema alternatives and
references whenever that schema node judges another instance item. The
conversion plan already caches the whole-schema boolean `marks_operands`,
but that boolean does not cache this per-node analysis. These costs precede
the required per-instance binding; their effect on validation latency has not
been measured.

One approach keeps the ordered site vector and adds a path-to-index map.
Discovery can merge forms into an existing indexed site or append a new site
without scanning all earlier actions. A walker-local map keyed by schema-node
identity can likewise memoize `marks` results, including no-marker and
ambiguous-marker outcomes. For example, a long list of `abs` actions can reuse
the same short/long-form analysis while each value is still bound separately.
Preserve traversal order, the rule that competing verbs clear accepted forms,
the hop backstop, and per-document literal-token and malformed-template checks.

Alternatively, compile an immutable operand-discovery plan alongside the
existing conversion plan. It would retain reference edges, property/item
routes, and precomputed verb/form sets for a particular resolved schema; each
validation supplies fresh instance and stored values. This can reuse more work
across editor requests but needs deliberate ownership for recursive nodes and
the same bounded cache/revision identity as the conversion plan. Keep live
binding, source positions, and diagnostics outside that cache. A benchmark
with increasing action counts and an operation-count probe would establish
whether the smaller indexing change is sufficient before introducing this
larger plan.

## 2026-10-31-type-aware-evaluation

### Find a scope join's shared history once

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

The persistent `TypeScope` correctly shares fact history between branches,
but joining branches repeatedly traverses the history they already share.
In `compose/expression/scope/mod.rs::TypeScope::join`, the two
`steps_since_shared` calls each build a complete ancestor-address `HashSet`
for the other branch, then `shared_ancestor` builds another complete set.
This happens even when the heads are identical or each branch added only one
step above a long shared prefix. `scope/facts.rs::short_circuit` joins a route
for every operand of a disjunction or failing conjunction, and the lifecycle
call checker joins continuing conditional branches. Thus a short branch late
in a long analysis can allocate and walk substantially more than its own
new facts. The touched-subject vector also uses a linear membership check.
No timing or allocation improvement has been measured during this review.

One approach introduces one internal history-split operation returning the
common link and both branches' new steps. It can use a single temporary
address index, with an immediate pointer-equality fast path, rather than
discovering the same common ancestor three times. For example, joining two
unchanged routes returns their shared scope without allocating an address
set. Keep the current oldest-first write replay and per-subject type join;
replace only ancestry bookkeeping, with tests for writes on one branch and
contradictory facts on both branches. A set alongside the ordered subject
vector can preserve diagnostic order while avoiding repeated membership scans.

Alternatively, store the persistent link depth and align the two heads by
depth before walking parent pointers together. This finds the common ancestor
without allocating an address index and can collect each branch's new steps
on that walk. It adds metadata to every link, so compare its steady cost with
the smaller one-pass temporary-index change. Preserve shared declaration
identity, overlapping-path invalidation, both continuing routes' writes, and
unavailable-global behavior. A work-count test with a long shared prefix and
one new fact per route would establish the improvement without a flaky timing
assertion.

### Reuse an authored position index across a diagnostic batch

**Importance:** low. **Packages impacted:** `darkmatter`; consumers include
`darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`.

The shared diagnostic record preserves the correct original-source position,
but locating many findings repeatedly scans the same authored text.
`compose/expression/diagnostics.rs::one_based_position` slices the whole prefix
through an offset, searches backward for its last newline, counts every prior
newline, and counts the current line's characters. `call_precheck.rs` calls
it for each advisory after source projection, and `lifecycle/validate.rs`
uses the same helper for located lifecycle findings. A large document with
many late-file warnings therefore repeats prefix work even though that
source buffer is immutable for the analysis. The library already has line
indexes in other subsystems, but their APIs and column contracts differ; a
byte-column index cannot directly replace this character-column helper.
No end-to-end diagnostic latency improvement has been measured.

One approach builds a small position index once per loaded source in each
diagnostic batch. Store newline offsets, binary-search the target line, then
count Unicode scalar values only from that line's start to the requested
offset. `one_based_position` can remain the convenience API for isolated
queries, while batch code borrows the index. For example, locating a hundred
warnings near the end of one file scans its earlier lines once rather than
a hundred times. Preserve the existing rejection of out-of-range offsets
and offsets inside a UTF-8 character, original CRLF bytes, and one-based
character columns.

Alternatively, provide a shared source-coordinate utility used by report,
schema-position, and editor adapters, with explicitly named byte, Unicode
scalar, and UTF-16 column operations. This reduces parallel indexing code
but has a wider blast radius than the batch-local helper and should retain
each consumer's existing coordinate contract. Keep source projection ahead
of coordinate conversion: rewritten body text, folded YAML, and transcluded
files must still locate in their own authored buffers. Compare LF/CRLF,
non-BMP text, nonzero offsets, and many findings in one source before claiming
a speedup.
