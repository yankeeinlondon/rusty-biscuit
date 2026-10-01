---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-reusable-path/biscuit-file/features/2026-09-30-reusable-path/spec.md"
plan: "biscuit-file/features/2026-09-30-reusable-path/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/tests/l1/magic_local_roots.rs
  - biscuit-file/lib/tests/l1/repository_scope_catalog.rs
  - biscuit-file/lib/tests/l1/resolution_context.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/compose/util.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/file_match.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/rewrite.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/schema/supplied.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/cli/src/commands/schema_interactive/supplied.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
docs_updated_during_phase_2:
  - biscuit-file/docs/topics/file-references.md
  - biscuit-file/docs/tech-spec/file-reference-struct.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_3:
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/error.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/tests/l1/file_tree.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/tests/l1/resolution_context.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/error/tests.rs
docs_updated_during_phase_3:
  - biscuit-file/docs/topics/file-references.md
  - biscuit-file/docs/tech-spec/file-reference-struct.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/biscuit-file/SKILL.md
packages:
  - biscuit-file
  - darkmatter
  - claudine
  - claudine-cli
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

## Phase 2

Phase 2 is the behavior-neutral Step 1 rename: the context's "where `./`
starts" value is now spelled `cwd` everywhere. No `base_dir` identifier
remains on `FileResolutionContext` or `DetailedResolution`, so a missed call
site fails to compile when Phase 3 reintroduces `base_dir` as the tree root.

### Wave 1: biscuit-file

Renamed public API (`biscuit-file/lib/src/file_reference/{context.rs,mod.rs,resolve.rs}`;
`fetch.rs` had no occurrences):

| Before | After |
| ------ | ----- |
| `FileResolutionContext::base_dir()` (and field) | `cwd()` |
| `new(base_dir)`, `from_snapshot(base_dir, ..)` parameter | `cwd` |
| `for_base(dir)` | `for_cwd(dir)` |
| `for_trusted_external_base(dir)` | `for_trusted_external_cwd(dir)` |
| `request_base_dir()` (and field) | `request_cwd()` |
| `is_trusted_external_authoring_base()` (and field `trusted_external_authoring_base`) | `is_trusted_external_authoring_cwd()` / `trusted_external_authoring_cwd` |
| `DetailedResolution::base_dir()` (and field) | `cwd()` |

Decisions on the names the plan's table omitted (raised in Phase 1):

- `is_trusted_external_authoring_base` → `is_trusted_external_authoring_cwd`
  (it names the authoring cwd, per the Phase 1 recommendation).
- internal `ResolutionContext::from_base` → `from_cwd`. The internal resolver
  context already used `cwd`; this removes the last translation layer, so
  there is one name end to end (`from_context` now reads `ctx.cwd`).
- `CompletionAnchors::base` → `cwd`; the ambient `complete_partial`'s local
  `base_abs` → `cwd_abs`.
- parameter names: `FileReference::resolve_from(cwd)` and
  `complete_partial(token, cwd)` (cosmetic; signatures unchanged).
- `FileReference::resolve_relative(base: Option<&Path>)` keeps its `base`
  parameter: it is the directory the returned path is relative *to*, not the
  context's cwd. Its local `base_dir` became `from_dir`, matching the
  `RelativePath { from, .. }` error field it feeds.
- **Kept as is:** `LaunchMagicScope::request_dir` (a request directory, per
  the plan), `RepositoryScopeCatalog::scope_for(base)` (a generic lookup
  directory), `resolve::diff_paths(target, base)` (generic), and the public
  enum variant `CandidatePlanOrder::AuthoringBaseFirst` with
  `RootProvenance::Source`'s "authoring base" wording. Renaming that variant
  is outside the plan's table and would ripple into Darkmatter. Phase 3 should
  decide whether "authoring base" prose collides with the new tree-root
  `base_dir` (see `message_to_agent`).

Doc comments touched by the rename were updated in the same edit: every
"base"/"base directory" that meant the context's working directory now says
`cwd` / "working directory" (context struct notes, derivation docs,
`validate`, `resolve_from`, `resolve_in_context`, `complete_partial*`,
`FileReferenceKind`, `RootProvenance`, `PartialCompletionForm`,
`implicit_relative_roots`, `build_anchoring_candidates`, `completion_roots`).

Tests: only `resolution_context.rs`, `repository_scope_catalog.rs`, and
`magic_local_roots.rs` named the renamed API. Their diff is rename-only
(method calls, one helper parameter, and one test function name
`..._laundered_via_for_cwd_or_trusted_external_derivation`). Constructor
argument audit: every `new(..)` / `from_snapshot(..)` argument in the 7 L1
files is a document directory, as Phase 1 recorded; no argument changed.
Local test variables named `base` were left alone (not API; renaming them
would make the diff no longer rename-only in spirit).

Docs updated for the rename: `biscuit-file/docs/topics/file-references.md`
("Base directory" glossary → "Working directory (`cwd`)", the kinds table,
explicit/implicit relative sections, the context example, the derivation
list, the trust-boundary section, both method tables, the error table, and
the `DetailedResolution` accessor list), `biscuit-file/docs/tech-spec/file-reference-struct.md`,
and the skill reference `.claude/skills/biscuit-file/references/file-references.md`.
The topic page's Rust snippets are not compiled by any test (checked:
no `include_str!` of the page); they were reviewed by hand.

Out-of-workspace hit: `content-policy/features/2026-09-28-content-policy/spikes/filechanged-paths`
(a spike, not a workspace member) calls only `from_snapshot`, whose
signature is unchanged. Left untouched.

**Neutrality check (biscuit-file, macOS):** `just test` 860 run, 860 passed,
0 skipped, plus 6 doctests passed (baseline: identical). `just test-l2` is
the "not applicable" stub (as at baseline). `just lint` clean.

### Wave 2: consumers

Two subagents, one for Darkmatter and one for Claudine plus messenger. Each
changed only the call sites the compiler flagged whose receiver is a
biscuit-file `FileResolutionContext` / `DetailedResolution`.

- **Darkmatter** (13 files under `darkmatter/lib/src/markdown/`): `for_base`
  → `for_cwd`, `for_trusted_external_base` → `for_trusted_external_cwd`,
  `base_dir()` → `cwd()`, `request_base_dir()` → `request_cwd()`,
  `is_trusted_external_authoring_base()` → `is_trusted_external_authoring_cwd()`.
  Darkmatter's own `base_dir` fields (expression `ResolutionContext.base_dir`,
  `FileReferenceDiagnostic`, etc.) are untouched, per the plan (Phase 4).
  `encode_file_resolution_context` (options.rs) encodes the same values in
  the same positions, so the context cache key is unchanged.
  `darkmatter/cli` and `darkmatter/docs/topics/magic-paths.md` needed nothing
  (constructor signatures are unchanged; the doc example uses `new(&launch_dir)`).
- **Claudine** (7 one-line edits): `caller.origin.base_dir()` → `cwd()` in
  `composition/error/render/mod.rs` (the serialized JSON key
  `detail["base_dir"]` is unchanged: it is output, and changing it is a
  Phase 7 question), `for_base` → `for_cwd` in `lifecycle/executor.rs`,
  `DetailedResolution::base_dir()` → `cwd()` in `harness/error.rs` (Claudine's
  own `ResolutionDetail::base_dir` projection keeps its name; it is a
  documented diagnostic field), plus `schema/supplied.rs`, the CLI's
  `schema_interactive/supplied.rs`, and two tests.
- **messenger**: no change needed (`research/load.rs` only calls `new(root)`).
- **claudine/rendezvous, claudine/gen, research, worktree**: no change.
  Rendezvous does not depend on `biscuit-file`; `claudine/gen` only calls
  `new(area)`; research and worktree own unrelated `base_dir` identifiers.

Constructor audit: every `new(..)` / `from_snapshot(..)` / `for_cwd(..)`
argument in the Phase 1 table was rechecked; all are working directories, so
none changed. Notes for Phase 3/4 (no behavior change now):

- `darkmatter/lib/src/markdown/compose/schema_validation.rs` ~776:
  `for_trusted_external_cwd(fallback)` is given `options.file_ref_fallback_dir`,
  a launch-area fallback directory, not a document directory. It is still the
  "where `./` starts" value, so `cwd` is the right name.
- `darkmatter/lib/src/markdown/schemas/detect.rs:105`:
  `request_context.for_cwd(request_context.cwd())` re-derives a context at its
  own cwd (clears `source_path`, re-selects scopes). Phase 3 must make sure
  `for_cwd` keeps the tree root so this self-derivation stays a no-op on it.
- Darkmatter's `expression/functions/mod.rs:1523` and
  `schema_validation.rs` (`NoMatch.resolved_from`) copy `ctx.cwd()` into
  Darkmatter fields still named `base_dir` / `resolved_from` (Phase 4).

### Checkpoint 2

- `cargo check --workspace --all-targets` exit 0 (macOS).
- Repo-wide search for `for_trusted_external_base`, `.for_base(`,
  `request_base_dir`, `is_trusted_external_authoring_base`, and
  `ResolutionContext::from_base` in Rust and Markdown, excluding feature/fix
  snapshot directories and `target/`: no hits. `FileResolutionContext::base_dir`
  and `DetailedResolution::base_dir` no longer exist, so the compiler proves
  there are no remaining calls to them. Remaining `.base_dir()` calls belong
  to Claudine's `SourceContext` / `ResolutionDetail` and sniff's builders.
- Constructor audit table (Phase 1): every row rechecked, no argument changed
  meaning.

| Area | `just test` (L1) | `just test-l2` | `just lint` | vs baseline |
| ---- | ---------------- | -------------- | ----------- | ----------- |
| biscuit-file | 860 run, 860 passed, 0 skipped; 6 doctests passed | not applicable (stub) | clean | identical |
| darkmatter | 8726 run, 8726 passed, 12 skipped | not rerun | clean | identical |
| claudine (`--no-fail-fast`) | 8071 run, 8070 passed, 1 failed, 9 skipped | not rerun | clean | identical; the failure is the pre-existing, host-dependent `compose_magic_does_not_emit_a_nested_file_without_its_scope` |
| messenger | 688 run, 688 passed, 2 skipped | not rerun | clean | no baseline recorded in Phase 1; no source change |

Darkmatter and Claudine L2 were not rerun: the edits are method renames that
the compiler checks, L1 counts are identical, and no L2 test reads a renamed
name differently. Phase 3 changes behavior and should rerun L2.

**Requirement-to-test mapping.** Phase 2 changes no behavior, so it adds no
tests; its requirement is "the existing suites pass unchanged except for
renamed identifiers".

| Requirement | Evidence |
| ----------- | -------- |
| `cwd()` returns what `base_dir()` returned | `resolution_context::*` assertions at the former `base_dir()` sites (now `cwd()`), plus `for_source` derivation test (`child.cwd()`) |
| `for_cwd` / `for_trusted_external_cwd` keep derivation and containment semantics | `resolution_context::invalid_request_root_cannot_be_laundered_via_for_cwd_or_trusted_external_derivation`, `repository_scope_catalog` trusted-external tests |
| `from_snapshot(cwd, ..)` unchanged | `magic_local_roots` synthetic snapshot cases |
| internal `from_cwd` (ambient `resolve_from`) unchanged | `context::tests::from_cwd_absolute_path_is_preserved` / `from_cwd_relative_path_is_joined_to_ambient_cwd` (in-module unit tests, renamed with the function) |
| consumers unchanged | Darkmatter and Claudine L1 counts identical to baseline |

No test was added, removed, or retiered, so tier placement is unchanged.

**Other operating systems.** Not cross-checked. The change is a pure rename:
after it, the old names occur nowhere in the repository's Rust source, so no
`#[cfg(windows)]` or Linux-only block can still call a removed name, and no
path handling changed. CI's Linux leg and the post-merge Windows leg cover
compilation.

## Phase 3

Phase 3 reintroduces `base_dir` on `FileResolutionContext` as the **tree
root**, with an origin, and enforces it as a boundary on relative references.
Rulings R2-R7 were applied as recorded (provisional, `yolo: true`).

### Wave 1: model (`context.rs`, `error.rs`)

- `pub enum BaseDirOrigin { Repository, Explicit, Vault, Home, Environment { name }, Fallback }`
  (R5), re-exported from `file_reference` and the crate root. Accessors:
  `base_dir()`, `base_dir_origin()`, `base_dir_is_boundary()` (false only for
  `Fallback`), `external_relative_allowed()`. Builders: `with_base_dir(dir)`,
  `allow_external_relative()` (R4).
- Selection (`select_tree`): repository root (direct or catalog-selected) >
  explicit > deepest containing vault (configured roots, then captured
  `VAULT` paths; first wins a depth tie) > opening anchor > fallback to `cwd`.
  Builders that change a selection input (`with_repository_root`,
  `with_repository_scope_catalog`, `with_base_dir`, `add_vault`, `with_env`)
  reselect. Constructors select too, so a captured `VAULT` containing `cwd`
  already makes a vault tree. `new` now delegates to `from_snapshot`.
- `repository_root()` is `Some` exactly when the selected tree is a repository,
  except in a context that `validate()` rejects (see the catalog note below).
- Errors (R2): `CwdOutsideBaseDir { base_dir, cwd }`,
  `RelativeTreeEscape { base_dir, candidate, reference }`, and one the rulings
  did not name: **`BaseDirNotRepositoryRoot { base_dir, repository_root }`**
  for the spec's "`with_base_dir` inside a repository is `InvalidConfiguration`".
  The spec's `InvalidConfiguration` is a `PortablePath` error (Phase 6); the
  context has no configuration-error variant, and `with_base_dir` is an
  infallible builder, so the conflict surfaces from `validate()` as a typed
  variant. Classification: the two context errors are `MissingContext` (like
  `RepositoryRootNotContainingSource`); `RelativeTreeEscape` is
  `InvalidReference` (like `RepositoryEscape`).
- `validate()`: explicit-vs-repository conflict, then the request `cwd`
  against `request_tree`, then (unless trusted external) `cwd` against `tree`.
  A repository tree reports `RepositoryRootNotContainingSource`; any other
  boundary tree `CwdOutsideBaseDir`; a fallback tree contains everything.
  Lexical containment (R6), unchanged from before.
- **Request tree.** The context now stores `request_tree`, kept equal to `tree`
  by builders until the first derivation and frozen afterwards. This is how a
  trusted external derivation "validates the original request
  independently" after it has dropped the source repository. Builders on a
  derived context reselect only the document's tree.
- **Derivation** (one private `derive` behind every `for_*`):
  - normal (`for_source`, `for_cwd`, `for_source_reference`): a boundary tree
    is kept unchanged even if the new `cwd` leaves it, so `validate()` reports
    the escape; a fallback tree has nothing to keep, so a tree is selected for
    the new `cwd` (catalog repository, containing vault, or the opening
    anchor);
  - trusted (`for_trusted_external_source`, `for_trusted_external_cwd`,
    `for_trusted_external_source_reference` (R3)): if the new `cwd` is still
    inside the current boundary tree it behaves like a normal derivation
    (keeps the repository; Darkmatter's
    `for_trusted_external_cwd(request_cwd())` relies on this). Otherwise it
    drops the explicit root, recomputes scopes from the catalog or clears
    repository/package/package-area, and selects a new tree. It never
    discovers a repository. The launch `@` scope and the opt-in are copied
    unchanged.
- **Opening anchor** (`for_source_reference`): `~` gives the captured home;
  a leading `{{VAR}}` (only parses as implicit relative) gives the captured
  value. It must be absolute on this host and lexically contain the resolved
  source (R6), else it supplies nothing. It ranks after a containing vault and
  never replaces a tree that already contains the document.

### Wave 2: boundary enforcement (`resolve.rs`)

- One `Boundary<'_>` enum (`Repository { sigil, root }` / `Tree { base_dir }`)
  and one `validate_lexical` / `validate_containment` pair replace
  `validate_repository_lexical` / the body of `validate_repository_containment`
  (kept as a thin wrapper for `FileReference::validate_repository_candidate`).
  Only the escape error differs, so there is no second containment check.
- The internal `ResolutionContext` gains `relative_boundary: Option<PathBuf>`,
  `Some(base_dir)` only for a boundary tree without the opt-in. Ambient
  contexts set `None`.
- Seams: `build_candidates` (lexical check of **every** relative candidate
  before probing, which is what `candidate_plan` sees), `build_search_roots`
  (recursive: each root joined with the payload), `resolve_direct_core` and
  `resolve_recursive_core` (real-landing check per candidate / root / match,
  via `candidate_boundary`, which picks the repository rule for `&`/`^` and the
  tree rule for effective `ExplicitRelative`/`ImplicitRelative`), and
  `expand_completion` (implicit-relative roots; `CompletionAnchors` carries
  `relative_boundary`). Effective `Absolute` (including an absolute
  `{{VAR}}` expansion) is never checked.
- `deepest_existing_ancestor` now treats `NotADirectory` like `NotFound`.
  Without that, a candidate beneath a regular file (`blocker/x.md`) failed in
  the new containment step instead of in the probe, which broke
  `detailed_resolution::io_probe_failure_stops_with_typed_error_identifying_candidate`.
- A **tree** root that does not exist skips the canonical step (the lexical
  result stands). Nothing under a missing root can exist, so this loses no
  protection. It keeps synthetic-path contexts and a dropped `TempDir` from
  turning every relative miss into an `Io` error. The repository (`&`/`^`) rule
  is unchanged and still reports `Io` for a missing repository root.

**Decisions taken in this phase (not in the rulings):**

1. **Ambient methods carry no tree.** `resolve()`, `resolve_from()`, and
   `complete_partial()` have no `FileResolutionContext`, so they behave like a
   fallback root (no boundary). Giving them one would need a live git
   discovery for every relative reference, and the spec's tree model is
   defined on the explicit context. The topic page says so.
2. **All relative candidates are checked before any probe.** A bare reference
   whose repository-root fallback candidate leaves the tree is rejected even
   when its `cwd` candidate exists (`a/../../x.md` from `repo/docs`). This
   matches the existing `^` behavior (every root checked up front) and the
   spec's "never silently try another root", and it keeps `candidate_plan`
   and resolution in agreement. Pinned by
   `file_tree::an_escaping_repository_fallback_candidate_is_an_error_not_a_skipped_root`.
3. **Catalog contexts are now strict on normal derivation.** Before, a
   catalog-backed context derived (`for_source`/`for_cwd`) to a `cwd` outside
   the catalog repository silently lost its repository and stayed valid. Now
   the tree is kept and `validate()` fails with
   `RepositoryRootNotContainingSource`, per "`cwd` must be inside `base_dir`
   after normal derivation". This is the root cause of three Darkmatter
   failures below.
4. **Completion errors rather than filters.** An implicit-relative completion
   token whose roots leave the tree returns `RelativeTreeEscape`, mirroring how
   `&`/`^` completion already returns `RepositoryEscape`.
5. `CandidatePlanOrder::AuthoringBaseFirst` was **not** renamed. "Authoring
   base" now collides with `base_dir`, but the variant is public, used by
   Darkmatter, and outside this phase's tasks. Phase 4 can rename it alongside
   Darkmatter's own `base_dir` → `cwd` rename.

### Wave 3: tests

New file `biscuit-file/lib/tests/l1/file_tree.rs` (declared in
`tests/l1/main.rs`; no tier marker, so it runs in L1; `just
check-tier-coverage biscuit-file` reports 0 stranded). 25 tests.

| Requirement | Test(s) |
| ----------- | ------- |
| Fallback when nothing names the tree; not a boundary | `without_any_tree_input_base_dir_is_a_fallback_to_cwd` |
| Repository is the tree root; equal explicit root accepted (both builder orders, respelled) | `repository_root_is_the_tree_root_and_an_equal_explicit_root_is_accepted` |
| Unequal explicit root inside a repository → `BaseDirNotRepositoryRoot` from `validate`, `resolve_detailed` (`MissingContext`), completion | `an_explicit_root_other_than_the_repository_root_is_a_configuration_error` |
| Explicit beats vault; explicit equal to `cwd` is still a boundary | `an_explicit_root_outranks_a_containing_vault_and_is_a_boundary_even_at_cwd` |
| Deepest vault wins; captured `VAULT` participates; configured roots win depth ties; non-containing vault supplies nothing | `the_deepest_containing_vault_wins_and_configured_roots_win_depth_ties` |
| `~` opening anchor → `Home` tree; in-tree `../` resolves; leaving home is `RelativeTreeEscape` | `a_home_anchored_opening_reference_makes_home_the_tree_root` |
| `{{NOTES}}` opening anchor → `Environment { name }` | `an_environment_anchored_opening_reference_makes_the_variable_the_tree_root` |
| Unset / relative / foreign-host / non-containing env value, and `~` without home → no tree | `unset_relative_foreign_or_non_containing_anchors_supply_no_tree_root` |
| Vault outranks the opening anchor | `a_containing_vault_outranks_the_opening_anchor` |
| Anchor never replaces a tree that contains the document | `an_anchor_in_a_link_does_not_replace_a_tree_that_already_contains_the_document` |
| `./../../x.md`, `../../x.md`, `a/../../../x.md` in a repository → `RelativeTreeEscape` with exact fields, through `resolve_in_context`, `resolve_detailed` (`InvalidReference`, no probes), `candidate_plan`; in-tree `../` still resolves | `relative_references_that_leave_a_repository_are_rejected` |
| Escaping fallback candidate is an error, not a skipped root | `an_escaping_repository_fallback_candidate_is_an_error_not_a_skipped_root` |
| Same boundary outside a repository with `with_base_dir` | `an_explicit_root_bounds_relative_references_outside_a_repository` |
| Fallback root is not a boundary; ambient `resolve_from` unaffected; completion allowed | `a_fallback_tree_root_does_not_reject_relative_references` |
| Opt-in permits escapes, copies to children, works in completion; does not excuse an invalid `cwd` or relax `&` | `the_reader_opt_in_permits_escaping_targets_and_survives_derivation` |
| Absolute `{{VAR}}` expansion is not checked | `an_absolute_environment_expansion_is_not_held_to_the_boundary` |
| `&`/`^` stay repository-only with an explicit root | `repository_sigils_stay_repository_only_with_an_explicit_root` |
| Recursive relative search cannot start outside the tree | `recursive_relative_searches_cannot_start_outside_the_tree` |
| Completion does not offer escaping relative roots; in-tree roots unchanged | `completion_does_not_offer_escaping_relative_roots` |
| Symlink/junction (R7): in-tree allowed; out-of-tree rejected for an existing target, a bare reference, a not-yet-created target, and completion; lexical plan cannot see it; opt-in and fallback permit | `the_boundary_follows_directory_links_to_where_they_land` |
| `for_source`/`for_cwd` keep tree, origin, launch scope; self-derivation is a no-op on the tree (Darkmatter `detect.rs`); leaving the tree is `CwdOutsideBaseDir` | `normal_derivation_keeps_the_tree_its_origin_and_the_launch_scope` |
| Derivation from a fallback selects a vault or follows `cwd` | `derivation_from_a_fallback_tree_selects_a_tree_for_the_new_document` |
| Trusted external drops repository/package/area, keeps launch `@`, takes the `~` anchor, drops the explicit root, accepts explicit destination state | `trusted_external_derivation_drops_source_anchors_but_keeps_the_launch_scope` |
| Trusted external never discovers a repository (destination inside a real git worktree); a catalog that contains the destination supplies it | `trusted_external_derivation_uses_a_catalog_repository_and_never_discovers_one` |
| Missing tree root is checked lexically (no `Io`) | `a_tree_root_that_does_not_exist_is_checked_lexically` |

R7: the symlink test uses directory links only: a symlink on Unix, a junction
(`mklink /J`, no privilege) on Windows. No file symlink is needed, so nothing is
skipped.

Load-bearing check: with `relative_boundary` forced to `None`, 8 or more of the
new tests failed (fail-fast stopped the run early). Reverted.

**Regression: existing tests changed for deliberate behavior changes**
(all in `tests/l1/resolution_context.rs`):

- `external_source_requires_explicit_trust`: a trusted external child no
  longer falls back to the launch repository for bare `shared.md`. It now
  asserts `repository_root() == None` and `Ok(None)`.
- `valid_request_supports_in_repo_and_explicit_trusted_external_derivations`:
  same change for the external child, plus a new assertion that an
  *in-repository* trusted derivation keeps the repository and still resolves
  `shared.md`.
- `normal_derivation_reenables_containment_after_trusted_external_derivation`:
  the external document now gets its own tree. The test uses a vault so that
  tree is a boundary, and asserts that a nested derivation inside the vault is
  valid and one outside it fails with `CwdOutsideBaseDir`. Before, it asserted
  the launch repository still bounded the external document.

No existing test relied on an in-repository `./../../outside.md` resolving.

### Checkpoint 3

| Area | `just test` (L1) | `just test-l2` | `just lint` | vs baseline |
| ---- | ---------------- | -------------- | ----------- | ----------- |
| biscuit-file (macOS) | 885 run, 885 passed; 6 doctests passed | not applicable (stub) | clean | +25 new tests |
| biscuit-file (`just cross-check --os linux`) | 822 run, 822 passed | n/a | n/a | includes all 24 `file_tree` tests at that time |
| biscuit-file (`just cross-check --os windows`, native) | 828 run, 828 passed | n/a | n/a | includes all 24 `file_tree` tests at that time (junction path) |
| claudine (`--no-fail-fast`) | 8071 run, 8070 passed, 1 failed, 9 skipped | 277 + 3 run, all passed | not rerun | identical; the failure is the pre-existing host-dependent `compose_magic_does_not_emit_a_nested_file_without_its_scope` |
| darkmatter | 8726 run, 8720 passed, **6 failed**, 12 skipped (full rerun after the final change) | 18 + 69 + 3 run, all passed | not rerun | 6 new failures (fallout, below) |

`cargo check --workspace --all-targets` is clean (no errors, no warnings).
The cross-checks ran before `a_tree_root_that_does_not_exist_is_checked_lexically`
and the missing-root change were added. That change is platform-neutral
(`ErrorKind::NotFound` from `dunce::canonicalize`), so it was not re-run
remotely.

**Claudine compile fix (in this phase).** Adding three `FileReferenceError`
variants broke Claudine's deliberately exhaustive
`file_reference_failure_slug` (`claudine/lib/src/harness/error.rs`) and its
mirror test (`harness/error/tests.rs`). Both now map `CwdOutsideBaseDir` and
`BaseDirNotRepositoryRoot` to `missing_context` (like
`RepositoryRootNotContainingSource`) and `RelativeTreeEscape` to
`permission_io` (like `RepositoryEscape`), and the test samples include all
three. Without this the workspace did not compile, so it could not wait for
Phase 7. Phase 7 may revisit the slug if Claudine wants a dedicated one.

**Darkmatter fallout (fixes belong to Phase 4; not changed here):**

| Test | Root cause | Category |
| ---- | ---------- | -------- |
| `markdown::schemas::format::tests::eager_file_validation_reuses_request_repository` | `for_trusted_external_cwd(nested_repo/docs)` outside the request repository now drops the repository; bare `spec.md` no longer falls back to the request repository root → `NoMatch` | spec: trusted external drops source anchors |
| `markdown::schemas::rewrite::tests::eager_rewrite_uses_request_repository_instead_of_rediscovering_from_child` | same (`for_trusted_external_cwd(child_base)`) | spec: trusted external drops source anchors |
| `markdown::compose::expression::functions::repository::tests::valid_misses_are_empty_strings` | `package("../../outside/…")` from `repo/docs` is now `RelativeTreeEscape` → "invalid file path" instead of an empty-string miss | spec: boundary; Phase 4 "Handle the new boundary errors" decides the mapping |
| `darkmatter::l1 expression_regression::regression_basename_in_interpolation` | a document with no path gets `for_cwd(".")` (a relative cwd) from a catalog-backed request; `.` is outside the repository tree → `RepositoryRootNotContainingSource` | decision 3 (catalog strictness) exposing a relative-cwd derivation in Darkmatter |
| `darkmatter::l1 expression_regression::regression_page_block_with_is_indexed_file` | same | same |
| `darkmatter-cli::l1 compose_transclusion::test_compose_link_transcluded_child` | `transclusion/resolver.rs` canonicalizes the child path (`/private/var/…` on macOS) and derives `for_source(canonical)` from a request whose tree is spelled `/var/…`; the derived cwd is lexically outside the tree, so link normalization fails and keeps `./sibling.md`. Reproduced by hand: `md compose` from the `/var` spelling keeps the link, from the `pwd -P` spelling it normalizes | decision 3; Darkmatter mixes canonical and lexical spellings |

A seventh failure from the first run,
`functions::repository::tests::uncaptured_repository_observation_is_fatal`,
was caused by its fixture dropping the `TempDir` before resolving (the tree
root no longer existed). The missing-root rule above fixed it; it now passes.

### Other operating systems

The change touches path containment, so Linux and native Windows were run
with `just cross-check` (results above). Windows exercised the junction branch
of the new symlink test and the `NotADirectory` change. WSL2 was not run: the
code has no WSL-specific branch, and the nightly leg covers it.
