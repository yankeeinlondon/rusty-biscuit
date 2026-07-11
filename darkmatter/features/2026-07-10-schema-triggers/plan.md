---
agent: claude/
created: 2026-07-10
total_phases: 8
phase: 1
yolo: true
---

# Schema Triggers — Execution Plan

Derived from [`spec.md`](./spec.md). Content-triggered schema layering: trigger
schemas live in ancestor-walked `schemas/` roots, activate by matching a
document's parsed frontmatter + normalized path (not location), and layer a
payload schema between the caller baseline and the document `$schema`.

## Design Ground Truth (verified against the code)

- **Schema-value resolution authority** is `lib/src/markdown/schemas/resolve.rs`
  (`resolve_schema`, `resolve_yaml_schema`, `merge_baseline`, `ResolvedSchema`
  with `imports` / `examples` / `referenced_files` dependency edges). Trigger
  discovery is a **sibling** module, not new policy inside the value resolver.
- **Top-level API** is `DarkmatterSchemas` in `lib/src/markdown/schemas/mod.rs`;
  `effective_for(&Markdown) -> EffectiveSchema` is the assembly seam. Baselines
  are attached via `with_baseline*`. `EffectiveSchema` already carries `origins:
  SchemaOriginMap` and `dependencies: Vec<PathBuf>` — the exact hooks triggers
  must extend.
- **SimplifiedSchema type-expr parser** lives in
  `lib/src/markdown/schemas/simplified/{grammar.rs,types.rs}`. The `file` type's
  `match(...)` glob already parses to `Constraint::Match(Vec<String>)` (globs
  with `!` negation) — the match grammar's `$path` predicate reuses this.
- **Standalone-schema classification** is `parse_standalone_schema_document`
  (`simplified/standalone.rs`), which recognizes the `kind: schema` envelope.
  The `kind: trigger-schema` envelope is the same envelope-claiming pattern.
- **CLI**: `cli/src/commands/schema/{mod,validate,detect,about}.rs`;
  `validate.rs` owns `validate_one` / `load_api` / `BASELINE_SCHEMA_ENV`. Compose
  flags (`baseline_schema`, `no_baseline_schema`, `allow_host`) live in
  `cli/src/args/command.rs`. Repo-root discovery uses `find_git_root_from` /
  sniff.
- **DMLS**: `dmls/src/overlay/schema.rs` is the effective-schema assembler
  (`assemble`, `SchemaBundle`, glob-matched extension baselines);
  `dmls/src/workspace/{discover,watch}.rs` own globset discovery +
  `didChangeWatchedFiles`; `dmls/src/graph` carries the `uses_schema` edge;
  `dmls/src/diagnostics` owns the code taxonomy. Trigger evaluation must be
  wired **once** in the library assembler and consumed by both surfaces.

## Conventions

- `just test` (L1), `just test-l2` (L2), `just lint` are run **inside the
  package area** (`darkmatter/`). Never run `cargo fmt`.
- New library code lands under `lib/src/markdown/schemas/triggers/`.
- Tasks marked **[P]** may proceed in parallel with their sibling **[P]** tasks.
- en-US spelling in every new symbol and doc.

---

## Phase 1 — Match grammar, trigger envelope, and the pure matcher

Foundational library core. No I/O, no discovery, no assembly wiring yet.
Satisfies acceptance criterion 1.

- [ ] Create the module skeleton `lib/src/markdown/schemas/triggers/mod.rs`
  (declared from `schemas/mod.rs`) with submodules `grammar`, `envelope`,
  `matcher`, and a `lint` seam; re-export the public surface from
  `schemas/mod.rs`.
- [ ] Define the match-expression AST in `triggers/grammar.rs`: a boolean tree
  node (`All`, `Any`, `None`, `MinMatch { count, of }`), property conditions
  (`property: <type-expr>` reusing `TypeExpr` from `simplified`), and the
  dollar-reserved `$path` predicate (`Vec<String>` globs with `!` negation,
  same shape as `Constraint::Match`).
- [ ] Implement match-expression parsing from `serde_yaml_ng::Value`:
  - mapping with only combinator / `$`-predicate keys → combinator node
    (sibling keys AND'd);
  - mapping with no combinator keys → implicit `all` of property conditions;
  - **mixing** combinator and property-condition keys in one mapping → load
    error (never a guess).
- [ ] Implement the outer-OR **match arms**: `match:` sequence → `anyOf` of arm
  expressions; `match:` mapping → single arm. Mirror the root `$schema` union
  convention.
- [ ] Enforce the **match-safe constraint subset** at load: reject constraints
  that resolve files, import types, load examples, provide defaults, mark
  generated values, or eager-existence-check `file`. Allow structural types +
  `required`, `enum`, `pattern`, length/range, item-count, key-count. `file` is
  testable only as a string-shaped value. Emit a distinct load error naming the
  forbidden constraint.
- [ ] Implement the **vacuous-trigger lint** (`triggers/lint.rs`): per arm,
  statically detect that no satisfiable path through the boolean tree contains a
  presence-requiring condition (a `required` gate or `$path` inside `all`/`any`,
  not under `none`); any vacuous arm is a **load-time error** (one vacuous arm
  makes the whole trigger vacuous because arms OR).
- [ ] Parse the `kind: trigger-schema` envelope in `triggers/envelope.rs`:
  `match:` (required), `$schema:` payload reference (deferred resolution — Phase
  4), reject unknown top-level keys. Do **not** resolve the payload here.
- [ ] Implement the **pure matcher** in `triggers/matcher.rs`:
  `fn matches(expr, frontmatter: &serde_json::Value, normalized_path: &str) ->
  bool`. Guard vs gate semantics: a bare type-expr key may be absent, but a
  present value that contradicts the type **defeats** the match; `required`
  gates presence; `enum`/`pattern` are value predicates; `$path` matches the
  boundary-relative `/`-separated path (bare basename glob ≡ `**/basename`,
  gitignore-style, case-sensitive). No I/O, no validator compilation.
- [ ] Add `SchemaError` variants in `errors.rs` for the load-time failures:
  malformed match grammar, forbidden-constraint, mixed-mapping, vacuous-arm
  (each with source context fields).
- [ ] **Table-driven unit tests** covering: every combinator, `min-match`
  N-of-M, outer-OR arms, free nesting, guard-vs-gate, type-contradiction
  defeat, `$path` normalization (bare basename, `**`, `!` negation,
  case-sensitivity), forbidden-constraint rejection, mixed-mapping rejection,
  and vacuous-arm rejection. Include the reserved-property-name v1 limitation
  (`all`/`any`/`none`/`min-match`).

**Checkpoint 1:** `just test` green for the new module; matcher proven
side-effect-free by construction (no filesystem/env types in its signature).
Run `just lint`.

---

## Phase 2 — Schema-root discovery (ancestor walk) and the trigger registry

Ancestor-walk `schemas/` discovery + transactional registry loading. Depends on
Phase 1 (envelope + match parsing). Satisfies acceptance criterion 2.

- [ ] **[P]** Implement the ancestor walk in `triggers/discovery.rs`: from the
  document directory up **through** the configured boundary (inclusive),
  collecting every `schemas/` directory as a root, **nearest first**. Never
  continue past the boundary to filesystem root / home even when boundary
  discovery failed. A document outside the boundary yields **no** roots.
- [ ] **[P]** Implement per-root file enumeration: regular `.yaml`/`.yml` only,
  **do not follow** directory or file symlinks, order by **UTF-8 filename
  bytes**. Two names colliding after case-folding within the scan are a **load
  error** (prevents case-sensitive vs case-insensitive divergence).
- [ ] **[P]** Implement lexical boundary-relative path normalization for
  `$path`: `/` separators on macOS/Windows/Linux, case-sensitive, `..` never
  present (out-of-boundary documents are ineligible). Unit-test with
  Windows-style and POSIX-style inputs.
- [ ] Implement **filename shadowing across roots**: nearest root wins by
  trigger **filename** with no merging (`{pkg}/schemas/claudine.trigger.yaml`
  fully shadows `{repo}/schemas/claudine.trigger.yaml`); a shadowed file is
  never evaluated.
- [ ] Implement **discovery classification**: a `schemas/` file that does *not*
  claim `kind: trigger-schema` is **silently ignored**; one that *does* claim it
  but is malformed (bad envelope, bad match, vacuous, unresolvable-later) is a
  **hard load error**.
- [ ] Build the `TriggerRegistry` type (per-boundary): the ordered, deduped set
  of unshadowed, loaded trigger envelopes keyed by boundary. Loading is
  **transactional** — if any unshadowed opted-in trigger is invalid, install
  **no** registry from that scan and surface the schema-load error.
- [ ] Unit + fixture tests: nested roots, inclusive boundary, filename
  shadowing, `.yaml`/`.yml` mix, symlink exclusion, case-fold collision,
  malformed-trigger atomicity, and pathless / out-of-boundary → no discovery,
  on macOS + Windows-compatible + Linux-compatible paths.

**Checkpoint 2:** `just test` green; discovery tests pass with tempdir fixtures
that pin their own `.git`/boundary marker (avoid the known ancestor-`.git`
tempdir flake). Run `just lint`.

---

## Phase 3 — Bare-name schema reference resolution

Add bare-name resolution as one more rung on the existing FileReference /
schema-resolution ladder. Behavior change with a pointed migration error.
Depends on Phase 2 (schema roots are the resolution context). Feeds acceptance
criterion 7.

- [ ] Thread the schema-root list into `resolve.rs` as **resolution context**
  (not a parallel resolver): a reference with **no path component** (`$schema:
  claudine.yaml`, `Name@claudine.yaml`, `example(...)` bare refs) resolves
  against the roots, nearest first. References with a path separator, `./`
  prefix, or magic-path prefix are **untouched**.
- [ ] Keep `FileReference` as the authority that turns the selected reference
  into a filesystem path; bare-name selection only chooses *which* reference.
- [ ] Implement the **pointed error**: when a bare name is not found in any
  schema root but a document-sibling file of that name exists, the error must
  suggest `./‹name›`.
- [ ] Unit tests: bare-name resolves nearest-root-first; path-qualified refs
  unchanged; sibling-only bare name produces the `./name` suggestion; shared
  ladder covers `$schema`, `Name@file` imports, and `example(...)`.

**Checkpoint 3:** `just test` green; the three reference kinds share one
resolver (assert no duplicated resolution path). Run `just lint`.

---

## Phase 4 — Effective-schema assembly integration (library)

Wire matcher + discovery + payloads into `DarkmatterSchemas::effective_for`.
This is the single implementation both surfaces consume. Depends on Phases 1–3.
Satisfies acceptance criteria 3 and 5.

- [ ] Add explicit trigger configuration to `DarkmatterSchemas` (mod.rs):
  `with_trigger_discovery(document_path, boundary)` and
  `with_trigger_registry(prebuilt)`. `new()` stays deterministic and **never**
  scans disk. Document that implicit CWD-based discovery is forbidden.
- [ ] Implement **payload resolution + merge-compatibility gate**: a trigger
  payload resolves via the document `$schema` value forms but must resolve to a
  **merge-compatible object schema** (SimplifiedSchema single object or a
  simple-object raw JSON Schema). Reject root unions / non-object payloads as
  trigger payloads (existing `validate_simple_object_schema` is the contract).
- [ ] Implement **cycle detection**: a payload that resolves to the trigger
  envelope itself, directly or through an import cycle, is a **hard load
  error** with source paths (reuse the `ImportCycle` machinery already in
  `resolve.rs`). Directly referencing a `*.trigger.yaml` file as a document
  `$schema` is an error naming the payload as the thing to reference.
- [ ] Implement **matching + precedence** in `effective_for`: assemble in order
  (1) caller baseline, (2) matching trigger payloads — **nearest root first,
  filename-lexicographic within a root** — (3) document `$schema` (always wins).
  Shadowing is applied **before** matching. Later layers win per top-level
  property via the existing merge contract (no nested-fragment combining).
- [ ] Extend **origins + dependencies**: `EffectiveSchema.origins` records each
  contributing trigger, and `dependencies` includes every contributing trigger
  envelope, payload, payload import, and example (sorted/deduped, matching the
  existing dependency-edge contract). Add a `SchemaOriginKind` variant for a
  trigger source.
- [ ] Provide the pure matching entry the surfaces call: given a parsed
  frontmatter snapshot + normalized path + registry, return the ordered matched
  trigger set (for the `md schema triggers` trace and DMLS).
- [ ] Tests: baseline → ordered-triggers → document-schema precedence;
  shadowing-before-matching; non-mergeable-payload rejection; `.trigger.yaml`
  envelope + separate payload resolve without self-reference; direct and
  indirect envelope/payload cycles fail with source paths; origin attribution;
  complete dependency collection.

**Checkpoint 4:** `just test` green; a Claudine-shaped fixture (envelope
`claudine.trigger.yaml` + payload `claudine.yaml`) activates and merges
correctly and does **not** re-apply a programmatically supplied identical
baseline. Run `just lint`.

---

## Phase 5 — CLI integration (`md compose`, `md schema validate`, triggers command)

Wire the library assembly into the `md` surfaces. Depends on Phase 4.
Parallelizable with Phase 6 (**[P]** at the phase level — independent surface).
Satisfies the CLI half of acceptance criterion 4 and part of 7.

- [ ] Source the discovery **boundary** for the CLI from the sniff-discovered
  repository root (`find_git_root_from`). A stdin/virtual document has no roots
  unless a synthetic source path + boundary is supplied.
- [ ] `md compose`: enable trigger discovery by default in the validation stage;
  match against the parsed frontmatter snapshot **after** `--set`/`--state` and
  interpolation pass 1 but **before** frontmatter shell execution; re-run
  effective-schema assembly after shell expansion + interpolation pass 2 so a
  newly concrete value can (de)activate a trigger.
- [ ] `md schema validate`: **honor triggers by default** without weakening the
  explicit-baseline contract (`--schema` / `BASELINE_SCHEMA` only; no default
  Darkmatter baseline). Implement the **two-pass assignment flow** in
  `validate_one`: resolve the pre-assignment effective schema for assignment
  coercion, apply assignments, discard, then resolve triggers again for
  validation (corrects the current "effective schema unaffected by assignments"
  assumption).
- [ ] Add `--no-trigger-schemas` to **both** `md compose` and `md schema
  validate` (`cli/src/args/command.rs`): disables discovery **and** bare-name
  schema-root lookup together; a path-qualified `$schema` still resolves. No
  env-var alias in v1.
- [ ] Add the `md schema triggers <file>` subcommand
  (`cli/src/commands/schema/`): lists roots, shadowed envelopes, matching
  triggers + arms, and the first defeating condition per non-matching arm.
  Emit through the shared structured trace so DMLS can reuse it later.
- [ ] Register the new command in `cli/src/commands/schema/mod.rs` and the CLI
  arg/command enums; confirm `md schema detect` is unaffected.
- [ ] CLI integration tests: triggers honored by default in validate/compose;
  `--no-trigger-schemas` raw mode; host opt-in / pending-shell-value behavior;
  assignment re-resolution; the `triggers` command trace output.

**Checkpoint 5:** `just test` and `just test-l2` green for the CLI; manually run
`md schema triggers <fixture>` and confirm the trace. Run `just lint`.

---

## Phase 6 — DMLS integration

Consume the same library assembly from the language server. Depends on Phase 4.
Parallelizable with Phase 5 (**[P]**). Satisfies the DMLS half of acceptance
criterion 4 and all of 6.

- [ ] Source the DMLS **boundary** from the containing LSP workspace folder,
  narrowed to a sniff-discovered repository root when one exists below it; a
  document outside every workspace folder has trigger discovery disabled.
- [ ] Build and cache a **per-boundary `TriggerRegistry`**; wire trigger
  evaluation into `dmls/src/overlay/schema.rs` assembly so triggers layer
  between the extension/base baseline and the document `$schema`. DMLS performs
  **no** interpolation or shell execution — it matches the last-good parsed
  source, so for the same settled on-disk source its activation is byte-for-byte
  the CLI result.
- [ ] Implement **hysteresis** on the per-document last-good frontmatter tree
  (`OverlayState`): activate as soon as a settled parse matches; deactivate only
  when a **clean** parse affirmatively does not match (mid-keystroke YAML
  breakage must not flap activation).
- [ ] Register schema roots into `didChangeWatchedFiles` (with the save-driven
  rescan fallback for watcher-less clients); add trigger files as dependency
  edges (`uses_schema`-style) so editing a payload re-validates every document
  it is active on.
- [ ] Treat an **envelope** as a **boundary-level** dependency even for
  documents it does not currently match: add/delete/rename/edit of any candidate
  trigger file rebuilds that boundary's registry and re-evaluates all indexed
  documents under the boundary. Payload/import/example changes may use the
  narrower reverse-dependency fan-out.
- [ ] Implement **transactional last-good registry**: on a malformed trigger
  edit, retain the last-good registry, publish a schema-load diagnostic on the
  offending trigger file, and do not flap documents to a partial set.
- [ ] Tests (in-memory LSP session fixture): activation parity vs CLI for a
  settled file; hysteresis across a broken→clean edit; boundary rebuild on
  envelope create/change/delete/rename re-validates all documents;
  payload-change fan-out hits only dependent documents; last-good registry
  retained after a malformed edit.

**Checkpoint 6:** `just test` and `just test-l2` green for `dmls`; the parity
test asserts identical trigger sets for CLI and DMLS on the same settled fixture.
Run `just lint`.

---

## Phase 7 — `md schema about` typed catalog and parity

Document the trigger grammar through the typed descriptor catalog. Depends on
Phase 1 (grammar frozen); may run **[P]** with Phases 4–6. Satisfies acceptance
criterion 8.

- [ ] Extend the typed descriptor catalog (`schemas/about.rs`) with descriptors
  for: the `kind: trigger-schema` envelope, the **match-safe constraint
  subset**, `$path` semantics, combinators (`all`/`any`/`none`/`min-match`),
  outer-OR arms, and the vacuous-arm lint. The catalog must identify the
  match-safe subset so `about` cannot imply broader support.
- [ ] Wire the new descriptors into `md schema about` output
  (`cli/src/commands/schema/about.rs`).
- [ ] Add/extend the existing parity tests that keep the catalog in lock-step
  with the implementation (the descriptor set must match the actually-accepted
  match grammar from Phase 1).

**Checkpoint 7:** `just test` green; `md schema about` renders the trigger
grammar section. Run `just lint`.

---

## Phase 8 — Corpus migration, cross-surface parity, and acceptance sweep

Final integration, the behavior-change migration, and the acceptance-criteria
sweep. Depends on all prior phases.

- [ ] **Grep the corpus** for path-less schema references (`$schema:` bare
  names, `Name@name`, bare `example(...)` refs). For each: confirm it is
  intentionally schema-root-resolved, or migrate it to `./name`. Record the
  audit result.
- [ ] Verify the sibling-only legacy reference emits the `./name` suggestion
  diagnostic end-to-end (CLI + DMLS).
- [ ] Author the **dialect-family fixtures** (`claudine.trigger.yaml`,
  `inline-compose.trigger.yaml`, `sequence.trigger.yaml`) with disjoint `none:`
  carve-outs and their separate payloads; use them as the shared parity corpus.
- [ ] **Cross-surface parity test (acceptance criterion 4):** pin that `md
  compose`, `md schema validate`, and DMLS produce **identical** effective
  schemas / trigger sets for the same settled file and configured boundary.
- [ ] Sweep all eight acceptance criteria against the implementation; check each
  off in the spec's acceptance list and note any deviation.
- [ ] Update drift surfaces: the `darkmatter` skill (SKILL.md schema-validation
  section) and `docs/topics/schema-definition.md` to document trigger schemas;
  add per-area `docs/dependencies.md` entries only if new crates were pulled in
  (none expected — globset/sniff/serde_yaml_ng already present).

**Checkpoint 8 (final):** From the package area, `just test`, `just test-l2`,
and `just lint` all green. All eight acceptance criteria demonstrably met. The
Claudine dialect family activates only on matching documents and never touches
plain Darkmatter documents. Confirm no `cargo fmt` was run.

---

## Dependency & Parallelism Summary

```
Phase 1 (matcher core)
   └─> Phase 2 (discovery + registry)
          └─> Phase 3 (bare-name refs)
                 └─> Phase 4 (assembly integration)
                        ├─> Phase 5 (CLI)      [P] ─┐
                        └─> Phase 6 (DMLS)     [P] ─┤
Phase 1 ───────────────────> Phase 7 (about)  [P] ─┤   (after grammar frozen)
                                                    └─> Phase 8 (migration + parity + sweep)
```

- **Phases 5, 6, and 7 run in parallel** once their prerequisites are met
  (5 & 6 after Phase 4; 7 after Phase 1). They touch disjoint surfaces (CLI
  args/commands, DMLS overlay/workspace, about catalog).
- **Within Phase 2**, the three **[P]** discovery-mechanic tasks (walk,
  enumeration, path normalization) are independent and can be built
  concurrently before the shadowing/registry tasks that consume them.
- Everything funnels through **Phase 4** — the single library assembly seam both
  surfaces consume, per the spec's "implemented once" mandate.
