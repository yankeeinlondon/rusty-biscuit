---
created: 2026-07-17
phase: 1
feature: 2026-07-13-file-resolution
spec: ./spec.md
plan: ./plan.md
---

# Phase 1 Inventory — Unified File-Reference Resolution

Written audit that the plan requires **before** the Phase 4 precedence switch.
Gate rulings live in [`decisions.md`](./decisions.md). All findings are
read-only; no production code changed in Phase 1. Line numbers are against HEAD
on branch `error-prop-and-file-resolution`.

Sections:

1. [D12 call-site audit](#1-d12-call-site-audit) — the 20 `resolve()`/`resolve_from()` files
2. [Fixture / prompt collision inventory](#2-fixture--prompt-collision-inventory)
3. [Extended migration sweep](#3-extended-migration-sweep-beyond-the-spec-list)
4. [Baseline record](#4-baseline-record)

---

## 1. D12 call-site audit

**Mechanism.** The precedence flip changes **only**
`ReferenceKind::ImplicitRelative` (bare, sigil-less paths). In
`collect_roots` (`biscuit-file/lib/src/file_reference/resolve.rs:190-198`)
implicit roots are today `[base/cwd, git_root]` and become `[git_root,
base/cwd]`. Every other kind — explicit `./`/`../` (`Relative`), `/`
(`Absolute`), `@` (`Magic`), `!` (`Package`), `vault:`, `Url`, and
`{{ENV}}`-only — is untouched. `resolve()` uses ambient CWD as base;
`resolve_from(base)` overrides it; `resolve_relative(base)` internally resolves
via `resolve()` (ambient CWD) and only uses `base` to relativize the result
**afterward** (`mod.rs:344-368`).

**Call-site count = 20 files.** The 17 named in the plan, plus **3 additional
genuine `FileReference` call sites** that surfaced via `resolve_relative`:
`darkmatter/lib/src/markdown/reference/mod.rs:46`,
`reference/graph.rs:855`, `reference/validate.rs:517,658`. All other `.resolve(`
grep hits are non-`FileReference` noise (`OpenCodeTool::resolve`,
`McpCatalog::resolve`, `ThemePair`/grammar `resolve`, `PathTemplate::resolve`,
effect `self.resolve`, doc lines, test names).

### Classification table

| # | File (relative) | Line(s) | Classification |
|---|---|---|---|
| 1 | `biscuit-file/cli/src/main.rs` | 448 (441,446) | **migrates to repo-first** — `bf reference` surfaces resolver semantics on an arbitrary user reference |
| 2 | `biscuit-file/lib/src/file_reference/mod.rs` | 348 (def) | unaffected — impl/def site of `resolve` inside `resolve_relative`; inherits new precedence |
| 3 | `biscuit-file/lib/src/lib.rs` | 85 | unaffected — `//!` doc example only |
| 4 | `claudine/cli/src/commands/schema_interactive/mod.rs` | 680 | unaffected — path comes from the file picker as an absolute path (`Absolute` kind) |
| 5 | `claudine/cli/src/commands/sequence.rs` | 104 | **migrates to repo-first** — top-level sequence-file CLI arg; bare name is implicit |
| 6 | `claudine/gen/src/agent_errors_check.rs` | 511 | unaffected — `is_scoped_fixture_reference` hard-requires `./_fixtures/…` (explicit `Relative`) |
| 7 | `claudine/lib/src/composition/resolve.rs` | 54, 175 | **migrates to repo-first** — top-level compose source + enrichment re-read |
| 8 | `claudine/lib/src/composition/sequence.rs` | 117 | unaffected — `resolve_from(base_dir)` gated by `is_file_reference_target` to `@!vault:%{{`; **edge caveat below** |
| 9 | `claudine/lib/src/stream/providers/opencode.rs` | 323,370,405 | unaffected — `OpenCodeTool::resolve`, not `FileReference` |
| 10 | `darkmatter/cli/src/commands/frontmatter.rs` | 258 | **migrates to repo-first** — top-level editor-target CLI arg |
| 11 | `darkmatter/cli/src/io/mod.rs` | 47 | **migrates to repo-first** — top-level `md` input-file CLI arg |
| 12 | `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs` | 205, 209 | **needs explicit transition policy** — canonical document-first resolver (`resolve_file_ref_with_fallback`) |
| 13 | `darkmatter/lib/src/markdown/compose/link_resolve.rs` | 145, 147 | **needs explicit transition policy** — link `LocalPath` targets are inherently document-relative |
| 14 | `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs` | 128 | unaffected — gated by `is_file_reference_target`; bare relative handled by separate `PathBuf` branch; **edge caveat below** |
| 15 | `darkmatter/lib/src/markdown/schemas/detect.rs` | 240 | **needs explicit transition policy** — `file`-type detection is document-relative by design |
| 16 | `darkmatter/lib/src/markdown/schemas/format.rs` | 340, 341 | **needs explicit transition policy** — `file`-typed schema value validator depends on document-first ordering |
| 17 | `darkmatter/lib/src/markdown/schemas/resolve.rs` | 293, 358, 858, 1257 | **needs explicit transition policy** — `try_bare_name_in_roots` walks schema roots nearest-first |
| 18 | `darkmatter/lib/src/markdown/reference/mod.rs` | 46 | **needs explicit transition policy** — `resolve_transclusion_target` document-relative (`resolve_relative`) |
| 19 | `darkmatter/lib/src/markdown/reference/graph.rs` | 855 | **needs explicit transition policy** — reference-graph node IDs from document-relative targets |
| 20 | `darkmatter/lib/src/markdown/reference/validate.rs` | 517, 658 | **needs explicit transition policy** — local-path / cross-doc-fragment validation, document-relative |

### Tally (from the table, authoritative)

- **migrates to repo-first: 5** — #1, #5, #7, #10, #11 (all top-level CLI / compose entry points where a bare name is implicit and repo-first is the intended new behavior).
- **needs explicit transition policy: 8** — #12, #13, #15, #16, #17, #18, #19, #20 (the Darkmatter compose / schema / reference document-relative resolvers). These are the surfaces Phase 5 migrates onto the shared context: they must stay document-first for their nested-document candidates, which the spec expresses as repository-root-**then**-source ordering with the launch-area fallback removed for nested docs (D2). "Needs a policy" here means each must adopt the shared repository-first-then-base implicit order **via the detailed context** rather than silently inheriting the raw `resolve_from` flip — NOT that they keep a permanent old-CWD-first dual behavior (D4 forbids that).
- **unaffected: 7** — #2, #3 (def/doc), #4, #6 (resolve only `Absolute`/explicit kinds), #8, #14 (guarded to special sigils), #9 (not `FileReference`).

### Cross-cutting caveats for later phases

1. **`%recursive` and `{{ENV}}`-interpolated forms can parse to
   `ImplicitRelative`.** So the "guarded to special forms" unaffected sites (#8,
   #14) have a narrow edge that prefers base-first today. Phase 6/7 must confirm
   these edges route through the shared builder rather than the plain-`PathBuf`
   branch.
2. **The three `reference/*` files (#18-20) use `resolve_relative`, which
   resolves through ambient CWD internally, not `base_dir`** (it only uses
   `base_dir` to relativize the *result*). This is a **pre-existing divergence**
   from their stated document-relative intent, and the precedence flip flows
   through it. Phase 5 must migrate these onto the explicit context (D2) so they
   stop reading ambient CWD — this is exactly the "no late ambient CWD" clause
   of AC12.

---

## 2. Fixture / prompt collision inventory

**Headline:** authored **bare (implicit)** file references are rare in shipped
authoring documents. **Exactly one** shipped prompt authors a bare
collision-eligible reference; everything else uses `@` magic or explicit
`./`/`../` (both untouched by the precedence flip). **No case exists today where
a repo-root candidate and a source-relative candidate both physically exist**,
so there is no *live* mis-resolution — the flip is safe against the current
tree. Findings are ranked by intent risk.

### 2.1 Shipped authoring documents (`prompts/`)

| File:line | Reference | Form | Bare? | Collision live? | Intent | Notes |
|---|---|---|---|---|---|---|
| `prompts/faster-builds-and-tests.md:8` | `::file _senior-reviewer.md` | Darkmatter `::file` | **yes** | No (single candidate) | **source-local** | Target is the sibling `prompts/_senior-reviewer.md`; no repo-root twin. **Prime `./` rewrite candidate.** After the flip, adding any repo-root `_senior-reviewer.md` would silently redirect it. |
| `prompts/code-comment-quality.md:23` | `::file @docs/comment-quality.md` | `::file` | no (`@`) | no | repository | `@` magic → `docs/comment-quality.md`; unaffected. |
| `prompts/_reviews/feature-review.md:41` | `::file ../_senior-reviewer.md` | `::file` | no (`../`) | no | source-local (explicit) | Explicit relative; unaffected. Safe form. |
| `prompts/feature.md:36-43` | `::file @prompts/feature-prompts/*.md when=…` | `::file` | no (`@`) | no | repository | All `@`-prefixed; unaffected. |
| `prompts/_reviews/suggestion-review.md:34` | `::file @{{review}}` | `::file` | no (`@`+interp) | no | repository | `@` magic + interpolation; unaffected. |
| `prompts/daily.md:2` | `sequence: "$(sniff repo packages --list)"` | lifecycle `sequence:` | no | no | n/a | Shell substitution, not a path. |

### 2.2 Committed data fixtures (non-Rust)

| File:line | Reference | Form | Bare? | Collision live? | Intent |
|---|---|---|---|---|---|
| `darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_fileref/doc.md:3` | `$schema: - "./arm-a.yaml"` | `$schema:` | no (`./`) | no | source-local (explicit); sibling exists. Safe form. |
| `claudine/lib/tests/fixtures/providers/opencode.ndjson:65` (+ `codex.ndjson:35,53`) | `sequence: path/to/file.yaml` | text in NDJSON | yes | no | unambiguous — illustrative captured output, never resolved. |
| `darkmatter/lib/tests/fixtures/render_tree/links_images.md:2` | `![alt](image.png)` | md image | yes | no | unambiguous — render-tree fixture, not resolved. |

### 2.3 Rust inline (`TempDir`) fixtures authoring bare references

These write both source doc and target into one `TempDir` used as **both** base
dir and repo root, so each is **single-candidate (no true collision today)** but
encodes source-local intent by construction. They become flip-sensitive only if
a test later gives them a repo root distinct from the source dir.

- **`claudine/lib/src/composition/sequence/tests.rs:360`** — `sequence: steps.yaml`
  in `relative_path_resolves_from_source_dir`. **Behavioral canary:** this test
  explicitly asserts source-local precedence and will need deliberate review in
  Phase 7 (it exercises the exact contract being changed).
- `darkmatter/lib/tests/reference_integration.rs` — many bare `::file child.md`,
  `a.md`–`e.md`, `mid.md`, `leaf.md`, `sub/child.md`, `::code example.rs` (lines
  44, 108-109, 127, 153-154, 181-184, 205, 277, 295, 317, 346, 376, 447, 581,
  608, 640, 675, 712, 732, 755, 788, 823, 865, 901, 1078, 1114).
- `darkmatter/lib/src/markdown/compose/tests/rendering.rs:980` — `::file _senior-reviewer.md`.
- `darkmatter/lib/src/markdown/compose/tests/shell.rs:403,427` — `::file child.md`.
- `darkmatter/lib/tests/shell_block_integration.rs:246,276,307` — `::file child.md`.
- `darkmatter/lib/tests/set_overlay_integration.rs:58` — `::file child.md`.
- `claudine/cli/tests/sequence_magic_reference.rs:31` — `sequence: steps.yaml`; `:234` — `sequence: does-not-exist.yaml`.

Error-path fixtures (target intentionally absent → unambiguous, no collision):
- `claudine/lib/src/composition/error/tests.rs:1651` — `proxy: nope.md`.
- `claudine/cli/tests/characterization_error_routes.rs:205,266` — `proxy: "no/such/target.md"`.
- `claudine/cli/tests/effective_diagnostic_render.rs:115,197` — `proxy: "no/such/target.md"`.

### 2.4 SOURCE-LOCAL INTENT — candidates for `./` rewrite (Phase 4/8)

1. **`prompts/faster-builds-and-tests.md:8`** — `::file _senior-reviewer.md` →
   `::file ./_senior-reviewer.md`. **Highest priority** — the only shipped
   authoring document with a source-local bare reference. Sibling
   `prompts/_senior-reviewer.md` is the intended target.
2. `claudine/lib/src/composition/sequence/tests.rs:360` — `sequence: steps.yaml`.
   Canary test encoding source-local precedence; confirm/adjust deliberately in
   Phase 7, do not leave green-but-wrong.
3. `darkmatter/lib/src/markdown/compose/tests/rendering.rs:980` — `::file _senior-reviewer.md`
   (mirrors the shipped prompt; source-local intent).
4. Bulk source-local `::file child.md` / `a.md`–`e.md` / `sub/child.md` inline
   fixtures (§2.3) — sibling files in one TempDir; source-local intent, no live
   collision, flip-sensitive only if given a distinct repo root.

---

## 3. Extended migration sweep (beyond the spec list)

The spec's named migration list is a **minimum, not an allowlist** (D5). Sweep
of `claudine/` and `darkmatter/` for ad-hoc resolution bypassing `FileReference`,
grouped by the five categories. **Known spec-tracked sites confirmed present**
(harness/resolve.rs, composition/sequence.rs, system_prompt/resolve.rs, cli
sequence.rs, composition/resolve.rs; darkmatter expression/resolve_ctx.rs,
link_resolve.rs, transclusion/resolver.rs, schemas/{format,resolve,rewrite,validate}.rs).
The sites below are **NEW / additional**.

### 3.1 Fallback `join` building a file-reference candidate

- `claudine/lib/src/messaging/resolve.rs:169,174,179` — `resolve_image_path`:
  full ad-hoc ladder joining `cwd` → `repo_root` → `std::env::current_dir()`
  onto a relative image path; no `FileReference`.
- `darkmatter/lib/src/markdown/reference/graph.rs:858,865` — after a
  `FileReference` attempt, falls back to `base_dir.join(raw_target)`.
- `darkmatter/lib/src/markdown/compose/link_normalization.rs:64` — `parent.join(raw)`
  to absolutize an authored link (distinct file from the known `link_resolve.rs`).
- `darkmatter/lib/src/style/bespoke.rs:218,225` — stylesheet ref via
  `source.parent().join(path)` then `current_dir().join(path)`.
- `darkmatter/lib/src/effects/verbs.rs:20` (`EffectEngine::resolve`) and
  `darkmatter/lib/src/effects/fs_write.rs:50` (`normalize_within`) — join a
  relative authored path onto `mutation_root`; effect-verb resolution fully
  bypasses `FileReference`.
- `darkmatter/cli/src/commands/frontmatter.rs:267,279` and
  `darkmatter/cli/src/commands/code_block.rs:224` (`resolve_file_path_raw`) —
  `current_dir().join(raw)` fallback.
- `claudine/lib/src/linking/paths.rs:149,157`, `linking/canonical.rs:187,194`,
  `linking/detector.rs:305,312` — a cohesive **linking-module** resolution
  family joining `home_dir`/`repo_root` onto authored provider-resource paths;
  none via `FileReference`. Consider migrating as a unit (but confirm scope: these
  resolve provider-config resource locations, which may be out of D5 scope).
- `darkmatter/dmls/src/graph/arena.rs:938` (`normalize_join`) — hand-rolled
  lexical base+rel join with `/`-reset for buffer paths.

### 3.2 `canonicalize` in resolution paths (silent degrade)

- `darkmatter/lib/src/markdown/reference/graph.rs:859,866` —
  `abs.canonicalize().unwrap_or(abs)`: failure silently degrades, can desync
  node IDs from `source_to_id`.
- `darkmatter/lib/src/markdown/compose/link_normalization.rs:65` (`.ok().or(...)`),
  **`:103-115`** (source-path canonicalize failure warns then `return Ok(())`,
  aborting all normalization — notable silent degrade-to-noop), `:126` (`unwrap_or`).
- `darkmatter/lib/src/markdown/compose/file_links/discovery.rs:86-87` — local
  canonicalize helper `unwrap_or_else(|_| path.to_path_buf())`, used at
  `:34,81,201,280,291`.
- `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs:370-371` — same
  silent-degrade helper, used at `:282`.
- `claudine/lib/src/system_prompt/context.rs:149-150` (`canonical_or_self`) —
  used in package-area resolution.
- `claudine/lib/src/mcp/state.rs:218` — `fs::canonicalize(path).ok()`.
- Lower priority (display/error-context only): `darkmatter/lib/src/markdown/mod.rs:196,219,985`;
  `claudine/lib/src/render/prompt/system.rs:84`.

### 3.3 Manual tilde (`~`) expansion

- `claudine/lib/src/messaging/resolve.rs:160-165` — `strip_prefix("~/")` + `dirs::home_dir()`.
- `claudine/lib/src/reporting/paths.rs:8-18` (`expand_path`) — general-purpose
  `~` expander.
- `claudine/lib/src/protect/service.rs:146` — `!path.starts_with('~')` branch.
- `claudine/cli/src/commands/wrap/harness_orch/prompt.rs:52` — `!path.starts_with('~')`
  filter on candidate prompt paths.
- **Divergent home sources to unify:** transclusion `resolver.rs:83` uses
  `std::env::var("HOME")`; expression `resolve_ctx.rs:103-104` uses
  `dirs::home_dir`; `messaging/resolve.rs` uses `dirs::home_dir`;
  `biscuit-file` `context.rs:157` uses `env_os("HOME")`. D11 wants ONE
  cross-platform provider.
- Reverse-direction `~/`-aliasing for display (still hand-rolled home logic):
  `darkmatter/.../expression/path_projection.rs:43-46`, `compose/util.rs:31-34`,
  `compose/link_normalization.rs:127`, `claudine/lib/src/stream/path_link.rs:47`.

### 3.4 Prefix / `is_absolute` classification

- **`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:1604,1673,1676-1683`**
  — `resolve_path_shape`: a full hand-rolled classifier (`is_absolute`,
  `starts_with("./")`/`"../"`/`== "."` → `base_dir.join`, `strip_prefix('@')` →
  magic scan) used as the fallback when `FileReference` misses. **Significant** —
  a complete parallel grammar.
- `claudine/lib/src/messaging/resolve.rs:155` — `is_absolute` branch (image ladder).
- `claudine/lib/src/stream/path_link.rs:35` — `is_absolute` gate (link rendering).
- `darkmatter/lib/src/style/bespoke.rs:213`, `effects/fs_write.rs:47`, `effects/verbs.rs:17` — `is_absolute` branching.
- `claudine/lib/src/linking/{paths.rs:146,154, canonical.rs:184,191, detector.rs:302,309}` — linking-family `is_absolute` branching.
- `darkmatter/cli/src/commands/frontmatter.rs:274`, `code_block.rs:219` — `is_absolute`.
- `claudine/cli/src/completion/schema_completion/mod.rs:73` — `raw.is_absolute() && raw.is_file()` during completion.
- `darkmatter/dmls/src/graph/{arena.rs:944, node.rs:395}` — dmls buffer-path classification.
- Borderline (URL-vs-local in reference parsing): `darkmatter/lib/src/markdown/reference/{types.rs:213, validate.rs:856}` (`starts_with("//")`).

### 3.5 Resolver-error suppression

- `darkmatter/cli/src/commands/frontmatter.rs:255-281` (`run_edit`) — `Err(_)`
  arm discards the typed `FileReferenceError` and falls back to
  `current_dir().join`; `:259` also `.wrap_err(...)`-flattens.
- `darkmatter/lib/src/markdown/reference/graph.rs:851,855` — `if let Ok(...)` on
  both `FileReference::new` and `resolve_relative`, silently dropping the typed
  error into the manual-join fallback (§3.1).
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:1666` — `if
  let Ok(Some(p)) = resolve_arg(...)` swallows the resolver error, then falls to
  `resolve_path_shape`.

### 3.6 Migration-owner notes

- **Linking module** (`claudine/lib/src/linking/{paths,canonical,detector}.rs`)
  is a cohesive, previously-unlisted resolution family. Flag for a scope
  decision: it resolves provider-config *resource* locations, which may fall
  outside D5's document-reference scope — decide in Phase 6/7 whether it migrates.
- `darkmatter/lib/src/effects/{verbs,fs_write}.rs` and `style/bespoke.rs` resolve
  paths with **no `FileReference` involvement at all** — pure bypass.
- `darkmatter/.../reference/graph.rs` and `expression/functions/mod.rs:1653+` are
  the clearest "try `FileReference`, then re-implement on miss" hybrids — they
  both bypass **and** suppress the typed error.
- `claudine/lib/src/messaging/resolve.rs::resolve_image_path` is a standalone
  ladder (absolute → tilde → cwd → repo_root → process cwd) hitting four of the
  five categories at once.

> **Scope caution (Rule 2/3):** several §3 sites — the linking family, dmls
> buffer paths, messaging image resolution, effect-verb `mutation_root`
> resolution — are adjacent resolution logic that is **not** document-backed
> file-reference resolution in the spec's D5 sense. They are recorded here for
> completeness; each needs an explicit in/out-of-scope ruling at its consuming
> phase rather than being swept in reflexively.

---

## 4. Baseline record

Captured on branch `error-prop-and-file-resolution` at Phase 1. `just lint` +
`just test` (L1/unit tier) per area. **All three areas are GREEN with ZERO
pre-existing failures** — any red in Phases 2–8 is attributable to this
feature's changes, not inherited debt.

| Area | `just lint` | `just test` result |
|---|---|---|
| **biscuit-file** | ✅ EXIT 0 | lib: **284 passed**, 4 skipped · cli: **61 passed** |
| **darkmatter** | ✅ EXIT 0 | lib: **5607 passed** (102 slow), 135 skipped · cli: **555 passed** (14 slow), 71 skipped · dmls: **566 passed**, 3 skipped |
| **claudine** | ✅ EXIT 0 | lib: **3526 passed** (10 slow), 7 skipped · contract: **47 passed**, 5 skipped · cli: **1996 passed** (100 slow), 167 skipped · gen: **152 passed**, 4 skipped |

**No pre-existing failures to carry forward.**

### Baseline-capture note (infrastructure)

Running `just test`/`just lint` for these areas **fails under the default Bash
sandbox** (exit 144 — the sandbox kills `cargo`/`clippy`). The clean baseline
above was obtained by running each area's recipes with the sandbox disabled.
Running all three areas' `just test` **in parallel also fails** via cargo
build-directory lock contention (`Blocking waiting for file lock`) — run them
**sequentially**. Later phases should budget for this: one area's full
`just test` compile + run is multi-minute, and the three cannot share a build
lock concurrently.

### Behavioral canary tests to watch (will change meaning in Phase 4)

- `biscuit-file/lib/tests/implicit_relative.rs:49` —
  `prefers_cwd_over_git_root_on_name_collision` (Phase 4 **inverts** this).
- `claudine/lib/src/composition/sequence/tests.rs:360` —
  `relative_path_resolves_from_source_dir` (source-local precedence canary).
- `claudine/cli/tests/level2_typed_error_render_capture.rs:580` —
  `level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux`
  (AC5 pinning test; Phase 6/8 **promotes** it to full parity, does not weaken).
- ~21 Darkmatter tests ratifying document-first/launch-fallback (enumerated in
  `plan.md:215`) — Phase 5 must update, not leave green-but-wrong.
