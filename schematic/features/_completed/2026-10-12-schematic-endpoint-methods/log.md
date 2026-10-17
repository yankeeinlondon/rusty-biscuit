---
implementation_1: "2026-10-13T12:37:36-07:00"
implementation_2: "2026-10-13T12:58:27-07:00"
---

# Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-13T12:37:36-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'schematic/features/2026-10-12-schematic-endpoint-methods/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `## Human rulings` section still reads "(pending)" for all three items, but its frontmatter sets `human_resolved: true` and each `human_in_the_loop` item carries an author `**DECISION:**` line; those lines are taken as the rulings
        - item 1: accept `CallError::Api { error, meta: Box<ResponseMeta> }` (record the accepted departure)
        - item 2: keep the generic `FullCall`; document `.full()` as the final endpoint-specific step
        - item 3: keep homelab's `RestError::Call`; review conversions and documentation
- all three rulings confirm existing code, but each implies record or documentation work, so each was added to the review's `findings` frontmatter as a **high** finding (`high: carry out ruling on item N: …`)
- starting the work on 'ruling on item 1: keep CallError::Api named fields' at 12:41:10
        - defect class: documentation, rustdoc, or a skill describes the combined call error's API variant in a shape other than the struct variant with boxed metadata, or describes the departure as still awaiting a ruling
        - class sweep: `CallError::Api` shape (tuple, missing `meta`, or "awaiting ruling"); sites checked: `schematic/README.md`, `schematic/{runtime,gen,schema}/README.md`, `schematic/docs/runtime/{README,calling-an-api,sending-and-responses,credentials,metadata-and-redaction}.md`, `.claude/skills/schematic-define/{SKILL.md,schematic/SKILL.md,schematic/gen.md}`, `.claude/skills/{sniff,homelab}`, `sniff/docs`, `homelab/docs`, rustdoc in `schematic/runtime/src/{call,lib,client}.rs` and the generator's prelude, `lib_rs`, and endpoint-method doc emitters; fixed here: none; clean: all listed
        - consumer matches reviewed and correct: `homelab/lib/src/rest.rs` (status from `meta.status`, wildcard to `RestError::Call`), `sniff/lib/src/remote/{envelope,bitbucket,github}.rs`, `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`, and the schematic tests; the compile-fail fixture `call_error_match_needs_a_wildcard` pins the wildcard requirement
        - recorded the accepted departure in `implementation-log.md` under a new `## Review 1 rulings` section; the spec is left untouched as a snapshot
        - no Rust changed, so no test or lint run was needed for this item
- work completed for 'ruling on item 1: keep CallError::Api named fields' at 12:43:00
- starting the work on 'ruling on item 2: document .full() as the final endpoint-specific step' at 12:43:20
        - defect class: a document, rustdoc comment, generated doc text, or skill that describes the endpoint call surface omits that `.full()` ends access to endpoint-specific setters, shows a setter after `.full()`, or still describes the pre-feature `send(request, &CallOptions)` call shape
        - class sweep: `.full()` ordering and pre-feature send shape; sites checked: every `.md` under `schematic/` (excluding `features/`, `docs/research/`, `reviews/`), `schematic/skill/`, `.claude/skills/schematic-define/`, `Call::full` and `FullCall` rustdoc, the generator's `full` forward, call-type and endpoint-method doc emitters, generated `schematic/schema/src`; fixed here: `schematic/skill/SKILL.md` (still said callers send with `client.send(request, &CallOptions::new())`), `schematic/README.md`, `schematic/docs/runtime/calling-an-api.md` (per-call methods table), `schematic/gen/README.md`, `.claude/skills/schematic-define/schematic/gen.md`, `FullCall` rustdoc, generated call-type doc (`gen/src/codegen/endpoint_methods/docs.rs`); clean: `Call::full` rustdoc, the generated `full` forward ("Chain it last …"), `calling-an-api.md` `.full()` section, `sending-and-responses.md`, `.claude/skills/schematic-define/SKILL.md`, `.claude/skills/schematic-define/schematic/SKILL.md`, `runtime/README.md`, `schema/README.md`, `docs/runtime/README.md`
        - regenerated the schema crate with `just generate`: 15 files under `schematic/schema/src/` changed, doc lines only
        - pagination spec and plan (`2026-10-13-schematic-pagination`) never write `.full()`, so nothing shows the wrong order; its refresh must state the order wherever it introduces `.full()` (reported, not edited)
        - test added: `full_docs_say_the_endpoint_setters_come_before_it` in `schematic/gen/src/codegen/endpoint_methods/tests.rs`
        - `just test` (1925 + 6 + 1 passed), `just lint`, `just test-schema` (39/39), and `just lint-schema` all passed
- work completed for 'ruling on item 2: document .full() as the final endpoint-specific step' at 12:48:10
- starting the work on 'ruling on item 3: keep homelab RestError::Call' at 12:48:20
        - defect class: a consumer adapter that folds the non-exhaustive combined call error into its own public error loses information, misclassifies a variant, lacks a test of its mapping, or is documented inconsistently with its variants
        - class sweep: consumer fold of the non-exhaustive `CallError`; sites checked: `homelab/lib/src/{rest,eversolo,samsung_tv}.rs`, `sniff/lib/src/remote/{envelope,bitbucket,github}.rs`, `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`; fixed here: `homelab/lib/src/rest.rs` (`RestError::Call` used `#[error("{0}")]` with no source, so the inner cause vanished from the error chain because `CallError`'s `Display` leaves the cause to `source()`; now `#[error(transparent)]`; `Status`/`Response` rustdoc corrected), `homelab/docs/eversolo-design.md` (variant table); clean: `eversolo.rs`, `samsung_tv.rs` (delegate to `rest.rs`), sniff `envelope.rs` and biscuit-speaks `elevenlabs.rs` (string-based errors that keep the wildcard's message)
        - existing L1 mapping tests confirmed for Transport, error status, and decode failure in homelab, sniff, and biscuit-speaks; the wildcard arm has no producer today, which the ruling accepts
        - test added: `rest::tests::a_call_error_kept_whole_still_reports_its_cause` (homelab lib, L1); it fails with the old attribute and passes with the fix
        - `.claude/skills/homelab/SKILL.md` already mentions the `Call` variant; no homelab README documents `RestError`
        - recorded the ruling in `implementation-log.md` under `## Review 1 rulings`; the orchestrator added the matching Item 2 bullet there
        - `cd homelab && just test` (244 passed, 8 tier-skipped; vitest 22/22) and `just lint` passed
- work completed for 'ruling on item 3: keep homelab RestError::Call' at 12:52:00
- cross-OS testing was not run: the changes are documentation, a regenerated doc string, a `thiserror` attribute, and a loopback-socket test that reuses an existing helper; none has `#[cfg]`, path, or process logic, and CI's pull request covers Linux and macOS

### Successful Completion

The implementation of review cycle 1 has completed successfully in 15 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no finding was deferred
- no finding was disputed
- the review never compared the implementation with the specification, so this cycle does not establish production readiness; the next review must do that
- the review's `## Human rulings` section still reads "(pending)"; this cycle read the rulings from the author's `**DECISION:**` lines and `human_resolved: true`, and the next review should confirm that reading

## Implementation of Review Findings #2

> **started at:** 2026-10-13T12:58:27-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'schematic/features/2026-10-12-schematic-endpoint-methods/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review has one finding (high: call build skips pre-send argument validation outside request encoding) and two observations (calling guide overstates required query and header arguments; generated-schema behavior tests run locally but not in CI)
- starting the work on 'high: call build skips pre-send argument validation outside request encoding' at 12:59:02
        - defect class: a call's early validation runs only one stage of request preparation, so other deterministic argument failures surface only when the call is awaited
        - delegated to a foreground `rust-developer` subagent, which reports its log items back to the orchestrator
        - the send's first-hop preparation is now one function, `Inner::prepare` in `schematic/runtime/src/client.rs`; `Inner::execute` calls it, so send and build share the checks rather than duplicating them
                - it covers request encoding, header and body content-type assembly, the origin check, credential resolution, `.anonymous()` stripping, and credential placement; the deadline check and the redirect loop stay send-only
        - new `pub(crate) Client::check_arguments` runs `prepare` with the call's own per-call settings and reports its `InvalidArgument` failures; `Call::build` and `FullCall::build` call it, and the public return type `Result<Self, InvalidArgument>` is unchanged
                - `InvalidUrl`, `CrossOriginRejected`, deadline expiry, and invalid client settings are not argument failures and remain send-time; the rustdoc and calling guide say so
                - no generated request can produce `InvalidUrl` (path parameters are always percent-encoded); only a hand-written `EndpointSpec` returning a malformed absolute path reaches it
        - generated `build` rustdoc (gen `endpoint_methods/mod.rs`) now names the checks; shipped output regenerated: exactly 547 replacements of that one doc line in `schema/src`, `openapi/` and `postman/` byte-identical
        - fixed a pre-existing broken private rustdoc link on `Exchange` (`Client::execute` → `Inner::execute`) next to the change
        - tests: new `schematic/runtime/tests/l1/call_build.rs` (12 L1 tests, each about 0.02 s), each case asserted through `Call::build`, `FullCall::build`, and the await of generic `client.send(request)` with no request reaching the server
                - unsupported credential kind; token ending in newline; invalid header name; header value ending in newline; raw body content type ending in newline; request `Authorization` plus per-call bearer; query slot conflict; cookie slot conflict; client-level credential conflict; retained `into_parts` rejection; `.anonymous()` parity; valid control that builds without sending and then sends exactly one request
                - red/green: with `build` reverted to only `into_parts`, 9 of the 12 failed
                - schema crate: new `build_refuses_a_credential_the_send_would_refuse` in `schema/tests/github_client.rs` reproduces the review's GitHub Basic-credential case
        - docs updated: `schematic/docs/runtime/calling-an-api.md` (`.build()?` section and method table), `.claude/skills/schematic-define/SKILL.md`, `.claude/skills/schematic-define/schematic/SKILL.md`
        - verification: `just test` (schematic) 1937 passed / 12 skipped plus fresh-output 6/6 and compile-fail 1/1; `just lint` passed; `just test-schema` 40/40; `just lint-schema` passed; `just check-tier-coverage schematic` 0 stranded; `cargo doc -D warnings` (incl. private items) passed
        - class sweep: a call's early validation runs only one stage of request preparation, so other deterministic argument failures surface only at await; sites checked: `Call::build`, `FullCall::build`, generated `{Endpoint}Call::build` forwards, `Client::send` → `Inner::execute`, `Client::send_raw` → `Inner::execute`, `Client::check_link`, `api_client!` `build`, `Client::build`; stages: into_parts, assemble (header name/value, body content type, URL), origin link, `auth.resolve`, anonymous stripping, `authenticate`/`place_credential` (header, query, cookie, credential header value), deadline expiry; fixed here: `Call::build`, `FullCall::build`, generated call `build` (by delegation, rustdoc regenerated); clean: `send`/`send_raw` (already ran the whole preparation, now via shared `prepare`), `check_link` (URL only, no request arguments), `api_client!` `build` and `Client::build` (client settings only), deadline expiry, origin refusal, `InvalidUrl` (non-argument kinds, send-time by design)
- work completed for 'high: call build skips pre-send argument validation outside request encoding' at 13:06:03
- starting the work on 'observation: calling guide overstates required query and header arguments' at 13:06:30
        - defect class: documentation describes endpoint-method arguments as including required query, header, or cookie values, while the generator takes only path values and then the body
        - class sweep: endpoint-method required-argument descriptions; sites checked: `schematic/docs/runtime/calling-an-api.md`, `schematic/README.md`, `schematic/{gen,runtime,schema,define,definitions}/README.md`, `schematic/docs/`, `schematic/skill/SKILL.md`, `.claude/skills/schematic-define/`, emitted rustdoc in gen `endpoint_methods/{mod,docs}.rs` (`required_args` documents "each path parameter, then the body"); fixed here: `schematic/docs/runtime/calling-an-api.md`; clean: all others
        - the guide now says arguments are each path value and then the body, in `{Endpoint}Request::new` order, and that query, header, and cookie values are always setters and are not enforced as required; no test reads the guide
- work completed for 'observation: calling guide overstates required query and header arguments' at 13:06:30
- starting the work on 'observation: generated-schema behavior tests run locally but not in CI' at 13:06:30
        - no code change: the review leaves scheduling the excluded `schematic-schema` suite to the author and adds no CI gate; the arrangement predates this feature
        - this cycle's evidence for the high finding is placed in the schematic-runtime L1 binary (`tests/l1/call_build.rs`), which CI runs; the schema-crate test is a supplementary local check
- work completed for 'observation: generated-schema behavior tests run locally but not in CI' at 13:06:30

### Successful Completion

The implementation of review cycle 2 has completed successfully in about 8 minutes (12:58:27 to 13:06). During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no findings were deferred
- no findings were disputed
- of the two observations, the calling-guide wording was corrected; the CI scheduling of the `schematic-schema` suite is an author decision the review explicitly left open, so no change was made

The files changed in this cycle are `schematic/runtime/src/{client,call}.rs`, `schematic/runtime/tests/l1/{call_build,main}.rs`, `schematic/gen/src/codegen/endpoint_methods/mod.rs`, the regenerated `schematic/schema/src/**` (one rustdoc line per call type), `schematic/schema/tests/github_client.rs`, `schematic/docs/runtime/calling-an-api.md`, `.claude/skills/schematic-define/SKILL.md`, and `.claude/skills/schematic-define/schematic/SKILL.md`.
