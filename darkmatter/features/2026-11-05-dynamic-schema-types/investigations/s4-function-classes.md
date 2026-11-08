# S4 — Function determinism classes (2026-11-05-dynamic-schema-types)

Read-only investigation, 2026-11-07. No builds or tests were run. Paths are
relative to the repository root
`/Volumes/coding/wt/rusty-biscuit/feat-better-research`. `F/` means
`darkmatter/lib/src/markdown/compose/expression/functions/`, and `E/` means
`darkmatter/lib/src/markdown/compose/expression/`.

## Headline

1. **The registry's `pure`/`context` split does not tell you whether a
   function is deterministic.** `FunctionBinding::pure` (`F/mod.rs:121`) only
   means "no `ResolutionContext` argument". 14 functions registered `pure`
   read the live clock or the host timezone:
   - `date()` calls `Local::now().year()` at `F/dates.rs:85`.
   - The `is_today`/`is_yesterday`/`is_tomorrow`/`is_this_month`/`is_this_year`
     family, local and `_utc`, reads `Utc::now()`/`Local::now()` at
     `F/dates.rs:119`.
   - `date_delta`, `older_than`, and `newer_than` convert RFC 3339 offsets into
     `Local` (`F/mod.rs:638-645`, through `bound_date`, `F/dates.rs:40-47`).

   A slot class therefore needs its own registration fact. It cannot be
   inferred from `EvaluationMode`.
2. **Recommendation for volatile slots:** do not reject them as a class. Do not
   use a wall-clock TTL either. Instead:
   - (a) Move clock, timezone, and `PATH` reads onto the request's
     **captured** context. This turns 16 "volatile" functions into
     fingerprintable captured-context reads.
   - (b) For what stays live-volatile, **resolve once per command/request
     snapshot and never reuse across requests** ("resolve on every use", where
     a use is one pinned request).
   - (c) Key the validator cache on the **concrete snapshot content**. Then
     re-evaluating a volatile slot whose values did not change skips
     recompilation.

   URL reads are the one case that is TTL-like, and their freshness comes from
   the server (`Cache-Control`), not from us. The proposal rejects nothing in
   the volatile class, so **it is not narrowing**.
3. **Separate from Q3's `fm.*` rule, a small set must be rejected passively**
   because these are runtime/document-only bindings or composition and shell
   effects:
   - `as_markdown`.
   - The four login-shell probes (`has_alias`, `has_builtin_function`,
     `has_user_function`, `can_execute`).
   - Bare document-property reads.
   - Host globals (Claudine `loop`/`err`/`timing`/`group`).
   - The `ctx` document-identity keys (`self`, `last_updated`, `hash`, `id`,
     `sid`) and `ctx.cwd`.

   The shell-probe rejection needs an author ruling (see Rulings needed).
4. **The read set is incomplete for cache correctness.** 23 input-reading
   functions observe the filesystem but record nothing. Even `load_data` drops
   two kinds of dependency: a missing-file probe and a missing contract import.
   S2's fingerprint has to add these recordings. It cannot rely on today's read
   set.

## Classes used

| Class | Meaning | Cacheable across requests? |
|---|---|---|
| **P — pure** | The output depends only on the arguments | Yes, keyed on the schema hash |
| **I — input-reading** | Reads local bytes, existence, or directory listings that can be fingerprinted | Yes, if every observation is recorded and hashed |
| **R — remote input** | `I` by invocation when the target is an HTTP(S) URL | Yes, under the server's freshness rules; revalidating means a fetch, done off the request path |
| **C — captured-context** | Reads a value frozen in the request capture (`ctx.*`, `env.*`, a Sniff observation) | Yes, keyed on a hash of the captured value (never the secret text) |
| **V — live-volatile** | Reads live clock, git, network, process state, or randomness at call time | No; one resolution per request snapshot |
| **X — reject in slot** | Document/runtime-only binding, nested composition, or shell launch | n/a; positioned passive schema error |

A slot's class is the **join** over every call and root it reaches in every
branch (ternaries, `||`, the lazy `and`/`or`, and callback bodies), ordered
`P < I/R/C < V < X`. The cache key is the union of each member's observations.

## Classification table (120 registered built-ins)

"Passive targets" says whether a pre-flight walk can learn what the call will
read from its literal arguments (S5's question). "Transitive" says whether the
call can reach other evaluation or schema resolution.

### Pure (56)

| Functions | Evidence | Notes |
|---|---|---|
| `is_null`, `is_empty`, `is_string`, `is_number`, `is_array`, `is_object`, `is_file_type`, `is_positive`, `is_negative`, `is_integer`, `min`, `max`, `abs`, `round` (14) | `F/predicates.rs:8-23` | `is_file_type` is syntax-only (`F/predicates.rs:46-49`) |
| `first`, `last`, `has_key`, `contains`, `length`, `number`, `keys`, `values`, `as_line_separated`, `as_csv`, `as_tsv`, `as_space_separated`, `as_unordered_list`, `as_ordered_list`, `as_json`, `as_json5` (16) | `F/collections.rs:10-29` | |
| `map`, `filter` (callback, 2) | `F/collections.rs:11-12` | Class is the join of the callback body |
| `and`, `or` (lazy, 2) | `F/mod.rs:174` | Join of operands. **Every** branch counts for rejection, not only the evaluated one |
| `starts_with` … `trim` (17) | `F/strings.rs:11-29` | `without_date` is a regex over text |
| `terminal` (1) | `F/terminal.rs:8-18` | `Terminal::new_optimistic(80)` is a fixed profile (`biscuit-terminal/lib/src/terminal.rs:491-516`). Pure |
| `is_date`, `is_date_utc`, `is_date_time`, `is_date_time_utc` (4) | `F/dates.rs:14-18` | Format checks only |

### Input-reading, local filesystem (28; `load_data`, `file_exists`, `frontmatter` family, and `validate_schema` also become R on a URL)

| Function | Observes | Recorded today? | Passive targets | Transitive |
|---|---|---|---|---|
| `load_data` | File bytes (`F/../data/load.rs:94-118`). Contract `Name@file` imports (`E/data/runtime.rs:492-516`) | The found file, yes (`load.rs:73`). **A not-found probe, no**: it returns at `load.rs:64-66`, before `record`. **A missing import, no** (`runtime.rs:496-497`) | Yes for literal `file` and `as`. A computed argument is judged at evaluation (Q5) | **Yes.** The `as` contract resolves named-type imports, and an imported schema may be a dynamic template, which re-enters resolution and needs the shared resolution stack / cycle error |
| `file_exists` | Existence of each candidate | Yes, the hit or every probed candidate (`F/fs.rs:225-247`) | Yes (literal) | No |
| `frontmatter`, `try_frontmatter`, `markdown_title`, `markdown_body_empty` | Markdown file bytes | The resolved path, yes (`F/fs.rs:1031`). **A not-found probe, no** (`F/fs.rs:1025-1027` returns first) | Yes (literal) | No; frontmatter is read stored, not composed (`F/fs.rs:1146-1149`) |
| `validate_schema` | Page bytes, plus its `$schema` chain and imports | Yes, both (`F/fs.rs:1031`, `1236-1238`) | **Only the first file.** The `$schema` target sits inside that file's content | **Yes.** It runs `DarkmatterSchemas::validate`, so a dynamic `$schema` resolves recursively and needs its own approvals and cycle detection |
| `find_files` | Glob directory listing | **No.** Only glob warnings (`F/fs.rs:1137`) | A literal pattern gives a *directory set*, not a file set | No |
| `find_first_index`, `find_last_index` | `read_dir` of the parent (`F/fs.rs:553`) | **No** | Yes (parent directory) | No |
| `absolute`, `relative`, `basename`, `basename_without_index`, `dirname`, `ext`, `parent_dir`, `file_trailing`, `dir_leading`, `join`, `link`, `code_link`, `is_indexed_file`, `file_index`, `increment_file_index`, `decrement_file_index` (16) | Candidate existence. `resolve_in_context` picks the existing candidate, else the first planned one (`E/resolve_ctx.rs:431-458`, through `F/fs.rs:330-360`) | **No** | Yes (literal) | No. An implicit bare path can flip between the document-first and repository candidates when a file appears |
| `has_skill`, `has_local_skill` | `is_dir` probes under home and repository skill roots (`F/fs.rs:942-1001`). Agent from `ctx.agent`, else captured `AGENT` (`E/resolve_ctx.rs:282-292`) | **No** | Yes (name, but roots include `~`) | No. **Home probes lie outside Q5's automatic grant**; see Rulings needed |

### Captured-context (5)

| Function | Observes | Evidence |
|---|---|---|
| `package`, `package_area` | Captured repository topology, plus a path-shape existence probe | `F/repository.rs:29-51` |
| `has_agentic_cli` | Captured Agent `PATH` scan | `F/agentic_cli.rs:19-31` |
| `ipv4`, `ipv6` | Captured interface addresses | `F/network.rs:33-50` (`context.observations.addresses()`) |

These are deterministic for a given capture. The key must hash the
observation, and DMLS refreshes them only on an explicit refresh or a new
captured request (spec, "Cache correctness and refresh").

### Live-volatile (31)

| Function | Live input | Evidence | Remediation |
|---|---|---|---|
| `date` | Clock year (current year omitted) and host timezone | `F/dates.rs:85`, `F/mod.rs:638-645` | Read captured `DateTime` (`ctx.year`, `ctx.timezone_iana`) → C |
| `is_today`, `is_yesterday`, `is_tomorrow`, `is_this_month`, `is_this_year`, each with its `_utc` variant (10) | `Local::now`/`Utc::now` | `F/dates.rs:119` | Same → C |
| `date_delta`, `older_than`, `newer_than` | Host timezone for offset inputs | `F/dates.rs:40-47`, `F/mod.rs:638-645` | Same → C |
| `has_command`, `has_binary` | **Process** `PATH`, read directly through `which::which`, plus executable probes | `F/fs.rs:269-293` | Use the captured env `PATH` (`file_resolution_context.env()`) → C. This is also a "never recapture process state" defect today |
| `recent_commits` | Live history walk, explicitly unmemoized | `F/git.rs:20-43` | Stays V (or snapshot it into the request) |
| `predict_conflicts` | Live merge simulation | `F/git.rs:62-92` | V |
| `branch_exists_on_remote`, `remote_vendor` | git config plus a network probe under `remote_policy()` (deny-all default, `E/resolve_ctx.rs:243-248`). Memoized per request only | `F/git.rs:96-137` | V; egress under FetchPolicy |
| `pr`, `pr_list`, `cicd`, `cicd_list` | Provider API over the network | `F/pull_requests.rs:18-54`, `F/cicd.rs:16-87`, `F/provider.rs` | V; egress under FetchPolicy |
| `ping`, `ping_under` | ICMP, under a separate ICMP grant that is denied by default | `F/network.rs:75-95`, `compose/icmp.rs:1-21` | V; no Q5 grant form exists (ruling) |
| `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute` | **Launch the login shell with its profile** | `F/shell.rs:1-40`, `compose/shell_expansion/probe.rs:4` | **X** (see below) |
| `as_markdown` | Nested composition: shell, transclusion, remote, lifecycle | `F/composition.rs:13-18` | **X** |

The 14 clock/timezone functions and the 2 `PATH` functions (16) move to C under
remediation (a). After that, 15 remain: 10 genuinely V (git, provider, ICMP)
and 5 X.

### Roots and bindings (not functions)

Reserved roots come from `E/catalog/roots.rs:67-105`.

| Root / key | Class | Note |
|---|---|---|
| `fm.*` | X | Q3 |
| Bare identifiers | **X** | A bare root falls through to a *document property* (`.claude/skills/darkmatter/expressions.md`, root-resolution order), so it is `fm.*` by another spelling. Allowed only as a callback local inside `map`/`filter` |
| `local.*` | P | Only inside a callback scope |
| Host globals (`loop`, `err`, `timing`, `group`, …) | X | Runtime-only (`ComposeOptions::with_host_globals`) |
| `env.*` | C | Frozen snapshot. Hash it and never persist the text |
| `ctx.*` DateTime keys (`now`, `today`, `timestamp`, …) | C | Captured once per request (`compose/context/capture/datetime.rs:4-14`), so the slot **is** time-dependent and the key must include the captured value. A capture granularity of `now`/`timestamp_ms` means effectively no reuse |
| `ctx` Git/Repo/FileChanges/Languages/Docs/Os/Hardware/Gpu/Agent/Network keys | C | Captured observations |
| `ctx.self`, `ctx.last_updated`, `ctx.hash` | **X** | Root-*document* identity (`compose/context/capture/document.rs:15`), which contradicts "schema's context only" |
| `ctx.id`, `ctx.sid` | **X** (random) | Per-execution nonce (`document.rs:17-20`, `101-108`). Any cache would be wrong by construction |
| `ctx.cwd` | **X** (recommend) | Launch directory (`compose/context/capture/invocation.rs:4-12`). It is not part of the schema's derived context, and keying on it destroys sharing |
| `current.*` | V | Re-observed on each reference (`E/catalog/roots.rs:89-95`) |

### Host and test functions

- **Host functions** (`E/host/mod.rs`). The `HostHandler` contract says "must be
  pure" (`E/host/mod.rs:77-79`), but nothing enforces it. Treat them as P by
  contract, and put the **host-set identity** (`facts_artifact` content hash)
  into the resolver/catalog version key, because Claudine's process and `md`
  see different sets.
  - In DMLS the analysis-only set has no handler, so calling one fails with "no
    implementation in this process" (`E/functions/mod.rs:849-860`). A slot that
    calls one is **pending in DMLS**, never resolved.
  - Lookup order puts host and test functions ahead of built-ins
    (`E/functions/mod.rs:820-834`). Classification must follow dispatch, so
    look up the class through the same `host::lookup` → `find_binding` path.
- **Test hooks** exist under `cfg(test | effects-instrumentation)` only and can
  be ignored.

### Not functions despite the brief

`match` is the `file(match(…))` schema constraint
(`darkmatter/lib/src/markdown/schemas/about.rs:395-404`), not an expression
function. The glob functions in the expression catalog are `find_files` and
`::file-links`.

## Functions to reject in a slot, independent of Q3

All of these are judged passively in every branch, the same way the call
precheck judges a proven-bad call:

1. `as_markdown`: a nested composition can run `::shell` and lifecycle steps.
   Q5 excludes shell from slots.
2. `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`:
   these run the user's login-shell profile. Q5 excludes shell *syntax*, not
   functions that launch a shell. **Ruling needed.**
3. Bare document-property reads, host globals, `ctx.self`, `ctx.last_updated`,
   `ctx.hash`, `ctx.id`, `ctx.sid`, and `ctx.cwd`.
4. Already covered by Q3/Q4: `fm.*`, and any slot in a baseline or kind
   definition.

Lifecycle state reaches expressions only through host globals and `fm`, so
items 3 and 4 cover it. There are no body-only expression *functions*. The
body-only constructs (directives, `::map`/`::filter` blocks) are not expression
syntax, so a slot cannot contain them.

## Recommendation for volatile slots

| Option | Validator cache | DMLS | Verdict |
|---|---|---|---|
| **TTL** | The key gains a time bucket and recompiles every bucket. Validation results differ between CI and local depending on cache age | Needs a timer-driven refresh. Diagnostics flip without an edit. A stale-but-in-TTL snapshot would report "valid", which the spec forbids for a known-stale snapshot | Reject as a general mechanism. Keep it only for URL reads, where the server's `Cache-Control` already sets the lifetime (see `.claude/skills/darkmatter/remote-cache.md`) |
| **Resolve on every use** (use = one request/command snapshot, which the spec already pins) | No cross-request reuse of the *resolution*. With (c) below, compile reuse stays whenever the values are unchanged. Claudine's many-document run resolves once per compose request, not per document, if the request is shared | Re-resolves on each diagnostics refresh, off the request path. The snapshot is marked `volatile`, and hover/completion may show it as stale but never as current-valid | **Recommended** for remaining V slots |
| **Reject** | None | None | Narrowing. Not recommended, except for the X set above, which is a safety exclusion rather than a determinism one |

Supporting changes, in priority order:

- **(a) Move volatile reads onto the captured context.**
  - Date functions read the captured `DateTime` group.
  - `has_command`/`has_binary` read the captured env `PATH`.

  This also fixes a mid-run inconsistency in compose itself: `ctx.today` is
  frozen while `is_today()` is live. The result is that 16 of the 31 V
  functions become C.
- **(b) Classify by invocation.**
  - `load_data`, `file_exists`, `frontmatter`, `try_frontmatter`,
    `markdown_title`, `markdown_body_empty`, and `validate_schema` are R when
    the first argument is an HTTP(S) URL, and I otherwise.
  - A computed first argument is "class unknown until evaluation". Treat it as
    R for permission (Q5's computed-target rule) and record the class it
    actually took.
- **(c) Validator cache keyed on the concrete snapshot.**
  - Today the validator cache is keyed on schema + context.
  - Add a content hash of the resolved concrete schema as the compile key.
    Then a volatile slot that re-resolves to the same members costs only
    evaluation.
  - S1/S2 should measure evaluation against compile to confirm this is the
    dominant saving.
- **(d) Fill the read-set holes before any cross-request cache ships.** The
  holes are listed in "Input-reading" above:
  - path-shape probes;
  - `find_files` and `read_dir` listings, as a hash of the sorted entries;
  - skill probes;
  - not-found probes in `load_data` and `load_markdown`;
  - missing contract imports.

  Without this, an I-class cache can serve a result that a file creation should
  have invalidated, which violates "Retain dependencies on failures".

DMLS cost, by class:
- **P and I**: one resolution per input change, driven by file watches (S3).
- **C**: one resolution per capture refresh.
- **V**: one evaluation per diagnostics refresh, plus a compile only when the
  values change.
- **Egress** (provider and ICMP V functions): with automatic grants only, DMLS
  is already deny-all (`remote_policy()` defaults to `FetchPolicy::deny_all`),
  so these slots stay pending with a grant code action and cost nothing until
  granted.

## Where the classification should live

- **`FunctionBinding` (`F/mod.rs:86-102`).** Add:
  - `class: SlotClass { Pure, InputReading, Captured, Volatile, Rejected }`.
  - `reads: Option<ReadSide { parameter: &'static str, targets: Local|Url|Glob|Directory|Probe }>`.

  The constructors should require the class explicitly; `pure()` defaulting to
  `Pure` is what hid the date functions. Then add a registry test that every
  binding declares its class, and that every `Context` binding which touches
  the filesystem declares `reads`, mirroring the existing
  `BindingWithoutCatalog` checks (`F/mod.rs:255-296`).
- **Expose a passive query**, e.g.
  `functions::slot_class(&SpannedExpr) -> SlotClassReport { class, passive_targets, computed_targets, rejected: Vec<(span, reason)> }`.
  - It walks every branch and resolves names the way dispatch does:
    lowercase, alias, then host → built-in.
  - Darkmatter's implementation of the S7 resolver trait calls it in its
    passive `admit` step. simplified-schema stays host-neutral: it only sees an
    opaque admission verdict.
- **Host functions** take the same fact through `HostFunctionsBuilder` and the
  facts artifact (bump `FACTS_ARTIFACT_FORMAT_VERSION`). Default to `Pure` per
  the handler contract.
- **Do not put it in `schemas/partials/functions.yaml`.** That file is the
  signature/doc catalog in SimplifiedSchema `function(...)` grammar. Adding an
  effects clause would change simplified-schema's `FunctionType` for a
  Darkmatter-only concern. `md schema about` can still *display* the class
  from the registration.

### Reusing `REMOTE_READ_FUNCTIONS` (`darkmatter/lib/src/markdown/compose/remote.rs:28-36`)

Reuse the **walk**, not the **list**.

- `collect_expression_urls` (`remote.rs:420-440`) is the right shape for S5's
  passive target collector: it walks every sub-expression and takes the
  literal first argument.
- The list itself is a hand-kept duplicate of what should be the
  `reads: Some(ReadSide { targets: Url, .. })` fact. It covers only the
  URL-capable readers, not path probes, globs, or directory listings.
- It is also matched inconsistently:
  - `collect_expression_urls` compares names **exactly** (`remote.rs:429`), so
    an alias or capitalized call such as `loaddata("https://…")` or
    `LOAD_DATA(...)` is missed by pre-fetch discovery and fetched only on
    demand.
  - `calls_read_side_function` folds case and underscores (`remote.rs:450-458`).

  For S5's batched pre-flight approval, that miss would turn a passively
  knowable target into a mid-run prompt.

Derive both functions from the binding facts, and have the slot walker use the
same derivation.

## Rulings needed

1. **Shell-launching probes** (`has_alias`, `has_builtin_function`,
   `has_user_function`, `can_execute`). Should these be rejected in slots as an
   extension of Q5's shell exclusion? (Recommended.) Or allowed as V? They run
   the user's shell profile, which goes beyond Q5's M5 replacement ("evaluate
   pure slots and read clean files … and nothing else"). Note that
   `can_execute` short-circuits through `has_binary` without a shell
   (`F/shell.rs:33-38`). It could be split, but rejecting it whole is simpler.
2. **Non-file local observations under Q5.** Q5 defines grants for a file and a
   directory. The open cases are:
   - `has_skill` probes `~/…/skills/<name>`;
   - `find_files` and `find_*_index` list directories, and a "clean" judgment
     is undefined for a listing that may include untracked entries;
   - `has_command` probes `PATH` directories.

   Proposal: a listing inside the defining schema's repository is
   automatically granted only when the repository observation shows no
   untracked or ignored entries under that directory. Anything else asks with
   the directory-grant choice. `has_skill`/`has_command` probes outside the
   repository are V/C reads that need no grant, because they reveal only
   existence. Confirm or overrule.
3. **Provider and ICMP egress in slots.** `pr*`, `cicd*`, and
   `branch_exists_on_remote` ride on FetchPolicy, so Q5's site grant can feed
   them. `ping`/`ping_under` need an ICMP grant, which Q5 has no persisted form
   for. Should slots reject ICMP, or should the policy file gain an
   ICMP-address grant?
4. **`ctx.cwd`**: reject (recommended), or substitute the schema's directory?

## Counts

Built-ins (120):
- 56 pure
- 28 input-reading; 7 of these are also R by invocation
- 5 captured-context
- 31 live-volatile as the code stands today. After remediation (a): 15, which
  splits into 10 V and 5 X.

Roots: 7 root or key families are rejected (X), as listed in the roots table.
