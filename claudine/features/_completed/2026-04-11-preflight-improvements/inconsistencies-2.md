# Pre-Flight Inconsistencies Audit (Follow-up)

Audit date: 2026-04-11
Status: **All inconsistencies addressed**

## Verification Results

### Inconsistency 1: Wrapper path separate shell audit
**Status: Fixed**
- The older audit path in `claudine/cli/src/commands/wrap/mod.rs` (Flow 2) is now correctly restricted to `Passthrough` mode via `if matches!(prompt_state.mode, HarnessPromptMode::Passthrough)`.
- In `Passthrough` mode, Flow 1 (pre-flight) and Flow 2 (per-attempt audit) now share the same `ShellApprovalOptions` instance, including the `approval_cache`, ensuring that decisions from the pre-flight carry over to the runtime audit.
- The documentation in `pre-flight-checks.md` now explicitly justifies the existence of the per-attempt audit for the passthrough path.

### Inconsistency 2 & 3: Harness commands not pre-flighted in compose/sequence
**Status: Resolved (Documented as Intended)**
- The documentation (`pre-flight-checks.md`) was updated to define and justify a **Two-Phase Discovery** model.
- Phase 1 (Template Directives) happens before composition.
- Phase 2 (Harness Commands) happens after composition because harness properties require the **effective (composed) frontmatter** to be fully resolved (including transclusions).
- Both phases share a single `approval_cache`, which is explicitly documented as the mechanism that makes the process appear as a single approval loop to the user.
- The implementation in `claudine/cli/src/commands/wrap/composition.rs` and `claudine/cli/src/commands/wrap/sequence.rs` follows this two-phase pattern correctly.

### Inconsistency 4 & 5: Documentation of Frontmatter Shell Expansion
**Status: Fixed**
- `claudine/docs/topics/pre-flight-checks.md` now explicitly lists **Frontmatter `$(cmd)` expressions** as a source of shell commands (Step 2 in the source list).
- The description of `Darkmatter's role` was updated to include "scanning top-level frontmatter values for `$(...)` shell expressions".
- Implementation in `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` was verified to run frontmatter scanning (Phase 1) before body directive scanning (Phase 2), including support for frontmatter interpolation before scanning.

### Inconsistency 6: Pipeline order in Darkmatter documentation
**Status: Fixed**
- The `darkmatter` skill files (`.gemini/skills/darkmatter/SKILL.md` and `compose.md`) now correctly list the **Inline Pre** phase order, including **Frontmatter Interpolation** and **Frontmatter Shell Expansion** as the first two steps.
- `darkmatter/docs/darkmatter-compose-pipeline.md` was also updated with the correct 6-step Inline Pre stage.

---

## Conclusion

All inconsistencies identified in the 2026-04-11 audit have been addressed. The system now has a consistent implementation and documentation surface for pre-flight shell approvals across all composition and wrapper paths.
