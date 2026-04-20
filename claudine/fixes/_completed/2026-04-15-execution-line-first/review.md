# Review: Execution Line First

## Summary

The implementation is **substantially correct**. The execution line now appears immediately after argument parsing in all three code paths, and the two preflight status messages bracket the heavy work. There are a few deviations from the spec — some intentional (documented in the plan), some worth noting.

## Spec Requirement Checklist

| # | Requirement | Direct Wrapper | Composition | Sequence |
|---|---|---|---|---|
| 1 | Execution line appears immediately | Done (`mod.rs:829`) | Done (`composition.rs:189`) | N/A (delegates to composition per step) |
| 2 | Status "starting pre-flight checks" (Info, Circular) before preflight | Done (`mod.rs:906`) | Done (`composition.rs:575`) | Done (`sequence.rs:108`) |
| 3 | Authorizations happen next | Done (`mod.rs:1236`) | Done (`composition.rs:660`) | Done (`sequence.rs:155,189`) |
| 4 | Status "pre-flight checks have passed" (Success, Circular) after checks | Done (`mod.rs:1259`) | **Deviation** — see below | **Deviation** — see below |

## Findings

### 1. Env details appear between the two status messages (mod.rs)

**Severity: Medium**

The plan's target order puts env details *after* "pre-flight checks have passed" (step 13). The actual code renders env details at `mod.rs:1108-1158`, which falls *between* MCP composition (ending ~1077) and harness preflight (starting ~1209). The actual output order is:

```
ℹ Starting pre-flight checks          ← line 906
Environment Variables:                 ← line 1110
System Prompt(appended):              ← line 1141
✓ Pre-flight checks have passed       ← line 1259
```

This is because the full `env_plan` is only available after MCP composition, and the harness preflight needs the prompt source which requires argument pipeline completion. The env details block sits between the "starting" and "passed" messages, which may feel odd to the user — the "starting" message fires, then a wall of env output appears before the "passed" confirmation.

**Suggestion:** Consider moving the "passed" message to *before* the env details block (i.e., right after harness preflight completes at ~1253, before the env output at ~1108). Alternatively, restructure to move env details after "passed". However, this may require passing `env_plan` differently, so it could be deferred to a follow-up.

### 2. Composition and Sequence "passed" messages use different wording and state

**Severity: Low**

The spec says `"pre-flight checks have passed"` with `StatusState::Success`. The direct wrapper matches exactly.

- **Composition** (`composition.rs:701`): `"Preflight: shell commands approved for this composition"` with `StatusState::Info`
- **Sequence** (`sequence.rs:203`): `"Preflight: shell commands approved for all N step(s) in the sequence"` with `StatusState::Info`

The plan acknowledges this for composition: *"The existing 'shell commands approved' status at line 686 already serves as the 'passed' message."* These messages are arguably more descriptive than the spec's generic wording, but they're inconsistent with the direct wrapper path.

**Suggestion:** Either:
- (A) Standardize all three paths to use `"Pre-flight checks have passed"` with `StatusState::Success` for consistency, or
- (B) Accept the deviation as intentional — the composition/sequence paths give more specific context about what was approved.

### 3. Composition and Sequence "passed" messages don't explicitly set `.theme(StatusTheme::Circular)`

**Severity: Very Low**

All "Starting pre-flight checks" messages explicitly set `.theme(StatusTheme::Circular)`. The direct wrapper's "passed" message also sets it. But the composition (`composition.rs:704`) and sequence (`sequence.rs:208`) "passed" messages omit the explicit theme — they rely on the default.

The default IS `Circular`, so the rendering is identical. But the code is inconsistent.

**Suggestion:** Add `.theme(StatusTheme::Circular)` to the composition and sequence "passed" messages for consistency with the "starting" messages.

### 4. No snapshot tests for composition or sequence paths

**Severity: Low**

The only snapshot test covers the direct wrapper path. There are no equivalent tests verifying execution order for the composition or sequence paths.

**Suggestion:** Add snapshot tests for:
- Composition path: execution line → "starting" → harness preflight → "passed" → env details
- Sequence path: sequence status → "starting" → phase-1 preflight → "passed" → step execution

### 5. Plan vs Implementation ordering divergence for env details

**Severity: Low (cosmetic)**

The plan's target order (section "Target Order") shows env details at step 13, *after* "pre-flight checks have passed" at step 12. The implementation places env details *before* "passed". This is a documentation vs code mismatch, not a spec vs code mismatch — the spec itself doesn't prescribe where env details go relative to the "passed" message.

**Suggestion:** Update the plan's target order to reflect the actual implementation, or note the deviation.

## Verdict

The core spec requirements are met: the execution line appears immediately, "starting pre-flight checks" fires before heavy work, and "pre-flight checks have passed" fires after. The deviations are in the composition/sequence paths (different wording, different status state) and in the ordering of env details relative to the "passed" message. None of these are blockers, but items 1 and 2 are worth discussing.
