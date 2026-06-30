# Error Catalog — Proposed Locked-Down Taxonomy (for ratification)

> A concrete first pass at the **public contract**: the final facet enums, the dotted
> `code` list, and the `detail` schemas. Implements the model in
> [`error-structure.md`](./error-structure.md). Once authors match on these in `when:`
> clauses, **codes / category / disposition / origin values / detail field names are
> versioned API** — so this list is meant to be argued over and *locked* before phase 7 of
> [`integrated-design.md`](./integrated-design.md).
>
> **Status: RATIFIED.** All §7 decisions are confirmed; the facet enums and code catalog
> below are the locked contract. Evolution from here is **additive-only** (new codes /
> detail fields are non-breaking; renames and removals are breaking).

---

## 1. The four facet enums (closed sets)

### `Category` — coarse domain (12)

> **Strategy** (_what is this about?_)

| category | represents |
|----------|------------|
| `auth` | Identity and authorization with the provider — the agent run cannot prove *who it is* or *that it is allowed*. Bad/expired/missing credentials, or authenticated-but-not-permitted. |
| `cap` | Usage and account limits imposed by the provider — the agent run is *allowed* but currently *capped*. Rate limits, plan/usage quotas (with *when the cap lifts*), and hard billing stops. |
| `timeout` | The agent run exceeded a time budget — stream silence (`step_timeout`) or wall-clock (`timeout`). The work may not be wrong, just too slow. |
| `provider` | The agent run itself — launching, executing, streaming, and the model's process-level behavior. Missing/unspawnable CLI, nonzero exit, interruption, stream-protocol errors, context-window pressure. The catch-all for "the run, as infrastructure." |
| `composition` | The author's prompt document is malformed — Darkmatter compose/expression/schema failures in the *input* the author wrote. Invalid file references, unknown functions, bad expressions, frontmatter that fails the prompt's own `$schema`. |
| `document` | A document the agent *produced or modified* fails a postcondition the author/caller asserted — missing/invalid frontmatter, empty body. The agent's *output* didn't meet expectations (an expectation sub-family, §4). |
| `vcs` | Git-state postconditions about the agent's work — files that should/shouldn't be committed or modified. Unexpected dirty files, unexpected commits, missing-expected changes. (The other half of the expectation sub-family.) |
| `io` | Filesystem and network plumbing beneath everything else — reads, writes, permissions, and outbound network/messaging. Environmental failures not specific to a higher domain. |
| `config` | Claudine/user configuration is invalid — the *settings*, not a prompt document. Bad config fields, malformed MCP catalog/server entries. |
| `usage` | Misuse of the Rust API by an integrating caller — bad arguments, unsupported operations. Distinct from `config` (settings) and `composition` (prompt authoring). |
| `runaway` | A content-guard tripped — the child flooded rather than failed. Volume cap, group-cycle repetition, or a user `exit_expression` match. Always `unrecoverable` (a retry reproduces the flood). |
| `internal` | A bug or unexpected invariant in Darkmatter/Claudine itself — *our* fault, not the user's. Unclassified errors land here. |

Each category is the dotted prefix of its codes, so `category` is always derivable from
`code` (`err.category == "cap"` ≡ `err.code` starts with `"cap."`). This *is* the
type/subtype hierarchy, as data.

### `Disposition` — generic-strategy facet (5)

> **Strategy** (_what do I do about it?_)

| Value | Meaning | Canonical response |
|-------|---------|--------------------|
| `transient` | same action may succeed if retried now | retry now |
| `throttled` | will succeed *later*, at a known/estimable time | wait for `reset_at`, then retry |
| `correctable` | needs a *different* action; won't self-resolve | re-prompt agent / fix doc / fix creds |
| `needs_input` | needs human input to decide the fix | prompt inline if interactive; else defer to rendezvous (human-in-the-loop) resolution |
| `unrecoverable` | no generic resolution | stop + surface |

### `Origin` — who remediates (5)

| Value | Meaning |
|-------|---------|
| `provider` | the agent run — platform, API, *and* model behavior (see §7.1) |
| `author` | the prompt-document author |
| `caller` | the Rust API integrator |
| `environment` | host / filesystem / network / credentials |
| `internal` | a bug in Darkmatter/Claudine |

### `Severity` — operator-facing (3, reuse existing `BadgeSeverity`)

`info · warning · error`. Default derivable from disposition (`transient`/`throttled` →
`warning`; `unrecoverable` → `error`; etc.); overridable per code.

---

## 2. Detail conventions

`detail` is the **typed, per-code instance payload** — the part of an error that varies
from occurrence to occurrence (which file, which property, when the cap lifts). `category`/
`code`/`disposition`/`origin` let a handler decide *what kind* of problem it is; `detail`
lets it act on *this specific* problem.

### 2.1 One payload, two consumers (render = handle)

The same `detail` struct feeds **both** the cause-driven renderer **and** the `err.detail.*`
handler surface — captured once at the throw site, never duplicated. This is the
load-bearing reason render and handle share a taxonomy (integrated-design §5.5):

| `detail` field | drives rendering | drives handling |
|----------------|------------------|-----------------|
| `FileReferenceDiagnostic.suggestions` | the "Did you mean `…`?" hint | `when: "!is_empty(err.detail.suggestions)"` |
| `FileReferenceDiagnostic.reference` + `base_dir` | the auto-linked path in the body | `when: "err.detail.reference starts_with 'features/'"` |
| `cap.plan_limit.reset_at` | "limit resets at HH:MM" in the block | `defer_until: "{{ err.reset_at }}"` |
| `document.missing_frontmatter.property` | "property `status` was not set" | `when: "err.detail.property == 'status'"` |

If a field is worth showing a human, it is worth exposing to a handler — so the rule is
*one struct, both jobs*.

### 2.2 Construction — typed at the throw site, never reparsed

`detail` is built from data **already in scope** where the error is raised, as a typed
struct — never reconstructed by parsing a `Display` string later. `resolve_arg` already
holds `base_dir`/`fallback_dir`; the cap classifier already holds `RateLimitInfo`. The
struct just keeps them. This is the same anti-flattening discipline as the rest of the
design, applied to the instance payload.

### 2.3 Projection to `err.detail.*`

A `detail` struct is projected via serde into the expression engine's value model — exactly
how `ctx.*` is exposed today. Struct fields become `err.detail.<field>`; a nested struct
flattens to *its* fields (so `FileReferenceDiagnostic`'s fields are reached directly as
`err.detail.reference`, not `err.detail.diagnostic.reference`). What an author sees:

```text
# code = composition.invalid_file_reference
err.detail.reference     "features/2026-06-21-opencode-log-fix/spec.md"
err.detail.kind          "not_found"
err.detail.base_dir      "/repo/claudine/features/2026-06-28-real-errors"
err.detail.suggestions   ["features/2026-06-21-opencode-log-fix/spec.md"]

# code = cap.plan_limit
err.detail.provider      "claude"
err.detail.reset_at      "2026-06-28T17:30:00Z"
err.detail.retry_after_ms 5400000
err.detail.limit_kind    "plan"
err.detail.scope         "account"

# code = document.missing_frontmatter
err.detail.doc           "features/2026-06-28-real-errors/spec.md"
err.detail.property      "iteration"
```

### 2.4 Reading `detail` in `when:` (the author surface)

Detail fields are ordinary values in the expression engine, so the existing operators and
functions apply — including its null propagation and collection helpers:

```yaml
# scalar equality — target one instance
failure: { when: "err.code == 'document.missing_frontmatter' && err.detail.property == 'status'" }

# optional field — `scope` absent means "whole repo" (null sentinel, §2.7)
failure: { when: "err.code == 'vcs.unexpected_dirty_files' && err.detail.scope == null" }

# collection — "the agent touched at least one file it shouldn't have"
failure: { when: "err.code == 'vcs.unexpected_commits' && !is_empty(err.detail.commits)" }

# string shape — only react to references under a known tree
failure: { when: "err.detail.reference starts_with 'features/'" }
```

Because `err.detail.*` is the *typed* payload, none of these match against human prose —
they read fields, the way the renderer does.

### 2.5 Shared field vocabulary (the cross-code ergonomic)

The single most important convention: **the same concept uses the same field name across
every code that carries it.** This is what makes a handler reusable across codes — a rule
keyed on `err.detail.provider` works for `auth.invalid`, every `cap.*`, every `timeout.*`,
and `provider.*` without rewriting per code.

| field | type (serialized) | meaning | appears in |
|-------|-------------------|---------|-----------|
| `provider` | string | provider slug (`claude`, `codex`, …) | `auth.*`, `cap.*`, `timeout.*`, `provider.*` |
| `doc` | path string | the document the error concerns | `document.*`, some `composition.*` |
| `property` | string | a frontmatter key | `document.missing_frontmatter`, `document.invalid_frontmatter` |
| `scope` | optional scope | a file/dir subset; `null` = whole repo | `vcs.*` |
| `reset_at` | RFC 3339 string | when a cap lifts | `cap.rate_limit`, `cap.plan_limit` |
| `retry_after_ms` | u64 | suggested wait before retry | `cap.*` |
| `problems` | string[] | validation problem messages | `*.schema_validation`, `document.invalid_frontmatter` |
| `path` | path string | a filesystem path | `io.*` |
| `reference` | string | a raw, unresolved reference | `composition.invalid_file_reference` |
| `suggestions` | string[] | did-you-mean candidates | `composition.*` |
| `message` | string | last-mile human detail (matching discouraged) | `internal.*`, catch-alls |

New codes must reuse an existing name when the concept already exists; only genuinely new
concepts get new names. A divergent spelling (`file` vs `path`, `prop` vs `property`) is a
review smell.

### 2.6 Promoted conveniences

A few high-traffic values are hoisted to the top of `err.*` so common handlers stay terse:
`err.reset_at`, `err.retry_after_ms` (from `cap.*` detail) and the predicate sugar
`err.is_transient` / `err.is_throttled` / `err.is_correctable` (from `disposition`). These
are **sugar only** — `err.detail.*` and `err.disposition` remain canonical, and a promoted
field is `null`/`false` when the active code doesn't carry it.

### 2.7 Types, serialization & optionality

- **Comparable types only.** Detail fields must serialize to values the expression engine
  can compare: strings, numbers, booleans, arrays, and stringized scalars. Timestamps are
  RFC 3339 strings (string-comparable and parseable by the engine's date functions);
  durations are integer milliseconds (`*_ms`); enums are snake_case strings (`kind: "not_found"`).
- **Paths are strings** in `detail` (and rendered as OSC8 links by the block builder, §8 of
  integrated-design — the author never hand-links).
- **Optional = `null`.** An absent optional field projects to `null`, consistent with the
  existing schema "null sentinel for absent" convention, so `err.detail.scope == null` is
  the idiomatic "no scope / whole repo" test. Handlers rely on null propagation: reading a
  field absent for the current code yields `null`, not an error.

### 2.8 Stability & discoverability

- **Additive is safe; rename/remove is breaking.** Adding a new `detail` field to a code is
  non-breaking (old `when:` clauses keep working); renaming or removing one breaks any
  document matching it. Same rule as `code` itself (§7.8).
- **Discoverable.** A `claudine errors` introspection surface (mirroring `claudine context`)
  lists every code with its `detail` schema, so authors learn what they can match on without
  reading source — and so the contract has a single rendered source of truth.

---

## 3. The code catalog (grouped by category)

> Columns: **code** · **disp** (disposition) · **origin** · **detail fields** · **subsumes / raised where**.
> `disp` abbreviations: T=transient, Th=throttled, C=correctable, NI=needs_input, U=unrecoverable.

### auth — identity / authorization

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `auth.invalid` | C | provider | `provider`, `reason` (`missing`\|`expired`\|`rejected`) | "Invalid Agent Auth"; `BadgeCategory::Auth`; stream auth failures |
| `auth.permission` | C | provider | `provider`, `action` | `BadgeCategory::Permission` (authenticated but not permitted) |

### cap — usage / account limits

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `cap.rate_limit` | **Th** | provider | `provider`, `reset_at`, `retry_after_ms`, `scope` | `BadgeCategory::RateLimit`; `RateLimitInfo` |
| `cap.plan_limit` | **Th** | provider | `provider`, `reset_at`, `retry_after_ms`, `limit_kind` (`plan`\|`usage`), `scope` (`account`\|`org`) | Ken's "Agent Plan Caps" (incl. *when it lifts*); `BadgeCategory::Quota` |
| `cap.billing` | C | provider | `provider`, `reason` | `BadgeCategory::Billing` (hard account problem — does *not* auto-lift) |

### timeout

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `timeout.step_silence` | T | provider | `elapsed_ms`, `limit_ms` | Ken's "Agent Timeouts"; `step_timeout` (stream silence) |
| `timeout.wall_clock` | T | provider | `elapsed_ms`, `limit_ms` | opt-in wall-clock `timeout` |

### provider — the agent run (platform + behavior)

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `provider.unavailable` | C | environment | `provider`, `path?` | `ProviderNotAvailable`; CLI not installed |
| `provider.launch_failed` | C | environment | `provider` | spawn failure |
| `provider.exited` | T | provider | `provider`, `exit_code`, `signal?` | nonzero exit; `SemanticErrorKind::AgentNative` |
| `provider.interrupted` | U | caller | `signal` | Ctrl-C / SIGTERM; `SemanticErrorKind::Interrupted` |
| `provider.stream_error` | T | provider | `provider`, `raw?` | OpenCode/stream protocol/parse errors; `SemanticErrorKind::ApiRemote` |
| `provider.context_pressure` | C | provider | `provider` | `BadgeCategory::ContextPressure` (severity `warning`) |

### composition — author's prompt document (Darkmatter)

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `composition.invalid_file_reference` | C | author | **`FileReferenceDiagnostic`** (`reference`, `kind`, `base_dir`, `suggestions`) | the spec's example; `frontmatter()`/`absolute()`/`relative()` |
| `composition.unknown_function` | C | author | `name`, `suggestions` | `UNKNOWN_FUNCTION_PREFIX` path |
| `composition.expression_invalid` | C | author | `expression`, `message` | parse / arity / arg-type |
| `composition.schema_load` | C | author | `source_path`, `message` | `SchemaLoad` |
| `composition.schema_validation` | C | author | `source_path`, `problems`, `pointer_paths` | `SchemaValidation` (prompt fails *its own* `$schema`) |
| `composition.missing_properties` | C | author | `missing`, `pointer_paths` | `MissingProperties` (compose-time) |
| `composition.frontmatter_parse` | C | author | `source_path` | YAML syntax error |
| `composition.lifecycle_invalid` | C | author | `property`, `message` | lifecycle action grammar errors |
| `composition.shell_expansion` | C | author | `command` | `$(...)` expansion failure |
| `composition.failed` | C | author | `source_path` | catch-all body compose failure (`ComposeFailed`) |

### document — agent's output postconditions (the expectation sub-family, §4)

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `document.missing_frontmatter` | C | provider | `doc`, `property` | Ken #1 |
| `document.invalid_frontmatter` | C | provider | `doc`, `property`, `problems` | Ken #2 (agent set it, fails schema) |
| `document.empty` | C | provider | `doc` | Ken #3 |

### vcs — git-state postconditions (expectation sub-family)

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `vcs.unexpected_dirty_files` | C | provider | `scope?`, `files` | Ken #4 |
| `vcs.unexpected_commits` | C | provider | `commits` | Ken #5 |
| `vcs.missing_dirty_files` | C | provider | `scope?` | Ken #6 |

### io — filesystem / network plumbing

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `io.read_failed` | C | environment | `path` | file read errors |
| `io.write_failed` | C | environment | `path` | atomic write / write errors |
| `io.permission_denied` | C | environment | `path` | EACCES |
| `io.network` | T | environment | `url?`, `message` | network failures, incl. outbound messaging sends (§7.4) |

### config — Claudine / user configuration

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `config.invalid` | C | caller | `field`, `message` | `ConfigValidation` |
| `config.mcp_invalid` | C | caller | `server?`, `message` | MCP catalog/server config (§7.4) |

### usage — Rust API misuse

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `usage.invalid_argument` | C | caller | `argument`, `expected` | bad API args |
| `usage.unsupported` | C | caller | `operation`, `provider?` | `claudine-contract` `Unsupported` provider, etc. |

### runaway — content-guard trips

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `runaway.volume` | **U** | provider | `measure`, `cap` | volume cap (50k lines / 32 MiB) |
| `runaway.repetition` | **U** | provider | `cycles` | group-cycle repetition (≥30) |
| `runaway.exit_expression` | U | provider | `expression` | user `exit_expressions` match |

> Runaway is `unrecoverable` **by design**: it maps to `ProcessTermination::Aborted` and
> must never be retried (a retry reproduces the runaway). Encoding that as a facet stops a
> generic "retry transient/throttled" handler from ever looping it.

### internal — our bugs

| code | disp | origin | detail | subsumes / raised |
|------|------|--------|--------|-------------------|
| `internal.bug` | U | internal | `message` | unexpected invariants; `SemanticErrorKind::Unknown` |
| `internal.serialization` | U | internal | `message` | non-author-facing serde failures |

**Totals:** 12 categories, ~38 codes.

---

## 4. The expectation sub-family is `document.*` + `vcs.*`

All six of Ken's bespoke errors share `origin = provider` + `disposition = correctable` and
a common shape (`{ expectation, subject, observed }`, see error-structure.md §9). That
uniformity is the point: a single reusable handler — *"on any `correctable` error whose
`origin == provider`, re-prompt the agent with `err` as feedback"* — covers all six and any
future postcondition, with **no code enumeration**. This is the facets-over-tree payoff,
proven on Ken's own list.

---

## 5. Mapping the three existing taxonomies onto this one

The unification (error-structure.md §3) made concrete — what each existing value becomes:

| Existing | Maps to |
|----------|---------|
| `SemanticErrorKind::Configuration` | `config.invalid` / `auth.invalid` (split by cause) |
| `SemanticErrorKind::AgentNative` | `provider.exited` / `provider.stream_error` |
| `SemanticErrorKind::ApiRemote` | `cap.*` / `auth.*` / `provider.stream_error` |
| `SemanticErrorKind::Interrupted` | `provider.interrupted` |
| `SemanticErrorKind::Unknown` | `internal.bug` (unclassified) |
| `BadgeCategory::Auth` | `auth.invalid` |
| `BadgeCategory::Billing` | `cap.billing` |
| `BadgeCategory::Quota` | `cap.plan_limit` |
| `BadgeCategory::RateLimit` | `cap.rate_limit` |
| `BadgeCategory::ContextPressure` | `provider.context_pressure` |
| `BadgeCategory::Permission` | `auth.permission` |
| `BadgeCategory::Config` | `config.invalid` |
| `RateLimitInfo { retry_after_ms, reset_at }` | `cap.*` detail (verbatim — no new capture) |
| `BadgeSeverity` | `Severity` (kept as-is) |

The `BadgeCategory` precedence (`Auth > Billing > Quota > RateLimit > Permission`,
`badges.rs:96`) becomes the tie-break order when one instance could match multiple codes
(§7.6).

---

## 6. `err.*` surface, worked against Ken's three needs

```yaml
# pattern  — every throttle, regardless of which cap
failure: { when: "err.is_throttled", defer_until: "{{ err.reset_at }}" }   # (handler illustrative only)

# pattern  — every agent expectation failure, one rule for all six bespoke codes
failure: { when: "err.origin == 'provider' && err.is_correctable", … }

# specific — just plan caps
failure: { when: "err.code == 'cap.plan_limit'", … }

# instance — the agent forgot one specific property
failure: { when: "err.code == 'document.missing_frontmatter' && err.detail.property == 'status'", … }
```

---

## 7. Decisions to ratify (the calls I made — confirm or overrule)

1. **`Origin` has no separate `agent` value.** I collapsed "the model's behavior" into
   `provider` (the whole agent run) and let `disposition` carry the infra-vs-behavior
   distinction (infra → `transient`/`throttled`; behavior → `correctable`). *Alternative:*
   add a 6th origin `agent`. I recommend against — disposition already discriminates, and
   the platform-vs-model line is often ambiguous (a timeout could be either).

   ✅ **RATIFIED** — keep as is; 5 origins (no separate `agent` — disposition carries the
   infra-vs-behavior distinction).

   

2. **A `Category` is not disposition-uniform.** `cap` holds both `throttled` (rate/plan)
   and `correctable` (billing). I treat category as pure *domain* and disposition as pure
   *strategy*. *Confirm* you're happy that `err.category == "cap"` does **not** imply
   "waitable."

   ✅ **RATIFIED** — category is pure *domain*, disposition is pure *strategy*;
   `err.category == "cap"` does **not** imply waitable. Strategy handlers match on
   `disposition`/`code`, never on `category` alone.

   

3. **`composition.*` vs `document.*` split.** Author's prompt input vs the agent's produced
   output — kept separate even though both can "fail schema validation"
   (`composition.schema_validation` vs `document.invalid_frontmatter`). Different `origin`,
   different remediation. *Confirm* the split is worth two codes for a similar check.

   ✅ **RATIFIED** — keep the split.

4. **Foldings to keep the category list at 12.** MCP → `config.mcp_invalid` /
   (runtime) `provider.*`; outbound messaging (Discord/Slack) → `io.network`; linking/sync
   → `io`/`config`. *Alternative:* promote `mcp` and/or `messaging` to their own categories
   if authors will commonly handle them distinctly.

   ✅ **RATIFIED** — keep the foldings; category list stays at 12 (MCP, messaging, and
   linking fold into `config`/`provider`/`io` rather than earning their own categories).

   

5. **`usage_cap` merged into `cap.plan_limit`** via `limit_kind: plan|usage` rather than a
   separate code. This is the generic "one variant + discriminant field" vs "two distinct
   codes" question; the trade-offs, given *our* faceted/contract model:

   **For merging (one code, `limit_kind` discriminant):**
   - *Coarse handlers don't need it anyway.* "React to any cap" is `err.category == "cap"`
     or `err.is_throttled` — neither cares how many codes exist. The classic argument for
     merging ("avoid making handlers enumerate codes") is already neutralized by the facets,
     so merging costs little on the coarse side.
   - *Shared everything.* Identical `detail` shape (`reset_at`/`retry_after_ms`/`scope`),
     one render path, one resume-message template. No duplication.
   - *Smaller locked surface.* One fewer code in the versioned contract to document/ratify.
   - *Matches provider ambiguity.* Providers frequently don't cleanly label "plan" vs
     "usage" in their payloads. One code lets the classifier carry a best-effort
     `limit_kind` (or `unknown`) without ever mis-stamping the *code*.

   **Against merging (favor two codes):**
   - *The discriminant becomes contract regardless.* `limit_kind: plan|usage` is a locked
     enum either way — merging only moves the distinction from the **code axis** to the
     **detail axis**; it doesn't remove it. The real question is *which axis*, not *whether*.
   - *Specific targeting gets verbose and the name lies.* "Only usage caps" becomes
     `err.code == 'cap.plan_limit' && err.detail.limit_kind == 'usage'` — and the code name
     `plan_limit` is now a misnomer for a usage instance.
   - *The decisive risk — disposition must stay uniform within a code.* `code → disposition`
     is the property handlers rely on (the §1 decision keeps it stable even where *category*
     isn't). If a usage cap could ever be a **non-lifting** ceiling (a budget exhausted that
     needs a human to raise it → `correctable`) while a plan window always auto-lifts →
     `throttled`, then a single `cap.plan_limit` could not carry a stable disposition, and
     strategy selection would have to drill into `detail` — reintroducing, *at the code
     level*, the very coupling we worked to avoid.

   **Resolution / recommendation.** Merge — **but only because the disposition-divergent
   case does not belong here at all.** Define the boundary by disposition, not by vocabulary:
   - auto-lifting limits with a known reset (plan window, rolling usage window) →
     `cap.plan_limit`, `throttled`, with `limit_kind` as **pure metadata** (same disposition
     for every value);
   - a hard account ceiling that does **not** auto-lift (budget exhausted, needs human/billing
     action) → `cap.billing`, `correctable` — it was never a "usage cap," it's a billing stop.

   This keeps `code → disposition` 1:1, makes `limit_kind` safe to merge, and uses the
   existing `cap.billing` code for the only case that would have forced a split.

   **Migration note (why this must be decided now):** *both* later changes are breaking —
   splitting `cap.plan_limit` later changes what that code emits for usage instances (breaks
   handlers that matched it), and merging two codes later removes one (breaks handlers that
   matched it). There is no free "decide later," so the disposition-uniformity rule above is
   the thing to lock. *Confirm the boundary (disposition, not vocabulary), or overrule toward
   two codes if you expect usage-specific `detail` or handling to diverge.*

   ✅ **RATIFIED** - use one code
   
6. **Multi-match precedence.** When an instance could carry two codes (e.g. an auth failure
   surfaced as a stream error), the most-specific classifying layer wins (delegation, §9 of
   integrated-design); for genuine ties, reuse the `BadgeCategory` precedence. *Confirm*
   that order, or define a new one.

   ✅ **RATIFIED** — most-specific classifying layer wins (delegation); genuine ties fall
   back to the `BadgeCategory` precedence (`Auth > Billing > Quota > RateLimit > Permission`).

7. **`provider.interrupted` is `origin: caller`** (the human pressed Ctrl-C). *Confirm* —
   arguably `environment`, but the human operator is the caller.

   ✅ **RATIFIED** — `origin: caller`.

8. **Code stability granularity.** Codes are the locked contract; `detail` field names too;
   `Display` strings stay free. *Confirm* we are comfortable that adding a *new* `detail`
   field is non-breaking but renaming one is breaking (drives an additive-only discipline +
   a `claudine errors` introspection command).

   ✅ **RATIFIED** — codes + `detail` field names are the locked contract; `Display` stays
   free. Additive-only evolution (new code/field = non-breaking; rename/remove = breaking),
   backed by a `claudine errors` introspection command.

---

## 8. What this is not

- Not the handler/dispatch design (out of scope per Ken).
- Not the postcondition *checks* that raise `document.*`/`vcs.*` (that's the handling layer).
- Not immutable — §7 is ratified and locked, but the contract still evolves *additively*
  (new codes / detail fields are non-breaking; renames and removals are breaking).
