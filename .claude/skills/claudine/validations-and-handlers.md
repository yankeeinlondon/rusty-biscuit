---
hash: ef46db3751d8e999-7ef9d1cfb1105a79
last_updated: 2026-06-30
---
# Retired: Validations and Handlers → Lifecycle Stacks

The harness **validation and handler DSL** (`pre_checks`, `post_checks`, `handle`,
`handle_<event>`, `deviate`) has been **removed**. Its gating, verification, and
recovery roles are now expressed through the prompt's **lifecycle stack**. See the
lifecycle reference: [`lifecycle.md`](lifecycle.md)
and the lifecycle spec at `claudine/features/2026-05-12-lifecycle/spec.md`.

## What the harness module still owns

The `harness` module is retained for: timeout parsing and enforcement
(`timeout`, `timeout_warn`, `step_timeout`, `step_timeout_warn`), the shell-audit
pre-flight, runtime attempt classification (`ProcessTermination`,
`FailureEvent`, attempt outcome / `error_kind`), speech helpers, and the lifecycle
recovery infrastructure that backs `Retry` / `Resume` / `Defer`. It no longer
parses or evaluates validation rules or handler tables.

## Removed-key diagnostics

A composed document that still declares any retired key fails preparation with a
typed `CompositionError::RemovedValidationKey` that names the offending key and its
replacement surface. The scan (`scan_removed_validation_keys` in
`composition/lifecycle.rs`) runs **before** lifecycle event blocks are parsed, so
the diagnostic is specific rather than a generic unknown-field error.

| Removed key | Replacement surface |
|-------------|---------------------|
| `pre_checks` | the `initialize` or `start` lifecycle stack |
| `post_checks` | the `success` or `finalize` lifecycle stack |
| `handle_<event>` (e.g. `handle_timeout`, `handle_inline_body_unchanged`) | the `blocked` or `failure` lifecycle recovery actions |
| `handle` | a lifecycle `shell` action or other lifecycle action |
| `deviate` | a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) |

## How the old roles map onto the lifecycle stack

- **Gating** (the old `pre_checks` — should this run start?) → a `when:` guard in the
  `initialize`/`start` stack plus a `Skip`, `Proxy`, or `Error` lifecycle action.
- **Verification** (the old `post_checks` — did the agent do the work?) → a `when:`
  guard in the `success`/`finalize` stack that raises an `Error` lifecycle action when
  the contract is unmet.
- **Recovery** (the old `handle_*` handlers) → `Retry`, `Resume`, `Defer`, or
  `Proxy` lifecycle actions in the `failure`/`blocked` stack.
- **Side-effect recovery** (the old `deviate`) → a lifecycle `shell` action followed by a
  recovery action.

Timeouts (`timeout`/`step_timeout` and their `*_warn` partners) are unchanged and
remain frontmatter properties; a timeout now routes to the `failure` lifecycle event,
where a `Retry` or `Resume` action can recover.