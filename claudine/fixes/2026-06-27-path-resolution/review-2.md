---
ready: false
agent: codex/default
created: 2026-06-27T17:09:59
implemented: true
---

# Review 2: Path Resolution

## Verdict

Not ready for production.

The review-1 blockers around schema `file` values are largely fixed: schema validation now receives both the prompt document directory and the launch-area fallback, and the new L1 tests cover direct pre-validation, optional dropping, status reporting, and sequence step pre-validation under an unrelated CWD.

Two preflight paths still do not carry the same fallback anchor, so caller-supplied launch-area-relative paths can still resolve differently before the provider run starts.

## Findings

### 1. [High] Lifecycle shell preflight still resolves read-side file functions from the prompt directory only

**Evidence:** `prepare_direct_with_schema` and `prepare_inline_with_schema` pass `options.file_ref_fallback_dir` into the Darkmatter compose pass (`claudine/lib/src/composition/prepare.rs:182`, `claudine/lib/src/composition/prepare.rs:348`), but then call `resolve_lifecycle_shell_commands` without forwarding that fallback (`claudine/lib/src/composition/prepare.rs:239`, `claudine/lib/src/composition/prepare.rs:391`). Inside `resolve_lifecycle_shell_commands`, the `ResolutionContext` is rebuilt with only `source_path.parent()` (`claudine/lib/src/composition/preflight.rs:253-261`).

**Why this matters:** Lifecycle shell commands are deferred lifecycle strings, but the shell exception resolves them at preflight so the approved command equals the executed command. If a shell command contains an early-binding read-side expression such as:

```yaml
start:
  stack:
    - shell: "test {{ file_exists(spec) }}"
```

and `spec` is supplied relative to the launch area, the event-time lifecycle context would now resolve it through the launch-area fallback, while shell preflight still sees only the prompt directory. That leaves one lifecycle surface on the original fragile behavior and violates the spec's goal that read-side file references no longer depend on the post-launch CWD or prompt-only anchoring.

**Test rigor:** Level 1 is sufficient. Existing L1 tests cover lifecycle event-time `file_exists`/`frontmatter` fallback, but I found no L1 regression for lifecycle `shell` command interpolation using `file_exists(spec)` or `frontmatter(spec, ...)` with a file that exists only under the launch-area fallback.

**Required fix:** Extend `resolve_lifecycle_shell_commands` to accept the same file-reference fallback, build its `ResolutionContext` with `.with_file_ref_fallback_dir(...)`, and pass `PrepareOptions::file_ref_fallback_dir.as_deref()` from both direct and inline prepare paths. Add an L1 test where lifecycle shell preflight resolves a launch-area-relative path after the ambient CWD is moved elsewhere.

### 2. [High] Sequence template preflight omits the fallback on `ComposeOptions`

**Evidence:** Sequence phase 1C correctly calls `pre_validate_schema(source, Some(&step_set_overrides), launch_area)` (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:243`) and later passes `file_ref_fallback_dir` into `PrepareOptions` (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:311`). However, the intermediate template shell preflight builds `ComposeOptions` with only `.with_source_file(...)` and `.with_set_overrides(...)` (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:275-283`) before calling `resolve_shell_approvals` (`claudine/cli/src/commands/wrap/sequence/phase1c.rs:292-298`).

**Why this matters:** Direct compose explicitly added `.with_file_ref_fallback_dir(prep_context.launch_workspace.launch_cwd.clone())` to its preflight `ComposeOptions` because Darkmatter's preflight compose can run schema validation and read-side interpolation before final prepare (`claudine/cli/src/commands/compose/prep.rs:263-270`). Sequence still lacks the equivalent. A sequence step with a `::shell` preflight path, schema `file` property, or read-side expression depending on a launch-area-relative file can pass the new pre-validation, then fail or discover different shell commands during template preflight before the corrected prepare path runs.

**Test rigor:** Level 1 is sufficient. Existing L1 coverage exercises sequence step schema pre-validation with the fallback, but does not cover the following `resolve_shell_approvals` preflight compose path with a launch-area-only file reference.

**Required fix:** Add `.with_file_ref_fallback_dir(launch_area.to_path_buf())` when building the phase 1C `ComposeOptions` if `launch_area` is present. Add an L1 sequence-phase regression that proves the template preflight path agrees with per-step pre-validation and final prepare when a file value exists only under the launch area.

## Verification-Level Assessment

All requirements in this fix are deterministic filesystem/path-resolution behavior. Level 1 coverage is the appropriate level; no Level 2 or Level 3 terminal/keyboard tests are required for production readiness.

Covered at Level 1:

- expression-level document-first / launch-area-fallback resolution;
- lifecycle event-time `file_exists` and `frontmatter` fallback;
- body `::file` transclusion remains document-relative;
- `$schema` references and root-union string arms remain document-relative;
- schema `file` property values use document-first / launch-area-fallback resolution;
- `pre_validate_schema`, `drop_invalid_optionals`, and schema status reports use the fallback;
- sequence step schema pre-validation uses the fallback.

Missing or insufficient at Level 1:

- lifecycle `shell` command interpolation uses the launch-area fallback for read-side file functions;
- sequence template shell preflight `ComposeOptions` carries the launch-area fallback.

## Verification Run

Ran:

```bash
cargo nextest run --color=never -p claudine -p darkmatter -E 'test(/file_ref_fallback/) + test(/launch_area_fallback/) + test(/pre_validate_schema_/) + test(/drop_invalid_optionals_/) + test(/status_report_marks_fallback_file_valid/) + test(/prepare_time_and_event_time_agree_on_file_reference/) + test(/body_file_transclusion_stays_document_relative_with_fallback/)'
```

Result: 18 passed. Nextest reported 2 tests as flaky because their first attempt exited successfully but leaked handles; both passed on retry. That leak behavior is residual test hygiene risk, not evidence that the path-resolution assertions failed.
