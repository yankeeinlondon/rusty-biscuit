# Spike: Logging Topic Schema (Phase 0)

> 2026-07-01. Goal: discover what structural information a fine-grained `agent-logging`
> topic schema must be able to express, using three lenses — the existing research
> frontmatter, the research prose, and **real on-host log data** for Claude Code and
> Codex — then draft and empirically validate a SimplifiedSchema.

## Lens 1 — existing research frontmatter: nearly empty

The 2026-04-29 research run predates the current sequence file's `target_schema`. Actual
captured frontmatter across all providers is three flat keys (`last_updated`,
`has_official_schema`, `schema_url`). Every structural fact lives in prose only.

## Lens 2 — research prose: rich but unqueryable

`claude-code.md` alone contains per-OS path tables (5 CLI surfaces + 6 desktop log files),
an explicit rotation statement ("no rotation; session boundary only"), format details, and
Rust schema sketches. The information is discoverable by research — it just had no schema
shape to land in. Staleness was irrelevant to this spike, as predicted.

## Lens 3 — real on-host logs: the evidence lens earns its keep

### Codex (`~/.codex/`)

- Session transcripts are **date-sharded**: `sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl`,
  plus `archived_sessions/`, a global `session_index.jsonl`, and `history.jsonl`.
- **Version-suffixed SQLite databases**: `logs_2.sqlite`, `state_5.sqlite`,
  `memories_1.sqlite`, `goals_1.sqlite` — schema migration by filename bump — with **live
  WAL/SHM files** (the exact shadow-home copy hazard we hit before:
  `is_volatile_state_file`).
- **The timezone kicker** — one artifact, three time representations:
  - filename `rollout-2026-06-25T09-51-36-…` → **local time**
  - record envelope `"timestamp":"2026-06-25T16:51:36.732Z"` → **ISO-8601 UTC**
  - payload `"started_at":1782406296` → **unix-seconds**

  A schema that captures "logs_directory: string" can never express this; a per-site
  time-semantics table can.

### Claude Code (`~/.claude/projects/`)

- `<sanitized-cwd>/<session-uuid>.jsonl` per session, sibling `<session-uuid>/subagents/`
  directories for subagent transcripts.
- `sessions-index.json` per project dir with an **explicit `"version": 1`** (in-band schema
  versioning exists in the wild) and `fileMtime` in **unix-millis**.
- First record of a session file was `"type":"last-prompt"` — a record type absent from the
  research's documented taxonomy (evidence catches vocabulary drift).

## Structural vocabulary the schema must express

Reality demands three flat record families (flat rows — the same shape lesson as the
signal catalog):

1. **`surfaces[]`** — a provider has *many* log surfaces, each with: role
   (session_transcript, subagent_transcript, session_index, prompt_history, app_log,
   state_db, live_metadata, statusline), per-OS path template, format (jsonl/json/sqlite/
   text), scope (per_session/per_project/global/per_process), naming convention, rotation
   policy, `live_locked` (open WAL/lock — never copy/symlink), and schema-versioning style
   (explicit_field / filename_suffix / none).
2. **`time_fields[]`** — per surface, per site: `unit` (iso8601/unix_seconds/unix_millis) +
   `zone` (utc/local/embedded_offset/unspecified) + `confidence`. Identical vocabulary to
   the signal catalog's detection records — the enums should be defined once and shared.
3. **`record_types[]`** — per surface: discriminator path + observed value vocabulary.

`live_locked` is a direct DRY win: the volatile-state knowledge currently hand-coded in
`repo_home.rs` (`is_volatile_state_file`) becomes catalog data.

## Grammar findings (verified with `md schema validate`)

1. **The current sequence `target_schema` blocks are not valid SimplifiedSchema.**
   `logs_directory: { macos: string, ... }` (YAML-native nested mapping) is **rejected**
   by the parser ("mapping property values are reserved for a future YAML-native schema
   shape"). The valid form is a quoted inline-object literal:
   `logs_directory: "{ macos: string, windows: string, linux: string }"`.
   `_fleet.md` and `_fleet.md` both use the invalid form today — the
   sequences run because `target_schema` is interpolated as prompt text, but any agent
   that transplants it verbatim into `$schema` produces a document whose schema fails to
   parse. **Phase 0 must normalize all target_schemas to the inline-literal form** (or
   darkmatter ships the reserved YAML-native shape first).
2. **Everything the draft schema needs works today**: enums with members inside inline
   objects, `string[]` inside inline objects, arrays of object literals (`"{ … }[]"`),
   optional properties inside objects, and deep validation with frontmatter line numbers
   (a bogus enum member inside `time_fields[]` fails with the offending line).
3. **Sidecar files MUST wrap their schema under a root `$schema:` key.** Darkmatter's
   resolver classifies a referenced YAML file as a SimplifiedSchema *only if its root
   holds a `$schema:` mapping*; a bare property mapping is classified as **raw JSON
   Schema**, which ignores unknown keywords and therefore validates **vacuously** — the
   file loads, missing files still error, but nothing is enforced. This initially looked
   like a validation bug; it is the documented disambiguation rule in
   `darkmatter/lib/src/markdown/schemas/resolve.rs`. With the wrapper in place, both
   reference forms enforce fully (deep enum members, required fields, line numbers):
   - string scalar: `$schema: ./_schema.yaml` (cleanest; supported at the resolve layer
     even though *inline* root strings are rejected by the parser)
   - root-union arm: `$schema: ["./_schema.yaml"]`
4. **`file` properties are lazy by default** — a `file`-typed *property* value does not
   check existence unless the `eager` constraint is set (`file(eager)`). Not needed for
   the logging schema (path fields are templates with placeholders → `string`), but the
   signal catalog's `evidence` field should be `file(eager)` so "fixture must exist" is
   schema-enforced.
5. Minor: `md schema validate` exits `2` on schema *definition* errors but `0` on document
   *validation* failure (report-style output). Sequence exit criteria that check the exit
   code should be aware; the criteria text says "returns `true`", which matches the
   rendered ✔/✗, not the exit code.

## Draft schema — PROMOTED

The draft ([`_schema.yaml`](_schema.yaml), pre-wrapper spike copy) was validated end-to-end
against real-evidence Claude frontmatter (positive) and corrupted/missing-field variants
(negative), then **promoted 2026-07-01** to `docs/research/agent-logging/_schema.yaml`
(with the root `$schema:` wrapper). The `_fleet.md` sequence file now points
target documents at `$schema: ./_schema.yaml` and its capture instructions fill the
`surfaces[]` / `time_fields[]` / `record_types[]` records, including the on-host evidence
requirement (the sequence already grants `state.user_dir` read).

## Pilot run (2026-07-01, Codex, OpenCode + GLM-5.2, `--yolo`)

The updated sequence was pilot-run for Codex only (temporary one-provider roster copy;
`claudine sequence` has no per-item selector — a `--only <name>` flag would remove that
friction). **Verdict: the fine-grained approach works end-to-end.** The produced
`codex.md` passes `md schema validate`, fills all three record families with
`confidence: observed` throughout, gets the time semantics right (filename **local**
ISO vs envelope **UTC** ISO vs payload **unix-seconds** — independently rediscovering the
three-representations finding), marks all WAL SQLite surfaces `live_locked: true` with
`filename_suffix` versioning, and sets `requires_claudine_update: true` with a concrete
reason (the rollout JSONL envelope was restructured to `{timestamp,type,payload}` —
directly actionable against claudine's codex stream parser).

Signal-catalog fodder discovered by the pilot that we did not know: rollout
`event_msg.token_count` carries `rate_limits.{primary,secondary}.resets_at`
(unix-seconds), and `goals_1.sqlite.thread_goals.status` includes `usage_limited` /
`budget_limited` states.

Defects the pilot surfaced (fixed inline or queued):

1. **`grant:` frontmatter is unimplemented** — silently ignored. Without it, OpenCode's
   `external_directory` permission is auto-rejected in non-interactive mode, and GLM
   interprets the rejection as "stop": two runs died at ~25s right after the reject,
   without writing anything. Workaround: `--yolo` (the YOLO permission overlay exists
   precisely to prevent this deadlock); real fix: implement `grant` → provider permission
   mapping. A comment now marks this in `_fleet.md`.
2. **Provider exit 0 ≠ exit criteria met.** Both truncated runs were reported
   `step succeeded`. Remedy (now in `_fleet.md`): a `success` lifecycle stack
   gating on `frontmatter(file, 'last_updated') != ctx.today` → styled stderr + `error`.
   This verification pattern should become standard in every research sequence.
3. **`markdown_file_empty` does not exist** (`markdown_body_empty` is the function) — the
   original `update:` expression could never have evaluated; whole-value strictness
   caught it on first run. `_fleet.md` carries the same bug.
4. **Lifecycle authoring friction** (documented here so the next author skips 3 failed
   runs): a stack item must have an `action:` key (scalar or array); a scalar `action`
   cannot take sibling keys — use an action *array* for message + control; `when:` is a
   whole expression evaluated directly (`frontmatter(file, ...) != ctx.today`), NOT an
   interpolated string (`'{{file}}'` inside `when:` crashes). The `_fleet.md`
   non-interactive sequence uses older shapes that likely no longer parse.
5. **Schema gap**: the `unit` enum needs `unix_nanos` — the pilot found
   `logs_2.sqlite.logs.ts_nanos` and had to mislabel it `unix_seconds`.
6. **`env.AGENT` / `env.MODEL` didn't reach the document** (`agent: open_code`,
   `model: default`) — either the env vars aren't set on this path or the agent guessed;
   worth a quick claudine-side check.

## Full run (2026-07-01, all 9 roster providers, `--yolo`)

`claudine sequence _fleet.md --yolo`: **9/9 succeeded (~50 min, ~5 min per
provider)**, every document schema-valid and stamped, no truncation (the success stack
never fired). Coverage: 3–18 surfaces and 12–25 time-field records per provider; only 8
`zone: unspecified` across all 94 fine-grained timestamp records.

Seven of nine set `requires_claudine_update: true`, and the reasons are concrete drift
findings against claudine's current code:

- **Kimi**: the wire parser pins `WIRE_PROTOCOL_VERSION='1.9'` and rejects the
  now-current 1.10 via strict equality — a live breakage waiting to happen.
- **Gemini**: on-disk transcripts changed to append-only `.jsonl` with a header line +
  `{"$set":{...}}` patches — legacy single-JSON ingestion is stale.
- **OpenCode**: the primary transcript/cost store is now SQLite (denormalized cost/token
  columns + event-sourced layer) which the JSONL-centric reporting module doesn't read.
- **Claude**: on-disk transcripts are a distinct format from stream-json with
  interactive-only record types claudine doesn't model.
- **Qwen**: an untapped debug-log surface + distributed-tracing spans.
- **Codex**: rollout envelope restructure (from the pilot) re-confirmed in update mode,
  which also found a surface the pilot missed (`update-check.json`).
- **Kilo** (roster-only provider): confirmed to be an OpenCode fork — the graduation
  path would start from the OpenCode adapter.

Housekeeping surfaced by the run: `providers.yaml` names Claude's file `claude.md`, but
the pre-existing research file was `claude-code.md` — the run created the new file and
left the old one as a stale duplicate (decide: `git rm claude-code.md` or align the
roster filename). `roo-code.md` is untouched because Roo is commented out of the roster.

## Live usage-cap specimen (2026-07-01, models/permissions runs)

The back-to-back agent-models + agent-permissions fleet runs hit the ZAI coding plan's
usage cap mid-flight (models 4/9 done — goose completed its write before the abort;
permissions 0/9). Three observations worth keeping:

1. **A live specimen of the signal catalog's target class**: OpenCode surfaced
   `AI_APICallError: Weekly/Monthly Limit Exhausted. Your limit will reset at
   2026-07-04 23:05:39` — with **no timezone on the reset time**. The exact
   `zone: unspecified` problem the detection-record schema forces us to track, observed
   in production on day one.
2. **Claudine handled it correctly**: classified the API error, aborted after 5
   no-progress retries (`provider stream failed 5 times with no progress`), exited
   non-zero. No hang, no false success.
3. **Resume gap closed**: re-running a sequence would have fully re-researched the
   completed docs (update mode). All three sequence files now carry an `initialize`
   skip stack — `file_exists(file) && frontmatter(file, 'last_updated') == ctx.today`
   → styled stderr + `skip` — making interrupted fleet runs resumable and same-day
   re-runs idempotent. Verified via dry-run: completed providers skip, missing files
   don't crash (`&&` short-circuits).

## Fleet completion (2026-07-02, kimi-for-coding/k2p7)

After switching both sequences to `kimi-for-coding/k2p7` (ZAI capped until Jul 4), the
resume run finished everything: **agent-models 9/9** (4 skipped via the initialize guard —
first production use, worked exactly as designed — 5 researched) and **agent-permissions
9/9**. Combined with logging, all **27 documents across 3 topics are schema-valid**, and
the `requires_claudine_update` reasons form a coherent work queue (18/27 true): e.g. Kimi
populates its model catalog dynamically from `/models` at login — though per Ken the
actual offering set is short and stable (`kimi-k2`, `kimi-k1.5`, `kimi-latest`,
`moonshot-v1`), so the right model is expected-offerings + drift-verification, not
"no static list", Codex permissions are a dual-layer sandbox+approval model
with Starlark execpolicy rules, OpenCode's last-rule-wins wildcard permission grammar
doesn't map cleanly onto PolicyEngine's axes.

**Anomaly + backlog item — requested model silently substituted.** One permissions launch
resolved OpenCode's primary model to `zai-coding-plan/glm-5.2` despite the frontmatter
(and `--dry-run` metadata, verified) saying `kimi-for-coding/k2p7`; the identical rerun
minutes later used k2p7. Transient OpenCode-side fallback. Claudine had the evidence
in-stream (`llm_call_start` model ≠ requested model) and stayed silent — a **model-mismatch
guard** (warn, or abort like the runaway guards) belongs on the wrapper backlog. It is
also a signal-catalog row: `model_resolved` / `model_fallback` detection is exactly this.

## Implications fed back into the spec

- The flat-record (`surfaces[]`/`time_fields[]`/`record_types[]`) approach is confirmed
  viable within today's SimplifiedSchema grammar — no darkmatter changes required.
- The `unit`/`zone`/`confidence` enums are shared vocabulary between the logging topic and
  the signal catalog's detection records — define once (shared schema fragment or the
  taxonomy module) and reference from both.
- `DarkmatterSchemas::detect()` bootstrap was **less useful than expected** for this topic:
  with 3-key frontmatter there is nothing to infer from. The realistic Phase 0 recipe is
  *evidence + prose → hand-draft schema → validate against real data* (exactly this
  spike), with `detect()` reserved for topics whose existing frontmatter is already rich.
- The evidence lens caught things no documentation pass would (local-time filenames,
  version-suffixed SQLite, undocumented record types) — reinforcing the signal catalog's
  fixture-first posture, and suggesting logging research prompts should also require
  on-host inspection when a `grant.read` for the provider's user dir is available (the
  sequence file already grants `{{state.user_dir}}` read).
