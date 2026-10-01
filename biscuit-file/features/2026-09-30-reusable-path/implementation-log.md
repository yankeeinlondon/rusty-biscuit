---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-reusable-path/biscuit-file/features/2026-09-30-reusable-path/spec.md"
plan: "biscuit-file/features/2026-09-30-reusable-path/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-09-30-reusable-path (8 phases)

## Rulings (R1-R11)

The author was not available: phase 1 ran in a non-interactive session, and
the plan's frontmatter sets `yolo: true`. Each ruling below therefore adopts
the plan's recommendation as the **working ruling**. Later phases cite these
numbers. The author can overturn any of them before the phase it gates; none
gates Phase 2 (the behavior-neutral rename).

| #   | Working ruling | Gates | Status |
| --- | -------------- | ----- | ------ |
| R1  | The default strategy has no `ParentDir`; an in-tree deep-parent position link outside a repository falls through to `EnvRootedPath` / `HomeDir` / `AbsolutePath`. Intended. Callers who want deep-parent relative links add `ParentDir`. The topic page records it. | Phase 6 | provisional |
| R2  | `FileReferenceError::CwdOutsideBaseDir { base_dir, cwd }` for non-repository context validation; `FileReferenceError::RelativeTreeEscape { base_dir, candidate, reference }` for a blocked relative candidate. `RepositoryEscape` and `RepositoryRootNotContainingSource` are untouched. | Phase 3 | provisional |
| R3  | Trusted-external counterpart: `for_trusted_external_source_reference`. | Phase 3 | provisional |
| R4  | Reader opt-in: `FileResolutionContext::allow_external_relative(self) -> Self` and `external_relative_allowed() -> bool`; copied on every child derivation. | Phase 3 | provisional |
| R5  | `pub enum BaseDirOrigin { Repository, Explicit, Vault, Home, Environment { name }, Fallback }`, with `base_dir_origin()` and `base_dir_is_boundary()` (false only for `Fallback`). | Phase 3 | provisional |
| R6  | Anchor selection (home / environment as tree root) compares lexically, which preserves the authored identity. Only the boundary check canonicalizes. | Phase 3 | provisional |
| R7  | Windows symlink fixtures: directory cases use junctions (no privilege needed); file-symlink cases skip with a stated reason when creation is denied (gated, never a silent pass). The macOS and Linux legs carry the symlink proof. | Phase 3, 6 | provisional |
| R8  | At Phase 4 start, check whether `2026-09-30-file-refs-use-magic` has landed. Build on whichever vocabulary is present and never reintroduce old names. If it has not landed, proceed and log that. | Phase 4 | provisional |
| R9  | `PortablePath` lives in `biscuit-file/lib/src/file_reference/portable/` (`path_identity`, `strategy`, `env_anchor`, `diagnostics`, `evaluate`), re-exported from `file_reference/mod.rs` and `lib.rs` under `file-reference`. | Phase 5 | provisional |
| R10 | No real-corpus acceptance run or timing measurement is scheduled. The author may ask for a corpus run after implementation. | author | provisional |
| R11 | A `cwd`/context failure on a *position* form is `UnresolvableInput`; on a kept *intent* form it is a `Finding` (the reference is returned unchanged), via a typed `Finding::ResolutionFailed { kind }` variant. | Phase 6 | provisional |

## Phase 1

Phase 1 changed no source code. It produced the rulings above, the
inventories below, and the baseline.

### Constructor audit inventory

Search: `rg 'FileResolutionContext::new\b|FileResolutionContext::from_snapshot|\.for_base\(|for_trusted_external_base|request_base_dir\('`,
excluding `target/` and spec directories.

**Classification result: every constructor and `for_base` argument in the
monorepo is a "document directory" (where `./` starts), so all of them become
`cwd` in Phase 2 with no meaning change.** No call site passes a value
intended as a tree root. A few pass a directory that *coincides* with the
repository root. Those still mean "where `./` starts". In Phase 3 the tree
root comes from `with_repository_root`, not from the constructor argument:

- `messenger/lib/src/research/load.rs:122`:
  `FileResolutionContext::new(root.clone()).with_repository_root(root)`. The
  cwd is the repository root on purpose.
- `claudine/gen/src/inputs.rs:255`: `new(area).with_repository_root(..).with_package_area(area)`;
  the cwd is the package area.
- `claudine/lib/src/composition/lifecycle/executor.rs:1730`:
  `for_base(self.effect_engine.mutation_root())`, the cwd for effect paths.
- `darkmatter/lib/src/markdown/compose/link_normalization.rs:584,659,693,1279,1292`
  (tests): `new(project_root)` / `new(root)` / `new(home)`. These are cwd
  values. The environment anchor is the separate `PROJECT_ROOT` env entry.

**Correction to the plan:** `messenger` is a genuine consumer
(`messenger/lib/src/research/load.rs`). It is not only an unrelated
`base_dir` owner, so Phase 2 Wave 2 must compile it.

Crates with `FileResolutionContext` in Rust source (file counts):
biscuit-file/lib 11, claudine/cli 17, claudine/lib 33, claudine/gen 1,
darkmatter/lib 25, darkmatter/cli 1, messenger/lib 1. `research` and
`worktree` do **not** use `FileResolutionContext`. Their `base_dir`
identifiers are unrelated (research library directory; `~/.worktree.json`).

Production / non-test constructor and derivation sites (all → `cwd`):

| Crate | Site | Call |
| ----- | ---- | ---- |
| biscuit-file | `lib/src/file_reference/context.rs:454,488,563,578,764` | definitions of `from_snapshot`, `new`, `for_base`, `for_trusted_external_base`, `request_base_dir` |
| messenger | `lib/src/research/load.rs:122` | `new(root)` |
| darkmatter-cli | `cli/src/commands/compose.rs:211,273` | `new(launch_dir)`, `from_snapshot(..)` |
| darkmatter | `lib/src/markdown/compose/util.rs:80,81` | `for_base(base_dir)`, `new(base_dir)` |
| darkmatter | `lib/src/markdown/compose/context/options.rs:760,1457,1515,2408` | `new(&base_dir)`, `for_base(&base_dir)` ×2, `request_base_dir()` |
| darkmatter | `lib/src/markdown/compose/context/capture/mod.rs:75` | `new(base_dir)` |
| darkmatter | `lib/src/markdown/compose/link_resolve.rs:182` | `for_base(dir)` |
| darkmatter | `lib/src/markdown/compose/schema_validation.rs:776,778,780` | `for_trusted_external_base(fallback)`, `for_trusted_external_base(request_base_dir())`, `new(fallback)` |
| darkmatter | `lib/src/markdown/compose/transclusion/resolver.rs:149` | `for_base(&base_dir)` |
| darkmatter | `lib/src/markdown/compose/expression/resolve_ctx.rs:362,404,450` | `for_base(base_dir)` (Darkmatter's own `ResolutionContext.base_dir` feeds it; renamed in Phase 4) |
| darkmatter | `lib/src/markdown/schemas/detect.rs:105` | `for_base(request_context.base_dir())` |
| darkmatter | `lib/src/markdown/schemas/resolve.rs:570` | `for_base(base_dir)` |
| claudine | `lib/src/system_prompt/resolve.rs:289,290` | `new(context.cwd)` |
| claudine | `lib/src/harness/resolve.rs:187` | `new(base_dir)` |
| claudine | `lib/src/invocation_context.rs:2108` | `from_snapshot(base_dir, home, env)` |
| claudine | `lib/src/composition/sequence/source.rs:95` | `new(base_dir)` |
| claudine | `lib/src/composition/resolve.rs:74,147,655` | `new(&cwd)`, `from_snapshot(..)`, `new(anchor.parent())` (655 is a test fixture helper) |
| claudine | `lib/src/composition/lifecycle/executor.rs:1730` | `for_base(mutation_root())` |
| claudine-cli | `cli/src/completion/scopes.rs:240` | `new(ctx.cwd)` |
| claudine-gen | `gen/src/inputs.rs:255` | `new(area)` |

Test-only sites (all cwd-meaning; renamed mechanically). Counts per file:

- biscuit-file/lib/tests/l1: `resolution_context.rs` 22,
  `finalized_reference_resolution.rs` 8, `completion_round_trip.rs` 5,
  `detailed_resolution.rs` 4, `magic_local_roots.rs` 4 (plus `//!` line 10),
  `repository_scope_catalog.rs` 4, `precedence_flip.rs` 2.
- darkmatter/lib (in-module tests): `compose/tests/schema.rs` 30,
  `compose/context/options.rs` 3315-3601 (6), `compose/schema_validation.rs`
  1550-3131 (7), `compose/link_normalization.rs` 7, `compose/util.rs`
  329,375, `compose/link_resolve.rs` 259, `compose/transclusion/resolver.rs`
  334, `compose/expression/{resolve_ctx.rs:571, path_projection.rs:214,
  functions/repository.rs:130, functions/mod.rs:3879,5123}`,
  `schemas/{validate.rs:1182,1184, file_match.rs:366, rewrite.rs:496,498,
  mod.rs:2876, detect.rs:710,726, resolve.rs:353,364,3120, format.rs:618,620}`,
  `reference/graph.rs:1114,1226`; darkmatter/lib/tests/l1:
  `link_interpolation_integration.rs`, `reference_integration.rs`,
  `unknown_identifier_warning.rs` (2).
- claudine: `lib/src/composition/resolve/tests.rs` 14,
  `schema/supplied/tests.rs` 7, `sequence/tests.rs` 3, `prep/tests.rs` 3
  (cli), plus 1-2 each in `error/tests.rs`, `lifecycle/control/tests.rs`,
  `lifecycle/executor/tests/filesystem_lookup.rs`,
  `looping/expression/tests/resolution_context.rs`, `preflight/tests.rs`,
  `prepare/service/tests.rs`, `prepare/tests.rs`,
  `sequence/preflight/tests.rs`, `sequence/expr.rs:202`,
  `cli/src/commands/{sequence.rs:556, schema_interactive/supplied.rs:147,
  wrap/composition/tests.rs}`,
  `cli/src/completion/operation_file/recovery_tests.rs` (2),
  `cli/tests/l1/sequence_sources_cli.rs`.

Docs and examples carrying constructor calls: see the skill/doc inventory
below. No `examples/` directory uses `FileResolutionContext`. The only
`from_snapshot` hit in an example is Loro's unrelated
`LoroDoc::from_snapshot`.

**Additional rename-adjacent names the plan's table omits** (Phase 2 should
rule on them so the vocabulary is consistent):

- `FileResolutionContext.base_dir` field and `request_base_dir` field
  (`context.rs:427,438`). These follow the accessor renames.
- `FileResolutionContext::is_trusted_external_authoring_base()` and its field
  `trusted_external_authoring_base` (`context.rs:770`). This names the
  *authoring cwd*. Recommendation: `is_trusted_external_authoring_cwd()`.
- internal `ResolutionContext::from_base(base)` (`context.rs:344`, ambient
  compatibility path for `resolve_from`). This is internal and already
  produces `cwd`. Optional rename to `from_cwd`.
- `FileReference::resolve_from(base: &Path)` and
  `complete_partial(token, base)` parameter names, plus the internal
  `CompletionAnchors::base` (`resolve.rs`, used at 1520/1569). These are
  cwd-meaning parameter names. Renaming them is cosmetic and does not change
  the API's signature.
- `FileReference::resolve_relative(base: Option<&Path>)` (`mod.rs:937`), whose
  local is named `base_dir`. It is a "relative from" directory, not a tree
  root.

### Accessor inventory (`base_dir()`, `DetailedResolution::base_dir()`, `request_base_dir()`)

Five types define a `base_dir()` method: `FileResolutionContext`
(`context.rs:733`), `DetailedResolution` (`mod.rs:333`), Claudine's
`SourceContext` (`claudine/lib/src/invocation_context.rs:548`), Claudine's
`ResolutionDetail` (`claudine/lib/src/harness/error.rs:113`), and Sniff's
builders (`SniffConfig::base_dir`, `DetectionPlan::base_dir`). Several calls
that look like the biscuit-file accessor belong to the two Claudine types.
Phase 2 must not rename those by text search.

**`FileResolutionContext::base_dir()` → `cwd()`** (genuine callers):

| Crate | Sites |
| ----- | ----- |
| biscuit-file | `lib/src/file_reference/mod.rs:724`; `resolve.rs:1477`; tests `tests/l1/resolution_context.rs:266,343`. Internal field reads in `context.rs` 374, 533, 536, 566, 734, 874, 915, 919, 929 |
| darkmatter | `schemas/file_match.rs:165`; `schemas/detect.rs:105`; `compose/expression/functions/mod.rs:1523`; `compose/expression/resolve_ctx.rs:361`; `compose/context/options.rs:2407`; `compose/schema_validation.rs:830,1194,1204,1215,1257,1269,1273`; tests `compose/tests/schema.rs:2451,2597,2598,2662` |
| claudine | `lib/src/composition/error/render/mod.rs:296`; `lib/src/composition/schema/supplied.rs:305`; test `lib/src/invocation_context/tests.rs:66` |
| claudine-cli | `cli/src/commands/schema_interactive/supplied.rs:29`; test `cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs:91` |

**Not `FileResolutionContext` (do not rename in Phase 2 Wave 1):**
`SourceContext::base_dir()` at `claudine/lib/src/invocation_context/tests.rs:624,669,696,701`,
`composition/sequence/preflight/tests.rs:112`,
`claudine/cli/src/commands/wrap/sequence/task_run.rs:123,779`;
`ResolutionDetail::base_dir()` at `composition/error/render/provider.rs:160`,
`composition/resolve/tests.rs:472`.

**`DetailedResolution::base_dir()` → `cwd()`:** defined at `mod.rs:333`
(field at 301, set at 734/757). The only consumer is
`claudine/lib/src/harness/error.rs:84` (`ResolutionDetail::from_detailed`).
Whether Claudine's own `ResolutionDetail::base_dir` follows is a Phase 2
Wave 2 call. It is a diagnostic projection (`claudine/docs/topics/error-architecture.md`).

**`request_base_dir()` → `request_cwd()`:** defined at `context.rs:764`;
callers `darkmatter/lib/src/markdown/compose/context/options.rs:2408` and
`darkmatter/lib/src/markdown/compose/schema_validation.rs:778`. Internal field
uses in `context.rs` 438, 468, 498, 765, 927, 929.

**Darkmatter's expression `ResolutionContext.base_dir`** (field only, no
method; `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:37`;
renamed in Phase 4): uses in `expression/functions/mod.rs` (28),
`resolve_ctx.rs` (5), `context/options.rs` (3331,3344,3345),
`schema_validation.rs:1030`, and
`claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs:409`.
Constructed positionally with `ResolutionContext::new(base_dir)` at
`options.rs:1468,1526`, `functions/repository.rs:132`, and in Claudine at
`dispatch/expression.rs:178` and `sequence/expr.rs:102`. Phase 4's
constructor audit must cover those.

**Unrelated `base_dir` owners** (excluded): sniff (`SniffConfig`,
`DetectionPlan`, CLI output), worktree (`WorktreeConfig`), research
(`discover_topics`), test-toolkit (worktree config JSON), biscuit-visualized
(`FileCache`). Claudine has many of its own fields: `SourceContext`,
`ResolutionDetail`, `StackExecutionContext`, `LoopExpressionLookup`,
`SourceExpressionLookup`, `EventMetaConditionLookup`,
`RelativePathEventFormat`. Darkmatter has its own fields too:
`FileReferenceDiagnostic`, `ContextCapture`, `EffectiveSchema`, schema
`Namespace`, `FileRefAnchors`, and others. playa and messenger have no Rust
`base_dir` identifier.

### `ComparisonKey` behavior audit (acceptance list for Phase 5)

Source: `darkmatter/lib/src/markdown/compose/link_normalization.rs:27-175`
(`ComparisonKey`, `comparison_key`, `drive_root`, `unc_root`) and `:520-571`
(`compute_relative_path`, `strip_macos_private`).

What it does today:

| Input aspect | Non-Windows | Windows | Spec requirement | Gap |
| ------------ | ----------- | ------- | ---------------- | --- |
| `.` segment | dropped (`Component::CurDir` filtered; `Path::components` already drops interior `.`) | dropped outside a verbatim namespace; **kept as a literal name** inside `\\?\` | collapse `.`; never reinterpret literal verbatim dot segments | matches |
| `..` segment | **kept as a literal component `".."`** (no collapse) | **kept as a literal segment** in both namespaces | collapse `..` without walking above a root (ordinary host paths); keep it literal under a verbatim prefix | **gap: no `..` collapse on ordinary paths.** `a/b/../c` and `a/c` produce different keys |
| Root | `rooted` flag only; no prefix | `root` = `C:` (drive letter upper-cased), `\\server\share`, or raw prefix text for `\\.\` and `\\?\Volume{…}`; plus `rooted` (separates `C:\a` from `C:a`) | different drives/shares are separate roots | matches |
| Verbatim vs legacy | n/a | `C:\x` ≡ `\\?\C:\x`; `\\server\share\x` ≡ `\\?\UNC\server\share\x`; DeviceNS and other `Verbatim` keep their own prefix | normalize a verbatim prefix only where safe simplification permits | **differs in approach.** The key equates spellings by root, regardless of whether `dunce` could simplify the whole path. Its docs say this is deliberate: a long descendant must stay inside a short root. Phase 5 must keep that property; see the note on `normalize_components` below |
| Separators | `/` only (from `Path::components`) | `\` always; `/` only outside verbatim | parse native components faithfully | matches |
| Case | case-sensitive | only the drive letter folded; components case-sensitive | do not case-fold filenames | matches |
| Encoding | lossless `OsString` | lossless UTF-16 units (`encode_wide`), so unpaired surrogates stay distinct | lossless identity | matches |
| Symlink aliases | `strip_macos_private` drops a leading `private` component before computing relative paths (`/private/tmp` ≡ `/tmp`) | same function, which is inert on Windows | **do not equate symlink aliases** in preference selection | **conflict.** This is an alias equation. Phase 5 must either drop it (tests must then compare canonicalized or same-spelling inputs) or record a deliberate, narrowly scoped exception |
| File vs directory `from` | `compute_relative_path` treats `from` as a file when its last component **has an extension**, else as a directory | same | relative strategies compute from `cwd` (a directory, given explicitly) | **heuristic to drop.** `PortablePath` knows `cwd`, so it must not guess from extensions |
| Output | `(parent_hops, forward_names)`, kept separate so a generated `..` stays distinct from a literal `..` name | same | reject a spelling that would change native components or introduce reference grammar | keep the split-output design |

Related in biscuit-file (the shared implementation must reconcile with it,
not duplicate it):

- `resolve.rs::normalize_components` collapses `.`/`..` through
  `Path::components` and then `dunce::simplified`. Every lexical comparison
  in the resolver funnels through it (candidate dedupe, repository
  containment, `diff_paths`). On Windows, `Path::components` still yields
  `ParentDir` for `..` under a verbatim prefix, so this function **collapses
  `..` inside `\\?\` paths**, which `ComparisonKey` deliberately refuses to
  do. This is unverified on a Windows host in phase 1. Phase 5 should
  pin it with a Windows test before choosing which behavior the shared
  identity adopts.
- `resolve.rs::diff_paths(target, base)` is a second relative-path
  implementation. It works on `Path` components after `normalize_components`
  and returns `"."` for equal paths. It is used by `mod.rs:953` (relative
  path from a resolved reference) and its own tests (`resolve.rs:1698-1760`,
  including the verbatim/legacy `C:\` case).

### Resolver touch-point map (Phase 3 boundary seam)

All entry points converge on two functions that share one prelude:
`interpolate` → `compute_effective_anchoring` → repository-root selection.

- **Entry points** (`file_reference/mod.rs`): `resolve` (640) and
  `resolve_from` (666) use ambient `ResolutionContext::from_ambient` /
  `from_base`. `resolve_in_context` (701) and `resolve_detailed` (722) call
  `resolve::resolve_core` (mod.rs:744). `candidate_plan` /
  `candidate_plan_with_order` (778/803) call `resolve::candidate_plan`
  (mod.rs:784). `validate_repository_candidate` (mod.rs:823) calls
  `resolve::validate_repository_containment`. `complete_partial` (881) and
  `complete_partial_in_context` (917) call `resolve::complete_partial` and
  `resolve::complete_partial_in_context`.
- **Shared prelude** (duplicated in `resolve_core` at resolve.rs:81 and
  `candidate_plan` at resolve.rs:1184): `interpolate` (1233) →
  `compute_effective_anchoring` (194) → `effective_public_kind` (270) →
  repository root (`resolve_repository_root` 682, or the launch scope for `@`).
- **Candidate planning**: `build_candidates` (1019) for direct references,
  which uses `build_anchoring_candidates` (1073) for the local anchoring
  family: `Absolute` verbatim, `ExplicitRelative` = `cwd.join`,
  `ImplicitRelative` = `implicit_relative_roots` (994: cwd, then repository
  root). It uses `collect_roots` (895) for sigil, magic, and vault kinds and
  runs `validate_repository_lexical` for `&`/`^`. `build_search_roots` (1102)
  and `collect_anchoring_roots` (1139) handle recursive (`%`) references.
- **Existing containment check** (the code to reuse):
  `validate_repository_lexical` (1288) and `validate_repository_containment`
  (1309: lexical, then `dunce::canonicalize` of root and of
  `deepest_existing_ancestor` (1341)). Callers: `resolve_direct_core` per
  candidate before probing (449), `resolve_recursive_core` per root (566)
  and per match (624), `expand_completion` per completion root (1536),
  `FileReference::validate_repository_candidate` (mod.rs:831).
- **Completion**: `complete_partial_in_context` (1459) → `expand_completion`
  (1506) → `completion_roots` (1556). `CompletionEntryForm::ImplicitRelative`
  pushes `anchors.base` (the cwd) and the repository root.

**Seam for Phase 3:** generalize the containment pair into one function, for
example `validate_tree_containment(kind, reference, candidate, root) ->
Result<(), FileReferenceError>`. It produces `RepositoryEscape` for `&`/`^`
and `RelativeTreeEscape` for relative kinds. Call it at the four existing
`validate_repository_containment` call sites in `resolve.rs`, gated on the
**effective** anchoring (`ExplicitRelative` / `ImplicitRelative`, including
the bare repository-fallback candidate) and on `base_dir_is_boundary() &&
!external_relative_allowed()`. Do the same in `build_anchoring_candidates`
/ `build_candidates` for the lexical pre-check that `candidate_plan` sees.
`ResolutionContext` (internal, `context.rs:281`) needs the new `base_dir`,
the boundary flag, and the opt-in copied in by `from_context` (`context.rs:372`).
`CompletionAnchors` needs the same for `ImplicitRelative` completion roots.
`Absolute` effective anchoring (including an absolute `{{VAR}}` expansion)
is untouched.

### Baseline

Recorded on macOS (this host) at `87f280dfb` with a clean working tree apart
from this feature's untracked plan and log. Phase 2's "no behavior change"
claim is checked against these numbers.

| Area | `just test` (L1) | `just test-l2` | `just lint` |
| ---- | ---------------- | -------------- | ----------- |
| biscuit-file | pass: 860 run, 860 passed, 0 skipped (plus 6 doctests passed) | not applicable (the recipe is a stub: "test-l2: not applicable for biscuit-file") | pass |
| darkmatter | pass: 8726 run, 8726 passed, 12 skipped | pass: 18 + 69 + 3 run, all passed | pass |
| claudine | **fail (pre-existing)**: with `--no-fail-fast`, 8071 run, 8070 passed, **1 failed**, 9 skipped | pass: 277 + 3 run, all passed | pass |

**Pre-existing failure (not caused by this feature):**
`claudine-cli::bin/claudine completion::composition::tests::compose_magic_does_not_emit_a_nested_file_without_its_scope`
(`claudine/cli/src/completion/composition/tests.rs:348`). It panics with
`completion must not flatten a nested path that runtime cannot resolve: ["@plan.md"]`.
The cause is test isolation. `ScopeContext::discover_from`
(`claudine/cli/src/completion/scopes.rs:214`) captures the real
`dirs::home_dir()`, and this host has `~/.claudine/prompts/plan.md`, so the
user-global prompt scope legitimately completes `@plan.md`. The test passes
on a host without that file. Treat it as a known failure in the Phase 2-8
comparisons. Fixing it means injecting a temporary home into the test's
`ScopeContext`. That is out of scope for this feature and has not been done.

Note: the default `just test` in claudine fails fast. The first run stopped
after 7439 of 8071 tests on this one failure, so use `just test
--no-fail-fast` when comparing counts.

### Skill and doc inventory

Files that mention renamed names, `with_env_path_whitelist`, `PROJECT_ROOT`,
`DOCS_BASE`, or link normalization, with the phase that must update them:

| File | Mentions | Update in |
| ---- | -------- | --------- |
| `biscuit-file/docs/topics/file-references.md` | `new` (444,465,513,515), `for_base` (484,502,546), `for_trusted_external_base` (535,548), `base_dir` (95,451,460,482,486,544,559,629), `{{PROJECT_ROOT}}` example (395,408) | Phase 2 rename; Phase 8 tree root/boundary |
| `biscuit-file/docs/tech-spec/file-reference-struct.md` | `for_base` (84) | Phase 2 rename |
| `.claude/skills/biscuit-file/references/file-references.md` | `new` (70,82), `for_base` (93,107), `for_trusted_external_base` (97), `base_dir()` (144) | Phase 2 rename; Phase 8 |
| `biscuit-file/lib/src/file_reference/context.rs` (doc comments) | `for_base` (153,420,884), `new` (397), `base_dir` (403,405,483,486), `for_trusted_external_base` (409) | Phase 2 |
| `biscuit-file/lib/tests/l1/magic_local_roots.rs` (`//!`) | `from_snapshot` (10) | Phase 2 |
| `darkmatter/docs/inline/link-normalization.md` | `PROJECT_ROOT`, `DOCS_BASE` (22), `with_env_path_whitelist` (23,39) | Phase 7 |
| `darkmatter/lib/src/markdown/compose/context/options.rs` (doc comments) | `PROJECT_ROOT`/`DOCS_BASE` (453,1195,1209-1210), `with_env_path_whitelist` (1208), `base_dir` (1918) | Phase 7 (1918: Phase 4) |
| `.claude/skills/darkmatter/compose.md` | `PROJECT_ROOT` (93), `source_base_dir` (156), link normalization (213,821,1094), `ResolutionContext (base_dir, …)` (1118) | Phase 4 (1118,156), Phase 7 |
| `.claude/skills/darkmatter/SKILL.md` | "Root-only link normalization" (82) | Phase 7 (wording check) |
| `darkmatter/docs/topics/magic-paths.md` | `FileResolutionContext::new` (107) | Phase 2 |
| `darkmatter/docs/topics/darkmatter-expressions.md` | expression `ResolutionContext.base_dir` (1179) | Phase 4 |
| `darkmatter/docs/topics/schemas/definition.md` | `resolve_from(base_dir)` (766) | Phase 8 (wording) |
| `darkmatter/lib/src/markdown/compose/expression/{resolve_ctx.rs,path_projection.rs}` (doc comments) | expression `base_dir` (resolve_ctx 46,106,330,331,381; path_projection 27,28,55) | Phase 4 |
| `darkmatter/lib/src/markdown/compose/{link_resolve.rs:36, pipeline/operations.rs:116}`, `darkmatter/lib/tests/l1/nested_composition.rs:232`, `darkmatter/lib/README.md:72`, `darkmatter/docs/cli/compose.md:11` | link normalization wording | Phase 7 (wording check) |
| `darkmatter/docs/{darkmatter-compose-pipeline.md:60,131, operations/link-resolve.md:28, composition/frontmatter-in-pipelining.md:52}` | links to `link-normalization.md` | Phase 7 only if that page moves (the `frontmatter-in-pipelining.md` link targets `../operations/…` but the page lives in `inline/`, so it may already be broken) |
| `darkmatter/lib/src/markdown/schemas/{validate,format,resolve,rewrite,file_match,example}.rs`, `darkmatter/lib/src/markdown/compose/util.rs:66` (doc comments) | `base_dir` as the document directory (~18) | Phase 2 (where the symbol is renamed) |
| `claudine/docs/topics/error-architecture.md` (189,250,261), `claudine/lib/src/diagnostics/mod.rs:45` | `base_dir` diagnostic field ("compatibility projection") | Phase 7 only if that diagnostic field changes |
| `claudine/lib/src/invocation_context.rs` (1759,2092,2174,2181), plus ~22 descriptive `base_dir` / `ctx_base_dir` doc lines across `claudine/lib/src/{harness/resolve.rs, composition/…}` and `claudine/lib/tests/l1/boundary_lint.rs` | document-directory `base_dir` | Phase 2 Wave 2 (rename where they describe the context accessor) |

`biscuit-file/README.md`, `.claude/agents`, `.claude/commands`, and
`.claude/skills/claudine/**` have no relevant mentions. `ComparisonKey`
appears in no doc. Excluded as unrelated `base_dir`: sniff (detection base),
worktree and test-toolkit (`~/.worktree.json`), research (library directory),
claudine `signals/harvest.rs` (output directory), DMLS lexical helpers,
frontmatter keys in fixtures, and historical review files.
