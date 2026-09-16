---
created: 2026-09-10
status: implemented
reviewed: true
implemented: true
review_iterations: 2
area: claudine
packages:
    - claudine
    - claudine-cli
---

# Restore Interactive Completion Before Initialize Consumes Caller File Inputs

## Problem

An explicitly supplied partial file reference can reach an `initialize` guard
without passing through Claudine's existing interactive completion flow. A
guard such as `frontmatter(spec, 'reviewed')` then raises a lifecycle evaluation
error instead of giving the user a chance to select the intended file.

Reported invocation, with the prompt spelling normalized to the file shown in
the execution header:

```sh
claudine compose prompts/review.md spec=fixes/2026-09-10-local -y --codex --model gpt-6-astra
```

From the repository root, the supplied substring matches the existing
`fixes/2026-09-10-local-affected-scope/spec.md`. The review router declares
`spec: file(required;eager;match(**/*spec*.md))`, but its first `initialize`
guard reads the unresolved value through `frontmatter()` and fails with
`invalid file path: fixes/2026-09-10-local`.

Assumption for the expected dialog: stdin and stderr are TTYs,
`prompt_for_missing` is enabled, and `--silent` is absent. The provider's
session mode and `-y` do not disable schema input interaction.

## Evaluation and Evidence

This is a regression against the documented partial-file contract, supported
by current source inspection. A live TTY reproduction and a bisected first bad
commit have not been established in this evaluation.

- `claudine/docs/topics/composition.md` and the claudine skill's
  `composition.md`, under **Provided Partial File References**, specify that a
  failed literal `file(match(...))` value becomes a substring query over glob
  candidates. One candidate requires confirmation; several require a chooser.
- `claudine/cli/src/commands/compose/prep.rs`:
  `defers_schema_verdict_to_initialize` tests for an authored `initialize`
  key. In `run_composition_inner`, that condition bypasses
  `pre_validate_with_interactive_collection` entirely.
- `claudine/lib/src/composition/schema/mod.rs`:
  `pre_validate_schema_for_mode` classifies eligible supplied partials as
  `CompositionError::UnresolvedFileReference`.
- `claudine/cli/src/commands/schema_interactive/mod.rs`:
  `pre_validate_with_interactive_collection` dispatches that error to
  `resolve_unresolved_file_reference`; `resolve_provided_file_reference`
  supplies the confirmation/chooser. None of this runs through the skipped
  branch. `resolve_interactive_options` gates on configuration, stdin/stderr
  TTY state, and `silent`, not YOLO or provider interactivity.
- `prompts/review.md` reads `frontmatter(spec, ...)` in its first guard before
  choosing a proxy target. Post-initialize validation is too late to repair
  this input.
- Implementation testing established that the shipped router's YAML schema
  list is a root-level union. Supplied-file inspection must support its
  uniquely applicable alternative; handling only a single schema mapping
  still reproduces the original lifecycle crash.
- Git blame attributes the schema-deferral branch's history to
  `1ee18d4a0f` and `dbc1c357da`. A later comment from `a1b9a883e9` says the
  pre-validator always runs, contradicting the current branch. These are
  investigation pointers, not proof of the first regressing commit.

The lifecycle error is correctly fatal once the expression raises. The defect
is allowing a recoverable caller file input to reach that expression without
offering the existing completion interaction.

## Required Behavior

### Resolve supplied partials before lifecycle consumption

1. Before an active document's `initialize` consumes caller inputs, inspect
   explicitly supplied `key=value` and `--set` values against its applicable
   file schema metadata. Offer existing partial resolution for eligible
   `file`/`file[]` properties with `match(...)`, including required and
   eager-optional properties.
2. Attempt literal resolution first through `biscuit_file::FileReference`.
   Preserve existing acceptance of valid explicit paths. Only failed literal
   resolution enters glob-plus-substring completion.
3. Reuse existing candidate scope, case-insensitive matching, ordering, file
   detail display, and biscuit-tui interaction. One candidate requires
   confirmation; multiple candidates require selection. YOLO must neither
   suppress the dialog nor silently choose a file.
4. Feed the selected file into effective overrides and the caller provenance
   records used by canonical preparation. Lifecycle expressions, composition,
   validation, and a proxy target must all consume the same selected identity.
   Preserve unrelated overrides and existing file-array value shape.
5. Handle multiple eligible supplied partials without allowing the first
   successful selection to leave another unresolved partial for a lifecycle
   expression to crash on.

### Preserve initialize-before-validation semantics

This early pass resolves supplied inputs; it must not run the full schema
verdict or collect every absent required property before `initialize`.

The review router declares alternatives requiring `spec`, `plan`, or `review`,
and routes using whichever input was supplied. Resolving `spec` must not prompt for
unrelated `plan` and `review` values before the proxy executes. Documents must
retain the ability to create or repair schema values during initialization,
skip, or hand off to another document before the stabilized schema verdict.

Do not fix this by deleting schema deferral, catching expression errors as
`false`, or adding a `file_exists()` fallback to the shipped review guard.
Those approaches either change lifecycle semantics or bypass the requested
completion behavior.

### Deterministic failure and origin handling

- With no candidates, declined confirmation, canceled selection, or disabled
  interaction, stop the unresolved supplied-file path with the existing typed,
  actionable schema/file diagnostic. Do not launch a provider or continue to
  the guard that would dereference the unresolved partial. Preserve the
  existing non-TTY error surface used by partial resolution.
- Honor `prompt_for_missing`, stdin/stderr TTY gates, and `--silent`.
- Use the frozen caller launch origin for caller-owned file inputs and
  candidate discovery. Do not recapture ambient process CWD after the wrapper
  changes directories. Retain distinct ownership for document-authored and
  `proxy.with` values.
- A chosen value must survive proxy handoff and fresh preparation without
  reinterpreting the original substring relative to a target document or
  prompting again for the same resolved input.
- Preserve existing behavior for ordinary strings, absent/null properties,
  unsupported or ambiguous schema shapes, and lazy output-file references.
  Do not turn every path-looking string into a completion request.

## Implementation Boundaries

Separate supplied-file resolution from missing-value collection and the full
schema verdict at the shared preparation boundary. Reuse the existing typed
classification and CLI chooser rather than creating a second resolver or
putting terminal interaction in the library or expression evaluator.

Audit direct compose, inline-compose, and coordinator-owned proxy entry for
consistent caller-input handling. Preserve sequence preflight and retry/resume
contracts; avoid repeated interactive work when a selected identity is already
installed. This fix does not introduce new sequence interaction modes.

Before modifying symbols, run GitNexus upstream impact analysis and report the
affected callers, execution flows, and risk. The graph consulted during this
evaluation was two commits behind HEAD; source reads underpin the findings
above. Refresh the graph before relying on implementation impact results.

Review affected docs and comments with the code change. In particular, correct
the contradictory “pre-validator always runs” comment in `compose/prep.rs` to
describe the resulting behavior. Update the public composition documentation,
README if its behavior guidance changes, and claudine skill snapshot together.
Render any new terminal status through `TerminalRenderable` components.

## Acceptance and Verification

Use isolated fixture documents and fake providers; tests must not modify the
user's real specification or invoke a paid provider. A dry run alone is not
evidence for this bug because it stops before lifecycle execution.

| Scenario | Required result |
| --- | --- |
| Reported router shape, one matching spec, TTY, `-y` | Confirmation appears before the guard; acceptance lets `frontmatter(spec, ...)` read the selected file and proxy successfully. |
| Same router with several matching specs | Chooser shows matching paths; guard and proxy consume exactly the selection. |
| Only `spec` supplied while router also declares required `plan` and `review` | No unrelated missing-value prompt before routing. |
| Valid literal file, with and without initialize | No completion dialog; existing resolution remains intact. |
| Supplied partial without initialize | Existing confirmation/chooser behavior remains intact. |
| No match, decline, cancellation, non-TTY, disabled configuration, or silent mode | Typed actionable failure, no guard dereference of the partial, no provider spawn. |
| Initialize supplies a missing value or repairs a non-file schema value | Lifecycle still runs before the full schema verdict. |
| Multiple supplied partials; required and eager-optional file properties; supported file-array input | All eligible values resolve with existing shapes and no stale override/provenance values. |
| Launch from a package area, then change runtime CWD or proxy to another directory | Candidates and selected caller identity remain anchored to launch context. |
| Proxy/fresh preparation after selection | Selected identity is retained; no duplicate prompt for that resolved value. |

Add L1 coverage for phase separation, gates, candidate filtering, and override
propagation. Add L2 terminal coverage that reaches the real initialize/proxy
pipeline and exercises acceptance, selection, and cancellation with a fake
provider. Use the shared terminal harness without bringing terminal or browser
windows into focus. Include a shipped-review-router regression fixture, not
only helper-level tests.

Run `just test`, `just test-l2`, and `just lint` in `claudine/` as appropriate to
the implementation. Validate portable path/origin behavior on macOS, Linux,
native Windows, and WSL2 using the repository's OS testing guidance; record
actual evidence and any untested rows. Do not run `cargo fmt` as part of this
fix unless explicitly requested.

Success means the reported partial can be selected before initialize reads it,
the selected file reaches the proxy unchanged, and initialize retains its
ability to bootstrap or route before full validation.

## Implementation Record

Implemented in the shared compose/inline coordinator, with library
classification in `claudine/lib/src/composition/schema/supplied.rs` and CLI
selection in `claudine/cli/src/commands/schema_interactive/supplied.rs`.
The existing chooser and typed diagnostic are reused. Selected overrides and
caller records update together, retaining launch origins across proxy hops.
Sequence proxy entry uses a denied interactive policy.

Root-union applicability defers file existence for caller-owned inputs while
retaining other constraints. Exactly one matching alternative supplies file
metadata; zero or multiple alternatives remain deferred without guessing.

Interactive coverage is split by what each level can observe. The PTY suite
(`level2_provided_partial_file_pty.rs`) drives the shipped router with bytes the
test process manufactures and proves ordering and data flow: the dialog precedes
`initialize`, the chosen file is the file the guard reads, and the choice survives
the proxy hop, across the accept, chooser, decline, cancel, and proxy-target
variants. The shared terminal harness (`level2_provided_partial_file_capture.rs`)
runs the same router and partial inside tmux and WezTerm and reads back what the
emulator drew: the styled candidate card and confirmation are on screen before
any provider is reached or lifecycle error printed, and accepting through the
emulator's own key path carries the selected spec to the provider. A
`#![cfg(windows)]` twin (`level2_windows_provided_partial_file_capture.rs`)
makes the same claim on native Windows through a `cmd.exe` pane and a compiled
provider fixture. All three share the fixture in
`cli/tests/common/review_router.rs`. The Windows run exposed and fixed a
pre-existing defect in the shared partial-matching predicate, which compared
the typed partial against native path spelling; see the Rulings below.

See [plan.md](./plan.md#validation-record) for the regression baseline, completed
macOS checks, and cross-platform verification status. This fix remains in the
active directory while the remaining OS evidence is pending.

## Rulings

- **2026-09-10 — real-terminal coverage (review 1 finding 4, review 2 human-review
  item).** Both reviews recorded that the delivered interactive tests used a
  pseudo-terminal where this specification named the shared terminal harness, and
  deferred the decision. Ruling: the harness coverage is required and was added as
  the rendering complement to the PTY suite (option 2 of review 2), not as a
  replacement for it. The specification's wording stands; it is not loosened to
  treat a pseudo-terminal test as harness coverage.
- **2026-09-10 — native Windows interactive coverage is required, and is now
  met.** Ken declined to accept the missing Windows proof as a deferral. The
  `#![cfg(windows)]` twin (`cli/tests/level2_windows_provided_partial_file_capture.rs`)
  passes on the Windows build host against a headless WezTerm mux server, and
  on its way there found a real defect the non-interactive Windows tests could
  not see: the partial-file substring predicate compared the typed `/`-spelled
  partial against native `\` candidate paths, so every `/`-spelled partial
  missed on Windows and the typed failure fired where macOS offered the
  confirmation. Fixed in `completion/scopes.rs` (portable spelling on both
  sides) with a unit test pinning both spellings. The CI Level 2 gap for
  Windows and WSL2 (`.github/ci/environments.json`, owner and expiry recorded
  there) remains a provisioning item, never an authorization to narrow this
  specification's OS matrix. WSL2 interactive coverage is also met: the Unix
  suites pass unchanged inside the guest in archive mode with tmux mandatory
  (17 of 17). Record:
  [plan.md](./plan.md#native-windows-interactive-coverage--met).
