---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-31
review_iterations: 0
implemented: false
created: 2026-10-30
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
    - simplified-schema
related:
    - 2026-10-30-reference-navigation-and-hover
    - 2026-10-28-schema-enhancements
    - 2026-10-28-extract-simplified-schema
---

# Schemas for YAML Documents

## Problem

Markdown frontmatter can be held to a schema: a `$schema` property names one
(inline or by reference), and schema triggers apply others. A YAML document
cannot. `md schema validate` reads only Markdown frontmatter, and DMLS offers
no schema diagnostics, completion, or hover in a YAML file.

Authors already work around this. `messenger/docs/platforms.yaml` is a plain
YAML data file whose header says:

> The `---` fences are required: `md schema validate` reads only frontmatter,
> and an unfenced file is reported "valid by default" without validation.

The two YAML kinds Darkmatter does understand, `kind: schema` and
`kind: schema-trigger`, are recognized by hand-written Rust parsers. There is
no way for anyone else to declare a YAML document kind and have its documents
checked.

### Revision note

The first version of this spec (same date) framed the work as validating the
two schema kinds against shipped definitions, and that is what was
implemented: a hard-coded two-entry kind table, a YAML route in
`md schema validate`, and a DMLS overlay for those two kinds. It did not hold
any other YAML document to a schema, and it changed no accept/reject outcome
(the Rust parsers stayed the oracle). This revision replaces that framing.
What of the earlier implementation is kept, reshaped, or removed is
[Design ruling 6](#6-what-of-the-earlier-implementation-is-kept--closed).

## Goal

A YAML document is held to the contract of a schema, by `md` and by DMLS,
through the same validator Markdown frontmatter uses, when either:

1. its top-level `kind` is claimed by a recognized `yaml-schema`, or
2. it has a root `$schema` (and no recognized `kind`).

## Design

### 1. The `yaml-schema` kind

A `yaml-schema` document defines a YAML document kind. It has largely the
structure of a `kind: schema` document, but it describes **only whole YAML
documents**, selected by their `kind`.

- Its `$schema` is the schema it **exports**: the contract every document of
  the claimed kind must meet. This is the same role `$schema` already has in a
  `kind: schema` file.
- The exported schema must declare its `kind` property as enumerable: a
  `literal(...)` (claims one kind) or an `enum(...)` (claims each listed
  value). The claimed kinds are those values, unioned across arms when the
  exported schema is a root union.
- Two `yaml-schema` documents that claim the same kind in one document context
  are an error, including a repository file that claims a shipped kind.
  The checkout-copy exception still needs the author’s decision under
  [Open Questions](#open-questions); identical filenames alone never establish
  that two definitions are the same source.

Darkmatter ships three, in `darkmatter/schemas/`, compiled into the binary:

| File | Claims | Defines |
|---|---|---|
| `schema.yaml` | `schema` | schema documents |
| `schema-trigger.yaml` | `schema-trigger` | trigger documents |
| `yaml-schema.yaml` | `yaml-schema` | `yaml-schema` documents, including itself |

```yaml
# darkmatter/schemas/schema-trigger.yaml
kind: yaml-schema
types:
    match-expression: { … }
    match-payload: match-expression | match-expression[]
$schema:
    kind: literal(schema-trigger; required) -> the document kind that claims this definition
    match: match-payload(required)
    $schema: schema
```

A user kind looks the same:

```yaml
# <repo>/schemas/roster.yaml
kind: yaml-schema
types:
    platform:
        name: string(required)
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
    platforms: platform[](required)
```

### 2. Discovery

`yaml-schema` documents are found in the shipped set plus the **document's**
five schema roots, the same roots triggers use: `SCHEMAS_DIR` (the folder
itself), the package `schemas/`, the package area `schemas/`, the document
context's tree-root `schemas/` (`base_dir()/schemas`), and `~/schemas`.
Do not substitute the process directory or walk ancestors.

Reuse Darkmatter's [schema-root enumeration](../../lib/src/markdown/schemas/triggers/discovery.rs):
regular YAML files directly inside each root, deterministic filename order,
no recursive scan, and the existing symlink and filename-collision rules.
A root reached more than once is searched once. Unlike trigger filename
shadowing, a different definition with the same filename is still a candidate:
kind claims, rather than filenames, decide conflicts. Compare claimed kind
strings exactly, including case; `roster` and `Roster` are different kinds.

Filesystem discovery uses the existing repository/workspace gate: `md`
discovers inside a repository, and DMLS inside a workspace. Shipped kinds
remain available everywhere, and an explicit root `$schema` still works
outside those gates. Disabling `schema-triggers` stops trigger matching;
it does not disable kind definitions or change a recognized `$schema` from
data into a contract. Schema-file lookup continues to use the shared
selector and its existing policy.

Discovery and preparation may read local schema sources through the one
captured document context. Validation over the prepared registry performs
no discovery or reference loading. Neither phase executes expressions,
runs commands, fetches remotes, or writes files. DMLS supplies cached source
text, including open unsaved definitions, through the same discovery path.
Failure handling and checkout copies are unresolved under
[Open Questions](#open-questions).

### 3. How a YAML document selects its schema

```mermaid
flowchart TD
    Y[YAML document] --> M{root is a mapping?}
    M -- no --> N[not validated]
    M -- yes --> K{top-level kind claimed<br/>by a yaml-schema?}
    K -- yes --> V1["validate against that definition only<br/>(a root $schema is an ordinary property it describes)"]
    K -- no --> S{root $schema?}
    S -- yes --> V2["validate against that schema<br/>(inline, or a reference to a schema file / JSON Schema);<br/>$schema is stripped first, as in frontmatter"]
    S -- no --> N
```

- **A recognized `kind` wins.** The claiming definition alone validates the
  file. A root `$schema` is then a property like any other, and the
  definition says what it is. For the three shipped kinds it is the exported
  schema, so `darkmatter.yaml` (`kind: schema`) is never validated against the
  globals it exports, and a trigger never against its payload.
- **Otherwise a root `$schema` is the contract**, using Markdown
  frontmatter’s declaration syntax and reference resolution: an inline mapping,
  root union, or reference to a schema export (including a JSON Schema file). `platforms.yaml` drops its `---` fences and is validated as
  written.
- A YAML file with neither, or whose root is not a mapping, is not validated.
  An unrecognized `kind` is not an error when registry preparation succeeded.
  A failed registry or an ambiguous claim must not silently fall through to
  another route and report success.
- Selection examines the authored `kind` string before soft conversion.
  Missing, null, or non-string `kind` values claim nothing; they may still be
  checked by an explicit root `$schema`. A present but invalid `$schema`
  (including null) on this fallback route is a schema error, not absence.
- Whole YAML documents receive **no Markdown baseline or trigger layers** on
  either route. This deliberately differs from Markdown frontmatter, whose
  globals and matching triggers describe composed Markdown. A YAML property
  named `title` or `content_policy` has only its selected schema's meaning.
  An explicit `$schema` is removed from the validation copy only, keeping
  the source map and document unchanged. In the claimed-kind route it stays.

```yaml
# platforms.yaml: no kind, so $schema is the contract
$schema: ./platforms.schema.yaml
roster_version: 1

# roster.yaml: kind is claimed, so only the roster definition applies
kind: roster
roster_version: 1
```

### 4. Surfaces

- `md schema validate` accepts `.yaml` and `.yml` files and reports through
  the same problem codes, exit codes, and output formats as frontmatter.
- DMLS provides diagnostics, completion, and hover in a YAML file selected by
  either route, ranged on the offending key or value, with the same
  `dm.schema.*` codes as frontmatter.
- The library exposes one entry point that takes a YAML document and its
  context and returns the selected schema's validation report.

#### Whole-file parsing and command behavior

A `.yaml` or `.yml` argument (any ASCII letter case) selects whole-file YAML
parsing after the normal file-argument resolver opens it. A leading `---`
is a YAML document-start marker, not a Markdown frontmatter boundary.
Exactly one YAML document is allowed; an optional terminating `...` is
allowed, but a second document is rejected. Use the shared admitted-YAML
loader and numeric model: duplicate keys, non-string mapping keys,
unsupported numeric values, and malformed YAML are located load errors,
even when no schema can be selected. Accept BOM and CRLF consistently with
standalone schemas. A valid scalar or sequence root is unvalidated, as the
selection flow states; this feature does not add schemas for such roots.

Report unvalidated input as **no schema selected**, using the existing
`no_schema` result, rather than implying that a contract was checked.
Preserve the command's exit codes: `0` for a passing or unvalidated input,
`1` for document validation failures (including kind semantic checks), `2`
for schema preparation or selection failures, and `3` for input/open/parse
failures. Mixed input batches retain precedence `3`, then `2`, then `1`.
Existing JSON result records remain the contract; do not invent a separate
YAML output format.

Assignments such as `key=value` remain Markdown-only and produce usage
error `64` with any YAML input, including a reference that resolves to YAML;
resolve and preflight the input batch before any assignment writes occur.
The built-in baseline and baseline environment settings are not loaded for
YAML-only invocations. An explicit `--schema` baseline with YAML is a usage
error rather than a silently ignored contract; mixed batches using that
option must be split. `--disable baseline-schema` is harmless for YAML.
Markdown inputs retain their current baseline and trigger behavior.

#### Passive values and source positions

Soft validation converts a copy using the shared SimplifiedSchema engine;
it never persists conversions or applies frontmatter shell/interpolation
stages. Whole-value `{{ ... }}` and `$(...)` strings are ordinary YAML data,
not pending values that bypass validation. A numeric property holding either
string fails; a string property may accept it as text. Relative file-valued
properties are checked in the data document's context; references inside a
definition resolve in that definition's source context.

Diagnostic spans refer to the original buffer, including after soft
conversion or removing the fallback `$schema`. Missing-property problems
point to the enclosing mapping, and unknown-property problems to the key.
Keep typed causes and source origins for invalid declarations and imports;
a failure in a referenced definition is related information at that source,
with the reference in the consuming document marked as the failure site.
DMLS converts original byte spans to protocol positions, including Unicode
and CRLF. A parse failure cannot be reported as an unclaimed success.

#### Editor lifetime

Reuse DMLS's [workspace registry worker](../../docs/lsp/indexing.md)
and per-document contexts. Prepare and compile outside the request dispatch
thread, and reuse the result while its sources and context are unchanged.
Watch definitions, imported dependencies, and searched missing locations;
creation, deletion, rename, or an unsaved edit must refresh affected open
YAML documents. A changed or removed `kind` or `$schema` must clear stale
schema diagnostics, completion, and hover. Never reuse a registry from
another workspace or package scope.

Completion and hover describe the selected definition. In a claimed schema
kind, `$schema` and `types` are authored schema declarations: retain the
existing schema-authoring providers within those values while offering
ordinary data properties elsewhere. In fallback data YAML, exclude the
selection-only `$schema` key from unknown-property diagnostics and offer
schema-reference assistance there. Do not enable Markdown body expression
providers in YAML. Incomplete YAML uses the existing recovery tree for
completion; it must not run a second validator with different acceptance
rules or publish the same parse problem twice.

### 5. Shipped and filesystem schemas behave identically

A schema compiled into the binary behaves exactly as the same file on disk.
Shipped schemas reference each other by relative path only; a bare-name
reference inside a shipped schema is an error (it would depend on the
consumer's schema roots). Closing the embedded path's remaining refusals
(whole-file references, file-reference union arms, inline and catalog root
unions, `example(...)`) is a separate fix; this feature needs only what the
three shipped `yaml-schema` files use.

## Design rulings

The following decisions establish the intended behavior. The unresolved
choices in [Open Questions](#open-questions) must be settled before the
implementation plan is finalized; the grammar examples are illustrative.

### 1. Declaring the claimed `kind` (the "abstract" constraint) — closed

`yaml-schema.yaml` must require that a `yaml-schema`'s exported schema declare
`kind` as a `literal(...)` or `enum(...)`. That is a constraint on a **type
expression**, which SimplifiedSchema cannot state today: `schema` and
`type-definition` accept any declaration. The draft writes it as an
`enum(required) | literal(required)` union, by analogy with an abstract type
that a concrete definition (`kind: literal(schema; required)`) satisfies.

Two levels of `kind` must be kept apart:

```yaml
kind: yaml-schema                     # ① this document's own kind: a string
$schema:
    kind: literal(schema; required)   # ② the claimed kind: a type expression
```

The constraint applies to ②. In the current draft it sits beside `$schema` in
each arm, so it describes ①.

**Ruling (2026-10-31): narrow the `type-definition` meta-type.** A
declaration can be required to use given type constructors, so
`yaml-schema.yaml` states the rule itself (syntax illustrative; the concrete
spelling is settled in planning):

```yaml
# yaml-schema.yaml
$schema:
    kind: literal(yaml-schema; required)
    $schema: schema(required; property(kind, type-definition(literal | enum; required)))
```

`schema.yaml`'s `kind: literal(schema; required)` passes; `kind: string` fails
with an ordinary located diagnostic. This constraint must inspect the
**declaration**, not validate a sample `kind` value: the property must exist,
be required, and denote a nonempty finite set of nonempty strings. Reject
numeric, boolean, or null literals and declarations made optional by a
null-admitting union. No kind-name regular expression is introduced.

Every resolved root-union arm must be an object schema meeting that rule.
Collect all its string choices; repeating a choice within one definition
is harmless, while a different definition claiming it conflicts. Named
aliases and local file arms resolve during preparation through the existing
resolver, with the existing cycle diagnostics. The meta-type check itself
remains parse-only: it neither follows those references nor expands imports.
Preparation checks claims on the resolved declarations and reports an
unverifiable or invalid arm instead of ignoring it. A source declaration
hidden behind a reference must not fail merely because it was not inline.

The addition is general: any meta-schema can require a declaration’s form.
It lives in `simplified-schema`.

Rejected: abstract types as a language concept (larger, with no other
consumer yet), and keeping the rule in Rust (one more rule the definition
cannot express).

### 2. Bootstrapping `yaml-schema.yaml` — closed

`yaml-schema.yaml` is both the definition of `yaml-schema` documents and one
of them, so it claims `yaml-schema` and validates itself.

**Ruling (2026-10-31): no special case.** The registry reads every file's
claim from its ②, `yaml-schema.yaml` included. Recursion is broken by order:

1. every shipped definition is prepared by the standalone resolver, which
   parses and lowers it and never consults the registry;
2. the registry is built from their claims;
3. shipped definitions validate against the completed shipped registry;
4. filesystem candidates validate against the shipped `yaml-schema`
   definition, then resolve and verify their exported claims before they
   can be installed in a context registry;
5. documents validate against that prepared registry.

Do not trust an invalid candidate's prepared export as an active definition.
The claim and failure policy below determines how to retain its failure
without installing it.

An L1 test validates all three shipped files against the registry,
`yaml-schema.yaml` against itself. A repository file that also claims
`yaml-schema` is a duplicate claim like any other.

Rejected: a built-in Rust claim for `yaml-schema`, which would leave the
file's own `kind: literal(yaml-schema)` unchecked.

### 3. Coercion: soft or strict — closed

Frontmatter uses soft validation: a value that converts without changing its
meaning is accepted, and constraints are never repaired.

```yaml
# schema: { port: number, retries: number(integer) }
port: "8080"     # passes (means 8080)
retries: "2.5"   # error (not an integer)
```

A YAML file is never composed, so the conversion decides only whether the
file passes; a program reading it still sees the string `"8080"`.

**Ruling (2026-10-31): soft, as in frontmatter.** A schema means the same
thing wherever it is applied: a frontmatter `$schema`, a YAML `$schema`, or a
`yaml-schema` kind. Validation writes nothing back.

Rejected: strict (one schema would accept a value in Markdown and reject it
in YAML), and a strict opt-in (a setting nobody has asked for; it can be
added later without changing the default).

### 4. Rules the definitions cannot express — closed

The Rust parsers of schema and trigger files enforce rules the grammar cannot
state, so those files are checked twice and produce overlapping diagnostics.
The rules are of two classes.

Structural (shape):

```yaml
kind: schema
name: foo        # unknown key: passes (a root schema is open), Rust rejects
description:     # null: passes (an optional property accepts null), Rust rejects
types: [a, b]    # a list: passes (`schema` accepts lists), Rust rejects
```

Semantic (meaning): an invalid `$path` glob, an empty or vacuous `match`,
`min-match` count bounds, a category used in `types` that is not declared.

**Ruling (2026-10-31): grow the grammar for structural rules; keep semantic
rules in Rust.** Two general additions to `simplified-schema`:

- **closed objects**: a schema, root included, can forbid keys it does not
  declare (a user's `roster` kind rejects a mistyped `platfroms:`);
- **non-null optional properties**: a property may be absent but not `null`.

"`types` must be a mapping" is a narrowing of the `schema` meta-type, the same
mechanism as [ruling 1](#1-declaring-the-claimed-kind-the-abstract-constraint--closed).
With these, the shipped definitions state the whole shape and the Rust
parsers' structural checks are deleted. The semantic rules stay in Rust,
documented as checks beyond the definition, and reach `md` and DMLS the same
way:

1. **Located.** Every semantic check returns a typed problem with a source
   span taken from the YAML loader's position map, exactly as a definition
   problem does. (Today `SchemaError::TriggerMatch` carries only a message,
   so an invalid `$path` glob is published at the file or line start; it
   gains a span.)
2. **One report.** The library's kind-document entry point returns the
   definition's problems and the kind's semantic problems together; `md
   schema validate` and DMLS both project from that report, with the same
   codes, severities, and ranges. Structural failures stop semantic checks;
   successful structural validation produces only additional semantic
   problems, so nothing is deduplicated.
3. **Only Darkmatter's kinds have semantic checks.** `schema`,
   `schema-trigger`, and `yaml-schema` each have a check that runs after
   their definition passes. A user's `yaml-schema` kind has no Rust code;
   its definition is its whole contract.
4. **Completion and hover come from the definition only.** Semantic checks
   add diagnostics and nothing else.
5. **Severity is judged where the defect is.** In the file itself a semantic
   problem is an **error**, like a definition problem: `md schema validate`
   fails on a trigger with an invalid `$path` glob, and DMLS marks the glob.
   A Markdown document the broken trigger would have matched is not itself
   wrong, so it keeps today's skipped-trigger **warning**. (Rejected: a
   warning in both places, which passes a broken trigger; an error in both,
   which lets one bad trigger in `~/schemas` break unrelated documents.)

Rejected: growing the grammar for semantic rules too (custom validators in
the language, for rules only Darkmatter's own kinds need), and changing
nothing (keeps the double check and its deduplication code).

### 5. The shape of `yaml-schema.yaml` — closed

The first draft had four defects: its arms declared `schema` where
`yaml-schema` documents export `$schema`; its root union's string arms
(`- schema-led`) parse as file references, not type names; its `types:` was a
list where a mapping is required; and its `types-led` arm made `$schema`
optional, so such a document would claim no kind.

**Ruling (2026-10-31): one shape, `$schema` required.** A types-only library
is a `kind: schema` file, so the union and its named types go. The target
file, written to `darkmatter/schemas/yaml-schema.yaml`:

```yaml
kind: yaml-schema
description: |-
    The definition of a `kind: yaml-schema` document: a YAML file that defines
    a YAML document kind. Its `$schema` is the contract every document of the
    claimed kind must meet.
$schema:
    kind: literal(yaml-schema; required) -> the document kind that claims this definition
    description: string -> what the kind is for
    categories: string[] -> the category names `category(...)` constraints in `types` may use, each `Name` or `Name -> description`
    $schema: schema(required; property(kind, type-definition(literal | enum; required))) -> the contract for documents of the claimed kind; its `kind` declaration names the kinds claimed
    types: schema -> reusable named types for `$schema`; these names are not document properties
```

It has the shape of `schema-trigger.yaml` and passes its own definition
([ruling 2](#2-bootstrapping-yaml-schemayaml--closed)). The `$schema` line
uses [ruling 1](#1-declaring-the-claimed-kind-the-abstract-constraint--closed)'s
narrowing, and the closed-object and non-null markers of
[ruling 4](#4-rules-the-definitions-cannot-express--closed) are added once
their spelling is settled; until that grammar lands the file does not parse,
so the plan lands the grammar before anything reads this file. The example is
illustrative, not an accepted grammar fixture; the exact spelling remains
an [Open Question](#open-questions).

### 6. What of the earlier implementation is kept — closed

The earlier implementation (about 2,600 lines across the library kind
registry, the `md schema validate` YAML route, the DMLS YAML overlay, and
about 1,300 lines of tests) encodes the superseded design: a hard-coded
two-kind table, raw validation, and deduplication of definition diagnostics
against the Rust envelope's.

**Ruling (2026-10-31): revert it, then build from this spec.** One revert
commit undoes the earlier implementation's commits, and its uncommitted
review-cycle edits are discarded. Kept:

- the inline root-union support for embedded schemas in
  `simplified-schema` (a parity fix, independent of this design);
- `darkmatter/schemas/schema.yaml`, `schema-trigger.yaml`, and
  `yaml-schema.yaml` in this spec's shape.

The `{{{ … }}}` escapes of inline-code examples in the documentation are
redone as a separate docs fix on the reverted pages. The reverted commits stay
in history for reference.

The revert is the first implementation step. Before executing it, identify
the exact commits and inspect their changes; preserve unrelated extraction
and reference-navigation work. Discard only the uncommitted edits that
belong to the earlier implementation, file by file, never the working tree
wholesale. If a commit mixes retained and superseded work, reverse only the
superseded changes and record that departure from a whole-commit revert in
the implementation log.

Rejected: keeping and reshaping components (each brings its old tests and
assumptions), and deciding per component during planning (defers the
decision).

### 7. Language and resolver compatibility — closed

**Decision (2026-10-31): extend schema envelopes explicitly, without making
arbitrary kinded data into schema files.** Today the standalone parser in
`simplified-schema` admits an informal export or `kind: schema`, so changing
the three shipped definitions to `kind: yaml-schema` otherwise makes them
unloadable. Add `yaml-schema` to that parser's supported envelopes, with the
same metadata and named-type namespace as `schema` and a required export.
The lower-level parser admits its structure without consulting a kind
registry; claim verification is a separate preparation step.

A `types`-only `kind: schema` remains valid for imports
but invalid as a `$schema` target. An ordinary `kind: roster` data file is
still not a standalone schema target. A `schema-trigger` remains forbidden
as a `$schema` target, including when discovery skipped it as invalid.
Keep the existing Draft 2020-12 JSON target checks. Update target inspection,
resolver classification, DMLS reference assistance, and the shipped corpus
together; do not fix only the registry's private loader.

The new constraints are opt-in and obey these compatibility rules:

- Existing objects remain open. A closed object admits its explicitly
  declared properties and existing pattern-dictionary matches; other keys
  fail validation. Closure belongs to each object, including each root-union
  arm, and does not close nested objects implicitly. Lower and merge closure
  using the shared object/layering machinery so sibling declarations and
  resolved imports are visible before unknown keys are rejected.
- Existing optional properties still accept null. The new absent-but-non-null
  marker distinguishes a missing key from an explicit null and rejects null
  **before** soft conversion. Combining it with a type that explicitly admits
  null is a declaration error; it must not silently override the existing
  rule that null-admitting types are optional. Apply it to optional envelope
  metadata, `types`, and trigger payloads where the current parsers reject
  explicit null. Leave ordinary user schemas unchanged unless they opt in.
- A mapping-only narrowing of `schema` checks a declaration's **carrier**
  (its authored YAML representation), not the kind of value that its exported
  schema accepts. `types` must be a mapping of valid named definitions;
  `$schema` may still be a mapping, reference, or root union. Constructor
  narrowing of `type-definition` examines the type it declares instead.
- Parsing, canonical serialization, JSON Schema export, validation, type
  queries, subtype checks, layering, and DMLS completion/hover must agree on
  the new constraints. Preserve them through import and export round trips;
  expose any custom JSON Schema vocabulary through the existing mechanism.
  Do not implement host-only validation rules with no library representation.

These are intentional language additions in `simplified-schema`, not new
Darkmatter-only spellings. Settle their exact authoring syntax under
[Open Questions](#open-questions) before replacing the shipped artifacts.

## Open Questions

### How should a checkout's shipped definitions avoid conflicting with the binary?

In this repository, the disk `darkmatter/schemas/` files are also compiled
into the binary. Searching the package-area root therefore finds a second
claim for each shipped kind. The current duplicate rule would make the
project's own YAML files fail. Filename shadowing would hide genuine
conflicts, so this needs an explicit source-identity policy.

- **Explicit source-checkout registration (recommended).** Tests and a
  deliberately configured source-development host register the exact shipped
  source identities as replacements for the embedded versions. Pros: disk
  edits can be tested, unrelated repositories cannot override built-ins by
  filename, and normal consumers keep the duplicate rule. Cons: checkout
  tooling and corpus fixtures must provide this registration; the ordinary
  CLI needs a documented way to use it if live source editing is desired.
- **Always reject disk claims of shipped kinds.** Pros: smallest runtime
  policy and no override ambiguity. Cons: validation in this checkout fails
  unless tools exclude its shipped directory, and source development needs
  isolated contexts to test definitions.
- **Collapse byte-identical copies automatically.** Pros: an unchanged
  checkout works without configuration. Cons: editing a definition immediately
  turns it into a conflict, and byte equality treats unrelated copies as the
  same identity; it does not solve live definition authoring.

Recommend explicit registration because it keeps the rule against accidental
built-in replacement while giving definition tests a controlled source path.
The author must decide whether this is test-only or a supported development
setting before implementation; no new command flag is prescribed here.

### What happens when a discovered definition cannot be prepared?

A malformed `yaml-schema` may reveal a kind claim before an import fails,
or may be too broken to reveal any claim. Silently skipping it can make a
previously checked document appear to have no schema. Failing all documents
would let one broken file in `~/schemas` disrupt unrelated work. Existing
trigger warning recovery alone does not settle this new kind-selection case.

- **Keep healthy definitions, reserve failed claims, and expose incomplete
  discovery (recommended).** Pros: a document naming a failed or ambiguous
  claim gets a schema-selection error, healthy kinds continue to work, and
  unrelated Markdown keeps its current behavior. Cons: needs an explicit
  incomplete-registry state; when a failed definition reveals no claim, an
  otherwise unknown kind must report that selection is unavailable rather
  than return unvalidated success. Report the discovery cause once, and
  surface the definition's own defect as an error when it is opened.
- **Fail registry preparation for the whole context.** Pros: simplest
  correctness rule and no silent loss of validation. Cons: a broken unrelated
  definition blocks every YAML selection that needs that registry, including
  healthy kinds and fallback selection.
- **Skip invalid definitions with a warning, as triggers do.** Pros: familiar
  recovery and low disruption. Cons: a misspelled or unavailable claim can
  lose validation while still exiting successfully; the warning must carry
  that limitation and this weakens the feature's contract.

Recommend the first option because it preserves useful validation while
making uncertainty observable. Infrastructure failures such as unreadable
roots remain preparation errors under every option, never empty registries.
Duplicate claims remain errors naming both sources; never select the first
claim by search order. DMLS must not publish a stale successful selection as
current after any failure.

### What is the concrete syntax for the new language constraints?

The spec needs closure at a schema root as well as nested objects,
absent-but-non-null properties, and narrowing of schema/type declarations.
The current `property(kind, type-definition(...))` examples do not define how
to distinguish requiring the declaration from requiring the described `kind`,
or how to carry a deferred check through a referenced root.

- **Extend existing constraint syntax and the shared declaration model
  (recommended).** Pros: one grammar and familiar property syntax; the
  generic library can validate and export the constraints. Cons: root objects
  need an explicit, unambiguous carrier for constraints; the exact spelling,
  deferred-reference behavior, and merge precedence must be written down.
- **Add reserved metadata keys inside schema mappings.** Pros: root closure
  is easy to express next to properties. Cons: reserves property names,
  complicates literal-key escaping, and changes schema-map classification.
- **Keep these checks only in Darkmatter Rust code.** Pros: no language
  additions and a smaller first implementation. Cons: abandons the accepted
  goal that definitions express their structural contract and prevents reuse
  by standalone library consumers.

Recommend extending constraints because it supports the intended reusable
language without reserving ordinary data keys. Before planning, replace the
illustrative examples with parseable root/nested/union examples and specify
how closure layers with existing objects, how non-null presence serializes,
and how unresolved references defer checks without making passive validation
perform I/O. This is a design choice to review, not permission to invent a
syntax during implementation.

## Out of scope

- Escaping `{{ … }}` examples in inline code across the documentation (a
  docs fix; inline code spans are interpolated by design).
- The embedded-schema refusals beyond what the shipped `yaml-schema` files use
  ([Design 5](#5-shipped-and-filesystem-schemas-behave-identically)).
- JSON and TOML documents.

## Acceptance

- `darkmatter/schemas/yaml-schema.yaml` exists; `schema.yaml`,
  `schema-trigger.yaml`, and `yaml-schema.yaml` are `kind: yaml-schema`, are
  shipped, and each passes its claiming definition (`yaml-schema.yaml`
  itself).
- A `yaml-schema` in any of the five schema roots claims its kind for
  documents in that context; two claims of one kind are an error naming both
  files.
- A YAML document with a claimed `kind` is validated by that definition only;
  one with no claimed `kind` and a root `$schema` is validated by that schema;
  one with neither is not validated. `messenger/docs/platforms.yaml` validates
  without `---` fences.
- Both routes give `md schema validate` and DMLS diagnostics ranged on the
  offending key or value, and DMLS completion and hover in the file.
- Editing a shipped `yaml-schema` file changes validation on the next build
  with no Rust change; an L1 test proves it against a modified copy.
- Every shipped and repository schema, trigger, and `yaml-schema` document
  passes its definition (the shipped-schema corpus test). Use the decided
  checkout-source policy so the corpus does not manufacture duplicate claims.
- Constructor narrowing rejects optional/non-string kind declarations and
  accepts valid finite claims across root unions and imports.
- Opt-in closure and non-null optional properties work at roots and nested
  objects, preserve current defaults, and survive canonical serialization,
  JSON export, imports, and layering. Test mapping-only `types` independently
  from the schema of data values.
- Shared library and normal `md schema validate` tests cover both selection
  routes, route precedence, unknown kinds, bad schema references, duplicate
  claims, and the chosen incomplete-discovery policy. Cover parse/load errors,
  null versus missing values, conversion failures, unvalidated outcomes,
  assignment rejection, baseline options, and mixed-batch exit precedence.
- DMLS tests cover unsaved definition edits and dependency creation/deletion,
  scope isolation, route changes clearing stale results, schema authoring
  inside exports, and Unicode/CRLF ranges. Exercise diagnostics, completion,
  and hover through normal protocol requests; no focused windows are needed.
- Passive corpus tests parse the shipped artifacts, and end-to-end command
  tests use real definitions. These are L1 tests under the monorepo toolkit;
  use the package `just test` recipes and nextest. No new terminal/browser
  test tier or multi-host performance spike is required. Reuse prepared
  registries and compiled validators rather than scanning or compiling on
  each editor request.

## Documentation

- `darkmatter/docs/topics/schemas/definition.md`: how a YAML document selects
  its schema, with the flow above.
- `darkmatter/docs/topics/schemas/authoring-schemas.md`: writing a
  `yaml-schema`, shown from the shipped definitions.
- `darkmatter/docs/topics/schemas/dmls-schema-support.md`: diagnostics,
  completion, and hover in YAML documents.
- `docs/kind-documents.md`: `yaml-schema` as the way to formalize a YAML
  document kind.
- SimplifiedSchema's language, schema-document, validation, and custom
  vocabulary pages: the new constraints and `yaml-schema` envelope, with
  complete parseable examples after the syntax decision.
- Darkmatter's CLI validation reference and DMLS workspace-indexing and
  configuration pages: YAML selection, discovery controls, unvalidated
  outcomes, exit codes, baseline options, and refresh behavior.
- `messenger/docs/platforms.yaml`: remove the obsolete fence-workaround
  comment when the whole-file route lands and verify the roster through it.
- Darkmatter and SimplifiedSchema agent skills: envelope recognition,
  ownership boundaries, and registry lifecycle. Keep README changes to a
  concise overview/link if public behavior needs announcing; topic pages
  remain authoritative.
