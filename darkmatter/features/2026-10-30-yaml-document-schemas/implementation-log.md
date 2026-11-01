---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/spec.md"
plan: "darkmatter/features/2026-10-30-yaml-document-schemas/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_kind_documents.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/kind_document.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/kind_document.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/tests/l1/kind_document_editor.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/docs/schemas/err.yaml
    - darkmatter/docs/schemas/schema-definition.yaml
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/kind_documents.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/tests/l1/kind_documents.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - simplified-schema/lib/src/simplified/standalone.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - docs/kind-documents.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/schema.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - simplified-schema
decisions_for_review:
    - |-
        The plan expected the pre-feature shipped-schema corpus test to pass
        unchanged after the revert, but it failed: the three kept definition
        files in darkmatter/schemas (schema.yaml, schema-trigger.yaml,
        yaml-schema.yaml) now declare `kind: yaml-schema`, a kind no code
        understands until phases 3-4, so the corpus walk could not classify
        them. I added them as exceptions in shipped_schema_corpus.rs instead
        of moving or deleting them (the plan keeps them in place until 3.3).
        yaml-schema.yaml uses the existing "Draft" class (its `non-null` and
        `property(...)` options do not parse until phase 2). The other two use
        a new test-only class, "UnclaimedKind": no loader claims the file, it
        is refused as a `$schema: ./file` target, and its bodies parse; it is
        not resolved inline because schema-trigger.yaml's `$schema` names a
        type from its own `types:`. Alternatives: reuse "NotASchema" (fails,
        because that class resolves `$schema` inline) or skip the files in the
        walk (loses the check that they stay parseable). If the reviewer
        prefers another shape, only these three exception entries and the
        UnclaimedKind arms in that one test file change; phase 3 moves the
        files and 7.3 re-derives the corpus anyway.
message_to_agent: |-
    Phase 1 left a temporary corpus classification you must update. In
    darkmatter/lib/tests/l1/shipped_schema_corpus.rs the three kept
    darkmatter/schemas/{schema,schema-trigger,yaml-schema}.yaml files are
    listed in EXCEPTIONS: yaml-schema.yaml as `Class::Draft { error: "non-null" }`
    and the other two as a new test-only `Class::UnclaimedKind`. Once phase 2
    makes `non-null` and `property(...)` parse, the Draft entry fails with
    "now parses; reclassify it" - reclassify yaml-schema.yaml as
    `Class::UnclaimedKind` then. When phase 3 moves the files to
    simplified-schema/schemas and gives `kind: yaml-schema` a loader, remove
    the exceptions (an exception whose file disappeared fails the walk) and
    delete `UnclaimedKind` if nothing uses it. The revert is uncommitted in
    the working tree (the orchestrator commits it). Gates were green at the
    end of phase 1: simplified-schema 1113/1113, darkmatter 12884/12884, lint
    clean in both.
---

# Implementation Log for 2026-10-30-yaml-document-schemas (7 phases)

## Phase 1

### 1.1 Preserved docs-escape work

- Wrote `preserved-docs-escape-fix.patch` (336 lines) in this directory: the
  tracked diffs of `darkmatter/docs/inline/schema-validation.md`,
  `darkmatter/docs/topics/schemas/definition.md`,
  `darkmatter/docs/topics/schemas/dmls-schema-support.md`,
  `docs/kind-documents.md`, plus the untracked
  `darkmatter/lib/tests/l1/schema_documentation_pages_compose.rs` via
  `git diff --no-index`.
- `git apply --check` of the patch succeeds against the discarded tree (verified
  right after 1.2, before the revert).

### 1.2 Discarded superseded uncommitted edits

- `git checkout HEAD --` on all 16 tracked modified paths; deleted the untracked
  `schema_documentation_pages_compose.rs`. No wholesale reset was used; the
  untracked `plan.md` and `implementation-log.md` were untouched.

### 1.3 Revert — departure from a whole-commit revert

The revert is **"reverse of four commits minus five carve-outs"** (rule 1), not a
whole-commit revert:

- `git revert --no-commit 8ebb21dd1 56f9e1abf 19184ffe5 2133a089f`.
- Only conflicts: modify/delete on `darkmatter/schemas/schema.yaml` and
  `darkmatter/schemas/schema-trigger.yaml` (edited by later planning commits);
  resolved by the carve-out.
- Restored to `HEAD`: `simplified-schema/lib/src/resolve.rs`,
  `darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs` (inline
  root-union parity fix), `darkmatter/schemas/schema.yaml`,
  `darkmatter/schemas/schema-trigger.yaml`, `darkmatter/schemas/yaml-schema.yaml`
  (the last was never touched by the four commits), and `schemas/kind.yaml`.
- `git revert --quit` then `git reset` so the result is an unstaged working-tree
  change; the orchestrator makes the single revert commit.
- Deleted by the revert: `darkmatter/lib/src/markdown/schemas/kind_documents.rs`,
  `darkmatter/lib/tests/l1/kind_documents.rs`,
  `darkmatter/cli/tests/l1/schema_validate_kind_documents.rs`,
  `darkmatter/dmls/src/{diagnostics,overlay}/kind_document.rs`,
  `darkmatter/dmls/tests/l1/kind_document_editor.rs`.
- Restored by the revert: `darkmatter/docs/schemas/schema-definition.yaml`
  (empty placeholder) and the `$schema:` line of `darkmatter/docs/schemas/err.yaml`
  (rule 4).
- The `mod` declarations for the deleted test modules were removed by the revert
  itself (they were added by the reverted commits), so 1.4 needed no manual edits.

### 1.4 Compiling baseline

- No `mod` edits were needed (see 1.3).
- First full darkmatter `just test` failed in two corpus tests:
  `shipped_schema_corpus::shipped_schema_artifacts_parse_passively` and
  `loadable_shipped_schemas_resolve_and_lower`, because the kept carve-out
  definition files now declare `kind: yaml-schema` (written by planning
  commits after the reverted Rust). This is the risk 1.3 named.
- Fix: classified the three files in `EXCEPTIONS` (see `decisions_for_review`):
  `yaml-schema.yaml` → `Class::Draft { error: "non-null" }`;
  `schema.yaml`, `schema-trigger.yaml` → new `Class::UnclaimedKind` (asserts
  no schema/trigger claim, refused as `./file` target with
  `SchemaError::SchemaDocument`, bodies parse passively). An attempt with
  `Class::NotASchema` failed on `schema-trigger.yaml` (`unknown named type
  match-payload` when its `$schema` is resolved inline without its `types`).
- `shipped_schema_corpus.rs` does not treat `schema.yaml`/`schema-trigger.yaml`
  as shipped (no embedded-catalog expectation), as the plan requires;
  `embedded.rs` no longer embeds them.

### Requirement → test mapping

Phase 1 changes no behavior beyond restoring the pre-feature baseline, so no
new behavior tests were added. The evidence is the existing suites at the
pre-feature expectations:

| Requirement | Evidence |
|---|---|
| Kind-table route removed, inline root-union parity fix kept | `darkmatter::l1 …embedded_sources` (`an_inline_root_union_prepares_and_a_file_arm_is_refused`) passes; `kind_documents` tests deleted |
| `schema-definition.yaml` placeholder and `err.yaml` `$schema:` restored | `shipped_schema_corpus::shipped_schema_artifacts_parse_passively` (`Placeholder`, `EnvelopeRejected` exceptions) |
| Kept YAML definitions stay parseable while unclaimed | the same corpus test via the new exceptions; `loadable_shipped_schemas_resolve_and_lower` asserts `./file` refusal |
| No consumer of removed API | `grep` for `validate_kind_document`, `kind_documents::`, `KindDocument`, `darkmatter:schema[-trigger]` across the repo: no hits |

The Input Robustness Matrix applies to later phases (new loaders); phase 1
adds no parser or loader.

### Gates

- `simplified-schema`: `just lint` clean; `just test` 1113/1113 passed.
- `darkmatter`: `just lint` clean (rerun after the corpus edit);
  `just test` 12884 passed, 10 skipped, 0 failed (second run; first run
  failed as described in 1.4); `just check-tier-coverage darkmatter`: 0
  stranded.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`,
  `change_class: package` → gates required and run.
- Dependent packages: the only public-API removals are
  `DarkmatterSchemas::validate_kind_document`, the `kind_documents` module, and
  the `darkmatter:schema`/`darkmatter:schema-trigger` embedded stems; a repo
  grep finds no consumer outside darkmatter, so no dependent-package tests
  target the changed symbols and none were run. `simplified-schema`'s only
  change is a module-doc deletion.
- Cross-OS: not run; the phase restores previously tested code and adds no
  path or OS-specific logic.
- Code-smell grep: not applicable in phase 1 (none of the listed new modules
  exist yet).
- No pre-existing or load-sensitive failures observed.
