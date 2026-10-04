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
source_files_during_phase_4:
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/nested.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/compose/util.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/rewrite.rs
  - darkmatter/lib/tests/l1/file_tree_roots.rs
  - darkmatter/lib/tests/l1/main.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/sequence.rs
docs_updated_during_phase_4:
  - darkmatter/docs/transclusion/block-transclusion.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - claudine/docs/topics/composition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/compose.md
  - .claude/skills/os/build-hosts.md
source_files_during_phase_5:
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity/tests.rs
  - biscuit-file/lib/src/file_reference/portable/text.rs
  - biscuit-file/lib/src/file_reference/portable/text/tests.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
docs_updated_during_phase_5:
  - biscuit-file/README.md
  - biscuit-file/docs/topics/file-references.md
  - darkmatter/docs/inline/link-normalization.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
source_files_during_phase_6:
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/portable/strategy.rs
  - biscuit-file/lib/src/file_reference/portable/diagnostics.rs
  - biscuit-file/lib/src/file_reference/portable/env_anchor.rs
  - biscuit-file/lib/src/file_reference/portable/env_anchor/tests.rs
  - biscuit-file/lib/src/file_reference/portable/evaluate.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/text.rs
  - biscuit-file/lib/src/file_reference/portable/text/tests.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/tests/l1/portable_path/mod.rs
  - biscuit-file/lib/tests/l1/portable_path/configuration.rs
  - biscuit-file/lib/tests/l1/portable_path/environment.rs
  - biscuit-file/lib/tests/l1/portable_path/inputs.rs
  - biscuit-file/lib/tests/l1/portable_path/platform.rs
  - biscuit-file/lib/tests/l1/portable_path/properties.rs
  - biscuit-file/lib/tests/l1/portable_path/strategies.rs
docs_updated_during_phase_6:
  - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
  - .claude/skills/os/windows.md
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/util.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/type_tests.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
docs_updated_during_phase_7:
  - darkmatter/docs/inline/link-normalization.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/compose.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/os/windows.md
source_files_during_phase_8:
  - biscuit-file/lib/src/file_reference/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/operations.rs
docs_updated_during_phase_8:
  - biscuit-file/README.md
  - biscuit-file/docs/topics/file-references.md
  - darkmatter/lib/README.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/composition/frontmatter-in-pipelining.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/architecture.md
source_code:
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
  - biscuit-file/lib/src/file_reference/error.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/tests/l1/file_tree.rs
  - biscuit-file/lib/tests/l1/main.rs
  - claudine/lib/src/harness/error/tests.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/nested.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/tests/l1/file_tree_roots.rs
  - darkmatter/lib/tests/l1/main.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/sequence.rs
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity/tests.rs
  - biscuit-file/lib/src/file_reference/portable/text.rs
  - biscuit-file/lib/src/file_reference/portable/text/tests.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - biscuit-file/lib/src/file_reference/portable/strategy.rs
  - biscuit-file/lib/src/file_reference/portable/diagnostics.rs
  - biscuit-file/lib/src/file_reference/portable/env_anchor.rs
  - biscuit-file/lib/src/file_reference/portable/env_anchor/tests.rs
  - biscuit-file/lib/src/file_reference/portable/evaluate.rs
  - biscuit-file/lib/tests/l1/portable_path/mod.rs
  - biscuit-file/lib/tests/l1/portable_path/configuration.rs
  - biscuit-file/lib/tests/l1/portable_path/environment.rs
  - biscuit-file/lib/tests/l1/portable_path/inputs.rs
  - biscuit-file/lib/tests/l1/portable_path/platform.rs
  - biscuit-file/lib/tests/l1/portable_path/properties.rs
  - biscuit-file/lib/tests/l1/portable_path/strategies.rs
  - darkmatter/lib/src/markdown/compose/type_tests.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
  - darkmatter/lib/src/markdown/compose/pipeline/operations.rs
documentation:
  - biscuit-file/docs/topics/file-references.md
  - biscuit-file/docs/tech-spec/file-reference-struct.md
  - darkmatter/docs/transclusion/block-transclusion.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - claudine/docs/topics/composition.md
  - biscuit-file/README.md
  - darkmatter/docs/inline/link-normalization.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/lib/README.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/composition/frontmatter-in-pipelining.md
completed_phase: 8
implemented: true
packages:
  - biscuit-file
  - darkmatter
  - darkmatter-cli
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

## Phase 4

Darkmatter's expression `ResolutionContext` now uses the same two terms as
`FileResolutionContext`, documents derive their context from the reference
that opened them, and the six Phase 3 fallout failures are resolved.

**R8:** `2026-09-30-file-refs-use-magic` has **not** landed (its spec is still
`draft-spec`; no implementation commits). Phase 4 builds on the current
vocabulary. Whichever lands second must keep `cwd` / `base_dir()` as defined
here and must not reintroduce `ResolutionContext.base_dir` as the document
directory.

### Wave 1: rename, provenance, errors

**Step 1, rename (behavior-neutral).** `ResolutionContext.base_dir` →
`cwd`. The compiler found 36 field uses; every one read it as the document
directory (audited by hand). Every `ResolutionContext::new(..)` argument in
Darkmatter and Claudine (about 90, mostly tests) is a document directory, so
`new(cwd)` keeps its positional meaning. The same rename applied to:

- `FileReferenceDiagnostic.base_dir` → `cwd` (it was filled from the
  document directory). Claudine's JSON detail **keeps the published key
  `base_dir`** (`composition/error/render/mod.rs`, now with a comment). Renaming
  a wire key is a catalog change; left for Phase 7 or later if wanted.
- parameter and local names in `resolve_ctx.rs` (`resolve_document_file_ref*`,
  `resolve_document_directory`), `path_projection.rs` (test
  `base_dir_relative_outside_repo` → `cwd_relative_outside_repo`, rename only),
  `util.rs::document_resolution_context`, `link_resolve.rs`, and the
  transclusion resolver.

**Step 2, tree root.** `ResolutionContext::base_dir()` and
`base_dir_origin()` read the tree root from the document's
`FileResolutionContext` (`file_context()`); Darkmatter does not re-derive the
rules. `repository_root` stays as the repository root, which is not always the
tree root. The three helpers that derived a document context
(`resolve_document_file_ref`, `_shape`, `resolve_document_directory`) now
share one `document_file_context`. Only the first had the "snapshot already
at this `cwd` → use it as is" shortcut; all three now have it, because
`for_cwd` drops the source path and a trusted-external derivation.

**`for_source_reference` adoption.**

- New crate-private `SourceOpening { reference, resolved }` and the single
  derivation rule `source_file_context(snapshot, path, derivation, opening)`
  in `context/options.rs`. Every Darkmatter surface that derives a file
  source's context goes through it: `ComposeOptions::source_file_resolution_context`
  (expression and frontmatter contexts), the transclusion resolver,
  `reference::resolve_transclusion_target`, and `link_resolve`.
- `ComposeSource::File` **stays canonical**; it is the source's identity for
  cycle detection, caches, and the pre-flight graph. `SourceOpening.resolved`
  is the lexical path the parent's context produced. This is the fix for
  `test_compose_link_transcluded_child`: the canonical `/private/var/…` child
  no longer derives its context outside the `/var/…` tree. (Switching the
  child path itself to lexical would have touched every canonical-identity
  comparison in pre-flight; rejected.)
- Plumbing: `ResolvedTarget::File` and `PreflightResolvedTarget::File` gain a
  `resolved` field (both are public enums; `PreflightResolvedTarget::File` is
  now a struct variant). `PreparedTransclusion::Markdown` carries `opening`;
  `markdown_child_options` and `with_accepted_source_file` take
  `Option<SourceOpening>`; `RootSource` is now a 3-tuple. Remote children and
  `with_source_file` / `with_source_url` clear the opening. The reference
  graph passes an opening for directive, prologue, and epilogue children;
  TOC-linking and the validation re-entry points have no opening reference
  and pass `None` (they derive from the path, as before).
- The expression context's `cwd` now comes from the derived file context when
  there is one, so the expression helpers and the file context spell it the
  same way.
- The child compose-cache key includes an opening hash: two directives
  reaching the same file through `~/a.md` and `./a.md` can have different
  tree roots.
- `encode_file_resolution_context` (graph and compose-cache identity) now
  encodes `base_dir`, `base_dir_origin`, and `external_relative_allowed`.
  Phase 3 added them without updating the identity, so two snapshots that
  differed only in `with_base_dir` shared a cache entry. `source_opening` is
  classified and encoded in both products too.
- **CLI root document:** `md compose` parses its input argument as a
  `FileReference` and uses `for_source_reference` /
  `for_trusted_external_source_reference`, so a quoted `"~/x.md"` argument can
  supply the tree root.
- **Pathless sources:** a string/stdin/URL source now uses the request
  snapshot unchanged instead of `snapshot.for_cwd(".")`. The relative `.`
  depended on the process directory and is lexically outside a catalog
  repository, which caused both `expression_regression` failures. Same change
  in the transclusion resolver's non-file branch.

**Boundary errors.** `FileRefFailure::classify` maps `RelativeTreeEscape`,
`CwdOutsideBaseDir`, and `BaseDirNotRepositoryRoot` explicitly to `NotFound`,
the kind their repository counterparts (`RepositoryEscape`,
`RepositoryRootNotContainingSource`) already get through the catch-all
("understood, but no usable path"). The typed cause stays in
`FileReferenceDiagnostic.source`. No new wire slug: the Claudine catalog locks
the `kind` slugs, and a dedicated `outside_tree` kind is a later decision.
Transclusion surfaces the typed `TransclusionError::FileReference(RelativeTreeEscape)`.

### Wave 2: tests and fallout

| Requirement | Test(s) | Where |
| ----------- | ------- | ----- |
| `::file ~/Downloads/a.md` → tree root `~`; in-tree `../b.md` composes; `../../outside.md` is `RelativeTreeEscape { base_dir: home }`; lenient mode shows a notice and warning and never reads the outside file | `file_tree_roots::a_home_anchored_document_resolves_within_home_and_cannot_leave_it` | darkmatter `tests/l1/file_tree_roots.rs` (new, declared in `main.rs`) |
| `::file "{{NOTES}}/inbox/a.md"` → tree root `$NOTES`, same in/out cases | `file_tree_roots::an_environment_anchored_document_resolves_within_the_variable_and_cannot_leave_it` | same |
| Control: same documents opened by absolute path have a fallback root, escape resolves | `file_tree_roots::the_same_document_opened_without_an_anchor_has_no_boundary` | same |
| Anchor never replaces a containing repository tree | `file_tree_roots::an_anchor_inside_the_repository_keeps_the_repository_as_the_tree` | same |
| Repository boundary for `./../../x`, `../../x`, `sub/../../../x`; in-tree `../shared.md` composes | `file_tree_roots::a_relative_link_that_leaves_the_repository_is_an_error` | same |
| Normal `md compose` path, `{{{NOTES}}}` authoring form: in-tree composes; escape fails with the boundary message and never prints the outside file | `compose_transclusion::compose_env_anchored_child_is_bounded_by_the_variable` | darkmatter-cli `tests/l1/compose_transclusion.rs` |
| `ResolutionContext::base_dir()` / `base_dir_origin()`: fallback, repository, `~` anchor | `resolve_ctx::tests::base_dir_is_the_tree_root_of_the_document_context` | darkmatter unit |
| Tree root, opt-in, and opening participate in graph and compose-cache identity | `options::tests::file_tree_and_source_opening_participate_in_graph_and_cache_identity` | darkmatter unit |
| `package()` / `package_area()` with an escaping `where` is a typed error, not a miss | `repository::tests::references_leaving_the_repository_tree_are_errors_not_misses` (the `../../outside/...` case moved out of `valid_misses_are_empty_strings`) | darkmatter unit |
| Claudine: prompt opened as `~/.claudine/prompts/x.md` gets `~` as tree root; in-tree resolves; escape is `RelativeTreeEscape`; `derive_source` (no anchor) is a fallback | `invocation_context::tests::an_external_prompt_opened_through_home_takes_home_as_its_tree_root` | claudine unit (L1) |

Load-bearing checks (each reverted afterwards):

- `source_file_context` ignoring the opening: 4 of the 5 `file_tree_roots`
  tests fail; the no-anchor control still passes.
- the two new identity encodings short-circuited: the identity test fails.

**Fallout tests changed for deliberate behavior** (spec: an external
document's repository anchors are dropped; the launch repository stays
reachable only through the launch `@` scope):

- `schemas::format::tests::eager_file_validation_reuses_request_repository`
  → `eager_file_validation_of_an_external_document_keeps_only_the_launch_scope`:
  bare `spec.md` is now `NoMatch` (a decoy in the child repository proves no
  rediscovery); `@spec.md` still reaches the request repository.
- `schemas::rewrite::tests::eager_rewrite_uses_request_repository_instead_of_rediscovering_from_child`
  → `eager_rewrite_uses_the_launch_scope_instead_of_rediscovering_from_child`:
  `@spec.md` resolves to the request file, not the decoy; with no repository
  for the external document the stored value is the full portable path, not
  `spec.md`. The context uses `without_home_dir()` so Windows (temp under the
  profile) does not render `~/…`.
- `valid_misses_are_empty_strings`: escape case moved to its own test (above).
- `expression_regression` ×2 and `test_compose_link_transcluded_child`: fixed by
  code (pathless sources; `SourceOpening`), tests unchanged.

**Claudine.** New `InvocationContext::derive_composition_source(&ResolvedCompositionSource)`:
`derive_source` followed by `for_source_reference` with `original_ref` (a no-op
on the tree when a repository contains the prompt). Used by `claudine compose`
(`compose/prep.rs`) and `claudine sequence` (`commands/sequence.rs`). The
proxy-handoff and task-run sites pass already-resolved paths and keep
`derive_source`. No Claudine call site was broken by the boundary beyond the
Phase 3 compile fix.

### Finding: `{{VAR}}` in a Darkmatter body directive

Darkmatter's interpolation stage evaluates `{{ … }}` before transclusion. In a
full `md compose`, `::file "{{NOTES}}/inbox/a.md"` became `/inbox/a.md`
(`NOTES` is an unknown expression name), so the environment anchor never
reached the file reference. `{{{NOTES}}}` (the documented "emit a literal
`{{ … }}`" form) works end to end and is pinned by the CLI test above. The `~`
anchor is unaffected. This matters for Phase 7: if Darkmatter's link
normalization starts *emitting* `{{VAR}}/…` links through `PortablePath`, a
later compose of that output would evaluate them as expressions. Phase 7 must
decide the emitted spelling for Darkmatter (for example the triple-brace form,
or an escape) and test a compose → recompose round trip.

### Checkpoint 4

| Area | `just test` (L1) | `just test-l2` | `just lint` |
| ---- | ---------------- | -------------- | ----------- |
| darkmatter (macOS) | 8734 run, 8734 passed, 12 skipped (Phase 3: 6 failed) | 3 + 18 + 69 run, all passed | clean (exit 0) |
| claudine (macOS) | 8072 run, 8071 passed, 1 failed, 9 skipped | 3 + 277 run, all passed | clean (exit 0; only the known macOS linker `__eh_frame` size note) |
| biscuit-file (macOS) | 885 passed, 6 doctests passed (no source change this phase) | n/a (stub) | clean |
| darkmatter (`just cross-check --os linux`, archive mode) | 7161 run, 7161 passed, 67 skipped | — | — |

The Claudine failure is the pre-existing, host-dependent
`compose_magic_does_not_emit_a_nested_file_without_its_scope` (same as the
Phase 1 and Phase 3 baselines). `cargo check --workspace --all-targets` is
clean. `just check-tier-coverage` reports 0 stranded tests for darkmatter and
claudine. The CLI test and the docs were added after the Linux run started;
the CLI test ran on macOS only.

Linux host note: the first Linux run failed before testing on stale
read-only kache links in this worktree's standing clone (373 files). Cleared
with the command recorded in the `os` skill and reran; recorded there.

### Other operating systems

- **Linux** (`just cross-check darkmatter --os linux`, archive mode): 7161
  run, 7161 passed, 67 skipped.
- **Native Windows** (`just cross-check darkmatter --os windows`): 7132 run,
  7125 passed, **7 failed**, 67 skipped. All seven also fail on Windows at the
  committed Phase 3 state (`1d008f155`, run from a temporary detached worktree
  with the same tests), so none is caused by Phase 4. Whether they predate the
  feature branch was not checked; Darkmatter was not run on Windows before.
  - `schemas::format::tests::resolve_file_reference_no_match_for_missing_absolute_path`
    (`/tmp/…` is a foreign absolute path on Windows);
  - `compose::tests::schema::schema_validation_integration::schema_number_increment_survives_quoted_persistence_round_trips`
    (`dirname(spec) + '/review-…'` gives `/review-2.md`, a foreign absolute
    path on Windows);
  - five `compose::tests::lazy_roots::ambient_repository::*` tests
    (repository discovery counted twice, and `current.current_package` empty).
- The new tests (`file_tree_roots`, the identity and accessor unit tests)
  passed on Linux and Windows. The Claudine test and the CLI end-to-end test
  were added later and ran on macOS only; CI covers the other legs.
- WSL2 not run: no WSL-specific code; the nightly leg covers it.
- Remote filtersets: on the Windows leg a parenthesized nextest filterset
  breaks inside the remote `just` recipe even when calling
  `scripts/cross-check.sh` directly; plain substring filters work.

## Phase 5

Path identity (shared internals). Ran in a single agent; no subagents.

### What was built

- **`biscuit-file/lib/src/file_reference/portable/`** (R9 location). `mod.rs`
  declares `path_identity` and `text`; `PathIdentity` and `RelativeRoute` are
  re-exported from `file_reference/mod.rs` and `lib.rs` under
  `file-reference`.
- **`PathIdentity`** (public): `new`, `components`, `starts_with`,
  `strip_prefix`, `relative_from(dir) -> Option<RelativeRoute>`. Fields: root
  text, `rooted`, `leading_parents`, lossless `OsString` names.
  `RelativeRoute`: `parent_hops`, `forward`, `to_path_buf` (`.` when empty).
- **Windows grammar is a portable UTF-16 parser**
  (`portable::path_identity::windows`), compiled on every host, so every
  Windows rule has a test that runs on macOS/Linux too. It mirrors std's
  `parse_prefix` (including the first-8-units `/`→`\` normalization and the
  "`\\?\` must be spelled with backslashes" rule). A `#[cfg(windows)]` test
  (`prefix_grammar_agrees_with_the_standard_library`) pins it to std's
  `Prefix` classification on a fixture table, and
  `host_identity_matches_the_portable_parser_on_windows` checks the real
  constructor against the portable one.
- **Text seam** (`portable::text`, crate-internal):
  `render_reference(Lead, names)` and `render_absolute(path)`, returning
  `TextRejection::{Unrenderable, NoPortableSpelling, ChangesComponents,
  GrammarMismatch}`. Renders via `try_portable_string`, then (a) rejects
  non-Unicode first, (b) on Windows rejects names that change without `\\?\`
  (`survives_without_verbatim_prefix`, ported from Darkmatter and compiled
  everywhere), (c) re-reads the rendered tail as a `PathIdentity` and requires
  exactly the input names (catches Unix `\` and literal verbatim `.`/`..`),
  (d) re-parses the full text with the reference parser and requires the
  lead's `FileReferenceKind`, no `%`, and no interpolation except the
  `Env` lead's own variable. Grammar checks therefore use the parser itself,
  not a duplicated sigil list. A legacy UNC absolute path keeps its native
  spelling (spec: "keep a faithful native Windows UNC absolute spelling").
  The module carries `#[cfg_attr(not(test), expect(dead_code, …))]` until
  Phase 6 consumes it; the `expect` will fail the build once it is used,
  forcing its removal.
- **`resolve::diff_paths`** now normalizes with `normalize_components` (the
  resolver's semantics, unchanged) and computes the route with
  `PathIdentity::relative_from`. The second relative-path algorithm is gone.
- **Darkmatter `link_normalization.rs`**: `ComparisonKey`, both
  `comparison_key`s, `drive_root`, `unc_root`, `compute_relative_path`, and
  `strip_macos_private` deleted; the repo/home/env arms use `PathIdentity`.
  The same-repo arm routes from the canonical source's **parent directory**.
  `survives_namespace_removal` / `is_reserved_dos_name` stay in Darkmatter
  (they are text policy, not identity; Phase 7 replaces this stage with
  `PortablePath`).

### Decisions

- **`PathIdentity` is public.** The plan calls it "internal", but migrating
  Darkmatter onto it and deleting the private copy needs a cross-crate type.
  It is documented as a comparison key that is never rendered.
- **Audit gaps closed** (Phase 1 table):
  - `..` collapse on ordinary paths: added; never above a root; a relative
    path keeps uncancellable `..` as a separate `leading_parents` count so a
    generated hop is never confused with a name.
  - Verbatim: `.`/`..` literal, `/` not a separator; verbatim disk/UNC roots
    equated with legacy roots **regardless of whole-path length** (keeps the
    "long descendant inside a short root" property the audit required).
  - `strip_macos_private` (symlink alias equation): **dropped**. Every operand
    in `normalize_links` is canonicalized before comparison, so it was inert
    for an existing target; a missing target under `/tmp` already failed the
    canonical repository-root prefix test before the alias mattered.
  - Extension heuristic for the "from" file: **dropped**; `relative_from`
    takes a directory and Darkmatter passes the source's parent.
  - New: a UNC or verbatim path is always rooted (`\\server\share` equals
    `\\server\share\`), and a drive letter is folded for both legacy and
    verbatim spellings.
- **Resolver normalization left as is.** The Windows leg confirmed that
  `resolve::normalize_components` collapses `..` under `\\?\`
  (`normalize_components_reduces_verbatim_paths` passed on native Windows).
  The spec asks to preserve the resolver's lexical semantics and that check
  governs what is allowed, not which spelling is preferred, so the resolver's
  containment and dedupe comparisons keep `Path::starts_with` on
  `normalize_components` output. Only `diff_paths` moved onto the shared
  route. The identity is used for portability preference (Darkmatter now,
  `PortablePath` in Phase 6).
- **Behavior change (Windows only): `resolve_relative` across drives/shares**
  now returns `FileReferenceError::RelativePath` (as its docs already said)
  instead of an absolute path assembled by the old common-prefix walk.
- **Known limitation, unchanged from `ComparisonKey`:** a legacy name with a
  trailing dot or space (`C:\x.`, which Win32 reads as `C:\x`) is compared as
  written, so it equals `\\?\C:\x.` (a different file). Neither identity nor
  the old key strips Win32 trailing characters; the text seam refuses to
  render such names on Windows, so no reference is written from them.

### Requirement → test mapping

| Requirement | Test(s) | Level |
| ----------- | ------- | ----- |
| whole-component prefix (`/opt/config` vs `/opt/config-old`) | `path_identity::tests::prefix_matches_whole_components_only`, `strip_prefix_returns_the_names_below_the_base`, `rooted_and_relative_paths_never_share_a_prefix` | biscuit-file unit |
| `.`/`..` collapse, never above a root, leading `..` kept | `dot_segments_collapse_on_ordinary_paths`, `parent_segments_never_walk_above_a_root`, `relative_paths_keep_leading_parent_hops` | unit |
| relative route from a directory; `None` across roots | `routes_between_directories_and_targets`, `equal_paths_give_an_empty_route_rendered_as_dot`, `the_from_operand_is_always_a_directory`, `routes_between_relative_paths_respect_leading_hops`, `route_to_path_buf_joins_hops_and_names`, `different_drives_and_shares_are_separate_roots` | unit |
| Windows drives / drive-relative / case | `drive_letters_are_case_insensitive_and_names_are_not`, `drive_absolute_and_drive_relative_differ` | unit (portable) |
| verbatim prefix normalization, UNC, device, `\\?\Volume` | `verbatim_drive_equals_its_legacy_spelling`, `unc_spellings_of_one_share_are_equal`, `device_and_other_verbatim_prefixes_keep_their_own_text`, `long_verbatim_descendant_stays_inside_a_short_legacy_root` | unit (portable) |
| literal verbatim dot segments | `verbatim_dot_segments_are_literal_names`, `verbatim_paths_split_only_on_backslash`, `a_literal_verbatim_parent_name_is_kept_in_a_route` | unit (portable) |
| lossless encoding | `unpaired_surrogates_stay_distinct` (portable), `host_identity_keeps_unpaired_surrogates_distinct_on_windows`, `non_unicode_names_stay_distinct_on_unix` | unit |
| Unix backslash is a name character | `backslash_is_part_of_a_unix_name` | unit (unix) |
| portable parser == std on Windows | `prefix_grammar_classifies_each_windows_form`, `prefix_grammar_agrees_with_the_standard_library` (windows), `host_identity_matches_the_portable_parser_on_windows` (windows) | unit |
| `diff_paths` on shared route; cross-drive `None`; absolute operands | existing `diff_paths_*` and `diff_paths_bridges_verbatim_and_legacy_spellings` (windows), new `diff_paths_across_drives_is_none` (windows), `diff_paths_requires_absolute_operands` | resolve unit |
| text seam: spelling per lead, round-trip kind | `text::tests::each_lead_spells_its_reference_form`, `rendered_text_parses_back_as_the_intended_kind` | unit |
| text seam: leading sigil in a bare name, `./` protection | `a_leading_sigil_in_a_bare_name_is_rejected_and_dot_slash_protects_it`, `a_colon_name_is_rejected_on_every_host` | unit |
| text seam: `{{VAR}}` in a literal filename (all leads, `./` does not protect) | `interpolation_in_a_literal_name_is_rejected_under_every_lead`, `interpolation_in_an_absolute_name_is_rejected` | unit |
| text seam: changes native components | `literal_dot_names_are_rejected`, `a_unix_backslash_name_is_rejected_rather_than_split`, `names_that_change_without_a_verbatim_prefix_are_detected`, `name_length_is_measured_in_utf16_units`, `a_windows_name_that_changes_meaning_is_rejected` (windows) | unit |
| text seam: non-Unicode → `Unrenderable`, even absolute | `non_unicode_names_are_unrenderable_before_any_other_check` | unit (unix) |
| text seam: absolute spellings, UNC native, unreducible → `NoPortableSpelling` | `an_absolute_unix_path_keeps_its_spelling`, `a_relative_input_is_not_an_absolute_spelling`, `windows_absolute_spellings` (windows) | unit |
| Darkmatter on shared identity; route from the source directory | `link_normalization::tests::test_normalize_links_routes_from_the_source_directory` (new; the extensionless `README` case fails under the old heuristic), all existing `test_normalize_links_*` | darkmatter unit, through `normalize_links` |
| Darkmatter Windows identity cases | `unc_spellings_share_an_identity_but_no_portable_text` (renamed from `comparison_key_equates_legacy_and_verbatim_unc`), `safe_repo_root_contains_declined_long_verbatim_descendant`; the surrogate test moved to biscuit-file | darkmatter unit (windows) |

Load-bearing check: mutating the verbatim branch of the Windows parser fails
the two verbatim-dot tests; disabling the seam's re-read check fails the Unix
backslash and literal-dot tests.

The Input Robustness Matrix does not apply: no file format or configuration
reader was added or changed. No `level2_`/`real_` markers were added; all new
tests are in-crate unit tests run by `just test` (L1).

### Gates

- `biscuit-file`: `just test` 925 passed (plus doctests; the new
  `PathIdentity` doctest passes); `just lint` clean.
- `darkmatter`: `just test` 8736 passed, 12 skipped; `just lint` clean.
- `just cross-check biscuit-file --os windows`: 869/869 passed (after
  fixing two portable tests whose `:` names are rejected as
  `ChangesComponents` on Windows rather than `GrammarMismatch`).
- `just cross-check biscuit-file --os linux`: 863/863 passed.
- `just cross-check darkmatter --os windows`: 7132 run, 7125 passed,
  **7 failed**, 67 skipped. The seven are exactly the pre-existing failures
  listed under Phase 4 (`resolve_file_reference_no_match_for_missing_absolute_path`,
  `schema_number_increment_survives_quoted_persistence_round_trips`, five
  `lazy_roots::ambient_repository::*`). All 24 `link_normalization` tests,
  including the `#[cfg(windows)]` ones edited here, passed.
- Darkmatter on Linux and WSL2 not run: the only Darkmatter change is
  platform-neutral identity plumbing, and the Windows-specific parts live in
  biscuit-file, which passed on Linux and Windows. CI covers the remaining legs.

## Phase 6

`PortablePath` core. Ran in a single agent; no subagents. Rulings R1, R7, R9,
and R11 applied as recorded.

### What was built

All under `biscuit-file/lib/src/file_reference/portable/` (R9), re-exported
from `file_reference/mod.rs` and `lib.rs` under `file-reference`:

- **`strategy.rs`**: `PortabilityPreference` (13 variants, spec meanings),
  `IntentForms::ALL` (a struct with a private field, so narrower sets can be
  added later without breaking callers), and
  `PortabilityPreference::DEFAULT_STRATEGY` (a `const` slice). `Display` gives
  `RepoRoot(docs)`-style names.
- **`diagnostics.rs`**: `Attempt { strategy, outcome, rejected }`,
  `AttemptOutcome`, `NotApplicable`, `SpellingProblem`, `EnvAnchorProblem`,
  `Finding`, `ResolutionProblem`, `ProbeError`, `InvalidTarget`,
  `ConfigurationProblem`, `FilterProblem`, `PortablePathError`. Everything is
  `Clone`; no `io::Error` is held. `PortablePathError::attempts()` and
  `findings()` exist on every variant; `Display` is a one-line headline then
  one line per attempt.
- **`env_anchor.rs`**: `PORTABLE_ENV_VARIABLES` (public const), declaration
  parsing (trim, skip empty, `[A-Z0-9_]+`, invalid recorded once, union with
  builder names, `BTreeSet` for dedupe and name order), value eligibility
  (`Unset` / `NotAbsolute` / `ForeignAbsolute` / `NotAPrefix`, via
  `PathIdentity::strip_prefix`), deepest-first with a stable sort for name
  ties.
- **`evaluate.rs`**: `PortablePath` builders, `PortableReference`
  (`reference`, `strategy`, `attempts`, `findings`, `into_reference`,
  `AsRef<FileReference>`), and the evaluator.
- `path_identity.rs` / `mod.rs`: the Phase 5 `expect(dead_code)` markers were
  removed (the seam is now consumed).
- `text.rs`: `Lead::Relative { parent_hops: 0 }` with no names now renders
  `./` instead of `.` (spec: "a target equal to CWD renders as `./`"). The
  one seam test row was updated. The seam is crate-internal; no consumer saw
  the old spelling.

### How evaluation works

1. Settings are checked before anything else: `with_ctx` with `with_cwd` /
   `with_base_dir`, filter syntax, a path input that is not host-absolute.
2. The context: a clone of `with_ctx`, or `FileResolutionContext::new(cwd)`
   (captures home and env once) plus `find_git_root(cwd)` and `with_base_dir`.
   `validate()` errors map to `InvalidConfiguration`.
3. Portable names come from the context's env plus the builder names.
4. Preferences run in order. `AuthoredIntent` needs no target. The target is
   established lazily, at the first target preference, so an input that only
   `AuthoredIntent` decides is looked up once. A reference input is resolved
   with `resolve_detailed`: a match, or the one candidate of a
   single-candidate plan when nothing matched; otherwise `UnresolvableInput`
   with a typed finding. A non-Unicode target is `UnrenderableTarget`.
5. Each target preference renders through the `text` seam and verifies the
   text by resolving it with `resolve_detailed` in the same context (for
   `ExternalRelativePath`, with `allow_external_relative()`). The resolver
   applies the boundary and the real-landing (symlink) check, so no parallel
   check exists. `Io` during verification aborts with `ProbeFailed`.

### Decisions and departures from the spec (spec left as decided)

- **`Attempt` has a `rejected` list.** The spec's illustrative `Attempt` has
  `strategy` and `outcome` only; it also says an attempt "can contain several
  candidate rejections or environment-anchor evaluations". `rejected` holds
  the earlier candidates (a shadowed `@` spelling, each ineligible variable);
  `outcome` is the match or the last rejection.
- **`NotApplicable::RouteShape { parent_hops, names }` replaces
  `TooManyParentHops { needed }`**: one typed reason for every relative shape
  mismatch, including "too few hops". Added reasons the spec's list did not
  name: `OutsideRepository`, `InsideBaseDir`, `NoSharedRoot`,
  `FilterNotASearchRoot`, `FilterUnavailable`, `HomeUnavailable`,
  `TargetNotFile` (search form on a directory), `NoPortableVariables`,
  `NotRewritable` (URL / `%` input), `UnsafeSpelling(SpellingProblem)`,
  `Rejected(ResolutionProblem)` (boundary denial or missing anchor during
  verification), `NoCandidate` (internal fallback, unreachable in practice).
- **Added error variants** for the spec's "must cover" list:
  `ProbeFailed { target, error: ProbeError, attempts }` (I/O other than
  absence, for a path target or during verification), `CwdUnavailable`,
  `RepositoryDiscoveryFailed`. `NoStrategyMatched` and `UnresolvableInput`
  carry `findings`; `UnrenderableTarget` carries `attempts`.
- **R11** as ruled: a kept intent form's lookup failure is
  `Finding::ResolutionFailed(ResolutionProblem)`; a position form's is
  `UnresolvableInput`.
- **The `reference` in `UnresolvableInput` and `NormalizationUnsupported` is
  `Box<FileReference>`.** Unboxed, `PortablePathError` exceeds
  `clippy::result_large_err`; boxing the arm follows the precedent in
  `claudine/lib/src/error.rs`.
- **Only `NotFound` is absence.** A path through a regular file (`ENOTDIR`) is
  a probe failure, matching the resolver's own probe.
- **`ParentDir` accepts any in-tree route with at least one hop** (it is
  "up, optionally down"); it does not accept same-directory or child routes.
- **`MagicPath(Some(filter))` spells from the filter root.** The filter is
  resolved with `candidate_plan`, must equal a context `@` root, and the only
  spelling tried is from that root (`@b.md`, not `@.claudine/prompts/b.md`
  from the earlier home root). Still verified by lookup (`Shadowed` if
  another root wins). Without a filter, every containing root is tried in
  resolver order.
- **Minimal churn** compares the authored text's literal route (leading `./`,
  `..` hops, then names; anything else disqualifies) with the generated route.
  A bare input qualifies only when its match (or single candidate) has
  `RootProvenance::Source`. `./a/../foo.md` is respelled `./foo.md`.
- **A value with a trailing separator** (`/opt/config/`) is eligible: path
  identity compares whole components, so it equals `/opt/config`. The plan's
  matrix row "trailing separator … is `NotAPrefix`" was read as the
  lexical-only-prefix case (`/opt/conf`), which is `NotAPrefix`.
- **URLs and `%` inputs** record `NotApplicable(NotRewritable)` for each target
  preference, then `NormalizationUnsupported` if nothing kept them.
- **Spec example drift (not a code change).** The spec's
  `from_path(".../file-references.md")` → `&biscuit-file/...` example says
  "e.g. from its root". From the repository root, `ChildDir` precedes
  `RepoRoot` in the spec's own default strategy, so the result is
  `./biscuit-file/docs/topics/file-references.md`. The test pins both: `&…`
  from elsewhere in the repository, `./…` from the root.

### Requirement → test mapping

New tests: `biscuit-file/lib/tests/l1/portable_path/` (declared as
`mod portable_path;` in `tests/l1/main.rs`; no tier marker, so L1;
`just check-tier-coverage biscuit-file` reports 0 stranded) and
`portable/env_anchor/tests.rs` (unit).

| Requirement | Test(s) |
| ----------- | ------- |
| each relative preference's shape; `strategy()` names the match; last attempt is the match | `strategies::each_relative_preference_writes_its_own_route_shape`, `a_relative_preference_refuses_other_route_shapes` |
| `ParentDir` catch-all; R1 (absent from the default, falls through) | `strategies::parent_dir_is_the_in_tree_catch_all_and_absent_from_the_default` |
| target equal to cwd → `./`, directory → `TargetNotFile` | `strategies::a_target_equal_to_cwd_is_dot_slash_with_a_not_file_finding` |
| fallback tree: upward route is external | `strategies::a_fallback_tree_treats_any_upward_route_as_external` |
| `ExternalRelativePath` opt-in, verified with the reader opt-in, `InsideBaseDir` | `strategies::external_relative_path_is_opt_in_and_verified_with_the_reader_opt_in` |
| `RepoRoot` needs the document's repository; another checkout; `with_ctx` stays non-repository | `strategies::repo_root_needs_a_repository_containing_the_target` |
| filters are eligibility only, whole component | `strategies::filters_restrict_eligibility_but_never_add_roots` |
| default vs reordered (Claudine's list), `^` and `@` with filters | `strategies::a_reordered_strategy_prefers_searched_forms_the_default_never_writes` |
| `MagicPath` filter not a root / unavailable | `strategies::a_magic_filter_must_name_a_search_root` |
| shadowing recorded, next root tried, never skipped | `strategies::a_shadowed_spelling_is_recorded_and_the_next_root_is_tried` |
| search forms need an existing file; single-location may be missing; non-file | `strategies::search_forms_need_an_existing_file_while_single_locations_do_not` |
| home from context; `HomeUnavailable`; visible `AbsolutePath`; `NoStrategyMatched` + `attempts()` + `Display` | `strategies::home_comes_from_the_context_and_absolute_is_a_visible_fallback` |
| empty strategy | `strategies::an_empty_strategy_matches_nothing` |
| input robustness matrix (both reads, control row, one edit per cell) | `environment::the_declaration_and_value_matrix_has_one_defined_outcome_per_cell` (+ unit: `env_anchor::tests::*`) |
| deepest anchor wins, name-order ties | `environment::the_deepest_anchor_wins_and_names_break_ties` |
| `HOME` declared → `{{HOME}}` | `environment::declaring_home_writes_the_variable_instead_of_tilde` |
| `with_portable_env` accumulates and dedupes | `environment::portable_names_accumulate_across_builder_calls` |
| `md clean` table (`^/foo.md`, `../../../foo.md` → `&foo.md`, portable vs non-portable `{{VAR}}`) | `inputs::the_md_clean_table_keeps_intent_and_rewrites_position` |
| `AuthoredIntent` on a path input | `inputs::a_path_input_is_never_authored_intent` |
| kept-reference findings (unset/relative portable var, later placeholders, sigil payload, missing, directory, R11 missing home) | `inputs::a_kept_reference_still_reports_its_findings` |
| URL / `%` kept, never rewritten (`NormalizationUnsupported`) | `inputs::urls_and_recursive_searches_are_kept_or_refused_never_rewritten` |
| position of `AuthoredIntent` in the strategy | `inputs::intent_after_same_dir_lets_a_plain_link_win` |
| minimal churn, bare repository-fallback link not same-dir | `inputs::minimal_churn_keeps_a_relative_link_already_in_the_chosen_form` |
| multi-candidate miss, boundary escape, missing anchor → `UnresolvableInput` with typed finding; single candidate missing → target | `inputs::an_input_without_one_target_is_unresolvable_with_a_typed_finding` |
| probe failure (ENOTDIR) never `TargetMissing` (unix) | `inputs::a_probe_failure_is_never_a_missing_target` |
| cleanup with reader opt-in, strict output | `inputs::an_opted_in_reader_replaces_an_escaping_link_without_writing_another` |
| idempotence over a corpus; candidate equality for single-location results | `properties::every_result_is_a_fixed_point_and_names_its_target`, `cleaning_authored_links_twice_changes_nothing_the_second_time` |
| `UnresolvableInput` fails identically on retry | `properties::an_unresolvable_input_fails_the_same_way_on_retry` |
| captured-state isolation, batch reuse | `properties::a_captured_context_isolates_evaluation_from_later_changes` |
| spec examples, suffix stays with the caller | `properties::the_documented_examples_hold` |
| builder conflicts in either order; directories/targets absolute; invalid context before strategies; discovery without a context; `with_base_dir` in a repository; filter syntax | `configuration::*` (5 tests) |
| sigil names protected by `./`; `{{VAR}}` and Unix `\` names refused; non-Unicode → `UnrenderableTarget` | `platform::a_leading_sigil_in_a_file_name_is_protected_by_dot_slash`, `interpolation_syntax_in_a_file_name_has_no_reference_spelling`, `a_unix_backslash_in_a_name_is_never_split` (unix), `a_non_unicode_target_is_unrenderable_even_with_absolute_path` (unix) |
| in-tree vs out-of-tree directory links (symlink / junction, R7) | `platform::a_relative_link_must_land_inside_the_tree` |
| Windows verbatim and drive-letter case; another drive has no route | `platform::windows_spellings_of_one_directory_route_alike`, `a_target_on_another_drive_has_no_relative_route` (windows) |

UNC and distinct-share identity are covered by the Phase 5 `path_identity` and
`text` tests (`windows_absolute_spellings` keeps a UNC spelling native); the
evaluator adds no UNC-specific code. R7: only directory links are used, so
nothing is skipped.

**Load-bearing check** (each mutation reverted afterwards): disabling minimal
churn failed 2 tests; making invalid names portable failed 3; dropping the
identity comparison in verification failed the shadowing test; dropping the
in-tree check failed 8.

### Gates

- `biscuit-file` (macOS): `just test` 973 passed (was 925 + new); `just
  doctest` 25 passed; `just lint` clean; `cargo check -p biscuit-file
  --no-default-features` clean; `just test-l2` is a "not applicable" stub.
- `just cross-check biscuit-file --os linux`: 911/911 passed.
- `just cross-check biscuit-file --os windows` (native): first run 914/916.
  Both failures were fixture bugs: `root.join("repo/docs/x.md")` kept `/` in
  the native text, which is invalid under `\\?\` (os error 123) and refused by
  `mklink /J`. The fixture now joins name by name; rerun 916/916 passed. The
  trap is recorded in the `os` skill (`windows.md`, Path spelling item 10).
- WSL2 not run: no WSL-specific code; the nightly leg covers it.
- `cargo check -p darkmatter -p darkmatter-cli -p claudine -p claudine-cli
  --all-targets`: clean. Their test suites were not rerun: this phase only
  added `biscuit-file` exports and changed one crate-internal spelling that
  only `PortablePath` uses.

### Docs and skills

- `biscuit-file/docs/topics/file-references.md`: new section "Portable
  References: `PortablePath`" (strategy table, opt-in preferences, filters,
  reference inputs, verification, portable variables, builders, diagnostics;
  records R1). Phase 8 expands it into the full guide with a Mermaid diagram.
- `.claude/skills/biscuit-file/SKILL.md` (pointer + trigger words),
  `references/api.md` (new section), `references/architecture.md` (module
  row).

## Phase 7

Consumer migration. Ran in a single agent; no subagents (Darkmatter was
sequential and Claudine turned out to be the skip case).

### What was built (Darkmatter)

- **`compose/link_normalization.rs` now uses `PortablePath`.** The three
  hand-written arms (same-repo relative, `~/`, `${VAR}/`) and the Windows
  `survives_namespace_removal` / `is_reserved_dos_name` / `render_*` helpers
  are deleted. Each absolute destination in the composed root document is
  split into path + suffix, re-spelled into the context's spelling (below),
  and evaluated with `PortablePath::from_path(..).with_ctx(&ctx)
  .with_portable_env(options.portable_env)` using the default strategy.
- **Context.** New `compose::util::source_link_context(options)`: the
  snapshot's derivation for the source (`source_file_resolution_context`), or
  a context captured from a file source's directory. `link_resolve` now uses
  the same helper, so both stages read links in one context. A source with
  neither a snapshot nor a path gets no context and `PortablePath` captures
  the process directory, home, and environment itself (the old stage applied
  only `~` / env there).
- **Suffix layer** (`split_suffix`): `#fragment`, `?query`, and a trailing
  `:line` / `:line-line` are split from the parsed destination
  (`ReferenceTarget::LocalPath`) and reattached. Only a digits-only tail after
  the last colon is a line suffix, so `C:/x.md` keeps its drive colon.
- **Spelling reconciliation** (`in_context_spelling`): `link_resolve`
  canonicalizes what it writes (`/private/var/…` on macOS) while the context
  keeps the opened spelling (`/var/…`); `PortablePath` compares lexically. A
  canonical target under the canonical form of `base_dir`, `cwd`, the
  repository root, or home is re-spelled under that anchor's context spelling.
  Load-bearing: disabling it fails 9 of the 20 unit tests on macOS.
- **Error/finding mapping** (Darkmatter vocabulary is `ComposeWarning` with
  stage `link_normalization`):
  - any `PortablePathError` (including `UnresolvableInput`, `ProbeFailed`):
    keep the destination byte-identical, one warning with the error's
    headline;
  - `strategy() == AbsolutePath`: keep the destination (it already is the
    absolute path); warn only when it has no faithful portable spelling
    (`try_portable_string` declines: UNC, device, unreducible verbatim) —
    the same warn/no-warn split the old stage had;
  - `Finding::InvalidPortableVariableName`: one warning per name per document;
  - `TargetMissing` / `TargetNotFile` findings are not repeated (reference
    validation owns missing links).
- **Emitted `{{VAR}}` spelling (decision; Phase 4 asked for it).** An
  `EnvRootedPath` result is written as the interpolation literal
  `{{{VAR}}}/rest`. A composed document is Darkmatter source again; a bare
  `{{VAR}}` would be evaluated as an (unknown) expression on recompose and
  collapse to `/rest`. The literal composes to `{{VAR}}`, `link_resolve`
  expands it from the captured environment, and normalization writes the
  literal again: compose → recompose is a fixed point (tested end to end
  through the CLI and through `compose_with` with the default operation
  order). Trade-off: a reader of the composed Markdown sees three braces.
- **`ComposeOptions`**: `with_env_path_whitelist`,
  `effective_env_path_whitelist`, `default_env_path_whitelist`, and the
  `PROJECT_ROOT` / `DOCS_BASE` defaults are removed. New
  `with_portable_env(names)` (accumulates, `BTreeSet` dedupe) and
  `portable_env()`. Options identity encodes the set in sorted order, so
  declaration order and repeats no longer change the graph/cache identity
  (tested), while a different name does.
- **One-way effect (spec: intentional).** Links the old stage generated as
  `${VAR}/…` were never `FileReference` syntax and never resolved; nothing
  reads them back. Previously env-anchored targets now become relative,
  `&`, `~`, or stay absolute unless the variable is declared portable. Other
  visible output changes: a same-directory target is `./x.md` (was bare
  `x.md`), so an authored `./x.md` now round-trips unchanged; a target two or
  more levels up inside the repository is `&path` (was `../../path`; R1); the
  old "found to be an offset of the … environment variable" warning is gone.

### Decisions and departures (spec left as decided)

- **Inputs are absolute destinations only (`from_path`), not
  `from_reference`.** In compose, `link_resolve` makes every resolvable link
  absolute before transclusion, so authored intent forms (`^`, `@`, `&`,
  `{{VAR}}`) are already gone by finalization. A relative or sigil
  destination still present is one `link_resolve` could not resolve; after
  transclusion it may be a child's link sitting in the root document, so
  evaluating it against the root context could retarget it. Those are left
  alone, as before. Consequence: compose does not keep an authored `^/foo.md`
  or `@x.md` spelling; it rewrites to the strategy's spelling of the same
  file (the old stage did the same). Keeping authored intent through compose
  would need `link_resolve` to carry the authored text and the authoring
  document per edit; recorded as a possible follow-up, not built.
- **No reader opt-in was added (task "Opt-in for cleanup reads").** No
  Darkmatter cleanup reads escaping relative links: `md clean` does no link
  work (the spec scopes a link-cleaning `md clean` out), and normalization
  never reads relative input. Adding `allow_external_relative()` to compose
  would be the "silent opt-in for arbitrary inputs" the plan forbids.
- **Windows helper tests removed from Darkmatter.**
  `unc_spellings_share_an_identity_but_no_portable_text`,
  `safe_repo_root_contains_declined_long_verbatim_descendant`,
  `unsafe_components_do_not_survive_namespace_removal`,
  `non_unicode_component_does_not_survive_namespace_removal`, and
  `component_length_is_measured_in_utf16_units` tested the deleted private
  helpers; `biscuit-file`'s `path_identity` / `text`
  (`survives_without_verbatim_prefix`) tests cover the same rules. The six
  end-to-end Windows tests (repo/env/home anchor × unsafe categories, the two
  over-`MAX_PATH` success controls, the declined verbatim destination) are
  kept and adapted: the warning text is now "left exactly as authored", env
  anchors are declared with `with_portable_env`, the env result is
  `{{{PROJECT_ROOT}}}/…`, and the env/home fixtures put `cwd` beside the
  target so a relative preference cannot claim it first.

### Claudine: the skip case

Claudine has no document-link rewriting. Its only absolute→text renderers are
shell-completion insert texts (`cli/src/completion/operation_file.rs`
`format_relative_insert`, `cli/src/completion/composition/compose.rs`
repo-/home-/scope-relative inserts). They produce command-line arguments
resolved from the shell's directory, a different contract from portable
document links (bare repo-relative text, not `&`), so adopting `PortablePath`
there would change completion UX and is out of scope. Claudine never called
`with_env_path_whitelist`. Its composition goes through Darkmatter, so the
new normalization reaches Claudine output; the full Claudine L1/L2 suites
were rerun (below). No Claudine docs mention link normalization or the
removed options; nothing to update.

### Requirement → test mapping

| Requirement | Test(s) |
| ----------- | ------- |
| relative rewrite (peer, same-dir, child) | `link_normalization::tests::a_peer_directory_target_becomes_a_relative_link`, `deep_and_same_directory_targets`, `css_font_and_script_destinations_normalize`, `spaced_html_attributes_normalize`, `angle_bracket_and_quoted_destinations_keep_their_delimiters` |
| route from the document's directory; deep in-repo → `&` (R1) | `a_distant_target_in_the_repository_is_repository_rooted`, `deep_and_same_directory_targets` |
| home | `a_target_under_home_is_home_rooted`; integration `test_home_dir_interpolation` |
| declared portable variable → `{{{VAR}}}` | `a_declared_portable_variable_is_written_as_an_interpolation_literal` |
| no built-in `PROJECT_ROOT` / `DOCS_BASE` | `no_variable_is_portable_unless_declared`; CLI `test_compose_portable_env_variable_round_trips` (undeclared leg) |
| `PORTABLE_ENV_VARIABLES` from the captured env, union with the option, invalid name warned once | `the_captured_environment_declares_and_supplies_variables` |
| deepest variable wins | `the_deepest_portable_variable_wins` |
| suffix handling (`#`, `?`, `:n`, `:a-b`, combined; drive colon kept) | `fragment_query_and_line_suffixes_are_reattached`, `split_suffix_only_takes_trailing_line_numbers_after_a_colon` |
| idempotence (run twice) | `normalizing_twice_changes_nothing_the_second_time`; compose → recompose: lib `link_interpolation_integration::test_env_var_interpolation`, CLI `test_compose_portable_env_variable_round_trips` |
| preservation: relative/sigil untouched; absolute fallback silent; failure kept + warned (unix ENOTDIR); remote URLs | `relative_and_sigil_destinations_are_left_alone`, `the_absolute_fallback_keeps_the_destination_without_a_warning`, `an_evaluation_failure_keeps_the_destination_and_warns`, `remote_urls_are_not_touched` |
| canonical vs lexical spelling | `a_canonical_destination_routes_from_a_lexical_context` |
| Windows unsafe verbatim components preserved + warned; over-`MAX_PATH` still normalizes (repo/env/home) | `repo_anchor_preserves_every_unsafe_category`, `env_anchor_preserves_every_unsafe_category`, `home_anchor_preserves_every_unsafe_category`, `anchored_over_max_path_destination_still_normalizes`, `env_anchored_over_max_path_destination_still_normalizes`, `home_anchored_over_max_path_destination_still_normalizes`, `declined_absolute_destination_is_preserved_and_warned` (windows) |
| options: no built-in names, accumulate/dedupe; identity is set-shaped | `type_tests::portable_env_has_no_built_in_names_and_accumulates`; `options::…::options_identity_ignores_unordered_set_insertion_order`, `options_identity_portable_env_and_host_element_boundaries_are_injective` |
| transcluded child's link normalized relative to the root | CLI `test_compose_link_transcluded_child`, lib `test_end_to_end_link_interpolation` (unchanged, pass) |
| spaced HTML attributes through the CLI | CLI `test_compose_html_spaced_attributes` (now asserts `./other.md` round-trips and no absolute path) |

All new unit tests are in `darkmatter/lib/src/markdown/compose/link_normalization.rs`
(`#[cfg(test)] mod tests`, the lib target); the integration and CLI tests are
in the existing `tests/l1/` binaries. No tier markers, so all are L1.

**Input robustness matrix.** Darkmatter adds no parser: `with_portable_env`
forwards names and `PORTABLE_ENV_VARIABLES` is parsed by `biscuit-file`
(matrix walked in Phase 6, `environment::the_declaration_and_value_matrix_…`).
The Darkmatter-side cells that matter (declaration from the captured env,
union with the option, invalid name reported once, undeclared variable never
written) are asserted through `normalize_links` output above.

### Windows defects found by cross-check (fixed)

The first `just cross-check darkmatter --os windows` run failed 11 new tests
(plus the 7 pre-existing). Two causes:

1. **`split_suffix` cut a verbatim path at the `?` of its `\\?\` prefix**, so
   every verbatim destination read as `\\` (not absolute) and was silently
   skipped. Fixed by skipping a `\\?\` / `//?/` prefix before searching for
   `#` / `?`; four verbatim rows added to
   `split_suffix_only_takes_trailing_line_numbers_after_a_colon`.
2. **Env fixtures used canonical (verbatim) values.** `FileReference`
   interpolation concatenates text (`resolve.rs` `interpolate`), so
   `{{VAR}}/x` under `VAR=\\?\C:\…` is one component under the prefix and
   `PortablePath` correctly refuses to write it. The fixtures now store
   `to_portable_string(&path)` (the spelling a user exports). The resolver
   behavior is pre-existing and unchanged; recorded as items 11 and 12 in the
   `os` skill's `windows.md`.

### Gates

- `darkmatter` (macOS): `just test` 8743 passed, 12 skipped; `just test-l2`
  all passed (18, 69, 3 across the three runs); `just lint` clean.
- `claudine` (macOS): `just test` 8071 of 8072 passed. The one failure,
  `claudine-cli completion::composition::tests::compose_magic_does_not_emit_a_nested_file_without_its_scope`,
  is environmental and pre-existing: the test reads the real home and this
  host has `~/.claudine/prompts/plan.md`; it passes with `HOME` pointed at an
  empty directory. Not caused by this phase (completion does not use
  normalization or `ComposeOptions`). `just test-l2` 277 + 3 passed; `just
  lint` clean (only the known `__eh_frame` linker note).
- `biscuit-file` (macOS): `just test` 973 passed; `just lint` clean; `just
  test-l2` is a "not applicable" stub. No `biscuit-file` source changed.
- Load-bearing check: disabling `in_context_spelling` failed 9 of 20
  normalization unit tests (reverted).

### Other operating systems

- Linux (`build-linux`, `just cross-check`): `darkmatter` 7169 passed;
  `darkmatter-cli` 797 passed (both before the Windows fix; the fix only
  touches a `\\?\` prefix branch and fixture env spelling, no-ops off
  Windows).
- Native Windows (`build-win-native`): full `darkmatter` run 7115 passed, 18
  failed = the 7 pre-existing (`lazy_roots::ambient_repository::*` ×5,
  `resolve_file_reference_no_match_for_missing_absolute_path`,
  `schema_number_increment_survives_quoted_persistence_round_trips`) + 11
  new, all fixed above. After the fix: `darkmatter` filtered to
  `link_normalization link_interpolation` 30/30 passed (includes the 7
  Windows-only tests); `darkmatter-cli` filtered to `compose_transclusion`
  11/11 passed.
- WSL2 not run: no WSL-specific code; the nightly leg covers it.

### Docs and skills

- `darkmatter/docs/inline/link-normalization.md`: rewritten for the
  `PortablePath` contract (examples per rule, Mermaid flow, suffixes,
  declaring portable variables, why `{{{VAR}}}`, warnings). Updated now
  rather than in Phase 8 because the old page described removed behavior
  (Drift Maintenance); Phase 8 may still polish it.
- `darkmatter/docs/darkmatter-compose-pipeline.md`: one-line summary of the
  stage (no `${ENV}`).
- `.claude/skills/darkmatter/compose.md`: Link Normalization bullet.
- `.claude/skills/biscuit-file/references/api.md`: lexical-comparison and
  verbatim-value notes, consumer pattern.
- `.claude/skills/os/windows.md`: Path spelling items 11 and 12.

## Phase 8

Documentation, skills, and final validation. Ran in a single agent; no
subagents (the docs work was small enough that parallel agents would have
cost more than they saved). No behavior changed in this phase: the two source
edits are doc comments only.

### Docs and skills

- `biscuit-file/docs/topics/file-references.md`: the `cwd` / `base_dir`,
  origin, precedence, boundary, fallback, reader opt-in, and symlink sections
  (including "a rule about references, not a sandbox") were already current
  from Phases 2–6 and were reviewed, not rewritten. Added a
  "How a reference is chosen" Mermaid flow for `PortablePath` evaluation and
  verification, and a diagnostics example (walking `attempts()`). The example
  was compiled and run as a scratch example (deleted afterwards): with
  `HOME=/Users/me` it prints `~/notes/x.md via HomeDir` and the skipped
  preferences, matching the page's claim (R1: no `../../x.md`).
- `biscuit-file/README.md`: functional-overview bullet and a short
  "Portable References" section (two-term model, boundary, opt-in, one
  `PortablePath` example, links into the topic page).
- Darkmatter: `lib/README.md` still described link normalization as writing
  `${ENV}` (drift, fixed); `docs/topics/schemas/definition.md` said imports
  resolve through `resolve_from(base_dir)`, but the code resolves in a context
  whose `cwd` is the schema file's directory (`schemas/resolve.rs`
  `resolve_file_reference_in_context`); wording fixed;
  `docs/composition/frontmatter-in-pipelining.md` linked to
  `../operations/link-normalization.md`, which does not exist (fixed to
  `../inline/link-normalization.md`, as Phase 1 suspected).
  `docs/inline/link-normalization.md` (rewritten in Phase 7) was reviewed and
  left as is.
- Claudine: no doc cites renamed names or removed options.
  `claudine/docs/topics/file-referencing.md` uses `${HOME}` / `${CWD}` in
  prose, but it is a design-intent document that forbids drift correction
  without Ken's approval; left untouched.
- Skills: `.claude/skills/biscuit-file/SKILL.md` (the source-file table now
  lists `FileResolutionContext`, `BaseDirOrigin`, `LaunchMagicScope`, and the
  `portable/` module instead of "ResolutionContext (internal)"; a pointer line
  for the two-term model) and `references/architecture.md` (context types on
  the `file_reference` row). `references/api.md` and `file-references.md` were
  already current. `.claude/skills/darkmatter/compose.md` keeps one sentence
  naming the removed `with_env_path_whitelist` / `PROJECT_ROOT` / `DOCS_BASE`
  on purpose: it tells an agent reading an older spec that the API is gone.
  `.claude/skills/claudine/**` has no relevant mentions.
- Dependencies: no feature commit touched a `Cargo.toml` or `Cargo.lock`
  (checked commit by commit; the branch-wide manifest diff comes from merged
  branches). `docs/dependencies.md` unchanged.

### Comment drift found and fixed (code treated as correct)

- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs`
  `ComposeOperation::LinkNormalization` still said "`${VAR}` for whitelisted
  environment-relative paths"; now describes the `PortablePath` forms.
- `biscuit-file/lib/src/file_reference/mod.rs`:
  `CandidatePlanOrder::AuthoringBaseFirst` and `RootProvenance::LocalRoot`
  said "authoring base", which now reads like the tree root `base_dir`; both
  now say the authoring `cwd` (`RootProvenance::Source`). The variant name
  itself is unchanged (still open, see below).
- Other "base directory" prose in Darkmatter and Claudine names those crates'
  own concepts (`ComposeContext` capture directory, terminal image root, hook
  directory) and is unrelated to `FileResolutionContext::base_dir`; left
  alone.

### Searches

- Old names (`for_base`, `for_trusted_external_base`, `request_base_dir`,
  `is_trusted_external_authoring_base`, `with_env_path_whitelist`,
  `effective_env_path_whitelist`, `default_env_path_whitelist`): none in
  `biscuit-file`, `darkmatter`, `claudine`, `docs`, or `.claude/skills`
  outside the one intentional skill sentence above and `features/` specs.
- `PROJECT_ROOT` / `DOCS_BASE`: only as user-declared variable names in tests,
  plus `link_normalization::tests::no_variable_is_portable_unless_declared`,
  which asserts they are no longer built in; the topic page's
  `{{PROJECT_ROOT}}` is a generic interpolation example.
- `${VAR}` output: none left (the `operations.rs` comment was the last).
- `unwrap()` / `expect()`: none in production code added by this feature
  (`portable/*.rs` above `#[cfg(test)]`, `link_normalization.rs`, and every
  added line in the feature commits). The one hit is a rustdoc example in
  `path_identity.rs`.
- Docs naming the feature: none in any `docs/` tree or README.
  `.claude/skills/os/build-hosts.md` contains the worktree directory name
  `<host>--feat-reusable-path` as an example path, which is not a reference
  to the spec.

### Gates (macOS)

- `biscuit-file`: `just test` 973 passed; `just lint` clean; `just test-l2`
  "not applicable" stub; `just check-tier-coverage biscuit-file` 0 stranded.
- `darkmatter`: `just test` 8743 passed, 12 skipped; `just test-l2` 18 + 69 + 3
  passed; `just lint` clean.
- `claudine`: `just test` stops at the known environmental failure
  `claudine-cli completion::composition::tests::compose_magic_does_not_emit_a_nested_file_without_its_scope`
  (reads the real `~/.claudine/prompts/plan.md`, see Phase 7). With an
  isolated `HOME` (and `RUSTUP_HOME`/`CARGO_HOME` pinned) the full suite is
  8072 passed, 9 skipped. `just test-l2` 277 + 3 passed; `just lint` clean
  apart from the known `__eh_frame` linker note.
- Pre-existing, not from this feature: `RUSTDOCFLAGS=-D warnings cargo doc -p
  biscuit-file` fails on `yaml/analyze/engine.rs:59` (public doc links to the
  private `report`). The new intra-doc links in `mod.rs` resolve.
- `just ci-local --plan`: 96 executing cells, 67 of them on the pull-request
  environments (ubuntu, macOS), so the 60–100 min band. That scope is the
  whole branch, which carries other merged branches not yet on `main` (968
  files differ from `main`), not this feature. No `.github/` or `scripts/ci/`
  file changed, so no new gate or cell kind; the cells this feature adds are
  the ordinary `biscuit-file`, `darkmatter*`, `claudine*` L1/L2/lint cells and
  their dependents' check cells.

### Cross-OS evidence

| OS | Evidence | Host |
| -- | -------- | ---- |
| macOS | all gates above, final tree | this Mac |
| Linux | `biscuit-file` 911/911, final tree (Phase 8); `darkmatter` 7169, `darkmatter-cli` 797 (Phase 7) | `build-linux` via `just cross-check` |
| Windows (native) | `biscuit-file` 916/916 (Phase 6); `darkmatter` normalization + interpolation 30/30 and `darkmatter-cli` `compose_transclusion` 11/11 after the Phase 7 fixes; the 7 pre-existing Darkmatter Windows failures remain | `build-win-native` |
| WSL2 | not run locally: no WSL-specific code; relies on the nightly leg | — |

Phase 8 changed only comments and Markdown, so the Windows evidence from
Phases 6 and 7 still applies; full Windows and WSL2 coverage comes from the
post-merge `main` push (Windows) and nightly (WSL2) legs.

### Final summary for review

**Rulings.** R1–R11 (top of this log) were applied as recorded; all remain
"provisional" working rulings the author may overturn.

**Deliberate behavior changes.**

- `FileResolutionContext::base_dir()` now means the tree root; the old meaning
  is `cwd()`. An explicit or bare relative reference that leaves a boundary
  tree, as written or through a symlink, fails with `RelativeTreeEscape`.
  An in-repository `./../../x.md` that climbs above the repository root now
  fails where it used to resolve. `allow_external_relative()` is the reader
  opt-in.
- Darkmatter link normalization runs through `PortablePath`:
  `with_env_path_whitelist` and the built-in `PROJECT_ROOT` / `DOCS_BASE`
  anchors are removed (`with_portable_env` / `PORTABLE_ENV_VARIABLES`
  replace them); a same-directory target is now `./x.md`; a deep in-repository
  target is `&path`; a declared variable is written as `{{{VAR}}}`. Old
  generated `${VAR}/…` links were never references, so nothing reads them back.

**Departures from the spec** (docs describe the code; the spec is left as
decided): `Attempt.rejected`; `NotApplicable::RouteShape` and the extra
reasons and error variants (Phase 6); boxed references in two error arms;
`MagicPath(Some(filter))` spells from the filter root; Darkmatter compose
feeds `from_path` with absolute destinations only, so authored intent is not
kept through compose; no Darkmatter reader opt-in (no cleanup reads escaping
links); Claudine was the skip case (Phase 7).

**Still open (not blocking review).** `CandidatePlanOrder::AuthoringBaseFirst`
is not renamed (9 uses across `biscuit-file`, Darkmatter, Claudine); its doc
now says "authoring `cwd`". Keeping authored intent through Darkmatter compose
is a possible follow-up.

**Status.** Implementation complete, ready for review. The feature was not
moved to `_completed` and nothing was committed.
