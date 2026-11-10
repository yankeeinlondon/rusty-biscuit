---
$schema: feature-review.yaml
ready: false
findings:
  - "R2-F1 (high): Cache reuse omits implicit and transitive request inputs"
  - "R2-F2 (high): Referenced schemas and validation sessions omit required function captures"
human_review: true
human_review_items:
  - "R2-F1: a cached schema that used a captured function is reused when re-calling that function with the new command's observations gives the same answer (rather than never sharing such results)"
  - "R2-F2: composition grows a copy of the request context for schema-slot captures instead of sharing the request epoch, so a document's ctx.* and a schema slot may observe separate captures in one command"
  - "R2-F2: has_agentic_cli keeps the Agent group's existing lazy scan of the process PATH (shared with document ctx.agent); only has_command/has_binary read the captured PATH"
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-08T14:35:55-08:00"
spec: "2026-11-05-dynamic-schema-types/spec.md"
implemented: true
description: "A **feature** review of `2026-11-05-dynamic-schema-types/spec.md`"
feature: "2026-11-05-dynamic-schema-types/review-2.md"
previous: "2026-11-05-dynamic-schema-types/review-1.md"
log: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/log.md"
implemented_by: "claude/opus"
next: "2026-11-05-dynamic-schema-types/review-3.md"
---

# Review 2: Dynamic Schema Types

The feature is **not production ready**. All five previous findings are verified fixed, including the additional instances listed in the repair ledger. Two new findings remain. Both have repairs within the existing specification; neither needs a human decision.

This review covers the current working tree, including the uncommitted repairs. The specification, its author decisions, the first review, the implementation log, and the repair log's `### Finding Ledger` were read before assessing new work. There is no `## Human rulings` section in the first review. Its accepted implementation choices are retained.

## Previous Finding Verification

The required instances below include every child in the repair ledger, not just the instances originally reported. Test references name modules in the Darkmatter package unless another package is identified. All cited slot-selected tests were rerun successfully.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Several slot paths perform reads before approval | R1-F1.1–R1-F1.20: seven inline slot positions; contract imports; schema-validation reads; three listing functions; five direct-reader controls; cache permission order; persisted listing grants; diagnostic sibling listing | **verified fixed** | `dynamic_schema_slot_read_order` exercises each inline position, imports, `validate_schema`, listings, direct readers, and folder-grant/preflight agreement through composition and its observation log. The cache test `an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread` passes. Inspection confirms `$schema` exclusion from interpolation, guarded schema reads, listing gates before enumeration, and grant coverage before cache input rechecks. The ledger's diagnostic-listing control is outside schema-slot error rendering. | None. R2-F1 concerns which inputs determine reuse, not the repaired authorization order. |
| R1-F2: Overridden trigger slots still evaluate | R1-F2.1–R1-F2.12: document/later-trigger overrides; unmatched/effective controls; shared preflight selection; validation/clean; reference checks; editor; cache identity; pending reporting; named definitions; templated root-union document | **verified fixed** | `dynamic_schema_slot_precedence` passes for discarded and surviving properties, named definitions, root-union every-arm/some-arm cases, preflight, validation, and cache separation. CLI compose/validate/clean tests and the editor's discarded-trigger test pass. `discarded_by_higher_layers` feeds both preflight and runtime; `without_properties` drops definitions reached only by discarded properties, and omitted names enter cache identity. Trigger payloads must be simple objects, so the library's deliberate refusal to prune a union payload does not leave an admitted sibling unhandled. | None. |
| R1-F3: Inline and trigger slot failures lose authored locations | R1-F3.1–R1-F3.28: compose/validate inline and trigger errors; referenced/imported controls; editor mapping; quoted/block carriers; approval/data/pattern/default errors; union arms; repeated slots; trigger-text changes; referenced payload; unproven override; cycle; YAML documents; unsaved ranges; clean; refusal subspans; contract-template causes | **verified fixed** | Every case in `dynamic_schema_slot_positions` passes, including the added contract-template cause cases. CLI inline/trigger/clean JSON checks and editor quoted/repeated/unsaved/trigger-related-location tests pass. Authored text travels through pending schemas and failures; the shared diagnostic projection proves spans against that text, preserves nested codes and data locations, and does not reopen files. The editor uses the shared span only for matching buffer text. | None. The accepted outer-slot primary location for a contract failure is retained. |
| R1-F4: Unsaved data buffers are ignored outside YAML and JSON | R1-F4.1–R1-F4.11: JSON5, JSONL, TOML, CSV, TSV, YAML, JSON, Markdown, unusual-extension import, buffer close, dirty-buffer automatic grant | **verified fixed** | All `schema_slots_editor::unsaved_buffers` cases pass, including the format-list guard, close-to-disk transition, and dirty-buffer refusal. The library buffer-close rehash test passes. `set_schema_buffer` retains text independently of extension; a cache input served from a buffer has no disk metadata shortcut. | None. Previously accepted limits for never-saved data files and slot-free Markdown imports are not reopened. |
| R1-F5: Stale completion candidates have no stale marker | R1-F5.1–R1-F5.10: scalar completion, hover, diagnostics, array item, union value/item, suggestions, pending literals, body hover, value-inserting quick fix | **verified fixed** | The held-refresh protocol test and every `stale_responses` case pass. Ordinary frontmatter candidates pass through `with_slot_notes`, preserving labels, edits, ordering, and existing detail; hover and affected quick fixes use the same status. Pending and stale states remain distinct, and the note disappears after refresh. | None. |

## Unblocked Findings

### R2-F1 (high): Cache reuse omits implicit and transitive request inputs

**Authority: spec.** “Resolution must additionally record the exact bytes or observations used,” including “captured context values.” The function-class decision also requires remaining volatile slots to resolve “once per command snapshot and never reuse them across requests.” A long-running caller can instead receive yesterday's date-dependent schema, an earlier network/tool/package snapshot, or a contract derived from an old Git remote. These are ordinary supported expressions and request changes.

**Defect class:** cache eligibility and dependency checks are derived from the initial expression text instead of all request inputs and slot evaluations that produced the result.

In the darkmatter package, [cache publication in `SlotSession`](../../lib/src/markdown/schemas/slots/session.rs:479), which shares resolved schemas between commands, classifies only `pending.slots()` and builds its context checks from that same initial list. [The context-input scanner](../../lib/src/markdown/schemas/slots/evaluator.rs:286) recognizes written `env` and `ctx` variables, but not the captured clock, addresses, package lookup, or installed-tool information consumed by function calls. The request identity includes the environment and resolution roots, but not these observations. Computing a fresh function result would work; a cache hit prevents that computation.

Two further resolution paths lose the same information. [Contract-template resolution](../../lib/src/markdown/schemas/slots/session.rs:710) evaluates another set of slots without returning their context dependencies or volatility to the outer cache entry. The simplified-schema package's [root-union resolution loop](../../../simplified-schema/lib/src/slots/pending.rs:302) can discover slots in later rounds; Darkmatter still classifies only the initial pending list when publishing the completed result. File observations alone cannot describe these inputs.

**Reproduction method.** Start from the existing session-cache fixture: a temporary committed repository with a schema and OS data. Resolve through the public `DarkmatterSchemas::effective_for` API using a fresh session for each request. Change exactly one captured observation, executable, or Git remote; keep schema text, environment, and resolution roots unchanged. Clock/address/package/tool fixtures use the existing captured-observation test constructors, avoiding sleeps and dependence on the review host. Except for the direct-clock case, each failing probe also clears the shared cache and proves that the same second request then gives the expected result. The direct-clock control proves its initial date-dependent member before changing the captured day.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Clock-function dispatch, shared by the 14 date functions | `enum({{ is_today('2024-06-16') ? ['macOS'] : ['Windows'] }}; required)`; captured day changes from June 16 to June 17 | Second request still exports `macOS` | Export `Windows` from the new captured day; fingerprint or confine the shared clock input for every date function |
| R2-F1.2 | `has_command` | Probe an executable by absolute path, then remove it before a new session | Second schema still says installed; cold control says missing | New session observes removal |
| R2-F1.3 | `has_binary`, which shares the command-probe handler | Same executable-removal reproduction | Same stale installed result | Same fresh result |
| R2-F1.4 | `ipv4` | Captured addresses change from `10.1.1.1` to `10.1.1.2` | Old enum member | New captured member |
| R2-F1.5 | `ipv6` | Captured addresses change from `2001:db8::1` to `2001:db8::2` | Old enum member | New captured member |
| R2-F1.6 | `package` | Same path; captured containing package changes from `alpha` to `beta` | `alpha` | `beta` |
| R2-F1.7 | `package_area` | Same path; captured containing area changes from `schemas` to `schemas/child` | `schemas` | `schemas/child` |
| R2-F1.8 | `has_agentic_cli` | Captured Kimi installation changes from present to absent; slot supplies a boolean literal | Literal remains `true` | Literal becomes `false` |
| R2-F1.9 | Nested contract's captured input | Outer enum loads OS data under `Os@./types.yaml`; `Os` is `string(pattern({{ is_today('2024-06-16') ? '.*' : '^Windows$' }}))[]` | New day's request succeeds with the old members; cold control rejects the data | Reapply the new day's contract and fail |
| R2-F1.10 | Nested contract's volatile classification | Same outer data load; contract pattern depends on `remote_vendor() == 'github'`; change origin from GitHub to GitLab | Warm request succeeds under the old contract; cold control rejects | Evaluate the volatile dependency in each new session |
| R2-F1.11 | Later root-union resolution round | First arm has a pure slot; second arm has `enum({{ [remote_vendor()] }}; required)`; change GitHub origin to GitLab | Completed union retains `github`; cold control exports `gitlab` | Classify every evaluated arm before sharing its result |
| R2-F1.12 | Direct volatile slot, clean control | Existing `a_volatile_slot_resolves_once_per_command` | One evaluation per command, no cross-command reuse | Correct; retain this behavior when volatility is transitive or found later |
| R2-F1.13 | Explicit environment inputs, clean control | Existing `a_changed_environment_input_is_a_miss` changes `env.SANDBOX_OS` | Cache miss and new members | Correct; retain |
| R2-F1.14 | File/content dependencies and read-set projection, clean controls | Existing source/data/import/missing-file/failure/mutation/cache-hit tests | Inputs invalidate as tested; a hit replays reads without evaluating | Correct; retain |
| R2-F1.15 | Permission acceptance of a cache hit, clean control | Existing narrower-grant and refused-before-rehash cases | Old grants are not borrowed; refusal precedes content recheck | Correct; retain R1-F1's repair |

The command-probe reproductions use temporary executable symlinks on macOS; they do not run the executable. Both names share `command_exists`, whose result currently records no executable observation. Environment changes themselves are already part of request identity; the missing case is changed availability with the same captured PATH or absolute argument. Aliases of the other functions reach the same catalog bindings, and all date functions reach the same clock dispatcher, so these are shared decision sites rather than separate parser implementations.

Follow the missing values through both decisions: they affect the concrete schema published into the shared cache, and the later session's decision to reuse that schema. Carry the full dependency/class summary from nested contracts and every union round, including failures. Merely fixing the scanner for function names leaves the nested and later-round instances open.

**Recommended repair:** keep resolutions with untracked captured or volatile inputs local to their command until their actual inputs can be compared correctly. Record cheap captured-value digests where available, using the values evaluation actually consumed; propagate dependency and volatility information across the existing resolution stack and union rounds. This preserves the static/data-file sharing fast path without inventing a TTL or another cache. Add public-result regression tests for the rows above, with changed-input and cold controls. This does not require changing the author-approved size/mtime shortcut for ordinary files.

### R2-F2 (high): Referenced schemas and validation sessions omit required function captures

**Authority: spec.** The schema-context decision allows captured `ctx` and environment inputs in the schema's derived request context, and the function-class investigation explicitly retains `package`, `package_area`, `ipv4`, `ipv6`, and `has_agentic_cli` as supported captured-context functions. The minimum guarantee requires keeping this evaluation boundary intact. Moving a valid inline schema to a referenced file should not turn a supported function into an unavailable context error. It currently does, and the editor worker also fails for the inline form.

**Defect class:** context requirements are discovered from document text, but the captures needed by separately discovered schema slots are never supplied to their resolution session.

In the darkmatter package, [`SlotSession::for_request`](../../lib/src/markdown/schemas/slots/session.rs:209), which builds a composition's session, copies the document request's observations. That suffices when the function call is visible in inline frontmatter. It does not add requirements from referenced schemas, imported definitions, or trigger payloads. [`SlotSession::for_context`](../../lib/src/markdown/schemas/slots/session.rs:229), used by validation and cleaning, supplies no structured observations. In the darkmatter-lsp package, [the editor's `run_job`](../../dmls/src/slot_jobs.rs:426) uses this latter constructor as well. The function handlers consequently return `FunctionContextNotCaptured`, projected as `dm.schema.slot_failed`.

**Reproduction.** Use the normal public composition fixture with `ComposeOptions::new()`, a temporary repository, and an empty home. The inline positive control is:

```markdown
---
$schema:
  value: number(default({{ length(ipv4()) }}))
---
Body.
```

It composes. Move that declaration unchanged into `schema.yaml` under `kind: schema`, and change the document to `$schema: ./schema.yaml`: composition fails because the Network capture is missing. Using `length` and a default makes the result valid even on a host with no matching addresses. The other rows similarly avoid assuming an installed tool or a populated package list.

For each of the five functions below, I ran the inline positive control, referenced-schema composition, imported-type composition, selected-trigger composition, and public effective-schema preparation with `SlotSession::for_context`. A separate test ran the actual DMLS `run_job` worker for all five functions plus a date-function control. Follow-up probes exercised the same five functions through a reference-check session and a nested data-contract slot, retaining the date control. No editor window or network request was needed.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Referenced `$schema` during composition | Each of `length(ipv4())`, `length(ipv6())`, `has_agentic_cli('kimi')`, `package('./child')`, and `package_area('./child')` supplies a correctly typed default | All five fail: missing Network, Agent, or Repo capture | Same successful resolution as the inline control, in the schema's context |
| R2-F2.2 | Imported named definition during composition | Put each unchanged declaration in `types.Value`; document uses `Value@./types.yaml` | All five fail with the corresponding missing capture | Supply the imported slot's required captures |
| R2-F2.3 | Selected trigger payload during composition | Put each declaration under a matching trigger's `$schema`; document contains only its matching topic | All five fail with the corresponding missing capture | Supply the selected surviving payload's captures |
| R2-F2.4 | Validation/clean session constructor | Each inline control prepared through public `DarkmatterSchemas::effective_for` with `SlotSession::for_context` | All five fail with the corresponding missing capture | Capture what the slots require before preparing their schema; validate and clean share this constructor |
| R2-F2.5 | Actual editor resolution worker | Run the same five inline defaults through DMLS `run_job` | All five produce missing-capture errors | Worker resolves supported slots off the request path |
| R2-F2.6 | Inline composition, clean control | All five functions visible in the document's own inline schema | All five compose successfully | Correct; preserve |
| R2-F2.7 | Clock functions, clean control | Boolean default from `is_today('2024-06-16')`, through all preceding composition/preparation forms and the editor worker | Resolves successfully | Correct: the clock has a lazy captured fallback; do not confuse this with R2-F1's stale reuse |
| R2-F2.8 | Command functions, clean contract | `has_command`/`has_binary` take PATH directly from the supplied file-resolution context | No structured Network/Agent/Repo capture is needed by this shared handler | Preserve the captured-PATH contract; changed executable availability is R2-F1 |
| R2-F2.9 | Reference-check resolution session | The same five defaults prepared with `SlotSession::for_context` in `ReferenceCheck` mode | All five fail with missing captures, rather than a read-approval deferral | Supply the required captures under the reference check's existing permission limits |
| R2-F2.10 | Nested data-contract slots | Outer enum loads an empty array under an imported object-array contract containing each default; a directory grant allows data and contract reads | All five inner functions fail with missing captures | Discover and supply requirements when the contract's slots are reached |

The five affected handlers fall into three shared capture providers: Network (`ipv4`, `ipv6`), Agent (`has_agentic_cli`), and Repo (`package`, `package_area`). Their getters explicitly require a captured observation; an empty observed collection is valid and is different from an absent capture. The fix must supply those observations, not silently replace the error with an empty result. Nested contract slots also use the session evaluator and must inherit this repair when their requirements first become known.

**Recommended repair:** collect the admitted, surviving slots' context requirements when preparing them, including separately loaded and imported slots, and supply the required structured observations through the schema resolution session. Reuse the request's captured environment and repository authority and derive paths from each defining schema; do not recapture process CWD or let a document's unrelated expression determine whether a schema function works. Apply the same construction to compose, validate/clean, reference-check resolution, and the editor worker. Add the function/provider matrix above at the public result boundary, retaining inline and clock controls.

## Blocked Findings

None. The recommendations preserve existing language, permission, and cache decisions. There is no pending author choice.

## Recurrence

Neither new finding repeats a defect class from the first review. R1-F1 concerned reads occurring before authorization; its permission-order cases now pass. R2-F1 concerns stale reuse after request inputs change, and R2-F2 concerns absent context captures. Neither is a partially repaired instance or a regression attributed to the earlier repair. No previous finding is carried forward.

## Verification and test rigor

| Requirement group | Strongest reviewed verification | Assessment |
| --- | --- | --- |
| Passive grammar, template inspection/export, typed substitution, supported positions, static parity | L1 simplified-schema tests, including its minimal-feature build | Appropriate; no terminal behavior is involved |
| Real sandboxing schema, passive corpus admission, resolved document validation | L1 shipped-schema/template checks and public composition fixtures; schema uses `load_data("../../../os.yaml", "string[]")` | Appropriate; carve-out removed |
| Approval order and scopes, imports, listing authorization, cache permissions, effective-layer precedence | L1 observation-log tests, scripted approval handlers, public composition and CLI JSON tests | Previous findings verified fixed |
| Authored diagnostic ranges and typed causes | L1 public library results, CLI JSON, and actual DMLS protocol responses | Previous finding verified fixed |
| Unsaved data formats, closing buffers, dirty-buffer grants, superseded workers | L1 actual language-server protocol tests and cache tests | Previous finding verified fixed |
| Stale/pending completion, hover, diagnostics, and affected quick fixes | L1 protocol tests with a deterministically held worker | Appropriate; these are protocol properties, not terminal key encoding or editor painting promises |
| Captured inputs and transitive cache classification | L1 public effective-schema probes with changed captured observations, executable availability, and Git remote; cold controls | R2-F1: eleven failing reproductions, with existing clean controls |
| Captured-function support across schema sources and callers | L1 public composition/preparation matrix and actual editor worker | R2-F2: five functions fail across six library paths and the worker; inline and date controls pass |

Commands run from their package areas:

- `just test slot` in Darkmatter: **344 passed**.
- `just test slot` in simplified-schema: **87 passed**, followed by the recipe's **1,437 passing minimal-feature tests**.
- `just check-tier-coverage darkmatter` from the repository root: **no stranded tests**.
- Temporary nextest probes through `just test review2_…`: eight changed-capture/nested-cache failures, one later-union-arm failure, and two executable-removal failures. Separate capture tests confirmed all five affected functions across referenced/imported/trigger/context-only/reference-check/nested-contract paths, plus the actual worker; inline positive controls and the date control passed.

The temporary probes used existing declared L1 targets and ordinary names. Each final probe test finished below one second; no sleeps, retries, focused windows, or external services were used. Intermediate fixture mistakes were corrected before drawing conclusions, including using canonical temporary paths and valid YAML carriers. Final probe additions were removed, restoring all three touched source/test files byte-for-byte to their pre-review contents. The persistent edits from this review are this file and the requested review/spec metadata. No implementation repair, commit, formatting command, full-suite lint, or cross-OS run was performed. Cross-OS proof is left to CI and does not determine readiness here.

## Input robustness matrix

The feature's new file/configuration loader remains the fixed-home, line-based schema read-grant store. Expression syntax is a text grammar, and existing `load_data` format parsers are outside this feature's new-loader boundary. The repairs do not add another configuration loader.

**Default outcome:** an invalid grant record rejects the entire store, reports its path and line, and grants nothing; a whole-file read/encoding failure reports the path without inventing a line. **Exceptions defined by the format:** a missing or empty store supplies no rules; comments, blank lines, CRLF, and a UTF-8 byte-order mark are allowed; identical records deduplicate; any matching denial overrides allowances.

I inspected `grants/store.rs` and reran its public loading and matching tests within the slot-selected suite. These cover valid actions/scopes, absent and empty arguments, unknown actions/scopes, invalid paths and URLs, site/query restrictions, invalid UTF-8 and unreadable files, duplicate records, and write/read round trips. The line format has no typed null or collection-element representation. The requested scan found no `serde(default)`, `filter_map`, `unwrap_or_default`, or `.ok()` fallback in this loader. CLI invalid-store wiring tests also passed. No new loader-shape finding was found.
