---
spec: /Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-13-schematic-pagination/spec.md
plan: schematic/features/2026-10-13-schematic-pagination/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-10-13-schematic-pagination (8 phases)

## Phase 1

Phase 1 is a preflight phase. It changes no production source, test, doc or skill. It reconciles the spec with the delivered prerequisites, records the coverage inventory and the migration ledger, captures the artifact baseline, and decides whether the input robustness matrix applies. Checked on 2026-10-13 against HEAD `0cd6075e6` on `feat/modern-config`.

The four investigations ran in parallel as read-only subagents. Their full evidence reports are appended under "Evidence" below. Scratch artifacts, including the generated-output baseline and file hashes, are in `/tmp/pagination-phase1/`; they are temporary and can be regenerated.

### Prerequisite gate: PASS

All three prerequisites are in `_completed`, each spec has `status: implemented`, and each review 3 is `ready: true`:

- `2026-10-11-schematic-generation-integrity`
- `2026-10-12-schematic-runtime-envelope`
- `2026-10-12-schematic-endpoint-methods`

Every contract the spec needs exists in code:

- `schematic-runtime` 0.1.0, with `ABI = 3` matching the generator's `RUNTIME_ABI = 3`, and no `pager` module yet
- `EndpointSpec`, the `Json`/`Text`/`Binary`/`Empty` codecs, `Response`/`Outcome`/`into_parts`, and `RequestMeta.page`
- `TransportErrorKind::{CrossOriginRejected, InvalidArgument, DeadlineExceeded, …}` and `ErrorKind::RedirectFailed { target, category }`
- `CallError`, `Call::map_request`, `.full()` and `.build()`
- `Client::check_link::<R>(&Url)` and the checked absolute-URL request path
- `Deadline` and `send_raw`
- partial generation, the isolated fresh-output compile check, query separation, and typed `ValidationDiagnostic`

Phase 2 is not blocked. File:line citations are in the contract map below.

### Integration corrections found during reconciliation

The spec is a snapshot and is not edited. These are the corrections later phases apply; each cites the contract map.

1. **`validate_api` does not exist.** The validator entry point is `validate_rust_client(apis) -> Vec<ValidationDiagnostic>` in `schematic/gen/src/validation.rs:138`, together with the per-output validators. Pagination validation is wired there (Phase 2.2).
2. **The diagnostics format is already fixed.** It is `severity= api= endpoint= feature= output=: message`, from `ValidationDiagnostic` in `schematic/gen/src/diagnostics.rs`. The P8.3 default `<api>.<endpoint_id>: <rule>: <detail>` is aligned to it: the rule becomes new `Feature` ids, and the nearest-id hint goes in the message.
3. **There is no public relative-URL resolver, and the envelope exposes only a redacted final URL.** `ResponseMeta.request.url` is a `RedactedUrl`. The runtime resolves only redirect `Location` headers, internally (`client.rs:939`). Because the pager lives inside the runtime crate, it retains the unredacted final URL through an in-crate seam and resolves with `Url::join`. The same seam is the place to add a shared resolver; no public API is needed (Phase 4.2).
4. **The checked absolute-URL path works only through `RequestParts.path`.** A generated request cannot carry an arbitrary URL, so the pager follows a link through a runtime-internal wrapper `EndpointSpec`. The wrapper keeps the listing request's `Api`, `Success`, `Failure`, `Decoder` and `ENDPOINT_ID`, so `check_link::<R>` and origin refusal report the listing endpoint (Phase 4.2).
5. **`RequestMeta.page` has no input.** The pager sets it, in-crate, on each paged send's metadata (Phase 4.2).
6. **The `RestApi` literal count is larger than 18.** `RestApi` has no `Default`. Adding two fields touches:
   - the 18 production literals
   - 3 registry test literals
   - the importer builder (`schematic/define/src/openapi/import/builder.rs:362`)
   - the definition emitter (`schematic/gen/src/definition_gen.rs:141-159`)
   - many test literals in define and gen (about 122 raw `RestApi {` grep hits)

   Phase 2.1 owns all of them.
7. **`trybuild` and `wiremock` are already dev-dependencies of `schematic-runtime`** (`schematic/runtime/Cargo.toml:31-32`) and are already listed in both dependency docs. The spec's pre-authorization to add them is moot. The existing trybuild harness clears `RUSTC_WRAPPER`, which addresses the snapshot-portability risk the spec raises about kache-remapped paths.
8. **Only three pager names are reserved.** The generator's reserved list holds `pages`, `fetch_page` and `collect_all` (`PLANNED_CALL_METHODS`, `schematic/gen/src/codegen/endpoint_methods/names.rs:39`). Every new knob and entry name joins that list in Phase 3.2: `page_size`, `page`, `offset`, `after`, `before`, `prev`, `keep_alive`, `partitioned`, `set_page_size`, `total`, `with_session`, `renew`, `page_limit`.
9. **The ABI will change.** Under `schematic/runtime/src/lib.rs:96-101`, any change generated code can observe bumps the ABI. Generated pager impls will observe the new runtime traits, so Phase 3 bumps the runtime `ABI` and the generator's `RUNTIME_ABI` together, from 3 to 4. No ruling covers this; it is recorded as a design choice.
10. **The policy constructor's name differs.** The spec text says `allow_any`; code uses `danger_allow_any`. Tests use the code's name.
11. **`InvalidArgument` has no kinds.** It is a struct with a private reason. Page-size bounds, zero limits and arithmetic overflow use `InvalidArgument::new(reason)`, and the reason never quotes the value.
12. **The runtime `Call` fields are private to generated code,** and the generated wrapper is a private tuple field `.0`. Pager entry points are therefore runtime methods on `schematic_runtime::Call<'a, A, R>`, bounded by the pager traits, and the generated inherent forwards call them through `self.0`.
13. **The session-close grace has no spec-2 ruling.** The runtime-envelope log assigns it to this spec, so default 21 (5 s, capped by the per-request timeout) stands as Proposed.

### Authorization record

The full search is under "Authorization record" in the contract map. In summary:

| Step | Recorded confirmation | Treatment |
| --- | --- | --- |
| Regenerate `schematic-schema` | Not recorded | Required scope |
| Dependent-crate migrations (sniff, biscuit-speaks) | Not recorded | Required scope |
| Runtime pager module | Not recorded | Required scope; the dev-dependencies are already present |
| Definition authoring (`RestApi` fields, defaults, tables, overlay) | Not recorded | Required scope |
| Docs, skills and READMEs | Not recorded | Required scope (repo drift rule) |
| Network verification of U rows (optional) | Not recorded; explicitly PENDING | **Skipped**: U rows stay unpaginated and are listed as the R33 verification task |
| EMQX `ListAuthUsers` response fix (optional) | Not recorded; explicitly PENDING, and conditional on verification | **Skipped**: left unchanged and unpaginated |

Recorded as the plan requires: the only instruction covering the required-scope steps is the author's run instruction to implement this plan phase by phase. The runtime-envelope run used the same basis (`2026-10-12-schematic-runtime-envelope` implementation log). Nothing is committed, so every step stays reversible. `yolo: true` in the plan frontmatter is not treated as a grant. The two optional steps are not taken in any phase unless the author records approval.

### Inventory decisions for later phases

The recount is in "Coverage inventory" below. Decisions taken from it, smallest option consistent with the spec, are adopted as defaults here and applied in Phase 6:

- **Counts.** 18 REST APIs; 547 endpoints (= 547 request types = 547 call types; OpenAI 265); 511 production `Endpoint` literals (the spec's ~519 is stale); 35 legacy declarations (GitHub 9, GitLab 9, Bitbucket 9, Gitea 8); 156 collection endpoints (157 if Bitbucket `GetSource` is counted); 40 GET root `Vec<T>`; 61 OpenAI list candidates (S6 36, item-id 2, S7 9, S8 11, opt-outs 3).
- **Bitbucket has 8 wrapper lists, not 9.** `ListDirectoryContents` was merged into `GetSource` (a `Binary` response) by `9e65a90b5` on 2026-10-12. `GetSource` is not a list, and its `page`/`pagelen` fields stay.
- **Bitbucket `page` stays a request field on the 8 lists.** R7's knob rule decides this: `NextLink` has no page input, so nothing is adopted. This overrides R7's summary phrase "every `with_pagination` query parameter". `pagelen` is adopted through the authored page-size knob.
- **ElevenLabs `ListVoices` stays unpaginated.** The definition sends `page_token`, but the research note names the input `next_page_token`. The authoring table's contradiction rule ("record the discrepancy and leave that endpoint unpaginated") applies, so its fields are kept. It goes on the R33 verification list, and biscuit-speaks keeps its hand loop. Expected paginated count: 94 (GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2).
- **Two GET `Vec<T>` endpoints get opt-out entries:** GitLab `ListIssueParticipants` (unverified) and Gitea `GetTagReference` (not a list). Every other GET `Vec<T>` under a default is a real list.
- **The U-row wrapper collections get explicit unpaginated entries,** so the coverage test can assert `Unpaginated` as R28 requires:
  - Anthropic `ListModels`
  - the 7 EMQX wrapper lists in each of the two variants
  - Hugging Face `ListModelDiscussions`
  - the 2 Gitea tree endpoints

  Other wrapper collections with no pagination evidence resolve as "not a list" with no entry: Artificial Analysis, LM Studio, Ollama, and the 3 other ElevenLabs wrappers. Hugging Face's 10 and EMQX `ListAuthUsers`' root arrays have no API default, so they already resolve as unpaginated lists.
- **OpenAI page-size bounds vary by endpoint** (100, 10000, 1000, 180, and bucket-width-dependent maxima). The S6 default `PageSize` therefore carries no `max`, and a per-entry bound is authored only where the vendored document states one that holds for every request. That avoids R13 rejecting valid requests.
- **OpenAI cursor fields are typed `String`, `Option<String>` or `serde_json::Value`.** The cursor selector reads JSON string-or-null from the raw page, independent of the Rust field type.
- **GitLab keyset constants share names with declared params on `ListGroupProjects`** (`order_by`, `sort`). Alternates are not generated (P8 item 7), so Phase 2 validates an alternate's constants within the alternate only, and logs this as an open question for when alternates are emitted.
- **No adoptable parameter is provider-required,** so R2a needs no authored default for any real definition. R2a stays covered by model and runtime tests.

### Migration ledger (adopted-field removals)

The full ledger is in the evidence below. Actual dependents of `schematic-schema`: sniff, biscuit-speaks, and five homelab crates (`homelab`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`). **model-citizen does not depend on `schematic-schema`,** contrary to the spec; it uses `schematic-definitions` types only.

| Site | Use | Migration (Phase 7.1) |
| --- | --- | --- |
| `sniff/lib/src/remote/github.rs:454` | `list_workflow_runs(..).per_page(limit)`, one page; anonymous retry on 401/403 | `.page_size(limit).fetch_page()`; per-call `.anonymous()` must survive into the pager |
| `sniff/lib/src/remote/bitbucket.rs:177,179` | `GetSource` `.pagelen`/`.page` hand loop | none (`GetSource` is not a list; fields kept) |
| `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576,578` | `ListVoices` `.page_size`/`.page_token` loop | none while `ListVoices` stays unpaginated |
| `schematic/schema/tests/github_client.rs:549,556` | `per_page` | `fetch_page()` in the test |
| `schematic/schema/tests/non_json_endpoints.rs:65` | `GetSource` `pagelen` | none |
| `schematic/gen/tests/l1/fresh_output_compile.rs:1588-1698` | wire-baseline program sets Gitea `page`/`limit` and OpenAI `limit`/`after` | migrate the program; re-baseline the 4–5 affected lines of `query_request_lines.txt`, recorded as an intended change |
| homelab crates, model-citizen | none | none |

Documentation showing adopted setters (`schematic/docs/runtime/calling-an-api.md`, `request-side.md`, `deadlines-and-timeouts.md`, and the `schematic-define` skill) is drift that Phase 8.1 corrects.

### Input robustness scope: configuration-reader matrix does not apply

`RestApi`, `Endpoint`, `EndpointParams`, `ParamDef` and `PaginationStyle` derive neither `Deserialize` nor `Serialize`. Every define-side `from_str` is test-only, and the generator CLI takes an API name, not a definition file. The only file reader is the OpenAPI importer. It reads an OpenAPI document, an import format that produces no pagination, and this feature does not change its parsing.

**Boundary recorded:** serde on the new pagination model (R1) is a model round-trip facility with no configuration reader, so the per-field input robustness matrix is not built. Phase 2.3 keeps strict model round-trip and decode tests, and invents no loader:

- unknown fields rejected
- absent kept distinct from null where the model distinguishes them
- empty collections kept distinct from absent ones

The R12/R17 response-shape tests in Phases 3 and 4 remain required. If a later phase adds a reader, the matrix is built before that reader.

### Baseline (task 1.3)

All on macOS (M4 Max, 16 CPUs), rustc 1.98.1, nextest 0.9.136, kache 0.26.3 as `rustc-wrapper`. No pre-existing failures.

| Check | Result |
| --- | --- |
| Fresh generation into a temp dir vs committed `schematic/schema` | byte-identical (empty `diff -r`) |
| Fresh OpenAPI (15 files) and Postman (15 files) vs committed | byte-identical; sha256 of all 73 generated files recorded |
| `just test` (schematic area; includes runtime, fresh-output and compile-fail) | pass, 51.2 s: L1 1937/1937 (12 skipped), fresh-output 6/6, compile-fail 1/1 |
| `just lint` (includes runtime) | pass, 76.5 s |
| `just test-schema` | pass, 40/40 |
| `just lint-schema` | pass |
| `schematic-schema` cold build, all targets | 44.7 s with kache, 69.8 s without; library alone 35.5 s; cold clippy 60.6 s |

Recipe coverage: `just test` and `just lint` include `schematic-runtime`, so the plan's "area recipes must include the runtime package" holds with no recipe change. The generated surface has 547 request structs, 547 call types, 35 legacy `impl Paginated`, and 197 pagination-looking request fields across 79 structs.

### Requirement-to-test mapping for this phase

Phase 1 changes no behavior, so it adds no test. The evidence is the recorded baseline: the full L1 suite, lint, schema suite, and generation and export byte-equality. The input-matrix decision above sets what Phase 2.3 must test.

### Deviations in this phase

- Tasks 1.1, 1.2 and 1.3 ran concurrently. The plan orders 1.1 before 1.2 and 1.3 to gate them. The gate was confirmed first, with a direct check that the prerequisite symbols exist, and the three tasks are read-only, so running them together changed no outcome.
- The input-applicability survey (the last bullet of 1.3) ran with the 1.2 dependents survey because both are code-reading tasks.

### Human review agenda (status slots)

The agenda as written in the spec, copied verbatim:

> 1. Every default adopted from "Planning-time details" (P8 items 1 to 8, and defaults 9 to 22), with the value actually used.
> 2. The reconciliation of spec 2's actual names and kinds with this spec's names (runtime package name, `EndpointSpec` trait, `TransportError` kinds, the page field of `RequestMeta`, cross-origin and deadline entry points).
> 3. Every design choice not dictated by this spec: names of the define-side and runtime-side types, the shape of `PagerError`, the `with_session` signature, the entry-point method names, the generated constant layout, which rules from design.md section 8 were kept, renamed, or dropped.
> 4. Every request-struct field removed under the adoption rule, and every dependent-crate edit it forced.
> 5. The authored versus left-unpaginated list: per API, the endpoint ids authored, the ids left unpaginated, the reason (unverified, provider gap, definition gap, not a list), and the source for any row the implementer verified. This is the verification task of R33.
> 6. The re-counted numbers (list endpoints, declared endpoints, endpoint literals) and how they differ from the parent's.
> 7. The quick decode-path sample: the one host used, release-binary size before and after at the current endpoint count, the decision, and the numbers; no adoption percentage gate.
> 8. Compile-time and diagnostic observations: build time (`just lint-schema` / `just test-schema`) for `schematic-schema` before and after; the implementor-list truncation; any rust-analyzer observation.
> 9. Which operating systems and environments the tests were actually run on, and which are left to CI; any test made tolerant for Windows timers or paths; any trybuild snapshot made portable and how.
> 10. Every deviation from this spec, with the reason; every requirement not met or met differently.
> 11. Every open design question found, with the implementer's recommended answer. Candidates already known: whether to expose labeled alternates; whether `deny` should follow same-origin next links; whether to keep the table's nearest-id hint; whether partition support should ship now or wait for a first real user.
> 12. Drift found between comments and code, and how it was resolved.
> 13. Anything done under a pre-authorization that the author may want to reverse (regeneration diff size, edited dependents, the `ListAuthUsers` fix if taken).
> 14. The **Proposed** items that carry no ratification, each with the value used and the alternative: R14's widened stall rule (any repeat anywhere versus consecutive only); R9/R10's one generic pager, trait-bound encoding, and diagnostic attributes; the pager decode path (S11); named-struct strategy variants (S12).
> 15. The adoption rule (R7, default 12): which plain parameters were adopted, that each adopted one has its matching knob and is reachable for a single page through `fetch_page()` (R11a, D57 item 3), and every parameter whose field was kept because the strategy authors no matching knob.
> 16. The session-close grace (R22a, default 21): the grace value used and whether spec 2 ruled otherwise.
> 17. The `InvalidContinuation` incomplete reason (R21, P8 item 1) and the additional argument/transport mappings in `IncompleteReason`, and the wrapper-list item type default (default 22 / R26).
> 18. The stale-rule widening (R14): any test that depended on consecutive-only detection, and any provider whose legitimate pagination repeats a continuation.
> 19. Provenance only, no open question: D55-5 (pager-managed parameters exempt from required fidelity, R2a, R7), D56-1 (anonymous send when no credential resolves), D56-3 (per-request time under a deadline), D56-5 (`EndpointSpec` name), and D57 items 2, 3, 4, 5, 7, and 13 (latest handle across partitions, `fetch_page()`, the importer overlay, `SessionError`, the `RedirectFailed` mapping, and `Default` on request structs) are Decided; the log records only where the implementation applied them and any difference found when reconciling with spec 2 and spec 4b.
> 20. The rotating-handle partition risk (R23, D57 item 2): that it was logged, and any out-of-order handle or provider handle invalidation observed in tests or documentation.

Status slots, updated by each phase that produces evidence:

1. **Defaults adopted** (P8 items 1–8, defaults 9–22). *Phase 1:* confirmed that no prerequisite rules over them, except that default 17 (`deny` follows same-origin links) is satisfied by the landed runtime (`origin.rs:10-24`). The grace in default 21 has no spec-2 ruling. The other values are applied in later phases. *Status:* open.
2. **Reconciliation with spec 2.**
   - *Phase 1:* done; see "Integration corrections".
   - Names that match: the runtime package `schematic-runtime`, `EndpointSpec`, `TransportErrorKind::{CrossOriginRejected, InvalidArgument, DeadlineExceeded}`, `RequestMeta.page: Option<u32>`, `Client::check_link::<R>`, and `Deadline`/`.deadline(&Deadline)`.
   - Differences: `validate_api` (actually `validate_rust_client`), the resolver, the absolute-URL seam, and `danger_allow_any`.
   - *Status:* done.
3. **Design choices** (type names, `PagerError`, `with_session` signature, entry names, constant layout, section 8 rules). *Phase 1:* the ABI 3→4 bump, runtime-side entry points on `Call`, and in-crate seams were chosen. *Status:* open for later phases.
4. **Removed request fields and dependent edits.** *Phase 1:* the ledger is planned (above). *Status:* open (Phase 7).
5. **Authored versus left-unpaginated list, and verification task (R33).**
   - *Phase 1:* inventory recorded.
   - Unverified and left unpaginated:
     - Anthropic `ListModels`
     - EMQX (both variants)
     - Hugging Face (11)
     - GitLab `ListIssueParticipants`
     - Gitea trees (2)
     - ElevenLabs `ListVoices` (input name)
   - Provider gaps: OpenAI `RetrieveVectorStoreFileContent` and `SearchVectorStore`.
   - Definition gap: EMQX `ListAuthUsers`.
   - Not a list: Gitea `GetTagReference`, Bitbucket `GetSource`.
   - *Status:* open (Phase 6 and 8).
6. **Re-counted numbers.** *Phase 1:* done (above). 18 APIs; 547 endpoints; 511 production literals vs ~519; 35 declared vs the parent's 27; 156 collections vs ~157; OpenAI 265/61. *Status:* done.
7. **Decode-path sample.** *Phase 1:* fact recorded. Unpaginated JSON already uses `serde_json::from_slice` (`codec.rs:252-254`), and only hooked decoding uses `from_value`. *Status:* open (Phase 4.1).
8. **Compile-time observations.** *Phase 1:* baseline recorded. `schematic-schema` cold build takes 44.7 s with kache and 69.8 s without (library alone 35.5 s); cold clippy 60.6 s; host load average was 30–41 during measurement. *Status:* open (after-build in Phase 7).
9. **OS and environments.** *Phase 1:* macOS only; no OS-specific work. *Status:* open (Phase 8.2).
10. **Deviations.** *Phase 1:* concurrent preflight tasks (above). *Status:* open.
11. **Open design questions.**
    - Candidates from the spec: expose alternates, `deny` with same-origin (answered by the runtime: followed), the nearest-id hint, and partition timing.
    - New in Phase 1:
      - may an alternate's constant shadow a declared plain param (GitLab `ListGroupProjects`)?
      - the ElevenLabs `ListVoices` input name
    - *Status:* open.
12. **Comment and code drift.** *Phase 1:* drift detected, not yet corrected:
    - the `Paginated` doc in `schematic/gen/src/codegen/traits.rs` describes a nonexistent `fetch_all_pages`
    - runtime docs show adopted setters

    Both are corrected when the code changes (Phases 3 and 8). *Status:* open.
13. **Pre-authorized actions the author may reverse.** *Phase 1:* none taken. The authorization record is above. *Status:* open.
14. **Unratified Proposed items** (R14, R9/R10, S11, S12). *Phase 1:* none applied yet. *Status:* open.
15. **Adoption rule (R7).** *Phase 1:* planned adoptions are in the inventory. Kept fields: Bitbucket `page` ×8, `GetSource` `page`/`pagelen`, OpenAI `order`, and ElevenLabs `ListVoices` `page_size`/`page_token`. *Status:* open.
16. **Session-close grace.** *Phase 1:* spec 2 made no ruling, so 5 s is the default. *Status:* open (Phase 5).
17. **`InvalidContinuation` reason, argument/transport mappings, wrapper item type default.** *Phase 1:* confirmed that `InvalidArgument` (struct) and `RedirectFailed` exist to map onto. *Status:* open (Phase 4).
18. **Stall-rule widening.** *Status:* open (Phases 3 and 4).
19. **Decided provenance.** *Phase 1:* the landed runtime matches each of these:
    - D56-1 (anonymous when no credential resolves; `credentials.rs:295-314`)
    - D56-3 (`Expiration::start`, `deadline.rs:104-123`)
    - D56-5 (`EndpointSpec`)
    - D57 item 7 (`RedirectFailed`)
    - D57 item 13 (`Default` rule)

    *Status:* open for application in later phases.
20. **Rotating-handle partition risk.** *Status:* open (Phase 5).

### Validation run in this phase

The full baseline above is the validation: `just test`, `just lint`, `just test-schema`, `just lint-schema`, fresh-output and compile-fail all pass, and no file in the repository changed apart from this log and the plan and spec frontmatter.

### Evidence

#### Pagination Phase 1, task 1.1: contract map

Worktree `/Volumes/coding/wt/rusty-biscuit/feat-modern-config`, checked 2026-10-13 against HEAD `0cd6075e6`. The only working-tree change is the untracked `schematic/features/2026-10-13-schematic-pagination/implementation-log.md` (12 lines, preflight header only).

Abbreviations:

| Short form | Path |
| --- | --- |
| RT | `schematic/runtime/src/` |
| GEN | `schematic/gen/src/` |
| DEF | `schematic/define/src/` |
| PG | `schematic/features/2026-10-13-schematic-pagination/` |
| RE | `schematic/features/_completed/2026-10-12-schematic-runtime-envelope/` |
| EM | `schematic/features/_completed/2026-10-12-schematic-endpoint-methods/` |
| GI | `schematic/features/_completed/2026-10-11-schematic-generation-integrity/` |
| MR | `schematic/reviews/2026-07-31-macro-review/spec.md` |

##### Prerequisite gate verdict

**PASS. No prerequisite contract is absent.** Phase 2 is not blocked on runtime contracts. Some integration corrections must be recorded first; they are listed under "Discrepancies with spec refresh notes" below.

All three prerequisites are in `_completed`. Each spec has `status: implemented` and each review 3 has `ready: true`:

- GI spec.md:22, review-3.md:10
- RE spec.md:22, review-3.md:3
- EM spec.md:22, review-3.md:3

Every contract that spec "Dependencies and ordering" (PG spec.md:322) requires exists in code:

- the runtime package
- `EndpointSpec` and typed send
- `Response`, plus `RequestMeta` with its page field
- `TransportError` with `CrossOriginRejected` and the pre-send `InvalidArgument` kind
- `ErrorKind::RedirectFailed`
- the managed client and credential resolution
- the origin policy and `check_link::<R>`
- `Deadline`
- `send_raw`

The generation-integrity deliverables are also present: partial generation, the isolated compile check, query separation, and typed diagnostics.

##### Contract map

Status key: **M** = matches the spec/refresh notes; **D** = differs (explained); **A** = absent.

###### 1. Runtime package, ABI, modules

| Spec expectation | Code | Status |
| --- | --- | --- |
| `schematic-runtime` 0.1.0 at `schematic/runtime` | `schematic/runtime/Cargo.toml:2-3` | M |
| Runtime `ABI` = 3 | RT lib.rs:102 `pub const ABI: u32 = 3;` | M |
| Generator `RUNTIME_ABI` = 3, equality enforced | GEN lib.rs:101. The equality test at GEN lib.rs:117-141 parses the runtime's lib.rs. The generated crate asserts it at `schematic/schema/src/lib.rs:68-70` (emitted by GEN output/assemble/lib_rs.rs:139-150). Partial generation checks the marker at GEN output/inventory.rs:136, 440-480. | M |
| Modules include `call`, `client`, `endpoint`, `codec`, `response`, `error`, `meta`, `origin`, `deadline`; no `pager` | RT lib.rs:51-67 also has `api_client` (private), `credentials`, `encode` (private), `hooks`, `options`, `raw`, `redact`, `transport`. No `pager` module exists. `grep pager\|paginat RT` hits only meta.rs:64-65. | M |
| ABI impact of adding a pager | The rule at RT lib.rs:96-101 is "Increment it with every change that generated code can observe". Generated code that implements pager traits would observe them, so an ABI bump is likely. This is not ruled anywhere; record it as a decision. | note |

###### 2. `EndpointSpec` and codecs

| Spec expectation | Code | Status |
| --- | --- | --- |
| `EndpointSpec` with `Api`, `Success`, `Failure`, `Decoder`, `ENDPOINT_ID`, `METHOD`, `into_parts` | RT endpoint.rs:29-61. `Api: Api` :31, `Success: Send + 'static` :33, `Failure: ErrorBody` :36, `Decoder: Decode<Output = Self::Success>` :38, `ENDPOINT_ID` :41, `METHOD: Method` :45. Also `DECLARED_ERROR_STATUSES` (default 400..=599) :49. `fn into_parts(self) -> Result<RequestParts, InvalidArgument>` :60. The trait bound is `Send + 'static` :29. | M (one extra const) |
| `RequestParts` | RT endpoint.rs:80-94. Fields `method`, `path`, `query: Vec<(String,String)>` (unencoded, duplicates kept), `headers`, `body: RequestBody`. All fields are `pub`, so the parts can be rebuilt for a follow-up request. | M |
| Codecs `Json`, `Text`, `Binary`, `Empty` | RT codec.rs:242 / 279 / 291 / 303. Re-exported at RT lib.rs:71. | M |
| `Decode` uses a live body | RT codec.rs:33-54: `fn decode(body: LiveBody) -> impl Future<Output = Result<Self::Output, DecodeFailure>> + Send`, plus `decode_hooked`. `LiveBody` (codec.rs:61) has `collect()` (:162), `chunk()`, `status()`, `headers()` (unredacted, :125). | M |
| S11 decode path (from_slice vs from_value) | `Json::decode` collects the body and calls `serde_json::from_slice` (codec.rs:252-254). `decode_hooked` with a `pre_response_json` hook parses to `Value` and then runs `from_value` (codec.rs:264-268). Unpaginated sends therefore use `from_slice` when there are no hooks. A pager that decodes the page type through `R::Decoder` reuses that path. Item extraction from the decoded page is the open S11 question. | M (fact) |
| Decode failure evidence | `DecodeFailure { kind, retained: Bytes }` RT codec.rs:207-221. `DecodeFailureKind::{Interrupted, Malformed, Hook}` :225-233. | M |

###### 3. Envelope

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Response<T,E>` | RT response.rs:27-32 `{ pub meta: ResponseMeta, pub outcome: Outcome<T,E> }` | M |
| `Outcome` | RT response.rs:36-41 `Success(T) \| Failure(ApiError<E>)` | M |
| `into_parts()` / `into_result()` | response.rs:70-72 `into_parts(self) -> (ResponseMeta, Outcome<T,E>)`. response.rs:62-67 `into_result(self) -> Result<T, ApiError<E>>` discards the metadata. Also `data()`, `error()`, `is_decode_failure()`, `is_interrupted()` (:45-84). | M |
| `ApiError<E>` | response.rs:98-104 `{ kind: ErrorKind<E>, body: Bytes }` (raw body kept) | M |
| `ResponseMeta` | RT meta.rs:117-134 `{ request: RequestMeta, status, headers (redacted, repeated kept), request_id: Option<String>, elapsed }`, `#[non_exhaustive]`. Elapsed is the whole send (meta.rs:128-133). | M |
| `RequestMeta.page: Option<u32>`, left unset by ordinary sends | meta.rs:64-66, documented "Reserved for pagers ... the runtime always leaves it `None`". Set to `None` in `capture` (:86) and `unassembled` (:103). No input sets it: `CallOptions` has only `auth` and `deadline` (RT options.rs:16-19). The fields are `pub` but the struct is `#[non_exhaustive]` (meta.rs:50), so code inside the runtime crate can assign `meta.request.page` after a send. | M. Pager must supply the input (RE implementation-log.md:3625-3631, 3939). |
| Final response URL for relative-link resolution | Exposed only as `ResponseMeta.request.url: RedactedUrl` (meta.rs:57). This is the hop that received the final response (meta.rs:44-48), with secret query values and userinfo redacted. `RedactedUrl::as_str()` is at RT redact.rs:217-224. The unredacted URL is not exposed. | D (see Discrepancies) |

###### 4. Errors

| Spec expectation | Code | Status |
| --- | --- | --- |
| `TransportError` | RT error.rs:16-26 `{ pub request: Box<RequestMeta>, pub kind: TransportErrorKind, pub elapsed: Duration }`, `#[non_exhaustive]`. Constructor `new` :30. | M |
| `TransportErrorKind` (full list) | RT error.rs:48-81: `CrossOriginRejected { target: RedactedUrl }`, `InvalidArgument(InvalidArgument)`, `InvalidUrl`, `Connect`, `Tls`, `Timeout`, `DeadlineExceeded { completed_requests: u64, deadline_elapsed: Duration }`, `InvalidClient(BuildError)`, `Other`. Marked `#[non_exhaustive]`. | M |
| Page-size error kind | There is no dedicated variant. `InvalidArgument` is a struct with a private `reason: Cow<'static, str>` (error.rs:109-124), built by `InvalidArgument::new(reason)`. It has no kinds or variants. Display is `"invalid argument: {reason}"` (:126-130). The docs say the reason must not quote the value (:107-108). A page-size bound error has to be `TransportErrorKind::InvalidArgument(InvalidArgument::new("..."))` or a new kind. | M (refresh note correct). Spec Design :240 leaves this open. |
| `ErrorKind<E>` (full list) | RT response.rs:140-168: `Declared(E)`, `Undeclared`, `MalformedError`, `MalformedSuccess`, `BodyInterrupted`, `RedirectRefused { reason: RedirectRefusal }`, `RedirectFailed { target: RedactedUrl, category: FailureCategory }`, `Hook`. Marked `#[non_exhaustive]`. | M (also has `Undeclared`, `MalformedError`, `Hook`, which the spec does not mention) |
| `RedirectRefusal` | response.rs:190-202: `CrossOrigin`, `HopLimit`, `MissingLocation`, `InvalidLocation`, `UnsafeLocation` | M |
| `FailureCategory` | response.rs:221-232: `Connect`, `Tls`, `Timeout`, `DeadlineExceeded`, `Other` | M |
| `CallError<E>` | RT call.rs:247-261 `Transport(TransportError) \| Api { error: ApiError<E>, meta: Box<ResponseMeta> }`, `#[non_exhaustive]`. `request_meta()` :266. | M |
| `BuildError` | RT error.rs:137-185, `#[non_exhaustive]`. Includes `OriginPolicyNotPermitted { definition, requested }`. | M |

###### 5. Client, calls, credentials

| Spec expectation | Code | Status |
| --- | --- | --- |
| Managed client | `Client<A: Api>` RT client.rs:129. `Client::new(ApiDefinition)` :204 (infallible). Settings are validated on first send or by `build()` :390. `ApiDefinition` :61-67 holds `base_url`, `auth`, `secret_names`, `request_id_header`, `cross_origin`. `DEFAULT_TIMEOUT` = 30 s at :50. | M |
| Credential order: per call, client, environment, anonymous | RT credentials.rs:295-314: `CallAuth::Anonymous` → anonymous, `PerCall` → that credential, `Inherit` → client credential, otherwise `resolve_env()` (:316), otherwise anonymous. | M |
| Typed `send` | `Client::send<R: EndpointSpec<Api=A>>(&self, R) -> Call<'_, A, R>` client.rs:573-578. No `CallOptions` argument. | M |
| `.full()` / `FullCall` | `Call::full(self) -> FullCall<'a,A,R>` RT call.rs:98-100. `FullCall` :52-54 forwards `anonymous`, `credential`, `deadline`, `build` (:134-168). Its await output is `Result<Response<R::Success,R::Failure>, TransportError>` :188-200. | M |
| `send_raw(request, &CallOptions)` | client.rs:626-631, returns `impl Future<Output = Result<RawResponse, TransportError>>`. It counts the deadline at :655-657. | M |
| `Call<'a, A, R>` and its private fields | RT call.rs:39-43 `{ client: &'a Client<A>, request: R, options: CallOptions }`, all private. The constructor `Call::new` is `pub(crate)` :57. There are no public accessors. Code inside the runtime crate (a `pager` module) can read the fields; generated code cannot. | M |
| `Call::map_request` | call.rs:90-93 `pub fn map_request(mut self, f: impl FnOnce(R) -> R) -> Self` | M |
| Per-call `.credential` / `.anonymous` / `.deadline(&Deadline)` | call.rs:68-85 | M |
| `.build()` checks | call.rs:125-131 (requires `R: Clone`) → `Client::check_arguments` client.rs:669-683 → `Inner::prepare` :849-893. Checked: encoding, header and body content type, credential kind and placement. URL errors, origin refusal, client settings and deadline are left to the send (call.rs:119-124). | M |
| Send core | `send_unboxed` client.rs:582-605 (`pub(crate)`) → `Inner::execute` :733-838. `completed()` counts the deadline (:1219-1224). | M |
| Generated API struct surface | The `api_client!` macro (RT api_client.rs:46+) forwards settings plus `send` (:165), `send_raw` (:173), `check_link` (:190). | M |

###### 6. Origin policy and links

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Client::check_link::<R>(&Url)` | client.rs:699-713: `pub fn check_link<R>(&self, url: &Url) -> Result<(), TransportError> where R: EndpointSpec<Api = A>`. It does no network work. A refusal is `CrossOriginRejected { target }` with redacted target, Anonymous auth and `Duration::ZERO`. Invalid client settings give `InvalidClient`. `Url` is re-exported at RT lib.rs:87. | M |
| Checked absolute-URL request path | `Inner::assemble` client.rs:969-990: if `is_absolute_url(&path)` (:1199-1203, case-insensitive `http://`/`https://` prefix), then `Url::parse(&path)` is used instead of joining onto the base. The declared query is still appended and the fragment cleared (:988-990). `prepare` then checks the origin with `origins.link(&assembled.url)` (:856-860) and refuses with `CrossOriginRejected`. Credentials are attached only if the decision says so (:870-876). Test: `runtime/tests/l1/redirects.rs:328-350`. To use it, a request's `into_parts()` must return `path` = absolute URL. Generated request types build `path` from the template, so the pager needs a wrapper `EndpointSpec` (same `Api`, `Success`, `Failure`, `Decoder`, `ENDPOINT_ID`) or an in-crate seam. No such wrapper exists today. | M (mechanism present; the pager integration type is new work) |
| `deny` and same-origin links | Module table at RT origin.rs:10-24: "`deny` follows no redirect at all ... but a same-origin link is still allowed so that paging keeps working." `redirect()` refuses everything under Deny (:161-166). `link()` = `decide()` (:171-189), where the same origin is followed with credentials under every policy. Test at redirects.rs:345-349. | M (spec default 17 is satisfied) |
| `MAX_REDIRECTS` | client.rs:54 `pub const MAX_REDIRECTS: usize = 10;`. Enforced at :943-944 → `RedirectRefusal::HopLimit`. | M |
| `danger_allow_any` | Runtime: `OriginPolicy::danger_allow_any()` RT origin.rs:81, variant `DangerAllowAny` :53. Define side: `CrossOrigin::danger_allow_any()` DEF cross_origin.rs:76, name "danger_allow_any" :92. Compile-fail fixture: `runtime/tests/fixtures/compile_fail/allow_any_needs_the_danger_name.rs`. | M. The spec's D28/R18 text says `allow_any`. |
| Scheme, userinfo, origin rules | origin.rs:180-183: a non-http(s) scheme or userinfo is refused. An origin is (scheme, host, effective port) (:5-8). | M |
| Relative-link resolver | **No public resolver.** The runtime resolves only redirect `Location` headers, internally (`current.url.join(location)` client.rs:939). There is no public helper to resolve a relative reference against a response URL, and the envelope exposes only the redacted final URL (meta.rs:57). | **D** (the spec says "reuse the runtime's resolver") |
| `Link` header sanitizing | Not sanitized; RE implementation-log.md:1310-1312, 3664 assign the decision to spec 3. | open, assigned to pagination |

###### 7. Shared deadline

| Spec expectation | Code | Status |
| --- | --- | --- |
| `Deadline` type, monotonic clock | RT deadline.rs:20-79. Built with `Deadline::after(Duration)`. Methods: `expires_at`, `remaining`, `is_expired`, `elapsed`, `completed_requests`. Uses `tokio::time::Instant` (:16), which is monotonic and pausable in tests. Clones share an `Arc`. | M |
| Per-request time = min(remaining, per-request timeout) | `Expiration::start` deadline.rs:104-123 picks the earlier of `start+timeout` and the deadline. Used at client.rs:743. An expired deadline sends nothing (client.rs:735-741). | M |
| What it counts | deadline.rs:67-74: logical sends whose outcome was classified, HTTP error statuses included. Redirect hops, pre-head failures and dropped sends are not counted. Recorded by `completed()` client.rs:1219-1224 and by `send_raw` :655-657. `DeadlineExceeded` reports `completed_requests` and `deadline_elapsed`. | M (refresh note correct; pager page counts are separate) |
| Session close grace / cleanup-deadline precedence | **No spec-2 ruling.** RE spec.md:174-175, 1456-1457, 1684-1686 say the grace after expiry "is spec 3's to decide". RE implementation-log.md:3609-3624 says "The session-close grace after expiry belongs to spec 3." No author ruling appears in RE review-1..3 (the only mention is review-1.md:225, which defers it). The parent still has S10 open (MR:1730). | M (default 21, 5 s grace, stands; it is unratified **Proposed**) |
| Stream bounds | codec.rs:71-77, 104-116: idle = per-request timeout, and the deadline bounds the whole stream after `into_streaming()`. | info |

###### 8. Generator: endpoint methods and `Paginated`

| Spec expectation | Code | Status |
| --- | --- | --- |
| `codegen/endpoint_methods` emitter | `GEN codegen/endpoint_methods/{mod.rs,names.rs,docs.rs,tests.rs}`. Entry `generate_endpoint_methods(api: &RestApi) -> TokenStream` mod.rs:59, called from GEN output/assemble/api.rs:70. | M |
| Local `{Endpoint}Call` wrapping `schematic_runtime::Call<'a, Tag, Request>` | mod.rs:132, 149-151: `pub struct #call<'a>(schematic_runtime::Call<'a, #tag, #request>);` with a private tuple field. Contents: setters via `map_request` (:134-145), `anonymous`/`credential`/`deadline` (:157-169), `full()` (:173), `build()` (:184), and `IntoFuture` (:189-201). The endpoint method is `#call(self.0.send(...))` (:222-227). Count: 547 call types in `schematic/schema/src` (openai 265). | M |
| Reserved names incl. `pages`, `fetch_page`, `collect_all` | GEN codegen/endpoint_methods/names.rs:39 `PLANNED_CALL_METHODS = ["pages", "fetch_page", "collect_all"]`. Also `SHARED_CALL_METHODS` :35, `CALL_TYPE_TRAIT_METHODS` :43, `API_CLIENT_METHODS` :47-67, `API_STRUCT_METHODS` :72. A reserved setter gets the suffix `_param` (:332-335); a reserved endpoint method gets `_endpoint` (:315-318). `is_call_type_method` is at :385-389. New pager names (`page_size`, `after`, `with_session`, ...) are **not** reserved yet. | M; extend the list |
| `Paginated` marker and its generators | GEN codegen/traits.rs:35-56 `generate_paginated_trait()` emits `pub trait Paginated {}`. traits.rs:86-102 `generate_paginated_impl` is emitted only when `endpoint.params.pagination.is_some()` (:88-91). Re-exported at codegen/mod.rs:51. | M |
| Imports in `output/assemble/*.rs` | shared.rs:6, 16, 29 (trait into `shared.rs`). api.rs:16, 67-69 (impl), `has_paginated` :84-90, `shared_import` :92-98 (used at :106/124, 157/176, 260/284). GEN validation.rs:88 lists "Paginated" in `IMPORTED_TYPE_NAMES`. | M |
| Generated `shared.rs` | `schematic/schema/src/shared.rs:173` `pub trait Paginated {}`. There are 35 `impl Paginated` (github 9, gitlab 9, bitbucket 9, gitea 8). | M |

###### 9. Generation-integrity deliverables

| Spec expectation | Code | Status |
| --- | --- | --- |
| Partial generation | GEN output/mod.rs:41-43, `prepare_rust_output` :236 (doc :215-235). ABI and shared-file byte checks at output/inventory.rs:440-480. Tests in `schematic/gen/tests/l1/partial_generation.rs`. | M |
| Isolated fresh-output compile (D49 item 1, GI spec.md:103, :170) | Recipe `just test-fresh-output` (schematic/justfile:80-82), run by `just test` (:71-74). Harness `schematic/gen/tests/common/fresh_output.rs` (temp crate outside the checkout, `--offline --locked`). Tests in `schematic/gen/tests/l1/fresh_output_compile.rs`: `slow_fixture_apis_compile_and_send_offline` :1033, `slow_real_apis_compile_offline` :1063, `slow_generated_types_reject_misuse_at_compile_time` :1293, ABI mismatch :1347, negative control :1486, `slow_query_wire_matches_pre_split_baseline` :1706. | M |
| Query-parameter separation | `RequestParts.query` RT endpoint.rs:83-88. Generator side in GEN codegen/request_structs/shared.rs:121-131 and into_parts.rs:18-78. Runtime encodes once: client.rs:990, 1097, 1112. | M |
| Diagnostics format | GEN diagnostics.rs: `ValidationDiagnostic` :182-189 `{ severity, api: Vec<String>, endpoint: Option<String>, feature: Feature, output: Output, message }`. Constructors `new` :193, `for_group` :212. Display :239-252 is `severity={} api={} endpoint={} feature={} output={}: {}` (api joined with ","; endpoint is `<none>` when absent). Example :28-31. `Feature` (:99) uses kebab ids (:145-167). `Output` is rust/openapi/postman (:67). | M. The spec P8.3 default `<api>.<endpoint_id>: <rule>: <detail>` must be aligned to this format with new `Feature` ids. |

###### 10. `RestApi` shape and literals

| Spec expectation | Code | Status |
| --- | --- | --- |
| Final `RestApi` fields | DEF types.rs:124-235, `#[derive(Debug, Clone, PartialEq, Eq)]`, **no `Default`, no constructor**. Fields: `name` :131, `description` :133, `base_url` :137, `docs_url` :139, `auth` :141, `auth_policy` :148, `env_auth` :156, `env_username` :161, `headers` :168, `endpoints` :170, `module_path` :176, `request_suffix` :181, `version` :186, `env_mapping` :224, `cross_origin: CrossOrigin` :227, `request_id_header: Option<String>` :230, `secret_names: Vec<String>` :234. | M |
| "18 RestApi literals" | 18 production literals in `schematic/definitions/src` (lmstudio, gitlab, emqx ×2, artificial_analysis ×2, unfolded_circle/core_rest, elevenlabs, gitea, github, eversolo, anthropic, huggingface, bitbucket, ollama ×2, openai (mod.rs:5067), samsung_smart_tv), plus 3 test literals in definitions/src/registry/mod.rs:434, 491, 533. Because `RestApi` has no `Default`, every other literal also needs the new fields. These include DEF openapi/import/builder.rs:362 (importer), DEF request.rs:716, DEF types.rs tests (:567-773), DEF openapi/export tests, and many in GEN (validation.rs, output/mod.rs, test_utils.rs, codegen tests, tests/l1/*). A raw grep for `RestApi {` finds about 122 hits across schematic crates (some are not literals). | D (scope larger than 18) |
| `cross_origin`, `request_id_header`, `secret_names` values | All 21 definition literals use `CrossOrigin::same_origin()` and `secret_names: vec![]`. `request_id_header` is `None` except github/mod.rs:517 `Some("x-github-request-id")` and openai/mod.rs:5086 `Some("x-request-id")`. | M |
| Importer definition emitter | GEN definition_gen.rs `rest_api_tokens` :117-160. The literal is built at :141-159 and emits every field (`auth_policy: None` :147, `env_mapping: None` :155, `cross_origin` :156 via `cross_origin_tokens` :411-421, `request_id_header` :157, `secret_names` :158). `endpoint_params_tokens` :308-324 always emits `pagination: None, response_pagination: None`. GEN import_pipeline.rs `run_import` :68 applies only `request_id_header` (:100) and `exclude_paths` (:103-121). | M |
| Generic overlay hook (R35) | **Absent.** Nothing in GEN/DEF/definitions. The `import-openai` recipe is at schematic/justfile:201-213. | A (new work, as expected by R35) |

###### 11. Legacy pagination

| Item | Defined | Readers |
| --- | --- | --- |
| `PaginationStyle` | DEF pagination.rs:35-60 (`#[non_exhaustive]`: `PageNumber` :39, `OffsetLimit` :47, `Cursor` :55). Constructors `github` :72, `gitlab` :90, `bitbucket` :103, `gitea` :121, `cursor` :139, `offset_limit` :156. `to_query_params` :171. | Production reader: GEN codegen/traits.rs:88-91 and output/assemble/api.rs:88 only (both drive the `Paginated` marker). |
| `PaginationResponse` | DEF pagination.rs:276-291 (`BodyField`, `LinkHeader`, `TotalCount`) | No reader. No definition calls `with_response_pagination`. |
| `EndpointParams::with_pagination` | DEF params/mod.rs:130-134 (adds query params and sets `pagination`). Also `has_pagination` :204, `with_response_pagination` :227, `has_response_pagination` :235. | 35 endpoints: github 9, gitlab 9, bitbucket 9 (via `bitbucket_pagination()` bitbucket/mod.rs:105-106), gitea 8. |
| `EndpointParams.pagination` / `.response_pagination` | DEF params/mod.rs:99, 105 (struct :86-106, derives `Default`). Re-exports: DEF lib.rs:167, prelude.rs:54. | Emitter writes `None` (definition_gen.rs:320-321). OpenAI literals spell `pagination: None, response_pagination: None` explicitly (e.g. openai/mod.rs:115-116). |
| `Endpoint` fields (R3 frozen test) | DEF types.rs:446-490: `id`, `method`, `path`, `description`, `request`, `response`, `headers`, `params`, `oauth_scopes` | n/a |

###### 12. `validate_api`

| Spec expectation | Code | Status |
| --- | --- | --- |
| `validate_pagination` wired into "the existing `validate_api` path in `schematic-gen/src/validation.rs`" (spec R2 :117, Design :214) | **No `validate_api` exists** in schematic/gen, define or definitions; the only one is in the spike prototype (`spikes/pagination/prototype/src/lib.rs:1605`). The actual functions in GEN validation.rs: `validate_rust_client(apis: &[&RestApi]) -> Vec<ValidationDiagnostic>` :138-152 (runs `check_routes`, `check_rust_api` :333, `check_endpoint_method_names` :407-427, and `check_group_type_names`), `validate_openapi` :160, `validate_openapi_registry` :173, `validate_postman` :215, and `ensure_no_errors` :228-239, which turns errors into `GeneratorError::Validation { output, diagnostics }`. Callers: GEN pipeline.rs:428, 637; output/mod.rs:410; re-exported at lib.rs:110. | **D**. Wire into `validate_rust_client` (and the per-output validators as appropriate), emitting `ValidationDiagnostic`. |

##### Review rulings affecting pagination

Runtime envelope, review 1 (`human_resolved: true`; "Human rulings" section at RE review-1.md:231-311):

- **Item 1** (:241-244): endpoint call types are the primary surface. Pagination builds on them; the generic `pager(request)` entry default is superseded. Implemented.
- **Item 2** (:260-264): accepted as built: the type-level `Decode::decode(LiveBody)`, the owned `LiveBody`, `ErrorBody`, and `decode_hooked` gated by `SupportsHooks`. Any `SupportsHooks` codec must ship a hook test.
- **Item 4** (:276-280): `Client::check_link::<R>(&Url)` replaces the type-free form. An absolute `http(s)://` path is sent after the same origin check. "the pager built on endpoint methods is the primary caller". The pager follows absolute links "through the checked absolute-URL request path".
- **Item 5** (:284-288): ElevenLabs `ListVoices` keeps optional `page_size` (integer) and `page_token` (string); "A later pagination strategy may adopt `page_token`"; "authorizes no other endpoint change".
  - Code: `schematic/definitions/src/elevenlabs/endpoints/voices.rs:17-23` (query name `page_token`), and the response field `next_page_token` at `elevenlabs/types/voices.rs:131`.
  - The reviewer noted the input name may really be `next_page_token`; this is unverified (RE review-2.md:105-107, review-3.md:84-86).
- **Item 6** (:292-300): keep both partial-generation checks (the ABI marker and the shared-file byte comparison).

Runtime envelope, adopted defaults recorded in the implementation log. These are not author rulings, but they are the landed contract:

- `deny` passes same-origin links (PD2; :592, :3288).
- `MAX_REDIRECTS = 10` (PD3; :1859, :3289).
- Tighten-only overrides (PD1; D56/D57 item 6; :1832-1835).
- Deadline counting rules (:3609-3624; arithmetic from D56-3).
- `RequestMeta.page` is never set; "The pagination spec must add its input" (:3625-3631, :3939).
- `InvalidArgument::new` is the page-size path (:3637-3644).
- `RedirectFailed` categories, and "Spec 3 maps `RedirectFailed` as it maps `TransportError` (D57 item 7)" (:3655-3663).
- `Link` headers are not sanitized; spec 3 decides (:1310-1312, :3664).
- Session-close grace "belongs to spec 3" (:3609-3624). No author ruling exists (no hits for grace/S10/P6 in RE reviews), so default 21 (5 s) stands as Proposed.

Endpoint methods (EM review-1.md, `human_resolved: true`). The rulings are recorded as `**DECISION:**` lines. The "Human rulings" section (:103-115) still reads "(pending)". Reviews 2 and 3 treat the DECISION lines as authority (review-2.md:30, review-3.md:34-42). Authorship cannot be verified from git, because the file's only commit is the cycle close.

- **Item 1** (:18): `CallError::Api { error, meta: Box<ResponseMeta> }`. Matches RT call.rs:254-260.
- **Item 2** (:20-27): keep generic `FullCall`; `.full()` is the final endpoint-specific step ("`.full().per_page(10)` is not" supported). Pagination docs must use this ordering consistently. Code: the `FullCall` doc at RT call.rs:45-51, and the compile-fail fixture `request_change_after_full.rs`.
- Build cost: chained setters without per-API features (EM spec.md:51 "DECISION: A"; review-1.md:82). Measured cost: whole command +4.5%, unit +34% (EM implementation-log.md:1260-1267).
- Reserved names `pages`, `fetch_page`, `collect_all` (EM implementation-log.md:612-620, departure 6 :1255; EM spec.md:180 also reserves "any method later specs add").
- `Call::map_request` is public (departure 4, :519, :1253).
- The API struct field is private (`pub struct GitHub(Client<GitHubApi>)`, EM implementation-log.md:335, :651).
- **Review 3** (`ready: true`, no findings):
  - `Call::build` shares first-hop preparation with the sends (:32, :54-69).
  - Pagination handoff (:107-111): "pager methods extend existing local calls through thin forwards into the shared runtime ... accepted full-response ordering, ABI 3, effective call settings, shared argument preparation, envelope preservation, private call-state integration, collision reservations, and deadline request-count semantics". "No contested requirement or new scope was introduced."
  - Observation (:103-105): the 40-test schema suite does not run in CI; scheduling it is the author's choice.

Generation integrity:

- The D49 item 1 isolated compile check is delivered (GI spec.md:103, :170; GI review-3.md:100).
- Author rulings at GI review-1.md:89-147:
  - An endpoint is `method + path`. GitHub `GetGitTree`/`GetGitTreeRecursive` and the Bitbucket pair are merged (:99, :118-129). This affects the "Gitea tree" row's assumptions only indirectly; the GitHub tree pair is now one endpoint.
  - Deletion of the stale tree is authorized for spec 1 (:144-147).

##### Discrepancies with spec refresh notes

1. **`validate_api` does not exist** (spec R2 :117, Design :214). Use `validate_rust_client` and the per-output validators in GEN validation.rs, with `ValidationDiagnostic` and new `Feature` ids. The refresh notes do not mention this.
2. **No runtime relative-URL resolver, and no unredacted final URL in the envelope.** The spec (:106, R18) says "reuse the runtime's resolver"; the refresh notes are silent.
   - The only resolution is internal `Url::join` for redirects (client.rs:939).
   - `ResponseMeta.request.url` is a `RedactedUrl` (meta.rs:57), with secret query values replaced.
   - The pager must either resolve against an internally retained final URL (a new in-crate seam) or use `url::Url::join` itself. Record this as an integration correction; it is not a missing prerequisite.
3. **The checked absolute-URL path exists only through `RequestParts.path`** (client.rs:980-984). Generated request types cannot carry an arbitrary URL, so the pager needs a wrapper `EndpointSpec` or a crate-internal send seam. The refresh notes describe the path as available, which is true at the runtime level.
4. **RestApi literal count.** The spec and refresh notes say 18. Code has 18 production literals plus 3 registry test literals. `RestApi` has no `Default`, so the new fields also touch the importer builder (DEF openapi/import/builder.rs:362), the emitter (GEN definition_gen.rs:141-159), and many test literals in DEF and GEN (about 122 `RestApi {` grep hits in total).
5. **Dev-dependencies already present.** `trybuild = "1.0"` and `wiremock = "0.6"` are already dev-dependencies of schematic-runtime (`schematic/runtime/Cargo.toml:31-32`), and both are listed in `docs/dependencies.md` (:21, :1640, :1646) and `schematic/docs/dependencies.md` (:281, :311, :411). The spec's pre-authorization to "add" them (:313) is moot for the runtime. The existing trybuild harness clears `RUSTC_WRAPPER` for portability (`runtime/tests/l1/compile_fail.rs:12-15`) and runs through `just test-compile-fail` (justfile:90-91), which addresses the spec's kache-path risk.
6. **`RequestMeta.page` has no input.** The refresh note says "ordinary sends leave it unset", which is correct. But the runtime always leaves it `None` and offers no way to request it, so the pager must set it (in-crate) after each send (RE implementation-log.md:3625-3631).
7. **Origin policy naming.** D28/R18 say `allow_any`; code uses `danger_allow_any` (runtime and define). The refresh notes already state this. The R18 test-matrix wording in the spec is stale.
8. **New pager names are not reserved.** `PLANNED_CALL_METHODS` (names.rs:39) holds only `pages`, `fetch_page`, `collect_all`. Knob names (`page_size`, `page`, `offset`, `after`, `before`, `prev`, `keep_alive`, `partitioned`, `with_session`, `total`, ...) must be added under the same policy. The refresh notes already require this.
9. **ElevenLabs input name.** The definition sends `page_token` (voices.rs:19). The spec table says research names it `next_page_token`; "confirm the sent name before adopting" remains open and unverified.
10. **Likely ABI bump** once generated code implements runtime pager traits (RT lib.rs:96-101). The refresh notes say ABI is 3 and do not mention a bump.
11. **`InvalidArgument` has no kinds or variants** (a struct with a private reason). This matches the refresh note "no dedicated page-size variant". The spec's "invalid-argument kinds" phrasing in plan 1.1 has no code counterpart.
12. **Generated `Call` wrapper uses tuple field `.0` (private)** (mod.rs:149-151). Forwards generated in the same module can use `self.0`. The runtime `Call` fields are private to generated code, so pager entry points must be runtime methods on `Call<'a, A, R>` (bounded by pager traits) or take the `Call` by value. This confirms the refresh note's "narrow integration seam".

Facts that match the refresh notes and need no correction: ABI 3, the modules, `EndpointSpec`, the codecs, `into_parts`/`into_result`, `RedirectFailed`, `CallError`, `.full()` ordering, `.build()` checks, `check_link::<R>`, `deny`, `MAX_REDIRECTS = 10`, the deadline semantics, the credential order, `send_raw(request, &CallOptions)`, `map_request`, the reserved names, the 547 call types, and the `RestApi` cross-origin, request-id and secret-name fields.

##### Authorization record

**No author confirmation of any spec-3 pre-authorization is recorded anywhere.**

Where the requirement for confirmation is recorded:

- **MR:2114** (D50 item 8, under "Decision D50 (confirmed by the author, 2026-10-11)", :2094), verbatim: "**Before each plan runs**, its spec needs a short pass to settle its planning-time details ... and the author must pre-authorize the steps that would otherwise pause an unattended run: regenerating `schematic-schema`; creating the `schematic-runtime` package; deleting the stale `schematic/gen/schematic` tree; editing dependent crates (sniff, biscuit-speaks, model-citizen, the five homelab crates); and, for spec 4, rewriting the 253 endpoint definitions." This is a process rule, not a grant.
- **MR:2612-2623** ("Preparation tasks needing author permission"): "The implementer cannot take them without the author's explicit approval." Listed tasks: regenerate `schematic-schema` (:2616); verify the U rows (:2618); fix EMQX `ListAuthUsers` (:2619); "any other destructive or wide-scope action" (:2623). None is annotated as granted.
- **D55, D56, D57 (MR:2217, :2241, :2274):** design rulings only. The only permission in them is D57 item 1 (:2280), "pre-authorized for spec 1".
- **PG spec.md:308:** "None is granted until the author confirms the list". Network verification (:314) and the `ListAuthUsers` fix (:312) are explicitly **PENDING**. Lines :310-317 list regeneration, dependent edits, definition edits, runtime module and dev-dependencies, network docs, `Paginated` removal, docs and skills, and the size measurement. :318 lists what is not authorized. This is agent-drafted request text.
- **PG plan.md:** frontmatter is `yolo: true`, `agent: codex/gpt-6.1-sol`. :38 says "Do not assume `yolo: true` grants those permissions." :53 tells Phase 1 to review actual authorization. These are agent statements.
- **PG implementation-log.md:** no authorization recorded.
- **Precedent (not applicable to spec 3):** RE implementation-log.md:575-582 records the agent's inference that the author's phase-by-phase instruction granted spec 2's pre-authorized scope. GI implementation-log.md:353-360 records "Pre-authorization items 1–8 still have no recorded author confirmation." The explicit author grants found anywhere are GI review-1.md:118, :144; RE review-1.md:284; and MR:2280, and none covers spec 3.

| Step | Recorded confirmation |
| --- | --- |
| Regenerate `schematic-schema` | Not recorded |
| Dependent-crate migrations | Not recorded |
| Runtime module and dev-dependency changes | Not recorded (dev-dependencies already present, so only the new module and docs are needed) |
| Definition authoring (18 APIs, `RestApi` fields, tables) | Not recorded |
| Docs, skills, READMEs | Not recorded |
| Network verification of U rows (optional) | Not recorded; explicitly PENDING |
| EMQX `ListAuthUsers` fix (optional) | Not recorded; explicitly PENDING, and conditional on the EMQX rows being verified |

Per plan.md:38, the two optional steps are skipped and logged. The other steps are required implementation scope, and their missing authorization must be recorded before proceeding. The only candidate source is the author's own run instruction, which does not appear in any pagination file.

##### Dependency availability

- Root `Cargo.toml` has **no `[workspace.dependencies]`** section. `schematic/schema` is excluded from the workspace (root Cargo.toml:81).
- **trybuild:** `schematic/runtime/Cargo.toml:31` (`trybuild = "1.0"`), used by `runtime/tests/l1/compile_fail.rs:16` with 10 fixtures in `runtime/tests/fixtures/compile_fail/`. It is not used by any other schematic crate.
- **wiremock 0.6** as a dev-dependency:
  - `schematic/define/Cargo.toml:27`
  - `schematic/oauth/Cargo.toml:21`
  - `schematic/runtime/Cargo.toml:32` (used in `runtime/tests/l1/{send,credential_resolution,client_settings,...}.rs`)
  - `schematic/schema/Cargo.toml:27`

  It is not a dependency of `schematic-gen` (only emitted into generated fixture crates, GEN cargo_gen.rs:46) or `schematic-definitions`.
- Other runtime dev-dependencies: `biscuit-test-harness`, `test-toolkit`, `rcgen`, `tokio-rustls`, `tokio` (with `test-util` for paused time) (runtime/Cargo.toml:22-30).
- `schematic/schema/Cargo.toml:16`: `schematic-runtime = { version = "0.1.0", path = "../runtime" }`.
- The area `just test` already covers schematic-runtime (justfile:71-74), along with `test-fresh-output` and `test-compile-fail`.

#### Coverage inventory: `2026-10-13-schematic-pagination` Phase 1 task 1.2

Recounted against the current tree of `feat/modern-config` on 2026-10-13 (HEAD `0cd6075e6`). Method: a throwaway crate (`/tmp/pagination-phase1/dump`, built `--offline`) calls the 18 `define_*_api()` constructors and dumps every endpoint (id, method, path, response descriptor, query params, `pagination`, `response_pagination`) to `endpoints.jsonl`. A Python pass (`analyze.py`) then resolves response struct fields from `schematic/definitions/src`, per module and excluding `#[cfg(test)]`. I spot-read the results against the sources: Bitbucket `GetSource`, ElevenLabs `ListVoices`, UC aliases, the EMQX `PaginationMeta`, and the OpenAI vector-store schemas in `specs/openai/openapi.yaml`.

"Root `Vec<T>`" means the response descriptor is literally `ApiResponse::Json` with `type_name == "Vec<T>"` (`json_vec_type`). This is the only form that R4 step 2 and default 22 recognize. A type alias to `Vec<T>` (UC) is not recognized.

##### Recount summary

| Figure | Spec / inventory (2026-10-09/11) | Current tree | Note |
|---|---|---|---|
| REST APIs (`RestApi` from `apis_by_module`) | 18 | **18** | anthropic, AA data, AA critpt, bitbucket, openai, elevenlabs, gitea, github, gitlab, huggingface, lmstudio, ollama native, ollama openai, emqx basic, emqx bearer, eversolo, samsung, UC core_rest |
| Endpoints (resolved, all APIs) | 547 | **547** | = generated request types 547 = call types 547 (`schema/src` `EndpointSpec` impls) |
| OpenAI endpoints | 265 | **265** | |
| Production `Endpoint {` literals in `definitions/src` | ~519 | **511** | 547 − 36 EMQX endpoints that basic and bearer share through helper fns. Excludes `#[cfg(test)]`, `registry/` and WebSocket literals; with tests counted, the raw grep gives 569 |
| Declared legacy pagination (`pagination: Some`) | 35 (Gitea 8, GitHub 9, GitLab 9, Bitbucket 9) | **35** (same split) | all `PageNumber`; **one Bitbucket declaration (`GetSource`) is now on a `Binary` endpoint**, see D1 |
| `response_pagination: Some` | 0 | **0** | |
| GET root `Vec<T>` endpoints | n/a | **40** | GitHub 8, GitLab 10, Gitea 9, HF 10, ElevenLabs 1, EMQX 1+1 |
| Non-GET root `Vec<T>` descriptors | n/a | **0** | UC `RestoreBackup`/`UploadResource` return *aliases* of `Vec` |
| Collection endpoints (inventory method) | ~157 | **156** (+ `GetSource` dual-shape = 157) | Bitbucket `ListDirectoryContents` (wrapper) was merged into `GetSource` (`Binary`) by `9e65a90b5` on 2026-10-12 |
| OpenAI list candidates | 61 (60 GET + 1 POST) | **61** (60 GET + `SearchVectorStore` POST) | S6 36 + item-id 2 + S7 9 + S8 11 + 3 opt-outs |
| Paginated after authoring (verified rows only) | — | **95**, or 94 if ListVoices stays blocked (D2) | GitHub 9, GitLab 9, Gitea 8, Bitbucket **8**, OpenAI 58, ElevenLabs 3 |
| Resolve as list under R4 (paginated + unpaginated) | — | **113** | 95 paginated, plus 13 root arrays with no default (HF 10, EL `ListModels`, EMQX `ListAuthUsers` ×2), plus 5 opt-outs (Gitea `GetTagReference`, GitLab `ListIssueParticipants`, OpenAI ×3) |
| Collections that resolve "not a list" (wrapper, no entry) | — | **43** | AA 6, LM Studio 1, Ollama 3, EMQX 13×2, ElevenLabs 3, Anthropic 1, HF discussions 1, Gitea trees 2 (113 + 43 = 156). See G1 |
| Request fields a strategy would adopt (R7) | — | **186** (+1 pending) | GitHub 18, GitLab 18, Gitea 16, Bitbucket 8 (`pagelen`), OpenAI 125 (after 47, limit 58, before 9, page 11), ElevenLabs 1 (`page_size`); `page_token` pending D2 |
| Plain pagination-looking fields kept (not adopted) | — | **10** | Bitbucket `page` ×8 on wrapper lists (NextLink has no page input); `GetSource` `page`/`pagelen` |

##### Per-API inventory

Columns: the **Origin** is the intended R4 origin. **Declared params** lists every query param the endpoint declares today; a bold param is one the strategy adopts (same name and location). `!` marks a required param. No adoptable param is required, so R2a needs no strategy default anywhere. **Evidence** uses the spec's labels.

###### GitHub (16 endpoints; 9 declared legacy; 8 GET `Vec<T>`)

API default: `PageNumber{page, 1-based}`, size `per_page` (max 100; server default 30 as docs only), end signal `Link` rel `next`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetGitTree | GET | opaque/wrapper `GitTreeResponse` | — | none | — | `recursive` | n/a | U (inventory: `truncated`, not page-paginated) | not a list (wrapper `tree`, no entry) |
| ListPullRequests | GET | root `Vec<T>` | `PullRequestSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `state`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListPullRequestFiles | GET | root `Vec<T>` | `PullRequestFile` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssues | GET | root `Vec<T>` | `IssueSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `state`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueComments | GET | root `Vec<T>` | `IssueComment` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueTimeline | GET | root `Vec<T>` | `TimelineEvent` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListTags | GET | root `Vec<T>` | `RepoTag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`** | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListWorkflowRuns | GET | wrapper at `workflow_runs` | `WorkflowRun` | table entry (wrapper) | default PageNumber + total from body `total_count`; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `status`, `branch`, `event` | yes | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListOrgRepos | GET | root `Vec<T>` | `RepositoryInfo` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()` | **`page`**, **`per_page`**, `repo_type`, `sort`, `direction` | yes (page, page_size) | V-notes (`sniff/docs/repos/github-api.md`, 2026-02-15; inventory 2026-10-09) | — |

Other GitHub endpoints (GetRepository, GetRepositoryContentRaw, GetIssue, GetTagReference [`GitRef`, not an array], GetBranchReference, GetAnnotatedTag) are not lists.

###### GitLab (19 endpoints; 9 declared legacy; 10 GET `Vec<T>`)

API default: `PageNumber{page}`, size `per_page` (max 100; server default 20), end signal header `X-Next-Page` present, total header `X-Total`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListRepositoryTree | GET | root `Vec<T>` | `TreeItem` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` + labeled alternate `keyset` = `NextLink{Link rel next}` + constants `pagination=keyset`, `order_by=id`, `sort=asc` (modeled, not generated) | **`page`**, **`per_page`**, `recursive`, `git_ref`, `path` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09); keyset constants V-notes | — |
| ListGroupProjects | GET | root `Vec<T>` | `Project` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` + labeled alternate `keyset` = `NextLink{Link rel next}` + constants `pagination=keyset`, `order_by=id`, `sort=asc` (modeled, not generated) | **`page`**, **`per_page`**, `archived`, `visibility`, `order_by`, `sort`, `include_subgroups` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09); keyset constants V-notes | — |
| ListMergeRequests | GET | root `Vec<T>` | `MergeRequest` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `state` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListMergeRequestCommits | GET | root `Vec<T>` | `Commit` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListMergeRequestChanges | GET | opaque/wrapper `MergeRequestChanges` | — | none | — | — | n/a | U | not a list (MR object with embedded `changes`) |
| ListIssues | GET | root `Vec<T>` | `Issue` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `state` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueNotes | GET | root `Vec<T>` | `Note` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueParticipants | GET | root `Vec<T>` | `User` | **table entry: opt-out** (required, else it inherits the default) | unpaginated | — | n/a | U (believed paginated) | unverified (R28); GET `Vec<User>` in an API with a default, so an opt-out entry is needed to keep it unpaginated |
| ListTags | GET | root `Vec<T>` | `Tag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`** | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListProjectPipelines | GET | root `Vec<T>` | `Pipeline` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`per_page`**, `status`, `source`, `git_ref`, `sha` | yes | V-notes (`sniff/docs/repos/gitlab-api.md`, 2026-02-15; inventory 2026-10-09) | — |

Note: ListGroupProjects declares plain `order_by` and `sort` query params; the keyset alternate's constants `order_by=id`/`sort=asc` share those names (alternates are not generated, but R2 `CollidesWithEndpointParam`/constant-vs-param handling must allow or reject this explicitly). ListRepositoryTree declares neither.

###### Gitea (16 endpoints; 8 declared legacy; 9 GET `Vec<T>`)

API default: `PageNumber{page}`, size `limit` (server cap 50; default 30 recorded), end signal `Link` rel `next`, total header `X-Total-Count`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetGitTree | GET | opaque/wrapper `GitTreeResponse` | — | none | (would be override `per_page`, `Flag(truncated)`, total `total_count`) | — | n/a | U | unverified; definition declares no `page`/`per_page` params; wrapper `tree` without entry = not a list |
| GetGitTreeRecursive | GET | opaque/wrapper `GitTreeResponse` | — | none | (would be override `per_page`, `Flag(truncated)`, total `total_count`) | — | n/a | U | unverified; definition declares no `page`/`per_page` params; wrapper `tree` without entry = not a list |
| ListPullRequests | GET | root `Vec<T>` | `PullRequestSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `state`, `sort` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListPullRequestFiles | GET | root `Vec<T>` | `PullRequestFile` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssues | GET | root `Vec<T>` | `IssueSummary` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `state`, `issue_type` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueComments | GET | root `Vec<T>` | `IssueComment` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListIssueTimeline | GET | root `Vec<T>` | `TimelineEvent` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListTags | GET | root `Vec<T>` | `RepoTag` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| ListReleases | GET | root `Vec<T>` | `Release` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`**, `draft`, `pre_release` | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |
| GetTagReference | GET | root `Vec<T>` | `GitRef` | **table entry: opt-out** | unpaginated | — | n/a | U (believed unpaginated prefix match) | opt-out (default 14): prefix-match ref lookup, not a list |
| ListOrgRepos | GET | root `Vec<T>` | `RepositoryInfo` | ApiDefault | default PageNumber; knobs `.page()`, `.page_size()`, `total()` | **`page`**, **`limit`** | yes | V-notes (`sniff/docs/repos/gitea-api.md`, 2026-02-15; inventory 2026-10-09) | — |


###### Bitbucket (15 endpoints; 9 declared legacy; 0 GET `Vec<T>`)

API default: `NextLink` from body `next` (absolute URL), backward `previous`, size `pagelen` (default 10, max 100), total `size`. The default reaches no endpoint by inheritance (no root arrays); each wrapper list needs a table entry at `values`.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| GetSource | GET | Binary | — | none | — | `page`, `pagelen` | n/a | V-def (Binary) | **not a list**: `ApiResponse::Binary` (dual shape: directory JSON or file bytes); legacy `page`/`pagelen` stay plain fields |
| ListPullRequests | GET | wrapper at `values` | `PullRequest` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`**, `state` | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListPullRequestComments | GET | wrapper at `values` | `PullRequestComment` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssues | GET | wrapper at `values` | `Issue` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssueComments | GET | wrapper at `values` | `IssueComment` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListIssueChanges | GET | wrapper at `values` | `IssueChange` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListTags | GET | wrapper at `values` | `Tag` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListDownloads | GET | wrapper at `values` | `Download` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |
| ListWorkspaceRepos | GET | wrapper at `values` | `Repository` | table entry (wrapper) | default NextLink; knobs `.page_size()`, `.prev()`, `total()` | `page`, **`pagelen`** | `pagelen` yes; `page` **not adopted** (NextLink has no page input) so its field is kept | V-notes next URL (`sniff/docs/repos/bitbucket-api.md`, 2026-02-15) + V-def params/`PaginatedResponse<T>` | — |


###### OpenAI (265 endpoints; 0 declared legacy; 0 GET `Vec<T>`; 61 list candidates)

API default: `Cursor{after <- last_id, has_more}`, size `limit`. Every list is a wrapper at `data`, so every one needs a table entry (none inherits by R4 step 2).

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListVoiceConsents | GET | wrapper at `data` | `VoiceConsentResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListBatches | GET | wrapper at `data` | `Batch` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListChatCompletions | GET | wrapper at `data` | `CreateChatCompletionResponse` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `model`, `metadata`, **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetChatCompletionMessages | GET | wrapper at `data` | `serde_json::Value` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListContainers | GET | wrapper at `data` | `ContainerResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `name` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListContainerFiles | GET | wrapper at `data` | `ContainerFileResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListConversationItems | GET | wrapper at `data` | `ConversationItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListEvals | GET | wrapper at `data` | `Eval` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order`, `order_by` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetEvalRuns | GET | wrapper at `data` | `EvalRun` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `order`, `status` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| GetEvalRunOutputItems | GET | wrapper at `data` | `EvalRunOutputItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `status`, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFiles | GET | wrapper at `data` | `OpenAIFile` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `purpose`, **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningCheckpointPermissions | GET | wrapper at `data` | `FineTuningCheckpointPermission` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | `project_id`, **`after`**, **`limit`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListPaginatedFineTuningJobs | GET | wrapper at `data` | `FineTuningJob` | table entry (override) | Cursor{after <- LastItem(`id`), has_more}; knobs `.after()`, `.page_size()` | **`after`**, **`limit`**, `metadata` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningJobCheckpoints | GET | wrapper at `data` | `FineTuningJobCheckpoint` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFineTuningEvents | GET | wrapper at `data` | `FineTuningJobEvent` | table entry (override) | Cursor{after <- LastItem(`id`), has_more}; knobs `.after()`, `.page_size()` | **`after`**, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListModels | GET | wrapper at `data` | `Model` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | no continuation in reply (`object`, `data` only) |
| AdminApiKeysList | GET | wrapper at `data` | `AdminApiKey` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`after`**, `order`, **`limit`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListAuditLogs | GET | wrapper at `data` | `AuditLog` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | `effective_at`, `project_ids[]`, `event_types[]`, `actor_ids[]`, `actor_emails[]`, `resource_ids[]`, `tenant_only`, **`limit`**, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListOrganizationCertificates | GET | wrapper at `data` | `OrganizationCertificate` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCosts | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `api_key_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroups | GET | wrapper at `data` | `GroupResponse` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroupRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListGroupUsers | GET | wrapper at `data` | `GroupUser` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListInvites | GET | wrapper at `data` | `Invite` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjects | GET | wrapper at `data` | `Project` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `include_archived` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectApiKeys | GET | wrapper at `data` | `ProjectApiKey` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `owner_project_access` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectCertificates | GET | wrapper at `data` | `OrganizationProjectCertificate` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectGroups | GET | wrapper at `data` | `ProjectGroup` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectRateLimits | GET | wrapper at `data` | `ProjectRateLimit` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectServiceAccounts | GET | wrapper at `data` | `ProjectServiceAccount` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectSpendAlerts | GET | wrapper at `data` | `ProjectSpendAlert` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectUsers | GET | wrapper at `data` | `ProjectUser` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListRoles | GET | wrapper at `data` | `Role` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListOrganizationSpendAlerts | GET | wrapper at `data` | `OrganizationSpendAlert` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageAudioSpeeches | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageAudioTranscriptions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCodeInterpreterSessions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageCompletions | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `batch`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageEmbeddings | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageFileSearchCalls | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `vector_store_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageImages | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `sources`, `sizes`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageModerations | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageVectorStores | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| UsageWebSearchCalls | GET | wrapper at `data` | `UsageTimeBucket` | table entry (override) | S8 Cursor{page <- `next_page`, has_more}; knobs `.page()`/cursor, `.page_size()` (bucket count) | `start_time`!, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `context_levels`, `group_by`, **`limit`**, **`page`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListUsers | GET | wrapper at `data` | `User` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `emails` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListUserRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectGroupRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectRoles | GET | wrapper at `data` | `Role` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListProjectUserRoleAssignments | GET | wrapper at `data` | `AssignedRoleDetails` | table entry (override) | S7 Cursor{after <- `next`, has_more}; knobs `.after()`, `.page_size()` | **`limit`**, **`after`**, `order` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListInputItems | GET | wrapper at `data` | `ItemResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListVectorStores | GET | wrapper at `data` | `VectorStoreObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListFilesInVectorStoreBatch | GET | wrapper at `data` | `VectorStoreFileObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `filter` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListVectorStoreFiles | GET | wrapper at `data` | `VectorStoreFileObject` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `filter` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| RetrieveVectorStoreFileContent | GET | wrapper at `data` | `serde_json::Value` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | provider gap: reply has `has_more`+`next_page`, no documented request input |
| SearchVectorStore | POST | wrapper at `data` | `VectorStoreSearchResultItem` | table entry: opt-out | unpaginated | — | n/a | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | provider gap: reply has `has_more`+`next_page`, no documented request input |
| ListVideos | GET | wrapper at `data` | `VideoResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListSkills | GET | wrapper at `data` | `SkillResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListSkillVersions | GET | wrapper at `data` | `SkillVersionResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListThreadItemsMethod | GET | wrapper at `data` | `ThreadItem` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`** | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| ListThreadsMethod | GET | wrapper at `data` | `ThreadResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` + backward `before <- first_id`, `.prev()` | **`limit`**, `order`, **`after`**, **`before`**, `user` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |
| BetaListInputItems | GET | wrapper at `data` | `BetaItemResource` | table entry (wrapper) | default Cursor; knobs `.after()`, `.page_size()` | **`limit`**, `order`, **`after`**, `include` | yes | V-def (`specs/openai/openapi.yaml`, vendored 2026-07-31, via importer params) | — |

OpenAI non-GET endpoints with list-shaped replies (D43: not lists, no entry): CreateConversationItems (POST `ConversationItemList`), CreateEmbedding (POST `CreateEmbeddingResponse`), CreateFineTuningCheckpointPermission (POST `ListFineTuningCheckpointPermissionResponse`), CreateImageEdit (POST `ImagesResponse`), CreateImage (POST `ImagesResponse`), CreateImageVariation (POST `ImagesResponse`), ActivateOrganizationCertificates (POST `OrganizationCertificateActivationResponse`), DeactivateOrganizationCertificates (POST `OrganizationCertificateDeactivationResponse`), ActivateProjectCertificates (POST `OrganizationProjectCertificateActivationResponse`), DeactivateProjectCertificates (POST `OrganizationProjectCertificateDeactivationResponse`).

###### ElevenLabs (42 endpoints; 0 declared legacy; 1 GET `Vec<T>`)

No API default (three different strategies). If a default were ever added, ListModels (GET `Vec<ModelInfo>`) would need an opt-out.

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListVoices | GET | wrapper at `voices` | `VoiceResponseModel` | table entry (wrapper) — **blocked** | Cursor{next_page_token <- next_page_token, has_more}, total `total_count`; knobs `.after()`, `.page_size()`, `total()` | **`page_size`**, **`page_token`** | `page_size` yes; `page_token` **name mismatch** with research input `next_page_token` | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | see discrepancy D2: confirm sent name before authoring/adopting |
| ListSharedVoices | GET | wrapper at `voices` | `LibraryVoiceResponseModel` | table entry (wrapper) | PageNumber{page, 0-based, has_more}, size `page_size` (30/100); knobs `.page()`, `.page_size()` | — | n/a (nothing declared) | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | — |
| ListModels | GET | root `Vec<T>` | `ModelInfo` | none | — | — | n/a | V-def | unpaginated list (root array, no default) |
| GetHistory | GET | wrapper at `history` | `SpeechHistoryItemResponseModel` | table entry (wrapper) | Cursor{start_after_history_item_id <- last_history_item_id, has_more}, size `page_size` (100/1000); knobs `.after()`, `.page_size()` | — | n/a (nothing declared; inputs are new) | V-notes (`schematic/docs/research/elevenlabs-api.md`, 2026-05-07) | — |
| ListServiceAccounts | GET | wrapper at `service-accounts` | `ServiceAccountModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |
| ListServiceAccountApiKeys | GET | wrapper at `api-keys` | `ApiKeyModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |
| ListWebhooks | GET | wrapper at `webhooks` | `WebhookModel` | none | — | — | n/a | U | unpaginated (U); wrapper without entry = not a list |


###### Anthropic (4 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListModels | GET | wrapper at `data` | `ModelInfo` | none | (would be Cursor{after_id <- last_id; before_id <- first_id; has_more}, limit 20/1..1000) | — | n/a | wrapper V-def; params U | U: unverified, stays unpaginated (no params declared) |

###### EMQX basic (36 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListNodes | GET | wrapper at `data` | `NodeInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListListeners | GET | wrapper at `data` | `ListenerInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListMetrics | GET | wrapper at `data` | `MetricsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListStats | GET | wrapper at `data` | `StatsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAlarms | GET | wrapper at `data` | `AlarmInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListClients | GET | wrapper at `data` | `ClientInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSubscriptions | GET | wrapper at `data` | `SubscriptionInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListTopics | GET | wrapper at `data` | `TopicInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRetained | GET | wrapper at `data` | `RetainedMessage` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRules | GET | wrapper at `data` | `RuleInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListAuthenticators | GET | wrapper at `data` | `AuthenticatorInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAuthUsers | GET | root `Vec<T>` | `AuthUser` | none (root array → unpaginated list) | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); **definition gap**: root array declared, server returns `{data, meta}` (U) |
| ListAuthzSources | GET | wrapper at `sources` | `AuthzSourceInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListBanned | GET | wrapper at `data` | `BanInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### EMQX bearer (38 endpoints = basic + Login/Logout) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListNodes | GET | wrapper at `data` | `NodeInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListListeners | GET | wrapper at `data` | `ListenerInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListMetrics | GET | wrapper at `data` | `MetricsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListStats | GET | wrapper at `data` | `StatsInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAlarms | GET | wrapper at `data` | `AlarmInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListClients | GET | wrapper at `data` | `ClientInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSubscriptions | GET | wrapper at `data` | `SubscriptionInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListTopics | GET | wrapper at `data` | `TopicInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRetained | GET | wrapper at `data` | `RetainedMessage` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListRules | GET | wrapper at `data` | `RuleInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); wrapper omits `meta` |
| ListAuthenticators | GET | wrapper at `data` | `AuthenticatorInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListAuthUsers | GET | root `Vec<T>` | `AuthUser` | none (root array → unpaginated list) | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared); **definition gap**: root array declared, server returns `{data, meta}` (U) |
| ListAuthzSources | GET | wrapper at `sources` | `AuthzSourceInfo` | none | — | — | n/a | U | config/state list, unpaginated; not a list without entry |
| ListBanned | GET | wrapper at `data` | `BanInfo` | none | (would be PageNumber{page, limit, Flag(meta.hasnext)}, total `meta.count`) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### Hugging Face Hub (26 endpoints) — U

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| ListModels | GET | root `Vec<T>` | `ModelInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelCommits | GET | root `Vec<T>` | `Commit` | none (root array → unpaginated list) | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListModelDiscussions | GET | wrapper at `discussions` | `Discussion` | none | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasets | GET | root `Vec<T>` | `DatasetInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasetFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListDatasetCommits | GET | root `Vec<T>` | `Commit` | none (root array → unpaginated list) | (would be PageNumber{p, 0-based}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSpaces | GET | root `Vec<T>` | `SpaceInfo` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListSpaceFiles | GET | root `Vec<T>` | `RepoFile` | none (root array → unpaginated list) | (would be NextLink{Link rel next, limit}) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| ListUserRepos | GET | root `Vec<T>` | `RepoInfo` | none (root array → unpaginated list) | (unverified endpoint) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |
| GetUserCollections | GET | root `Vec<T>` | `Collection` | none (root array → unpaginated list) | (unverified endpoint) | — | n/a | U | U: unverified, stays unpaginated (no params declared) |

###### Remaining APIs (no pagination, no default)

| Endpoint | Method | Shape | Item | Origin | Strategy + knobs | Declared params (adoptable in **bold**) | Knob authored for adoptable? | Evidence | Unpaginated reason |
|---|---|---|---|---|---|---|---|---|---|
| UnfoldedCircleCoreRest::RestoreBackup | PUT | alias `BackupRestoreReportItems` = `Vec<BackupRestoreReportItem>` | `BackupRestoreReportItem` | none | — | — | n/a | V-def | non-GET write returning an array (alias, not `Vec<T>` descriptor): D43 not a list |
| UnfoldedCircleCoreRest::UploadResource | POST | alias `ResourceItems` = `Vec<ResourceItem>` | `ResourceItem` | none | — | — | n/a | V-def | non-GET write returning an array (alias, not `Vec<T>` descriptor): D43 not a list |

No GET `Vec<T>` endpoint exists in these 8 APIs. CritPt, Eversolo (option-list wrappers only), and Samsung have no list endpoints.

##### Legacy pagination declarations

All 35 come from `EndpointParams::with_pagination` (`gitea/mod.rs` 8, `github/mod.rs` 9, `gitlab/endpoints/*` 9, and `bitbucket/mod.rs` through `bitbucket_pagination()` 9). Each one materializes exactly two optional `Integer` query params: the page param ("Page number (1-indexed, default: 1)") and the size param ("Items per page (default: D, max: M)"). None sets `response_pagination`. The declared defaults are GitHub 100/100, GitLab 100/100 (`gitlab()` = `github()`), Gitea 50/100 and Bitbucket 50/100. They disagree with the providers (30, 20, 30 with cap 50, and 10).

| API | Endpoint | Response | Legacy style | Materialized query params | Resolves as |
|---|---|---|---|---|---|
| Bitbucket | GetSource | `Binary` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | **not a list** (Binary); params stay plain |
| Bitbucket | ListPullRequests | `PaginatedResponse<PullRequest>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListPullRequestComments | `PaginatedResponse<PullRequestComment>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssues | `PaginatedResponse<Issue>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssueComments | `PaginatedResponse<IssueComment>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListIssueChanges | `PaginatedResponse<IssueChange>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListTags | `PaginatedResponse<Tag>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListDownloads | `PaginatedResponse<Download>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Bitbucket | ListWorkspaceRepos | `PaginatedResponse<Repository>` | PageNumber 50/100 | `page`, `pagelen` (both optional Integer) | wrapper list (table entry) |
| Gitea | ListPullRequests | `Vec<PullRequestSummary>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListPullRequestFiles | `Vec<PullRequestFile>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssues | `Vec<IssueSummary>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssueComments | `Vec<IssueComment>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListIssueTimeline | `Vec<TimelineEvent>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListTags | `Vec<RepoTag>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListReleases | `Vec<Release>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| Gitea | ListOrgRepos | `Vec<RepositoryInfo>` | PageNumber 50/100 | `page`, `limit` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListPullRequests | `Vec<PullRequestSummary>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListPullRequestFiles | `Vec<PullRequestFile>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssues | `Vec<IssueSummary>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssueComments | `Vec<IssueComment>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListIssueTimeline | `Vec<TimelineEvent>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListTags | `Vec<RepoTag>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListReleases | `Vec<Release>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitHub | ListWorkflowRuns | `WorkflowRunsResponse` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | wrapper list (table entry) |
| GitHub | ListOrgRepos | `Vec<RepositoryInfo>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListRepositoryTree | `Vec<TreeItem>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListGroupProjects | `Vec<Project>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListMergeRequests | `Vec<MergeRequest>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListMergeRequestCommits | `Vec<Commit>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListIssues | `Vec<Issue>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListIssueNotes | `Vec<Note>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListTags | `Vec<Tag>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListReleases | `Vec<Release>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
| GitLab | ListProjectPipelines | `Vec<Pipeline>` | PageNumber 100/100 | `page`, `per_page` (both optional Integer) | root list (ApiDefault) |
##### OpenAI group reconciliation

Classification comes from the current reply struct fields plus the declared query params. It matches the inventory's group sizes exactly.

| Group | Count | Members (current definitions) | Request inputs declared | Reply signal |
|---|---|---|---|---|
| S6 (API default `after <- last_id`) | 36 | ListVoiceConsents, ListBatches, ListChatCompletions, GetChatCompletionMessages, ListContainers, ListContainerFiles, ListConversationItems, ListEvals, GetEvalRuns, GetEvalRunOutputItems, ListFiles, ListFineTuningCheckpointPermissions, ListFineTuningJobCheckpoints, AdminApiKeysList, ListAuditLogs, ListOrganizationCertificates, ListInvites, ListProjects, ListProjectApiKeys, ListProjectCertificates, ListProjectRateLimits, ListProjectServiceAccounts, ListProjectSpendAlerts, ListProjectUsers, ListOrganizationSpendAlerts, ListUsers, ListInputItems, ListVectorStores, ListFilesInVectorStoreBatch, ListVectorStoreFiles, ListVideos, ListSkills, ListSkillVersions, ListThreadItemsMethod, ListThreadsMethod, BetaListInputItems | `after`, `limit` on all 36 | `has_more`, `first_id`, `last_id` |
| S6 item-id variant (`after <- LastItem(id)`) | 2 | ListFineTuningEvents, ListPaginatedFineTuningJobs | `after`, `limit` | `has_more` only (no `first_id`/`last_id` in the struct): confirmed |
| S7 (`after <- next`) | 9 | ListGroups, ListGroupRoleAssignments, ListGroupUsers, ListProjectGroups, ListRoles, ListUserRoleAssignments, ListProjectGroupRoleAssignments, ListProjectRoles, ListProjectUserRoleAssignments | `limit`, `after`, `order` | `has_more`, `next` (vendored spec: "Cursor to fetch the next page") |
| S8 (`page <- next_page`) | 11 | UsageCosts, UsageAudioSpeeches, UsageAudioTranscriptions, UsageCodeInterpreterSessions, UsageCompletions, UsageEmbeddings, UsageFileSearchCalls, UsageImages, UsageModerations, UsageVectorStores, UsageWebSearchCalls | `page` and `limit` on **all 11**; `start_time` required (not pager-managed) | `has_more`, `next_page` |
| Opt-out: no continuation | 1 | ListModels | none | `object`, `data` only: confirmed |
| Opt-out: provider gap | 2 | RetrieveVectorStoreFileContent (GET), SearchVectorStore (POST) | none; the vendored spec has only path params, and `VectorStoreSearchRequest` has `query`, `rewrite_query`, `max_num_results`, `filters`, `ranking_options` | `has_more`, `next_page`: confirmed against `specs/openai/openapi.yaml` |

**`before` declared (9; each needs a backward leg `before <- first_id`):** ListAuditLogs, ListProjectRateLimits, ListProjectSpendAlerts, ListOrganizationSpendAlerts, ListVectorStores, ListFilesInVectorStoreBatch, ListVectorStoreFiles, ListThreadItemsMethod, ListThreadsMethod. All 9 are S6 and reply with `first_id`.

**Non-GET list-shaped replies (D43: not lists, no entry):** CreateConversationItems (POST `ConversationItemList`, which has `has_more`/`first_id`/`last_id`), CreateFineTuningCheckpointPermission (POST, list wrapper), Activate/DeactivateOrganizationCertificates and Activate/DeactivateProjectCertificates (POST), CreateEmbedding, CreateImage/CreateImageEdit/CreateImageVariation.

**Inventory rows that changed or were wrong (none blocks a group assignment):**
- UsageCosts and UsageImages **do** declare `limit`. The inventory said "UsageCosts and UsageImages declare no `limit`"; UsageCosts' `limit` has been present since the 2026-07-31 import (`61b6cc7aa`). All 11 S8 endpoints can therefore adopt `limit` as their size knob.
- AdminApiKeysList declares `after`, `order` **and `limit`**; the inventory listed only `after`, `order`.
- ListAuditLogs also declares array filters (`project_ids[]`, `event_types[]`, ...), UsageFileSearchCalls declares `vector_store_ids`, and UsageWebSearchCalls declares `context_levels`. None of them is pager-related.
- Wording only, not a contradiction: the `after` descriptions of ListGroups, ListGroupUsers and ListProjectGroups say "ID of the last group/user", but their reply schema documents `next` as "Cursor to fetch the next page". S7 (`after <- next`) agrees with the reply schema, so they stay S7.

##### Discrepancies and gaps

- **D1, Bitbucket `GetSource` (was `ListDirectoryContents`).** Commit `9e65a90b5` (2026-10-12) merged the directory listing into `GetSource` with `response: ApiResponse::Binary`, because one route serves both a JSON directory page and raw file bytes. The legacy declaration (page/pagelen) is still on it. Under R4 it is **not a list**: it is opaque, and a wrapper entry cannot name `values` on a `Binary` body. Bitbucket therefore has **8** wrapper lists, not the 9 in the authoring table. Its `page`/`pagelen` fields are kept, and sniff (`sniff/lib/src/remote/bitbucket.rs:177-179`) keeps hand-paging it. Coverage counts should say 8 + 1 declared non-list.
- **D2, ElevenLabs `ListVoices` input name.** The definition declares `page_token` (string) and `page_size` (`deadcabd9`, 2026-10-12). The research note `schematic/docs/research/elevenlabs-api.md` (2026-05-07, lines 489-490) names the request input **`next_page_token`**. `biscuit-speaks` sends `page_token`, but only a wiremock test checks it (`biscuit-speaks/lib/tests/elevenlabs_http.rs:150`), which is not provider evidence. Under the table's contradiction rule, the cursor leg stays unpaginated until the sent name is confirmed. If `next_page_token` is right, the `page_token` param is a definition bug, and fixing it is an endpoint-literal change that R3 does not list as an exception. `page_size` matches and can be adopted.
- **D3, GitLab `ListIssueParticipants`.** It is a GET `Vec<User>` in an API that gets a default, so R4 makes it **inherit the default unless it has an opt-out entry**. The authoring table calls it "believed paginated, unverified" and R28 requires it to stay unpaginated, so it needs an opt-out entry (a second GitLab entry alongside default 14's enumeration).
- **D4, Gitea `GetTagReference`.** This is the only Gitea GET `Vec<T>` needing an opt-out, as the spec expects. GitHub's `GetTagReference` returns a single `GitRef`, so GitHub needs no opt-out. Every GitHub GET `Vec<T>` (8) is a real list.
- **D5, EMQX `ListAuthUsers`.** The definition still declares root `Vec<AuthUser>` (both variants). Because EMQX has no default, it resolves as an unpaginated list. The type fix stays pending the author's approval.
- **D6, the "~519 Endpoint literals" figure.** The current count is 511 production literals (547 endpoints minus 36 shared EMQX), or 569 raw `Endpoint {` matches including test modules. Neither is 519, so the R3 zero-diff check should state its own counting rule.
- **G1, unpaginated wrapper collections resolve "not a list", not "Unpaginated list".** R4 promotes only GET `Vec<T>` without an entry. The U wrappers that R28 says should "resolve to `Unpaginated`" have no root array: Anthropic `ListModels`, EMQX's 8 paginated lists ×2 (7 wrappers per variant; `ListAuthUsers` is a root array), HF `ListModelDiscussions`, Gitea trees, and ElevenLabs' 3 other wrappers. To assert `Unpaginated` rather than "not a list", they need explicit wrapper entries marked unpaginated, with item types. That is 43 wrapper collections, or 18 if only the U rows that R28 names get entries (Anthropic 1, EMQX 7×2, HF 1, Gitea trees 2). Otherwise the coverage test asserts "not a list". The plan should decide which.
- **G2, OpenAI page-size bounds are not uniform.** `limit` documents 1..100 with default 20 on 19 endpoints, 1..10000 with default 10000 (ListFiles), 0..1000 with default 100 (ListGroups, ListGroupUsers), default 1000 (ListRoles, ListProjectRoles), 1..180 with default 7 (UsageCosts), and a bucket-width-dependent maximum (31/168/1440) on the other usage endpoints. Several give no bound. An API-default `PageSize` must carry no `max` (or a per-entry override), or `InvalidPageSize`/R13 checks will reject valid requests.
- **G3, OpenAI cursor field types vary.** `first_id`/`last_id` are typed `String`, `Option<String>`, `serde_json::Value`, or `Option<serde_json::Value>` across S6 wrappers, and S7 `next`/S8 `next_page` are `serde_json::Value` (nullable). Selectors must read a string or null from any of these. Item types include `serde_json::Value` (GetChatCompletionMessages, RetrieveVectorStoreFileContent), which default 22 allows.
- **G4, GitLab keyset constants vs plain params.** `ListGroupProjects` already declares plain `order_by` and `sort`, the same names as the keyset alternate's constants. Alternates are not generated, but alternate validation (R2/R6) must define whether a constant may shadow a declared param.
- **G5, ElevenLabs wire names with hyphens.** `ListServiceAccounts` is at `service-accounts` (serde rename) and `ListServiceAccountApiKeys` at `api-keys`. Any entry must use the wire path, not the Rust field name. This is moot while they stay unpaginated.
- **G6, the "ElevenLabs gets a default" assumption.** The authoring table gives ElevenLabs three different strategies and no API default. Its only GET `Vec<T>` (`ListModels`) therefore needs no opt-out unless a default is later added.

##### Adoption candidates

Every row below is query-located. A row is adopted only if the knob is authored (R7). Under each listed strategy the knob is authored, except where noted.

| API | Endpoints | Param | Location | Strategy input | Knob authored? |
|---|---|---|---|---|---|
| GitHub | 9 (8 root + ListWorkflowRuns) | `page` | query | PageNumber page | yes, `.page()` |
| GitHub | same 9 | `per_page` | query | PageSize | yes, `.page_size()` |
| GitLab | 9 declared (not ListIssueParticipants) | `page`, `per_page` | query | PageNumber page / PageSize | yes / yes |
| Gitea | 8 declared | `page`, `limit` | query | PageNumber page / PageSize | yes / yes |
| Bitbucket | 8 wrappers | `pagelen` | query | NextLink adjustable size (query) | yes, `.page_size()` |
| Bitbucket | 8 wrappers | `page` | query | none (NextLink has no page input) | **no, field kept** |
| Bitbucket | GetSource | `page`, `pagelen` | query | not a list | **no, fields kept** |
| OpenAI | 47 (S6 36 + item-id 2 + S7 9) | `after` | query | Cursor forward | yes, `.after()` |
| OpenAI | 9 (listed above) | `before` | query | Cursor backward leg | yes, `.prev()`/`.before()` |
| OpenAI | 58 (all paginated) | `limit` | query | PageSize | yes, `.page_size()` (bounds: G2) |
| OpenAI | 11 S8 | `page` | query | Cursor (named `page`) | yes, cursor knob |
| OpenAI | 47 with `order` | `order` | query | none (plain filter) | not adopted, field kept |
| ElevenLabs | ListVoices | `page_size` | query | PageSize | yes, if authored |
| ElevenLabs | ListVoices | `page_token` | query | Cursor input `next_page_token` per research | **pending D2**: name mismatch, so no adoption until confirmed |
| ElevenLabs | GetHistory, ListSharedVoices | (none declared) | — | new inputs `start_after_history_item_id`/`page_size`, `page`/`page_size` | n/a (no field removed) |

**Call sites of adopted fields** (a quick grep of the 8 `schematic-schema` dependents plus schematic's own tests and docs). This is a pointer for the 1.2 migration ledger, not the full ledger:
- `sniff/lib/src/remote/github.rs:454`: `list_workflow_runs(..).per_page(limit)` (adopted).
- `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576-578`: `list_voices().page_size(..)` and `.page_token(..)` (adopted if ListVoices is authored). The loop itself is a hand-written pager.
- `schematic/schema/tests/github_client.rs:549`: `list_pull_requests(..).per_page(5)`.
- Docs: `schematic/docs/runtime/calling-an-api.md:22,83,221` (`.per_page`), `schematic/docs/runtime/deadlines-and-timeouts.md:43` (`.page(page)` hand loop), `.claude/skills/schematic-define/schematic/SKILL.md:122`, `.claude/skills/schematic-define/schematic/gen.md:247`.
- Not affected (fields kept): `sniff/lib/src/remote/bitbucket.rs:177-179` and `schematic/schema/tests/non_json_endpoints.rs:65` (`GetSource` `pagelen`/`page`).
- No hits in `homelab/*`. `sniff` gitlab/gitea call sites use no page setters.

Artifacts: `/tmp/pagination-phase1/{endpoints.jsonl,analyzed.json,analyze.py,render.py,dump/}`.

#### Phase 1: dependents of schematic-schema, adopted-field ledger, serialized authoring inputs

Worktree: `/Volumes/coding/wt/rusty-biscuit/feat-modern-config` (branch `feat/modern-config`), 2026-10-13. Read-only survey; no repo file edited.

##### Dependents of schematic-schema

Found by searching every `Cargo.toml` (normal and dev sections) for `schematic-schema` / `schematic-definitions`:

| Crate (manifest) | Dependency | Generated modules used | Paginated API used? |
| --- | --- | --- | --- |
| `sniff` (`sniff/lib/Cargo.toml:44`) | schematic-schema | `github`, `gitlab`, `gitea`, `bitbucket`, `schematic_runtime` (`sniff/lib/src/remote/{github,gitlab,gitea,bitbucket,envelope}.rs`) | **yes** |
| `biscuit-speaks` (`biscuit-speaks/lib/Cargo.toml:27`) | schematic-schema | `elevenlabs`, `prelude`, `schematic_runtime` (`biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`) | **yes** (ElevenLabs `ListVoices`) |
| `homelab` lib (`homelab/lib/Cargo.toml:10`) | schematic-schema | `eversolo`, `samsung_smart_tv`, `samsung_smart_tv_remote_ws`, `ws_shared`, `schematic_runtime` | no |
| `eversolo-integration` (`homelab/eversolo-integration/Cargo.toml:13`) | schematic-schema | `eversolo` (types), `unfolded_circle_integration_ws` | no |
| `arcam-amp-integration` (`homelab/arcam-amp-integration/Cargo.toml:12`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `sony-receiver-integration` (`homelab/sony-receiver-integration/Cargo.toml:12`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `unfolded-integration-helper` (`homelab/unfolded-integration-helper/Cargo.toml:10`) | schematic-schema | `unfolded_circle_integration_ws` | no |
| `schematic-schema` own tests (`schematic/schema/tests/*.rs`) | (self) | `github`, `bitbucket`, `huggingface`, `artificial_analysis`, ws modules | **yes** |
| `schematic-gen` tests (`schematic/gen/tests/l1/fresh_output_compile.rs`) | none in Cargo; regenerates the crate into a temp dir (`FreshCrate::generate`) and compiles embedded Rust test programs against it | `gitea`, `gitlab`, `openai`, fixture APIs | **yes** (wire-baseline program) |

**Correction to the spec's list:** `model-citizen` does **not** depend on `schematic-schema`. It depends on `schematic-definitions` only (`model-citizen/lib/Cargo.toml:39`). It uses response/body types and no endpoint params: `schematic_definitions::ollama::{ShowModelResponse, ShowModelBody}` (`model-citizen/lib/src/scanner/ollama.rs:276,284,295,642`) and `schematic_definitions::lmstudio::{ListModelsResponse, ModelInfo}` (`scanner/lmstudio.rs:374,405`). R32 still holds for it trivially. The homelab dependents are the **five** crates listed above.

Other `schematic-definitions` dependents: `schematic-schema` (`schematic/schema/Cargo.toml:15`) and `schematic-gen` (`schematic/gen/Cargo.toml:26`). None outside `schematic/` reads `EndpointParams`. Inside, the legacy pagination metadata is read in two places, both by `schematic-gen` for the `Paginated` marker: `schematic/gen/src/output/assemble/api.rs:88` and `schematic/gen/src/codegen/traits.rs:91` (`p.pagination.is_some()`). `schematic/gen/src/definition_gen.rs:320-321` writes `pagination: None, response_pagination: None` into the definitions it emits.

##### Adopted-field migration ledger

The 35 declared endpoints are GitHub 9, Gitea 8, GitLab 9 and Bitbucket 9 (`with_pagination` in `schematic/definitions/src/{github,gitea}/mod.rs`, `gitlab/endpoints/*.rs`, `bitbucket/mod.rs:105` `bitbucket_pagination()`). GitHub `ListWorkflowRuns` is one of the GitHub 9. Bitbucket `GetSource` (`bitbucket/mod.rs:175-190`, `ApiResponse::Binary`) is one of the Bitbucket 9.

###### Production dependents

| # | Crate | file:line | Endpoint / type | Field | Kind | Single page or loop | Planned migration |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | sniff | `sniff/lib/src/remote/github.rs:451-454` | GitHub `ListWorkflowRuns` / `ListWorkflowRunsCall` | `per_page` | call setter `.per_page(limit as i64)` | single page, bounded by `limit`. The call is `.clone()`d and resent `.anonymous()` on 401/403 (lines 457-471) | **fetch_page**: `client.pager(ListWorkflowRunsRequest::new(owner, repo)).page_size(limit).fetch_page()`. The result type changes from the `workflow_runs` wrapper to `Page<WorkflowRun>`, and the code at ~475+ that reads the wrapper (`total_count` etc.) must adapt. **Gap to check:** the pager or `fetch_page()` needs per-request `.anonymous()` (and must be clonable or rebuildable) to keep the anonymous retry. |
| 2 | sniff | `sniff/lib/src/remote/bitbucket.rs:175-177` | Bitbucket `GetSource` / `GetSourceCall` | `pagelen` | call setter `.pagelen(100)` | inside a hand loop `for page in 1..=MAX_LISTING_PAGES` (`:174`, const `:149`) that stops when the body's `next` is `None` (`:188-192`) | **Depends on whether `GetSource` gets a strategy (open; see Notes).** If it is paginated (NextLink, size `pagelen`), `pagelen` is adopted: replace the loop with pager iteration or `collect_all` with `.page_size(100)`. Its page limit is 10 today (`MAX_LISTING_PAGES`), and it reports `truncated=true` at the limit (matches `Incomplete{PageLimitReached}`). If it is opted out (Binary response), **no change**. |
| 3 | sniff | `sniff/lib/src/remote/bitbucket.rs:179` | Bitbucket `GetSource` / `GetSourceCall` | `page` | call setter `call.page(page)` | same hand loop | Under R7's knob rule a NextLink strategy has no page-number knob, so `page` is **not adoptable** and the field stays (no change). R7 also says "every `with_pagination` query parameter … is adopted". These conflict for Bitbucket `page`; see Notes. If the loop is replaced per row 2, this call goes away with it. |
| 4 | biscuit-speaks | `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:576` | ElevenLabs `ListVoices` / `ListVoicesCall` | `page_size` | call setter `.page_size(VOICES_PAGE_SIZE)` | hand loop over all pages (`list_voices_raw`, `:561-615`), ending on `has_more` plus `next_page_token` | If the `ListVoices` Cursor strategy adopts `page_size` (it needs the PageSize knob): **remove** the hand loop and use `client.pager(ListVoicesRequest::new()).page_size(VOICES_PAGE_SIZE).collect_all()`. This keeps the "has_more without token" guard as an engine error. Otherwise, use `fetch_page()` per iteration. |
| 5 | biscuit-speaks | `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:577-579` | ElevenLabs `ListVoices` / `ListVoicesCall` | `page_token` | call setter `call.page_token(token)` | same hand loop | Adopted only if the strategy's cursor input is `page_token`. The definition sends `page_token` (`schematic/definitions/src/elevenlabs/endpoints/voices.rs:17-23`), but the spec table names the input `next_page_token`. If the input is authored as `page_token`, it is adopted: same migration as row 4. If it is `next_page_token`, it does **not** collide, the `page_token` field stays, and a second wire name appears, which is wrong. Confirm the sent name before adopting. |

Not affected, but similar in name:
- `sniff/lib/src/remote/bitbucket.rs:600-601`: `pagelen: None, page: None` belong to the **response** struct literal `PaginatedResponse { .. }` (fallback for `ListDownloads`). Response fields are not request fields: **no change**.
- `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:613`: `next_page_token: None` is in the **response** literal `ListVoicesResponse`: **no change**.
- These first-page-only plain calls set no adoptable field, so they need **no change** (R11a: the typed send still returns the wrapper): sniff `github.rs:349,377,381`, `gitlab.rs:330,359,363`, `gitea.rs:404,444,448`, `bitbucket.rs:372-389` (sets `.state()` only), `bitbucket.rs:496,560,589-591`.
- `sniff/lib/src/remote/gitea.rs:166` `get_git_tree_recursive` sets no `per_page`, and the spec leaves the tree unpaginated: **no change**.
- homelab (5 crates) and model-citizen: **zero uses**.
- Other `.limit(..)`/`limit:` hits in sniff (`remote/types.rs:364,388,643,664`, `provider.rs:151,765`, `focused.rs:330,720`, `filesystem/*`) belong to sniff's own query types and do not touch schematic: **no change**.
- **Hand-written pagination outside scope:** `sniff/lib/src/remote/focused.rs` builds page and size params by hand (`pagination_params` `:1922`, `direct_job_pagination_params` `:1932`) and reads `Link rel="next"` (`:104`, `:1905`) over its own HTTP client. It does not use schematic-schema. Retiring it is out of scope.

###### schematic-internal tests (same-phase migration required)

| # | file:line | Endpoint / type | Field | Kind | Planned migration |
| --- | --- | --- | --- | --- | --- |
| 6 | `schematic/schema/tests/github_client.rs:549` | GitHub `ListPullRequests` / `ListPullRequestsCall` | `per_page` | call setter `.per_page(5)` | **fetch_page** (`client.pager(req).page_size(5).fetch_page()`), or drop `per_page` from the test and assert only `state=closed`. The test compares the endpoint method with generic `send`, so dropping it keeps its intent. |
| 7 | `schematic/schema/tests/github_client.rs:556` | `ListPullRequestsRequest` | `per_page` | request builder `.with_per_page(5)` | same as row 6 |
| 8 | `schematic/schema/tests/non_json_endpoints.rs:65` | Bitbucket `GetSource` / `GetSourceCall` | `pagelen` | call setter `.pagelen(10)` | Only if `GetSource` is paginated: **remove** the setter (the test is about binary decoding; `pagelen` is incidental). Otherwise no change. |
| 9 | `schematic/gen/tests/l1/fresh_output_compile.rs:1588-1589` (inside the `WIRE_CAPTURE_TEST` raw string, compiled against the regenerated crate) | Gitea `ListPullRequestsRequest` | `page`, `limit` | field assignment `scalars.page = Some(2); scalars.limit = Some(0);` | Both are adopted (PageNumber `page` plus size `limit`). Wire line 1 of `schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt` (`?page=2&limit=0&state=…`) depends on them. Either capture through `gitea.pager(req).page(2)…fetch_page()`, if the pager exposes a start-page knob and sends `limit=0` (the R13 bounds may refuse 0, so the baseline changes), or drop the fields and update the baseline line (an approved wire change, which must be logged). |
| 10 | `fresh_output_compile.rs:1608` | Gitea `ListPullRequestsRequest` | `page` | assignment `paged.page = Some(1)` | Same choice as row 9. Baseline line 5 (`base-url-with-query … &page=1`). |
| 11 | `fresh_output_compile.rs:1625, 1627` | OpenAI `ListConversationItemsRequest` (`schematic/definitions/src/openai/mod.rs:535`, declares `limit`, `order`, `after`, `include`) | `limit`, `after` | assignment | Adopted (Cursor `after` plus size `limit`). Baseline line 7 (`?limit=5&order=a%2Bb&after=x%25y&include=…`) checks `after` percent-encoding and declared-order interleaving with `order`. A pager-sent query may reorder params. Either migrate to `fetch_page()` and re-baseline, or move the encoding check to a non-adopted param. |
| 12 | `fresh_output_compile.rs:1638` | OpenAI `BetaListInputItemsRequest` (`openai/mod.rs:4997`) | `limit` | assignment | Adopted. Baseline line 8 (`?beta=true&limit=3&include=…`). Same choice as row 11. |
| 13 | `fresh_output_compile.rs:1654-1655` (`into_parts_returns_unencoded_query_pairs_apart_from_the_path`, same raw string) | Gitea `ListPullRequestsRequest` | `page`, `limit` | assignment | Assert on non-adopted params only (`state`), or remove. `into_parts` will not carry adopted fields. |
| 14 | `fresh_output_compile.rs:1691` | OpenAI `ListConversationItemsRequest` | `limit` | assignment | Same as row 13. The expected `parts.query` at `:1695` changes. |
| 15 | `fresh_output_compile.rs:1698` | OpenAI `BetaListInputItemsRequest` | `limit` | assignment | Same as row 13 (`:1701`). |

Not affected (the APIs have no strategy, so nothing is adopted):
- Fixture APIs in `fresh_output_compile.rs:897,925,962,1235,1277,1279`: `FixtureBearer::list_widgets().limit(..)`, `FixtureNames::list_items(..).limit(..)`, `FixtureSettings` `EchoRequest::with_page`.
- Codegen unit tests on ad-hoc endpoints without strategies: `schematic/gen/tests/l1/query_params_codegen.rs:259,482-494,586,673-678`, `schematic/gen/src/codegen/request_structs/tests.rs:863,1065-1338`, `schematic/gen/src/codegen/endpoint_methods/tests.rs:446`, `schematic/gen/tests/l1/e2e_generation.rs:888-912`.
- Generated doc examples in `schematic/schema/src/**/requests.rs` (for example `gitlab/requests.rs:15-16`) are regenerated.

###### Documentation that shows adopted setters (drift, not compiled)

No test reads these files. Update them when the fields are removed:
- `schematic/docs/runtime/calling-an-api.md:22, 33, 83, 221`: `list_releases(..).per_page(10)`, `list_pull_requests(..).per_page(50)`
- `schematic/docs/runtime/deadlines-and-timeouts.md:43`: hand loop with `.page(page)`; a natural pager example
- `schematic/docs/generation/request-side.md:84`: Gitea `ListPullRequestsRequest::with_page(2)`, with the expected `parts.query` containing `page`
- `.claude/skills/schematic-define/schematic/SKILL.md:122`, `.claude/skills/schematic-define/schematic/gen.md:241-247`
- `sniff/docs/repos/repo-tech-design.md:857`: quotes the `list_workflow_runs(..).per_page(..)` line

##### Grep commands run

Run from the worktree root. `target/` was excluded.

```sh
grep -rn --include=Cargo.toml -E 'schematic-(schema|definitions)' .
grep -rn --include='*.rs' -E 'schematic_(schema|definitions)::[a-z_]+' homelab sniff/lib biscuit-speaks model-citizen schematic/schema/tests schematic/gen
P='\.(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\(|\b(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\s*[:=]'
grep -rnE "$P" sniff/lib/src/remote biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs homelab/lib/src model-citizen/lib/src
P2='\.(page|per_page|limit|after|before|pagelen|page_size|page_token|next_page_token|start_after_history_item_id)\(|with_(page|per_page|limit|after|before|pagelen|page_size|page_token)\(|\.(page|per_page|limit|after|before|pagelen|page_size|page_token)\s*=\s'
grep -rnE "$P2" --include='*.rs' sniff biscuit-speaks homelab model-citizen so-you-say
grep -rnE '<P with "= Some|None|[0-9]">' schematic/schema/tests schematic/gen/tests schematic/gen/src schematic/runtime schematic/definitions/tests schematic/define/tests
grep -rnE '\bwith_(page|per_page|limit|after|before|pagelen|page_size|page_token|start_after_history_item_id)\(' --include='*.rs' --include='*.md' .
grep -rnE '[A-Z][A-Za-z]+Request\s*\{|Request::(new|default)\(|\.send\(' --include='*.rs' sniff/lib/src/remote biscuit-speaks/lib homelab model-citizen/lib
grep -rnE '\.full\(\)|link_header|rel="next"|Link' sniff/lib/src/remote/*.rs
grep -rnE '\.(pagination|response_pagination)\b|PaginationStyle|PaginationResponse' --include='*.rs' .
grep -rnE '<setter regex>' --include='*.md' schematic/docs schematic/README.md schematic/skill .claude/skills sniff/docs biscuit-speaks
```

Results: homelab (all five crates) and model-citizen have **zero** matches on schematic request or call types. Every match is listed in the ledger above.

##### Serialized authoring inputs

**Conclusion: serde on the define model is only a round-trip and export facility. No configuration reader exists for REST API definitions, so the new pagination model would flow through no deserializer. "Input robustness" covers only the Rust-authored model and the OpenAPI importer, which is a separate import format.**

Evidence:

1. **The core REST authoring types do not derive `Deserialize`, or `Serialize`.**
   - `RestApi`: `schematic/define/src/types.rs:123`, `#[derive(Debug, Clone, PartialEq, Eq)]`
   - `Endpoint`: `types.rs:445`, same derives
   - `EndpointParams`: `schematic/define/src/params/mod.rs:85`, `#[derive(Debug, Clone, PartialEq, Eq, Default)]`
   - `ParamDef`: `params/param_def.rs:24`
   - `QueryParamType` (`:68`) and `ParamStyle` (`:113`): no serde
   - `PaginationStyle`: `schematic/define/src/pagination.rs:35`, `#[derive(Debug, Clone, PartialEq, Eq)]`

   A `RestApi`/`Endpoint`/`EndpointParams`/`PaginationStyle` value therefore cannot be read from JSON, YAML or TOML at all.
2. **Define-side types that do derive `Deserialize`:**
   - `PaginationResponse` (`pagination.rs:276`)
   - `RestMethod` (`types.rs:54`)
   - `ApiResponse` (`response.rs:43`)
   - `FormFieldKind`, `FormField`, `ApiRequest` (`request.rs:41,108,325`)
   - `Schema` (`schema.rs:113`)
   - `AuthStrategy`, `AuthMethod`, `EnvAuthStrategy`, `AuthPolicy`, `ApiKeyLocation` (`auth.rs:71,193,226,256,338`)
   - `OAuth2Config`, `OAuth2GrantType`, `PkceRequirement`, `OAuth2ClientAuthMethod` (`oauth.rs:31,52,64,77`)
   - `EnvList`, `ApiKeyEnv`, `EnvMapping` (`headers/env.rs:26,108,148`)
   - OpenAPI `x-schematic` extensions `SchematicDocExtension`, `SchematicOpExtension`, `SchematicSchemaExtension`, `AuthExtension` (`openapi/extensions.rs:40,86,127,144`)
   - WebSocket model: `FrameFormat`, `RequestIdType`, `CorrelationHints`, `AuthFlowHints`, `HeartbeatHints`, `WebSocketRuntimeHints`, `WebSocketEndpointHints` (`websocket/hints.rs:18,57,98,130,157,194,243`); `MessageDirection`, `MessageSchema`, `ConnectionLifecycle` (`websocket/lifecycle.rs:25,60,102`); `WebSocketEndpoint`, `WebSocketApi` (`websocket/mod.rs:151,201`); `ParamType`, `ConnectionParam` (`websocket/params.rs:21,57`)

   `PaginationResponse` derives serde, but it is only reachable inside `EndpointParams`, which does not.
3. **Production readers of these types: none.**
   - Every `serde_json::from_str::<DefineType>` in `schematic/define/src` is inside `#[cfg(test)]`. Examples: `pagination.rs:472-509`, `types.rs:549`, `auth.rs:536,591`, `oauth.rs:108-207`, `request.rs:650-866`, `openapi/extensions.rs:248-510`, `websocket/mod.rs:296-857`.
   - The `x-schematic` extension types are only **written** by the exporter (`openapi/export/mod.rs:78`, `openapi/export/paths.rs:84`). The importer never reads them back; nothing under `define/src/openapi/import` references them.
   - Production deserialization in `schematic/gen/src` is limited to `serde_json::Value` in `asyncapi_import.rs:218` (the AsyncAPI document), and to generated client and WS runtime code (`codegen/client/methods.rs:223,231`, `ws_codegen/client.rs:715,723`, `ws_codegen/host.rs:359,372`) that decodes API **responses**, not definitions. `schematic/definitions/src` deserializes only in tests (for example `gitlab/types.rs:65`).
4. **CLI:** the `schematic-gen` subcommands (`schematic/gen/src/main.rs`) take `--api <NAME>`, a name looked up in the compiled `schematic_definitions` registry, not a definition file. The `import --input` and `import-asyncapi --input` flags take OpenAPI or AsyncAPI documents.
5. **The OpenAPI importer is a separate import format.** `schematic/define/src/openapi/import/builder.rs:223-250` (`parse_source`) reads a path via `serde_yaml_ng::from_str` into `openapiv3::OpenAPI`, or JSON via `serde_json::from_str`. It then maps to a `RestApi` in Rust. It emits no pagination: there is no `pagination` reference under `define/src/openapi`, and `gen/src/definition_gen.rs:320-321` writes `pagination: None`. Per D57 item 4, OpenAI pagination will come from a hand-written overlay that the generated definitions call (`openai_pagination::apply(api)`), not from the imported document.

Existing serde-related tests in `schematic/define`, all round trips except where noted:
- `types.rs:544` `rest_method_serde_roundtrip`
- `pagination.rs:463,477,487,502`: the `pagination_response_serialization_*` tests (4)
- `request.rs:647,655,665,677,688,696,851`: form-field kind and form-field round trips, `api_request_serde_roundtrip_{json,form_data}`, `integration_all_request_variants_serializable`
- `auth.rs:533,585`
- `oauth.rs:131,152,183,215`. `endpoint_with_oauth_scopes_serde` at `:215` only asserts field access, because `Endpoint` has no serde.
- `websocket/mod.rs:293,355,366,382,406,441,470,551,567,578,599,627,648,660,677,698,713,727`, including deserialize-with-defaults tests at `:660,698,713`
- `openapi/extensions.rs:236,336,381,459,479,488,502`
- `openapi/options.rs:194,215,235,254`: export serialize to JSON or YAML, then re-parse as a `Value`
- `tests/openapi_tests.rs:62`: parses a YAML OpenAPI fixture into `openapiv3::OpenAPI` (importer input)

Implication for the plan: input-robustness work for the new `PageStrategy` or API fields needs **no** serde derive and no tolerance of unknown or missing fields. If a derive is added for symmetry with `PaginationResponse`, it is optional and covered only by round-trip tests. Validation (R2/R5/R7) operates on the Rust-constructed model.

##### Notes

- **The R7 adoption list conflicts with the knob rule for Bitbucket `page`.** R7 says every `with_pagination` query param on the 35 endpoints is adopted. `PaginationStyle::bitbucket()` materializes both `page` and `pagelen` (`schematic/define/src/pagination.rs:105-106`). The Bitbucket strategy is `NextLink`, with size `pagelen` and no page-number input, and R7 forbids adopting a param whose knob is not authored. Result: `pagelen` is adopted and `page` stays a request field (with no `CollidesWithEndpointParam`, since NextLink has no `page` input). This decides ledger row 3. Log it as the default chosen.
- **Bitbucket `GetSource` is one of the 35 but returns `ApiResponse::Binary`** (directory listing or raw file). The authoring table says each Bitbucket list is "a table entry at `values` with its item type". That cannot be checked structurally against a Binary response, and a file path returns no envelope. The planner must decide to paginate it (and handle the dual shape) or opt it out. This decides ledger rows 2, 3 and 8.
- **ElevenLabs `ListVoices` name mismatch:** the definition sends `page_token`. The spec table names the cursor input `next_page_token`, which is the response field. Author the input as `page_token <- next_page_token`, or `page_token` is not adopted and a second param name appears. `GetHistory` and `ListSharedVoices` declare **no** params (`params: None`, `elevenlabs/endpoints/history.rs:13`, `voices.rs:128`), so adding their strategies removes no field.
- **OpenAI:** no production dependent uses `schematic_schema::openai`. The only uses are the `schematic-gen` wire-baseline tests (rows 9-15). Re-baselining `schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt` lines 1, 5, 7 and 8 (and possibly line 2) would be a deliberate wire-test change and must be logged. Alternatively, keep the lines by sending through `fetch_page()` with matching knob values, if the pager serializes adopted params in the same position and order.
- **The sniff `ListWorkflowRuns` anonymous retry** relies on `Call: Clone` plus `.anonymous()`. Check that the pager or `fetch_page()` surface offers per-send credential override and is cheap to rebuild. If it does not, the migration in row 1 loses behavior.
- The legacy `Paginated` marker readers (`gen/src/output/assemble/api.rs:88`, `gen/src/codegen/traits.rs:91`) are removed in the generator phase (spec phase 7). They are not dependents but are listed for completeness.

#### Artifact baseline (task 1.3)

##### Host and toolchain context

- Apple M4 Max, 16 CPUs, 128 GiB, Darwin 27.2.0 arm64. rustc and cargo 1.98.1, nextest 0.9.136, just 1.58.0.
- Compiler cache: kache 0.26.3 through `~/.cargo/config.toml` (`build.rustc-wrapper = "kache"`). `RUSTC_WRAPPER` and `CARGO_TARGET_DIR` are unset. The repository has no `.cargo/config.toml`.
- HEAD `0cd6075e6f0bffd6d9c791a3ff2f93616545ba1f`. The only working-tree change is the untracked implementation log.
- Other Phase 1 agents ran at the same time (load average 30 to 41), so the cold-build times are upper bounds.

##### Generation baseline

- Command: `cargo run -p schematic-gen -- generate --api all --output <tmp>/schema-gen/src --openapi-out <tmp>/openapi --postman-out <tmp>/postman -v`. The generator writes the manifest one level above `--output`, so no tracked file was touched.
- It ran in 1.8 s and printed 27 expected `query-in-path` warnings (9 endpoints across 3 outputs).
- `diff -r` against `schematic/schema/src` and `Cargo.toml` is empty: the committed client equals a fresh generation.

##### Export baseline

- OpenAPI (15 files) and Postman (15 files) are byte-identical to the committed `schematic/openapi` and `schematic/postman`; both diffs are empty.
- `hashes.txt` holds the sha256 of all 73 generated files.

##### Request surface baseline

- 547 `*Request` structs, 547 `*Call<'a>` types and 547 `EndpointSpec` impls.
- 35 legacy `impl Paginated`: bitbucket 9, github 9, gitlab 9, gitea 8.
- 197 candidate pagination fields across 79 structs (in `adoptable-fields.txt`):
  - by name: limit 66, after 47, page 46, per_page 18, pagelen 9, before 9, page_token 1, page_size 1
  - by API: openai 125, bitbucket 18, github 18, gitlab 18, gitea 16, elevenlabs 2
- 19 near-miss names are listed separately for information, for example OpenAI `starting_after`.

##### Recipe results (warm cache, kache on)

| Recipe | Wall time | Result |
| --- | --- | --- |
| `just test` | 51.2 s | pass: L1 1937/1937 (12 skipped); fresh-output 6/6; compile-fail 1/1 |
| `just test-fresh-output` | 23.3 s | pass: 6/6 |
| `just test-compile-fail` | 1.9 s | pass: 1/1 |
| `just lint` | 76.5 s | pass |
| `just test-schema` | 0.9 s (up to date) | pass: 40/40 |
| `just lint-schema` | 0.4 s (up to date) | pass |

Cold build times for the generated crate, each on an empty target dir under `/tmp`:

| Build | Time |
| --- | --- |
| Full build, all targets, with kache | 44.7 s |
| Full build, all targets, without kache | 69.8 s |
| `schematic-schema` library alone | 35.5 s (32.8 s in the front end) |
| `schematic-definitions` | 60.8 s |
| Cold `clippy` (equivalent to `lint-schema`) | 60.6 s, pass |

##### Coverage of recipes

- `test` and `lint` both cover `schematic-runtime`, along with define, gen, definitions and oauth. Neither covers `schematic-schema`; only `test-schema` and `lint-schema` do.
- `test` also runs `test-fresh-output` and `test-compile-fail`.
- `lint` runs clippy on `schematic-define` without `--all-features`, so its `openapi`-gated code is linted only indirectly, through the gen and definitions runs.

##### Pre-existing failures

None.
