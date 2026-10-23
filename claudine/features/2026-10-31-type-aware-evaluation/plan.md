---
total_phases: 9
created: 2026-10-12
phase: 1
agent: claude/sonnet
yolo: "true"
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-dmls
    - biscuit-file
    - claudine
    - claudine-cli
---

# Type-aware Evaluation, Narrowing, and Diagnostics Implementation Plan

## Summary and Definition of Done

The work completes Darkmatter's expression type system (phases C to E of the
charter). Most code lands in `darkmatter/` (library, `darkmatter-cli`, DMLS);
`biscuit-file/` gains passive structural validation, and `claudine/` moves its
parsed-expression scans onto Darkmatter APIs and registers host functions.

| Spec section | Delivers | Plan phase |
| --- | --- | --- |
| 1 (Phase C) | The one variable-type projection becomes complete | 2 |
| 2 (Phase D) | Flow-sensitive narrowing, shared by composition and DMLS | 3 |
| 3, 4 | `is_file_type`, hardened `file_exists`, guarded file arguments | 4 |
| 5 | Host extensions through one contract and the `.facts.json` artifact | 5 |
| 6 (Phase E) | Shared diagnostics, type-based absence suppression | 6 |
| 7 (Phase E) | `--allow` / `--deny` / `--disable` CLI verbs | 7 |
| 8, 9 | `--output report-json`, performance evidence | 8 |
| Deliverables | Docs, skills, completions, final gates | 9 |

**Success means:**

- All 39 acceptance criteria have an observable L1 test (traceability table in
  `implementation-log.md`, maintained from Phase 1).
- `just test` and `just lint` pass in `darkmatter/` (including DMLS) and in
  each other changed area (`biscuit-file/`, `claudine/`); grammar/schema
  changes have a shipped-artifact corpus test and an `md compose` end-to-end
  test.
- Passivity (no I/O, no evaluation, no host-function invocation, no value
  mutation) is proven with failing access stubs or counters, never by return
  values alone.
- Composition, `md compose --output report-json`, and DMLS report identical
  diagnostics for the same input; no consumer walks schema syntax itself.
- The before/after performance sample is in the implementation log.
- Terminal state: **implementation complete, ready for review.** The author
  closes the feature; the agent never moves it to `_completed`, never runs
  `just complete`, never commits unless told, and never runs `cargo fmt`.

## Phase 1 — Contracts, Rulings, and Baselines

Nothing in this phase changes product code. It settles what the spec leaves
to the implementer, verifies the inputs this spec depends on exist, and
captures the "before" numbers that cannot be captured afterwards.

### Necessary Rules

These rules bind every later phase.

1. **The spec is binding; there are no open product questions.** Internal
   representation and helper placement are implementation choices.
2. **Dependency gate (ruling for the author).** The spec builds on Phase B
   (`2026-10-28-schema-enhancements`: catalog `returns`, argument-type
   categories, coercion engine, `expressions.strict`, minimal projection) and
   on `2026-10-27-darkmatter-lifecycle-ownership` (global/host catalog files)
   and `2026-10-29-recursive-schema-types`. At planning time all three spec
   files read `finalized-spec`. Phase B has since been implemented (as of
   2026-10-23, through its Phase 5; its documentation phase was in progress):
   its seams are named in spec.md section 10 and in the Dependency audit task.
   Task "Dependency audit" below records which landed.
   **If a dependency is not implemented, stop and report to the author**:
   this plan does not absorb Phase B work. The plan below assumes the
   dependencies are in place and names each seam it extends.
3. **One projection, one narrowing engine, one diagnostics model.** Phase C
   extends Phase B's single projection function. Narrowing and diagnostics
   live in the Darkmatter library and are consumed by composition, the CLI,
   DMLS, and Claudine. A second implementation anywhere is a defect.
4. **Facts are Rust-registered only.** No callback and no behavior syntax in
   the catalog grammar. A boolean result alone never implies a fact.
5. **Passive means passive.** Projection, narrowing, `is_file_type`, the
   always-false warning, and reports never read files, environment, or
   network, never execute expressions, and never invoke a host function.
6. **Impossible branches are still checked** using the incoming type (no
   empty/never type). A truthy guard never proves `null` on the false side.
7. **Removed means removed.** Legacy `--allow-*` / `--no-*` flags get no
   aliases; scripts, docs, tests, and completions change in the same change.
8. **Comments follow code.** Any behavior change includes a pass over the
   symbol's `///` and inline comments (repo comment-quality rules). Docs under
   `docs/` describe current behavior, never name a feature or fix, and are
   written for a developer new to the repo, with compact examples and Mermaid
   where a flow is clearer than prose.
9. **Verification.** All tests are L1, portable to macOS, Linux, native
   Windows, and WSL2, never focus a terminal or browser, use nextest via
   `just test`, the Test Toolkit, and `CliProcessFixture`; register new
   modules in the consolidated L1 roots. Add no CI cells.
10. **Rulings recorded for the author (not blocking, defaults stated):**
    - *Performance scope.* The spec asks for one host and a quick sample. The
      plan does exactly that (macOS dev host). Any wider measurement (other
      OSes, repetition) is left to the author.
    - *`--strict-style` mapping.* `--deny style` keeps its blocking set
      (unknown/deprecated style schema issues, inline horizontal-rule
      deprecations); informational notices never block. Default: preserve
      the former behavior test-for-test.
    - *Diagnostic code for `file_exists` always-false.* The spec gives no
      code; default is a new `dm.expression.*` code registered in the shared
      library diagnostics and listed in CLI help.
    - *Facts-artifact hash.* Spec says "content hash of the catalog file".
      Default: `biscuit-hash` BLAKE3 over the raw catalog bytes, hex-encoded
      (repo rule: all hashing goes through `biscuit-hash`).
    - *Facts-artifact schema versioning.* The artifact carries its own
      `format_version` integer in addition to the catalog hash; an unknown
      version is "incomplete analysis".
    - *Where `report-json` types for `source.file` come from.* Default: the
      authored file path as given on the command line, as the spec says null
      only when no named origin exists.
    - *Reconciliation with the implemented Phase B (2026-10-23).* spec.md
      section 10 records the decisions: `is_file_type` and the test host
      predicate declare `unknown | null`; `file_exists` keeps Phase B's
      inspecting `unknown | null` binding (`file_exists(42)` is `false`);
      Phase C extends the declaration-only `DeclaredTypes` layer; "the
      compatibility API" is `call_check`; host catalogs use the strict catalog
      loader. Phase 4 and the Dependency audit below were updated to match.

### Input Robustness Matrix

The only new file-format reader is the **host facts artifact**
(`<catalog stem>.facts.json`) read by DMLS. CLI arguments are not
configuration (the registry-mapping tests in Phase 7 cover them). The catalog
reader and schema readers are Phase B's/earlier work and are regression
boundaries here, not new reader projects.

Load-bearing fields: `format_version`, `catalog_hash`, `functions` (a map
keyed by canonical function name), per function `facts[]` with `parameter`,
`kind`, `type`, and the behavior metadata `lazy`, `conversion_recovery`,
`source_exclusions`.

| Shape | Required outcome (DMLS public result) |
| --- | --- |
| absent file | incomplete-analysis diagnostic for affected host functions; no category, no narrowing |
| absent field (each load-bearing one) | incomplete analysis; never read as "no facts" |
| explicit `null` field | incomplete analysis; distinct from absent in the message |
| wrong type, whole field (e.g. `functions: 3`) | incomplete analysis |
| wrong type, one element (`facts: [{..}, 123]`) | incomplete analysis for that function; element never silently dropped |
| wrong type, every element | incomplete analysis; never read as empty |
| empty (`functions: {}`, `facts: []`) | `facts: []` for a registered function means "no facts" (valid); `functions: {}` with registered host functions means those functions lack metadata, so incomplete analysis |
| duplicate key | rejected as invalid artifact (incomplete analysis); never last-wins |
| trailing/invalid content | rejected (incomplete analysis) |
| hash mismatch / unknown `format_version` | incomplete analysis |

One test walks the whole matrix from a real artifact generated by the host
build, one edit per cell, with a control row proving the unedited artifact
yields the positive result (`is_project_file(source) && frontmatter(source,
'title')` is category 1). Before declaring done, grep the reader for
`#[serde(default)]`, `Option<T>` where absent and null must differ,
`filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` on load-bearing
parses.

### Spikes

A spike runs once, before the work it informs.

- [ ] **Spike S1 — biscuit-file structural-validation gap** (runs once; informs
  Phase 4 only)
  - Classify each string in acceptance criterion 13 and 14's corpus
    (`./docs/spec.md`, `/tmp/spec.md`, `\Users\ken\data\foobar.json`,
    `@docs/spec.md`, `vault:notes/spec.md`, `%@spec.md`, `https://…`, `http://…`,
    `42`, `hello world`, `""`, `https://`, `&`/`^`/`~`/env-template forms,
    NUL, unsupported schemes, `file://foo/bar`) against today's `FileReference`
    construction and `file_exists_fn` handling.
  - Output: a table in `implementation-log.md` of accepted/rejected today
    versus required, which names exactly the structural checks to add to
    `biscuit-file`. No code changes; this is a read-and-run exercise on the
    current host.
- The spec's own figures already answer the performance question, so there is
  no performance spike: the baseline capture below is a normal task.

### Wave 1 — Inventory and Baselines

- [ ] **Dependency audit**
  - Confirm, by reading code, that Phase B seams exist: catalog `returns`
    (`darkmatter/schemas/partials/functions.yaml`, loaded by
    `compose/expression/catalog/mod.rs`, `expression_function_catalog()`),
    argument-type categories 1–5 and the compatibility API
    (`compose/expression/call_check.rs`: `ArgumentCategory`, `ArgumentType`,
    `classify_argument`, `check_call`, `check_expression_calls`,
    `check_prepared_calls`), passive literal classification (`check_call`
    binds all-literal calls through `FunctionRegistration::bind_call`),
    composition rejection (`compose/call_precheck.rs`,
    `MarkdownError::rejected_call()`), `[expressions] strict` /
    `dmls.expressions.strict` in DMLS config and its codes
    (`dmls/src/diagnostics/call_check.rs`, `codes.rs`), the projection
    function (`DeclaredTypes::top_level_types()` in
    `compose/expression/scope/projection.rs`), registration facts
    (`call_binding/registration.rs`: `EvaluationMode`, `reject_sources`,
    `numeric_recovery`), narrowing-fact registration slots, and
    `BindingView`/global catalog loading. Record file paths and signatures
    in `implementation-log.md`. Apply Rule 2 if any is missing.
- [ ] **Surface inventory** — parallelizable with Dependency audit
  - Map criteria 1–39 to existing tests and required additions
    (traceability table in `implementation-log.md`).
  - Inventory every consumer of expression analysis: Darkmatter
    `expression/{absence,binding,semantics,lint,prepared,path_projection}.rs`,
    `expression/{call_check.rs,scope/projection.rs}`, `call_precheck.rs`,
    `interpolation/evaluator.rs`, `directive_targets.rs`, DMLS
    `diagnostics/` (including `call_check.rs`) and `config/`, Claudine
    `composition/looping/config.rs` and `composition/lifecycle/validate.rs`.
    Mark each read-only scan that must move to a library API.
  - Inventory every legacy flag spelling (`--allow-*`, `--no-baseline-schema`,
    `--no-schema-triggers`, `--strict-style`) across `darkmatter/cli/src/args`,
    `commands/`, completions, justfiles, scripts, examples, docs, tests.
- [ ] **Performance baseline ("before")** — runs before any product edit
  - On the current host, pick one existing representative repository document
    and record composition time and DMLS analysis time, separating checking
    from unrelated I/O where practical. Record host, input, observations in
    `implementation-log.md`.
- [ ] **Run Spike S1** (see above).

**Checkpoint 1:** dependency audit passed (or work stopped and reported);
traceability table, inventories, baseline, and S1 table exist in the log.

## Phase 2 — Complete the Variable-Type Projection (spec §1; criteria 1–8, 31, 32)

Goal: every variable an expression can read gets an honest type, from the one
projection function, for every expression surface.

### Wave 2 — Type model and layer coverage

One integration owner for the projection module and the shared type/union
normalization; no competing edits.

- [ ] **Union normalization and optionality**
  - Implement the annex's union-normalization table (all rows) and `| null`
    optionality rules inside the existing type model; cycle-aware.
  - Tests: criterion 2 (every normalization row), order independence.
- [ ] **Root-union projection**
  - Arms contribute projected types; an arm not declaring the property
    contributes `unknown`; fold with normalization. Result independent of arm
    and key order and validator error order.
  - Tests: criterion 1 (`spec`, `design`, `review` → `unknown`; `plan` →
    `file`; `area` → `string | null`).
- [ ] **All effective-schema layers**
  - Project from the final `EffectiveSchema` after resolution and merging
    (imported named types, baseline, trigger, document), including layers
    `EffectiveSchema.simplified` does not represent.
  - Tests: criterion 3 (layered fixture), including pattern-only shapes
    declaring no literal name.
- [ ] **Raw JSON Schema projection** — parallelizable with Root-union work
  (disjoint module)
  - Annex's conservative projection: enumerable literal properties, `anyOf`,
    type arrays with `"null"`, `allOf`; anything indeterminate is `unknown`.
  - Tests: criterion 3 raw-JSON-Schema half.

### Wave 3 — Paths, reserved roots, calls, operators

- [ ] **Nested paths**
  - Constant members/indices yield declared member/item types with `| null`
    when absent-able (including unguaranteed array index); dynamic keys fall
    back to `unknown`. Follow recursive named types lazily along the requested
    path only.
  - Tests: criteria 4, 32 (finite path through a recursive type terminates
    without expanding the schema).
- [ ] **`fm` and precedence**
  - Type lookup follows value lookup: fixed namespaces, registered host
    globals, then document properties; `fm.x` equals unshadowed `x`; a global
    shadowing a property does not hide it through `fm`; an unavailable global
    keeps its binding diagnostic (even under `||`) and never falls back to a
    same-named property.
  - Tests: criterion 31.
- [ ] **Reserved roots and host globals**
  - Types from `darkmatter/schemas/darkmatter.yaml` (`fm`, `ctx`, `env`,
    `err`, `timing`, `current`, `seq`, `loop`, `group`) and the host catalog
    file as registered by lifecycle ownership; unavailable global gets no
    type.
  - Tests: criterion 5 (`err`, `ctx`, `timing`, Claudine `seq.index`).
- [ ] **Call and operator typing**
  - Call-as-argument: callee `returns`; overloads union the `returns` of every
    overload the argument count can select (no type-directed overload
    selection). `x || y` = `x` minus `null` ∪ `y`. `c ? a : b` = union of
    branch types (each under Phase D narrowing once Phase 3 lands; until then
    incoming types). Other operators `unknown`. These types reach the call
    check through `ArgumentType::of`, which today types every call and
    operator argument `unknown`.
  - Never evaluate a function or suppress an inner-call diagnostic.
  - Tests: criterion 6 (`min(length(count), 5)` and `min(maybe || 0, 5)`
    category 1 via the compatibility API and silent in DMLS under both
    `expressions.strict` settings; inner `length(count)` stays category 2).

### Wave 4 — Surfaces and passivity

- [ ] **Every surface uses the projection**
  - Body and frontmatter interpolation, conditional blocks, and lifecycle
    event action expressions (event globals typed per declaration). Remove any
    surface-local typing.
  - Tests: criterion 8 (`min(flag, 5)` in a lifecycle action rejected exactly
    like the body).
- [ ] **Passivity and stability proofs**
  - Failing I/O stubs and instrumentation: no file/network/env access, no
    expression execution; authored frontmatter, effective frontmatter, and
    `EffectiveState::data()` unchanged; existing schema-stage `null`
    materialization unchanged (eligibility, exclusions, idempotence,
    preservation of present values).
  - Tests: criterion 7.

**Checkpoint 2:** criteria 1–8, 31, 32 pass; `just test` and `just lint` pass in
`darkmatter/`; DMLS and composition agree on every projection fixture.

## Phase 3 — Flow-Sensitive Narrowing (spec §2; criteria 9–12, 33, 34)

Goal: one library narrowing engine, used identically by composition and DMLS.

### Wave 5 — Engine core

Single owner (new narrowing module plus the type-environment hook-ins).

- [ ] **Fact model and stable subjects**
  - Facts: non-null (internal, not a schema type), true-implies (suitable for
    type), true-iff (type / exclusion). Stable subjects only: resolved
    variable or constant-member/index path; bare and `fm` paths share a fact
    only when both resolve to the same document binding. Calls and dynamic
    indices are never subjects. Parentheses preserve facts; `!` swaps.
- [ ] **Truthiness and predicate guards**
  - Truthiness: true side removes `null`; false side proves nothing.
    `x ? a : b`, `!x ? a : b`, `x && f(x)`, `::block when="x"`.
  - Predicate facts from registration (Phase B slots): true-implies narrows
    true side only; true-iff narrows true side to the type and removes the
    type from a finite union on the false side; `unknown` minus a family stays
    `unknown`; narrowing preserves compatible refinements but never invents
    item types, members, or file existence.
- [ ] **Short-circuit and compound flow**
  - `a && b`: right side gets facts from `a` true. `a || b`: right side gets
    facts from `a` false. Compound guards keep only facts valid on every
    route (`is_number(x) || is_string(x)` → `number | string`;
    `is_number(x) || ready` → nothing). Restore incoming types after the
    region except common facts.
- [ ] **Impossible branches**
  - Keep checking with the incoming type for a contradicted subject; no never
    type. `is_number(flag) ? min(flag, 5) : 0` with `flag: boolean(required)`
    rejects the document, as does a false `::block` with `min(flag, 5)`.

### Wave 6 — Integration

- [ ] **Hook narrowing into every checking path**
  - Composition checking, the compatibility API, DMLS static walk, and
    lifecycle action expressions all call the same engine; conditional
    blocks narrow their bodies; nested blocks compose.
  - Tests: criterion 9 (rows g/h through all three paths), 10 (`x ? …`,
    `x && …`, `::block when="x"` category 1; unguarded category 2), 11
    (test-registered true-implies predicate does not narrow the false branch).
- [ ] **Fact lifetime in lifecycle flow**
  - Facts end at the guarded region and never cross events, loop iterations,
    retries, or proxy handoffs. `set` invalidates facts for the destination
    and overlapping ancestor/descendant paths before later actions; an
    undeterminable destination discards document-property facts. Re-entering
    a guard recomputes.
  - Tests: criterion 34 (guard, `set`, later read in runtime and DMLS;
    overlapping paths; event re-entry; shadowed `fm` path).
- [ ] **No mutation, no guard leakage**
  - Compare state immediately before/after analysis and evaluation in
    fixtures with no authored writes, separately from schema normalization;
    authored bytes unchanged.
  - Tests: criteria 12, 33 (negation and compound guards; false truthy branch
    does not prove null; guards on calls do not refine arguments; contradictory
    guard still reports a known-invalid call in the dead branch).

**Checkpoint 3:** criteria 9–12, 33, 34 pass; a parity test runs the same
fixtures through composition and DMLS and compares categories.

## Phase 4 — File Predicates and Guarded File Arguments (spec §3, §4; criteria 13–16, 18, 35)

Depends on Spike S1's table. Can start once Checkpoint 1 is done; the
`is_file_type` fact registration needs Phase 3's fact kinds, so the guarded-call
task waits for Wave 5.

### Wave 7 — Passive structural validation (`biscuit-file`)

- [ ] **Shared passive validator**
  - Add the checks S1 named to `biscuit-file` (public passive API; no
    resolution context, no filesystem/env/network). Covers local relative and
    absolute, Windows, project-relative, vault, recursive, HTTP(S),
    `&` repo-root, `^` repository-scoped, `~` home, and environment-template
    forms; a well-formed template is valid without reading variables.
    Malformed reserved forms, unsupported schemes, NUL, malformed templates,
    `""`, and `https://` are invalid. No new reference spelling or glob
    grammar; no second grammar in Darkmatter.
  - Host-independent acceptance: identical results on every OS.
  - Tests in `biscuit-file/`: full corpus, with failing access stubs.
- [ ] **`file://` adapter**
  - Preserve the existing `file://` input adapter ahead of the shared rules
    (`file://foo/bar` stays supported); do not collapse separators in a way
    that changes UNC or URL meaning. The adapter is not a new scheme.

### Wave 8 — Darkmatter functions

- [ ] **`is_file_type`**
  - New pure-dispatch function (display category `Filesystem`, works through
    the context-free expression API), declared in
    `darkmatter/schemas/partials/functions.yaml` as
    `is_file_type(value: unknown | null): boolean` with `category(Filesystem)`
    and inline examples. Declaration, handler, and true-implies
    "suitable for `file`" fact registered together. Add `is_file_type` to the
    expression-function verb lists in `darkmatter/schemas/partials/{lifecycle,md-lifecycle}.yaml`
    and `claudine/schemas/{action,lifecycle}.yaml`, which the lifecycle parity
    tests derive from the function catalog. True for file values and
    structurally valid strings (`42`, `hello world` true as strings; numeric
    `42` false); never mutates, resolves, or accesses anything. No
    `is_file` alias.
  - Tests: criteria 13, 35 (no resolution context; validator corpus).
- [ ] **`file_exists` / `fileexists`**
  - Keep Phase B binding (`file_exists(value: unknown | null)` inspects its
    argument: `file_exists(42)` and `file_exists(null)` are `false`).
    Validate the bound reference before path resolution or remote fetch;
    malformed returns `false` with no access. Valid references keep existing
    outcomes (local failure `false`; HTTP(S) without a remote runtime errors;
    with one, denied/failed is `false`, success `true`). Alias identical.
    Malformed input is valid input, not category 5.
  - Tests: criteria 14, 35 (`fileexists` parity).
- [ ] **Always-false warning**
  - Shared library analysis (composition and DMLS) warns when an authored
    literal argument is provably malformed; never for a merely `string`-typed
    argument; enabled by default; blocks under `--deny all`; analysis never
    calls `file_exists`; uses the same passive binding and validation.
  - Tests: criterion 15 plus composition-side warning (criterion 35).
- [ ] **No `file_exists`-specific setting**
  - Confirm DMLS config has none and none was added. Test: criterion 18.
- [ ] **Guarded file arguments**
  - `is_file_type(source) && frontmatter(source, 'title')` is category 1 with
    `source: unknown`; unguarded is category 4 (silent default; warns under
    `expressions.strict`). The guard rewrites nothing, proves nothing.
  - Tests: criterion 16 (both strictness settings).

**Checkpoint 4:** `just test`/`just lint` pass in `biscuit-file/` and
`darkmatter/`; the 13/14 corpus passes under failing stubs; `fileexists` and
`file_exists` agree.

## Phase 5 — One Function Contract for Host Extensions (spec §5; criteria 17, 37)

### Wave 9 — Registration and the facts artifact

- [ ] **Registration validation**
  - Registration rejects duplicate canonical names/aliases, built-in
    collisions, a fact referencing a nonexistent parameter, and a narrowing
    fact on a non-boolean function — all before evaluation. Runtime and
    editor receive the same resolved signatures and facts. Function catalogs
    and global catalogs are registered for their roles and never applied as a
    document schema.
  - Tests: criterion 37 (registration half).
- [ ] **Facts artifact writer (host build)**
  - Generate `<catalog stem>.facts.json` beside the host signature catalog
    from Rust registration: canonical name, per-fact parameter position, kind,
    type, plus laziness, conversion recovery, and source-type exclusions; the
    catalog content hash (`biscuit-hash`) and `format_version`. Deterministic
    byte output (sorted keys); the same generator path serves the test-host
    registration so a stale committed artifact is detectable.
- [ ] **Facts artifact reader (DMLS)**
  - Discover by naming convention whenever a registered signature catalog is
    loaded; read as inert data; verify hash and version. Missing, unreadable,
    mismatched, malformed, or incomplete artifacts produce an
    incomplete-analysis diagnostic for the affected host functions and never a
    category or narrowing result. Implement the Input Robustness Matrix table
    above and its single walking test, with the control row.

### Wave 10 — Claudine

- [ ] **Claudine function registration and artifact**
  - Register Claudine's host functions through the same Rust API as built-ins
    with `Catalog` declarations in the host catalog file; emit the artifact in
    the Claudine build and keep a freshness check.
- [ ] **Remove independent scans** — parallelizable with the previous task
  (disjoint files; coordinator integrates)
  - Move `claudine/lib/src/composition/looping/config.rs` and
    `composition/lifecycle/validate.rs` onto Darkmatter library APIs or
    remove them; Claudine keeps no definition of variable visibility or types.
    Run Claudine's L1 suite (`just test` in `claudine/`) for regressions.
- [ ] **Host predicate end-to-end**
  - Test-registered `is_project_file(value: unknown | null): boolean` with a
    true-implies `file` fact: category 1 in `is_project_file(source) &&
    frontmatter(source, 'title')` in composition and DMLS; a counter proves the
    host function is never invoked during analysis. Test: criteria 17, 37.

**Checkpoint 5:** criteria 17 and 37 pass; matrix test green; Claudine area
`just test` and `just lint` pass.

## Phase 6 — Shared Diagnostics (spec §6; criteria 19–22, 36)

### Wave 11 — Diagnostic model and absence rule

- [ ] **Shared diagnostic record**
  - Library owns code, severity, message, authored source range, expected and
    actual type. Composition, CLI, and DMLS adapt, never re-derive. Move
    Phase B's `dm.expression.incompatible_call` and
    `dm.expression.uncertain_call` from `dmls/src/diagnostics/codes.rs` into
    the library unchanged; they are the shared call codes. Preserve
    authored origin across frontmatter interpolation, transclusion, and
    lifecycle subtree composition; one-based positions in the library/JSON,
    adapters translate to editor coordinates. Blocking policy changes
    outcome, not code or severity.
  - Tests: criterion 19 (same category 5 call from composition and DMLS —
    the `report-json` leg is added in Phase 8).
- [ ] **CLI severities**
  - Composing commands report category 5 as errors (rejecting the document)
    and category 3 as warnings; categories 2 and 4 only under DMLS
    `expressions.strict`. Test: criterion 20 (composes with a warning; fails
    under `--deny all` once Phase 7 lands — assert the severity part now).
- [ ] **Type-based absence suppression**
  - Replace the catalog-name rule: a direct bare-variable argument to a
    parameter whose declared type explicitly includes `| null` suppresses
    `dm.expression.undeclared_property`. Plain `unknown` does not; optional
    non-null parameters do not; overloads suppress only when every
    argument-count candidate admits null at that position. "Direct" keeps its
    meaning (`is_empty(trim(x))` warns). Applies to host functions. Never
    hides unknown-function, unavailable-global, or binding errors.
  - Remove the function-name list from `AbsenceScope`
    (`expression/absence.rs`; callers `interpolation/evaluator.rs`,
    `expression/mod.rs`); DMLS applies the same rule in its static walk.
  - Tests: criteria 21, 36 (`is_null(x)`, `is_empty(x)`, `file_exists(x)`
    silent; `min(x, 5)`, `is_empty(trim(x))`, `is_number(x)` warn; mixed
    overload; host function with `| null`; `AbsenceScope` has no name list).
- [ ] **Privacy**
  - Diagnostics gain no evaluated argument values, environment values, or
    file contents; producing one never triggers evaluation or I/O. Instrumented
    test: criterion 22 (report leg added in Phase 8).

**Checkpoint 6:** criteria 19 (library+DMLS), 20 (severity), 21, 22, 36 pass;
DMLS and composition produce identical fields on the shared fixtures.

## Phase 7 — CLI Policy Verbs (spec §7; criteria 20, 23–28, 38)

All edits are in `darkmatter/cli` (`args/`, `commands/`, completions) plus the
library seam that exposes recovery and diagnostic-code identity. One owner for
the shared registry and args files; per-command wiring may be parallel after
the registry lands.

### Wave 12 — Registry

- [ ] **Single command registry**
  - One registry drives validation, help, and completions. Records, per
    identifier: kind (recovery, host, feature, style), the commands that
    accept it, and which diagnostic codes each recovery governs (selector
    identity). Fixed migration identifiers: `missing-hyperlinks`,
    `missing-image-refs`, `missing-transclusions`, `ctx-override`,
    `invalid-frontmatter-assignment`, `reassigned-frontmatter-property`,
    `shell-timeout`, `host=HOST`, `baseline-schema`, `schema-triggers`,
    `style`, plus `all`.
  - Rules: repeatable options; unknown, empty, or unsupported identifiers are
    usage errors; repeated identical selections harmless; `--allow` accepts
    only recovery/host identifiers (never arbitrary codes; known-invalid
    calls, parse errors, unavailable globals not allowable); `--disable`
    accepts feature identifiers and no `all`; `--deny host=HOST` is a usage
    error; `--disable baseline-schema` conflicts with `--baseline-schema PATH`.
- [ ] **Precedence and conflict resolution**
  - Specific beats `all` regardless of order; equally specific opposites
    (including a recovery spelled as a code) and `--allow all --deny all` are
    usage errors detected before any composition/render effect. `--allow all`
    covers only the non-access recoveries and never grants an unnamed host or
    authorizes shell execution. `--deny all` does not revoke host consent.
  - Tests: criteria 23, 38 (both orders; conflict detection via code
    spelling).

### Wave 13 — Per-command migration

Parallelizable by command (disjoint files) after Wave 12; coordinator owns the
shared registry and `args/command.rs`.

- [ ] **`md compose`**, **`md render`** (explicit and default `md FILE`),
  **`md clean`**, **`md schema validate`**
  - Replace policy flags with the three verbs; each command accepts only its
    meaningful identifiers. `--allow-any-missing-reference` becomes the three
    specific allowances. `--strict-style` → `--deny style` (informational
    notices never block).
  - Retained behavior per the migration table, including shell-timeout
    recovery yielding an empty string. Help text explains that allowance
    controls recovery while denial can still make its warning blocking.
  - Tests: criteria 24, 26, 38 (each registry mapping; baseline conflict;
    rejected host denials; disabled checks stay disabled and are not reported
    as passing).
- [ ] **Removed spellings**
  - Old flags are usage errors on every migrated command (no aliases).
    Update completions, justfiles, scripts, examples, docs, and tests together.
    Test: criterion 25 (every removed flag; grep for stragglers in repo
    scripts and docs).
- [ ] **Blocking and stopping**
  - `--deny all` makes warnings/errors blocking except recoveries selected by
    a more specific allowance; informational notices and off diagnostics never
    block; denial never executes a non-running branch; allowance never makes an
    invalid call valid. A blocking diagnostic known from available passive
    inputs stops before the next effectful step; one found after an effect
    stops later work with no rollback. Do not reorder the pipeline. A failed
    command writes no composed document to stdout and publishes no final file
    or viewer launch.
  - Tests: criteria 20 (`--deny all` fails the category 3 document), 27, 28
    (human and `--output json`; the `report-json` leg is in Phase 8).

**Checkpoint 7:** criteria 20, 23–28, 38 pass; `just test` and `just lint` pass
in `darkmatter/`; completions regenerate cleanly; no legacy flag spelling
remains outside tests that assert it is rejected.

## Phase 8 — Report Output and Performance Evidence (spec §8, §9; criteria 19, 22, 28, 29, 30, 39)

### Wave 14 — `report-json`

- [ ] **Output selector and gating**
  - Add `report-json` to `md compose --output`; every other command
    (`render`, default `md FILE`, `clean`, `schema validate`) rejects it as an
    unsupported output format before any work. `--output json` and human
    output unchanged; rendering JSON and multi-file schema-validation JSON
    keep their formats.
- [ ] **Report builder**
  - One object, fixed schema, every field always present: `outcome`,
    `diagnostics[]` (each with `code`, `severity`, `message`, `source`
    {`file`, `line`, `column`}, `expected_type`, `actual_type`; unavailable
    ones `null`), `document` (string on success, `null` on failure with
    non-zero exit). Report creation never evaluates or renders again; stdout
    carries only the object; progress/perf output goes to stderr; no
    duplicate human footer.
  - Any parsed invocation emits exactly one object, including input-read,
    schema-load, parse, binding, and runtime failures; earlier diagnostics
    survive later failure; failures without source/types use null fields and an
    existing typed code. Usage errors occur before report mode (stderr/exit as
    today). Diagnostic order: source order for passive checks, discovery order
    at runtime, concurrent children in authored directive order. No partial
    document streamed; final output and viewer wait for blocking checks.
  - Tests: criteria 29, 39 (input-read, schema, parse, runtime failures;
    success; warnings surviving failure; null fields; authored positions from
    transcluded content; one JSON object on stdout; non-compose rejection).
- [ ] **Parity and privacy legs**
  - Complete criterion 19 (identical fields across composition,
    `--output report-json`, DMLS) and criterion 22 (no argument values,
    environment values, or file contents; instrumented no-extra-I/O), and
    criterion 28's `report-json` leg.

### Wave 15 — Performance evidence

- [ ] **"After" measurement**
  - Repeat the Phase 1 baseline on the same host and input, composition and
    DMLS analysis. Record in `implementation-log.md`; explain any visible
    regression. No synthetic workload, extra host, repetition requirement, or
    CI job. Confirm existing expression depth/resource bounds are preserved
    and that bounded analysis stays conservative. Test: criterion 30 is the
    log entry plus a bounds regression test.

**Checkpoint 8:** criteria 19, 22, 28, 29, 30, 39 pass.

## Phase 9 — Documentation, Skills, and Final Verification

### Wave 16 — Docs and skills

Parallelizable (disjoint files). Audience: a developer with no experience of
this repository; compact example per rule; Mermaid for the narrowing flow, the
facts-artifact delivery path, and the CLI precedence rules; no page names or
links to a feature or fix.

- [ ] **Darkmatter docs**
  - README and current topic pages: expressions (projection, narrowing, guard
    limits, pure `is_file_type`, broad `| null` absence suppression), schemas,
    composition, CLI policy (verbs, registry identifiers, precedence,
    stopping), report output and failure behavior, DMLS settings
    (including the incomplete-analysis diagnostic), help text, and completions.
- [ ] **Claudine and biscuit-file docs**
  - Claudine host-integration docs (registration, catalog file, facts
    artifact, removed scans); `biscuit-file` reference docs if the public
    passive validation API changed.
- [ ] **Skills and drift**
  - Update `.claude/skills/` (darkmatter, claudine, biscuit-file; `rust-testing`
    only if a new test pattern emerged) and CLAUDE.md-level conventions only
    if repo-wide conventions changed. Run a comment-drift pass over every
    changed symbol. Update `docs/dependencies.md` if crates were added.

### Wave 17 — Final gates

- [ ] **Acceptance traceability**
  - Every criterion 1–39 mapped to a passing test in `implementation-log.md`;
    grep the matrix reader for the smell list; confirm no `.facts.json`
    drift and no leftover legacy flag spellings.
- [ ] **Package gates**
  - `just test` and `just lint` in `darkmatter/` (library, CLI, DMLS),
    `biscuit-file/`, and `claudine/`; `just test-l2` only where those areas
    already have affected L2 tests. Cross-OS: use the `os` skill before
    claiming any OS cannot be tested; no `#[cfg(windows)]` behavior is
    introduced without a path-comparison review.
- [ ] **Hand-off**
  - Write the final implementation-log summary (departures from the spec,
    rulings taken, drift found and how resolved) and stop at "implementation
    complete, ready for review." Do not commit, move the feature, or run
    `just complete`.

**Checkpoint 9:** all 39 criteria traceable; all gates green; docs and skills
updated; implementation log complete.

## Wave Index

| Wave | Phase | Concurrency |
| --- | --- | --- |
| 1 | 1 | Dependency audit, Surface inventory parallel; baseline and S1 then |
| 2 | 2 | Root-union and raw JSON Schema parallel; one owner for type model |
| 3 | 2 | Tasks mostly parallel after Wave 2; shared type module single owner |
| 4 | 2 | Sequential |
| 5 | 3 | Single owner |
| 6 | 3 | Sequential |
| 7 | 4 | Validator then adapter |
| 8 | 4 | `is_file_type` and `file_exists` parallel; warning after both |
| 9 | 5 | Registration, then writer, then reader |
| 10 | 5 | Claudine registration and scan removal parallel |
| 11 | 6 | Shared record first; severities and absence rule parallel after |
| 12 | 7 | Single owner |
| 13 | 7 | Per-command wiring parallel |
| 14 | 8 | Selector, builder, then parity |
| 15 | 8 | Sequential |
| 16 | 9 | Docs parallel |
| 17 | 9 | Sequential |

Every delegated brief must state that the entire session tree is
non-interactive, and must name its owned files.
