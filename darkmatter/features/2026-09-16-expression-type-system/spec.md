---
created: 2026-09-16
status: draft
clarified: true
needs_rulings: false
clarified_by: codex/gpt-6-astra
references:
  spikes/function-declarations-findings.md: >-
    Findings from the completed passive function-declaration representation
    prototype, including parser evidence, compatibility gaps, and limits.
  spikes/function-declarations.yaml: >-
    Proposed function declarations illustrating separate call presence,
    schema value types, and shared behavior metadata; syntax is not ratified.
supersedes:
  - ../2026-07-15-type-system/spec.md
  - ../2026-07-22-explicit-null/spec.md
---

# Expression Type System

Chartered by the dasherized-identifiers feature's ratified "ship now +
charter" split ([2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md),
Resolved Decision 16). This clarified document combines a phased charter, confirmed requirements,
and implementation-planning details; it is ready for planning. The
2026-09-16 clarification session settled the three-way disposition of the
earlier drafts (see [Lineage](#lineage)) and re-homed the July draft's
design into the [declarations annex](declarations-design.md), which now carries the Phase C/E detail.

## Status

Clarification and risk review are complete; no human rulings remain. The
phases below are a charter, not an implementation plan;
Phase C/E design detail lives in the
[declarations annex](declarations-design.md). Splitting into per-increment
specs is deferred until the first increment is scheduled.

## Summary & Motivation

Darkmatter's expression language has runtime values but no static type
vocabulary for expressions:

- `SimplifiedType`
  ([`types.rs`](../../lib/src/markdown/schemas/simplified/types.rs)) has `Any`
  but no `unknown` and no `null`.
- The function catalog
  ([`docs/schemas/expression-functions.yaml`](../../docs/schemas/expression-functions.yaml))
  plus `ExpressionFunctionDescriptor`/`ParamType`
  ([`catalog/mod.rs`](../../lib/src/markdown/compose/expression/catalog/mod.rs))
  is a parallel ad-hoc typed system that shares only the type vocabulary.
- The expression parser is untyped.

Consequences:

- Absence intent can only be suppressed through the interim catalog-driven
  rule (dasherized Resolved Decision 9) instead of parameter nullability.
- Parameter type mistakes are invisible until runtime.
- Optional-but-typed properties have no honest type: an optional `A` is
  really `A | null`, and the vocabulary cannot say so.

## Scope (phased, one spec)

- **Phase A — `unknown` and `null` types.** Add both to `SimplifiedType`.
  Union machinery already exists at property and root level; the `null`
  keyword is the missing piece. Its specced core, absorbed from the
  superseded
  [2026-07-22-explicit-null](../2026-07-22-explicit-null/spec.md) draft: an
  explicit `null` property type, so an arm can declare a property as
  deliberately absent —

  ```yaml
  $schema:
    - spec: file(required)
      review: null
    - review: file(required)
      spec: null
  ```

  — the XOR authoring idiom: a caller passes a spec file or a review file,
  not both. That idiom is a Phase A test case. Rename commitment: when this
  lands, the dasherized spec's "type is `any` by default" language becomes
  `unknown`. `unknown` is the canonical expression-facing name; `any` remains
  an equivalent schema alias. This does not introduce stricter checks for
  unknown values or remove existing runtime checks.
- **Phase B — function schemas in SimplifiedSchema.** Express the
  expression-engine function catalog's signatures in SimplifiedSchema,
  unifying the ad-hoc `ParamType` system; preserve the parity tests. Built-in
  and caller-supplied functions share the full declaration contract below.
- **Phase C — type-aware parsing/evaluation.** Untyped variables default to
  `unknown`; optional-but-typed properties carry `A | null`, with null as
  YAML's representation of undefined, under the translation model below.
  The declaration-layer design that feeds this phase — schema-derived
  variable declarations, strict root validation, host retention — is in
  the [declarations annex](declarations-design.md).
- **Phase D — flow-sensitive narrowing.** Inside conditional blocks
  (`::block when="x"` narrows `file | null` to `file`), in ternary
  truthiness branches (`x ? frontmatter(x, 'foo') : null` narrows `x` in
  the true branch), and in `&&`-guarded calls
  (`file_exists(x) && frontmatter(x, 'foo')`). The confirmed file-predicate
  contracts below replace today's narrower catalog signature. A true
  `is_file_type` result establishes suitability for a file parameter within
  the guarded branch, without changing the stored value.
- **Phase E — diagnostics.** Generalize the dasherized spec's interim
  suppression to "any parameter whose type admits null"; add parameter type
  diagnostics — a known-null passed to a non-null parameter is an error, a
  known concrete union including null produces a warning. This does not
  impose a blanket warning on permissive `unknown`; the opt-in `file_exists`
  warning is specified separately. Check both conditional branches; by
  default, stop execution only when it reaches an invalid call, before
  invocation. The explicit CLI blocking policy below can stop it earlier.
  The advisory `file_exists` exception is specified below. Landing this phase is the
  explicit retirement trigger for dasherized Decision 9's interim rule.
  The declaration-layer diagnostics distinctions are specified in the
  [declarations annex](declarations-design.md).

Phases A and C share one ratified translation model, recorded in full in
the [declarations annex](declarations-design.md): optionality is a
**default constraint** (`optional` unless `required`), exactly like
`max-length: 5` refines `string`. A schema declaring
`string(max-length: 5; required)` gives the variable a runtime type that is
a string constrained to five characters — calling it merely "string" is
incomplete. Likewise a schema declaring `string` — optional by default —
gives the variable the effective runtime type `string | null`; calling it
merely "string" is incomplete in the same way. In general, an optional
property with declared type A has effective runtime type `A | null`, and
`null` is YAML's representation of undefined. Required-by-default was
considered and rejected: it would make `string(optional)` → `string | null`
the more intuitive reading, but optional-by-default was chosen because far
fewer properties tend to be required.

Also in scope: the CLI policy consolidation confirmed below and
machine-readable warning output (`--output json` currently emits the document
only). The consolidation replaces the earlier proposal for a separate
`--deny-warnings` flag; warning selection belongs under `--deny`.

## Confirmed Clarifications — 2026-09-17

### Unknown and null are distinct types

An unschematized variable has type `unknown`: the union of all types,
including `null`. `any` is an accepted equivalent schema alias. Both
spellings must be accepted by the schema grammar. This permissive type does
not require new static proof before an expression may use its value;
existing runtime operation checks still apply. A name absent from every
visibility layer can still be an unknown-root error on a strict surface:
type `unknown` does not declare every possible variable name.

`null` accepts only a null value or an omitted property. The `required`
constraint does not contradict that type:

```yaml
$schema:
  review: file(required)
  foobar: null(required)
```

Both declarations are legal. `review` must be a valid filepath; `foobar`
must be explicitly null or omitted. A non-null `foobar` fails validation.

```yaml
$schema:
  review: unknown
  spec: unknown(required)
```

Both declarations have the same type and accept the same values, including
null and omission. `required` has no effect on `unknown`, because this type
already admits all types, including null. `null(required)` likewise accepts
both explicit null and omission. These decisions concern SimplifiedSchema;
they do not redefine raw JSON Schema's `required` key-presence rule.

### Root unions are inclusive unless the author excludes overlap

```yaml
$schema:
  - schema: file(required)
  - review: file(required)
```

This schema accepts a valid `schema` file, a valid `review` file, or both.
A property not declared by an arm has type `unknown` in that arm. Multiple
arms may legitimately match; validation succeeds without requiring one
unique match, and expressions need not choose between matching arms.
Supplying neither file satisfies no arm and fails validation.

An author can exclude overlap explicitly:

```yaml
$schema:
  - schema: file(required)
    review: null
  - review: file(required)
    schema: null
```

This schema accepts exactly one valid file. The opposite property may be
omitted or explicitly null. Two non-null file values fail, as does supplying
neither file. This decision does not change existing coercion or runtime
arm-selection policies and does not settle correlated narrowing between
properties. The tagged `id`/`name` example discussed during clarification
illustrates alternatives; it does not authorize new literal-value syntax.

### Phase A acceptance cases

Validate the following through the schema parser and validator, and verify
expression type projection when Phase C lands:

| Declaration or schema | Input | Expected result |
|---|---|---|
| `file(required)` | Valid filepath | Accepted |
| `file(required)` | Null or omitted, at final validation | Rejected |
| `null` or `null(required)` | Null or omitted | Accepted; type `null` |
| `null` or `null(required)` | Any non-null value | Rejected |
| `unknown`, `any`, or either with `(required)` | Any value, null, or omitted | Accepted; type `unknown` |
| Inclusive union above | Either valid file or both valid files | Accepted |
| Inclusive union above | Neither file | Rejected |
| Exclusive union above | One valid file; opposite null or omitted | Accepted |
| Exclusive union above | Both files or neither file | Rejected |

Existing runtime errors for invalid operations on unknown-typed values must
remain errors; accepting `unknown` in the grammar must not bypass those
checks. The completed representation prototype below provides limited
parser evidence; it does not replace these implementation acceptance cases.

### Conditional diagnostics and execution

Darkmatter must type-check both branches of a conditional expression,
including a branch that execution skips. Reporting a type diagnostic does
not by itself stop execution before the expression runs. By default, a type error causes execution to fail
only if it reaches the invalid call, and must fail before invoking that
function. The explicit CLI blocking policy below is an exception to this
default. Ordinary runtime failures, including I/O, access, and parsing
failures, remain possible.

For example, with this schema:

```yaml
$schema:
  review: null(required)
```

- `true ? 'ok' : frontmatter(review, 'title')` returns `ok` and reports the
  invalid null argument in the skipped branch.
- `false ? 'ok' : frontmatter(review, 'title')` reports the same problem and
  fails before invoking Darkmatter's `frontmatter` function, which reads
  frontmatter from a file.

DMLS must report these type diagnostics passively: analysis must not read
the target file or execute expression functions. Existing reachable-root
validation and short-circuit execution remain separate from checking types
in both branches. Warning promotion follows the confirmed CLI blocking
policy below.

### File predicates and ownership

The Darkmatter expression language adds
`is_file_type(value: unknown) -> boolean`. It checks whether a value is
already a file value or is a string structurally convertible to a supported
file reference. It does not test whether the referenced file exists.

The structural check accepts all supported `biscuit-file` reference forms:
local relative and absolute paths, Windows paths, project-relative
references, vault references, recursive references, and HTTP(S) references.
Grammar acceptance is independent of the host operating system. Empty
strings and malformed reference syntax return `false`. A typed file value
returns `true`; numeric `42` returns `false`, while string `'42'` and string
`'hello world'` are valid possible filenames and return `true`.

This check must not resolve references, inspect the current directory,
read environment variables or configuration, search directories, access
files, or use the network. Returning `true` guarantees neither existence
nor readability. Testing conversion must not mutate the input or document
state. No separate `is_file` alias is authorized.

The `biscuit-file` library owns the passive structural validity rules for
its `FileReference` representation; Darkmatter owns the expression function
and DMLS consumes its type information. Merely constructing a
`FileReference` is not sufficient validation: malformed HTTP(S) references
such as `https://` must fail the predicate even if the current constructor
classifies them as URL references. Any missing passive structural checks
belong in `biscuit-file`, rather than a second grammar in Darkmatter.

Darkmatter's existing existence predicate becomes
`file_exists(value: string | file | null) -> boolean`. Null returns `false`.
For a value of a known type outside this signature, DMLS emits a warning
explaining that the result is always false; execution returns `false`
without a type error. This is an explicit advisory exception to general
invalid-call handling. A warning for an `unknown` argument is opt-in and disabled by default;
this setting does not change execution. The known-unsupported-type warning
remains enabled by default.

Malformed strings, including `https://` and the empty string, return `false`
before filesystem or network access. Reuse the passive validity rules owned
by `biscuit-file` and used by `is_file_type`; do not introduce a second
reference grammar. DMLS warns that the result is always false when invalid
contents are statically provable, such as a known malformed literal. A
value's ordinary `string` type alone does not justify that warning.

This explicitly changes the current behavior in which malformed URL-like
strings can fall back to a local-reference path. Preserve valid existing
forms and normalizations; the compatibility corpus must include previously
supported forms such as `file://` where the existing normalization supports
them. This requirement does not redefine outcomes for valid references:
local parse/resolution failures remain false; a valid HTTP(S) reference in a
local-only context still errors because remote evaluation is unavailable;
with a remote runtime, denied hosts or failed fetches remain false and a
successful fetch remains true. Structural rejection must precede those
existing resolution paths.

### Guarded file arguments

A true result from `is_file_type` establishes that its argument is suitable
for a file parameter within the guarded branch, including when its original
type was `unknown` or `string`:

```yaml
$schema:
  source: unknown
source: './docs/spec.md'
```

```text
is_file_type(source) && frontmatter(source, 'title')
```

The guarded `frontmatter` call has a suitable argument type. The guard does
not rewrite the stored string, prove file existence, or grant filesystem or
network access. The later file read remains subject to ordinary resolution,
access policy, and runtime failures. An unguarded `unknown` argument remains
permitted; the guard improves static information rather than granting
permission to use it.

For `file_exists(source)` within this guard, the opt-in unknown-input
warning is unnecessary because the guard removes that uncertainty.
Darkmatter owns the diagnostic meaning and DMLS exposes the opt-in setting;
its exact configuration surface remains a planning detail. The CLI policy
verbs and diagnostic boundaries are defined below.

### One function contract for built-ins and host extensions

Darkmatter's function declarations must describe parameter and result
types, supported conversions, advisory handling of unsupported inputs, and
any facts established when the function returns true. Built-ins and
caller-supplied functions use the same contract. A boolean result alone
does not imply a type guard; such facts must be explicitly declared.

Darkmatter owns interpretation and type checking. A host such as Claudine
supplies a function implementation and its matching declaration, and is
responsible for honoring that contract. DMLS uses the same declarations
without invoking functions. Custom host-provided analysis callbacks are
not part of this contract. Exact declaration syntax and Rust APIs are
implementation-planning decisions.

For example, a hypothetical host function `is_project_file` may declare
that a true result establishes suitability for a file parameter. With that
declaration, Darkmatter must understand:

```text
is_project_file(source) && frontmatter(source, 'title')
```

This illustrates the extension contract; it does not require adding an
`is_project_file` built-in.

### Declaration visibility preserves existing schema normalization

Declaration projection, root validation, type checking, and expression
evaluation must not insert or replace properties merely to provide names
or types. This boundary preserves existing composition-stage schema
normalization: after initial frontmatter interpolation and before coercion,
an absent optional, no-default top-level property is materialized as null
only when its winning declaration comes from the document's inline or
referenced SimplifiedSchema. Baseline and trigger properties, raw JSON
Schema, root unions, nested properties, required properties, and defaulted
properties remain excluded. Present values remain unchanged, repeated
schema passes remain idempotent, and validation-only APIs remain passive.
Hosts must not add placeholder properties to work around missing declaration
visibility. No expansion or removal of existing null insertion is authorized.

### Conditional and file-predicate acceptance cases

| Scenario | Expected result |
|---|---|
| Invalid call in the skipped branch of the conditional above | Diagnostic; expression returns `ok`; function not invoked |
| Invalid call in the reached branch above | Diagnostic; execution fails before invocation |
| DMLS analyzes either conditional | Same type problem reported; no expression function or target-file read |
| `is_file_type` receives a typed file value | `true` |
| `is_file_type` receives `./docs/spec.md`, `/tmp/spec.md`, or `\Users\ken\data\foobar.json` as a string | `true` on every supported host |
| `is_file_type` receives `@docs/spec.md`, `vault:notes/spec.md`, or `%@spec.md` as a string | `true` without discovery or resolution |
| `is_file_type` receives `https://example.com/spec.md` or `http://example.com/spec.md` as a string | `true` without network access |
| `is_file_type` receives string `42` or `hello world` | `true` |
| `is_file_type` receives numeric `42`, an empty string, or malformed reference `https://` | `false` |
| `file_exists` receives null | `false` |
| `file_exists` receives known numeric `42` | DMLS warning that the result is always false; runtime returns `false` |
| `file_exists` receives known malformed `https://` or an empty string | Always-false DMLS warning; runtime `false` before filesystem/network access |
| `file_exists` receives a dynamic string with contents not statically known | No always-false warning solely because its type is `string`; runtime checks structure |
| Existing valid reference forms and normalizations are exercised | Existing resolution, access, and remote-runtime outcomes preserved |
| `file_exists` receives an `unknown`-typed argument with default settings | No unknown-input warning; existing runtime behavior |
| Same argument with the DMLS unknown-input warning enabled | Warning; unchanged execution |
| `is_file_type(source)` is true before a guarded file call | Argument is suitable for a file parameter; stored value unchanged |
| A host predicate declares the same true-result fact | Same checking as the built-in guard; DMLS does not invoke the host function |
| Declaration projection, root validation, type checking, or evaluation runs | No property inserted or replaced merely to supply names or types |
| Existing schema-stage normalization runs | Eligible insertion and all existing exclusions preserved; present values unchanged |

Verify passive behavior using access instrumentation or failing access
stubs, not merely by checking return values. Cross-platform tests must use
the same reference corpus. The representation prototype below did not
execute these predicates; their behavior still requires implementation tests.

## CLI Policy Consolidation — Confirmed 2026-09-18

### Scope and verb meanings

Consolidate the growing set of policy flags on `md compose` and `md render`
and all other commands exposing affected controls under three verbs:

| Option | Meaning |
|---|---|
| `--allow <id\|all>` | Permit a particular recovery or explicitly addressed access |
| `--deny <id\|all>` | Make selected diagnostics blocking |
| `--disable <id>` | Stop a feature from running |

Existing `--allow-*` options move under `--allow`. Existing
`--no-baseline-schema` and `--no-trigger-schemas` behavior moves under
`--disable`. Selection of individual warnings uses `--deny`; there is no
additional standalone `--deny-warnings` flag. `--allow` does not enable an
optional diagnostic, and this design does not introduce a fourth verb.

Identifiers such as `missing-hyperlinks`, `host=example.com`,
`baseline-schema`, and `trigger-schemas` are illustrative. The complete
identifier registry and parameter spelling remain to be specified. The
`darkmatter-cli` package owns argument parsing and migration; the Darkmatter
library continues to own composition behavior and diagnostic meaning.

Migrate all affected commands together: `md compose`, explicit `md render`,
default `md FILE` rendering, `md clean`, and `md schema validate`, wherever
the existing controls apply. Each command accepts only identifiers meaningful
for that command. This consolidation does not add composition behavior to
rendering. Public Rust API compatibility is a separate planning concern.

### Broad allowance preserves recovery behavior without granting access

`--allow all` permits all recoverable, non-access allowances covered by this
consolidation: missing references, invalid frontmatter, context overrides,
duplicate assignments, and shell-timeout recovery. Each keeps its existing
individual recovery behavior, including an empty string for a recovered
shell timeout. This is broader than the existing
`--allow-any-missing-reference`, which covers only the three missing-reference
categories.

`--allow all` does not grant access to unnamed remote hosts or independently
authorize shell execution. Remote destinations must be allowed explicitly;
the spelling for a destination-specific allowance remains open. Allowing
recovery from a shell timeout does not authorize starting the shell command.

### Specific rules override broad rules regardless of order

For recovery and diagnostic policies, a specific identifier overrides
`all`, independent of argument order. Thus a selected denial can override
`--allow all`, and a selected allowance can override `--deny all`.

Equally specific opposing rules are usage errors: explicit `--allow <id>`
and `--deny <id>` for the same identifier conflict, as do `--allow all` and
`--deny all`. Resolve and validate policy before composition or rendering
begins. These precedence rules do not imply host revocation semantics or
feature enable/disable precedence; those are distinct from recovery and
diagnostic policy. An allowance does not bypass mandatory runtime argument
checks or grant access beyond an explicitly addressed access permission;
changing diagnostic blocking does not make an invalid runtime call valid.

### Blocking diagnostics and stopping behavior

`--deny all` makes every reported, enabled warning and error blocking,
including a type error reported in a skipped conditional branch.
Informational notices are not blocking. Optional diagnostics remain disabled
unless separately enabled, and a specific allowance still overrides the
broad denial. This policy never executes a skipped branch.

Without an applicable CLI denial, the earlier conditional examples retain
their default behavior: a skipped invalid call can produce a diagnostic
while the chosen valid branch succeeds. With `--deny all`, that reported
type error prevents a successful command outcome.

Stop when a blocking diagnostic becomes known; do not perform subsequent
work or effects. Use available passive analysis before effectful steps when
it can establish the problem. This does not promise discovery of every
problem before every effect, reorder the composition pipeline, or roll back
effects already completed. A problem discovered during execution stops
subsequent work. These CLI rules do not override a host such as Claudine's
workflow decisions.

On failure, withhold the composed document from standard output. Diagnostics
may still be emitted, including an explicitly requested machine-readable
failure report. No partial composed document is emitted as successful output.

### Explicit diagnostic report output

Add a report format to the existing `--output` selector, for example
`--output report-json`; its exact identifier is a planning choice. Preserve
existing `--output json` as document-only output, and preserve ordinary human
output. No separate top-level diagnostic-output flag is needed.

The report format emits one JSON result containing the command outcome and
diagnostics. On success it also contains the document. On failure it contains
no document content: the document field is null or omitted, with that exact
schema choice reserved for planning, and the command exits nonzero. An
explicitly requested failure report may appear on standard output without
violating the prohibition on failed composed-document output.

Planning must define the report schema, including diagnostic codes,
severity, source positions, and relevant type information, with consistent
handling when information is unavailable. Diagnostic content follows the privacy boundary below.

### Diagnostic content and privacy

The Darkmatter library owns structured diagnostic codes, severity, messages,
source positions, and relevant expected/actual type information. CLI reports
and DMLS consume the same fields. Do not add evaluated arguments,
environment values, or file contents to enrich diagnostics, and do not
perform extra evaluation or I/O to obtain diagnostic details.

Existing messages and authored source may already contain sensitive text;
this is not a guarantee that diagnostics are secret-free. No debug-value
option or general redaction subsystem is introduced. A successful report's
document is intentionally included and is not redacted by this policy.

### Remove migrated legacy spellings in the same change

For commands included in the migration, remove their old flag spellings
without compatibility aliases. Update affected repository scripts,
examples, documentation, tests, and shell completions in the same change.
The removed spellings must produce a usage error. This is a confirmed
breaking CLI migration, not a deprecation period.

### CLI acceptance cases

In this table, `missing-hyperlinks` is an illustrative identifier for the
same selected recovery policy in each case; the registry spelling is not
yet final.

| Scenario | Expected result |
|---|---|
| `--allow all --deny missing-hyperlinks`, in either order | Selected denial wins; other broad allowances remain |
| `--deny all --allow missing-hyperlinks`, in either order | Selected allowance wins; other broad denials remain |
| `--allow missing-hyperlinks --deny missing-hyperlinks`, in either order | Usage error before composition/rendering effects |
| `--allow all --deny all`, in either order | Usage error before composition/rendering effects |
| An individual migrated allowance handles an existing failure | Same recovery behavior and output as the legacy allowance |
| `--allow all` handles a shell timeout after separately authorized execution | Existing empty-string recovery |
| `--allow all` is supplied without explicit remote host access | No unnamed remote host permission is granted |
| `--allow all` is supplied without shell authorization | No independent shell authorization is granted |
| A migrated command receives a removed legacy flag | Usage error; no compatibility alias |
| Repository command examples and completions target a migrated command | New verb spelling only |
| Compose, explicit/default render, clean, and schema validation expose an affected control | New spelling in the same migration; only command-meaningful identifiers |
| A skipped branch reports a type error under `--deny all` | Command fails; skipped branch is not executed |
| Same skipped error without an applicable denial | Default valid-branch success and diagnostic remain |
| An informational notice is reported under `--deny all` | Notice alone does not block success |
| An optional diagnostic is disabled under `--deny all` | It remains disabled |
| Passive analysis establishes a blocking diagnostic before an effectful step | Stop before that step; no subsequent work/effects |
| A blocking diagnostic becomes known after an earlier effect | Stop subsequent work; no rollback of the earlier effect |
| A command fails during composition | No composed document on standard output |
| Explicit report output succeeds | One JSON result with outcome, diagnostics, and document |
| Explicit report output fails | Nonzero exit; one JSON result with outcome and diagnostics; no document content |
| Existing `--output json` or human output is selected | Existing document-only JSON or human format preserved |
| CLI and DMLS report the same type problem | Shared code, severity, message, source position, and expected/actual type information where available |
| A diagnostic is enriched for reporting | No added evaluated arguments, environment values, file contents, evaluation, or I/O |

The migration of `--strict-style` still requires an explicit identifier
mapping, while preserving the confirmed distinction between informational
notices and blocking warnings/errors. Its informational notices must not
silently become errors.

## Interface Seam

Darkmatter owns parsing, evaluation, and type safety end-to-end. Callers
(e.g. claudine) ask Darkmatter to evaluate and extend the language by passing
in functions plus type definitions (SimplifiedSchema) — never bespoke grammar
or evaluation. Verified today: claudine already calls Darkmatter's
parser/evaluator and supplies variables via `EvaluationLookup`; the residual
coupling is read-only AST walks in claudine
([`composition/looping/config.rs`](../../../claudine/lib/src/composition/looping/config.rs),
[`composition/lifecycle/validate.rs`](../../../claudine/lib/src/composition/lifecycle/validate.rs))
that this work should formalize or replace. The
`EvaluationLookup::is_known_variable_root` hook (default `true`) remains the
sanctioned third-party warning opt-in.

## Lineage

Resolved 2026-09-16, as an **annex-absorb** consolidation (recorded as
Resolved Decision 17 in
[2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md));
this spec is the primary. Both earlier drafts are superseded **in place**
— they remain where they are as the historical record, each carrying
supersession frontmatter and a pointer:

- [2026-07-15-type-system](../2026-07-15-type-system/spec.md) — the large
  July draft (schema-derived variable declarations, strict root
  validation). Its design body is re-homed, with dated corrections, into
  the [declarations annex](declarations-design.md) as Phase C/E detail.
  One stance is consciously overturned in the re-homing: the July draft
  held that schema nullability and optional absence contribute no `null`
  type to the union; the ratified translation model (above) gives an
  optional `A` the effective runtime type `A | null`. Its stale ten-file
  `inputs` list and its "does not authorize changes" gating language were
  dropped rather than carried.
- [2026-07-22-explicit-null](../2026-07-22-explicit-null/spec.md) — the
  small null-type draft. Its specced core (the `null` keyword) and its XOR
  authoring idiom are folded into Phase A above, the idiom as a Phase A
  test case; nothing else was carried because nothing else was specified.

## Representation Prototype and Planning Outcome

The completed [function-declaration prototype](spikes/function-declarations-findings.md)
explored one shared representation for built-ins and host extensions. Its
[candidate declarations](spikes/function-declarations.yaml) separate ordered
call arguments and their presence requirements from schema value definitions
and shared behavior metadata. This direction expresses conversions,
advisory outcomes, and true-result facts without function-name-specific
analysis rules. Exact field names and policy identifiers remain proposals,
not syntax ratified by parser acceptance.

A required function argument must be supplied before its value is checked;
an omitted argument is distinct from an explicitly supplied null. This
call-presence rule must not be inferred from SimplifiedSchema property
omission acceptance: `null(required)` and `unknown(required)` retain their
confirmed schema meaning. Variadic minimum counts, lazy evaluation, and
overload dispatch still need an audit against existing runtime behavior.
Arity alone cannot distinguish same-count overloads.

The passive checker exited successfully for its versioned snapshot: the
candidate metadata envelope and nine representative value definitions were
accepted by installed `md 0.1.0`; `unknown`, `null`, and `ip-address` were
rejected. The installed binary is not proven to come from the current
worktree. The first two types are Phase A additions; the `ip-address`
refinement requires a current-source support check and a shared
representation that preserves its constraints. Replacing it with plain
string would not preserve catalog parity. Snapshot expectations for rejected
types must change when those types are supported; they are not permanent
regression requirements.

The envelope accepts proposed value payloads permissively, so its success
does not validate all nested definitions or the meaning of behavior
metadata. No generic checker, editor integration, argument conversion,
overload dispatcher, or function execution was proven. Catalog types alone
also do not capture every runtime conversion and fallback: migration must
preserve actual behavior rather than introduce stricter rejection from the
displayed type. Full-catalog parity and metadata validation remain required
implementation evidence.

Independent clarity and risk review found no remaining blocking human
rulings. The specification is ready for planning around the agreed type
vocabulary, shared declarations, guarded calls, passive diagnostics, and
CLI policy migration. No further prototype is required; the limits above
are explicit planning and validation work.

## Performance and Completion Criteria

### Performance evidence

Implementation acceptance requires before/after measurements on
representative repository documents and controlled growth in declaration
count, union alternatives, and expression depth. Measure composition and
editor analysis on the same host with the same fixtures and settings,
isolating checking costs from unrelated I/O. Record and explain regressions
for review before acceptance. No numeric regression threshold is imposed;
acceptance requires explicit reviewer judgment. Preserve existing expression
depth and resource bounds.

This evidence is an implementation acceptance requirement, not an approved
pre-implementation spike. It does not authorize new CI jobs. Correctness
must hold on macOS, Linux, native Windows, and WSL2.

### Phase completion

| Phase | Required evidence |
|---|---|
| A — type vocabulary | Schema parser and validator cover `unknown`/`any`, null, required-null behavior, and inclusive/exclusive union cases above |
| B — shared function declarations | Full catalog information and behavior parity, including parameter/result types, conversions, advisory behavior, and true-result facts; built-ins and host extensions use shared metadata without host-specific analysis |
| C — declarations and evaluation | All annex library and host-integration acceptance paths pass, including effective-schema projection and conservative raw JSON Schema coverage; any increment deferral is documented |
| D — narrowing | Runtime and editor analysis agree for truthiness, conditional blocks, ternaries, guarded `&&` calls, and declared predicate facts, without stored-value mutation |
| E — diagnostics and CLI | CLI/DMLS share diagnostic codes and source positions; null-admitting parameter handling replaces the interim suppression rule; all affected command surfaces, policies, output formats, and migration cases are covered |

An increment may defer documented work, including the annex's raw JSON
Schema projection, but the full charter is not complete while a required
capability remains deferred. Existing runtime behavior is preserved except
for explicit changes in this specification.

Use package-area `just test`, `just test-l2`, and `just lint` as applicable,
with nextest for unit and integration tests. Grammar/schema changes require
a passive shipped-artifact corpus test and an end-to-end test through normal
invocation paths. Terminal and browser tests must not take window focus.
Verify the stated cross-platform behavior and collect the performance
evidence above. Update affected READMEs, dependency documentation, and skills
alongside implementation.

Completion means implementation complete and ready for review. The author
owns moving the feature to `_completed` after review; an agent must not make
that move or run `just complete`. This clarification authorizes no commits.

### Preparation and dependencies

Routine read-only inspection and local fixture preparation need no separate
pre-authorization. This feature introduces no external service, telemetry,
or deployment requirement. Existing file and remote dependencies remain;
new dependencies require a concrete planning proposal, and no new crate is
approved or added by this clarification.

## Remaining Decisions and Planning Work

### Human rulings

No human rulings remain after clarification, independent review, and the
completed representation prototype. The details below are implementation
planning work within the confirmed requirements.

### Implementation-planning details

- Choose internal Rust names and migration of existing `Any` and `ParamType`
  users. The expression-facing name, alias, and shared function contract are
  settled; internal symbol spelling is not a human-ruling blocker.
- Reconcile validation documentation that treats required null as absence
  with the confirmed new types; identify affected callers and tests. Check
  `required`/null consistency without universally excluding null.
- Specify the DMLS configuration surface for its opt-in unknown-input
  warning; its default and unchanged execution are settled.
- Complete the CLI identifier registry, explicit remote-host spelling, and
  mapping of `--strict-style`. Command coverage, severity boundaries,
  precedence, stopping behavior, and legacy-flag removal are settled. Do not
  infer host revocation or feature precedence from recovery/diagnostic rules.
- Define the report format identifier and complete field schema, including
  codes, severity, messages, source positions, type information, and whether
  the failed document field is null or omitted. Preserve document-only
  `--output json` and the confirmed diagnostic privacy boundary.
- Plan the concrete shared function-declaration format and migration,
  including advisory behavior and true-result facts, and the corresponding
  public Rust API compatibility work. Follow the prototype's separation of
  call presence and schema value types; audit variadic minimums, overloads,
  conversion/fallback behavior, and preservation of catalog refinements.
