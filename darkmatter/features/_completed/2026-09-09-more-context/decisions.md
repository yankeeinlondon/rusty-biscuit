# More Context — Implementation Decision Record

Recorded 2026-09-16 during Phase 1 of `plan.md`. This file fixes the
architecture boundaries later phases build on. It records decisions, not
behavior that already exists; where a decision adopts an open-question
recommendation from `spec.md`, the spec section is updated to match and the
decision is listed for Ken's confirmation in the plan's `human_review_items`.

## D1 — Open-question outcomes (Q1–Q3)

Confirmed by Ken on 2026-09-17 as spec rulings R34 (Q1), R35 (Q2), and
R36 (Q3). R37, ruled the same day, settles the AC29 migration guard as an
allowlist test rather than a literal zero-match grep.

| Question | Adopted outcome | Spec sections updated |
|---|---|---|
| Q1 execution identity | Per-execution 128-bit random nonce shared by `ctx.id` and `ctx.sid`; typed entropy-failure compose error; versioned, length-prefixed tuple | Document identity, Q1, AC4, AC36 |
| Q2 memo scope | Memoize per expression evaluation, per key; every lifecycle event evaluates in fresh scopes | `current`/`current_env` required behavior, Q2, AC27, AC28, AC36 |
| Q3 cache prerequisite | Cache-disable portion of `fixes/2026-09-16-content-policy-no-cache` implemented in Phase 1: no local artifact is persisted; `--cache-root` backs only raw remote-URL bodies | Caching, Q3, AC36 |

### Identity tuple (Q1)

Version tag `darkmatter.ctx.id/v1`, then each field as a little-endian `u64`
byte length followed by its bytes, in this order:

1. root source bytes (UTF-8 as loaded at request creation)
2. `ctx.timestamp_ms` as a little-endian `i64` (8 bytes, still length-prefixed)
3. hostname (`""` when unavailable)
4. repository name (`""` outside a repository)
5. execution nonce (16 bytes from the OS CSPRNG)

`ctx.id` is the `biscuit-hash` xxHash of the encoded tuple in its established
output convention; `ctx.sid` is full lowercase-hex BLAKE3 of the same bytes.
The nonce is drawn once per root compose request and shared by every fragment,
transclusion, and `as_markdown` child. Entropy failure is a typed compose error
(never a silent fallback to a zero or time-derived nonce). Test vectors freeze
the encoding by injecting a nonce through a crate-internal seam; production code
has no public nonce override.

## D2 — Root-source ownership

The request, not a capture group, owns the root source. When the root compose
request is created it retains the bytes it loaded plus
`ComposeSource::{File, Url, Unknown}` metadata (canonical native path and
optional modification time for `File`). The Document group projects from that
retained value; it never reopens a file or refetches a URL. Child pipelines
(transclusion and `as_markdown`) receive the root's document values by
reference through the request epoch and never substitute their own source.

## D3 — Fixed resolution and repository anchors

`FileResolutionContext` and the captured repository observation are captured
once per request in `ComposeOptions` and are immutable for the request,
including for `current.*`. Freshness applies only to mutable facts (branch,
dirty files, recent history, environment). No downstream resolver, lazy
provider, or function re-derives CWD, the repository root, or package topology.

## D4 — Eager requirements versus lazy capabilities

Two planning products stay distinct:

- **Eager requirements** — the existing demand-driven `ContextRequirements`
  (`ctx.*` groups, refined below group level where cost differs, e.g. Git
  history demanded only by `ctx.recent_commits`).
- **Deferred capabilities** — the set of `current.*` keys, `current_env`, and
  lazy function dependencies a document can reach. Planning records them as
  metadata; nothing is observed until evaluation reaches the reference.

A `current.*` reference never adds an eager requirement, and an eager
requirement never satisfies a `current.*` read.

## D5 — Refresh-provider interface

A request-owned trait object (working name `CurrentProvider`) carried by
`ComposeOptions` next to the captured context:

- one method refreshes exactly one descriptor key against the retained launch
  roots and returns the key's schema-shaped value, or a
  `PartialRuntimeCapture` diagnostic when the capability was not supplied;
- ambient `md compose` installs a provider backed by Sniff against the fixed
  anchors of D3; Claudine installs an invocation-owned provider built from its
  launch evidence authority;
- a missing capability fails closed — no provider ever falls back to ambient
  discovery.

Memoization (Q2) lives in the per-expression evaluation scope, not in the
provider, so sharing one provider across child pipelines shares no stale
observation.

## D6 — Descriptor-pair ownership

One descriptor entry owns a variable/function pair (R29). The entry defines the
shared semantics and output format and projects exactly one `ctx.<name>`
variable and one `<name>(args)` function into the existing schema and
expression catalogs. Phase 4 owns this representation; no other phase edits the
descriptor registries independently. `claudine context` and
`claudine context --expressions` read the projections and never define their
own entries. The `has_agentic_cli` name set is a generated Darkmatter artifact
owned by `claudine-gen`.

## D7 — ICMP policy separation

ICMP consent is a separate grant set on `ComposeOptions`, parsed from the same
`--allow-host` input but holding only exact IP addresses and strict CIDRs
(with explicit IPv6 scope where given). It is never consulted by, and never
widens, `biscuit_file::FetchPolicy`; an HTTP host grant never authorizes ICMP
and a CIDR grant never authorizes HTTP. ICMP results never enter any persistent
store (Q3).

## D8 — Shared recursion budget

`as_markdown` nesting and `::file` transclusion consume one budget: the existing
`TransclusionRuntime` stack bounded by `max_transclusion_depth`. Every
`as_markdown` call pushes a node onto the same ancestry used for cycle
detection, so mixed function/transclusion recursion reaches the same typed
depth-limit error. No second counter is introduced.
