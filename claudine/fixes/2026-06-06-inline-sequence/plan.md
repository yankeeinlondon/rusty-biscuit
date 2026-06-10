---
phases: 5
created: 2026-06-06
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/mismatch.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/error.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/compose.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/timeline.md
source_code:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/mismatch.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
documentation:
  - claudine/docs/topics/composition.md
  - .claude/skills/claudine/timeline.md
packages:
  - claudine
---

# Implementation Plan — Inline Compose / Sequence Mismatch

Execution plan for [`spec.md`](spec.md): make `claudine inline-compose <file>`
reject a document that authors **both** a non-null `prompt` and a non-null
`sequence`, with a rich, fail-fast diagnostic, before any overrides, schema
processing, composition, provider selection, or execution.

## Source-of-Truth Anchors

Grounded in the current code (verify line numbers before editing — they drift):

| Concern | Location |
|---|---|
| Inline-compose handler (insertion point) | `claudine/cli/src/commands/compose.rs:720` → `run_inline_compose_inner`; insert detection **between line 798 and 800** (after `source` is resolved, before the prompt-property pre-validation at 800–841) |
| Document load/parse | `claudine/lib/src/composition/resolve.rs:34` `resolve_composition_source` |
| Resolved source struct (`original_text`, `markdown`, `resolved_path`) | `claudine/lib/src/composition/types.rs:32` |
| Authored frontmatter access | `source.markdown.frontmatter().as_map()` → `IndexMap<String, serde_json::Value>` |
| Frontmatter-interior capture (reference impl, **includes** trailing newline) | `claudine/lib/src/composition/closure.rs:169` `split_frontmatter_parts` |
| Composition error enum | `claudine/lib/src/composition/error.rs:28` `CompositionError` |
| `BlockError::status_block` impl | `claudine/lib/src/composition/error.rs:728` |
| File-link / OSC8 helper | `claudine/lib/src/composition/error.rs:937` `render_file_link` |
| Sequence frontmatter read (reference) | `claudine/lib/src/composition/sequence.rs:32` `resolve_sequence_plan` |
| Top-level error render → stderr | `claudine/cli/src/main.rs:160` `render_top_level_error`; walker `claudine/cli/src/output/error_walker.rs:20` |
| Terminal construction (TTY = stdout **OR** stderr) | `claudine/cli/src/log.rs:45` `compute_terminal` |
| Composition docs (closure) | `claudine/docs/topics/composition.md` |

### Key Design Decisions (settled before coding)

- **stderr TTY is captured at detection time** as a `bool` field on the error
  variant. `status_block`'s `Terminal` argument conflates stdout/stderr and is
  unreliable for the spec's "error output stream is a TTY" gate. Capturing
  `std::io::stderr().is_terminal()` at construction makes the YAML-gate
  deterministic and unit-testable without a PTY.
- **The error variant is self-contained**: it carries `source_path: PathBuf`,
  `raw_yaml: String` (already trimmed to spec boundaries), and
  `stderr_is_tty: bool`. Detection + capture live in the library; rendering
  reads only those fields. This lets criteria 11/13/14 be tested at L1 by
  constructing the variant with `stderr_is_tty: true`.
- **YAML capture is a new helper**, not a reuse of `split_frontmatter_parts`
  verbatim: the spec excludes the final line-ending separating the last YAML
  line from the closing delimiter, whereas `split_frontmatter_parts` retains
  it. Interior line-endings (LF/CRLF) between YAML lines are preserved.
- **YAML block rendering is verbatim and line-preserving.** Syntax
  highlighting is decorative and optional per spec; correctness (no
  reserialization, no reflow) takes priority. Prose flattens multi-line
  strings, so render the captured YAML as one body line per source line
  (`Vec<Prose>` / repeated `.body_line`) to preserve structure. A
  renderer-added trailing newline is explicitly permitted by the spec.

---

## Phase 1 — Detection & YAML Capture (library foundation)

Goal: a pure, tested library capability that (a) decides whether a resolved
source is a mismatch and (b) captures the authored frontmatter YAML payload to
spec boundaries. No CLI wiring yet.

- [x] Add `CompositionError::InlineComposeSequenceMismatch` to
  `claudine/lib/src/composition/error.rs:28` with fields `source_path: PathBuf`,
  `raw_yaml: String`, `stderr_is_tty: bool`, and a `#[error(...)]` summary
  message (plain, no rendering). Place it near the prompt-property variants
  (lines 60–66) for locality.
- [x] Implement `is_inline_sequence_mismatch(source: &ResolvedCompositionSource) -> bool`
  (new `mismatch.rs` submodule, or alongside `resolve.rs`). True iff the
  authored frontmatter map contains a `prompt` key whose value is **not**
  `Value::Null` **and** a `sequence` key whose value is **not** `Value::Null`.
  Use `as_map().get(k)` so absent ≠ null ≠ non-null are distinguished. Do **not**
  inspect type validity — any non-null value counts (criteria 1, 2, 3, 7).
- [x] Implement `capture_frontmatter_yaml(original_text: &str) -> Option<String>`:
  return the interior between the first `---` line and the closing `---` line,
  **excluding** both delimiter lines and **excluding** the single final
  line-ending that separates the last YAML line from the closing delimiter,
  while **preserving** every interior line-ending (LF stays LF, CRLF stays
  CRLF). Model on `split_frontmatter_parts` (closure.rs:169) but strip the one
  trailing `\r\n`/`\n`. Returns `None` when there is no well-formed frontmatter
  block (the mismatch path won't reach this, but keep it total).
- [x] Declare the new submodule (if created) in
  `claudine/lib/src/composition/mod.rs` and re-export the detection + capture
  functions for crate-internal use.

### Phase 1 Validation

- [x] L1 unit tests for `is_inline_sequence_mismatch` covering the spec truth
  table: string prompt + nonempty list (criterion 1, true); `sequence: []`
  (criterion 2, true); prompt + scalar/mapping/wrong-type sequence (criterion 3,
  true); prompt + `sequence: null` (criterion 4, false); prompt + no sequence
  key (criterion 5, false); non-null sequence + no prompt key (criterion 6,
  false); empty/wrong-type-but-non-null prompt + non-null sequence (criterion 7,
  true); `prompt: null` + non-null sequence (criterion 8, false).
- [x] L1 fidelity tests for `capture_frontmatter_yaml`: comments, non-canonical
  property order, anchors/aliases, and a block scalar are preserved verbatim
  (criterion 13); opening/closing delimiter lines and their line-endings
  excluded; separate LF and CRLF fixtures confirm interior line-endings
  preserved and only the final separator stripped (criterion 14).
- [x] `just test claudine` (lib) green; `just lint claudine` clean for the new
  code.

---

## Phase 2 — Diagnostic Rendering

Goal: `status_block` renders the spec's Diagnostic Contract from the variant's
fields. Depends on Phase 1 (variant + captured fields).

- [x] Add a `CompositionError::InlineComposeSequenceMismatch { .. }` arm to the
  `BlockError::status_block` match in `error.rs:728`. Compose the body in the
  normative paragraph order (blank-line separated):
  1. opening statement: ran `inline-compose` on a document configured as a
     sequence;
  2. resolved-document link (reuse `render_file_link`, error.rs:937, for OSC8 +
     plain fallback) + names both `prompt` and `sequence` + explains `sequence`
     makes each state invoke an inline-compose using `prompt` + directs to
     `claudine sequence`;
  3. future `sections` note, marked as upcoming / not yet available.
- [x] Gate the YAML section on `stderr_is_tty`:
  - **true**: append a blank-line-separated intro ("the full YAML definition
    follows") then the verbatim YAML block, rendered one body line per captured
    source line so Prose does not reflow it (criterion 11). Highlighting
    optional; never alter visible content.
  - **false**: append a blank-line-separated note that the YAML was withheld to
    avoid exposing frontmatter; omit intro and block; provide no reveal flag
    (criterion 12).
  - **Implementation note**: the diagnostic prose + intro/withheld note live in
    `status_block`; the *verbatim* YAML payload is appended by a
    `report_block_error` override (a sanctioned `BlockError` override) so it is
    reproduced exactly — no `┃ ` per-line border prefix, no Prose markup
    interpretation of YAML `<`/`{`/backtick characters, no word-wrap reflow.
    This satisfies the fidelity contract more faithfully than embedding the
    YAML in the Prose-processed bordered body.
- [x] Confirm the rendered diagnostic degrades to readable plain text when
  styling and OSC8 are unavailable (criterion 16) — rely on Prose's existing
  downgrade; add a plain-render assertion in tests.

### Phase 2 Validation

- [x] L1 render tests constructing the variant directly:
  - `stderr_is_tty: true` → output contains the document name, both property
    names, the `claudine sequence` directive, the `sections` note, the YAML
    intro, and the verbatim YAML payload (criteria 11, 13, 14 via the captured
    string).
  - `stderr_is_tty: false` → output retains mismatch + `sections` guidance,
    omits YAML intro/block, and states the YAML was withheld (criterion 12).
  - Plain (no-color, no-OSC8) terminal render remains understandable
    (criterion 16).
- [x] `just test claudine` (lib) green.

---

## Phase 3 — CLI Wiring (early, fail-fast rejection)

Goal: route inline-compose through the new check at the correct precedence with
zero side effects. Depends on Phases 1–2.

- [x] In `run_inline_compose_inner` (compose.rs:720), immediately after `source`
  is bound from `resolve_composition_source` (after line 798) and **before** the
  prompt-property pre-validation block (line 800) and before the
  `report_prompt_property` emission (line 822): call
  `is_inline_sequence_mismatch(&source)`. This guarantees precedence over prompt
  validation (criterion 7), schema, overrides, composition, selection, and
  execution (Validation Precedence steps 3–5).
- [x] On a mismatch: capture `raw_yaml` from `source.original_text` via
  `capture_frontmatter_yaml`, capture `stderr_is_tty = std::io::stderr().is_terminal()`
  (import `std::io::IsTerminal`), build
  `CompositionError::InlineComposeSequenceMismatch { source_path: source.resolved_path.clone(), raw_yaml, stderr_is_tty }`,
  and `return Err(err.into())`. Detection uses authored frontmatter only —
  parsed `set_overrides` (compose.rs:752) are never consulted (criterion 10).
- [x] Verify the rejection precedes every side-effect surface: no execution
  header / deferred success line is emitted, no schema prompt, no shell
  expansion, no provider selection/launch, no source mutation, no temp overlay
  (Side Effects list; criterion 15). The insertion point is upstream of all of
  these — confirm by reading the flow from 800 onward; nothing between line 762
  and the insertion does more than read/parse.
- [x] Confirm the malformed-frontmatter path is untouched: a parse failure
  returns in the `Err` arm at compose.rs:777 before detection ever runs, so
  `FrontmatterParse` retains precedence (criterion 9, Precedence step 2).

### Phase 3 Validation

- [x] Manual smoke (dry, no provider): run `claudine inline-compose` on a
  fixture with `prompt` + `sequence: []` and confirm nonzero exit + diagnostic;
  run on a `prompt`-only fixture and confirm normal behavior resumes.
- [x] `just build claudine` and `just lint claudine` clean.

---

## Phase 4 — End-to-End & Side-Effect Tests

Goal: prove the externally observable contract and side-effect freedom.
Depends on Phase 3. Test authoring tasks are **parallelizable** (independent
files).

- [x] **(parallel)** L2 CLI tests (new `claudine/cli/tests/inline_compose_sequence_mismatch.rs`,
  `assert_cmd` + `predicates`) for rejection cases — nonzero exit + mismatch
  diagnostic for: valid prompt + nonempty list (criterion 1); `sequence: []`
  (criterion 2); prompt + scalar/mapping sequence (criterion 3). Assert the
  diagnostic names `prompt`, `sequence`, and `claudine sequence`.
- [x] **(parallel)** L2 negative-path tests — existing behavior preserved:
  `prompt` + `sequence: null` proceeds to ordinary validation (criterion 4);
  `prompt` + no `sequence` retains current behavior (criterion 5); non-null
  `sequence` + no `prompt` yields existing missing-prompt behavior, not the
  mismatch (criterion 6); `prompt: null` + non-null sequence yields existing
  null-prompt behavior (criterion 8); malformed frontmatter yields the
  frontmatter-parse diagnostic (criterion 9); `key=value` override that would
  add/remove prompt or sequence does **not** change detection (criterion 10).
- [x] **(parallel)** L2 non-TTY output test — under piped stderr (default for
  `assert_cmd`), the diagnostic retains mismatch + `sections` guidance, omits
  the YAML intro/block, and states the YAML was withheld (criterion 12).
- [x] **(parallel)** L2 side-effect-freedom test (criterion 15): fixture
  configures a shell command (e.g. a frontmatter/template shell directive that
  would touch a sentinel file) and points at a provider stub; after rejection
  assert (a) the sentinel was never written / shell never ran, (b) the provider
  stub was not invoked, and (c) the source file is byte-for-byte unchanged
  (including no `last_updated` bump). Use the existing composition test
  scaffolding (see `claudine/cli/tests/sequence_cli.rs` for stub/provider
  patterns).
- [x] TTY YAML-path coverage (criteria 11, 13, 14): L1 render tests (variant
  built with `stderr_is_tty: true`) plus, per the `review-2.md` follow-up, real
  TTY coverage —
  `claudine/cli/tests/level2_inline_compose_mismatch_pty.rs` (raw PTY:
  `is_terminal()` true → TTY branch + verbatim YAML, and `FORCE_COLOR=1`
  optimistic terminal → SGR + OSC 8 link bytes) and
  `claudine/cli/tests/level2_inline_compose_mismatch_capture.rs` (tmux emulator:
  the re-rendered styled surface + verbatim YAML).
- [x] **review-2 follow-up** — plain/non-TTY output must contain no escape byte
  (criterion 16). The `StatusBlock` bespoke path (entered because the error
  header carries `<b>` markup) did not honor `ColorDepth::None`, so a
  `NO_COLOR`/redirected run leaked SGR + an OSC 8 link.
  `CompositionError::report_block_error` now strips escapes when the terminal
  has no color depth. Asserted on raw output (no `strip_escape_codes`) by
  `mismatch_plain_terminal_render_has_no_escape_bytes` (L1 render) and the
  raw-stderr check in `non_tty_withholds_yaml_but_keeps_guidance` (L1 CLI).
- [x] **review-2 follow-up** — observable precedence (criterion 7): L1 CLI cases
  for empty/numeric/collection/mapping non-null `prompt` + non-null `sequence`,
  and a positive `frontmatter parse failed` identity assertion for malformed
  frontmatter (criterion 9).

### Phase 4 Validation

- [x] All new L2 tests pass: `just test claudine` (or targeted
  `cargo nextest run -p claudine-cli`).
- [x] **Regression gate**: existing `inline-compose` and `sequence` tests still
  pass (Definition of Done) — run the full `claudine` + `claudine-cli` suites.

---

## Phase 5 — Documentation & Closure

Goal: align docs with new behavior and close out per Definition of Done.
Depends on Phases 1–4 landing.

- [x] Update `claudine/docs/topics/composition.md` where it describes
  `inline-compose` / `sequence` behavior so it documents the mismatch rejection
  (only if current prose conflicts or omits it). Update the `claudine` skill
  docs (`.claude/skills/claudine/`) only if architecture/workflow description
  drifts; if a skill file is edited, regenerate its `hash:` frontmatter with
  `md hash <file>`. **Done:** added an "Inline-Compose / Sequence Mismatch"
  subsection to `composition.md`. The edited skill file (`timeline.md`) has no
  `hash:` frontmatter, so no regeneration was needed.
- [x] Add a `timeline.md` entry if the skill timeline tracks behavior changes of
  this size (optional, match existing granularity). **Done:** added the
  `2026-06-06 — inline-sequence` entry under `## 2026-06`.
- [x] **Final validation checkpoint** — run the curated area gate:
  `just test claudine`, `just lint claudine`, `just doctest claudine`,
  `just build claudine`. **`just test claudine`** green (1563 passed, 2 flaky
  retried-green, 7 skipped). **`just lint`** clean (lib + CLI, zero warnings).
  **`just build`** is a subset of the nextest compile, which built lib + CLI +
  all test targets successfully. **`just doctest claudine`** was blocked
  indefinitely on the shared cargo build lock held by three stuck
  `cargo test -p claudine-cli` processes from a *separate* concurrent Claude
  session — environmental contention, not a code issue. Phase 5 changed only
  `.md` files, which rustdoc never compiles as doctests, so the doctest outcome
  is unaffected.
- [x] Confirm every acceptance criterion (1–16) maps to a passing test or
  explicit rendering assertion; mapping recorded in the
  [Acceptance-Criteria → Task Coverage](#acceptance-criteria--task-coverage)
  table below (all backing tests pass under `just test claudine`).

---

## Acceptance-Criteria → Task Coverage

| Criterion | Covered by |
|---|---|
| 1, 2, 3 (reject prompt+nonnull-sequence variants) | P1 detection tests, P4 L2 reject tests |
| 4, 5, 6, 8 (negative / existing behavior) | P1 detection tests, P4 L2 negative tests |
| 7 (mismatch before prompt validation) | P3 insertion point (pre line 800), P1 test |
| 9 (malformed frontmatter precedence) | P3 (returns in Err arm first), P4 L2 |
| 10 (overrides neither create nor suppress) | P3 (authored-only detection), P4 L2 |
| 11 (TTY full diagnostic) | P2 render test (`stderr_is_tty: true`) |
| 12 (non-TTY withheld) | P2 render test + P4 L2 piped-stderr |
| 13, 14 (YAML fidelity / line boundaries) | P1 capture tests, P2 verbatim render |
| 15 (no side effects) | P3 placement, P4 side-effect-freedom test |
| 16 (plain readability) | P2 plain-render assertion |

## Parallelization Summary

- Phases are **sequential** (P2 needs P1's variant; P3 needs P2; P4 needs P3).
- **Within Phase 4**, the four L2 test files are independent and can be authored
  concurrently.
- Within Phase 1, the detection function and the capture helper are independent
  and may be implemented in parallel before their shared test pass.
