---
created: 2026-09-17
spec: ./spec.md
---

# Binding ruling: shell-free initialization

The author's ruling is authoritative: `initialize` runs before preflight and
must not execute user-authored shell commands. Approval cannot grant an
exception. Spec R2 and AC5 define the amended contract.

This supersedes review 1's proposed approval gate for early catch shells and
all earlier plan/design statements permitting approved initialization shells.
The original review and implementation log are historical evidence, not the
current contract. Earlier passing results do not verify this amendment.

## Implementation and review checklist

- Reject every initialization shell form during validation, including dead
  branches, and enforce the prohibition for programmatically built stacks.
- Refuse bootstrap frontmatter shell expansion regardless of supplied approvals;
  caller inputs and proxy overlays cannot grant an exception.
- Keep early blocked/failure/finalize catch chains shell-free, including catches
  reached through evaluation errors, rejected handoffs, and stabilized-read,
  schema, or audit failures. `no_error` cannot suppress the prohibition.
- Preserve non-shell initialization, non-shell catches, exactly-once routing,
  and approved shell execution after successful preflight.
- Verify direct, inline, loop, proxy, and harness-adopted entry paths using
  isolated fixtures, no real providers, and no terminal/browser focus changes.
- Update README, composition/lifecycle references, skill guidance, schema/help
  descriptions where applicable, source comments, and old approval-based tests.
- Record fresh checks and remaining gaps in evidence.md. Require a new review
  against amended R2; do not move the fix to `_completed`.
