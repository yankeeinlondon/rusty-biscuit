---
ready: false
implemented: true
agent: codex/default
created: 2026-07-05T14:08:45
---

# Review 1: Darkmatter Base Frontmatter Schema

## Findings

### High: Current HR documentation still advertises removed root-level `hr:` behavior

The implementation removes the two runtime compatibility paths required by the spec: `style::parse::from_frontmatter` no longer merges top-level `hr`, and `render_tree::entrypoints::resolve_hr_defaults` no longer reads root `hr`. However, current user-facing documentation still tells authors to use root-level `hr:` as page defaults:

- `darkmatter/docs/topics/horizontal-rules.md:7` says bare rules inherit page-level `hr` defaults and its examples use:
  ```yaml
  hr:
    style: waves
  ```
- `darkmatter/docs/topics/horizontal-rules.md:25` still lists precedence as `Page frontmatter hr`.
- `darkmatter/docs/rendering/hr.md:17` still lists `Top-level hr: alias` in the precedence model.
- `darkmatter/docs/rendering/style.md:837` through `darkmatter/docs/rendering/style.md:857` still says the top-level `hr:` alias is retained for a release cycle and participates in precedence.

This directly conflicts with the spec's "Deprecated Root `hr` Removal" section, which requires updating docs that still advertise root `hr` and making `style.hr` the only supported horizontal-rule frontmatter surface. A user following these docs will author frontmatter that now silently has no effect in normal rendering.

Verification level: docs-only/user contract issue. The runtime removal itself has Level 1 regression coverage for the canonical `style.hr` path, and existing Level 2 horizontal-rule tests cover real-terminal rendering for canonical `style.hr.*`; no Level 2 mismatch is needed for this finding.

### Medium: `DARKMATTER_NO_BASELINE_SCHEMA` disables an explicit `--baseline-schema`

`apply_compose_baseline_schema` returns early when `DARKMATTER_NO_BASELINE_SCHEMA` is truthy, before checking the explicit `--baseline-schema` argument:

- `darkmatter/cli/src/commands/compose.rs:620`
- `darkmatter/cli/src/commands/compose.rs:625`
- `darkmatter/cli/src/commands/compose.rs:629`

The spec resolution says `--no-baseline-schema` or `DARKMATTER_NO_BASELINE_SCHEMA=1` are for raw compose behavior, while `--baseline-schema PATH` replaces the default with a custom baseline. The explicit flag should win over an ambient environment variable, or the CLI documentation should state that the environment variable disables all baselines, including explicit ones. There is no test for `DARKMATTER_NO_BASELINE_SCHEMA=1 md compose --baseline-schema custom.yaml ...`; the existing custom-baseline test removes the env var.

Verification level: Level 1 CLI integration is sufficient. No terminal emulator behavior is involved.

### Medium: Base schema docs overstate validation defaults

`darkmatter/docs/schemas/darkmatter-schema.md:3` says the library adds the base schema to every document it "composes or validates." The implementation does not do that for validation by default:

- `DarkmatterSchemas::new()` still creates no baseline by default.
- `md schema validate` still builds `DarkmatterSchemas::new()` and only attaches a baseline from `--schema` or `BASELINE_SCHEMA`, preserving the previous CLI contract.

That behavior is consistent with the spec's CLI integration section and resolution, but the new docs are too broad. Suggested wording: the library exposes the base schema and `md compose` injects it by default; `md schema validate` uses it only when supplied as a baseline.

Verification level: Level 1 CLI/API behavior is sufficient. The existing `schema_validate_no_schema_no_baseline_is_vacuous_success` test verifies the unchanged validate default.

## Test-Level Assessment

- Nested mapping object syntax: verified at Level 1 by parser/JSON-schema equality tests. Level 1 is appropriate because this is in-process schema parsing and conversion.
- Sequence union arms with mapping object shapes: verified at Level 1 by schema validation tests. Level 1 is appropriate.
- `generated` constraint parsing, JSON Schema annotation, static-required suppression, and non-nullable present values: verified at Level 1. Level 1 is appropriate; no terminal or OS input behavior is involved.
- Base schema parsing/conversion and public library accessors: verified at Level 1. Level 1 is appropriate.
- `md compose` default baseline injection, opt-out flag/env, custom baseline, unknown keys, and document `$schema` precedence: verified at Level 1 CLI integration tests. Level 1 is appropriate.
- `md schema about` rendering includes real-terminal Level 2 coverage for the schema-reference table rendering. No Level 3 behavior is required.
- Canonical `style.hr.*` rendering has existing Level 2 coverage for terminal output; the removed root-level `hr:` path is a docs/contract cleanup issue, not a terminal-rendering verification mismatch.

## Summary

Most core implementation requirements are present: the authored baseline schema parses and converts, nested mapping syntax works, `generated` has the intended static-validation semantics, compose injects the base schema by default, and the deprecated root `hr` runtime paths were removed.

The feature is not production-ready until the stale HR documentation is corrected and the compose baseline precedence contract is resolved or explicitly documented.

## Verification

Ran `just test` in `darkmatter/`: passed for `darkmatter` and `darkmatter-cli` Level 1 suites.
