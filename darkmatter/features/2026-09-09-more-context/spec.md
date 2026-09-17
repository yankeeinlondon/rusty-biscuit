---
status: draft
implemented: true
implemented_by: claude/opus
created: 2026-09-09
reviewed: true
reviewed_by: codex/gpt-6-astra
reviewed_on: "2026-09-11"
review_iterations: 6
clarified: claude/claude-fable-5-1
rulings: R1-R33 ruled by Ken 2026-09-10/11; folded into body; Q1-Q3 recommendations adopted 2026-09-16 (plan Phase 1, see decisions.md), pending Ken's confirmation
inputs:
  - ../../docs/schemas/darkmatter.yaml
  - ../../docs/schemas/expression-functions.yaml
  - ../../lib/src/markdown/compose/context/capture/groups.rs
  - ../../lib/src/markdown/compose/context/capture/snapshot.rs
  - ../../lib/src/markdown/compose/context/capture/repo.rs
  - ../../lib/src/markdown/compose/shell_expansion/alias.rs
  - ../../lib/src/markdown/compose/remote_fetch.rs
  - ../../lib/src/markdown/compose/cache/operation.rs
  - ../../lib/src/markdown/compose/subtree.rs
  - ../../../sniff/lib/src
  - ../../../sniff/lib/src/filesystem/repo/types.rs
  - ../../../sniff/lib/src/filesystem/git/recent_commits.rs
  - ../../../claudine/lib/src
  - ../../../claudine/lib/src/composition/lifecycle/context.rs
  - ../../../claudine/lib/src/composition/reserved.rs
  - ../../../claudine/docs/providers.yaml
  - ../../../prompts/_implement/implement-plan.md
related:
  - ../_completed/2026-06-15-context-vars-additions
  - ../_completed/2026-07-01-has-command-fn
  - ../_completed/2026-05-28-darkmatter-hashing
  - ../../fixes/2026-09-16-content-policy-no-cache
  - decisions.md
depends-on:
  - ../../fixes/2026-09-16-content-policy-no-cache
---

# More Context

## Summary

Adds document-identity, network, git-history, and shell-probe context to
Darkmatter compose: ten new `ctx.*` variables, fourteen new expression
functions, and two fixes: the broken `ctx.worktree` value under Claudine, and
the sniff `"root"` sentinel leaking into `ctx.current_package_area`. It also
introduces the `current` and `current_env` globals (lazily evaluated mirrors of
`ctx` and `env`, **R30**–**R33**, replacing Claudine's lifecycle-only
`current.ctx.*` / `current.env.*` shape) and the variable + function pair rule
(**R29**), with `recent_commits` as the first pair. Network probes (gateway, Tailscale, ICMP) land in **sniff** so
other consumers can reuse them. Claudine's `claudine context` surfaces stay in
sync because they are projected from the same descriptor catalog.

> Reader's note (inline review, 2026-09-11): R1–R33 remain the record of
> Ken's rulings. Clarifications below preserve those goals; Q1–Q3 identify
> unresolved conflicts rather than silently overruling them. `reviewed: true`
> records completion of this review, not approval of those open decisions.
> Source line numbers are navigation hints, not stable contracts.

## Scope

In scope:

- the `ctx.*` variables and expression functions listed below, in Darkmatter
  compose (both `md compose` and Claudine-driven compose)
- the `ctx.worktree` fix
- the empty-string scope fix for `ctx.area`, `ctx.current_package_area`,
  `ctx.current_package`, including removal of sniff's `"root"` sentinel at the
  source (**R24**–**R26**)
- the sniff additions required to support them
- descriptor updates so `claudine context` / `claudine context --expressions`
  list every new variable and function
- a claudine-gen step that emits the `has_agentic_cli` name enum into
  Darkmatter (**R13**)
- the `current` and `current_env` reserved roots in every Darkmatter
  expression surface, and the clean-break migration of Claudine's lifecycle
  `current.ctx.*` / `current.env.*` spelling to `current.*` / `current_env.*`
  (**R30**–**R33**)

Out of scope:

- DMLS evaluating any of these values (diagnostics/hover only see descriptors)
- changes to remote-fetch behavior beyond reusing its allow-list for ICMP
- any change to the persistent compose cache beyond the prerequisite fix
  (**R18**; the cache-disable portion is implemented as
  `fixes/2026-09-16-content-policy-no-cache` during Phase 1, see Q3)
- anything not listed in this document

## Background (verified in code)

- `ctx.*` variables are schema-projected from the `ctx:` block of
  `docs/schemas/darkmatter.yaml`. Capture is grouped (`ContextGroup`),
  demand-driven, captured once per request, and fail-closed under Claudine: a
  group Claudine does not supply yields `null` plus a `PartialRuntimeCapture`
  diagnostic. Every new variable must map to a group; every new group needs a
  Claudine evidence builder.
- `ctx.timestamp_ms` and `has_command` already exist.
- `Markdown::hash` already yields the `{fm}-{body}` xxHash form printed by
  `md hash`.
- `biscuit-hash` has an optional `blake3` feature.
- `shell_expansion/alias.rs` resolves aliases by spawning `$SHELL -ic` under
  `setsid` with a timeout; the new shell probes reuse it.
- The capture entry points receive only a base directory, not the document
  path. `ctx.self` and the other document-identity values require threading the
  root document path into capture (implementation note, not a design question).
- The repository observation captured per request already carries the package
  list behind `ctx.current_package` / `ctx.current_package_area`; the new
  `package` / `package_area` functions look up in that data (**R12**).
- The descriptor catalog has no `dir` parameter type; path arguments are
  `file | string`, and `absolute()` already defines how a path string resolves
  (document base dir, repo root, `@` magic roots) (**R12**).
- Darkmatter cannot depend on claudine (claudine depends on darkmatter), so
  anything sourced from `claudine/docs/providers.yaml` must reach Darkmatter by
  generation (**R13**).
- `ipnet` is already in the lock file transitively; it becomes a direct
  Darkmatter dependency (**R14**). `docs/dependencies.md` must be updated.

## Context Variables

Availability is distinct from absence: an observed no-repository/no-route
result uses the documented `""`, `[]`, `false`, or `null` value. Missing
required evidence also emits `PartialRuntimeCapture` and uses the existing
group's schema-compatible empty/null projection; it must never trigger ambient
fallback. Required scope strings stay `""` and required `tailnet` stays `false`
in that projection, with the diagnostic distinguishing unavailable evidence
from an observed absence. All other nullable fields remain optional in schemas.

Group names marked *proposed* are new `ContextGroup` variants.

### Document identity (group: *proposed* `Document`)

Per **R1**, all five describe the root document of the compose request (the
prompt being executed), captured once at the start of execution. Transcluded
or included fragments see the root document's values.

- `ctx.self` — `string`. Absolute canonical path of the root document with
  native separators (**R5**). Present for a successfully resolved local root;
  see the source-kind contract below for non-file inputs.
- `ctx.last_updated` — `datetime`. Filesystem modification time of the root
  document. Null when the filesystem does not report one.
- `ctx.hash` — `string`. `{fm}-{body}` xxHash computed over the on-disk source
  before composition; equals what `md hash <file>` prints for that file (**R3**).
- `ctx.id` — `string`. xxHash over: page source, `ctx.timestamp_ms`, hostname,
  repository name, and a per-execution random nonce (Q1). Distinguishes
  executions (**R2**): two runs with identical inputs in the same millisecond
  differ with practical (not mathematical) certainty. Repository name is `""`
  outside a git repository; the id is still generated (**R4**).
- `ctx.sid` — `string`. Same inputs as `ctx.id`, hashed with blake3 via
  `biscuit-hash`'s `blake3` feature, providing a cryptographic digest (**R2**). This is not encryption or a
  secrecy guarantee: a known candidate page and known inputs can be checked
  against an unkeyed digest. Same inputs and nonce as `ctx.id`, so the same
  uniqueness guarantee.

Source-kind contract: `ComposeSource` already supports `File`, `Url`, and
`Unknown` (`context/options.rs`); adding document identity must preserve all
three entry points. For a local file, retain the bytes read for the root at
request creation and compute `ctx.hash` through `Markdown::hash` on that
source, before composition or caller overrides. Do not re-open the file later
and accidentally hash a different revision. An unavailable modification time
is `null`; an unreadable explicit local root follows normal source-load error
handling. For URL and in-memory roots, `ctx.self`, `ctx.last_updated`, and
`ctx.hash` are `null`: do not invent a native path or perform an extra fetch.
`ctx.id` / `ctx.sid` still use the supplied root source. `claudine context
--values` without a document uses these absent-document projections rather
than selecting a file implicitly.

All five fields retain root identity in transclusions and `as_markdown`.
Length-prefix and version the hash input tuple (UTF-8 source, integer epoch
milliseconds, hostname, repository name, execution nonce); concatenate neither
ambiguous strings nor display-formatted timestamps. The exact encoding is
recorded in `decisions.md` (D1). The nonce is 16 bytes from the operating
system's CSPRNG, drawn once per root compose request and shared by every
fragment of that request; failure to obtain entropy is a typed compose error,
never a fallback to a fixed or time-derived nonce. Missing hostname uses `""`,
with the normal capture diagnostic where evidence was not supplied. Use the
existing `biscuit-hash` xxHash output convention and full BLAKE3 hexadecimal
output; freeze test vectors through a crate-internal nonce injection seam (no
public override).

### Host (group: existing `Os`)

- `ctx.hostname` — `string`. Host name as reported by sniff's `OsInfo`. Null
  when unavailable.

### Network (group: *proposed* `Network`)

- `ctx.tailnet` — `boolean(required)`. True when any host interface has an
  address in `100.64.0.0/10` (CGNAT range) (**R7**). Known false positive:
  CGNAT ISP addresses and other VPNs using that range are reported as `true`;
  this is accepted. Uses the same CIDR helper as `ipv4` / `ipv6` (**R14**).
- `ctx.gateway` — `string`. Primary IPv4 gateway address. `null` when there is
  no default route, or when the default route has no gateway address (**R15**,
  **R21**).
- `ctx.gateway_v6` — `string`. Primary IPv6 gateway address. `null` when there
  is no default route or the route has no gateway address (**R21**). Link-local gateways keep their `%scope` suffix (e.g.
  `fe80::1%en0`) so the value stays routable and usable with `ping()` (**R15**).

"Primary" (**R15**): the gateway of the lowest-metric UP default route, the
rule sniff already applies when choosing the default-route interface on Linux
and Windows; on macOS, the first default route in the routing-socket dump. Both
families ship in the first cut.

### Git (group: existing `Git`)

- `ctx.recent_commits` — `string[]`. The last 10 commits of the captured
  repository, newest first, sourced from sniff's
  `get_recent_commits_by_count(repo_root, 10)` (**R27**). Each element is one
  commit rendered exactly as `sniff repo recent-commits --plain` renders that
  commit (the per-commit block below), so an element is multi-line. `[]`
  outside a repository.

Per-commit plain block, verified against the library renderer
(`sniff/lib/src/filesystem/git/recent_commits.rs:760-806`, `plain = true`):

```text
- [<7-char hash>] <type>(<scope>) at <time>: <description>

    **Description:**

    - <bullet point>

    **Files Impacted:**

    - <added|modified|deleted|renamed|copied>: <repo-relative path>

```

Format facts the element contract inherits: the `<type>(<scope>) ` prefix is
omitted when the message is not a conventional commit (`- [abc1234] at
<time>: <description>`); `<time>` is `1:01pm Today`, `12:32pm Yesterday`, or
`2026-04-01 at 9:30am` in the viewer's local time zone; the `**Description:**`
block is omitted when the message has no bullet points; the block ends with a
blank line. Concatenating the elements reproduces
`CommitDescSet::describe(true)` byte for byte. Two consequences carried into
AC24: the renderer skips commits that touched no files
(`render_commit_centric`, `if files.is_empty() { continue; }`), so an empty
commit shortens the array; and the `Today` / `Yesterday` labels make elements
day- and zone-dependent.

Cost: `CommitDesc` carries per-file changes and package attribution, so capture
is demand-driven — only when `ctx.recent_commits`, `current.recent_commits`,
or `recent_commits(...)` is referenced — which does **not** follow from the existing `Git`
group gate alone: `ctx.branch` also demands that group. Retain the public Git
group (R27), but add a recent-history requirement within it. Only an eager
`ctx.recent_commits` demand captures the launch-time history; a lazy
`current.recent_commits` or function call registers a deferred capability and
does no history I/O until reached. This refines the broad evidence-builder
instruction in R27 without changing its values. Implementation note: `get_recent_commits_by_count` also runs
`detect_repo` for package attribution the plain block never prints; a variant
that skips attribution is acceptable if the output is unchanged.

## Expression Functions

All function names (including aliases) are described in `docs/schemas/expression-functions.yaml` with
frontmatter/body parity. Unless stated, a failed probe returns the documented
failure value rather than erroring.

### Composition (**R11**, **R20**)

- `as_markdown(content: string) -> string` — treats `content` as Markdown,
  composes it with Darkmatter, and returns the composed Markdown text. Full
  compose: every stage (interpolation, conditionals, transclusion, shell, link
  resolution) runs on the content through the **same** request-scoped
  `ComposeOptions` and the **same** captured ctx snapshot as the root
  document. The base directory for relative references is the root document's
  directory. The existing transclusion depth limit guards recursion (content
  that composes itself through `as_markdown` errors at the limit). Consent and
  effect policy are the root's: the same allow-list and preflight apply, so a
  `ping()` or `$()` inside the content appears in preflight (**R20**).

Nested composition contract: run `as_markdown` through a child pipeline that
shares the root context, resolution authority, consent, diagnostics, and
recursion budget. Increment depth for every `as_markdown` invocation, even
without `::file`; share cycle ancestry with transclusions. Return the composed
Markdown serialization, including any resulting frontmatter, without rendering
terminal/HTML output. Perform root-only link normalization once at the outer
root. Preserve nested source provenance in diagnostics.

Preflight must not execute functions, shell profiles, ICMP, or lazy globals to
discover effects. Statically available `as_markdown` content is scanned
recursively, including conditionally unreachable branches. If effect-bearing
content or a shell command cannot be determined without execution, reject that
shape before execution under pre-approved mode, using the existing dynamic
command-shape diagnostic contract. No nested pipeline may obtain fresh consent
implicitly. Frontmatter-local restrictions survive nesting: `as_markdown`
called from frontmatter cannot introduce remote reads through a nested body.
ICMP remains subject to its explicit effect grant on every surface (R6).

### Repository (**R12**, **R26**)

- `package_area(where: file | string) -> string` — name of the package area
  containing `where`; `""` on any miss.
- `package(where: file | string) -> string` — name of the package containing
  `where`; `""` on any miss.

Shared rules: after normal argument resolution, a pure in-memory prefix lookup over the captured repository
observation's package list (the data behind `ctx.current_package` /
`ctx.current_package_area`); no re-discovery. Valid lookup misses never error. A miss is any of:
`where` outside the captured repository, `is_monorepo == false`, or no
package/area whose path prefixes `where`. A miss returns `""`, never `null`,
so the two functions and the three scope variables share one spelling of "no
area" (**R26**). The `string` overload is a path
string resolved through `FileReference` with the same captured
`FileResolutionContext` as `absolute()` (document base dir, repo root, `@`
magic roots). Preserve typed argument/reference errors; “never errors” in R12
means a lookup miss, not accepting malformed references. Remote references are
not fetched. Match path components, not string prefixes (`foo` must not match
`foobar`); choose the deepest containing package/area independently, including
an area directory with no containing package. Reuse established native path
normalization and comparison behavior, including Windows drive/UNC handling.
Nonexistent local descendants may match lexically; do not add filesystem
canonicalization or topology discovery to the lookup stage.

### Git (**R28**)

- `recent_commits(count: number) -> string[]` — the `count` newest commits of
  the captured repository, each element in the same per-commit plain block as
  `ctx.recent_commits`. Evaluated at **call** time: it performs git I/O when
  called and is never cached across calls (`EvaluationMode::Context`, like
  `predict_conflicts` in `expression/functions/git.rs`). Any `count` ≥ 1 is
  accepted, including counts above 10; `count` ≤ 0 or non-integer → compose
  error. Outside a repository → `[]`. "Repository" is the request's captured
  repository root, never re-discovered from the CWD. `recent_commits(10)`
  differs from `ctx.recent_commits` only by *when* it is evaluated.

### Network

- `ipv4(filter?: string) -> string[]` — the host's IPv4 addresses (**R14**).
  Loopback and link-local addresses are excluded by default. With a filter:
  a CIDR selects addresses inside that network, and a CIDR that covers loopback
  or link-local (e.g. `"127.0.0.0/8"`) includes them; any other string (e.g.
  `"192.168.10"`) is a substring match over the default (non-loopback,
  non-link-local) set.
- `ipv6(filter?: string) -> string[]` — as `ipv4` for IPv6 addresses
  (`"::1/128"` includes loopback).
- CIDR rule (**R14**): the filter is a CIDR when it contains `/` followed by a
  valid prefix length (≤32 for v4, ≤128 for v6) and the network part parses;
  anything else is a substring match. Implemented with `ipnet`; `ctx.tailnet`
  reuses the helper.
- `ping(address: IpAddress, timeout?: number) -> boolean | null` — ICMP reachability
  of `address` (v4 or v6). `timeout` is in milliseconds, default 100. Governed
  by **R6**:
  - target not allow-listed via `--allow-host` / `ComposeOptions` (an exact IP
    or a CIDR containing it) → `null` plus a warning diagnostic
  - allow-listed, no reply within `timeout` → `false`
  - allow-listed, host cannot send ICMP (privilege or API failure) → compose
    error
  - planned pings are surfaced by compose preflight as approvable effects,
    like `$()` shell
- `ping_under(address: IpAddress, timeout: number, attempts?: number) -> boolean | "unstable" | null`
  — `attempts` pings (default 3), each with `timeout` ms as the timeout.
  `true` if all replies arrive under `timeout`, `false` if none do,
  `"unstable"` otherwise. Consent and error rules identical to `ping` (**R6**).
  The return type must be representable in the descriptor catalog as
  `boolean | "unstable"`.

ICMP arguments: `IpAddress` is a descriptor-level validated string, accepting
IPv4 and IPv6 literals and scoped IPv6 (`fe80::1%en0`); it is not a Rust
`IpAddr` alias that loses the scope. No DNS lookup. Reject malformed addresses,
non-finite or non-positive time budgets, and fractional/non-positive attempts
before probing. Numeric conversions must be checked for overflow. Attempts
run sequentially; each has a monotonic deadline and `ping_under` has a bounded
total budget of attempts × timeout plus bounded setup/cleanup. A reply at
or after the threshold counts as late. Any send/API failure aborts the call,
even after earlier successes. A denied multi-attempt call returns one `null`
and diagnostic and sends no packets. Descriptor return types include `null`.

> Reader's note: `biscuit_file::FetchPolicy` currently supports exact hosts and
> wildcard host patterns, not CIDRs (`file_reference/fetch.rs`). R6 therefore
> needs an ICMP policy projection, not just reuse of the HTTP matcher.

Keep the `--allow-host` entry point, but parse exact IP/CIDR grants into an
ICMP-specific policy carried by `ComposeOptions`. CIDR grants must not widen
HTTP fetch permission; existing HTTP exact/wildcard behavior stays unchanged.
ICMP hostname/wildcard grants do not authorize an IP indirectly. Validate
CIDRs strictly here (the substring fallback belongs only to address filters).
Normalize IP spelling before comparison and retain an explicit IPv6 scope
constraint when supplied. Preflight effects are typed ICMP records, not shell
command strings; the runtime validates the actual target against the grant
before sending. A dynamic target may only use an already approved IP/CIDR
capability, never acquire permission as a side effect of evaluating it.

Address enumeration uses sniff and preserves routable IPv6 scope suffixes;
CIDR matching compares address bits without the suffix. Deduplicate identical
address-and-scope pairs and use stable address ordering. A valid CIDR of the
other IP family yields `[]`; malformed filters retain R14's substring behavior.

### Shell (**R8**, **R9**, **R10**, **R16**, **R17**)

The probes query the user's login shell (`$SHELL`), not the shell Claudine was
launched from, by spawning `$SHELL -ic <dialect-specific query>` per call
through the existing alias-resolution mechanism (`setsid` + timeout). Dialects
supported in the first implementation: bash, zsh, fish, PowerShell. Any probe
failure (no `$SHELL`, unsupported shell, timeout, non-zero exit) returns
`false`. On Windows, when `$SHELL` is unset, probe with `pwsh`, falling back
to `powershell.exe`, loading the profile so aliases and functions are visible
(**R17**).

- `has_alias(name: string) -> boolean` — `name` is an alias in the login shell.
- `has_binary(name_or_path: string) -> boolean` — alias of the existing
  `has_command` (**R16**): true when `name_or_path` is found on PATH or is an
  existing executable absolute path. Same implementation, both names
  documented.
- `has_builtin_function(name: string) -> boolean` — `name` is a *shell builtin*
  (`cd`, `export`, `alias`, ...) in the login shell (**R9**; the function name
  is kept, only the description changes).
- `has_user_function(name: string) -> boolean` — `name` is a user-defined
  function in the login shell.
- `can_execute(name: string) -> boolean` — `has_alias(name) || has_binary(name)
  || has_builtin_function(name) || has_user_function(name)` (**R10**).

“Login shell” here means the configured shell executable, not adding `-l`:
`-ic` is the existing Unix interactive-startup contract. Use each dialect's
own invocation switches; PowerShell cannot be invoked as a POSIX shell.
Reuse the bounded subprocess launcher, **not** `resolve_alias`'s result:
that function rejects aliases whose expansion does not resolve to an external
binary, whereas `has_alias` must recognize aliases to builtins and functions.
On PowerShell, aliases and functions use their native command categories;
cmdlets count as builtin commands for `has_builtin_function`.

Probe names are data, never interpolated as executable shell syntax. Queries
must not execute the named command. Bound profile startup, stdout/stderr, and
process cleanup; use detached sessions on Unix and the appropriate Windows
process containment, with closed stdin and no focus changes. Preserve the
existing two-second timeout per shell query. Passive validation, hover, and
preflight never launch a profile. `can_execute` may short-circuit (checking
`has_binary` first avoids unnecessary profile launches); its logical OR result
is unchanged. Use controlled shell/profile fixtures, never developer dotfiles.

### Agentic CLIs (**R13**, **R19**)

- `has_agentic_cli(agent: enum(...)) -> boolean` — whether the named agentic
  CLI is installed on the host, detected through sniff's `InstalledAiClients`
  (PATH index). An unknown name is a compose error, never `false`.

Name source: `claudine/docs/providers.yaml` (the provider roster). The accepted
names are each entry's `slug` plus its `cli_aliases` (so `kimi_code` resolves
to the same provider as `kimi`); each entry's `sniff_binding` names the sniff
`AiCli` variant used for detection. Because darkmatter cannot depend on
claudine, the enum reaches Darkmatter by **generation**: claudine-gen emits the
enum (names plus the name → sniff-binding mapping) into Darkmatter's
descriptor/binding source. This makes claudine-gen a producer of a Darkmatter
artifact for the first time.

Implementation tasks: define the generated artifact's location inside
Darkmatter, and add a drift check that fails when the committed artifact no
longer matches `providers.yaml`. Roster entries with `skip_research: true`
remain valid roster identities and are included in the enum (**R22**). Note
the coupling: claudine-gen's per-provider loader fails loudly for a skipped
entry, so the enum emission must read the roster directly rather than route
through per-provider generation. No entry currently sets the flag.

## Variable and function pairs (**R29**)

When a context variable and an expression function share a name they share one
semantic definition and one output format. `ctx.<name>` is the value captured
once at the start of execution; `<name>(args)` is the same descriptor evaluated
lazily at call time, with parameters that give the caller more control. Both
are declared from the **same descriptor entry**, so `claudine context` and
`claudine context --expressions` cannot drift: the variable is listed once, the
function once, both projected from that entry.

Pairs (first instance; more will follow):

- `recent_commits` — `ctx.recent_commits` (last 10, captured at start) and
  `recent_commits(count)` (any count, evaluated at call).

Implementation note (not a ruling): today variables live in the `ctx:` block of
`darkmatter.yaml` and functions in `expression-functions.yaml`. The implementer
chooses how one entry projects both (a pairing marker on one side, or a shared
source both files are generated from); AC26 checks the outcome, not the
mechanism.

## The `current` and `current_env` globals (**R30**–**R33**)

Ken's definition (2026-09-11):

- `current` and `ctx` are exactly the same data structure. `ctx` is eager,
  captured once at the start of execution; `current` is lazy, each key
  evaluated when referenced. The spelling is a direct mirror: `current.<key>`
  for every `ctx.<key>`, so `ctx.repo == current.repo` unless the
  repository's name changed while the prompt was executing. There is **no**
  `current.ctx.*` nesting. The primary purpose of `current` is evaluation inside
  lifecycle events, where state may have changed since launch, but it is
  available in every expression surface where `ctx` is (**R31**).
- `current_env` is the same lazy mirror of `env`, as a separate global:
  `current_env.<key>` for every `env.<key>`, evaluated when referenced. It
  replaces the old `current.env.*` shape (**R32**).
- Clean break: Claudine's lifecycle shapes `current.ctx.*` and `current.env.*`
  are removed, not aliased; every usage migrates to `current.*` /
  `current_env.*`. Ken acknowledged this is a change from the current
  environment (**R33**).

Definition of `current_env` (implementation note folded into the ruling):
`env.*` is read from a frozen snapshot, never from the live process. Under
Claudine-driven compose the environment comes from the frozen invocation
snapshot (`claudine/lib/src/composition/resolve.rs:145` passes
`provisional_context.env().clone()` into the evidence bundle;
`darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs:68-69`
documents the bundle as starting "with the invocation's immutable
environment"; `.claude/skills/claudine/composition.md:90` says the same). Under
ambient `md compose` it is `std::env::vars()` collected once at capture
(`capture/mod.rs:103`, `context/runtime.rs:154`). `current_env.<key>` therefore
means: **re-read the live process environment for `<key>` at reference time**
(`std::env::var(key)`), which is what Claudine's `LifecycleCurrent::capture_env`
already does for `current.env.*` (`lifecycle/context.rs:460-474`), now spelled
`current_env.<key>` and available everywhere.

Verified today:

- Darkmatter's evaluator reserves three roots, `doc`, `ctx`, `env`
  (`docs/topics/darkmatter-expressions.md:174-184`;
  `lib/src/markdown/compose/subtree.rs:258`), and supports injected globals
  that can be `InjectedGlobal::Lazy`, memoized per `LayeredLookup` instance
  (`subtree.rs:77-95`, `:147`, `:193-200`).
- Claudine injects `current` only for lifecycle handler strings
  (`claudine/lib/src/composition/reserved.rs:19` lists it among
  `LATE_BINDING_ROOTS`; `lifecycle/context.rs:522-551` injects it lazily),
  shaped `current.ctx.*` / `current.env.*` by `LifecycleCurrent::to_value`
  (`:506-511`), and re-captures the full ambient context at event time
  (`:487-503`). Documented at `claudine/docs/topics/lifecycle.md:437`.
- Claudine's `LATE_BINDING_ROOTS` (`reserved.rs:19`: `err`, `timing`,
  `current`) is the single known-root authority consulted by
  `lifecycle/validate.rs:591` (`resolves_outside_frontmatter`) and by
  `preflight.rs:460` (`late_binding_root_in_expr`).
- Lifecycle `shell` commands are approved at preflight against an
  **early-binding-only** lookup (`doc.*`, `ctx.*`, `env.*`, frontmatter;
  `preflight.rs:291-300`); a `shell` command referencing any late-binding root
  is rejected at preflight (`first_late_binding_root`, `:441`).
- `prompts/format.md:25` already spells `{{length(current.dirty_files)}}` —
  the R31 spelling — inside a lifecycle `stdout` string, which cannot resolve
  under today's `current.ctx.*` nesting.

Required behavior:

- `current` and `current_env` become Darkmatter **reserved roots** available in
  every expression surface: frontmatter, body, `when=`, `md compose`, and
  Claudine-driven compose (including lifecycle handler strings). The
  reserved-roots table in the expressions docs and the descriptor catalog list
  both.
- `current` mirrors every `ctx` key by name; `current_env` mirrors every `env`
  key by name. Neither has any other member: `current.ctx.x`, `current.env.x`,
  and `current_env.ctx.x` are unknown paths and fail the way any unknown
  `ctx.<missing>` path fails today.
- Each `current.<key>` / `current_env.<key>` is evaluated lazily and memoized
  **per expression evaluation, per key** (Q2): repeated reads of one key within
  a single expression observe one value, while a later expression (another
  `{{ }}` interpolation, frontmatter value, `when=` condition, or `$()`
  branch) observes the fact afresh. There is no atomic snapshot across
  different keys. Today's lazy-global memo is per `LayeredLookup` instance
  (per subtree compose) and must be narrowed to this scope for the two reserved
  roots. Every lifecycle event starts fresh evaluation scopes; mutable facts
  are observed when the handler reaches them, rather than at launch. Child
  pipelines share the invocation-owned provider, never memoized observations,
  and no memo lock is held while a nested expression evaluates.
- Under Claudine-driven compose the lazy `current.*` read must go through the
  same evidence path as `ctx` (fail-closed: an unsupplied group yields `null`
  plus the `PartialRuntimeCapture` diagnostic), not a fresh ambient capture.
  `current_env.*` is launch-area independent and reads the live process
  environment directly.
- `shell` early-binding exception (kept): lifecycle `shell` commands remain
  approved at preflight against early-binding surfaces only, so a `shell`
  command that references `current.*` or `current_env.*` is rejected at
  preflight exactly as `current.*` is today. `current_env` must be added to
  `LATE_BINDING_ROOTS` (`claudine/lib/src/composition/reserved.rs:19`) so both
  the preflight rejection and the undefined-variable scan treat it as a known
  late-binding root; the `err`/`timing`/`current` lists in the doc comments at
  `lifecycle/validate.rs:460`, `:577` and in the two lifecycle docs
  (`claudine/docs/topics/lifecycle.md:813`, `.claude/skills/claudine/lifecycle.md:834`)
  follow.
- Capture planning tracks eager requirements separately from lazy capabilities,
  including function dependencies, across all expression surfaces and nested
  documents. Unreached lazy references perform no host probes. Do not build the
  whole `current` object to resolve one key. Bare-root enumeration must use
  descriptor keys without materializing every expensive value.
- Fresh Claudine reads require an invocation-owned evidence provider that can
  refresh the requested fact against the retained launch root; replaying the
  frozen evidence bundle cannot implement freshness. Missing capability is
  fail-closed, never an ambient fallback. Request-owned identity, source path,
  resolution anchors, and repository root remain fixed even for `current`;
  mutable facts such as branch and working-tree changes can refresh. This is
  an intentional extension of snapshot-only capture, not permission to
  rediscover CWD or repository topology downstream.
- Preflight may list deferred context-read requirements as metadata, but never
  evaluates them. Reserved roots cannot be shadowed by frontmatter or injected
  globals. Preserve current unknown-path behavior, and define missing
  `current_env` values the same way as missing `env` values.

Migration list (**R33**). Repo-wide grep on 2026-09-11 for `current.ctx.` and
`current.env.` in `*.md`, `*.rs`, `*.yaml`, `*.toml` under `prompts/`,
`claudine/`, `darkmatter/`, and `.claude/skills/`, excluding `target/`,
`node_modules/`, `.gitnexus/`, and features/fixes `_completed` dirs. The
implementer must repeat the grep and treat the result as the blast radius.

- Shipped prompts and templates: no `current.ctx.` / `current.env.` hits.
  `prompts/format.md:25` already uses `current.dirty_files` and must compose
  after migration (AC28).
- Claudine code (the nesting's source and its consumers):
  `lib/src/composition/lifecycle/context.rs:44-45` (module doc), `:443-456`
  (`LifecycleCurrent { ctx, env }`), `:460-474` (`capture_env` doc),
  `:487-503` (`capture_at_event`), `:506-511` (`to_value` nests `ctx` / `env`);
  `lib/src/composition/reserved.rs:19` (`LATE_BINDING_ROOTS`, add
  `current_env`); `lib/src/composition/looping/engine.rs:991`, `:1004-1005`;
  `cli/src/commands/compose/prep.rs:578`;
  `cli/src/commands/wrap/composition/pipeline.rs:1698`, `:1701`;
  `cli/src/commands/wrap/composition/preflight.rs:144`;
  `cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs:406`,
  `:419`.
- Claudine tests: `lib/src/composition/lifecycle/context/tests.rs:516-517`,
  `:526-530`, `:561`, `:574`, `:587`;
  `lib/src/composition/lifecycle/tests/diagnostics.rs:339`, `:516`;
  `lib/src/composition/looping/engine/tests/mod.rs:15`, `:31-32`, `:36-37`;
  `lib/src/composition/prepare/service/tests.rs:160-164`, `:176`, `:195`;
  `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs:14`, `:25-26`,
  `:31`, `:63`; `cli/tests/composition_seams.rs:287`, `:325`.
  `cli/tests/level2_lifecycle_control.rs:938` is a false positive (a local
  recording struct's `env` field, not the lifecycle global).
- Darkmatter tests that use `current.ctx.today` as an arbitrary injected-global
  fixture and must be re-pointed once `current` is a reserved root:
  `lib/src/markdown/compose/subtree.rs:631`,
  `lib/src/markdown/compose/tests/frontmatter.rs:492` (fixture globals named
  `current` also at `subtree.rs:627`, `:715`, `:740`;
  `tests/frontmatter.rs:487`, `:599`, `:631`).
- Claudine user docs: `claudine/docs/topics/lifecycle.md:437` (the globals
  table row), `claudine/docs/topics/composition.md:797`, `:868`.
- Skills (`.claude/skills/claudine/SKILL.md`, `lifecycle.md`,
  `composition.md`): updated 2026-09-11 to the ratified shape with a
  "ratified, implementation pending" marker. At completion the implementer
  removes the markers and re-hashes `composition.md`; no re-edit of the
  content is expected.
- Other in-flight feature specs that describe the old shape as history and are
  outside the AC29 grep scope (code, shipped prompts, user docs, skills):
  `claudine/features/2026-08-26-finalized-references/spec.md:751`,
  `claudine/features/2026-08-01-faster-compose/spec.md:115`, `:376`,
  `claudine/features/2026-07-13-proxy-with/spec.md:488`, `plan.md:852`.

## Fixes

### ctx.worktree

Expected: the directory basename of the linked worktree the compose runs from;
`null` in the main checkout or outside a repository (the schema already says
this).

Diagnosed cause: Claudine's compose capture builds repository evidence with
sniff's `GitRequest::summary()`, which sets `include_worktrees: false`. The
evidence projection (find the current worktree in the enumerated list, take
`filepath.file_name()`) therefore never finds anything and always yields
`null`. The ambient path (`try_current_worktree_name` in `snapshot.rs`) works.

Required behavior: the Claudine compose path must produce the same value as the
ambient path. Enumerating all linked worktrees is not required; a cheap
current-worktree lookup is acceptable (see Sniff additions).

### Empty-string scope (**R24**, **R25**, **R26**)

Expected: `ctx.area`, `ctx.current_package_area`, and `ctx.current_package`
are strings that are `""` — never a sentinel, never `null` — when there is no
scoped area or package. Rationale (Ken, 2026-09-11): an empty string is falsy,
so the variables work directly in conditional clauses (`when="ctx.area"`,
`{{ ctx.area || 'repo root' }}`) while still naming the scope when one exists.

Expected values by position (monorepo layout with area `sniff` holding package
`sniff-lib` at `sniff/lib`, and top-level package `model_id`):

| Position | `ctx.area` | `ctx.current_package_area` | `ctx.current_package` |
|---|---|---|---|
| inside `sniff/lib` (package under an area) | `sniff-lib` | `sniff` | `sniff-lib` |
| inside `model_id` (top-level package) | `model_id` | `""` (today `"root"`) | `model_id` |
| inside `sniff/` but not a package | `sniff` | `sniff` | `""` (today `null`) |
| at the monorepo root | `""` | `""` (today `null`) | `""` (today `null`) |
| outside a monorepo | `""` | `""` (today `null`) | `""` (today `null`) |

Diagnosed cause, in three parts:

- `ctx.area` is **not defective**: `capture/repo.rs:124-147` already projects
  `""` at the monorepo root and outside a monorepo. The literal `monorepo-root`
  Ken saw is the implement-plan prompt's own fallback
  (`prompts/_implement/implement-plan.md:15`:
  `area: "{{ ctx.area ? ctx.area : ctx.is_monorepo ? 'monorepo-root' : 'repo-root' }}"`),
  which fires exactly when `ctx.area` is empty; ternaries are right-associative
  (`expression/parser.rs:35`), so it parses as intended. `ctx.area` is covered
  by the new contract and AC21 only so it cannot regress; the implement-plan
  fallback keeps working unchanged.
- The real defect is sniff's sentinel: `make_package_area`
  (`sniff/lib/src/filesystem/repo/detection.rs:1388-1393`) assigns
  `package_area: "root"` to packages directly under the repo root (field doc at
  `repo/types.rs:157-158`), and `area_for_dir` / the directory fallback return
  `"root"` at the repo root (`types.rs:311-352`, tests at `:799`, `:839`;
  `aggregate_view.rs:103, 294, 308, 319`). Darkmatter's
  `current_package_context` (`capture/snapshot.rs:734-759`, used by both the
  ambient and the Claudine evidence path) passes the string through, so inside
  a top-level package `ctx.current_package_area` is the truthy string `"root"`.
  `repo.rs:133` and `repository_scope.rs:43` already special-case `"root"`,
  proving the leak; a June Claudine fix works around it with
  `ctx.current_package_area == 'root' ? ctx.current_package || '' : ctx.current_package_area`
  (`claudine/fixes/2026-06-30-completion-failures/spec.md:68`).
- `ctx.current_package` / `ctx.current_package_area` are declared nullable
  (`docs/schemas/darkmatter.yaml:181-182`, "or null when unavailable") while
  `area` is `string(generated; required)` (`:183`). `null` is already falsy
  (`docs/topics/darkmatter-expressions.md:94-99`), so the `null → ""` part is
  about one consistent spelling and string typing, not truthiness.

Required change:

- Sniff (**R24**): eliminate the sentinel at the source. `Package.package_area`
  is `""` for packages directly under the repo root; `area_for_dir` and the
  directory fallback return `""` at the repo root and wherever they returned
  `"root"`. Sniff CLI output (`sniff repo area` prints an empty line at the
  root instead of `root`; keep its exit-status contract distinct from "no
  results"), `aggregate_view`, and sniff tests are updated accordingly. The
  sentinel is gone from the data model only: CLI listings such as
  `sniff repo package-areas` may render the empty area with a display label
  (for example `(root)`) chosen at render time, never stored in `package_area`.
- Darkmatter (**R25**): `ctx.current_package` and `ctx.current_package_area`
  become `string(generated; required)` in `darkmatter.yaml`, projected as `""`
  whenever there is no package / no area (including the monorepo root and
  outside a monorepo); the "or null when unavailable" wording is removed.
  `ctx.area` keeps its existing `""` contract. The `"root"` special-cases at
  `repo.rs:133` and `repository_scope.rs:43` are removed because nothing
  produces the sentinel any more.
- Functions (**R26**): `package(where)` / `package_area(where)` return `""` on
  a miss (see Repository functions).

Cleanup list. The implementer must repeat this grep (repo-wide, excluding
`target/`, `node_modules/`, `.gitnexus/`, and features/fixes `_completed`
dirs) for `== 'root'` / `== "root"` / `!= "root"` against
`package_area` / `current_package_area` / `area` and treat the result as the
blast radius. Hits on 2026-09-11:

- Prompts and templates (workarounds to remove, or comparisons that are dead
  today because `ctx.area` never yields `"root"` and become simple truthiness
  checks):
  `system-prompt.md:3` (`ctx.area == 'root' ? 'package' : 'package area'`,
  currently always the else branch), `prompts/_reviews/feature-review.md:137`
  (`when="ctx.area != 'root'"` → `when="ctx.area"`), and the shipped
  implement-plan prompt fallback at `prompts/_implement/implement-plan.md:15`
  (works unchanged; no edit needed). Claudine prompt fixtures: none found
  outside `_completed`; `claudine/fixes/2026-06-30-completion-failures/spec.md:68`
  records the workaround and stays as history.
- Sniff lib: `repo/detection.rs:1388-1393` (source), `:1930`, `:1978`, `:2556`,
  `:2563`, `:2599` (tests); `repo/types.rs:157-158`, `:311-352`, `:454`,
  `:613`, `:766`, `:799`, `:838-839`; `repo/aggregate_view.rs:103`, `:294`,
  `:308`, `:319`, `:326`, `:973`, `:1043`, `:1056`; `repo/aggregate.rs` test
  fixtures passing `"root"` as an area (`:666-687`, `:912`–`:1506`).
- Sniff CLI: `output/filesystem/mod.rs:1343`, `:1368`, `:1420`, `:1445`,
  `:2447`; `output/filesystem/package_areas.rs:28`, `:43`, `:181`, `:386`,
  `:405`, `:415`; `output/filesystem/repo.rs:61`;
  `output/filesystem/deps.rs:100`; `output/repo_json.rs:2932`, `:2989`;
  `args/repo.rs:494` (help text); `sniff/docs/cli/repo_deps.md:66`.
- Darkmatter: `lib/.../capture/repo.rs:133`,
  `lib/.../repository_scope.rs:43` (and its test fixture at `:70`),
  `cli/src/commands/compose.rs:203`, `:265`.
- Claudine: `lib/src/composition/launch_workspace.rs:135`,
  `lib/src/composition/lifecycle/control.rs:311`,
  `lib/src/system_prompt/context.rs:140`, `lib/src/events/environment.rs:558`,
  `cli/src/completion/scopes.rs:297-322`,
  `cli/src/commands/wrap/env/package_context.rs:247`,
  `cli/src/commands/wrap/env/mod.rs:68`, `cli/src/commands/wrap/env/tests.rs:721`.

Every sentinel-specific branch above becomes an `is_empty()` check or is
deleted; sentinel fixtures become `""`. Inspect each occurrence: a real area
named `root`, or an unrelated root-directory label, must not be rewritten.

## Caching (**R18**)

Q3 outcome: the cache-disable portion of
`fixes/2026-09-16-content-policy-no-cache` is a hard prerequisite, implemented
during Phase 1 of this feature's plan. Persistent storage is limited to raw
remote-URL response bodies under `--cache-root`; no composed document, operation
result, or document snapshot is persisted, so no composed output containing
these values can be replayed.

Ken's ruling: local file transclusion does not need caching. Persistent caching
is for content that arrives through an expensive or agentic operation (for
example, summarizing a website). Invalidating such content requires a
`ContentPolicy` struct that defines the freshness policy of the cached content.
Until `ContentPolicy` exists, nothing is cached persistently.

Consequences here: the new `ctx.*` values and `ping` / `ping_under` results
never participate in any persistent cache, and this feature makes no change to
the existing volatile-exclusion list in `cache/hashing.rs`. Disabling the
current `--cache-root` persistent cache and designing `ContentPolicy` are a
separate fix (`fixes/2026-09-16-content-policy-no-cache`); its cache-disable
portion ships before this feature.

## Sniff additions

New capabilities sniff does not have today:

- **Gateway address** parsing, both families, on macOS, Linux, and Windows
  (**R15**); today only the default-route interface *name* is parsed. IPv4:
  read the gateway column in the existing Linux (`/proc/net/route`), Windows
  (`route print`), and macOS (PF_ROUTE sysctl, `RTAX_GATEWAY`) parsers. IPv6:
  add passes over `/proc/net/ipv6_route`, `route print -6` (or
  `Get-NetRoute -AddressFamily IPv6`), and AF_INET6 entries in the BSD
  routing-socket dump, preserving `%scope` on link-local gateways.
- **CGNAT/Tailscale helper**: `true` when any interface address is inside
  `100.64.0.0/10` (**R7**). May be a small function over the existing
  `ip_addresses` aggregation rather than a new detector.
- **ICMP probe module**: single ping with millisecond timeout plus the
  multi-attempt variant, working on macOS, Linux, native Windows, and WSL2
  (**R6**). Must report "cannot send ICMP" distinctly from "no reply".
- **Cheap current-worktree exposure** (optional): a way for Claudine's summary
  request to learn the current worktree name without `include_worktrees: true`,
  if enumerating worktrees is judged too costly.
- **`"root"` sentinel removal** (**R24**): `Package.package_area` is `""` for
  top-level packages; `area_for_dir` and the directory fallback return `""` at
  the repo root; `aggregate_view`, sniff CLI output, docs, and tests follow.
  This is a behavior change for every sniff consumer that compares against
  `"root"` (see the cleanup list under Fixes).
- **Per-commit plain formatter exposure** (**R27**): the `--plain` renderer
  already lives in the sniff **library**
  (`sniff/lib/src/filesystem/git/recent_commits.rs`), not the CLI, but only as
  the private `render_commit_block` behind the set-level
  `CommitDescSet::describe(plain)`. Expose the per-commit formatter from the
  library (for example `CommitDesc::describe_plain(&self, repo_root, today)`)
  so Darkmatter builds each array element from it and the format has one
  owner. `sniff repo recent-commits --plain` must keep delegating to the same
  code.
- **Claudine's Git-group evidence asks for commits** (**R27**): Claudine's
  `GitRequest::summary()` sets `commit_count: 0` (`sniff/lib/src/request.rs`),
  and raising it would only fill `GitInfo.recent: Vec<CommitInfo>` (sha,
  message, author, timestamp, refs — `git/types.rs:1396-1411`), which lacks
  the per-file change kinds, paths, and parsed bullet points the plain block
  prints. Verified: the `CommitDesc` path is required. When eager recent history is
  demanded within the `Git` group, the Claudine evidence builder calls
  `get_recent_commits_by_count(repo_root, 10)` (`git/mod.rs:28-33`) alongside
  the summary request; `summary()` itself is unchanged.

Existing sniff surfaces reused as-is: interface enumeration, primary
interface, `ip_addresses`, `wan_ip_address`, `OsInfo` hostname, `AiCli` enum
and `InstalledAiClients`, `get_recent_commits_by_count` / `CommitDescSet`.

## Claudine

`claudine context` and `claudine context --expressions` iterate the
Darkmatter descriptor catalog, so they are kept in sync by updating
`darkmatter.yaml`, `expression-functions.yaml`, and the function bindings. No
separate Claudine listing needs editing. Only `claudine context --values`
performs its own ambient capture; it must produce the new values too.

Claudine-driven compose must supply evidence for the proposed `Document` and
`Network` groups (and the `Os` addition), otherwise the fail-closed rule turns
every new value into `null`. The existing `Git` group's evidence builder must
additionally request commit descriptions only when eager history is demanded
(**R27**, see Sniff additions).

Pairs are listed once: `claudine context` shows `recent_commits` as a variable
and `claudine context --expressions` shows it as a function, both projected
from the one descriptor entry (**R29**).

`current` / `current_env` migration (**R31**–**R33**): lifecycle handler
strings today resolve `current.ctx.*` / `current.env.*` through Claudine's own
injected global (`LifecycleCurrent`). Once `current` and `current_env` are
Darkmatter reserved roots, that global and its nesting are removed — no alias,
no transition period. Every handler, test, doc, and skill in the migration list
under "The `current` and `current_env` globals" moves to `current.<key>` /
`current_env.<key>`; `current_env` joins `LATE_BINDING_ROOTS`; the `shell`
preflight rejection of late-binding roots is unchanged. AC28 and AC29 guard
the outcome.

claudine-gen gains the `has_agentic_cli` enum emission described above
(**R13**).

## Rulings

R1–R10 ruled by Ken 2026-09-10; R11–R33 ruled 2026-09-11.

- **R1** — Document identity values (`ctx.self`, `ctx.last_updated`,
  `ctx.hash`, `ctx.id`, `ctx.sid`) describe the root document of the compose
  request and are captured once at the start of execution. Transcluded and
  included fragments see the root document's values.
- **R2** — `ctx.id` is unique per execution (the ms timestamp stays in the hash
  input) and is never cache-stable; the spec says so explicitly. `ctx.sid` uses
  the same inputs with blake3 via `biscuit-hash`'s `blake3` feature.
- **R3** — `ctx.hash` is computed over the on-disk source before composition
  and equals `md hash <file>` for that file.
- **R4** — The repository-name input to `ctx.id` / `ctx.sid` is `""` outside a
  git repository; the id is still generated.
- **R5** — `ctx.self` is the absolute canonical path with native separators.
- **R6** — `ping` / `ping_under` are network effects under the existing
  allow-list consent model (`--allow-host` / `ComposeOptions`, exact IP or
  containing CIDR). Not allow-listed → `null` + warning. Allow-listed, no reply
  within timeout → `false`. Allow-listed but ICMP cannot be sent → compose
  error. Planned pings appear in compose preflight as approvable effects. ICMP
  transport lives in sniff and works on macOS, Linux, native Windows, WSL2.
  Timeouts in ms; `ping` default 100 ms; `ping_under` default 3 attempts with
  the `true` / `false` / `"unstable"` tri-state.
- **R7** — `ctx.tailnet` is true when any host interface address is in
  `100.64.0.0/10`; CGNAT/VPN false positives are accepted.
- **R8** — Shell probes query the user's login shell (`$SHELL`) via
  `$SHELL -ic <query>` per call, reusing the alias-resolution mechanism.
  Dialects: bash, zsh, fish, PowerShell. Any probe failure returns `false`.
- **R9** — `has_builtin_function` means *shell builtin*; name kept, description
  changed.
- **R10** — `can_execute` is the plain logical OR of `has_alias`, `has_binary`,
  `has_builtin_function`, `has_user_function`.
- **R11** — `as_markdown(content: string) -> string` treats `content` as
  Markdown and fully composes it (all stages) through the same request-scoped
  `ComposeOptions` and the same captured ctx snapshot as the root document;
  relative references resolve against the root document's directory; the
  transclusion depth limit guards recursion; consent and effect policy are the
  root's. Returns the composed Markdown text.
- **R12** — `package_area` / `package` take `(where: file | string)` (no `dir`
  type in the catalog; the string overload resolves like `absolute()`), return
  `""` on any miss (outside the captured repo, `is_monorepo == false`, no
  match; amended from `null` by **R26**), never error, and are a pure
  in-memory prefix lookup over the captured repository observation's package
  list with no re-discovery.
- **R13** — `has_agentic_cli` names come from `claudine/docs/providers.yaml`:
  each entry's `slug` plus `cli_aliases`; `sniff_binding` names the sniff
  `AiCli` variant used for detection through `InstalledAiClients`. The enum
  reaches Darkmatter by claudine-gen generation (names + binding mapping) into
  Darkmatter's descriptor/binding source. Unknown name → compose error.
  `skip_research: true` entries remain valid roster names and are included in
  the generated enum (**R22**).
- **R14** — `ipv4` / `ipv6` return `string[]`; loopback and link-local are
  excluded by default and included only by an explicit covering CIDR. A filter
  is a CIDR when it has `/` plus a valid prefix length (≤32 v4, ≤128 v6) and a
  parseable network; otherwise it is a substring match. `ipnet` becomes a
  direct dependency; `ctx.tailnet` reuses the CIDR helper.
- **R15** — `ctx.gateway` and `ctx.gateway_v6` both ship in the first cut.
  "Primary" is the gateway of the lowest-metric UP default route (macOS: first
  default route in the routing-socket dump). `null` when no default route.
  Link-local IPv6 gateways keep the `%scope` suffix. Sniff reads the gateway
  column in its existing v4 parsers and adds v6 passes on all three OSes; AC13
  fixtures cover both families on all three OSes.
- **R16** — `has_binary` is an alias of `has_command`. Confirmed.
- **R17** — Windows shell-probe rule confirmed: with `$SHELL` unset, probe with
  `pwsh`, falling back to `powershell.exe`, loading the profile.
- **R18** — Local file transclusion does not need caching. Persistent caching
  is for content that arrives through an expensive or agentic operation.
  Invalidating such content requires a `ContentPolicy` struct that defines the
  freshness policy of the cached content. Until `ContentPolicy` exists, nothing
  is cached persistently. New ctx values and ping results never enter a
  persistent cache; the volatile-exclusion list is untouched; disabling
  `--cache-root` and designing `ContentPolicy` are a separate unscheduled fix.
- **R19** — Roster flow detail: unknown `has_agentic_cli` names error (see
  R13).
- **R20** — Content composed through `as_markdown` is subject to the same
  allow-list and preflight as the root; a `ping()` inside it appears in
  preflight (clarifies R6).
- **R21** — A default route with no gateway address (on-link or
  point-to-point: macOS `link#N`, Linux gateway `0.0.0.0`, VPN `utun` routes)
  yields `null` for `ctx.gateway` / `ctx.gateway_v6`, the same as "no default
  route". AC13 carries a fixture of this shape.
- **R22** — Roster entries flagged `skip_research: true` are included in the
  generated `has_agentic_cli` enum; the flag governs research and generation
  fan-out, not host detection.
- **R23** — Acceptance criteria AC1–AC20 were signed off 2026-09-11 as the
  definition of done; AC21–AC23 were added the same day with the empty-string
  fix Ken asked for directly, AC24–AC28 with R27–R30, and AC29 with R31–R33;
  the definition of done is AC1–AC29. L1 criteria must pass CI on macOS,
  Linux, native Windows, and WSL2; AC2 and AC28 are the L2 criteria (the
  Claudine leg of AC21 is L1
  unless the implementer shows it needs a real Claudine run, in which case it
  joins AC2 at L2).
- **R24** — The sniff `"root"` sentinel is eliminated at the source:
  `Package.package_area` is `""` for packages directly under the repo root, and
  `area_for_dir` / the directory fallback return `""` at the repo root and
  wherever they returned `"root"`. Sniff CLI output, `aggregate_view`, and
  sniff tests are updated accordingly. Darkmatter's `"root"` special-cases
  (`repo.rs:133`, `repository_scope.rs:43`) are removed since nothing produces
  the sentinel any more. The implementer must grep `"root"` in
  `sniff/lib/src/filesystem/repo` and every sniff CLI rendering of package
  areas and treat the result as the blast radius (recorded under Fixes).
- **R25** — `ctx.current_package` and `ctx.current_package_area` become
  `string(generated; required)` and are `""` whenever there is no package / no
  area, including at the monorepo root and outside a monorepo. `ctx.area`
  keeps its existing `""` contract. The schema wording "or null when
  unavailable" is removed for both.
- **R26** — R12 is amended: `package(where)` and `package_area(where)` return
  `""` (not `null`) on any miss (outside the captured repo, non-monorepo, no
  match), so there is one spelling of "no area" across the three ctx variables
  and the two functions. The Repository function definitions and AC17 replace
  every `null` expectation for these with `""`.
- **R27** (2026-09-11) — `ctx.recent_commits` is `string[]`: the last 10
  commits of the captured repository, each element one commit in exactly the
  per-commit block `sniff repo recent-commits --plain` prints (multi-line).
  Group: existing `Git`. `[]` outside a repository. The formatter lives in the
  sniff library; the per-commit formatter is exposed from the library so the
  format has one owner. Capture is demand-driven. Claudine's evidence builder
  requests the commit descriptions (`CommitDesc` path, not `GitInfo.recent`)
  when the `Git` group is demanded.
- **R28** (2026-09-11) — `recent_commits(count: number) -> string[]`: same
  meaning and per-element format as `ctx.recent_commits`, evaluated at call
  time with git I/O, never cached across calls (`EvaluationMode::Context`).
  `count` ≥ 1 (any size); ≤ 0 or non-integer → compose error. Outside a
  repository → `[]`. Repository = the request's captured root, never
  re-discovered from CWD.
- **R29** (2026-09-11) — Variable + function pair rule: a context variable and
  an expression function that share a name share one semantic definition and
  one output format; `ctx.<name>` is captured once at start, `<name>(args)` is
  the same descriptor evaluated lazily at call time with caller-controlled
  parameters; both are declared from one descriptor entry so `claudine
  context` and `claudine context --expressions` cannot drift. `recent_commits`
  is the first pair.
- **R30** (2026-09-11) — `current` global: every `ctx` variable is also
  available on `current`, a lazily loaded mirror of `ctx`; each key evaluates
  its descriptor when referenced. `current` becomes a Darkmatter reserved root
  in every expression surface, listed in the expressions docs and the
  descriptor catalog. Memo scope is per compose request, per key (working
  assumption). Claudine's lifecycle `current.ctx.x` spelling is reconciled
  with `current.x` (the "alias or migration" clause is superseded by **R33**:
  migration only). Whether preflight enumerates `current.*` reads is an
  implementation note.
- **R31** (2026-09-11) — `current` and `ctx` are exactly the same data
  structure: `ctx` is eager (captured once at the start of execution),
  `current` is lazy (each key evaluated when referenced). Spelling is a direct
  mirror, `current.<key>` for every `ctx.<key>` (`ctx.repo ==
  current.repo` unless the repository's name changed while the prompt was
  executing). There is no `current.ctx.*` nesting. Its primary purpose is
  evaluation inside lifecycle events, where state may have changed since
  launch, but it is available in every expression surface where `ctx` is.
- **R32** (2026-09-11) — A lazy mirror of `env` is added as the separate
  global `current_env`: `current_env.<key>` for every `env.<key>`, evaluated
  when referenced; it replaces the old `current.env.*` shape. Because `env.*`
  is read from the frozen invocation snapshot (ambient compose: the snapshot
  taken at capture), `current_env.<key>` means re-reading the live process
  environment for that key at reference time.
- **R33** (2026-09-11) — Clean break. The Claudine lifecycle shapes
  `current.ctx.*` and `current.env.*` are removed, not aliased; every usage
  (code, tests, shipped prompts, user docs, skills) migrates to `current.*` /
  `current_env.*`. Ken acknowledged this is a change from the current
  environment. `current_env` joins `LATE_BINDING_ROOTS`; the `shell`
  early-binding preflight rejection continues to apply to both roots.

## Open Questions

The following review questions exposed conflicts not resolved by the earlier
rulings. On 2026-09-16 Phase 1 of the plan adopted each recommendation and
folded it into the body above (`decisions.md`, D1). These outcomes await Ken's
confirmation; they are not rulings R34+.

### Q1 — Execution identity guarantees (R2)

A millisecond timestamp plus identical source/host/repository inputs can repeat;
unkeyed BLAKE3 also cannot promise that a candidate page is unrecognizable.

- **Recommend adding a per-execution random nonce to both hash tuples.** Pros:
  preserves all listed inputs and makes accidental repeated execution IDs
  negligible. Cons: expands R2's input contract and requires an OS-randomness
  source plus explicit entropy-failure handling. This best serves the stated
  execution-identity goal; keep the digest wording above and promise practical,
  not mathematical, uniqueness. Share the nonce across the root's fragments.
- Keep the exact four inputs and weaken the promise to a best-effort execution
  fingerprint. Pros: smallest implementation and reproducible test vectors.
  Cons: same-millisecond executions deliberately collide; fails the current
  uniqueness intent and requires revising AC4.
- Use a keyed BLAKE3 digest for `sid` plus an execution nonce. Pros: prevents
  outsiders without the key from checking candidate inputs. Cons: introduces
  key storage, rotation, and cross-process semantics absent from this feature.
  Choose only if candidate confidentiality is an actual requirement.

**Outcome (adopted 2026-09-16):** per-execution nonce, as recommended. The
tuple gains a fifth field (16 CSPRNG bytes), entropy failure is a typed compose
error, and AC4/AC36 test same-millisecond executions.

### Q2 — Freshness versus memoization (R30–R32)

Request-wide first-read memoization contradicts AC27's second read after a
branch switch. It also turns “at reference time” into “at first reference time.”

- **Recommend memoization per expression evaluation, per key.** Pros: one
  expression sees a consistent repeated-key value while later expressions and
  lifecycle events can observe changes; matches AC27's intent. Cons: more
  observations across expressions and no atomic snapshot across different keys.
  Share the invocation-owned provider, not cached observations, across child
  pipelines; do not hold memo locks while evaluating nested expressions.
- Memoize per compose request/event, per key. Pros: bounded I/O and stable
  values throughout a document. Cons: AC27 must change to compare distinct
  requests/events; state changes after first read remain invisible.
- Never memoize. Pros: literal per-reference freshness. Cons: repeated keys
  can disagree within a single expression and duplicate expensive probes.

Tests and public documentation must use the selected scope consistently. The
identity fields remain request-owned under every option; lazy lookup does not
create a new execution identity.

**Outcome (adopted 2026-09-16):** memoize per expression evaluation, per key,
as recommended. AC27/AC28/AC36 test same-expression stability and
later-expression freshness.

### Q3 — Persistent-cache rollout dependency (R18)

The statement “nothing is cached persistently” cannot be guaranteed while the
fix disabling the existing `--cache-root` cache is unscheduled. Avoiding new
cache-key fields does not prevent stale composed output from containing them.

- **Recommend making the cache-disable portion of the referenced fix a hard
  prerequisite before this feature ships.** Pros: directly honors R18 without
  implementing a second cache policy. Cons: this feature depends on scheduling
  that work. Record `depends-on` when that prerequisite has an active spec;
  do not pretend that the current `related` link orders implementation.
- Add a narrow persistent read/write bypass for any subtree using these
  volatile values, propagated to ancestors. Pros: permits independent rollout.
  Cons: widens this feature's cache scope and needs dependency tracking through
  generated content, functions, and lazy globals.
- Disable persistence for all composition in this feature. Pros: simple and
  correct. Cons: duplicates the deferred fix and changes existing public
  `--cache-root` behavior here, requiring an explicit scope amendment.

**Outcome (adopted 2026-09-16):** hard prerequisite, as recommended. The fix
was activated as `fixes/2026-09-16-content-policy-no-cache` and its
cache-disable portion implemented in Phase 1: composed `::file` children,
`::code` and `::toc-linking` results, and document snapshots are never read
from or written to the persistent store. Pending Ken's ruling on the fix's
open question, `--cache-root` stays scoped to raw remote-URL bodies, which
carry no composed context. A warm-cache regression test proves composed
output is not replayed.

## Acceptance Criteria (**R23**)

Test level per `rust-testing`: L1 unless marked L2. Tests that launch `md`
use `CliProcessFixture`; pure library tests do not need a CLI subprocess.
L1 must pass CI on macOS, Linux, native Windows, and WSL2.

- **AC1** — Descriptor corpus: every new `ctx.*` variable appears in
  `claudine context` and every new function in `claudine context --expressions`.
- **AC2** — `ctx.worktree` under a real Claudine compose from a linked worktree
  equals the ambient value (directory basename); from the main checkout it is
  `null`. (L2: needs a real Claudine run.)
- **AC3** — `ctx.hash` for a fixture equals the output of `md hash <file>`.
- **AC4** — `ctx.id` differs across two runs of the same document, including
  two runs with an identical frozen clock, hostname, and repository name;
  `ctx.sid` differs from `ctx.id` in the same run. Frozen test vectors with an
  injected nonce match the D1 encoding, and an entropy failure is a typed
  compose error.
- **AC5** — `ctx.self` is absolute, canonical, native-separated; `ctx.hash`,
  `ctx.id`, `ctx.self` inside a transcluded fragment equal the root document's.
- **AC6** — `ping` on a non-allow-listed target yields `null` plus a warning
  diagnostic; the preflight lists the planned ping as an approvable effect.
- **AC7** — `ping` on an allow-listed unreachable target yields `false` within
  the timeout; an ICMP send failure yields a compose error.
- **AC8** — `ping_under` returns `true` / `false` / `"unstable"` for
  all-fast / all-slow / mixed attempt outcomes (stubbed probe results).
- **AC9** — `ctx.tailnet` is `true` for a fake interface fixture with a
  `100.64.0.0/10` address and `false` without one; no real Tailscale required.
- **AC10** — `has_alias`, `has_user_function`, `has_builtin_function` return
  correct values with a stub `$SHELL` (existing `alias.rs` test pattern) for
  bash, zsh, fish, and PowerShell dialects; missing `$SHELL` returns `false`
  on Unix. Windows exercises the `pwsh` / `powershell.exe` fallback, including
  neither executable being available.
- **AC11** — `can_execute` is `true` when exactly one of its four inputs is
  `true`, also when multiple inputs are `true`, and `false` when all are `false`.
- **AC12** — `has_binary("sh")` and `has_command("sh")` return the same value;
  both names appear in the descriptor catalog.
- **AC13** — `ctx.gateway` and `ctx.gateway_v6` parse correctly from captured
  route-table fixtures for each of macOS, Linux, and Windows (six fixtures:
  both families on all three OSes); `null` when no default route and `null`
  for an on-link / point-to-point default route with no gateway address
  (**R21**); a v6 fixture with a link-local gateway yields the address with
  its `%scope` preserved.
- **AC14** — ICMP probe round-trip against `127.0.0.1` / `::1` passes on
  macOS, Linux, native Windows, and WSL2 in CI (**R23**).
- **AC15** — `as_markdown` on a string containing `{{ ctx.hostname }}` and a
  `::file` transclusion of a fixture relative to the root document yields the
  root's `ctx.hostname` and the fixture's content (same ctx snapshot, same base
  dir).
- **AC16** — A document whose content composes itself through `as_markdown`
  hits the transclusion depth limit and yields a compose error, not a hang.
- **AC17** — `package("...")` and `package_area("...")` return `""` (never
  `null`; **R26**) for a path outside the captured repository, for any path in
  a non-monorepo fixture, and for a file at the repository root of a monorepo
  fixture; both return the expected names for a file inside a package of a
  monorepo fixture.
- **AC18** — `has_agentic_cli("kimi_code")` and `has_agentic_cli("kimi")`
  return the same value; `has_agentic_cli("not_a_provider")` is a compose
  error.
- **AC19** — Drift check: regenerating the `has_agentic_cli` enum from
  `claudine/docs/providers.yaml` produces the committed artifact byte-for-byte;
  the check fails when a roster slug or alias is added, removed, or renamed
  without regeneration.
- **AC20** — With a fake interface fixture holding `127.0.0.1`, a link-local
  address, and `192.168.10.5`: `ipv4()` excludes loopback and link-local;
  `ipv4("127.0.0.0/8")` includes `127.0.0.1`; `ipv4("192.168.10")` matches by
  substring; `ipv4("192.168.10/99")` is an invalid CIDR and is treated as a
  substring match (returning nothing for that fixture).
- **AC21** — For each of the five positions in the empty-string scope table
  (inside a package under an area, inside a top-level package, in an area dir
  but not a package, at the monorepo root, outside a monorepo), `ctx.area`,
  `ctx.current_package_area`, and `ctx.current_package` equal the tabled
  values, and are strings (never `null`, never `"root"`), through BOTH the
  ambient `md compose` path and the Claudine evidence path, using a monorepo
  fixture (L1 for both legs; if the Claudine leg proves to need a real
  Claudine run it is L2 alongside AC2, and the spec must say so).
- **AC22** — Sniff: `Package.package_area` is `""` for a top-level package;
  `area_for_dir` at the repo root is `""`; and no synthetic `"root"` area value
  remains in Sniff library or CLI output (`sniff repo area` at the root
  prints nothing rather than `root`). A real area named `root` remains valid;
  test semantic projections rather than banning that word repo-wide.
- **AC23** — `when="ctx.current_package_area"` is false at the monorepo root
  and inside a top-level package, and true inside an area.
- **AC24** — On a fixture repository with 12 commits (each touching at least
  one file, with timestamps at least two days old so no `Today` / `Yesterday`
  label appears, and the same `TZ` for both sides), `ctx.recent_commits` has
  length 10 and each element is byte-equal to the corresponding commit block
  of `sniff repo recent-commits --plain` run on that fixture; outside a
  repository it is `[]`.
- **AC25** — On the AC24 fixture, `recent_commits(3)` returns the 3 newest
  commits in the same per-element format, `recent_commits(12)` returns 12, and
  `recent_commits(0)` is a compose error. A commit made between capture and
  the call (fixture-driven, mid-compose) appears in `recent_commits(1)` but not
  in `ctx.recent_commits`, proving the function is evaluated lazily.
- **AC26** — Descriptor corpus: `recent_commits` appears exactly once as a
  variable in `claudine context` and exactly once as a function in
  `claudine context --expressions`, both projected from one descriptor entry
  (the corpus test asserts a single source, not two hand-written entries).
- **AC27** — `current.branch` equals `ctx.branch` at the start of a compose;
  after a branch switch between expression evaluations (fixture-driven)
  `current.branch` reflects
  the new branch while `ctx.branch` does not. `current.recent_commits` exists
  and has the AC24 shape. A `current.*` reference resolves in frontmatter, in
  the body, and in `when=`. Two reads of `current.branch` within one expression
  agree even if the branch switches between them (Q2 memo scope). A
  `current.ctx.x` reference is an unknown-path
  error, not an alias (clean break, **R33**).
- **AC28** — Claudine lifecycle handlers that reference `current.*` and
  `current_env.*` work in every lifecycle event. After a controlled provider changes a dedicated
  test environment key between evaluations in the composing process,
  `current_env` reflects the change while `env` does not, and each lifecycle
  event observes the value current when that event evaluates (Q2).
  A subprocess cannot mutate its parent's environment; do not test freshness
  by exporting a variable in a child shell or changing fixture-owned HOME. Every migrated shipped prompt (including
  `prompts/format.md`) composes. (L2: needs a real Claudine run.)
- **AC29** — Documentation agrees with this spec (see Documentation): each
  listed file describes eager `ctx`, lazy `current`, lazy `current_env`, lazy
  parameterized functions, the pair rule, and the `current.<key>` /
  `current_env.<key>` spelling; the reserved-roots table lists `current` and
  `current_env`; and a repo-wide grep for `current.ctx.` / `current.env.` over
  code, shipped prompts, user docs, and skills (excluding `target/`,
  `node_modules/`, `.gitnexus/`, features/fixes `_completed` dirs, and the
  other in-flight feature specs listed in the migration list) returns nothing.
  Grep-based check at L1.

Additional review acceptance criteria (Q1–Q3 must be resolved before their
associated gates can pass):

- **AC30** — In-memory and URL roots compose without an invented local path or
  extra fetch; document fields follow the source-kind contract. Changing a
  local root after initial loading does not change its captured hash. Nested
  `as_markdown` and transclusion retain the original document identity.
- **AC31** — Instrumented capture shows zero history calls for `ctx.branch`,
  zero network/profile calls for unrelated values, and zero observations for
  unreached lazy references. Claudine fresh reads use supplied providers;
  unsupplied evidence emits `PartialRuntimeCapture` without ambient fallback.
- **AC32** — Passive schema validation, DMLS completion/hover, and preflight
  perform zero probes. Nested statically known effects appear in preflight;
  dynamic unapprovable shell shapes fail before execution. Frontmatter cannot
  bypass local-only reads using `as_markdown`; recursion through mixed
  function/transclusion nesting reaches one shared depth limit.
- **AC33** — ICMP fixtures cover invalid numbers, overflow, scoped IPv6,
  family mismatch, denied multi-attempt calls, and a send failure after an
  earlier success. CIDR ICMP permission does not authorize HTTP to that range.
  Return-type descriptors include denial `null` and the `"unstable"` literal.
- **AC34** — Shell fixtures cover aliases to builtins, aliases to missing
  binaries, function-versus-builtin classification, injection-shaped names,
  noisy/hanging profiles, and detached child cleanup. No test reads a real
  user profile, runs the probed command, or activates terminal/browser windows.
- **AC35** — Path fixtures distinguish sibling prefixes and deepest nested
  packages; area-only directories resolve correctly. Typed malformed-reference
  diagnostics survive; valid nonexistent descendants use lexical lookup.
- **AC36** — Two independent executions with identical frozen clocks produce
  different `ctx.id` and `ctx.sid` values (Q1). Repeat-key reads within one
  expression agree, while reads in later expressions or lifecycle events
  observe changes (Q2). Warm persistent-cache runs with `--cache-root` write no
  composed, operation, or snapshot artifact and cannot replay an earlier
  execution's identity or probe output (Q3).
- **AC37** — Empty/unborn repositories yield `[]`; repositories with empty
  commits produce fewer rendered elements as specified. Git failures in a
  known repository are reported, not silently relabeled “outside a repo.”
  Reject counts outside the supported integer range without truncation; do
  not allocate `count` slots before learning how many commits exist.

Use package-area `just test` and `just lint` for Darkmatter and affected Sniff,
Claudine, and generator surfaces; use `just test-l2` for the marked lifecycle
and real-transport cases. Keep deterministic probe coverage in L1 using
fixtures. AC14's real ICMP gate must use an explicitly configured CI capability
on all four environments, with bounded failure and actionable diagnostics;
do not substitute Internet connectivity or silently skip a required leg.

## Documentation

These files must agree with this spec at completion (AC29). Collectively they describe
eager `ctx`, lazy `current`, lazy `current_env`, lazy parameterized functions
(`recent_commits(count)` and later pairs), the pair rule (**R29**), and the
`current.<key>` / `current_env.<key>` spelling; no `current.ctx.` /
`current.env.` text remains in them.

- Claudine skill: `.claude/skills/claudine/SKILL.md`,
  `.claude/skills/claudine/lifecycle.md`,
  `.claude/skills/claudine/composition.md`. Being updated ahead of
  implementation on 2026-09-11 by the orchestrator with a "ratified,
  implementation pending" marker; the implementer removes the marker when the
  behavior lands.
- Darkmatter skill: `.claude/skills/darkmatter/compose.md` — reserved roots
  (`doc`, `ctx`, `env`, `current`, `current_env`) and the pair rule.
- Darkmatter expressions doc:
  `darkmatter/docs/topics/darkmatter-expressions.md` — the reserved-roots
  ("Namespaces") table at `:174-184` gains `current.*` and `current_env.*`.
- Claudine user docs: `claudine/docs/topics/lifecycle.md` (globals table at
  `:437`, known-roots sentence at `:813`), `claudine/docs/topics/composition.md`
  (`:797`, `:868`), `claudine/docs/topics/context/context-variables.md`
  (eager/lazy pairing of `ctx` and `current`).

Update the Darkmatter and Sniff READMEs for the public additions, the source-kind
and freshness contracts, and sentinel migration. Update root and per-area
`docs/dependencies.md` for every actual direct dependency/feature change
(including `ipnet`, BLAKE3, and any ICMP/nonce dependency). Keep DMLS descriptor
completion and passive-validation tests in sync without enabling evaluation.
Document only implemented behavior once Q1–Q3 are decided.
