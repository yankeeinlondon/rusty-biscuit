# Field Source Matrix (Phase B, item 2)

> **Status:** CHECKPOINT B review artifact (2026-07-04). This matrix is the contract
> generator v1 consumes: every serializable `ProviderInfo` field, plus every spec
> table-A field not yet in `ProviderInfo`, mapped to exactly one declared source per
> `design/catalog-generation.md`. Wrong source declarations are expensive later —
> review the **research** rows hardest.
>
> Evidence base: the serialized `ProviderInfo` array for all 7 compiled providers
> (`claudine providers --describe --format json`, re-captured 2026-07-04 after the
> Roo Code removal and canonical slug rename; 31 serialized fields per provider)
> joined against `docs/providers.yaml` and every research `_schema.yaml` sidecar.
> Research declarations were verified against what the sidecars carry **today** —
> a topic that exists but lacks a typed key is declared `facts` with a graduation note.
>
> **Amendment (2026-07-06, Phase F staged demotion):** the `static_models` row is
> retired — the field was removed from `ProviderInfo` (registry entry, coercion,
> and emitter deleted; `expected_offerings` is the validation baseline). The
> `model_catalog_source` row's vocabulary lost its `Static` member: `false` now
> coerces to `none`, and the codex/kimi overrides pin `none` (their listing
> surfaces remain future drift-channel candidates). Live registry state is 43
> serialized fields / 11 research rows; the counts and rows below are the
> Checkpoint B snapshot, kept as history.

## Summary counts

| Declared source | Current `ProviderInfo` fields | Table-A-only fields | Total |
| --- | ---: | ---: | ---: |
| roster | 10 | 0 | 10 |
| research | 5 | 5 | 10 |
| facts | 16 | 10 | 26 |
| **Total rows** | **31** | **15** | **46** |

Of the 26 facts rows, 16 have current constants and are scraped into
`docs/providers/facts/<slug>.yaml` (16 keys per provider file); the 10 table-A-only
rows have no current value and enter facts files as `TODO` markers at scaffold time
(onboarding step 4), never via the scraper.

Checkpoint B round-2 rulings (2026-07-04) amend these counts prospectively:
`supports_skills` graduates facts → research at v1 (its row), and the table-A
`config_files` research row is retired as a separate field (Open question 5) — so
the v1 contract is effectively research 10 (6 current + 4 table-A) / facts 25.
The table above is kept as the pre-ruling snapshot.

Excluded from the matrix (justified below): the non-serialized behavior half of
`ProviderInfo` (behavior trait objects / accessor surfaces absent from the describe
payload). `event_mapping` is deliberately **included** as data.

## Conventions

- **Field name** = the serialized `--describe` key = the facts/override key.
- **Expected shape** uses the A1 registry `SchemaExpectation` vocabulary
  (`String`, `Boolean`, `EnumSubsetOf`, `RecordArray`). Rows marked *(vocab gap)*
  need the vocabulary extensions listed in [Registry vocabulary extensions](#registry-vocabulary-extensions-needed-by-generator-v1).
- **Coercion** column: `string_literal` / `bool_literal` are the shared scalar
  coercions; everything else is a named-`Coercion`-variant candidate (Checkpoint A
  ruling 2). Coercions that can drop records must collect loud skips (ruling 4).
- **Canonical slug** = `ProviderInfo.slug` = the `Provider` slug: `claude`, `codex`,
  `gemini`, `goose`, `kimi`, `opencode`, `qwen` (plus roster-only `pi`, `kilo`).
  Facts files are named by this slug (matching A1's `claude.yaml`): `kimi.yaml` /
  `opencode.yaml` / `qwen.yaml`. Note the slug is distinct from the Rust `Provider`
  variants (`KimiCode`, `OpenCode`, `QwenCode`) and their serde snake_case forms
  (`kimi_code`, `open_code`, `qwen_code`), which are unchanged.
- **Research doc filename == slug** (Open question 4, RESOLVED). The 2026-07-04
  canonical-slug rename made the roster `file:` stem equal the slug fleet-wide
  (`kimi.md`, `opencode.md`, `qwen.md`), so A1's join on
  `docs/research/<topic>/<slug>.md` is now trivially correct. The roster `file:`
  key remains and must equal `<slug>.md`.

## Roster rows (identity facts, `docs/providers.yaml`)

All keys below now exist on every roster entry (Pi and Kilo carry the subset that is
known; see [Roster gaps](#roster-gaps)). Values equal the current describe constants
for the 7 compiled providers — including a constant that looks stale (see
[Drift findings](#drift-findings)); drift is recorded, not silently corrected.

| Field | Rust shape | Roster key | Expected shape | Coercion | Notes |
| --- | --- | --- | --- | --- | --- |
| `provider` | `Provider` | `slug` | String | `ProviderVariantFromSlug` (named) | The enum variant must pre-exist (onboarding step 3 hand wiring); generation references `Provider::<Variant>`, it can never create one. |
| `display_name` | `&'static str` | `display_name` | String | string_literal | Distinct from roster `name` ("Claude" vs "Claude Code") — `name` stays the sequence-template heading, `display_name` is the catalog field. |
| `slug` | `&'static str` | `slug` | String | string_literal | A1 entry, unchanged. |
| `short_name` | `&'static str` | `short_name` | String | string_literal | |
| `binary` | `&'static str` | `binary` | String | string_literal | A1 entry, unchanged. Version-scoped identity (kimi → kimi-cli history) is a future concern, noted in spec. |
| `agent_offset` | `&'static str` | `repo_dir` | String | string_literal | A1 entry, unchanged. `repo_dir` was missing for goose/kimi/qwen; now added (`.goose`, `.kimi`, `.qwen`). |
| `cli_aliases` | `&'static [&'static str]` | `cli_aliases` | StringArray *(vocab gap)* | `string_slice` (named, shared list emitter) | |
| `docs_url` | `&'static str` | `docs_url` | String / Url *(vocab gap)* | string_literal | Distinct from roster `site`; both kept. agent-cli topic URLs are verification inputs only (design rule 3), never merged. |
| `usage_dashboard_url` | `Option<&'static str>` | `usage_dashboard_url` (optional) | String / Url *(vocab gap)* | `optional_string_literal` (named) | Absent roster key ⇒ `None` (goose, opencode). The usage topic sidecar has no dashboard-URL key today, so this stays roster (URLs are roster-owned by design anyway). |
| `sniff_binding` | `&'static str` (sniff program id) | `sniff_binding` | String | string_literal | Values: `Claude`, `Codex`, `GeminiCli`, `Goose`, `KimiCli`, `Opencode`, `QwenCli`. |

### Roster gaps

- **Pi / Kilo** carry `slug`, `binary`, `display_name`, `short_name`, `cli_aliases`
  (single-alias lists — not invented beyond what is known). `docs_url`,
  `usage_dashboard_url`, and `sniff_binding` are omitted rather than guessed;
  onboarding fills them.
- **Roo Code is fully removed** (2026-07-04): enum variant, roster entry, research
  documents, and facts file are all gone (Open questions 1 and 2, RESOLVED). For
  future deprecations the roster now supports a `skip_research: true` entry flag
  (documented in the `docs/providers.yaml` header): identity stays in the roster
  but the entry is excluded from fleet fan-out, and `claudine-gen` fails loudly if
  asked to generate a skipped provider. No current entry sets it.

## Research rows (topic sidecar carries the fact today)

Every row below was verified against the actual `_schema.yaml` sidecar; frontmatter
paths are quoted exactly. (The former Roo coverage caveat is moot: Roo Code was
fully removed 2026-07-04, so no compiled provider lacks research coverage on these
rows — see Open questions 1 and 2, RESOLVED.)

| Field | Rust shape | Declared source | Expected shape | Coercion | Notes |
| --- | --- | --- | --- | --- | --- |
| `static_models` | `&'static [&'static str]` | `research:agent-models` `default_models` | RecordArray (requires `id`) | `DefaultModelsToStaticModels` (named): extract `id`, dedup, sort lexically to match hand-written ordering | **Large value drift**: compiled lists are stale hand copies from unchained-ai (codex still lists `gpt-3.5-turbo`/`o3`; research says `gpt-5.5`/`gpt-5.4`…; claude research says `claude-fable-5`/`claude-opus-4-8`…). Graduation regenerates against research — this diff is the point, reviewed at generate time. `catalog_id` joins arrive with Phase F. |
| `model_catalog_source` *(renamed from `dynamic_source`, 2026-07-05 ruling)* | `ModelCatalogSource` — reshaped mechanism-only: `None` / `Static` / `ShellCommand { program, args }` (provider-specific `OpencodeCli`/`OpencodeCliQwenFiltered` variants deleted) | `research:agent-models` `dynamic_listing.available` | Boolean, or EnumSubsetOf(ModelCatalogSource) | `DynamicListingToModelCatalogSource` (A1; output vocabulary reshaped) | Catalog/override shape mirrors serde: bare member string for unit variants, externally tagged `{shell_command: {program, args}}` object for the data variant. `false` coerces to `static` (now correct for qwen — research-fed `static_models`; its override is deleted). `true` still cannot select a mechanism: opencode pins the ShellCommand object; codex pins `static` (`codex debug models` is the future ShellCommand candidate, pending verification); kimi pins `static` (listing surfaces are HTTP/ACP, not shell commands — research reports `true`, so the pin cannot be deleted). |
| `model_env_vars` | `&'static [&'static str]` | `research:agent-models` `model_selection` | RecordArray (requires `method`, `site`) | `EnvVarSitesToStringSlice` (A1, skip-loudly) | A1 entry, unchanged. One-env-var-per-record fleet-prompt mandate already ratified (Checkpoint A ruling 4). |
| `session_log_paths` | `&'static [PathTemplate]` (serialized `{raw, segments}`) | `research:agent-logging` `surfaces` | RecordArray (requires `role`, `path_macos`, `format`) | `SurfacesToSessionLogPaths` (named): filter `role == session_transcript`, take `path_macos` | Placeholder grammar **RULED (Ken, 2026-07-04, Checkpoint D question 6): `{snake_case}`** (`{sanitized_cwd}`/`{session_id}`) is the canonical catalog grammar. Audit finding: the whole committed catalog (facts values, all 7 `data.rs`, overrides) is already conformant — v1 adopted the research grammar; the legacy `<…>` form survives only in two historical doc-comments in `path_template.rs`. No data migration needed. Per-OS paths exist in research but the catalog field is single-list today. |
| `config_paths` | `&'static [PathTemplate]` | `research:agent-cli` `config_paths` (frontmatter key renamed from `config_files`, 2026-07-04 — Open question 5 ruling) | RecordArray (requires `os`, `scope`, `path`) | `ConfigPathRecordsToConfigPaths` (named): filter `os == macos` (user+repo scopes), take `path`, document order | The research records are per-OS; the current catalog field collapses to one list. Per the Open question 5 ruling, `ConfigFileSpec` is this field's eventual richer TYPE (upgrade deferred until the agent-cli schema carries `format` on every record; graduation note, not a v1 change); until then this is a lossy-but-honest projection. |

### Table-A research rows (field not yet in `ProviderInfo`)

> **Phase D wave-1 stamp (2026-07-04):** `model_cli_flag`, `resume` (as
> `ResumeSupport`, support level only per the resume-parity ruling), and
> `non_interactive_conflicting_flags` are now live `ProviderInfo` fields;
> `sandbox` is **DEFERRED (Ken, 2026-07-04, Checkpoint D question 5)**: no
> catalog field until the permissions six-axis work (schema v2) gives it a real
> consumer — the `apply_sandbox` overrides are behavior and stay regardless.
> The research records remain the design input when the consumer arrives.

| Field *(table A)* | Rust shape (proposed) | Declared source | Expected shape | Coercion | Notes |
| --- | --- | --- | --- | --- | --- |
| `config_files` — **RETIRED as a separate field (2026-07-04, Open question 5 RESOLVED)** | — (`ConfigFileSpec` becomes the eventual richer TYPE of `config_paths`, above) | `research:model-config` `model_config_paths` (frontmatter key renamed from `config_files`, 2026-07-04) feeds a future `model_config_paths` catalog field when needed | — | — | The two research populations are genuinely different and stay two fields: agent-cli's general config-file inventory feeds the existing `config_paths`; model-config's model-extension file list is a distinct population, covered by a future `model_config_paths` catalog field (not a v1 field). The spec's `config_format` retirement stands — per-entry format arrives with the `ConfigFileSpec` type upgrade to `config_paths`. |
| `model_cli_flag` | `Option<&'static str>` | `research:agent-models` `model_selection` | RecordArray (requires `method`, `site`) | `CliFlagSitesToFlag` (named, skip-loudly): records with `method == cli_flag`, first bare `--flag` token site; annotated sites skipped loudly | Same source array as `model_env_vars`, different filter — two registry rows sharing one frontmatter path is already legal (paths are per-row). |
| `sandbox` | `SandboxSupport` (future catalog-types enum) | `research:agent-permissions` `sandbox` | RecordArray-shaped object (requires `supported`) *(vocab gap: Record)* | `SandboxRecordToSandboxSupport` (named) | Sidecar carries `{supported, modes[], backends[], filesystem_control, network_control}` today. The enum must be designed backwards from the fleet's `modes`/`backends` vocabularies before graduation. |
| `resume` | `ResumeSpec` (handle capture + follow-up injection, per spec's resume row) | `research:resume` `support` + `session_id_capture` + `resume_invocations` | EnumSubsetOf(ResumeSupport) + two RecordArrays | `ResumeRecordsToResumeSpec` (named, multi-path) | Multi-path extraction (one field, three frontmatter paths) needs registry support for multiple paths per entry — today `DeclaredSource::Research` holds one path (Open question 6). |
| `non_interactive_conflicting_flags` | `&'static [&'static str]` | `research:non-interactive-sessions` `claudine_strategy.conflicting_flags` | StringArray *(vocab gap)* | `FlagListToStringSlice` (named, skip-loudly): keep bare `--flag` tokens, loudly skip annotated entries | Observed data contains annotated entries ("--output-format json for live wrapping") — the same compound-record problem as env-var sites; the NIS fleet prompt should gain a bare-flag-per-record mandate at the next refresh (mirror of Checkpoint A ruling 4). |

## Facts rows (topic-less today, `docs/providers/facts/<slug>.yaml`)

The 16 rows with current constants are scraped verbatim for the 6 non-claude
providers (codex, gemini, goose, kimi, opencode, qwen; 16 keys per file).
**`claude.yaml` is deliberately NOT expanded**: the A1 collision gate
(`UnknownFactsKey`) rejects any key without a facts-declared registry entry, and
the registry stays at its A1 six entries until generator v1 — so `claude.yaml`
keeps only `supports_skills` for now. The 6 new files are consumed by no gate yet
and carry the full set.

Scrape determinism note: nested object keys are emitted in lexical order
(serde_json without `preserve_order`); top-level keys follow constant field order via
`MATRIX_FACTS_FIELDS`. Stable either way — v1's record emitters define final shape.

| Field | Rust shape (serialized view) | Expected shape | Coercion sketch | Graduation |
| --- | --- | --- | --- | --- |
| `supports_skills` | `bool` | Boolean | bool_literal (A1) | **RULED (2026-07-04, Open question 3 RESOLVED): research-fed at v1** via the `skills` topic `support` enum→bool mapping — `first_class` → `true`, `partial` → `true`, `convention_only` → `false`, `none` → `false`, `unknown` → `false`. The goose/kimi `false` constants are ruled stale (drift finding 3); v1's diff-reviewed regeneration adopts the research values (all 9 documented providers say `first_class` today). This row stays in the facts table only as the pre-graduation snapshot. |
| `stream_protocol` | `Option<&'static str>` (`stream-json`/`jsonl`/`ndjson`/`wire-json-rpc`/None) | String (nullable) *(vocab gap: Optional)* | `optional_string_literal`; enum candidate | → `non-interactive-sessions` (`io_contract.framing` / `data_format`). **Vocabulary RULED (Ken, 2026-07-04, Checkpoint D question 7): option (b)** — normalize to the framing vocabulary (`ndjson`/`jsonl`/`json-rpc`) and graduate from NIS; the provider-native mode label (`stream-json`) already lives in `output_formats` Stream records, so nothing is lost. Execute AT the NIS-graduation moment (StreamProtocol variant rename is a shape change: emit.rs + regen + consumer audit in one change), not before. |
| `event_mapping` | `EventMapping` (16 canonical events × support level/aliases/registration) | RecordArray (requires `event`, `support_level`) | `EventMappingRecords` (named, record constructor) | Partially → `hooks` topic (`hooks[]` carries native_event → claudine_event + blocking), but the `support_level` *kind* (hook vs stream_parse vs wrapper vs wire_proxy vs acp) is a Claudine integration decision joined from hooks + NIS + acp topics — likely a compound-join field or long-term facts. **Included as data, not excluded**: it passes the OQ5 litmus (pure selection among enumerable support strategies plus string parameters; the runtime control flow lives in the parsers/hook engines, not in this table). |
| `resource_support` | `ResourceSupport` (skills/commands/agents/scripts specs + skill frontmatter matrix) | RecordArray-shaped object *(vocab gap: Record)* | `ResourceSupportRecord` (named) | → `skills` + `slash-commands` + `subagents` topics (locations/format/portability all carried today), but the shape is a 4-way compound and the ratified portability enum (5 classes) postdates this struct — graduate per sub-record during Phase D linking work, not as one jump. |
| `session_locations` — **RETIRED (Ken, 2026-07-05)** | — | — | — | Field deleted entirely (`ProviderInfo` field, registry entry, emit code, facts keys, serialization-test row; `catalog.json` loses the key). It had NO runtime consumer, and the semantic ruling it was waiting on (Open question 5 follow-up) resolved to "the field means nothing coherent": claude's value was a directory, codex's mixed an app log with shell snapshots. |
| `memory_files` | `&'static [PathTemplate]` | RecordArray | `PathTemplateList` (named) | → `system-prompt` `config_sources` (scope/path/mode carried today) once an extraction rule (mode==append ∧ format==markdown?) is ratified. Also duplicated inside `system_prompt.memory_files` — the generated shape should reference one list (v1 cleanup candidate). |
| `output_formats` | `&'static [OutputFormatSpec]` (format/native_name/cli_flag/stdin/selector) | RecordArray | `OutputFormatRecords` (named) | → `non-interactive-sessions` `output_formats[]` (name/cli_value/stream/format carried today), but `selector.kind` (flag_value vs flag vs transport_flag vs default) and `stdin_supported` need schema additions — topic carries the fact family, not the full shape. |
| `entrypoints` | `&'static [EntrypointSpec]` (subcommand/required_flags/mode) | RecordArray | `EntrypointRecords` (named) | → `non-interactive-sessions` `invocation[]` (command/stdin/prompt_arg carried today); the subcommand-vs-required-flags split is not expressible in the current records. |
| `system_prompt` | `SystemPromptSpec` (append/replace × interactive/non-interactive delivery + memory files) | Record *(vocab gap)* | `SystemPromptSpecRecord` (named) | → `system-prompt` topic: `claudine_delivery` carries the strategy *kinds* today (enum aligns 1:1 with the constant's variants) but not the flag/key/env-var *sites*, and has no interactive/non-interactive split — needs schema v2 (site fields) before graduation. |
| `yolo` | `YoloSupport` (direct_flag / env_var / non_interactive_only / none) | Record *(vocab gap)* | `YoloRecordToYoloSupport` (named) | → `agent-permissions` `yolo` — booleans carried today, but `mechanism` is prose ("--permission-mode bypassPermissions or --dangerously-skip-permissions; …"), not a typed flag site; the planned permissions schema v2 (typed YOLO switches) is the graduation trigger. |
| `reasoning` | `ReasoningSupport` (named_levels / numeric_budget / binary_toggle / provider_specific / not_documented) | Record *(vocab gap)* | `ReasoningRecord` (named) | No topic carries reasoning/thinking control today (checked: agent-models, model-config, agent-cli sidecars). Long-term facts, or a future agent-models v2 key. |
| `known_gaps` | `&'static [KnownGap]` (area/note/tracker) | RecordArray | `KnownGapRecords` (named) | Human-curated by nature — closer to overrides than research. Candidate to stay facts permanently, with the generate report's missing-topic listing absorbing part of its job. |
| `acp` | `AcpSupport` (server_mode/client_supported/events_via_acp) | EnumSubsetOf(AcpServerMode) at `acp` `support` + facts record | `AcpRecord` (mixed-source: research `support` → `server_mode`; facts record → `client_supported`/`events_via_acp`) | **RULED / server_mode GRADUATED (Ken, 2026-07-05):** `server_mode` describes PROVIDER ACP capability (observational; Claudine does not use ACP today), research-fed from the `acp` topic using the sidecar `support` vocabulary verbatim (`native/adapter/partial/none/unknown` — new `AcpServerMode` in catalog-types replaces the old `NotSupported/Native/AvailableViaWireProxy`). Kimi's `available_via_wire_proxy` was a Claudine integration detail, retired (kimi has native `kimi acp`). Values: claude/codex `adapter`, the other five `native`. `client_supported`/`events_via_acp` STAY facts-fed; the registry entry declares the research source and the coercion merges the facts sub-record (facts `server_mode` key is delete-on-graduate enforced). `AcpSupport::is_supported()` now means "provider speaks ACP", decoupled from `EventSupportLevel::Acp` wiring. |
| `prompt_arg_conventions` | `PromptArgConventions` (prompt_flags/entrypoint/value_taking_flags) | Record *(vocab gap)* | `PromptArgRecord` (named) | `prompt_flags`/`entrypoint` → NIS `invocation[]` eventually. **Ruled (OQ7a, 2026-07-04):** `value_taking_flags` is hoisted to shared code before v1 generation — the record shape shrinks to `prompt_flags`/`entrypoint`. |
| `cli_sensitive_axes` | `CliSensitiveAxes` (10 booleans over the PolicyEngine axis taxonomy) | Record *(vocab gap)* | `AxesRecord` (named) | → `agent-permissions` — the axes are Claudine's taxonomy; the sidecar carries adjacent facts (`permission_entities`, `cli_params`) but not the axis booleans. The summary's six-axis classification is the graduation vehicle (schema v2). |
| `repo_home_root_files` | `&'static [&'static str]` | StringArray *(vocab gap)* | `string_slice` | Shadow-home sync knowledge (claude: `.claude.json`; empty elsewhere). agent-cli `config_paths` records the same path with `scope: user`, but "lives at $HOME root AND must be shadow-synced" is a Claudine operational fact — likely permanent facts. |
| `unmapped_native_events` *(NEW, ruled [I] by Ken, 2026-07-05)* | `&'static [UnmappedNativeEvent]` (native_event/description/remediation) | RecordArray (requires `native_event`, `description`, `remediation`) | `UnmappedNativeEventRecords` (named) | Provider-native hook events firing at phases the 16-event model cannot represent; users must configure them directly in the provider, and CLI reports state so. Values: gemini `BeforeToolSelection`, opencode `tool.definition`; empty elsewhere. Graduation candidate: research-fed from the `hooks` topic (`hooks[]` records where `claudine_event: unknown`) once that extraction rule is ratified. |

### Table-A facts rows (no current constant — no scrape; `TODO` at scaffold time)

> **Phase D wave-1 stamp (2026-07-04):** `billing_models` (legacy values
> recovered from `edd22f733^`), `allowed_env_keys`, `stdout_noise_prefixes`,
> `stderr_noise_prefixes`, `suppress_structured_stderr_on_success`,
> `supports_interactive_inline_closure`, and `model_required_in_non_tty` are
> now live facts-fed `ProviderInfo` fields. `platform_kind` values were
> **ratified (Ken, 2026-07-04, Checkpoint D question 4)** — claude/codex/
> gemini/kimi/qwen `vendor_platform`, goose/opencode `agent_aggregator`
> (gemini/qwen/kimi's aggregator traits are escape hatches, not primary UX) —
> and the field landed facts-fed the same day. Still table-A-only:
> `prompt_delivery` (strategy enum design pending).

| Field *(table A)* | Rust shape (proposed) | Coercion sketch | Graduation |
| --- | --- | --- | --- |
| `billing_models` | `&'static [BillingModel]` | named record emitter | **RATIFIED as facts (2026-07-04)** — kept as a facts field (useful even without a consumer today). Eventual → `usage` topic remains possible — sidecar has `api`/`metrics`/`limit_states` but no billing-model key today; needs a schema addition. |
| `stdout_noise_prefixes` | `&'static [&'static str]` (DisplayPolicy field) | `string_slice` | → NIS `io_contract.noise_handling` is prose today; needs a typed key. Lands inside the generated `DisplayPolicy` section (Phase G owns placement). |
| `stderr_noise_prefixes` | `&'static [&'static str]` (DisplayPolicy field) | `string_slice` | Same as above. |
| `supports_interactive_inline_closure` | `bool` | bool_literal | No topic key; candidate agent-cli/NIS schema addition. |
| `prompt_delivery` | `PromptDeliverySpec` selection enum (OQ5 ruling: selection is data) | named enum coercion | → NIS `invocation[]` (stdin_support/prompt_arg carried today) once the strategy enum is designed; implementations stay behavior-half. |
| `structured_stream_flag` | `Option<&'static str>` | `optional_string_literal` | **RETIRED (OQ7b, 2026-07-04):** derived from `output_formats` (Stream record's flag/selector), never a catalog field. |
| `allowed_env_keys` | `&'static [&'static str]` | `string_slice` | Multi-topic aggregation of `env_vars[]` (permissions/model-config/mcp/logging/system-prompt/agent-cli, per the spec's no-standalone-env-topic decision) — the registry cannot express a multi-topic source today (Open question 6). |
| `suppress_structured_stderr_on_success` | `bool` | bool_literal | → NIS `io_contract.stderr` (diagnostics_only vs structured_events carried today; the *suppress-on-success* policy is a Claudine decision derived from it). |
| `model_required_in_non_tty` | `bool` | bool_literal | No typed topic key (NIS `claudine_strategy` prose only); facts until a NIS schema addition. |
| `platform_kind` | `PlatformKind` (`VendorPlatform` / `AgentAggregator`) | named enum coercion | New classification (spec 2026-07-02); no topic — human-owned facts, plausibly permanent. |

### Table-A field details

One block per field above, for Checkpoint B trade-off review: what the field means,
where the knowledge lives in code **today** (these are pre-catalog fields — the
current embodiment is hardcoded `WrapperProfile` overrides or provider literals),
example values sourced from research/constants, the sourcing trade-off, and a
recommendation. Trade-off options referenced below: **(i)** hand-maintained facts,
**(ii)** typed key added to an existing topic schema (self-updating after one fleet
refresh), **(iii)** derive from another catalog field, **(iv)** drop from table A.

#### `billing_models`

- **What it is.** The commercial models under which a provider's usage is paid for:
  subscription plan, per-token API billing, or provider-only (the CLI is free and all
  cost is upstream API usage). Feeds usage/cost reporting UX and "which dashboard and
  limit semantics apply" logic.
- **Who consumes it.** Only the legacy tree: `BillingCapabilities` / `BillingModel`
  (`lib/src/agents/model.rs:129`/`:135`), populated per provider in
  `lib/src/provider/<slug>/legacy.rs` (claude `:132`, codex `:132`, gemini `:109`,
  kimi `:99`, qwen `:119`). **No live output consumer**: `cli/src/commands/providers.rs:299`/`:318`
  deliberately keeps the `AgentCapabilities` facade out of structured describe JSON,
  and Phase C item 3 deletes every `legacy.rs`. This field is the landing spot that
  keeps the data alive through that deletion.
- **Example values.** claude `[subscription, per_token]` (Max/Pro plans + API-key
  billing; usage research distinguishes the two acquisition paths); codex
  `[subscription, per_token]` (ChatGPT plans; API keys billed per token); goose
  `[provider_only]` ("Goose itself is free; all cost is provider API usage").
- **Trade-offs.** (i) is near-zero churn (billing models change ~yearly); (ii) needs
  a `usage` schema addition (sidecar has `api`/`metrics`/`limit_states`, no
  billing-model key) and a fleet refresh; (iii) not derivable; (iv) loses data that
  Phase C deletion would otherwise orphan.
- **Recommendation.** **(i) facts** — churn is negligible and the Phase C `legacy.rs`
  deletion needs a home for this data before any usage-topic key could exist.
- **Ratified (2026-07-04, Checkpoint B round 2).** Ken confirmed: keep as a facts
  field — useful even without a consumer today.

#### `stdout_noise_prefixes`

- **What it is.** Literal line prefixes on child **stdout** that the wrapper filters
  before parsing/display — human chrome some CLIs print even in structured mode.
  Determines whether structured stdout parses clean.
- **Who consumes it.** `WrapperProfile::stdout_noise_prefixes()` (default empty,
  `cli/src/commands/wrap/profile/mod.rs:438`); sole override is Gemini
  (`profile/gemini.rs:92`). Threaded via `wrapper_stages.rs:329` and
  `composition/mod.rs:1185` into `WrapIo` (`exec/mod.rs:53`) and applied line-by-line
  in `exec/spawn.rs:104`/`:183`.
- **Example values.** gemini: `"Created execution plan for "`,
  `"Expanding hook command: "`, `"Skill conflict detected: "`, `"[LocalAgentExecutor]"`;
  goose: none — stdout purity comes from the `--quiet` flag instead (NIS research:
  without it a human session banner precedes JSON), showing flag-based purity and
  prefix filtering are alternative strategies for the same fact family.
- **Trade-offs.** Drifts with provider releases, so (ii) is attractive — but NIS
  `io_contract.noise_handling` is prose today; the typed key costs a fleet refresh.
  (iii) no donor field. (iv) would push the knowledge back into hardcode.
- **Recommendation.** **(i) facts now, (ii) later** — graduate when NIS gains a typed
  noise key at its next scheduled refresh; do not force a refresh for this alone.

#### `stderr_noise_prefixes`

- **What it is.** Same as above for child **stderr**: prefixes of known-noise lines
  suppressed from the user-visible stderr surface.
- **Who consumes it.** `WrapperProfile::stderr_noise_prefixes()` (default empty,
  `profile/mod.rs:475`); overrides: gemini (`gemini.rs:102`), codex (`codex.rs:112`),
  opencode (`opencode.rs:180` → `opencode_default_tui_noise_prefixes()` at `:189`).
  Applied in `exec/spawn.rs:105`/`:220`. **Not** the same mechanism as
  `SILENT_PROVIDER_EXTENSION_KINDS` (`cli/src/commands/wrap/live_semantic_sink/provider_extension.rs:136`),
  which suppresses *parsed structured event kinds* on the semantic sink — that table
  is a separate future catalog candidate, not covered by this field.
- **Example values.** codex `["Reading prompt from stdin..."]`; opencode: TUI
  formatter prefixes (`"✱ "`, `"$ "`, `"> build "`, `"████ "`, `"⚙ "`) that leak to
  stderr even with `--format json`; gemini `["Skill conflict detected: ", "[LocalAgentExecutor]"]`.
- **Trade-offs.** Same as stdout, plus a sharper hazard: OpenCode's stderr is
  simultaneously noise-filtered AND a promoted lifecycle-evidence channel (NIS:
  `--print-logs` stderr carries model/permission/cap/auth signals) — an over-broad
  research-scraped list could silently eat wrapper-grade evidence.
- **Recommendation.** **(i) facts** — the OpenCode dual-role makes this a curated
  suppression list, reviewed by a human, with NIS as input rather than source.

#### `supports_interactive_inline_closure`

- **What it is.** Whether Claudine can recover a final assistant body **after an
  interactive session ends**, enabling `inline-compose` closure for interactive runs
  (non-interactive closure works everywhere via the structured stream).
- **Who consumes it.** `WrapperProfile::supports_interactive_inline_closure()`
  (default `false`, `profile/mod.rs:561`); sole `true` override is Codex
  (`codex.rs:116`, backed by its captured-output path). Gate at
  `composition/mod.rs:692`: interactive + inline without support is rejected up front.
- **Example values.** codex `true`; claude/gemini/goose/kimi/opencode/qwen `false`.
- **Trade-offs.** This is a *Claudine-capability* fact — "our closure machinery can
  do X given this provider's surfaces" — not an observable provider property, so no
  research topic can honestly carry it (ii fails the ownership test). (iii) it is
  loosely implied by Codex's last-message capture mechanism but the implication is a
  Claudine engineering judgment, not a derivation. (iv) the gate is live code.
- **Recommendation.** **(i) facts, plausibly permanent** — same class as
  `known_gaps`: Claudine-owned operational truth.

#### `prompt_delivery`

- **What it is.** *Which strategy* carries the prompt body to the child: stdin,
  appended argv, argv inserted after a subcommand, or a wire JSON-RPC `prompt`
  request. Per the OQ5 ruling the strategy *selection* is data; the mechanics stay
  in the behavior half.
- **Who consumes it.** The only **required** (no-default) `WrapperProfile` method
  (`profile/mod.rs:492`), implemented by all seven providers: claude `claude.rs:57`
  (stdin when non-interactive), codex `codex.rs:57`, gemini `gemini.rs:135`, goose
  `goose.rs:56` (`InsertArgs` — `-t <prompt>` positioned after the `run`
  subcommand), kimi `kimi.rs:59` (`WireRpc` non-interactive, `--prompt` argv
  interactive), opencode `opencode.rs:105`, qwen `qwen.rs:88`. The `PromptDelivery`
  enum (`profile/mod.rs:49`) is applied by the wrapper pipeline via `apply_to`.
- **Example values.** claude `stdin` (non-interactive `-p` mode); goose
  `subcommand_insert` (`run -t "<prompt>"`, or `-i -` stdin form per NIS research);
  kimi `wire_rpc` (JSON-RPC `prompt` request after `initialize`).
- **Trade-offs.** Spec already flags this "highest effort; may stay behavior". Kimi
  shows delivery entangled with transport (the wire session orchestrator), goose
  shows position-dependence — a bare enum undersells both. But the *selection* enum
  is exactly what NIS `invocation[]` (stdin support + prompt arg) can grow to carry.
- **Recommendation.** **(ii) eventually — NIS schema addition** once the strategy
  enum is designed; catalog selects the strategy, profiles keep the mechanics.
  Facts-with-TODO until that design exists (as scaffolded).

#### `structured_stream_flag`

- **What it is.** The argv addition that switches a provider into machine-readable
  stream output for wrapped runs.
- **Who consumes it.** `WrapperProfile::apply_structured_stream()` (default no-op,
  `profile/mod.rs:557`), gated by `supports_structured_stream()` (`:516`) — which
  **already derives from the catalog** (`provider_info(..).stream_protocol.is_some()`),
  the existing precedent for option (iii). Impls: claude `claude.rs:79`
  (`--print --verbose` + `--output-format stream-json`), codex `codex.rs:104`
  (`--json`, assumes `exec` entrypoint present), gemini `gemini.rs:149`, qwen
  `qwen.rs:110`, kimi `kimi.rs:89` (wire mode). Call sites `wrapper_stages.rs:339`/`:359`.
- **Example values.** claude `--output-format stream-json` (with `--print --verbose`
  companions); codex `--json` (under `exec`); qwen `--output-format stream-json`
  (NIS also documents `--include-partial-messages` enrichment).
- **Trade-offs.** A single `Option<&'static str>` under-models reality: claude needs
  companion flags, codex's flag is entrypoint-relative. `output_formats` already
  carries per-format `cli_flag` + `selector` records including the `stream: true`
  row — this field is that record's projection.
- **Recommendation.** **(iii)/(iv) — derive from `output_formats`' `stream: true`
  record and drop the standalone field** (fold into Open question 7); keeping both
  invites the two copies to disagree.

#### `allowed_env_keys`

- **What it is.** Provider-required env var names the wrapper's sensitive-key env
  sanitizer must let through to the child (the sanitizer strips secret-shaped keys
  by default).
- **Who consumes it.** `WrapperProfile::allowed_env_keys()` (default empty,
  `profile/mod.rs:505`); overrides: codex `codex.rs:91`, gemini `gemini.rs:61`, kimi
  `kimi.rs:16`, qwen `qwen.rs:60`. Consumed by the env-plan build at
  `cli/src/commands/wrap/env/mod.rs:229` feeding `sanitize_process_env`
  (`env/sanitize.rs:36`).
- **Example values.** codex `[OPENAI_API_KEY, CODEX_API_KEY]`; gemini
  `[GEMINI_API_KEY, GOOGLE_API_KEY]`; qwen `[DASHSCOPE_API_KEY, QWEN_API_KEY]`.
- **Trade-offs.** (ii) is what the spec sketches — aggregate credential-classed
  `env_vars[]` across six topics — but the registry cannot express multi-topic
  sources (Open question 6), **and** this is a security allowlist: auto-widening it
  from scraped research would silently expand which secrets reach child processes.
  (iii) no single donor field.
- **Recommendation.** **(i) facts, deliberately hand-ruled** — research env
  inventories serve as review input (a generate-report "candidate keys not in
  allowlist" listing would be ideal), never as the automatic source.

#### `suppress_structured_stderr_on_success`

- **What it is.** When a structured-stream run **succeeds**, drop the child's
  captured stderr instead of replaying it — for providers whose stderr on success is
  pure noise.
- **Who consumes it.** `WrapperProfile::suppress_structured_stderr_on_success()`
  (default `false`, `profile/mod.rs:481`); sole `true` override is Gemini
  (`gemini.rs:106`). Threaded through `wrapper_exec.rs:138`, `wrapper_stages.rs:532`,
  and `composition/mod.rs:1707` into `exec/spawn.rs:712`.
- **Example values.** gemini `true`; opencode must stay `false` — its stderr is a
  promoted lifecycle-evidence channel ("stderr is no longer ignorable", NIS opencode
  research); claude/codex `false`.
- **Trade-offs.** NIS `io_contract.stderr` carries the `diagnostics_only` vs
  `structured_events` classification, so (iii) tempts — but a mechanical
  `diagnostics_only → true` mapping would flip claude/codex/goose to suppression, a
  behavior change nobody ruled. The *policy* is Claudine's; the classification is
  merely its strongest input.
- **Recommendation.** **(i) facts** — Claudine-owned display policy, with the NIS
  stderr classification cited as review evidence at generate time.

#### `model_required_in_non_tty`

- **What it is.** The provider has no usable default model in headless runs; the
  wrapper must resolve one (flag/env/config) before spawn or fail with remediation
  guidance instead of letting the child die opaquely.
- **Who consumes it.** OpenCode only, and it is the matrix's clearest hardcoded
  provider check: `composition/mod.rs:970` gates on `provider == Provider::OpenCode`
  before calling `apply_opencode_model_resolution`
  (`cli/src/commands/wrap/profile/resolve.rs:114`), whose no-model error
  (`resolve.rs:149`) enumerates the remediation set (`OPENCODE_MODEL`, `--model`,
  config `model` key). A catalog bool is precisely what replaces that `==` literal
  when Phase C extracts the shared prep stage.
- **Example values.** opencode `true` (resolution order: CLI switch → `OPENCODE_MODEL`
  → config default); claude/codex/gemini `false` (server-side defaults exist).
- **Trade-offs.** (ii) blocked today — NIS carries this only as `claudine_strategy`
  prose; typed key = schema addition + refresh. (iii) not derivable. (iv) the gate is
  live, load-bearing code.
- **Recommendation.** **(i) facts now** — it directly de-hardcodes the Phase C shared
  prep stage; graduate to a typed NIS key opportunistically at the next refresh.

#### `platform_kind`

- **What it is.** Classifies the CLI as a `VendorPlatform` (predominantly the
  vendor's own models — Claude Code, Codex) or an `AgentAggregator` (model-agnostic
  front-end — OpenCode, Goose, and roster-only Pi). Spec (2026-07-02) intends it to
  predict model-selection UX centrality and API-shim flexibility.
- **Who consumes it.** **Nothing today** — no code embodies this classification
  anywhere in lib or cli. First plausible consumers: Phase F model-catalog joins
  (aggregators need provider/model pair selection UX; vendor platforms need
  rolling-alias handling) and provider-recommendation/reporting surfaces.
- **Example values.** claude `vendor_platform`, codex `vendor_platform`, opencode
  `agent_aggregator`, goose `agent_aggregator` (spec's own examples; the remaining
  providers need Ken's call — e.g. gemini/qwen/kimi are vendor CLIs with aggregator
  traits).
- **Trade-offs.** Zero maintenance either way; the honest question is (iv): with no
  consumer, the field is speculative and its value set is a judgment call, not an
  observable. But it is spec-ratified, one enum per provider, and cheap.
- **Recommendation.** **(i) facts** — keep, human-owned and plausibly permanent, but
  flagged: if Phase F lands without consuming it, revisit (iv).

## Exclusions

Excluded from the matrix — each with the reason:

1. **The behavior half of `ProviderInfo`** — the four behavior trait objects and any
   fn-pointer/accessor surfaces (spec.md counts 36 struct fields; the describe
   serialization carries 31). They are absent from the serialized payload by
   construction, fail the OQ5 litmus (they ARE the runtime control flow), and belong
   to `behavior.rs` permanently. The exact field names live in the mid-split
   `lib/src/provider/` tree (off-limits to this pass by coordination constraint);
   generator v1's registry-covers-all-fields test must enumerate them explicitly
   against the post-split `data.rs` field list.
2. **`resource_support.provider`** (nested duplicate of the top-level `provider`
   discriminator) — not a top-level field; the record emitter reproduces it from the
   `provider` row.
3. **Nothing else.** `event_mapping` was the candidate the task flagged, and it is
   **included** as facts-declared data — justification in its row: it is a pure
   selection table (canonical event → support-level strategy + string names), with
   all sequencing/conditional logic living in the parsers and hook engines that
   consume it.

## Drift findings

Recorded, not corrected (roster/facts values mirror the describe constants exactly):

1. **`docs_url` staleness (1):** `opencode` → `https://github.com/opencode-ai/opencode`
   (roster `repo:` already says `anomalyco/opencode`). The constant looks like a
   pre-rename URL. Fix belongs in generator v1's first regeneration (roster
   `docs_url` update + diff review), not in this pass.
2. **`static_models` staleness (all providers with non-empty lists):** codex's
   compiled list (`gpt-3.5-turbo`, `o3`, …) vs research (`gpt-5.5`, `gpt-5.4`, …);
   claude's compiled list tops out at `claude-opus-4-5` vs research
   (`claude-fable-5`, `claude-opus-4-8`, `claude-sonnet-5`, …). Expected — the spec
   itself says these are hand-derived from a stale unchained-ai catalog.
3. **`supports_skills` contradiction (goose, kimi) — RESOLVED (2026-07-04):** Ken
   ruled the goose/kimi `false` constants stale; the recent skills research round is
   trusted. No ad-hoc constant fix now — v1's diff-reviewed regeneration adopts the
   research values via the ratified enum→bool mapping (Open question 3).
4. **`stream_protocol` vs NIS `data_format` vocabulary:** claude constant
   `stream-json` vs research `ndjson` — same fact, two vocabularies.
5. **`session_log_paths` placeholder grammar:** `<encoded-directory>` (constants) vs
   `{sanitized_cwd}` (research).
6. **`name` vs `display_name`, `site` vs `docs_url`:** not drift — deliberately
   distinct roster keys (heading/homepage vs catalog identity); called out so the
   duplication is not "fixed" by accident.

## Registry vocabulary extensions needed by generator v1

The A1 `SchemaExpectation` vocabulary (`String`, `Boolean`, `EnumSubsetOf`,
`RecordArray`) covers the scalar rows. The matrix needs, at minimum
(**approved as proposed 2026-07-04**, Open question 6):

- `StringArray` (cli_aliases, repo_home_root_files, conflicting flags),
- `Record` (single inline-object shapes: yolo, acp, system_prompt, …),
- optionality markers (`usage_dashboard_url`, `stream_protocol`),
- and `DeclaredSource::Research` support for **multiple frontmatter paths per field**
  (resume). Multi-**topic** sources are NOT added: `allowed_env_keys` stays facts
  per the Open question 6 ruling (2026-07-04).

## Open questions for Checkpoint B

1. **Roo roster entry — RESOLVED (2026-07-04):** Roo Code was fully removed instead
   (enum variant, roster entry, research documents, facts file). Neither uncommenting
   nor an entry-level skip was needed for Roo, but the roster gained a
   `skip_research: true` flag for future keep-identity-but-skip-fleets deprecations
   (identity stays; fleet fan-out and generation exclude the entry, generation
   failing loudly).
2. **Research-row failure posture for roo — RESOLVED (2026-07-04):** moot after the
   full Roo removal; no compiled provider lacks research coverage on the
   research-declared rows. Fail-loudly remains the general design for future
   coverage gaps.
3. **`supports_skills` graduation — RESOLVED (2026-07-04):** trust the recent skills
   research round; graduate via the `support` enum→bool mapping — `first_class` →
   `true`, `partial` → `true`, `convention_only` → `false`, `none` → `false`,
   `unknown` → `false`. The goose/kimi `false` constants are ruled stale; v1's
   diff-reviewed regeneration adopts the research values.
4. **Research-doc filename join — RESOLVED (2026-07-04):** the canonical slug rename
   (`kimi`, `opencode`, `qwen`) plus research-stem alignment made slug == file stem
   hold fleet-wide, so generator v1 resolves `docs/research/<topic>/<slug>.md`
   directly. The roster `file:` key remains but is now constrained to equal
   `<slug>.md`; no per-entry stem indirection is needed.
5. **`config_paths` vs `config_files` split — RESOLVED (2026-07-04):** Ken prefers
   `config_paths` over `config_files` for language consistency. The delegated
   structural decision, recorded verbatim:
   - The two research populations are genuinely different and STAY two fields:
     agent-cli's general config-file inventory feeds the existing catalog field
     `config_paths`; model-config's model-extension file list is a distinct
     population.
   - Language unification: rename the frontmatter key `config_files` →
     `config_paths` in the **agent-cli** topic, and `config_files` →
     `model_config_paths` in the **model-config** topic (distinct name because it
     is a distinct population; keeps the `-paths` language). (Executed 2026-07-04:
     both sidecars, all provider docs, and both `_fleet.md` prompts renamed.)
   - The table-A `config_files: &[ConfigFileSpec]` proposal is retired as a
     separate field: `ConfigFileSpec` becomes the eventual richer TYPE of
     `config_paths` (upgrade deferred until the agent-cli schema carries `format`;
     graduation note, not a v1 change), and a future `model_config_paths` catalog
     field covers the model-extension population when needed.
6. **Registry shape growth — RESOLVED (2026-07-04):** the vocabulary extensions
   above are approved as proposed (`StringArray`, `Record`, optionality markers,
   multi-path research sources for `resume`). `allowed_env_keys` stays facts rather
   than growing multi-topic machinery — matching its row's security-allowlist
   recommendation.
7. **Shared-constant fields — RESOLVED (2026-07-04):** approved as recommended.
   (a) `value_taking_flags` is hoisted to shared code: the extractor consumes
   `COMMON_VALUE_TAKING_FLAGS` directly and the per-provider
   `PromptArgConventions.value_taking_flags` field is removed (the union semantics
   are the deliberate design, per its doc comment; the latent flag-arity hazard —
   `-c` boolean for claude vs value-taking for codex — is documented, and
   per-provider precision returns as a facts field only if it ever bites).
   (b) `structured_stream_flag` is DERIVED from `output_formats` (the Stream-format
   record's flag/selector) and dropped from table A — precedent:
   `supports_structured_stream()` already derives from `stream_protocol`
   (`profile/mod.rs:516`). Second copies drift; a dedicated field returns only if a
   provider's stream selection stops being expressible in `output_formats`.
8. **`ModelCatalogSource` reshape + field rename — RESOLVED (2026-07-05, Ken):**
   the enum becomes mechanism-only — `None` / `Static` /
   `ShellCommand { program: &'static str, args: &'static [&'static str] }`;
   the provider-specific `OpencodeCli` / `OpencodeCliQwenFiltered` variants are
   deleted (no filter parameter — opencode is the only shell-command user:
   `ShellCommand { program: "opencode", args: &["models"] }`). The `ProviderInfo`
   field is renamed `dynamic_source` → `model_catalog_source` everywhere (struct,
   serialized describe key, override key, catalog.json). Qwen leaves shell
   sourcing entirely: the research coercion (`available: false` → `static`) is
   now correct because its `static_models` are research-fed, so its override is
   deleted. Codex keeps a `static` pin (`codex debug models` is the future
   ShellCommand candidate, pending verification); kimi keeps a `static` pin
   (research reports `available: true` via HTTP/ACP surfaces — not shell
   commands — so the pin cannot be deleted); opencode's pin becomes the
   ShellCommand object (the research boolean cannot express program/args).
   Generate-time unchained ruling (recorded, no code yet): model ground truth
   joins arrive with Phase F's committed unchained-ai artifact — the registry
   will re-point `static_models` there; the runtime enum stays mechanism-only.
