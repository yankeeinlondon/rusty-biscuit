---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-27-union-partial-file-completion/spec.md`
fix: 2026-09-27-union-partial-file-completion/review-2.md
spec: 2026-09-27-union-partial-file-completion/spec.md
previous: 2026-09-27-union-partial-file-completion/review-1.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T17:12:54-07:00
implemented: true
next: 2026-09-27-union-partial-file-completion/review-3.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-27-union-partial-file-completion/log.md
ready: false
findings:
    - "[high] Union arm selection ignores file match patterns"
human_review: false
recurrence: false
---

# Review 2: Union Partial File Completion

## Assessment

**Not production ready.** Both unblocked findings in review 1 have been addressed: caller-relative file paths now work through the affected schema shapes, and the inline picker has real-terminal tests. Review 1 had no blocked findings. One requirement remains unmet: a selected file's `match(...)` pattern does not rule out the other union arm, and even a settled arm accepts a file outside its pattern.

This is a different defect class from review 1's lost caller origin. Its valid paths resolve from the right directory; the schema then accepts them under the wrong pattern. There is no recurrence of the earlier finding class.

## Verification

| Requirement | Strongest verification and result |
| --- | --- |
| Caller-origin resolution for a single schema, a settled union, and an undecided union | Level 1 shipped-CLI test, `compose_caller_file_resolves_from_the_launch_directory_for_every_schema_shape`: passed. A fresh fixture with both path spellings also composed successfully in all three shapes. |
| Partial-file completion and prompt order | Existing Level 1 PTY tests exercise chooser, confirmation, no match, cancellation, and provider order. The new no-`initialize` chooser test reaches the provider after selecting a file in a separate directory. |
| Inline picker, visible rows, preserved history, and no blank residue | `just test-l2 inline_prompt_scrollback`: 14 real-terminal captures passed in tmux and WezTerm. They exercise `compose` and `inline-compose` picker submit and cancel, plus the file and required-property prompts. Level 2 is the appropriate level for rendered scrollback. |
| Focused diagnostic excerpt and caller-origin error wording | Existing Level 1 tests check line selection and wording; existing Level 2 captures check rendered excerpts. No changes in this review cycle displaced those tests. |
| Test wiring | `just check-tier-coverage claudine`: zero stranded tests. The new Level 2 module is declared in `cli/tests/level2/main.rs` and selected by the live `test-l2` recipe. |

The input robustness matrix does not apply: these changes do not add or change a file-format or configuration reader. Cross-OS CI evidence is outside this readiness decision.

## Unblocked Findings

### 1. Union arm selection ignores file match patterns (high)

**Defect class:** A `file(match(...))` glob is used to offer a completion candidate but is discarded when validation decides which union arm accepts the resulting path.

The spec's union rule says that a selected path failing an arm's `match` must rule that arm out, leaving the matching arm to validate. Claudine's [`file_reference_target`](../../lib/src/composition/schema/supplied.rs) merges patterns to find candidates, but Darkmatter's [schema conversion](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs) deliberately omits `match` from validation. Consequently, [root-arm selection](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs) and the final validator see both declarations as the same eager file type. The current [composition documentation](../../docs/topics/composition.md) explicitly describes this departure from the spec. The existing chooser tests prove a picked file reaches the provider, but never prove that its tree selected the matching arm.

I copied the same small fixture for each case: real `features/x/spec.md` and `fixes/x/spec.md` files, a stub `goose` provider, and a document with one of three schemas. The single schema accepts only the `fixes` pattern; the settled union has `kind: fix`; the undecided union omits `kind`. For each schema I changed only the caller's `spec` value and ran both shipped commands. `claudine compose --goose <document> spec=<path>` exited zero in all six cases. `md schema validate --no-trigger-schemas --format json <document> spec=<path>` returned `valid: true` in all six cases.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Claudine and Darkmatter, single schema | `fixes/x/spec.md`, matching the sole `fixes` pattern | Both accept. | Accept; control. |
| Claudine and Darkmatter, single schema | `features/x/spec.md`, outside the sole `fixes` pattern | Both accept. | Accept under the existing single-schema rule that `match` only suggests files; clean comparator. |
| Claudine and Darkmatter, settled union | `kind: fix` with `fixes/x/spec.md` | Both accept. | Accept; control. |
| Claudine and Darkmatter, settled union | `kind: fix` with `features/x/spec.md` | Both accept. | Rule out the `fix` arm and reject the combination. |
| Claudine and Darkmatter, undecided union | `fixes/x/spec.md` | Both accept; the two arms remain indistinguishable by path. | Rule out the `feature` arm and select the `fix` arm. |
| Claudine and Darkmatter, undecided union | `features/x/spec.md` | Both accept; the two arms remain indistinguishable by path. | Rule out the `fix` arm and select the `feature` arm. |
| Claudine's partial-file chooser, with and without `initialize` | A partial matches both trees and the user picks the `fixes` file | Existing Level 1 PTY tests launch successfully, but assert only the chosen path, not the selected arm. | Keep the chooser result and prove that only the matching arm applies. |

Apply the spec's arm-selection rule consistently to explicit valid caller paths and chooser picks, through early arm selection and final validation. Add public-result tests where the two arms have an observable arm-specific requirement, plus the mismatched settled-arm control above. Preserve the existing single-schema suggestion rule, candidate discovery, and caller-origin resolution.

## Blocked Findings

None.
