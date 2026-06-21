---
ready: false
agent: codex/default
created: 2026-06-20T13:12:17
implemented: true
---

# Review 4

Not ready for production. The original invalid whole-value `{{ ... }}` leak appears covered by Level 1 Darkmatter and Claudine tests, and the targeted regression checks I ran pass. However, the newly added best-effort preflight interpolation has a functional hole that still produces `NotPreApproved` for valid documents when the frontmatter shell command depends on the same context-requiring key that best-effort skipped.

## Findings

### High: best-effort preflight can approve the wrong command when a shell command depends on the skipped key

The best-effort path in `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:387` marks a key as resolved after an evaluation error but intentionally does not insert any value into `seed_map`. That works for the current regression test because the failing key is only a sibling and the shell command can resolve from `spec`. It breaks when the shell command references the failed key directly.

`darkmatter/lib/src/markdown/compose/preflight/collect.rs:524` calls this best-effort interpolation with `resolution_context = None`. For a document like:

```yaml
---
exists: "{{ file_exists('existing.md') }}"
cmd: "$(echo '{{ exists }}')"
---
```

preflight cannot evaluate `exists`, marks it resolved anyway, then resolves `cmd` with `exists` missing and discovers `echo`. Real composition later has a resolution context, evaluates `exists` to `true`, and tries to execute `echo true`. I reproduced the user-visible result through `md compose`: with `exact echo` in `.darkmatter-shell-whitelist`, preflight reports `echo`, then composition fails with:

```text
Command 'echo true' at frontmatter.cmd was not pre-approved ... This is a bug in the pre-flight scanner
```

This is the same class of failure the post-review change is trying to eliminate: the approval set is no longer a faithful superset of execution. The tests only cover an unrelated failing sibling at `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:1030`; they need a dependent-key regression where the skipped context-requiring key feeds a `$(...)` command. The fix should avoid treating errored keys as usable dependencies for other templated keys, or otherwise preserve an unresolved/deferred state that prevents collecting a command with missing data.

Verification level: Level 1 is the correct tier. This is parser/composition/preflight behavior, not terminal rendering or keyboard input.

## Requirement Coverage

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Whole-value `{{ ... }}` parse/evaluation failures are fatal even with `fail_fast = false` | Level 1 Darkmatter unit tests plus Level 1 Claudine CLI regression | OK |
| Typed whole-value `{{ ... }}` results are preserved | Level 1 Darkmatter unit tests | OK |
| Mixed malformed interpolation remains lenient | Level 1 Darkmatter unit test | OK |
| Whole-value `$()` parse/expansion failures are fatal when shell expansion is enabled | Level 1 Darkmatter unit tests | OK |
| Padded whole-value `$()` values expand when shell expansion is enabled | Level 1 Darkmatter unit tests | OK |
| Raw expansion syntax does not appear in successful effective frontmatter | Level 1 leak-guard tests plus Level 1 Claudine dry-run regression | OK |
| Preflight approval set remains a superset of real execution | Level 1 tests cover sibling-key case only | Gap |
| Required package tests pass | Targeted Level 1 tests passed; full `just test darkmatter` was interrupted at the non-interactive 60s ceiling after 1499/4614 passing tests | Not verified |

## Verification Performed

- `cargo nextest run -p darkmatter collects_resolved_command_despite_context_requiring_sibling_key whole_value_parse_failure_is_fatal_without_fail_fast execute_expands_padded_whole_value execute_aborts_on_padded_malformed_whole_value` passed: 5 tests run, 5 passed.
- `just test darkmatter` was started to verify acceptance criterion 7. It exceeded the non-interactive session ceiling while still running, so I interrupted it. At interruption, 1499 tests had passed, 3115 had not run, and no failure had appeared before the interrupt.
- Manual Level 1 CLI reproduction of the dependent-key preflight gap produced `Command 'echo true' ... was not pre-approved`.

No Level 2 or Level 3 tests are required for this spec. The observable behavior is compose preparation, shell-command preflight, diagnostics, and effective frontmatter content; Level 1 is the appropriate verification tier.
