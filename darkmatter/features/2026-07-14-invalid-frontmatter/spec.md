---
clarified: "claude/claude-opus-4-8"
review_iterations: 1
---
# Invalid Frontmatter

Darkmatter has a "clean" feature that helps to cleanup "semi-standard" Markdown to be more standards based. What I've noticed is that it doesn't currently validate the YAML frontmatter and there have been more than one situation where an Agent produced an invalid entry in the YAML which just passed through the clean check unchallenged.

The most common error -- by far -- is that a property is assigned what is intended to be a _string_ value but the string is NOT quoted because YAML allows non-quoted strings but when certain characters are present (I think the starting character is the main determinant) the string must be quoted to be considered valid (both single and double quote characters are fine so long as they are consistent).

## Research

The [research](./research.md) file provides a deep dive into common YAML problems and enumerates ~15 algorithmic opportunities across three certainty tiers. Treat it as a **menu of opportunities, not a plan** — it catalogs what is *possible*, not what v1 ships. The [Scope](#scope) section below is the authority on what v1 does.

The three certainty tiers used throughout are the research tags:

- `deterministic` — the error is identified with certainty **and** the fix is provable. Safe to auto-apply.
- `deterministic-find-non-deterministic-solution` — the problem is identified with certainty, but the fix is a guess that could introduce a follow-on problem. Detect and report; never mutate.
- `non-deterministic-find` — a suspected smell that cannot be proven. Suggest only; never mutate.

## Scope

### Target surface

`md clean` validation and repair apply to the **frontmatter block only**. Body ` ```yaml ` fenced code blocks are **never** inspected or mutated — they are frequently intentional broken-YAML examples (this feature's own `research.md` is full of them), and auto-fixing them would corrupt documents.

The underlying `biscuit-file` engine is **general-purpose** and operates on any YAML source, so other callers (`bf` CLI, future consumers) can reuse it; only the `md clean` *integration* is frontmatter-scoped. See [Architecture and Ownership](#architecture-and-ownership).

### v1 In-Scope (auto-applied)

v1 auto-applies **all three `deterministic` opportunities**, each gated by the hard safety gate (see [Safety Gate](#safety-gate)):

1. **Source normalization** — BOM removal, CRLF/CR → LF, and parse-equivalent trailing-whitespace / final-newline cleanup.
2. **Parse-equivalent whitespace cleanup** — whitespace around flow collections, mapping colons, and sequence markers, applied *only* when the original and candidate parse to equal `serde_yaml_ng::Value`.
3. **Schema-proven scalar quoting** — e.g. `release: 1.20` → `release: "1.20"`, *only* when an authoritative schema rejects the node solely for not being a string and quoting the exact lexeme passes the full schema. The schema is resolved by `md clean` at **full compose parity** (see [Schema resolution](#schema-resolution)), so this tier is live in the common case rather than dormant. When no schema constrains a given key, that key is skipped by this tier (open item: silent skip vs. non-deterministic suggestion — see [Open Questions](#open-questions)).

The **flagship no-schema case** — an unquoted plain scalar that fails to parse, or parses to the wrong shape, because it begins with a reserved indicator (e.g. `title: @daily-report` → `title: "@daily-report"`) — is also auto-fixed under the same safety gate. This case spans **both** variants:

- the **parse-shape** variant (no schema needed — the raw text is unparseable or resolves to an unintended shape), and
- the **schema-type** variant (a schema proves a string was required).

### v1 Out-of-Scope (detected and reported, never mutated)

Everything in the other two tiers is **detected and reported but never mutated** in v1:

- Duplicate keys.
- Schema-guided key correction (e.g. `timeuot` → `timeout`).
- Schema-guided shape / type repair.
- Anchor / alias repair.
- Multi-document handling.
- All `non-deterministic-find` lints: ambiguous scalars, suspicious empty values, block-scalar smells, comment-truncation / indicator smells, style/indentation inconsistency, and similar/misplaced keys.

These are surfaced as suggestions (see [Behavior and UX](#behavior-and-ux)). Promotion of any of them to auto-fix is out of scope for v1.

## Behavior and UX

### Integration

Validation is **folded into the existing `md clean` command** — not a new subcommand, and not library-only. Callers of `md clean` get frontmatter validation for free.

### Default behavior

Deterministic repairs **auto-apply in place by default**. This is deliberate: the agents that produce these bugs will not opt in, so the fix must be default-on to reach the population that needs it.

### Non-deterministic findings

Non-deterministic findings print to **STDERR** as suggestions rendered with `TerminalRenderable` components. In v1 they **do not change the exit code**.

### Exit-code stability

`md clean`'s existing exit-code contract stays stable in v1. No new failure exit codes are introduced. Any future `--strict` / exit-code gate that would make findings fail the command is **explicitly deferred** to a later round (see [Open Questions](#open-questions)).

### Schema resolution

`md clean` today performs **no** schema work (`cli/src/commands/clean.rs` runs only the `cleanup*` formatting passes). v1 adds schema resolution at **full parity with `md compose`** so the schema-aware tier can fire:

- Inject the Darkmatter baseline schema by default (covers Darkmatter-owned keys: `ctx`, `hash`, `style`, `replace`).
- Honor an inline or file-reference `$schema` frontmatter property (covers the author's own keys).
- Perform repo-scoped trigger-schema discovery (ancestor-walk to the Git root for `schemas/*.yaml`).
- Expose the same escape hatches: `--baseline-schema PATH`, `--no-baseline-schema`, `--schema`, `--no-trigger-schemas`.

Schema resolution is **lazy and short-circuiting** for performance (see [Performance](#performance)): it runs only when a non-empty frontmatter block is present, and its result is cached per `clean` run.

### `--json` diagnostic contract

A machine-readable `--json` diagnostic contract is defined for v1, based on the research's `YamlDiagnostic` / `YamlRepair` sketch. Each diagnostic carries:

- `code` — a stable diagnostic code.
- `span` — a source range.
- `classification` — the certainty-tier tag.
- `message` — a human-readable explanation.
- `repairs[]` — zero or more candidate repairs, each with:
  - `span` — the source range the repair patches.
  - `replacement` — the replacement text.
  - `explanation` — why the repair is offered.

The **shape** above is committed for v1. The exact JSON field names and their stability guarantees are to be pinned in a later round (see [Open Questions](#open-questions)).

## Architecture and Ownership

This section supersedes and expands the earlier intent that "most of the fixing solution be located in biscuit-file." That intent is satisfied: the schema-**independent** engine (the bulk of the opportunity count) lives in `biscuit-file`. Schema-proven quoting living in Darkmatter is **acceptable and expected**, not a deviation.

### biscuit-file — schema-agnostic core

`biscuit-file` owns a schema-**agnostic** diagnose/repair engine. Exact signatures are TBD, but the surface is:

- `Yaml::diagnose() -> Vec<YamlDiagnostic>`
- `repair_candidates()`

It covers everything that is schema-independent:

- Source normalization.
- Parse-equivalent whitespace cleanup.
- Parse-error / parse-shape quoting (the no-schema flagship case).
- Duplicate-key detection.
- Anchor / alias detection.
- Multi-document detection.
- The schema-free `non-deterministic-find` lints.

### Darkmatter — schema-aware layer

Darkmatter layers the schema-**aware** repairs on top, using its own `SimplifiedSchema` / `DarkmatterSchemas` engine, and emits the **same diagnostic shape**:

- Schema-proven scalar quoting.
- Schema-guided key correction.
- Schema-guided shape / type repair.

(Of these, only schema-proven quoting is auto-applied in v1; the rest are report-only per [Scope](#scope).)

### Shared types

`YamlDiagnostic`, `YamlRepair`, and `SourceSpan` live in `biscuit-file` and are re-exported so both layers produce **one uniform shape**.

### Safety gate

The hard safety gate is: parse original → apply edit → reparse → require exact `serde_yaml_ng::Value` equality; and when a schema is associated, require identical schema results too. Its two halves are split across the boundary:

- The **`serde_yaml_ng::Value`-equality** half lives in `biscuit-file`.
- The **schema-equality** half lives in Darkmatter.

### biscuit-file groundwork (in-scope regardless)

Two small, unambiguous prerequisites in `biscuit-file`:

1. **Preserve structured error location.** Retain `serde_yaml_ng::Error`'s byte/line/col location instead of only a rendered string.
2. **Retain or reread original source for `YamlSource::Path`.** It currently stores only the path; format-preserving repair patches source spans rather than reserializing the parsed `Value`, so the raw source text must be available. (Retain-vs-reread and its TOCTOU implications are an [open question](#open-questions).)

## Performance

Performance is an explicit priority. `md clean` is a hot path (pre-commit hooks, agent runs), and both the safety-gate reparse and the newly-added full-parity schema resolution (including trigger-schema git-root discovery) add cost. v1 adopts a **relative no-regression posture** backed by concrete lazy/short-circuit requirements rather than a hard millisecond budget:

- **No frontmatter → zero cost.** Schema resolution *and* trigger-schema discovery run only when a non-empty frontmatter block is present. A document with no frontmatter pays nothing.
- **Per-run caching.** Trigger-schema discovery and the built validator are cached per `clean` invocation.
- **Reparse only candidates.** The safety-gate double-parse applies only to candidate edits, not to every document; an already-clean document parses once and is not reparsed.

**Acceptance:** no measurable regression on the two common cases — documents with no frontmatter, and documents that are already clean. A hard per-document budget is deferred until after benchmarking (see [Open Questions](#open-questions)).

## Open Questions

None of the following are decided; they must not be treated as requirements until resolved.

- **Acceptance criteria / definition of done** — no formal AC or DoD is written yet. Expected to fold in: idempotency, the never-mutate-on-non-deterministic guarantee, byte-for-byte preservation of untouched ranges and comments, the safety-gate invariants, and CRLF/BOM cross-platform behavior. Test corpus is expected to be the [YAML Test Suite](https://github.com/yaml/yaml-test-suite) plus mutation tests over real monorepo frontmatter.
- **Hard performance budget** — a concrete per-document millisecond budget, to be set after benchmarking. The [Performance](#performance) posture (no-regression + lazy/short-circuit) stands until then.
- **Unconstrained-key behavior for the schema tier** — when the effective schema does not constrain a given key, is schema-proven quoting silently skipped for it, or is a non-deterministic suggestion emitted?
- **`--json` field names + stability** — the diagnostic *shape* is committed, but exact field names and stability guarantees are not.
- **`YamlSource::Path` retain-vs-reread** — retain the source at parse time vs. reread on demand, and the TOCTOU implications of rereading.
- **Idempotency** — is a repeated `md clean` guaranteed to be a fixed point (clean output cleans to itself)?
- **Ordering** — where does frontmatter validation run relative to `clean`'s existing body-normalization passes?
- **Deferred `--strict` / exit-code gate** — a future mode that makes findings affect the exit code.
