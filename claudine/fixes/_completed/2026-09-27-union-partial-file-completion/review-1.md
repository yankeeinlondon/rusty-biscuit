---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-27-union-partial-file-completion/spec.md`
fix: 2026-09-27-union-partial-file-completion/review-1.md
spec: 2026-09-27-union-partial-file-completion/spec.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T08:25:50-07:00
implemented: true
next: 2026-09-27-union-partial-file-completion/review-2.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-27-union-partial-file-completion/log.md
ready: false
findings:
    - "[high] Root-union file selection can reject a valid caller-owned path"
    - "[high] The inline provider picker lacks real-terminal verification"
human_review: false
recurrence: false
---

# Review 1: Union Partial File Completion

## Assessment

**Not production ready.** The reported partial-file flow passes its new process tests, including union-arm selection, confirmation, cancellation, and prompt order. Two gaps remain: a valid file can fail after union selection, and the new inline provider picker has no test in a real terminal emulator to establish the required scrollback behavior.

## Verification and scope

| Check | Result |
| --- | --- |
| `just test-cli union_partial` in `claudine/` | Five Level 1 process tests passed. |
| `just test-cli level1_provider_picker_pty` in `claudine/` | One Level 1 PTY test passed. |
| `just check-tier-coverage claudine` | No stranded test. |
| Isolated shipped-CLI fixture with an existing spec, a prompt under `prompts/`, and a stub provider | Single schema and discriminated union controls passed; an undecided union failed on a relative path, and both relative and absolute paths failed when the same fixture was a git repository. |

The new Level 1 tests are declared by `claudine-cli`'s consolidated `l1` target and selected by its recipe. The Level 2 capture tests are declared by `level2/main.rs` and selected by `test-l2`. This review did not rerun the full package suites or the Level 2 terminal suite; the implementation log records earlier passing runs. No OS evidence is required for this readiness decision.

The file-format robustness matrix does not apply: this fix changes schema decisions and error excerpts, but adds no file-format or configuration reader.

## Requirement coverage

| Requirement | Strongest applicable evidence | Assessment |
| --- | --- | --- |
| Composition-tolerant union-arm choice and union of candidate patterns | Level 1 library tests plus Level 1 PTY runs through the CLI | Verified for partial values and the covered arm shapes. |
| No-match, decline, cancellation, and one-file confirmation before the provider picker | Level 1 PTY tests | Appropriate for prompt order and program decisions; the real-terminal confirmation capture already exists for the related file chooser. |
| Caller-origin wording for a late missing-file verdict | Level 1 CLI test using distinct launch and document directories | Verified for the tested missing-file case; finding 1 covers valid relative values. |
| Inline provider picker, bounded display, retained scrollback, and no blank screen | Level 1 PTY escape-byte test | Level 2 real-terminal capture is missing; see finding 2. |
| Focused frontmatter excerpt, source line numbers, elision, and highlighting | Level 1 excerpt tests and Level 2 captures for schema parse and error rendering | Appropriate for the tested diagnostic shapes. |

## Findings

### 1. Root-union file selection can reject a valid caller-owned path (high)

**Defect class:** Union-arm projection does not consistently preserve the caller's file-resolution origin when the caller's value is already a valid path or becomes a selected file.

The Claudine early resolver, [`unresolved_supplied_files`](../../lib/src/composition/schema/supplied.rs), correctly checks each caller record against its launch directory. Darkmatter's [`project_root_arm_caller_values`](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs) is the later step that determines which root-union arm applies; when it cannot commit an arm, validation can fall back to the prompt document's directory. Claudine's late message rebasing only rewrites a value that is still unresolved from the caller's directory, so it cannot repair a valid caller-relative value that the later step judged from the wrong directory.

I reproduced the failure through the shipped `claudine` binary with a real file at `fixes/x/spec.md`, a two-arm schema at `prompts/p.md`, a private home, and a stub `goose` provider. From the fixture root, `claudine compose --goose prompts/p.md spec=fixes/x/spec.md` failed while the single-schema control passed. Passing the absolute path made the non-git union succeed. After `git init` in the fixture, even that absolute-path union case failed without `initialize`. The implementation log records the same class and explains that a successful chooser can hand this absolute path to the failing path. The new CLI control in [`compose_schema_cli.rs`](../../cli/tests/l1/compose_schema_cli.rs) deliberately passes an absolute path for the root-union shape, so it does not exercise the relative-path case.

| Site and shape checked | Observed | Expected |
| --- | --- | --- |
| Single-schema `file`, prompt in a subdirectory, existing caller-relative path | Passes in the existing CLI control. | Pass. |
| Root union with an explicit `kind: fix` discriminator, prompt in a subdirectory, existing caller-relative path | Passes in the isolated CLI fixture. | Pass. |
| Root union without a settled discriminator, prompt in a subdirectory, existing caller-relative path | Fails in the isolated CLI fixture; the implementation log also records a message naming `prompts/`. | Pass from the caller's launch directory. |
| Same undecided root union, non-git fixture, absolute path | Passes in the isolated CLI fixture. | Pass. |
| Same undecided root union, git fixture without `initialize`, relative or absolute path | Both fail in the isolated CLI fixture. The chooser supplies an absolute path, so its selected-file route is exposed. | Compose and launch the selected arm. |

Fix the root-union projection at the shared Darkmatter boundary, then make the CLI control use the relative path and add a process test for the no-`initialize` chooser path. Use the same fixture across single and union schemas so a wrong base directory changes the public outcome.

### 2. The inline provider picker lacks real-terminal verification (high)

**Defect class:** The new picker is verified only by manufactured PTY bytes, while the contract also requires the terminal emulator to retain scrollback and leave no full-screen residue.

Claudine's [`prompt_one_shot_provider`](../../cli/src/commands/wrap/selection_ui.rs) now asks `biscuit-tui` for a bounded inline viewport. [`level1_provider_picker_pty.rs`](../../cli/tests/l1/level1_provider_picker_pty.rs) proves that the process does not emit the alternate-screen entry sequence and that the selected provider launches. A PTY captures output bytes but does not render a screen or keep the emulator's scrollback. The existing Level 2 partial-file captures pass an explicit provider flag, so they never open this picker.

| Site and shape checked | Observed | Expected verification |
| --- | --- | --- |
| `compose` with two provider stubs and no provider flag in a Level 1 PTY | Picker bytes appear, no alternate-screen entry is emitted, and the chosen stub launches. | Retain this test for program behavior. |
| `compose` with the same fixture inside tmux or WezTerm | No picker capture test exists. | Capture the visible pane before and after selection or cancellation, and verify prior text remains available without blank residue. |
| `inline-compose` with no provider flag inside a real terminal | No picker capture test exists. | Exercise the same picker through this public command, or demonstrate one shared test genuinely covers both entry paths. |
| Existing real-terminal partial-file confirmation tests | The picker is bypassed by `--goose`. | Keep these as file-chooser evidence; they do not establish provider-picker behavior. |

Add a Level 2 test through the area's live `test-l2` recipe using a detached terminal harness. Seed a recognizable line before the command, capture the picker, submit or cancel it, and assert that the line remains in the terminal history and that the pane has no blank full-screen residue. Keep the harness window unfocused.
