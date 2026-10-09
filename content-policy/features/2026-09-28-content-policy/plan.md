---
total_phases: 6
created: 2026-09-29
phase: 1
agent: claude/opus
yolo: true
spec: 2026-09-28-content-policy
depends-on:
    - 2026-09-28-recursive-schema-types
source_files_during_phase_2:
    - content-policy/lib/src/lib.rs
    - content-policy/lib/src/model.rs
    - content-policy/lib/src/diagnostic.rs
    - content-policy/lib/src/grammar.rs
    - content-policy/lib/src/normalized.rs
    - content-policy/lib/src/time.rs
    - content-policy/lib/src/aggregate.rs
    - content-policy/lib/src/reader.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/robustness_matrix.rs
    - content-policy/lib/tests/fixtures/stamped-note.md
docs_updated_during_phase_2:
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - Cargo.lock
    - content-policy/lib/Cargo.toml
    - content-policy/lib/src/lib.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/src/renew.rs
    - content-policy/lib/tests/common/mod.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/renewal.rs
docs_updated_during_phase_3:
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - content-policy/docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - Cargo.lock
    - content-policy/cli/Cargo.toml
    - content-policy/cli/src/main.rs
    - content-policy/cli/src/args.rs
    - content-policy/cli/src/commands.rs
    - content-policy/cli/src/output.rs
    - content-policy/cli/tests/common/mod.rs
    - content-policy/cli/tests/check.rs
    - content-policy/cli/tests/renew.rs
    - content-policy/cli/tests/lifecycle.rs
    - content-policy/lib/src/diagnostic.rs
    - content-policy/lib/src/renew.rs
    - content-policy/lib/tests/renewal.rs
    - content-policy/schemas/content-policy.yaml
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
docs_updated_during_phase_4:
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - content-policy/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - Cargo.lock
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/boundary_containment.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - content-policy/lib/Cargo.toml
    - content-policy/lib/src/lib.rs
    - content-policy/lib/src/model.rs
    - content-policy/lib/src/diagnostic.rs
    - content-policy/lib/src/grammar.rs
    - content-policy/lib/src/normalized.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/src/renew.rs
    - content-policy/lib/src/path_form.rs
    - content-policy/lib/src/fingerprint.rs
    - content-policy/lib/src/provider.rs
    - content-policy/lib/src/file_adapter.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/renewal.rs
    - content-policy/lib/tests/robustness_matrix.rs
    - content-policy/lib/tests/file_changed.rs
    - content-policy/lib/tests/file_renewal.rs
    - content-policy/lib/tests/file_adapter.rs
    - content-policy/lib/tests/fake/mod.rs
    - content-policy/cli/src/commands.rs
    - content-policy/cli/src/output.rs
    - content-policy/cli/tests/file_changed.rs
    - content-policy/schemas/content-policy.yaml
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
docs_updated_during_phase_5:
    - biscuit-file/docs/topics/file-references.md
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - content-policy/docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - content-policy/docs/topics/policy-lifecycle.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/content-policy/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
source_code:
    - Cargo.lock
    - Cargo.toml
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/yaml/analyze/diagnostic.rs
    - biscuit-file/lib/src/yaml/analyze/engine.rs
    - biscuit-file/lib/src/yaml/analyze/locate.rs
    - biscuit-file/lib/src/yaml/analyze/mod.rs
    - biscuit-file/lib/src/yaml/analyze/scan.rs
    - biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs
    - biscuit-file/lib/src/yaml/analyze/tests/locate.rs
    - biscuit-file/lib/src/yaml/analyze/tests/mod.rs
    - biscuit-file/lib/src/yaml/analyze/tests/scan.rs
    - biscuit-file/lib/src/yaml/analyze/tests/tab_indentation.rs
    - biscuit-file/lib/src/yaml/mod.rs
    - biscuit-file/lib/tests/corpus/yaml_corpus.json
    - darkmatter/cli/src/commands/hash.rs
    - darkmatter/lib/src/effects/verbs.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/hash/options.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/tab_indentation_repair_parity.rs
    - content-policy/cli/Cargo.toml
    - content-policy/cli/src/main.rs
    - content-policy/justfile
    - content-policy/lib/Cargo.toml
    - content-policy/lib/src/lib.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - content-policy/lib/src/model.rs
    - content-policy/lib/src/diagnostic.rs
    - content-policy/lib/src/grammar.rs
    - content-policy/lib/src/normalized.rs
    - content-policy/lib/src/time.rs
    - content-policy/lib/src/aggregate.rs
    - content-policy/lib/src/reader.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/robustness_matrix.rs
    - content-policy/lib/tests/fixtures/stamped-note.md
    - content-policy/lib/src/renew.rs
    - content-policy/lib/tests/common/mod.rs
    - content-policy/lib/tests/renewal.rs
    - content-policy/cli/src/args.rs
    - content-policy/cli/src/commands.rs
    - content-policy/cli/src/output.rs
    - content-policy/cli/tests/common/mod.rs
    - content-policy/cli/tests/check.rs
    - content-policy/cli/tests/renew.rs
    - content-policy/cli/tests/lifecycle.rs
    - content-policy/schemas/content-policy.yaml
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/boundary_containment.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - content-policy/lib/src/path_form.rs
    - content-policy/lib/src/fingerprint.rs
    - content-policy/lib/src/provider.rs
    - content-policy/lib/src/file_adapter.rs
    - content-policy/lib/tests/file_changed.rs
    - content-policy/lib/tests/file_renewal.rs
    - content-policy/lib/tests/file_adapter.rs
    - content-policy/lib/tests/fake/mod.rs
    - content-policy/cli/tests/file_changed.rs
documentation:
    - biscuit-file/lib/README.md
    - darkmatter/docs/cli/hash.md
    - darkmatter/docs/cli/clean.md
    - content-policy/docs/dependencies.md
    - docs/dependencies.md
    - claudine/docs/getting-started/index.md
    - biscuit-terminal/docs/research/terminal-multiplexing/about.md
    - biscuit-terminal/docs/research/terminal-multiplexing/cmux.md
    - biscuit-terminal/docs/research/terminal-multiplexing/ghostty.md
    - biscuit-terminal/docs/research/terminal-multiplexing/tmux.md
    - biscuit-terminal/docs/research/terminal-multiplexing/wezterm.md
    - biscuit-terminal/docs/research/terminal-multiplexing/zellij.md
    - claudine/docs/research/acp/gemini-cli.md
    - claudine/docs/research/acp/json-rpc.md
    - claudine/docs/research/acp/kimi-code-cli.md
    - .claude/skills/playa/audio-programming/Android.md
    - .claude/skills/playa/audio-programming/IOS.md
    - .claude/skills/playa/audio-programming/crates.md
    - .claude/skills/playa/audio-programming/linux.md
    - .claude/skills/playa/audio-programming/macOS.md
    - .claude/skills/playa/audio-programming/typescript-libraries.md
    - .claude/skills/playa/audio-programming/windows.md
    - sniff/docs/research/audio-programming/Android.md
    - sniff/docs/research/audio-programming/IOS.md
    - sniff/docs/research/audio-programming/crates.md
    - sniff/docs/research/audio-programming/linux.md
    - sniff/docs/research/audio-programming/macOS.md
    - sniff/docs/research/audio-programming/typescript-libraries.md
    - sniff/docs/research/audio-programming/windows.md
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - biscuit-file/docs/topics/file-references.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/content-policy/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
completed_phase: 6
implemented: true
packages:
    - content-policy
    - content-policy-cli
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - claudine-cli
    - claudine
---

# Plan: Content Policy — Evaluation, Baselines, and Renewal

## Summary and Definition of Done

**What is being built.** A new `content-policy` package area with a library
(`content-policy/lib`, crate `content-policy`) and a CLI
(`content-policy/cli`, crate `content-policy-cli`, binary `policy`). Markdown
documents declare `content_policy:` rules in frontmatter. The library parses and
validates them, evaluates them against an evidence record under an explicit
clock, aggregates actions (`remove` > `archive` > `refresh`), and plans and
applies byte-exact renewal edits. The CLI exposes `policy check` (with
`--needs-action`) and `policy renew` (with `--write`).

The spec's two increments map onto plan phases as follows:

| Increment | Plan phases | Content |
| --- | --- | --- |
| Prerequisites | Phase 1 | Rulings, one spike, Biscuit File fixes, UTC stamping, document migration, package scaffold |
| Increment 1 (time rules) | Phases 2–4 | `Evergreen`, `TimeSensitive`, `ValidFor`, `ValidUntil`: model, evaluation, reader, renewal, CLI, schema file, docs |
| Increment 2 (`FileChanged`) | Phase 5 | Path layer, provider contract, file adapter, fingerprints, renewal, CLI, schema member |
| Closure | Phase 6 | Darkmatter base-schema line (gated on `2026-09-28-recursive-schema-types`), skill, OS evidence, final audit |

**Work outside the new crates** (spec, "Changes outside the new crates"):

- Biscuit File gets a tab-indentation repair, three locator fixes, and
  public relative-path containment.
- Darkmatter `md hash`, Darkmatter auto-rehash, and Claudine write-back
  stamp the UTC date.
- 23 documents migrate from `Duration(...)` and `update_policy`.
- The editor schema ships, and Darkmatter's base schema references it.

**Done means all of the following are true:**

1. Every acceptance criterion in the spec (1–33) has a passing test or check.
   The second half of AC 21 (the base-schema line) is the only exception, and
   only while `2026-09-28-recursive-schema-types` has not landed (see ruling
   R1).
2. `just test`, `just test-l2`, and `just lint` pass in `content-policy/`,
   `biscuit-file/`, `darkmatter/`, and `claudine/`.
3. `cargo tree -p content-policy` (default features) shows no PDF crate and no
   path to `darkmatter`. The same holds with `--all-features`.
4. The crates build and test on macOS, Linux, native Windows, and WSL2. The
   `FileChanged` path layer has Windows evidence (see Phase 6).
5. `content-policy/README.md`, `content-policy/docs/topics/policy-lifecycle.md`,
   `docs/dependencies.md`, the Biscuit File docs, and a new
   `.claude/skills/content-policy/` skill describe the shipped behavior. No
   **planned** marker remains for delivered behavior.
6. The implementing agent stops at "implementation complete, ready for review".
   It never moves the spec to `_completed` and never runs `just complete`.

## Input Robustness Matrix

The feature reads one file format, YAML frontmatter, plus the serialized
normalized policy (JSON), so this section applies. The outcomes below are taken
from the spec or from rulings R3, R4, R8, and R9. Every cell is asserted through
the **public result**: the evaluation report or its diagnostics, or the CLI exit
code and stream. Parser return values do not count.

**YAML frontmatter.** The load-bearing fields are the policy value, each
policy entry, the long-form `rule` and `action`, any date-position property
(the default `last_updated` or an `@name` target), and a `FileChanged`
fingerprint property.

| Shape | Policy value (`content_policy`) | One entry | `rule` (long form) | `action` (long form) | Date property | Fingerprint property (Phase 5) |
| --- | --- | --- | --- | --- | --- | --- |
| absent | Caller's default policy; report marks it `defaulted` | n/a | Validation error: `rule` required | Validation error: `action` required (R3) | `unknown`, missing baseline | `unknown`, missing baseline |
| explicit null | Validation error (`content_policy:` with no value); **not** the default | Validation error for that entry (`- ` with no value) | Validation error | Validation error | `unknown`, missing baseline (spec defines null as missing; still a separate test from absent) | `unknown`, missing baseline (separate test) |
| wrong type, whole field | String: validation error suggesting the list form. Mapping or number: validation error | Number, bool, or list entry: validation error naming the entry | Number or mapping: validation error | Number: validation error | Number, bool, list, or object: wrong-type validation error | Non-string: validation error (ruling 23) |
| wrong type, one element | `[ValidFor(3mo), 123]`: validation error for entry 2 and **no verdict**. It is never filtered out | — | — | — | — | — |
| wrong type, every element | `[123]`: validation error per entry and no verdict. It is never read as empty | — | — | — | — | — |
| empty | `[]`: validation error suggesting `Evergreen`. `{}`: wrong-type validation error | `{}` entry: validation error (`rule` required) | `""`: validation error (unparseable rule) | `""`: validation error (unknown action) | `""`: invalid-date validation error | `""`: malformed-shape validation error |
| duplicate key | Duplicate `content_policy`: validation error, no verdict | Duplicate `rule` or `action` inside one entry mapping: validation error (R4) | (same as entry) | (same as entry) | Duplicate `last_updated`: validation error, no verdict | Duplicate fingerprint key: validation error |
| trailing or invalid content | Malformed YAML: malformed-frontmatter error, CLI exit `1`. An unquoted `{{ }}` gets a message naming Darkmatter's expression protection. A rule string with trailing text, such as `ValidFor(3mo) x`: validation error | Unbalanced parentheses from the flow-list comma trap: diagnostic naming the comma split and showing the block form | `ValidFor(3mo))`: validation error | `archive x`: validation error | `2026-09-28x`, `2026-02-30`, or `2026-09-28T10:00:00Z`: invalid-date or dates-only validation error | `blake3-lf:zz`, `blake3-lf`, or `:abc`: malformed-shape validation error. A well-formed unknown scheme (`sha256:ab`) is `unknown`, not an error |

**Serialized normalized policy (JSON).** Its load-bearing fields are
`grammar_version` and `entries` (R8).

| Shape | `grammar_version` | `entries` |
| --- | --- | --- |
| absent | Validation error | Validation error |
| explicit null | Validation error | Validation error |
| wrong type, whole field | `"1"`: validation error | Object: validation error |
| wrong type, one / every element | — | Validation error naming the element. It is never filtered or read as empty |
| empty | — | `[]`: validation error (empty policy) |
| duplicate key | Validation error | Validation error |
| trailing or invalid content | Trailing bytes after the JSON object: validation error. A value newer than the library's version: validation error (fail closed) | Unknown field: validation error (`deny_unknown_fields`) |

**Test obligations:**

- There is one matrix test per format, and each walks the whole table.
  - The YAML test starts from a real-tool fixture: a migrated
    `biscuit-terminal/docs/research/terminal-multiplexing/*.md` note after
    `md hash` has stamped it.
  - The JSON test starts from a policy serialized by the library itself.
- Each cell is one edit to the fixture.
- A **control row** asserts that the unedited fixture yields a valid report
  (`fresh` at a fixed `--at`), which proves each edit is load-bearing.
- **Smell grep before declaring the matrix done:** in `content-policy/lib/src`,
  grep for `#[serde(default)]` on load-bearing fields, `Option<T>` where absent
  and null must differ, `filter_map(` with `as_str()`, `unwrap_or_default()`,
  and `.ok()` on load-bearing parses. Justify or fix every hit.
- A review finding about any one cell must list the state of every other cell
  in the same row and column.

## Phase 1 — Rulings, Spike, and Prerequisites

This phase records the rulings, runs one spike, and lands the changes outside
the new crates that phases 2–5 depend on, plus the empty package scaffold.

### Necessary Rules

The spec reports `needs_rulings: false`. Planning still surfaced the points
below, which the spec leaves to "implementation design" or does not address.
Each has a recommendation, and the author confirms or overrides it before
Wave 2 starts. Record confirmed rulings in the implementation log.

- **R1 — The dependency has not landed.** `2026-09-28-recursive-schema-types`
  is still `draft-spec`.
  - *Recommendation:* Ship `content-policy/schemas/content-policy.yaml` in
    Phase 4 without `(required)` on `rule`, and add the `FileChanged` member in
    Phase 5. Both load and validate today; evaluation alone enforces a required
    `rule`.
  - The base-schema line (spec task 4, second half of AC 21) waits in Phase 6
    behind a gate.
  - If the dependency has not landed by then, the plan ends at "ready for
    review, AC 21 part 2 blocked on dependency". The author decides whether the
    feature can close with that item outstanding.
- **R2 — Increments versus phases.** The spec plans "two phases of one plan".
  This plan splits increment 1 into Phases 2–4 for reviewability. The increment
  boundary is the Phase 4 checkpoint.
- **R3 — A long-form entry without `action` is a validation error.**
  - The spec's schema makes `action` required.
  - The spec's model says "defaulting to `refresh`", which applies to the
    compact form.
  - *Recommendation:* The evaluator and the schema must agree, so a missing
    `action` is an error. The compact form is how an author takes the default.
- **R4 — Duplicate keys anywhere in the block are validation errors.** This
  covers top-level keys and keys inside a policy-entry mapping. `serde_yaml_ng`
  rejects duplicates recursively, as the spike's strict reader showed, and one
  rule is simpler than a top-level-only rule.
- **R5 — How to spell a multi-entry `--default-policy` / `CONTENT_POLICY_DEFAULT`.**
  - *Recommendation:* A value that begins with `[` is parsed as a YAML flow
    sequence by the same policy parser, with the same comma-trap diagnostic.
    Any other value is one compact rule with action `refresh`.
  - Examples: `TimeSensitive` and
    `'["ValidFor(3mo)", {rule: "ValidUntil(2027-01-01)", action: archive}]'`.
  - An empty or invalid value is a usage error (exit `2`).
- **R6 — Clock injection for CLI tests.**
  - AC 7 requires `policy renew --on 2026-12-29 --write`, but renew rejects a
    future `--on`, and today is 2026-09-29.
  - *Recommendation:* Add a hidden (`hide = true`) `--today <YYYY-MM-DD>`
    option on `renew` that overrides "the current UTC date". It is used only by
    tests and is not documented in help or on the docs page.
  - `check` needs nothing new, because `--at` already covers it.
  - The rejected alternative was a fourth `CONTENT_POLICY_*` environment
    variable, which ruling 24 does not list.
- **R7 — Policy identity canonical form.**
  - *Recommendation:* `xxh64:` + 16 lowercase hex digits (the messenger
    `canonical.rs` convention), computed over the compact JSON of
    `{grammar_version, entries}`.
  - Each entry contributes its rule name, its parameters with inline baselines
    stripped, its `@name` reference names, and its action.
  - Entries are **sorted** before hashing, because reordering never changes the
    verdict (AC 4).
  - The defaulted/declared flag is **excluded**: a declared `ValidFor(6mo)` and
    the built-in default name the same policy.
  - `grammar_version` starts at `1` and is included, so a grammar change
    invalidates cached artifacts.
- **R8 — Serialized normalized policy.** The normalized policy derives
  `Serialize`/`Deserialize` with `deny_unknown_fields`, and `grammar_version`
  is a required `u32`. Deserializing a version greater than the library's is a
  validation error. The normalized form holds entries only; evidence stays
  outside it.
- **R9 — Content fingerprint shape.**
  - The value must match `^[a-z0-9][a-z0-9-]*:[0-9a-f]+$`. Anything else is a
    validation error.
  - For a *recognized* scheme (`blake3`, `blake3-lf`), a hex length other than
    64 is also a validation error.
  - Uppercase hex is rejected, because renewal writes lowercase and a
    case-folded match would hide a typo.
  - Following ruling 23, a well-formed value with an unrecognized scheme is
    `unknown`.
- **R10 — `blake3-lf` normalization.** Only CRLF pairs become LF. A lone CR is
  data and is kept. Hashing goes through `biscuit-hash` with its `blake3`
  feature.
- **R11 — Where path rules are enforced.**
  - `FileReference::class()` requires Biscuit File's `file-reference` feature
    (gix). AC 22 keeps that feature out of the default build.
  - *Recommendation:* Core performs a **lexical** path-form check that is
    independent of features. It covers every lexical row of the path table:
    absolute paths, drive letters, `\`, the prefixes `~ @ % & ^` where they are
    not allowed, `vault:`, `{{`, URLs, surrounding whitespace, `,`, and `)`.
    Failures are validation errors, with or without the adapter.
  - The bundled file adapter cross-checks with `FileReference::class()`,
    discovers the boundary, and resolves the path.
  - A boundary escape comes back from the provider as a distinct
    `OutsideBoundary` observation. The evaluator promotes it to a validation
    diagnostic, so the document gets no verdict.
  - With no provider configured, a lexically valid `FileChanged` entry yields
    `unknown` with a missing-provider reason.
- **R12 — Provider contract.**
  - The contract is synchronous. No network provider is in scope, so no async
    boundary is introduced.
  - The file provider returns one of `Present(bytes)`, `Missing`, `NotAFile`,
    `Unreadable(reason)`, or `OutsideBoundary`.
  - **Core** computes fingerprints per scheme, so the fake provider and the
    real adapter share one hashing path.
  - The bundled adapter's feature is `file-adapter` and is off by default. It
    enables `biscuit-file/file-reference`. The CLI enables it.
- **R13 — Report details the spec leaves open.**
  - Document identity is the path as the caller supplied it, never
    canonicalized. The evidence-map API takes an optional caller label.
  - Reader warnings, such as the tab repair, are a `warnings` field of the
    report. They appear in `--json` output and in the terminal rendering, not
    on stderr.
- **R14 — UTC stamping tests stay portable.**
  - Each of the three writers takes its stamping instant from an injected
    `DateTime<Utc>` (or a small helper that accepts one), instead of calling
    `chrono::Local::now()` inline.
  - Tests pass an instant such as `2026-09-28T23:30:00Z`. They show that a
    `+10:00` `FixedOffset` view of that instant is already `2026-09-29`, then
    assert that the stamp is `2026-09-28`.
  - No test mutates the process `TZ`, which is not portable to Windows.
- **R15 — Mixed line endings.** content-policy uses the spec's per-line
  terminator rule now. `2026-09-28-hash-writer-byte-fidelity` is still
  `draft-spec`. If that fix later keeps Darkmatter's global rule, aligning
  content-policy's insertions becomes a follow-up of that fix, not a task in
  this plan.
- **R16 — Dependency and PDF guard.** AC 11 and AC 22 are enforced by a
  `just deps-check` recipe in `content-policy/justfile`, called from
  `just lint`. It runs `cargo tree` for the default features and for
  `--all-features`, and fails on any `pdf`/`lopdf` crate or a `darkmatter`
  crate. It is not a nextest test, which would shell out to cargo.
- **R17 — New skill.** AC 9's "applicable skills" is met by a new
  `.claude/skills/content-policy/SKILL.md` in Phase 6. No existing skill covers
  this area.

### Spikes

Both spike questions the spec raised (the reader and byte-exact edits, and
`FileChanged` path resolution) already have findings next to the spec. They are
not repeated. The spike findings, rather than a new spike, also settle YAML
1.1 value arrival, duplicate-key detection, CRLF offsets, and writer parity.
No performance spike is needed, because the spec names no cost.

- [x] **Spike S1 — Embedded schema reference** (one host, about 1 hour)
    - **Question:** can the base schema, embedded with `include_str!` in
      `darkmatter/lib/src/markdown/schemas/mod.rs`
      (`darkmatter_base_schema_ref`), resolve a relative file reference such as
      `x@../../../content-policy/schemas/content-policy.yaml` in both
      `md schema validate` and DMLS? If it cannot, what is the smallest change
      that makes it resolve?
    - **Method:** use a **non-union** placeholder type, which today's
      Darkmatter accepts, so the answer does not wait on
      `2026-09-28-recursive-schema-types`. Work in a throwaway branch or
      directory, and revert.
    - **Output:** `spikes/embedded-schema-ref/findings.md`, which feeds task
      6.1. The spec says "the task must also make the relative reference
      resolve", and S1 sizes that work now instead of discovering it at the
      end.
    - Do not widen the spike to test union `[]`, which is the dependency's
      job.

### Wave 1 (parallel; one subagent each, disjoint files)

- [x] **1.1 Package scaffold**
    - Create `content-policy/lib` (`content-policy`) and `content-policy/cli`
      (`content-policy-cli`, `[[bin]] name = "policy"`), and add both to the
      root `Cargo.toml` `members`.
    - Library default dependencies: `serde`, `serde_json`, `chrono`,
      `biscuit-hash` (features `xx_hash`, `blake3`), and
      `biscuit-file = { default-features = false, features = ["yaml"] }`.
    - Declare the empty optional `file-adapter` feature (R12). It enables
      `biscuit-file/file-reference`.
    - Add `content-policy/justfile`, modeled on `biscuit-hash/justfile`: the
      imports, `build`, `test`, `test-l2` (not applicable, with its message),
      `test-l3`, `lint`, `install`, `sanity`, and the `deps-check` recipe
      (R16). The test scope is `content-policy --all-features;
      content-policy-cli`.
    - Add both crates to `docs/dependencies.md`, and add a
      `content-policy/docs/dependencies.md` if the area convention calls for
      one.
    - Confirm that `sniff repo package-areas` lists `content-policy`, and read
      `just ci-local --plan` to see the new cells.
    - Done when `cargo check -p content-policy -p content-policy-cli` passes
      and `just deps-check` passes.
- [x] **1.2 Biscuit File locator fixes** (spec change table rows 2–4; AC 29,
  Biscuit File half)
    - `parent_path` in `biscuit-file/lib/src/yaml/analyze/scan.rs` must place a
      zero-indent block-list item (`content_policy:\n- X`) under its key, not
      at root `[Index(0)]`.
    - `locate_yaml_value` in `locate.rs` must return `None` for a multi-line
      plain or quoted scalar, as its module docs already say.
    - `YamlValueLocation` must report an anchor, a tag, or an alias on the
      located value. Add a field or enum, since renewal refuses all three.
    - Each change needs its own Biscuit File test. Also run the existing
      `md clean` tests, because they share the scanner.
- [x] **1.3 Biscuit File tab-indentation repair** (spec change table row 1;
  AC 27, repair half)
    - Add a repair to `analyze_yaml` (`biscuit-file/lib/src/yaml/analyze/`)
      that replaces leading tabs with spaces, as Darkmatter's tab normalization
      does. That includes tabs beyond the first indentation level inside a
      block scalar.
    - Its value-equality gate compares against the tab-normalized
      interpretation, because the original does not parse.
    - Biscuit File tests cover the repair itself, including the block-scalar
      `<TAB><TAB>Line two` → `"Line one\n  Line two"` case.
    - A **Darkmatter-side** test (Darkmatter already depends on Biscuit File)
      asserts that the repaired record equals Darkmatter's tab-normalized
      record for the same text.
    - Verify that `md clean` (through
      `darkmatter/cli/src/commands/clean/frontmatter_repair.rs`) now offers the
      repair. Update the Biscuit File docs page for the repair engine.
- [x] **1.4 UTC `last_updated` stamps** (spec change 2; AC 20)
    - Switch the three `chrono::Local::now()` stamps to the UTC date, with
      injected-clock tests per R14:
        - `darkmatter/cli/src/commands/hash.rs:188`
        - `darkmatter/lib/src/effects/verbs.rs:46`
        - `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:2274`
    - Update any docs page or comment that says "local date" for these writers.
    - Run `just test` in `darkmatter/` and `claudine/`.
- [x] **1.5 Document migration** (spec change 1; AC 16, content half)
    - Rewrite the 23 documents listed in the spec's Backwards compatibility
      table:
        - the 6 `biscuit-terminal/docs/research/terminal-multiplexing/*.md`
          notes (`Duration(3mo)`, and `about.md` has `Duration(12mo)`);
        - 14 `update_policy` documents in `.claude/skills/playa/audio-programming/`
          and `sniff/docs/research/audio-programming/`;
        - 3 in `claudine/docs/research/acp/`.
    - Each document gets `content_policy:` with space-indented `- ValidFor(<n><unit>)`
      entries, and `Duration(6 mo)` becomes `ValidFor(6mo)`.
    - Remove every `update_policy:` key.
    - `json-rpc.md` becomes `- ValidFor(1yr) # pending: MajorVersion(latest_version)`.
    - Touch nothing else in these files. In the eight tab-indented documents,
      only the migrated lines change indentation; the rest of the frontmatter
      stays tab-indented, and the reader repairs it.
    - Before editing, confirm the set with a repository grep for `Duration(` and
      `update_policy:` inside frontmatter. Report any count other than 23
      instead of silently widening the change.

### Wave 2

- [x] **1.6 Rulings confirmed** — Record the author's answers to R1–R17 in the
  implementation log, and fold any overrides into Phases 2–6 before they
  start.

### Phase 1 checkpoint

- [ ] `just test` and `just lint` pass in `biscuit-file/`, `darkmatter/`, and
  `claudine/`. `cargo check` passes for the new crates, and `just deps-check`
  passes.
- [x] Spike S1 findings are written.
- [x] A grep finds no frontmatter `Duration(` and no `update_policy:` in the
  repository, excluding specs and spikes that quote them.

## Phase 2 — Policy Model, Frontmatter Reader, and Evaluation (Increment 1)

This phase builds the library core for the four time and constant rules,
through evaluation and reports. Renewal comes in Phase 3.

### Wave 3

- [x] **2.1 Core types** (one agent; later waves build on these)
    - Add `EvidenceRecord`, which wraps `IndexMap`/`Map<String, serde_json::Value>`;
      `Rule`, with `Evergreen`, `TimeSensitive`, `ValidFor { duration, baseline }`,
      and `ValidUntil { deadline }`; `Baseline` (`Inline(date)`,
      `Reference(name)`, `Defaulted`); `Action`; `PolicyEntry`; and `Policy`.
    - `Policy` is non-empty by construction, carries a grammar version, and has
      a `defaulted` flag.
    - Add `PolicyOptions`, holding the key, the default policy, and the date
      property. It has **no** way to express "no default" (AC 10).
    - Add the result enums (`triggered`, `not_triggered`, `unknown` with typed
      reasons), `Status`, `Resolution`, and the `Report` skeleton.
    - Add validation diagnostics that carry entry and property locations.
    - Public items get rustdoc per the repo's comment standards.

### Wave 4 (parallel)

- [x] **2.2 Declaration parser and validation** (AC 1, 10, 16, 17, 26; the
  policy-value columns of the matrix)
    - Parse the compact grammar, positive integer durations in
      `d`/`wk`/`mo`/`yr`, `YYYY-MM-DD` dates, `@name` references, and long-form
      `{rule, action}` entries.
    - Names are case-sensitive.
    - These are validation errors, each with a test and a targeted message:
        - an empty list (the message suggests `Evergreen`);
        - a single string (the message suggests the list form);
        - a null or malformed policy value;
        - `Evergreen` combined with another rule;
        - `Duration(...)` (an unknown rule);
        - a dotted `@a.b` reference, even when that literal key exists;
        - a missing `action` (R3);
        - unbalanced parentheses from the flow-list comma split (the message
          shows the block form).
    - Collect **all** invalid entries, not just the first.
    - Normalized serialization and grammar version per R8, and a newer version
      is rejected.
    - Policy identity per R7 (AC 23).
- [x] **2.3 Time semantics** (AC 3)
    - Evaluation time is an explicit `DateTime<Utc>`, and every date takes
      effect at 00:00 UTC.
    - `d`/`wk` are day increments. `mo`/`yr` use calendar arithmetic that
      clamps to the month's end, matching Darkmatter's `parse_duration_spec`.
      Cross-check it with a table test, not a dependency.
    - A deadline outside chrono's range is a validation error.
    - Tests cover Jan 31 + 1mo, Feb 29 + 1yr, `ValidFor` due on its computed
      day and not the day before, `ValidUntil(2027-01-01)` triggered at the
      start of Jan 1 and not at 23:59:59 on Dec 31, and a future baseline
      yielding `unknown` (inconsistent baseline).
- [x] **2.4 Frontmatter reader** (AC 18, 27 reader half, 28; the reader rows of
  the matrix)
    - Find the frontmatter block, including under a BOM, CRLF, or lone CR.
    - Parse the YAML **without the block's final line terminator**.
    - Report "no frontmatter" distinctly from an empty record.
    - Reject duplicate keys (R4).
    - Apply the Biscuit File tab repair (1.3) in memory and emit a warning
      (R13).
    - Reject `{{ }}`-only documents with a message naming Darkmatter's
      expression protection.
    - Return the source spans renewal needs: the block's byte range and its
      terminator style.
    - Evidence value typing follows the spec's Evidence values table, with one
      test per row, plus the YAML 1.1 arrival rows (`yes`, `True`, `010`,
      `.inf`, `~`) (AC 2).
- [x] **2.5 Aggregation** (AC 4)
    - Implement the status and action-resolution tables as a pure function over
      entry results.
    - Add an exhaustive test over every combination of confirmed and unknown
      actions for up to three entries, and a permutation test showing that
      entry order never changes the result.

### Wave 5

- [x] **2.6 Evaluator and report** (AC 1, 2, 5, 10, 11)
    - The public API evaluates `(Policy or declaration value, EvidenceRecord,
      evaluation time, options)` with no document involved, and a convenience
      API evaluates document bytes through the reader.
    - Resolve baselines, resolving the default date property for shorthand, and
      record each baseline's source and value.
    - Evaluate every entry without short-circuiting.
    - Any invalid entry means the report has no verdict.
    - The report fields are listed under the spec's Report shape. Freeze the
      JSON field names here, because they become public contract in Phase 4.
    - Tests:
        - inline and referenced dates are equivalent;
        - evaluation never mutates its input, including on first evaluation;
        - an absent policy under a `TimeSensitive` default gives `stale`;
        - an `update_policy`-only document uses the default policy;
        - the plain-map evaluation needs no Markdown.
- [x] **2.7 Robustness matrix tests** — Build the YAML and JSON matrix tests
  and the control row from the Input Robustness Matrix section, then run the
  smell grep.

### Phase 2 checkpoint

- [x] `just test` and `just lint` pass in `content-policy/`, including
  `deps-check`.
- [x] Every cell of the matrix, except the Phase 5 fingerprint column, is
  asserted.
- [x] Through the library, steps 1, 2, 5 (with a hand-edited baseline), 6, and
  8 of the spec's lifecycle example give the stated results.

## Phase 3 — Renewal (Increment 1)

This phase adds the library's renewal planning and application. It depends on
Phase 2 and on tasks 1.2 and 1.3.

### Wave 6 (parallel)

- [x] **3.1 Renewal planner** (AC 6, 19, 25 library half)
    - Renewal covers **every** renewable entry.
    - The update date defaults to today's UTC date from an injected clock, and
      a future date is rejected.
    - Inline `ValidFor` dates are replaced in place. `@name` rules edit the
      property. Shorthand rules create or update the date property.
    - Identical writes to a shared property are consolidated.
    - These are conflicts: two different writes to one property, and a write
      that would change a `ValidUntil`'s referenced deadline.
    - A missing or `null` target is labeled `new baseline`. A present malformed
      value is an error, because renewal does not repair it.
    - `Evergreen`, `TimeSensitive`, or `ValidUntil`-only policies produce a
      "nothing to renew" plan.
    - Plan everything before any edit. Missing evidence produces diagnostics
      and no partial plan.
    - The plan carries an `xxh64` plan fingerprint of the bytes it was read
      from, via `biscuit-hash`.
- [x] **3.2 Span-targeted editor** (AC 12, 14, 29 renewal half, 30, 31)
    - Translate spans from the frontmatter slice into whole-file offsets.
    - Narrow the edit to the date inside a plain or single-quoted rule string.
    - Fill `last_updated:` → `last_updated: 2026-09-28`, and write
      `last_updated: 2026-09-28   # todo` when a comment follows.
    - Append a missing property with the preceding line's terminator.
    - Create a new block for a document with no frontmatter, after any BOM and
      with the body's terminator.
    - Include the tab repair as its own listed edit.
    - Apply edits with `apply_edit_set`.
    - Refuse, with a reason, each shape in the spec's refused list:
        - a value inside a flow list;
        - a block or multi-line scalar;
        - a double-quoted string with escapes (the first three messages name
          the block-list form);
        - an anchor, alias, or tag;
        - a flow-mapping entry;
        - an unterminated block or one closed by `...`;
        - a `----` fence;
        - a span that does not decode to the parsed value.
    - A flow list whose edits all fall outside the brackets renews.
    - Use the frontmatter-reader spike's 64-fixture matrix as a source of test
      cases. Add one byte-exact test per row of the spec's Darkmatter
      comparison table, and a mixed line-ending test.

### Wave 7

- [x] **3.3 Apply and safety net** (AC 12, 31)
    - `apply` re-reads the file, compares the plan fingerprint, and on a
      mismatch returns a conflict and writes nothing.
    - Before writing, re-parse the edited text. If anything other than the
      target values (and the tab repair) changed, write nothing. Test this
      with a deliberately corrupted edit.
    - The write goes through a shared library helper, so a library caller can
      apply a plan without the CLI.
    - Write atomically, via a temp file and rename in the same directory.
      Check `biscuit-file` for an existing atomic-write helper before writing a
      new one.

### Phase 3 checkpoint

- [x] `just test` passes. Through the library, with an injected clock, all
  eight lifecycle steps give the stated results.
- [x] Renewal of each migrated document from 1.5 in a temp copy is byte-exact
  apart from `last_updated`, and apart from the listed tab repair for the eight
  tab-indented documents.

## Phase 4 — CLI, Schema File, and Increment 1 Documentation

### Wave 8 (parallel)

- [x] **4.1 `policy check`** (AC 15, 24; CLI half of AC 7 and 27)
    - Use clap subcommands.
    - `--at`, `--key`, `--default-policy` (R5), `--date-property`, and
      `--needs-action`.
    - Configuration precedence is flag, then `CONTENT_POLICY_KEY`,
      `CONTENT_POLICY_DEFAULT`, `CONTENT_POLICY_DATE_PROPERTY`, then the
      built-in value. Add one test per level per setting.
    - Default output is a `biscuit-terminal` `Prose` summary plus a `Table` of
      entries. `--plain` removes styling. `--json` prints only the report on
      stdout.
    - Exit codes: `0` when a report was produced; `1` for invalid declarations,
      unreadable files, and malformed frontmatter, with diagnostics on stderr
      even in `--json` mode; `2` from clap.
    - `--needs-action` prints `true`, `false`, or `unknown` and exits `0`. Add
      one test per answer.
    - Load the `cli` and `biscuit-terminal` skills first.
- [x] **4.2 `policy renew`** (AC 25 CLI half; CLI half of AC 7)
    - `--on`, `--write`, the hidden `--today` (R6), and `--plain`/`--json`.
    - The preview labels "new baseline" and lists the tab repair separately.
      "nothing to renew" exits `0`.
    - Exit `1` for refusals, conflicts, and missing evidence, writing nothing.
- [x] **4.3 Editor schema file** (AC 21, first half)
    - Add `content-policy/schemas/content-policy.yaml` (`kind: schema`) with
      `short_form`, `long_form`, and `policy`, following the spec excerpt.
    - Sibling references use `@./content-policy.yaml` (or `@this`).
    - Patterns use `\x28`/`\x29`/`\x2C`/`\x20`, with one `suggest(...)` on the
      first member, holding date-only suggestions.
    - `rule` has no `(required)` (R1).
    - No `FileChanged` member yet.
    - Verify with `md schema validate`: the file loads, and in a test document
      that names it as `$schema`, a mixed compact and `{rule, action}` list
      validates, while `Duration(3mo)` and `action: delete` are flagged.
      Capture this as a Darkmatter or content-policy test that runs the
      library's schema validator, not a shell script.

### Wave 9

- [x] **4.4 Lifecycle end-to-end through the CLI** (AC 7)
    - One test drives the eight lifecycle steps through the `policy` binary,
      on a temp copy, with `--at` and the hidden `--today`.
    - Assert the result of each step. For step 3, assert byte-exact file
      content.
- [x] **4.5 Increment 1 documentation** (AC 13)
    - Update `content-policy/docs/topics/policy-lifecycle.md` and
      `content-policy/README.md` to describe the shipped time rules:
        - the CLI output, exit codes, and configuration precedence;
        - the empty-list rule and the always-present default;
        - the frozen JSON field names;
        - the refused shapes;
        - the tab-repair warning.
    - Remove the **planned** markers for delivered behavior. Keep them for
      `FileChanged` and the base-schema line.
    - Follow the repo's docs audience rule: examples per rule, and a Mermaid
      diagram for the renew flow. No `docs/` page may name this spec.
    - Update `docs/dependencies.md` for any crate added since 1.1 (for example
      `clap` or `biscuit-terminal` in the CLI).
- [x] **4.6 Test-input declaration**
    - AC 16's test evaluates the 23 migrated repository documents. Spell those
      reads in the forms the `rust-testing` skill lists, so CI schedules the
      narrowed cell described in `docs/cicd/test-inputs.md`.
    - Confirm with `just ci-local --plan`.

### Phase 4 checkpoint (increment 1 complete)

- [x] `just test` and `just lint` pass in `content-policy/`.
- [x] Every AC except 8, 32, 33, and the second half of AC 21 is covered.
- [x] `policy check` over the 23 migrated documents reports no diagnostics.

## Phase 5 — `FileChanged` (Increment 2)

### Wave 10 (parallel)

- [x] **5.1 Biscuit File containment** (spec change table row 5)
    - Expose a public relative-path containment check. It wraps the
      crate-private `validate_repository_containment` in
      `biscuit-file/lib/src/file_reference/resolve.rs`, alongside
      `validate_repository_candidate`.
    - It checks lexically and, for an existing target, after following
      symlinks.
    - Add Biscuit File tests for inside, `../` escape, and a symlink escape.
      Update the file-reference docs.
- [x] **5.2 `FileChanged` declaration and lexical path rules** (AC 8
  validation, 32 rejected lexical forms, 33; matrix fingerprint column)
    - Parse `FileChanged(<path>, @<property>)`. The one-argument form is an
      error.
    - The lexical path validation in core (R11) has one test per rejected row
      of the spec's path table. `,` and `)` are rejected, and a quoted rule
      whose path contains ` #` or `: ` evaluates.
    - Fingerprint value typing per R9 is added to the matrix tests.
    - Identity includes the path (AC 23).
- [x] **5.3 Provider contract and fingerprint schemes** (AC 8)
    - Add the synchronous file-provider trait and its observation enum (R12).
    - Core computes `blake3-lf`/`blake3` fingerprints via `biscuit-hash`, with
      the R10 normalization.
    - Map the spec's outcome table:
        - a match is `not_triggered`;
        - a difference is `triggered`;
        - `Missing` is `triggered`, "source removed";
        - `Unreadable` and `NotAFile` are `unknown`;
        - an absent or `null` property is `unknown`;
        - an unrecognized scheme is `unknown`;
        - `OutsideBoundary` is a validation diagnostic.
    - Identical requests share one observation per run.
    - A fake provider drives one test per outcome row.
    - A line-ending test shows that `blake3-lf` is stable across LF and CRLF,
      while `blake3` changes.
    - The evidence-map API takes a caller-supplied base directory. Test it with
      no document behind the map.

### Wave 11

- [x] **5.4 Bundled file adapter** (feature `file-adapter`; AC 8, 32)
    - Discover the boundary from the base directory: the repository root via
      Biscuit File's repository scope catalog, or else the current working
      directory as the tree root.
    - Build `FileResolutionContext::from_snapshot(base, None, HashMap::new())`
      and add the repository, package, and package-area roots. Outside a
      repository, the tree root serves as the repository root and there are no
      package roots, so `^` falls back to it.
    - Rewrite an implicit relative path to `./<path>`, so no silent
      repository-root retry happens.
    - Run the containment check (5.1), then call `resolve_detailed`, and map a
      broken symlink to `Missing`.
    - Test every accepted and rejected form in the spec's path table:
        - `../` inside and outside the boundary;
        - `&` and `^` inside and outside a repository;
        - an escape outside a repository, from two different working
          directories;
        - a bare path that exists only at the repository root (not found);
        - a directory (`unknown`);
        - an unreadable file, gated with `#[cfg(unix)]`, plus the Windows
          equivalent if the `os` skill lists one.
    - Load the `os` skill before writing path-comparison code.
    - Tests must not rely on the process working directory. Pass the tree root
      explicitly through an internal seam, so tests stay parallel-safe under
      nextest.
- [x] **5.5 `FileChanged` renewal** (AC 8)
    - First capture writes a `blake3-lf:` value, and renewal of an existing
      value keeps its `blake3:` scheme.
    - A missing or unreadable file writes nothing and returns diagnostics.
    - An unrecognized scheme is reported for correction.
    - A fingerprint property outside a flow list renews even when the policy
      list is flow-style.

### Wave 12 (parallel)

- [x] **5.6 CLI and schema for `FileChanged`**
    - The CLI enables `file-adapter`. The base directory is the document's
      directory.
    - `check` and `renew` tests cover a `FileChanged` document, including the
      lifecycle: capture, match, edit the watched file, triggered, renew,
      fresh.
    - Add the `FileChanged` member to `content-policy/schemas/content-policy.yaml`.
      It requires `@<property>`, and its path pattern excludes what the
      constraint parser allows (at least `,`, `)`, and `\`). Re-run the 4.3
      schema test, adding a `FileChanged(path)` case that must be flagged.
- [x] **5.7 Increment 2 documentation**
    - Update the topic page and README for `FileChanged`: the path table, the
      boundary rules with the two-directory example, the fingerprint schemes,
      and the outcome table.
    - Document the `file-adapter` feature for library callers. Remove the
      **planned** markers.

### Phase 5 checkpoint

- [x] `just test` and `just lint` pass in `content-policy/` and `biscuit-file/`.
- [x] AC 8, 32, and 33 are covered. The fingerprint column of the robustness
  matrix is asserted.
- [x] `just deps-check` still passes with `--all-features`: the `file-adapter`
  feature adds gix but no PDF crate and no `darkmatter`.

## Phase 6 — Base-Schema Line, Skill, and Cross-OS Closure

### Wave 13 (gated task in parallel with the rest)

- [x] **6.1 Darkmatter base-schema line** (spec task 4; AC 21 second half)
    - **Outcome (2026-09-29): blocked on dependency, skipped per R1.** The
      dependency is still `draft-spec`, and this tree's `md schema validate`
      rejects `policy[]@…content-policy.yaml` ("cannot apply `[]`/constraints
      to the union-typed named type"). Remaining steps are listed in the
      implementation log, Phase 6, task 6.1.
    - **Gate:** `2026-09-28-recursive-schema-types` has landed, meaning
      `policy[]` over a union-typed named type loads. If it has not, record
      "blocked on dependency" in the implementation log and skip this task
      (R1).
    - Add `content_policy: policy[]@../../../content-policy/schemas/content-policy.yaml`
      beside `last_updated` in whichever base schema file Darkmatter loads at
      that time. Check both `darkmatter/docs/schemas/darkmatter.yaml` and
      `darkmatter/schemas/partials/doc.yaml`.
    - Apply the change S1 identified, so the embedded baseline resolves the
      relative reference.
    - If the dependency also lifted constraints on union references, add
      `(required)` on `rule`.
    - Acceptance is by behavior. In a Darkmatter test and a DMLS test, an
      ordinary Markdown document with no `$schema` validates a mixed list and
      flags `Duration(3mo)` and `action: delete`.
    - `cargo tree` shows no new dependency.
- [x] **6.2 content-policy skill** (R17)
    - Add `.claude/skills/content-policy/SKILL.md`, under 200 lines. It covers
      the library API entry points, the rule grammar, the renewal limits, the
      `file-adapter` feature, the dependency rule (never depend on Darkmatter),
      and the `just` recipes.
    - Add pointers to the new UTC-stamp behavior in the `darkmatter` and
      `claudine` skills, where they mention `last_updated`.
- [x] **6.3 Cross-OS evidence**
    - Load the `os` skill.
    - Run `just test content-policy` and `just test biscuit-file` on native
      Windows and on WSL2 through the hosts the skill names. Focus on the path
      layer (drive letters, `\`, case-insensitive matching, symlinks) and on
      CRLF fingerprinting.
    - Record the evidence in the implementation log. Fix any failure forward
      in this phase.
- [x] **6.4 Final audit**
    - Walk AC 1–33 and map each to its test name in the implementation log.
    - Re-run the robustness smell grep.
    - Confirm that no `docs/` page names this spec, and that no **planned**
      marker remains for delivered behavior.
    - Review every behavior-changed symbol's rustdoc for drift, following the
      Code Comment Quality rules in `CLAUDE.md`.
    - Set the spec's frontmatter to `status: implemented` and
      `implemented_by: claude/opus`. Stop at "ready for review".

### Phase 6 checkpoint

- [x] `just test`, `just test-l2`, and `just lint` pass in every touched area.
- [x] `just ci-local --plan` has been reviewed.
- [x] Either 6.1 is done, or it is logged as blocked on
  `2026-09-28-recursive-schema-types` with the remaining steps listed.
