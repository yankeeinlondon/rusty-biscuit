---
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/lib/src/composition/ownership/tests.rs
    - claudine/lib/src/composition/ownership/tests/setters.rs
    - claudine/cli/src/argv/partition/tests.rs
    - claudine/cli/src/commands/compose/tests.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
docs_updated_during_phase_2:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/ownership.rs
    - claudine/lib/src/composition/provider_tail.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/ownership/tests/setters.rs
    - claudine/cli/src/commands/compose/setters.rs
    - claudine/cli/src/commands/compose/tests.rs
    - claudine/cli/src/completion/engine/tokens.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/switch_catalog_guard.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
packages:
    - claudine
    - claudine-cli
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

## Phase 2

Run 2026-10-02, macOS host. Tests only: no production code changed.

### Where the suite lives

Spike A showed controlled switch metadata is reachable only inside the
`claudine` lib crate (private `SwitchSource`), so the suite is split by what
each row needs:

| File | What it covers | Catalog |
| --- | --- | --- |
| `lib/src/composition/ownership/tests/setters.rs` (new, declared by `mod setters;` in `ownership/tests.rs`) | the whole setter ownership table plus acceptance extras, the resolved-to-Claude check, the missing-value rows, the schema-source matrix, the unestablished/unreadable schema | controlled (`CONTROLLED`, scoped by command path) |
| `cli/src/argv/partition/tests.rs` | rows that need the pre-clap partition (`--yolo`, `-m gpt5` between provider tokens, `--config=x=y`, `--` with a declared `phase`), row 4 with a no-value switch chosen from the generated metadata (Codex `--ephemeral` at `exec`), the every-provider union then the Claude check | real compiled catalog |
| `cli/src/commands/compose/tests.rs` | setter semantics through the real override path: partition → `own_arguments` → `parse_composition_positionals` → `merge_set_overrides`, as `run_composition_inner` does | real compiled catalog |
| `cli/tests/l1/provider_tail_ownership.rs` | binary: the union row resolved to Claude fails before the spawn, `x=y` is not rerouted, `phase=2` is not provider data, no lifecycle ran | real compiled catalog, fake `claude` |

All are L1: lib and bin unit tests, and an already-declared `tests/l1` file.
No test or module name carries a tier marker.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| Control: `-c model_reasoning_effort=low` forwarded unchanged | `setters::every_setter_ownership_row_keeps_setters_and_forwards_exact_tokens` (control row) |
| `--codex -c x=y phase=2` | same table |
| `-c x=y phase=2`, no provider hint (every provider is a candidate) | same table (`Candidates::Every`); `partition::tests::the_no_hint_union_forwards_a_value_the_resolved_provider_may_refuse` |
| `--codex --yolo phase=2` | `partition::tests::setters_after_provider_switches_are_claudines_for_every_routed_row`; lib table (`|` marker) |
| A provider-only switch that takes no value | lib table (`--quiet`); `partition::tests::a_researched_no_value_switch_never_takes_the_setter_after_it` (Codex `--ephemeral`, asserted `SwitchValue::None` in the generated data) |
| `--claude --add-dir a b phase=2` / `--add-dir x=y phase=2` | lib table; partition routed rows |
| `--codex -c phase=2`, `phase` declared | `setters::a_declared_setter_after_a_string_switch_leaves_it_without_a_value`; message: `setters::the_missing_value_error_names_the_conflicting_setter` (**red**) |
| Unrecognized switch then `phase=2` | lib table; partition routed rows |
| `--codex --config=x=y phase=2` | lib table; partition routed rows |
| `--codex -c x=y -m gpt5 phase=2` | lib table; partition routed rows |
| Claudine option interrupting a variadic run | lib table (`--add-dir a | b`); partition routed rows (`--add-dir a -m gpt5 b`) |
| `--codex -c phase=2 x=y` fails, `x=y` never reattaches | `setters::a_declared_setter_after_a_string_switch_leaves_it_without_a_value` |
| `--codex -- -c x=y phase=2`, with and without declared `phase`; no check on the opaque part | lib table (three `--` rows); partition routed rows |
| Inline / root union (second arm) / raw JSON Schema / source-relative external schema | `setters::every_schema_source_protects_a_declared_parameter_after_a_switch` (each source has a same-named decoy schema at the launch directory declaring only `decoy`; asserts `phase` declared and `decoy` not) |
| Unestablished schema: contested-value error with `--`/`--set`; unreadable schema errors | `setters::an_unestablished_schema_never_routes_a_contested_setter_silently` |
| Resolved to Claude fails before spawn, `x=y` not rerouted | `setters::a_union_forwarded_value_fails_for_a_resolved_provider_that_takes_none` (controlled); `provider_tail_ownership::a_union_setter_row_resolved_to_claude_fails_before_its_spawn` (binary, no spawn) |
| `count=3`, `enabled=true`, `phase=`, `label=a=b` keep types | `compose::tests::reclaimed_setters_keep_their_types` |
| Switch values stay unchanged strings | `compose::tests::a_switch_value_is_forwarded_as_an_unchanged_string_and_never_a_setter` |
| Last occurrence wins across a switch | `compose::tests::the_last_shorthand_setter_wins_across_a_provider_switch` |
| Shorthand beats `--set` wherever `--set` sits | `compose::tests::a_reclaimed_shorthand_setter_beats_set_wherever_set_is_placed` |
| Dotted key is not a setter | `compose::tests::a_dotted_key_is_never_a_reclaimed_setter` |
| `argv=` after a switch stays an error | `compose::tests::argv_stays_reserved_after_a_provider_switch` |

### Red and green set (checkpoint 2)

- **Red (1):** `the_missing_value_error_names_the_conflicting_setter`. Fails for
  the intended reason. The message for `--codex -c phase=2` is "provider
  argument `-c` takes a value for Codex (at its root command), but none was
  forwarded with it. A `key=value` the document declares as a parameter is
  Claudine's, never the switch's value. Name the provider (for example
  `--codex`), or pass provider arguments after `--`." It does not name
  `phase` and does not suggest a separate provider value. The test covers both
  failing inputs and all four schema sources. It is `#[ignore]`d with a reason
  naming Phase 3, following the repository's precedent for tests authored
  ahead of their behavior (`lib/src/composition/sequence/tests.rs`,
  `clean_break`), so `just test` stays green. **Phase 3 removes the
  `#[ignore]`.** Run it with
  `cargo nextest run -p claudine -E 'test(/ownership::tests::setters/)' --run-ignored all`.
- **Green on arrival (everything else).** The dependency already landed the
  routing (Phase 1, Spike A). To prove the table is load-bearing and not
  vacuous, I temporarily removed `open.taken == 0` from `own_in`'s setter
  offer, which lets a switch keep taking setters as before the fix. The table,
  the union check, and the unestablished-schema test went red. Source restored
  from git; no production change remains.

### Deviations and findings

- **No-hint binary row is not reachable without a terminal.** A document with
  no `agent` cannot resolve a provider non-interactively ("agent resolution
  failed"), and a caller `agent=claude` setter does not resolve one either. So
  the binary fixture takes its union from `agent: [claude, codex]`. The
  every-provider union is covered at unit level (lib `Candidates::Every`, CLI
  `PROVIDERS_DISPLAY_ORDER`).
- **Observation, outside this fix:** an authored `agent: "{{ env.OWN_AGENT }}"`
  with `OWN_AGENT=claude` fails launch with "references an invalid Agent
  provider '{{ env.OWN_AGENT }}'". The topic docs only say a templated `agent`
  narrows nothing for ownership; they do not say whether it can resolve a
  launch. It is recorded here for the author to judge; nothing was changed.
- **Guards triggered by the new tests and resolved.**
  - `dispatch_inventory::cli_dispatch_guard_holds_the_line` flagged
    `provider == Provider::Claude` in the controlled source. The controlled
    catalog is now one table scoped by command path (Claude-like at the root,
    Codex-like at `exec`), the way the research scopes switches, so no
    provider branch is needed.
  - `dispatch_inventory_matches_committed_file`: the new tests add plain
    `reference` sites (no dispatch). Regenerated with
    `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`;
    `docs/providers/dispatch-inventory.json` now has 1800 sites (was 1788).
  - `switch_catalog_guard::switch_metadata_is_never_written_by_hand` exempts
    only paths with a `tests` segment or a `tests.rs` file. That is why the
    controlled catalog lives at `ownership/tests/setters.rs`, not beside
    `ownership.rs`. `ownership.rs` is unchanged.
  - Clippy `type_complexity` on a tuple row table: replaced with a
    `RoutedRow` struct.
- The fixture in the first binary attempt used `probed()`, whose `$schema`
  declares `x`, so `x=y` was correctly Claudine's. The spec row requires that
  no schema claims `x`; the fixture now declares only `phase`.

### Gates

From `claudine/` on macOS: `just test` → 8262 passed, 10 skipped (baseline
9 skipped; +1 is the ignored red test), 0 failed. `just lint` → exit 0 (only
the existing `__eh_frame` linker notice). No OS-specific code was touched; the
new lib and CLI unit tests use portable temp paths. The binary test is in a
file that is already `#![cfg(unix)]` (shell stubs), like its neighbors.

### Input robustness matrix

Not applicable. No reader or format changed in this phase, and the setter-token
shapes the plan lists are asserted above.

## Phase 3

Run 2026-10-02, macOS host.

### What already held (dependency), verified and checked off

- **Wire the classifier into the partition.** Done by `2026-07-13-cli-switches`
  (Phase 1, Spike A): `partition_composition_tail` only splits off Claudine
  options and the authored `--`; `composition::own_arguments` decides every
  other token. No setter branch exists in `partition.rs`, and its "Ownership
  model" docs already describe this. No change.
- **Setter path.** `compose`/`inline-compose` (`compose/prep.rs`) and
  `sequence` (`commands/sequence.rs`) call `own_caller_arguments` once, then
  `parse_composition_positionals(&[pre-file args, owned tokens].concat())` →
  `merge_set_overrides` → `CallerInputLayers::from_caller_overrides`. Reclaimed
  setters keep original relative order after the pre-file ones, so last-wins
  is `Map::insert` order (ruling 3). No new merge path.
- **inline-compose / sequence plumbing.** Same `prep.rs` path for both
  composition kinds; `sequence` uses the same calls. Binary proof per command
  is Phase 4.

### Changed

1. **The missing-value error names the declared setter** (the one red Phase 2
   test). `SwitchAssignment` gained `declared_setter: Option<String>`: when a
   declared parameter directly follows a switch that would have taken it
   (string or variadic, space form, nothing taken yet), `own_in` records its
   key on that switch's assignment. `check_in` copies it onto the new
   `TailMismatch::declared_setter` for a `MissingValue`, and `Display` then
   says: "`-c` takes a value for Codex (at …), but none was forwarded with it:
   the `phase` setter after it is a parameter the document declares, so it
   stays Claudine's and is never the switch's value. Give `-c` a separate
   provider value, or put intentional provider arguments after `--`."
   - The key is stored on the tail, not computed in the ownership error, so the
     **per-launch** resolved-provider check names it too (union
     `[claude, codex]` passes ownership through Claude, then the Codex launch
     fails naming `phase`).
   - Values are never shown; only the key.
   - The existing substring "`-c` takes a value for Codex" is kept, so the
     PTY/L2 prompt tests are unaffected.
   - **Behavior change (no setter):** a missing value with no declared setter
     after the switch (`--codex -c`, or `-c -m gpt5 phase=2`, where a Claudine
     option cut the run) no longer says "A `key=value` the document declares
     as a parameter is Claudine's…". That sentence only applies to the
     declared-setter case, which now has its own message. Those still end with
     the shared `ESCAPE` guidance.
2. **One setter grammar.** It was encoded five times: lib `setter_key`, CLI
   `parse_compose_setter`, and completion's `split_setter`,
   `is_setter_shaped`, `is_setter_name_partial` (the last two said "Duplicated
   here so the engine stays self-contained", stale since the engine already
   calls `claudine::composition`). The lib now owns `setter_key` plus a new
   public `is_setter_name`; all four others delegate.
   - **Empty key, ruled here:** `parse_compose_setter` keeps rejecting `=v`
     ("setter key must not be empty") before delegating. The grammar reads
     `=v` as a bare word, so after a provider switch `-c =v` still forwards it
     as `-c`'s value. Among Claudine's own tokens it stays an error, exactly as
     before. That keeps setter behavior byte-for-byte (plan Wave 2) while
     removing the copy. It is documented on `parse_compose_setter`.
3. **Launch-path invariance guard** (no production change; Spike A found no
   reclassifying path). `switch_catalog_guard::composition_arguments_are_classified_once_per_invocation`
   pins `own_arguments(` to its definition plus one caller, and
   `own_caller_arguments(` to its definition plus `prep.rs` and `sequence.rs`.
   I put it in the existing guard to reuse that file's source walker rather
   than add a fourth copy, and extended the module docs.

### Requirement-to-test mapping (Phase 3)

| Requirement | Test |
| --- | --- |
| Missing-value error names `-c`, `phase`, Codex; suggests a separate value or `--`; echoes no token; every schema source | `ownership::tests::setters::the_missing_value_error_names_the_conflicting_setter` (un-ignored, now green) |
| Launch-time check names the setter; only a setter directly after the switch is named | `ownership::tests::setters::only_a_setter_directly_after_the_switch_is_named_by_the_launch_check` (new) |
| Same through the compiled binary (`-c phase=2 x=y`, union schema, raw JSON schema) | `provider_tail_ownership::{a_claudine_option_interrupting_a_value_run_fails_rather_than_reattaching, schema_names_come_from_a_source_relative_union, a_raw_json_schema_contributes_its_top_level_names}`, now also asserting the key, the remedy, and no echoed value |
| One setter grammar | `compose::tests::the_setter_parser_and_ownership_share_one_key_grammar`, `completion::engine::tests::completion_setter_shapes_match_the_shared_grammar` (new; corpus covers `_`/`-`/digits, first-`=` split, dotted, digit-first, leading `-`, space, non-ASCII, `=v`, `=`, no `=`) |
| Reclaimed setter keeps caller provenance; file reference anchors at the caller's directory; outranks `proxy.with`; absent from provider argv | `compose_caller_file_provenance::setters_after_a_provider_switch_keep_caller_file_provenance` (new; direct and proxied; root-level decoy proves anchoring) |
| No second classification on retry/resume/proxy/step | `switch_catalog_guard::composition_arguments_are_classified_once_per_invocation` (new) |

All new tests are L1, in already-declared targets (`ownership/tests/setters.rs`
via `mod setters;`, bin unit tests, and `tests/l1/*` files already in
`main.rs`). No tier markers. The provenance test is not `cfg(unix)`: it
uses `install_goose`, which has a compiled Windows fixture, like its
neighbors in that file.

### Drift pass

- `completion/engine/tokens.rs` module docs and `is_setter_shaped` said the
  grammar was duplicated so the engine stays self-contained. That was stale and
  is now false. Rewritten.
- `parse_compose_setter`'s `## Returns` said the empty key was "recognized but
  invalid"; reworded to explain why it is rejected although the grammar reads
  it as a bare word.
- `TailMismatch` Display: the generic declared-setter sentence moved into the
  declared-setter branch (see Changed 1).
- `partition.rs` module docs and `argv::looks_like_setter` were already current
  (Phase 1). No change.
- `docs/topics/argv-normalization.md` ("The resolved-provider check") gained the
  declared-setter example and explains why it asks for a separate value.
- Skill: `.claude/skills/claudine/SKILL.md` "Argv pre-parsing" row notes
  `declared_setter` and the single grammar.

### Gates

From `claudine/` on macOS:
- `just test`: 8268 passed, 9 skipped (Phase 2 had 10 skipped; the
  un-ignored test now runs), 0 failed.
- `just lint`: exit 0.
- `dispatch_inventory_matches_committed_file` failed once because the new tests
  shifted reference sites. Regenerated with
  `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::` (1800 → 1802
  sites, plain references from the new launch-check test).

No pre-existing failures. No OS-specific code changed. I did not run
cross-check: the new binary provenance test mirrors
`direct_and_proxy_targets_agree_and_caller_values_outrank_proxy_with`, which
already runs on Windows. Windows CI runs on push to `main`.

### Input robustness matrix

Not applicable: no file format or configuration reader was added or changed.
