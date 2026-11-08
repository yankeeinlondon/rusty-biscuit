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
    - sniff
    - darkmatter-cli
    - claudine-cli
    - claudine
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
    - |-
        Sniff's per-path answer has a seventh state, `Filtered`. The plan listed Clean, Dirty, Untracked, Ignored,
        Conflicted, and Outside. A file with a `filter` attribute (Git LFS) holds bytes on disk that are not what Git
        stores, so it must never count as clean; without its own state it would have looked clean when LFS is not
        installed (the pointer text matches its blob). The alternative was folding it into `Dirty`, which hides why.
        Reverting means mapping the attribute check to `Dirty` in `sniff/lib/src/filesystem/git/tracked_state.rs`.
    - |-
        Pre-flight asks only about the reads of the root document's own `$schema` template. The plan also wanted
        matched trigger payloads and transcluded children. A trigger whose payload holds slots is still skipped as an
        invalid trigger until the trigger work (pending payloads) lands, and the pre-flight walk keeps no parsed child
        documents to ask the schemas about. Consequently the rule "a slot under a property a higher layer redeclares is
        never asked about" has nothing to filter yet: only one layer can hold a template. A child's reads, and any read
        not asked up front, are still judged when the slot evaluates (through the same handler on a terminal). The
        next phase extends `DarkmatterSchemas::pending_templates_for` when payloads become pending; nothing built here
        needs redoing.
    - |-
        The pre-flight discovery walk now leaves a schema template unjudged. Before, discovery validated the document's
        schema and failed on any template ("unresolved slots"), so pre-flight could never reach the approval step. Now a
        discovery pass (`defer_shell_pending_schema_problems`) treats `UnresolvedSlots` as "judged later" at the three
        places it prepares the effective schema; the terminal pass still fails until resolution lands. The alternative was
        to resolve during discovery, which would read files before anything is approved. Reverting means removing
        `schema_validation::defers_template`.
    - |-
        Slot reads and shell commands share one pre-flight, slot reads first, but the CLI shows one numbered prompt per
        read, followed by the shell prompts, rather than one combined menu. The plan said "asked together"; the shell
        side already prompts one command at a time, and a combined menu would need a new multi-answer prompt for both.
        The handler receives every slot request in one `approve_slot_reads` call, so a different front end can show a
        single menu. Changing it means rewriting `cli/src/approval.rs::approve_slot_reads_with_io` only.
    - |-
        Pre-flight without an approval handler does not fail for an unapproved slot read (shell commands do fail there
        with ApprovalRequired). The read stays `dm.schema.slot_approval_required` when its slot resolves, positioned at
        the slot, which is what the plan asks of data outputs and library callers. Pre-flight hands the handler's
        run-only answers to the caller in `ComposePreflightApprovals::slot_grants` (persisted ones are already in the
        store) for the session that resolves the slots. The alternative, failing pre-flight, would report the problem
        at the document instead of at the slot.
    - |-
        Pre-flight counts a read as known only when its target is a URL or an absolute or `./`/`../` path (after
        folding literals and pure functions). Any other spelling (a bare name, a magic root) is found by probing
        candidate files, which pre-flight must not do, so it is judged when the slot evaluates instead. The cost is a
        prompt during evaluation instead of in the batch for those spellings. Widening it needs a passive candidate
        resolver.
    - |-
        Deny wins anywhere in the grant book; there is no ranking by specificity. The plan said "deny beats allow at
        equal specificity and anywhere in the book", which reduces to the same rule. So `allow file /a/b.yaml` under
        `deny dir /a` is denied. A glob is denied by any deny it could reach, including one inside its folder
        (`/a/**` is denied by `deny dir /a/secret`). The alternative (a more specific allow wins) would let a narrow
        allow punch through a deliberate folder deny.
    - |-
        A persisted answer whose store write fails still applies for the run, with a warning in the trace, instead
        of failing the run. A handler that fails while a slot is evaluating counts as no answer, so the read is
        refused. Both fail toward "not allowed beyond this run". Failing the run on a write error is the alternative;
        it would change `SlotGrants::apply` and its two callers.
    - |-
        Grant store details the plan left open: `url`, `url-any-query`, and `site` take only http and https; a
        `url-any-query` line that has a query is rejected rather than stripped; a store error about the whole file
        (unreadable) has line 0 and its diagnostic has no line. Each is one branch in
        `slots/grants/store.rs`.
    - |-
        The data-output and `md compose` proofs of `approval-required` wait for the phase that makes commands resolve
        templates. No CLI surface resolves slots yet: `md compose` of a template document asks (on a terminal) and then
        fails with the existing unresolved-template error. The approval matrix is proved in-process, through the real
        resolver, Git, and grant store (`schemas/slots/approval_tests.rs`), and the CLI prompt through its scripted-IO
        seam.
    - |-
        A pass that must read nothing (a reference check, or the pre-flight discovery walk: both set "defer what only
        composition can judge") uses only a snapshot an earlier pass pinned and otherwise leaves the whole template
        for composition. The plan's ReferenceCheck mode resolves with automatic and persisted grants and defers only
        the property whose slot was unapproved. Deferring the whole schema is simpler and reads nothing before
        approval; the cost is that a reference check run before composition does not judge template-typed values.
        `SessionMode::ReferenceCheck` exists for a caller that wants the plan's behavior. Changing it means passing
        `ReferenceCheck` instead of pins-only in `compose/schema_validation.rs::prepare_schemas` and adding per-
        property deferral.
    - |-
        The Q14 rule (a trigger slot under a property the document `$schema` redeclares is never read) is applied to
        pre-flight's questions only. When a matched trigger payload resolves, all of its slots are evaluated,
        including shadowed ones. A template resolves as a whole (`PendingSchema::resolve`), and leaving some slots
        unresolved would need partial resolution, which the spikes ruled out (S6). The alternative is a placeholder
        value per shadowed slot, chosen by position kind. Changing it touches
        `DarkmatterSchemas::resolve_pending_payload`.
    - |-
        A cached resolution that read a URL, a folder listing, or a glob is served only to the command that produced
        it, like a volatile one. The plan's cross-command check rechecks files by `stat` but says nothing about inputs
        that cannot be rechecked cheaply. Refetching a URL or relisting a folder on every lookup would cost as much as
        resolving. Changing it means extending `cache::InputCheck::holds` and `rechecks`.
    - |-
        Context inputs for the cache are found passively from the slot's text (each `ctx.KEY`'s group, each
        `env.NAME`, the whole environment for a bare or computed `env`) and digested afresh, both when an entry is
        stored and when it is looked up. The evaluator's own captured digests were not used, because a failed slot has
        none, and the cache keeps failures. The cost is that a `ctx`-reading slot captures its group a second time
        when stored. Reverting means storing `SlotEvaluation::context_digests` and not caching failures that read
        context.
    - |-
        An invalid grant store found while a slot resolves (rather than in pre-flight) fails every slot of the
        template with `dm.schema.slot_grant_store_invalid`, the store's file and line in the message, rather than as a
        separate record at the store's line. The resolution's error type has no place for a second file's position.
        Pre-flight still reports it at the store's file and line. Changing it means a store-error variant in
        `SchemaError::SlotResolution`'s projection.
    - |-
        A document's own `load_data` whose contract names a type holding slots fails as unresolved, even in a
        composition that has a slot session. Only a contract met inside a slot's evaluation resolves through the
        session (N33's wording: "nested resolution"). Resolving a document's own contracts would make every
        `load_data` call able to evaluate schema slots and take approvals. Changing it means having
        `contract.rs::prepare_contract` find the composition's session instead of only the active slot's.
    - |-
        The resolution key includes a template identity beyond the schema files read: the `$schema` value as written
        plus its folder for a document, and the trigger path plus payload for a trigger. An inline `$schema` template
        has no schema file whose text could key it. The plan's key lists file hashes, the entry path, and slot keys
        only. Nothing needs redoing if the reviewer prefers hashing the expanded template instead; that is one
        function, `cache::resolution_key`.
    - |-
        `md schema triggers` gained a required `pending: boolean` on each trigger record (and a "(payload pending:
        holds schema slots)" suffix in text output) to show a template payload, rather than a new state enum. Changing
        it means editing `cli/schemas/output/schema-triggers.yaml`, `TriggerTraceData`, and
        `cli/src/commands/schema/triggers.rs`.
    - |-
        Without a slot session, a document matching a template trigger payload now fails its effective schema with
        `UnresolvedSlots` (for example the lifecycle call check's `effective_schema_for`, which then sees no schema),
        where before Phase 6 the trigger was skipped as invalid. That follows N31's rule (no session means
        unresolved). The alternative, silently skipping the layer, would validate against less than the effective
        schema.
    - |-
        (N34 fork) A kind-selection refusal carries its code in a new generic field on the simplified-schema cause
        type (`KindSelectionCause::Reserved/Unavailable { code: Option<&'static str> }`), the pattern `SlotFailure`
        and `SlotAdmissionError` already use, rather than a dedicated `SlotsNotAllowed` variant that would teach
        simplified-schema a Darkmatter rule. Reverting means removing the field and the `described()` helper in
        `yaml_documents/select.rs`.
    - |-
        (N34 fork) The baseline code is derived, not stored: a `SchemaError::Baseline` whose `source` is
        `UnresolvedSlots` maps to `dm.schema.slot_not_allowed_in_baseline` through `slot_prohibition_code`, which
        avoids a code field on `SchemaError::Baseline`. A caller that rebuilds a `Baseline` error and drops `source`
        loses the code.
    - |-
        (N34 fork) A global catalog holding a slot is refused when it loads (`CatalogErrorReason::SlotsNotAllowed`),
        whether or not it is used as the baseline, because a catalog's globals apply before any slot could resolve.
        The alternative was refusing it only when registered as the baseline.
    - |-
        (N34 fork) A slot inside a trigger `match` keeps the existing invalid-trigger code
        `dm.schema.invalid_trigger`, with a message naming the slot; N34 gives match conditions no code of their own.
    - |-
        (N41 fork) Claudine's shell completion treats a template schema as pending: it offers the template's property
        names, and values that depend on a slot fall back to the untyped completer (a debug trace records it).
        Resolving slots during completion would break "completion evaluates nothing"; offering nothing is the silent
        behavior being fixed. Changing it means editing
        `claudine/cli/src/completion/schema_completion/mod.rs::load_prompt_schema` and two call sites in
        `engine/mod.rs`.
    - |-
        (N41 fork) Claudine's launch path reports a template schema through the existing
        `CompositionError::SchemaLoad` fallthrough (carrying the unresolved-slots message) rather than a dedicated
        `SchemaPending` variant. Adding one is one arm in
        `claudine/lib/src/composition/schema/translate.rs::schema_error_to_composition_error` plus its rendering.
    - |-
        (Phase 7) `md compose` attaches its one slot session after pre-flight, so pre-flight's answers for this
        run can be handed to it. Its own reference check runs before pre-flight and keeps using a pass that reads
        nothing a slot names, so a template is left for composition to judge there. The plan says one session per
        command. Building the session first would mean a session that can take grants after it is built.
        Redoing it means adding that and attaching the session before the reference check.
    - |-
        (Phase 7) In `md graph` and `md validate refs`, a template whose read nobody granted leaves the whole
        schema unchecked for that run. The plan said to defer only the property that depends on the slot. The
        existing deferral works per schema; per property would need a partial resolution that keeps the slots it
        could resolve. Redoing it means changing `SlotSession::resolve` and `schema_validation::defers_template`.
    - |-
        (Phase 7) `md schema validate` judges automatic grants against the repository of the launch folder, once
        per run, not against each document's own repository. A document in another repository gets no automatic
        grant and needs approval. One session per run keeps grants decided once. The alternative is a session per
        document, built in `validate_one`.
    - |-
        (Phase 7) The sessions of `md schema validate` and `md clean` are built from a file context
        (`SlotSession::for_context`), which has none of the request's captured observations. A slot calling
        `has_command` or `has_binary` there sees an empty `PATH`. Passing the command's observations through would
        need a new public setter on the session.
    - |-
        (Phase 7) `grants_needed` lists only the grants that would allow the read, both once and persisted, in
        prompt order. Deny choices are not listed. A file record in `md schema validate` repeats the union of its
        diagnostics' grants, each once. A once grant can only be given at a prompt, so a script can act only on the
        persisted ones. Listing persisted grants alone would be shorter but would not match the prompt.
    - |-
        (Phase 7) If the approval handler fails while asking (for example, stdin cannot be read), `md schema
        validate` and `md clean` log the failure and go on. The read then reports
        `dm.schema.slot_approval_required`, as an evaluation-time ask already does. The alternative is a new
        error outcome for a file.
    - |-
        (Phase 7) A schema file that holds slots reports `slots: N` only when N > 0, and gets one note row in the
        formatted report. A concrete schema has no `slots` key. The count comes from parsing the file as a
        SimplifiedSchema, which reads imported schema files but nothing a slot names.
    - |-
        (Phase 7) `md schema validate` records gain an optional `code` naming a slot where none may appear (a kind
        definition), and a baseline holding a slot reports `dm.schema.slot_not_allowed_in_baseline` instead of
        `dm.schema.baseline`. The plan did not list these. The Phase 6 handoff noted they were not shown.
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
    - |-
        The editor serves a template from the slot worker's own resolution session, not a separate snapshot copy. Each
        background job builds a fresh session, resolves the document's schema with it, and the server keeps that session;
        requests then use it in "pinned snapshots only" mode, which reads and evaluates nothing. The plan described a
        `SlotSnapshots` store holding snapshots and a `Dependencies.slot_reads` list on the overlay's dependencies; the
        store exists (`dmls/src/slot_jobs.rs`) but holds the session, its failure, and the paths it read, and the watched
        paths come from it rather than from `Dependencies`. The alternative, copying resolved schemas into the overlay,
        would duplicate the library's pin and cache logic. Changing it means replacing the `SlotUse::Pins` path in
        `overlay/schema.rs::assemble` and moving the read list into the overlay's dependency record.
    - |-
        The editor's "about 25 ms of work" bound is a smaller work allowance, not a clock: each slot gets 25,000 visits and
        4 MiB of expansion (a quarter of a command's), and exceeding it is the ordinary `dm.iteration.*` failure at the
        slot. A wall-clock deadline would make results depend on how loaded the machine is and would flake in tests. A
        500-member list resolves well inside it. If a time limit is wanted, `SlotSession::with_cancel` already gives a
        hook: a timer could set the flag, which reports `dm.schema.slot_canceled` instead.
    - |-
        A failure that the editor's smaller allowance caused is never put in the process-wide resolution cache, so a
        command with the full allowance never receives it. A successful editor resolution is shared as usual. The plan did
        not cover this; without it, the editor could cache a work-limit failure that `md` would then report.
    - |-
        Fixed a library bug found by the editor tests: the cross-command cache trusted an unchanged file `stat` even when
        an editor buffer with different text was open for that file, so an unsaved edit to a data file was ignored. The
        check now re-hashes whenever a buffer is open (`slots/cache.rs::InputCheck::holds`).
    - |-
        Quick fixes for an unapproved read offer the library's allow choices (file and folder, this session and always)
        plus "always deny" and "deny for this session" of the first target. The library's `grants_needed` list carries
        allows only and no allow/deny word, so the deny choices are added by DMLS. "This session" grants live in the
        server's memory until it exits. Adding the decision to `GrantNeeded` would mean changing all 18 CLI output schemas.
    - |-
        An inline `$schema` slot is positioned by finding its `{{ … }}` text inside the document's `$schema` value: the
        library reports an inline slot's span relative to its type string, not the document. The alternative is to make
        the library project inline slot spans into document coordinates, which is the better long-term fix; the DMLS
        lookup (`diagnostics/slots.rs::authored_slot_span`) would then be deleted.
    - |-
        While no resolution serves a template, completion offers the template's literal parts (`enum(Haiku, {{ … }})`
        offers `Haiku`) and hover adds a "pending" or "stale" note. Completion items carry no stale marker (the plan said
        hover and completion annotate "stale"); a per-item marker would clutter every list while a refresh runs, which is
        usually milliseconds. Diagnostics carry the stale marker as one `dm.schema.slot_pending` information record.
    - |-
        Severity in the editor: `dm.schema.slot_pending` is information (nothing is wrong yet), `dm.schema.slot_approval_required`
        is a warning (it needs the author's decision, not a fix), and every other slot failure is an error. When a new
        template has no resolution yet but the previous one failed, the previous failure stays shown until the refresh
        lands, to avoid flicker on every keystroke.
    - |-
        A template left unresolved because only pinned snapshots may be used now keeps its resolution handle
        (`UnresolvedSlots::pending`), so a caller can still read its literal parts. Before, that path dropped the handle.
        No existing caller depended on its absence.
    - |-
        An unsaved edit reaches a slot's `load_data` only for open YAML and JSON buffers, the same buffers schema
        resolution already reads. A change to any other open file a slot read still marks the document stale, and the
        refresh reads that file from disk.
message_to_agent: |-
    Phase 8 (DMLS) is complete; see the log's Phase 8 section. For Phase 9:
    - The editor behavior is documented in `darkmatter/docs/topics/schemas/schema-slots.md` ("In the editor"), with a
      short pointer in `dmls-schema-support.md`, and in the skill `.claude/skills/darkmatter/dmls.md` ("Schema slots").
      `darkmatter/docs/lsp/features.md` and `architecture.md` were not touched; Phase 9's doc task covers them (the slot
      worker, `dmls.grantSlotRead`/`dmls.refreshSchemaSlots`, `executeCommandProvider`, the slot watch registration
      `dmls-schema-slot-watchers`).
    - Timing sample: the worker logs `schema slots resolved` at debug with `elapsed_ms`; requests never resolve, and
      `tests/l1/no_side_effects.rs` proves it with the `Work::SlotResolution` counter.
    - The work bound is a visit/byte allowance (`dmls/src/slot_jobs.rs::DMLS_SLOT_LIMITS`), not a clock; see decisions.
    - Library changes this phase: `SlotSession::{with_cancel, with_slot_limits}`, `DarkmatterSchemas::with_slot_pins_only`
      is public, `GrantTarget::parse`, `report_diagnostics::template_slot_records`, and a cache fix (an open buffer is
      never trusted by `stat`).
    - The Phase 7 work was still uncommitted when Phase 8 started, so the working tree holds both phases.
    - Carried over: the host sweep (Sun/Wed 04:00) can delete target/ mid-build, and an outside process may commit
      the tree.
source_files_during_phase_5:
    - darkmatter/cli/src/approval.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/read_gate.rs
    - darkmatter/lib/src/markdown/compose/expression/read_gate_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/approval_tests.rs
    - darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/auto.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/matching.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/request.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/store.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/tests.rs
    - darkmatter/lib/src/markdown/schemas/slots/host.rs
    - darkmatter/lib/src/markdown/schemas/slots/mod.rs
    - sniff/lib/src/filesystem/git/mod.rs
    - sniff/lib/src/filesystem/git/tracked_state.rs
    - sniff/lib/src/filesystem/git/tracked_state_tests.rs
    - sniff/lib/src/filesystem/git/types.rs
docs_updated_during_phase_5:
    - darkmatter/docs/topics/schemas/schema-slots.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_6:
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/tests/l1/yaml_schema_envelope.rs
    - darkmatter/lib/src/markdown/schemas/slots/cache.rs
    - darkmatter/lib/src/markdown/schemas/slots/session.rs
    - darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
    - darkmatter/lib/src/markdown/schemas/slots/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/host.rs
    - darkmatter/lib/src/markdown/schemas/slots/evaluator.rs
    - darkmatter/lib/src/markdown/schemas/slots/codes.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
    - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
    - darkmatter/lib/src/markdown/output/command/data.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/dynamic_schema_slots.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/schemas/output/schema-triggers.yaml
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/yaml_document.rs
    - darkmatter/dmls/tests/l1/yaml_document_editor.rs
    - claudine/cli/src/completion/schema_completion/mod.rs
    - claudine/cli/src/completion/schema_completion/keys.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/completion/engine/mod.rs
    - claudine/cli/src/completion/engine/tests.rs
docs_updated_during_phase_6:
    - darkmatter/docs/topics/schemas/schema-slots.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - darkmatter/docs/cli/schema.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_7:
    - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
    - darkmatter/lib/src/markdown/schemas/slots/session.rs
    - darkmatter/lib/tests/l1/diagnostic_projection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/shared_diagnostics.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/src/commands/graph.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/commands/validate.rs
    - darkmatter/cli/src/output/mod.rs
    - darkmatter/cli/tests/common/fixture.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - darkmatter/cli/tests/l1/dynamic_schema_slots.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/cli/schemas/output/clean.yaml
    - darkmatter/cli/schemas/output/code-block.yaml
    - darkmatter/cli/schemas/output/compose.yaml
    - darkmatter/cli/schemas/output/delta.yaml
    - darkmatter/cli/schemas/output/edit.yaml
    - darkmatter/cli/schemas/output/get.yaml
    - darkmatter/cli/schemas/output/graph.yaml
    - darkmatter/cli/schemas/output/hash.yaml
    - darkmatter/cli/schemas/output/list-themes.yaml
    - darkmatter/cli/schemas/output/render.yaml
    - darkmatter/cli/schemas/output/rm.yaml
    - darkmatter/cli/schemas/output/schema-about.yaml
    - darkmatter/cli/schemas/output/schema-detect.yaml
    - darkmatter/cli/schemas/output/schema-triggers.yaml
    - darkmatter/cli/schemas/output/schema-validate.yaml
    - darkmatter/cli/schemas/output/set.yaml
    - darkmatter/cli/schemas/output/toc.yaml
    - darkmatter/cli/schemas/output/validate-refs.yaml
    - claudine/docs/research/sandboxing/providers/sandboxing.yaml
docs_updated_during_phase_7:
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/cli/graph.md
    - darkmatter/docs/cli/schema.md
    - darkmatter/docs/cli/validate.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/schema-slots.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_8:
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/cache.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/mod.rs
    - darkmatter/lib/src/markdown/schemas/slots/grants/store.rs
    - darkmatter/lib/src/markdown/schemas/slots/host.rs
    - darkmatter/lib/src/markdown/schemas/slots/session.rs
    - darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
    - darkmatter/dmls/src/capabilities.rs
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/slots.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/yaml_document.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/slot_jobs.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/no_side_effects.rs
    - darkmatter/dmls/tests/l1/schema_slots_editor.rs
docs_updated_during_phase_8:
    - darkmatter/docs/topics/schemas/schema-slots.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/darkmatter/dmls.md
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

## Phase 5

### What was done

Wave 10's two separable pieces ran as two parallel subagents (Sniff, and the
grant store), each verifying only its own tests; everything after them was
done serially in this session, because the gate, pre-flight, and prompt all
touch the same types.

- **Sniff tracked state (N25).** New `sniff/lib/src/filesystem/git/tracked_state.rs`:
  `TrackedState::{Clean { index_blob, head_blob }, Dirty, Untracked, Ignored,
  Conflicted, Filtered, Outside}` (`Filtered` is new, see decisions) and
  `GitRepo::tracked_state(&[PathBuf])` (index and `HEAD` tree read once per
  call, no status walk; a path is `Clean` only when `HEAD` and index agree and
  its bytes, after gix's worktree-to-git conversion, hash to the index blob),
  plus `GitRepo::worktree_blob_id(path, bytes)` (autocrlf-aware). Docs in
  `sniff-library-architecture.md` and the sniff skill.
- **Grant store and book (N23, N26).** New `schemas/slots/grants/{mod,store,matching}.rs`:
  `GrantTarget::{File, Dir, Url, UrlAnyQuery, Site}`, `GrantScope`,
  `GrantDecision`, `Grant`; `GrantStore` at `<home>/.config/darkmatter/schema-read-policy`
  (path from `FileResolutionContext::home_dir()`; parser per Matrix 2; atomic
  append through temp-then-rename creating the folder); `GrantBook` (persisted
  + run-only + `--allow host=` values; deny wins anywhere).
- **Automatic grant (N24, Q11, Q13, Q21).** New `grants/auto.rs`:
  `RepositoryGrounds` opens the repository at the request's observed root
  (new `ComposeOptions::slot_repository_root`, never a new discovery) and asks
  Sniff once per path. Content of an existing file: Clean, blob id of the bytes
  read equals index and `HEAD`, no differing open buffer; the BLAKE3 of those
  bytes is kept. Probes and absent targets need only "defining file and target
  in the repository"; listings, globs, and URLs never.
- **Gate (N27, N28).** New `grants/gate.rs`: `SlotGrants` (deny → allow →
  automatic, memoized per run for Q17, and a post-read check that the
  observation's BLAKE3 equals the granted bytes → handler → `ApprovalRequired
  { needed }`), `RunGates` (a gate per slot, by defining file), the slots' own
  `FetchPolicy` (exact hosts of granted URLs/sites and `--allow host=`) and
  `remote_runtime` (new `RemoteFetchRuntime::with_policy_from_config`).
  `ReadTarget` gained `observed_hash`; `GateRefusal::ApprovalRequired` gained
  `needed: Vec<Grant>`. The evaluator now takes `SlotGates`
  (`with_gates`, `with_remote`); `with_gate` stays as one gate for all.
- **Targets and pre-flight (N29, N30).** New `grants/targets.rs::collect_targets`
  (passive; every branch; literal or pure-folded arguments; URLs and absolute
  or `./`/`../` paths are known, anything else computed) and
  `grants/request.rs` (`SlotGrantRequest`, `SlotGrantDecision`, the choice
  list). `ShellApprovalHandler::approve_slot_reads` (provided method, default
  unanswered). `DarkmatterSchemas::pending_templates_for` (document `$schema`
  layer). `compose_preflight_approvals*` judges and asks slot reads after the
  blacklist checks and before any shell prompt, returns
  `ComposePreflightApprovals::slot_grants` and `stats.slot_reads_asked`; an
  invalid store is the new `ShellExpansionError::SlotGrantStore`
  (diagnostic `dm.schema.slot_grant_store_invalid` at the store's file and
  line). A discovery pass now leaves an `UnresolvedSlots` schema to the
  terminal pass (`schema_validation::defers_template`), without which
  pre-flight failed on every template.
- **CLI prompt.** `cli/src/approval.rs`: `CliShellApprovalHandler::approve_slot_reads`
  and the scripted-IO seam `approve_slot_reads_with_io` (one numbered prompt
  per read, exactly the request's choices, re-asks on a bad number, ended
  input is "deny for this run" for this and every later request). The CLI
  installs the handler only on a terminal and never for a data output
  (unchanged rule), so neither prompts.
- **Docs.** `darkmatter/docs/topics/schemas/schema-slots.md` "What a slot may
  read" rewritten (decision order diagram, automatic grant table, store
  format, prompts; the run that uses the grants stays **planned**);
  `sniff-library-architecture.md`. Skills: `darkmatter/schema.md`,
  `sniff/SKILL.md`, `sniff/remote-and-repository.md`.

### Departures

- The plan's Q14 filtering (a slot a higher layer redeclares is never asked)
  and trigger-payload templates are not collected: only the document layer
  can hold a template today (see `decisions_for_review`).
- `approval-required` on a CLI data output is not proved by a spawn test: no
  CLI surface resolves slots yet (Phases 6/7). It is proved for library
  callers in-process.

### Requirement-to-test mapping

Boundary: Darkmatter's crate-internal resolver path through
`DarkmatterSlotHost` and `RunGates`, crossing into Sniff (real Git
repositories built by spawned `git` in temp dirs) and the filesystem (real
grant store files); F2, tier L1. The CLI prompt is tested at its scripted-IO
seam (no window, no TTY). Remote cases use a local wiremock server.

| Behavior | Test (`markdown::schemas::slots::approval_tests::…` unless noted) |
| --- | --- |
| clean committed file read with no grant | `automatic::a_clean_committed_file_in_the_schemas_repository_is_read_without_a_grant` |
| dirty, untracked, ignored, LFS-filtered refused; file and folder offered | `automatic::a_dirty_untracked_ignored_or_filtered_file_needs_a_grant` |
| no repository | `automatic::no_repository_means_no_automatic_grant` |
| other repository | `automatic::a_file_of_another_repository_needs_a_grant` |
| schema outside the request's repository (Q13) | `automatic::a_schema_outside_the_requests_repository_gets_no_automatic_grant` |
| symlink escaping (Unix; Windows skipped by `cfg`, reason in the test) | `automatic::a_symlink_escaping_the_repository_is_judged_by_where_it_points` |
| unsaved buffer differing (and equal buffer no change) | `automatic::an_unsaved_buffer_that_differs_from_disk_needs_a_grant` |
| file changed between status and read (hash mismatch), dirty from then on | `automatic::bytes_read_that_differ_from_the_granted_ones_are_refused` |
| `file_exists` of an untracked in-repo path automatic (Q11) | `automatic::an_existence_probe_of_an_untracked_path_in_the_repository_is_automatic` |
| `find_files` never automatic | `automatic::a_folder_listing_is_never_automatic` |
| missing in-repo file: typed not-found, no grant (Q21) | `automatic::a_missing_file_in_the_repository_fails_as_not_found_without_a_grant` |
| missing file outside the repository needs a grant | `automatic::a_missing_file_outside_the_repository_needs_a_grant` |
| persisted file grant | `granted::a_persisted_file_grant_allows_a_dirty_file` |
| run-only folder grant | `granted::a_folder_grant_for_this_run_allows_every_file_below_it` |
| persisted deny beats automatic and a run-only allow; run-only deny | `granted::a_deny_beats_the_automatic_grant_and_every_allow` |
| computed target asks the handler at evaluation: unanswered, deny, a choice not offered, allow | `granted::a_computed_target_asks_the_handler_during_evaluation` |
| persisted answer written to the user store, read back by the next run | `granted::a_persisted_answer_is_written_to_the_users_store` |
| slot fetch policy: no grant / other port refused before the network, exact URL, site, `--allow host=` fetched | `remote::a_url_is_fetched_only_under_a_grant_naming_its_scheme_host_and_port` |
| one batch with a shell command, slots first; run-only grant returned | `preflight::slot_reads_and_shell_commands_are_asked_in_one_batch_slots_first` |
| automatically granted read not asked | `preflight::a_read_the_automatic_grant_covers_is_not_asked` |
| library caller without a handler: nothing asked, nothing granted | `preflight::without_a_handler_nothing_is_asked_and_the_read_stays_unapproved` (plus every `approval_required` case above) |
| repository content cannot grant (store at repo root, beside the document, a `.dmls.toml` naming one) | `preflight::repository_content_cannot_grant` |
| invalid store stops pre-flight at file and line, with its code | `preflight::an_invalid_grant_store_stops_pre_flight_at_its_line` |
| unselected trigger payload with slots asks nothing | `preflight::an_unselected_trigger_payload_with_slots_asks_for_nothing` |
| literal + pure-function target known up front; `env` computed; untaken branch counted | `preflight::a_target_of_literals_and_pure_functions_is_known_before_evaluation` |
| CLI prompt numbering, labels, saved lines | darkmatter-cli `approval::tests::slot_reads::every_choice_is_numbered_and_names_what_it_saves` |
| "deny for this run" chosen; ended input denies this and later requests | `approval::tests::slot_reads::deny_for_this_run_is_a_choice_and_the_answer_to_ended_input` |
| bad number re-asks | `approval::tests::slot_reads::a_number_naming_no_choice_asks_again` |
| URL choices (exact, any query, site); inline schema shows no file | `approval::tests::slot_reads::a_url_read_offers_the_url_its_queryless_form_and_the_site` |
| Sniff states (17 tests: clean ids, staged, worktree, same-size edit, added, deleted, untracked, ignored, conflicted, filtered, outside, order, no status walk, autocrlf, bare, exec bit, symlink) | sniff `filesystem::git::tracked_state_tests::*` |

Input Robustness Matrix 2 (grant store loader), all in
`markdown::schemas::slots::grants::tests`: control row with all ten line kinds
(`control_row_loads_every_kind_and_decides`); every invalid field shape as its
own test from `invalid_line_cases!` (absent target, absent kind, empty after
trim, relative `file`/`dir`, drive-less rooted path on Windows, URL fragment,
unparseable URL, non-http(s) scheme for each URL kind, `site` with a path,
userinfo, query, or fragment, unknown verb, unknown kind, `url-any-query`
with a query), each asserting path, 1-based line, code, and that the whole
store fails; not UTF-8 and unreadable files; missing and empty files are
empty stores; comments, blank lines, CRLF, BOM accepted; duplicates
idempotent; allow + deny for one target is deny; write → read → write → read
round trip; no home. The CLI-spawn wiring row is not possible yet (no CLI
surface loads the store outside `md compose` pre-flight of a template
document); the in-process pre-flight test proves the loader is wired to the
public entry point and its diagnostic.

Loader smell grep over `schemas/slots/grants/` and `evaluator.rs`: no
`serde(default)` or `filter_map(.. as_str)`. The `.ok()`/`unwrap_or_default()`
hits all fail closed and none is a load-bearing parse: a handler error is
"no answer" (the read is refused), a slot that does not parse yields no
pre-flight reads (admission reports it), a fold that fails is "computed"
(judged at evaluation), a repository that cannot be opened grants nothing
automatically, and the Phase 4 environment digest of a string map cannot
fail.

### Gates

Run on macOS after every subagent finished, with nothing else running.

- `darkmatter`: `just lint` pass (first runs caught two test-only clippy
  findings, fixed). `just test`: the first full run had one failure,
  `l1 semantic_results_never_persist::semantic_results_have_no_path_to_disk`
  (a guard: the slot gate named the transport cache's `FileStore`). Fixed by
  deriving the slots' runtime from the command's
  (`RemoteFetchRuntime::with_other_policy`) so only the transport names the
  store; second full run 15383 passed, 10 skipped. `just check-tier-coverage
  darkmatter`: 0 stranded.
- `sniff`: `just lint` pass; `just test` 3562 passed, 33 skipped;
  `just check-tier-coverage sniff`: 0 stranded.
- Dependents: `cargo check --all-targets -p claudine -p claudine-cli
  -p messenger` passes. Targeted tests, one package at a time:
  `claudine` `test(/preflight|harness::shell|bootstrap/)` 338 passed;
  `claudine-cli` `test(/shell_approval|shell_options|preflight/)` 75 passed.
  Sniff's other dependents were not run: its change is additive (a new module,
  two new `GitRepo` methods, a crate-private helper) and no dependent
  glob-imports its git module.
- Cross-OS (path comparison and Git line-ending conversion carry Windows
  risk): `just cross-check sniff --os windows` pass (2500 tests, the 15
  non-Unix tracked-state tests among them). `just cross-check darkmatter --os
  windows` failed once:
  `slots::grants::tests::write_read_write_read_round_trips`, a fixture that
  joined `"spaced name/f.yaml"` as one string, so the first write kept a `/`
  that the reload canonicalized to `\` (a known trap, `os` skill
  `windows-paths.md`). The fixture now joins by component; the second
  Windows run passed (11138 tests). That change touches only the test file;
  its 51 tests pass on macOS and lint passes.
- Gate skipping: not applicable. `affected_scope.py` over the working tree
  reports `packages: [darkmatter, darkmatter-cli, sniff, test-toolkit]`,
  `change_class: package`.
- No pre-existing, skipped, or load-sensitive failures were seen.

## Phase 6

### What was done

Two disjoint pieces ran as parallel forks: the N34 slot prohibitions
(kind, baseline/catalog, DMLS extension, trigger `match`) and the N41
consumer sweep (Claudine, Messenger). Each fork verified only its own
tests. The session, cache, schema integration, trigger payloads, contract
recursion, and staleness work was done serially in this session, because
they share types.

- **Session (N31–N33, N35).** New `schemas/slots/session.rs::SlotSession`
  (public, built per command): the command's `DataRuntime`, captured
  observations, the document's remote runtime (for the slots' own),
  `--allow host=` values, repository root, home (grant store), handler,
  run-only grants, `SessionMode::{Full, ReferenceCheck, Passive}`. One
  `SlotGrants` is built lazily on first use (Q17), pins per resolution key
  (re-verified by rehashing when `memo_generation` moved, N35), and a
  thread-local resolution stack gives `dm.schema.slot_cycle`.
  `resolve_contract_slots` resolves a `load_data` contract that names a
  template, but only inside a slot's evaluation (N33). The contract memo is
  keyed by the active session.
- **Cache (N21, N22).** New `schemas/slots/cache.rs`: `ResolutionKey`
  (BLAKE3 of canonical JSON: version `SLOT_RESOLVER_VERSION = 1`, the
  catalog's hash, the template identity, each source file's text hash, the
  slots, the context identity, the repository, home presence). Entries hold
  the outcome (success or a cacheable failure), each input with its `stat`,
  context-input digests, and the reads the gate allowed (recorded by
  `RecordingGates`). A lookup trusts an unchanged `stat` (Q19), recomputes
  the context digests, and checks the requesting session's grants cover
  every read. A hit replays the observations into the command's runtime
  (read set). It is cleared by `reset_process_caches_for_bench`. The work
  counter is `Work::SlotResolution`.
- **Schema integration.** `DarkmatterSchemas::with_slot_session` and the
  crate-internal `with_slot_pins_only`. `resolve_template` resolves a layer's
  `UnresolvedSlots { pending }` for the document `$schema`
  (`effective_for_with_override`), `validate_yaml_document`'s root
  `$schema`, and matched trigger payloads. `ComposeOptions::with_slot_session`
  / `slot_session()` are added. The root pipeline builds one session per
  command when none is attached. `prepare_schemas` hands it to the schemas
  (pins only for deferring passes). The new `SchemaError::SlotResolution
  (SlotResolutionFailure { error, sources })` in simplified-schema carries a
  failed resolution, and `report_diagnostics::template_diagnostics` projects
  it (and `UnresolvedSlots` as `dm.schema.slot_pending`).
- **Trigger payloads (N34).** `ResolvedPayload::pending`: a template payload
  resolves to a pending payload (dependencies are its schema files), is never
  a finding, and is resolved by `resolve_trigger_layers` only when matched,
  then checked by the extracted `assemble::finish_file_payload`.
  `TriggerTraceEntry::pending` / `TriggersData` `pending` flow into
  `md schema triggers` (terminal and Markdown text, and the
  `schema-triggers.yaml` output schema).
- **Pre-flight.** `pending_templates_for` now returns `PendingTemplate
  { pending, slots }` for the document layer plus matched trigger payloads,
  with Q14 applied (a trigger slot under a property the document `$schema`
  redeclares is left out). Lifecycle collects through
  `targets::collect_slot_targets`.
- **N34 prohibitions (fork).** Kind claims: `KindSelectionCause::{Reserved,
  Unavailable}` gained `code`, set to `dm.schema.slot_not_allowed_in_kind`.
  Baseline, catalog (`CatalogErrorReason::SlotsNotAllowed`), and the DMLS
  extension layer: `SchemaError::Baseline` with an `UnresolvedSlots` source,
  read by `codes::slot_prohibition_code`. Trigger `match`: refused at any
  depth, with a message naming the slot.
- **N41 consumers (fork).** Claudine CLI completion:
  `PromptSchema::{Resolved, Pending}` through `load_prompt_schema`. A template
  offers its property names, and values that need a slot fall back to the
  untyped completer. The sweep found no other site that silently drops a
  template (list below).
- **Docs.** `schema-slots.md` covers where slots may appear, resolving in a
  command (session, pins, cache, cycles), and new failure rows; the
  remaining **planned** markers name only Phase 7/8 surfaces.
  `schema-triggers.md` gains "A payload that holds slots", and
  `docs/cli/schema.md` the pending listing. Skill: `darkmatter/schema.md`.

### Departures

- The plan's staleness test "with a lifecycle/shell effect" runs as two
  compositions sharing one session: the second runs a `::shell cp` that
  rewrites the data file, then a third validates. It is not one document,
  because frontmatter validation runs before shell expansion in the
  pipeline.
- The plan's "context (`ctx`/`env`) change ⇒ miss" is proved for `env`;
  `ctx` groups use the same recomputed-digest path (`context_digest_now`),
  but no test varies a captured `ctx` group.
- N38's `md schema validate`, `md clean`, DMLS, and carrying pre-flight's
  run-only grants into `md compose` are Phase 7/8. `SlotSession::with_run_grants`
  is the seam.

### Swallow sites (N41 sweep)

Fixed: `claudine/cli/src/completion/schema_completion/mod.rs` (`effective_for(..).ok()?`).
Already typed (they pass the unresolved error on): `claudine/lib/src/composition/schema/mod.rs`
(→ `CompositionError::SchemaLoad`), `claudine/cli/src/commands/signals.rs`,
`claudine/gen/src/inputs.rs`, `messenger/lib/src/research/load.rs`.
Intentionally best-effort: `claudine/lib/src/composition/schema/supplied.rs`, whose input
is already concrete.

### Requirement-to-test mapping

Boundaries: the slot session and process cache inside the Darkmatter crate,
crossing into Git (real repositories built by spawned `git` in temp dirs) and
the filesystem (real data, schema, and trigger files); F2, tier L1. The
end-to-end cases cross the public composition API (`Markdown::compose_with_warnings`
on a `ComposeRequest` with a temp home, so the user's real grant store is never
read). Evaluator runs are counted by `Work::SlotResolution` and compiles by
`Work::ValidatorCompile`; the counting cases are gated on the `work-counters`
feature, which `just test` enables.

| Behavior | Test (`markdown::schemas::slots::session::tests::…` unless noted) |
| --- | --- |
| warm hit: zero evaluator runs, zero compiles, same schema | `reuse::a_second_command_is_served_from_the_cache_without_evaluating_or_compiling` |
| read set equal cold vs warm | `reuse::a_hit_reports_the_read_set_a_cold_run_reports` |
| pinned within a command | `reuse::one_command_pins_its_snapshot`; l1 `dynamic_schema_slots::one_session_keeps_its_pinned_snapshot_for_every_pass` |
| edited data file ⇒ miss | `reuse::an_edited_data_file_is_a_miss` |
| edited schema file ⇒ miss | `reuse::an_edited_schema_file_is_a_miss` |
| missing probed file created ⇒ retry; failure cached until then | `reuse::a_cached_failure_is_served_until_the_missing_file_appears` |
| failure cached, then repaired ⇒ recovered | `reuse::a_repaired_data_file_recovers` |
| context (`env`) change ⇒ miss | `reuse::a_changed_environment_input_is_a_miss` |
| permission narrowing ⇒ miss (once-grant result not served) | `reuse::narrower_grants_are_not_served_a_result_broader_ones_produced` |
| grant-dependent failure never cached | `reuse::an_approval_failure_is_never_cached` |
| volatile slot: once per command, never across | `reuse::a_volatile_slot_resolves_once_per_command` |
| Q19: unchanged `stat` trusted (documented limitation), changed `stat` rehashes | `a_cross_command_check_trusts_an_unchanged_stat_and_rehashes_a_changed_one` |
| N35/Q17: rewrite during the run re-resolves with the earlier automatic grant; another command needs approval | `reuse::a_rewrite_during_the_run_re_resolves_under_the_grant_decided_earlier`; l1 `dynamic_schema_slots::a_rewrite_by_the_runs_own_shell_step_re_resolves_with_the_grant_decided_earlier` (a real `::shell cp`, Unix) |
| unrelated write keeps the pin | `reuse::an_unrelated_write_keeps_the_pin` |
| failed re-resolve keeps `dm.data.*` | `reuse::a_failed_re_resolution_keeps_its_data_code_and_publishes_nothing` |
| passive session resolves and reads nothing; deferring pass uses pins only | `a_passive_session_resolves_nothing`, `a_pins_only_pass_uses_only_a_pinned_snapshot` |
| compose: valid document under the resolved list, read set has the data file | l1 `dynamic_schema_slots::a_document_valid_under_the_resolved_list_composes_and_reads_the_data_file` |
| compose: invalid value is an ordinary validation problem (no slot code) | l1 `…::a_value_outside_the_resolved_list_is_a_validation_problem` |
| compose: dirty data, library caller never asked, `slot_approval_required` at the slot in the schema file with a line | l1 `…::a_dirty_data_file_needs_approval_and_a_library_caller_is_never_asked` |
| value position (`default`) through the session equals the literal | l1 `…::a_slot_supplied_default_lowers_like_the_same_literal_default` |
| trigger selected: payload resolves and applies, read set has its data | l1 `…::a_matched_template_trigger_payload_resolves_and_applies` |
| trigger unselected: read set empty, not an invalid-trigger warning | l1 `…::an_unselected_template_trigger_payload_reads_nothing` |
| `EffectiveSchema` holds no `Constraint::Dynamic`; no session ⇒ `UnresolvedSlots` | l1 `…::the_effective_schema_holds_only_resolved_constraints` |
| N33 cycle at the outer slot (two schemas loading each other) | l1 `…::two_schemas_whose_slots_load_each_other_are_a_cycle_at_the_outer_slot` |
| outside a slot, a contract naming a template is unresolved | l1 `…::outside_a_slot_a_contract_naming_a_template_is_unresolved` |
| layering equality unchanged | l1 `effective_schema_layering::*` (run with the above) |
| Q14 / unselected payload in pre-flight asks nothing | `approval_tests::preflight::an_unselected_trigger_payload_with_slots_asks_for_nothing` (still green after `pending_templates_for` grew trigger payloads) |
| N34 kind claim (inline and through `Name@file`) | l1 `yaml_kind_registry::a_kind_definition_holding_a_slot_is_a_broken_claim_with_the_slot_code`; dmls l1 `yaml_document_editor::a_kind_definition_holding_a_slot_is_one_error_under_the_kind_slot_code` |
| N34 baseline/catalog (direct, imported, in-memory) | l1 `global_catalog::a_baseline_holding_a_slot_fails_with_the_slot_code` |
| N34 DMLS extension layer | dmls `diagnostics::frontmatter::tests::a_schema_extension_holding_a_slot_is_refused_with_the_baseline_slot_code` |
| N34 trigger `match` condition | `triggers::grammar::tests::rejects_a_slot_in_a_match_condition` (7 cells + control); l1 `trigger_discovery_recovery::a_slot_in_a_match_condition_is_an_invalid_trigger` |
| prohibition codes only for prohibitions | `slots::codes::tests::only_a_prohibition_has_a_slot_code` |
| N41 Claudine completion reports a template as pending | claudine-cli `completion::schema_completion::tests::a_template_schema_loads_as_pending_with_its_slots_and_property_names`, `…::a_concrete_schema_loads_as_resolved`, `completion::engine::tests::setter_completion_of_a_template_schema_names_its_properties` |

`md schema triggers`'s `pending` field is covered by the existing conformance tests
in `cli/tests/l1/output_commands.rs`, which validate every data output against
`schema-triggers.yaml` (they pass with the field required). No test spawns the
CLI on a pending trigger; listing one is tested at the library trace
(`TriggerTraceEntry::pending`) through the l1 trigger cases. The Input
Robustness Matrix does not apply: no file-format loader changed this phase.
The loader smell grep over `slots/cache.rs`, `session.rs`, `evaluator.rs`, and
`host.rs` found only fail-closed hits (a missing `stat` forces a rehash, an
unreadable file stays unreadable, the env digest of a string map cannot fail).

### Gates

Run on macOS after both forks finished, with nothing else running.

- `darkmatter`: `just lint` pass (the first runs caught test helpers that only the
  `work-counters` cases use, a collapsible `if`, and one `err().expect` in the
  fork's test; all fixed). `just test`: 15418 passed, 10 skipped, first run.
  `just check-tier-coverage darkmatter`: 0 stranded.
- `simplified-schema`: `just test` 1633 passed; `just lint` pass.
- Dependents: `cargo check --all-targets` of every package depending on
  darkmatter or simplified-schema (research, biscuit-icon-cli, claudine,
  claudine-cli, claudine-gen, playa-cli, sniff-detection-cli, messenger,
  messenger-cli, biscuit-speaks-cli, biscuit-terminal-cli): clean, no warnings.
  `cargo clippy -p claudine-cli --all-targets -D warnings`: clean. Targeted:
  claudine-cli `test(/completion/)` 641 passed; claudine
  `test(/composition::schema|schema|trigger|preflight/)` 1403 passed; messenger
  `test(/research|schema/)` 5 passed.
- Gate skipping: not applicable. `affected_scope.py` over the working tree reports
  `packages: [claudine-cli, darkmatter, darkmatter-cli, darkmatter-lsp,
  simplified-schema, sniff, test-toolkit]`, `change_class: package` (the tree
  still holds Phase 5's uncommitted work too).
- Cross-OS (the cache turns portable read-set paths back into native paths and
  compares `stat` times): `just cross-check darkmatter --os windows` passed on the
  first run, 11170 passed (1 slow), 66 skipped. The shell-rewrite compose test is
  `#[cfg(unix)]` because it runs `cp`; its session-level twin runs everywhere.
- No pre-existing, skipped, or load-sensitive failures were seen.

## Phase 7

### What was done

The library seams came first, done serially in this session because the CLI
work depends on them; the library-level tests were then written by one
subagent while this session wrote the spawned-`md` tests (disjoint files).

- **Library seams.**
  - `SlotSession::for_request` is public, and the new
    `SlotSession::for_context(DataRuntime, &FileResolutionContext)` serves
    commands that validate without composing.
  - `SlotSession::approve_reads(templates, context)` is the batched
    collect-and-approve step: Full mode with a handler only. It shares
    `SlotGrants::undecided_requests` / `apply_all` with the compose
    pre-flight, which now uses them too.
  - `DarkmatterSchemas::approve_slot_reads(&Markdown)` wraps it.
  - `Markdown::compose_slot_approvals(request, handler)` is the slot half
    of the pre-flight, for a run with no shell operation.
  - `SessionMode::ReferenceCheck` turns a resolution that failed only for
    a grant (`slot_approval_required`, `slot_denied`) into `UnresolvedSlots`,
    and `schema_validation::defers_template` defers that for a
    ReferenceCheck session as for `for_reference_check`.
  - `CleanSchemaConfig::with_slot_session`: `CleanSchemaContext::effective_schema`
    asks once (a `Once`) before the first resolution.
  - `Diagnostic::grants_needed: Option<Vec<GrantNeeded>>` (`{kind, target,
    scope}`, skipped when `None`) is filled by `report_diagnostics::with_expression_cause`
    from `ExpressionError::SlotReadRefused { ApprovalRequired { needed } }`.
  - `report_diagnostics::template_diagnostics` is public, and
    `error_diagnostics` now projects a bare `SchemaError` template failure,
    which `md clean`'s failure path hands it.
- **`md compose`.** One session per invocation, built after pre-flight by
  `commands/compose.rs::with_command_slot_session`
  (`for_request(&request)` shares the command's `DataRuntime`, so slot reads
  join the envelope's `read_set`, plus `with_run_grants(slot_grants)`, plus the
  handler only on an interactive formatted run). With no shell operation
  enabled, `compose_slot_approvals` still asks first. The lifecycle path
  (`commands/compose/lifecycle.rs`) gets the same hand-off.
- **`md graph`, `md validate refs`.** `with_reference_check_slot_session`
  attaches a ReferenceCheck session. They never prompt and defer an
  unapproved template.
- **`md schema validate`.**
  - One session per invocation (`for_context` at the launch context; handler
    only when formatted and interactive), attached to the loaded schemas.
  - Each Markdown document runs `approve_slot_reads` before its first
    resolution.
  - A template failure is `FileOutcome::Slots`, whose record holds
    `error: "slots"`, `valid: false`, `problems: []`, `message`,
    `diagnostics`, and a deduplicated `grants_needed`. It exits 1.
  - A YAML schema file reports `slots: N` passively
    (`SimplifiedSchema::try_from_yaml(..).pending_slots()`, reading nothing a
    slot names), with a note row in formatted output.
  - The slot-prohibition codes now surface: a kind definition holding a slot
    gives the record's `code`, and a baseline holding one gives the run
    diagnostic's code (`codes::slot_prohibition_code`).
- **`md clean`.** `schema_config` attaches a `for_context` session (handler
  only when it may prompt). An unapproved read fails the run with the slot's
  diagnostic, so the document is never reported clean and `--save` writes
  nothing.
- **Output schemas.**
  - Every file in `cli/schemas/output/` (18) gains `diagnostic.grants_needed:
    grant-needed[]` and the `grant-needed` type (identical everywhere, as
    `every_schema_file_shares_one_diagnostic_type` requires).
  - `schema-validate.yaml`'s `file-record` gains the `slots` error value and
    the `slots`, `diagnostics`, `grants_needed`, and `code` fields.
- **Shipped schema (N40).** `sandboxing.yaml`'s `os` type is now
  `enum({{ load_data("../../../os.yaml", "string[]") }}; required)`. The
  corpus carve-out was already gone (Phase 2). The new corpus test pins the
  template's passive admission.
- **Docs and skill.**
  - `schema-slots.md`: the per-command table replaces the planned marker, the
    pre-flight hand-off is no longer planned, and it covers reference-check
    sessions, `for_context`, and a `grants_needed` example. DMLS is still
    marked **planned** (Phase 8).
  - `definition.md` (JSON output, the `schema` kind check), plus
    `docs/cli/{schema,clean,compose,graph,validate,data-contracts}.md`.
  - Skill: `.claude/skills/darkmatter/schema.md`.

### Departures

- The plan's `md compose` session is "built from ... the grant book". It is
  built after pre-flight, so `md compose`'s own reference check, which runs
  first, still uses the composition's transient session pins-only and reads
  nothing. See decisions.
- N31 says a ReferenceCheck session "defers that property's judgment". The
  deferral is of the whole template layer (the existing `defers_template`
  path), not one property.
- N40's `awaiting_enum_slots` deletion was already done in Phase 2. This phase
  adds only the assertion.
- `MD_DRY_RUN` is not set in the end-to-end tests. The fixture builder sweeps
  `MD_*`, and no case passes `--show`. Nothing opens a window.

### Requirement-to-test mapping

Boundaries: the CLI process boundary and the filesystem and Git (spawned
`md` on real repositories that `git` built in temp dirs, with the shipped
`sandboxing.yaml` and `os.yaml` read by `include_str!` inside each test),
F3/L1. Library surfaces were tested in-process at the crate's public API,
F2/L1. No terminal is attached, so no case can prompt; prompting itself is
Phase 5's `approval.rs` seam tests.

CLI (`darkmatter-cli::l1 dynamic_schema_slots::…` unless noted):

| Behavior | Test |
| --- | --- |
| compose control: valid, `read_set` = `[os.yaml]`, conforms to `compose.yaml` | `compose_resolves_the_shipped_schema_and_reports_its_data_file_read` |
| `os_support: [BeOS]` fails naming the resolved members, no slot code | `compose_rejects_a_value_outside_the_resolved_list_naming_its_members` |
| dirty `os.yaml`, data output: exit 1, `slot_approval_required` at schema line 8, `grants_needed` (4, prompt order), empty read set, empty stderr | `compose_data_output_on_a_dirty_data_file_fails_naming_the_grants_needed` |
| no terminal (formatted, stdin fed): never prompts | `compose_without_a_terminal_never_prompts_and_reports_the_approval_needed` |
| persisted grant in the home store resolves the dirty file | `compose_with_a_persisted_grant_reads_the_dirty_data_file` |
| Matrix 2 wiring: invalid store line, `slot_grant_store_invalid` at the store and line 2 | `compose_with_an_invalid_grant_store_fails_at_its_line` |
| `md schema validate <schema file>` is passive: `slots: 1`, valid with `os.yaml` deleted | `schema_validate_of_the_shipped_schema_file_is_passive` |
| `md schema validate <document>` clean: valid, no `slots` | `schema_validate_of_a_document_resolves_with_the_automatic_grant` |
| `md schema validate <document>` dirty: `error: slots`, `valid: false`, `problems: []`, diagnostics, record and diagnostic `grants_needed`, exit 1 | `schema_validate_of_a_document_with_an_unapproved_slot_is_unavailable_never_valid` |
| kind-definition slot: record `code` | `schema_validate_names_the_code_of_a_slot_in_a_kind_definition` |
| baseline slot: run diagnostic code | `schema_validate_names_the_code_of_a_slot_in_the_baseline` |
| `md clean` dirty: failure, run diagnostic with grants, `--save` writes nothing | `clean_with_an_unapproved_slot_never_reports_the_document_clean` |
| `md clean` control: no findings | `clean_of_a_document_valid_under_the_resolved_list_reports_no_findings` |
| `md graph` / `md validate refs` dirty: deferred, exit 0 | `graph_defers_an_unapproved_slot`, `validate_refs_defers_an_unapproved_slot` |
| conformance (no repository: no automatic grant) | `output_commands::conforms_schema_validate_slots_failure_names_the_grants_needed`, `…::conforms_schema_validate_template_schema_file_counts_its_slots`, `…::conforms_clean_unapproved_slot_fails_with_the_grants_needed` |

Library (`darkmatter::l1 dynamic_schema_slot_surfaces::…`, by the subagent):

| Behavior | Test |
| --- | --- |
| `compose_slot_approvals` asks once (one request, canonical file); its grant feeds `with_run_grants`, and the read reaches the read set | `compose_slot_approvals::a_dirty_data_file_is_asked_once_and_its_grant_lets_the_composition_read_it` |
| no handler: no grants, composition needs approval | `compose_slot_approvals::without_a_handler_nothing_is_granted_and_the_composition_needs_approval` |
| clean file: never asked | `compose_slot_approvals::a_clean_committed_data_file_is_granted_automatically_without_asking` |
| `approve_slot_reads`: Full asks once then resolves; a second call asks nothing | `approve_slot_reads::a_full_session_asks_once_and_the_answer_resolves_the_template` |
| ReferenceCheck never asks | `approve_slot_reads::a_reference_check_session_never_asks` |
| ReferenceCheck: ungranted becomes `UnresolvedSlots`; automatic resolves and validates | `reference_check::an_ungranted_read_leaves_the_template_unresolved`, `…::an_automatically_granted_read_resolves_and_the_resolved_list_validates` |
| reference validation defers with the session, fails without it | `reference_check::a_reference_validation_with_a_reference_check_session_defers_the_unapproved_template` |
| `grants_needed` order, targets, JSON; absent on a `dm.data.*` slot failure | `grants_needed::an_approval_record_lists_the_grants_a_prompt_would_offer`, `…::a_slot_failure_that_is_not_an_approval_names_no_grants` |
| bare `SchemaError` projection; `template_diagnostics` pending at the document | `schema_error_projection::*` |
| `CleanSchemaConfig` session: fails without a handler; asks once for two resolutions | `clean_schema_config::*` |
| N40 passive admission of the shipped template | `darkmatter::l1 shipped_schema_corpus::the_shipped_sandboxing_schema_is_admitted_as_a_template_without_resolving` |

The Input Robustness Matrix does not apply: no file-format loader changed this
phase. Matrix 2's CLI wiring cell is the invalid-store test above. The loader
smell grep over the changed `slots/` files found only intentional fail-closed
uses: `approve_reads` returns 0 on an invalid store, which resolution then
reports typed.

### Gates

Run on macOS after the subagent finished, with nothing else running.

- `darkmatter` `just test`: 15449 passed, 10 skipped (1 slow), first run.
  The last small edits came after that run's build: the slot-prohibition
  `code` field, its two tests, and a store-line tweak in one test. The whole
  `darkmatter-cli` package was then rerun: 2421 passed. `just lint`: pass.
  `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents:
  - `cargo check --all-targets -p claudine -p claudine-cli -p claudine-gen -p messenger -p research`: clean.
  - `affected_scope.py` puts `claudine` and `claudine-cli` in scope because
    of the shipped `sandboxing.yaml`. Targeted runs: claudine
    `test(/claudine_schema_shipped_corpus/)` 67 passed; claudine-cli
    `test(/research_sequence_artifacts|schema_completion|completion::engine/)`
    119 passed.
- Gate skipping: not applicable. `affected_scope.py` over the working tree
  reports `packages: [darkmatter, darkmatter-cli, darkmatter-lsp,
  test-toolkit, claudine, claudine-cli]`, `change_class: package`.
- Cross-OS (canonical paths in `grants_needed` and in the new tests): `just
  cross-check darkmatter --os windows` passed on the first run, 11185 passed
  and 66 skipped.
- No pre-existing, skipped, or load-sensitive failures were seen. One
  subagent test first failed on an empty read set; the cause was the test
  setup (no data runtime on the request), documented on
  `SlotSession::for_request`. It passed alone after the fix.

## Phase 8

### What was done

All of it was done in this session, serially: the worker, the overlay hook,
the router, and the diagnostics depend on one another, and the tests drive
the whole server. No subagent was used.

- **Library seams** (`darkmatter`):
  - `SlotSession::with_cancel(Arc<AtomicBool>)`: Darkmatter's slot host now
    implements `SlotHost::is_canceled`, so a superseded editor job stops
    with `dm.schema.slot_canceled`, which is never cached or pinned.
  - `SlotSession::with_slot_limits(IterationLimits)`: the evaluator gets
    the session's allowance. A work-limit failure under custom limits is
    never cached for another session (`session.rs::exhausted_work`).
  - `DarkmatterSchemas::with_slot_pins_only` is public.
  - A pins-only miss keeps the template's `pending` handle
    (`session.rs::unresolved_template`).
  - `GrantTarget::parse(kind, argument)` shares the grant store's
    line judgment (`store.rs::target_from_words`).
  - `report_diagnostics::template_slot_records` returns each record with its
    slot. `slot_resolution_diagnostics` is now a projection of it.
  - Bug fix: `slots/cache.rs::InputCheck::holds` trusted an unchanged
    `stat` while an editor buffer with other text was open.
- **Worker and store** (`dmls/src/slot_jobs.rs`, new):
  - The `dmls-slots` thread runs `SlotJob`s. Each job uses a fresh Full-mode
    session with no handler, the in-memory once grants, the cancel flag, and
    `DMLS_SLOT_LIMITS` (25,000 visits, 4 MiB). It runs
    `overlay::resolve_slot_templates` inside `buffers::with`.
  - Reads: every local observation (file, exists, missing probe, listing),
    with `stat`. `.git/index` and `HEAD` are added when a read lies in the
    repository.
  - `SlotSnapshots` keeps one entry per document: the installed session and
    its failure, its reads, the stale flag, the in-flight `seq`/key/cancel,
    and a generation. Methods: `demand`, `take_demands`, `install` (seq and
    cancel checked), `invalidate_paths`, `invalidate_all`, `recheck_stats`,
    `watched_paths`, `add_once`, `forget`.
- **Overlay**: `schema_for` puts the snapshot generation in the schema
  cache key and assembles through `SlotUse::Pins(session)`.
  - A pins miss serves the installed failure, if any.
  - It records a demand when the pins missed (and the key was not already
    resolved) or the entry is stale.
  - It returns a `SlotStatus { refreshing, unresolved, template_shape }` on
    `DocumentOverlay::slots`. `SchemaBundle::templated` marks a served
    template.
  - The analysis root layers the template's literal shape when no bundle
    serves.
- **Router**: `after_message` = schema watches, `dispatch_slot_jobs`,
  `sync_slot_watches` (one `RelativePattern` per path, id
  `dmls-schema-slot-watchers`; a rejection switches to the save fallback).
  - `Internal::SlotPrepared` installs and republishes the document.
  - Invalidation: buffer open/change/close (`note_schema_buffer`), watched
    changes, saves (`recheck_stats` without watches), grants, refresh, and
    `didChangeConfiguration`.
  - `workspace/executeCommand` handles `dmls.grantSlotRead` (`persisted`
    goes through `GrantStore::append` under the snapshot's home; `once` is
    kept in memory) and `dmls.refreshSchemaSlots`. `executeCommandProvider`
    is advertised.
  - Test seams: `StartupHooks::slot_gate` and the
    `StartupEvent::{SlotJobQueued, SlotPreparing, SlotPrepared}` events.
- **Serving**: `diagnostics/slots.rs` (new). The severities:
  - `slot_pending` is information;
  - `slot_approval_required` is a warning, with `data.choices`;
  - other failures are errors.

  An inline slot is ranged at its `{{ … }}` text in the `$schema` value. A
  slot in another file is ranged at the `$schema` value and gets related
  information. A stale served snapshot adds one pending record.
  `code_actions.rs` turns each choice into a quick-fix `Command`. Hover
  appends a pending or stale note. `enum_members` reads the literal members
  of an unresolved `Constraint::Dynamic` list.
- **Docs and skill**:
  - `schema-slots.md`: the **planned** DMLS row is gone, and a new "In the
    editor" section has a Mermaid flow, a state table, grants, refresh
    triggers, the `stat` limitation, and limits.
  - `dmls-schema-support.md`: a pointer section.
  - Skill: `.claude/skills/darkmatter/dmls.md`, "Schema slots".

### Departures

- `Dependencies.slot_reads` is not on the overlay's `Dependencies`. The
  reads live on the store's installed entry, and the watches come from
  there (see decisions).
- The work bound is a visit and byte allowance, not 25 ms of wall time (see
  decisions).
- Completion items carry no "stale" annotation. Hover and diagnostics do.
- A slot syntax error is caught passively at lowering (the existing
  `SchemaError::Grammar` path), before any job. So "syntax diagnostics" on
  first load are that path's, at the definition.

### Requirement-to-test mapping

Boundaries crossed: the LSP protocol (the in-memory `Connection` drives the
real router, overlay, slot worker, and grant store), the filesystem (data
files, the grant store under the fixture `HOME`), and Git (`git` builds the
repository for the index-watch cell). The rung is F2 and the tier is L1. No
terminal or window is involved.

`darkmatter-lsp::l1 schema_slots_editor::…` unless noted:

| Behavior | Test |
| --- | --- |
| persisted grant resolves; frontmatter judged against loaded members (control + `BeOS` rejected naming members) | `a_persisted_grant_resolves_the_slot_and_the_frontmatter_is_judged_against_the_loaded_members` |
| first-load approval failure at the slot (warning, choices incl. deny); no verdict on other fields (never unconstrained); no prompt (no server request beyond registrations) | `an_unapproved_read_is_reported_at_the_slot_and_the_frontmatter_is_never_judged_unconstrained` |
| code action ⇒ command ⇒ store written (exact line) ⇒ refresh | `a_quick_fix_runs_the_grant_command_which_writes_the_store_and_refreshes_the_document` |
| once grant resolves, store never written | `a_once_grant_resolves_for_this_server_process_and_is_never_written_to_the_store` |
| invalid command arguments (relative path, bad decision, bad scope, wrong type, unknown command) refused, nothing written | `a_grant_command_with_an_invalid_target_is_refused_and_changes_nothing` |
| unsaved data buffer refreshes; disk untouched | `an_unsaved_edit_of_the_data_file_refreshes_the_snapshot` |
| saved dependency change refreshes through its own watcher (registration asserted) | `a_saved_change_to_the_data_file_refreshes_the_snapshot_through_its_watch` |
| missing file creation refreshes | `creating_a_missing_data_file_refreshes_the_snapshot` |
| last-good served while the refresh is held: old verdict + one information `slot_pending`; hover says stale; then current | `while_a_refresh_is_held_the_last_good_snapshot_is_served_marked_stale` |
| a held job does not delay completion; literal members offered before, loaded after; no request evaluates | `a_held_slot_job_never_delays_a_completion_and_the_template_offers_its_literal_members` |
| a newer edit cancels the in-flight job; only the newest installs | `a_newer_edit_of_the_template_cancels_the_job_in_flight` |
| `.git/index` event (staging) refreshes an automatically granted file; registration includes `index` and `HEAD` | `staging_a_change_to_an_automatically_granted_file_refreshes_it_through_the_index_watch` |
| slot syntax error reported passively at the definition, nothing validated | `a_slot_syntax_error_is_reported_passively_without_a_slot_job` |
| rescan fallback: a save re-checks by `stat` | `without_watchers_a_save_re_checks_the_slot_inputs_by_stat` |
| request path evaluates nothing before or after install (counter, with control) | `no_side_effects::schema_slot_requests_evaluate_no_slot_before_or_after_the_worker_resolves_it` |
| store: stale seq rejected, superseded job canceled; one demand per key; stale re-demands; `stat` change; missing file appears; once grant changes generation | `darkmatter-lsp slot_jobs::tests::*` (5) |
| editor work bound: 500 members resolve; runaway stops with `dm.iteration.*`; the command allowance resolves it after (failure not cached) | `slot_jobs::tests::the_editor_work_bound_resolves_a_realistic_list_and_stops_a_runaway_one` |
| open buffer defeats the `stat` shortcut (regression for the cache bug) | `darkmatter markdown::schemas::slots::session::tests::an_open_buffer_with_other_text_fails_the_check_though_the_stat_is_unchanged` |

The Input Robustness Matrix applies only to the `dmls.grantSlotRead`
argument, a JSON object read by serde. Every field is required (no
`#[serde(default)]`). These shapes are rejected: wrong type for the whole
argument, an unknown decision, an unknown scope, and a relative path (the
store's own judgment through `GrantTarget::parse`). Absent and null fields
are rejected by serde's required-field rule. The grant store loader is
unchanged and its matrix is Phase 5's. Smell grep over the new files: no
`unwrap_or_default()` or `.ok()` on a load-bearing parse. The two
`.ok()` calls in `diagnostics/slots.rs` serialize `data` and compare
canonical paths; neither decides an outcome.

### Gates

Run on macOS with no subagent running.

- `darkmatter` `just test`: 15473 passed, 10 skipped (37 slow), first run.
  `just lint`: passed (after one `manual_contains` fix); it passed again
  after the last edit. `just check-tier-coverage darkmatter`: 0 stranded.
- Gate skipping does not apply. `affected_scope.py` over the working tree
  (Phases 7 and 8, both uncommitted) reports `packages: [darkmatter,
  darkmatter-cli, darkmatter-lsp, test-toolkit, claudine, claudine-cli]`,
  `change_class: package`.
- Dependents:
  - `cargo check --all-targets -p claudine -p claudine-cli -p messenger`: clean.
  - claudine `test(/claudine_schema_shipped_corpus/)`: 67 passed.
  - claudine-cli `test(/research_sequence_artifacts|schema_completion|completion::engine/)`: 119 passed.
- Cross-OS (watch paths, canonical keys, grant-store paths):
  - `just cross-check darkmatter --os windows`: 11186 passed.
  - `just cross-check darkmatter-lsp --os windows`: first run 1778 passed and
    1 failed. The failure was the Phase 7 test
    `diagnostics::frontmatter::tests::a_schema_extension_holding_a_slot_is_refused_with_the_baseline_slot_code`.
    Its `canonical_tempdir` helper used raw `canonicalize` (`\\?\C:\…`), so
    the extension path did not resolve. The helper now uses
    `biscuit_file::canonicalize_simplified`. Second run: 1779 passed,
    including all 14 `schema_slots_editor` tests. Phase 7's cross-check had
    covered only the `darkmatter` package, which is why it was missed.
- Load-sensitive or pre-existing failures: none on macOS.
