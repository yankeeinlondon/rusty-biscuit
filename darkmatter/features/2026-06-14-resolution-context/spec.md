---
created: 2026-06-14
reviewed: false
status: draft
area: darkmatter, claudine
component: markdown/compose expression evaluation (resolution context, token resolution)
---

# Resolution Context & Token Resolution for Expression Evaluation

## Summary

Darkmatter's expression engine gates seven **read-side functions** — `file_exists`,
`frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema`,
`absolute`, `relative` — behind `EvaluationLookup::resolution_context()`. Only one
production lookup supplies that context, so every other surface that evaluates the
*same* grammar (frontmatter interpolation, the `$()` shell ternary condition,
reference-graph `when=`, the public condition API, and claudine's loop/hook
conditions) silently cannot run any read-side function. The originally-reported
`file_exists`-in-frontmatter failure is one instance of a systemic asymmetry.

This fix has three parts:

1. **Resolution context** — give every legitimate evaluation surface a resolution
   context so read-side functions resolve identically wherever the grammar runs.
2. **`$()` token resolution** — specify and document the precedence by which a
   token inside a frontmatter shell expression resolves (literal → expression
   function → executable → frontmatter property → null), and reject a `$()` that
   contains no shell command with a helpful "use `{{ }}`" diagnostic.
3. **`doc.*` namespace** — add an explicit frontmatter accessor available in every
   expression surface, so authors can reference a property unambiguously even when
   its name collides with an executable.

It also establishes the resolution-ordering invariant in documentation and adds a
guide for authoring future expression functions.

## Problem Statement

A prompt derives an optional `spec` path from whether a sibling `spec.md` exists:

```yaml
$schema:
    plan: file(required)
    spec: file
possible_spec: "{{dir}}/spec.md"
spec: "{{ file_exists(possible_spec) ? possible_spec : '' }}"
```

Composing it fails with a misleading file-reference error
(*"invalid variable name … must match [A-Z0-9_]+"*). The real failure is one stage
earlier: `file_exists` never evaluated. The same expression resolves correctly in
the document **body**, and composing emits the giveaway warning:

```
key 'spec': failed to evaluate '…':
Filesystem function 'file_exists' requires a document resolution context,
which is unavailable here
```

So the expression parses and evaluates fine *in the body*; in frontmatter it
cannot evaluate, the literal `{{ … }}` survives, and the downstream `file`-typed
schema check hands that literal to `biscuit-file`'s `FileReference` parser (whose
`{{ … }}` grammar is environment-variable-only), producing the confusing message.

## Root Cause

`resolution_context()` is the **only** capability that differs between evaluation
lookups; `get`/`get_string` are always implemented. The fs gate
(`compose/expression/mod.rs:522-537`) only reaches `dispatch_fs` when
`lookup.resolution_context()` is `Some`; otherwise a known fs function returns the
recoverable "requires a document resolution context" error. The gate depends
solely on the lookup — **not** on parse mode and **not** on `{{ … }}` bracketing.

- Read-side set: `FS_FUNCTIONS` (`functions.rs:977-1005`), gated by
  `is_fs_function`. Context type: `ResolutionContext`
  (`expression/resolve_ctx.rs:17-31`) = `base_dir`, `magic_paths`, optional
  `remote_fetch`. Public with `ResolutionContext::new(base_dir)` (`pub use` at
  `expression/mod.rs:80`); built for compose by
  `ComposeOptions::expression_resolution_context` (`types.rs:1204-1220`).
- The only production lookup returning `Some(ctx)` is `ResolvingLookup`
  (`state.rs:316-345`). Every other lookup inherits the trait default `None`
  (`expression/mod.rs:194-196`).

## Full Scope — Every Evaluation Surface

| # | Surface | Location | Has context? | In scope? |
|---|---------|----------|--------------|-----------|
| — | Body `{{ }}` interpolation | `compose/mod.rs:1291` | ✅ | reference |
| — | `::block when="…"` | `compose/mod.rs:1413` | ✅ | reference |
| — | Transclusion `::file when="…"` | `compose/mod.rs:1520` | ✅ | reference |
| — | Shell-discovery `when=` gate | `shell_expansion/discovery.rs:357` | ✅ | reference |
| 1 | **Frontmatter interp — pre-shell pass** | `mod.rs:617` → `frontmatter_interpolation.rs:253/298` | ❌ | **yes** |
| 2 | **Frontmatter interp — post-shell pass** | `mod.rs:690` | ❌ | **yes** |
| 3 | **`$()` shell ternary condition** | `frontmatter_shell_expansion.rs:988` | ❌ | **yes** (real run) |
| 4 | **`$()` shell ternary branch interp** | `frontmatter_shell_expansion.rs:1142` | ❌ | **yes** (real run) |
| 5 | **Reference-graph `when=`** | `markdown/reference/graph.rs:297` | ❌ | **yes** |
| 6 | **`evaluate_condition_against` public API** | `conditions.rs:251/268-328` | ❌ | **yes** |
| 7 | **claudine loop `until`/`while`/`action`** | `claudine loop_expression.rs:69` | ❌ | **yes** |
| 8 | **claudine hook `when=`** | `claudine dispatch/expression.rs:85/150` | ❌ | **yes** |
| 9 | `markdown::transform` pipeline | `transform/mod.rs:363/477/513` | ❌ | **no** — document asymmetry |
| 10 | Shell-command discovery **frontmatter** preflight | `shell_expansion/discovery.rs:464` | ❌ | **no context needed** — see §"$() Token Resolution" |

`markdown::transform` (#9) is a separate pipeline using a bare `EffectiveState`;
out of scope, asymmetry documented. Non-expression surfaces are unaffected: the
`replace:` map, transclusion target resolution, and `$schema` file references.

## Decided Sub-Behaviors

- **A — Optional `file` field "absent" → empty string (DECIDED).** Schema
  validation treats an empty-string value of a **non-`required`** `file` field as
  absent/valid, so the motivating ternary's `else ''` branch passes while `file`
  typing (and its completions) is preserved. Required `file` fields still reject
  empty.
- **B — Remote URL args in frontmatter → local-only now (DECIDED).** The
  frontmatter resolution context is local-filesystem only (no remote runtime). A
  remote URL argument to a read-side function in frontmatter **fails loudly**
  (including `file_exists(url)`, which must error rather than silently return
  `false`). Extending the remote pre-fetch discovery sweep to frontmatter is a
  deferred follow-up.

## §1 — Resolution Context (surfaces #1–#8)

Thread a resolution context into every in-scope surface so the read-side functions
evaluate identically wherever the grammar runs.

- **darkmatter:** build the context at the frontmatter call sites via
  `options.expression_resolution_context(&runtime.remote_fetch)` (`runtime` is in
  scope at `mod.rs:617/690`) and thread it into `interpolate_frontmatter`, the
  `$()` ternary condition/branch helpers (`frontmatter_shell_expansion.rs`),
  `reference/graph.rs` (#5), and `evaluate_condition_against`/`ShortcutLookup`
  (#6, using its existing `work_dir`). Mechanism: give the seed states an optional
  `ResolutionContext` returned from `resolution_context()`, or wrap them like
  `ResolvingLookup`. The seed map still drives `get`/`get_string`; only
  `resolution_context()` changes.
- **claudine (#7, #8):** claudine reuses darkmatter's engine but supplies its own
  lookups. Prompt **composition** delegates to `darkmatter::compose()` and inherits
  the fix automatically — the motivating `spec:` case needs **no claudine change**.
  **Loop/hook** conditions are evaluated by claudine after composition with
  `LoopExpressionLookup` / `EventMeta*Lookup`. `prompt_path` is already in scope at
  `loop_engine.rs:295`; thread `prompt_path.parent()` (and the hook definition's
  source dir) into those lookups and override `resolution_context()` to return
  `Some(ResolutionContext::new(base_dir))`. No darkmatter change — the type and
  constructor are already public. The probe is re-run each iteration against that
  fixed root. `ResolutionContext::new` supplies `base_dir` only (no magic paths /
  remote runtime), which is correct for loop/hook conditions.

## §2 — Token Resolution in `$()` Shell Expressions (DEDICATED SECTION)

A frontmatter `$( … )` value is a **shell expansion**. Within it, the engine and
the shell coexist, and a token resolves by this **precedence ladder**:

1. **Quoted** (single/double) → string literal.
2. **Numeric** → number literal.
3. **`true` / `false`** → boolean literal. *Never* a command or a property.
4. **`name(...)`** (trailing parentheses) → expression function. These are **safe
   functions** — they spawn no process and require **no preflight/approval**. No
   shell executable may contain `(` or `)`, so this is an unambiguous syntactic
   distinction, not a heuristic.
5. **Bare name / path:**
   - **Path-bearing** (`/usr/bin/doit`, `./doit`) → **executable**: it exists and
     is executable, or it does not. Never a frontmatter property.
   - **Bare relative** (`doit`):
     - found on `PATH` → **executable** (a shell command — subject to
       preflight/approval),
     - not found on `PATH` → **frontmatter property**,
     - property absent → **`null`**.

### Validity rule and diagnostic

A `$()` is valid only if it resolves to **at least one shell command** in executed
position — for a ternary, at least one **branch** is a real shell pipeline; for a
non-ternary, the directive itself is. The **condition** is always a (safe)
expression and never counts as the command.

A `$()` that contains no shell command — e.g. `"$( file_exists('x') ? 'a' : 'b' )"`
(an expression-function condition with string-literal branches) — is a **user
error**: it is entirely expression-engine content. The current failure is a
branch-level parse message; this fix replaces it with a targeted diagnostic, e.g.
*"`$( … )` contains no shell command — `file_exists(...)` is a darkmatter
expression; did you mean `{{ … }}`?"*

Intermixing is fully supported when a real command is present, e.g.
`"$( file_exists('Cargo.toml') ? cargo build : make )"` runs `cargo build` or
`make`; the condition uses the engine (needs the §1 context at the real run), the
branches are shell pipelines.

### Preflight behavior

The preflight (shell approval discovery) does **not** evaluate the expression
engine and needs **no resolution context** for selection: `directive_reachable_pipelines`
already enumerates **both** ternary branches (`frontmatter_shell_expansion.rs:944-965`),
so the approved set is a superset of what can run and nothing executes unapproved.
Preflight does perform a read-only `PATH` probe to classify bare names
(executable → needs approval; otherwise property/null → ignored). Safe functions
are excluded by construction (the `()` syntax). The §1 context is needed only by
the **real run** of surfaces #3/#4, so the chosen branch's condition resolves.

## §3 — The `doc.*` Frontmatter Namespace

Add `doc` as a third explicit lookup namespace alongside the existing `ctx.*`
(run/context) and `env.*` (environment):

- **Bare `doc`** → the whole root frontmatter object.
- **`doc.<path>`** → a root frontmatter property (dotted traversal for nested
  values), e.g. `doc.build`, `doc.config.retries`.
- A property literally named `doc` is reached as **`doc.doc`** (its nested child
  as `doc.doc.child`).

- **Why:** the §2 ladder lets an executable shadow a same-named frontmatter
  property (`build`, `test`, `make`, …), and resolution otherwise varies with what
  is installed. `doc.build` is unambiguous — in `$()` it bypasses the
  executable-first ladder; in `{{ }}` it is the explicit, safe form of a bare
  property reference.
- **Availability:** every expression surface — `{{ }}` interpolation (frontmatter
  and body), `when=` conditions, the `$()` ternary condition/branches, and
  claudine loop/hook. It is a lookup capability: each `EvaluationLookup` handles
  the `doc.` prefix exactly like the existing `env.`/`ctx.` prefixes (strip →
  resolve against root frontmatter). During frontmatter interpolation it reads the
  incrementally-resolved map (the same source bare names already use).
- **Distinct from `frontmatter()`:** `doc.build` is *this* document; the
  `frontmatter('other.md')` function reads *another* file's frontmatter. The docs
  must state the distinction.
- **Breaking change + migration:** bare `doc` previously resolved to a frontmatter
  *property* named `doc`; it now means the whole object. Every existing bare
  `{{doc}}` (or bare `doc` in a `$()`/condition) that means the property must
  migrate to `{{doc.doc}}`. Known occurrences (~16): `prompts/clarify.md` (7),
  `prompts/documentation.md` (4, incl. `file_exists({{doc}})`),
  `claudine/docs/research/usage/_usage.md` (5). Migrate **with** the feature —
  doing it earlier breaks them before the namespace exists — and grep to confirm
  no bare `{{ doc }}` remains. The `{{doc}}` in the completed-feature doc
  `darkmatter/features/_completed/2026-04-08-setting-props/tech-design.md` (2) is a
  historical example; update or leave as a dated artifact.

## Two Correctness Bugs Found While Scoping

1. **`REMOTE_READ_FUNCTIONS` over-includes `absolute`/`relative`** (`remote.rs:20`):
   both require a resolution context but never touch the network, yet are
   registered as remote egress for the pre-fetch discovery scanner. Split the
   concepts — all seven remain context-requiring, but the remote-discovery list
   keeps only the five that actually fetch.
2. **Side effects that newly surface once frontmatter has a context** (acknowledge
   + test, not blockers): `markdown_title` writes to STDERR on a multiple-`H1`
   document (`functions.rs:733`); `date` / `is_today*` read the system clock
   (`functions.rs:401/500-505`) and are not referentially transparent.

## Relationship to `interpolation-error-handling`

The in-flight `interpolation-error-handling` fix makes **parse** errors loud, adds
`&&`, and preserves whole-value types. It is complementary: the failure here is an
**evaluation** error (the expression parses cleanly), which that spec keeps
lenient. This fix makes the expression *succeed* by supplying the missing
capability. Sequenced together, a read-side function in any surface either resolves
(this fix) or fails loudly (that fix) — never leaks a literal. (A future, more
general "key-drop sentinel" for representing absence pairs naturally with that
spec's typed-`null` work; out of scope here — see Decision A.)

## Documentation Work

A full audit of the skill, READMEs, topic docs, and code comments was performed.
Classification: **(X)** wrong today regardless of this feature; **(F)** accurate
today, must flip when the code lands; **(G)** new content. **(F)/(G) edits are
applied *with* the implementation** so no authoritative doc/comment describes
unbuilt behavior in the interim.

### Already fixed (X — wrong today, corrected ahead of implementation)

- `docs/inline/fm-interpolation.md` — the inverted "shell expansion runs before
  interpolation" statement (now describes the real two-pass order). ✅
- `docs/topics/darkmatter-expressions.md:17` — the false "never read more than the
  local document" claim (now notes read-side functions read other files/URLs). ✅

### Code comments / rustdoc (flip in the same commit as the code)

- `expression/mod.rs:191-196` (F) — `resolution_context()` trait default: reframe
  `None` as the opt-out/test case; name the seven read-side functions.
- `expression/mod.rs:530-537` (F) — remove the misleading "(e.g. frontmatter
  interpolation)" example from the fs-gate comment (frontmatter is exactly what
  gains a context). **Highest-risk stale example — must change.**
- `frontmatter_interpolation.rs:1-5, 44-51` (F) — module doc + `FrontmatterSeedState`:
  add `doc.*` and the resolution context to the inputs; drop "only seed/ctx/env".
- `frontmatter_interpolation.rs:402-404` (F) — `collect_variable_roots`: state how
  `doc.<key>` is treated for dependency ordering (mechanism-dependent).
- `state.rs:307-315` (F) — `ResolvingLookup` doc: generalize beyond "the
  interpolation stage"; note `absolute`/`relative` are local-only.
- `mod.rs:1285-1290` (F) — body-wrap comment: drop the "those functions never run"
  global generalization.
- `mod.rs:560-567` (G) — pipeline docstring: add the frontmatter ordering invariant
  (Interp p1 → Schema → Shell → Interp p2). `mod.rs:607-705` inline comments are
  already correct — do not touch.
- `frontmatter_shell_expansion.rs:1-5` (G) — module doc: summarize the §2 ladder +
  no-command diagnostic.
- `frontmatter_shell_expansion.rs:815-829, 971-987` (F) — ternary seed-state build +
  `evaluate_ternary_condition` doc: the condition now carries a context and may use
  read-side functions + `doc.*`.
- `frontmatter_shell_expansion.rs:942` (G) — `directive_reachable_pipelines`: add a
  "preflight only — no context; selection is expression-free" note to **lock** that
  this one seed state stays context-free.
- `conditions.rs:180-194, 265-273` (F) — `evaluate_condition_against` resolution
  order + `ShortcutLookup` doc: add `doc.*` and read-side via `work_dir`.
- `reference/graph.rs:266-290` (F) — `when=` state build: note it is wrapped for a
  context and that `doc.*` resolves.
- `remote.rs:16-28` (X, **code-entangled**) — remove `absolute`/`relative` from
  `REMOTE_READ_FUNCTIONS` **and** fix the doc comment together (Bug 1 / Goal 6); not
  a doc-only edit.
- claudine `loop_expression.rs:45-57, 69-85` (F/G) — lookup doc + new
  `resolution_context()` override note (base_dir = prompt parent, re-probed per
  iteration).
- claudine `dispatch/expression.rs:123-128` (F) — `EventMetaConditionLookup` doc:
  add `doc.*` + context. **Do NOT** touch the `:784-791` "hard-coded to None" test
  comment — that is about `ctx.*`, not the resolution context.

### Topic docs / skill / READMEs (apply with the feature)

- `docs/topics/darkmatter-expressions.md` (G) — the authoritative reference; add:
  a **Read-Side Functions** section (the 7); a **Token Resolution in `$()`** section
  (the §2 ladder + diagnostic); a **Namespaces** subsection (`doc.*`/`ctx.*`/`env.*`,
  and `doc.*` vs the `frontmatter()` function); the availability invariant; update
  the "two surfaces / identical everywhere" intro to list every surface; add `doc.*`
  to "Where Expressions Read Values From" and to the `evaluate_condition_against`
  resolution order. Add the **"Authoring a New Expression Function"** guide here (or
  a sibling file linked from here): pure vs. context-aware (`PURE_FUNCTIONS` vs
  `FS_FUNCTIONS`), registration + the mandatory `catalog.rs` descriptor (parity
  tests enforce set equality), how fs functions obtain paths via `ResolutionContext`
  / handle remote URLs, and the cross-pass availability + fail-loudly contract.
- `docs/inline/fm-interpolation.md` (F/G) — add `doc.` and read-side functions to the
  "Available Variables" table and examples.
- `docs/inline/interpolation.md:5-9` (F) — "two stages" → note the frontmatter stage
  is two passes; add `doc.*` / read-side to the sources.
- `docs/inline/fm-shell-expansion.md:43-83` (F/G) — fix the placement list (insert
  Schema Validation + pass-2); add (or link) the §2 token-resolution ladder.
- `docs/darkmatter-compose-pipeline.md:14-22` (F) — add a pass-2 interpolation node /
  note to the Mermaid diagram.
- `docs/topics/remote-url-references.md:11-19` (F) — remote URLs are body/post-shell
  only (frontmatter is local-only and fails loud); affirm `absolute`/`relative` are
  never remote.
- `.claude/skills/darkmatter/SKILL.md` (F/G) — two-pass interpolation in the pipeline
  list; the remote/frontmatter + `absolute`/`relative` caveat; a Progressive
  Disclosure row pointing at the expressions topic (read-side fns, `doc.*`, `$()`
  ladder). **Regenerate `hash:` via `md hash` after editing.**
- `.claude/skills/darkmatter/compose.md` (F/G) — two-pass interpolation; add `doc.*`
  and read-side rows to the Variable Resolution table; refresh the `expression/`
  module list (`functions.rs`, `catalog.rs`, `resolve_ctx.rs`).

### Unrelated cleanup (out of scope, noted)

- `docs/topics/side-effects.md:72-73` has a stray `</content>`-style artifact — a
  separate cleanup, not this feature.

## Goals

1. Read-side functions evaluate identically across surfaces #1–#8.
2. claudine loop/hook conditions resolve read-side functions against the
   prompt/hook document root.
3. The motivating `file`-typed `spec` ternary composes (path when present; empty ⇒
   absent per Decision A).
4. `$()` token resolution follows the §2 ladder; an all-expression `$()` errors
   with a `{{ }}` suggestion; preflight stays expression-free.
5. `doc.*` resolves the document's frontmatter in every expression surface.
6. `absolute`/`relative` are no longer registered as remote egress.
7. Documentation establishes the ordering rule, the namespaces, the ladder, and
   the authoring guide; the false ordering statement is fixed; skill updated.

## Non-Goals

- No change to the `{{ … }}` grammar, the function catalog, or the read-side set.
- No change to parse-vs-evaluation error surfacing (owned by
  `interpolation-error-handling`).
- No change to `biscuit-file`'s `FileReference` grammar.
- No `markdown::transform` (#9) parity; no remote-in-frontmatter pre-fetch (B).
- No general key-drop absence mechanism (Decision A keeps the empty-string form).

## Test Plan

- **Unit (frontmatter interp):** `file_exists(local_path)` resolves both ways
  (assert written-back frontmatter); `absolute`/`relative` match body-pass results;
  a remote URL arg fails loudly.
- **Unit (`$()` token resolution):** ladder coverage — quoted/number/bool literal;
  `name()` function (safe, no approval); bare name found on `PATH` → command (and
  is approved); bare name absent from `PATH` → frontmatter property; absent
  property → null; path-bearing executable. `doc.build` resolves the property even
  when a `build` executable exists.
- **Unit (`$()` diagnostic):** `"$( file_exists('x') ? 'a' : 'b' )"` errors with the
  `{{ }}` suggestion; `"$( file_exists('Cargo.toml') ? cargo build : make )"`
  resolves the intended branch at the real run.
- **Unit (`$()` preflight):** both branches enumerated for approval without
  evaluating the condition; safe functions never approved.
- **Unit (`doc.*`):** resolves in frontmatter, body, `when=`, `$()` condition, and
  claudine loop conditions; distinct from `frontmatter('other.md')`.
- **Unit (claudine):** `loop.until="file_exists('artifact')"` flips false→true as
  the file appears; a hook `when=` read-side function resolves against its base dir.
- **Integration (compose):** the motivating `spec: file` document composes, with
  `::block when="spec"` branching correctly both ways.
- **Regression:** `absolute("https://…")` no longer registered as remote egress;
  the original misleading "invalid variable name" path no longer triggers.

## Affected Code

- `compose/frontmatter_interpolation.rs` — signature + per-key lookup; `doc.` prefix.
- `compose/mod.rs` — frontmatter call sites (`:617/690`); pipeline docstring.
- `compose/frontmatter_shell_expansion.rs` — ternary condition/branch context
  (`:826/942/988/1142`); the §2 ladder + no-command diagnostic.
- `compose/state.rs` — lookup context wiring; `doc.` prefix.
- `compose/conditions.rs` — `ShortcutLookup` context + `doc.` prefix.
- `markdown/reference/graph.rs` — `when=` lookup context + `doc.` prefix.
- `compose/remote.rs` — `REMOTE_READ_FUNCTIONS` split.
- `markdown/schemas/` — optional-`file` empty-as-absent (Decision A).
- `compose/expression/` — shared `doc.` namespace helper, if factored.
- `claudine/lib/src/composition/{loop_expression.rs,loop_engine.rs,loop_actions.rs}`,
  `claudine/lib/src/dispatch/expression.rs` — context-bearing lookups + `doc.` prefix.
- Docs: `docs/topics/darkmatter-expressions.md`, `docs/inline/fm-interpolation.md`,
  pipeline docs; rustdoc; `.claude/skills/darkmatter/{SKILL.md,compose.md}`
  (regen `SKILL.md` hash).

## Resolved Decisions

- **A — Optional `file` empty ⇒ absent.** A non-`required` `file` field accepts an
  empty-string value as absent/valid (see Decided Sub-Behaviors).
- **B — Remote in frontmatter: local-only, fail loud.** No remote runtime in the
  frontmatter context; remote URL args fail loudly (see Decided Sub-Behaviors).
- **C-condition scope: keep.** Read-side functions remain usable in the `$()`
  ternary **condition** (surfaces #3/#4 in scope); the §2 ladder + `doc.*` keep it
  coherent.
- **D — `ShortcutLookup` (#6): accept.** The `work_dir`-based context makes
  read-side functions resolve for external callers of `evaluate_condition_against`.
  This is a public-API capability addition — note it in the changelog.
- **E — Fix mechanism: override.** Store an `Option<ResolutionContext>` on each
  lookup and override `resolution_context()`. `ResolvingLookup` is `pub(crate)` and
  hard-wired to `EffectiveState`, so it cannot wrap the seed states, and claudine
  (separate crate) cannot use it at all — the override is the only approach that
  works across every surface and both crates.
- **`doc` namespace: bare `doc` is the whole object.** A frontmatter property named
  `doc` is reached as `doc.doc`; the ~16 existing bare `{{doc}}` property references
  migrate with the feature (see §3).
