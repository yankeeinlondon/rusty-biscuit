# S5 — Safety and approval: design review

Spec: `2026-11-05-dynamic-schema-types` (Rulings Q5–Q7 in force). Read-only
investigation of the code at `feat/better-research`; no builds were run.
Paths are relative to the repository root.

## 0. Findings that shape everything else

These are facts in the current code that the design has to accommodate. The
first four are worth an author ruling (see §8).

| # | Finding | Evidence |
|---|---|---|
| F1 | The shell whitelist/blacklist are **not** user-level. `md compose` keeps them beside the document (or the launch dir for stdin), and a library caller with no policy root gets the **repository root** first, then home. "Beside the shell whitelist" therefore means "in repository content", which contradicts Q5's "repository content … can never grant access". | `cli/src/commands/compose.rs:421-431`; `lib/src/markdown/compose/shell_expansion/store.rs:14-66`; `docs/inline/preflight-checks.md:221-237` |
| F2 | DMLS config (`.dmls.toml`) lives at the workspace root, and its `[shell] policy_path` is workspace-controlled. Nothing workspace-side may locate the slot policy file. | `dmls/src/config/mod.rs:4,38`; `docs/lsp/configuration.md:485-497` |
| F3 | `biscuit_file::FetchPolicy` is host-only (`HostPattern::Exact`/`Wildcard`); it cannot express scheme or port, nor exact-URL / URL-without-query grants. `RemoteReadConfig::is_host_allowed` is also host-only, and `--allow host=` today grants a host for every scheme and port and for document reads too. | `biscuit-file/lib/src/file_reference/fetch.rs:13-70`; `darkmatter/lib/src/markdown/compose/remote.rs:163-180` |
| F4 | Many catalog "context" functions are not reads: network (`ping`, `ping_under`, `pr`, `pr_list`, `cicd`, `cicd_list`, `branch_exists_on_remote`, `remote_vendor`), composition (`as_markdown`, which composes a document and can reach transclusion/shell), and directory listings (`find_files`). Q5 only covers file/URL reads, "pure", and "time- or state-dependent" functions. M5's "nothing else" requires the network and composition functions to be rejected (or given their own grant). | `lib/src/markdown/compose/expression/functions/*.rs` (`FunctionBinding::context` list); `functions/composition.rs:13`; `functions/network.rs:94-139`; `functions/git.rs:97-124` |
| F5 | The request's **captured** repository observation holds only root + topology (`RepositoryObservation { root, info: RepoInfo }`); no Git status. File status is captured only on demand when `ctx.file_changes` is read, through `GitInfo::file_changes()`. The Sniff skill says not to use that for path classification (it carries lossy display paths and line stats). | `lib/src/markdown/compose/context/repository_scope.rs:20-48`; `context/capture/observations.rs:27-45`; `context/capture/snapshot.rs:64-80,319-333,943-967` |
| F6 | `DataRuntime` invalidation has no path. A shell command or a lifecycle effect empties the whole memo ("may have written any file"). Q7's "a write … recorded against a path" exists only for the lifecycle verbs that take a path. | `context/options.rs:2441-2445`; `shell_expansion/mod.rs:713-718`; `lib/src/lifecycle/md_host.rs:469-473,606-610`; `lib/src/lifecycle/executor.rs:989-1011` |
| F7 | Shell's data-output "approval required" has no typed code of its own. It is `dm.shell.failure`, with the suggested rules written into the prose message. "Typed `approval-required` … as shell does today" therefore needs a new code for slots. | `lib/src/markdown/compose/report_diagnostics.rs:662-672`; `cli/src/commands/compose.rs:1235-1258` |
| F8 | DMLS schema-dependent requests **wait** on the registry worker's document job. Slot resolution must not go in that lane, or a request would block on evaluation. | `dmls/src/registry_jobs.rs:1-20` |
| F9 | The `load_data` memo is shared by the whole command and is checked after host policy but before the file read. A slot gate has to run **before** `ctx.data.load_source`; otherwise a document's own ungated read of the same file lets the slot read it from the memo. | `lib/src/markdown/compose/expression/data/load.rs:60-112` |

## 1. Passive target collection

### Where the template comes from

The pre-flight walk already prepares each walked document's schemas. The
inline root pipeline runs schema validation (`preflight/collect.rs:503-541`).
`DiscoverySiteContext::effective` calls `schema_validation::prepare_schemas`
and then `effective_for` to type shell holes (`collect.rs:1524-1535`).
`effective_for` resolves the document `$schema` through
`resolve::resolve_schema_with_roots` and matches triggers
(`schemas/mod.rs:688-760`). Every named definition keeps its defining site
(`simplified-schema/lib/src/simplified/definitions.rs:103-113`,
`Definition::origin: SchemaReferenceSite`).

**Recommendation.** Collect slot targets from the passively resolved
**template**, never by scanning frontmatter text. Once S7 lands, resolution
returns an `EffectiveTemplate`. It lists every slot as
`(defining file, span, Expr, slot kind)` across:

- the document `$schema` layer. An inline `$schema`'s defining file is the
  document itself (Q3);
- every definition reached through `Name@file` / `Name@this` imports and
  aliases, using `Definition::origin`. The "same repository as the defining
  file" test uses this origin, not the importer;
- **selected** trigger payloads only. Matching stays passive (Q6), and a
  payload that no document selects contributes nothing. When the walk cannot
  decide a match because a matched-on key is still shell-pending, include that
  payload. This follows the walk's condition-blind superset rule (the same
  rule as both ternary branches for shell). Never include unmatched payloads;
- baseline and kind definitions never carry slots (Q4). They are rejected at
  parse time, so the collector does not visit them.

Add a deferral flag, `defer_unresolved_schema_slots`, next to
`defer_shell_pending_schema_problems` on the walk's inline options
(`collect.rs:526-534`). With it set, the walk's schema stage records slot
targets instead of failing on an unresolved template. Validation of values
against pending parts moves to the terminal pass.

### Determinable argument forms

Reuse the pattern of `remote::collect_expression_urls` (`remote.rs:416-442`:
recursive walk with `expr_children`, first argument is a `StringLiteral`) and
`calls_read_side_function` (`remote.rs:452-460`, alias-folded names). Follow
the shell pre-flight rule in `docs/topics/darkmatter-expressions.md:1779-1786`:
**no evaluation**, enumerate both ternary branches and both sides of `||`, and
exclude pure calls by construction.

| Form | Determinable? | Target |
|---|---|---|
| `load_data("lit", …)`, `file_exists("lit")`, `frontmatter`/`try_frontmatter`/`markdown_title`/`markdown_body_empty`/`validate_schema("lit")` | yes | the literal, resolved as a `FileReference` in the defining file's context; URL when http(s) |
| A subexpression made only of literals and **pure** catalog functions (`"a/" + "b.yaml"`, `lower("OS.yaml")`) | yes, by constant folding | fold with a lookup that fails on any variable; only `FunctionBinding::pure` functions may run |
| `ctx.*`, `env.*`, callback locals, or the result of another read (`map(load_data(…), x => load_data(x.path))`) | no | computed: checked at evaluation (below) |
| `find_files("glob")` | its static base only | treat it as a directory grant on the glob's literal base, with **no** automatic grant (a listing exposes untracked names); see Q-B |
| `fm.*` and lifecycle state | rejected passively (spec) | — |
| network/compose functions (F4) | rejected passively | positioned schema error (see Q-B) |

A literal local reference still needs **candidate probing** (bare and `%`
forms search several roots, `fs.rs:96-115`). The walk resolves it with the
same `resolve_document_file_ref` (stat only, as the shell walk's `PATH`
probe does) and checks the grant against `canonicalize_simplified` of the
match. A missing literal target keeps its first candidate (the
`DataSourceIdentity::local` spelling, `data/runtime.rs:48-82`). It gets no
automatic grant because it is not tracked, so it is asked about like any
other target.

### Computed targets

Put one gate in `ResolutionContext`: `slot_gate: Option<Arc<SlotReadGate>>`.
It is present only while a slot resolves, so document evaluation keeps
today's rules (Q5 "excluded and unchanged"). Each read-capable function calls
`gate.check(&DataSourceIdentity)` immediately after resolving its identity and
before any memo lookup or I/O. For `load_data` that is between
`load.rs:69` and `load.rs:87`. `resolve_arg` (`fs.rs:96`) is the shared
resolution chokepoint for the other file functions. The gate checks: persisted
deny, then run grants (once grants, `--allow host=`), then persisted allows,
then the automatic clean-file grant. Then:

- **interactive run:** call the slot approval handler in-stage. This needs
  single-flight per target key, as shell's `reserve_allow_once` does with
  `pending_allow_once` + condvar (`shell_expansion/types.rs:1281-1330`),
  because transclusion is concurrent;
- **non-interactive run, data output, DMLS, library without a handler:** fail
  with `SlotReadError::ApprovalRequired` (typed, positioned at the slot).

The CLI currently leaves the in-stage shell handler `None` on purpose
(`compose.rs:407-433`). The slot handler has to be passed through for
computed targets.

### Ordering with shell hole typing

Shell template holes are typed from the effective schema
(`collect.rs:1548-1557`, `DeclaredTypes::for_schema(effective)`). An enum
filled from a slot changes a hole's approval type. **Recommendation:** during
the walk, resolve every slot that needs no prompt (pure, automatic, persisted,
or `--allow`). Those reads are allowed, and the walk already reads files.
That leaves only prompt-requiring slots pending. A shell hole typed by a still-pending enum
gets the conservative choice set (exact bytes only, no template shape)
instead of a widened `string` type.

## 2. Approval plumbing

### Keep the rule sets separate and share the phase

`compose_preflight_approvals_with` (`preflight/lifecycle.rs:131-160`) runs
the walk and then `approve_collected` (`:164-230`), which reads the shell
policy files and calls the handler once per undecided site. Add a sibling
step, `approve_slot_reads(report.slot_targets, slot_policy, slot_handler)`,
in the same function after the warnings check (`:150`) and **before** the shell
decisions, so slot snapshots can type holes. Return its result in
`ComposePreflightApprovals` (`:72-77`) as `pinned_slots: SlotPreApprovals`,
and hand it back with `ComposeOptions::with_pre_approved_slot_reads(…)`, the
same way `with_pre_approved_commands`/`with_preflight_graph` are used. Use a
separate handler trait. The CLI implements both traits on one prompt object,
so the batch is one terminal session.

```rust
pub trait SlotReadApprovalHandler: Send + Sync {
    fn approve(&self, request: SlotReadApprovalRequest) -> Result<SlotReadDecision, SlotReadError>;
}

pub struct SlotReadApprovalRequest {
    pub target: SlotTarget,                 // File(canonical PathBuf) | Url(url::Url)
    pub slot: SlotSite,                     // defining file, authored line/col, expression text
    pub why_not_automatic: AutomaticRefusal,// Dirty | Untracked | Ignored | OtherRepository | NoRepository | Remote | OutsideDefiningRepository
    pub offered: OfferedSlotGrants,         // which SlotGrantScope values are valid here
    pub policy_path: PathBuf,
}

pub enum SlotGrantScope {
    File,                 // canonical path
    Directory,            // canonical parent, recursive (component prefix)
    UrlExact,             // normalized URL incl. query, fragment dropped
    UrlIgnoringQuery,     // scheme+host+port+path; offered only when url.query().is_some()
    Site,                 // url.origin(): scheme+host+port (default port made explicit), no subdomains
}

pub enum SlotReadDecision {
    AllowOnce(SlotGrantScope),
    AllowPersist(SlotGrantScope),
    DenyPersist(SlotGrantScope),   // Q5: deny is persisted
    Deny,                          // this run only: the answer to EOF/cancel (see Q-G)
}
```

`OfferedSlotGrants` plays the role of shell's `OfferedChoices`: a handler may
not return a scope that was not offered (`types.rs:563-566`).

### User-level policy file

- **Location:** `<home>/.config/darkmatter/schema-read-policy`, with `home`
  taken from the request (`FileResolutionContext::home_dir()`, populated
  only by `RequestSnapshot::from_process`, `context/request.rs:78-82`).
  Nothing in the document, repository, `.dmls.toml`, or editor
  initialization options may move it (F1, F2). A library option
  `ComposeOptions::with_slot_read_policy(SlotReadPolicy)` lets embedders supply
  an in-memory policy or another path. This is **not** "beside the shell
  whitelist" (Q-A).
- **Format:** line-based UTF-8 like the shell files, with the same loader
  behavior: malformed lines are skipped and reported with a `PolicyLineWarning`,
  and an unreadable file is a typed `PolicyIo` error, never an empty policy
  (`store.rs:69-181`). Paths are canonical portable absolute
  (`biscuit_file::to_portable_string`, as in `DataSourceIdentity::read_set_entry`).

```text
# darkmatter schema-slot read policy
allow file   /Users/me/code/repo/claudine/docs/os.yaml
allow dir    /Users/me/data/
allow url    https://example.com/list.json?v=2
allow url-path https://example.com/list.json
allow site   https://example.com:443
deny  site   http://tracker.example:80
```

- **Precedence:** any matching `deny` wins over any allow and over the
  automatic grant (mirrors shell: blacklist before whitelist,
  `lifecycle.rs:195-198`). A grant covers only schema-slot reads.
- **Writes:** append one line, as `append_rule` does (`store.rs:317-334`).
  The CLI and DMLS share this file. Append is preferable to an atomic rename,
  which would let a concurrent writer's line be lost. Dedup on load.

### Remote grants and `FetchPolicy`

A persisted site grant must not widen document reads, which keep
`--allow host=` only. So it may not be added to the request's shared
`RemoteReadConfig`. Instead the gate decides first. For an allowed URL, slot
evaluation fetches through the shared remote runtime under a **slot-scoped**
policy: the request's `--allow host=` hosts, plus the site, url-path, and
exact grants for that URL. The single-flight/transport cache is fine because
authorization precedes the fetch (`load.rs:84-93`). `FetchPolicy` needs one
addition, `HostPattern::Origin { scheme, host, port }` (or a darkmatter-side
origin check before the host check), so that a site grant does not cover
another scheme or port (F3). `--allow host=HOST` stays a one-run grant at
**host** granularity, as today (Q-F).

### CLI prompt and data-output failure

- **Prompt:** add `CliSlotApprovalHandler` beside `CliShellApprovalHandler`
  (`cli/src/approval.rs:13-300`). Reuse `approve_with_io` and
  `can_prompt_interactively` (`:162,300`), and the
  "number naming no listed choice is refused" and EOF-denies behavior. It shows
  the slot's defining file and line, the target, and why the automatic grant
  did not apply. The handler is built only when `sink.is_none()` and stdin is
  interactive (`compose.rs:438-445`).
- **Data output:** `SlotReadError::ApprovalRequired` maps in
  `report_diagnostics::error_diagnostics` to a new code
  `dm.schema.slot_approval_required` (and `dm.schema.slot_read_denied` for a
  deny). The source is the slot in its defining file (M3 position rule). The
  message lists the exact policy lines that would grant it, as
  `approval_required_report` does for shell (`compose.rs:1239-1258`). Do not
  add a field to `Diagnostic`, because one type serializes into all 17 output
  schemas. Put the grant lines in `reason`/message text, or a structured field
  only if the author wants every schema file touched.

## 3. Clean-file automatic grant

### What "clean" must mean and how to judge it

Judge clean **on the bytes actually read**, not on a status bit observed
earlier. That removes the gap between check and read:

1. `canonicalize_simplified(target)` must lie inside the canonical root of the
   defining file's repository (component prefix). This handles a symlink that
   leaves the repository.
2. Look the repository-relative Git-spelled path up in the **index**. It needs
   a regular-file entry: not gitlink/submodule, not conflicted. A file in a
   nested repository or submodule has no entry, so it gets no grant. That
   gives "other repository" for free.
3. Look the same path up in the `HEAD` tree. HEAD oid must equal index oid
   (nothing staged).
4. Hash the worktree bytes **while `load_data` reads them**, as a Git blob in
   the repository's object hash (after Git's worktree→ODB filter conversion)
   and compare with that oid. Store the oid on `LoadedSource`, so a memo hit
   can still be verified. This is the same "hash while reading" field S2
   needs for fingerprints (spec "Cache correctness").

Ignored and untracked targets fail step 2. Dirty targets fail step 3 or 4.

### Reading from the captured observation

The request holds one repository observation (F5), established when the
request is created (`ComposeRequest::prepare`, `context/request.rs:393-410`;
`ComposeOptions::request_repository`, `context/options.rs:905-911`). It has no
Git index/HEAD data. **Recommendation:** add a
`RepositoryCleanliness` part to that observation (index entries for the
needed paths plus the HEAD tree id). Fill it at most once per request per
repository root in a `OnceLock`, the way `CapturedObservations::agentic_clis`
lazily fills one shared scan (`observations.rs:31-36,96-100`). After that,
every pass (reference check, walk, terminal compose, lifecycle) uses the same
observation and never recaptures. Embedders (Claudine) supply it through
`ContextCaptureEvidence` (a new `with_repository_cleanliness`), so their
launch evidence stays authoritative, as `CurrentProvider` requires
(`context/current.rs:76-87`).

**Defining file outside the request's repository** (a `~/schemas` file, a
schema from a sibling repo): the request has no observation of that
repository. The strict reading is "no automatic grant; ask". See Q-D.

### What Sniff has and lacks

- Has: `GitRepo::discover`, `changed_paths(ChangedBase::Omitted)` (one status
  walk, ignored excluded, native paths, rejects conflicts), `working_state()`
  (status walk plus a hash of every changed path), and guarded index reads
  (`open::guard_index_read`/`read_index`). Source:
  `sniff/lib/src/filesystem/git/working_state.rs:1-80`, and the Sniff skill
  "Focused modules".
- Lacks: a **per-path** cleanliness query. `changed_paths` cannot tell
  tracked-clean from ignored (both are absent from status), and its cost is a
  walk of the whole worktree including untracked files. That is O(repo) for a
  question about one or two files. **Add**
  `GitRepo::path_cleanliness(&[PathBuf]) -> Vec<PathCleanliness>` and
  `GitRepo::verify_blob(rel, &bytes) -> bool`. They do one guarded index read
  plus a HEAD-tree lookup per path, apply the filter pipeline for
  `verify_blob`, add a counter such as `git.path_cleanliness.paths_checked`,
  and walk no files.
- **Cost:** one index parse per repository per request (it scales with
  tracked-file count; this monorepo is large, and S1 should measure it), plus
  O(depth) tree lookups per target, plus hashing bytes that `load_data` reads
  anyway. Compare a full status walk, which is likely hundreds of ms here.
  Add Darkmatter work counters (`note_work!`) such as
  `RepositoryCleanlinessObservations` (assert ≤ 1 per repository per request),
  `SlotTargetsCollected`, and `SlotGrantChecks`.
- **OS risk:** under `core.autocrlf`/`eol`/LFS, worktree bytes ≠ blob bytes.
  Without filter conversion every clean file on Windows or under LFS reads as
  dirty and prompts. Hashing must use gix's filter pipeline. Record the trap
  in the `os` skill when it is built.

### DMLS and unsaved buffers

DMLS keeps open YAML/JSON texts in `SourceBuffers`
(`simplified-schema/lib/src/resolve/buffers.rs:1-80`; used from
`dmls/src/overlay/mod.rs:877,901,1338-1346`). For a slot dependency with an
open buffer, the grant requires buffer bytes = disk bytes = index = HEAD.
Otherwise it is dirty, so the slot is pending with a grant code action. A data file
type the client never sends to DMLS (TOML/CSV outside the document selector)
is read from disk, as `md compose` would read it. DMLS also has to notice Git
state changes that flip grants (`git add`, commit, checkout). It watches only
`.md`/`.yaml` today (spec S3). Add watches for `<gitdir>/index` and
`<gitdir>/HEAD`, and re-judge on change.

## 4. Surfaces

| Surface | Code path that changes | Unapproved slot |
|---|---|---|
| `md compose` | `cli/src/commands/compose.rs:531-560` → `compose_preflight_approvals_with` gains slot approvals; `with_pre_approved_slot_reads`; terminal `schema_validation::validate…` uses pinned snapshots | formatted: prompt in the batch (literal) or in-stage (computed); data output: `dm.schema.slot_approval_required`, exit 1 |
| `md schema validate <document>` | `cli/src/commands/schema/validate.rs:503-580` (`validate_one`), which calls `DarkmatterSchemas` directly with no walk. It needs a mini walk: effective template → collect targets → same `SlotReadPolicy`/handler → resolve → validate. Same for `validate_yaml` (`:602`) | outcome "unavailable check", exit 1, never `Validated` |
| `md schema validate <schema>` | standalone classifier stays passive (template admission only) | n/a (no resolution) |
| `md clean` | `cli/src/commands/clean.rs:184,330-371` → `clean_document` with `CleanSchemaConfig`; pass the policy and handler through the config | constraint reported unavailable; never "clean" |
| Pre-flight walk | `preflight/collect.rs:503-560` (deferral flag, target collection), `DiscoverySiteContext::effective` (`:1524`), `preflight/lifecycle.rs:131-230` | collected and batched; the snapshot is pinned for the command |
| Reference checks | `ComposeOptions::for_reference_check` (`context/options.rs:1129-1135`) gains `defer_unapproved_schema_slots`; `md compose` runs it **before** the walk (`compose.rs:361-400`), so it resolves with automatic and persisted grants only and the walk reuses those snapshots; `md graph`/`md validate refs` (`cli/src/commands/graph.rs`, `validate.rs`) do the same | deferred, never prompts |
| DMLS | a new slot-resolution lane, **not** `RegistryJob::Document` (F8); diagnostics provider reads the published snapshot; `providers/code_actions.rs:56` adds grant actions | positioned `dm.schema.slot_approval_required` on the slot; code actions for each offered scope (once is meaningless in an editor, so persisted allow/deny only); request never waits |
| Library callers | explicit `resolve(template, &SlotReadPolicy, Option<Arc<dyn SlotReadApprovalHandler>>)`; `ComposeOptions::with_slot_read_policy` / `with_slot_approval_handler` | `SlotReadError::ApprovalRequired` |

**DMLS code action mechanics.** DMLS has no `workspace/executeCommand` today
(no `Command` producers in `dmls/src/providers/code_actions.rs`). A
`WorkspaceEdit` on a file under `~/.config` would open an unsaved buffer
outside the workspace in many editors. Add an `executeCommandProvider` with
one command, `darkmatter.grantSchemaRead {scope, target}`. It appends to the
user policy file (path from DMLS's `RequestSnapshot`, `dmls/src/main.rs:145`),
then schedules a slot refresh. DMLS picks up external policy edits by stat'ing
the file at each refresh (the file is outside the workspace watch).

**Default library policy.** Load the user file from the request's home when
one is known; this matches shell's library fallback to home
(`store.rs:52-56`). `SlotReadPolicy::none()` is for tests and hermetic hosts.

## 5. Q7 interaction (own writes and the automatic grant)

The facts: shell and lifecycle side effects invalidate the whole memo with no
path (F6). Only path-taking lifecycle verbs (`set_frontmatter` family,
`ensure_file`, `append_line`, `append_jsonl`) know what they wrote
(`lifecycle/executor.rs:989-1006`). With content-verified cleanliness (§3), a
written dependency no longer hashes to HEAD. A non-interactive lifecycle that
writes `os.yaml` and then validates would therefore fail
`approval-required` on every run, and an interactive one would prompt
mid-run.

**Recommendation:** grants are decided **once per run per target** and kept
as run-scoped grants for the rest of that run. A path that had an automatic
or persisted grant at its first decision keeps it after any write during the
run. Re-resolution (Q7) re-reads fresh bytes under the same grants and never
reuses the stale snapshot. This needs no write attribution, which shell
commands cannot provide anyway. It also avoids recapturing repository state,
which "never a fresh capture" forbids.

The risk is small. The run's own approved steps produced the change, or
another process changed a file that was clean in the same repository during
the run. A grant is never **created** by a write, never outlives the run, and
never applies in DMLS, which has no run. An editor save of a lifecycle-written
file shows as dirty → pending with a code action. If the author wants it
narrower, restrict the carry-over to paths named by path-taking lifecycle
verbs, and let shell-written changes lose the grant (Q-E).

## 6. Pure-evaluation work limit

- **Reuse:** `IterationBudget` (`expression/iteration.rs:13-73,238-380`)
  provides request-wide `max_visits` (default 100 000) and
  `max_expansion_bytes` (16 MiB) with typed `dm.iteration.visit_limit`/
  `dm.iteration.expansion_limit` errors. The evaluator charges a visit per
  callback item (`expression/mod.rs:873-877`) and takes its budget from the
  lookup or resolution context (`:566-571`).
- **Gaps:** (a) only `::map` regions charge bytes (`body_origin.rs`).
  Expression results (`join`, `as_csv`, `replace` over loaded data) do not;
  (b) there is no cancellation check in `evaluate_expr`; (c) `load_data` has
  no size cap (none in `expression/data/`).
- **Recommendation:** give each slot resolution its **own** `IterationBudget`,
  not the request's. A schema then cannot exhaust the document's allowance,
  and a cache hit replays its recorded `ExpansionUsage` as cached children do
  (`iteration.rs:76-95`). Use smaller limits in DMLS, set from S1's
  cold-resolution numbers. Charge `reserve_bytes` for each slot's flattened
  candidate list, and cap the candidate count. In DMLS, check a cancellation
  flag in `reserve_visit` and between slots, and drop superseded results as
  the spec requires. The AST is finite and there is no recursion or user
  function, so visits + bytes + cancellation bound pure work. A separate step
  counter is not needed.

## 7. Biggest risks

1. Policy-file location: "beside the shell whitelist" places grants in
   repository content (F1). Getting this wrong breaks the M5 guarantee.
2. Network/compose functions in slots (F4). Without a passive rejection
   list, opening a folder can reach the network through `pr()`/`ping()`, and
   `as_markdown` can compose.
3. Memo bypass (F9): the gate must precede the shared memo.
4. Persisted site grants leaking into document reads through a shared
   `RemoteReadConfig`/`FetchPolicy` (F3).
5. False "dirty" from Git filters (autocrlf/LFS) on Windows. False "clean"
   if the check uses status bits instead of read bytes (time-of-check versus
   time-of-use).
6. DMLS lane choice (F8): resolution in the registry lane would block requests.
7. Exfiltration channel: a granted site plus a computed query built from
   `ctx.env`/repository content can send data to that site. Consider letting
   computed URL targets match only exact/url-path grants, not site grants.
   S2 must also never persist resolved values derived from secret env values
   to disk.

## 8. Questions for the author

- **Q-A (defect in Q5 wording).** The shell whitelist is document-local or
  repository-root, not user-level (F1). Confirm the slot policy lives at
  `<home>/.config/darkmatter/schema-read-policy` and that no workspace config
  (`.dmls.toml`, editor settings) can relocate it.
- **Q-B.** Slots may call network functions (`ping*`, `pr*`, `cicd*`,
  `branch_exists_on_remote`, `remote_vendor`), `as_markdown`, and `find_files`.
  Recommend: reject the first two groups passively as schema errors; allow
  `find_files` only under a directory grant with no automatic grant.
- **Q-C.** Existence probes (`file_exists`, candidate probing). Should an
  existence check outside the defining repository need a grant? Recommend
  yes: gate the probed path, and leave probes inside the repository ungated.
- **Q-D.** A defining schema file outside the request's observed repository.
  Recommend no automatic grant (strict "captured observation"). The
  alternative is a lazily observed second repository per request.
- **Q-E.** Q7 carry-over: run-scoped retention of first-decision grants
  (recommended) versus losing the grant on any write.
- **Q-F.** `--allow host=` is host-wide (every scheme and port) and also
  governs document reads. Confirm it stays that way as the one-run "site"
  grant for slots, even though it is broader than Q5's scheme+host+port site.
- **Q-G.** Q5 lists only a persisted deny. The prompt still needs a
  this-run deny for EOF/cancel. Confirm, or make EOF a persisted deny.
- **Q-H.** Diagnostic codes: Q5 says "as shell does today", but shell uses
  `dm.shell.failure` (F7). Confirm the new codes
  `dm.schema.slot_approval_required` / `dm.schema.slot_read_denied`, with the
  grant lines in the message rather than a new `Diagnostic` field.
