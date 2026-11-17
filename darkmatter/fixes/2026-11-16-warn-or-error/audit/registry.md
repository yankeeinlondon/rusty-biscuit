# Policy registry audit (`md` CLI)

Tree: `/Volumes/coding/wt/rusty-biscuit/fix-sequences`, HEAD `baee016e5`.

**Note on the "interim change":** it is no longer uncommitted. `git diff HEAD` is empty for darkmatter; the change (`DEFAULT_RECOVERIES`, the `check_references` downgrade, the new `ReferenceIssueCode::EmptyTarget`) landed in commit `11a909a82` ("fix(darkmatter): report empty reference targets through policy"). Below, **tree** = HEAD; **before** = `11a909a82^`. The before-state is reasoned from `git show 11a909a82`, not run. Tree behavior was probed with `target/debug/md`, which was built after `11a909a82` (16:47 vs 16:42) and is newer than every `.rs` file in `darkmatter/{cli,lib}/src`. Probe files: `(probe fixtures, not kept)`.

Paths are relative to `darkmatter/`. `P` = `cli/src/args/policy.rs`, `C` = `cli/src/commands/compose.rs`, `M` = `cli/src/commands/mod.rs`, `T` = `cli/tests/l1/policy_verbs.rs`.

## How the registry resolves

- **Which verbs a kind takes** (`P:102-110`): Recovery takes `--allow` and `--deny`. Host takes `--allow`. Feature takes `--disable`. Diagnostics takes `--deny`.
- **Which verbs each command declares** (`P:49-55`):
  - compose takes allow, deny and disable;
  - render and the default `md FILE` take deny only;
  - clean and schema validate take disable only.
- **Help and completions.** `entries()` (`P:242-246`) filters the registry by command and kind, and `long_help()` (`P:417-451`) and `complete()` (`P:387-414`) both use it. So **every entry is listed in help and completions on exactly the verb × command pairs where it parses.** No entry is hidden, and none is listed where it is refused.
- **`recovers(id)`** (`P:573-577`, tree) is true when one of these holds:
  - the id is named in `--allow`; or
  - the id is not named in `--deny`, and either `--allow all` is given, or `--deny all` is absent and the id is in `DEFAULT_RECOVERIES` (`P:132`).

  Before the change it was only `allowed.contains(id) || (allow_all && !denied.contains(id))`.
- **`denies(id)`** (`P:590-592`) is `deny_all || denied.contains(id)`. Only `style` is ever asked through it.
- **`blocking_policy()`** (`P:595-611`):
  - it applies `deny_all`;
  - each denied entry contributes its `codes` (`style` has none, `P:231`);
  - denied raw codes are added;
  - entries named in `--allow` are exempted.

## Per-entry table

| id | kind | verbs × commands | consulted at (file:line) | default (no flag) | with `--allow id` | with `--deny id` | wired? | tests |
|---|---|---|---|---|---|---|---|---|
| `missing-hyperlinks` | Recovery | allow+deny × compose (`P:136-142`) | `C:30` via `recovers_missing`, used at `C:797` (downgrade to warning) and `C:805` (refuse filter); code blocking at `C:812-823`. Only inside `check_references` (`C:781`), which runs only for file input (`C:369`, lifecycle `C:518`). | **Tree:** warning on stderr after the content, exit 0 (probe `link.md`). JSON gives a `warning` diagnostic. **Before:** `Refused`, broken-reference report, exit 1. **Stdin (both):** no check at all, silent exit 0 (probe). | **Tree:** same as default (no-op), except under `--deny all`, where it keeps the recovery and exempts the code. **Before:** recovered and reported, exit 0. | Recovery off, so a missing target refuses with exit 1 (probe). It also blocks warning-severity variants (a link inside inserted text, an empty target `[x]()`). Beyond cancelling `--allow all`, it also re-enables the error. On stdin it does nothing (probe). | yes (file input only) | `T::each_missing_reference_allowance_recovers_its_own_kind_only` (**stale, asserts default exit 1**, see F1), `T::a_specific_denial_beats_allow_all_in_either_order`, `T::a_denied_missing_link_inside_inserted_text_blocks`, `T::an_empty_link_target_warns_with_the_link_and_blocks_only_when_denied`, `T::a_recovery_denied_by_its_code_conflicts_with_its_allowance`; `cli/tests/l1/compose_value_provenance.rs::missing_link_inside_inserted_text_warns` |
| `missing-image-refs` | Recovery | allow+deny × compose (`P:143-149`) | `C:31`, otherwise as for hyperlinks | Tree: warning, exit 0 (probe `image.md`). Before: exit 1. | Tree: no-op except under `--deny all`. Before: recovered. | Exit 1 (probe). Also blocks an empty `![y]()` warning (probe `empty.md`). | yes (file input only) | `T::each_missing_reference_allowance_recovers_its_own_kind_only` (stale); none dedicated to deny |
| `missing-transclusions` | Recovery | allow+deny × compose (`P:150-156`) | `C:32` (reference check). `C:355` gives `ComposeOptions::with_allow_missing_transclusions`, read by `lib/src/markdown/compose/transclusion/engine.rs:2048` (`removes_missing`, NoMatch only) and `lib/src/markdown/compose/preflight/collect.rs:2379`. Warning code set at `engine.rs:2055` and `pipeline/phases.rs:744`. Duplicate warning removed at `C:598-600` and `cli/src/commands/compose/lifecycle.rs:540-542` | **Tree:** directive removed, one warning, exit 0 (probe `part.md`, JSON shows one `dm.reference.missing_transclusion` warning). An absolute missing path on stdin is also removed with a warning (probe `abs.md`). A relative path on stdin fails with "missing source context" whatever the flags (not NoMatch). **Before:** exit 1. | Tree: no-op except under `--deny all`. Before: directive removed, exit 0. | Exit 1 (probe). On stdin it becomes a hard `TransclusionError` (probe). | yes | `T::each_missing_reference_allowance_recovers_its_own_kind_only` (stale); `cli/tests/l1/compose_lifecycle.rs::a_missing_include_runs_failed_then_finalize` (**stale**, F1) |
| `ctx-override` | Recovery | allow+deny × compose (`P:157-163`) | `C:354` → `with_allow_ctx_override`, then `lib/src/markdown/compose/pipeline/mod.rs:1262`, `context/effective_state.rs:815` (`merge_ctx`); warning code at `pipeline/mod.rs:1271` | Error `CtxMergeError`, exit 1 (probe `ctx.md`) | Warning "Document ctx was not an object…", exit 0 | Same as default (exit 1). Only effect: it cancels `--allow all` (probe). | yes | `T::a_specific_recovery_beats_deny_all_in_either_order`, `T::an_independently_denied_diagnostic_still_blocks_a_recovered_command`, `T::opposite_selections_are_usage_errors…`, `T::repeated_identical_selections_are_harmless`; `cli/tests/l1/compose_refs_and_missing.rs::test_compose_scalar_ctx_without_allow_override_fails` / `…_with_allow_override_succeeds` |
| `invalid-frontmatter-assignment` | Recovery | allow+deny × compose (`P:164-170`) | `C:356-357`, then `lib/src/markdown/compose/transclusion/engine.rs:596` | `TransclusionError: invalid frontmatter assignment`, exit 1 (probe `inv.md`) | Warning, exit 0 | Same as default. Only cancels `--allow all` (probe). | yes | `cli/tests/l1/compose_state_set.rs::test_set_overlay_strict_rejects_invalid`, `::test_set_overlay_permissive_invalid_warns`; no deny test |
| `reassigned-frontmatter-property` | Recovery | allow+deny × compose (`P:171-177`) | `C:358-359`, then `engine.rs:626` | `TransclusionError: duplicate frontmatter property`, exit 1 (probe `reas.md`) | Warning "rightmost assignment wins", exit 0 | Same as default. Only cancels `--allow all` (probe). | yes | `compose_state_set.rs::test_set_overlay_strict_rejects_reassigned`, `::test_set_overlay_permissive_reassigned_warns_and_rightmost_wins`; no deny test |
| `shell-timeout` | Recovery | allow+deny × compose (`P:178-184`) | `C:444-446` → `with_allow_shell_timeout(true)` (`lib/src/markdown/compose/context/options.rs:1610`), giving `ShellTimeoutBehavior::EmptyString` read at `shell_expansion/executor.rs:324,380-386` and `frontmatter_shell_expansion.rs:1786`; warning code at `shell_expansion/mod.rs:739` | Timeout is an error, exit 1 (from test, not probed: needs a shell permission) | Output replaced with an empty string plus a warning, exit 0 | Same as default. Only cancels `--allow all` (inferred). | yes | `cli/tests/l1/compose_shell.rs::test_compose_allow_shell_timeout_emits_warning` (and the preceding timeout-fails test), `T::allow_all_recovers_a_shell_timeout_as_an_empty_string`, `T::blocking_policy_exempts_only_named_recoveries` (unit, `P:673`), `cli/src/args/cli.rs:232,248` (unit) |
| `untyped-map-variables` | Recovery | allow+deny × compose (`P:185-191`) | `C:286` → `with_allow_untyped_map_variables`; `lib/src/markdown/compose/collection_blocks/analysis.rs:180`, `call_precheck.rs:676` | `CollectionError dm.collection.untyped_map_source`, exit 1 (probe `map.md`) | Warning, exit 0 | Same as default. Only cancels `--allow all` (probe). | yes | `cli/tests/l1/compose_collection_controls.rs::untyped_map_variables_recovers_a_map_source`, `::map_recovery_precedence_and_conflicts`, `::a_blocked_recovery_fails_before_any_command_runs`, `::a_recovered_source_still_needs_shell_approval`, `::a_recovered_source_warning_survives…`; unit `P:686-749` |
| `untyped-filter-variables` | Recovery | allow+deny × compose (`P:192-198`) | `C:287`; `analysis.rs:181`, `call_precheck.rs:677` | Error, exit 1 (by test macro) | Warning, exit 0 | Same as default. Only cancels `--allow all`. | yes | `compose_collection_controls.rs::untyped_filter_variables_recovers_a_filter_source`, `::filter_recovery_precedence_and_conflicts` |
| `host` (`host=HOST`) | Host | allow × compose (`P:199-205`) | `M:128` `policy.hosts()` → `build_remote_read_config` (`C:76-93`, `allowed_hosts`), consumed in `lib/src/markdown/compose/remote.rs`, `preflight/collect.rs`, `schemas/slots/session.rs` | Remote reads from an unnamed host are refused unless config `network: allow` | That host is readable, plus schema-slot http(s) reads on it | n/a: `--deny host=…` is a usage error (`P:299-302`); `--deny all` keeps it | yes | `T::a_host_is_an_allowance_only`, `T::allow_all_grants_no_unnamed_host`, `T::deny_all_keeps_explicit_host_consent`; `compose_remote_caching.rs` (5), `compose_icmp.rs` (3) |
| `baseline-schema` | Feature | disable × compose, clean, schema validate (`P:206-212`) | compose `C:278` (`apply_compose_baseline_schema` → `compose_baseline` `C:1207-1208`); clean `M:87` → `cli/src/commands/clean.rs:401`; schema validate `M:213` → `cli/src/commands/schema/validate.rs:288,404-408`; completion `cli/src/args/path_completion.rs:263`; conflict with an explicit baseline at `P:488-494` | Baseline schema applied | n/a | n/a | yes on all 3. No effect on a YAML input to `schema validate` (documented, `docs/cli/policy.md:109-113`). | `T::a_disabled_baseline_stays_disabled_under_deny_all`, `T::disabling_the_baseline_conflicts_with_an_explicit_baseline`; `clean_schema.rs`, `schema_validate_baseline_selection.rs`, `standalone_baseline.rs`, `compose_base_schema.rs` |
| `schema-triggers` | Feature | disable × compose, clean, schema validate (`P:213-219`) | compose `C:279` (`with_trigger_schemas(false)`); clean `M:88` → `clean.rs:418`; schema validate `M:214` → `validate.rs:320,550,572`; `path_completion.rs:264` | Trigger discovery on | n/a | n/a | yes on all 3. YAML input to `schema validate` is a documented no-op. | `schema_validate_baseline_selection.rs:393`, `compose_setters.rs:676`, `compose_schema.rs:65`, `standalone_baseline.rs:415-453`, `schema_trigger_rename.rs`, `schema_triggers.rs` |
| `collection-blocks` | Feature | disable × compose (`P:220-226`) | `C:288-290` → `ComposeOptions::disable(ComposeOperation::CollectionBlocks)` | `::map`/`::filter` stage runs | n/a | n/a (usage error) | yes | `compose_collection_controls.rs::disabling_collection_blocks_leaves_the_directives_unexpanded`, `::a_disabled_stage_does_not_fail_on_an_untyped_source`, `::collection_blocks_cannot_be_allowed/denied`; unit `P:702-714` |
| `style` | Diagnostics | deny × compose **and** render (`P:227-233`) | **render only:** `cli/src/commands/render.rs:71` (`denies("style")`) → `StylePreparationPolicy::with_deny_style` (`render.rs:178`, `lib/src/style/layer.rs:452`), `render.rs:214,222,234`; policy built at `cli/src/args/render_prepare.rs:40`. **Compose: consulted nowhere** (no `denies("style")` in `C`, no style module used by compose; `codes: &[]` so `blocking_policy` adds nothing). | Render: warning "unknown style key…", exit 0 (probe `style.md`) | n/a (usage error, `P:316`) | Render: error, exit 1 (probe). **Compose: no-op.** With `--disable baseline-schema --deny style` it gives exit 0, the same as without it (probe). | render yes; **compose no** | `T::deny_style_and_deny_all_block_a_style_issue_on_every_render_path` (its compose leg is vacuous, F3); `T::completions_offer_the_registry_identifiers_per_command`; `cli/src/render.rs:219-221` (unit) |

## Duplicates (on both lists)

Exactly the 9 `Recovery` entries appear on both `--allow` and `--deny` (`P:105`). `host` is allow-only, `style` is deny-only, and the Features are disable-only.

| id | evidenced default (tree) | default before `11a909a82` | proposed single list (default-error → `--allow`, default-warn → `--deny`) |
|---|---|---|---|
| `missing-hyperlinks` | **warn** (file input), exit 0; stdin: not checked | error, exit 1 | `--deny` |
| `missing-image-refs` | **warn**, exit 0 | error | `--deny` |
| `missing-transclusions` | **warn**, directive removed, exit 0 | error | `--deny` |
| `ctx-override` | error | error | `--allow` |
| `invalid-frontmatter-assignment` | error | error | `--allow` |
| `reassigned-frontmatter-property` | error | error | `--allow` |
| `shell-timeout` | error (from test) | error | `--allow` |
| `untyped-map-variables` | error | error | `--allow` |
| `untyped-filter-variables` | error | error | `--allow` |

What a single list would cost in today's semantics:

- **Dropping `--allow` for the three `missing-*` entries** loses the `--deny all --allow missing-hyperlinks` carve-out. The exemption comes from `blocking_policy` at `P:606-609`, and `T::a_denied_missing_link_inside_inserted_text_blocks` relies on it.
- **Dropping `--deny` for the six default-error entries** loses the `--allow all --deny <id>` carve-out. Tests that rely on it:
  - `T::a_specific_denial_beats_allow_all…`, which uses `missing-hyperlinks`, so it would be unaffected;
  - `compose_collection_controls.rs::*_recovery_precedence_and_conflicts`;
  - the "deny by code" spelling.

  A code such as `dm.context.ctx_override` would fall through `P:331-334` to `PolicySelector::Code`. It would then add a blocking code but no longer turn off the recovery (inferred).
- With a single list, the conflict check at `P:554-566` becomes dead for recoveries.

## Findings

**F1. Default missing-reference recovery breaks tests that still assert exit 1** (inferred from probes; tests not run, per instructions). With the tree binary:
- `md compose link.md|image.md|part.md` exits 0.
- `md compose --allow missing-image-refs link.md` exits 0.

These contradict:
- `T::each_missing_reference_allowance_recovers_its_own_kind_only` (`T:180-210`), which expects `.code(1)` with no flag and for the wrong-kind allowance;
- `cli/tests/l1/compose_lifecycle.rs::a_missing_include_runs_failed_then_finalize` (`:89-96`, expects status 1 and a `failed` event). The probe `life.md` gives exit 0 with events initialize then finalize;
- `cli/tests/l1/compose_file_interpolate.rs::every_reference_gate_names_a_mapped_copys_missing_link` (`:387-405`), whose `compose` gate expects 1. The probe `mapped/root.md` gives exit 0, `"outcome": "success"`.

**F2. With the new default, malformed hyperlink, image and transclusion references no longer fail compose.** The refuse filter at `C:803-806` exempts by **kind** (`recovers_missing(policy, i.kind)`), not by code. Any Error-severity issue of a recovered kind therefore passes, whether `InvalidUrl`, a missing fragment, or `RemoteUnreachable`. The downgrade at `C:793-799` is correctly restricted to `MissingLocalTarget`.
- Probe `url.md` (`[a](http://[::1)`): `md compose` gives exit 0 with "malformed URL in hyperlink" on stderr. `md validate refs` gives exit 1, and `--deny missing-hyperlinks` gives exit 1.
- Before `11a909a82` the same leak existed, but only under `--allow missing-hyperlinks` or `--allow all`. Now it is the default.

**F3. `style` is accepted on `md compose --deny` but never consulted there.**
- The registry lists it for compose (`P:230`), and compose help and completion list it.
- Compose never calls `denies("style")`, and `style` has `codes: &[]`, so `blocking_policy()` (`P:600`) gets nothing from it.
- Probe: `md compose --disable baseline-schema --deny style style.md` exits 0 (`--deny all` also exits 0, because compose emits no style diagnostic). Without `--disable baseline-schema`, compose fails with or without `--deny style`, because the baseline schema rejects the key.
- `T:497`, `fixture.command().args(["compose", "--deny", "style", "--output", "html"])…failure()`, is **vacuous**. `md compose --output html` is itself a usage error (exit 2: `html` is not a compose output). So the test passes for the wrong reason.
- `docs/cli/policy.md:117-126` describes `style` for rendering only. Line 38 lists `style` under compose `--deny` anyway.
- Compose also does not accept `--deny dm.style.unknown_key`, because the `dm.style.*` codes are absent from `COMPOSE_DIAGNOSTIC_CODES` (`lib/src/markdown/compose/policy.rs:78-118`).

**F4. Hyperlink and image policy only applies to file input.**
- `check_references` runs only when `resolved_input.is_some() && lifecycle.is_none()` (`C:369`), or in lifecycle runs on file input (`C:518`).
- On stdin, `--deny missing-hyperlinks` and `--deny missing-image-refs` are accepted no-ops. Probe: `md compose - --deny missing-hyperlinks < link.md` exits 0 with no diagnostic.
- `missing-transclusions` still acts on stdin through the library option (`C:355`), but only for an absolute or anchored target.

**F5. For the six default-error recoveries, `--deny <id>` alone is the same as no flag** (probes for ctx, inv, reas and map). The reason is structural: their diagnostic exists only when recovered, so nothing remains for the denial to block. It matters only to cancel `--allow all`.

**F6. In the tree, `--allow <id>` for the three `missing-*` entries does nothing without `--deny all`, and the same goes for `--allow all` over them.**
- Help still lists them under `--allow`, with summaries saying "(the default; `--deny` blocks it)" (`P:141,148,155`).
- `--deny <id>` now does more than cancel `--allow all`: it turns the default recovery off (missing targets refuse again), and it blocks the warning-severity variants (inserted-text and empty targets).
- Before the change, `--deny <id>` alone was meaningful only for warning-severity variants. Missing-local targets already refused by default. `EmptyTarget` did not exist before this commit (`lib/src/markdown/reference/validate.rs` diff).

**F7. Docs drift relative to the tree** (README-level claims that a missing reference fails by default):
- `docs/cli/policy.md:66-68`: "Without the allowance the problem fails the command". Lines 17-18 and 52-54 still present `--allow missing-hyperlinks` as needed.
- `docs/cli/compose.md:277` says compose "exits with status 1" on missing targets. Lines 169-170 and 299-310 also need updating.
- `docs/use-skills/use-darkmatter/cli.md:208` lists the `missing-*` entries as `--allow` recoveries.
- The `docs/cli/policy.md:171-185` flowchart has no "default recovery" branch.

**F8. Stale removed-flag hint in a library error.** `lib/src/markdown/compose/context/merge.rs:36` tells the user to "pass `--allow-override`". That flag never existed. The real flag is `--allow ctx-override`; the old spelling was `--allow-ctx-override`. Seen in probe `ctx.md`.

**F9. Legacy flags.** `REMOVED_FLAGS` (`cli/src/main.rs:136-153`) contains 13 flags:
- `--allow-missing-hyperlinks`
- `--allow-missing-image-refs`
- `--allow-missing-transclusions`
- `--allow-any-missing-reference`
- `--allow-ctx-override`
- `--allow-invalid-frontmatter-assignment`
- `--allow-reassigned-frontmatter-property`
- `--allow-shell-timeout`
- `--allow-host`
- `--no-baseline-schema`
- `--no-schema-triggers`
- `--no-trigger-schemas`
- `--strict-style`

`reject_removed_flags` (`main.rs:160-173`) only exits with a clap `UnknownArgument` usage error. It runs before parsing, on every argument before `--`, with `=value` stripped. None of these flags is declared or consulted anywhere else; grep of `cli/src` finds them only there.
- `--no-trigger-schemas` is missing from the docs table at `docs/cli/policy.md:364-377`.
- Side effect (inferred): any argv token equal to one of these spellings is rejected, even as the value of another option, for example `--state --strict-style`.
- Test: `T::every_removed_flag_is_a_usage_error_naming_its_replacement` (`T:129`).

**F10. No other policy-like flags bypass the registry.**
- The only `--allow`, `--deny` and `--disable` arguments use `PolicyValueParser`:
  - `cli/src/args/command.rs:64` (clean disable) and `:127,137,147` (compose allow, deny, disable);
  - `cli/src/args/render.rs:54` (render and root deny);
  - `cli/src/args/target.rs:92` (schema validate disable).
- `md --deny … compose` (a top-level option beside a subcommand) is rejected; `T:101-116` covers it. The root `--deny` is hidden in root help (`cli/src/args/cli.rs:39`).
- `path_completion.rs:225-265` re-reads `--disable` from the command line for setter completion, but parses it through the registry (`parse_selector`).
- `md validate refs` and `md graph --validate` take no policy at all.

**F11. Accepted but ineffective, by command:**
- `style` on compose (F3).
- `missing-hyperlinks` and `missing-image-refs` on compose with stdin input (F4).
- `baseline-schema` and `schema-triggers` on `md schema validate` with a YAML input (documented).

Every other entry is consulted on every command that accepts it.
