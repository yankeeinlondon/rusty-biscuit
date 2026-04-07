# Sequence Review

## Findings

1. `[P1]` External sequence files do not use Claudine's normal file-reference resolution.
`claudine/lib/src/composition/sequence.rs:46-52` resolves `sequence: "..."` by doing `source_dir.join(path_str)`. That bypasses the `biscuit_file::FileReference` logic used by the main composition resolver in `claudine/lib/src/composition/resolve.rs:17-25`. As written, examples like `sequence: "@claudine/providers.yaml"` in `claudine/docs/research/non-interactive-sessions/_details.md:2` will not resolve, and neither will other supported reference forms such as magic `@...`, package `!...`, env interpolation, or vault references. This is a functional gap against the design's "same document-centric reference behavior" requirement.

2. `[P1]` `FAIL_FAST` is only injected into Darkmatter preparation, not into the full per-step runtime.
`claudine/cli/src/commands/wrap/sequence.rs:66-94` adds `FAIL_FAST` only to `PrepareOptions.env_overrides`, which reaches `ComposeContext` in `claudine/lib/src/composition/prepare.rs:49-64`. The outer preflight pass in `claudine/cli/src/commands/wrap/sequence.rs:69-85` builds `ComposeOptions` without those env overrides, so `::shell` discovery can see a different environment than the actual composed step. Then the spawned child environment in `claudine/cli/src/commands/wrap/composition.rs:206-224` only gets `OPERATION`; `FAIL_FAST` is never inserted there at all. That means the current implementation does not satisfy the stated contract that `{{env.FAIL_FAST}}`, `::shell`, and the child provider process all see the same value.

3. `[P1]` "Allow once for this sequence run" approval reuse is not actually implemented across steps.
The design called for previously approved commands to remain approved for later steps in the same sequence. The runner does keep a `cumulative_approved` set for template composition in `claudine/cli/src/commands/wrap/sequence.rs:47-48,88-93`, but each step creates a brand-new approval handler/cache in `claudine/cli/src/commands/wrap/sequence.rs:77-85`. That means preflight can still prompt again for the same command on the next step. The same problem exists for harness commands because `execute_composition_request_inner()` creates fresh shell options per step in `claudine/cli/src/commands/wrap/composition.rs:483-487` and reruns harness preflight at `claudine/cli/src/commands/wrap/composition.rs:551-557` with no sequence-level carryover. The current behavior therefore misses the sequence-level "allow once" requirement for both template and harness shell commands.

4. `[P2]` External template support is narrower than the design and silently ignores some invalid configurations.
The implementation uses a custom regex renderer in `claudine/lib/src/composition/sequence.rs:243-264` that only understands `{{key}}` and `{{key || default}}`. The tech design described building a per-item template-evaluation document and using Darkmatter-style interpolation; this implementation will not support richer interpolation behavior that authors already expect elsewhere in composition. It also silently ignores some bad shapes: `template` is treated as `root.get("template").and_then(|v| v.as_object())` at `claudine/lib/src/composition/sequence.rs:191`, so a non-object template is dropped instead of rejected, and any `template` attached to the plain `{ sequence: [...] }` form is ignored because the function returns early at `claudine/lib/src/composition/sequence.rs:156-167`. That leaves misconfigured files failing "softly" instead of clearly.

5. `[P2]` CLI and integration coverage for `claudine sequence` is effectively missing.
The library parser has good unit coverage: `cargo test -p claudine composition::sequence -- --nocapture` passed 18 sequence-focused tests. But there are no sequence-specific CLI tests in `claudine/cli/tests` (`rg -n "sequence" claudine/cli/tests` returns no matches), and `cargo test -p claudine-cli sequence -- --nocapture` did not run any `sequence` integration tests. That leaves the most failure-prone behavior untested: fail-fast stop/continue semantics, the "no sequence property" error path, external file-reference resolution, `FAIL_FAST` visibility in real child env/preflight, and cross-step approval reuse.

## Additional Gaps

- `fail_fast` is read with `.and_then(|v| v.as_bool()).unwrap_or(true)` in `claudine/lib/src/composition/sequence.rs:31-35`, so an invalid type like `"false"` silently falls back to `true` instead of producing a validation error. That is likely to be confusing in real usage.

## Ergonomics And Performance

- The step loop currently performs multiple independent passes over the same step state: an outer template-shell preflight, a full `prepare_direct()` composition, then a later harness parse/preflight inside the single-step executor. Once the correctness issues above are fixed, this area is the main place to reduce repeated work.
- `SingleCompositionOutcome` was introduced for richer step results, but the sequence runner still calls the exit-code-only wrapper. That leaves `provider` and `selection_reason` unused and currently produces a dead-code warning during `cargo test -p claudine-cli sequence -- --nocapture`. Either consume the richer result in sequence reporting or remove the unused fields until they are needed.

## Suggested Test Additions

- Add CLI/integration tests for `--fail-fast true` stopping on the first failing step and `--fail-fast false` continuing but returning exit code `1`.
- Add an integration test that uses `sequence: "@claudine/providers.yaml"` or another non-trivial file reference so resolution matches the documented contract.
- Add a test that proves `FAIL_FAST` is visible consistently in preflight, composed prompt interpolation, and the child process environment.
- Add a sequence test with repeated shell commands across multiple steps and assert that a single "allow once" approval covers the whole run, including harness shell commands.
