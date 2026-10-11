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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 21
formerly: 2026-09-16-expression-type-system
depends-on:
    - 2026-10-28-schema-enhancements
related:
    - 2026-10-16-core-schema-types
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-29-recursive-schema-types
    - 2026-09-15-dasherized-identifiers
---

# Type-aware Evaluation, Narrowing, and Diagnostics

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

Phase B built one Darkmatter library function that maps each top-level variable to a design-time type, covering only single-shape SimplifiedSchema declarations and typing everything else `unknown`. Phase C **extends that same function**; it never builds a second projection, and no consumer (DMLS, Claudine, the CLI) walks schema syntax itself.

| Source of a variable | Phase B | After this spec |
|---|---|---|
| Single-shape top-level property `A` / `A(required)` | `A \| null` / `A` | unchanged |
| Root-union document schema | `unknown` | per the annex's root-union rule: each arm contributes its projected type, an arm that does not declare the property contributes `unknown`, and the results fold with the annex's union normalization |
| Imported named types, baseline, trigger, and document layers | partially (whatever `EffectiveSchema.simplified` held) | every layer of the final `EffectiveSchema`, after resolution and merging, including layers that `EffectiveSchema.simplified` does not represent |
| Raw JSON Schema | `unknown` | the annex's conservative raw JSON Schema projection (enumerable literal properties become visible; anything indeterminate is `unknown`) |
| A path into a typed object (`meta.count`) | `unknown` | the declared type of that nested property, with `\| null` when it or any object on the path is optional |
| A runtime-present key the schema does not declare | `unknown` | `unknown` |
| Reserved roots (`fm`, `ctx`, `env`, `err`, `timing`, `current`, `seq`, `loop`, `group`) and host globals | `unknown` | the shapes declared in `darkmatter/schemas/darkmatter.yaml` and in the host's catalog file, as registered by `2026-10-27-darkmatter-lifecycle-ownership`; a global the event declares unavailable is not given a type |
| A call used as an argument | `unknown` | the callee's declared `returns` type; for an overloaded function, the normalized union of the `returns` types of every overload the call's argument count can select |
| `x \|\| y` | `unknown` | the union of `x`'s type with `null` removed and `y`'s type |
| `c ? a : b` | `unknown` | the union of the two branch types, each computed under the narrowing of section 2 |
| Any other operator | `unknown` | `unknown` |

The [declarations annex](declarations-design.md) gives the normative detail for union normalization, optionality, root unions, open and pattern-based shapes, and raw JSON Schema; where it mentions strict roots, the strikes at its top apply.

Rules that hold throughout:

- **Passive.** Projection reads no file, runs no expression, fetches nothing, and never inserts or replaces a property to supply a name or a type. Existing composition-stage normalization (an absent optional, no-default top-level property from the document's own SimplifiedSchema becomes `null` before coercion) is unchanged, and no new insertion is introduced.
- **Deterministic.** The result is independent of mapping order, the arm validation selected, and validator error order.
- **Every expression surface uses it.** Body and frontmatter interpolation, conditional blocks, and the expressions in lifecycle event actions are all checked against the same projection; an event's globals are typed per that event's declaration.
- **`unknown` stays permissive.** A variable of type `unknown` needs no proof before use; existing runtime checks still apply when the call runs.

With this, worked-example rows f and i of Phase B become exact: `min(length(count), 5)` is category 1 (`length` returns `number`), and `min(maybe || 0, 5)` is category 1 (`|| 0` removes `null`).

### 2. Phase D — flow-sensitive narrowing

Inside code that a guard protects, a variable's design-time type is refined. Narrowing is static only: it never rewrites a stored value, proves that a file exists, or grants any access.

| Guard | Where it narrows | Effect |
|---|---|---|
| Truthiness of a bare variable or path: `x ? a : b`, `::block when="x"`, `x && f(x)` | the true branch, the block body, the right operand | `null` is removed from `x`'s type |
| `!x ? a : b` | the false branch | as above |
| A predicate call with a registered **true-implies** fact (`is_file_type(x)`; a host predicate declaring the same fact) | the true side only | `x` is suitable for the fact's type (for example a `file` parameter) |
| A predicate call with a registered **true-iff** fact (`is_number`, `is_string`, `is_boolean`, `is_null`, `is_array`, `is_object`) | both sides | the true side narrows `x` to the fact's type; the false side removes that type from `x`'s union |

Facts are the Rust-registered narrowing facts Phase B records; a boolean result alone never implies a fact. Narrowing composes through `&&`, nested ternaries, and nested conditional blocks.

Phase B's rows g and h become exact: `is_number(foo) ? min(foo, 5) : 0` and `is_string(foo) ? 0 : min(foo, 5)` (with `foo: string | number`) are category 1.

Runtime and editor analysis agree on every narrowing: the same library code computes it for composition and for DMLS.

### 3. File predicates

**`is_file_type(value: unknown): boolean`** is a new expression function (category `Filesystem`). It returns `true` when the value is a file value or a string that is structurally a supported `biscuit-file` reference: local relative and absolute paths, Windows paths, project-relative references, vault references, recursive references, and HTTP(S) references, accepted identically on every host OS. It returns `false` for an empty string, malformed reference syntax (including `https://`), and any non-string, non-file value (numeric `42` is `false`; the string `"42"` and the string `"hello world"` are `true`). It never resolves a reference, inspects the current directory, reads the environment or configuration, searches directories, touches the filesystem, or uses the network, and it never mutates its input. `true` guarantees neither existence nor readability. There is no `is_file` alias.

`biscuit-file` owns the passive structural rules. Merely constructing a `FileReference` is not enough (`https://` must fail even though the constructor classifies it as a URL); any missing structural check is added to `biscuit-file`, never as a second grammar in Darkmatter.

**`file_exists(value: string | file | null): boolean`** keeps Phase B's binding (no special case: `file_exists(42)` binds `"42"`; `null` returns `false`). This spec adds:

- A structurally malformed string (`""`, `"https://"`) returns `false` **before** any filesystem or network access, using the same `biscuit-file` rules as `is_file_type`. This replaces today's fallback in which a malformed URL-like string could be tried as a local path.
- Valid references keep their existing outcomes and normalizations (including `file://` where it is supported today): a local resolution failure is `false`; a valid HTTP(S) reference with no remote runtime is still an error; with a remote runtime, a denied host or failed fetch is `false` and a successful fetch is `true`.
- DMLS warns that the result is always `false` when the argument is a literal that is provably malformed. An argument whose type is merely `string` never gets that warning.

The charter's advisory warning for arguments of a known unsupported type and its opt-in DMLS "unknown input" warning for `file_exists` do not exist: Phase B retired the first (decision 63), and `expressions.strict` already warns for an `unknown` argument (category 4), so the opt-in setting is moot and no `file_exists`-specific setting is added.

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

Example: a host predicate `is_project_file(value: unknown): boolean` that registers a true-implies fact "suitable for `file`" makes `is_project_file(source) && frontmatter(source, 'title')` category 1, with no `is_project_file` built-in added to Darkmatter.

Claudine's remaining read-only AST walks (`composition/looping/config.rs`, `composition/lifecycle/validate.rs`) either move onto Darkmatter library APIs or are removed; Claudine keeps no independent definition of variable visibility or types.

### 6. Phase E — diagnostics

- **Shared diagnostics.** The Darkmatter library owns each diagnostic's code, severity, message, source position, and (where relevant) expected and actual types. Composition, the CLI, and DMLS report identical fields for the same problem.
- **CLI severities.** A composing command reports category 5 calls as errors (and rejects the document, per Phase B) and category 3 calls as warnings, matching DMLS's default. Categories 2 and 4 are reported only by DMLS under `expressions.strict`.
- **Retiring the interim absence-predicate suppression.** Today a direct bare-variable argument to `is_null`/`is_empty` (or an alias) suppresses the advisory `dm.expression.undeclared_property` warning for that variable. That catalog-name rule is replaced by a type rule: a direct bare-variable argument to a parameter whose declared type admits `null` suppresses the warning. "Direct" keeps its current meaning (`is_empty(trim(x))` still warns for `x`), and DMLS applies the same rule in its static walk. The function-name list in `AbsenceScope` is removed.
- **Privacy.** Diagnostics never gain evaluated argument values, environment values, or file contents, and producing a diagnostic never triggers extra evaluation or I/O. Authored source and existing messages may already contain sensitive text; no redaction subsystem or debug-value option is added.

### 7. Phase E — CLI policy verbs

Every command that exposes an affected control (`md compose`, explicit `md render`, default `md FILE` rendering, `md clean`, `md schema validate`) replaces its policy flags with three verbs:

| Option | Meaning |
|---|---|
| `--allow <id\|all>` | permit a particular recovery, or explicitly addressed access |
| `--deny <id\|all>` | make the selected diagnostics blocking |
| `--disable <id>` | stop a feature from running |

- The existing `--allow-*` flags (missing hyperlinks, image references, transclusions, any missing reference, invalid frontmatter, context overrides, duplicate assignments, shell-timeout recovery, remote hosts) move under `--allow`. `--no-baseline-schema` and `--no-schema-triggers` (as `2026-10-27-darkmatter-lifecycle-ownership` names it) move under `--disable`. `--strict-style` maps onto the verbs so that its informational notices are never promoted to errors. There is no standalone `--deny-warnings`, and `--allow` never enables an optional diagnostic.
- The complete identifier registry, the spelling of a remote-host allowance (for example `host=example.com`), and the `--strict-style` mapping are implementation details, chosen within these rules and documented in the CLI docs and completions. Each command accepts only identifiers meaningful to it.
- **`--allow all`** permits every recoverable, non-access allowance (missing references, invalid frontmatter, context overrides, duplicate assignments, shell-timeout recovery), each with its existing recovery behavior (a recovered shell timeout still yields an empty string). It never grants an unnamed remote host or authorizes shell execution.
- **Precedence.** A specific identifier overrides `all` regardless of argument order (`--allow all --deny missing-hyperlinks` denies that one). Equally specific opposites (`--allow x --deny x`, `--allow all --deny all`) are usage errors, detected before any composition or rendering effect. These rules do not imply host revocation or feature-disable precedence.
- **Blocking.** `--deny all` makes every reported warning and error blocking; informational notices never block; diagnostics that are off stay off. Denial never executes a branch that would not run, and an allowance never makes an invalid runtime call valid.
- **Stopping.** When a blocking diagnostic becomes known, the command stops before further work or effects, using passive analysis before effectful steps where it can establish the problem. It does not reorder the pipeline or roll back effects already done. These rules do not override a host's workflow decisions.
- **Output on failure.** A failed command never writes a composed document to standard output.
- **Removed spellings.** The old flags are removed in the same change with no aliases; using one is a usage error. Repository scripts, examples, docs, tests, and shell completions are updated together. The `darkmatter-cli` package owns parsing and migration; the library keeps owning composition behavior and diagnostic meaning.

### 8. Phase E — report output

`--output report-json` is added to the existing `--output` selector. `--output json` stays document-only and human output is unchanged.

The report is one JSON object with a fixed schema. Every field is always present:

```json
{
  "outcome": "failure",
  "diagnostics": [
    {
      "code": "dm.expression.argument_type",
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
- Each diagnostic always has `code`, `severity` (`"error"`, `"warning"`, or `"info"`), and `message`; `source`, `expected_type`, and `actual_type` are present and `null` when unavailable. The diagnostic code shown above is illustrative; codes come from the shared library diagnostics.

### 9. Performance evidence

Before/after measurements on representative repository documents, and with controlled growth in declaration count, union alternatives, and expression depth, are recorded in the implementation log: composition and DMLS analysis on the same host with the same fixtures, isolating checking cost from unrelated I/O. Every regression is explained there for review; no numeric threshold is imposed and no CI job is added. Existing expression depth and resource bounds are preserved.

## Out of scope

| Work | Owner |
|---|---|
| `unknown`, `null`, `alphanumeric`, `shell` | `2026-10-16-core-schema-types` |
| The catalog grammar, coercion engine, argument-type categories, composition-time rejection, `expressions.strict`, and the minimal projection | `2026-10-28-schema-enhancements` |
| The global catalog file, host catalog files, and lifecycle event ownership | `2026-10-27-darkmatter-lifecycle-ownership` |
| Strict-root validation, an unknown-root error, and an `is_known_variable_root` hook | struck (decided 2026-10-10); Darkmatter has no strict mode and no root-membership hook |
| Rescanning a whole-value expression's string result for `{{ … }}` | not done: inserted text is data, as Darkmatter's composition docs specify |

## Acceptance criteria

All criteria are L1 tests run with nextest through `just test` in `darkmatter/` (and `claudine/` where Claudine code changes), with `just lint` passing in both. Grammar and schema changes get a passive shipped-artifact corpus test and an end-to-end test through `md compose`. Passivity is proven with access instrumentation or failing access stubs, not return values alone. No test focuses a terminal or browser window; every test is portable to macOS, Linux, native Windows, and WSL2; no CI cell is added.

**Phase C — projection**

1. The projection for a root-union schema matches the annex's motivating table (`spec`, `design`, `review` → `unknown`; `plan` → `file`; `area` → `string | null`), independent of arm and key order.
2. Union normalization produces every row of the annex's normalization table.
3. A schema assembled from an imported named type, the baseline, a trigger, and the document projects every layer's top-level properties; a raw JSON Schema document with `properties`, `anyOf`, a type array including `"null"`, and an `allOf` exposes its enumerable literal properties with the annex's conservative types; a pattern-only shape declares no literal name.
4. `meta.count` with `meta: { count: number(required) }` (optional `meta`) projects to `number | null`.
5. `err`, `ctx`, and `timing` project to the shapes in `darkmatter/schemas/darkmatter.yaml`; a Claudine host global (`seq.index`) projects to its host-catalog type; an unavailable global gets no type.
6. Phase B's worked example rows f and i are category 1 through the compatibility API and silent in DMLS under both settings.
7. Projection performs no I/O, executes no expression, and leaves authored frontmatter, effective frontmatter, and `EffectiveState::data()` unchanged; existing schema-stage `null` materialization keeps its eligibility, exclusions, idempotence, and preservation of present values.
8. An expression in a lifecycle event action that calls `min(flag, 5)` with `flag: boolean(required)` is rejected exactly as the same call in the body is.

**Phase D — narrowing**

9. Rows g and h are category 1 through the compatibility API, in DMLS, and at composition.
10. With `x: number` (optional), `x ? min(x, 5) : 0`, `x && min(x, 5)`, and a `::block when="x"` body calling `min(x, 5)` are category 1 for the guarded call; the same call unguarded is category 2.
11. `is_string(foo) ? 0 : min(foo, 5)` narrows only through a true-iff fact: a test-registered predicate with a true-implies fact does not narrow its false branch.
12. Narrowing never mutates a stored value (the frontmatter is byte-identical after composition of each guard fixture).

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
26. An informational notice under `--deny all` (including every `--strict-style` notice) does not block success.
27. A blocking diagnostic that passive analysis establishes stops the command before any effectful step; one discovered after an effect stops later work without rolling back the earlier effect.
28. A failed composition writes no document to standard output in human, `--output json`, or `--output report-json` mode.
29. `--output report-json` on success emits one JSON object with `outcome: "success"`, `diagnostics`, and the document string; on failure it exits non-zero with `outcome: "failure"`, `diagnostics`, and `document: null`; every diagnostic carries all six fields, unavailable ones as `null`. `--output json` and human output are unchanged.

**Performance**

30. The implementation log contains the before/after measurements of section 9, each regression explained.

## Open questions

None.
