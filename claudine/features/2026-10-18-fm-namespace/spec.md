---
area: darkmatter
status: finalized-spec
created: 2026-10-10
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
review_iterations: 5
human_review: false
pipeline_step: 8
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-24-sequence-containers
    - 2026-10-27-darkmatter-lifecycle-ownership
implemented: true
---

# The `fm` Frontmatter Namespace

Authors read a document's own frontmatter through a reserved expression root.
Today that root is `doc`, which collides with an ordinary property name (the
shipped `prompts/clarify.md` has to write `doc.doc` to read its `doc`
property) and does not say what it holds. This spec renames the root to `fm`
("frontmatter") across Darkmatter and Claudine, returns `doc` to authors as an
ordinary property name, states the fixed list of reserved roots and how they
shadow frontmatter, and removes the `current_env` root, which contradicts the
goal that the environment is stable within each document lifecycle. Each
document captures `env` once, at the start of its own lifecycle; every
environment consumer of that document (expressions, context environment
values, file-reference expansion, schema roots, shell approval) reads that one
captured map, never an independent read of the process environment.

```md
---
doc: specs/2026-10-18-fm-namespace/spec.md
env: staging
---
Spec: {{ doc }}            <!-- the frontmatter property `doc` -->
Spec again: {{ fm.doc }}   <!-- the same property, through the namespace -->
Stage: {{ fm.env }}        <!-- "staging": `env` is a reserved root -->
Home: {{ env.HOME }}       <!-- the environment captured for this document -->
```

## Decided

Decided 2026-10-10.

### 1. `doc` is renamed `fm`, with no alias

- `fm` is the reserved root for the current document's frontmatter, with
  the semantics `doc` has today, with the literal-bracket dependency
  correction stated below:
  - bare `fm` is a value snapshot of the effective frontmatter at that
    evaluation, including caller overrides and injected frontmatter values.
    It is not the raw YAML or a frontmatter object frozen for the entire
    lifecycle. A later expression sees successful lifecycle `set` writes;
    an object returned by an earlier evaluation does not change;
  - reads do not execute stored strings as expressions or shell commands:
    produced values remain data, preserving Darkmatter's inserted-text rule;
  - `fm.<path>` is dotted traversal into it, and `fm['key']` bracket access
    reaches keys the identifier form cannot spell (`fm['release.channel']`);
  - a missing `fm.<path>` is absent (`null`) and never falls back to another
    root;
  - during frontmatter interpolation `fm.a` waits for the templated key `a`
    exactly as bare `a` does, and bare `fm` contributes no dependency. Bare
    `fm` contains only values resolved at that point, not a promise of every
    final value. Self-reference through `fm.a` keeps the existing cycle error;
  - literal bracket paths have the same dependency rule: `fm['a']` waits for
    `a`, and `fm['release.channel']` waits for the single frontmatter key
    `release.channel`, not `release`. This closes an existing gap in
    Darkmatter's [dependency collector](../../../darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs),
    which currently skips the bare namespace in an indexed expression.
    Dynamic indexes retain existing behavior; this change does not add a new
    scheduling model for computed property names;
  - in `$( … )` token resolution, `fm.build` bypasses the executable-first
    ladder and always reads the property;
  - a frontmatter property literally named `fm` is read as `fm.fm`.
- The rename is hard. `doc` is no longer reserved anywhere, and no alias,
  deprecation warning, or typed "renamed" error is added. After this spec,
  `doc` is an ordinary root: `doc.x` reads member `x` of a frontmatter
  property named `doc`; when no such property exists, it is absent (`null`)
  and reported through the existing `dm.expression.undeclared_property`
  advisory like any other undeclared property.

| Expression | Before | After |
| --- | --- | --- |
| `{{ fm.title }}` | property `fm`'s `title` (usually absent) | the frontmatter `title` |
| `{{ doc.title }}` | the frontmatter `title` | property `doc`'s `title` (usually absent) |
| `{{ doc }}` | the whole frontmatter object | the frontmatter property `doc` |
| `{{ doc.doc }}` | the frontmatter property `doc` | property `doc`'s `doc` member |
| `has_key(fm, 'x')` | property `fm` (usually absent) | whether the frontmatter has `x` |

- The rename covers every surface that classifies or resolves the root:
  Darkmatter's reserved-root catalog (`RESERVED_ROOTS` in
  `darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs`) and the
  helpers in `compose/expression/doc_namespace.rs`; every production site that
  special-cases the literal `doc` / `doc.` prefix (today:
  `compose/conditions.rs`, `compose/frontmatter_shell_expansion.rs`,
  `compose/context/effective_state.rs`, `compose/frontmatter_interpolation.rs`,
  `markdown/errors/blocks.rs`); DMLS root classification, completion, and
  hover. Internal Rust names follow the new spelling (for example
  `fm_namespace.rs`); the implementer chooses them.
- Claudine's hook-dispatch `doc` root is renamed **`hook`**, not `fm`
  (decided 2026-10-11). The hook payload is the event object an agent CLI
  (Claude Code, Codex, Gemini, …) sends to Claudine when one of its hooks
  fires, normalized by Claudine and read by dispatch templates, hook `when`
  conditions, and event matchers in `claudine/lib/src/dispatch/expression.rs`.
  It is not frontmatter, so it does not take the `fm` name:
  `{{ doc.tool_name }}` becomes `{{ hook.tool_name }}`, and bare `hook` is the
  whole payload. It is the same hard rename with no alias: after it, `doc`
  gets no special treatment in a hook-dispatch expression. The unprefixed
  event fields (`tool_name`, `extra.*`, `tool_input.*`, and the other existing
  paths) keep their current behavior. `hook` has exactly the old event-object
  shape: normalized event fields, `extra`, and the existing `os`, `hardware`,
  `git`, and `project` groups, with the same omission/null rules. It excludes
  the process `env` root. Missing `hook.<path>` is absent and never falls back
  to a bare field or another root; `hook.extra.doc` reaches an extra field
  named `doc`. The rename also covers harness validation messages, which use
  Claudine's [event expression adapter](../../lib/src/dispatch/expression.rs).
  This is a hook-only namespace, not another entry in Darkmatter's fixed
  frontmatter-root catalog, and hook expressions have no implicit
  frontmatter. Hook `env.*` behavior remains unchanged by this feature.
  Claudine's hook docs, dispatch configuration examples,
  tests, and the `claudine` skill are migrated in the same change.
- The authored global catalog `darkmatter/schemas/darkmatter.yaml` declares
  `fm` instead of `doc`; its partial `darkmatter/schemas/partials/doc.yaml`
  is renamed `partials/fm.yaml` (with `git mv`), its type key becomes `fm`, and
  its description says it is the `fm` root. Claudine's schema descriptions
  that say "a top-level key in the `doc` namespace"
  (`claudine/docs/schemas/partials/mutations.yaml`, `side-effect.yaml`) say
  `fm`.
- String values that merely contain the letters `fm` are unaffected: the hash
  kind `fm` (`md hash`'s `{fm}-{body}` simple form) is not an expression root.

### 2. The implicit namespace is `fm`, and reserved roots win

- A bare name is a frontmatter property, exactly as today: `{{ title }}` and
  `{{ fm.title }}` read the same value.
- Exception: a bare name that is a reserved root resolves to that root, never
  to the frontmatter. A frontmatter property whose name is a reserved root is
  read as `fm.<name>`.

```md
---
env: staging
ctx: notes
---
{{ env }}       <!-- the environment object, not "staging" -->
{{ fm.env }}    <!-- "staging" -->
{{ fm.ctx }}    <!-- "notes" -->
```

- Root classification order stays as Darkmatter documents it today: reserved
  root, then a global the host declared for the scope, then a document
  property (absent is `null`).

### 3. Reserved roots

The pipeline's end state reserves exactly these roots: `fm`, `ctx`, `env`,
`err`, `timing`, `current`, `seq`, `loop`, `group`. At this spec's point in
the pipeline the reserved set is:

| Root | Owner at this step | Meaning |
| --- | --- | --- |
| `fm` | Darkmatter (fixed) | the document's frontmatter (this spec) |
| `ctx` | Darkmatter (fixed) | context captured once per composition run |
| `env` | Darkmatter (fixed) | environment captured once per document lifecycle (section 4) |
| `current` | Darkmatter (fixed) | lazy mirror of `ctx`, observed when referenced |
| `err` | Claudine (declared in every lifecycle and approval scope) | the active failure |
| `timing` | Claudine (declared in every lifecycle and approval scope) | observed durations |
| `group` | Claudine (declared in every lifecycle and approval scope) | the enclosing group's variables |

- Darkmatter's fixed catalog becomes `["fm", "ctx", "env", "current"]`.
  `hook` is reserved only in Claudine hook dispatch, as described above;
  ordinary Darkmatter documents can still have a property named `hook`.
  `err`, `timing`, and `group` keep their current mechanism: Claudine declares
  them in the scope's binding declarations
  ([Claudine lifecycle bindings](../../lib/src/composition/lifecycle/bindings.rs)),
  and a declared global already wins over a document property, so inside every
  Claudine lifecycle event and approval scope they shadow frontmatter exactly
  like a fixed root (`fm.err`, `fm.group` read the document). Moving every
  global into one Darkmatter-owned declaration is
  `2026-10-27-darkmatter-lifecycle-ownership`.
- `loop.*` arrives with `2026-10-20-lifecycle-grammar` (replacing the
  `_loop_*` frontmatter properties) and `seq.*` with
  `2026-10-24-sequence-containers` (replacing `state`/`previous`/`next`).
  This spec does not reserve them.
- Other names found in today's binding code, which are **not** reserved roots
  and are left unchanged here:
  - `outputs` — a Claudine global declared only in the sequence shell
    approval scope (unavailable there); elsewhere it is a sequence-owned
    frontmatter-root property injected into each sequence step. Retired by
    `2026-10-24-sequence-containers` (`seq.outputs`).
  - `state`, `previous`, `next`, `sequence_id` — frontmatter-root properties
    injected into each sequence step (`claudine/docs/topics/flow-control/sequences.md`,
    "Reserved keys at the frontmatter root"). They are frontmatter, so
    `fm.state` reads the same value as `state`. Retired by
    `2026-10-24-sequence-containers`.
  - `_loop_count`, `_loop_is_first`, `_loop_is_last`, `_loop_last_output` —
    loop ambient frontmatter properties. Retired by
    `2026-10-20-lifecycle-grammar`.
  - `tracking` — an entry in the authored catalog
    `darkmatter/schemas/darkmatter.yaml` with no runtime root behind it (the
    runtime global is `timing`). Reconciled by
    `2026-10-27-darkmatter-lifecycle-ownership`.

### 4. `current_env` is removed; `env` is captured per document

- `env` is captured once at the start of each document's lifecycle (before
  its `initialize`) and does not change during that lifecycle, including
  approvals, provider preparation, loop iterations, retry/resume attempts,
  failure handlers, and `finalize`: loop iterations, retries, and resumes
  reuse the captured map. Each new document captures its own: each sequence
  task, each group leg, and each `proxy` target, so an environment change
  between tasks is seen by the next task. Transclusion does not start a
  lifecycle: an included file reads its including document's `env`. A plain
  `md compose` run is one document, so its behavior is unchanged.
- **Reader's note — this changes a request contract, not just a name.**
  Darkmatter's [composition request contract](../../../darkmatter/docs/topics/compose-requests.md#one-environment-per-request)
  currently makes `env.NAME`, context environment values, and environment
  expansion in file references read one captured map. Claudine also has
  [sequence resolution tests](../../lib/src/composition/sequence/tests.rs)
  and [proxy resolution tests](../../lib/src/composition/lifecycle/control/tests.rs)
  requiring reuse of the request environment. Those contracts move from
  "one map per request" to "one map per document": every consumer named
  above reads the document's captured map, and the tests are updated to
  supply a map per document. An implementer must not refresh only `env.*`
  while file references, schema roots, or shell approval use older values.
  Sequence shell approval happens before later tasks capture their
  environment; later tasks' commands are approved as typed templates and
  their `env` values are checked against the template at run time
  (`2026-10-23-shell-template-approval`), so a changed value never runs
  unchecked.
- A child shell cannot change Claudine's parent-process environment by running
  `export` (or its Windows equivalent). No new environment mutation action is
  introduced here. Tests of different document inputs use supplied maps;
  examples must not imply that a prior task's shell can update a later
  task's `env`.
- The `current_env` root is removed: from Darkmatter's reserved-root catalog
  and its lazy-root machinery (the refresh capability's environment reads in
  `compose/context/current.rs` and `compose/context/capture/capabilities.rs`,
  pre-flight's dynamic-shape detection of `current_env.*` reads in
  `compose/preflight/collect.rs`, and the nesting errors in
  `compose/expression/error.rs`), from Claudine's late-namespace list
  (`LATE_NAMESPACES` in `claudine/lib/src/composition/lifecycle/bindings.rs`,
  which becomes `["current"]`) and the approval refusals and error docs that
  name it (`claudine/lib/src/composition/error/mod.rs`), and from current
  documentation, skills, and tests of the retired lazy
  behavior. Tests and examples of the ordinary `current_env` property are
  allowed and must be identified in the migration audit.
  `rg -l current_env --type rust` lists 42 files today
  (2026-10-10), the largest being
  `darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs`,
  `compose/context/current.rs`,
  `claudine/cli/tests/level2/level2_ac28_lifecycle_lazy_roots.rs`,
  `darkmatter/lib/tests/l1/current_root_documentation_contract.rs`,
  `compose/tests/lazy_roots.rs`, and
  `darkmatter/cli/tests/l1/compose_lazy_roots.rs`.
- `current_env` gets no typed "removed" error. Like `doc`, it becomes an
  ordinary root: `current_env.HOME` reads a frontmatter property named
  `current_env` (absent is `null`, with the undeclared-property advisory).
- `current` (the lazy `ctx` mirror) is unchanged and stays refused in shell
  approval scopes.

### 5. Migration of repository documents

The same change migrates every repository use of the `doc` root, the
`current_env` root, and prose that names them:

- shipped prompts (`prompts/`, `darkmatter/prompts/`, `claudine/prompts/`),
  for example `prompts/clarify.md`'s `doc.doc` → `doc` (or `fm.doc`); the
  implementing agent edits them;
- test fixtures that mirror shipped prompts (for example
  `claudine/lib/tests/fixtures/frozen_prompts/prompts/clarify.md`) and every
  Rust test, inline test document, and DMLS fixture that writes `doc.` or bare
  `doc` as a root;
- Darkmatter and Claudine `docs/` topic pages, READMEs, and DMLS docs
  (`darkmatter/docs/topics/darkmatter-expressions.md` Namespaces and Host
  Bindings sections and its root-classification Mermaid diagram,
  `darkmatter/docs/inline/fm-interpolation.md`, `inline/interpolation.md`,
  `topics/state-management/index.md`, `topics/compose-requests.md`,
  `claudine/docs/topics/flow-control/lifecycle.md`,
  `topics/state-management/context-variables.md`, `topics/composition.md`,
  `topics/pre-flight-checks.md`, `darkmatter/dmls/docs/diagnostics.md`, and
  any other page the search finds);
- every Claudine hook-dispatch template, `when` condition, and event matcher
  that writes `doc.` or bare `doc` for the hook payload, in configuration
  examples, docs, and tests, which become `hook.` / `hook` (section 1);
- skills: `.claude/skills/darkmatter/` (`compose.md`, `schema.md`,
  `library-surfaces.md`, `testing.md`) and `.claude/skills/claudine/`
  (`cli-commands.md`, `cli-reference.md`); add a dated entry to
  `.claude/skills/claudine/timeline.md` rather than rewriting earlier entries.

Historical records are not migrated: feature and fix specs, reviews, research
notes, and existing timeline entries keep the names they were written with.

### 6. Migration boundaries and passive tooling

The hard rename deliberately breaks external documents and hook configuration
that used the old roots. Update current README/topic documentation with a
compact before/after table for both `doc` → `fm` and hook `doc` → `hook`,
plus the removal of live environment reads. No automatic rewriting of user
files is introduced: a property named `doc` or `current_env` is valid data,
so a blind replacement can change a document's meaning.

Migrate authored expression spans only. Leave file paths, ordinary YAML keys,
provider payload keys, escaped examples, literal tokens, and inserted strings
alone unless their documented meaning actually refers to an old root.
Expression and shell-command validators, advisory classification, and editor
features must consume the updated catalog without evaluating expressions,
running shells, observing `current`, or reading the process environment.
DMLS (Darkmatter's language server) must still offer `doc` and `current_env`
as ordinary properties when the document or its schema declares them. Their
removal as built-in roots must not hide legitimate document keys.

Keep dotted `fm` lookup's existing cost: traverse the effective map and copy
only the returned value. Only bare `fm` needs the whole-object snapshot.
This rename needs no performance spike or benchmark matrix.

## Out of scope

- The lifecycle grammar and the `loop.*` root: `2026-10-20-lifecycle-grammar`.
- The `seq.*` and `group.*` container roots and the retirement of `state`,
  `previous`, `next`, `outputs`, and `sequence_id`:
  `2026-10-24-sequence-containers`.
- One file declaring every global, Darkmatter ownership of `err`, `timing`,
  and `group`, and reconciling the catalog's `tracking` entry:
  `2026-10-27-darkmatter-lifecycle-ownership`.
- Expression typing beyond today's: `2026-10-28-schema-enhancements` and
  `2026-10-31-type-aware-evaluation`.

## Acceptance criteria

1. **Rename complete.** Outside historical records (`**/features/**`,
   `**/fixes/**`, `**/reviews/**`, `**/research/**`,
   `.claude/skills/claudine/timeline.md` entries dated before this change),
   audit authored expressions for old frontmatter-root and hook-payload-root
   uses. These searches are discovery aids, not proof of semantic correctness:
   they also match legitimate `doc` properties and may miss bracket access,
   multiline expressions, shell tokens, and ordinary expression-valued fields.

   ```sh
   rg -nP '\{\{[^}]*(?<![\w.\-/])doc\b(?!\.(md|yaml|txt))' \
     -g '!**/features/**' -g '!**/fixes/**' -g '!**/reviews/**' -g '!**/research/**'
   rg -nP 'when[:=].*(?<![\w.\-/])doc\.[A-Za-z_]' \
     -g '!**/features/**' -g '!**/fixes/**'
   rg -nP '"(!?\s*)doc(\.[A-Za-z_][\w-]*)+"' -g '*.rs' | rg -v 'doc\.(md|yaml|pdf|txt)'
   ```

   Use the existing passive expression extractors for the additional forms;
   do not rely on global text replacement. Remaining hits are allowed for
   ordinary properties, their documentation/tests, or unrelated paths and
   strings. The implementation log classifies remaining expression hits and
   confirms coverage of dispatch and harness validation as well as document
   composition. No production Rust code special-cases the literal `doc` root.
2. **`fm` behaves as `doc` did.** Darkmatter L1 tests cover bare `fm` (whole
   object), `fm.<path>`, `fm['key']`, absent `fm.<path>` → `null` with no
   fallback, frontmatter-interpolation ordering (`fm.a` waits for templated
   `a`; bare `fm` adds no dependency), `$( … )` token resolution of
   `fm.build`, and `fm.fm` for a property named `fm`. Cover literal bracket
   dependencies (including a key containing a dot), self-reference cycles,
   effective caller overrides, and a lifecycle `set` followed by both bare
   and qualified reads of the updated value. Claudine L1 tests cover
   hook-dispatch `hook.<field>` and bare `hook` resolving the hook payload in
   a dispatch template, a hook `when` condition, and an event matcher, and
   `doc.<field>` no longer resolving the payload, missing qualified fields
   without fallback, unchanged unprefixed fields, nested `hook.extra.doc`,
   and harness validation messages using the renamed root.
3. **`doc` is userland.** With frontmatter `doc: {title: X}`, `{{ doc.title }}`
   renders `X` and `{{ doc }}` is that object; with no `doc` property,
   `{{ doc.title }}` renders empty and compose reports
   `dm.expression.undeclared_property`. A `$schema` that declares `doc` types
   it like any other property. `current_env.HOME` behaves the same way (reads
   a frontmatter `current_env`).
4. **Reserved roots shadow frontmatter.** With frontmatter keys `fm`, `ctx`,
   `env`, and `current`, the bare names resolve to the roots and `fm.<name>`
   to the frontmatter values, in `md compose` and in a Claudine lifecycle
   event. In a Claudine lifecycle event, frontmatter keys `err`, `timing`, and
   `group` are read as `fm.err`, `fm.timing`, `fm.group`, and the bare names
   resolve to (or report unavailability of) the globals. Darkmatter's
   reserved-root catalog test asserts exactly `["fm", "ctx", "env", "current"]`.
5. **Live environment root removed.** `current_env` has no reserved-root,
   refresh, preflight, or approval behavior; Claudine's late-namespace list
   is `["current"]`. Audit remaining text as historical records or examples
   and tests of the ordinary property. Tests prove `env` stays fixed across
   every event, loop iteration, and retry/resume attempt in one document
   lifecycle. Tests using supplied environment maps prove each new document
   captures its own map: a later sequence task sees a map changed between
   tasks, each group leg captures its own, and a `proxy` target captures its
   own; within one document, approvals, file references, and `env.*` read
   the same map. No new test mutates the real process environment.
6. **Editor support.** DMLS completes `fm` (not `doc`) as a built-in root and the
   frontmatter properties after `fm.`, hovers `fm` with its catalog
   description, no longer offers `current_env` as a built-in root, and treats
   `doc.` like any other property path. Declared ordinary `doc`, `current_env`,
   and `hook` properties still complete and receive schema-based hover and
   diagnostics. Existing undeclared-property suppression and warning identity
   rules apply to the released names; passive tooling performs no observations.
7. **Docs and skills updated.** The pages and skills listed in Decided §5
   describe `fm`, the scoped reserved-root list, and per-document environment
   capture, with `current_env` appearing only as an ordinary property
   or in explicit migration guidance; the root-classification Mermaid
   diagram names
   `fm ctx env current`. No `docs/` page names this spec.
8. **Gates.** `just test` and `just lint` pass in `darkmatter/` and
   `claudine/`; Claudine's L2 lazy-root test is updated to cover `current`
   only and runs under `just test-l2`. Tests use the repository Test Toolkit,
   never focus terminal or browser windows, and are portable to macOS, Linux,
   native Windows, and WSL2. Run the existing opt-in prompt corpus checks
   locally for the changed shipped prompts; CI never tests internal prompt
   folders. Frozen prompt fixtures and tests of current schemas cover their
   renamed expressions through the normal invocation path. No CI cells are
   added for this feature.

## Open questions

None.
