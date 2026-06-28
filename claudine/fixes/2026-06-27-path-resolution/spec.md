---
status: ready for planning and implementation
reviewed: true
created: 2026-06-27
area: claudine
packages:
    - claudine
    - darkmatter
review_iterations: 4
---

# Lifecycle File-Reference Resolution Ignores the Launch Area

## Symptom

A caller-supplied path variable that points at a real file reads as **missing**
inside a lifecycle event. Given `prompts/review-feature.md`:

```yaml
initialize:
    info: "spec [{{spec}}]: {{file_exists(spec)}}"
```

invoked from the `claudine/` package area with
`spec=fixes/2026-06-26-malformed-yaml-in-agent-prompt/spec.md`, the `initialize`
event renders:

```
spec [fixes/2026-06-26-malformed-yaml-in-agent-prompt/spec.md]: false
```

even though the file exists and resolves cleanly through `FileReference`
(`bf fixes/2026-06-26-malformed-yaml-in-agent-prompt/spec.md` from `claudine/`
prints its frontmatter). Because `iteration`, `design`, and `dir` all branch on
`file_exists(spec)` / `frontmatter(spec, …)`, the whole prompt mis-derives
(e.g. `iteration` stays `1` instead of incrementing, `review_file` loses its
`ctx.area` prefix).

## Root Cause

`spec` is supplied **relative to the launch directory** — the package area the
user/agent invoked claudine from (`claudine/`). Three facts combine to break its
resolution at event-time:

1. **The wrapper repositions the process.** Before the provider/lifecycle runs,
   `switch_process_cwd(child_cwd)`
   (`claudine/cli/src/commands/wrap/exec/mod.rs:44`) `chdir`s the process to the
   **repo root** for skill discovery + permission scope. This is intentional and
   correct — see the launch-CWD-switch design. The launch directory is captured
   beforehand in `LaunchWorkspaceContext.launch_cwd`.

2. **Lifecycle event strings are deferred (late binding).** `initialize` /
   `start` / `success` / … are interpolated **at event-time** via Darkmatter
   DM2, which is **after** the `chdir`. So `std::env::current_dir()` at that
   point is the repo root, not the launch area.

3. **File-reference resolution is anchored at the document dir, not the launch
   area.** When the `initialize` `StackExecutionContext` is built
   (`claudine/cli/src/commands/wrap/composition/mod.rs:1628`):

   ```rust
   let base_dir = request.prepared.resolved_path.parent().or(effective_repo_root);
   // => prompts/   (the document's directory)
   ...
   ctx_base_dir: Some(launch_workspace.launch_cwd.as_path()),  // => claudine/  (launch area)
   ```

   `base_dir` is documented (`lifecycle_executor.rs:203`) as *"Base directory for
   read-side expression functions and file references."* It is set to the
   **document directory** (`prompts/`). The launch area **is** already captured
   and threaded in — but only as `ctx_base_dir`, which feeds `ctx.*` capture, not
   file references.

`file_exists`'s resolver (`resolve_arg`,
`darkmatter/lib/src/markdown/compose/expression/functions.rs:931`) tries
`resolve_from(base_dir)` then falls back to `resolve()` (ambient process CWD):

| Anchor tried           | Value at event-time | `…/fixes/…/spec.md` |
|------------------------|---------------------|---------------------|
| `base_dir`             | `prompts/`          | missing             |
| ambient CWD fallback   | repo root (post-`chdir`) | missing        |
| **launch area** (never tried) | `claudine/`  | **EXISTS**          |

The launch area — the one anchor `spec` is actually relative to — is never
consulted for file references.

## Why Schema Validation Already Resolves It Correctly

The user observed that `$schema` `file`-typed validation accepts the same
area-relative path that `file_exists` rejects. This is **not** a smarter
resolver — `darkmatter`'s schema file validator
(`schemas/format.rs:139`, `resolve_file_reference`) also ultimately resolves
against the **ambient process CWD**. The difference is purely **when** it runs:

- **Schema validation + prepare-time / body interpolation** run during
  `prepare`, **before** `switch_process_cwd`. At that moment the ambient CWD is
  still the launch area (`claudine/`), so the ambient fallback happens to land on
  the right directory. Verified: from `claudine/`, a `spec: file(required)`
  schema **passes** and a body `{{file_exists(spec)}}` is **`true`**.
- **Lifecycle event interpolation** runs **after** `switch_process_cwd`. The
  ambient CWD is now the repo root, and the explicit `base_dir` is the document
  dir — both wrong.

So schema validation is correct only by the **luck of timing** (it executes
while ambient CWD is incidentally right). Lifecycle resolution can't rely on
ambient CWD because the wrapper has mutated it out from under the resolver. Run
the *same* `file`-typed schema dry-run from the repo root and schema validation
**fails too** — confirming the resolver is identical and only the anchor/timing
differs.

## The DRY Fix

The infrastructure already distinguishes two anchors and already captures the
launch area once (`launch_workspace.launch_cwd`), threading it in as
`ctx_base_dir` so that **`ctx.*` resolves against the launch area regardless of
the `chdir`**. The bug is that **file references were left on the fragile
implicit anchor** (document dir + ambient CWD) instead of the same stable
captured launch area.

Unify on a single, explicit, stable fallback anchor for caller-supplied file
references — the captured launch area — so resolution no longer depends on the
timing of `std::env::current_dir()`. This must preserve the existing
document-first contract: references authored inside the prompt document still
resolve next to that document before caller-supplied fallback semantics apply.

1. **Thread the launch area into the read-side resolver as the explicit
   fallback.** Add a named fallback field to
   `darkmatter::markdown::compose::expression::ResolutionContext`, e.g.
   `file_ref_fallback_dir: Option<PathBuf>` or `launch_dir: Option<PathBuf>`.
   Replace `resolve_arg`'s implicit `resolve()` (ambient CWD) fallback with
   `resolve_from(file_ref_fallback_dir)` when present. Keep
   `resolve_from(base_dir)` (document-relative) as the first attempt so
   genuinely document-embedded references (`::file _senior-reviewer.md`) keep
   resolving next to the document; only the fallback changes from "wherever the
   process happens to be" to "the launch area we captured at start." This makes
   prepare-time and event-time resolution identical.

2. **Do not overload `base_dir`.** `base_dir` already means "the prompt
   document's parent" in Darkmatter and Claudine lifecycle evaluation, and
   `ctx_base_dir` already means "where `ctx.*` capture runs." Keep both meanings
   intact. The new file-reference fallback should be a third, explicitly named
   anchor so future readers do not have to infer fallback behavior from
   `base_dir`.

3. **Thread the launch-area fallback into every lifecycle event context.** At
   the `StackExecutionContext` construction sites, continue setting `base_dir`
   to the prompt parent and `ctx_base_dir` to `launch_workspace.launch_cwd`.
   Extend `StackExecutionContext::resolution_context()` so the returned
   `ResolutionContext` also carries the launch-area fallback. This covers
   `initialize`, preflight `blocked`/`finalize`, loop lifecycle events, and the
   provider run-loop events (`start`/`success`/`failure`/`finalize`) instead of
   fixing only the first observed `initialize` case.

4. **Make `file` property schema validation use the same explicit fallback.**
   `$schema` references themselves should remain document-relative; that is an
   established and correct contract in `schemas::resolve`. The fragile piece is
   the `format: darkmatter-file` validator used for frontmatter values typed as
   `file`, which currently calls `FileReference::resolve()` and therefore
   depends on ambient CWD. Add a schema-validation option or constructor that
   carries the file-reference fallback directory into the `darkmatter-file`
   format validator, then use the same document-first / launch-area-fallback
   resolver as expression functions. If the `jsonschema` format callback cannot
   receive per-validation state directly, build the validator with a closure
   that captures an immutable resolver config, or register a small resolver
   object when constructing `DarkmatterSchemas`; avoid process-global or
   thread-local state unless the `jsonschema` API leaves no other route.

Net effect: `ctx.*` and file references share one launch-area anchor, and no
read-side resolution depends on the mutated ambient CWD. The fragile
"agreement with the ambient-CWD schema validator" comment at
`functions.rs:919` is replaced by an explicit shared anchor.

### Design Decision: Named Fallback Anchor

Use a distinct field on `ResolutionContext` for the caller/launch fallback.
This is preferable to changing `base_dir` because `base_dir` is already a
document-location contract used by transclusion, body interpolation, lifecycle
conditions, and tests. Reusing it for launch-area behavior would fix the
observed bug while making document-authored references ambiguous.

Recommended shape:

```rust
pub struct ResolutionContext {
    pub base_dir: PathBuf,
    pub file_ref_fallback_dir: Option<PathBuf>,
    // existing fields...
}
```

Resolution order for local filesystem arguments:

1. absolute paths are returned as-is by `FileReference`;
2. document-relative resolution via `resolve_from(base_dir)`;
3. launch-area fallback via `resolve_from(file_ref_fallback_dir)` when present;
4. no ambient-CWD fallback in production composition/lifecycle/schema paths.

Tests may continue using `ResolutionContext::new(base_dir)` without a fallback;
that preserves today's small-unit-test ergonomics. Production constructors
(`ComposeOptions::expression_resolution_context`,
`frontmatter_resolution_context`, lifecycle event construction, and Claudine
schema validation) should pass the fallback when they know the launch area.

## Scope

- `darkmatter/lib/src/markdown/compose/expression/functions.rs` — `resolve_arg`
  fallback (launch-area anchor on `ResolutionContext`) and `resolve_ctx.rs` to
  carry it.
- `darkmatter/lib/src/markdown/compose/context/options.rs` — production
  `ResolutionContext` builders must carry the explicit fallback when the caller
  provides one.
- `claudine/lib/src/composition/lifecycle_executor.rs` and
  `claudine/lib/src/composition/loop_engine.rs` — lifecycle and loop
  `StackExecutionContext` resolution contexts must include the launch-area file
  fallback while preserving prompt-parent `base_dir`.
- `claudine/cli/src/commands/wrap/composition/mod.rs` — `StackExecutionContext`
  construction for `initialize`, preflight `blocked`/`finalize`, and wrapper
  run-loop handoff must pass the launch-area fallback.
- `darkmatter/lib/src/markdown/schemas/format.rs` /
  `schemas/validate.rs` / `schemas/mod.rs` — explicit fallback anchor for
  `file`-typed property validation. Do not change `$schema` reference
  resolution except where API plumbing is needed.
- `claudine/lib/src/composition/schema_validation.rs` and
  `darkmatter/lib/src/markdown/compose/schema_validation.rs` — pass the
  launch-area fallback into schema validation when validating caller-supplied
  effective frontmatter.
- The prompt authors' `{{ctx.area}}/{{spec}}` prefixing in
  `prompts/review-feature.md` is for **agent legibility**, not a workaround, and
  should remain unchanged.

## Verification Goals (success criteria)

- From the launch area `claudine/`, with `spec` area-relative, the `initialize`
  event's `{{file_exists(spec)}}` resolves **`true`** — matching prepare-time
  body interpolation and schema validation.
- A single invocation no longer produces divergent answers between prepare-time
  (`true`) and event-time (`false`) for the same `file_exists(spec)`.
- Resolution result is **independent of the post-launch `chdir`**: an L1 test
  that captures a launch area, switches the process CWD to a different root, then
  evaluates a deferred-event `file_exists`/`frontmatter` against an
  area-relative path must pass.
- `iteration`, `dir`, `design`, and `review_file` in `review-feature.md` derive
  correctly (e.g. `iteration` increments when the spec already carries
  `review_iterations`).
- Document-relative body references (`::file _senior-reviewer.md`) still resolve
  next to the document (no regression from the fallback change).
- `$schema: ./schema.yaml` and root-union `$schema` string arms still resolve
  relative to the prompt document, not the launch area.
- A `file`-typed schema property and `{{file_exists(spec)}}` agree for the same
  `spec` value in prepare-time body interpolation, lifecycle event
  interpolation, and post-`chdir` schema validation.
- A regression test covers a path that exists under the launch area but not
  under either the prompt directory or the repo root, so the test proves the new
  fallback is being used.
- A regression test covers an intentionally conflicting filename that exists in
  both the prompt directory and launch area; the prompt-directory file must win.

## Notes / Open Questions

- No open design questions remain for this spec. The review decision is to use a
  distinct file-reference fallback field rather than overloading `base_dir`.
- If implementation discovers that `jsonschema` cannot capture a per-validator
  fallback directory for custom formats, document that API constraint in the
  implementation plan and prefer a small Darkmatter-side validation wrapper over
  process-global mutable state.
