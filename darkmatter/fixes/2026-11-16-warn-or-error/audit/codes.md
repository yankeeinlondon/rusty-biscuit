# `--deny <CODE>` audit: does each listed code do anything?

Tree: `/Volumes/coding/wt/rusty-biscuit/fix-sequences` at HEAD `baee016e5` (no
uncommitted changes under `darkmatter/` apart from the untracked spec
`darkmatter/fixes/2026-11-16-warn-or-error/`). Probes were run with
`target/debug/md` (built 16:47, newer than every `.rs` source). The probe
fixtures are in `(probe fixtures, not kept)`, and `probe.sh` there prints exit code,
outcome, and `[code, severity]` from `md compose --output json`.

Paths are relative to `darkmatter/` unless they start with `simplified-schema/`
or `biscuit-file/`. `lib/` = `darkmatter/lib/src/`, `cli/` = `darkmatter/cli/src/`.

## How a deny takes effect (needed to read the table)

- `--deny CODE` resolves in `cli/args/policy.rs:324-333`. A code that a
  recovery governs (`POLICY_REGISTRY.codes`) becomes **that recovery's entry**
  (`PolicySelector::Entry`). Any other code becomes `PolicySelector::Code`.
- `ResolvedPolicy::blocking_policy` (`cli/args/policy.rs:595-611`) produces
  `BlockingPolicy::deny_code`. `BlockingPolicy::blocks`
  (`lib/markdown/compose/policy.rs:168-178`) never blocks `Severity::Info`.
  It does block an Error-severity warning, but errors already fail the
  document, so the blocking policy only matters for **warnings**.
- For a **recovery** code, denying it also flips `ResolvedPolicy::recovers`
  (`cli/args/policy.rs:574-578`): the recovery is turned off and the problem
  becomes a hard error. That is how deny works for all 9 recovery codes.
  The BlockingPolicy path cannot be reached for a *recovered* warning of a
  recovery code, for these reasons:
  - With `--deny <id|code>`, `recovers` is false unless the id is also allowed
    by name, and allowing it by name is a usage conflict
    (`cli/args/policy.rs:555-566`).
  - With `--deny all`, a recovery recovers only if it is named by `--allow`.
    It is then exempt (`cli/args/policy.rs:606-609`), and `--allow all --deny all`
    is a conflict (`:552-554`).
  - The only blockable warnings with a recovery code are ones the recovery
    does not produce: an **empty** link or image target, and a missing target
    **inside inserted text**. Both are always warnings
    (`cli/commands/compose.rs:808-825`).
- The places that judge warnings are `call_precheck.rs:316-321` (passive
  call check), `pipeline::stop_if_blocked`/`stop_if_blocked_with`
  (`lib/markdown/compose/pipeline/mod.rs:850-935`), `blocking_guard` (`:949`),
  `ReportWatch` (`:1001-1045`, inactive under `defer_expression_failures`,
  `:1014`), `check_references` (`cli/commands/compose.rs:781-832`), and the
  final `blocked_diagnostics(&blocking, &report.warnings, &[])`
  (`cli/commands/compose.rs:613`). For render they are `cli/commands/render.rs:138`
  (link titles) and `render.rs:193-235` (style).

## Table

Severity and default columns marked "probe" were observed with `md compose
--output json` (or `md render`). Every other entry comes from reading the code.

| code | constant (path) | emitted at | severity | judged by deny? | reachable from md compose/render? | deny test | default (not denied) |
|---|---|---|---|---|---|---|---|
| `dm.expression.incompatible_call` | `expression::INCOMPATIBLE_CALL_CODE` (`lib/markdown/compose/expression/diagnostics.rs:39`) | `expression/diagnostics.rs:292` (severity from `call_severity`, `:76` = always Error); rejection via `rejection_diagnostic` `:320` in `call_precheck.rs` | **error only** (probe) | **no-op**: always fails | yes (probe `inc.md`: `{{ min("abc", 5) }}`) | none (and none needed) | fails (exit 1) |
| `dm.expression.uncertain_call` | `expression::UNCERTAIN_CALL_CODE` (`expression/diagnostics.rs:44`) | `expression/diagnostics.rs:294` → warning built at `call_precheck.rs:439-476` | warning (probe) | **yes**, at `call_precheck.rs:316-321` | yes (probe `unc.md`) | CLI `policy_verbs.rs` `a_blocking_diagnostic_found_after_an_effect_stops_later_work_without_rollback` (:437); lib `blocking_policy.rs` `a_transcluded_child_blocks_before_its_own_expressions_run`, `report_diagnostics.rs` `a_policy_stop_reports_every_warning_with_its_own_severity`, `expression_recovery.rs` `pending_nested_blocking_diagnostic_stays_fatal` | warns, continues |
| `dm.expression.always_false_call` | `expression::ALWAYS_FALSE_CALL_CODE` (`expression/call_check.rs:896`) | `expression/diagnostics.rs:308` → `call_precheck.rs:439` | warning (probe) | **yes** (`call_precheck.rs:316`) | yes (probe `af.md`: `{{ file_exists("") }}`) | CLI `invalid_trigger_warnings.rs` `compose_lifecycle_blocked_call_warning_keeps_the_warning` (:385); lib `blocking_policy.rs` `a_blocking_call_warning_stops_the_document_before_any_expression_runs` | warns, continues |
| `dm.expression.incomplete_analysis` | `expression::host::INCOMPLETE_ANALYSIS_CODE` (`expression/host/mod.rs:74`) | `expression/diagnostics.rs:330`, only for `FunctionState::Incomplete` (`host/mod.rs:586`), which is built only in `HostFunctions::analysis_only` (`host/artifact.rs:118-156`, called at `lib/markdown/schemas/catalog.rs:803`). Sets are made visible only by `.enter()` in `dmls/src/providers/dsl.rs:351,856` | warning | would be (`call_precheck.rs:316`) | **no**: `md` never installs or enters host functions (there is no `install_host_functions`/`HostFunctions::enter` caller outside DMLS and tests), so `host::lookup` (`host/mod.rs:530-545`) finds nothing | none | n/a (never emitted) |
| `dm.expression.undeclared_property` | `ComposeWarning::UNDECLARED_PROPERTY_CODE` (`lib/markdown/compose/context/report.rs:771`) | `report.rs:1017`, via `unknown_identifiers.rs:119` | warning (probe) | **yes** (runtime `blocking_guard`/`ReportWatch`/final check) | yes (probe `undeclared.md`: `{{ missing }}`) | lib `runtime_blocking.rs` `a_stop_within_a_stage_keeps_its_earlier_warnings`, `a_stop_keeps_the_warnings_reported_before_it`; `expression_recovery.rs` `unrelated_warning_survives_recovery`. No CLI test by code | warns, continues |
| `dm.expression.undeclared_member` | `expression::UNDECLARED_MEMBER_CODE` (`expression/diagnostics.rs:49`) | `expression/member_check.rs:65` (candidate); reported at `member_sites.rs:389` | warning (probe) | **yes** | yes (probe `mem.md`) | CLI `compose_member_warnings.rs` `denying_the_code_fails_with_no_document`; lib `compose_member_warnings.rs` `a_placeholder_does_not_bypass_member_denial`, `ac8_same_stage_denial_still_blocks_first` | warns, continues |
| `dm.expression.unknown_context_variable` | `ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE` (`report.rs:769`) | `report.rs:994`, via `interpolation/evaluator.rs:514` (the `rewrite.rs:1314` hit is in tests) | warning (probe) | **yes** | yes (probe `ctx.md`: `{{ ctx.typo }}`) | lib `runtime_blocking.rs` `frontmatter_context_warning_stops_a_later_nested_shell`, `a_context_warning_stops_the_next_expression`; `diagnostic_source_provenance.rs` `a_policy_stop_keeps_every_warning_located` (+2). No CLI test by code | warns, continues |
| `dm.expression.parse_failure` | `ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE` (`report.rs:775`) | warning form: `interpolation/rewrite.rs:449-454` (Lenient only). Error form: `report_diagnostics.rs:689` (page-block condition), `:990` (`expression_code`) | **error** in md compose (probe `parse.md`). The warning exists only under `ExpressionFailurePolicy::Lenient` (`rewrite.rs:78-85`), i.e. `defer_expression_failures` (`context/options.rs:1487-1493`), which is set only by preflight discovery (`preflight/collect.rs:534`) and `for_reference_check` (`options.rs:1174`). Discovery truncates those warnings (`collect.rs:547`) | **no-op** | the warning is never delivered to md compose (preflight-only) | none | fails |
| `dm.expression.evaluation_failure` | `ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE` (`report.rs:777`) | warning form `rewrite.rs:433-438` (Lenient only); error form `report_diagnostics.rs:665, 692, 994` | **error** in md compose (probe `eval.md`) | **no-op** | the warning form is preflight-only, as for parse_failure | none | fails |
| `dm.glob.skipped_symlink` | `ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE` (`report.rs:782`) | `report.rs:1060`, via `glob_listing.rs:32` (`find_files()`, `expression/functions/fs.rs:1246`) and `transclusion/engine.rs:1478` (`::file-links`) | warning (probe) | **yes**: `find_files` warnings are drained at `pipeline/mod.rs:160` and judged at `cli/commands/compose.rs:613` | yes, but only when a relative boundary is in force, e.g. inside a git repo. Probe `gl/` was silent outside a repo and warned 2× after `git init`; the boundary comes from `biscuit-file/lib/src/file_reference/resolve.rs:1353-1363` | **none** | warns, continues |
| `dm.remote_cache.io_failure` | `RemoteFetchStats::CACHE_WARNING_CODE` (`lib/markdown/compose/remote_fetch.rs:86`) | `remote_fetch.rs:91`, projected at `pipeline/mod.rs:157`; source `remote_fetch.rs:642-662` | warning | **yes**, at the final `compose.rs:613` only, after all effects | yes, needs a remote fetch plus a failing persistent-cache write (inferred, not probed) | **none** (`runtime_blocking.rs` `a_shell_warning_stops_the_next_shell` is about the uncoded volatile-shell notice) | warns, continues |
| `dm.schema.missing_simplified_envelope` | `simplified_schema::SchemaAdvisory::MISSING_SIMPLIFIED_ENVELOPE_CODE` (`simplified-schema/lib/src/advisory.rs:42`) | projection `report.rs:814-835` via `schema_validation.rs:646`. The only constructor `SchemaAdvisory::missing_simplified_envelope` (`advisory.rs:24`) is called only from `simplified-schema/lib/src/triggers/prepared.rs:114` (deserializing a stored advisory) and a test | warning | would be | **no, dead**: no loader creates it. `lib/markdown/schemas/tests/mod.rs:97` and `lib/markdown/compose/tests/frontmatter.rs:270` say "The loader no longer emits" it, and `cli/tests/l1/schema_validate.rs:187` and `dmls/tests/l1/lsp_session.rs:183` assert its absence | none | n/a |
| `dm.schema.invalid_trigger` | `markdown::schemas::SCHEMA_INVALID_TRIGGER_CODE` (re-export `lib/markdown/schemas/mod.rs:2630` of `simplified-schema/lib/src/triggers/mod.rs:67`) | `report.rs:848-868` (`from_invalid_trigger`), via `schema_validation.rs:213` | warning | **yes** (also held and delivered on failure, `cli/commands/compose.rs:980-1000`) | yes | CLI `invalid_trigger_warnings.rs` `compose_deny_of_the_code_or_all_stops_before_writing_the_document` (:216), `compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell`; lib `compose/tests/invalid_triggers.rs` `denying_the_code_stops_before_the_first_effect` | warns, continues |
| `dm.reference.missing_hyperlink` | `policy::MISSING_HYPERLINK_CODE` (`policy.rs:21`) | `check_references` (`cli/commands/compose.rs:781-832`, code via `missing_reference_code` `policy.rs:65-72`); record `report_diagnostics.rs:134` | warning when recovered (the default), **error** when denied (probe `mhl.md`). Empty `[x]()` and inserted-text targets are warnings either way | deny turns the **recovery off** (real target → error). BlockingPolicy judges only empty and inserted-text targets (`compose.rs:812-825`) | yes | CLI `policy_verbs.rs` `an_empty_link_target_warns_with_the_link_and_blocks_only_when_denied`, `a_denied_missing_link_inside_inserted_text_blocks` (by id), `a_specific_denial_beats_allow_all_in_either_order` | **warns, continues** (`DEFAULT_RECOVERIES`) |
| `dm.reference.missing_image` | `policy::MISSING_IMAGE_CODE` (`policy.rs:23`) | as above | as above (probe `mimg.md`: warning by default, error when denied) | as above | yes | **none** for deny (only allow tests, e.g. `policy_verbs.rs` `each_missing_reference_allowance_recovers_its_own_kind_only`) | **warns, continues** |
| `dm.reference.missing_transclusion` | `policy::MISSING_TRANSCLUSION_CODE` (`policy.rs:25`) | `check_references` (as above); `transclusion/engine.rs:2055` (`skipped_transclusion_warning`); `pipeline/phases.rs:744` (`tolerated_failure_warning`) | warning by default, **error** when denied (probe `mtr.md`; with `ignore_invalid: true` too, probe `mtr_ign.md`) | deny turns the **recovery off**. No route found where the BlockingPolicy judges a recovered transclusion warning (inferred) | yes | **none** for deny | **warns, continues**; the directive is removed |
| `dm.context.ctx_override` | `policy::CTX_OVERRIDE_CODE` (`policy.rs:27`) | `pipeline/mod.rs:1271` | warning only when recovered. Unrecovered, the failure carries **another code**: `dm.schema.type_mismatch` (baseline) or `dm.compose.failure` (probe `ctxo.md`) | recovery-off only; the warning can never be blocked (see above) | yes | only conflict/usage tests (`policy_verbs.rs` `opposite_selections_are_usage_errors_in_either_order_before_any_effect`, `a_specific_recovery_beats_deny_all_in_either_order`) | **fails** (not a default recovery) → a bare `--deny` is a no-op; it matters only with `--allow all` |
| `dm.transclusion.invalid_frontmatter_assignment` | `policy::INVALID_FRONTMATTER_ASSIGNMENT_CODE` (`policy.rs:30`) | `transclusion/engine.rs:599` | warning only when recovered. Unrecovered: error `dm.transclusion.failure` (probe `ifa.md`) | recovery-off only | yes (`::file ./child.md set=42`) | none | **fails** → bare deny is a no-op |
| `dm.transclusion.reassigned_frontmatter_property` | `policy::REASSIGNED_FRONTMATTER_PROPERTY_CODE` (`policy.rs:32`) | `transclusion/engine.rs:629` | as above: unrecovered → `dm.transclusion.failure` error (probe `rfp.md`). `--allow … --deny all` → exit 0 | recovery-off only | yes | none | **fails** → bare deny is a no-op |
| `dm.shell.timeout_recovered` | `policy::SHELL_TIMEOUT_CODE` (`policy.rs:34`) | `shell_expansion/mod.rs:739` | warning only when recovered; unrecovered fails with "timed out" (`cli/tests/l1/compose_shell.rs` around :150) | recovery-off only (inferred from the same `recovers` logic; not probed, it needs shell approval) | yes (`--timeout` + `::shell`) | only `policy_verbs.rs` `a_recovery_denied_by_its_code_conflicts_with_its_allowance` (usage error); `runtime_blocking.rs` uses it as an *unrelated* denial | **fails** → bare deny is a no-op |
| `dm.config.not_saved` | `config::load::CONFIG_NOT_SAVED_CODE` (`lib/config/load/error.rs:14`) | `lib/markdown/schemas/slots/grants/gate.rs:375`; `shell_expansion/types.rs:1776` (code from `config/permissions/persist.rs:115`); `config/init.rs:139` is `md config init`, not compose | warning | **yes** (slot warnings appended at `pipeline/mod.rs:165`; shell ones at `inline/shell_expansion.rs:210`, `shell_blocks/mod.rs:370`) | yes, but only for an interactive "always allow" whose config write fails (inferred) | **none** | warns, continues |
| `dm.link.legacy_structured_title` | `policy::LINK_LEGACY_TITLE_CODE` (`policy.rs:39`) | `report.rs:936, 953`; render `cli/commands/render.rs:138` | warning (probe `lt.md`) | **yes** (compose final check; render `render.rs:138`) | yes, both commands (probe) | CLI `link_titles.rs` `compose_deny_names_each_code`, `render_warns_on_stderr_and_deny_blocks_it`; lib `link_title_warnings.rs` `a_denied_code_stops_the_document_with_the_warning` | warns, continues |
| `dm.link.malformed_structured_title` | `policy::LINK_MALFORMED_TITLE_CODE` (`policy.rs:37`) | as above | warning (probe `ltm.md`) | **yes** | yes, both (probe) | CLI `link_titles.rs` `compose_deny_names_each_code` (compose only; no render deny test for this code) | warns, continues |
| `dm.collection.malformed` | `collection_blocks::diagnostics::MALFORMED_CODE` (`lib/markdown/compose/collection_blocks/diagnostics.rs:13`); not `expression::data::MALFORMED_CODE` (`dm.data.malformed`) | `collection_blocks/error.rs:119` → `report_diagnostics.rs:700` | **error only** (probe `cmal.md`) | **no-op** | yes | none | fails |
| `dm.collection.untyped_map_source` | `…::UNTYPED_MAP_SOURCE_CODE` (`diagnostics.rs:16`) | warning `collection_blocks/analysis.rs:256-261` (recovered); error `error.rs:120` | error by default; warning under `--allow untyped-map-variables`/`all` (probe `umap.md`) | recovery-off only. Probe: `--allow all --deny <code>` → error exit 1; `--allow untyped-map-variables --deny all` → exit 0 | yes | CLI `compose_collection_controls.rs` `map_recovery_precedence_and_conflicts` | **fails** → bare deny is a no-op |
| `dm.collection.untyped_filter_source` | `…::UNTYPED_FILTER_SOURCE_CODE` (`diagnostics.rs:19`) | `analysis.rs:257`; `error.rs:121` | same as map (probe `ufil.md`) | recovery-off only | yes | CLI `compose_collection_controls.rs` `filter_recovery_precedence_and_conflicts` | **fails** → bare deny is a no-op |
| `dm.collection.invalid_source` | `…::INVALID_SOURCE_CODE` (`diagnostics.rs:22`) | `error.rs:122` → `report_diagnostics.rs:700` | **error only** (probe `cinv.md`) | **no-op** | yes | none | fails |
| `dm.expression.invalid_callback` | `collection_blocks::diagnostics::INVALID_CALLBACK_CODE` = re-export of `simplified-schema/lib/src/expression/parser.rs:797` | `expression/diagnostics.rs:288`; `report_diagnostics.rs:993`; `collection_blocks/error.rs:123` | **error** (probes `icb.md`, `cpat`-style `[a, b] => a`). A Warning form is possible in principle when an `InvalidCallback` call has a category-3 finding (`diagnostics.rs:286-296`), but none was seen (inferred: always error) | no-op in practice | yes | none | fails |
| `dm.collection.pattern_mismatch` | `…::PATTERN_MISMATCH_CODE` (`diagnostics.rs:25`) | `error.rs:124` | **error only** (probe `cpat.md` `{x} => x`) | **no-op** | yes | none | fails |
| `dm.collection.invalid_sort_key` | `…::INVALID_SORT_KEY_CODE` (`diagnostics.rs:28`) | `error.rs:125` (built at `analysis.rs:243`) | **error only** (probe `csort.md`) | **no-op** | yes | none | fails |
| `dm.collection.callback_failed` | `…::CALLBACK_FAILED_CODE` (`diagnostics.rs:31`) | `error.rs:128` | **error only** (probe `cfail.md`) | **no-op** | yes | none | fails |
| `dm.iteration.visit_limit` | `…::VISIT_LIMIT_CODE` = re-export of `expression/iteration.rs:19` | `iteration.rs:202` → `error.rs:127,130`, `report_diagnostics.rs:988` | **error only** (probe `--max-iteration-visits 1`) | **no-op** | yes | none | fails |
| `dm.iteration.expansion_limit` | `…::EXPANSION_LIMIT_CODE` = re-export of `iteration.rs:22` | `iteration.rs:203` → same sites | **error only** (probe `--max-expansion-bytes 2`) | **no-op** | yes | none | fails |
| `dm.style.unknown_key` (render list) | `policy::STYLE_UNKNOWN_KEY_CODE` (`policy.rs:41`) | `lib/style/layer.rs:457, 743, 776` | warning | **yes**, at `cli/commands/render.rs:214` (records re-marked Error, `:231-233`) | render: yes (probe `sty.md`: exit 0 → exit 1 when denied). Compose rejects the code as a usage error (probe), although `--deny style` is accepted there | CLI `output_commands.rs` `render_deny_takes_the_style_codes_it_reports`; `render_style_carried_warnings.rs` `denying_the_unknown_key_code_keeps_the_other_warnings` | warns, continues |
| `dm.style.deprecated` (render list) | `policy::STYLE_DEPRECATED_CODE` (`policy.rs:43`) | `layer.rs:744`; `cli/commands/render.rs:306` (inline rule) | warning | **yes** (`render.rs:214`) | render: yes (probe `styd.md`) | `render_dispatch.rs` `denying_style_blocks_an_inline_rule_style_and_a_warning_code_blocks_on_its_own`; `render_style_carried_warnings.rs` `denying_the_deprecation_code_keeps_the_other_warnings`; `output_commands.rs` `render_deny_takes_the_style_codes_it_reports` | warns, continues |
| `dm.link.legacy_structured_title` (render list) | same as compose row | `render.rs:138` | warning | yes | yes (probe) | `link_titles.rs` `render_warns_on_stderr_and_deny_blocks_it` | warns |
| `dm.link.malformed_structured_title` (render list) | same | `render.rs:138` | warning | yes | yes (probe: `md render ltm.md --deny …` exit 1) | **none on render** | warns |

## The 9 recovery identifiers without `--allow <id>`

| id | code | severity with no flags | where the default comes from |
|---|---|---|---|
| `missing-hyperlinks` | `dm.reference.missing_hyperlink` | **warning**, exit 0 (probe) | `DEFAULT_RECOVERIES` (`cli/args/policy.rs:132`) + `recovers` (`:574-578`) |
| `missing-image-refs` | `dm.reference.missing_image` | **warning**, exit 0 (probe) | same |
| `missing-transclusions` | `dm.reference.missing_transclusion` | **warning**, exit 0, directive removed (probe) | same |
| `ctx-override` | `dm.context.ctx_override` | error, reported under a **different code** (`dm.schema.type_mismatch` with the baseline, `dm.compose.failure` without) | not a default recovery |
| `invalid-frontmatter-assignment` | (`dm.transclusion.invalid_frontmatter_assignment`) | error `dm.transclusion.failure` | not default |
| `reassigned-frontmatter-property` | (`dm.transclusion.reassigned_frontmatter_property`) | error `dm.transclusion.failure` | not default |
| `shell-timeout` | (`dm.shell.timeout_recovered`) | error ("timed out") (inferred from `compose_shell.rs` test, not probed) | not default |
| `untyped-map-variables` | `dm.collection.untyped_map_source` | error under its own code (probe) | not default |
| `untyped-filter-variables` | `dm.collection.untyped_filter_source` | error under its own code (probe) | not default |

Where the default came from: `DEFAULT_RECOVERIES` is **not an uncommitted
change**. It was committed in `11a909a82` ("fix(darkmatter): report empty
reference targets through policy", 2026-11-16 16:41), which is in HEAD;
`git status` shows nothing modified under `darkmatter/`. The untracked spec
`darkmatter/fixes/2026-11-16-warn-or-error/spec.md:177-186` calls it
"Interim, replace under D2". The same spec (`:201-215`) says 48 CLI L1 tests
fail with the interim defaults.

## Findings

### Dead or unreachable codes (listed, but `md compose` never reports them)

1. `dm.expression.incomplete_analysis`: incomplete host functions exist only in
   `HostFunctions::analysis_only` (`lib/markdown/compose/expression/host/artifact.rs:118-156`;
   its only caller is `lib/markdown/schemas/catalog.rs:803`). They become visible
   only through `HostFunctions::enter`, which only DMLS calls
   (`dmls/src/providers/dsl.rs:351,856`). `install_host_functions` has no
   non-test caller. Reachable from DMLS only.
2. `dm.schema.missing_simplified_envelope`: never constructed by any loader.
   The only producer is a deserializer round-trip
   (`simplified-schema/lib/src/triggers/prepared.rs:114`). Test comments
   (`lib/markdown/schemas/tests/mod.rs:97`) and absence assertions
   (`cli/tests/l1/schema_validate.rs:187`) confirm it is dead.
3. `dm.expression.parse_failure` / `dm.expression.evaluation_failure` **as
   warnings**: produced only under `ExpressionFailurePolicy::Lenient`
   (`interpolation/rewrite.rs:78-85`). That policy applies only to preflight
   discovery (`preflight/collect.rs:534`, warnings truncated at `:547`) and the
   reference check (`context/options.rs:1174`, which judges reference issues
   only). In a real compose they are errors (probe).

### `--deny` is a no-op (the code is always an error, yet listed as deny-able)

11 codes: `dm.expression.incompatible_call` (its catalog text even says "always
an error", `policy.rs:79`), `dm.expression.parse_failure`,
`dm.expression.evaluation_failure`, `dm.expression.invalid_callback`
(inferred: always error), `dm.collection.malformed`, `dm.collection.invalid_source`,
`dm.collection.pattern_mismatch`, `dm.collection.invalid_sort_key`,
`dm.collection.callback_failed`, `dm.iteration.visit_limit`,
`dm.iteration.expansion_limit`. Each was probed: the same error and exit 1
with or without `--deny <code>`.

### `--deny` on a non-default recovery code only means "not `--allow all`"

For `dm.context.ctx_override`, `dm.transclusion.invalid_frontmatter_assignment`,
`dm.transclusion.reassigned_frontmatter_property`, `dm.shell.timeout_recovered`,
`dm.collection.untyped_map_source`, and `dm.collection.untyped_filter_source`:

- A bare `--deny <code>` does nothing, because the default is already a hard error.
- It changes the outcome only when combined with `--allow all`
  (`cli/args/policy.rs:574-578`).
- The BlockingPolicy can never block their *warning*, as argued above.
- When the recovery is off, three of them fail under a **different code** than
  the one denied. Probes: `ctx_override` → `dm.compose.failure` or
  `dm.schema.type_mismatch`; `invalid_frontmatter_assignment` and
  `reassigned_frontmatter_property` → `dm.transclusion.failure`. So `--deny X`
  can fail with a report that never mentions `X`.

### Default-recovered reference codes

- `--deny dm.reference.missing_*` works by turning the recovery off. The real
  missing target becomes an error under the same code (probe).
- The BlockingPolicy path matters only for empty `[x]()`/`![x]()` targets and
  for missing targets in inserted text (`cli/commands/compose.rs:808-825`).
- There is no deny test for `missing_image` or `missing_transclusion`.
- Probe: `--allow missing-hyperlinks --deny all` keeps an inserted-text missing
  link as a non-blocking warning (exit 0).

### Tests and docs that contradict the tree

- `cli/tests/l1/policy_verbs.rs` `each_missing_reference_allowance_recovers_its_own_kind_only`
  (:179-191) asserts that `md compose` of a missing link, image, or transclusion
  exits **1** with no flags. The binary exits **0** with a warning (probes
  `mhl.md`, `mimg.md`, `mtr.md`). The test presumably fails (inferred, not
  run). The spec lists it among the 48 failures.
- `docs/cli/policy.md:66-68` still says "Without the allowance the problem
  fails the command". It never says the three missing-reference recoveries are
  on by default. The registry summaries do say "(the default; `--deny` blocks it)"
  (`cli/args/policy.rs:141,148,155`).

### Codes listed on both commands, and asymmetries

- Codes listed for both compose and render: `dm.link.legacy_structured_title`
  and `dm.link.malformed_structured_title` (`policy.rs:57-58` and `:100-101`).
  Both work on both commands (probes). There is no render deny test for the
  malformed code.
- `dm.style.unknown_key` and `dm.style.deprecated` are render-only codes, yet
  `--deny style` is accepted on compose (`cli/args/policy.rs:239-245`).
  `md compose --deny dm.style.unknown_key` is a usage error (probe, exit 2).

### Codes that are real but have no deny test

`dm.glob.skipped_symlink` (works by probe, inside a git repo only),
`dm.remote_cache.io_failure`, `dm.config.not_saved`,
`dm.reference.missing_image`, `dm.reference.missing_transclusion`, and, as a
real blocking test (not just usage-conflict tests), all six non-default
recovery codes.

### Other surprises

- `dm.remote_cache.io_failure` is judged only at the end (`compose.rs:613`),
  because cache warnings are appended after the whole pipeline
  (`pipeline/mod.rs:157`). A deny there stops output but not earlier effects.
  The same applies to `dm.glob.skipped_symlink` raised by `find_files()`
  (`pipeline/mod.rs:160`).
- `dm.glob.skipped_symlink` depends on a tree boundary that exists only when a
  relative boundary is in force. A symlink to `/etc/hosts` was listed silently
  outside a git repo and skipped with a warning inside one (probe).
