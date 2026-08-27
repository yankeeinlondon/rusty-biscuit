---
total_phases: 8
created: 2026-08-27
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
---

# Execution Plan: Finalized file-reference grammar — one sigil catalog, one CWD model

Reference: [`spec.md`](spec.md) · Delta chronicle: [`sigil-delta.md`](sigil-delta.md) ·
Design-intent (normative): `claudine/docs/topics/file-referencing.md`

## Goal

Land the finalized file-reference grammar across `biscuit-file`, `darkmatter`,
and `claudine`: add `&` and `^`, remove `!`, flip implicit-relative ordering to
composition-CWD first, re-anchor multi-homed sigil bases to the reference's own
scope through a single `RepositoryScopeCatalog` projection, materialize
caller-passed file parameters as anchored values, add `ctx.cwd`, and set
`AGENT_CWD` in every spawned child's environment — with repo containment
enforced for `&`/`^` on both lexical and resolved targets.

Dependency direction is fixed: `biscuit-file` ← `darkmatter` ← `claudine`.
Phases follow that order. No backward-compatibility shims; call sites and
fixtures are updated directly.

## Dependency and parallelism map

```
Phase 1 (baseline + audit)
  └─> Phase 2 (biscuit-file grammar) ─> Phase 3 (biscuit-file catalog/resolve)
                                           └─> Phase 4 (darkmatter projection + resolver retirement)
                                                  └─> Phase 5 (darkmatter ctx.cwd + materialization)
                                                          └─> Phase 6 (claudine source-scope + ctx.cwd)
Phase 7 (claudine AGENT_CWD + spawn-seam guard)  -- independent of Phases 2-6;
                                                      may start any time after Phase 1
Phase 8 (docs, audit proofs, L2, cross-platform) -- after Phases 2-7
```

- **Phase 7 is fully parallelizable** with Phases 2–6: it touches only the
  Claudine child-environment seam and depends on nothing in the grammar work.
- Within Phase 2, parse-layer work and the error/provenance vocabulary are one
  commit-sized unit; within Phase 4, the projection and the resolver-site
  replacements are separable but sequenced (sites consume the projection).
- Phases 2–3 are biscuit-file-only; Phase 4–5 are darkmatter-only. One engineer
  can hold Phase 7 while another holds Phases 2–6 without file overlap beyond
  `claudine/lib` in Phase 6 vs 7 (different modules).

## Ground rules (from spec + AGENTS.md)

- Explicit resolution stays snapshot-only: no ambient CWD, HOME, Git, Cargo
  metadata, or topology discovery after context capture.
- One owner per responsibility per the spec's "Ownership and reuse boundaries"
  table; a lower layer never grows a dependency to satisfy a higher one.
- US English; comments follow the repo's comment-quality rules (no HOW-narration,
  fix drifted comments in the same change as behavior changes).
- Tests run via `just test` / `just test-l2` / `just lint` per package area
  (nextest underneath). L2/L3 must not steal terminal focus.
- Never `cargo fmt` unless told; never commit unless told.

---

## Phase 1 — Baseline, dependency gate, and consumer audit

The spec's `depends-on` (fixes/2026-08-12-ctx-launch-anchor) requires that
fix's contract and acceptance criteria complete before implementation starts,
and the baseline must be verified green rather than assumed.

- [ ] Confirm `claudine/fixes/2026-08-12-ctx-launch-anchor` is complete: its spec ACs are satisfied and it has been moved to `fixes/_completed/` (or record the owner's explicit go-ahead in this feature's notes); note from its `log.md` that the Linux/WSL/native-Windows gates were partially deferred — decide with the owner whether those gates block this feature's Phase 8 or run concurrently.
- [ ] Record the baseline commit (`git rev-parse HEAD`) in this feature directory and verify green: `just test` and `just lint` in `biscuit-file/`, `darkmatter/`, and `claudine/` (including `just test-cli ''` in claudine if the area splits it).
- [ ] Run GitNexus impact analysis (`impact` upstream) for the symbols the spec names: `FileReferenceKind`, `RootProvenance`, `FileReferenceError`, `candidate_plan`, `resolve_in_context`, `complete_partial`, `document_resolution_context`, `find_package_area_from`, `find_git_root_from`, `derive_source`, `prompt_magic_roots`, `build_child_env`. Save the blast-radius summary to `claudine/features/2026-08-26-finalized-references/consumer-audit.md`.
- [ ] Complete the compiler-exhaustiveness consumer inventory in `consumer-audit.md`: every match/constructor of `FileReferenceKind`, internal `ReferenceKind`, `RootProvenance`, `FileReferenceError`, candidate-plan, completion-root, and `resolve_in_context` consumers across the workspace. The spec's known blast radius includes: darkmatter schema resolution/rewrite/detect/format, transclusion, expression helpers (`path_projection`, `functions/mod.rs` git-root fns), preflight, TOC linking (`link_resolve`, `link_normalization`), claudine sequence source resolution, harness diagnostics/resolution, system prompts, and CLI composition completion. Verify none are missed.
- [ ] Locate and list in `consumer-audit.md` the existing conflict fixtures that lock repository-first implicit ordering (cited by ctx-launch-anchor AC10 — darkmatter `resolve_ctx.rs`, `schema_validation.rs`, `options.rs` area, and any claudine fixtures) so Phase 3/6 flip them deliberately.

**Checkpoint:** `consumer-audit.md` exists with the symbol inventory and fixture list; all three areas green at the recorded baseline commit. Exit criteria: dependency complete or explicitly waived; audit reviewed against the spec's Scope section.

---

## Phase 2 — biscuit-file: grammar layer (parse, errors, provenance vocabulary)

Implements D1, D2, D9, D10 (grammar parts) in
`biscuit-file/lib/src/file_reference/{parse.rs, error.rs, mod.rs}`. Purely
syntactic; no resolution changes yet, so downstream compile breakage is
expected and deliberately deferred to Phases 4–6.

- [ ] Add `FileReferenceKind::RepositoryRoot` (`&`) and `FileReferenceKind::RepositoryScoped` (`^`) plus the internal `ReferenceKind` variants and `DetectedKind` arms in `parse.rs::detect_kind`, inserted at the documented D9 position (after `@`, before `~`).
- [ ] Implement defensive-sigil handling for `@`, `&`, `^` exactly once (generalize the existing `@` logic): consume at most one following `/`; reject a payload that remains rooted (`is_rooted_magic_payload`-style check) with `InvalidSyntax`; reject `&\x`-style backslash separators on every host (`/` is the only portable sigil separator); reject empty payloads as `InvalidSyntax`.
- [ ] Remove the `!`/Package kind: delete `DetectedKind::Package`, `ReferenceKind::Package`, `FileReferenceKind::Package`, and their match arms; a leading `!` now returns a dedicated removed-sigil error variant (new `FileReferenceError::RemovedSigil` or equivalent) whose message names `!` and suggests `^` — it must not fall through to implicit-relative.
- [ ] Implement D9 reserved-introducer/scheme rejection in `detect_kind` with the documented host-independent order: HTTP(S) → `vault::`/`vault:` → `@`/`&`/`^`/`!`/`~` → drive-absolute classification → generic RFC-scheme guard (`[A-Za-z][A-Za-z0-9+.-]*:` that is not a supported scheme → typed `UnsupportedScheme` error, `file:` included) → explicit-relative → implicit. Drive-relative `C:path` is a typed error, never implicit; `C:/x` and `C:\x` stay absolute.
- [ ] Make a second leading `%` invalid (`%%x` → `InvalidSyntax`) rather than a recursive filename — enforce in `strip_recursive`/`parse`.
- [ ] Update `resolve.rs::injected_sigil` (grammar set): interpolation must not inject `&`, `^`, or the removed `!` either; keep the existing rejection message shape.
- [ ] Replace `RootProvenance::Package` (which actually denoted a package area) with distinct `PackageRoot` and `PackageArea` variants across `mod.rs`, `resolve.rs`, and diagnostics.
- [ ] Add typed error variants in `error.rs`: outside-repository (names the sigil and the reference's CWD) and repository-escape (identifies sigil, authored reference, repository root, and escaped candidate — must not leak an unrelated ambient path), plus `UnsupportedScheme` and the removed-sigil variant; delete `MissingPackageContext` and the `Workspace`/`cargo_metadata` variant; re-map `classify_error` accordingly (outside-repo and escape classify as `MissingContext`-vs-`InvalidReference` per D4/D5 semantics — escape is a typed failure that stops resolution).
- [ ] Delete `FileReference::with_package_area_magic_path` from `mod.rs` (it exists only to feed `find_package_area`, which Phase 3 deletes); grep the workspace for callers and update them (expected: none in darkmatter/claudine hot paths — confirm via `consumer-audit.md`).
- [ ] Update the module-level docs in `mod.rs` (the `!README.md` example in the `//!` header must go; show `&`/`^` instead).
- [ ] L1 parse tests (host-independent, run everywhere): empty/rooted/backslash payloads after `@`/`&`/`^`; `&/x` ≡ `&x` equivalence; `~user`; `%%`; removed `!` diagnostic text; drive-relative `C:path` rejection; `file:`, `file:///`, `\\?\`, `\\.\`, and misspelled schemes as typed errors; `C:/abs` preserved; reserved-punctuation filenames reachable via `./` (e.g. `./!weird-name.md`, `./name:part`); interpolation-sigil-injection rejections for `&`/`^`.

**Checkpoint:** `just test` and `just lint` pass in `biscuit-file/`. Downstream crates are expected to fail compilation — capture the full compiler error list into `consumer-audit.md` as the Phase 4–6 work list. Verify no parse test reads the filesystem.

---

## Phase 3 — biscuit-file: scope catalog, resolution, containment, completion

Implements D3–D7 core + AC1, AC2 (engine level), AC8, AC11 in
`biscuit-file/lib/src/file_reference/{context.rs, resolve.rs, mod.rs}`.

- [ ] Add the pure-data `RepositoryScopeCatalog` type to `context.rs` (exported from `mod.rs`): repository root, monorepo package-area fallback policy, package-area roots, package roots. Validated constructor accepts only absolute, lexically normalized roots; rejects a package/package-area root outside its repository; dedupes roots without filesystem I/O; no `sniff` dependency. Component-aware, most-specific-first scope selection (`scope_for(base) -> {package_root?, package_area_root?, repository_root?}`).
- [ ] Extend `FileResolutionContext`: add the distinct package-root anchor (alongside the existing `package_area`), a `with_repository_scope_catalog` builder carrying the caller-supplied catalog, and recomputation in `for_source`/`for_base`/`for_trusted_external_*` — source-specific anchors are re-derived from the catalog by component-aware containment, never blindly copied from the previous document. A trusted-external derivation not covered by the catalog clears repository/package/package-area anchors. `validate()` semantics updated so normal and trusted-external derivations cannot retain stale repository scopes.
- [ ] Delete `find_package_area` from `context.rs`, remove `cargo_metadata` from the `file-reference` feature in `biscuit-file/lib/Cargo.toml` (and lockfile), and drop the re-export from `mod.rs`. `find_git_root` and `ResolutionContext::from_ambient` stay (ambient convenience + `bf` CLI only).
- [ ] Flip `implicit_relative_roots` in `resolve.rs` to composition-CWD first, then repo root; collapse when equal; keep it the single authority for direct + recursive + anchoring paths. Update the doc comment that currently cites the repository-first "Phase 4 precedence", and update the in-file tests (`caller_supplied_repository_root_is_tried_before_base` becomes CWD-first).
- [ ] Implement `&` resolution in `collect_roots`/candidate building: single candidate `{repo-root}/payload`, no package/area consultation, no fallback; typed outside-repository error when the reference's CWD is not inside a repository; lexical containment on the normalized payload (`&../outside.md` → typed escape error).
- [ ] Implement `^` resolution: candidate order package root → package-area root → repo root (missing levels skipped, duplicates collapsed), selected from the reference's own base via the catalog; never consults home; same typed outside-repository error as `&`; every candidate passes containment; an escape is a typed error that stops resolution (does not advance to the next scope).
- [ ] Implement the `@` intrinsic scope chain (D6): registered prepends → package root → package-area root → repo root → home → registered appends. Package/area/repo/home come from the intrinsic list exactly once (no caller should need to double-register them as convention roots — Phase 6 removes Claudine's double registration).
- [ ] Implement one shared containment helper for `&`/`^` (lexical check via `normalize_components` + canonical check on an existing target or deepest existing ancestor via `canonicalize`/`dunce` handling for junctions/reparse points) and route direct, recursive (`%`), completion, and the future lazy-ancestor path through it. Other kinds retain current symlink behavior.
- [ ] Completion (AC8): add direct `&`/`^` entry forms to `classify_token`/`CompletionEntryForm`/`completion_roots` enumerating the same roots in execution order; `%` completion remains `Ok(None)` without reinterpreting the token; malformed rooted payloads keep typed parse errors; implicit completion order flips with the engine; magic completion mirrors the new intrinsic chain.
- [ ] L1 resolution tests: for each D1 sigil, fixtures prove documented candidate order, first-match-wins, miss-as-`Ok(None)`, and typed errors; conflict fixtures where CWD and repo root both hold the file prove CWD wins (engine-level AC2); containment matrices — lexical `..` escapes, in-repo symlink still resolving, symlink-to-outside rejected, deepest-existing-ancestor behavior; catalog scope-selection matrices (root package, nested package, area-only, outside repo, second catalog); completion/execution parity tests per token form.
- [ ] Windows-correctness pass on the new code paths: `Path`/`PathBuf` and portable-path helpers only (no manual separator replacement); junction/reparse-target containment tests written now, gated to run on native Windows (AC9 — execution happens in Phase 8).

**Checkpoint:** `just test` and `just lint` pass in `biscuit-file/`; `grep -r cargo_metadata biscuit-file/` returns nothing; `grep -rn "find_package_area\b" biscuit-file/` returns nothing. Update `consumer-audit.md` with the resolved downstream breakage list for Phases 4–6.

---

## Phase 4 — Darkmatter: single catalog projection and resolver retirement

Implements the ownership table's Darkmatter rows + D7 wiring + AC4 (adapter
half). Centered on `darkmatter/lib/src/markdown/compose/util.rs` and the
request boundary in `compose/context/`.

- [ ] Implement the retained-observation → `RepositoryScopeCatalog` projection exactly once in Darkmatter, at the request boundary beside the existing repository capture group (`compose/context/capture/snapshot.rs` / `repo.rs` area): input is the retained `RepoInfo` plus the already-observed repository root expressed in a spelling lexically compatible with the reference base. Rebuild package and area roots from their repository-relative identities under that root spelling — never copy a foreign-spelled absolute `Package::path`. No filesystem observation inside the projection.
- [ ] Preserve Sniff topology semantics in the projection: nested-package ownership, `RepoInfo::package_area_label_for_dir`'s first-component fallback for newly scaffolded directories, known-area vs root behavior. Export the projection (pub) so Claudine's `derive_source` calls it — this is the only sniff→catalog adapter in the workspace.
- [ ] Use the projection in Darkmatter's ambient `md` request path (the `compose/context/options.rs` ambient fallbacks at ~lines 1025/1081), replacing `find_git_root_from` + `find_package_area_from` there.
- [ ] Delete `find_package_area_from` and `package_area_for_reference` from `compose/util.rs`; update the re-exports in `compose/mod.rs`.
- [ ] Replace every per-reference `find_git_root_from` fallback that feeds a resolution candidate with reads from the request's catalog: `document_resolution_context` (`compose/util.rs`), transclusion resolver (`compose/transclusion/resolver.rs:153`), `link_resolve.rs:166`, `link_normalization.rs:381`, `schema_validation.rs:54`, `schemas/resolve.rs:550`, `schemas/rewrite.rs:395`, `schemas/detect.rs:129`, `schemas/format.rs:389`, `expression/path_projection.rs:68`, and the `git_root`-style expression functions (`expression/functions/mod.rs:2214/2242`). `find_git_root_from` survives only for display-only helpers such as `abbreviate_path`; evaluate `schemas/clean.rs:156` and `shell_expansion/store.rs` against the "never feeds a resolution candidate" rule and record the ruling in `consumer-audit.md`. The `file_links/discovery.rs` boundary check is a different contract and stays.
- [ ] Rework `document_resolution_context` to build `FileResolutionContext` from the catalog (repository root + package/area anchors per source base) so `for_source` recomputation (Phase 3) drives nested documents; drop the now-dead `package_area` parameter plumbing at call sites.
- [ ] Update the reference graphing, TOC linking, preflight, expression functions, transclusion, and schema surfaces to consume the new grammar through `FileReference` (no prefix checks) — remove any surviving `starts_with('!')`-style dispatch found in the Phase 1 audit.
- [ ] Seeded work-counter/inventory guards (AC4): explicit resolution performs no ambient CWD, HOME, Git, Cargo metadata, or topology discovery — extend the existing work-counter test seams to fail on ambient discovery during `resolve_in_context`/`candidate_plan` driven from a document context.
- [ ] L1 tests: projection fixtures comparing the catalog's selected scopes with the retained `RepoInfo` across repository root, known area outside a package, newly scaffolded area, root and nested packages, a second repository, and symlink-equivalent root spellings (macOS `/var` vs `/private/var` exercised locally); a source-inventory guard test proving Darkmatter contains the only sniff-observation → catalog adapter.

**Checkpoint:** `just test` and `just lint` pass in `darkmatter/`; repository search shows no `find_package_area_from` / `package_area_for_reference` and no resolver-side `find_git_root_from` (only the display/allowlisted survivors recorded in `consumer-audit.md`). A standalone `md compose` of a monorepo document resolves `^`/implicit/`@` references against the document's own scopes.

---

## Phase 5 — Darkmatter: `ctx.cwd` and caller parameter materialization

Implements D8.1–D8.6 + AC5 + AC6 (Darkmatter half) + AC12. Centered on
`compose/context/` (groups, catalog, capture), `darkmatter/docs/schemas/darkmatter.yaml`,
and the caller-file binding in `compose/schema_validation.rs`.

- [ ] Add the no-I/O `ContextGroup::Invocation` to `compose/context/capture/groups.rs` (+ its KEYS table entry); requesting it never triggers repository capture and works outside a repo.
- [ ] Add `ctx.cwd` end to end: typed descriptor in `compose/context/catalog.rs`, `darkmatter/docs/schemas/darkmatter.yaml`, context help text, and every single-sourced projection. Value is the captured absolute launch directory converted with biscuit-file's portable-path helpers (no ad hoc separator replacement). Ambient compatibility entry points capture the process CWD exactly once at the request boundary; downstream composition never calls `current_dir()` to populate it. Ambient capture failure projects `null` plus the existing partial-capture diagnostic.
- [ ] Extend caller-file binding to both schema arms (D8.2–D8.5): a string is a file parameter only when the effective SimplifiedSchema selects `file` or `file(eager` for that property; arrays and unions recurse into the selected file arm. Eager probes the complete ordered candidate list, requires an existing regular local file, and materializes the winning absolute native path. Lazy (non-recursive, local) builds the unprobed candidate plan and materializes the first candidate as a lexically normalized absolute path — no existence probing, absence is not an error, `&`/`^` containment still runs against an existing target or deepest existing ancestor. Recursive lazy is a typed parameter-binding error suggesting `file(eager)`; lazy HTTP(S) stays a typed remote reference.
- [ ] Implement origin-decided anchoring (D8.3): CLI key/value and `--set` file values use the immutable launch file-resolution context; document frontmatter and defaults use that document's `SourceContext`; `proxy.with` evaluates/materializes in the proxying source before handoff; sequence task parameters use the sequence document that authored them. Once materialized, an absolute value is never re-anchored by a proxy target, retry, resume, loop iteration, or sequence task.
- [ ] Separate raw input from effective value (D8.6): the input layer retains the caller's raw override plus origin so a fresh epoch can reapply schema selection; every downstream expression, body, lifecycle, proxy, and launch-plan consumer sees the materialized effective value (frontmatter/lifecycle keeps native identity; Markdown presentation uses the existing portable sidecar). No downstream consumer reparses the raw relative string.
- [ ] Regression test (AC5): reproduce the 2026-08-26 `CompositionError` shape — repo-root shared prompt, launch from a package area, relative `spec`, `parent_dir(spec)`/`dirname(spec)`-derived sibling read inside the proxied prompt's `success` guard — and prove `{{ parent_dir(spec) }}/other-file.md` works from any document in the chain.
- [ ] AC5 matrix tests: caller CLI overrides, document defaults, `proxy.with`, sequence task parameters, direct/proxy/retry/resume/loop/sequence consumers, scalar/array/union arms, lazy first-plan selection (launch candidate wins even when absent), eager first-existing selection, lazy recursive rejection, lazy remote preservation, and ordinary string overrides untouched.
- [ ] Passive-contract tests (AC12): validation-only schema APIs remain non-mutating; only successful composition materializes caller file values.
- [ ] Seeded inventory/work guards: downstream composition never calls `current_dir()` for `ctx.cwd`; outside-repository documents still get `ctx.cwd` without requesting the repository group; a forced ambient CWD failure yields `null` + typed diagnostic.

**Checkpoint:** `just test` and `just lint` pass in `darkmatter/`; `md compose` of a fixture using `{{ ctx.cwd }}` and a lazy/eager `spec` parameter behaves per D8; the 2026-08-26 regression passes.

---

## Phase 6 — Claudine: source-scope integration, conventions, and `ctx.cwd` projections

Implements D7's Claudine consumers + AC4 (parity half) + AC6 (projection half)
+ AC7. Centered on `claudine/lib/src/invocation_context.rs`,
`claudine/lib/src/composition/resolve.rs`, and the prepared-context catalog.

- [ ] Rework `InvocationContext::derive_source` (`invocation_context.rs:777`): stop computing `package_area_root`/`package_root` via the local `package_roots()` helper; call Darkmatter's projection (Phase 4) on the retained `RepoInfo` plus the source-compatible repository-root spelling, then build the `FileResolutionContext` from the resulting catalog. Delete the now-unused local `package_roots` helper. A source in another repository continues to get its context from the invocation's per-repository cache.
- [ ] Provision anchors per D7 through the catalog-based `FileResolutionContext`: document-authored references resolve against the document's own scopes; caller-passed parameters use the immutable launch file-resolution context (`launch_file_resolution_context`). Source-relative convention roots are recomputed with the source's scope — not stored as launch-derived absolute prepend roots a nested document inherits. Request-stable roots (captured home, explicitly application-global prepend/append) remain stable.
- [ ] Review convention registration against D6 (`composition/resolve.rs::prompt_magic_roots` and its callers at resolve.rs:92/177/442 and `invocation_context.rs:1425`): the package root, package-area root, repository root, and home directory are no longer also registered as convention roots — they come from `@`'s intrinsic list exactly once. Claudine's conventions (`<package>/prompts`, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, peer-agent skills directories, `~/.claudine/prompts`) stay as registered prepend roots. Update `cli/src/completion/scopes.rs` and `cli/completion` surfaces that enumerate the same roots.
- [ ] Update `composition/resolve.rs::with_prompt_magic_paths` (the ambient legacy path at resolve.rs:420) to the catalog projection, or retire it if the Phase 1 audit shows it redundant with the context-aware path — record the ruling.
- [ ] Add `ctx.cwd` to Claudine's prepared-context catalog inputs and its schema/single-sourcing projections (the same surfaces `ctx.area`/`ctx.current_package` flow through); launch-CWD capture failure remains the existing invocation error; `ctx.cwd` immutable across retry/resume epochs and identical in preflight, body, effective frontmatter, and lifecycle.
- [ ] Switch the last ambient biscuit-file call in Claudine — `cli/src/commands/providers.rs:410` `find_git_root(&cwd)` — to the invocation context.
- [ ] Sweep the consumer-audit list: sequence source resolution (`composition/sequence/preflight`), harness diagnostics/resolution (`harness/`), system prompts (`system_prompt/prepare.rs`), overlay (`cli/src/commands/wrap/overlay.rs`), and CLI composition completion consume the correct source context and materialized parameter values; remove `!`-grammar handling everywhere (AC3 — no workspace code produces or consumes `Package`).
- [ ] Reconcile ctx-launch-anchor AC10's conflict fixtures explicitly (AC2): update the fixtures that locked repository-first so the composition-CWD copy wins for document-authored references and the launch-directory copy wins for caller-passed parameters; document the supersession in the fixture comments (spec D3 re-rules review-3 Finding 4).
- [ ] Parity test (AC4): a standalone `md compose` and a `claudine compose` of the same document produce identical `^` and implicit candidate plans and identical intrinsic `@` package/area/repository/home segments; Claudine's registered convention roots asserted separately at their D6 prepend/append positions so they cannot fail the parity test.
- [ ] Collision fixtures (AC7): the skill example `@.claude/skills/name/SKILL.md` finds the repo's copy first and falls back to `~/.claude/skills/...`; prompt-lookup conventions keep working; lock the complete effective prepend → intrinsic → append order; prove intrinsic roots occur exactly once; prove a nested document gets its own source-relative convention roots.

**Checkpoint:** `just test` and `just lint` pass in `claudine/` (library + CLI suites, including `dispatch_inventory`); parity test green; no `FileReferenceKind::Package` / `RootProvenance::Package` / `!`-file-reference usage remains anywhere in the workspace.

---

## Phase 7 — Claudine: `AGENT_CWD` and the spawn-seam inventory guard

Implements D8.7 + AC6 (`AGENT_CWD` half). **Parallelizable: may start
immediately after Phase 1; it has no dependency on Phases 2–6.**

- [ ] Add one shared child-environment contribution helper in the `claudine` lib that sets `AGENT_CWD` to the captured absolute launch directory, overwriting any inherited value. This is the only place the variable is written for Claudine's children.
- [ ] Define the capture rule: an ordinary top-level or nested Claudine invocation captures its own absolute entry CWD before any process-directory mutation and does not adopt inherited `AGENT_CWD`; the hidden provider-hook `handle` entry point (`cli/src/commands/handle.rs`) instead adopts the wrapper-supplied absolute `AGENT_CWD` as retained launch evidence, rejects a present non-absolute value, and uses the hook process's entry CWD only when the variable is absent. Any CLI route that can spawn a child captures this launch fact at entry even without building a composition `InvocationContext`.
- [ ] Wire the helper into every spawn seam: the provider launch (`claudine/cli/src/commands/wrap/env/mod.rs` `build_child_env`/`build_child_env_with_launch`), hook runners (`lib/src/dispatch/runner/bash.rs`), `::shell` execution (`lib/src/harness/shell.rs`, `lib/src/composition/sequence/task/shell.rs`), and the lifecycle executor (`lib/src/composition/sequence/../lifecycle/executor.rs`). From the Phase 1 audit, also govern `lib/src/model_catalog/provider_sources.rs`, `lib/src/system_prompt/context.rs`, and `lib/src/linking/paths.rs` — each either calls the helper directly or is recorded in the guard allowlist as an indirect governed path (an allowlist may describe an indirect path but may not exempt a child from `AGENT_CWD`).
- [ ] Extend `debug_assert_child_env` (`cli/src/commands/wrap/exec/spawn/setup.rs`) — or add a sibling assertion — so the provider seam asserts `AGENT_CWD` presence and absoluteness.
- [ ] Build the spawn-seam inventory guard as a new test in the style of `cli/tests/dispatch_inventory.rs`: scan production sources in `claudine/lib/src` and `claudine/cli/src`, classify every `std::process::Command` and `tokio::process::Command` construction (including aliased, helper-returned, and inline `status`/`output` forms), and fail when a child can execute without the shared contribution helper. Exclude `#[cfg(test)]` bodies, test-only files, and clap's unrelated `Command` builder. Commit the generated inventory artifact next to `dispatch-inventory.json` with its regenerate command.
- [ ] Scanner unit fixtures prove the guard is non-vacuous: for each supported construction form, present one governed and one deliberately ungoverned seam and assert pass/fail — without mutating production source during the test.
- [ ] Behavioral fixtures (AC6): `AGENT_CWD` present in every spawned child's environment (provider, hook, `::shell`, lifecycle executor, sequence shell task) as the captured absolute launch directory; overwrites an inherited stale value; stable across retry/resume/loop/sequence re-entry; a wrapper → provider → `handle` → hook-action chain retains the wrapper's launch value despite the provider child CWD; an ordinary nested invocation with a stale inherited value publishes its own entry CWD; missing and non-absolute inherited values on `handle` cover the fallback/error rule.
- [ ] Document the environment contract (D8.7 residual risk): `AGENT_CWD` is un-namespaced, Claudine overwrites it for its own children, and an unrelated tool reading it with different expectations is an accepted risk — note it wherever the child environment is documented (`claudine/docs/topics/execution-flow.md` environment section).

**Checkpoint:** `just test` and `just lint` pass in `claudine/`; the new inventory guard runs green and fails when temporarily pointed at an ungoverned fixture; all behavioral `AGENT_CWD` fixtures pass.

---

## Phase 8 — Documentation, audit proofs, L2, and the cross-platform matrix

Implements the documentation scope, AC9, AC10, AC13, and closes AC12.

- [ ] Refresh `biscuit-file/docs/topics/file-references.md` to the finalized grammar: the D1 sigil table with candidate orders, `&`/`^` containment guarantees (lexical + resolved target, with the time-of-check/time-of-use limitation stated — biscuit-file is not a sandbox), the D9 reserved-introducer rules, the flipped implicit order, the `@` effective order under registered conventions, and the `&` shell-control-operator quoting note (`spec='&docs/plan.md'`).
- [ ] Prepare and land the design-intent document amendment required by AC13/OQ1 (`claudine/docs/topics/file-referencing.md`): distinguish forms authors may *encounter* (`file:` URIs, device prefixes) from forms the grammar *accepts* (rejected + reserved with typed errors). This is the one design-intent edit the spec mandates; any other wording change still requires Ken's approval.
- [ ] Apply the drift rule to `.claude/skills/` entries that describe file referencing (claudine composition skill, shell-completions topic where it documents magic-root order) and to per-area `docs/dependencies.md` if `cargo_metadata` leaves biscuit-file's dependency set.
- [ ] Run the repository-search proof set and record results in `consumer-audit.md`: zero `!` file references in prompts (every `!` is expression negation); no `cargo_metadata` in `biscuit-file/`; no `find_package_area_from` / resolver-side `find_git_root_from` in `darkmatter/`; no `FileReferenceKind::Package` / `RootProvenance::Package` / `MissingPackageContext` anywhere; Darkmatter holds the only sniff→catalog adapter and Claudine calls it.
- [ ] Add/extend L2 coverage on the real compose and sequence surfaces per the repo's testing taxonomy (L2 fixtures must not take terminal focus): compose with `&`/`^`/`@`/implicit references from a nested package document, a proxied prompt with a materialized `spec`, a sequence task passing file parameters, and completion parity through the CLI completion surface.
- [ ] Cross-platform validation (AC9/AC10): host-independent parser tests run everywhere; filesystem-specific containment tests (junctions/reparse points) run on native Windows. Drive the matrix: local macOS full suites, then `build-linux`, `build-win`, and `build-win-native` for the three affected areas — coordinate with the Phase 1 ruling on the ctx-launch-anchor deferred gates so both land green before hosted CI.
- [ ] Final validation sweep: `just test`, `just test-l2`, and `just lint` pass in `biscuit-file/`, `darkmatter/`, and `claudine/`; no L2 fixture took focus during the runs.
- [ ] Walk the spec's acceptance criteria AC1–AC13 one by one against the implemented state and record the verdict matrix in this feature directory (e.g. `acceptance.md`); flag any criterion that needs a spec correction because the design-intent document disagrees (spec: the design-intent document wins).

**Checkpoint:** all three areas green on macOS + Linux + Windows environments; documentation and skills updated; acceptance matrix complete with every AC satisfied or explicitly escalated.
