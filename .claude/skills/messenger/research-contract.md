# Provider Research Contract

Platform research is becoming typed, reviewed metadata (feature
`messenger/features/2026-09-17-research-metadata-pipeline/`). It never changes
`CapabilitySet` or delivery behavior.

- **Roster** `messenger/docs/platforms.yaml`: five platforms, seven sending
  interfaces whose `adapters` are exactly `ProviderKind::as_str()`, research-only
  companions (`adapters: []`), curated sources (cap 10 per platform),
  `refresh_interval_days` (default and per-platform override, 1 to
  `MAX_REFRESH_INTERVAL_DAYS` = 3660 in both schemas and SR-ROSTER), and
  explicit exclusions (email, desktop, APNs, FCM).
- **Schema v1 (frozen)** `docs/research/platforms/_schema.yaml`, with every named
  type in `_types.yaml` (shared by the roster, overrides, and
  `docs/research/implementation/_schema.yaml` mappings schemas). Rules the schema
  cannot express, and their Rust owners, are in `_rules.md`.
- **Fleet prompt** `docs/research/platforms/_fleet.md`: the shared three-pass
  instructions. Each platform document's `prompt` only delegates to it.
- **Typed layer** `messenger::research` (feature `research`, off by default;
  pulls in `darkmatter`, `biscuit-file`, `biscuit-hash`, `serde_path_to_error`).
  `Loader` binds `$schema` to the shipped schema for the file kind, validates
  through Darkmatter's library, version-gates, then deserializes the *authored*
  frontmatter; `validate_document` runs the `SR-*` rules in `Scope::Fragment`
  (fixtures, candidates) or `Scope::Accepted` (baselines: full roster, only
  investigated gaps). Only a `ValidatedDocument` yields executable constraints
  (`eligibility()`); `assess::evaluate` decides fingerprint reuse.
- **Fixtures** `lib/tests/fixtures/research/`; `lib/tests/research_corpus.rs`
  runs the corpus through Darkmatter and the typed rules;
  `lib/tests/research_validation.rs` pins targeted behavior with variants
  written into a temp workspace that copies the shipped schemas.

## Publication and consumers

- **Snapshot** = the five accepted documents, `docs/research/platforms/catalog.json`,
  and the generated region of `docs/research/summary/platforms.md`, selected by
  the committed manifest `docs/research/publication.json` (`research::publish`,
  a port of the Phase 1 manifest + journal spike). Journal and staging live in
  the gitignored `messenger/.research-state/publication/`.
- `research::generate::{generate, check}` drive it: fleet load →
  `Fleet::validate` (Accepted scope) → `Fleet::snapshot` → `publish`. With a
  published snapshot, documents come **only** from verified bytes or explicit
  `updates` (partial refresh); without one, the fixed paths are the initial
  baseline. Refusal, interruption, or a pending journal leaves the previous
  snapshot selected; writers refuse with `RecoveryRequired` until `recover`.
- `research::project` builds the catalog (serialization-only DTOs; sorted by
  spelled IDs; no clock, no host paths; `refresh_due` recorded as a date so
  staleness is judged by readers). `Date` is fixed-width `YYYY-MM-DD`, so date
  arithmetic is `Date::checked_plus_days` (`None` past 9999-12-31): a
  `last_updated` whose refresh date overflows is an SR-ROSTER finding,
  `project` returns that diagnostic instead of a catalog, and selection
  reports the platform `schema_invalid`, never `expired` or current. `research::report` reads the published
  catalog through `CatalogView` (reason codes only, so no executable type can be
  deserialized) and stays available when today's validation fails.
  `research::delta` is the mechanical fact-level comparison with the six fixed
  flags. `report::emitted_surfaces` is Messenger's own surface list per
  adapter and must change with the adapter code.
- CLI: `messenger research validate|generate [--check]|report|recover`
  (`--root`, `--today`, `--json`; exit 0 / 1 findings-drift-refusal / 2 usage /
  3 cannot run). Tests: `lib/tests/research_publication.rs` (fault injection at
  every step), `lib/tests/research_lifecycle.rs` (delta),
  `cli/tests/research_cli.rs` (real binary). Baseline fixture:
  `lib/tests/fixtures/research/lifecycle/fleet/`.

## Refresh runner and fleet lock

- A platform refresh is one `claudine sequence --yolo <run>/run.md` execution,
  run by the Messenger-owned runner (`messenger research run RUN_ID`; library
  `research::refresh::runner`). The runner takes the OS-backed fleet lock
  (`messenger/.research-state/runs/fleet.lock`, std `File::try_lock` through
  `publish::fsutil::PublicationLock`) for the whole execution — one research
  run at a time — marks the record `launched`, runs the sequence from the
  repository root, and records what it observed: `finished` (exit `0`, the
  status the run's `check-run` steps decided stays), `interrupted` (Ctrl-C,
  exit `130` → the run rests `interrupted`), or `stopped` (any other exit or a
  spawn failure → an undecided run rests `failed` with the exit named). The
  run's `RunLimits` elapsed-time cap is enforced here. The run owns the
  sequence's process tree and `run()` returns (releasing the lock) only after it
  settles (`refresh/owned_tree.rs`, sysinfo, PID + start-time identity): on the
  limit or a `try_wait` error it sends `SIGINT` to the direct child only (Unix;
  Claudine's Ctrl-C cleanup path, exit 130), waits up to `STOP_GRACE` (10 s;
  tests use `#[doc(hidden)] execute_with_grace`), then kills every observed
  descendant deepest first plus the child. A normal exit kills nothing:
  Claudine has settled its tasks, and Playa's detached audio worker must
  outlive it. Descendants are sampled every fourth poll (~1 s) because Claudine
  puts each task in its own process group, so a group kill cannot reach them;
  never move Claudine into a new group from here (it would leave the
  terminal's foreground group and break Ctrl-C → 130). A Unix double-fork +
  `setsid` completed between samples after its parent exits escapes. Every prepared prompt
  reserves an invocation through its `start` shell hook (`research admit-agent`);
  `RunRecord.invocations` persists across resumptions. The preparation lock
  serializes admission updates, and refusal prevents the provider launch.
  Missing counts in ledger-era records mean zero; malformed present counts
  are errors. The runner installs hooks on legacy prepared prompts too.
- Cleanup, promotion, and stale-run recovery are refused
  (`RefreshError::RunnerLive`) while a runner holds the fleet lock. The OS
  releases the lock when the holder dies, so a crashed runner leaves a
  launched `active` record that `runs` reports as stale; `cleanup --recover
  RUN_ID` marks it `interrupted` explicitly, never silently.
- Put validation and the delta inside the sequence (they are `shell:` steps),
  so the runner's elapsed-time cap covers them.
- The former budget ledger workflow was removed 2026-10. Ledger-era records
  with status `exhausted` stay readable,
  resumable like `interrupted`, never promotable, never shown successful; a
  leftover `budget.json` beside a record is ignored (reported as a note by
  `runs`) and never deleted.

## Refresh and review (`research::refresh`)

- **Commands**: `prepare` (selection + limits + run dir; `--dry-run`,
  `--resume RUN_ID`), `run RUN_ID` (the fleet-locked runner), `check-run
  RUN_ID [--through validation|review]` (the sequence's `shell:` steps),
  `runs`, `promote RUN_ID… --approved-by NAME | --renewal`, `reject`,
  `cleanup [--apply|--recover RUN_ID]`. `prepare` prints `messenger research
  run RUN_ID`; the runner spawns `claudine sequence --yolo …/run.md`.
- **Run layout** `messenger/.research-state/runs/<platform>/<run_id>/`:
  `run.json`, `run.md`, `inputs/{discovery,
  reconcile,sources,review}/`, `outputs/{discovery.md, suggested-sources.json,
  source-proposal.json, evidence-review.json}`, `candidate/<platform>.md` (with
  schema copies so `md schema validate` works there), `source-checks.json`,
  `validation.json`, `delta.json`. Renewal records go to `renewals/`.
- **Selection reasons**: `missing`, `expired`, `forced`, `schema_invalid`,
  `prompt_changed`, `schema_changed`, `no_review_record`, `version_changed`
  (`--observed-version IFACE=VER`). What a document was "researched under"
  comes from the newest published review record or local renewal record; a
  generate-only snapshot has neither, so every platform reads
  `no_review_record`.
- **Selection fails closed**: `StateArea::list` keys each run on the platform
  directory holding it and returns unreadable records (and records whose
  platform or run ID disagree with their directory) as errors. Selection turns
  those into `Skip::UnreadableRun { path, error }` for that platform, never
  into "no open run". `prepare` and `prepare --resume` hold
  `runs/prepare.lock` (an OS `File::try_lock`, released on process exit, so no
  stale lock) across selection and creation; a concurrent second one gets
  `RefreshError::PrepareBusy` (CLI exit `3`, like the publication lock).
  It is separate from Claudine's `runs/fleet.lock`. `select::blocking_runs`
  is the one rule for both: `resume` calls it (excluding the resumed run)
  under the same lock and refuses with `RefreshError::OtherRunBlocks { path }`
  (CLI exit `1`), because a `failed` run is not open and a newer run may exist.
- **Integrity rules** (in `check`, beyond validation): same platform, same
  `created`, every chronology ID kept, removed IDs listed in `changes`,
  `last_updated` moves only with a successful check, and a curated source's
  `retrieved` changes only with a successful check on that exact date.
- **Approval**: `approval::decide` → `Ineligible` / `Renewal` /
  `HumanRequired`. Renewal = frontmatter equal after dropping `last_updated`,
  `agent`, `model`, and `sources[*].retrieved`; same body hash; no curated
  change; same prompt and schema fingerprints as the last review/renewal; no
  inaccessible check; every URL source rechecked on its recorded date.
- **Decision inputs**: `refresh::input::{Maintainer, DecisionReason}` are
  trimmed and never blank. A `Maintainer` also holds no control character
  (`char::is_control`), because the CHANGELOG renders it on one Markdown line.
  A `DecisionReason` may span lines: it lives only as JSON in `run.json`.
  `promote::Request::Human` and `promote::reject` take them, so the library
  cannot record an unnamed decision. Review and run records deserialize
  through the same constructors, so an invalid persisted name fails to load.
  The CLI maps `RefreshError::Input` to exit `2`.
- **History**: a human promotion adds `docs/research/reviews/{date}-{platform}-
  {run_id}.json` to the same publication; `generate` carries every review
  record forward and renders `CHANGELOG.md` from them (rule `SR-REVIEW`).
- **Initial baseline**: publication needs every active platform, so the first
  promotion passes all five run IDs to one `promote`. One run alone is refused
  (`GenerateError::Refused` with `missing`) and nothing is written.
- **Recovery**: `prepare --resume` reruns from the first incomplete stage
  (reconcile when validation failed), at most twice, preserving the run's
  invocation count. Only failed, interrupted, or legacy exhausted records
  resume; another open or unreadable run for the platform blocks resumption.
  Resume resets the attempt to unlaunched `active` and clears its
  previous runner observation.
- **Recorded status**: the Messenger runner rereads `run.json` after the
  sequence settles, preserving statuses decided by `check-run`. Exit `130`
  records `interrupted`; another nonzero exit or spawn failure changes an
  undecided `active` run to `failed`. Exit `0` records `finished` without
  making incomplete research successful. A launched `active` record whose
  runner died needs explicit stale-run recovery; an unlaunched prepared run
  is started with `messenger research run RUN_ID`.
- **Recipes** (`messenger/justfile`): `research-validate|generate|check|
  report|runs|cleanup` wrap the offline commands; `research-publish NAME
  RUN…` is `promote --approved-by`. `research-refresh SECONDS INVOCATIONS
  [prepare args] [-- sequence args]` builds both CLIs, puts the target's
  `debug/` first on PATH (via `cygpath -u` on Git Bash), runs `prepare
  --json` from the repository root, then each run's `messenger research run`
  in turn. It continues past a failed platform and exits 1 at
  the end. Needs `jq`; bash 3.2-safe (no `mapfile`). Workflow docs:
  `messenger/docs/user-guide.md#provider-research`.
- **Launching a live run** (checked 2026-09-18 on the shipped roster):
  - Put this worktree's `target/debug` first on PATH. An older installed
    `claudine` may lack the current sequence behavior, and the `shell:` steps
    call `messenger research check-run`.
  - `run.md` has no `agent` hint. With no terminal, `claudine sequence` fails
    with `AgentResolutionFailed` before the first step. With a terminal, it
    opens a picker, and that wait runs under the runner's elapsed-time cap.
    Pass a provider flag (for example `--claude`) after `--` to
    `just research-refresh` or to `messenger research run RUN_ID -- --claude`;
    the runner appends it to the sequence command.
  - `sequence --dry-run` still runs the `shell:` steps. `check-run` then marks
    the run `failed` because it has no outputs. Rehearse only in a throwaway
    Git repository holding a copy of `messenger/docs`, never on a real run.
  - Run the printed commands from the repository root. Prepared inputs name
    only repository-relative `/` paths, and the checks are `check-run RUN_ID
    --through STAGE --root .`, so no host path reaches an agent or a shell.
    Claudine hands a `shell:` string verbatim to `sh -c` (Unix) or `cmd /D /C`
    (Windows, raw tail) with no argument-vector form, no per-step working
    directory, and `{{ }}` template expansion first, so a spliced path could
    never be quoted safely. A step runs in the launch directory, or in its Git
    root after an agent step; Claudine starts agents in that Git root. Both
    are the repository root when launched from there, as `just
    research-refresh` does. So `prepare` and `prepare --resume` refuse a root
    that is inside a Git work tree but not its top level
    (`RefreshError::NotRepositoryTopLevel`, exit 2, nothing written). A root
    outside any Git repository stays allowed. The check treats a root holding
    its own `.git` as the top level, then compares canonical paths, so
    `/private/var`, `\\?\`, and case spellings are not refused.

## Gotchas

SimplifiedSchema types must be one line and regex groups need
`pattern('…')`. YAML data files need `---` fences for `md schema validate`,
but fenced files cannot be read by `biscuit_file::Yaml` (Claudine sequence
sources). Darkmatter coerces `"2000"` to a number, so strict scalars are a Rust
check.

Resolve schemas once: `DarkmatterSchemas::validate` re-resolves imports per
document (~100 ms debug), which pushed corpus tests past nextest's 30 s
termination under full-suite load. `Loader` caches the effective schema per
contract kind; test helpers cache by canonical `$schema` path.

Semantic fixtures are held to "every finding carries the expected rule", which
exposed several Phase 2 fixture defects (duplicated matrices, dangling IDs).
When adding a fixture, run the typed corpus test, not only `md schema
validate`. Header conventions: `# expect-rule`, `# validate-scope: accepted`,
`# expect-ineligible: <fact> <reason>`.

The summary artifact is hashed by its generated region only
(`ArtifactScope::GeneratedRegions`), so maintainers may edit the prose around
it; editing inside the markers is a verification failure. Any other artifact
is whole-file. Manifest and journal paths are checked with
`paths::is_portable` before use: a hostile manifest could otherwise make
publication remove files outside the repository.

`Loader::load_document_text` judges a candidate's text *as if* it sat at the
accepted path, so a relative `$schema` resolves the way it will after
publication. Load candidates this way, not from their state-area location.

Dense `Table`s cannot render at narrow widths ("Table could not be rendered in
N columns"); the human report uses word-wrapped `UnorderedList`s instead, and
`UnorderedList` items are plain text (do not `Prose::escape_text` them).

`delta::compare` asserts both documents describe one platform; a worker can
write another platform's document into the candidate slot, so `check` skips
the delta on a platform mismatch and reports it as a finding. A candidate
whose frontmatter does not parse is a validation finding, never a hard error.

`promote` finishes a run whose candidate is already the published document
(an interrupted publication rolled forward by `recover`) instead of refusing
it as a stale baseline.
