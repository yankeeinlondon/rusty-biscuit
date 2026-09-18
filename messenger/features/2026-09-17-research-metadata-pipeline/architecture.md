---
title: Research metadata pipeline architecture record
date: 2026-09-17
status: phase-1-proposed
review_required:
    - additional lifecycle subcommands (see "CLI and recipe surfaces")
    - artifact layout additions (see "Artifacts: authored versus generated")
---

# Architecture Record

This record converts the Phase 1 evidence into the decisions later phases rely
on. It adds no requirement beyond the [specification](spec.md); where the spec
left a choice to implementation planning, the choice and its reason are here.

Evidence:

- [Surface inventory](spikes/surface-inventory/inventory.md): emitted payload
  surfaces, receipts, error loss, fingerprint inputs (revision `d57faf7e8`).
- [Schema pilots](spikes/schema-pilots/findings.md): SimplifiedSchema
  expressiveness and the Rust-side rule list.
- [Publication spike](spikes/publication/findings.md): snapshot protocol with
  macOS, Linux, and native-Windows evidence.
- [Orchestration findings](spikes/orchestration/findings.md), section "Phase 1
  production-code probes": discovery isolation, cancellation, budget hooks.
- [Fixture matrix](fixture-matrix.md): acceptance criteria → fixtures → phase.

## Authorities

| Authority | Owns | Never owns |
|---|---|---|
| Roster `messenger/docs/platforms.yaml` | Platform, interface, adapter identity; coverage; curated sources; refresh interval | Facts |
| Accepted platform documents | External facts, evidence, gaps, changes, chronology | Implementation status |
| Reviewed mappings + checked-out code/tests | Implementation status per adapter | External facts, `CapabilitySet` values |
| Generated artifacts | Nothing; they are projections | — |

No projection writes `CapabilitySet`, provider code, or delivery behavior. A
`requires_messenger_update` gap is the only channel from research to code.

## Stable identifiers

All identifiers match `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`, are never reused after
removal (removal is recorded in `changes`), and are compared byte-for-byte.

| Identifier | Values / rule | Reason |
|---|---|---|
| `platform_id` | `discord`, `slack`, `telegram`, `whatsapp`, `signal` | Roster keys and document file stems |
| `adapter_id` | `discord`, `discord-webhook`, `slack`, `slack-webhook`, `telegram`, `whatsapp`, `signal` | Exactly `ProviderKind::as_str()`; already persisted in receipts, `MessageRef` tags, and CLI route config, so new spellings would re-key stored data |
| `interface_id` (sending) | `discord_bot_api`, `discord_webhook`, `slack_web_api`, `slack_incoming_webhook`, `telegram_bot_api`, `whatsapp_cloud_api`, `signal_cli_jsonrpc` | The spec requires distinct interface records for Discord bot/webhook and Slack Web API/webhook. Each maps to exactly one adapter |
| `interface_id` (research-only companion) | `{platform_id}_{name}`, e.g. `discord_gateway`, `discord_interactions`, `slack_events_api`, `slack_socket_mode`, `whatsapp_webhooks`; final list in Phase 2 | Relationship kinds per spec (`receives_events_for`, `responds_to_interactions_from`, `requires_companion_interface`); `adapters: []` and `role: companion` |
| `operation` | Provider-native operation name in snake case, e.g. `create_message`, `execute_webhook`, `chat_post_message`, `send_message`, `send_photo` | Scopes constraints and errors |
| Fact IDs | Unique within a platform document; logical key `(platform_id, interface_id, operation, fact_id)` for constraints and `(platform_id, interface_id, error_id)` for errors | Spec logical key; document-local uniqueness keeps authoring simple while the catalog uses the full key |
| `source_id` | Unique within a platform document | Evidence references are document-local |

## Module ownership and feature graph

```text
messenger (lib)
  feature research = [darkmatter, biscuit-file, biscuit-hash]   # opt-in, no default
    messenger::research::
      model       authored DTOs (serde, closed records), schema-version gate
      load        passive loader: FileReference + captured context, Darkmatter
                  library schema validation, then typed deserialization
      validate    semantic rules (identity, constraints, bindings, interaction,
                  errors, coverage) producing stable-order diagnostics
      assess      reviewed implementation mappings + input fingerprints
      project     catalog DTOs, report models, the five handoffs
      delta       fact-level before/after comparison + fixed flags
      approval    human-approval and unchanged-renewal rules
      publish     manifest/journal protocol: publish, recover, read_verified
      state       local state area: runs, candidates, retention, cleanup plans
      run_config  per-platform limit validation (no defaults)
messenger-cli
  depends on messenger with features [..chat providers.., desktop, research]
  `messenger research …` subcommands, TerminalRenderable output, JSON output
claudine-cli (Phase 5)
  budget ledger + admission in its existing launch path; knows nothing about
  research records
```

- Ordinary send builds (`--no-default-features`, default, `desktop`) have no
  `darkmatter`, `biscuit-file`, `biscuit-hash`, or YAML dependency today;
  Phase 3 keeps that true (check in [fixture matrix](fixture-matrix.md)).
- `messenger` must not depend on Claudine: `claudine` already depends on
  `messenger`, so the edge would be a cycle. Messenger reaches Claudine only
  through `just` recipes that invoke the `claudine` binary; it never spawns it.
- `darkmatter` does not depend on `messenger` or `claudine`, so the `research`
  feature adds no cycle.
- `messenger-cli` enables `research` unconditionally. The spec places the
  maintenance commands in `messenger-cli`; a second binary or CLI feature flag
  would be speculative. The cost is a larger `messenger` binary.
- The `research` module performs no network access, spawns no process, and
  never evaluates frontmatter expressions. Source rechecks happen inside
  agent passes, not in Messenger.
- The shared atomic replacement primitive (`std::fs::rename`, `sync_all`,
  bounded Windows retry on errors 5/32, Unix directory fsync, tagged temp
  names) lives private to `messenger::research::publish` for this feature. The
  publication spike found four near-duplicate helpers elsewhere; consolidating
  them into `biscuit-file` is a separate follow-up, not this feature's scope.

## Schema and type boundaries

- One SimplifiedSchema file, `messenger/docs/research/platforms/_schema.yaml`,
  describes platform documents. The roster and mappings files have their own
  small schemas beside them. Darkmatter's library validates shape; Messenger
  deserializes the same vocabulary into Rust types with
  `#[serde(deny_unknown_fields)]` and closed enums.
- The library entry points exist today: `md schema validate` is a thin
  wrapper over `darkmatter::markdown::schemas::DarkmatterSchemas::validate`
  on a `darkmatter::markdown::Markdown`, with
  `darkmatter::markdown::compose::capture_file_resolution_context` for file
  resolution. Messenger calls these directly and never shells out to `md`.
  Passivity (no expression evaluation, effects, or network) is proved in a
  dev-only test with Darkmatter's `effects-instrumentation` feature, which
  exists to let downstream suites assert that no `EffectEngine` is built and
  no network access is attempted.
- Type layers, so that an unknown or conflict cannot inhabit an executable
  type: authored DTOs (mirror the file) → validated records (IDs resolved,
  references checked) → executable projections (only known, eligible facts;
  constructed solely by `validate`) → catalog DTOs (serialization only).
- Rules that SimplifiedSchema cannot express, and that Rust therefore owns, are
  the `SR-*` rules in the schema-pilot findings. Phase 3 assigns them to the
  validators: identity (`SR-VERSION`, `SR-STRICT-SCALARS`, `SR-ROSTER`,
  `SR-UNIQUE`, `SR-REF`, `SR-EVIDENCE`, `SR-GAP`, `SR-CHANGE`), constraints
  (`SR-STATE-VALUE`, `SR-CONDITION`, `SR-APPLICABILITY`, `SR-AGGREGATE`,
  `SR-KIND-UNIT`, `SR-ENFORCEABLE`, `SR-COVERAGE`), bindings (`SR-FORMAT`,
  `SR-IMAGE`), interaction (`SR-INTERACTIVITY`), errors (`SR-ENVELOPE`,
  `SR-MATCH`, `SR-MATCH-OVERLAP`, `SR-ORIGIN-PHASE`, `SR-FIXTURES`), and
  mappings (`SR-MAPPING`).
- Schema decisions from the pilots:
  - **Two schema files.** Named types live in a sibling
    `docs/research/platforms/_types.yaml`, imported as `Name@./_types.yaml`,
    because a standalone schema file otherwise leaks its named types into the
    document root (pilot L2). This is a layout addition to the spec's single
    `_schema.yaml`.
  - **Root closure** uses the root pattern key
    `"<pattern::^(prompt|hash)$>": any`, which the pilots showed closes the
    root although Darkmatter documents roots as open (L1). Rust also rejects
    unknown top-level keys outside the allowlist, so the contract does not
    depend solely on that behavior. Ask Darkmatter to document it or add a
    `strict` root option.
  - **Flat records, not unions.** SimplifiedSchema cannot make arrays of
    unions or require a union-typed property (L3–L5), so knowledge,
    conditions, and match signatures are flat `kind`/`state` records and
    state/value and operand consistency are Rust rules.
  - **One knowledge state per record.** A fact whose value is known but whose
    overflow behavior conflicts is split into separate records, as the pilot
    did. This keeps "only known, fully resolved records are executable"
    enforceable by type.
  - **Explicit per-interface duplication** of shared service bounds (for
    example Discord bot and webhook `content`), because the logical key
    includes the interface.
  - **Strict scalars.** Darkmatter always coerces types (L9: `"2000"` becomes
    a number, `9.50` becomes `"9.5"`), so the Rust loader rejects string
    numbers and non-string versions itself.
  - **Fenced YAML.** `md` validates only frontmatter (L10), so the roster and
    mappings files carry `---` fences.
  - **One-line types.** Multi-line inline objects fail to load from a
    standalone schema file (L6, probably a Darkmatter bug). Phase 2 either
    accepts one-line types or gets a Darkmatter fix first; this is a reviewability
    cost, not a correctness risk.
- Vocabulary additions adopted for Phase 2 (all from pilot evidence): units
  `bytes` and `unknown`; constraint kinds `hard_min` and `item_count_min`;
  condition kinds `permission_grant` and `release_age`; interface `direction`;
  text-binding `content_role`; error `candidate_match` (never executable) and
  `replay_safety`; fixture provenance `observed_in_repo`; image `submissions`
  as an array; the provider-neutral rich-object and interactive surface names
  listed in the findings. Attribution, location, and expression use the
  dedicated `attribution_binding`, `location_binding`, and
  `expression_bindings` records the spec defines; the pilot's generic
  `capabilities` shape is not carried forward.
- Schema versioning: `schema_version` is a positive integer; version 1 is
  frozen at the Phase 2 checkpoint. The loader accepts exactly the supported
  set (`{1}`); any other value is an error, never a best-effort parse. A
  version bump requires migrating every accepted document before publication,
  because publication refuses mixed schema versions. The catalog and manifest
  record both the version and the schema file's hash.

## Artifacts: authored versus generated

Paths are relative to `messenger/` unless they start with `.claude/`.

| Path | Kind | Committed | Notes |
|---|---|---|---|
| `docs/platforms.yaml` (+ `docs/platforms.schema.yaml`) | Authored | Yes | Roster |
| `docs/research/platforms/_fleet.md` | Authored | Yes | Shared research instructions |
| `docs/research/platforms/_schema.yaml` | Authored | Yes | Document schema |
| `docs/research/platforms/_types.yaml` | Authored | Yes | **New path.** Named types for `_schema.yaml` (pilot L2) |
| `docs/research/platforms/{platform}.md` | Authored, accepted only through promotion | Yes | Candidates never write here |
| `docs/research/platforms/_overrides.yaml` | Authored | Yes, only when needed | Spec rules for overrides |
| `docs/research/implementation/mappings.yaml` (+ schema) | Authored, human-reviewed | Yes | **New path.** Reviewed adapter mappings and fingerprints; separate from external facts |
| `docs/research/platforms/catalog.json` | Generated | Yes | |
| `docs/research/summary/platforms.md` | Generated tables + authored guidance | Yes | Machine-derived sections are delimited and regenerated; prose outside them is authored |
| `docs/research/CHANGELOG.md` | Generated from approved review records | Yes | Accepted changes only |
| `docs/research/reviews/{date}-{platform}-{run_id}.json` | Generated at promotion | Yes | Durable delta + structured evidence review for accepted changes |
| `docs/research/publication.json` | Generated | Yes | **New path.** Snapshot manifest and selection point |
| `.claude/skills/messenger/platform-metadata.md` | Generated | Yes | Repo root |
| `.research-state/` | Local working records | No (gitignored) | **New path.** Per worktree |

Generated files start with a "generated — do not edit" marker naming the
command that produces them. The manifest makes hand edits detectable: a
verified read refuses any artifact whose bytes differ from the manifest.

## Local state area

`messenger/.research-state/` (add `messenger/.research-state/` and
`*.publish-tmp` to the root `.gitignore` in Phase 4):

```text
.research-state/
  publication/            lock, journal.json, tx/<txid>/{new,old}/   (publish spike)
  runs/<platform>/<run_id>/
    run.json              stage, status, created, platform, schema/prompt hashes
    budget.json(.lock)    Claudine-owned ledger (read-only to Messenger)
    inputs/discovery/     prepared discovery input: identification + questions + schema only
    inputs/reconcile/     discovery output + curated sources + previous research
    candidate/{platform}.md
    source-checks.json    attempt record per curated source
    delta.json, evidence-review.json, validation.json
  renewals/               routine unchanged-renewal maintenance records
```

`run_id` is `{UTC date}-{8 hex}`; statuses are `active`, `awaiting_review`,
`exhausted`, `interrupted`, `failed`, `rejected`, `accepted`. Workers run with
their working directory set to the prepared `inputs/<pass>/` directory, never the
repository root. This separates inputs; it is not a sandbox, because an
agent can still read absolute paths.

## Snapshot publication and recovery

Adopt the publication spike's strategy A without change:

- `docs/research/publication.json` is the single selection point. Fields:
  `format` (`messenger-research-publication/1`), `schema {version, xxh64}`,
  `inputs[] {path, xxh64}` (roster, schema, fleet prompt, mappings, overrides,
  and each accepted document's Darkmatter frontmatter/body hash pair), `artifacts[]
  {path, xxh64, len}`, `snapshot_id`. Sorted keys, LF, no timestamps or host
  paths.
- The journal and staged/backup copies live under
  `.research-state/publication/`; recovery rolls back or forward depending on
  whether the committed manifest matches the old or new hash, and refuses
  otherwise.
- Every generation, `--check`, report, and promotion reads through
  `read_verified`. Writers (`generate`, `promote`) refuse with
  `RecoveryRequired` when a journal is pending and direct the operator to
  `messenger research recover`, so no lifecycle mutation is hidden inside
  `generate`.
- Partial refresh: a new snapshot combines newly accepted documents with prior
  accepted documents only if those satisfy the current schema version and
  coverage; otherwise publication is refused and the previous snapshot stays
  selected.
- Readers outside the protocol (editors, git, agents reading the skill file)
  can see a mixture during a publication or after a crash until recovery. The
  requirement covers eligibility for generation and reporting, which
  `read_verified` enforces.

## Relevant-input fingerprinting

- Each accepted assessment in `mappings.yaml` is scoped to
  `(adapter_id, category)` rather than to a whole adapter, and lists its own
  relevant inputs as repository-relative file paths (optionally with symbol
  names as documentation) and the fact IDs it assessed. Its fingerprint
  covers both sides: xxh64 of each code/test file, and xxh64 of each assessed
  fact record's canonical JSON (key-sorted, compact, produced by the Rust
  loader). The pilot used SHA-256; production uses `biscuit-hash` xxh64 as
  the spec requires.
- Content is hashed after normalizing CRLF to LF so Windows checkouts with
  `core.autocrlf` produce the same fingerprint.
- Reuse requires fingerprint equality. Any difference makes that assessment
  `unassessed` and emits a `requires_messenger_update` gap; unrelated commits
  change only the recorded `inspected_revision`, which is provenance, not a
  reuse key.
- Per-category scoping limits churn from shared files that the surface
  inventory found (renderers, `prepared.rs`, `validate.rs`, receipts): a
  shared renderer edit invalidates only the formatting assessments that list
  it. Symbol-level hashing is out of scope; the spec disclaims complete
  dependency analysis.

## CLI and recipe surfaces

Required and stable: `messenger research validate`, `generate`,
`generate --check`, `report` (platform/interface/operation filters, `--json`).

Additional explicit subcommands (**maintainer review required**):

| Command | Why it is needed |
|---|---|
| `messenger research prepare <platform> --max-seconds N --max-invocations N [--force]` | Validates the run configuration (no defaults), selects whether a refresh is due, creates the run directory and the three prepared inputs, and prints the `claudine sequence` invocation. The spec assigns run-configuration validation to `messenger-cli` |
| `messenger research runs [--platform P] [--json]` | Run inspection and storage-location reporting (spec: `messenger-cli` owns these) |
| `messenger research promote <run_id> --approved-by <name>` or `--renewal` | The only operation that writes accepted documents; records approval (or verifies the unchanged-renewal rule), then publishes through the journal |
| `messenger research recover` | Explicit recovery of a pending publication journal |
| `messenger research cleanup [--older-than 30d] [--apply]` | Dry-run by default; prints exact removals and lost resumability; deletes only with `--apply`, never unattended, never touches Git |

Package-area recipes wrap these, plus `research-refresh <platform>` (prepare,
then `claudine sequence` with the budget ledger) and `research-publish`. No
recipe uses Unix-only filesystem utilities. Human output uses
`TerminalRenderable` components; `--json` output has no escape sequences.

## Orchestration and budget boundary

- One platform run is one budgeted `claudine sequence`: discovery,
  reconciliation, source-list maintenance, validation (a shell task invoking
  `messenger research validate` on the candidate), delta, the independent
  evidence reviewer, and at most two recovery attempts. Validation and delta
  run inside the sequence, so the active clock charges them.
- Claudine owns the ledger file exclusively (Messenger reads it for
  reporting). The ledger follows the orchestration findings: invocation
  charged before spawn and never refunded; wall-clock segments with a
  heartbeat; admission in `execute_attempt_phase` before
  `execute_harness_attempt` so lifecycle retries, resumes, proxies, and group
  tasks are all counted; retry waits capped to the remaining budget; grants
  only by recorded operator decision.
- Crash accounting: time is charged through the last heartbeat plus one
  heartbeat interval; the run then enters `interrupted`, an
  operator-resumption state, so the downtime until an operator resumes is not
  charged. This follows the spec's rule that only explicit stop/suspension for
  human approval or operator resumption pauses the clock, while still never
  refunding consumed time.
- The run stops in `suspended` (awaiting human approval) at the end of the
  sequence; promotion happens outside the budget because approval time is
  explicitly not charged.
- Discovery must never run through `inline-compose` or a sequence whose source
  has a top-level `prompt`, because inline-compose hands the worker the
  existing document path by design.
- Cancellation evidence covers local processes only. On Unix, a timeout
  signals the child's process group; a descendant that calls `setsid` escapes,
  and a crash of the wrapper orphans the worker (recovery must kill a
  verified orphan group). On Windows, the Job Object terminates the tree.
  Reports state that remote model work and billing may continue.
- GitNexus impact for the hook points was `UNKNOWN` because the index was
  mid-update; Phase 5 must refresh the index (`just gitnexus`) and re-run
  impact before editing. `isolate_into_process_group` is CRITICAL and stays
  unchanged.

## Cleanup protection rules

`cleanup` only considers `.research-state/runs/*` and `renewals/*` older than
the threshold, and never:

- a run in `active`, `awaiting_review`, or `interrupted` state, or one whose
  ledger lock is held;
- `publication/` while a journal is pending;
- anything under `docs/research/` (accepted evidence and reviews are
  committed and outside the state area).

The preview lists each path, its age, its status, and whether removing it
ends the ability to resume that run.

## Resolution of the spec's planning items

| Spec item | Resolution |
|---|---|
| Discovery isolation, accounting, shared time, cancellation reporting, recovery | Orchestration boundary above; Phase 5 list in the orchestration findings |
| Inline-compose and operating systems | Discovery never uses inline-compose; per-OS gaps below |
| Publication mechanism | Strategy A (manifest + journal) |
| Review-artifact and local paths, inspection, cleanup | `docs/research/reviews/`, `.research-state/`, `runs`, `cleanup` |
| Explicit limits before live research | `prepare` requires both limits; no defaults; Phase 7 operator supplies values |
| Schema and mappings against pilots, fingerprint invalidation | Schema-pilot findings; per-category fingerprints |

## Evidence gaps carried forward

- WSL2: neither the publication prototype nor the budget probes ran (SSH to
  `build-win` was reset; `W:` was nearly full). Owed before Phase 4 and
  Phase 5 checkpoints.
- Native Windows: the Claudine binary was not run; the evidence is OS-level
  Job Object behavior only.
- Remote model cancellation and billing: unverifiable locally on every OS.

## Recorded, not fixed

The surface inventory found suspected runtime defects: the Slack webhook
response parsing, the Signal group-send method name, and Telegram's loss of
`retry_after` on HTTP 429. This feature must not change delivery behavior. They
become research priorities for Phase 7 and candidate `requires_messenger_update`
gaps, then separate fixes.
