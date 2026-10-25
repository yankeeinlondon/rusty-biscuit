---
area: darkmatter
status: finalized-spec
created: 2026-09-16
owner: Ken Snyder <ken@ken.net>
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
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 2
completed: true
human_review: false
pipeline_step: 21
formerly: 2026-09-16-expression-type-system
depends-on:
    - 2026-10-16-type-narrowing
    - 2026-10-28-schema-enhancements
related:
    - 2026-10-16-core-schema-types
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-29-recursive-schema-types
    - 2026-09-15-dasherized-identifiers
implemented: true
implemented_by: "claude/opus"
---

# Type-aware Evaluation, Narrowing, and Diagnostics

Darkmatter is the repository's Markdown composition library. DMLS is its editor language server; Claudine is a host application that uses the same expression engine in lifecycle actions. A design-time type describes what analysis can prove without executing the document; a runtime value is the actual value read when an expression runs.

Darkmatter's expression type system, phases C to E. After `2026-10-28-schema-enhancements` (Phase B), Darkmatter knows the declared type of every function parameter and of a document's top-level properties, and rejects a call that can never work. This spec makes that knowledge complete and useful: every variable an expression can read gets an honest type (Phase C), a guard such as `x ? … : …`, `is_number(x)`, or `is_file_type(x)` refines that type inside the code it protects (Phase D), and the CLI and DMLS report the same diagnostics, which the author controls with three CLI verbs and can request as machine-readable JSON (Phase E). An author gains fewer false warnings in guarded code, real warnings where typed values are misused, and one consistent policy surface on the command line.

## Background

This file was the type-system charter (all five phases). On 2026-10-10 the charter was split and retired:

| Phase | Spec |
|---|---|
| A — the `unknown`, `null`, `alphanumeric`, and `shell` types | `2026-10-16-core-schema-types` |
| B — function schemas, the shared coercion engine, the catalog migration, the five argument-type categories, composition-time rejection of category 5 calls, DMLS call diagnostics, `expressions.strict`, and the minimal expression-variable type projection | `2026-10-28-schema-enhancements` |
| C — type-aware evaluation; D — flow-sensitive narrowing; E — diagnostics and CLI policy | this spec |

The charter's Phase B annexes moved to `2026-10-28-schema-enhancements` as its reconciliation input. The [declarations annex](declarations-design.md) stays here as Phase C design detail, with the strikes listed at its top.

Phase B's rules are inputs to this spec and are not restated: the argument-type categories and their DMLS severities, the conversion and union rules, the `f64` numeric model, the migration-ledger null rule, and the rule that a category 5 call anywhere in a document, including a branch that does not run, rejects the document at composition.

## Decided

### 1. Phase C — the variable-type projection becomes complete

Phase B built one Darkmatter library function that maps each top-level variable to a design-time type, covering only SimplifiedSchema declarations it can project to a single type and typing everything else `unknown`: `DeclaredTypes::top_level_types()` in `darkmatter/lib/src/markdown/compose/expression/scope/projection.rs`, built from the schema alone (`DeclaredTypes::new(schema)`, no static frontmatter values) and read by the design-time call check (`check_call`, `check_expression_calls`, `check_prepared_calls` in `compose/expression/call_check.rs`). That function is the declared-type layer of `TypeScope` (`2026-10-16-type-narrowing`). Phase C **extends that same function**; it never builds a second projection, and no consumer (DMLS, Claudine, the CLI) walks schema syntax itself.

| Source of a variable | Phase B | After this spec |
|---|---|---|
| Single-shape top-level property `A` / `A(required)` | `A \| null` / `A` | unchanged |
| Root-union document schema | the property's definition when every inline arm that declares it declares it identically; otherwise `unknown` | per the annex's root-union rule: each arm contributes its projected type, an arm that does not declare the property contributes `unknown`, and the results fold with the annex's union normalization |
| Imported named types, baseline, trigger, and document layers | partially (whatever `EffectiveSchema.simplified` held) | every layer of the final `EffectiveSchema`, after resolution and merging, including layers that `EffectiveSchema.simplified` does not represent |
| Raw JSON Schema | `unknown` | the annex's conservative raw JSON Schema projection (enumerable literal properties become visible; anything indeterminate is `unknown`) |
| A path into a typed object (`meta.count`) | `unknown` | the declared type of that nested property, with `\| null` when it or any object on the path is optional |
| A runtime-present key the schema does not declare | `unknown` | `unknown` |
| Reserved roots (`fm`, `ctx`, `env`, `err`, `timing`, `current`, `seq`, `loop`, `group`) and host globals | `unknown` | the shapes declared in `darkmatter/schemas/darkmatter.yaml` and in the host's catalog file, as registered by `2026-10-27-darkmatter-lifecycle-ownership`; a global the event declares unavailable is not given a type |
| A call used as an argument | `unknown` | the callee's declared `returns` type; for an overloaded function, the normalized union of the `returns` types of every overload the call's argument count can select |
| `x \|\| y` | `unknown` | the union of `x`'s type with `null` removed and `y`'s type |
| `c ? a : b` | `unknown` | the union of the two branch types, each computed under the narrowing of section 2 |
| Any other operator | `unknown` | `unknown` |

Return-type inference never evaluates a function or suppresses a diagnostic on an inner call. The argument-count union for overloads is deliberately conservative; this spec does not add type-directed overload selection to inference. Literal arguments use Phase B's passive literal classification; environment values and function results are not executed to obtain a more precise static type.

`fm` denotes the effective document frontmatter, so `fm.meta.count` and an unshadowed `meta.count` have the same type. Type lookup follows value lookup's precedence: fixed namespaces, registered host globals, then document properties. A host global shadowing a property does not hide that property through `fm`. Catalog presence alone does not register a global or make it available. An unavailable global still produces its binding diagnostic, even under a truthiness guard or `||`; it never falls back to a same-named document property.

For nested paths, retain structured-object and array item types. Literal member access and constant indices use the declared member or item type; add `null` when a member or index can be absent, including an array index not guaranteed by the shape. Dynamic keys or indeterminate shapes fall back to `unknown`. Recursive named types introduced by `2026-10-29-recursive-schema-types` are followed lazily along the requested path, one definition per segment, with cycle-aware normalization; projection never expands a recursive type into an infinite tree. A variable whose type is recursive projects to its named reference (`TypeExpr::Ref`), not an unfolded level (section 11). Unsupported raw JSON Schema constructs retain the annex's conservative fallback.

The [declarations annex](declarations-design.md) gives the normative detail for union normalization, optionality, root unions, open and pattern-based shapes, and raw JSON Schema; where it mentions strict roots, the strikes at its top apply.

Rules that hold throughout:

- **Passive.** Projection reads no file, runs no expression, fetches nothing, and never inserts or replaces a property to supply a name or a type. Existing composition-stage normalization (an absent optional, no-default top-level property from the document's own SimplifiedSchema becomes `null` before coercion) is unchanged, and no new insertion is introduced.
- **Deterministic.** The result is independent of mapping order, the arm validation selected, and validator error order.
- **Every expression surface uses it.** Body and frontmatter interpolation, conditional blocks, and the expressions in lifecycle event actions are all checked against the same projection; an event's globals are typed per that event's declaration.
- **`unknown` stays permissive.** A variable of type `unknown` needs no proof before use; existing runtime checks still apply when the call runs.

With this, worked-example rows f and i of Phase B become exact: `min(length(count), 5)` is category 1 (`length` returns `number`), and `min(maybe || 0, 5)` is category 1 (`|| 0` removes `null`).

### 2. Phase D — flow-sensitive narrowing

_Amended 2026-10-16._ Flow-sensitive narrowing is delivered by `2026-10-16-type-narrowing`, before this spec, as `TypeScope`: one Darkmatter scope that every consumer reads, with guards narrowing both sides, equality narrowing to literals of every scalar type, and one subtype relation. Its guard table and rules (stable subjects, short-circuit flow, limited truthiness evidence, predicates observing original values, conservative exclusions, impossible branches remaining checked, and the lifetime of a fact) are normative there and are not restated here. Phase B's rows g and h are exact from that fix onward.

This spec extends `TypeScope`; it never builds a second narrowing engine:

- **New facts register into the scope's fact registry.** `is_file_type` (section 3) registers a true-implies fact, and host predicates register theirs through section 5's contract, into the same registry built-ins use.
- **The projection extensions of section 1 are the scope's declared-type layer.** A guard narrows whatever type section 1 projects, including nested paths, host globals, and call return types.
- **Expression types read the scope.** The `c ? a : b` and `x || y` rows of section 1 compute each operand's type from the scope as narrowed for that operand.

Runtime and editor analysis agree on every narrowing: the same library code computes it for composition and for DMLS.

### 3. File predicates

**`is_file_type(value: unknown | null): boolean`** is a new expression function (category `Filesystem`). It returns `true` when the value is a file value or a string that is structurally a supported `biscuit-file` reference: local relative and absolute paths, Windows paths, project-relative references, vault references, recursive references, and HTTP(S) references, accepted identically on every host OS. It returns `false` for an empty string, malformed reference syntax (including `https://`), and any non-string, non-file value (numeric `42` is `false`; the string `"42"` and the string `"hello world"` are `true`). It never resolves a reference, inspects the current directory, reads the environment or configuration, searches directories, touches the filesystem, or uses the network, and it never mutates its input. `true` guarantees neither existence nor readability. There is no `is_file` alias.

`biscuit-file` owns the passive structural rules. Merely constructing a `FileReference` is not enough (`https://` must fail even though the constructor classifies it as a URL); any missing structural check is added to `biscuit-file`, never as a second grammar in Darkmatter.

The display category `Filesystem` does not make `is_file_type` context-dependent: it uses pure dispatch and works through the context-free expression API. Register its declaration, handler, and narrowing fact together. Structural validation covers all supported reference kinds, including `&` repository-root, `^` repository-scoped, `~` home, and environment-template forms. A well-formed template is valid without reading its variables; an unset variable may still fail during resolution. Reserved malformed forms, unsupported schemes, NUL-containing paths, and malformed templates are false rather than implicit local paths. Host-independent acceptance describes syntax only; a Windows reference accepted on macOS need not resolve there. No new reference spelling or glob grammar is introduced.

**`file_exists(value: unknown | null): boolean`** keeps Phase B's binding: it inspects its argument without converting it, so a `string` or `file` value is checked as a reference and every other value (null, a number, a boolean, an array, an object) returns `false` (`file_exists(42)` is `false`; Phase B decision 82). This spec adds:

- A structurally malformed string (`""`, `"https://"`) returns `false` **before** any filesystem or network access, using the same `biscuit-file` rules as `is_file_type`. This replaces today's fallback in which a malformed URL-like string could be tried as a local path.
- Valid references keep their existing outcomes and normalizations (including `file://` where it is supported today): a local resolution failure is `false`; a valid HTTP(S) reference with no remote runtime is still an error; with a remote runtime, a denied host or failed fetch is `false` and a successful fetch is `true`.
- DMLS warns that the result is always `false` when the argument is a literal that is provably malformed. An argument whose type is merely `string` never gets that warning.

Validate the bound reference before path resolution or remote fetching. Preserve the existing `file://` input adapter before applying the shared structural rules, so a currently supported `file://foo/bar` does not become an unsupported-scheme failure. `is_file_type` uses that same passive adapter for these supported forms; the adapter is not a new `FileReference` scheme. Do not collapse separators in a way that changes UNC or URL meaning. The `fileexists` alias receives identical behavior and diagnostics. A malformed reference is a valid input to this predicate and returns false; it is not a category 5 argument error.

The always-false warning comes from the shared library analysis and is reported by composition as well as DMLS. It is enabled by default and can block under `--deny all`; analysis never calls `file_exists` to determine the answer. The warning is limited to authored literal arguments and uses the same passive binding and structural validation as runtime.

The charter's advisory warning for arguments of a known unsupported type and its opt-in DMLS "unknown input" warning for `file_exists` do not exist: Phase B retired both in its section **`file_exists` Has No Exception**. Because the parameter is `unknown | null`, every argument, including an `unknown` one, is category 1, so `expressions.strict` adds no warning for `file_exists` either, and no `file_exists`-specific setting is added.

### 4. Guarded file arguments

A true `is_file_type(x)` makes `x` suitable for a `file` parameter within the guarded code, whatever its declared type:

```yaml
$schema:
    source: unknown
source: './docs/spec.md'
```

```text
is_file_type(source) && frontmatter(source, 'title')
```

Without the guard, `source` (`unknown`) against `frontmatter`'s file parameter is category 4: silent by default, a warning under `expressions.strict`. With the guard it is category 1 in both settings. The guard does not rewrite `source`, prove the file exists, or grant access; the read remains subject to ordinary resolution, access policy, and runtime failure. An unguarded `unknown` argument is still allowed.

### 5. One function contract for built-ins and host extensions

A host such as Claudine extends the expression language only by supplying functions, never grammar or evaluation:

- Each host function has a declaration in Phase B's catalog grammar (`function(parameters(…); returns(…); …)`), written in the host's catalog file that `2026-10-27-darkmatter-lifecycle-ownership` registers, and an implementation registered with Darkmatter under the same name through the same Rust registration API the built-ins use.
- Registration may record the same facts built-ins can (true-implies or true-iff narrowing facts, conversion recovery, source-type exclusions). No custom analysis callback exists.
- Darkmatter binds, type-checks, and narrows host calls exactly as it does built-ins. DMLS reads the host declarations from the same registered catalog file and never invokes a host function.
- The host is responsible for its implementation honoring its declaration.

Function catalogs declare signatures; global catalogs declare variable shapes. Register each explicitly for its role, without applying either as a document schema. Duplicate canonical names or aliases, built-in name collisions, a fact referencing a nonexistent parameter, or a narrowing fact on a non-boolean function are registration errors before evaluation. Runtime code and editor analysis must receive the same resolved signatures and facts.

**Host facts reach DMLS through a generated companion artifact.** Narrowing, laziness, conversion recovery, and source exclusions stay in Rust registration rather than the `function(...)` grammar, so a separately running DMLS process cannot infer them from the host signature file. Without a shared delivery mechanism, `is_project_file(source)` could narrow in composition but warn in the editor. Therefore:

- The host build generates a passive companion artifact from its Rust function registration. For each registered function it records the canonical function name, each fact's parameter position, the fact kind and type, and the other registered behavior metadata analysis needs (laziness, conversion recovery, source-type exclusions).
- The artifact is JSON. It is written next to the host's signature catalog, named with the catalog's file stem plus a `.facts.json` suffix (a catalog `functions.yaml` gets `functions.facts.json`). DMLS discovers it by that naming convention whenever it loads a registered signature catalog; no additional registration of its location exists.
- The artifact records the version of the signature catalog it was generated with, as a content hash of that catalog file. DMLS loads it alongside the catalog and checks it against the loaded catalog's version.
- DMLS reads the artifact as inert data and never executes host code. Rust registration remains the single authority; no behavior syntax is added to the catalog grammar.
- A missing, unreadable, or version-mismatched artifact, or one lacking metadata for a registered function, makes DMLS report incomplete analysis for the affected host functions. It never claims a category or narrowing result known only to the runtime.

Example: a host predicate `is_project_file(value: unknown | null): boolean` that registers a true-implies fact "suitable for `file`" makes `is_project_file(source) && frontmatter(source, 'title')` category 1, with no `is_project_file` built-in added to Darkmatter.

Claudine's remaining read-only scans of parsed expressions in [loop configuration](../../lib/src/composition/looping/config.rs) and [lifecycle validation](../../lib/src/composition/lifecycle/validate.rs) either move onto Darkmatter library APIs or are removed; Claudine keeps no independent definition of variable visibility or types.

### 6. Phase E — diagnostics

- **Shared diagnostics.** The Darkmatter library owns each diagnostic's code, severity, message, source position, and (where relevant) expected and actual types. Composition, the CLI, and DMLS report identical fields for the same problem. Phase B's call codes, `dm.expression.incompatible_call` (category 5, wrong argument count, ambiguous overload) and `dm.expression.uncertain_call` (category 3, plus 2 and 4 under `expressions.strict`), are defined today in DMLS (`darkmatter/dmls/src/diagnostics/codes.rs`); they move to the library unchanged and become the shared call codes. Composition currently reports only category 5, as a rejection (`MarkdownError::rejected_call()`); the category 3 warning below is new.
- **CLI severities.** A composing command reports category 5 calls as errors (and rejects the document, per Phase B) and category 3 calls as warnings, matching DMLS's default. Categories 2 and 4 are reported only by DMLS under `expressions.strict`.
- **Retiring the interim absence-predicate suppression.** Today a direct bare-variable argument to `is_null`/`is_empty` (or an alias) suppresses the advisory `dm.expression.undeclared_property` warning for that variable. That catalog-name rule is replaced by a type rule: a direct bare-variable argument to a parameter whose declared type explicitly includes `| null` suppresses the warning; an `unknown` (`any`) parameter does not. "Direct" keeps its current meaning (`is_empty(trim(x))` still warns for `x`), and DMLS applies the same rule in its static walk. The function-name list in `AbsenceScope` is removed.
- **Privacy.** Diagnostics never gain evaluated argument values, environment values, or file contents, and producing a diagnostic never triggers extra evaluation or I/O. Authored source and existing messages may already contain sensitive text; no redaction subsystem or debug-value option is added.

**Intentional scope of absence-warning suppression.** Only an explicit `| null` suppresses: the new rule suppresses the missing-property warning for any function, including a host function, whose parameter declares `| null` (for example `unknown | null` or `string | null`). A plain `unknown` parameter does not, so `is_number(x)` still warns unless its signature declares `| null`. This is broader than absence predicates by design. For overloads, suppress only when every argument-count candidate admits null at that position; optional omission alone is not null acceptance. Existing fallback and condition suppression remains. Suppression never hides unknown-function, unavailable-global, or argument-binding errors.

Shared diagnostics carry the authored source range, not a position in generated output. Preserve the origin across frontmatter interpolation, transclusion, and lifecycle subtree composition. JSON positions are one-based; adapters translate them to the editor protocol's coordinates. Compare identical analysis inputs and settings for parity; an editor analyzing an unsaved buffer is not expected to match a CLI reading different bytes. Blocking policy changes command outcome, not the underlying diagnostic's code or severity.

### 7. Phase E — CLI policy verbs

Every command that exposes an affected control (`md compose`, explicit `md render`, default `md FILE` rendering, `md clean`, `md schema validate`) replaces its policy flags with three verbs:

| Option | Meaning |
|---|---|
| `--allow <id\|all>` | permit a particular recovery, or explicitly addressed access |
| `--deny <id\|all>` | make the selected diagnostics blocking |
| `--disable <id>` | stop a feature from running |

- The existing `--allow-*` flags (missing hyperlinks, image references, transclusions, any missing reference, invalid frontmatter, context overrides, duplicate assignments, shell-timeout recovery, remote hosts) move under `--allow`. `--no-baseline-schema` and `--no-schema-triggers` (as `2026-10-27-darkmatter-lifecycle-ownership` names it) move under `--disable`. `--strict-style` maps onto the verbs so that its informational notices are never promoted to errors. There is no standalone `--deny-warnings`, and `--allow` never enables an optional diagnostic.
- The migration identifiers below are fixed; one command registry drives validation, help, and completions. Each command accepts only identifiers meaningful to it. Options are repeatable (`--allow missing-hyperlinks --allow shell-timeout`); unknown, empty, or unsupported identifiers are usage errors, and repeated identical selections are harmless.

| Existing control | Replacement | Behavior retained |
|---|---|---|
| `--allow-missing-hyperlinks` | `--allow missing-hyperlinks` | retain content when a hyperlink target is missing |
| `--allow-missing-image-refs` | `--allow missing-image-refs` | retain content when an image target is missing |
| `--allow-missing-transclusions` | `--allow missing-transclusions` | remove a missing transclusion directive |
| `--allow-any-missing-reference` | the three specific missing-reference allowances above | same combined recovery; no overlapping group identifier |
| `--allow-ctx-override` | `--allow ctx-override` | existing context-override recovery |
| `--allow-invalid-frontmatter-assignment` | `--allow invalid-frontmatter-assignment` | recover invalid assignments while preserving valid siblings |
| `--allow-reassigned-frontmatter-property` | `--allow reassigned-frontmatter-property` | rightmost assignment wins |
| `--allow-shell-timeout` | `--allow shell-timeout` | recovered timeout produces an empty string |
| `--allow-host HOST` | `--allow host=HOST` | existing exact-host consent and host validation |
| `--no-baseline-schema` | `--disable baseline-schema` | baseline is not applied |
| `--no-schema-triggers` (formerly `--no-trigger-schemas`) | `--disable schema-triggers` | existing trigger-discovery and bare-name lookup disable behavior |
| `--strict-style` | `--deny style` | block unknown/deprecated style schema issues and inline horizontal-rule deprecations; retain informational notices |

`--deny` also accepts shared diagnostic codes applicable to that command. `--allow` accepts only registered recovery and host identifiers, never arbitrary diagnostic codes; known-invalid calls, parse errors, and unavailable globals cannot be allowed. `--disable` accepts only registered feature identifiers and no `all`. `--disable baseline-schema` conflicts with an explicit `--baseline-schema PATH`, retaining the existing usage error. An allowance controls recovery, while denial can still make its reported warning blocking; help must explain that distinction.

> **Reader's note:** fixing the migration here prevents different commands from inventing different names or recovery rules. The combined missing-reference flag becomes three selections so specificity has one meaning. The style selector preserves the existing strict-style checks, including inline deprecations, without promoting informational notices. Additional recovery identifiers require a documented recovery contract, not merely a diagnostic code.

- **`--allow all`** permits every recoverable, non-access allowance (missing references, invalid frontmatter, context overrides, duplicate assignments, shell-timeout recovery), each with its existing recovery behavior (a recovered shell timeout still yields an empty string). It never grants an unnamed remote host or authorizes shell execution.
- **Precedence.** A specific identifier overrides `all` regardless of argument order (`--allow all --deny missing-hyperlinks` denies that one). Equally specific opposites (`--allow x --deny x`, `--allow all --deny all`) are usage errors, detected before any composition or rendering effect. These rules do not imply host revocation or feature-disable precedence.
- **Selector identity.** The registry records which diagnostic codes each recovery selector governs. A diagnostic code naming the same recovery is equivalent for conflict detection: spelling `--allow missing-hyperlinks` and denying that recovery's code must not bypass the equally-specific-opposites error. A separately denied, unrelated diagnostic still blocks an otherwise recovered command.
- **Access and disabled features.** `host=HOST` is an allowance only; `--deny host=HOST` is a usage error rather than a new revocation API. `--deny all` does not revoke explicit host consent. A disabled feature produces no diagnostics from work it did not perform; selecting a denial does not re-enable it. Its skipped checks must not be described as passing validation.
- **Blocking.** `--deny all` makes every reported warning and error blocking except recoveries explicitly selected by a more specific allowance. Informational notices never block; diagnostics that are off stay off. Denial never executes a branch that would not run, and an allowance never makes an invalid runtime call valid.
- **Stopping.** When a blocking diagnostic becomes known, the command stops before further work or effects, using passive analysis before effectful steps where it can establish the problem. It does not reorder the pipeline or roll back effects already done. These rules do not override a host's workflow decisions.
- **Available analysis inputs.** Passive checking uses the resolved declarations and registered binding view available at that stage. It does not execute an expression, shell, or lifecycle action to discover a type. Once preparation supplies the declarations, check all authored calls before expression evaluation; for a newly reached transcluded document, check before evaluating that child. Do not promise rejection before earlier pipeline stages required to obtain the analysis inputs. Later writes invalidate facts as `2026-10-16-type-narrowing` §8 describes.
- **Output on failure.** A failed command never writes a composed document to standard output.
- **Removed spellings.** The old flags are removed in the same change with no aliases; using one is a usage error. Repository scripts, examples, docs, tests, and shell completions are updated together. The `darkmatter-cli` package owns parsing and migration; the library keeps owning composition behavior and diagnostic meaning.

### 8. Phase E — report output

`md compose --output report-json` adds a composition report to the existing `--output` selector. Existing `--output json` behavior and human output are unchanged. Report mode is supported only by `md compose` in this change; explicit rendering, default `md FILE`, cleaning, and schema validation reject it as an unsupported output format before work.

> **Reader's note:** rendering JSON represents an abstract syntax tree, and schema validation can report multiple files. Restricting this new report to composition preserves those established output contracts rather than silently replacing them with a single composed-document report. The CLI policy verbs still apply to all commands named in section 7.

The report is one JSON object with a fixed schema. Every field is always present:

```json
{
  "outcome": "failure",
  "diagnostics": [
    {
      "code": "dm.expression.incompatible_call",
      "severity": "error",
      "message": "min: argument `a` expects number; received boolean",
      "source": { "file": "notes.md", "line": 4, "column": 18 },
      "expected_type": "number",
      "actual_type": "boolean"
    }
  ],
  "document": null
}
```

- `outcome` is `"success"` or `"failure"`. On success `document` holds the composed document as a string; on failure `document` is present and `null`, and the command exits non-zero. A failure report on standard output does not violate the no-document-on-failure rule.
- Each diagnostic always has `code`, `severity` (`"error"`, `"warning"`, or `"info"`), and `message`; `source`, `expected_type`, and `actual_type` are present and `null` when unavailable. The code shown above is Phase B's category 5 call code; the message is illustrative. Codes come from the shared library diagnostics.
- `source`, when present, has `file`, `line`, and `column`; `file` is null for input without a named origin. Coordinates point to the start of the authored range. Expected/actual types use the same canonical spelling as editor messages.
- A successfully parsed report-mode invocation emits exactly one object, including input-read, schema-load, parse, binding, and runtime failures. Earlier diagnostics remain in a failure report; failures without a source or types use null fields and an existing typed diagnostic code. CLI usage errors occur before report mode is established and retain ordinary stderr/exit behavior. A broken stdout cannot guarantee a complete report and remains an output error.
- Standard output contains only that object: progress, performance output, and other status messages go to stderr. Do not duplicate report diagnostics in a human-error footer. Diagnostic order follows source order for passive checks and discovery order for runtime diagnostics; concurrent child results are collected in authored directive order. Report creation never evaluates or renders the document again.
- No partial document is streamed before success. Final document/file output and viewer launch wait until blocking checks have passed; already completed lifecycle effects and cache writes retain section 7's no-rollback rule. This is not a transaction over composition effects.

### 9. Performance evidence

A quick before/after sample on one host using an existing representative repository document is recorded in the implementation log for composition and DMLS analysis, separating checking cost from unrelated I/O where practical. Record the host, input, and observations, and explain a visible regression for review. No synthetic growth workload, additional host, repetition requirement, statistical threshold, or CI job is required. Existing expression depth and resource bounds are preserved; bounded analysis that cannot prove a fact remains conservative and never falsely proves a call valid or invalid.

### 10. Reconciliation with the implemented Phase B

_Added 2026-10-23._ `2026-10-28-schema-enhancements` is implemented. This
section records the corrections its delivered code forced here; sections 1, 3,
6, and 8 now name the delivered symbols and codes. No outcome of this spec
changed.

- **`is_file_type` declares `unknown | null`.** *Decision:* the signature is
  `is_file_type(value: unknown | null): boolean`, like every inspecting
  predicate in Phase B's catalog (`is_null`, `is_number`, `file_exists`).
  *Alternatives:* keep `value: unknown`. *Why:* Phase B's binder admits `null`
  only through an explicit `| null` arm, so with `unknown` alone
  `is_file_type(null)` would be a type error, contradicting this spec's
  "`false` for any non-string, non-file value". Phase B also classifies an
  `unknown` argument to a plain `unknown` parameter as category 4, so under
  `expressions.strict` the guard `is_file_type(source)` would itself warn,
  contradicting criterion 16 ("only the unguarded call warns"); to
  `unknown | null` it is category 1. The test host predicate
  `is_project_file` takes the same signature so it receives the same
  checking (criterion 17). *Downsides:* under section 6's
  type rule, `is_file_type(x)` with an undeclared `x` now suppresses
  `dm.expression.undeclared_property`, like the other predicates.
- **`file_exists`'s delivered signature replaces this spec's assumption.**
  *Decision:* section 3 uses `file_exists(value: unknown | null)` with Phase B's
  inspecting binding. *Alternatives:* none; this is the delivered,
  owner-decided contract (Phase B decision 82, revising its decision 63, which
  this spec had copied). *Why:* section 3 assumed `file_exists(42)` binds
  `"42"`, which Phase B reversed before implementing. *Downsides:* none; the
  structural rejection and always-false warning added here are unchanged.
- **Phase C extends the declaration-only layer.** *Decision:* the projection
  extensions of section 1 extend `DeclaredTypes` as the call check builds it,
  from declarations only. A static frontmatter value never narrows a declared
  type. (`DeclaredTypes::with_static_frontmatter` stays the presence checks'
  layer.) *Alternatives:* let a set optional value remove `null`. *Why:*
  Phase B decided this (its Phase 5 decision 1): a frontmatter value is a
  runtime value that setters, caller inputs, and interpolation can replace.
  *Downsides:* a document that sets an optional property still gets the
  strict-mode category 2 warning.
- **The compatibility API is `call_check`.** *Decision:* "the compatibility
  API" in this spec and its plan means Phase B's `ArgumentCategory`,
  `classify_argument`, `check_call`/`CallCheck`, and the expression-level walk
  `check_expression_calls`/`check_prepared_calls`. Composition reaches it
  through `compose/call_precheck.rs` and DMLS through
  `dmls/src/diagnostics/call_check.rs`. Section 1's call-return and operator
  typing feeds `ArgumentType::of`, which today types every call and operator
  argument `unknown`. *Alternatives:* a second classification entry point.
  *Why:* rule "one projection, one narrowing engine, one diagnostics model".
  *Downsides:* none found.
- **Host catalogs use Phase B's strict catalog loader.** *Decision:* a host
  function catalog is a tagged `kind: schema` file with a `categories:` list.
  Every function names exactly one declared `category(...)`; non-function
  `types:` entries are helper types. *Alternatives:* a looser host loader.
  *Why:* one catalog grammar and one loader. Phase B's loader rejects a
  function without a category (its Phase 4 decision 8). *Downsides:* a host
  must declare categories even for one function.

### 11. Reconciliation with the implemented recursive schema types

_Added 2026-10-23._ `2026-10-29-recursive-schema-types` is implemented. This
section records what its delivered code settles for sections 1 and 2. No
outcome of this spec changed.

- **A recursive variable projects to its named reference.** *Decision:*
  `DeclaredTypes::top_level_types()` already types a property of a cyclic
  named type as `TypeExpr::Ref(DefKey)` when given the definition table
  (`DeclaredTypes::with_definitions`); section 1's extensions keep that
  reference wherever a recursive type appears (nested members, union arms,
  call returns), and union normalization compares references by `DefKey`
  (`is_subtype_with_definitions`). The call check sees through a reference
  to its definition body. *Alternatives:* unfold one level. *Why:* the
  prerequisite decided this so comparison stays by identity and messages
  name the type (`is node, expected …`). *Downsides:* a consumer that
  needs the body must hold the definition lookup.
- **Nested paths open one definition per segment.** *Decision:* a constant
  member path through a recursive type resolves the reference at each
  segment it crosses and nothing else, as the library's
  `property_def_at_path_with_definitions` already does for schema lookups; a
  back-reference reached by the path keeps its name (`node[]`). *Why:*
  criterion 32 requires termination without expanding the schema.
  *Downsides:* none found.
- **Definitions are looked up across every layer.** *Decision:* the declared
  layer resolves a reference with `EffectiveSchema::definition(&DefKey)` (or
  the `DefinitionLookup` trait), which searches the document's table and
  every trigger layer's. *Alternatives:* keep passing
  `EffectiveSchema::definitions()`, as the call check, composition precheck,
  DMLS, and `lifecycle_declared_types` do today. *Why:* `definitions()` is
  the document's table only, and section 1 adds trigger and other layers
  whose recursive types live in their own tables; a trigger payload's `$defs`
  keys may have been renamed in the merged JSON, so a JSON-key lookup could
  bind the wrong body. *Downsides:* `DeclaredTypes::with_definitions` widens
  from a `&DefinitionTable` to a lookup; its callers change with it.
- **Guards narrow a recursive arm by its definition.** *Decision:* the
  section 2 extension gives `TypeScope`'s guard narrowing the same
  definition lookup, so a `Ref` arm is classified by its definition body's
  family, one level, without expansion. Without a lookup, a `Ref` arm stays
  in every family, as delivered. *Alternatives:* keep the delivered
  behavior. *Why:* section 2 requires a guard to narrow whatever type
  section 1 projects; today a recursive arm survives on both sides of every
  guard. *Downsides:* none found; the fallback is the delivered conservative
  result, which never proves a call valid or invalid falsely.
- **Raw JSON Schema `$defs` stay conservative.** *Decision:* a raw `$ref`
  has no typed definition (the prerequisite keeps raw definitions as JSON
  keys only), so the raw projection types it `unknown`. *Why:* the annex's
  rule that anything indeterminate is `unknown`. *Downsides:* a raw
  recursive schema gets no precise variable types.

## Out of scope

| Work | Owner |
|---|---|
| `unknown`, `null`, `alphanumeric`, `shell` | `2026-10-16-core-schema-types` |
| The catalog grammar, coercion engine, argument-type categories, composition-time rejection, `expressions.strict`, and the minimal projection | `2026-10-28-schema-enhancements` |
| The global catalog file, host catalog files, and lifecycle event ownership | `2026-10-27-darkmatter-lifecycle-ownership` |
| Strict-root validation, an unknown-root error, and an `is_known_variable_root` hook | struck (decided 2026-10-10); Darkmatter has no strict mode and no root-membership hook |
| Rescanning a whole-value expression's string result for `{{ … }}` | not done: inserted text is data, as Darkmatter's composition docs specify |

## Acceptance criteria

All criteria are L1 tests run with nextest through `just test` in `darkmatter/` (including DMLS) and in other changed package areas (`claudine/` and `biscuit-file/` as applicable), with `just lint` passing in each changed area. Grammar and schema changes get a passive shipped-artifact corpus test and an end-to-end test through `md compose`. Passivity is proven with access instrumentation or failing access stubs, not return values alone. No test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; this spec introduces no new CI job or environment requirement beyond the repository's normal scope planning.

**Phase C — projection**

1. The projection for a root-union schema matches the annex's motivating table (`spec`, `design`, `review` → `unknown`; `plan` → `file`; `area` → `string | null`), independent of arm and key order.
2. Union normalization produces every row of the annex's normalization table.
3. A schema assembled from an imported named type, the baseline, a trigger, and the document projects every layer's top-level properties; a raw JSON Schema document with `properties`, `anyOf`, a type array including `"null"`, and an `allOf` exposes its enumerable literal properties with the annex's conservative types; a pattern-only shape declares no literal name.
4. `meta.count` with `meta: { count: number(required) }` (optional `meta`) projects to `number | null`.
5. `err`, `ctx`, and `timing` project to the shapes in `darkmatter/schemas/darkmatter.yaml`; a Claudine host global (`seq.index`) projects to its host-catalog type; an unavailable global gets no type.
6. In Phase B's worked-example table, the outer `min` call in `min(length(count), 5)` and the call `min(maybe || 0, 5)` are category 1 through the compatibility API and silent in DMLS under both settings. The inner `length(count)` retains category 2: silent by default and a warning under `expressions.strict`.
7. Projection performs no I/O, executes no expression, and leaves authored frontmatter, effective frontmatter, and `EffectiveState::data()` unchanged; existing schema-stage `null` materialization keeps its eligibility, exclusions, idempotence, and preservation of present values.
8. An expression in a lifecycle event action that calls `min(flag, 5)` with `flag: boolean(required)` is rejected exactly as the same call in the body is.

**Phase D — narrowing**

Criteria 9, 10, and 12 are first met by `2026-10-16-type-narrowing`; here they must still hold once section 1's projection and section 5's host facts extend `TypeScope`. A review confirms no narrowing logic exists outside `TypeScope`.

9. `is_number(foo) ? min(foo, 5) : 0` and `is_string(foo) ? 0 : min(foo, 5)`, with `foo: string | number`, make the guarded `min` call category 1 through the compatibility API, in DMLS, and at composition.
10. With `x: number` (optional), `x ? min(x, 5) : 0`, `x && min(x, 5)`, and a `::block when="x"` body calling `min(x, 5)` are category 1 for the guarded call; the same call unguarded is category 2.
11. `is_string(foo) ? 0 : min(foo, 5)` narrows only through a true-iff fact: a test-registered predicate with a true-implies fact does not narrow its false branch. `is_file_type` and a test-registered host predicate register their facts into `TypeScope`'s fact registry, and their narrowing appears identically in composition, DMLS, and the shell template hole type.
12. Narrowing never mutates a stored value: compare the state immediately before and after analysis/evaluation in fixtures with no authored writes, separately from existing schema normalization. Authored source bytes remain unchanged. Composition may still materialize eligible optional properties and execute authored `set` actions; those changes must not be attributed to narrowing.

**File predicates and guarded calls**

13. `is_file_type` returns `true` for a file value and for the strings `./docs/spec.md`, `/tmp/spec.md`, `\Users\ken\data\foobar.json`, `@docs/spec.md`, `vault:notes/spec.md`, `%@spec.md`, `https://example.com/spec.md`, `http://example.com/spec.md`, `42`, and `hello world`, on every host OS; and `false` for numeric `42`, `""`, and `https://`. Failing filesystem, environment, and network stubs prove no access.
14. `file_exists("")` and `file_exists("https://")` return `false` with no filesystem or network access (failing stubs); `file_exists(null)` is `false`; the existing valid-reference corpus (including `file://` forms supported today) keeps its outcomes with and without a remote runtime.
15. DMLS reports an always-false warning for `file_exists("https://")` and `file_exists("")`, and none for a `string`-typed variable argument.
16. With `source: unknown`, `frontmatter(source, 'title')` is category 4 and `is_file_type(source) && frontmatter(source, 'title')` is category 1; under `expressions.strict` only the unguarded call warns.
17. A test-registered host predicate with a declared true-implies `file` fact receives the same checking as `is_file_type`; DMLS reads its declaration from the host catalog file and a counter proves the host function is never invoked during analysis.
18. DMLS's configuration model contains no `file_exists`-specific setting.

**Phase E — diagnostics**

19. The same category 5 call produces identical code, severity, message, source position, and expected/actual types from composition, `md compose --output report-json`, and DMLS.
20. A category 3 call composes successfully with a warning; with `--deny all` the same document fails.
21. `is_null(x)`, `is_empty(x)`, and `file_exists(x)` with undeclared `x` raise no `dm.expression.undeclared_property`; `min(x, 5)` and `is_empty(trim(x))` still raise it; DMLS agrees; `AbsenceScope` contains no function-name list.
22. Reports contain no evaluated argument values, environment values, or file contents, and producing them performs no extra evaluation or I/O (instrumented).

**Phase E — CLI**

23. `--allow all --deny <id>` and `--deny all --allow <id>`, in either order, apply the specific rule; `--allow <id> --deny <id>` and `--allow all --deny all` are usage errors before any composition or rendering effect.
24. Each migrated allowance reproduces its legacy flag's recovery and output (shell timeout recovered as an empty string included); `--allow all` grants no unnamed remote host and authorizes no shell execution.
25. Every removed legacy flag is a usage error on every migrated command; repository scripts, examples, docs, and completions use only the new verbs.
26. An informational style notice under `--deny all` or `--deny style` does not block success; style schema issues and inline deprecations retain their former `--strict-style` blocking behavior under `--deny style`.
27. A blocking diagnostic established with already available passive analysis inputs stops the command before its next effectful step; one discovered after an effect stops later work without rolling back the earlier effect. Preparation needed to obtain those inputs is not reordered.
28. A failed composition writes no document to standard output in human, `--output json`, or `--output report-json` mode, and does not publish final file output or launch a viewer. Prior effects are not rolled back.
29. `--output report-json` on success emits one JSON object with `outcome: "success"`, `diagnostics`, and the document string; on failure it exits non-zero with `outcome: "failure"`, `diagnostics`, and `document: null`; every diagnostic carries all six fields, unavailable ones as `null`. `--output json` and human output are unchanged.

**Performance**

30. The implementation log contains the before/after measurements of section 9, each regression explained.

**Additional boundary checks**

31. Bare and `fm` paths agree unless shadowed by a host global. Unavailable globals retain their binding error under `||` and predicate guards; a same-named document property remains reachable through `fm`.
32. Constant array/tuple indices project the item type with null for possible absence; dynamic indices remain conservative. A finite nested path through a recursive named type terminates without fully expanding the schema.
33. Negation and compound guards propagate only justified facts. `is_number(x) || ready` does not prove `number`; a false bare-variable branch does not prove null. Guards on calls do not refine their arguments. A contradictory guard still reports a known-invalid call in its unreachable branch.
34. A lifecycle fixture guards a numeric value, changes it with `set`, then reads it in a later call. Runtime and DMLS discard the prior guard fact; overlapping path writes, event re-entry, and a shadowed `fm` path are also covered.
35. `is_file_type` works without a resolution context. The shared passive validator covers repository/home/template references, malformed reserved forms, unsupported schemes, NUL, and HTTP(S) syntax. Supported `file://` inputs retain their adapter behavior. `fileexists` matches `file_exists`; malformed literals warn in both composition and DMLS without I/O.
36. A parameter that explicitly declares `| null` suppresses the direct missing-property warning, including on host functions; a plain `unknown` (`any`) parameter, optional non-null parameters, and mixed-nullability overloads do not suppress it. Other binding/type errors remain visible.
37. Registration rejects name/alias collisions and invalid fact declarations. Host analysis and runtime use the same facts: the host build writes `<catalog stem>.facts.json` beside its signature catalog, and DMLS loads it with the catalog, so `is_project_file(source) && frontmatter(source, 'title')` is category 1 in both composition and the editor. A missing artifact, an artifact whose recorded catalog hash differs from the loaded catalog, and an artifact lacking a registered function each make DMLS report incomplete analysis rather than a category or narrowing result. The editor never invokes the host handler.
38. CLI tests cover each registry mapping, unknown identifiers, repeated identical options, baseline-file conflicts, rejected host denials, and disabled checks staying disabled. Specific denial beats `--allow all`; specific recovery beats `--deny all`; independently denied diagnostics still block.
39. Report tests cover input-read, schema, parse, and runtime failures as well as success, earlier warnings surviving later failure, null source/type fields, authored positions from transcluded content, and stdout containing one JSON object. Non-compose commands reject `report-json` before effects; existing rendering JSON and multi-file schema-validation JSON retain their formats.

## Documentation and migration deliverables

Update Darkmatter's README and current topic pages for expressions, schemas, composition, CLI policy, and DMLS settings, plus CLI help and completions. Document the broad null-admitting suppression rule, guard limits, pure `is_file_type`, and report failure behavior with compact examples. Update Claudine's host integration docs and the relevant skills when its parsed-expression scans and function registration change. Update biscuit-file's reference docs if its public passive validation API changes. Keep current docs independent of feature-spec identifiers and lifecycle locations.

## Open questions

None.
