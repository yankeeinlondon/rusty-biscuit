---
ready: false
implemented: true
agent: codex/default
created: 2026-06-27T15:52:51
---

# Review 1: Path Resolution

## Verdict

Not ready for production.

The implementation fixes the main lifecycle `file_exists(spec)` fallback path, but schema `file` validation does not preserve the required document-first resolution order, and the pre-validation/status-report schema path still cannot receive the explicit launch-area fallback.

## Findings

### 1. [High] `file` schema values skip the prompt document directory when a fallback is configured

**Evidence:** `darkmatter/lib/src/markdown/schemas/format.rs:71` registers `darkmatter-file` with only `fallback`, and `resolve_file_reference` chooses `reference.resolve_from(fallback)` when present (`darkmatter/lib/src/markdown/schemas/format.rs:168`). It never tries the prompt document directory first. The comment at `darkmatter/lib/src/markdown/schemas/format.rs:159` explicitly says the validator has no document-dir anchor. Claudine now passes the launch-area fallback into prepare-time schema validation (`claudine/lib/src/composition/prepare.rs:182`, `claudine/cli/src/commands/compose/prep.rs:748`), so this behavior is active on the production compose path.

**Why this matters:** The spec requires a shared document-first / launch-area-fallback resolver for `file`-typed schema properties and says document-authored references must still resolve next to the prompt before caller fallback semantics apply. With the current code, a prompt like:

```yaml
$schema:
  spec: "file(required)"
spec: ./local.md
```

will fail schema validation when `./local.md` exists beside the prompt but not under the launch area. Meanwhile `{{file_exists(spec)}}` uses `ResolutionContext::base_dir` first and would resolve the same value correctly. That reintroduces disagreement between schema validation and expression functions, just in the opposite direction from the original bug.

**Test rigor:** Level 1 is the right level for this deterministic path-resolution behavior. Existing L1 tests cover fallback-only schema validation and `$schema` reference document-relative behavior, but there is no L1 regression where a `file` property value exists in both the prompt dir and fallback dir, or only in the prompt dir, proving schema value validation keeps prompt-dir precedence.

**Required fix:** Thread the document directory into the `darkmatter-file` validator path, or move schema `file` validation to a resolver object that has both anchors. The expected order must match expression resolution: document dir first, then launch-area fallback, with no ambient-CWD fallback on production paths.

### 2. [High] Schema pre-validation and status reporting still use ambient-CWD behavior

**Evidence:** `build_schema_status_report` calls `load_effective_schema(source, None)` (`claudine/lib/src/composition/schema_validation.rs:854`), `pre_validate_schema` does the same (`claudine/lib/src/composition/schema_validation.rs:1065`), and `drop_invalid_optionals` does the same (`claudine/lib/src/composition/schema_validation.rs:1203`). The sequence path calls `pre_validate_schema` before per-step prepare (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:242`) and later builds the status report with no fallback (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:393`).

**Why this matters:** The spec says production schema validation should use the same explicit launch-area fallback instead of depending on `std::env::current_dir()`. The main prepare path now has a `PrepareOptions::file_ref_fallback_dir`, but the earlier schema helpers do not accept one. That leaves sequence aggregation, interactive missing-property status, and optional-drop pre-validation on the old implicit behavior. At best this makes reports disagree with final prepare; at worst it can drop or reject a literal `file` optional/required value before the corrected prepare path gets a chance to validate it.

**Test rigor:** Level 1 is sufficient. Current tests do not cover `pre_validate_schema`, `drop_invalid_optionals`, or sequence phase 1C with a file value that exists under the launch area but not the ambient CWD.

**Required fix:** Extend these schema helper APIs to accept the same fallback anchor, thread it from `CompositionPrepContext.launch_workspace.launch_cwd` and sequence `launch_area`, and add focused L1 regressions for direct interactive pre-validation and sequence phase 1C.

## Verification-Level Assessment

All requirements in this fix are filesystem/path-resolution behavior, not terminal rendering or OS keyboard behavior. Level 1 tests are appropriate; no Level 2 or Level 3 tests are required for production readiness here.

Covered at Level 1:

- lifecycle `file_exists(spec)` after `chdir` resolves via launch-area fallback;
- expression-level prompt-dir precedence over fallback;
- body `::file` transclusion remains document-relative;
- `$schema` references remain document-relative.

Missing or insufficient at Level 1:

- `file`-typed schema property values remain document-first when fallback is configured;
- `pre_validate_schema` / `drop_invalid_optionals` / schema status reports use the explicit fallback instead of ambient CWD;
- sequence phase 1C schema pre-validation uses the explicit fallback.

