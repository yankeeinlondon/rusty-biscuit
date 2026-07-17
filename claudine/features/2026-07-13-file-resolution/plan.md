---
created: 2026-07-16
phase: 2
total_phases: 8
agent: claude/default
yolo: true
spec: ./spec.md
depends_on:
    - ../2026-07-13-error-propogation/spec.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_1:
    - claudine/features/2026-07-13-file-resolution/decisions.md
    - claudine/features/2026-07-13-file-resolution/inventory.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/reference_grammar.rs
    - biscuit-file/lib/tests/resolution_context.rs
docs_updated_during_phase_2:
    - biscuit-file/docs/dependencies.md
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
    - biscuit-file
---

# Unified File-Reference Resolution — Execution Plan

Derived from [`spec.md`](./spec.md). Grounded against HEAD on branch `claudine`.

## Shape of the Work

`biscuit-file` is the critical path. Darkmatter and Claudine cannot migrate until
the shared library exposes public classification, an explicit resolution context,
a detailed resolve outcome, and the flipped implicit precedence. Phases 2–4 build
that; Phases 5–7 consume it and can run largely in parallel; Phase 8 proves the
contract end-to-end.

```text
P1 audit/gates ──► P2 classification+context ──► P3 detailed resolver ──► P4 precedence flip
                                                                             │
                                              ┌──────────────────────────────┼───────────────┐
                                              ▼                              ▼               ▼
                                        P5 darkmatter                  P6 claudine     P7 claudine
                                                                        harness/proxy   sequence/CLI
                                              └──────────────┬───────────────┘───────────────┘
                                                             ▼
                                                    P8 L2 + docs + acceptance
```

### Grounding Facts (verified, not assumed)

| Fact | Evidence |
|---|---|
| `ReferenceKind` is `pub(crate)`; no public classification exists | `biscuit-file/lib/src/file_reference/mod.rs:402-411` |
| Implicit order today is **base/CWD then git root** | `biscuit-file/lib/src/file_reference/resolve.rs:190-198` |
| `biscuit-file` does **not** depend on `sniff` (cycle risk is real: `sniff` → `biscuit-file`) | `biscuit-file/lib/Cargo.toml:40-80` |
| `home_dir()` reads `$HOME` only — returns `None` on native Windows | `biscuit-file/lib/src/file_reference/context.rs:157-159` |
| No `Home` kind exists in the grammar at all | `parse.rs:64-88` |
| Interpolation runs **after** classification is fixed | classify `parse.rs:87`; interpolate `resolve.rs:273-292`, called `resolve.rs:31` |
| Claudine has **four** private grammars, not one | `harness/resolve.rs:25`, `composition/sequence.rs:84`, `system_prompt/resolve.rs:190`, `cli/src/commands/sequence.rs:97` |
| Darkmatter's document-first+fallback contract exists on **only 2 of 5** surfaces | present: `expression/resolve_ctx.rs:200`, `schemas/format.rs:324` — absent: `transclusion/resolver.rs:128`, `link_resolve.rs:147`, `schemas/resolve.rs:356` |
| `claudine/lib` already depends on both `sniff` and `biscuit-file` | `claudine/lib/Cargo.toml:15,17` |
| Workspace `resolve()`/`resolve_from()` call sites: **20 files** | see Phase 1 audit list |

## Decision Gates

These four gates block Phase 2. Each has a recommended default so execution is
not stalled; record the ruling in `decisions.md` before writing code.

### G1 — Ordering against error-propagation (**highest risk**)

`../2026-07-13-error-propogation/spec.md` is `status: draft` with **no plan.md**,
and its reader note (`:41-50`) asserts it must land *first*. It owns the Claudine
`composition.invalid_file_reference` wrapper and the `err.detail.*` catalog that
this feature's D8 populates. Taking that literally blocks this feature entirely
behind an unplanned one.

**Recommended default: split the seam.** The typed detailed outcome
(classification, ordered candidates, root provenance, probe dispositions) is
owned by `biscuit-file` and has **no** dependency on error-propagation — build it
here in Phases 2–4. Only the Claudine semantic wrapper and `err.detail.*`
projection defer to error-propagation. Phase 6 lands a narrow typed adapter that
error-propagation later widens, replacing its `null` projections with real data
per that spec's `:263-268` null contract. This keeps Phases 2–5 and 7 unblocked.

### G2 — `@` semantic collision (**verified low risk**)

`resolve_harness_path` defines `@foo` as **repo-root-relative**
(`harness/resolve.rs:46-53`); `FileReference` defines `@` as **magic-root
search**. These are different contracts on the same sigil, so migration silently
changes meaning.

**Verified:** zero `@`-prefixed `proxy:`/`sequence:` values exist in `prompts/`
or `.claudine/`. **Recommended default: adopt `FileReference` magic semantics
outright**, no compatibility shim. Record as an intentional behavior change.
Confirm the zero-usage finding still holds at execution time (Phase 1 task).

### G3 — Coordination with `biscuit-file/features/2026-06-13-resolve-tuple/`

That open spec adds `resolve_tuple()` and states `resolve()` and all existing
methods are unchanged. It is additive and non-conflicting, but it centralizes
path-abbreviation across seven copies and will want the same root/provenance data
this feature builds in D3.

**Recommended default: build D3's candidate/root plan as a reusable public
surface** so resolve-tuple consumes it rather than growing an eighth copy. Do not
implement resolve-tuple here.

### G4 — Acceptance criterion 4 vs. the already-applied workaround

Commit `2d7c847d4` ("make proxy paths relative") already rewrote every authored
proxy value to `./_implement/...`. The bare form from the motivating incident
**no longer exists in the tree**, so criterion 4 ("the motivating router
reference resolves successfully without rewriting it to `./`") cannot be proven
against live prompts as they stand.

**Recommended default: prove it with a dedicated L2 fixture** (Phase 8), and
separately decide whether to revert `prompts/` to the bare spelling once
repository-first lands. Reverting is optional — the `./` spelling remains
correct and pins source-local intent. Do **not** treat the revert as a gate.

---

## Phase 1 — Audit, Gates, and Collision Inventory

No production code. Output is `decisions.md` plus a written inventory that D12
requires **before** the precedence switch. Tasks 1–5 are parallelizable.

- [x] Record rulings for G1–G4 in `claudine/features/2026-07-13-file-resolution/decisions.md`, each with rationale
- [x] Re-verify G2's zero-usage claim: `grep -rn --include='*.md' -E '(proxy|sequence):[[:space:]]*@' prompts .claudine` returns empty
- [x] **[parallel]** D12 audit: classify all 20 `resolve()`/`resolve_from()` call-site files as *migrates to repo-first* / *needs explicit transition policy* / *unaffected*. Files: `biscuit-file/cli/src/main.rs`, `biscuit-file/lib/src/{file_reference/mod.rs,lib.rs}`, `claudine/cli/src/commands/{schema_interactive/mod.rs,sequence.rs}`, `claudine/gen/src/agent_errors_check.rs`, `claudine/lib/src/composition/{resolve.rs,sequence.rs}`, `claudine/lib/src/stream/providers/opencode.rs`, `darkmatter/cli/src/{commands/frontmatter.rs,io/mod.rs}`, `darkmatter/lib/src/markdown/compose/{expression/resolve_ctx.rs,link_resolve.rs,transclusion/resolver.rs}`, `darkmatter/lib/src/markdown/schemas/{detect.rs,format.rs,resolve.rs}` — **DONE** (`inventory.md` §1). 20 files confirmed (17 named + 3 additional `reference/{mod,graph,validate}.rs`). Tally: 5 migrate / 8 need-policy / 7 unaffected.
- [x] **[parallel]** Fixture collision inventory: for every committed Claudine/Darkmatter fixture and prompt authoring a bare reference, determine whether **both** repo-root and source-relative candidates exist. Classify each collision by author intent; list source-local intents for `./` rewrite — **DONE** (`inventory.md` §2). No LIVE collisions; one shipped source-local bare ref (`prompts/faster-builds-and-tests.md:8`) flagged for `./` rewrite.
- [x] **[parallel]** Extend the migration inventory beyond the spec's named list (D5 calls it a minimum, not an allowlist): sweep for fallback `PathBuf::join`, `canonicalize`, tilde expansion, prefix classification, and resolver-error suppression across `claudine/` and `darkmatter/` — **DONE** (`inventory.md` §3). Notable new sites: `expression/functions/mod.rs:1604+` (`resolve_path_shape` full parallel grammar), `messaging/resolve.rs` (5-rung image ladder), the `linking/` family, `darkmatter/effects` + `style/bespoke.rs` (pure `FileReference` bypass), `reference/graph.rs` (bypass + error suppression). Scope-caution note recorded for adjacent non-document-reference sites.
- [x] **[parallel]** Confirm whether `darkmatter/lib/Cargo.toml:90`'s `sniff` entry is a real dependency or dev-only (it sits below `[dev-dependencies]`). If dev-only, Phase 5 must promote it to supply `repository_root` — **RESOLVED: real dependency.** `[dependencies]` starts `Cargo.toml:28`, `sniff` is `:90`, `[dev-dependencies]` starts `:108`. Confirmed used in 46 non-test src sites (e.g. `compose/context/capture/snapshot.rs`, `editor/mod.rs`). Phase 5 does **not** need to promote it.
- [x] **[parallel]** Capture baseline: `just test` + `just lint` in `biscuit-file`, `darkmatter`, `claudine`; record pre-existing failures so later phases do not misattribute them — **DONE** (`inventory.md` §4). All three areas GREEN, zero pre-existing failures. Infra note: recipes need the Bash sandbox disabled (exit 144 otherwise) and must run sequentially (build-lock contention).

**Checkpoint 1:** `decisions.md` exists with G1–G4 ruled. Collision inventory
written. Baseline recorded. **No precedence change may begin until this passes.**

---

## Phase 2 — `biscuit-file`: Public Classification, Explicit Context, Home Kind

Foundation only — **no precedence change yet**, so this phase stays green against
existing tests. Satisfies D1, D2, D7, D11.

- [x] Add public `FileReferenceKind` + `FileReferenceClass` (`recursive` as a modifier over a kind, per D1) in `biscuit-file/lib/src/file_reference/mod.rs`; export from `lib.rs:137-148` — **DONE** (`mod.rs`, exported `lib.rs`).
- [x] Add a public accessor on `FileReference` returning its class. Keep `ReferenceKind`/`ParsedReference` internal — expose classification, not the parser types — **DONE** (`FileReference::class()`; `ReferenceKind::public_kind()` projects the internal enum).
- [x] Reconcile `CompletionEntryForm` (`mod.rs:46-54`) with the new public kind; it currently rustdoc-links a private type and covers only `Magic`/`ImplicitRelative` — **DONE** (rustdoc now links public `FileReferenceKind`).
- [x] Add the `Home` kind to `detect_kind` (`parse.rs:64-88`): recognize `~` and `~/...`, plus `~\...` on Windows. Reject `~user` with a typed error — never fall through to magic or implicit — **DONE** (`detect_home`; `FileReferenceError::UnsupportedUserHome`; `ReferenceKind::Home` resolves against `ctx.home_dir`).
- [x] **[parallel]** Windows path grammar in `detect_kind` (D7): `.\foo.md` / `..\foo.md` classify explicit-relative; `C:\...` and `\\server\share\...` classify absolute; `C:foo.md` stays drive-relative (**not** absolute); URL scheme match becomes ASCII case-insensitive and is checked before any drive/path classifier — **DONE** (`is_absolute_reference`/`is_explicit_relative`/`starts_with_ignore_ascii_case`; host-independent lexical classification so it is testable on macOS).
- [x] **[parallel]** Replace `home_dir()` (`context.rs:157-159`) with a cross-platform provider — `$HOME` alone is not a Windows contract. Prefer an existing workspace/ecosystem provider over hand-rolling `USERPROFILE` — **DONE** (`dirs::home_dir()`; `dirs = "6"` gated behind `file-reference`, matching the workspace convention).
- [x] Introduce the public resolution context (D2) carrying `source_path`, `base_dir`, `repository_root`, `package_area`, `home_dir`, the env snapshot, and configured magic/vault roots. Owned/borrowed shape is an implementation decision; candidate construction **must not** reread ambient state — **DONE** (`FileResolutionContext`, owned; consumed by `FileReference::resolve_in_context`, which reads no ambient state).
- [x] Make `repository_root` **caller-suppliable**. `biscuit-file` must not depend on `sniff` (`sniff` already depends on `biscuit-file` — that is a crate cycle). Ambient compatibility methods keep their internal `gix` discovery at `context.rs:56-79` — **DONE** (`ResolutionContext.repository_root` + `resolve_repository_root` seam; ambient `from_ambient`/`from_base` still fall back to `find_git_root`; no `sniff` dep added).
- [x] Implement lexical containment validation after absolutization — component-aware, **no** canonicalization through symlinks. A caller-provided root is accepted only when it contains the source, or the operation documents a different trust boundary — **DONE** (`FileResolutionContext::validate` via `normalize_components` + `Path::starts_with`; `resolve_in_context` calls it first).
- [x] Eliminate the three ambient reads that bypass the env snapshot: `resolve.rs:48` (live `std::env::var` in `interpolated_url_string`, which also silently re-emits `{{NAME}}` on miss), `mod.rs:223`/`mod.rs:357` and `resolve.rs:375` (`current_dir`), `resolve.rs:454` (direct `home_dir`) — **DONE for the snapshot-bypassing reads**: `interpolated_url_string` now sources from `ctx.env`; `magic_completion_roots` receives home captured once via the provider. The `current_dir` reads in the *ambient convenience* methods (`with_package_area_magic_path`, `resolve_relative`, `complete_partial`) are single-capture entry points that define those methods' ambient behavior; the new `resolve_in_context` path performs **zero** ambient reads (AC12). Those convenience reads are removed at the call sites when Claudine/Darkmatter migrate onto the context (Phases 5–7) — removing them from the shared methods now would break their contract or touch downstream, which this phase forbids.
- [x] Add typed missing-context failures for absent home/repo/base rather than silent `None` — **DONE for home** (`FileReferenceError::MissingHomeContext` from the `Home` arm) plus `RepositoryRootNotContainingSource` (containment). Absent-repo/absent-base become errors only once the flip / top-level surfaces make them reachable (Phases 3–4/7); under Phase 2's preserved precedence, absent repo still falls back to base by design.
- [x] L1 tests: parsing distinguishes explicit vs implicit **without filesystem access**; `~`/`~\` home-pinned and `~user` rejected; Windows drive/UNC/drive-relative and POSIX absolute cases (target-gated or pure-parser fixtures per D7) — **DONE** (`parse.rs` unit tests + `tests/reference_grammar.rs` pure-parser public-API tests; `tests/resolution_context.rs` for context/home/containment; `resolve.rs` unit tests for `Home`/missing-context/caller-supplied-root).

**Checkpoint 2:** `just test` + `just lint` green in `biscuit-file`. Public class
is observable. **Behavior unchanged** — the existing
`prefers_cwd_over_git_root_on_name_collision` test (`lib/tests/implicit_relative.rs:49`)
still passes. Nothing downstream is touched yet.

---

## Phase 3 — `biscuit-file`: Detailed Resolver, Candidate Plan, Probe Dispositions

Satisfies D3, D4, D8, D10. Still **no precedence change** — build the machinery,
flip in Phase 4.

- [ ] Extract a candidate/root **builder** separable from matching (D3), so diagnostics, completion, and tests can inspect the exact ordered candidates without reimplementing the algorithm
- [ ] Candidate records retain root provenance (`repository`, `source`, `package`, `home`, `magic`, `vault`) alongside the path — provenance is **data**, never inferred from string prefixes
- [ ] Dedupe duplicate lexical candidates **stably**, preserving first-seen order
- [ ] Add the context-aware detailed resolve operation (name is an implementation decision; `resolve_detailed`/`resolve_with_context` are representative) returning classification + ordered plan + match/failure
- [ ] Typed detailed outcome (D8) retains: raw authored reference, parsed kind, source/base, repository root when available, ordered candidates attempted, **per-candidate probe disposition** (missing / non-file / matched / I/O failure), failure classification, and underlying `FileReferenceError`. `NoMatch` is a typed outcome, not `Ok(None)`
- [ ] Replace `Path::is_file()` probing with a fallible metadata operation: `NotFound` and non-regular-file **advance** the search; permission/invalid-path/other I/O **stop** with candidate path + typed source attached. `is_file()` collapses these into `false` and is insufficient
- [ ] Preserve direct symlink-to-regular-file selection while `resolve_recursive` keeps `follow_links(false)` (`resolve.rs:141`) — these are distinct behaviors, both retained
- [ ] Route recursive resolution's roots through the shared builder while retaining its **global lexical winner** (`resolve.rs:176-178`). Changing recursive winner selection is explicitly out of scope
- [ ] Keep `resolve()`/`resolve_from()` at their `Result<Option<PathBuf>, _>` convenience shape; project them from the detailed outcome without changing order
- [ ] Fix `FileReferenceError::Git`/`Workspace` (`error.rs:15,18`) to carry `#[source]` — they currently stringify, so gix/cargo_metadata causes are not chainable, which D8's "underlying error where one exists" requires
- [ ] Request-scoped repository-root caching (D10): discovery keyed off base/source, never global mutable state; scoped so one worktree's root cannot leak into another. **Linked git worktrees must resolve to their own root**
- [ ] L1 tests: detailed resolver retains candidate/root provenance on both success and `NoMatch`; legacy methods project without reordering; missing candidates fall through while permission/metadata errors stay typed and identify the candidate; linked-worktree anchoring

**Checkpoint 3:** `just test` + `just lint` green in `biscuit-file`. A test can
assert the full ordered candidate plan. Downstream still untouched.

---

## Phase 4 — `biscuit-file`: Precedence Flip + Interpolation Anchoring

**The breaking change.** Gated on Phase 1's collision inventory. Satisfies D4,
D12, OQ1.

- [ ] Flip `collect_roots` for `ImplicitRelative` (`resolve.rs:190-198`) to **repository root first, then base**; keep the `git_root != base` dedupe. When no git root is discoverable, base is the only candidate
- [ ] Mirror the flip in `implicit_relative_completion_roots` (`resolve.rs:466-482`) — it independently duplicates the order today and would otherwise teach the opposite of what executes (D9)
- [ ] Implement OQ1 **option 2** (reclassify after interpolation): expand once from the captured env snapshot, then classify **filesystem anchoring only** (absolute / explicit-relative / implicit-relative) for the payload
- [ ] Interpolation must **not** inject `@`, `!`, `%`, `vault:`, or a remote URL scheme — those sigils stay author-controlled. Reject rather than honor an injected sigil
- [ ] Diagnostics record **both** authored kind and effective anchoring. This closes the silent bug where `{{DIR}}/foo.md` classifies implicit (`parse.rs:87`, test `parse.rs:212-214`) yet `PathBuf::join` at `resolve.rs:259` discards the root when `DIR` expands absolute — the inverse guard already exists at `resolve.rs:243-250`
- [ ] Apply the audited transition policy from Phase 1: any call site genuinely needing CWD-first selects a **shared explicit** policy living in `biscuit-file`, explicit at the call site, documented, and unused by every Claudine/Darkmatter surface in scope. No permanent dual-behavior layer; no Claudine-only candidate loop
- [ ] Rewrite fixtures/prompts flagged as source-local intent to `./`. Leave unambiguous bare references alone
- [ ] Invert `prefers_cwd_over_git_root_on_name_collision` (`lib/tests/implicit_relative.rs:49`) and close the coverage gap the existing test names — `collect_roots` for `ImplicitRelative` is currently only unit-tested in the no-git-root branch (`resolve.rs:591-612`)
- [ ] L1 tests: repo-before-base for implicit; explicit yields exactly **one** base-relative candidate with no fallback; both-exist → repo wins; only-source-exists → falls back; no-git-root → base only; duplicate roots dedupe stably; special kinds unchanged; interpolation authored/effective-kind diagnostics
- [ ] **[parallel]** Update `biscuit-file/docs/topics/file-references.md` — the sigil table (`:26`), the implicit prose (`:50-54`), the example (`:56-59`), and the `Ok(None)` note (`:61-62`) all currently document CWD-first and become wrong the moment this phase lands. Clarify `resolve_from`'s base-vs-CWD wording (`:329`)
- [ ] **[parallel]** Update the `biscuit-file` skill reference to the ratified terminology and precedence; refresh its `hash:` via `md hash <file>`

**Checkpoint 4:** `just test` + `just lint` green in `biscuit-file`. Docs, skill,
implementation, and tests agree on repository-first (acceptance 7). Downstream
packages may now be red — that is expected and Phases 5–7 close it.

---

## Phase 5 — Darkmatter Migration

Satisfies D5's Darkmatter surfaces. **Parallelizable with Phases 6–7.** The
document-first contract already exists on expression + schema-`file` surfaces;
transclusion and link-resolve never received it — that asymmetry is the bulk of
the work here.

- [ ] Promote `sniff` to a real dependency in `darkmatter/lib/Cargo.toml` if Phase 1 confirms it is dev-only; supply `repository_root` via `sniff::filesystem::git::repo_root` / `GitRepo::discover` into the shared context
- [ ] Migrate `resolve_file_ref_with_fallback` (`compose/expression/resolve_ctx.rs:200-211`) onto the detailed resolver. **Per D2 the launch-area fallback is removed for nested documents** — only repository and authoring-document candidates participate; launch dir stays a base only for top-level references
- [ ] Close the legacy hatch at `compose/context/options.rs:331-332` where `file_ref_fallback_dir: None` preserves ambient-CWD behavior
- [ ] Migrate `schemas/format.rs:324-343` — remove the `None`/`None` → `reference.resolve()` ambient-CWD escape at `:341`, and the CWD re-read at `:348-357` that leaks process CWD into diagnostics even for anchored paths
- [ ] Migrate `schemas/rewrite.rs:308` (`rewrite_file_value`) and `schemas/validate.rs` `FileAnchors` (`:228-234`, `:572-582`) to the shared context
- [ ] Migrate `$schema` resolution (`schemas/resolve.rs:305-358`): the sibling probe `base_dir.join(trimmed)` at `:337` and the `resolve_from(base_dir)` at `:356-358`. Reconcile the doc contradiction — `docs/topics/schema-definition.md:358` claims `$schema` uses "the same order" while `:758` says document-parent only
- [ ] Migrate transclusion `resolve_path` (`compose/transclusion/resolver.rs:64`): delete the `is_file_reference_target` classifier (`:191-197`), replace the ambient `file_ref.resolve()` at `:128` with the context-aware resolver, remove the `~` arm at `:82-101` (now a shared kind), and remove the manual join at `:170-174`. Its doc-comment at `:58-63` already describes the intended contract rather than the shipped one
- [ ] Migrate `link_resolve.rs`: the ambient `resolve()` at `:147` and the post-miss `dir.join(raw)` fallback at `:157-163` that bypasses shared classification and diagnostics. Note `:151`/`:160` silently degrade on `canonicalize` failure
- [ ] Every nested file-backed document establishes a **new** `base_dir` (D2) — transcluded/included documents become the source for their own references; the entry document's directory must not leak inward, while request-level worktree data stays available
- [ ] **[parallel]** Update the ~21 tests that ratify document-first/launch-fallback: `expression/resolve_ctx.rs:262,284,304`; `expression/functions/mod.rs:3296,3321,3347`; `schemas/mod.rs:1655,1681,1707,1740,1795,1820,1844,1869`; `compose/schema_validation.rs:1324`; `compose/tests/schema.rs:376`; `compose/cache/hashing.rs:692`; `compose/preflight/collect.rs:868`. Leaving them green-but-wrong is contract drift
- [ ] **[parallel]** Update the load-bearing contract comments: `schemas/format.rs:8-10,306-315,334`; `schemas/rewrite.rs:16,60-63,323`; `schemas/validate.rs:228-234,572-582`; `schemas/mod.rs:211-216`; `compose/context/options.rs:324-332`; `expression/resolve_ctx.rs:26-30,177-199`; `expression/functions/mod.rs:1433-1440`
- [ ] **[parallel]** Update Darkmatter docs claiming every plain relative path is source-relative: `docs/inline/file-links.md:22,61-65`; `docs/transclusion/transclusion-design.md:56,275,278,593`; `docs/transclusion/block-transclusion.md:40-42,56`; `docs/topics/schema-definition.md:358-360,758`; `docs/topics/darkmatter-expressions.md:438,472,487-488,852`; `docs/inline/fm-interpolation.md:132`; `docs/topics/magic-paths.md:9`. **`docs/topics/context-variables.md:67` directly contradicts the shipped order** and must be corrected
- [ ] Verify `file_ref_fallback_dir` stays folded into `options_hash` (`compose/cache/hashing.rs:203`) — cache correctness depends on context identity

**Checkpoint 5:** `just test`, `just test-l2`, `just lint` green in `darkmatter`.
No Darkmatter resolution path reads ambient CWD after context capture.

---

## Phase 6 — Claudine Migration: Harness and Proxy Routes

Satisfies D5's lifecycle surfaces, D6, and D8's Claudine half.
**Parallelizable with Phase 5.**

- [ ] Delete the private grammar in `resolve_harness_path` (`claudine/lib/src/harness/resolve.rs:25-68`). Its three branches — absolute `:41-43`, `@`→repo-root `:46-53`, and **everything-else → `source_dir.join` `:56-67`** — are the entire defect: `./foo.md` and `foo.md` take the identical path, so implicit never tries repo root. Make it a thin typed adapter over `FileReference` or remove it
- [ ] Apply the G2 ruling: `@` becomes magic-root search, not repo-root join. Record as an intentional behavior change
- [ ] Build the request-scoped `FileResolutionContext` from `HarnessResolutionContext` (`resolve.rs:10-15`), sourcing `repository_root` via `sniff` (`claudine/lib/Cargo.toml:15` already has it) and reusing it across the run per D10 — not per reference
- [ ] Funnel **all four** proxy routes through one resolver with the context of the document authoring the current target (D6): `harness_orch/loop_control/proxy.rs:82-94`, `composition/pipeline.rs:1157-1175`, `harness_orch/loop_control/control_dispatch.rs:176-215`, `composition/looping/engine.rs:414-450`
- [ ] **Fix the live latent bug:** `control_dispatch.rs:181` calls `resolve_harness_path` directly, bypassing `resolve_proxy_target`'s existence check (`composition/lifecycle/control.rs:246-251`), so a `failure`-stack proxy to a missing file swaps `source_path` to a nonexistent path at `:209-214` instead of failing
- [ ] **Nested proxy provenance (D6):** when a proxied target authors another proxy, the *target* document becomes the new source. Retaining the original source path is a context-provenance bug. No route may `PathBuf::join` the target directly
- [ ] Preserve `RematerializeInputs.file_ref_fallback_dir` threading (`composition/types.rs:500`, populated `prepare.rs:181-183,357-359`, forwarded `harness_orch/prompt.rs:80-82`) — this is the **only** path carrying the launch-area anchor into a proxied target's compose. Reconcile it with D2: launch dir is a base for **top-level references only** and must not become a third fallback for nested documents
- [ ] Land the typed diagnostic per the G1 ruling: replace `detail: String` flattening at `harness/resolve.rs:34,61-64`, `lifecycle/control.rs:249`, and `harness/audit.rs:34` (full typed-error collapse). Fix `harness/error.rs:122-123` where `PathResolutionFailed` emits only `path` and **drops `detail`** from the diagnostic payload
- [ ] Stop suppressing typed errors into `eyre!` strings at `loop_control/proxy.rs:89-91`, `control_dispatch.rs:183`, `pipeline.rs:1162`. Follow the pattern already done right at `looping/engine.rs:428-437` (`LifecycleErrorInfo::from_harness_error`), which is the only route threading `err.code`/`err.detail.*` today
- [ ] Widen the boundary lint at `claudine/lib/tests/boundary_lint.rs:60-63` — it is a literal-substring check against one file and will **not** catch the three `eyre!` flattening sites above
- [ ] L1 tests: proxy delegates to `FileReference` and returns the shared typed diagnostic; **every route produces the same result for the same source/context/reference tuple**; a nested proxy uses the proxied document as its own source

**Checkpoint 6:** `just test` + `just lint` green in `claudine`. All four proxy
routes provably agree (acceptance 5).

---

## Phase 7 — Claudine Migration: Sequence, System-Prompt, CLI, Completion

Satisfies D5's remaining surfaces, D9, D11. **Parallelizable with Phases 5–6**;
touches disjoint files from Phase 6.

- [ ] Migrate `resolve_sequence_reference` (`composition/sequence.rs:84-136`): delete the private `~` expansion at `:87-94` (now a shared kind per D11), the `is_file_reference_target` classifier at `:140-146`, and the **plain-relative manual join at `:134-135`** which conflates implicit with explicit exactly as the harness resolver did. Note `./`/`../` currently fall through to that join by design (`:82-83`) and `~` never reaches `FileReference` at all
- [ ] Preserve the `@/x` → `@x` normalization (`sequence.rs:98-102`) as explicit `FileReference` configuration, not local string surgery
- [ ] Migrate system-prompt **file reference** `resolve_file_ref` (`system_prompt/resolve.rs:190-203`) — a third private grammar: absolute passthrough, else `cwd.join(path)` at `:195`, with no `@`/`~`/magic/repo-root support. Callers: `:52` (`--append-system-prompt`), `:65` (`--replace-system-prompt`)
- [ ] **Leave `discover_standard_file` (`system_prompt/resolve.rs:80-115`) alone.** Per D5 it is filename **discovery** over known anchors, not reference resolution. Name it as such in its docs so the distinction survives
- [ ] Unify the divergent sequence-source resolvers: `resolve_sequence_source` (`cli/src/commands/sequence.rs:88-100`) falls back on `NotMarkdown` to a **bare** `FileReference::new` at `:97` that never gets the `prompts/` magic roots — so `@foo.yaml` resolves differently from `@foo.md` today
- [ ] Fix the composition enrichment divergence: `read_source_text_for_enrichment` (`composition/resolve.rs:171-186`) uses only `.with_package_area_magic_path()` and **skips** `with_prompt_magic_paths` (`:106-124`), so a file that resolved at launch via a `prompts/` root fails to re-resolve for error enrichment and silently degrades the render (`:165-168`)
- [ ] Keep Claudine's prompt magic roots (`composition/resolve.rs:133-151`) as explicit `FileReference` configuration per D1 — package area, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `~/.claudine/prompts`
- [ ] Preserve the interactive autocomplete fallback at `cli/src/commands/compose/prep.rs:380-389` — it **keys on the `FileNotFound` error variant**, so changing error types breaks it silently. Re-point it at the typed no-match outcome
- [ ] D9: completion must emit values that execute through the **same** candidate builder in the same context. Verify `cli/src/completion/` ranks/displays from the shared plan and never teaches source-relative-only syntax while execution is repository-first
- [ ] Top-level CLI references with no source document use the launch context: **repository root first, then launch directory**. In-memory/stdin surfaces with neither return a typed missing-context error, never an unrelated ambient CWD (D2)
- [ ] L1 tests: external sequences and composition sources honor the explicit/implicit contract; existing sequence `~` references resolve through `FileReference` with **unchanged** user-visible behavior; no production Claudine resolver manually classifies prefixes or joins/expands reference strings

**Checkpoint 7:** `just test` + `just lint` green in `claudine`. All four private
grammars are gone (acceptance 1).

---

## Phase 8 — Integration, Cross-Platform, and Acceptance

- [ ] L2: build the motivating fixture — a router at `<repo>/prompts/` proxying the **bare** `prompts/_implement/implement-suggestions.md` — and prove it resolves to `<repo>/prompts/_implement/...`, not the doubled `<repo>/prompts/prompts/...` (acceptance 4; see G4 — live prompts were already patched to `./`)
- [ ] L2: paired fixture proving `./prompts/_implement/implement-suggestions.md` stays source-relative and **fails** when that exact source-local path is absent
- [ ] L2: schema `file(...)`, expression functions, sequence references, and transclusions resolve **shared** fixtures identically (acceptance 6)
- [ ] L2: nested transclusion/schema fixtures prove each authored document supplies its own base without losing the request's worktree context (acceptance 14)
- [ ] L2: completion-produced references execute without rewriting (acceptance 6/D9)
- [ ] L2: no-color/TTY errors show ordered attempted candidates through the typed pipeline (acceptance 8). Per G1, `err.detail.*` field coverage is error-propagation's to complete
- [ ] **[parallel]** Cross-platform evidence: Linux via Docker, Windows via the `x86_64-pc-windows-gnu` target — both available on this macOS host. Cover POSIX absolute, Windows drive/UNC/drive-relative, backslash explicit-relative, and home-pinned `~` on all three (acceptance 9, 11). Do not defer as impossible
- [ ] **[parallel]** Decide G4's optional prompt revert: whether to restore bare spellings in `prompts/` now that repository-first works. Not a gate
- [ ] **[parallel]** Update Claudine lifecycle, composition, sequence, and system-prompt docs to the explicit/implicit terminology; document `~` as the shared home-pinned form and remove the private sequence expansion from docs
- [ ] **[parallel]** Record the behavior changes in package timelines/release notes: implicit flips source-first → repository-first; Darkmatter's launch-area fallback is removed for nested documents; sequence `~` moves into `FileReference` with unchanged meaning; `@` in harness proxy changes from repo-root to magic search (G2); I/O probe failures that read as `not found` become typed errors
- [ ] **[parallel]** Refresh affected skill `hash:` values via `md hash <file>`
- [ ] Full gate: `just test` + `just lint` in `biscuit-file`, `darkmatter`, `claudine`; relevant L2 suites in each area (acceptance 10)
- [ ] Walk all 15 acceptance criteria against the implementation and record evidence per criterion

**Checkpoint 8 (final):** All 15 acceptance criteria evidenced. Three package
areas green on `just test` + `just lint` plus L2. No regression against the
Phase 1 baseline.

---

## Risk Register

| Risk | Severity | Mitigation |
|---|---|---|
| Error-propagation is an unimplemented draft with no plan, yet owns D8's Claudine wrapper and `err.detail.*` | **High** | G1 seam split — `biscuit-file` owns the typed outcome (no EP dependency); only the Claudine wrapper defers. Phases 2–5, 7 unblocked |
| Precedence flip breaks an unaudited consumer | **High** | Phase 1's D12 inventory is a hard gate on Phase 4. 20 call-site files enumerated |
| `@` silently changes meaning in proxy targets | Medium → **Low** | Verified zero authored `@` proxy/sequence values. Re-verify in Phase 1 |
| `biscuit-file` → `sniff` dependency would create a crate cycle | Medium | Caller-supplied `repository_root`; `biscuit-file` keeps internal `gix`. Called out explicitly in Phase 2 |
| Darkmatter tests ratify the *old* contract and pass green while wrong | Medium | ~21 tests + ~8 comment blocks enumerated in Phase 5 as explicit tasks |
| Windows behavior unverifiable on a macOS host | Medium | Docker + `x86_64-pc-windows-gnu` are both available; target-gated cases and pure-parser fixtures per D7 |
| `file_ref_fallback_dir` removal breaks proxied-target compose | Medium | Phase 6 task reconciles the rematerialize anchor against D2's top-level-only rule |
| `resolve-tuple` spec collides on the resolve surface | Low | G3 — additive; expose D3's plan as reusable so it consumes rather than copies |
| Autocomplete fallback keys on `FileNotFound` and breaks silently | Low | Explicit Phase 7 task |

## Parallelization Summary

- **Phase 1:** 5 of 7 tasks parallel
- **Phases 2–4:** strictly sequential (each builds on the last); several tasks parallel *within* each phase
- **Phases 5, 6, 7:** fully parallel with each other once Checkpoint 4 passes — disjoint packages/files. This is the widest fan-out in the plan
- **Phase 8:** L2 suites parallel per package; docs/timeline tasks parallel with test work
