---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-15T22:31:55-07:00
spec: 2026-09-08-frontmatter-matters/spec.md
implemented: true
description: A **fix** review of `2026-09-08-frontmatter-matters/spec.md`
fix: 2026-09-08-frontmatter-matters/review-1.md
findings:
    - "[high] Configured OpenCode defaults are not delivered through a shadow HOME"
    - "[low] Rebuild documentation still describes the removed catalog fallback"
---

# Review 1: Frontmatter Matters

## Resolution (2026-09-17)

Both findings are resolved.

1. Configured defaults now pass through the profile's normal model mapping, so
   the child receives both `--model <id>` and `MODEL=<id>`. The direct and
   composition process regressions were added in `f570958a0`. The later
   provider-overlay work in `9ef76132f` also removed global `HOME` replacement;
   OpenCode repo isolation now fails closed before spawn because its additive
   config selector cannot provide the requested isolation.
2. The `rebuild_target_launch` documentation now states the actual catalog
   contract: it may order list hints and warn during initial preparation, but
   it never demotes the document model.

The implementation is ready for production. The original verdict and findings
below record the state reviewed on 2026-09-15.

## Original Verdict

The frontmatter regression itself is fixed: an unrecognized document model is
forwarded to OpenCode on argv and through `MODEL`, the warning policy behaves as
specified, dry-run agrees with live resolution, and sequences carry the model
to every step. The central resolver is a sound simplification and the focused
tests for those paths pass.

The fix is **not ready for production**. The provider-neutral fallback for an
OpenCode configured default assumes OpenCode can read the same config file after
Claudine has replaced `HOME`. That assumption is false for `--repo`: Claudine
discovers the model under the original `~/.config/opencode`, exports only the
generic reporting variable `MODEL`, launches OpenCode under a shadow home that
does not contain `.config/opencode`, and does not add `--model`. The provider can
therefore run without the model Claudine reports.

Human review is not required. The defect and the missing regression boundary
are deterministic and can be corrected and verified by an agent.

## Findings

### 1. Configured OpenCode defaults are not delivered through a shadow HOME (high) — Resolved

`configured_default_model_in` correctly finds the global default in
`~/.config/opencode/opencode.jsonc`, `opencode.json`, or legacy `config.json`
(`cli/src/commands/wrap/profile/opencode.rs:12-42`). When that source wins,
`resolve_model_and_validate` adds only `MODEL` and intentionally adds no model
argument (`cli/src/commands/exec_prep/mod.rs:100-102`) because the design assumes
the provider will read its own file.

That does not hold when repo-only isolation is enabled. Both direct wrapping and
composition force a shadow home for `--repo`
(`cli/src/commands/wrap/mod.rs:551-565` and
`cli/src/commands/wrap/composition/pipeline.rs:451-474`). The shadow-home builder
sets `HOME` to `~/.claudine`, while OpenCode declares no root-level state files
and its provider home is `.opencode`; nothing materializes the original
`~/.config/opencode` directory into the child home
(`cli/src/commands/wrap/repo_home.rs:283-322` and
`lib/src/provider/opencode/data.rs:640`). OpenCode does not use the generic
`MODEL` variable for selection, so the child has neither the discovered config
nor an explicit `--model` selector.

This recreates the core failure mode the fix is meant to remove: Claudine can
report one model while the provider launches with another model or no model.
It affects direct `claudine opencode --repo ...` and composition with `--repo`
when the model is supplied only by OpenCode's global configuration.

Required change: make the configured-default decision authoritative across
environment rewriting. The smallest robust option is to apply the discovered
default through the profile's normal model mapping, producing `--model <id>`
and `MODEL=<id>`, instead of relying on the provider to rediscover the file.
If preserving implicit provider selection is important, the alternative is to
materialize the exact OpenCode config search path into every shadow environment;
that is broader and easier to drift as OpenCode adds config layers.

Add a Level 1 `CliProcessFixture` regression that places only a model in the
fixture's `~/.config/opencode/opencode.jsonc`, clears `OPENCODE_MODEL` and
`MODEL`, runs the direct wrapper with `--repo`, and has the OpenCode stub record
argv, `MODEL`, `HOME`, and config visibility. It must prove that the launched
model is the configured value. Add the same representative row for composition
because that pipeline builds its environment independently. These tests would
have caught the defect; the current unit tests stop before the child-environment
boundary, and the current `wrap_opencode` Level 1 tests cover only missing,
provider-env, and explicit-CLI sources.

**Resolution:** `f570958a0` made every resolved source authoritative through
`WrapperProfile::apply_model`, which gives OpenCode both `--model <id>` and
`MODEL=<id>`, and added process coverage for direct and composition routes.
`9ef76132f` subsequently replaced shadow homes with provider-owned overlays and
made unsupported OpenCode `--repo` requests fail before spawn.

### 2. Rebuild documentation still describes the removed catalog fallback (low) — Resolved

The documentation on `rebuild_target_launch` says an invalid frontmatter model
"falls back exactly as it would directly"
(`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:630-636`).
The implementation and this fix's contract now forward unrecognized models
unchanged. This is behavior-documentation drift at a high-impact launch seam;
rewrite the sentence to say the catalog may warn but never demotes the document
model.

**Resolution:** the `rebuild_target_launch` documentation now describes those
catalog semantics directly.

## Requirement Verification Levels

| Requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Unrecognized frontmatter model reaches child argv and `MODEL` | Level 1 fake-provider process test | Appropriate and passing. |
| One warning names model/provider; `--silent` suppresses it; recognized model does not warn | Level 1 fake-provider process tests | Appropriate and passing. |
| Resolver precedence and scalar/list catalog behavior | Level 1 unit tests | Appropriate for pure resolution logic and passing. |
| Dry-run shows the launch model, warns, and performs no refresh or provider launch | Level 1 process test | Appropriate and passing. No real-terminal behavior is claimed. |
| Direct OpenCode provider-env selection and missing-model failure | Level 1 fake-provider process tests | Appropriate and passing. |
| OpenCode global config parsing and filename precedence | Level 1 unit tests over an explicit directory | Adequate for parsing, but not for launch delivery; finding 1 identifies the missing process boundary. |
| Two-step sequence launches both steps with the document model and warns once | Level 1 fake-provider process test | Appropriate and passing. |
| Configured default remains authoritative after child-environment rewriting | Level 1 fake-provider process tests for direct and composition routes | Adequate and passing; configured defaults are delivered explicitly, while unsupported OpenCode repo isolation fails before spawn. |

Level 2 and Level 3 are not required for this fix. Its observable contracts are
argv, environment, warning text, dry-run data, and subprocess suppression; none
depends on a real terminal renderer or the terminal emulator's input encoder.

## Verification Performed

- `just test-cli --test compose_frontmatter_model`: **5 passed**.
- `just test-library model_catalog`: **75 passed**, 4,125 filtered out.
- `just test-cli --test wrap_opencode`: **15 passed**.
- GitNexus index refreshed at current `a97a7c7ef`; upstream impact for
  `resolve_document_model` is **high** (8 impacted symbols, 4 direct callers,
  sequence execution affected across 3 modules). The route-by-route review is
  therefore material, not merely stylistic.

The full `just test`, `just test-l2`, and `just lint` gates were not rerun in
this review. The specification records their implementation-time results, and
the focused current-tree runs above are sufficient to reproduce the coverage
boundary and establish the production blocker. Cross-OS execution evidence is
left to CI and does not affect this readiness verdict.

## Original Design Assessment

The shared `resolve_document_model` helper is an ergonomic improvement: the
catalog now orders list hints and supplies warnings without silently changing
launch intent, and the graph confirms the main compose and sequence routes use
the common seam. The provider-neutral `ModelSource` also removes an unnecessary
OpenCode-keyed branch. No meaningful performance regression was found; catalog
refresh remains gated and dry-run remains subprocess-free.

The remaining defect is caused by treating configuration discovery and model
delivery as separable. Once Claudine rewrites the child's environment, a value
read from provider configuration must either be delivered explicitly or the
configuration itself must be preserved in that exact child environment.
