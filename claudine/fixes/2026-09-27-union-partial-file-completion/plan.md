---
created: 2026-09-27
total_phases: 5
phase: 1
agent: claude/opus
yolo: true
spec: 2026-09-27-union-partial-file-completion
source_files_during_phase_1:
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
---

# Plan: union partial file completion, early failure, and focused excerpts

## Summary and Success Criteria

### The work

The spec reports five defects (C1–C5) on one user path:
`claudine compose prompts/clarify.md spec=fix`. The work falls into four
independent technical tracks that meet in Claudine's compose preparation:

1. **Arm selection (C1 → R1, R2/D1).** `supplied_file_shape`
   (`lib/src/composition/schema/supplied.rs:119`) validates each union arm
   against *raw* frontmatter. It must apply the same composition-tolerant rule
   the pre-validator uses (`schema/mod.rs:607-636`), through one shared
   predicate. When no unique arm is selected, a new fallback lets the
   existence check run anyway if every arm that declares the property types it
   as an eager `file(match)`. The chooser then searches the de-duplicated union
   of those arms' patterns.
2. **Ordering and base directory (C2, C3 → R2, R3, R4).** Decidable input
   failures must be reported before the provider picker. The late verdict must
   name the caller's origin for caller-owned values, not the document
   directory.
3. **Inline provider picker (C4 → R5).** `prompt_one_shot_provider`
   (`cli/src/commands/wrap/selection_ui.rs:40`) passes `None` (alternate
   screen) to `run_standalone`. It must pass a bounded inline height, sized the
   same way the `schema_interactive` choosers size theirs.
4. **Focused excerpts (C5 → R6).** `FrontmatterExcerpt`
   (`lib/src/composition/frontmatter_excerpt.rs`) always captures the whole
   block. It must render only the focused regions (±3 lines, with ancestors),
   locate caller-input properties inside `$schema` union arms, and omit the
   excerpt when nothing is locatable. This removes `FrontmatterHighlight::BlockOnly`.
   The work touches `biscuit-terminal` (locator) and Darkmatter (`CodeBlock`
   line-number offset; see ruling **N1**).

### Packages touched

`claudine` (lib), `claudine-cli`, `biscuit-terminal` (lib), and `darkmatter`
(lib; added by ruling N1, since the spec's `packages` list does not include it).

### Definition of done

- [ ] `claudine compose prompts/clarify.md spec=fix` (union schema, `doc:
      "{{spec || design}}"`, `initialize` stack) opens the glob+substring
      chooser or the `Use this file? (Y/n)` confirmation **before** any other
      prompt, and composes with the chosen path.
- [ ] With zero candidates, a declined chooser, or a non-interactive run, the
      `no existing file matched reference` error prints **before** the
      provider picker, and the picker never renders.
- [ ] The D1 two-arm `features`/`fixes` shape lists candidates from both trees.
      Picking a `fixes` file makes the composed frontmatter validate against
      the `fixes` arm.
- [ ] Every unresolved caller-supplied file reference names the caller's
      launch directory. The late verdict no longer reports `…/prompts` for
      `spec=fix` typed at the repo root.
- [ ] The provider picker renders inline below the cursor (no `\x1b[?1049h`)
      and leaves scrollback intact.
- [ ] Frontmatter excerpts show only focused regions with real line numbers,
      `⋮` elision, and highlights. No `BlockOnly` path remains. Unlocatable
      problems omit the excerpt.
- [ ] Every R7 test (1–8) exists and passes. The existing `supplied.rs` tests
      pass unchanged. Every changed snapshot or L1 expectation is listed with
      its reason in the Phase 5 changed-expectations table.
- [ ] `just test` and `just lint` are green in `claudine/`, `biscuit-terminal/`,
      and `darkmatter/`. The Windows compile surface is unchanged in shape: no
      new `#[cfg(unix)]` except on PTY tests, which already follow that
      convention.
- [ ] The docs listed in Phase 5 have been updated to match.

---

## Phase 1 — Rulings, Reproduction, and Spikes

The goal is to lock the open design points, reproduce C1 on today's code
(R7.1: reproduce first), and remove the unknowns from later phases.

### Necessary Rules

These rulings are proposed by the planner. Each is needed before
implementation can proceed with confidence. The author should confirm or
overrule them before Phase 2 starts. Under `yolo: true`, implementation
proceeds on the stated default.

- [x] **N1 — Line-number offset lives in Darkmatter's `CodeBlock`.**
      `FrontmatterExcerpt` renders through `darkmatter::markdown::CodeBlock`,
      and `CodeBlockMeta` (`darkmatter/lib/src/markdown/dsl/mod.rs:45-53`) has
      no starting-line field. Per R6 ("add that capability to the renderer
      rather than faking numbers"), add `start_line: Option<usize>` (default
      `None` = 1) to `CodeBlockMeta`, honored by both the terminal renderer
      (`markdown/code_block.rs`) and `markdown/output/code_block.rs`.
      `HighlightSpec` line numbers are interpreted in the same absolute
      numbering. **Default:** do this, and add `darkmatter` to the spec's
      `packages`. **Rejected:** rendering through `SourceContext`'s
      text-gutter prose. It fakes numbers in text and loses the syntax
      highlighting R6 requires Claudine to keep.
- [x] **N2 — Non-contiguous regions render as one `CodeBlock` per region.**
      Regions are joined by a single `⋮` elision line in the gutter column.
      Rendering stays in Claudine's `FrontmatterExcerpt::render_appendix`, so
      TTY/`NO_COLOR` gating and highlighting are unchanged. `biscuit-terminal`
      supplies *line selection* only (N3).
- [x] **N3 — `biscuit-terminal` gains a pure, non-falling-back locator.**
      Add a public `SourceContext` method (working name
      `focused_yaml_regions(&self, keys: &[YamlKeyPath], context: usize) ->
      Option<Vec<FocusedRegion>>`). Each region is a sorted, merged set of
      1-based source lines, carrying the highlighted target lines and the
      ancestor lines. It returns `None` when nothing resolves; it never falls
      back to the whole block. `focused_yaml_excerpt` is reimplemented on top
      of it, keeping its current fallback for its existing callers.
      `locate_key_region` learns **sequence-item arms**: a path segment that
      matches a `- key:` item inside a sequence, so `$schema` → every arm's
      `spec` is found. A `YamlKeyPath` with a wildcard-arm segment (working
      name `YamlKeyPath::in_every_arm("$schema", "spec")`) expresses
      "the key in every arm".
      - **Note from Phase 1:** `locate_key_region`
        (`biscuit-terminal/lib/src/errors/source_context.rs:237`) fixes one
        indentation per level from the level's first meaningful line. In a
        sequence item, `  - spec: …` has indent 2 but its sibling `    doc:
        file` has indent 4. The arm-aware walk must treat the key after a
        `- ` marker as sitting at `indent + 2` (the marker's column plus its
        width), and treat each item as its own scope.
- [x] **N4 — Context width and ancestor rule.** A named constant
      `EXCERPT_CONTEXT_LINES: usize = 3` lives in Claudine's
      `frontmatter_excerpt.rs` (D2) and is passed to the locator. Ancestor
      header lines are always shown even when they fall outside the ±3 window.
      The "≤7 lines around it" bound in R7.8 counts window lines only, not
      ancestors. When the merged regions cover every line of the block, the
      whole block renders; that is the one allowed exception.
- [x] **N5 — One shared composition-tolerance predicate.** Extract from
      `schema/mod.rs:619-636` a `pub(super) fn
      is_composition_independent(problem: &ValidationProblem, instance:
      &serde_json::Value) -> bool`. It returns true for `Missing`, and for
      `Type`/`Invalid` it returns `!value_needs_composition(raw)` for the
      problem's top-level property. The pre-validator keeps its extra
      `caller_resolved_eager_files` exclusion layered *on top*.
      `supplied_file_shape` switches from `validate(..).valid` to "no problem
      survives the predicate". The rule is not copied.
- [x] **N6 — D1 fallback scope.** The fallback runs when tolerant selection
      yields **zero or several** arms. It applies per caller-supplied property,
      and only when every arm that declares the property declares it as an
      eager `file` with a non-empty `match`, with the same `is_array`. If any
      arm declares it as `string` (or another type), or omits `eager`, there is
      no fallback. `supplied_file_shape` itself is unchanged in contract and
      still returns `None` for conflicting discriminants and string-only
      alternatives (R7.5). The fallback is a separate function, so the
      existing tests keep passing unchanged. Patterns are de-duplicated in
      arm order, then pattern order.
- [x] **N7 — R4 is fixed in Claudine, not Darkmatter.** Darkmatter's
      `while resolving from <dir>` (`darkmatter/.../schemas/format.rs:337`)
      correctly reports the base it was given. For a `NoMatch` problem on a
      property that has a caller record, Claudine re-resolves the value
      against `record.origin()` and emits the typed `UnresolvedFileReference`
      with the caller's base directory. This is the same shape the early pass
      emits, and it extends `classify_unresolved_file_reference`
      (`classify.rs:220`) to unions using the N6 arm/pattern rule. **Fallback:**
      if spike S2 finds that the late path has lost the caller records, thread
      them through. Do not change Darkmatter's message.
      - **Confirmed by S2:** the records are reachable at
        `translate_schema_failure` (`schema/translate.rs:75`) as
        `options.caller_input_records`, so nothing needs threading on the main
        late path. Only `post_shell_validate` (`schema/mod.rs:221`) lacks them.
- [x] **N8 — Picker height.** Promote `schema_interactive`'s chooser sizing
      (options + chrome, capped) into one shared `pub(crate)` helper and use it
      from both `schema_interactive` and `prompt_one_shot_provider`. There is
      no new cap value; the existing one is reused.
      - **Amended in Phase 1 (S3):** no sizing function or cap exists today.
        `inline_height(rows)` (`schema_interactive/mod.rs:604`) only wraps
        `HeightSpec::Cells(rows)`, and every call site hard-codes its rows
        (`ChooseOne`/`ChooseMany` use 8). `HeightSpec::Cells(n)` is clamped
        only to the terminal height (`HeightSpec::resolve`,
        `biscuit-tui/lib/src/core/frame.rs`), so the viewport reserves `n`
        rows whatever the option count. The shared helper is therefore
        `pub(crate) fn chooser_height(option_count: usize) -> Option<HeightSpec>`,
        which returns `min(option_count + CHOOSER_CHROME_ROWS,
        CHOOSER_MAX_ROWS)` with `CHOOSER_MAX_ROWS = 8`. That names the
        existing hard-coded 8 as the cap; it is not a new value. Phase 2
        derives `CHOOSER_CHROME_ROWS` from what `ChooseOne` actually draws
        besides its rows (check the component; do not guess). Consequence:
        schema enum choosers with fewer than 8 options shrink to fit. This is
        the "sized to option count plus chrome, capped" behavior that R5
        names. Any L1 PTY expectation that depends on the old fixed 8 rows
        goes into the Phase 5 changed-expectations table.
- [x] **N9 — R3 ordering for `initialize`-authoring documents.**
      Required-missing collection for documents that author `initialize` stays
      *after* `initialize` (unchanged; `initialize` may supply those values).
      Only the supplied-file existence check (R1/R2) is guaranteed to run
      pre-picker for them. For documents without `initialize`, the existing
      pre-picker `pre_validate_with_interactive_collection` order is kept, and
      Phase 3 asserts it.
      - **Confirmed by S1:** every non-success outcome of the supplied-file
        pass already returns an error before target selection
        (`prep.rs:193`, and `:437` for proxied targets). The C2 gap exists
        only because the pass returns an empty pending list for unions (C1).

### Wave 1 — Reproduction and spikes (all parallel)

- [x] **Reproduce C1 (R7.1)**
    - Add `union_partial_with_templated_file_sibling_reaches_chooser` beside
      the cases in `cli/tests/l1/level1_provided_partial_file_pty.rs`. Reuse
      its `seed_specs`, PTY helpers, and `CliProcessFixture` (no raw spawns;
      see `spawn_site_guard.rs`).
    - Fixture: a union `$schema` whose arms are `spec: file(required;
      match(**/*spec*.md); eager)` and `design: file(required;
      match(**/*design*.md))`, each with `doc: file`. Frontmatter has
      `doc: "{{spec || design}}"` and a shell-free `initialize` stack. Pass
      `spec=everywhere --claude` with the provider stub.
    - Assert the fixed behavior: `Use this file? (Y/n)` appears, and the stub
      launches with the resolved path. Run it on today's code and **record the
      failure output in this plan** under "Reproduction record" below. The
      expected red is: no chooser, `no existing file matched reference`
      printed after the stub selection.
    - Leave the test in place and red. Phase 2 turns it green. Do not
      `#[ignore]` it; the branch is not merged between phases.
- [x] **S1 — Order map (spike)**
    - Trace the exact order, for `compose` and `inline-compose`, with and
      without `initialize`, of: `resolve_supplied_file_inputs`
      (`prep.rs:186`, `:430`), `pre_validate_with_interactive_collection`,
      provider selection (`wrap/composition/target.rs:471`), `initialize`,
      and the staged-boot verdict.
    - Confirm what `resolve_supplied_file_inputs_with`
      (`cli/src/commands/schema_interactive/supplied.rs:70`) does on zero
      candidates, decline, cancel, and non-interactive. Confirm each returns
      an error (not a pass-through) before target selection.
    - Output: a short ordered list appended to this plan under "Spike
      findings". It feeds Phase 3.
- [x] **S2 — Late-verdict path (spike)**
    - Find where the post-`initialize` `SchemaValidation` for C3 is produced
      (staged boot → Darkmatter `format.rs:337`). Determine whether caller
      input records (with their origins) are reachable there.
    - Output: the function to change for N7, and whether records must be
      threaded through.
- [x] **S3 — Inline `ChooseOne` safety (spike)**
    - Confirm that `biscuit_tui::run_standalone` with `Some(HeightSpec::Cells(n))`
      never emits `\x1b[?1049h`, and restores the cursor and scrollback on
      submit, `Esc`, and `Ctrl-C` (`biscuit-tui/lib/src/core/standalone/mod.rs:167`).
      Find the existing chooser sizing function in
      `cli/src/commands/schema_interactive/mod.rs` for N8.
- [x] **S4 — Excerpt expectation inventory (spike)**
    - List every test and snapshot that asserts excerpt content or whole-block
      output. Known candidates: `lib/src/composition/frontmatter_excerpt.rs`
      tests, `lib/src/composition/error/tests.rs`,
      `cli/tests/l1/{wrap_compose_validation,compose_schema_cli,level1_schema_prompt_pty,level1_inline_compose_mismatch_pty}.rs`,
      and `cli/tests/level2/level2_malformed_frontmatter_capture.rs`. Also
      list any `insta` snapshots.
    - List every `FrontmatterHighlight` producer (`error/mod.rs:3203-3335`)
      and classify the new behavior of each: focused, windowed line, or
      omitted.
    - Output: a table for Phase 4 and Phase 5.

### Checkpoint 1

- [x] N1–N9 are confirmed or amended (record amendments inline).
- [x] The R7.1 test exists and fails the recorded way.
- [x] S1–S4 findings are appended to this plan, with no open unknown about
      where each change lands.

#### Reproduction record

Recorded 2026-09-27 on `feat/schema-enhancement` (debug build, macOS).

- **Test:** `union_partial_with_templated_file_sibling_reaches_chooser` in
  `cli/tests/l1/level1_provided_partial_file_pty.rs`. It is left red on
  purpose; Phase 2 (R1) turns it green.
- **Deviations from the task text:** the test uses `--goose`, not `--claude`,
  so it shares `compose_command` and the stub conventions with the file's
  existing tests. The stub is a new local `stage_recording_goose_stub`: it
  writes its argv (and stdin, when stdin is not the terminal) to the marker,
  so the test can assert that the chosen path reached the provider. The body
  is `Spec document: {{spec}}`, not `{{doc}}`, because a manual run showed
  that `{{doc}}` in a body renders the whole context object rather than the
  `doc` property. The fixture keeps `doc: "{{spec || design}}"` in the
  frontmatter, which is the trigger.
- **Manual check of the stub:** with a single (non-union) schema and a
  literal `spec=` path, the recording stub received `run -t "<prompt>"`, and
  the prompt carried the resolved absolute spec path. The test's final
  assertion therefore checks something real.
- **Observed (red):** no chooser. `wait_for_marker("Use this file")` timed out,
  and the transcript was:

  ```text
  Claudine ▸ Goose  Compose  prompt sourced from …/cwd/plan.md

  ⤫ CompositionError: schema validation
  ┃ Schema validation failed for …/cwd/plan.md.
  ┃ /spec: no existing file matched reference `everywhere` while resolving
  ┃ from `…/cwd`
  ┃ Problems:
  ┃ - `/spec`
       yaml
   1 │ ---
   2 │ $schema:
   …  (all 11 frontmatter lines, nothing highlighted)
  11 │ ---
  ```

  This matches the expected red: no chooser, and `no existing file matched
  reference` printed after provider selection (here `--goose` pre-selects the
  provider). It also shows C5: the whole block renders with no highlight.
  C3 does not show here, because the document sits in the launch directory.
  R7.6 in Phase 3 needs the document under `prompts/` to expose it.
- The four existing tests in the file still pass.

#### Spike findings

##### S1 — Order map

`compose` and `inline-compose` share one path: `run_composition_inner` →
`prepare_and_run_active_document` (`cli/src/commands/compose/prep.rs`).
`inline-compose` adds only the steps marked **[inline]**. "Authors
`initialize`" means `staged_boot::authors_initialize`
(`wrap/composition/staged_boot.rs:63`), a check for an `initialize` key.
Across every mode, the supplied-file pass finishes before provider selection.
Only `initialize` and the final schema verdict move.

**A. No `initialize` (first document):**

1. **[inline]** `kind.on_source_resolved` (`prep.rs:168`).
2. `resolve_supplied_file_inputs` (`prep.rs:186`). An error returns at `:193`.
3. `pre_validate_with_interactive_collection` (`prep.rs:198`, its only call
   site). This collects missing values. Its own unresolved-partial fallback
   is at `schema_interactive/mod.rs:131`.
4. **[inline]** `finalize_inline_prompt_state` (`prep.rs:222`).
5. Schema stage `Validate` (`prep.rs:470`).
6. `eagerly_resolve_target` (`prep.rs:514`) → `prompt_for_agent_state`
   (`target.rs:352`) → `prompt_one_shot_provider` (`target.rs:471`). The
   picker needs a TTY stderr, no `--provider`, and no dry run.
7. **[inline]** `on_header_emitted` (`prep.rs:569`).
8. Eager shell preflight (`prep.rs:622`).
9. Schema verdict: the single route is `prepare_collecting_missing(..,
   Validate)` (`prep.rs:1466`); the loop route prepares iteration 1 with
   `defer_schema_verdict=false` (`prep.rs:1331`).

**B. With `initialize` (live run):**

1. **[inline]** `on_source_resolved` (`prep.rs:168`).
2. `resolve_supplied_file_inputs` (`prep.rs:186`).
3. `pre_validate_with_interactive_collection` is **skipped**
   (`prep.rs:194-195`).
4. **[inline]** `finalize_inline_prompt_state` (`prep.rs:222`).
5. Schema stage `DeferToStabilizedReread`, `staged=true` (`prep.rs:470`, `:483`).
6. `eagerly_resolve_target` and the provider picker (`prep.rs:514`).
7. **[inline]** `on_header_emitted` (`prep.rs:569`).
8. `staged_boot::bootstrap_document` (`prep.rs:1348`): a shell-free read with
   no verdict.
9. `initialize`: the single route is `route_staged_initialize`
   (`prep.rs:1661`, inside `run_staged_single`); the loop route runs it in
   the engine inside `run_loop_with_overrides` (`prep.rs:961`).
10. Verdict: the single route is `reread_and_audit` (`prep.rs:1681`), then
    `prepare_collecting_missing(.., Validate)` (`:1690-1695`); the loop route
    is `reread_and_audit` (`prep.rs:1009`), then `prepare_staged` (~`:1019`).
    Failures route through `route_stabilized_failure`.

A dry run with `initialize` keeps the stage at `Defer`, and `initialize`
never fires. Proxied targets (`first=false`) re-run
`resolve_supplied_file_inputs` at `prep.rs:430`, still before `prep.rs:514`.

**`resolve_supplied_file_inputs_with` outcomes**
(`schema_interactive/supplied.rs:70`):

| Case | Path | Result |
|---|---|---|
| Zero candidates | `choose_provided_file_reference` → `Ok(None)` (`mod.rs:302`) → `downgrade_to_schema_validation` (`supplied.rs:103`) | `Err(SchemaValidation)` |
| Declined | single candidate, `n`/`Esc` → `confirm_one_file` `Ok(false)` → downgrade | `Err(SchemaValidation)` |
| Cancelled | multi-candidate chooser `Esc`/`Ctrl-C` → I/O error, logged at debug (`supplied.rs:38-47`) → downgrade | `Err(SchemaValidation)` |
| Non-interactive | `!interactive.allowed()` (`supplied.rs:95`) → downgrade on the first pending item | `Err(SchemaValidation)` |

All four propagate through `?` at `prep.rs:193` (or `:437` for proxied
targets) before target selection, so nothing passes through silently. It
returns `Ok(())` only when `pending` is empty (`supplied.rs:78`).

**The actual C2 gap is C1.** For a root union with no selected arm,
`unresolved_supplied_files` returns an **empty** `Vec`
(`lib/.../schema/supplied.rs:43-45`), so the pass returns `Ok(())`. Without
`initialize`, `pre_validate`'s own fallback may catch the value. With
`initialize`, it reaches the post-`initialize` verdict. Once R1 and the N6
fallback return a non-empty pending list, the existing early-error plumbing
already satisfies R2 for every outcome. Phase 3's ordering task is therefore
mostly assertions (R7.3), not new plumbing.

**Open item for Phase 3:** `read_confirm_key`
(`cli/src/completion/autocomplete_ui.rs:136-146`) matches only
`y`/`n`/`Enter`/`Esc`. In raw mode `Ctrl-C` arrives as a key event, so the
single-candidate dialog probably ignores `Ctrl-C` and keeps waiting. This was
not run. Check it in Phase 3. If confirmed, fixing it is a small, adjacent
change: map `Ctrl-C` to "cancelled".

A second picker call site, `prompt_for_agent_state` at `target.rs:161`
(a request-level resolver), was not traced. The compose paths use
`target.rs:352`.

##### S2 — Late-verdict path

- **Where the late error is built.** `run_staged_single` (`prep.rs:1586`) →
  `route_staged_initialize` (`:1661`) → `reread_and_audit` (`:1681`) →
  `prepare_collecting_missing` (`prep.rs:1531`, `Validate`) →
  `CompositionKind::prepare_staged` (`compose/mod.rs:394`) → `prepare_document`
  (`lib/.../prepare/service.rs:118`) → `prepare_with_schema`
  (`schema/mod.rs:183`). Darkmatter validates during compose
  (`darkmatter/lib/src/markdown/compose/schema_validation.rs:307`). Claudine
  then runs `handle_compose_error` (`schema/translate.rs:45`) →
  **`translate_schema_failure` (`translate.rs:75`)**, which returns
  `build_schema_validation_error` (`translate.rs:257`) at `:132`. The spike
  found this by reading the code, not by running it.
- **Why Darkmatter names the document directory.** For the union,
  `project_root_arm_caller_values` (`schema_validation.rs:915`) fails to
  resolve `fix` from the caller's origin and rejects every arm.
  `collect_applicable_root_schema_fragments` returns `None`, and
  `resolve_caller_file_overrides` (`:580`) silently skips `spec`. The raw
  value is then resolved in document context, and the problem carries
  `caller_file == None`. N7 stands: Darkmatter reports the base it was given.
- **`classify_unresolved_file_reference`** (`classify.rs:212`) is called only
  from `pre_validate_schema_for_mode` (`schema/mod.rs:652`, `:668`). That runs
  only when there is no `initialize`, so it is **not on the late path**. For a
  root union it returns `None` at `classify.rs:218`. A property-level union
  also yields `None` (`atom_for_property`, `classify.rs:82`). It takes no
  caller records and copies `problem.message` into `reason`, so today it
  would repeat the document-directory text.
- **Caller records are reachable. No threading is needed on the main path.**
  The type is `darkmatter::markdown::compose::CallerInputRecord` (`origin()`
  at `darkmatter/.../compose/context/options.rs:47`), held in
  `CallerInputRecords = BTreeMap<String, CallerInputRecord>` (`:53`). It
  reaches `translate_schema_failure` as `options.caller_input_records`
  (`lib/.../prepare.rs:80`). The staged path fills it (`prep.rs:1311`), and
  `reread_and_audit` passes it through (`staged_boot.rs:316-330`).
- **Function to change for N7:** `translate_schema_failure`. Before the
  `build_schema_validation_error` returns at `:132` and `:153`, call an
  extended `classify_unresolved_file_reference` that also takes
  `&CallerInputRecords` and applies the N6 arm/pattern rule to unions. For a
  `NoMatch` on a property with a record, re-resolve `record.raw()` against
  `record.origin()`. Build the same `UnresolvedFileReference` shape that
  `schema/supplied.rs:97` builds, with `record.origin().base_dir()` in
  `reason`. Also route the existing pre-validator calls (`mod.rs:652`,
  `:668`) through the record-aware form, so the early and late surfaces agree.
- **Secondary sites that lack the records:** `handle_retry_error`
  (`translate.rs:198`) and `post_shell_validate` (`schema/mod.rs:221`,
  returning at `:294`). Compose should fail before `post_shell_validate`
  runs, so it is lower priority. If Phase 3 covers it, the records must be
  added to its parameters. This is the plan's "thread through" fallback,
  scoped to that one function.
- **The chooser does not reappear after `initialize`.**
  `prepare_collecting_missing` (`prep.rs:1541`) retries only
  `MissingProperties`. N7 fixes the message but reopens no dialog. The early
  pass (R1 + N6) is the only place completion happens. That is consistent
  with R2.

##### S3 — Inline `ChooseOne` safety

- `biscuit_tui::run_standalone(component, state, height: Option<HeightSpec>)`
  (`biscuit-tui/lib/src/core/standalone/mod.rs:167`) computes
  `fullscreen = height.is_none()` (`:205`). `EnterAlternateScreen` runs only
  when `fullscreen` is true (`terminal_lifecycle.rs`, `prepare_terminal_inner`).
  `Some(HeightSpec::Cells(n))` uses `Viewport::Inline` (`:237-259`), so it
  never emits `\x1b[?1049h`. `HeightSpec` is `Cells(u16)` (reserves that many rows, clamped only to the terminal height) or
  `Percent(u8)` (`core/frame.rs:280`).
- Cleanup is identical on submit, `Esc`, and `Ctrl-C`. `Ctrl-C` is a raw-mode
  key event (`is_ctrl_c`, `:414`), not a signal. `finalize_inline_viewport`
  (`inline_viewport.rs:17`) always runs; with the default chrome it clears the
  viewport and returns the cursor to the viewport's top-left (fzf-style). It
  then drains pending DSR replies, pops the kitty keyboard flags, and disables
  raw mode. `TerminalGuard` repeats the restore on panic. Scrollback is kept,
  because the inline viewport only uses rows below the cursor. `Esc` maps to
  `ABORTED_KIND`, `Ctrl-C` maps to `CANCELLED_KIND`, and no TTY on either
  stream gives `Err("no interactive terminal available")` (`:214`).
- **Chooser sizing (N8 input):** `schema_interactive/mod.rs:604` is a private
  `fn inline_height(rows: u16) -> Option<HeightSpec>` that wraps
  `HeightSpec::Cells(rows)`. It has **no cap constant and no chrome math**.
  Each call site hard-codes its rows: `BooleanSwitch` 2 (`:435`), `ChooseOne`
  enum 8 (`:446`), `ChooseMany` 8 (`:463`), `TextInput` 2 (`:513`), number
  `TextInput` 4 (`:546`). See the N8 amendment.
- **Picker today:** `prompt_one_shot_provider` (`wrap/selection_ui.rs:29`)
  calls `run_standalone(ChooseOne::new(), state, None)` at `:40`. The state is
  a bare `ChooseOneState::from_options(options)` with no label, so **the
  picker has no title text**. Options come from `provider_option_to_choice`
  (`:324`): the label is the provider's `Display` name (for example `Goose`)
  and the value is its slug. Pre-picker text comes only from
  `agent_prompt_message` (`wrap/composition/target.rs:410`), which returns
  `None` for `NoAgent` and `ListMultipleInstalled`, the states that matter
  here. `review_sequence` (`:74`) also passes `None`, and it stays out of scope.
- **Tests:** no PTY test drives the one-shot provider picker today.
  `ALT_SCREEN_ENTER` (`"\x1b[?1049h"`) is in `cli/tests/common/pty.rs:186`.
  `sequence_overlay_pty.rs:658` already asserts
  `!transcript.contains(ALT_SCREEN_ENTER)`, which is a model for R7.7.
- **Consequence for R7.3 (Phase 3):** there is no title to assert on. Assert
  on the absence of the staged stubs' provider display names in the options
  list, and on the absence of the picker's raw-mode entry
  (`KBD_ENHANCEMENT_PUSH` after the error). Do not match the `Claudine ▸
  <Provider>` header: it appears only after a provider has been selected.

##### S4 — Excerpt expectation inventory

There are **no `insta` snapshots** for excerpts. The only `.snap` files in
`claudine/` are the `wrap_basics` help output and the config TUI. The `> N │`
gutter strings seen in tests come from Darkmatter's own status-block excerpt,
not from Claudine's appendix. The appendix is a numbered `CodeBlock` that
marks its highlight with a background color, not a `>` glyph.
`wrap_compose_validation.rs:286` checks only the dry-run "Frontmatter
(resolved)" display, and `level1_schema_prompt_pty.rs` makes no excerpt
assertions. Neither is affected.

**Tests that assert excerpt content** (Phase 4 input; each change goes into
the Phase 5 table):

| Test | Location | Asserts today | Phase 4 impact |
|---|---|---|---|
| `appendix_shows_yaml_when_tty` | `lib/.../frontmatter_excerpt.rs:453` | highlight on L5; contains `iteration` (L3) | survives (inside ±3) |
| `appendix_empty_when_not_tty`, `schema_span_appendix_withheld_when_not_tty`, `capture_line_appendix_empty_when_not_tty` | `:446`, `:533`, `:562` | `""` off-TTY | survive |
| `appendix_plain_when_no_color` | `:462` | no ESC bytes | survives |
| `schema_span_*` (3) | `:481`, `:491`, `:501` | `highlight_line` 3 / not 4 / fallback 2 | accessor changes with the model |
| `capture_line_recognizes_four_dash_fence` | `:544` | private `block` starts `----\n`, ends `\n----` | rewrite against regions |
| `capture_line_appendix_highlights_fence_line` | `:569` | highlight 1; contains `name:` and `----` | survives if within ±3 |
| `block_*`, `locate_*`, `value_line_offset_*`, `capture_*_none` | `:371-:558` | helpers only | unaffected |
| `enrich_wraps_lifecycle_nested_span_with_excerpt`, `new_lifecycle_errors_get_frontmatter_excerpt`, `invalid_file_reference_anchors_…` | `lib/.../error/tests.rs:50`, `:1269`, `:1869` | `WithFrontmatter` present | survive (keys authored) |
| `enrich_is_idempotent` | `error/tests.rs:105` | wrapper present for `PromptPropertyMissing` | **breaks**: the key is absent, so the excerpt is omitted. Switch the fixture to a locatable error. |
| `enrich_frontmatter_fence_mismatch_attaches_excerpt` / `_highlights_line_one` | `:120` / `:144` | present; highlight 1 | survive (windowed line) |
| `enrich_frontmatter_parse_regular_error_gets_block_only_excerpt` | `:168` | excerpt present (`BlockOnly`) | **changes**: windowed if the YAML error has a location, otherwise omitted; rename it |
| `enrich_frontmatter_interpolation_focuses_on_receiving_key` | `:194` | `Property("iteration")` | survives |
| `enrich_schema_parse_*` | `:229`, `:258` | highlight 3 / not 4 / 2 | accessor changes |
| `autocomplete_errors_do_not_get_…`, `enrich_is_noop_for_unrelated_error` | `:1322`, `:70` | no wrapper | survive |
| `proxy_with_*`, `runtime_set_shape_…`, `proxy_only_parameter_…` | `:2537`, `:2566`, `:2618`, `:2647`, `:2678` | `frontmatter_block_spec()` equals `Property(path)` | survive |
| `event_set_failure_projects_…` | `lib/.../executor/tests/runtime_set.rs:257` | highlight 9; contains `{{unknown_root}}` | survives if the value is in the window |
| `assert_stack_failure` / `assert_group_set_failure` | `lib/.../sequence/task/tests.rs:2818` / `:3712` (callers with `Some`: `:2978`, `:3016`, `:3102`, `:3844`) | excerpt `Some`/`None`; output contains value | recheck each `Some` case |
| `appends_frontmatter_yaml_block_for_lifecycle_leak_on_tty` | `cli/src/output/error_walker/tests.rs:177` | contains `review_file` (L2), `success:` (L3); highlight L4 | survives (±3) |
| `appends_yaml_block_for_inline_sequence_mismatch_on_tty` | `:199` | whole block via `capture(doc, None, …)`; `prompt: Do it`, `sequence:` | **changes**: `capture` with no key is omitted; focus `prompt` + `sequence` |
| `schema_parse_appends_highlighted_…` / `…withholds_…` / `withholds_frontmatter_yaml_block_when_not_tty` | `:240` / `:262` / `:189` | offending line present / no leak off-TTY | survive |
| `fence_mismatch_non_tty_has_no_ansi_and_no_appendix` | `:377` | Darkmatter's `> 1 │ ----`; appendix withheld | survives |
| `compose_schema_grammar_error_force_color_highlights_line_and_links_file` | `cli/tests/l1/compose_schema_cli.rs:429` | contains the `spec: file(required, …)` line | survives |
| `level1_pty_mismatch_takes_tty_branch_with_yaml_block` | `cli/tests/l1/level1_inline_compose_mismatch_pty.rs:101` | whole block, `leading comment` (L2) … `alias:` (L9) | **changes** unless the focused `prompt`/`sequence` window covers L2–L9 |
| `non_tty_withholds_yaml_but_keeps_guidance` | `cli/tests/l1/inline_compose_sequence_mismatch.rs:190` | no leak off-TTY | survives |
| `level2_malformed_frontmatter_renders_highlighted_diagnostic_in_tmux` | `cli/tests/level2/level2_malformed_frontmatter_capture.rs:163` (needles `:130`) | `yaml`, `prompt:…` (L3), `sequence:` (L4) | survives (±3 around L1) |
| `level2_tmux_mismatch_renders_…` / `level2_wezterm_mismatch_…` | `level2/level2_inline_compose_mismatch_capture.rs:92` / `:165` | whole block: `# leading comment`, `sequence: &seq`, `prompt: \|-`, `alias: *seq` | **changes**: the fixture uses an anchor/alias, and N3 returns `None` for unsafe YAML, so the excerpt would be **omitted**. Decide in Phase 4 whether an unsafe-YAML block that is also small enough to fit the window falls under the "covers the whole block" exception. |
| `level2_schema_parse_renders_highlighted_excerpt_in_tmux` | `level2/level2_schema_parse_capture.rs:158` | offending row has a background color; sibling `spec: "x"` (L4) rendered | survives (±3) |
| tmux + WezTerm removed-key tests | `level2/level2_removed_validation_key_capture.rs` (~`:130`, `:193`) | `---`, `pre_checks:`, `command: test`; styled row | survives if within ±3; verify |
| `…renders_headline_excerpt_and_osc8_link_in_tmux` | `level2/level2_invalid_file_reference_capture.rs:256` | must **not** contain `agent:` (L2); highlight on `iteration` (L4) | the ±3 window includes L2. S4 could not tell how this passes today with the whole block. Run it in Phase 4 before touching it. |
| `…excerpt_includes_schema_parent_in_tmux` | `:450` | `$schema:`, `spec:`, `iteration:` | survives (ancestors) |

**`FrontmatterHighlight` producers** (`frontmatter_block_spec`,
`lib/.../error/mod.rs:3203-3337`; the enum is at `:3357`):

| Variant | Today | Phase 4 |
|---|---|---|
| `FrontmatterParse(FrontmatterFenceMismatch{line})` | `Line` | windowed line |
| `FrontmatterParse(FrontmatterParse{source})` (`resolve.rs:26`) | `BlockOnly` | windowed line when `source.location()` is `Some` (convert the YAML-relative line as Darkmatter `blocks.rs:143` does), otherwise omitted |
| The 22-variant lifecycle `{property}` group (`:3211-3235`), `LifecycleEvaluationError{property:Some}`, `InvalidFileReference`, `LifecycleSayConflict`, `LifecycleUnknownEffect`, `RemovedValidationKey`, `UnresolvedFileReference`, `UnsupportedInteractiveSchema` | `Property` | focused |
| with/set group (8 variants, `:3247`) | `Property("{property}.{path}")` | focused |
| `PromptPropertyWrongType`, `ModelHintWrongType`, `InteractiveHintWrongType`, `InlineHashMalformed`, `SchemaLoad` | `Property(fixed)` | focused |
| `AgentHintWrongType`, `AgentResolutionFailed` | `Property("agent")` | focused when `agent:` is authored; otherwise omitted (it may come from CLI or config) |
| `PromptPropertyMissing` | `Property("prompt")` | omitted (the key is absent by definition) |
| `SchemaParse` with span / without | `SchemaSpan` / `Property("$schema[.p]")` | windowed / focused |
| `MissingProperties`, one entry | `Property(name)` | focused on the declaration inside `$schema` (every arm for unions, per the R6 first bullet), plus a top-level key if one exists; otherwise omitted |
| `MissingProperties`, several | `BlockOnly` | `Properties(Vec)` with the same rule per name |
| `SchemaValidation`, one / several | `Property(ptr)` / `BlockOnly` | `Property` / `Properties`: every problem carries a pointer. Use the `$schema`-declaration rule for caller inputs. |
| `ComposeFailed(Interpolation{key:Some})` | `Property` | focused |
| `ComposeFailed(Interpolation{key:None})` and every other `ComposeFailed` | `BlockOnly` | omitted (locations point into the body) |
| `ShellExpansionFailed` | `BlockOnly` | origin `Frontmatter{key, line}` gives focused/windowed; `Body`, `ShellBlock`, or no origin (`PolicyIo`, `Preflight`) gives omitted. There is no public `origin()` accessor, so a `match` is needed. |
| `InlineComposeSequenceMismatch{source_path}` | `BlockOnly` | **focus `prompt` and `sequence`**: both keys are authored by definition for this error. The plan's default was "omit"; S4 found a locatable key, which the plan allows. |

`LifecycleEvaluationError{property:None}` and every unlisted variant already
produce no excerpt (`_ => None`).

**Current API.** `FrontmatterExcerpt { block, highlight_line, stderr_is_tty }`
has private fields and is re-exported at `composition/mod.rs:76`.
Constructors: `capture(&str, Option<&str>, bool)`,
`capture_schema_span(&str, Option<&str>, usize, bool)`, and
`capture_line(&str, usize, bool)` (the last uses the `----` near-miss block).
There is also a `#[cfg(test)] highlight_line()`.
`render_appendix(&self, &Terminal) -> String` returns `""` off-TTY and
otherwise `"\n\n"` plus a `yaml` `CodeBlock`, with ANSI stripped under
`ColorDepth::None`. The only production constructor site is
`enrich_frontmatter_text` (`error/mod.rs:3158`). Storage is
`CompositionError::WithFrontmatter{excerpt}` (`:2540`) and
`DiagnosticSnapshot.frontmatter_excerpt` (`snapshot.rs:92`, `#[serde(skip)]`,
filled at `lifecycle/context.rs:150`). Rendering happens at
`cli/src/output/error_walker.rs:45` and `lib/src/diagnostics/restored.rs:168`.
`error_walker/tests.rs` builds excerpts directly at `:162`, `:201`, `:221`,
and `:381`.

---

## Phase 2 — Foundations (four independent tracks)

Each track touches a disjoint crate or file set and can run as a concurrent
subagent. The tracks do not depend on one another.

### Wave 2 — parallel tracks

- [ ] **Tolerant arm selection** (`claudine` lib; R1, N5)
    - Extract `is_composition_independent` in `schema/mod.rs`. Rewire the
      pre-validator filter through it, with no behavior change.
    - In `supplied_file_shape`, replace `projected.validate(&candidate).valid`
      with a check that no problem survives the shared predicate, evaluated
      against the `candidate` instance. Keep the eager relaxation exactly as
      it is.
    - Update the `supplied_file_shape` doc comment: it now names
      composition tolerance as the second relaxation.
    - Unit tests (R7.5) in `supplied.rs`: selects the `spec` arm when a
      sibling `file` holds `{{spec || design}}`; the same with `$(…)`; still
      `None` for conflicting discriminants and for string-only alternatives. A
      literal invalid sibling still rules an arm out. Existing tests are
      unchanged.
- [ ] **Focused-region locator** (`biscuit-terminal`; N3)
    - Add `FocusedRegion` and `SourceContext::focused_yaml_regions`, the
      non-falling-back locator.
    - Extend `locate_key_region` for sequence-item (`- key:`) segments, and
      add the every-arm path form.
    - Reimplement `focused_yaml_excerpt` on top of it. Its whole-block
      fallback stays for existing callers.
    - Unit tests: mid-file key ±3 plus ancestors; a key in both union arms
      gives two regions; a missing key gives `None`; adjacent regions merge;
      the region at the start or end of the block is clamped; anchors, aliases,
      and merge keys still give `None`.
    - Update `biscuit-terminal/docs` for `SourceContext` if it is documented
      there.
- [ ] **`CodeBlock` start line** (`darkmatter`; N1)
    - Add `CodeBlockMeta::start_line`, honored for gutter numbering and
      highlight lookup in both renderers. The default keeps current output
      byte-identical.
    - Tests: `start_line = 12` numbers the first line `12`; the highlight on
      absolute line 14 marks the third line; the gutter width grows for
      `start_line + len` (for example 98→102).
    - Update the Darkmatter `CodeBlock`/DSL docs. Decide in-task whether the
      Markdown DSL fence meta (`dsl/parser.rs`) gets a matching key. The
      default is **no** (Rule 2: there is no consumer for it).
- [ ] **Inline provider picker** (`claudine-cli`; R5, N8)
    - Move the chooser sizing into a shared `pub(crate)` helper, and use it in
      `schema_interactive` and `prompt_one_shot_provider`.
    - Update the `prompt_one_shot_provider` doc to state that it is inline and
      bounded.
    - L1 PTY test (R7.7): with no provider flag and a stub for at least two
      providers on the fixture `PATH`, the capture has no `\x1b[?1049h`, and
      selecting a provider launches the stub. It is `#![cfg(unix)]` and gated
      with `expect_level!(Level::L1, pty_available(), …)`, following
      `level1_schema_prompt_pty.rs`.

### Checkpoint 2

- [ ] The R7.1 reproduction test is **green** (a direct consequence of R1).
- [ ] `just test` and `just lint` are green in `claudine/`, `biscuit-terminal/`,
      and `darkmatter/`.
- [ ] The existing `supplied.rs` tests are unchanged and passing.

---

## Phase 3 — Union Fallback, Early Failure, and Caller Base Directory

This phase depends on Phase 2 (tolerant selection) and S1/S2.

### Wave 3 — lib (parallel)

- [ ] **D1 union fallback** (`supplied.rs`; R2, N6)
    - When `supplied_file_shape` returns `None` for a root union, compute the
      per-property fallback atom: every declaring arm has an eager
      `file(match)` with the same `is_array`, and the patterns are the
      de-duplicated union in arm order. Feed it into the existing pending loop,
      so the result is the same `UnresolvedFileReference` with the merged
      `patterns`.
    - Unit tests: two-arm `features`/`fixes` gives the merged patterns;
      mixed `file`/`string` arms give no fallback; one arm non-eager gives no
      fallback; a resolvable literal gives no pending entry.
- [ ] **Caller-origin late verdict** (`classify.rs` + the S2 site; R4, N7)
    - For `NoMatch` problems on caller-owned properties, re-resolve against
      the caller record's origin. When the value is still unresolved, emit
      `UnresolvedFileReference` with `record.origin().base_dir()` in `reason`.
      Extend `classify_unresolved_file_reference` to unions via the N6 rule
      (union arm or fallback atom).
    - Update the `classify_unresolved_file_reference` doc: its "`None` for
      unions" contract changes.
    - Unit test: a union schema, a caller value from origin `A`, and a
      document in `A/prompts`. The error names `A`, never `A/prompts`.

### Wave 4 — CLI ordering and L1 coverage (after Wave 3)

- [ ] **Early-failure ordering** (`prep.rs`, `schema_interactive/supplied.rs`; R2, R3, N9)
    - Using S1, make sure every non-success outcome of the supplied-file pass
      (zero candidates, declined, cancelled, non-interactive) returns its error
      from `prep.rs:186` before target selection. This includes
      `initialize`-authoring documents. Fix any path S1 found that defers or
      swallows the error.
    - Make sure the chooser uses the `UnresolvedFileReference.patterns` it is
      given, so the merged D1 patterns take effect with no CLI-specific union
      logic.
    - Update the comment block at `prep.rs:176-179` if the ordering contract
      it states changes.
- [ ] **L1 PTY: fixed flow (R7.2)**
    - Extend the R7.1 fixture with a two-match variant: the chooser renders,
      the user picks, and the stub launches with the chosen path.
- [ ] **L1 PTY: ordering (R7.3)**
    - No provider flag, several provider stubs, zero candidates: the error is
      printed, and the capture contains **no** picker title text. (Take the
      exact title string from `provider_option_to_choice` or the picker
      chrome, not from memory.)
    - No provider flag, one candidate: `Use this file? (Y/n)` appears in the
      capture **before** the picker title.
- [ ] **L1 PTY: D1 (R7.4)**
    - Two-arm shape (`features/**/spec.md` / `fixes/**/spec.md`,
      discriminated by an optional `kind`) with `spec=cli` matching one file in
      each tree. The chooser lists both. Picking the `fixes` file launches the
      stub with that path, and composition succeeds (the `fixes` arm
      validates).
- [ ] **L1: caller base directory (R4, R7.6)**
    - Launch from the fixture repo root, with the document in `prompts/`, in
      a non-interactive run that reaches the late verdict. The message names
      the launch directory and does not contain `prompts` as the base. Use
      `ambient_context` or `cwd` per the spawn contract, with a call-site
      comment if an escape is used.

### Checkpoint 3

- [ ] R7.2, R7.3, R7.4, and R7.6 are green. The R7.1 and R7.7 tests are still
      green.
- [ ] Manual check with a debug build against the real
      `prompts/clarify.md spec=fix`, using a stub `claude` on `PATH` in a PTY,
      as in the spec's reproduction: the chooser comes first, and the stub
      launches with the chosen spec.

---

## Phase 4 — Focused Frontmatter Excerpts

This phase depends on Phase 2 (locator and `start_line`) and S4. It can start
in parallel with Phase 3: it touches `frontmatter_excerpt.rs` and
`error/mod.rs` only, and Phase 3 does not edit those files.

### Wave 5 — excerpt model and rendering

- [ ] **Excerpt model** (`frontmatter_excerpt.rs`; R6, N2, N4)
    - Replace the `block` + `highlight_line` model with the focused regions:
      each region is a source-text slice, its `start_line`, and its highlighted
      lines. Add `EXCERPT_CONTEXT_LINES = 3`.
    - Capture constructors return `None` when nothing is locatable (no
      whole-block fallback). The single exception is regions that already
      cover the whole block.
    - `render_appendix` renders one Darkmatter `CodeBlock` per region, with
      `start_line` and absolute highlights, joined by `⋮`. TTY gating and
      `NO_COLOR` stripping are unchanged.
    - Update the module `//!` doc. It currently says "captures the verbatim
      frontmatter block", which will be stale.
- [ ] **Property location for caller inputs** (R6 first bullet)
    - When a property is not a top-level frontmatter key, locate it inside
      `$schema`. For unions, locate it in every arm (the N3 every-arm path).
      Also include a same-named top-level key when one exists.
- [ ] **Highlight producers** (`error/mod.rs:3203-3371`)
    - Delete `FrontmatterHighlight::BlockOnly`.
    - Multi-problem `SchemaValidation` and multi-entry `MissingProperties`
      gain a `Properties(Vec<String>)` variant, which shows the union of the
      regions.
    - `Line(n)` and `SchemaSpan` become windowed (±3).
    - `FrontmatterParse` without a location, `ComposeFailed` without a key,
      `ShellExpansionFailed`, and `InlineComposeSequenceMismatch` produce no
      excerpt unless S4 found a locatable key for them.
    - Update the adjacent comments (for example "falls through to BlockOnly").

### Wave 6 — tests (parallel after Wave 5)

- [ ] **Excerpt unit tests (R7.8)**
    - A mid-file property gives ≤7 window lines plus ancestors.
    - `/spec` against a verbatim copy of `prompts/clarify.md` frontmatter
      highlights the arm-0 `spec` line (line 5 in the spec's excerpt; assert
      against the real line in the fixture).
    - An unlocatable property gives no excerpt.
    - Two problems give two regions with `⋮` between them.
    - Rendered gutter numbers match source lines.
- [ ] **Update existing expectations**
    - Update each test from the S4 inventory that asserted whole-block
      output. Record every change, with its reason, in the Phase 5 table.
      Never loosen an assertion without a reason.

### Checkpoint 4

- [ ] `rg 'BlockOnly' claudine/lib/src` finds nothing.
- [ ] `just test` and `just lint` are green in `claudine/`.

---

## Phase 5 — Verification, Documentation, and Hand-off

### Wave 7 — parallel

- [ ] **Full verification**
    - Run `just test` and `just lint` in `claudine/`, `biscuit-terminal/`,
      and `darkmatter/`. Run `just test-l2` in `claudine/`, because
      `level2_malformed_frontmatter_capture.rs` touches excerpt output.
    - Confirm that Windows compiles: `cargo check` for the `claudine-cli`,
      `biscuit-terminal`, and `darkmatter` targets per the `os` skill's
      cross-check guidance. There is no new `cfg(unix)` outside PTY tests.
    - Review `just ci-local --plan` before any push. That is an author step;
      the agent does not push.
- [ ] **Docs drift**
    - Claudine's `composition.md` ("frontmatter-rooted errors append a
      highlighted, line-numbered YAML block"): state that the excerpt is
      focused and omitted when unlocatable. Cover union partial completion and
      the D1 merged patterns in the schema-validation section.
    - `.claude/skills/claudine/` snapshots of the same topics, and
      `cli-reference.md` if it describes the picker as full-screen.
    - Darkmatter `CodeBlockMeta::start_line` docs, and `biscuit-terminal`
      `SourceContext` docs.
    - Add `darkmatter` to the spec's `packages` frontmatter (N1).
- [ ] **Changed-expectations table**
    - Fill in the table below: file, old expectation, new expectation, reason.

#### Changed expectations

| Test / snapshot | Was | Now | Reason |
|---|---|---|---|
| _(filled in by Phase 4/5)_ | | | |

### Checkpoint 5 (terminal state)

- [ ] Every Definition-of-done box is checked.
- [ ] The spec's `status` is set to `implemented` and `implemented: true`. The
      spec is **not** moved to `_completed`, and `just complete` is not run
      (that is the author's step). The terminal state is "implementation
      complete, ready for review".

---

## Dependency Overview

```text
Phase 1  Wave 1: [R7.1 repro] [S1] [S2] [S3] [S4]      (all parallel)
            │
Phase 2  Wave 2: [R1 lib] [bt locator] [dm start_line] [R5 picker]   (parallel)
            │                  │             │
Phase 3  Wave 3: [D1 fallback] [R4 classify] │   ← needs R1, S1, S2
         Wave 4: [ordering + L1 R7.2/3/4/6]   │
            │                                 │
Phase 4  Wave 5–6: excerpts (R6) ←────────────┘   ← needs locator, start_line, S4;
            │                                         may overlap Phase 3
Phase 5  Wave 7: verification + docs
```

## Risks

- **Tolerance could over-select arms.** With composition tolerance, both
  union arms may pass for `clarify.md`-like shapes that differ only in
  templated properties. Mitigation: N6 treats "several arms" as the D1
  fallback, not as a selection. The R7.5 tests pin the `None` cases.
- **The every-arm YAML locator is heuristic.** It is indentation-based, not a
  parser. Mitigation: it returns `None` rather than guessing (N3), and the
  excerpt is then omitted, which R6 allows.
- **Snapshot churn in Phase 4.** S4 bounds it in advance, and the
  changed-expectations table makes each change reviewable.
- **PTY tests are Unix-only.** This matches the existing
  `level1_provided_partial_file_pty.rs` convention. Windows coverage of the
  lib-level logic comes from the unit tests.
