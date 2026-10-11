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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 8
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-24-sequence-containers
    - 2026-10-27-darkmatter-lifecycle-ownership
---

# The `fm` Frontmatter Namespace

Authors read a document's own frontmatter through a reserved expression root.
Today that root is `doc`, which collides with an ordinary property name (the
shipped `prompts/clarify.md` has to write `doc.doc` to read its `doc`
property) and does not say what it holds. This spec renames the root to `fm`
("frontmatter") across Darkmatter and Claudine, returns `doc` to authors as an
ordinary property name, states the fixed list of reserved roots and how they
shadow frontmatter, and removes the `current_env` root, which contradicts the
rule that the environment is captured once per document.

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
  exactly the semantics `doc` has today:
  - bare `fm` is the whole frontmatter object (a snapshot);
  - `fm.<path>` is dotted traversal into it, and `fm['key']` bracket access
    reaches keys the identifier form cannot spell (`fm['release.channel']`);
  - a missing `fm.<path>` is absent (`null`) and never falls back to another
    root;
  - during frontmatter interpolation `fm.a` waits for the templated key `a`
    exactly as bare `a` does, and bare `fm` contributes no dependency;
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
  gets no special treatment in a hook-dispatch expression. Claudine's hook docs, dispatch configuration examples,
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
  `err`, `timing`, and `group` keep their current mechanism: Claudine declares
  them in the scope's `BindingView` (`claudine/lib/src/composition/lifecycle/bindings.rs`),
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
    approval scope (unavailable there); elsewhere it is a reserved
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
  its `initialize`) and does not change during that lifecycle. In a sequence,
  each task's document captures `env` at its own start, so an environment
  change between tasks is seen by the next task. Transclusion does not start a
  lifecycle: an included file reads its including document's `env`. A plain
  `md compose` run is one document, so its behavior is unchanged.
- The `current_env` root is removed: from Darkmatter's reserved-root catalog
  and its lazy-root machinery (the refresh capability's environment reads in
  `compose/context/current.rs` and `compose/context/capture/capabilities.rs`,
  pre-flight's dynamic-shape detection of `current_env.*` reads in
  `compose/preflight/collect.rs`, and the nesting errors in
  `compose/expression/error.rs`), from Claudine's late-namespace list
  (`LATE_NAMESPACES` in `claudine/lib/src/composition/lifecycle/bindings.rs`,
  which becomes `["current"]`) and the approval refusals and error docs that
  name it (`claudine/lib/src/composition/error/mod.rs`), and from every doc,
  skill, and test. `rg -l current_env --type rust` lists 42 files today
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
   these searches return no expression that uses `doc` as the frontmatter
   root:

   ```sh
   rg -nP '\{\{[^}]*(?<![\w.\-/])doc\b(?!\.(md|yaml|txt))' \
     -g '!**/features/**' -g '!**/fixes/**' -g '!**/reviews/**' -g '!**/research/**'
   rg -nP 'when[:=].*(?<![\w.\-/])doc\.[A-Za-z_]' \
     -g '!**/features/**' -g '!**/fixes/**'
   rg -nP '"(!?\s*)doc(\.[A-Za-z_][\w-]*)+"' -g '*.rs' | rg -v 'doc\.(md|yaml|pdf|txt)'
   ```

   The only permitted hits are in tests that pin the new userland meaning of
   `doc` (criterion 3), and the implementation log lists each remaining hit
   with the reason it is not a frontmatter-root use. No production Rust code
   special-cases the literal `doc` root.
2. **`fm` behaves as `doc` did.** Darkmatter L1 tests cover bare `fm` (whole
   object), `fm.<path>`, `fm['key']`, absent `fm.<path>` → `null` with no
   fallback, frontmatter-interpolation ordering (`fm.a` waits for templated
   `a`; bare `fm` adds no dependency), `$( … )` token resolution of
   `fm.build`, and `fm.fm` for a property named `fm`. Claudine L1 tests cover
   hook-dispatch `hook.<field>` and bare `hook` resolving the hook payload in
   a dispatch template, a hook `when` condition, and an event matcher, and
   `doc.<field>` no longer resolving the payload.
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
5. **`current_env` absent.** `rg -n current_env` finds no match outside
   historical records, and Claudine's late-namespace list is `["current"]`.
   Tests prove `env` is fixed within one document lifecycle (every event,
   including each loop iteration, reads the same `env`), and that in a
   sequence a second task observes an environment value changed after the
   first task started, using an injected environment observation (no test
   mutates the real process environment).
6. **Editor support.** DMLS completes `fm` (not `doc`) as a root and the
   frontmatter properties after `fm.`, hovers `fm` with its catalog
   description, no longer offers `current_env`, and treats `doc.` like any
   other property path.
7. **Docs and skills updated.** The pages and skills listed in Decided §5
   describe `fm`, the reserved-root list of §3, per-document `env` capture, and
   contain no `current_env`; the root-classification Mermaid diagram names
   `fm ctx env current`. No `docs/` page names this spec.
8. **Gates.** `just test` and `just lint` pass in `darkmatter/` and
   `claudine/`; Claudine's L2 lazy-root test is updated to cover `current`
   only and runs under `just test-l2`. Tests use the repository Test Toolkit,
   never focus terminal or browser windows, and are portable to macOS, Linux,
   native Windows, and WSL2. No CI cells are added.

## Open questions

None.
