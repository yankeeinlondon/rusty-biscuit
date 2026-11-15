---
total_phases: 4
created: 2026-11-14
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
---

# Plan — One mutation root for every lifecycle host

## Summary and Definition of Done

Today every lifecycle host picks its own mutation root — the directory within
which file-writing lifecycle actions may write. `md compose` uses the composed
document's directory (`darkmatter/cli/src/commands/compose/lifecycle.rs:377-381`);
Claudine uses the launch workspace's `child_cwd` on most routes
(`claudine/cli/src/commands/compose/prep.rs:1134`,
`wrap/composition/pipeline.rs:1362`, `wrap/composition/staged_boot.rs` via its
caller, `wrap/harness_orch/loop_control.rs:334`), the sequence source's tree on
the sequence route (`wrap/sequence/mod.rs:283`), and the task's authoring
directory on task routes (`wrap/sequence/task_run.rs:111,257`). The same
document can therefore write under one host and be refused under another.

This fix makes every host use one boundary — the caller's captured launch
scope's `local_root()` (the Git working tree top level containing the caller's
working directory at launch, else that directory) — and makes every
file-writing lifecycle action (seven frontmatter writers, both `ensure_file`
overloads, `ensure_dir`, `append_line`, `append_jsonl`, and `edit`) select its
target through the unchanged file-reference contract, then check that target
against the boundary before anything is opened, created, or written. A missing
target is created at the contract's first candidate (beside the document for an
implicit relative path), never at `mutation_root.join(raw)`.

All target-resolution logic lives in Darkmatter's lifecycle executor; Claudine's
executor is a thin adapter over `StackInputs`
(`claudine/lib/src/composition/lifecycle/executor.rs:466-484`), so Claudine only
rewires the root its engines are constructed with. biscuit-file needs no
changes: `LaunchMagicScope::local_root`
(`biscuit-file/lib/src/file_reference/context.rs:194-197`), `PathIdentity`
(`.../portable/path_identity.rs:55`), and `canonicalize_simplified`
(`biscuit-file/lib/src/path_text.rs:61`) already exist.

**Definition of Done:**

- Every production `EffectEngine` construction for a lifecycle host sets its
  mutation root to the request's `FileResolutionContext::launch_magic_scope()
  .local_root()`; no other root-selection remains on any lifecycle path.
- `mutation_target` resolves every file-writing action's operand through the
  file-reference contract, refuses a URL or an unmatched recursive reference
  without fetching or writing, creates a missing target at the contract's first
  candidate, and refuses a target outside the boundary with an error naming the
  target and the effective root before any parent directory is created, any
  document is loaded, or any editor is invoked.
- The seven frontmatter verbs resolve through that same resolver; a successful
  write to the active document is mirrored into live state by comparing the
  resolved target's identity (canonical-with-missing-tail, compared through
  `PathIdentity`), and a write to any other file never mutates the active
  document's state.
- The boundary stays fixed across initialization, recomposition, `prep`/`proxy`,
  sequence members and containers, parallel groups, loop iterations, retries,
  and resumed attempts, while the authoring base follows the active document.
- Every verification bullet in the spec's Verification section has a
  deterministic test at the library or CLI-fixture level; existing tests that
  encoded the old root-join creation are rewritten to the new contract.
- The four behavior documents, the repo skills, and the caller-skill sources
  state the new rule; `just publish-skills` ran in both areas; `just test` and
  `just lint` pass in `darkmatter/` and `claudine/`.
- No performance spike (the spec records none is needed: no new discovery pass
  runs). No Input Robustness Matrix applies: no parser, config loader, or
  deserializer gains a new input shape — the change reselects targets and adds
  a containment check; the file-reference grammar itself is unchanged.

---

## Phase 1 — Darkmatter library: one resolver and one boundary check

### Necessary Rules

Every choice the specification left to the implementer is settled here. Rules
are decisions, not questions; each carries its evidence.

**R1 — One boundary selector, read from the request's captured launch scope.**
Every lifecycle engine construction sets `mutation_root` to
`<request file-resolution context>.launch_magic_scope().local_root()`. That
method returns the launch repository root when one was captured, else the
launch directory (`biscuit-file/lib/src/file_reference/context.rs:192-197`),
which is exactly the author ruling (spec, "Author rulings"). No new
root-selection function or repository scan is written (spec: "Every lifecycle
engine construction uses this selector; there is no need for another
root-selection function or repository scan").

**R2 — The concrete context object at each construction site.** The launch
scope is preserved verbatim by every derivation and rebuild
(`FileResolutionContext::for_source` keeps it, context.rs:744-745; Darkmatter's
`document_context` restores it after an external rebuild,
`darkmatter/cli/src/request.rs:303`; Claudine's `derive_source` threads it,
`claudine/lib/src/invocation_context.rs:1461-1467`), so any of the request's
contexts answers identically. Use the one in scope at each site:

| Site | Today | Becomes |
|---|---|---|
| `darkmatter/cli/src/commands/compose/lifecycle.rs:377-381` | `source_path.parent()` | `compose.request.resolution_context().launch_magic_scope().local_root().to_path_buf()` |
| `claudine/cli/src/commands/compose/prep.rs:1134` (loop route) | `prep_context.launch_workspace.child_cwd` | `loop_prepare_options.file_resolution_context.launch_magic_scope().local_root()` |
| `claudine/cli/src/commands/compose/prep.rs:1965-1970` (staged single, into `StagedLifecycleRuntime::new`) | `prep_context.launch_workspace.child_cwd` | `options.file_resolution_context.launch_magic_scope().local_root()` |
| `claudine/cli/src/commands/wrap/composition/pipeline.rs:1362` | `env_plan.child_cwd` | `request.prepared.input_layers.file_resolution_context.launch_magic_scope().local_root()` |
| `claudine/cli/src/commands/wrap/sequence/mod.rs:283` | `source_context.base_dir()` | `file_resolution_context.launch_magic_scope().local_root()` (the function's existing parameter) |
| `claudine/cli/src/commands/wrap/sequence/task_run.rs:110-111` | `task.origin_dir` | `task_source_context.file_resolution_context().launch_magic_scope().local_root()` |
| `claudine/cli/src/commands/wrap/sequence/task_run.rs:257` (root step guard) | `task.origin_dir` | `source_context.file_resolution_context().launch_magic_scope().local_root()` (the `source_context` built at line 252-256) |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:334` | `run.child_cwd` | `run.prompt_state.file_resolution_context().launch_magic_scope().local_root()` |

`StagedLifecycleRuntime::new` keeps its `mutation_root: &Path` parameter (its
caller now passes the launch-scope root); update its inline comment, which
currently says "the launch workspace". `HarnessPromptState::file_resolution_context()`
exists at `claudine/cli/src/commands/wrap/harness_orch/types.rs:124-127`, and
`PrepareOptions.file_resolution_context` is a public field (used at
`harness_orch/prompt.rs:238-245`).

**R3 — Discovery failures fail closed, unchanged; no new discovery runs.**
The fix adds no repository scan; hosts read an already-captured scope. A
corrupt repository is an error, not an absent one: sniff's `trusted_discover`
reports corruption as `SniffError::git("discover", …)` and only upward-search
exhaustion means "no repository" (`sniff/lib/src/filesystem/git/open.rs:32-49`);
Darkmatter maps that to `ContextBuildError::Discovery`
(`darkmatter/lib/src/markdown/compose/context/request.rs:298-315`) and Claudine's
`InvocationContext::capture` propagates it, both before any lifecycle action
runs. The launch-directory fallback is therefore reachable only after a
successful observation of no working tree. Tests pin this with a corrupt-`.git`
fixture (R10-test 4).

**R4 — Target selection algorithm in `mutation_target`.** Replace
`darkmatter/lib/src/lifecycle/executor.rs:1262-1286` with:

1. Parse with `FileReference::new` (existing, both entry points) and resolve
   with `reference.resolve_in_context(&document_context)` where
   `document_context = self.inputs.file_resolution_context.for_source(self.inputs.source_path)`
   (unchanged). `Ok(Some(path))` is the target — an existing file is the
   contract's lookup result.
2. `Ok(None)` with URL kind is already refused without fetching (a URL never
   reaches a probe: `biscuit-file/.../resolve.rs:104-112` returns
   `RemoteNotLocal`; `resolve_edit_target` refuses earlier with
   `EditError::UrlRefused`, executor.rs:1242-1252). Keep both; do not
   reimplement.
3. `Ok(None)` with `reference.class().recursive == true` is refused with the
   existing `MutationTargetError::NoLocalTarget` channel, message naming the
   reference and saying a search directory is never a new filename. Today the
   code wrongly takes `candidate_plan`'s first entry, which for a recursive
   reference is a search *root* (a directory), per
   `biscuit-file/.../mod.rs:976-981`.
4. `Ok(None)` otherwise resolves to the first `candidate_plan(&document_context)`
   candidate — for every kind, including implicit relative. **Delete the
   `ImplicitRelative → mutation_root().join(raw)` branch** (executor.rs:1275-1277);
   the spec names it as the behavior being replaced. This gives the author
   ruling's example: `edit: notes.md` opens `prompts/notes.md` if it exists,
   else an existing root-level `notes.md`, else creates `prompts/notes.md`.
5. Check containment with `crate::effects::fs_write::ensure_within(self.inputs.effect_engine.mutation_root(), &target)`
   (already `pub(crate)`, used cross-module at `editor/service.rs:32`) and
   return the cleaned path. An `EffectError::OutsideMutationRoot` failure —
   whose display names the target and the root
   (`darkmatter/lib/src/effects/error.rs:15-16`) — flows through
   `error_or_action` as an action dispatch failure honoring `no_error`, the
   spec's "Path-resolution and containment failures remain action dispatch
   failures". `ensure_within` canonicalizes the deepest existing ancestor and
   judges a missing tail lexically (`fs_write.rs:125-143`), so symlink escapes
   and sibling text prefixes are refused while missing targets work.

The engine's own root-relative `resolve` (verbs.rs:15-24) is untouched: direct
low-level callers keep the existing explicit-root API, per the spec.

**R5 — The seven frontmatter verbs route through the resolver.** In
`dispatch_side_effect` (executor.rs:1019-1026), `set_frontmatter`,
`merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`,
`decrement_frontmatter`, `append_frontmatter`, and `prepend_frontmatter` take
their file operand through the same `path(0)` helper the file verbs use
(executor.rs:1004-1007), so the engine receives an absolute, resolved,
containment-checked target. Resolve operand 0 exactly once per action, pass
that same resolved path string to the engine call **and** the resolved
`PathBuf` to the mirror (R6) — the spec's "Pass that same resolved target to
the operation and, when applicable, the live-frontmatter update. Do not resolve
the operand again from a different base." Restructure the match arms so the
frontmatter verbs bind `let target = path(0)?;` once and hand
`target.to_string_lossy()` (or a `Cow<str>`) to the engine; keep `http_post`
and `set` exactly as they are.

**R6 — Mirror identity through the resolved target.** Change
`mirror_frontmatter_mutation` (executor.rs:1373-1420) to take the resolved
target `&Path` instead of reading `values.first()` as text, and rewrite
`targets_document` (executor.rs:1427-1438) to compare identities: canonicalize
both the resolved target and `self.inputs.source_path` with the missing-tail
rule and compare through `biscuit_file::PathIdentity`. Make
`fs_write::canonicalize_with_missing_tail` (fs_write.rs:151) `pub(crate)` and
reuse it rather than writing a second canonicalizer — it is the "existing
canonical-path rules where filesystem identity is needed" the spec names.
`lexical_normalize` in the executor (executor.rs:2010, used only by the old
`targets_document`) is deleted with it. `canonicalize_simplified` is used only
when a canonical path leaves a private comparison (as the editor service
already does); this comparison stays private, so it is not needed here. A
mutation of any other file must not update the active document's live state.

**R7 — The engine guards reads that prepare a mutation.**
`EffectEngine::load` (verbs.rs:27-36) runs
`ensure_within(self.mutation_root(), &path)` before `read_to_string`, so a
direct low-level caller cannot have an outside target read before its write is
refused (spec: "Engine reads that prepare a mutation must check containment
before loading the target"). The guarded write (`atomic_write_guarded`) and
editor rechecks (`editor/service.rs:175,178,202`) stay as they are.

**R8 — Everything else is preserved.** Failure classification and `no_error`
routing, located diagnostics, `side_effect_attempted()` read-cache
invalidation after an attempted effect or edit (including an attempt that
wrote before failing), exclusive `create_new` behavior, editor symlink and
nonregular-entry refusals, frontmatter hashing and literal-value storage,
event ordering, shell approval, and network allowlists are unchanged.
`http_post` and `set` do not resolve file targets. Read-side expression
functions and ordinary file-reference search rules outside lifecycle mutation
are untouched (spec, "Out of scope").

**R9 — File-less input uses its captured authoring directory.** `md compose`
already synthesizes `launch_dir.join("<stdin>")` as the lifecycle source
(`darkmatter/cli/src/commands/compose.rs:257`) and uses the launch context for
stdin (compose.rs:242-245); `for_source` then yields the launch directory as
the authoring base. Inline groups and tasks use the containing document's
context (already the case at `task_run.rs:134,137`). No change is needed; tests
pin stdin behavior.

**R10 — Tests that encode the old contract, and their new expectations.**

1. `darkmatter/lib/src/lifecycle/executor/tests.rs:574
   a_missing_bare_relative_target_is_created_under_the_mutation_root` —
   retitle and re-assert: a missing bare relative target is created at the
   document's directory (the contract's first candidate). In that fixture the
   document sits at the fixture root so the path is unchanged; add a
   document-in-subdirectory variant that proves creation lands beside the
   document, not at the engine root.
2. `claudine/cli/tests/l1/lifecycle_mutation_root.rs:74
   lifecycle_relative_writes_land_in_launch_repo_when_prompt_lives_in_another_repo`
   — under the new contract a relative `events.log` authored by a prompt in the
   foreign library resolves to the library side (its first candidate), which is
   outside the caller's root, so the action is refused. Rewrite the test to
   assert: the run fails with `outside the mutation root` naming the target
   and the caller's repo root, `events.log` is created nowhere (neither
   library nor launch repo), and — as the positive path, replacing the old
   assertion — an absolute target inside the caller's root (the shape the
   second test in the file already uses) still writes. Update the module doc
   comment, which describes the removed behavior.
3. Any other expectation that breaks when the suites run (sequence/task/loop
   fixtures writing relative markers with documents outside the caller root)
   is corrected to the new rule — creation beside the authoring document, or
   refusal naming target and root — never to a root-joined creation. The
   fixtures in `compose_initialize_acceptance.rs` and `sequence_containers.rs`
   keep passing unchanged where their documents sit inside the caller tree,
   because the document directory is then inside the boundary.
4. New boundary-selection tests cover: inside a repository (root = top level),
   outside any repository (root = launch dir), launch from a monorepo package
   subdirectory (root = the repository top, not the package directory), launch
   inside a linked worktree (`git worktree add`; root = the worktree's own
   top, never the main checkout — sniff distinguishes them,
   `sniff/lib/src/filesystem/git/worktree.rs:55-57`), and a corrupt `.git`
   directory at launch (a `.git` directory with a garbage `HEAD` file) where
   the command fails before any file action (R3).

**R11 — Documentation and skills updated in this change.** The four behavior
pages are `darkmatter/docs/topics/lifecycle/index.md` (rewrite "Creating the
file" bullets at ~lines 554-576 and add the boundary/base distinction, the
subdirectory example, the shared-prompt case, and migration away from
root-relative creation), `darkmatter/docs/topics/side-effects.md`
(mutation-root section at lines 72-79 plus the document-aware lifecycle
resolver versus the low-level engine's root-relative API),
`claudine/docs/topics/flow-control/lifecycle.md` (the sequence-root exception
passage at lines 1320-1326 is removed and replaced by the one-boundary rule
and the contract's creation candidate), and
`claudine/docs/topics/state-management/side-effects.md` (line 125:
containment through filesystem aliases — canonical ancestor checks — not only
lexical). Repo skills: `.claude/skills/darkmatter/lifecycle.md` ~line 71 ("`md`'s
mutation root is the prompt's own directory (Claudine's is the launch
workspace), so an `md` prompt in a sibling library cannot edit a
`{{ ctx.cwd }}` path" — rewrite to the one boundary and the contract's
creation candidate). `.claude/skills/claudine/cli-reference.md:678` needs no
change: its "filesystem writes are restricted to the configured mutation root"
is the generic engine constraint of the `--side-effects` report and stays
true; no other Claudine repo-skill text describes a host's mutation root
(searched `launch workspace`/`beside the prompt` across
`.claude/skills/claudine/`). Caller-skill sources:
`claudine/docs/use-skills/use-claudine/lifecycle.md:97` ("`edit` writes only
inside the launch workspace" → the caller's working tree); no `use-darkmatter`
page names the root (searched). Then `just publish-skills`
in both areas (never edit installed skills). READMEs change only if an
overview example becomes incorrect. Docs state the rule directly and never
link back to the spec.

**R12 — Sequencing against the dependency is ordering only.** The spec's
`depends-on: 2026-11-14-absent-equals-null` (simplified-schema area) owns
null/default handling and `no_error: null` acceptance; it touches the
lifecycle *parser* and schema validation, while this fix touches target
resolution in the executor and host wiring. No function is shared, so the two
land in either order without coordination; this plan's work does not depend on
that fix's outcome.

**R13 — biscuit-file is not modified.** `LaunchMagicScope::local_root`,
`PathIdentity`, `canonicalize_simplified`, `resolve_in_context`,
`candidate_plan`, and the URL/recursive behaviors all exist and are public or
crate-internal to their consumers as needed. Any temptation to add a
root-selection or containment helper to biscuit-file is declined; the
containment check reuses Darkmatter's `ensure_within` (spec: "Reuse
Darkmatter's guarded file-write helpers").

**R14 — Verification commands.** `just test` and `just lint` in `darkmatter/`
and `claudine/` are the gate. The darkmatter L2 edit-action suite
(`darkmatter/cli/tests/level2/level2_edit_action.rs`) exercises the changed
target-selection surface in a real terminal; run it with `just test-l2` in
`darkmatter/` filtered to that suite, through the focus-preserving harness
(windows must not gain focus). Claudine L2 suites do not touch mutation-root
selection and are out of scope for this fix. No prompt tests, no browser
tests.

### Tasks

Wave 1 (two tasks in parallel; they touch disjoint files in `darkmatter/lib`):

- [x] **Resolver rework** (`darkmatter/lib/src/lifecycle/executor.rs`)
  - Replace `mutation_target` per R4: contract lookup → recursive-refusal →
    first candidate → `ensure_within` containment, returning the cleaned path;
    delete the implicit-relative root-join branch.
  - Route the seven frontmatter verbs through `path(0)` in
    `dispatch_side_effect`, resolving operand 0 once per action and passing
    the same resolved target to the engine call (R5).
  - Change `mirror_frontmatter_mutation` to take the resolved `&Path`; rewrite
    `targets_document` to canonical-with-missing-tail + `PathIdentity`
    comparison; make `canonicalize_with_missing_tail` `pub(crate)` in
    `fs_write.rs` and delete the now-unused `lexical_normalize` (R6).
  - Risks: `dispatch_side_effect`'s error mapping must keep containment
    failures as dispatch failures through `error_or_action`; the
    `path(0)`-for-frontmatter change alters the engine input from raw text to
    an absolute path, so any engine-side behavior keyed on the raw string
    (none found — `resolve` joins then guards) is unaffected.
- [x] **Engine read guard** (`darkmatter/lib/src/effects/verbs.rs`)
  - Add `ensure_within` to `EffectEngine::load` before reading (R7), with a
    unit test proving an outside target fails on load, not on save.
  - Update the module's `///` docs where they describe load behavior.

Wave 2 (after Wave 1 compiles):

- [ ] **Library test suite** (`darkmatter/lib/src/lifecycle/executor/tests.rs`)
  - Update the two R10.1 tests; extend `Fixture` to build a
    `FileResolutionContext` with `with_repository_root` so the implicit
    relative candidate order (document dir → repository root) is exercisable,
    and place the source document in a subdirectory for the
    creation-beside-document cases. New tests live in this module beside the
    edit tests; the integration files under `darkmatter/lib/tests/l1/` change
    only where the Phase 3 sweep finds breakage.
  - Add: `./` pinning versus bare fallback-to-root (the removed-beside-document
    later `edit: notes.md` opening the root-level file), nested missing-parent
    creation, missing recursive refusal, URL refusal with zero fetches,
    outside-root refusal naming target and root before any parent exists,
    frontmatter-verb routing (a `set_frontmatter` on the active document is
    visible to the next action; on another file it is not), and mirror
    identity across an equivalent path spelling.
  - Keep table-driven coverage over all seven frontmatter writers, both
    `ensure_file` overloads, `ensure_dir`, both append verbs, and `edit`
    (spec, "Every writer"), asserting through the crate's public executor API,
    not private functions.

## Phase 2 — Host wiring: the same boundary everywhere

All five tasks are independent file edits and run as one parallel wave. Each
task applies its R2 row, updates surrounding comments that name the old root
(the "Lifecycle writes target the project…" comments at prep.rs:1130-1133 and
pipeline.rs:1358-1361 say the launch workspace; staged_boot.rs:117-118 and
loop_control.rs:332-333 similarly), and leaves the host's failure reporting of
capture errors untouched (R3).

Wave 1 (parallel):

- [ ] **`md compose` root** (`darkmatter/cli/src/commands/compose/lifecycle.rs`)
  - Replace the `source_path.parent()` selection (lines 377-381) with
    `compose.request.resolution_context().launch_magic_scope().local_root()`.
  - Update the struct docs that describe the root. Prerequisite: Phase 1
    Wave 1 has landed (the executor now honors the boundary correctly).
- [ ] **Claudine prep routes** (`claudine/cli/src/commands/compose/prep.rs`)
  - Rework lines 1134-1138 (loop route) and 1965-1970 (staged single) per R2,
    and update `staged_boot.rs`'s parameter comment.
- [ ] **Claudine wrapper pipeline**
  (`claudine/cli/src/commands/wrap/composition/pipeline.rs`)
  - Rework line 1362 per R2; the pipeline's no-invocation fallback path
    (tests, ambient fallback) reads the scope off
    `request.prepared.input_layers.file_resolution_context`, which every
    prepared composition carries, so no `Option` handling is introduced.
- [ ] **Claudine sequence and task roots**
  (`claudine/cli/src/commands/wrap/sequence/mod.rs`, `.../task_run.rs`)
  - Rework mod.rs:283, task_run.rs:110-111, and task_run.rs:257 per R2; delete
    the "A task stack mutates relative to the document that authored the
    task" comment's implication that the authoring directory is the boundary —
    the authoring directory remains the *resolution base*, the boundary is the
    caller's root.
- [ ] **Claudine loop harness root**
  (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`)
  - Rework line 334 per R2, using `run.prompt_state.file_resolution_context()`;
    update the adjacent comment.

## Phase 3 — Verification suites

Spec's Verification section, mapped to deterministic tests. Wave 1 tasks are
parallel; Wave 2 runs after Wave 1's suites exist so the sweep sees every new
fixture.

Wave 1 (parallel):

- [ ] **Contract table tests** (darkmatter lib executor tests, plus one CLI
  fixture test in `darkmatter/cli/tests/l1/compose_lifecycle.rs`)
  - The spec's example as a table: prompt at `prompts/doc.md` run from the
    repo root where `specs/auth.md` exists — `edit: specs/auth.md` opens the
    root file (second candidate); `ensure_file`/`append_line` on it act on the
    same file; `ensure_file: ["specs/new.md", …]` creates
    `prompts/specs/new.md` (first candidate); `edit: "~/notes.md"` is refused.
  - Cover implicit, explicit (`./`), and nested missing-parent forms, stdin
    input, and inline-composition text input (R9), asserting through the
    public result (executor outcome and the filesystem).
- [ ] **Caller/source differ tests** (claudine CLI L1 fixture, extending
  `lifecycle_mutation_root.rs`)
  - An external shared prompt writing an absolute target inside the caller's
    root succeeds; writing beside itself through a relative target is refused
    before anything is created (R10.2's rewrite covers this; keep both cases
    in the one module).
  - Launch outside any repository with a prompt inside one: the caller's
    launch directory still wins (the boundary is the launch scope, not the
    document's repository).
- [ ] **Reference-semantics tests** (darkmatter lib executor tests)
  - Prefixed targets inside the root (`&`, `^`, `@`, and `vault:` with a vault
    root configured in the fixture) resolve and write; the same prefixes
    outside the root are refused — prefixes are not write grants (spec: "This
    fix does not reinterpret either prefix as a write grant").
  - A missing nonrecursive prefixed target is created at its prefix's first
    candidate when that lies inside the root, refused otherwise.
  - An environment-expanded reference whose expanded value is absolute is
    treated as an absolute target (containment decides).
  - An unmatched recursive `%` reference is refused; a URL makes zero fetch
    attempts (assert the error kind; no network in tests).
- [ ] **Boundary-enforcement tests** (darkmatter lib + CLI tests)
  - An in-root `../` target succeeds; an escaping `..`, a sibling text prefix
    (`~/proj-x` versus `~/proj`), and a symlink escape fail before any file or
    parent is created.
  - Symlink fixtures use the existing cross-platform pattern
    (`biscuit-file/lib/tests/l1/glob_reference/mod.rs:127-132`: unix symlink /
    windows `symlink_file`); Unix-gated tests stay `#[cfg(unix)]` and the
    Windows counterpart is its own test, not absent evidence.
  - Keep the existing editor symlink, `create_new`, and race tests green.

Wave 2 (after Wave 1):

- [ ] **Lifetime and wiring tests** (claudine L1 fixtures: compose staged boot,
  wrap composition tests, sequence containers, loop primitives)
  - Change source documents through `prep`/`proxy`, retry, resume, and loop
    progression: the boundary stays fixed (writes always land inside the
    caller's root) while the authoring base follows the active document
    (creation lands beside the *active* document).
  - Parallel members with different source directories resolve relative
    targets against their own documents and do not share a mutable path base.
  - Host wiring per the spec's list — `md compose`, Claudine compose and
    inline composition, wrapper composition, root and nested sequence/group
    events, task adapters, staged initialization, loop events — through the
    existing fixture seams, asserting one shared set of path-expectation
    helpers (a small `mod` in each area's test commons) rather than copying
    the resolution algorithm into assertions.
  - Boundary selection fixtures per R10.4: repository, no repository, monorepo
    package subdirectory, linked worktree, corrupt `.git`.
  - Passive surfaces: one guard test that schema validation and composition
    without a lifecycle host dispatch no mutation and open no editor.
- [ ] **Existing-test sweep and full L1 runs**
  - Run `just test` in `darkmatter/` and `claudine/`; fix every expectation
    that encoded root-joined creation or a host-specific root, per R10.3.
  - Run `just lint` in both areas.
  - Complexity/risk: this is where undocumented couplings surface; the fix is
    always to the new contract (refusal naming target and root, or creation at
    the contract's first candidate), never a compatibility branch keyed on
    file existence (spec: "Do not retain a compatibility branch whose choice
    depends on whether the file exists").

## Phase 4 — Documentation, skills, and closure

Wave 1 (parallel):

- [ ] **Darkmatter docs** — rewrite the two pages per R11 with the
  boundary/base distinction, a Mermaid or table rendering of the two-step
  selection (resolve by contract → check boundary), the subdirectory example,
  the shared-prompt case, and the migration note (a bare name that used to be
  created at the root is now created beside the document; use an absolute
  caller-based path or `./` pinning as appropriate).
- [ ] **Claudine docs** — remove the sequence-root exception passage and state
  the one boundary; update the side-effects page's containment wording to
  filesystem-identity-aware checks.
- [ ] **Skills** — rewrite the `.claude/skills/darkmatter/lifecycle.md`
  passage named in R11 (`.claude/skills/claudine/` needs no change, per R11's
  search), and update the caller-skill sources under each area's
  `docs/use-skills/` (`use-claudine/lifecycle.md` at minimum); run
  `just publish-skills` in `darkmatter/` and `claudine/`; check both READMEs
  for now-incorrect overview examples.

Wave 2 (after Wave 1):

- [ ] **Final validation and implementation log**
  - `just test` and `just lint` in both areas one final time; run the darkmatter
    L2 edit-action suite via `just test-l2` (focus-preserving harness only).
  - Write `implementation-log.md` beside this plan recording what landed, any
    spec departures (with the doc corrections that cover them), and the test
    inventory mapped to the spec's verification bullets.
  - Confirm no `Co-authored-by`/agent attribution would accompany any commit
    (commits happen only on explicit instruction, separately).

---

## Verification checkpoints

- After Phase 1 Wave 1: `just test darkmatter lifecycle` (package plus filter
  term, the recipe's pass-through args) is green for the updated executor
  tests; the resolver's behavior is fully pinned before any host changes.
- After Phase 2: `md compose` and Claudine engine constructions all read the
  launch scope (a `rg -n "launch_workspace.child_cwd|source_context.base_dir\(\)|origin_dir|env_plan.child_cwd|source_path.*parent"` sweep over the two
  areas returns no lifecycle-engine construction hits — the remaining
  `child_cwd` uses are process/env wiring, not mutation roots).
- After Phase 3: both areas' `just test` green; every spec Verification bullet
  maps to a named test in the implementation log.
- After Phase 4: `just publish-skills` output reviewed; installed skills not
  edited by hand; final `just test` + `just lint` green in both areas.
