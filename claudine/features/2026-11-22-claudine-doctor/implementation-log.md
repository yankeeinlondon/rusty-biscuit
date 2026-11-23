---
spec: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/spec.md
plan: claudine/features/2026-11-22-claudine-doctor/plan.md
implemented-by: claude/opus
started-phase: 1
source_files_during_phase_1:
    - claudine/lib/src/doctor/mod.rs
    - claudine/lib/src/doctor/report.rs
    - claudine/lib/src/doctor/config.rs
    - claudine/lib/src/doctor/tests.rs
    - claudine/lib/src/doctor/config_tests.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/config/load.rs
    - claudine/lib/src/config/json_conversion.rs
    - claudine/lib/src/config/persist.rs
    - claudine/lib/src/config/claudine_config.rs
    - claudine/lib/src/config/setup.rs
    - claudine/lib/src/dispatch/loader.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - darkmatter/lib/src/config/load.rs
    - darkmatter/lib/src/config/init.rs
    - darkmatter/lib/src/markdown/schemas/tests/lazy_positions.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/config_inspect_working_file.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/tests/l1/claudine_editor_parity.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/union_unknown_keys.rs
docs_updated_during_phase_1:
    - claudine/docs/topics/configuration.md
    - darkmatter/docs/topics/configuration.md
    - simplified-schema/docs/topics/language/unions.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
packages:
    - claudine
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - simplified-schema
decisions-for-review:
    - |-
        The plan leaves wiring the seven production check callbacks to Phase 4. I wired the three that Phase 1 builds (`config`, `repo-config`, `stray-config-keys`) into the registry now and gave the other four a placeholder callback that returns an `info` row ("this check is not available yet"). Leaving the three unwired would have made their code unused, which fails the lint gate, or would have needed lint suppressions that hide real dead code. The alternative is lint allowances now and wiring everything in Phase 4. To reverse: point the three entries back at the placeholder. Phase 4 still replaces the four placeholders.
    - |-
        A missing `~/.darkmatter.yaml` is an `error` under `config` with the remedy `claudine setup`, the same as a missing `~/.claudine/config.yaml`. The spec only says "missing required user configuration". Normal Darkmatter startup creates the file, but doctor never does, and the registration repair needs it. The alternative is `warn` or `info` for the Darkmatter file. Reversing it changes one severity in `doctor/config.rs::darkmatter_findings` and one test.
    - |-
        Doctor never shows a raw validator message, because those messages can quote authored values. An invalid value is shown by its kind and location ("has an invalid value at `/tts` (line 2, column 6)"). A pointer segment is shown only if the scope's schema declares that name somewhere, or if it is a list index; any other segment shows as `*`, so a messenger route's name is hidden. A semantic failure names its setting only when the message starts with a plain setting path. The remedy tells the user that any other `claudine` command prints the full message. The alternative is to show filtered messages, which keeps more detail but needs a per-message allowlist that can drift. Reversing it means changing the wording helpers in `doctor/config.rs`.
    - |-
        `repo-config` reports a repository file that exists but is invalid or unreadable as `ok` ("present; its problems are reported under `config`"). The spec says the row reports whether the file exists, so its own check passes, and the errors are already counted once under `config`. The alternative is `warn`, which would count the same problem in two rows. Reversing it changes one arm of `check_repo_config`.
    - |-
        SimplifiedSchema now reports an unknown key inside a union (for example `tts: {provider: say, pitch: 3}` or a misspelled lifecycle verb) by name, at the key, with the `UnknownKey` code, instead of the generic "not valid under any of the schemas" failure. The plan asked for this (Rule 9). It also changes two behaviors outside doctor. (1) DMLS grades unknown keys as warnings unless `[schema] strict` is set, so in the editor a misspelled lifecycle verb or a `shell` under `initialize` drops from error to warning, as every other unknown key already is. `md schema validate`, composition, and Claudine still refuse the document. (2) `md set` refuses to edit a key that an object-union property does not declare (it used to allow it, because the old problem named no key). One Darkmatter test fixture relied on the old behavior and now edits a declared key. The alternatives are to keep union-derived unknown keys at `ConstraintViolation` so DMLS keeps showing an error, or to have DMLS grade union-member unknown keys as errors. To reverse: change the `code` in `simplified-schema/lib/src/validate.rs::union_unknown_key_problem`, or change DMLS's classifier, and update the regenerated A3 golden.
    - |-
        A flat (pre-2026) action or route entry still reports only its migration message. The union's unknown-key problems below the entry are dropped as before, because every key of a flat entry (`type`, `command`) is unknown to the keyed arms and would otherwise show up as a stray key. Rule 8 asks to keep a flat entry's "genuine" unknown keys for `stray-config-keys`, but separating those needs the keyed conversion's view of the entry. I left that undone. The alternative is to run `wire::upgrade_flat` on the entry and validate the result to find the keys that are still unknown. Doing it later is additive to `config/load.rs::project`.
    - |-
        Darkmatter places an unknown key in `~/.darkmatter.yaml` at its value (for example column 8 of `colour: red`), not at the key, as Claudine now does. Doctor shows Darkmatter's position as it is, because changing it would change every `md` message for that file. The alternative is to make Darkmatter's config loader locate unknown keys at the key token, matching Claudine. Doing that later changes the position only, not which keys are found.
    - |-
        `darkmatter::config::inspect_working_file` returns a `Read` error when there is no home directory, instead of a separate "no home" error kind. Doctor reports "no home directory" before it asks, so the case never reaches the user through doctor. The alternative is a new `ConfigErrorKind` variant, which every match on that type would need to handle.
    - |-
        When SimplifiedSchema names unknown keys inside a union, it ignores arms that are not objects (a literal such as `true`, or an enum of words). Only object arms say which keys they accept, so the rejected keys are those that no object arm declares. A union with no object arm keeps its old message.
    - |-
        `file-read: []` (an empty list) is accepted by both scopes, so doctor treats it as valid. The plan expected it to be refused. I tested what the schema does and listed it with the valid cases; the scope-refusal case uses an invalid glob instead. If an empty list should be refused, the change goes in the permissions schema, not in doctor.
message-to-agent: |-
    Phase 1 is done and its gates pass (Claudine, Darkmatter, and SimplifiedSchema `just test`/`just lint`/tier coverage). What Phase 2 builds on:
    - `claudine/lib/src/doctor/mod.rs` holds `Severity`, `DoctorRow`, `run`, `Environment`, `Pass` (one cached reading of the disk per pass), `CheckDescriptor`, and the seven-entry `REGISTRY`. `config`, `repo-config`, and `stray-config-keys` are wired; `registration`, `darkmatter-permissions`, `tts`, and `path` use `pending_check`. Replace each one as its phase builds it.
    - `Pass::config()` returns `doctor::config::ConfigObservations`: the four files (`claudine_user`, `claudine_repo`, `darkmatter_user`, `darkmatter_repo`, each an `ObservedFile` with a path and a state), `user_config` and `effective_config` (typed, or the reason it is unusable), plus `home_available`, `context_available`, `in_repository`. The struct has a `cfg_attr(not(test), expect(dead_code))` because some fields are only read by tests so far; remove the attribute when the registration and permissions checks read them, or the `expect` will fail the lint.
    - `Finding::fixable` and `Pass::environment` carry the same kind of `expect(dead_code)`; remove each when your repair first uses it.
    - `report::row` already renders repair attempts (`Repaired:`, `Repair failed:`, `Already changed:`) and forces a failed repair to `error`; `run_with` runs a repair only when a fresh check of its row is fixable, then rereads the row and its `prerequisites`.
    - Never put a raw validator message or an authored value in a finding; use the wording helpers in `doctor/config.rs` (`redacted_pointer`, `named_setting`). `report::display_text` escapes control characters for every row.
    - The robustness matrix's `CliProcessFixture` process cases are deferred to Phase 4, when the CLI command exists.
    - SimplifiedSchema's `just test` takes no package filter; run it from `simplified-schema/` to cover both crates.
---

# Implementation Log for 2026-11-22-claudine-doctor (5 phases)

## Phase 1

- started 2026-11-23; the host-registration v2 code has landed in this tree
  (no `HostRevision`/`refresh_host_registration` remain in `claudine/lib/src`),
  so the phase builds on v2 directly as Rule 18 requires.
- while the phase ran, the author's pipeline committed the host-registration
  v2 work that was uncommitted at session start (`ce553e706`, `ebdff458b`,
  `33c412d45`); nothing in this phase depends on those commits beyond v2
  itself.

### Report Foundation (Rules 1–4)

- `claudine/lib/src/doctor/mod.rs`: public `Severity` (`ok < info < warn <
  error`, serialized lowercase), `DoctorRow { id, severity, summary, detail,
  fixable }` (serializes to exactly the five stable JSON fields),
  `run(snapshot, fix)`, and `title_of(id)` for the CLI's headings. The static
  `REGISTRY` lists the seven checks in report order, each a
  `CheckDescriptor { id, title, prerequisites, check, repair }`.
  `Environment` captures the launch context once with
  `composition::capture_file_resolution_context` (its failure is kept as a
  reason-free `ContextUnavailable`, since the error text can name authored
  values); `Pass` is one reading of the disk and caches the configuration
  observations on first use.
- `run_with` inspects every check, then with `fix` runs each repair in
  registry order, each from a fresh `Pass`, only when that fresh reading is
  fixable; afterwards every attempted row and every row an attempted check
  names as a prerequisite are read again from one new pass.
- `doctor/report.rs`: `Finding` (severity, scope, path, summary, notes,
  remedy, fixable), `CheckOutcome` (findings + the ok summary), and
  `RepairOutcome { changes, failure }`. A row's severity is its worst
  finding; the summary is the worst finding's (`(+N more)` when several
  share that severity); the detail lists findings ordered general → user →
  repository, then by path, stable within a path, with `Remedy:` lines and
  `Repaired:` / `Repair failed:` / `Already changed:` lines from the attempt.
  A failed attempt keeps the row at `error` even when the disk now reads
  healthy; `fixable` is always the last reading's. Every dynamic string
  passes through `display_text`, which escapes control and bidirectional
  formatting characters (`\u{1b}`), so no file name or key can move the
  cursor or break a row. Prose markup escaping is left to the Phase 4
  renderer, which owns Prose.
- tests: `doctor/tests.rs` (13 tests) drive fake checks whose state is a
  marker file in a temp home, so repairs are real writes and re-reads real
  reads: aggregation and ordering, blocked → info, fixable without the flag
  and no writes, verified post-repair state, second run is a no-op (mtime
  kept), fresh-evidence ordering between repairs and prerequisite re-reads,
  partial failure with recorded changes and remaining fixability, a failed
  attempt over a healed disk, a no-change failure beside an unrelated repair,
  control/bidi escaping, the JSON shape, and the registry order.

### Union Diagnostics (Rule 9)

- delegated to a subagent. `simplified-schema/lib/src/validate.rs`:
  `build_problems`'s `AnyOf` branch now asks `union_unknown_key_problem`
  for the keys every type-applicable arm rejects at the union's own node
  (`union_rejected_keys` / `arm_rejected_keys`: an arm failing `type`,
  `const`, or `enum` there is left out; `additionalProperties` /
  `unevaluatedProperties` errors add keys; a same-node nested union is
  resolved recursively; an applicable arm with no rejection empties the
  intersection). The result is shaped exactly like an ordinary closed-object
  unknown-key problem (instance path at the object, `offending_properties`,
  jsonschema's wording), listed ahead of the existing deeper / file /
  nullable problems, deduplicated by (path, message); the generic union
  failure remains the fallback. Side effect: a plain nullable object now
  reports an extra key beside a deeper failure, which it used to drop.
- tests: `simplified-schema/lib/tests/l1/union_unknown_keys.rs` (10 tests:
  scalar-or-object union, shape parity with a closed-object problem,
  nullable object, deeper failure beside a same-node key, another arm's
  known key never stray, a key every arm rejects, a literal arm does not
  veto, a union reached through a named union type, a valid control, an
  open mapping arm). 6 of the 10 fail against the old validator.
- ran: simplified-schema focused filters (92 + 88 passed),
  `cargo nextest run -p simplified-schema --features work-counters,terminal
  -E 'test(/validat|union|unknown|additional|anyof|any_of|nullable|soft|coerc|default/)'`
  (616 passed), simplified-schema `just lint` clean, Darkmatter
  `-E 'test(/config/)'` (194 passed), Claudine config/load/schema filters
  (161 passed). No downstream expectation changed. The subagent added an
  "Unknown Keys in a Union" section to
  `simplified-schema/docs/topics/language/unions.md` and corrected its claim
  that discriminant-free unions keep byte-identical diagnostics.
- note: `simplified-schema`'s `just test <filter>` runs its
  no-default-features leg (`test-minimal`) unfiltered, so a filtered run
  there still executes ~2,200 tests; the subagent ran that leg by hand
  with `-E` instead. The recipe was not changed.

### Darkmatter Observation (Rules 5, 7)

- delegated to a subagent: `darkmatter::config::load::inspect_working_file
  (context, scope) -> Result<ConfigFile, ConfigError>` and crate-private
  `read_working_file`, `working_path`, `working_revision`,
  `read_working_bytes` in `darkmatter/lib/src/config/load.rs`;
  `init::config_path` and `init::observe` now share the path rule, byte
  reader, revision, and judge with inspection (observe keeps its own call of
  the shared byte reader so `InitError::Read` still carries the
  `io::Error`). Whole-file failures are `Err`; invalid `permissions` and
  `register` entries stay inside the successful `ConfigFile`. No accessor was
  missing. The subagent also added a "Checking one file without loading"
  section to `darkmatter/docs/topics/configuration.md`.
- tests: `darkmatter/lib/tests/l1/config_inspect_working_file.rs` (8 tests:
  absent, valid in each scope, invalid permissions kept inside, invalid
  register entry kept as a finding, whole-file failures with locations,
  writes nothing with a legacy whitelist present and the user file missing,
  a reference is not followed, a context without the scope is refused).
  Ran: `cargo nextest run -p darkmatter --test l1 -E 'test(config_inspect_working_file)'`
  (8 passed), `-E 'test(/^config_/)'` (172 passed), `darkmatter-cli`
  `init_command` (4 passed), Claudine `darkmatter_setup` /
  `installed_registration` / `config_indentation` (57 passed), Darkmatter
  `just lint` clean.

### Claudine Observation and Observation Integration (Rules 5, 6, 8)

- `config/load.rs`: `ConfigProblem` gained `kind: ConfigProblemKind`
  (`UnknownKey`, `Syntax`, `InvalidSetting`) and `unknown_key()`; every
  initializer (load, json_conversion, persist, claudine_config) sets it.
  An unknown key's problem is now located at the key token (a locating
  problem with `ValidationProblemCode::UnknownKey` and the key as
  `offending_property`, so `PositionMap` selects the key), and is
  recognized by the validator's `UnknownKey` code as well as the old
  message. Duplicates are removed by (kind, pointer, message) rather than
  only adjacent ones. `Display` is unchanged for established callers.
- `dispatch/loader.rs::refused_action_mappers`: the passive half of
  `compile_canonical_runtime`, sharing its `compile_canonical_action_mapper`;
  matchers stay tolerated.
- `config/setup.rs::same_file` is now `pub(crate)` so doctor uses setup's
  `PathIdentity` + `canonicalize_simplified` comparison for aliases.
- `doctor/config.rs`: `observe(environment)` builds the shared observation
  record (`ConfigObservations`: the four `ObservedFile`s with
  `ClaudineState` / `DarkmatterState`, `user_config` = the user file alone
  typed and semantically judged, `effective_config` = merged with usable
  repository overrides and judged again, plus availability flags). A path
  is probed with no-follow metadata first, so a dangling link and a
  non-file are unreadable rather than absent; a missing user YAML with a
  `config.json` beside it gets the legacy remedy and the JSON is never
  opened. Claudine files are read with `config::load::read_document`;
  Darkmatter files with `inspect_working_file`. Semantic checks follow the
  normal load's order (typed read, merge, `validate`) plus the mapper
  check; a merged failure is attributed to the user file when the user
  file fails alone, otherwise to the repository file with a note naming the
  user file; a user-only failure superseded by a valid merged pair is not a
  `config` finding (it stays in `user_config` for speech and the handoff).
- the three checks: `config` (home/context availability, each file's
  state, located categories, semantic findings, ok notes per file),
  `repo-config` (info outside a repository or when the file is absent; ok
  when present, aliased, or present with problems, which are under
  `config`), `stray-config-keys` (one error per unknown key, sorted by
  pointer, with line/column; a file whose syntax prevents key validation is
  an info naming it; a user setting written in the repository file gets a
  move remedy).
- redaction: a Claudine problem is shown by kind and location; an invalid
  value's pointer keeps only segments the scope's lowered schema declares
  somewhere (and list indices), every other segment shows as `*` (a
  messenger route's name, say); a semantic failure names its setting only
  when the validation message leads with a plain setting path, and never
  keeps the rest. Darkmatter problems are shown by `ConfigErrorKind` with
  the same pointer rule (`register` added). Unknown key names are the only
  authored text shown.
- tests: `doctor/config_tests.rs` (30 tests) over temp homes and
  repositories through the built-in registry: the control; no writes with a
  legacy file present; missing user files; outside a repository; no home /
  no context; top-level unknown key located at the key and named only in
  the stray row; nested unknown keys in the `tts` mapping, the imported
  `permissions` mapping, and a keyed action union (needs Rule 9); a known
  key of another union arm not reported; a user setting in the repository
  file; the shared YAML loader's document shapes once (duplicate key,
  trailing nonsense, malformed, second document, non-string key, empty
  document; BOM, comments, `key:` null, `{}` accepted); invalid values
  without their text; route-name redaction; the `tts` field shapes; repo
  override null/empty and event list shapes (mixed and all-invalid
  arrays); semantic rule naming without value; a refused regex mapper; a
  merged-only `active_messenger` interaction naming both files; a valid
  merged pair superseding a user-only problem; a repository at the home
  judged once; dangling link and directory (Unix); an invalid repository
  file; Darkmatter field shapes (wrong scalar, blank reference, wrong
  collection, mixed and all-invalid `register`, `shell-commands` invalid
  element, scope-invalid `file-read` globs, unknown key, duplicate key) and
  their valid defaults (`{}`, null, empty mapping, `[]`, a third-party
  register entry). Every redaction case asserts the sentinel `SECRET` is
  absent from the serialized rows.
- ran: `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` (43
  passed); `cargo nextest run -p claudine -E 'test(/config|setup|registration/)'`
  (727 passed) to check the location change against existing callers.
- departures found while testing: `file-read: []` is accepted by both scope
  projections (only the shared, scope-less shape says `not-empty`), so the
  plan's expected refusal does not exist; the test asserts acceptance.
  Darkmatter locates an unknown key at its value, not the key; doctor
  reports Darkmatter's own location.
- the robustness matrix's process-proof cases (`CliProcessFixture`) need the
  command, which Phase 4 wires; they belong to Phase 4's CLI Contracts.

### Downstream expectations changed by the union diagnostics

The full suites found tests that pinned the old generic union message where
the validator now names the rejected key. Each new diagnostic is the more
precise and correct one; the tests were updated, not the behavior:

- Claudine `lib/tests/l1/claudine_schemas.rs` (`ac5_shell_placement`,
  `ac5_single_action_dictionaries`): `shell` under `initialize` and `then`
  in a root dictionary are now `'shell' was unexpected` /
  `'then' was unexpected` at the same pointers.
- Claudine `lib/tests/l1/claudine_schemas_embedded.rs`
  (`a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written`):
  a misspelled verb (`inf0`) and a misplaced `shell` are now `UnknownKey`
  problems with no arm description; beside a known verb, the position is the
  unknown key's own line (3:5, was 2:5). The test now pins the code per case.
- Claudine `lib/tests/l1/sequence_schema.rs` and Darkmatter
  `cli/tests/l1/claudine_schema_bundle.rs`: group `execution` /
  `max_paralel` and the same lifecycle cases, by name.
- Darkmatter `cli/tests/l1/schema_validate.rs`: an unknown lifecycle verb is
  `'bogus' was unexpected`, not the `anyOf` text.
- Darkmatter `lib/src/markdown/schemas/tests/lazy_positions.rs`: a nullable
  inline object with a deeper failure now also reports its unknown key
  (4 problems, was 3: the old validator silently dropped it).
- Darkmatter `lib/tests/content_policy_fixtures/mod.rs`: an extra long-form
  field is located at its key (new `Anchor::Key`), not the item start.
- Darkmatter `cli/tests/l1/get_set_rm.rs`
  (`a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`): the
  fixture edited the undeclared `style.width`. `md set` blocks problems at
  or under the edited path; the old union failure at `/style` named no key,
  so the edit slipped through, and now the unknown-key problem names
  `width` and refuses it. The test's subject is comment preservation, so it
  now edits the declared `style.margin`.
- DMLS `tests/l1/core_schema_types.rs`, `claudine_editor_parity.rs`, and the
  regenerated golden `tests/fixtures/response_contracts/a3_lifecycle_faults.json`:
  the same items are `dm.schema.unknown_key` at the key token. DMLS grades
  unknown keys as warnings outside `[schema] strict`, so an unknown verb in a
  lifecycle stack is now an editor warning (an error in strict mode);
  `md schema validate` still exits 1 for it (see decisions-for-review).

### Gates and verification (all serial, after both subagents finished)

| Command | Result |
| --- | --- |
| `cd claudine && just test` | 12768 passed, 15 skipped; `🐢 none at or above 5.0s`; `just slow-tests fresh` exit 0 |
| `cd claudine && just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `cd simplified-schema && just test` | 2592 + 2172 passed, none slow |
| `cd simplified-schema && just lint`, `just check-tier-coverage simplified-schema` | clean, 0 stranded |
| `cd darkmatter && just test` | 17855 passed, 10 skipped, none slow |
| `cd darkmatter && just lint`, `just check-tier-coverage darkmatter` | clean, 0 stranded |

The earlier `just test` runs that failed were the expectation changes listed
above; each was then rerun alone, fixed, and the full suites rerun green.
Host load during the runs was 4–15 on this host; no test was slow.

Dependents: the other packages that depend on Darkmatter or SimplifiedSchema
(`biscuit-icon`, `biscuit-speaks`, `biscuit-terminal`, `messenger`, `playa`,
`research`, `sniff` CLIs/libs) were searched for assertions on union
messages, `init::observe`, or `inspect_working_file`; none exist, so their
suites were not run.

Cross-OS: the only OS-specific code is the Unix-gated dangling-link test;
path identity reuses setup's `PathIdentity`/`canonicalize_simplified`. The
plan (Rule 18) leaves the local cross-check out because it creates a
throwaway commit; remote coverage is left to the regular CI schedule.

### Requirement-to-test mapping (Phase 1)

| Requirement | Tests |
| --- | --- |
| Rules 1–4 registry, ordering, aggregation, blocked, fix/no-fix, partial failure, redaction | `doctor::tests::*` (13) |
| Rule 5 four files, absence vs failure, aliases, legacy existence-only | `doctor::config::tests::{the_valid_control…, an_inspection_writes_nothing…, a_missing_user_file…, outside_a_repository…, without_a_home_or_a_context…, a_repository_at_the_home…, a_dangling_link_and_a_directory…, an_invalid_repository_file…}` |
| Rule 6 semantic checks, merge attribution, mapper check | `a_semantic_rule_names_the_setting…`, `a_mapper_the_runtime_compiler_refuses…`, `a_repository_selection_with_no_user_route…`, `a_valid_merged_pair_supersedes…` |
| Rule 7 Darkmatter passive inspection | `darkmatter::l1 config_inspect_working_file::*` (8); `doctor::config::tests::darkmatter_*` |
| Rule 8 structured unknown keys at the key token | `a_top_level_unknown_key…`, `nested_unknown_keys…`, `a_user_setting_in_the_repository_file…` |
| Rule 9 union unknown keys | `simplified-schema::l1 union_unknown_keys::*` (10); `a_known_key_of_another_arm…` |
| Robustness matrix: shared YAML loader shapes | `syntax_cases!` (6), `a_duplicate_key_names_the_repeated_key_line`, `a_bom_comments_explicit_null…` |
| Matrix: TTS field shapes | `tts_shapes_are_judged_by_the_user_schema` |
| Matrix: settings and overrides | `override_null_inherits_and_empty_mappings…`, `an_invalid_value_is_located_without_its_text`, `a_route_name_the_user_chose…` |
| Matrix: Darkmatter and imported permissions, registration | `darkmatter_files_are_judged_by_their_scope_schema`, `darkmatter_defaults_and_empty_values_are_valid`, `nested_unknown_keys…` (imported `permissions`) |
| Matrix: process proof through `CliProcessFixture` | deferred to Phase 4 (the command is wired there) |

Code-smell review of the new load-bearing parse paths: no new
`#[serde(default)]`, no `Option<T>` collapsing null and absence (the typed
reads reuse the existing schema-default projection), and no
`filter_map`/`unwrap_or_default()`/`.ok()` on a load-bearing parse; the one
`unwrap_or_default()` (an unknown key's display name) runs only on a
problem already known to be `UnknownKey`, and the legacy `config.json`
probe is existence-only by design.
