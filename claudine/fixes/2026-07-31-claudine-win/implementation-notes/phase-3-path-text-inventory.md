# Phase 3 path-to-text inventory

This inventory was refreshed on native Windows after the library path-rendering
commit (`4b664428d`) with the CLI/generator Phase 3 worktree changes and the
follow-up schema-diagnostic and permission-query conversions present. It covers
Rust sources under `claudine/lib/src`, `claudine/cli/src`, and
`claudine/gen/src` only.

The exact search was:

```powershell
rg -n '\.display\(\)|to_string_lossy\(\)|file://' `
  claudine/lib/src claudine/cli/src claudine/gen/src -g '*.rs'
```

`D`, `L`, and `U` below mean `.display()`, `to_string_lossy()`, and a literal
`file://`, respectively. Line numbers describe this snapshot and should be
re-located by expression after later edits.

## Counts and historical comparison

The pre-edit `408 hits / 117 files` figure is reproducible against
`cdbc33e9b`, the parent of the library-rendering commit, when a hit means one
matching source line and explicit `tests.rs` and `tests/` paths are excluded.
That historical count includes matches inside inline `mod tests` blocks. A
strict reclassification of the same snapshot produces 380 production lines,
384 occurrences, and 113 files. This distinction explains why the historical
figure must not be compared directly with the strict production count below.

The current worktree has:

| Scope | Matching lines | Occurrences | Files |
|---|---:|---:|---:|
| Strict production | 136 | 137 | 66 |
| Tests and fixtures | 122 | 122 | 48 |
| Comments | 0 | 0 | 0 |
| Union | 258 | 259 | 106 |

The extra production occurrence is the two `to_string_lossy()` calls on
`wrapper_stages.rs:186`. By spelling, the current totals are 129 `D`
occurrences, 117 `L` occurrences on 116 lines, and 13 `U` occurrences.

Relative to the historical 408-line inventory, 150 matching lines have been
removed. Most importantly, there are **zero production hand-built `file://`
URLs**. All 13 remaining `U` occurrences are test fixtures or assertions for
`Url::from_file_path` output.

## Production classification

### Native OS, argv, environment, and provider-configuration boundaries

These values are delivered to the OS, parsed as native command-line state, or
written into provider configuration that names a native executable/path. They
must not use portable display spelling.

| File | Hits | Reason to remain native |
|---|---|---|
| `cli/src/argv/partition.rs` | L264,309 | Converts opaque `OsString` argv tokens for the parser's existing lossy boundary. |
| `cli/src/commands/wrap/composition/pipeline.rs` | L536,925,928,934,935 | Reads child environment and system-prompt delivery values. |
| `cli/src/commands/wrap/env/mod.rs` | D430 | Supplies the native child working directory to launch setup. |
| `cli/src/commands/wrap/env/sanitize.rs` | L51,108 | Classifies native environment keys; it is not display output. |
| `cli/src/commands/wrap/launch_plan.rs` | L441,720,769,935 | Handles environment keys/values, provider arguments, and temporary output paths. |
| `cli/src/commands/wrap/policy.rs` | L23 | Builds provider argv from a native path. |
| `cli/src/commands/wrap/profile/claude.rs` | D35,47 | Passes temporary-file paths to the provider. |
| `cli/src/commands/wrap/profile/kimi.rs` | D39,45 | Passes temporary-file paths to the provider. |
| `cli/src/commands/wrap/profile/opencode.rs` | D65,77 | Writes/passes the temporary instruction-file path expected by OpenCode. |
| `cli/src/commands/wrap/repo_home.rs` | L143,207 | Interprets native filenames while constructing the shadow home. |
| `cli/src/commands/wrap/session_report.rs` | L61 | Reads an `OsString` environment value. |
| `cli/src/commands/wrap/system_prompt.rs` | D245,269,290 | Passes temporary paths through argv/environment delivery. |
| `cli/src/commands/wrap/wrapper_stages.rs` | L186 (twice) | Preserves provider-supplied environment keys and values. |
| `cli/src/completion/engine/mod.rs` | L345 | Reads a native clap candidate value; presentation occurs later. |
| `gen/src/main.rs` | L379 | Converts a filesystem entry name into the generator's slug input. |
| `lib/src/actions/bash_executor.rs` | L85 | Returns the resolved executable used by process launch. |
| `lib/src/config/codex.rs` | L104,252 | Serializes and compares the native notification executable in Codex config. |
| `lib/src/config/mod.rs` | L53 | Resolves the executable command used by generated hook commands. |
| `lib/src/mcp/export.rs` | L345 | Writes native `cwd` into provider configuration. |
| `lib/src/mcp/inject.rs` | L195 | Writes native `cwd` into the runtime provider overlay. |
| `lib/src/provider/path_template.rs` | L132,133 | Expands home/repository placeholders into native provider paths. |

### Filesystem identity, matching, persistence, and session identity

These strings are comparison keys, resolver inputs, persisted identity, hash
inputs, or policy grammar. Portable rendering would change behavior rather
than presentation.

| File | Hits | Reason to remain native |
|---|---|---|
| `cli/src/commands/compose/prep.rs` | D179,246; L335 | `D` fields are tracing; `L335` feeds the document resolver. |
| `cli/src/commands/mcp/list.rs` | L166 | Looks up the existing native repository-state key. |
| `cli/src/commands/schema_interactive/mod.rs` | D686 | Supplies the executable schema file-reference value. |
| `cli/src/commands/wrap/harness_orch/session_key.rs` | D107,111; L130,200,203 | Defines resumable-session identity and hashes delivered environment state. |
| `cli/src/commands/wrap/live_semantic_sink/mod.rs` | D729 | Preserves operational CWD in `EventMeta`; `dispatch/runner/mod.rs:80-86,144-149` converts it back with `PathBuf::from` for condition context and action working-directory behavior. |
| `cli/src/commands/wrap/sequence/iterate.rs` | L663 | Feeds the resolved handoff target back into the document resolver. |
| `cli/src/commands/wrap/sequence/phase1c.rs` | D304 | Feeds a source path back into composition resolution. |
| `cli/src/commands/wrap/sequence/task_run.rs` | D281 | Feeds the requested task path into composition resolution. |
| `cli/src/commands/wrap/wrapper_stages.rs` | D493 | Retains the original document reference in runtime state. |
| `cli/src/completion/operation_file.rs` | D124 | Internal candidate sort key; emitted values are rendered separately. |
| `lib/src/linking/agents.rs` | L187,457 | Tests provider read-through paths for `.claude/agents`. |
| `lib/src/linking/commands.rs` | L200,537 | Tests provider read-through paths for `.claude/commands`. |
| `lib/src/linking/hashing.rs` | L45 | Feeds a relative path into the content-identity hash. |
| `lib/src/linking/skills/native.rs` | L65 | Tests provider read-through paths for `.claude/skills`. |
| `lib/src/linking/skills/portable.rs` | L207 | Tests provider read-through paths for `.claude/skills`. |
| `lib/src/mcp/state.rs` | L186,210,214 | Canonical repository keys and fallback persisted identity. |
| `lib/src/permissions/matchers.rs` | L13 | Enters the private separator-neutral comparison grammar. |
| `lib/src/permissions/providers/claude.rs` | L320,803,808,813,818,944,961 | Builds/normalizes policy path patterns and fixtures used by the matcher. |
| `lib/src/permissions/providers/codex.rs` | L342,380,586,903 | Builds policy patterns; L903 is provider argv. |
| `lib/src/permissions/providers/gemini.rs` | L107,138,171,363 | Interprets native config filenames and builds policy patterns. |
| `lib/src/permissions/providers/goose.rs` | L209 | Builds a policy path pattern. |
| `lib/src/permissions/providers/kimi.rs` | L254 | Builds a policy path pattern. |
| `lib/src/permissions/providers/opencode.rs` | L261,545,551,557 | Builds and compares policy path patterns. |
| `lib/src/permissions/providers/qwen.rs` | L324,676,994 | Builds and compares policy path patterns. |
| `lib/src/protect/path.rs` | L76,88,245,252 | Native paths enter the separator-neutral security comparison boundary. |
| `lib/src/protect/service.rs` | L154 | Supplies the resolved path to the sensitive-path checker. |
| `lib/src/reporting/ingest.rs` | D179,334,379,849 | Persists and queries the SQLite `source_file` identity; presentation must convert separately. |
| `lib/src/system_prompt/change_state.rs` | L34 | Feeds native scope identity into the hash. |

### Tracing-only structured fields

The plan explicitly permits native spelling for tracing-only structured
fields. These sites do not construct user-facing completion values or links.

| File | Hits |
|---|---|
| `cli/src/commands/sequence.rs` | D284 |
| `cli/src/commands/wrap/exec/stream_capture.rs` | D69,88,99,137,150,172 |
| `cli/src/commands/wrap/harness_orch/attempt.rs` | D54 |
| `cli/src/commands/wrap/harness_orch/loop_control.rs` | D459,1186,1333,1415 |
| `cli/src/commands/wrap/harness_orch/loop_control/requeue.rs` | D205 |
| `cli/src/commands/wrap/mod.rs` | D329 |
| `cli/src/completion/walker.rs` | D166 |
| `cli/src/telemetry.rs` | D69,89,198 |
| `lib/src/composition/looping/engine.rs` | D406 |
| `lib/src/config/atomic.rs` | D133,137 |
| `lib/src/config/backup.rs` | D67 |
| `lib/src/config/migration.rs` | D52,53 |
| `lib/src/dispatch/logging.rs` | D59 |
| `lib/src/dispatch/mod.rs` | D241,242,409 |
| `lib/src/harness/parse/mod.rs` | D128,147,171 |
| `lib/src/messaging/send.rs` | D569,577 |
| `lib/src/signals/harvest.rs` | D195,260,285 |
| `lib/src/system_prompt/prepare.rs` | D403 |

## Test and fixture inventory

All entries below are non-production. `U` entries are URL fixtures/assertions,
not URL construction sites.

| File | Hits |
|---|---|
| `cli/src/argv/partition.rs` | L345 |
| `cli/src/cli_utils.rs` | U68,77 |
| `cli/src/commands/compose/prep/tests.rs` | L52 |
| `cli/src/commands/sequence.rs` | L423,480 |
| `cli/src/commands/wrap/env/tests.rs` | L413,506 |
| `cli/src/commands/wrap/exec/stream_capture.rs` | L262 |
| `cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs` | L493 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/budget_scoping.rs` | D31 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs` | D18,84,174 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs` | D158,230 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs` | D26,710,717 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs` | D37,59,121 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/recovery_identity.rs` | D166 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs` | D13 |
| `cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs` | D26,118 |
| `cli/src/commands/wrap/harness_orch/prompt.rs` | D292 |
| `cli/src/commands/wrap/harness_orch/session_key/tests.rs` | D210 |
| `cli/src/commands/wrap/live_semantic_sink/tests/provider_extension_and_opencode.rs` | U747 |
| `cli/src/commands/wrap/system_prompt/tests.rs` | L79 |
| `cli/src/completion/autocomplete_ui.rs` | U607,616 |
| `cli/src/completion/walker/tests.rs` | D88,112,144,197,225,241 |
| `lib/src/composition/agent_message.rs` | U196,200 |
| `lib/src/composition/error/tests.rs` | U45,589 |
| `lib/src/composition/lifecycle/control/tests.rs` | D455 |
| `lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs` | D41,113,197; L166,171,258,263 |
| `lib/src/composition/looping/config/tests.rs` | L19 |
| `lib/src/composition/looping/engine/tests/mod.rs` | L67,89 |
| `lib/src/composition/preflight/tests.rs` | D812 |
| `lib/src/composition/prepare/service/tests.rs` | D20 |
| `lib/src/composition/sequence/expr.rs` | D200 |
| `lib/src/composition/sequence/preflight/tests.rs` | D33,135,265; L159,211,440 |
| `lib/src/composition/sequence/task/tests.rs` | D59,1391,1420,1449,1492,2410 |
| `lib/src/composition/sequence/tests.rs` | D589 |
| `lib/src/config/backup.rs` | L103 |
| `lib/src/config/codex.rs` | D536 |
| `lib/src/dispatch/loader/tests.rs` | L12 |
| `lib/src/harness/report.rs` | U233 |
| `lib/src/harness/resolve/tests.rs` | D399; L244,255,263,311,322,330 |
| `lib/src/mcp/export.rs` | L752,802 |
| `lib/src/mcp/state.rs` | D296 |
| `lib/src/protect/path.rs` | D440,444,492,493,494,495,496,497,498,499; L485 |
| `lib/src/protect/scrub.rs` | D202 |
| `lib/src/protect/service/tests.rs` | D57,98,230; L354 |
| `lib/src/render/prompt/system/tests.rs` | D155; L241; U284 |
| `lib/src/stream/path_link.rs` | L136,153,186,212,229,240,260,273,289; U283,291 |
| `lib/src/system_prompt/context.rs` | D212,223,251 |
| `lib/src/system_prompt/prepare/tests.rs` | D512,805 |
| `lib/src/system_prompt/resolve/tests.rs` | D12,44,75,220 |

## Boundary audit

- Visible library/CLI path labels and completion insertions converted in Phase
  3 use `biscuit_file::to_portable_string` or `try_portable_string` at the
  presentation seam.
- Local hyperlinks use `Url::from_file_path`; there is no production
  concatenation of `file://` with display text.
- Filesystem calls and resolver inputs remain `Path`, `PathBuf`, or native
  spelling.
- Provider argv, child environment keys/values, executable paths, and temporary
  file arguments remain native.
- Permission/protect matching never consumes portable rendered text; it uses
  the private comparison grammar introduced in Phase 1.
- Repository/MCP/database keys, document references, session compatibility
  keys, and hash inputs remain native identity.
- `EventMeta.cwd` remains native operational identity because dispatch
  rehydrates it as a `PathBuf` for condition-context capture and working
  directory behavior.
- Tracing-only path fields remain native under the plan's explicit tracing
  exception.

Every remaining production hit is classified above. There are zero unresolved
presentation decisions, and no identity, argv, environment, filesystem, hash,
session, or operational working-directory consumer is routed through portable
rendering.
