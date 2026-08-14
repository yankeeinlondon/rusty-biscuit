---
status: draft
created: 2026-08-13
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-13
review_iterations: 1
depends-on: ../../claudine/fixes/_completed/2026-08-12-ctx-launch-anchor/spec.md
area: claudine
packages:
    - claudine
    - claudine-cli
    - darkmatter
---

# Finalize fix/ctx-launch-anchor: clear the blocking cells of run 31651014023

## Summary

The last completed CI run for `fix/ctx-launch-anchor` left a set of blocking
cells that mixes two populations. The per-OS investigations (`windows.md`,
`linux.md`, `macos.md`, and `wsl.md`) and the consolidated `problems.md`
establish that four problem groups require disposition on this branch:

- **P1** — one launch-epoch regression test reads the raw `ComposeContext`
  backing map even though target `AGENT`/`MODEL` overrides are defined on the
  effective view. The prepared composition itself already consumes that
  effective view; this is a test-contract defect, not evidence of ambient
  identity leaking into composed `ctx.agent` or `ctx.model`.
- **P2** — the shipped-prompt hash pin went stale when the merge from main
  combined two independent edits to `prompts/_implement/implement-plan.md`.
- **P3** — the PR's new Windows tests exposed two real Windows product defects
  (CommonMark consuming backslashes in path-shaped interpolation and `\\?\`
  verbatim-prefix leakage), one broken test-stub layer (`.cmd` shims), and one
  quoting-expectation mismatch.
- **P4** — three real-composition tests (router target and two `wrap_perf`
  parity tests) now exceed the 90-second test budget on an Ubuntu cell that was
  green on main. The correlation warrants measurement, but does not yet prove
  launch-context capture is the cause.

Everything else that blocked (WSL2 environment assumptions, sniff
non-hermeticity and lints, dmls Neovim provisioning, rendezvous-daemon SDDL
brittleness, and the unchained-ai ConPTY stall) was verified byte-identical on
main's baseline run 31588186544 and is main drift this branch merely
scheduled. This spec repairs P1-P4 and defines a policy decision for the
main-drift cells so the merge gate can return a trustworthy verdict.

> **Reader's note:** Review corrected four draft conclusions. F1 now preserves
> Darkmatter's established raw-versus-effective context contract instead of
> baking target overrides into the raw launch snapshot. F3 is treated as a
> public interpolation-semantics decision rather than a small escaping patch.
> F6 uses Darkmatter's existing canonical command normalizer. F7 no longer
> asks separate CLI processes to share an in-memory invocation epoch, which is
> impossible by construction.

## Scope

**In scope (changes on this branch):**

1. P1 assertion correction and deterministic test hygiene; no change to
   `ComposeContext::get()` or launch evidence.
2. P2 fixture re-derivation and hash-pin refresh.
3. P3a Darkmatter interpolation fix after Open Question 1 is decided; P3b
   Claudine path projection fix; P3c native Windows test-fixture replacement;
   P3d expectation alignment.
4. P4 capture-cost measurement with a bounded outcome: fix a measured
   regression, or restructure the tests without weakening their assertions.
5. If Open Question 2 approves it, temporary entries in
   `.github/ci/ci-baseline.toml` for the main-red P5/P6 cells, with exact source
   runs, owners, reasons, and expirations.

**Out of scope (main-side follow-ups, enumerated for handoff in
`problems.md`):** sniff `ExecutableIndex` hermeticity, sniff/sniff-cli clippy
cleanups, dmls Neovim probe compatibility, messenger D-Bus skip-arm widening,
rendezvous-daemon SDDL assertion rewrite, unchained-ai ConPTY shutdown
ordering, the L2 onboarding-wizard/config seeding gap, the code-block
color-mode contract on CI tmux, and the sniff-cli/darkmatter-cli Windows path
rendering siblings of P3a/P3b. This branch may record or link follow-up work,
but must not absorb those implementation fixes.

## Fix specification

### F1 — Assert target identity through the effective context view (P1)

**Established contract.** `ComposeContext::get()` reads the immutable raw
capture backing map. `ComposeContext::as_object()` and internal
`get_effective()` apply compose-time `AGENT`/`MODEL` overrides from the
context's environment. `PrepareOptions::env_overrides` are applied to the
prepared context before composition. This split is intentional and is also
required by the completed launch-anchor spec: launch evidence is captured
once, then the resolved target's identity overlay is applied without
re-capturing or re-anchoring that evidence.

**Change.** Correct
`stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity` so
both the first preparation and stabilized reread assert `agent` and `model`
through `as_object()`. Raw `.get()` assertions remain appropriate for
launch-captured facets such as `area` and newly extended `os`; they must not be
used to assert target overlays.

Do not fold `input_layers.env_overrides` into
`ContextCaptureEvidence`, repopulate raw `values.agent`/`values.model` during
extension, or change `ComposeContext::get()`. Those approaches would collapse
the raw/effective distinction, contradict the prior spec's apply-after-capture
ordering, and make target selection part of immutable launch evidence.

The regression test must not mutate process-global environment variables
inside the parallel test process. Its seeded context environment and
`CallerInputLayers::env_overrides` are sufficient to make the effective-view
assertions deterministic. If additional coverage is needed, use supplied
evidence or separate test processes rather than `set_var`/`remove_var` in an
unserialized test.

**Acceptance.** The first and extended epoch contexts expose `codex` and
`gpt-test` through `as_object()` while retaining one launch construction, one
extension, and zero ambient fallbacks. The test passes when the host process
has `AGENT`/`MODEL` unset or set to unrelated values.

### F2 — Re-derive and re-pin the shipped implement-plan fixture (P2)

Follow the drift test's own review workflow:

1. Diff shipped `prompts/_implement/implement-plan.md` against
   `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`.
   Mirror the merged `phase:` fallback into the fixture and confirm that the
   `{{iteration}}` to `{{phase}}` edits remain inside the `success.stack` block
   the fixture intentionally omits.
2. Verify the resulting shipped hash with `md hash
   prompts/_implement/implement-plan.md`.
3. Run `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p
   claudine-cli --test shipped_prompt_route_drift` from the repository root.
4. Re-run without the update variable and confirm
   `fixture_preserves_the_shipped_schema_and_loop_semantics` also passes. The
   update mode is not itself verification because it mutates the expected pin.

**Acceptance.** Both drift tests pass on the normal, non-update run. The PR
description records that main's pin is independently stale after 69a15f6c0;
the branch refresh absorbs that merged edit here, while main receives a
separate handoff if this branch does not land first.

### F3 — Preserve interpolated Windows paths through Markdown composition (P3a)

**Contract.** A scalar inserted into body prose must retain its literal text
semantics through Darkmatter's later CommonMark cleanup pass. For a
Windows-shaped value such as
`C:\Users\x\AppData\Local\Temp\.tmpZZZ\repo`, parsing the composed Markdown
must yield that exact text. The serialized Markdown source may contain the
escaping required to express that text; tests must distinguish source spelling
from parsed text semantics.

The contract is syntax-aware:

- prose and inline-code interpolation must preserve the value's literal text;
- frontmatter and Darkmatter directives must continue to receive the raw value
  required by their own parser or executor;
- fenced/indented code remains uninterpolated unless explicitly enabled, and
  opted-in code interpolation preserves raw code bytes; and
- intentional Markdown generation must remain possible through an explicit
  contract rather than accidental string injection.

This is a Darkmatter-wide behavior decision. Select Open Question 1 before
implementation; do not apply a global `\` to `\\` rewrite. A global rewrite
would double backslashes in code/directive contexts and still fails to define
whether interpolated Markdown is data or syntax.

**Required coverage after the decision:**

1. An OS-independent Darkmatter unit matrix using a drive path, UNC path, a
   hidden directory segment, and every CommonMark-escapable punctuation class
   that can follow a Windows separator.
2. Separate prose, inline-code, opted-in-fence, frontmatter, `::shell`, and
   transcluded-child cases. Preflight and execution must observe identical raw
   command bytes.
3. A test that locks the chosen intentional-Markdown escape hatch so the fix
   does not silently flatten authored formatting.
4. A passive shipped-prompt corpus test and a Claudine end-to-end compose test
   through the normal invocation path.
5. Documentation updates in `darkmatter/docs/inline/interpolation.md` and the
   corresponding Darkmatter skill snapshot when public semantics change.

**Acceptance.** The five P3a regressions pass on `windows-latest`; the
OS-independent matrix passes on macOS, Linux, and Windows; and no Unix output
or directive command bytes drift unintentionally.

### F4 — Keep `\\?\` verbatim paths out of projections (P3b)

**Contract.** As documented by `invocation_context::canonical_key`, Windows
verbatim paths are comparison/cache-key forms. They must not reach authored
path comparisons, prepared `ctx.*`, `SystemPromptContext`, or
`PreparedSystemPrompt::source`.

**Change.** Replace `system_prompt/context.rs`'s direct
`std::fs::canonicalize` projection with the crate's existing dunce-style path
normalization. Prefer `dunce::canonicalize` for existing paths and a
`dunce::simplified` fallback for non-existing authored paths; `dunce` is
already a `claudine` dependency. Keep `canonical_key` unchanged for internal
cache equality.

Apply the same projected-path helper to fixture expectations in
`ctx_launch_anchor_baseline.rs`; do not teach tests to expect `\\?\`. Review
the behavior comments at both helpers and remove any claim that direct
`std::fs::canonicalize` is safe for projected values.

**Acceptance.** A native-Windows unit test proves drive and UNC projections
are simplified while cache keys still compare canonically.
`normal_session_composes_the_shipped_root_system_prompt_from_launch_context`
passes, and no prepared context value or `prepared.source` in the Windows
regression logs begins with `\\?\`.

### F5 — Replace the broken Windows test stubs (P3c)

1. Make `printf.cmd` end with `exit /b 0`; `<nul set /p` reaches EOF and leaves
   `ERRORLEVEL=1` even when it emits the expected bytes.
2. Replace provider `.cmd` trampolines with a real native test-fixture
   executable. The fixture must support Codex stdin capture, Goose argv/stdin
   capture, generated response output, and the loop invocation counter used by
   this test module. Build it as test support and stage or copy it under the
   provider executable names; it must not add a second production CLI binary.

Do not rely on the draft's claimed `current_exe`/L2 pattern: there is no
`feedback_l2_probe_no_production_bin` fixture in this repository, and a normal
libtest executable cannot accept arbitrary provider argv before libtest parses
it. A re-exec solution is acceptable only if it has a real pre-libtest fixture
entrypoint and is proven on Windows. Do not compile the helper with `rustc` at
test runtime; archived/guest test environments are not guaranteed to carry a
toolchain.

**Acceptance.** The native fixture proves multiline argv and stdin transport
without a `.bat`/`.cmd` hop. All five currently P3c-affected
`ctx_launch_anchor_baseline` tests pass on `windows-latest`, and unblocking the
stub reveals no new path expectation failures.

### F6 — Derive the JIT preflight expectation canonically (P3d)

The quoting layer is already established:
`darkmatter::markdown::compose::shell_expansion::policy::normalize_command`
quotes every argument containing a backslash and escapes backslashes inside
that quoted normalized form. The Windows CI output matches that contract.

Build the expected command from executable plus argument vector with
`normalize_command` instead of formatting a raw string. Do not normalize the
approval set a second time and do not change product quoting; approval and
execution already compare the canonical normalized command.

**Acceptance.** The test asserts the same argument vector on every OS and
`template_preflight_combines_launch_facts_with_the_selected_target` passes on
`windows-latest`.

### F7 — Bound the real-composition latency regression (P4)

The Ubuntu `claudine-cli` cell was green on main, while the three subject tests
exceeded nextest's 90-second per-test limit and pass locally in about 23
seconds. Treat launch-context capture as a hypothesis until timings identify
it.

1. Record per-process wall time and the existing `--perf` stage breakdown for
   each invocation in the two parity tests. Add focused timing around
   `InvocationContext::capture_at`/`capture_for_wrapper`, repository
   observation, topology initialization, `capture_launch_context`, system
   prompt preparation, and provider handoff. Measure both a repository launch
   and the non-repository temporary `HOME`; do not infer the launch CWD from
   `HOME`.
2. Within each CLI process, assert the completed launch-anchor work-count
   contract (one launch observation and no duplicate topology probe for one
   repository). Separate CLI invocations are separate processes and are
   expected to construct separate `InvocationContext` values.
3. If measurement shows duplicate or super-linear discovery inside one
   process, fix that production cost and add a work-counter regression. If
   each invocation is bounded but a test exceeds 90 seconds only because it
   launches multiple processes serially, replace the pairwise test with
   per-mode tests that each assert the same fixed stdout fixture. Merely
   splitting the test while dropping the parity assertion is not acceptable.
4. Do not raise the global nextest timeout. A test-specific timeout increase is
   a last resort and requires measurements showing one irreducible invocation
   needs the larger budget on a standard two-core runner.

**Acceptance.** `shipped_implement_prompt_runs_real_router_target` and both
perf/non-perf output contracts pass on `ubuntu-latest` in two consecutive CI
runs. The PR notes include the per-stage measurements, invocation work counts,
and the reason for either the production fix or test restructuring.

### F8 — Disposition the main-drift blocking cells (P5/P6)

The baseline is keyed by `{package, environment, tier}`, not by individual test
identity. Adding one entry accepts a failed cell as a whole until expiration,
so it can mask a new regression in that same cell. Any baseline change must
therefore be paired with an exact identity diff against both source run
31588186544 (including its WSL2 leg) and branch run 31651014023.

If Open Question 2 approves temporary baselining, add entries only for the
verified main-red cells: `claudine/wsl2-ubuntu/L1`,
`sniff/wsl2-ubuntu/L1`, `messenger/wsl2-ubuntu/L1`, `sniff` L1 on all three
GitHub-hosted OSes, `sniff/ubuntu-latest/lint`,
`sniff-cli/ubuntu-latest/lint`, `dmls/ubuntu-latest/L2`,
`rendezvous-daemon/windows-latest/L1`, and
`unchained-ai/windows-latest/L1`.

Every entry must include `owner = "@yankeeinlondon"`, a reason naming the
observed failure family, the exact `source_run`, and the ratified expiration.
Use the same WSL2 run identifier cited by `wsl.md`, not the phrase "its WSL2
leg." Do not duplicate an existing entry. Record main-side follow-ups for the
cheap mechanical fixes, but keep their implementations out of this branch.

**Acceptance.** `ci-rollup` accepts each entry against a scheduled `FAIL` and
reports neither `baseline-no-result` nor `baseline-now-passing`. `just ci-diff`
shows no new failing test identity hidden inside an accepted cell. Passing or
expired entries block as required by `.github/ci/README.md`.

## Phases

1. **Phase 1 — Cross-platform corrections:** F1 and F2. Verify the normal
   non-update hash test after re-pinning.
2. **Phase 2 — Decide and implement Windows product behavior:** resolve Open
   Question 1, implement F3 and F4, update interpolation documentation, and run
   the OS-independent tests locally. Native-Windows proof comes from CI.
3. **Phase 3 — Windows test infrastructure:** F5 and F6. Batch these with
   Phase 2 so one Windows CI round trip exposes the complete next failure
   layer.
4. **Phase 4 — Latency:** measure and resolve F7 independently of the Windows
   work.
5. **Phase 5 — Gate policy:** resolve Open Question 2 and, if approved, apply
   F8 only after the PR-subject tests are green.
6. **Phase 6 — Verification:** run the package-area L1/lint gates, then full CI
   and `just ci-diff`. Repeat full CI once for the F7 family.

This macOS host cannot provide native Windows runtime proof. Windows-facing
tests must run through non-interactive CI and must not open or focus terminal
or browser windows.

## Verification matrix

- `cd darkmatter && just test && just lint` for F3 and its public contract.
- `cd claudine && just test && just lint` for F1, F2, F4, F5, F6, and F7.
- The targeted shipped-prompt drift command in F2, followed by a normal run
  without `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES`.
- Native `windows-latest` L1 cells for `darkmatter`, `claudine`, and
  `claudine-cli`.
- Two consecutive `ubuntu-latest` `claudine-cli` L1 runs for F7.
- Canonical Level 2 in the Claudine and Darkmatter package areas because this
  fix changes terminal-visible composed values, context descriptors, and
  rendering inputs. Run through each package area's `just test-l2` recipe
  without focusing a host window.
- Full `ci-verdict` plus exact `just ci-diff` review for F8. A green verdict
  alone is insufficient because a cell-wide baseline can hide new identities.

The pre-CI local acceptance record was Level 1 only. CI run `31753281913`
subsequently exposed failures in terminal-visible surfaces, so the affected-
package Level-2 gate above supersedes the earlier Level-1-only judgment. No
browser behavior is changed; browser tests remain unnecessary unless the
implementation expands into that surface.

## Success criteria

1. Every P1-P3 regression in `problems.md` is green on its blocking macOS and
   Windows cells.
2. Parsed composed Markdown preserves Windows path text, directive consumers
   receive the intended raw bytes, and the intentional-Markdown behavior
   selected in Open Question 1 is documented and tested.
3. The F7 family is green on `ubuntu-latest` in two consecutive runs with
   measurements explaining the chosen resolution.
4. PR-subject cells are green before any F8 baseline is applied.
5. The merge gate reports no unapproved blocking cells, and every accepted
   main-drift cell has source evidence, an owner, a reason, and an unexpired
   policy entry.
6. No new red test identity appears relative to run 31651014023, including
   inside cell-wide baseline entries.
7. Relevant comments and public interpolation documentation agree with the
   implemented behavior.

## Ratified Decisions

### 1. Body-interpolation values are literal text by default

**Decision (ratified 2026-08-13): Option A.** Body-prose interpolation treats
evaluated values as literal text. Intentional Markdown structure requires the
explicit `raw_markdown(value)` escape hatch.

The corpus audit found a bounded migration surface. Three shipped prompt body
expressions intentionally generate Markdown lists and must use the escape
hatch when F3 lands:

- `prompts/faster-builds-and-tests.md:17`
- `prompts/code-comment-quality.md:18`
- `prompts/context.md:34`

All three call `as_unordered_list`. Representative downstream Claudine research
fleet documents use scalar/path interpolation in their bodies and require no
migration. Frontmatter render helpers retain their parser-specific raw values,
and fenced documentation examples are not body-prose interpolation sites.

This is a major Darkmatter contract decision. The current string-first
pipeline lets a value inject Markdown syntax accidentally, then cleanup parses
that value as authored source. That is why a Windows separator before `.` is
lost.

**Option A — Literal text by default, explicit `raw_markdown(value)` escape
hatch (recommended).** Body-prose interpolation inserts evaluated scalars as
literal text in the Markdown event model; an explicit function is required to
generate structure.

- **Pros:** establishes a safe, predictable default; fixes every punctuation
  class rather than only `\.`; makes data-versus-syntax intent reviewable; and
  suits this repository's low-cost refactoring stage before external adoption.
- **Cons:** intentionally changes existing documents that rely on implicit
  Markdown injection; requires a shipped-prompt corpus audit, an expression
  wrapper/value representation, and syntax-aware handling for directives and
  code contexts.

**Option B — Preserve only path-shaped replacement tokens through cleanup.**
Recognize Windows drive/UNC path values and protect their backslashes across
the CommonMark parse/serialize boundary.

- **Pros:** smallest compatibility impact and directly fixes the observed
  regression.
- **Cons:** path recognition is heuristic, risks missing quoted/extended/UNC
  forms, leaves other interpolated punctuation vulnerable, and creates a
  special semantic class not visible in the expression language.

**Option C — Keep raw Markdown interpolation and add an explicit
`markdown_literal(value)` helper.** Update path-bearing prompts/tests to call
the helper.

- **Pros:** minimal engine change and no migration for intentional Markdown
  injection.
- **Cons:** preserves a dangerous default, makes ordinary `ctx.repo_root`
  interpolation non-portable, and pushes a platform trap onto every document
  author.

The audit did not reveal a prohibitive compatibility surface. F3 therefore
proceeds with Option A and the three migrations above; it must not fall back to
a path heuristic or global backslash rewrite.

### 2. This branch may add evidence-backed, short-lived CI baselines

**Decision (ratified 2026-08-13): Option A.** Eligible main-drift cells may be
baselined through `2026-09-30` only when their exact failing identities have
been diffed against the source and branch runs. A cell without identity-level
diff evidence must follow Option B and receive its main-side fix before this
branch proceeds.

**Option A — Add short-lived entries and file main-side follow-ups
(recommended).** Use exact source-run evidence, set expiration to 2026-09-30,
and require identity-level `ci-diff` review on this branch.

- **Pros:** unblocks this branch without absorbing unrelated fixes; gives each
  debt item an owner and deadline; preserves visibility because producer cells
  remain red.
- **Cons:** a cell-wide entry can mask a new failure until `ci-diff` catches it,
  and it grows policy debt temporarily.

**Option B — Land the main-side fixes first, then rebase this branch.** Add no
new baseline entries here.

- **Pros:** keeps the baseline smaller and avoids masking any cell-level
  regression.
- **Cons:** delays the launch-anchor fix behind several unrelated packages,
  including Windows-only issues that need their own CI iterations.

The shorter expiration avoids inheriting the existing November migration
horizon. Phase 5 remains responsible for exact source-run metadata, ownership,
and per-cell identity evidence.
