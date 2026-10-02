---
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation log: 2026-07-22-setters

## Phase 1 rulings (2026-10-01, Ken Snyder)

Dependency state at ruling time: `2026-07-13-cli-switches` has a plan but no
implementation log. Neither R8 (switch metadata) nor R9 (the shared ownership
function) is in `lib/` or `cli/`. `partition_composition_tail` still forwards
everything after the first unowned switch (`cli/src/argv/partition.rs`, the
`Ownership::Unowned` arm).

1. **Sequencing: wait for R9.** Phases 1 and 2 (rulings, spikes, red
   regression suite) may run now. Phase 3 onward is blocked until R9's shared
   ownership function is on the branch. No interim stopgap.
2. **Injectable switch metadata: required.** R9's ownership function takes the
   switch-metadata lookup as data, so unit fixtures use controlled switch
   types. Binary tests use real generated metadata, with fixtures chosen from
   it. This is a requirement on the dependency's R9 design.
3. **Reclaimed setters keep their argv positions.** Reclaimed setters return
   to the Claudine argv in their original relative order, so
   `parse_composition_positionals` collects them in one pass and last-wins
   comes from the existing `Map::insert` order. No separate merge path.
4. **Schema-claimed setter vs. missing value: accepted default (spec wins).**
5. **Dotted keys are not setters: accepted default (spec wins).**
6. **`argv=` after a provider switch: accepted default (dependency owns it).**
7. **Spec status:** set to `planned`.
8. **Measurement: accepted default.** No benchmark; any wider measurement is
   the author's call.

## Phase 1

Run 2026-10-02, macOS host. No source code changed in this phase.

### Spike A: dependency state (contradicts the ruling-1 premise)

The rulings above say neither R8 nor R9 has landed. **That is no longer true.**
`2026-07-13-cli-switches` has all seven phases checked in its `plan.md` and is
in its review cycle (`review-1.md` … `review-3.md`; review-2/3 and some of its
fixes are still uncommitted in this worktree). Relevant commits on this branch:
`618c6965b`/`70e6584f0` (R8 catalog and lookup), `1b3c87762` (regenerated
catalog, ten providers), `c4fb12c3d` (R9 ownership), `3f3d2e64b` (partition
rewrite), `89502294a`/`3cf8fd68a` (completion and review fixes).

What has landed, and the functions later phases must call:

- **R8 lookup:** `claudine::provider::{lookup_switch, match_switch_token}`
  over the generated `ProviderInfo::cli_switches`.
- **R9 ownership:** `claudine::composition::own_arguments(&ArgumentsAfterFile,
  &SchemaParameters, &[OwnershipCandidate]) -> Result<OwnedArguments,
  OwnershipError>` in `lib/src/composition/ownership.rs`. `OwnedArguments`
  holds `claudine: Vec<String>` (setters and positionals, in original order)
  and `tail: ProviderTail` (with per-switch `SwitchAssignment`s).
- **Resolved-provider check:** `composition::check_launch_tail(&ProviderTail,
  Provider, &[&str]) -> Result<(), TailMismatch>`, wired through
  `SwitchContext::check` at preflight, before each `sequence` step, and in
  `harness_orch/launch.rs::build_harness_launch` before every spawn.
- **CLI glue:** `cli/src/commands/compose/ownership.rs::own_caller_arguments`
  (shared by all three commands) reads the authored `$schema`/`agent`;
  `cli/src/argv/partition.rs::partition_composition_tail` now only splits off
  Claudine options (leaving a `CallerArgument::ClaudineOption` marker) and the
  authored `--`. The "first unowned switch forwards everything" rule is gone and
  the "Ownership model" module docs already describe the new behavior.
- **Setter shape:** `composition::setter_key` (lib). `argv::looks_like_setter`
  already delegates to it and its doc comment was already updated.
- **Ruling 2 (injectable metadata) is met, but only crate-internally.** The
  seam is the private `SwitchSource` trait and `own_in`/`check_in`; the lib's
  own tests use `struct Fixed` over `CliSwitchCatalog::Researched(...)` via
  `match_token_in` (`lib/src/composition/ownership/tests.rs`). Controlled
  metadata is therefore reachable only from tests inside the `claudine` lib
  crate, not from `cli/src/argv/partition/tests.rs`.

**The headline defect is already fixed.** Reproduction with the built binary
(spec "Reproduction", shell-free `plan.md`): both the control
(`phase=2 --codex -c 'model_reasoning_effort="medium"'`) and the defect
ordering (`--codex -c 'model_reasoning_effort="medium"' phase=2`) render
`Phase 2`, and the stderr "Provider args" row is
`-c model_reasoning_effort="medium"` in both.

Checked against the spec's ownership table by reading `own_in`: rows 1, 2, 3,
5, 6, 7 (fails), 8, 9, `-c x=y -m gpt5 phase=2`, the interrupted variadic run,
`-c phase=2 x=y` (fails, no reattach), and the `--` escape hatch all follow
from the existing rules. Gaps found:

1. **Missing-value error does not name the setter key.** `compose plan.md
   --codex -c phase=2 --dry-run` with `phase` declared prints
   "provider argument `-c` takes a value for Codex (at its `exec` command), but
   none was forwarded with it. A `key=value` the document declares as a
   parameter is Claudine's, never the switch's value. Name the provider (for
   example `--codex`), or pass provider arguments after `--`." The spec requires
   `phase` to be named and guidance to "supply a separate provider value".
   `TailMismatch` has no field for the conflicting key, and the shared `ESCAPE`
   suffix suggests naming a provider even when one was named. Phase 3 work.
2. **Two copies of the setter key grammar.** `composition::setter_key` (lib)
   and `parse_compose_setter` (`cli/src/commands/compose/setters.rs`) encode
   the same grammar and differ on an empty key: `=v` is `None` for
   `setter_key` (so ownership treats it as a bare word) but
   `Some(Err("setter key must not be empty"))` for `parse_compose_setter`.
   This is the "setter shape check shared, not copied" task in Phase 3.

### Spike B: fake-provider fixtures

- Fake providers that record exact argv already exist: `stub()` + `launches()`
  in `cli/tests/l1/provider_tail_ownership.rs` and `provider_tail_launch.rs`
  (each launch written `\x1f`-separated to `<home>/launches/launch-NNN`).
  Both files are `#![cfg(unix)]` because the stubs are `/bin/sh` scripts.
  `compose_caller_file_provenance.rs` has `install_retrying_goose` and
  `install_resumable_claude` (unix) for retry/resume.
- `--dry-run` needs no provider installed and renders the redacted tail as the
  stderr "Provider args" row (`wrap/composition/dry_run.rs`). Shown above.
- **Minimal extension: none needed for macOS/Linux.** Phase 4's binary tests
  can reuse these helpers (copy the `stub`/`launches` pattern, or lift both
  into `tests/common/` if a third file needs them). Native Windows has no
  argv-recording fake provider; the dry-run reproduction is the one binary
  test that can run on every OS, and the routing itself is covered portably by
  the lib unit tests.

### Baseline

From `claudine/` on macOS: `just test` → 8247 passed, 9 skipped, 0 failed
(57 s run). `just lint` → exit 0. No pre-existing failures.

### Guards

The plan names six partition tests. Four were removed or renamed when the
dependency rewrote the partition (`2c7f98dcf`, `3f3d2e64b`):

| Plan name | Status | Successor and what it protects |
| --- | --- | --- |
| `reported_command_forwards_config_switch` | removed | `the_headline_command_forwards_one_setter_and_applies_the_next` (`cli/src/argv/partition/tests.rs`) and `a_string_switch_takes_one_setter_and_the_next_setter_is_claudines` (`lib/.../ownership/tests.rs`): `-c model_reasoning_effort=low` reaches Codex unchanged; `phase=2` stays Claudine's |
| `setter_before_tail_stays_with_claudine` | removed | `setters_before_the_file_stay_with_clap`, `positionals_are_left_for_argv_in_order` |
| `owned_flag_after_tail_is_reclaimed` | removed | `a_claudine_option_after_a_provider_switch_is_reclaimed_and_marked` |
| `explicit_separator_forwards_opaque_tail` | renamed | `explicit_separator_forwards_an_opaque_tail`, `only_the_first_separator_is_consumed` |
| `switch_before_file_errors` | present | switch before the file is an ordering error |
| `separator_before_file_errors` | present | `--` before the file is an ordering error |

Other guards, all present (the `tests/l1/` files are declared in
`cli/tests/l1/main.rs`):

- `cli/src/commands/compose/tests.rs` unit setter tests (`setter_*`, `shorthand_value_*`,
  `positionals_*`, `merge_shorthand_wins`): key grammar, JSON5-then-string
  values, empty value, first-`=` split, dotted key rejected, last-wins,
  shorthand over `--set`, `argv` setter rejected.
- `tests/l1/argv_normalization.rs`: pre-clap normalization rules.
- `tests/l1/wrap_direct_argv.rs`: direct-wrapper argv is unchanged (out of
  this fix's routing scope).
- `tests/l1/completion_setter.rs`: setter completion.
- `tests/l1/compose_caller_file_provenance.rs`: caller provenance through
  proxy, proxied retry/resume (unix), loops, and sequence steps.
- `tests/l1/provider_tail_ownership.rs` and `provider_tail_launch.rs`: the
  dependency's binary tests of ownership, the resolved-provider check, and
  exactly-once tail forwarding through retry, proxy, resume, and sequence.

### Rulings: status after the spikes

- Ruling 1's blocker is resolved: R9 is on the branch, so Phase 3 is not
  blocked. Its intent (no interim stopgap, no second parser) still holds.
- Ruling 2 is met by the private `SwitchSource` seam (see Spike A).
- Rulings 3–8 are unaffected.
- The spec acceptance bullet naming `reported_command_forwards_config_switch`
  refers to a removed test. Its successors (table above) protect the same
  behavior; the spec is left as written (it is a snapshot) and this mapping is
  the evidence.
