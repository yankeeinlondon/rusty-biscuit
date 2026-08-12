---
ready: true
agent: codex/default
created: 2026-06-27T21:35:04
---

# Review 4: Path Resolution

## Verdict

Ready for production.

The review-3 cache-key blocker has been addressed: `ComposeOptions::file_ref_fallback_dir` is now included in Darkmatter's compose options hash, and the fix has both focused hash coverage and an end-to-end persistent-cache regression. I did not find remaining gaps against the spec.

## Findings

No blocking findings.

## Verification-Level Assessment

All requirements in this fix are deterministic filesystem/path-resolution behavior. Level 1 coverage is appropriate; no Level 2 or Level 3 terminal/keyboard tests are required.

Verified at Level 1:

- expression-level document-first / launch-area-fallback resolution;
- lifecycle event-time `file_exists` and `frontmatter` fallback after a process `chdir`;
- lifecycle shell preflight and sequence template preflight use the launch-area fallback;
- loop expression resolution uses the launch-area fallback where lifecycle context is available;
- document-relative body `::file` transclusion still resolves next to the prompt document;
- `$schema: ./schema.yaml` and root-union `$schema` string arms still resolve relative to the prompt document;
- `file`-typed schema property values use document-first / launch-area-fallback resolution;
- schema pre-validation, invalid-optional dropping, status reports, and sequence step pre-validation use the fallback;
- compose cache identity is sensitive to `file_ref_fallback_dir`, including a regression where two launch areas must not share a stale cached `file_exists` result.

## Verification Run

Ran:

```bash
cargo nextest run --color=never -p darkmatter -E 'test(/file_ref_fallback/) + test(/options_hash_sensitive_to_file_ref_fallback_dir/) + test(/debug_impl_includes_file_ref_fallback_dir/) + test(/schema.*fallback/) + test(/compose_cache.*fallback/) + test(/launch_area/)'
```

Result: 12 passed. One simple builder test reported a leaked-handle failure on the first try but passed on retry; the selected feature tests, including cache-key and cache-regression coverage, passed.

Ran:

```bash
cargo nextest run --color=never -p claudine -p claudine-cli -E 'test(/file_ref_fallback/) + test(/launch_area/) + test(/lifecycle_shell_read_side/) + test(/template_preflight_.*fallback/) + test(/sequence_step_pre_validation_uses_launch_area_fallback/) + test(/pre_validate_schema_uses_launch_area_fallback_not_cwd/) + test(/drop_invalid_optionals_keeps_file_under_launch_area_fallback/)'
```

Result: 13 passed.
