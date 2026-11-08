---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/spec.md"
plan: "darkmatter/features/2026-11-05-dynamic-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/enum_item_tests.rs
    - darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs
    - darkmatter/lib/tests/l1/dynamic_slots_fixture_repos.rs
    - darkmatter/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/fixtures/dynamic_slots/value-positions.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/revision.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/minimum.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/maximum.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/default-retries.yaml
    - simplified-schema/lib/tests/fixtures/dynamic_slots/values/slug-pattern.yaml
docs_updated_during_phase_1:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/autocomplete.md
    - simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter-lsp
    - darkmatter
    - simplified-schema
decisions_for_review:
    - |-
        Array-item completion offers every scalar candidate of the item type, not only enum members.
        The plan asked for enum members inside a list such as the sandboxing `os[]`. The fix routes a
        `- ` item and a flow-list element of a one-type array through the same code that completes a
        plain value, so `boolean[]` items now offer true/false, `file[]` items offer paths, and
        `literal(...)` item types offer their value, besides enum members. The alternative was an
        enum-only branch, which is smaller but leaves a second, narrower copy of the value logic.
        Reverting means replacing `array_item_value_completions` with an enum-members-only lookup.
    - |-
        An unclosed flow list (`os_support: [` with no `]`) still offers nothing.
        The frontmatter no longer parses at that point, which stops every completion, not only enums;
        `suggest(...)` lists behave the same today. Editors close the bracket as it is typed, and the
        tests use that closed form. Making it work would mean a tolerant YAML recovery for the whole
        frontmatter provider, which is a separate change; the limit is documented in the DMLS docs.
    - |-
        The fixture repository offers five states of the data file: clean, dirty, untracked, ignored,
        and no repository. The approval matrix (Phase 5) also needs symlink-escaping and other-repository
        cells; they are left for Phase 5 to add on top of `SlotRepo` (it exposes the root and paths),
        because their shape depends on the grant rule built there. Adding them now would only move work.
    - |-
        A template's lowering error carries only its slots for now. The plan's `UnresolvedSlots` also holds
        a `pending` replay handle; that handle needs the resolution entry points, which Phase 3 builds, so
        the struct is `#[non_exhaustive]` with `slots` alone and Phase 3 adds the field. Adding a placeholder
        field now would have meant a type with nothing to put in it.
    - |-
        Validating a document against an unresolved template returns the existing "could not prepare" failure
        (`ValidationFailure::Unprepared`) whose error is the typed `UnresolvedSlots`. The plan asked for the new
        `SimplifiedSchemaError::Unresolved` from validation too, but validation returns a `ValidationFailure`,
        not a `SimplifiedSchemaError`; adding a new failure variant would ripple through every host that
        matches on it. `to_json_schema`, `layer_under`, and `layer_over` do return `Unresolved`. Reverting
        means adding a `ValidationFailure::Unresolved` variant and updating its matches.
    - |-
        A Markdown document's `$schema`, read by the `SimplifiedSchema` factory, now reports suggestion and slot
        positions in the Markdown document (as a standalone schema file already did); before, a Markdown
        suggestion's position was relative to its type string. No production caller read those positions.
        The alternative was to project only slots, which would leave the two kinds of position in different
        coordinates. Reverting removes one call in `schema/layer.rs::from_frontmatter`.
    - |-
        When a named type is reused with a slot bound or default beside an inherited one (`Name(min({{ x }}))`
        over a type with `min(3)`), both are kept until resolution; the "stricter bound wins" and "defaults must
        agree" merge rules cannot be applied before the slot's value is known. Phase 3's substitution must
        re-apply them after it replaces the slot. The alternative, rejecting a slot beside an inherited bound,
        would forbid a reasonable schema. A slot bound on a reference that turns out to be a string is
        retargeted to a length bound; on a list it stays in the list's constraints.
    - |-
        The subtype check keeps returning yes/no. The plan asked for an "unresolved" result where a template can
        reach a subtype or coercion check; changing `is_subtype`'s return type is a wide public change, and the
        hosts that call it work on resolved schemas. Instead an `enum` or `literal` with a slot is a subtype of
        nothing but itself (its members are not known), which fails closed. This also fixed a hazard: two
        different template enums would otherwise have compared as the same type ("no members" each).
    - |-
        Suggestion queries and union completion gained a `pending` flag (`SuggestionQuery.pending`,
        `AlternativeCompletion.pending`) instead of a separate pending list: the literal elements are offered
        and the flag says the list is incomplete. Each slot itself is available from `pending_slots()` on the
        schema or the type. The structs are public with public fields, so a host building them by hand must
        now set the flag; none does today.
    - |-
        The lower-level `parse_yaml_schema_with_source` still cannot project positions through a `|` or `>`
        block scalar (it never could for suggestions); the factories and standalone loader, which every
        schema file goes through, can, and the block-carrier tests run there. Extending the lower-level scanner
        was left out as unrelated to slots.
    - |-
        The shipped-schema corpus exception for Claudine's sandboxing schema was removed in Phase 2, not Phase 7.
        Once slots parse, that schema passes its passive `kind: schema` admission, and the exception's own guard
        failed the suite with "remove this carve-out"; the spec says to remove it when admission works. The
        schema file itself still has its old `as_csv(...)` slot text: changing it to the `load_data` spelling
        (N40) and the resolved-validation compose test stay in Phase 7. Keeping the exception would have meant
        rewriting its guard to expect success, which is the same as deleting it.
    - |-
        Every slot written in a file now records that file as its origin, not only slots reached through an
        import. The plan said "no origin" means the schema's own file. That breaks for a root union of two schema
        files (each arm's slots would look alike) and for trigger payloads, which must read relative to the trigger
        file. So: a slot from a schema file, an imported type, or a trigger payload names its file; only a slot in a
        document's inline `$schema` or in-memory text has none (it evaluates in the request's own directory). The
        alternative was keeping "none" and carrying an "entry file" on the pending handle, which cannot express
        unions. Reverting means removing the root stamping in `expand_imports` and adding an entry-file field.
        Darkmatter's trigger-failure finding now names the slot's file for a referenced template too.
    - |-
        Resolution re-runs the original call rather than patching a saved copy. The public entry points remember the
        exact text of every schema file the first attempt read, and resolving replays the same call with those
        texts served in place of the files and the slot values substituted right after imports are expanded. This
        follows the plan's "replay" design and adds that the replay never rereads the disk, so the snapshot always
        matches the texts a cache key would hash. Cost: every public resolution keeps a copy of the schema texts it
        reads for the length of the call, even for schemas without slots. The alternative (recording only after a
        template is found) would need a second read and could mix two file versions.
    - |-
        A root union whose arms each have slots is resolved in rounds: the first attempt stops at the first arm with
        slots, so only those are evaluated first, and later arms' slots are evaluated as resolution reaches them.
        The plan expected all slots up front. Each round adds at least one slot, so it always ends, but a failing
        slot in a later arm is reported only after earlier arms' slots were evaluated. The alternative is to make
        the union resolvers collect every arm's slots before stopping; that changes their error handling and can
        be added later without changing the public API.
    - |-
        The host's failure carrier keeps the host's own error and an optional code, with no separate "data location"
        field. The plan sketched a `data_location` field; the library has no type for it without naming a host
        type, so Darkmatter recovers its `DataError` (code and data location intact) with
        `SlotFailure::downcast_ref`. Admission refusals are their own outcome (`SlotResolutionError::Refused`),
        separate from evaluation failures, and every slot is admitted before any runs, so one forbidden slot means
        nothing is evaluated. Adding a field later is additive.
    - |-
        A slot-supplied default is judged against its property by lowering that one property and running the
        library's structural validator on the default; static defaults are still not judged (unchanged). This is
        what "judged against the type after resolution" needs without a second validator implementation. A value
        problem stops at the first bad slot (one `SlotResult` error) rather than collecting all of them.
    - |-
        The full slot documentation lives on the existing `language/slots.md` page that Phase 2 created; the plan named
        `language/types.md`, which now has a short "Values From Slots" section with the position table and links
        to `slots.md`. Keeping one canonical page avoids two copies of the rules.
    - |-
        Date functions read one clock reading per request, with the host's UTC offset fixed at that reading. The
        plan said to read the "captured clock and timezone". The reading is shared by every expression context of
        a request (it rides on the request's captured observations) and seeded from the capture that formats
        `ctx.now`, so `is_today(ctx.today)` is always true. A datetime with an offset from another season is
        converted with today's offset, not with the daylight-saving rule of its own date; that differs only within
        an hour of local midnight across a DST change. The alternative is converting with the IANA zone, which
        needs a timezone database crate. A call with no request context (an in-memory evaluation) reads the clock
        once for that call. Reverting means going back to `chrono::Local` in `functions/dates.rs`.
    - |-
        `has_command`/`has_binary` search the PATH of the request's captured environment, never the process PATH.
        A request whose environment has no PATH (a library caller that built an empty one) finds no bare command
        name; absolute paths still work. This is the plan's rule (Q11); noting it because it can change an answer
        for library callers that never set PATH.
    - |-
        Slot classes: a pure handler is Pure by construction (it has no context to read through, and a source-guard
        test fails if a pure or clock handler file calls the live clock, `std::env::var`, or `which::which`); every
        context handler must declare its class or the function registry refuses to build. The plan also asked for a
        class fact on host functions through `HostFunctionsBuilder` and the facts artifact (with a format-version
        bump). Host handlers receive only their bound arguments, so they are Pure by contract and are classed Pure
        without an artifact change. If a host function ever gets a context, it would need the fact then.
    - |-
        URL pre-fetch discovery now matches function names the way the evaluator dispatches them (case folded, then
        canonical name or alias) instead of folding underscores. A spelling the evaluator would never run (for
        example `markdown_bodyempty`) is no longer treated as a read; `loaddata`, `LOAD_DATA`, and `MarkdownTitle`
        are found up front. The plan said "fold case and underscores exactly as evaluation does"; evaluation does
        not fold underscores, so following evaluation was chosen.
    - |-
        The document read set grows, as the plan intended (Q18): a missing `load_data`/`frontmatter` target lists
        every candidate probed, the path functions list their probes up to the first file found, `has_skill` lists
        the folders it looked in, `find_first_index`/`find_last_index` list the folder they read, and a contract's
        missing import lists every location searched. `find_files` is the exception: its input is a glob pattern,
        not a path, so it is recorded only in the new observation log, never in the read set (whose entries are
        paths and URLs). The alternative was adding each listed folder; `GlobListing` does not report them.
    - |-
        The slot read gate is asked before every content read and every URL fetch, and before the shared
        `load_data` memo, as N28 says. Two kinds of input are checked after the fact instead: the existence checks
        that candidate resolution makes (`stat`, before the gate knows which path the reference resolves to) and a
        data contract's schema imports (read during contract preparation). For both, a function's every
        observation is checked before its result can reach the slot, so a refused input never influences a value.
        Checking them beforehand would mean planning every candidate of every reference before resolution, which
        duplicates biscuit-file's resolver. The refusal type has no list of grant choices yet; Phase 5 adds them.
    - |-
        A slot that fails on a missing or unreadable file reports `dm.schema.slot_failed` with the reference fields
        (`construct`, `reference`, `failure`), not a `dm.data.*` code. The plan's test wording said "a missing file
        keeps `dm.data.*` and `data_location`", but the documented loading-data contract gives a missing file the
        general code and reference fields; there is no data location for a file that was never read. Malformed
        files and contract mismatches do keep their `dm.data.*` code and `data_location`.
    - |-
        A refused slot pattern now has its own result kind (`SlotResultKind::PatternRejected`) in simplified-schema,
        so Darkmatter can report `dm.schema.slot_pattern_rejected`; before, it shared `Inadmissible` with other
        value problems and the two could not be told apart. Reverting means mapping by message text.
    - |-
        Admission reports the first refused part of a slot (the library's admission error holds one span per slot),
        though the walk finds them all. A schema author fixing one refusal may then see the next.
    - |-
        Inside a slot, `ctx.*` values are captured on demand in the slot's own folder (as a `::block when=`
        condition does), while the date functions read the request's clock reading. Around midnight `ctx.today`
        and `is_today(...)` in one slot could disagree by a day. Making the slot's `ctx` capture share the request
        snapshot is session work (Phase 6), where the request snapshot is available.
source_files_during_phase_2:
    - simplified-schema/lib/src/completion.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/expression/scan.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/schema/error.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/layering.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/schema/tests.rs
    - simplified-schema/lib/src/schema/view.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/grammar/slot_tests.rs
    - simplified-schema/lib/src/simplified/lint.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/query.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/subtype.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/slots.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/expression_scanner.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_object.rs
    - simplified-schema/lib/tests/l1/slot_spans.rs
    - simplified-schema/lib/tests/l1/slot_templates.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
docs_updated_during_phase_2:
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/simplified-schema-struct.md
docs_created_during_phase_2:
    - simplified-schema/docs/topics/language/slots.md
skills_files_updated_during_phase_2:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/buffers.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/simplified/definitions.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/slots.rs
    - simplified-schema/lib/src/slots/host.rs
    - simplified-schema/lib/src/slots/pending.rs
    - simplified-schema/lib/src/slots/substitute.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/dynamic_slots.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
docs_updated_during_phase_3:
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/host-capabilities.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/slots.md
    - simplified-schema/docs/topics/language/types.md
    - simplified-schema/docs/topics/simplified-schema-struct.md
    - simplified-schema/docs/topics/validation.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/context/capture/clock.rs
    - darkmatter/lib/src/markdown/compose/context/capture/datetime.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/capture/observations.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/data/load.rs
    - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/observation_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/composition.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/data.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/network.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/shell.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/read_gate.rs
    - darkmatter/lib/src/markdown/compose/expression/read_gate_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/admission.rs
    - darkmatter/lib/src/markdown/schemas/slots/codes.rs
    - darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
    - darkmatter/lib/src/markdown/schemas/slots/host.rs
    - darkmatter/lib/src/markdown/schemas/slots/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/tests.rs
    - darkmatter/lib/tests/l1/load_data_runtime.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/resolve/buffers.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/slots/substitute.rs
    - simplified-schema/lib/tests/l1/dynamic_slots.rs
docs_updated_during_phase_4:
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/topics/compose-requests.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/loading-data.md
    - darkmatter/docs/topics/schemas/index.md
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language/slots.md
docs_created_during_phase_4:
    - darkmatter/docs/topics/schemas/schema-slots.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/requests.md
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
    Phase 4 is complete; see the log's Phase 4 section for the gate results. What Phase 5 needs:
    - Gate: `compose::expression::read_gate::{ReadGate, ReadTarget{function, kind: functions::ReadKind, location:
      ReadLocation::{Local, Remote, Glob}}, GateRefusal::{Denied, ApprovalRequired}}` (`GateRefusal` is
      `#[non_exhaustive]` and carries no grant choices yet; add them). `ResolutionContext::slot_gate` is set only by
      `schemas::slots::evaluator::SlotEvaluator::with_gate`; the default is `DenyAllReads`. Pre-checks run at
      content/fetch sites (Content for `load_data`/`frontmatter` family, Probe for URL `file_exists`); every other
      input reaches the gate through the post-call check in `functions::call_entry` (Probe for existence/missing
      probes, Listing for folders and globs). Probes inside the repository must be automatic (N24) or every path
      function in a slot asks.
    - Host: `schemas::slots::DarkmatterSlotHost::new(SlotEvaluator)` is crate-internal; `host.evaluations()` gives
      each slot's `SlotEvaluation { value, observations, context_digests }` for N21's key/cache.
      `compose::report_diagnostics::slot_resolution_diagnostics(error, text_of)` projects a `SlotResolutionError`
      (use `PendingSchema::read_sources()` texts for `text_of`). Codes: `schemas::slots::codes`.
    - Data runtime: `DataRuntime::{read_mark, observations_since, record_entry, memo_generation}` and the
      thread-local `data::runtime::capture_observations`; `Observation.entry` is a portable path, a URL, or (for
      `Glob`) the pattern as written.
    - The slot evaluator has no remote runtime yet (URL reads fail as not enabled after the gate); N27's slot
      `FetchPolicy` is yours.
    - Carried: the host sweep (Sun/Wed 04:00) can delete target/ mid-build; an outside process may commit the tree.
---

# Implementation Log for 2026-11-05-dynamic-schema-types (9 phases)

## Baseline

Taken 2026-11-08 on macOS (Apple silicon), package-area recipes, serially.

| Area | `just test` | `just lint` |
| --- | --- | --- |
| `simplified-schema` | pass: 1550 (lib) + 1362 (`l1`), 0 skipped | pass |
| `sniff` | pass: 3545, 33 skipped | pass |
| `darkmatter` | pass: 15254, 10 skipped, 1 slow (see below) | pass |

- `simplified-schema` was measured before any Phase 1 edit; `sniff` and
  `darkmatter` after them (a host sweep corrupted the first attempt, see
  Phase 1). The Phase 1 edits to `darkmatter` add tests, fixtures, the N42
  completion fix, and two doc paragraphs, so any later red test outside
  those is attributable to later phases.
- Pre-existing, load-sensitive: `darkmatter-cli::l1
  compose_shell_template::every_suggested_rule_approves_the_command` took
  5.23 s in the full run (load average about 16) and 2.96 s alone.
- Pre-existing, environmental: the macOS linker warns
  `__eh_frame section too large` for `darkmatter-cli`; not a test or lint
  failure.

## Departures

The plan was reconciled on 2026-11-08 with the investigation outcomes and
Rulings Q8–Q20 (see the plan's "Reconciliation" section). The grant-store
location (now Q8) and the breadth staging (now Q16: value positions delivered,
whole types deferred) are rulings, no longer departures.

- **N35, mutation mechanism.** `DataRuntime::invalidate_memo` records no paths,
  so per-path invalidation of a pinned snapshot does not exist to reuse. The
  plan replaces it with a stricter check: a memo-generation counter plus a
  re-hash of the snapshot's recorded observations on each use after the counter
  moves. Grants decided earlier in the run are reused (Q17).

## Decisions applied

Phase rules N1–N42 (with N6a, N12a, N12b) are applied as written unless a later entry here says
otherwise.

## Measurements

None yet. Phase 9 measures the implementation once on one host against the S1
budgets (plan N1); the simulated baseline is in
`investigations/s1-s2-s3-s6-performance.md`.

## Phase 1

### What was done

- **Baseline** recorded above.
- **Fixtures.** Reviewed the first run's files against N4/N6.
  - `simplified-schema/lib/tests/fixtures/dynamic_slots/`: the list-position
    fixtures (`schema-with-slot.yaml`, `suggest-slot.yaml`,
    `imported-types.yaml` with `nested/`, `os.yaml`) match the plan and were
    kept. Added the Q16 value positions: `value-positions.yaml` (one
    whole-argument slot each in `literal`, `min`, `max`, `default`,
    `pattern`) and its data under `values/`; README table extended.
  - `darkmatter/lib/tests/fixtures/dynamic_slots/claudine/` (sandboxing
    shape, `os.yaml`, `provider.md`) was kept. The README named a builder,
    `tests/dynamic_slots_fixtures/mod.rs`, that did not exist; it is now
    written: `SlotRepo::build(source, RepoState)` copies the tree into a
    fresh `git init` repository (fixture identity, no host Git config) with
    the data file `claudine/docs/os.yaml` in one of `Clean`, `DirtyData`,
    `UntrackedData`, `IgnoredData` (committed `.gitignore`), or
    `NoRepository`. Declared in `tests/l1/main.rs`; its own tests
    (`l1/dynamic_slots_fixture_repos.rs`) prove each state with
    `git status --porcelain --ignored` and that the schema-relative slot
    path `../../../os.yaml` names the data file.
- **DMLS enum completion (N42, Q20).**
  - Cause: on a `- ` item line, `union_item_completions` handles only
    union/object item arrays and `block_array_suggestions` only
    `suggest(...)`; a flow `[…]` element consulted only `suggest(...)`.
    Nothing offered the members of a single-arm enum item type, named
    (`os[]`) or inline (`enum(...)[]`). A scalar `os` value already worked.
  - Fix (`dmls/src/providers/frontmatter.rs`): the scalar candidate logic
    of `value_completions` moved into `def_value_completions(def, …)`; a new
    `array_item_value_completions` builds the item definition with the
    existing `array_item_def(array_item_atoms(..))` and calls it. Block
    items (`block_array_suggestions`, renamed `block_array_item_completions`)
    and flow elements fall back to it when there is no `suggest(...)`.
  - Docs: `docs/topics/schemas/dmls-schema-support.md` (enum values bullet)
    and `dmls/docs/autocomplete.md` (new "Items of an array of one scalar
    type" section). Skill: `.claude/skills/darkmatter/dmls.md`.

### Requirement-to-test mapping

| Behavior | Test (boundary, tier) |
| --- | --- |
| Empty `- ` item of named `os[]` offers every member | `darkmatter-lsp providers::frontmatter::enum_item_tests::empty_block_item_of_named_enum_array_offers_every_member` (schema file on disk + overlay + completion provider, F2, L1) |
| Bare `-` item | `…::bare_dash_item_of_named_enum_array_offers_every_member` |
| Partial item filters and its text edit replaces exactly the partial | `…::partial_block_item_filters_members_and_replaces_the_partial` |
| Flow elements `[]`, `[macOS, ]`, `[macOS, W]` | `…::flow_array_elements_offer_members` |
| Inline `enum(fit, fill)[]` | `…::inline_enum_array_item_offers_members` |
| Control: scalar `os` value unchanged | `…::scalar_enum_value_keeps_offering_members` |
| Fixture states | `darkmatter::l1 dynamic_slots_fixture_repos::*` (5 tests, real `git`) |

Regression proof: with the two fallbacks reverted, 5 of the 6
`enum_item_tests` fail (each `left: []`) and the scalar control passes;
with the fix all 6 pass. Targeted commands:
`cargo nextest run -p darkmatter-lsp --lib -E 'test(enum_item_tests)'` and
`cargo nextest run -p darkmatter --test l1 -E 'test(dynamic_slots_fixture_repos) | test(test_layout)'`.

The Input Robustness Matrix does not apply to this phase: no loader or
parser was added (the two loaders it names arrive in Phases 3 and 5).

### Gates

- `just test` and `just lint` in `darkmatter/`, `sniff/`, `simplified-schema/`:
  all pass (counts in Baseline). `just check-tier-coverage darkmatter`:
  0 stranded.
- Not skipped: the scope calculator was not used to skip anything, since
  the phase changes compiled code.
- Dependents: no `darkmatter` library source changed (only its tests); the
  DMLS change is internal to the `darkmatter-lsp` binary crate, which no
  package depends on. No dependent tests were run.
- Cross-OS: not run. The DMLS change has no OS-specific code; the fixture
  builder uses `canonicalize_simplified`, `Path::join`, and Git's `/`
  porcelain paths, and sets `GIT_CONFIG_GLOBAL` to `NUL` on Windows. The
  nightly Windows/WSL2 cells prove it after merge.

### Incidents

- A scheduled host sweep (`~/.local/bin/rusty-biscuit-sweep.sh`, Sunday
  04:00, `cargo sweep --maxsize 120GB`) ran during the first baseline
  attempt and removed 505 GiB from this worktree's `target/` mid-build;
  `sniff`'s test build and lint failed with missing `build/*/output` and
  `.rmeta` files. The attempt was stopped, the sweep allowed to finish
  (not interrupted: it is not this session's process), and every gate
  rerun from scratch.
- While this phase ran, a process outside this session committed the
  working tree several times (`dffd6b0e8`, `2fe27e5ea`, `705fa8f55`,
  `4c8fa60cd`, by the author identity). This session staged and committed
  nothing; the committed content is this phase's work.

## Phase 2

### What was done

All in `simplified-schema` except one match arm in Darkmatter.

- **AST (N3).** `Slot`, `DynamicKind`, `DynamicElement`, `DynamicList`,
  `ValueSlotKind`, `Constraint::Dynamic`, `Constraint::DynamicValue` in
  `simplified/types.rs`, re-exported from `simplified` and the crate root.
  `Constraint::slots()`, `PropertyAtom::{suggestion_candidates,
  has_pending_suggestions, has_pending_values}`. `keyword()` for `Dynamic`
  is `"{{"` for a leading slot, else `"<members>"`/`"suggest"` (it returns
  `&'static str`, so "first element's authored text" is approximated).
- **Scanner (N5).** `expression::scan::scan_expression_at`, the one closing
  rule, used by the grammar (`read_slot`), the grammar's own
  `argument_list_end`/`find_top_level`, and `source.rs::scan_expression`
  (argument lists skip a slot whole, so `split_top_level`,
  `scan_constraint_groups`, `constraint_call_arguments` read it as one
  argument).
- **Grammar (N4, N7).** `enum(...)` members and `suggest(...)` arguments take
  a slot as a complete element; `literal`, `default`, `min`, `max`, `pattern`
  take a whole-argument slot. `Dynamic`/`DynamicValue` are built only when a
  slot is present. Positioned errors: complete element, whole argument,
  unterminated slot, empty slot, expression parse error (offset mapped into
  the expression). One-`suggest`-per-definition and the exact-type rule hold
  for dynamic lists; a dynamic suggest list keeps literal duplicates (N7
  dedupe happens at substitution). `null(default({{…}}))` and value slots on
  `type-definition`/`schema` are rejected.
- **Deferred references.** `reinterpret_dynamic_suggestions` retargets a
  reference's dynamic suggest list; resolve.rs's suggest checks use
  `is_suggestion_list`/`suggestion_list_span`.
- **Serialize.** Literals keep their quoting; slots are written back as
  `{{ expr }}`; export → parse → export is stable.
- **Spans.** `project_atom_suggestions` projects slot `span`/`expression_span`
  and dynamic candidate spans in both value and array lists. The Markdown
  frontmatter factory path now projects candidate and slot spans into the
  document (`project_keyed_declaration_spans`), as the standalone path did.
- **Lowering (N10, passive half).** `SchemaError::UnresolvedSlots(Box<
  UnresolvedSlots { slots }>)` (Display + terminal status block).
  `to_json_schema_with_definitions` rejects a template before lowering
  (`slots::reject_unresolved`); `atom_fragment_without_null_wrap` rejects an
  atom with slots, so any other route into lowering is closed too.
  New module `slots.rs`: `collect_slots`, `has_dynamic`, `definition_slots`,
  `reachable_slots`.
- **Object (N13, passive half).** `is_resolved()`, `pending_slots()`,
  `SimplifiedSchemaError::Unresolved`; `to_json_schema`, `layer_under`,
  `layer_over` return it; validation returns `ValidationFailure::Unprepared`
  holding `UnresolvedSlots`; a whole-file `$schema` reference or root-union
  file arm that reaches a template is kept as an unlowered layer.
  `TypeDefinition::pending_slots()`.
- **Consumers.** `SuggestionQuery.pending` and `AlternativeCompletion.pending`
  (literal elements offered, list flagged incomplete); lint judges only
  literal candidates, and none when a value slot shapes the target;
  `is_subtype` gives an unresolved enum/literal no identity (it was about to
  compare `enum({{a}})` and `enum({{b}})` equal as "no members");
  `format.rs` `forms(enum)` admits a template's literal members; narrowing
  markers; recursive-definition use sites with a slot are always inlined;
  `is_array_container_constraint` and `reinterpret_bounds` know value slots;
  `has_default` counts a slot default.
- **Darkmatter.** `triggers/finding.rs::related_path` classifies the new
  `SchemaError` variant (names an imported slot's defining file). Trigger
  match conditions already exclude slots (`is_match_safe_constraint` is an
  allow-list).
- **Docs.** New `simplified-schema/docs/topics/language/slots.md` (resolution
  marked **planned**); `language/index.md`, `simplified-schema-struct.md`,
  `diagnostics.md`, `language-core.md` updated. Skills:
  `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`.

### Requirement-to-test mapping

Boundary for all rows: the `simplified-schema` crate's public API, in-process
(F1 for the grammar unit tests, F2 for factory and file-backed tests over the
shipped fixtures in `tests/fixtures/dynamic_slots/`), tier L1.

| Behavior | Test |
| --- | --- |
| `scan_expression_at`: same end as the whole-text scan; quoted `}}`/`{{`, nested braces; unclosed, empty, `{{{`, literal token, wrong byte; escaped opener | `l1 expression_scanner::scan_expression_at_*` (4) + doctest |
| enum slot with literals and `; required`/`default` tail; mixed order; multiple slots; `||` inside a slot in a union and array | `lib simplified::grammar::slot_tests::{an_enum_slot_mixes…, slots_and_literals_alternate…, a_slot_inside_a_union…}` |
| static lists stay `Members`/`Suggest`/`LiteralValue` (static parity) | `slot_tests::a_static_list_never_builds_a_dynamic_constraint`, `l1 declaration_grammar::slots::a_static_list_keeps_its_static_constraint`, `schema_object::templates::a_static_schema_is_resolved_and_lowers_its_lists_as_before` |
| quoted `'{{ x }}'` stays literal; `abc{{x}}` keeps today's behavior; data-shaped literal beside a slot | `slot_tests::a_brace_that_does_not_open_an_element_is_text`, `declaration_grammar::slots::data_shaped_text_beside_a_slot_stays_literal` |
| complete-element, unterminated, empty, parse-error-offset errors | `slot_tests::{a_slot_must_be_a_complete_list_element, an_unterminated_or_empty_slot…, a_slot_expression_parse_error…}`, `declaration_grammar::slots::slot_errors_are_located_in_the_expression` |
| suggest slots: target, deferred reference, duplicates kept only when dynamic, one list per definition, exact-type rule | `slot_tests::{a_suggestion_slot_keeps…, duplicate_literals…, a_definition_still_holds_one…, suggestion_slots_follow_the_exact_type_rule}` |
| value positions: literal/min/max/default/pattern, array-level, reference target; whole-argument errors; `null` default, `type-definition` | `slot_tests::{value_positions_take…, a_value_slot_must_be_the_whole_argument, value_slots_keep_the_type_rules…}` |
| serialize round trip, literal quoting kept | `declaration_grammar::slots::templates_round_trip…`, `schema_object::templates::{export_keeps_every_slot…, export_keeps_literal_quoting…}` (all three list fixtures + value-positions fixture) |
| spans through plain, `'…'`, `"…"`, `\|`, `>`, CRLF, frontmatter offset, Markdown factory, imported defining file; slot as one argument in the source map | `l1 slot_spans::*` (8) |
| factories admit templates; `is_resolved`/`pending_slots`; `to_json_schema` → `Unresolved`; validation → `Unprepared(UnresolvedSlots)`; layering either side fails atomically | `schema_object::templates::*` (7), `slot_templates::validating_against_a_template_never_reports_success` |
| query/suggestion/lint/completion/subtype on templates | `l1 slot_templates::*` (6) |
| parse, query, export evaluate nothing and read nothing (counters + missing data files + provenance) | `lib schema::tests::context_boundary::a_template_reads_and_evaluates_nothing_but_its_schema_files` |

The Input Robustness Matrix does not apply: the change is to the
type-expression text grammar; no file-format or configuration loader was
added or changed. The loader code-smell grep over the new code found one
`.ok()?` on `str::from_utf8` of a `&str`'s bytes (cannot fail; not a
load-bearing parse).

### Gates

Run on macOS after all edits, with no subagent running (this phase used none).

- `simplified-schema`: `just test` pass (1596 with `work-counters,terminal`;
  1406 in the no-`file` minimal build), `just lint` pass (clippy with
  features and `--no-default-features`), doctests pass (41, including the
  new `scan_expression_at` and `is_resolved` examples),
  `just check-tier-coverage simplified-schema`: 0 stranded.
- `darkmatter`: `just test` pass (15254 passed, 10 skipped), `just lint`
  pass, `just check-tier-coverage darkmatter`: 0 stranded. On the first full
  run `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry`
  failed on its own carve-out guard ("…validates now…; remove this
  carve-out"); the carve-out was removed (see decisions) and the rerun was green.
  `darkmatter-cli::l1 compose_shell_template::every_suggested_rule_approves_the_command`
  ran 5.08 s in one full run: the pre-existing load-sensitive test from the
  baseline, not touched here, and under 5 s on the final run.
- Dependents (compile): `cargo check --all-targets -p darkmatter
  -p darkmatter-cli -p darkmatter-lsp -p claudine -p claudine-cli -p messenger`
  passes.
- Dependents (targeted tests, serially; each package has more than 200
  tests, so only tests reading schemas through the changed symbols ran):
  `claudine` `test(composition::schema) | test(claudine_schemas) |
  test(container_schema_recursion) | test(host_catalog)` 999 passed;
  `claudine-cli` `test(schema_completion)` 53 passed; `claudine-gen`
  `test(schema_compat) | test(model_config_research)` 3 passed;
  `messenger` `test(research)` 1 passed.
- Gate skipping: not used; this phase changed compiled code, so
  `affected_scope.py` was not consulted to skip anything.
- Cross-OS: not run. The change adds no path or OS-specific code; the one
  file-backed test joins `CARGO_MANIFEST_DIR` with `Path::join`, and the CRLF
  span case runs on every OS. The nightly Windows/WSL2 cells cover it.

## Phase 3

### What was done

All in `simplified-schema`, plus one match arm (and its comment) in
Darkmatter's `triggers/finding.rs`.

- **`slots` module (N8, N9, N11, N6a, N12b).** `slots.rs` gained submodules:
  `slots/substitute.rs` (`SlotKey`, `SlotValues`, `with_values` /
  `active_values` thread-local scope, `apply_active`, `substitute`,
  `check_slot_pattern`, `MAX_SLOT_PATTERN_BYTES`), `slots/host.rs`
  (`SlotHost` with `admit`/`resolve`/`is_canceled`, `SlotRequest`,
  `SlotRunRequest`, `SlotAdmissionError`, `SlotFailure`,
  `SlotResolutionError::{Refused, Failed, Schema, Canceled}`), and
  `slots/pending.rs` (`PendingSchema` with `slots()`, `template()`,
  `read_sources()`, `resolve()`, plus the shared `resolve_with` evaluation
  loop). New error `SchemaError::SlotResult { slot, kind: SlotResultKind }`
  (`Empty`, `WrongShape`, `Inadmissible`), with Display and a terminal status
  block. `UnresolvedSlots` gained `pending: Option<PendingSchema>` (and a
  crate-private expanded template).
- **Substitution rules.** Arrays flattened one level, `[]` adds nothing, an
  empty final list is `Empty` at the first slot, first occurrence kept
  across literals and slots; enum elements strings only; suggest elements
  any scalar (numbers via `numeric::format_number`, then the literal
  candidate interpreter `interpret_suggestion_parts`, now `pub(crate)`);
  `literal` scalar with type kept; `default` any non-null value, then
  judged by lowering its property and validating with the structural
  validator; `min`/`max` numbers through `numeric::admit_json_number`,
  counts (string length, array items) non-negative integers; `pattern` a
  string passing the cap (lookaround, backreference, > 4096 bytes, or
  `regex::RegexBuilder` failure at its default size limit). After a list is
  substituted, the merge rules a slot skipped are re-applied: stricter bound
  wins, a lower bound above its upper is refused at the later slot, a second
  default must be equal.
- **Buffer bytes API (N18).** `resolve::buffers::read` is public. It also
  records each text it serves into a running `recording` scope.
- **`resolve.rs` integration (N10, N12).** Origin stamping: `expand_imports`
  stamps the root's own slots with the root namespace's file
  (`DefNamespace::file()`, new), and `rehome_definition` (renamed from
  `rehome_definition_examples`) stamps each lifted definition's slots with
  its file. `apply_active` runs after import expansion at the inline-mapping,
  root-union, standalone, and standalone-root-union lowering sites; the
  embedded site refuses any slot. `resolve_yaml_schema_owned_by` (which
  `resolve_schema`, `resolve_schema_with_roots`, `resolve_yaml_schema`, and
  `resolve_yaml_schema_with_roots` delegate to) and
  `prepare_standalone_yaml_schema` (and so `prepare_yaml_schema_text`) run
  through `resolving_entry`, which records the texts read and, only on an
  `UnresolvedSlots` error, attaches a `PendingSchema` whose replay clones the
  arguments. Nested entry calls and calls inside a slot-value scope pass
  through. `resolve_catalog_declaration`, lowering alone, and embedded
  preparation keep `pending: None`.
- **`SimplifiedSchema::resolve_slots` (N13).** A template layer keeps a
  `Rebuild` (its constructor, its text copied only when the layer is a
  template, its path and binding, and the buffers it read); `resolve_slots`
  reruns it inside `with_values` and returns a new schema; `select` on a
  template composes the rebuild. `lower_expanded` in `layer.rs` substitutes
  before lowering.
- **Darkmatter.** `triggers/finding.rs::related_path` classifies
  `SchemaError::SlotResult` (its slot's file).
- **Docs.** `simplified-schema/docs/topics/language/slots.md` (resolution
  documented; the "planned" marker removed for it; result shapes, pattern
  cap, where a slot evaluates, failures; whole types marked **planned**),
  `language/types.md` ("Values From Slots"), `language/index.md`,
  `simplified-schema-struct.md`, `host-capabilities.md`, `validation.md`,
  `diagnostics.md`. Skills: `.claude/skills/simplified-schema/SKILL.md`,
  `.claude/skills/darkmatter/schema.md`.

### Requirement-to-test mapping

Boundary: the `simplified-schema` crate's public API, in-process, with a
stub `SlotHost` (F2: real fixture files and the real resolver; the stub
replaces only expression evaluation, which this crate never does). Tier L1.
All in `simplified-schema/lib/tests/l1/dynamic_slots.rs` unless noted.

| Behavior | Test |
| --- | --- |
| Matrix 1 control row (shipped fixture → macOS, Linux, Windows; repeated literal once; `; required` kept; slot evaluated in the schema file) | `control::the_enum_fixture_resolves_to_its_data_with_the_repeated_literal_once`, `result_shapes::the_control_row_resolves` |
| Matrix 1 cells: null, scalar string, number, mapping | `result_shapes::a_list_slot_takes_only_an_array` |
| nested array element | `result_shapes::an_array_is_flattened_one_level_and_a_nested_array_is_refused` |
| `[]` adds nothing; empty final list is `Empty` | `result_shapes::an_empty_result_adds_nothing_and_an_empty_list_is_its_own_error` |
| enum element number / every element number / boolean / null | `result_shapes::enum_members_must_each_be_strings` |
| suggest scalars accepted; `["a", {}]` refused | `result_shapes::suggestions_take_every_scalar_and_refuse_a_mapping_element`, `a_numeric_suggestion_list_reads_numbers_by_their_canonical_text` |
| dedupe `["a","a"]`, across two slots and a literal | `result_shapes::the_first_occurrence_of_each_value_is_kept` |
| data with `,` `;` `)` `{{` stays one value | `result_shapes::data_holding_list_syntax_stays_one_value` |
| suggest fixture order | `control::the_suggestion_fixture_keeps_literals_and_data_in_order` |
| value positions, valid + invalid each (literal, default, min/max incl. counts, pattern) | `value_positions::*` (6), `control::every_value_position_takes_its_shape` |
| `min > max` at the later slot | `value_positions::a_lower_bound_above_the_upper_is_refused_at_the_later_slot` |
| pattern cap: lookaround, backreference, > 4 KiB, size limit; authored lookaround unchanged | `value_positions::pattern_takes_a_string_the_linear_engine_compiles`; unit `slots::substitute::tests::a_slot_pattern_is_refused_…` |
| stricter inherited bound / equal default merge after substitution | `value_positions::a_slot_bound_beside_an_inherited_one_keeps_the_stricter` |
| admission first, nothing evaluated on refusal | `host_contract::a_refused_slot_stops_the_resolution_before_anything_is_evaluated` |
| every failure attempted; typed cause downcasts | `host_contract::every_failing_slot_is_attempted_and_keeps_its_typed_cause` |
| cancellation publishes nothing; template still resolves | `host_contract::cancellation_publishes_nothing_and_the_template_still_resolves` |
| repeated resolves independent; template kept | `host_contract::repeated_resolutions_start_from_the_template` |
| a named-type slot used twice evaluated once | `host_contract::a_slot_in_a_named_type_used_twice_is_evaluated_once` |
| root-union later arms reached | `host_contract::slots_in_later_root_union_arms_are_reached_and_resolved` |
| imported slot evaluates in its defining file (N12) | `control::a_slot_in_an_imported_type_evaluates_in_its_defining_file` |
| trigger payload slot's origin is the trigger file; inline document slot has none | `entry_points::a_trigger_payload_slot_evaluates_in_the_trigger_file`, `only_entry_points_carry_a_handle_…` |
| recursive named type with a slot | `entry_points::a_recursive_named_type_with_a_slot_resolves_into_its_definition` |
| `resolve_slots` new concrete schema, template unchanged, failure publishes nothing | `entry_points::resolve_slots_returns_a_new_concrete_schema_and_leaves_the_template`, doctest on `resolve_slots` |
| layering after resolution equals layering the static twin; whole-property precedence | `entry_points::layering_a_resolved_template_equals_layering_its_static_twin` |
| imports/examples/referenced files and JSON identical to the static twin | `entry_points::a_template_resolves_with_the_same_edges_as_its_static_twin` |
| only entry points carry `pending`; catalog/lowering do not | `entry_points::only_entry_points_carry_a_handle_and_lowering_alone_never_does` |
| embedded schema refuses slots | `entry_points::an_embedded_schema_refuses_slots` |
| N12b: `read_sources` lists exact texts; replay never rereads the disk | `control::resolution_reads_the_recorded_schema_texts_never_the_disk_again`; unit `resolve::buffers::tests::a_recording_scope_lists_every_text_read_once_in_order` |
| N18 `buffers::read` inside/outside `with` | `buffer_reads::read_serves_an_open_buffer_inside_with_and_the_disk_outside` |
| no host named in the slot modules | `the_slot_modules_name_no_host`; `dependency_direction_guard` unchanged and passing |

Input Robustness Matrix 1 is the slot result validator above (default:
located `SlotResult`, nothing published; exceptions `[]` and suggestion
scalars; `null` never conflated with `[]`). Darkmatter's per-kind wiring
rows belong to Phase 4. Code-smell grep over `src/slots.rs` and
`src/slots/*.rs` (`serde(default)`, `unwrap_or_default()`,
`filter_map(..as_str)`, `.ok()`): one `unwrap_or_default()` in
`buffers::recording` (an empty frame list cannot happen while the frame is
pushed; not a load-bearing parse), a `filter_map` in `resolve_with` that
collects admission refusals (every refusal is kept), and
`json_number(..).map_or(Value::Null, …)` in `admitted_number`, unreachable
because admission already rejected a non-finite number.

### Incidents

- A `just lint` was started from the repository root by mistake (a wrong
  relative `cd`); it was stopped after a few minutes and rerun in
  `darkmatter/`.

### Gates

Run on macOS after all edits, with no subagent running (this phase used none).

- `simplified-schema`: `just test` pass (1633 with `work-counters,terminal`;
  1435 in the no-`file` build), doctests pass (39 run, 3 ignored, including
  the new `resolve_slots` example), `just lint` pass after one fix
  (`clippy::type_complexity` on the recording thread-local; a `ReadSources`
  alias), `just check-tier-coverage simplified-schema`: 0 stranded.
- `darkmatter`: `just lint` pass, `just test` pass (15254 passed, 10
  skipped), `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: `cargo check --all-targets -p claudine -p claudine-cli
  -p messenger` passes. Targeted tests, serially (each package has more than
  200 tests; the same schema-reading filters Phase 2 used): `claudine` 999
  passed, `claudine-cli` `test(schema_completion)` 53 passed, `claudine-gen`
  3 passed, `messenger` `test(research)` 1 passed.
- Gate skipping: not applicable. `affected_scope.py` over the working tree
  reports `packages: [darkmatter, simplified-schema, test-toolkit,
  darkmatter-cli]`, `change_class: package`, so both gates ran.
- Cross-OS: not run. The new code compares no paths by text: origins are the
  resolver's existing canonical paths (`canonical_path` →
  `canonicalize_simplified`), and the tests build expected paths with
  `biscuit_file::canonicalize_simplified`. The nightly Windows/WSL2 cells
  cover it.
- No failing or skipped tests were found in this phase.

## Phase 4

### What was done

All in the `darkmatter` library, plus four small `simplified-schema` changes
(listed last). Done serially in one session (no subagents): almost every
task touches `functions/mod.rs` and `functions/fs.rs`, so parallel agents
would have collided.

- **Registration facts (N20).** `FunctionBinding` carries a required
  `SlotClass::{Pure, InputReading, Captured, Volatile, Rejected}` and, for a
  reader, `ReadFact { kind: Content | Probe | Listing, remote }`
  (`.slot(..)`, `.reads(..)`, `.reads_or_fetches(..)`). `pure(..)` is `Pure`
  by construction, `lazy`/`callback` are `Pure` (their operands decide), and
  a `context(..)` handler without a class makes the registry refuse to build
  (`RegistryError::UndeclaredSlotClass`; `InconsistentReadFact` for a reader
  fact on a non-reader). Values are the S4 table: 28 input-reading, the 14
  date functions plus `has_command`/`has_binary`, `package`, `package_area`,
  `ipv4`/`ipv6`, `has_agentic_cli` captured, 4 git functions volatile, the 11
  Q10 functions rejected. `read_fact(name)`/`slot_class(name)` resolve as
  dispatch does (a test or host function shadows the built-in and is Pure).
- **Captured clock and `PATH` (N20, Q11).** New
  `compose/context/capture/clock.rs` (`ClockReading`, `CapturedClock`): one
  reading per snapshot, seeded by the capture that formats `ctx.now`
  (`populate_datetime` takes it), kept across later group captures through
  `DocumentSeed::clock`, shared by every `ResolutionContext` clone through
  `CapturedObservations::clock()`. The 14 date functions are a new
  `FunctionBinding::clock` handler kind (`ClockFn`) and convert offsets with
  the captured fixed offset (`parse_date_or_datetime(s, zone)` and the
  `is_*_with(value, today, zone)` helpers now take a `FixedOffset`).
  `has_command`/`has_binary` search the captured environment's `PATH`
  (`which::which_in`).
- **Remote-read discovery (Q18).** `REMOTE_READ_FUNCTIONS` is gone;
  `remote.rs::fetches_url` reads `ReadFact::remote` through dispatch-style
  name resolution, so `loaddata(...)`/`LOAD_DATA(...)` are pre-fetched and
  `calls_read_side_function` agrees with evaluation.
- **Observation log (N17, N18, N22, N35-generation).** `DataRuntime` keeps an
  ordered `Observation { entry, kind: File|Url|Exists|MissingProbe|Listing|Glob,
  hash }` log with `read_mark`/`observations_since`, `record_entry` (cache
  replay), and `memo_generation`. `LoadedSource::hash` keeps the BLAKE3 of the
  bytes read, so a memo hit observes again; contract observations are kept
  per memo key (stored inside the single-flight closure so a waiting caller
  sees them too). Every input-reading family observes: `load_data` (missing
  probe, hash, `None` on failure; bytes through `buffers::read`), the
  frontmatter family (same, also through `buffers::read`), `file_exists`, the
  path functions (`observe_reference_probes`), `has_skill`, `find_*_index`
  (folder listing), `find_files` (glob; observation only), contract imports
  and `validate_schema`'s schema files (texts from `buffers::recording`;
  missing import locations included). A thread-local
  `capture_observations` frame gives exact per-call slices.
- **Read gate (N28).** New `compose/expression/read_gate.rs` (`ReadGate`,
  `ReadTarget`, `ReadLocation`, `GateRefusal`), `ResolutionContext::slot_gate`,
  `authorize_slot_read` at the content/fetch sites (before the read is
  recorded and before the `load_data` memo), and a post-call check in
  `functions::call_entry` over every observation of a `reads`-registered
  function before its value returns (an input the gate cannot name is
  refused). New `ExpressionError::SlotReadRefused`; `try_frontmatter`
  re-raises it. With no gate nothing runs differently.
- **Admission, codes, evaluator, host (N14, N15, N16, N17, N19, N37).** New
  `schemas/slots/`: `codes.rs` (15 `dm.schema.slot_*` codes,
  `SLOT_DIAGNOSTIC_CODES`), `admission.rs` (`slot_class_report`: passive walk
  of every branch and callback, the class join, and the refused bindings and
  calls), `evaluator.rs` (`SlotEvaluator`: the slot's derived context
  `request.for_source(defining_file)`, the command's `DataRuntime`, a fresh
  default `IterationBudget`, a `ReadGate` defaulting to `DenyAllReads`;
  returns the value, its observations, and context digests for each captured
  `ctx` group, `env.NAME`, and the whole environment), and `host.rs`
  (`DarkmatterSlotHost` implementing `admit`/`resolve`; a failure is a
  `SlotFailure` wrapping the `ExpressionError` with `failure_code`).
  `compose::report_diagnostics::slot_resolution_diagnostics` positions every
  refused or failed slot at its authored position in its defining file and
  keeps `data_location` and reference fields (N19).
- **simplified-schema.** `SlotResultKind::PatternRejected` (a refused slot
  pattern was `Inadmissible`, so its code could not be told apart);
  `resolve::buffers::recording` and `ReadSources` are public; the drifted
  `Slot::origin` doc (Phase 3 changed stamping) now says every file-written
  slot is stamped.
- **Docs.** New `darkmatter/docs/topics/schemas/schema-slots.md` (linked from
  `schemas/index.md`; approval, cache, and surfaces marked **planned**);
  `darkmatter-expressions.md` (one clock reading per composition, captured
  `PATH`); the read-set lists in `compose-requests.md` (plus an
  "Observations" section), `loading-data.md`, `cli/data-contracts.md`;
  `simplified-schema/docs/topics/diagnostics.md`, `language/slots.md`
  (`PatternRejected`). Skills: `.claude/skills/darkmatter/{schema,expressions,requests}.md`.

### Departures

- N35's "per-path recording does not exist" is now partly untrue: the
  observation log records paths and hashes; `memo_generation` is still the
  trigger for re-verification, as planned.
- The gate is asked before every content read and fetch, but candidate
  resolution's existence checks (`stat`) and a contract's import schema reads
  happen before the gate is asked about them; their results are checked
  before any value reaches the slot (the post-call check). See
  `decisions_for_review`.

### Requirement-to-test mapping

Boundary: Darkmatter's crate-internal expression dispatch and the
`simplified-schema` public resolver driven through `DarkmatterSlotHost`
(F2: real schema and data files in temp dirs, the real resolver, the real
expression engine; nothing is stubbed). Tier L1. No CLI surface resolves
slots yet, so no spawn test exists in this phase.

| Behavior | Test |
| --- | --- |
| every binding declares a class; S4 table; 28 readers | `functions::registration_tests::every_binding_declares_its_slot_class` |
| no pure/clock handler reads the live clock, env, or `which` | `functions::registration_tests::no_handler_source_reads_the_live_clock_or_process_state` |
| date functions read the captured reading (local vs UTC date, optional year, offset conversion) | `functions::dates::tests::every_date_function_reads_the_requests_captured_clock` |
| two evaluations in one request see one clock | `functions::dates::tests::two_evaluations_in_one_request_see_one_clock`, `context::capture::clock::tests::every_clone_sees_the_first_reading` |
| `ctx.now` and the date functions share one reading; a later capture keeps it | `context::capture::tests::the_captured_date_values_and_the_date_functions_share_one_reading`, `…::a_later_capture_of_a_snapshot_keeps_its_clock_reading` |
| `has_command`/`has_binary` ignore the process `PATH` | `functions::fs_tests::…::has_command_searches_the_captured_path_not_the_process_path` |
| `loaddata("https://…")`, `LOAD_DATA`, `MarkdownTitle` discovered; `not_frontmatter`, `absolute`, `find_files` not | `compose::remote::tests::discovery_folds_case_and_aliases_as_evaluation_does`, `…::a_read_side_call_is_recognized_by_its_dispatched_name` |
| memo hit re-observes the same hash | `data::observation_tests::a_memo_hit_observes_the_same_read_again` |
| failed read keeps its dependency (`hash: None`) | `…::a_failed_read_keeps_its_dependency_without_a_hash` |
| `load_data` not-found probe recorded | `…::a_missing_data_file_records_the_candidates_it_probed` |
| missing contract import recorded | `…::a_missing_contract_import_is_a_dependency`; l1 `load_data_runtime::a_failed_contract_keeps_every_import_read_before_the_failure` (updated expectation) |
| frontmatter family, path functions, skills, listings, glob each observe | `…::a_frontmatter_read_observes_…`, `…::a_path_function_observes_…`, `…::a_skill_probe_observes_…`, `…::listings_observe_…_and_a_glob_stays_out_of_the_read_set` |
| generation grows on `invalidate_memo`; `record_entry` replays | `…::the_memo_generation_grows_on_each_invalidation`, `…::a_recorded_entry_is_replayed_into_the_read_set_as_written` |
| document `load_data` results unchanged | the existing `load_data_*` L1 suites (508 of 509 targeted pass unchanged; the one change is the missing-import row above) |
| every input-reading function asks the gate; refusal typed | `expression::read_gate_tests::every_input_reading_function_asks_the_gate_and_a_refusal_is_typed`, guarded by `the_call_table_covers_every_input_reading_function` |
| content read asks before reading; nothing read or recorded | `…::a_content_read_asks_before_reading_and_a_refusal_reads_nothing` |
| a file the document already loaded is still gated | `…::a_file_the_document_already_loaded_is_still_gated_for_the_slot` |
| refusal never recovered (`try_frontmatter`) | `…::a_refused_read_is_never_recovered_into_a_value` |
| URL asked before any fetch | `…::a_remote_read_asks_before_any_fetch` |
| `None`/allowing gate changes no answer; pure functions never ask | `…::an_allowing_gate_changes_no_answer_and_pure_functions_never_ask` (and every existing suite) |
| admission: `fm.x`, `fm['x']`, `title`, host global, `ctx.cwd/self/last_updated/hash/id/sid`, bare/computed `ctx`, `current.x`/`local.x` outside a callback, positioned at the read | `schemas::slots::tests::admission::document_bindings_and_identity_are_refused_at_their_read`, `…::a_computed_ctx_key_is_refused` |
| each Q10 function refused at its call (case folded) | `…::every_forbidden_function_is_refused_at_its_call` |
| forbidden in an untaken ternary branch, a `||` operand, `&&`, a callback | `…::a_forbidden_read_or_call_in_a_branch_that_would_not_run_is_still_refused` |
| allowed `ctx.today`, `env.HOME`, `current.*`/`local.*` inside a callback, readers, volatile | `…::the_schemas_own_context_is_admitted` |
| syntax refusal positioned | `…::a_slot_that_does_not_parse_is_a_positioned_syntax_refusal` |
| class join over every branch | `…::the_class_report_joins_every_reachable_call` |
| `load_data("../os.yaml","string[]")` resolves; relative to the schema file (decoy beside it ignored); observation hashed | `…::evaluation::a_load_data_slot_resolves_to_its_data_relative_to_the_schema_file` |
| slot in an imported schema reads relative to its own file | `…::a_slot_in_an_imported_schema_reads_relative_to_its_own_file` |
| default gate refuses (`dm.schema.slot_approval_required`), reads nothing | `…::without_a_grant_the_default_gate_refuses_the_read_and_reads_nothing` |
| the command's read set gains a slot's reads | `…::the_command_read_set_gains_what_a_slot_read` |
| work limit trips `dm.iteration.visit_limit` | `…::a_work_limit_stops_the_slot_with_the_iteration_code` |
| a date function in a slot reads the request's clock | `…::a_date_function_in_a_slot_reads_the_requests_captured_clock` |
| context digests (ctx group, `env.NAME`, unset env), no value text | `…::context_inputs_are_kept_as_digests` |
| malformed file keeps `dm.data.malformed` + `data_location` line/column; primary position at the slot in its file | `…::failures::a_malformed_data_file_keeps_its_data_code_and_location_and_is_positioned_at_the_slot` |
| missing file keeps reference fields | `…::a_missing_data_file_keeps_its_reference_fields` |
| refused binding positioned at the read; nothing evaluated | `…::a_refused_binding_is_positioned_at_the_read_and_nothing_is_evaluated` |
| wrong shape, empty list, rejected pattern codes | `…::a_wrong_shape_and_an_empty_list_get_their_slot_codes` |
| position without text stays null | `…::a_position_without_text_stays_null` |
| Matrix 1 wiring rows: control + one invalid shape per slot kind (enum, suggest, literal, default, min/max, pattern) | `…::positions::each_position_takes_its_control_value_and_refuses_one_invalid_shape` |
| slot codes are 15 unique three-segment `dm.schema.slot_*` codes | `schemas::slots::codes::tests::every_code_is_a_unique_three_segment_schema_code` |
| `PatternRejected` for every cap reason | simplified-schema `dynamic_slots::value_positions::pattern_takes_a_string_the_linear_engine_compiles` (updated) |

Input Robustness Matrix 1 (slot result validator) is a library loader;
Darkmatter's per-kind wiring rows are the `positions` test above. Matrix 2
(grant store) is Phase 5. Loader smell grep over the new files
(`schemas/slots/`, `read_gate.rs`, `data/runtime.rs`, `clock.rs`): two hits
fixed (a URL observation that did not parse would have been skipped by the
gate: now refused; a contract memo waiter could miss the stored
observations: now stored inside the single-flight closure). The remaining
hits are not load-bearing parses: `observations_since` past the end (empty),
the capture frame pop (always pushed), and serializing a `BTreeMap` of
strings (cannot fail).

### Gates

Run on macOS after all edits, with nothing else running (this phase used no
subagents).

- `darkmatter`: `just test` pass (15303 passed, 10 skipped), `just lint`
  pass (first run caught two methods only tests used; one removed, one now
  covered by a test), `just check-tier-coverage darkmatter`: 0 stranded.
- `simplified-schema`: `just test` pass (1633, plus 1435 in the no-`file`
  build), `just lint` pass, `just check-tier-coverage simplified-schema`:
  0 stranded.
- Dependents: `cargo check --all-targets -p claudine -p claudine-cli
  -p messenger` passes. Targeted tests, one package at a time (each has more
  than 200 tests): `claudine` `test(/load_data|read_set|has_command|date|schema/)`
  1218 passed; `claudine-cli` `test(/schema_completion|load_data|compose/)`
  728 passed; `messenger` `test(/research|compose/)` 1 passed.
- Gate skipping: not applicable. `affected_scope.py` over the working tree
  reports `packages: [darkmatter, simplified-schema, test-toolkit,
  darkmatter-cli, darkmatter-lsp]`, `change_class: package` (the tree still
  holds Phase 3's uncommitted changes too), so the gates ran.
- Cross-OS: not run. Path handling in the new code goes through
  `DataSourceIdentity::local` / `biscuit_file::canonicalize_simplified`, and
  the tests build expected paths the same way. The `has_command` test takes
  its `PATH` folder from `which::which` on the host, so it holds where
  `PATHEXT` applies. The nightly Windows and WSL2 cells cover the rest.
- No pre-existing, skipped, or load-sensitive failures were seen.
