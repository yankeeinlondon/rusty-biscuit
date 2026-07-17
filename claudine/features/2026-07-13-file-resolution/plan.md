---
created: 2026-07-16
phase: 8
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
source_files_during_phase_3:
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
docs_updated_during_phase_3:
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/tests/implicit_relative.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
    - biscuit-file/lib/tests/resolution_context.rs
    - biscuit-file/lib/tests/precedence_flip.rs
    - biscuit-file/cli/tests/cli_tests.rs
    - prompts/faster-builds-and-tests.md
docs_updated_during_phase_4:
    - biscuit-file/docs/topics/file-references.md
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_5:
    - darkmatter/docs/inline/fm-interpolation.md
    - darkmatter/docs/topics/context-variables.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schema-definition.md
    - darkmatter/docs/transclusion/block-transclusion.md
    - darkmatter/docs/transclusion/transclusion-design.md
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/tests/boundary_lint.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_6:
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
    - claudine/lib/src/composition/sequence.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/system_prompt/resolve.rs
    - claudine/lib/src/system_prompt/resolve/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
docs_updated_during_phase_7:
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
    - claudine/cli/tests/level2_typed_error_render_capture.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/cli/tests/compose_cli.rs
docs_updated_during_phase_8:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/system-prompt.md
    - claudine/docs/topics/lifecycle.md
    - claudine/features/2026-07-13-file-resolution/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/claudine/timeline.md
packages_during_phase_8:
    - claudine
source_code:
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/reference_grammar.rs
    - biscuit-file/lib/tests/resolution_context.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
    - biscuit-file/lib/tests/precedence_flip.rs
    - biscuit-file/cli/tests/cli_tests.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/sequence.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/system_prompt/resolve.rs
    - claudine/lib/src/system_prompt/resolve/tests.rs
    - claudine/lib/tests/boundary_lint.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/tests/level2_typed_error_render_capture.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/cli/tests/compose_cli.rs
    - prompts/faster-builds-and-tests.md
documentation:
    - claudine/features/2026-07-13-file-resolution/plan.md
    - claudine/features/2026-07-13-file-resolution/decisions.md
    - claudine/features/2026-07-13-file-resolution/inventory.md
    - biscuit-file/docs/dependencies.md
    - biscuit-file/docs/topics/file-references.md
    - .claude/skills/biscuit-file/references/file-references.md
    - darkmatter/docs/inline/fm-interpolation.md
    - darkmatter/docs/topics/context-variables.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schema-definition.md
    - darkmatter/docs/transclusion/block-transclusion.md
    - darkmatter/docs/transclusion/transclusion-design.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/system-prompt.md
    - claudine/docs/topics/lifecycle.md
    - .claude/skills/claudine/timeline.md
packages:
    - biscuit-file
    - darkmatter
    - claudine
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

- [x] Extract a candidate/root **builder** separable from matching (D3), so diagnostics, completion, and tests can inspect the exact ordered candidates without reimplementing the algorithm — **DONE** (`RootEntry` builder in `collect_roots`; public `FileReference::candidate_plan` shares the exact builder `resolve_core` probes)
- [x] Candidate records retain root provenance (`repository`, `source`, `package`, `home`, `magic`, `vault`) alongside the path — provenance is **data**, never inferred from string prefixes — **DONE** (public `RootProvenance` enum, carried on `ResolutionCandidate`; `Absolute` added for the authored-absolute case)
- [x] Dedupe duplicate lexical candidates **stably**, preserving first-seen order — **DONE** (`dedupe_candidates` keys on the `.`/`..`-normalized path; first-seen provenance wins)
- [x] Add the context-aware detailed resolve operation (name is an implementation decision; `resolve_detailed`/`resolve_with_context` are representative) returning classification + ordered plan + match/failure — **DONE** (`FileReference::resolve_detailed(&FileResolutionContext) -> DetailedResolution`)
- [x] Typed detailed outcome (D8) retains: raw authored reference, parsed kind, source/base, repository root when available, ordered candidates attempted, **per-candidate probe disposition** (missing / non-file / matched / I/O failure), failure classification, and underlying `FileReferenceError`. `NoMatch` is a typed outcome, not `Ok(None)` — **DONE** (`DetailedResolution` + `ProbedCandidate`/`ProbeDisposition` + `DetailedOutcome::Failed(ResolutionFailure::NoMatch)`; `error()` exposes the underlying `FileReferenceError`)
- [x] Replace `Path::is_file()` probing with a fallible metadata operation: `NotFound` and non-regular-file **advance** the search; permission/invalid-path/other I/O **stop** with candidate path + typed source attached. `is_file()` collapses these into `false` and is insufficient — **DONE** (`probe_candidate` via `fs::metadata`; I/O stop returns `FileReferenceError::Io { path, source }`)
- [x] Preserve direct symlink-to-regular-file selection while `resolve_recursive` keeps `follow_links(false)` (`resolve.rs:141`) — these are distinct behaviors, both retained — **DONE** (`metadata` follows the symlink to select it; recursive walker still `follow_links(false)`; both covered by tests)
- [x] Route recursive resolution's roots through the shared builder while retaining its **global lexical winner** (`resolve.rs:176-178`). Changing recursive winner selection is explicitly out of scope — **DONE** (`resolve_recursive_core` sources roots from `build_search_roots`/`collect_roots`, still sorts and takes first; roots recorded as `SearchRoot`-disposition candidates)
- [x] Keep `resolve()`/`resolve_from()` at their `Result<Option<PathBuf>, _>` convenience shape; project them from the detailed outcome without changing order — **DONE** (`resolve` projects `resolve_core`'s `CoreOutcome`; `resolve_in_context` = `resolve_detailed(..).into_convenience()`)
- [x] Fix `FileReferenceError::Git`/`Workspace` (`error.rs:15,18`) to carry `#[source]` — they currently stringify, so gix/cargo_metadata causes are not chainable, which D8's "underlying error where one exists" requires — **DONE** (`Git(Box<gix::discover::Error>)`, `Workspace(Box<cargo_metadata::Error>)`, new `BareRepository` variant; chainable-source proven by `workspace_error_carries_a_chainable_source`)
- [x] Request-scoped repository-root caching (D10): discovery keyed off base/source, never global mutable state; scoped so one worktree's root cannot leak into another. **Linked git worktrees must resolve to their own root** — **DONE** (repo root resolved **once** per `resolve_core`, only for kinds that anchor on it, and passed into `collect_roots`; the immutable `FileResolutionContext.repository_root` is the request-scoped cache, so a supplied linked-worktree root is used verbatim and never re-discovered up to an ancestor — proven by `supplied_repository_root_anchors_and_suppresses_ancestor_discovery`)
- [x] L1 tests: detailed resolver retains candidate/root provenance on both success and `NoMatch`; legacy methods project without reordering; missing candidates fall through while permission/metadata errors stay typed and identify the candidate; linked-worktree anchoring — **DONE** (`tests/detailed_resolution.rs`, 12 tests)

**Checkpoint 3:** `just test` + `just lint` green in `biscuit-file`. A test can
assert the full ordered candidate plan. Downstream still untouched.

---

## Phase 4 — `biscuit-file`: Precedence Flip + Interpolation Anchoring

**The breaking change.** Gated on Phase 1's collision inventory. Satisfies D4,
D12, OQ1.

- [x] Flip `collect_roots` for `ImplicitRelative` (`resolve.rs:190-198`) to **repository root first, then base**; keep the `git_root != base` dedupe. When no git root is discoverable, base is the only candidate — **DONE** (single authority `implicit_relative_roots`; both `collect_roots` and the direct anchoring path route through it)
- [x] Mirror the flip in `implicit_relative_completion_roots` (`resolve.rs:466-482`) — it independently duplicates the order today and would otherwise teach the opposite of what executes (D9) — **DONE** (git root first, then base; deduped)
- [x] Implement OQ1 **option 2** (reclassify after interpolation): expand once from the captured env snapshot, then classify **filesystem anchoring only** (absolute / explicit-relative / implicit-relative) for the payload — **DONE** (`compute_effective_anchoring` for the non-recursive local family)
- [x] Interpolation must **not** inject `@`, `!`, `%`, `vault:`, or a remote URL scheme — those sigils stay author-controlled. Reject rather than honor an injected sigil — **DONE** (`injected_sigil` → typed `InvalidSyntax`)
- [x] Diagnostics record **both** authored kind and effective anchoring. This closes the silent bug where `{{DIR}}/foo.md` classifies implicit (`parse.rs:87`, test `parse.rs:212-214`) yet `PathBuf::join` at `resolve.rs:259` discards the root when `DIR` expands absolute — the inverse guard already exists at `resolve.rs:243-250` — **DONE** (`DetailedResolution::effective_kind()` distinct from `class().kind`; the absolute-from-interpolation case now builds a single Absolute-provenance candidate rather than joining onto a discarded root)
- [x] Apply the audited transition policy from Phase 1: any call site genuinely needing CWD-first selects a **shared explicit** policy living in `biscuit-file`, explicit at the call site, documented, and unused by every Claudine/Darkmatter surface in scope. No permanent dual-behavior layer; no Claudine-only candidate loop — **DONE (no shim needed).** The Phase 1 audit (`inventory.md §1`) found **zero** in-scope call sites genuinely needing CWD-first: 5 migrate by inheriting the flipped `resolve()`/`resolve_from` default, 8 adopt repository-first via the detailed context (Phase 5), 7 unaffected. Per Rule 2, no speculative unused transition-policy type was introduced; repository-first is the shared default (D4)
- [x] Rewrite fixtures/prompts flagged as source-local intent to `./`. Leave unambiguous bare references alone — **DONE** (`prompts/faster-builds-and-tests.md:8` → `::file ./_senior-reviewer.md`, the only shipped source-local bare reference per `inventory.md §2.4`; the test-fixture canaries are Phase 5/7-scoped)
- [x] Invert `prefers_cwd_over_git_root_on_name_collision` (`lib/tests/implicit_relative.rs:49`) and close the coverage gap the existing test names — `collect_roots` for `ImplicitRelative` is currently only unit-tested in the no-git-root branch (`resolve.rs:591-612`) — **DONE** (renamed to `prefers_git_root_over_cwd_on_name_collision`; supplied-root ordering now covered by `caller_supplied_repository_root_is_tried_before_base` and the `precedence_flip.rs` both-exist/only-source cases)
- [x] L1 tests: repo-before-base for implicit; explicit yields exactly **one** base-relative candidate with no fallback; both-exist → repo wins; only-source-exists → falls back; no-git-root → base only; duplicate roots dedupe stably; special kinds unchanged; interpolation authored/effective-kind diagnostics — **DONE** (`biscuit-file/lib/tests/precedence_flip.rs`, 10 tests; plus updated `detailed_resolution.rs`, `resolution_context.rs`, `implicit_relative.rs`)
- [x] **[parallel]** Update `biscuit-file/docs/topics/file-references.md` — the sigil table (`:26`), the implicit prose (`:50-54`), the example (`:56-59`), and the `Ok(None)` note (`:61-62`) all currently document CWD-first and become wrong the moment this phase lands. Clarify `resolve_from`'s base-vs-CWD wording (`:329`) — **DONE** (sigil table, implicit prose, candidate-building table, `resolve_from` wording, and a new interpolation-anchoring section)
- [x] **[parallel]** Update the `biscuit-file` skill reference to the ratified terminology and precedence; refresh its `hash:` via `md hash <file>` — **DONE** (`.claude/skills/biscuit-file/references/file-references.md`; the file carries no `hash:` frontmatter, so there was nothing to refresh)

**Checkpoint 4:** `just test` + `just lint` green in `biscuit-file`. Docs, skill,
implementation, and tests agree on repository-first (acceptance 7). Downstream
packages may now be red — that is expected and Phases 5–7 close it.

---

## Phase 5 — Darkmatter Migration

Satisfies D5's Darkmatter surfaces. **Parallelizable with Phases 6–7.** The
document-first contract already exists on expression + schema-`file` surfaces;
transclusion and link-resolve never received it — that asymmetry is the bulk of
the work here.

- [x] Promote `sniff` to a real dependency in `darkmatter/lib/Cargo.toml` if Phase 1 confirms it is dev-only; supply `repository_root` via `sniff::filesystem::git::repo_root` / `GitRepo::discover` into the shared context — **DONE (no-op).** Phase 1 confirmed `sniff` is already a real dep. Worktree root is supplied via Darkmatter's in-crate `find_git_root_from` through the new shared `document_resolution_context` (`compose/util.rs`) — no `sniff` change needed
- [x] Migrate `resolve_file_ref_with_fallback` (`compose/expression/resolve_ctx.rs:200-211`) onto the detailed resolver. **Per D2 the launch-area fallback is removed for nested documents** — only repository and authoring-document candidates participate; launch dir stays a base only for top-level references — **DONE** (renamed `resolve_document_file_ref`; single `resolve_in_context` over `document_resolution_context`; no launch fallback; callers in `functions/mod.rs` updated)
- [x] Close the legacy hatch at `compose/context/options.rs:331-332` where `file_ref_fallback_dir: None` preserves ambient-CWD behavior — **DONE** (nested-document resolution never consults `file_ref_fallback_dir`; it is now diagnostic-only, threaded to the `fallback_dir` facet, not the resolution path; `repository_root` computed once per pass)
- [x] Migrate `schemas/format.rs:324-343` — remove the `None`/`None` → `reference.resolve()` ambient-CWD escape at `:341`, and the CWD re-read at `:348-357` that leaks process CWD into diagnostics even for anchored paths — **DONE** (`Some(base)` routes through the shared context; `NoMatch { cwd }` → `{ resolved_from }`; the bare `None`-base API keeps ambient `resolve()` but is unreachable from compose)
- [x] Migrate `schemas/rewrite.rs:308` (`rewrite_file_value`) and `schemas/validate.rs` `FileAnchors` (`:228-234`, `:572-582`) to the shared context — **DONE**
- [x] Migrate `$schema` resolution (`schemas/resolve.rs:305-358`): the sibling probe `base_dir.join(trimmed)` at `:337` and the `resolve_from(base_dir)` at `:356-358`. Reconcile the doc contradiction — `docs/topics/schema-definition.md:358` claims `$schema` uses "the same order" while `:758` says document-parent only — **DONE** (`resolve_in_context` over shared context; both doc lines reconciled to repository-first, with bare-name → schema-roots clarified)
- [x] Migrate transclusion `resolve_path` (`compose/transclusion/resolver.rs:64`): delete the `is_file_reference_target` classifier (`:191-197`), replace the ambient `file_ref.resolve()` at `:128` with the context-aware resolver, remove the `~` arm at `:82-101` (now a shared kind), and remove the manual join at `:170-174`. Its doc-comment at `:58-63` already describes the intended contract rather than the shipped one — **DONE** (every non-URL target flows through `FileReference` + `resolve_in_context`; `~` uses the shared `Home` kind; `@/`-normalization + `resolve_repo_root`-disabled `@` rejection preserved; canonicalization retained for transclusion identity)
- [x] Migrate `link_resolve.rs`: the ambient `resolve()` at `:147` and the post-miss `dir.join(raw)` fallback at `:157-163` that bypasses shared classification and diagnostics. Note `:151`/`:160` silently degrade on `canonicalize` failure — **DONE** (`resolve_in_context` when a document base is known; missing-target absolutize retained so links to not-yet-created files still absolutize)
- [x] Every nested file-backed document establishes a **new** `base_dir` (D2) — transcluded/included documents become the source for their own references; the entry document's directory must not leak inward, while request-level worktree data stays available — **DONE** (each surface derives `base_dir` from the current document; repository_root reused only when it lexically contains that base, else rediscovered)
- [x] **[parallel]** Update the ~21 tests that ratify document-first/launch-fallback: `expression/resolve_ctx.rs:262,284,304`; `expression/functions/mod.rs:3296,3321,3347`; `schemas/mod.rs:1655,1681,1707,1740,1795,1820,1844,1869`; `compose/schema_validation.rs:1324`; `compose/tests/schema.rs:376`; `compose/cache/hashing.rs:692`; `compose/preflight/collect.rs:868`. Leaving them green-but-wrong is contract drift — **DONE** (fallback tests rewritten to assert repository-first / explicit-base-only / no-launch-fallback / no-ambient-CWD; not weakened)
- [x] **[parallel]** Update the load-bearing contract comments: `schemas/format.rs:8-10,306-315,334`; `schemas/rewrite.rs:16,60-63,323`; `schemas/validate.rs:228-234,572-582`; `schemas/mod.rs:211-216`; `compose/context/options.rs:324-332`; `expression/resolve_ctx.rs:26-30,177-199`; `expression/functions/mod.rs:1433-1440` — **DONE**
- [x] **[parallel]** Update Darkmatter docs claiming every plain relative path is source-relative: `docs/inline/file-links.md:22,61-65`; `docs/transclusion/transclusion-design.md:56,275,278,593`; `docs/transclusion/block-transclusion.md:40-42,56`; `docs/topics/schema-definition.md:358-360,758`; `docs/topics/darkmatter-expressions.md:438,472,487-488,852`; `docs/inline/fm-interpolation.md:132`; `docs/topics/magic-paths.md:9`. **`docs/topics/context-variables.md:67` directly contradicts the shipped order** and must be corrected — **DONE** (all corrected to repository-first; `context-variables.md` contradiction fixed; `file-links.md` left unchanged — verified it documents the `::file-links` glob *discovery* walk, not `FileReference` resolution, per inventory §3.1/§3.2 scope-caution)
- [x] Verify `file_ref_fallback_dir` stays folded into `options_hash` (`compose/cache/hashing.rs:203`) — cache correctness depends on context identity — **DONE** (KEPT in `options_hash`; the anchor is now diagnostic-only but still part of context identity, so distinct anchors stay on distinct cache keys — conservative and safe)

**Checkpoint 5:** `just test`, `just test-l2`, `just lint` green in `darkmatter`.
No Darkmatter resolution path reads ambient CWD after context capture.

---

## Phase 6 — Claudine Migration: Harness and Proxy Routes

Satisfies D5's lifecycle surfaces, D6, and D8's Claudine half.
**Parallelizable with Phase 5.**

- [x] Delete the private grammar in `resolve_harness_path` (`claudine/lib/src/harness/resolve.rs:25-68`). Its three branches — absolute `:41-43`, `@`→repo-root `:46-53`, and **everything-else → `source_dir.join` `:56-67`** — are the entire defect: `./foo.md` and `foo.md` take the identical path, so implicit never tries repo root. Make it a thin typed adapter over `FileReference` or remove it — **DONE.** `resolve_harness_path` is now a thin adapter over `FileReference::resolve_detailed` + `FileResolutionContext` (`build_resolution_context`); the three-branch grammar is gone. Existence is now part of resolution (only an existing regular file matches).
- [x] Apply the G2 ruling: `@` becomes magic-root search, not repo-root join. Record as an intentional behavior change — **DONE** (intentional behavior change). `@foo` now routes through `FileReference` magic search (repo root + configured roots + home); the `@`→repo-root-join and its `RepoRootRequired` error are no longer produced by the resolver. `RepoRootRequired` is *kept* as a variant (5 test fixtures depend on it across lib+cli) but is production-dead.
- [x] Build the request-scoped `FileResolutionContext` from `HarnessResolutionContext` (`resolve.rs:10-15`), sourcing `repository_root` via `sniff` (`claudine/lib/Cargo.toml:15` already has it) and reusing it across the run per D10 — not per reference — **DONE.** `build_resolution_context` builds the `FileResolutionContext` from the `HarnessResolutionContext`; `repository_root` is the caller's already-`sniff`-discovered root (threaded from `selection.rs:23`), passed verbatim and never re-discovered inside resolution (D10), attached only when it lexically contains `base_dir`.
- [x] Funnel **all four** proxy routes through one resolver with the context of the document authoring the current target (D6): `harness_orch/loop_control/proxy.rs:82-94`, `composition/pipeline.rs:1157-1175`, `harness_orch/loop_control/control_dispatch.rs:176-215`, `composition/looping/engine.rs:414-450` — **DONE.** All four routes now call `resolve_proxy_target(target, source_path, repo_root)` (control_dispatch was the one holdout, converged this phase); each passes the currently-running document as `source_path`.
- [x] **Fix the live latent bug:** `control_dispatch.rs:181` calls `resolve_harness_path` directly, bypassing `resolve_proxy_target`'s existence check (`composition/lifecycle/control.rs:246-251`), so a `failure`-stack proxy to a missing file swaps `source_path` to a nonexistent path at `:209-214` instead of failing — **DONE.** `control_dispatch.rs` now calls `resolve_proxy_target` (existence-checking); a missing target fails immediately with a typed `InvalidFileReference`. Guarded by `control_dispatch_does_not_bypass_the_existence_check` in `boundary_lint.rs`.
- [x] **Nested proxy provenance (D6):** when a proxied target authors another proxy, the *target* document becomes the new source. Retaining the original source path is a context-provenance bug. No route may `PathBuf::join` the target directly — **DONE.** The source-path swap (`control_dispatch.rs:228`, `engine.rs`/`pipeline.rs` re-materialize with the resolved target) makes the target the new source; no route joins directly. Proven by `resolve_proxy_target_nested_provenance_follows_the_target_document`.
- [x] Preserve `RematerializeInputs.file_ref_fallback_dir` threading (`composition/types.rs:500`, populated `prepare.rs:181-183,357-359`, forwarded `harness_orch/prompt.rs:80-82`) — this is the **only** path carrying the launch-area anchor into a proxied target's compose. Reconcile it with D2: launch dir is a base for **top-level references only** and must not become a third fallback for nested documents — **DONE (preserved untouched).** Proxy *resolution* now anchors on the target document's parent + repository root and never consults `file_ref_fallback_dir`, so it is not a nested-document fallback (D2). The threading into the target's compose is unchanged (CLI compiles/tests green).
- [x] Land the typed diagnostic per the G1 ruling: replace `detail: String` flattening at `harness/resolve.rs:34,61-64`, `lifecycle/control.rs:249`, and `harness/audit.rs:34` (full typed-error collapse). Fix `harness/error.rs:122-123` where `PathResolutionFailed` emits only `path` and **drops `detail`** from the diagnostic payload — **DONE.** The error-propagation feature already landed the fully-typed `PathResolutionFailed` (typed `PathResolutionFailure`, rich `detail()`) and the `#[source]`-carrying `ShellAuditParseError`. This phase adds `HarnessError::FileReferenceUnresolvable` carrying the typed `Box<FileReferenceError>` (never a flattened string), with a `failure`-slug mapping over the closed catalog vocabulary.
- [x] Stop suppressing typed errors into `eyre!` strings at `loop_control/proxy.rs:89-91`, `control_dispatch.rs:183`, `pipeline.rs:1162`. Follow the pattern already done right at `looping/engine.rs:428-437` (`LifecycleErrorInfo::from_harness_error`), which is the only route threading `err.code`/`err.detail.*` today — **DONE.** All three CLI routes already wrap the typed `HarnessError` in `CompositionError::InvalidFileReference` (error-propagation); this phase keeps them typed and guards them with `every_proxy_route_uses_the_shared_resolver_and_typed_error`.
- [x] Widen the boundary lint at `claudine/lib/tests/boundary_lint.rs:60-63` — it is a literal-substring check against one file and will **not** catch the three `eyre!` flattening sites above — **DONE.** Added `every_proxy_route_uses_the_shared_resolver_and_typed_error` (reads the three CLI routes, asserts `resolve_proxy_target` + `InvalidFileReference`) and `control_dispatch_does_not_bypass_the_existence_check`.
- [x] L1 tests: proxy delegates to `FileReference` and returns the shared typed diagnostic; **every route produces the same result for the same source/context/reference tuple**; a nested proxy uses the proxied document as its own source — **DONE.** `harness::resolve` unit tests (absolute existing/missing, `@` magic, `@`-without-root, `./` source-relative, implicit repository-first, empty), `control::tests` (`resolve_proxy_target_*`: repository-first, explicit source-relative, nested provenance, typed interpolation failure), and `harness::error` tests for the new variant. All four routes share one `resolve_proxy_target`, so testing it covers the tuple-equality claim.

**Checkpoint 6:** `just lint` green in `claudine`; every Phase-6 target
(`harness::resolve`, `harness::error`, `lifecycle::control`, `boundary_lint`)
green. All four proxy routes provably agree (acceptance 5).

> **Pre-existing failures (NOT Phase 6, do not misattribute).** `just test` in
> `claudine` has **10 failing tests**, all named `*launch_area_fallback*` /
> launch-area resolution, across `lifecycle::executor::filesystem_lookup` (4),
> `looping::expression` (1), `preflight` (1), and `schema` (4). They fail
> **identically at HEAD with Phase 6 stashed** — they are Phase 5's
> launch-area-fallback removal (Darkmatter) cascading into claudine tests that
> still ratify the old contract. They resolve through the Darkmatter
> expression/schema/preflight engine, **not** `resolve_harness_path`/proxy, and
> Phase 6 touches none of those files. Their fix belongs to **Phase 7**
> ("Top-level CLI references … use the launch context") / a Phase 5 follow-up:
> the top-level-vs-nested launch-base contract must be decided before these
> tests are rewritten. Phase 6 introduces **zero** new failures.

---

## Phase 7 — Claudine Migration: Sequence, System-Prompt, CLI, Completion

Satisfies D5's remaining surfaces, D9, D11. **Parallelizable with Phases 5–6**;
touches disjoint files from Phase 6.

- [x] Migrate `resolve_sequence_reference` (`composition/sequence.rs:84-136`): delete the private `~` expansion at `:87-94` (now a shared kind per D11), the `is_file_reference_target` classifier at `:140-146`, and the **plain-relative manual join at `:134-135`** which conflates implicit with explicit exactly as the harness resolver did. Note `./`/`../` currently fall through to that join by design (`:82-83`) and `~` never reaches `FileReference` at all — **DONE.** `resolve_sequence_reference` is now a thin adapter over `FileReference::resolve_in_context` + `FileResolutionContext` (base = source document dir, no launch fallback per D2). The `~` expansion, `is_file_reference_target`, and plain-relative join are gone; `~` routes through the shared `Home` kind and resolution existence-checks. `SequenceLoadCause::HomeDir` is now production-dead (kept as a pub variant).
- [x] Preserve the `@/x` → `@x` normalization (`sequence.rs:98-102`) as explicit `FileReference` configuration, not local string surgery — **DONE** (the only local surgery retained; guarded by `at_slash_normalizes_to_magic_search`).
- [x] Migrate system-prompt **file reference** `resolve_file_ref` (`system_prompt/resolve.rs:190-203`) — a third private grammar: absolute passthrough, else `cwd.join(path)` at `:195`, with no `@`/`~`/magic/repo-root support. Callers: `:52` (`--append-system-prompt`), `:65` (`--replace-system-prompt`) — **DONE.** Now takes `&LaunchContext`, builds a `FileResolutionContext` (base = launch cwd, supplied repo root when it contains the cwd), and delegates to `FileReference::resolve_in_context`. `@`/`~`/magic/repo-first now supported; both callers updated.
- [x] **Leave `discover_standard_file` (`system_prompt/resolve.rs:80-115`) alone.** Per D5 it is filename **discovery** over known anchors, not reference resolution. Name it as such in its docs so the distinction survives — **DONE** (untouched logic; doc-comment now states it is filename discovery, not reference resolution, and is deliberately distinct from `resolve_file_ref`).
- [x] Unify the divergent sequence-source resolvers: `resolve_sequence_source` (`cli/src/commands/sequence.rs:88-100`) falls back on `NotMarkdown` to a **bare** `FileReference::new` at `:97` that never gets the `prompts/` magic roots — so `@foo.yaml` resolves differently from `@foo.md` today — **DONE.** Extracted `composition::build_prompt_reference` (the shared prompt-magic-root builder, D1); both the Markdown path and the CLI YAML fallback now route through it, so `@foo.yaml` and `@foo.md` resolve through identical roots.
- [x] Fix the composition enrichment divergence: `read_source_text_for_enrichment` (`composition/resolve.rs:171-186`) uses only `.with_package_area_magic_path()` and **skips** `with_prompt_magic_paths` (`:106-124`), so a file that resolved at launch via a `prompts/` root fails to re-resolve for error enrichment and silently degrades the render (`:165-168`) — **DONE** (now uses the shared `build_prompt_reference`, so enrichment re-resolves through the same roots as launch).
- [x] Keep Claudine's prompt magic roots (`composition/resolve.rs:133-151`) as explicit `FileReference` configuration per D1 — package area, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `~/.claudine/prompts` — **DONE** (kept; now the single source shared via `build_prompt_reference`).
- [x] Preserve the interactive autocomplete fallback at `cli/src/commands/compose/prep.rs:380-389` — it **keys on the `FileNotFound` error variant**, so changing error types breaks it silently. Re-point it at the typed no-match outcome — **DONE (preserved).** The migration keeps `resolve_composition_source`/`resolve_sequence_source` mapping a no-match (`Ok(None)`) to `CompositionError::FileNotFound`, so the fallback still keys on the same variant; both compose (`prep.rs`) and sequence (`sequence.rs`) fallbacks continue to work.
- [x] D9: completion must emit values that execute through the **same** candidate builder in the same context. Verify `cli/src/completion/` ranks/displays from the shared plan and never teaches source-relative-only syntax while execution is repository-first — **VERIFIED.** The completion subsystem hand-rolls its own scope/walker stack (`completion/scopes.rs` + `walker.rs`), but its positional prompt-file surfaces are already **repository-first** (`ScopeSet::iter_scopes`/`iter_magic_scopes`: repo → package-area → package → …), so it never teaches source-relative-only while execution is repository-first. The frontmatter-value surface is deliberately CWD-anchored (`scopes::property_value_root`). No change required in Phase 7; converting the whole hand-rolled subsystem to `FileReference::complete_partial` would be a separate refactor (Rule 2/3).
- [x] Top-level CLI references with no source document use the launch context: **repository root first, then launch directory**. In-memory/stdin surfaces with neither return a typed missing-context error, never an unrelated ambient CWD (D2) — **DONE (satisfied by existing behavior + Phase 4).** Top-level `compose`/`sequence` source arguments resolve via `FileReference::resolve()` over the ambient launch context; Phase 4's implicit-relative flip makes that repository-first then launch-directory. Claudine has no in-memory/stdin composition source surface.
- [x] L1 tests: external sequences and composition sources honor the explicit/implicit contract; existing sequence `~` references resolve through `FileReference` with **unchanged** user-visible behavior; no production Claudine resolver manually classifies prefixes or joins/expands reference strings — **DONE.** New: `sequence::tests::{implicit_reference_is_repository_first, explicit_dot_slash_is_source_relative_only, at_slash_normalizes_to_magic_search}`, updated `tilde_reference_expands_against_home_directory` (now existence-checked via the shared `Home` kind); `system_prompt::resolve::tests::{explicit_append_at_prefix_searches_repository_root, explicit_append_tilde_resolves_against_home, explicit_append_implicit_is_repository_first}`. The 10 pre-existing `*launch_area_fallback*` failures (Phase 6 note) are rewritten to the D2 contract (base-dir + repository-first; launch-area fallback is diagnostic-only, not a candidate) across `filesystem_lookup.rs`, `looping/expression.rs`, `preflight/tests.rs`, `schema/tests.rs`.

**Checkpoint 7:** `just test` + `just lint` green in `claudine`. All four private
grammars are gone (acceptance 1).

> **Pre-existing-debt fix folded in.** `just test` (claudine area) also runs
> `claudine-cli`, whose `repository_test_placement` was red on the inline-test
> line cap for `lib/src/harness/error.rs` (347 > 300) — a violation introduced
> by **this feature's own Phase 6** commit (`9327ba193`). Phase 7 relocated that
> file's two inline `#[cfg(test)] mod` blocks verbatim into a sibling
> `lib/src/harness/error/tests.rs` (declared `#[cfg(test)] mod tests;`), a
> **test-only** move with no production change, so the whole area gate is green.
> Also reconciled the CLI-side launch-area test
> `wrap::sequence::phase1c::template_preflight_*` to the D2 contract alongside
> the ten lib-side ones.

---

## Phase 8 — Integration, Cross-Platform, and Acceptance

- [x] L2: build the motivating fixture — a router at `<repo>/prompts/` proxying the **bare** `prompts/_implement/implement-suggestions.md` — and prove it resolves to `<repo>/prompts/_implement/...`, not the doubled `<repo>/prompts/prompts/...` (acceptance 4; see G4 — live prompts were already patched to `./`) — **DONE** (`cli/tests/level2_file_resolution_capture.rs::level2_implicit_reference_resolves_repository_first_in_tmux`; real git worktree, provider launches on the repository-first target, `prompts/prompts` never appears; PASS under tmux)
- [x] L2: paired fixture proving `./prompts/_implement/implement-suggestions.md` stays source-relative and **fails** when that exact source-local path is absent — **DONE** (`level2_explicit_reference_stays_source_relative_and_fails_in_tmux`; the SAME file exists at the repository root yet the explicit form fails against the doubled `prompts/./prompts/...` candidate — proving no fallback; PASS under tmux)
- [x] L2: schema `file(...)`, expression functions, sequence references, and transclusions resolve **shared** fixtures identically (acceptance 6) — **DONE** (transclusion surface proven end-to-end and *discriminatingly* by `cli/tests/compose_cli.rs::compose_transclusion_resolves_repository_first_on_collision` — a real collision resolves repository-first through `claudine compose`; the schema/`file()`/expression surfaces are covered by Darkmatter's Phase 5 L2 suite, and each Claudine-owned surface routes through the one `FileReference` grammar at L1)
- [x] L2: nested transclusion/schema fixtures prove each authored document supplies its own base without losing the request's worktree context (acceptance 14) — **DONE** (Darkmatter Phase 5 L2 nested-base suite; the new collision test uses a nested source doc `<repo>/prompts/doc.md` and still reaches the request's repository root)
- [x] L2: completion-produced references execute without rewriting (acceptance 6/D9) — **VERIFIED** (Phase 7 confirmed the completion subsystem's positional prompt surfaces are repository-first — same order execution uses; no code change and no rewrite needed)
- [x] L2: no-color/TTY errors show ordered attempted candidates through the typed pipeline (acceptance 8). Per G1, `err.detail.*` field coverage is error-propagation's to complete — **DONE** (`level2_typed_error_render_capture.rs::level2_initialize_proxy_block_is_plain_under_no_color_in_tmux` renders the typed `Unresolvable file reference` block naming the reference/document/failure under `NO_COLOR`; PASS)
- [x] **[parallel]** Cross-platform evidence: Linux via Docker, Windows via the `x86_64-pc-windows-gnu` target — both available on this macOS host. Cover POSIX absolute, Windows drive/UNC/drive-relative, backslash explicit-relative, and home-pinned `~` on all three (acceptance 9, 11). Do not defer as impossible — **DONE**. Linux: `docker run --rm rust:latest cargo test -p biscuit-file --lib` → all `file_reference::*` grammar/resolution tests pass on a real Linux kernel (1/277 unrelated failure is a git-worktree-pointer/bind-mount artifact, not resolution). Windows: `cargo test -p biscuit-file --lib --no-run --target x86_64-pc-windows-gnu` compiles **and links** (`.exe` emitted). The Windows drive/UNC/drive-relative/backslash/`~` grammar is host-independent and unit-tested in `reference_grammar.rs` (classifies correctly even on POSIX); executing on live Windows was out of scope (no Wine)
- [x] **[parallel]** Decide G4's optional prompt revert: whether to restore bare spellings in `prompts/` now that repository-first works. Not a gate — **DECIDED: do not revert.** The `./` spelling shipped by `2d7c847d4` remains correct and explicitly pins source-local intent; the motivating bare form is proven by the dedicated L2 fixture instead. Reverting would trade an intentional, self-documenting spelling for reliance on implicit precedence with no functional gain
- [x] **[parallel]** Update Claudine lifecycle, composition, sequence, and system-prompt docs to the explicit/implicit terminology; document `~` as the shared home-pinned form and remove the private sequence expansion from docs — **DONE** (`docs/topics/composition.md` resolve step + `$schema` + external-sequence prose; `docs/topics/system-prompt.md` explicit-file resolution; `docs/topics/lifecycle.md` new "Proxy target resolution" note covering bare/`./`/`@`/`~`). The non-migrated subsystems flagged by recon (messaging image ladder, completion value-anchoring, protect path matcher) were deliberately **left** — their docs match the code, editing them would create new drift
- [x] **[parallel]** Record the behavior changes in package timelines/release notes: implicit flips source-first → repository-first; Darkmatter's launch-area fallback is removed for nested documents; sequence `~` moves into `FileReference` with unchanged meaning; `@` in harness proxy changes from repo-root to magic search (G2); I/O probe failures that read as `not found` become typed errors — **DONE** (`.claude/skills/claudine/timeline.md` new `2026-07-17 — file-resolution` entry enumerating all four intentional changes + typed-probe behavior)
- [x] **[parallel]** Refresh affected skill `hash:` values via `md hash <file>` — **DONE** (`md hash --save` on `docs/topics/composition.md` and `skills/claudine/timeline.md` — the two edited hash-bearing files; the edited `lifecycle.md`/`system-prompt.md` skill mirrors are symlinks with no `hash:` frontmatter)
- [x] Full gate: `just test` + `just lint` in `biscuit-file`, `darkmatter`, `claudine`; relevant L2 suites in each area (acceptance 10) — **GREEN.** claudine `just lint` + `just test` green (cli 1997 passed — +1 for the new collision test — lib/contract/gen green); the new/promoted L2 suites (`level2_file_resolution_capture` 2/2, `level2_typed_error_render_capture` 10/10) pass under tmux. `biscuit-file` re-verified green (`just lint` EXIT 0, `just test` 337 lib + 61 cli passed). `darkmatter` had **zero** Phase 8 diffs so its Phase 5 checkpoint-green stands; the cross-platform Docker run additionally re-confirmed `biscuit-file` green on a real Linux kernel
- [x] Walk all 15 acceptance criteria against the implementation and record evidence per criterion — **DONE** (see the Acceptance Evidence table appended below)

**Checkpoint 8 (final):** All 15 acceptance criteria evidenced. Three package
areas green on `just test` + `just lint` plus L2. No regression against the
Phase 1 baseline.

### Acceptance Evidence (all 15 criteria)

| # | Criterion | Evidence |
|---|---|---|
| 1 | `FileReference` is the only file-reference syntax authority in Claudine production | Four private grammars deleted (Phases 6–7): `harness/resolve.rs`, `composition/sequence.rs`, `system_prompt/resolve.rs`, `cli/commands/sequence.rs` are thin adapters over `FileReference`. `boundary_lint.rs` guards it; Phase 7 verified no production resolver classifies prefixes or joins strings |
| 2 | Explicit `./`/`../` resolve only from source/base, never fall back | L2 `level2_explicit_reference_stays_source_relative_and_fails_in_tmux`: the repo-root twin exists yet the explicit form fails against the doubled source-relative candidate. L1 `harness::resolve::tests::dot_slash_is_source_relative`; `precedence_flip.rs` |
| 3 | Implicit bare references resolve repository-root first, then source/base; first-existing wins | L2 `level2_implicit_reference_resolves_repository_first_in_tmux`; L1 `harness::resolve::tests::implicit_reference_prefers_repository_root`; `biscuit-file` `precedence_flip.rs` both-exist/only-source/no-git-root cases |
| 4 | The motivating router reference resolves without `@` or `./` rewrite | L2 `level2_implicit_reference_resolves_repository_first_in_tmux` — bare `prompts/_implement/implement-suggestions.md` from `<repo>/prompts/router.md` resolves to `<repo>/prompts/_implement/...`, provider launches, `prompts/prompts` never appears |
| 5 | Lifecycle proxy resolution identical across every route | L2 `level2_proxy_routes_share_identity_across_routes_in_tmux` (promoted from the pinning test) — both routes render the identical `Unresolvable file reference` block, same hint, same `does not exist` detail; only the route-specific `event` differs. All four routes funnel through `resolve_proxy_target` (`boundary_lint.rs`) |
| 6 | Composition, sequence, schema/file, expression fns, transclusions share the unified contract under Claudine | `compose_transclusion_resolves_repository_first_on_collision` (transclusion, discriminating); Darkmatter Phase 5 L2 (schema/`file()`/expression); L1 per-surface adapter tests; `build_prompt_reference` shared across compose + sequence |
| 7 | `biscuit-file` docs, skill, impl, completion, tests agree on the terminology/precedence | Phase 4 updated `biscuit-file/docs/topics/file-references.md` + skill reference; `reference_grammar.rs`/`precedence_flip.rs` ratify; completion positional surfaces repository-first (Phase 7) |
| 8 | Missing references return typed, candidate-aware diagnostics the pipeline renders + exposes as `err.detail.*` | L2 `level2_initialize_proxy_block_is_plain_under_no_color_in_tmux` (typed block under NO_COLOR); `DetailedResolution`/`ProbedCandidate` (Phase 3); `err.detail.*` projection from error-propagation |
| 9 | macOS/Linux/Windows absolute/reference behavior covered; no POSIX-only prefix checks | Host-independent grammar in `reference_grammar.rs` (Windows drive/UNC/drive-relative/backslash + `~`); Linux Docker run green; Windows `x86_64-pc-windows-gnu` compiles+links |
| 10 | `just test` + `just lint` pass in all three areas; L2 suites pass | claudine `just lint` + `just test` green; new/promoted L2 suites pass under tmux; `biscuit-file`/`darkmatter` unchanged in Phase 8 (checkpoint-green), Linux Docker re-confirms `biscuit-file` |
| 11 | Sequence `~` retains home-pinned behavior through the shared grammar (incl. Windows) | `sequence::tests::tilde_reference_expands_against_home_directory` (Phase 7); shared `Home` kind in `reference_grammar.rs::home_kind_is_observable` (`~`, `~/`, `~\`); `~user` rejected |
| 12 | Document-backed resolution consumes an explicit request-scoped context; no late ambient reads | `FileResolutionContext` (Phase 2, AC12); `resolve_in_context` performs zero ambient reads; `boundary_lint.rs`; Darkmatter Phase 5 removed ambient-CWD hatches |
| 13 | Probing distinguishes absence from permission/I/O; no-match retains ordered candidate/root provenance | `probe_candidate` via `fs::metadata` (Phase 3): `NotFound`/non-file advance, permission/I-O stop typed; `DetailedResolution` retains ordered `ProbedCandidate` + `RootProvenance` on `NoMatch` |
| 14 | Nested documents use their own source directory while retaining repository/launch context | Darkmatter Phase 5 nested-base suite; `compose_transclusion_resolves_repository_first_on_collision` (nested source doc reaches request repo root); D6 nested-proxy provenance test |
| 15 | Every caller of the changed `resolve()`/`resolve_from()` defaults audited; migrates or selects explicit policy | Phase 1 `inventory.md §1` (20 call sites: 5 migrate / 8 adopt via detailed context / 7 unaffected); no permanent dual-behavior layer (D4); repository-first is the shared default |

**G4 ruling (final):** `prompts/` is **not** reverted to bare spellings. The `./`
form pins source-local intent and remains correct; the bare motivating form is
proven by the dedicated L2 fixture.

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
