# Error Structure — Mini-Design (Handleability)

> A sibling concern to [`integrated-design.md`](./integrated-design.md). That document
> structures errors so a **human reads** them well (cause-driven *rendering*). This one
> structures errors so a **program reacts** to them well (cause-driven *handling*) — by
> an API caller (Rust) or a prompt-document author (frontmatter `when:`).
>
> **Scope discipline (Ken's instruction):** this designs the *structure that makes
> handling ergonomic*, **not** the handler/dispatch mechanism itself. Where the handler
> is mentioned, it is only to justify a structural choice.

---

## 1. The question, sharpened

A handler — wherever it lives — needs to express three granularities, and Ken named all
three:

1. **Tap a pattern** — "retry any transient provider failure", "notify me on any auth
   problem". Coarse, reusable, the most valuable kind.
2. **Target a specific error** — "when the plan cap is hit, wait for the reset".
3. **Target an instance's specifics** — "when the agent forgot to set frontmatter
   *`status`* specifically, re-prompt it".

So the error must expose, simultaneously, a **coarse class**, a **stable specific
identity**, and a **typed instance payload**. The structure is the contract that lets a
handler bind to any of those layers.

---

## 2. The load-bearing constraint: handle-taxonomy and render-taxonomy must be ONE

The integrated design's central lesson is that *control flow keyed off error strings* is a
correctness bug (`is_fatal_eval_error` matches `message.starts_with(UNKNOWN_FUNCTION_PREFIX)`,
`rewrite.rs:20`). If we now build a **separate** classification layer for handling — a
parallel `match` on messages, or a second taxonomy bolted on beside the typed errors — we
rebuild that exact landmine, this time wired to user-authored `when:` clauses.

The codebase already shows the leak: `StreamExecutionSummary.error_kind` is
`Option<String>` (`stream/summary.rs:69`) even though a typed `SemanticErrorKind` exists
(`stream/semantic.rs:36`). The classification is collapsed back to a string at the summary
boundary.

**Therefore: the typed error is the single source of truth for both rendering and
handling. `err.*` is a *projection* of the typed error, never an independent parallel
structure.** Every facet a handler matches on is a method on the error, not a re-parse of
its `Display`.

---

## 3. What already exists — unify, don't reinvent

The seeds of every facet below are already in the tree, but **fragmented, stream-only, and
not exposed to handlers**:

| Existing | Shape | What it really is | Problem |
|----------|-------|-------------------|---------|
| `SemanticErrorKind` (`semantic.rs:36`) | `Configuration \| AgentNative \| ApiRemote \| Interrupted \| Unknown`, snake_case serde, stable `as_str()`, replay-default `Unknown` | a coarse **origin/category** facet for stream errors | stream-only; overlaps BadgeCategory; not on `err.*` |
| `BadgeCategory` (`badges.rs:22`) | `Auth \| Billing \| Quota \| RateLimit \| ContextPressure \| Permission \| Config`, with a **precedence** order (`Auth > Billing > Quota > RateLimit > Permission`, `badges.rs:96`) | a **domain category** facet for operator badges | a *second*, overlapping category enum |
| `BadgeSeverity` (`badges.rs`) | `Info \| Warning \| Error` | a **severity** facet | badge-only |
| `RateLimitInfo` (`summary.rs:9`) | `{ retry_after_ms: Option<u64>, reset_at: Option<DateTime<Utc>> }` | the **throttle-timing detail** Ken asked for — *"WHEN the cap will be lifted"* already exists | rate-limit-only; not generalized to "throttled" |
| `StreamExecutionSummary.error_kind` (`summary.rs:69`) | `Option<String>` | re-stringified `SemanticErrorKind` | the §2 leak in miniature |

The design is mostly **promotion and unification**: collapse the two overlapping category
enums into one, generalize `RateLimitInfo` into a "throttle" detail, reuse the
`as_str`-stability discipline (`semantic.rs:642` already tests it), and expose the result
uniformly. The `reset_at` field means the marquee use case (plan caps) needs *no new data
capture* — only a uniform place to read it.

---

## 4. Why a pure type/subtype tree is not enough (answering "my first thought")

Ken's instinct was a type/subtype hierarchy. It is *necessary but not sufficient*, because
the facets handlers care about most are **orthogonal to the domain tree**:

- "Retry anything **transient**" cuts across `Provider`, `Io`, and `Network` branches.
- "**Wait** for the limit to lift" applies to `cap.plan_limit` *and* `cap.rate_limit`,
  which sit in the same branch, but the *response* (`wait → retry`) is shared with any
  future throttle in a different branch.
- "Notify me about **the author's** mistakes but auto-recover **the agent's**" is a cut by
  *origin*, not by domain.

A strict tree forces you to encode these by enumerating leaves or duplicating sub-branches.
That is precisely the brittleness that makes handlers non-reusable.

**Resolution: keep the hierarchy, but as *one facet among several*.** A dotted `code`
(`cap.plan_limit`) gives the tree for free (prefix-match `cap.*`), while orthogonal facets
(`disposition`, `origin`) express the cross-cutting patterns a tree cannot. Hierarchy +
facets beats either alone.

---

## 5. The faceted identity model

Every handleable error exposes five facets. Three are small closed enums (stable, the
"tap a pattern" surface); one is the stable specific id; one is the open typed payload.

```rust
pub trait Diagnostic: BlockError {        // supertrait: render + classify share one chain (§6)
    fn category(&self) -> Category;        // closed enum — coarse domain tap
    fn code(&self) -> &'static str;        // stable dotted id, e.g. "cap.plan_limit" — the contract
    fn disposition(&self) -> Disposition;  // closed enum — generic-strategy tap (reuse enabler)
    fn origin(&self) -> Origin;            // closed enum — who must correct it
    fn detail(&self) -> ErrorDetail;       // typed, serde → err.detail.* — instance specifics
    fn severity(&self) -> Severity { /* default from disposition */ }
}
```

### 5.1 `category` — the coarse domain tap (unifies the two existing enums)

A single closed enum, superseding `SemanticErrorKind` ∪ `BadgeCategory`:

`Auth · Cap · Timeout · Provider · Composition · Document · Vcs · Io · Config · Usage ·
Runaway · Internal`

- `Cap` generalizes `Quota`/`Billing`/`RateLimit` (anything usage-bounded that lifts).
- `Composition`/`Document`/`Vcs` are new coverage the stream-only enums never had — and
  exactly where the bespoke errors and the integrated-design errors live.

### 5.2 `code` — the stable specific id (and the hierarchy)

A dotted `&'static str`, always prefixed by its category slug: `cap.plan_limit`,
`auth.invalid`, `timeout.step_silence`, `document.missing_frontmatter`,
`vcs.unexpected_dirty_files`, `composition.invalid_file_reference`. This *is* Ken's
type/subtype tree, expressed as data: `err.category == "cap"` ≡ `err.code` starts with
`"cap."`. The dotted code is the **public API contract** (§10).

### 5.3 `disposition` — the generic-strategy tap (the reuse enabler, subsumes fatality)

The single most valuable facet for reusable handlers — it answers *"what class of response
could resolve this?"*:

| Disposition | Meaning | Canonical response |
|-------------|---------|--------------------|
| `Transient` | same action may succeed now | retry now |
| `Throttled` | will succeed *later*, at a known/estimable time | wait until `reset_at`, then retry |
| `Correctable` | needs a *different* action; won't self-resolve | re-prompt agent / author fixes doc |
| `NeedsInput` | needs interactive human input | prompt the human |
| `Unrecoverable` | no generic resolution (bug, hard rejection) | stop / surface |

**This subsumes the integrated design's `is_authoring_fatal()`.** "Fatal in lenient compose
mode" is just "this disposition halts composition." An authoring error is
`Correctable`+`origin=Author`; the current inconsistency (unknown-function halts,
missing-file warns) becomes *visible* as two `Correctable`+`Author` errors with different
halting behavior — which is exactly the drift the integrated design's characterization gate
exists to pin. The bespoke predicate folds into the taxonomy.

### 5.4 `origin` — who must correct it

`Provider` (the wrapped agent / its platform) · `Author` (the prompt-doc author) · `Caller`
(API misuse) · `Environment` (host/io/net) · `Internal` (our bug).

**`disposition × origin` is the strategy matrix** that makes patterns reusable without a
handler design:

| | origin=Provider/Agent | origin=Author | origin=Caller | origin=Environment |
|---|---|---|---|---|
| Transient | retry now | (rare) | — | retry now |
| Throttled | wait→retry | — | — | wait→retry |
| Correctable | re-prompt agent | fix the document | fix the API call | fix the env/creds |
| Unrecoverable | stop + report | stop + report | stop + report | stop + report |

A handler that says "retry `Transient`+`Provider`, wait-and-retry `Throttled`, surface
everything else" is *fully reusable across every domain* — impossible with a domain tree
alone.

### 5.5 `detail` — the typed instance payload (where the specifics live)

Per-code typed payload, serde-serialized into the `err.detail.*` namespace. Crucially,
**the detail payload is the *same structured data the rendering work already captures*** —
no second copy:

- `composition.invalid_file_reference` → **`FileReferenceDiagnostic`** (the integrated
  design's struct *is* this detail) → `err.detail.reference`, `.kind`, `.suggestions`.
- `cap.plan_limit` → generalized `RateLimitInfo` → `err.detail.reset_at`, `.retry_after_ms`,
  `.limit_kind`, `.scope`, `.provider`.
- `document.missing_frontmatter` → `{ doc, property }`.
- `vcs.unexpected_dirty_files` → `{ scope: Option<Scope>, files: Vec<Path> }`.

Representation choice (open, §12): a typed-enum `ErrorDetail` is exhaustive but couples the
projection to every code; a **serde→Value map** (how `ctx.*` already works) decouples it and
matches the expression engine's native value model. Recommendation: author detail as a typed
struct per code (honest construction), project via serde into `err.detail.*` (uniform
surface). Stable field names = contract.

---

## 6. Delegation symmetry: the render-leaf and the handle-leaf are the same

The integrated design (§9) already defines how a transparent wrapper
(`MarkdownError::Interpolation`) delegates *rendering* to its meaningful cause (the
`FileReference`). **Classification delegates by the identical rule:** a transparent wrapper
returns its cause's `category/code/disposition/origin/detail`; a layer that *deliberately
classifies* (e.g. the provider layer deciding "this is a plan cap, not a raw stream error")
owns its facets and does **not** delegate.

Consequence: the deepest *meaningful* cause is simultaneously what renders and what
handlers bind to. There is **one** cause-chain walk, not two. The `err` exposed to handlers
is that meaningful cause; `err.cause.*` exposes the next layer down (same shape) for the
rare handler that needs to drill (e.g. "a cap whose underlying stream error was X").

This is why `Diagnostic: BlockError` is a supertrait, not a sibling registry — render and
handle resolve through the same `as_block_error`/deepest-cause machinery.

---

## 7. The `err.*` projection — the author/caller-facing surface

This **formalizes and extends** the existing late-binding `err.{kind, variant, msg}`
(today's lifecycle handle) into the faceted shape:

| `err.*` | Source facet | Use |
|---------|-------------|-----|
| `err.category` | `category` | coarse pattern match |
| `err.code` | `code` | specific match / prefix match |
| `err.disposition` | `disposition` | strategy pattern match |
| `err.origin` | `origin` | who-fixes pattern match |
| `err.detail.*` | `detail` | instance-specific match |
| `err.msg` | `Display` | human text — **discouraged for matching** |
| `err.cause.*` | delegated cause | drill deeper |
| `err.retry_after`, `err.reset_at` | promoted detail | ergonomic throttle access |
| `err.is_transient`, `err.is_throttled`, … | promoted disposition | ergonomic predicates |

Ken's three granularities map directly (illustrative — *not* a handler design):

```yaml
# 1. tap a pattern (reusable)
failure: { when: "err.disposition == 'throttled'", … }
failure: { when: "err.origin == 'author'", … }

# 2. target a specific error
failure: { when: "err.code == 'cap.plan_limit'", … }

# 3. target an instance's specifics
failure: { when: "err.code == 'document.missing_frontmatter' && err.detail.property == 'status'", … }
```

`err.{kind,variant}` can remain as deprecated aliases for `{category,code}` during
migration, but new contract = the faceted names.

---

## 8. The error universe — representative catalog

Mapping Ken's named set + bespoke set + the integrated-design errors into the model.
*Representative, not exhaustive.*

| Error | category | code | disposition | origin | key `detail` fields |
|-------|----------|------|-------------|--------|---------------------|
| Agent timeout (silence) | Timeout | `timeout.step_silence` | Transient | Provider | `kind`, `elapsed_ms`, `limit_ms` |
| Agent timeout (wall) | Timeout | `timeout.wall_clock` | Transient | Provider | `elapsed_ms`, `limit_ms` |
| Plan / usage cap | Cap | `cap.plan_limit` | **Throttled** | Provider | **`reset_at`**, `retry_after_ms`, `limit_kind`, `scope`, `provider` |
| Rate limit | Cap | `cap.rate_limit` | Throttled | Provider | `reset_at`, `retry_after_ms`, `provider` |
| Invalid agent auth | Auth | `auth.invalid` | Correctable | Provider/Env | `provider`, `reason` (expired/missing/rejected) |
| Provider crash / nonzero exit | Provider | `provider.exited` | Transient¹ | Provider | `exit_code`, `signal` |
| Runaway (volume/repetition) | Runaway | `runaway.volume` / `.repetition` | Unrecoverable² | Provider | `guard`, `measure` |
| Interrupted (Ctrl-C) | Provider | `provider.interrupted` | Unrecoverable | Caller | — |
| **MissingFrontmatter** | Document | `document.missing_frontmatter` | Correctable | **Agent** | `doc`, `property` |
| **InvalidFrontmatter** | Document | `document.invalid_frontmatter` | Correctable | Agent | `doc`, `property`, `problems[]` |
| **DocumentEmpty** | Document | `document.empty` | Correctable | Agent | `doc` |
| **UnexpectedDirtyFiles** | Vcs | `vcs.unexpected_dirty_files` | Correctable | Agent | `scope?`, `files[]` |
| **UnexpectedCommits** | Vcs | `vcs.unexpected_commits` | Correctable | Agent | `commits[]` |
| **MissingDirtyFiles** | Vcs | `vcs.missing_dirty_files` | Correctable | Agent | `scope?` |
| Invalid file reference | Composition | `composition.invalid_file_reference` | Correctable | Author | `FileReferenceDiagnostic` |
| Schema validation | Composition | `composition.schema_validation` | Correctable | Author | `doc`, `problems[]`, `pointer_paths[]` |
| Unknown expression fn | Composition | `composition.unknown_function` | Correctable | Author | `name`, `suggestions[]` |
| Caller arg misuse | Usage | `usage.invalid_argument` | Correctable | Caller | `argument`, `expected` |

¹ A crash may be transient (retry) or unrecoverable depending on cause; the classifying
layer decides per instance. ² Runaway maps to `ProcessTermination::Aborted` and must
**not** be retried (it would reproduce the runaway) — `Unrecoverable` encodes that as data,
preventing a generic "retry transient" handler from looping it.

---

## 9. The expectation sub-family (postconditions) — a coherent shape

The six bespoke errors are not "the engine broke" — they are **postcondition assertions
about the agent's work product**, evaluated by a check *after* the agent runs. They form a
family with a shared shape worth a common struct:

```rust
pub struct ExpectationFailure {
    pub expectation: &'static str,   // "frontmatter_set" | "body_filled" | "files_committed" | …
    pub subject: ExpectationSubject, // Document(path) | Property(path, key) | GitScope(scope?) | Repo
    pub observed: ObservedState,     // what was actually found
}
```

Properties shared across all six: `origin = Agent` (the agent failed its instruction, even
though *we* detected it), `disposition = Correctable` (re-prompting with the failure as
feedback is the natural fix). Distinguishing them as a family matters because a single
reusable handler — "on any `Correctable`+`Agent` expectation failure, re-prompt the agent
with `err` as context" — covers all six and any future postcondition, *without* enumerating
codes. This is the payoff of facets over a tree, demonstrated on Ken's own list.

(The *checks* that raise these — when/where to assert dirty-files, empty-body, etc. — are
the handling layer and are **out of scope** here. We only specify the *shape* they raise.)

---

## 10. Stability & discoverability — the new contract

The render redesign has no API-stability constraint: `Display` strings can change freely.
**Handling inverts that.** The moment `err.code == "document.missing_frontmatter"` appears
in an author's frontmatter, that string — and every `category`, `disposition`, `origin`
value, and `detail` field name — is a **public, versioned contract**.

Requirements this imposes:

- **Single source of truth.** A registry of codes + their facets + detail schema, modeled
  on the existing `Described` catalog (`catalog/mod.rs`) and the descriptor catalogs that
  back `claudine context`. One place defines every code.
- **Additive evolution.** New codes/fields may be added; existing ones must not change
  meaning. Renaming/removing a code is a breaking change.
- **Discoverability.** An introspection surface mirroring `claudine context` — e.g.
  `claudine errors` / `md errors` listing every code with its category, disposition,
  origin, and detail fields — so authors discover what to match on without reading source.
- **Stability tests.** Reuse the discipline already present at `semantic.rs:642`
  (`error_kind_as_str_is_stable`) for every facet's string projection.

---

## 11. Recovery inputs the structure must provide

Two recovery behaviors are confirmed requirements. Each constrains what the *error
structure* must carry — the handler/daemon that consumes them is out of scope.

### 11.1 Absolute-time scheduling ("resume at this absolute time")

Recovery for a `throttled` error is "act again **at** the time the cap lifts," not "after a
relative delay." This separates **when** (now / after-delay / at-absolute-time / deferred)
from **what** (retry / resume / proxy) — and today's control actions only model a *relative*
`delay`. So the structure must expose the lift time as an **absolute timestamp**:
`err.detail.reset_at` (RFC 3339), already present (`RateLimitInfo.reset_at`). A handler
consumes it directly, e.g. `until: "{{ err.reset_at }}"`.

The structural consequence beyond carrying the field: the typed error must **survive process
exit**. A far-future reset (a plan cap hours away) cannot be served by blocking the worker;
it must be persisted and re-entered by the rendezvous daemon at the wake time. That requires
the whole typed error — `code` + `detail` — to round-trip through serialization, which the
serde-projected detail and stable codes already guarantee. "Resume at absolute time" is
therefore *persist the typed error → schedule a wake at `reset_at` → re-enter*. **Designing
that scheduler is out of scope; carrying a serializable, absolute `reset_at` is the
structural requirement.**

### 11.2 Resume corrective input — known-now vs determined-later

A `resume` delivers a follow-up message to the live agent session. The corrective input has
two sources, which impose different requirements on the structure:

- **Known-now (synchronous).** At error time the corrective message is mechanically
  derivable — *"you forgot to set the `ready` frontmatter property."* This is fully
  expressible **today** (once errors are typed): the `document.*`/`vcs.*` detail carries
  exactly the fields a templated resume message needs —
  `resume: "Set '{{ err.detail.property }}' in {{ err.detail.doc }} as the prompt requires."`
  **Structural requirement:** every expectation error's `detail` must be *sufficient to
  author its own corrective message* via `err.detail.*`. This is a constraint on the detail
  schema (e.g. `document.missing_frontmatter` must carry both `doc` **and** `property`, not a
  pre-baked message string) — and a concrete reason the expectation sub-family (§9) earns its
  typed fields.

- **Determined-later (asynchronous).** When the corrective action *cannot* be computed at
  error time, the error is handed to the rendezvous daemon to be resolved later —
  **human-in-the-loop ~95% of the time.** This is the true home of the `needs_input`
  disposition: not an inline interactive prompt, but a *deferred* handoff where a human
  decides the resume message (or an alternate action) out-of-band. **Structural requirement:**
  the persisted typed error must carry enough context for a human to decide cold — the same
  serializable `code` + `detail` + source/scope, surviving exit. This is future,
  daemon-dependent work; the structure's only job is to **not lose** the information the human
  will need.

**Net:** both behaviors are satisfied by two structural properties already in the model —
*(a)* an absolute, serializable `reset_at`, and *(b)* a `detail` schema rich enough to either
author a corrective message now **or** hand a human enough to author one later. No new
error-structure machinery is required; these are constraints on `detail` *completeness* and
*serializability*, not new types. The `defer`/rendezvous backend remains
unimplemented (`LifecycleDeferNotImplemented`) and out of scope here.

---

## 12. Non-goals & open questions

**Non-goals:** the handler/dispatch mechanism; the matching DSL beyond the `err.*` shape it
binds to; the postcondition *checks* that raise expectation errors; the `defer` scheduler.

**Open questions:**

- **Detail representation** — typed-enum `ErrorDetail` vs serde→Value map. Recommend the
  latter for engine uniformity; confirm against `when:` evaluation needs.
- **Final category list & granularity** — the §5.1 set is a proposal; needs ratification
  (e.g. is `Billing` distinct from `Cap`? is `ContextPressure` its own category?).
- **`code` scoping** — globally unique vs category-scoped (dotted code makes this moot if we
  mandate the category prefix).
- **Precedence when multiple classifications apply** — `BadgeCategory` already encodes a
  precedence (`Auth > Billing > Quota > RateLimit > Permission`, `badges.rs:96`); the
  unified taxonomy likely needs an equivalent for the rare multi-class instance.
- **Disposition of provider crashes** — Transient vs Unrecoverable is per-instance; does the
  classifier need provider-specific knowledge to decide, and where does that live?
- **`err.cause.*` depth** — how many levels to project before it becomes noise.

---

## 13. Summary

The handling structure is **not a new system** — it is the same typed errors the rendering
redesign produces, exposed through a `Diagnostic` facet set (`category`, `code`,
`disposition`, `origin`, `detail`) that (a) unifies the three partial taxonomies already in
the tree, (b) gives Ken's type/subtype hierarchy via the dotted `code` while adding the
*orthogonal* facets a tree cannot express, and (c) projects to a stable `err.*` surface that
lets a handler tap a pattern, target a code, or target an instance. The single most
important rule is §2: **one taxonomy, one source of truth, projected — never a parallel
string-matched classification, which would rebuild the very control-flow-by-string bug the
whole effort exists to remove.**
