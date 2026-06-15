---
phase: 1
title: Baseline Orientation
created: 2026-06-15
status: complete
---

# Phase 1 — Baseline Orientation Findings

This is an investigation-only phase. No production behavior was changed. It
records the confirmed current state of every surface the feature touches, the
exact current definitions that later phases will edit, and a reproducible
recording of the motivating failure. All line numbers are confirmed against the
current tree (the spec's relative paths live under
`darkmatter/lib/src/markdown/`, not `darkmatter/lib/src/`).

## 1. Workspace packages

`cargo metadata --no-deps --format-version 1` resolves the in-scope members:

| Package | Manifest |
|---------|----------|
| `darkmatter` | `darkmatter/lib/Cargo.toml` |
| `darkmatter-cli` | `darkmatter/cli/Cargo.toml` (binary: `md`) |
| `claudine` | `claudine/lib/Cargo.toml` |
| `claudine-cli` | `claudine/cli/Cargo.toml` |
| `claudine-contract` | `claudine/contract/Cargo.toml` |

The expression engine, frontmatter interpolation, shell expansion, schemas,
reference graph, conditions, and remote discovery all live in the **`darkmatter`**
library. The loop/hook lookups (surfaces #7/#8) live in **`claudine`**. No other
workspace member is in the blast radius.

## 2. Read-side function gating — the single point of asymmetry

`EvaluationLookup::resolution_context()` is the **only** capability that differs
between lookups. `get`/`get_string` are always implemented.

- Trait + default: `compose/expression/mod.rs:166-197`. The default returns
  `None` (`:194-196`).
- The fs gate: `compose/expression/mod.rs:522-538`. `dispatch_fs` is only reached
  when `lookup.resolution_context()` is `Some`. Otherwise a known fs function
  returns the recoverable error: *"Filesystem function '{name}' requires a
  document resolution context, which is unavailable here"* (`:533-537`). The gate
  depends solely on the lookup — not on parse mode, not on `{{ }}` bracketing.
- Read-side set (the seven): `FS_FUNCTIONS` at
  `compose/expression/functions.rs:977-1005` = `absolute`, `relative`,
  `file_exists`, `frontmatter`, `markdown_body_empty`, `markdown_title`,
  `validate_schema`. Gated via `is_fs_function` (`functions.rs:1068`).
- `ResolutionContext`: `compose/expression/resolve_ctx.rs:17-37` =
  `base_dir: PathBuf`, `magic_paths: Vec<(PathBuf, PathPosition)>`,
  `remote_fetch: Option<RemoteFetchRuntime>` (the last is `pub(crate)`).
  `ResolutionContext::new(base_dir)` supplies `base_dir` only (no magic paths, no
  remote runtime). Re-exported `pub` from `expression/mod.rs`.
- The compose builder is `ComposeOptions::expression_resolution_context(&remote_fetch)`
  at `compose/types.rs:1204`.

## 3. Lookup types and their `resolution_context()` behavior

| Lookup | Location | `resolution_context()` | `doc.*`? |
|--------|----------|------------------------|----------|
| `ResolvingLookup` (wraps `EffectiveState` + ctx) | `compose/state.rs:316-345` | **`Some(ctx)`** — the only production lookup that supplies one | no |
| `EffectiveState` (bare) | `compose/state.rs:177-192` | default `None` | no |
| `FrontmatterSeedState` | `compose/frontmatter_interpolation.rs:48-96` | default `None` | no |
| `$()` ternary seed state (a `FrontmatterSeedState`) | `compose/frontmatter_shell_expansion.rs:784/826/942` | default `None` | no |
| `ShortcutLookup` (public `evaluate_condition_against`) | `compose/conditions.rs:268-327` | default `None` | no |
| reference-graph `when=` effective state | `markdown/reference/graph.rs:266-298` | default `None` | no |
| `LoopExpressionLookup` (claudine) | `claudine .../loop_expression.rs:53-85` | default `None` | no |
| `EventMetaExpressionLookup` / `EventMetaConditionLookup` (claudine) | `claudine .../dispatch/expression.rs:85-157` | default `None` | no |

Only `ResolvingLookup` returns `Some`. Every in-scope surface (#1–#8) inherits the
trait default `None`, so no read-side function can run on any of them today. This
confirms the spec's root-cause analysis exactly.

`ResolvingLookup` is `pub(crate)` and hard-wired to `EffectiveState`
(`state.rs:316-319`), so it cannot wrap the seed states and claudine (a separate
crate) cannot use it — confirming Decision F that an `Option<ResolutionContext>`
override on each lookup is the only viable mechanism.

## 4. Expression namespace handling (`ctx.*`, `env.*`, bare, no `doc.*`)

All lookups share the same prefix discipline; **no lookup handles a `doc`
namespace today** (grep for `"doc."`/`"doc"` across the compose tree returns
nothing in the lookups).

- `EffectiveState::get` (`state.rs:177-192`): `ctx.` → context, `env.` →
  environment, otherwise nested frontmatter key **then fall back to `ctx.*`** so
  `when="repo"` resolves `ctx.repo`.
- `FrontmatterSeedState::get` (`frontmatter_interpolation.rs:59-85`): `ctx.`,
  `env.`, then dotted/simple seed-data key. No ctx fallback for bare names.
- `ShortcutLookup::get` (`conditions.rs:308-327`): `ctx`/`ctx.` (lazy capture),
  `env.`, plain data, **then fall back to `ctx.{path}`**.
- `LoopExpressionLookup::get` (`loop_expression.rs:69-84`): `env.`, ambient
  (`_loop_*`), boolean literal, frontmatter.
- `EventMetaExpressionLookup::get` (`dispatch/expression.rs:85-120`): `env.`,
  `ctx`/`ctx.` → `None` (deliberately unresolved here), `extra.`, `tool_input.`,
  `tool_response.`, env-path, top-level. `EventMetaConditionLookup` is the only
  hook surface that resolves `ctx.*` (via `CtxLookup`).

Implication for Phase 2/3: a `doc.` interceptor must run **before** the bare-key
lookup and **before** any legacy bare→`ctx.*` fallback, so a missing frontmatter
property named `doc` never collapses bare `doc` into `ctx.doc`.

## 5. Schema validation for `file` fields (Decision A baseline)

- A `file` atom lowers to `{ "type": "string", "format": "darkmatter-file" }`
  (`schemas/simplified/convert.rs:427-444`).
- `required` is hoisted to the JSON-Schema `required` array
  (`convert.rs:82-97`); it is **not** carried on the property fragment. So
  required vs. optional is decided by presence in `required`, while the value
  itself is always validated by the `darkmatter-file` format.
- The format validator `validate_file_reference` (`schemas/format.rs:70-72`)
  returns `resolve_file_reference(value).is_ok()` — it parses via
  `biscuit_file::FileReference` and confirms the path exists. An **empty string
  fails** for both required and optional fields today (it neither parses to a real
  reference nor resolves). There is currently **no empty-as-absent carve-out**;
  that is the Phase 5 change.

## 6. Context-requiring vs. remote-read discovery functions

- Context-requiring set = `FS_FUNCTIONS` (the seven), `functions.rs:977-1005`.
- Remote egress discovery set = `REMOTE_READ_FUNCTIONS`, `compose/remote.rs:20-28`.
  **It still lists all seven, including `absolute` and `relative`.** Neither
  touches the network, so both are mis-registered as remote egress for the
  pre-fetch discovery scanner (`collect_expression_urls`, `remote.rs:384-407`).
  This is the spec's "Bug 1" — Phase 5 removes the two from the remote list while
  keeping them context-requiring.

## 7. Validation checkpoint — recorded baseline

Reproducible with the prebuilt `target/debug/md` (no source changes needed).

### 7a. Motivating frontmatter failure (`file`-typed `spec` ternary)

Fixture (sibling `spec.md` present):

```yaml
$schema:
    plan: file(required)
    spec: file
plan: "prompt.md"
possible_spec: "{{dir}}/spec.md"
spec: "{{ file_exists(possible_spec) ? possible_spec : '' }}"
```

`md compose <fixture>` fails with the exact misleading error the spec describes —
the literal `{{ … }}` survived frontmatter interpolation (because `file_exists`
could not evaluate without a resolution context) and was handed to the
`file`-typed schema check:

```
⤫ MarkdownError: schema validation failed
  invalid spec: `{{ file_exists(possible_spec) ? possible_spec : '' }}` is
  not a valid file reference: file reference syntax is invalid: invalid
  variable name ` file_exists(possible_spec) ? possible_spec : '' ` --
  must match [A-Z0-9_]+
```

Without the `$schema`, the same document surfaces the giveaway warning directly
and leaves the literal in place:

```
key 'spec': failed to evaluate 'file_exists(possible_spec) ? possible_spec : ""':
Filesystem function 'file_exists' requires a document resolution context,
which is unavailable here
```

### 7b. A non-frontmatter surface that already works (body interpolation)

The **body** `{{ file_exists(possible_spec) }}` evaluates and resolves to a real
boolean (`false`/`true` depending on path resolution) rather than erroring,
because the body uses `ResolvingLookup` (the one lookup that supplies a
`ResolutionContext`). This is the asymmetry surfaces #1–#8 must eliminate.

## Phase 1 conclusions for downstream phases

- The fix is uniformly an `Option<ResolutionContext>` override per lookup
  (Decision F) plus a shared `doc.` interceptor (Phase 2), threaded into the
  frontmatter/`$()`/graph/conditions call sites in darkmatter (Phase 3) and the
  loop/hook lookups in claudine (Phase 6).
- `REMOTE_READ_FUNCTIONS` is the only place `absolute`/`relative` are
  mis-listed (Phase 5).
- The `darkmatter-file` format validator is the single chokepoint for the
  optional-`file` empty-as-absent carve-out (Phase 5).
