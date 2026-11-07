---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-render-surface/spec.md
plan: darkmatter/features/2026-11-05-render-surface/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
decisions_for_review:
    - |-
        Phase 1 wrote no code. The plan's Phase 1 has one task: finish the two
        prerequisite features (dotted paths, then loading data) before render
        work starts. The plan was written when both were unfinished. By the
        time this phase ran, both were done: every task in both plans is
        checked, both specs say `implemented: true`, and the last review of
        each says `ready: true` with no findings. So this phase only confirmed
        that the promised APIs exist and their test suites pass, and recorded
        that here. It did not reopen or re-run either feature's plan.
        The alternative was to re-run their plan phases anyway. That would
        repeat finished, reviewed work and could disturb their review results.
        Nothing needs redoing if the reviewer agrees. If the reviewer thinks a
        prerequisite is incomplete, the gap belongs in that feature's own
        review cycle, not in this feature.
    - |-
        Phase 2 ran its three tasks one after another instead of running the two
        Wave 2 tasks in parallel. The layer and normalization code needed the new key
        names and descriptor rows from the model task. Running them in parallel would
        have had two agents editing the same style files at once. Nothing needs
        redoing; the only cost was wall-clock time.
    - |-
        Keys that the schema accepts but that do nothing yet now show an
        informational "known but inactive" notice. The notice only reaches the debug
        log. New keys use an internal marker of 10 until they are wired. Keys wired in
        this phase were set to 8: the padding sides, `hr.padding`, `page.theme`,
        `page.code.mode`, and `code-block.line-numbers`. The older component keys
        (`table.padding`, `margin`, `border`, and so on) still carry 9, so they still
        show the notice even though rendering already uses them. The spec's own fill
        replacement, `{ table: { padding: 0, padding-left: 4 } }`, therefore logs a
        notice for `padding`. The alternative is to raise the active-wiring threshold
        to 9 and silence those notices too. That is a one-line change plus updating
        the tests that expect the notice. The reviewer should pick one before the docs
        phase.
    - |-
        Snake-case spellings of the renamed page keys (`page.left_margin`) and of
        new keys (`page.margin_left`) are reported as unknown keys that suggest the
        kebab-case name, not as deprecated aliases. Treating them as aliases would have
        kept a spelling the spec removes. Other existing snake-case aliases still
        warn as deprecated in documents only. In style files they are unknown and
        excluded with a suggestion; in setters they are an error. Changing that would
        touch only the alias handling in `walker.rs` and `layer.rs`.
    - |-
        `page.code.mode` is case-sensitive (`Dark` fails), so that the schema enum
        and the runtime parser agree. Theme names keep the case and alias leniency of
        `ThemePair::try_from`. Their schema type is `string(suggest(...))`, so an
        unknown theme passes schema admission and fails at runtime, with a typed error.
        The alternative was an enum of every alias: it would be stricter at the
        schema, but it would offer the aliases in completion.
    - |-
        For document styles, `prepare_style_layers` uses only the schema's
        unknown-key verdict. It ignores the schema's type problems, because rendering
        has never schema-checked a document, and compose's own schema stage owns those
        problems. The built-in runtime validator still checks document values.
        Document unknown keys use `dm.style.unknown_key` (as warning under render,
        error under compose), not `dm.schema.unknown_key`. Setter failures use the
        `dm.schema.*` codes. To change this, edit the document branch of
        `style/layer.rs`.
    - |-
        A nested border setter after a scalar border
        (`style.table.border=thin style.table.border.left=false`) replaces `thin`
        with `{ left: false }`; it does not carry the weight over. The spec says only
        that the nested field "switches to the object arm". Carrying `thin` over as
        `weight: thin` is possible but would be a style-only translation rule.
    - |-
        Theme resolution was moved onto the request snapshot in Phase 2, not
        Phase 4. The CLI's `ResolvedTheme`, which read `THEME` and `CODE_THEME` from
        the process, was replaced by `PresentationDefaults::from_snapshot`. The old
        `--theme`, `--code-theme`, `--code-block`, and `--line-numbers` flags still
        work until Phase 4 removes them. They are applied over the document's style,
        so a flag wins and an omitted flag leaves the document's value. This needed
        `Cli.code_block` to become `Option` with no clap default; `md schema about`
        falls back to `inverse`. Phase 3 moves `--code-block` onto `schema about`
        anyway.
    - |-
        A stylesheet value is validated on every target. An empty value, a
        `file://` value, or a malformed reference fails even for terminal output.
        Only HTML and MarkdownPlus read local CSS or emit a remote link. Before this
        phase, terminal output also read the local file and failed when it was
        missing. That read is gone, as plan Rule 17 asks. With stdin input, a relative
        stylesheet now resolves against the launch context instead of the process
        directory.
    - |-
        Other observable changes from the new document normalization:
        - A document's `page.meta: {}` now reads as absent.
        - `page.meta` must be an object at parse time; the schema already required
          that.
        - A document's `page.alignment` is now also written into every component,
          including `hr.alignment`. The page builder already passed page alignment to
          components, so the only possible visible difference is a horizontal rule.
          No existing test changed.
        - A local-style `padding` on `hyperlinks` or `images` now reaches the CSS
          overlay, where before it was ignored.
    - |-
        Phase 3 built the new render arguments next to the old ones instead of
        removing the old ones. The plan's argument task says "removals", and the
        spec removes the 38 root presentation flags, root `--save`, `--list-themes`,
        and `md render --max-width`. All of these still work and are only hidden
        from root help. Removing them now would break the render path, which still
        reads them and which Phase 4 rewires. It would also break about 80 test
        assertions that Phase 5 migrates to the new style setters. Two removals were
        made now because a working replacement already exists: root `--feature` is
        no longer combined with `md render --feature` (that is now exit 2), and
        `--code-block` exists only on `md schema about`. The alternative was to
        remove everything now and rewrite those tests against setters that do not
        apply yet, which would fail. Phase 4 should remove `--save` and
        `--list-themes` along with their dispatch branches. Phase 5 should remove the
        presentation flags and `--max-width` together with the test migration.
    - |-
        The two Phase 3 tasks ran one after the other, not in parallel, because
        both edit the same CLI crate and would have blocked or broken each other's
        builds. Nothing needs redoing; the cost was wall-clock time.
    - |-
        Root help shows the commands, a "Shared controls" section (`--width`,
        `-v`, `--magic-root`, `--completions`), and a "Render" section with only
        `--style`. It ends with the sentence `` `md [ARGS] [OPTIONS]` is `md render
        [ARGS] [OPTIONS]`; see `md render --help`. `` The spec's sentence says
        `[INPUT]`. `[ARGS]` was used because the positional is now named `ARGS`.
        The other render options are hidden on the root command only, so
        `md render --help` still lists them. `--width` is a global option, so its
        "Shared controls" heading also appears in `md render --help`. Changing the
        sentence or the headings touches only `args/cli.rs` and the help tests.
    - |-
        `md --max-width 30 render doc.md` is now a usage error (exit 2), like the
        other render options given before a subcommand. Before, it was accepted.
        `md render --max-width` itself still works until Phase 5 removes it.
        `md code-block --code-block MODE` used to be accepted through the old global
        flag but did nothing. It is now an unknown argument, and its row was removed
        from `docs/cli/code-block.md`.
    - |-
        How a `--style FILE` argument is split into a key and a file (Rule 14):
        a backslash that is a property-path escape (`\.` or `\\`) before the first
        `@` does not count as a path separator. So `a\.b@x.yaml` selects key `a.b`,
        while `themes\user@host.yaml` stays a Windows path. The spec's two rules
        ("no separator before the `@`" and "escaped dots in the key") conflict on
        this one case. A leading `@` is always a magic reference (`@file.yaml`), so
        an empty key never reaches the key parser. Changing this touches only
        `render_style.rs::classify_style_argument` and its tests.
    - |-
        Style-file failures reuse existing codes instead of new ones:
        `dm.input.read_failure` (with the typed file-reference failure kept) for
        references that cannot be opened or are refused, `dm.data.malformed` for bad
        content, with the file, line, and column. A missing key, or a key that is not
        a mapping, uses `dm.style.invalid_root`. The five new `dm.style.*` error codes
        were not added to `RENDER_DIAGNOSTIC_CODES`. That list holds the warnings
        `--deny` can block, and these are always errors. If the reviewer wants
        `--deny` to accept them, it is one line each there.
source_files_during_phase_2:
    - darkmatter/lib/src/style/apply.rs
    - darkmatter/lib/src/style/bespoke.rs
    - darkmatter/lib/src/style/coverage_tests.rs
    - darkmatter/lib/src/style/descriptor.rs
    - darkmatter/lib/src/style/error.rs
    - darkmatter/lib/src/style/layer.rs
    - darkmatter/lib/src/style/length.rs
    - darkmatter/lib/src/style/mod.rs
    - darkmatter/lib/src/style/normalize.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/src/style/presentation.rs
    - darkmatter/lib/src/style/schema/common.rs
    - darkmatter/lib/src/style/schema/components.rs
    - darkmatter/lib/src/style/schema/hr.rs
    - darkmatter/lib/src/style/schema/mod.rs
    - darkmatter/lib/src/style/schema/page.rs
    - darkmatter/lib/src/style/walker.rs
    - darkmatter/lib/src/style/warning.rs
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
    - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
    - darkmatter/lib/benches/render_pipeline_steps.rs
    - darkmatter/schemas/partials/style.yaml
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/artifact.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/io/mod.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/cutover_reference.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/style_effective_lowering.rs
    - darkmatter/lib/tests/l1/style_frontmatter_parity.rs
    - darkmatter/lib/tests/l1/style_frontmatter.rs
    - darkmatter/lib/tests/l1/style_layers.rs
    - darkmatter/lib/tests/l1/tree_features_characterization.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
    - darkmatter/cli/tests/l1/feature_flag.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/cli/tests/l1/mermaid_features.rs
    - darkmatter/cli/tests/level2/level2_frontmatter_tables.rs
    - darkmatter/example-docs/rendering/code-blocks.md
    - darkmatter/example-docs/rendering/style-prop.md
docs_updated_during_phase_2:
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/docs/rendering/style.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/rendering.md
source_files_during_phase_3:
    - darkmatter/cli/src/args/render.rs
    - darkmatter/cli/src/args/render/tests.rs
    - darkmatter/cli/src/args/render_prepare.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/args/path_completion.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/src/render_style.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/src/lib.rs
    - darkmatter/cli/src/output/report.rs
    - darkmatter/lib/src/markdown/compose/expression/data/parse.rs
    - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/render_style_files.rs
    - darkmatter/cli/tests/l1/feature_flag.rs
    - darkmatter/cli/tests/l1/help.rs
    - darkmatter/cli/tests/l1/mermaid_features.rs
    - darkmatter/cli/tests/l1/output_envelope.rs
    - darkmatter/cli/tests/l1/render_basic.rs
    - darkmatter/cli/tests/level2/level2_code_block_styling.rs
docs_updated_during_phase_3:
    - darkmatter/docs/cli/render.md
    - darkmatter/docs/cli/code-block.md
    - darkmatter/docs/rendering/code-highlighting.md
    - darkmatter/docs/rendering/style.md
    - darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/cli.md
message_to_agent: |-
    Phase 3 built CLI preparation. Nothing is wired into rendering yet.

    - Arguments (`cli/src/args/render.rs`, `args/render_prepare.rs`):
      `RenderOptions { args, literal_args, style, output, show, max_width, indent, deny, features, mermaid }`,
      `classify_render_args(&args, &literal_args) -> RenderInput { input, setters: Vec<RawStyleSetter> }`,
      `setter_hint(&OsStr)` (meant for the "input did not resolve" message; not
      wired yet), and `PreparedRenderOptions::prepare(&RenderOptions)`
      (`input, setters, styles, policy, show, indent, mermaid, features, output`).
      `run_render` already builds it and reads `input`/`mermaid` from it.
    - Style preparation (`cli/src/render_style.rs`):
      `prepare_cli_style_inputs(&md, opened, &prepared.styles, &prepared.setters, request) -> CliStyleInputs { schema, layers, trigger_findings }`,
      then `inputs.prepare(Some(document_layer), StylePreparationPolicy::render().with_deny_style(..)) -> PreparedStyle`
      and `inputs.warnings(&prepared) -> Vec<Diagnostic>`.
      Failures are `ClassifiedFailure` reports that keep their codes. The
      schema is selected once by `commands/schema/validate.rs::select_document_schema`.
      Build the document layer with `StyleLayer::document(..)`; its positions must
      be converted to document lines.
    - Phase 4 must handle:
      1. `ReportSink::failure` / `CommandRun::finish` apply
         `with_document_fallback`, which would stamp the document path onto a
         setter record that has no `source`. Exempt the style setter codes, as
         `dm.setter.invalid` already is.
      2. `trigger_findings` from `CliStyleInputs` are returned but not delivered
         to any report yet.
      3. The old root flags are still present and hidden, as are `--save`,
         `--list-themes`, and `render --max-width`, because their tests have not
         been migrated (see decisions_for_review). Phase 4 removes the `--save` and
         `--list-themes` dispatch. Phase 5 removes the presentation flags with its
         test migration. `has_explicit_arguments` destructures `RenderOptions`, so
         removing a field there requires listing or unlisting it there and in the
         generated root-rejection test's sample map (`args/render/tests.rs`).
      4. `terminal_page` gained a `mermaid: bool` parameter and an
         `#[allow(clippy::too_many_arguments)]`; Phase 4 replaces that signature.
    - `simplified-schema/docs/topics/types/container-types.md` is still modified by
      someone outside this feature. Leave it alone.
---

# Implementation Log for 2026-11-05-render-surface (7 phases)

## Phase 1

### Scope found

Phase 1 (Wave 1, "Complete dependencies") requires `2026-11-06-dotted-paths` and
then `2026-11-04-loading-data` to be finished before Phase 2. When this phase
started, both were already finished:

| Dependency | Plan tasks | Spec `implemented` | Last review |
|---|---|---|---|
| `2026-11-06-dotted-paths` | all `[x]` (plan `phase: 11` of 10) | `true` | review-4: `ready: true`, no findings |
| `2026-11-04-loading-data` | all `[x]` (plan `phase: 10` of 9) | `true` | review-2: `ready: true`, no findings |

So no dependency phase was re-run. No product source, docs, or skills were
edited in this phase.

### Checkpoint: the shared APIs exist

The Wave 1 checkpoint names five surfaces. Each one was confirmed in the tree:

- `PropertyPath`: `simplified-schema/lib/src/property_path.rs`, re-exported from
  `simplified_schema` (`lib.rs` `pub use property_path::{…}`)
- `SetterValue` / `SetterValueError`: `simplified-schema/lib/src/setter_value.rs`,
  re-exported from `simplified_schema`
- schema path candidates: `darkmatter::markdown::schemas::completion::path_candidates(&PathQuery)`
  (`darkmatter/lib/src/markdown/schemas/completion.rs`)
- shell-word recovery and shared completer: `darkmatter/cli/src/args/path_completion.rs`
  (`complete_compose_word`, `complete_{get,rm,set}_path`, `complete_set_value`,
  plus the private `select_schema`/`document_effective` helpers that Phase 3 will
  reuse for render completion)
- public format detection: `biscuit_file::detect_from_extension(&Path) -> FileType`
  (`biscuit-file/lib/src/detect.rs`)
- reader location helpers: `biscuit_file::data_locations::{locate_json,
  locate_jsonl, locate_toml, locate_json5, …}`
- compose setters now use the shared types
  (`darkmatter::markdown::compose::setters::{RawSetter, SetterPlan, TokenClass,
  classify_token}`). The private `parse_shorthand_value` that Rule 1 mentions no
  longer exists.

### Prerequisite suites (macOS, this worktree, run serially)

| Command (from area dir) | Result |
|---|---|
| `simplified-schema: just test` | 1362 run, 1362 passed, 0 skipped |
| `biscuit-file: just test` | 1273 passed; second variant 6 passed, 104 skipped (feature-gated variant) |
| `darkmatter: just test` | 14868 run, 14868 passed, 10 skipped, 110.5 s, exit 0 |
| `darkmatter: just lint` | exit 0 (includes zed-dmls wasm32-wasip2 check) |

No test failed, so none were rerun and none are load-sensitive.

### Gate-skip check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
returned `packages: [biscuit-file, darkmatter, darkmatter-lsp, simplified-schema,
test-toolkit, claudine, claudine-cli, darkmatter-cli]`, `change_class: package`.
Those packages come from the uncommitted loading-data review edits already in
the worktree, not from this phase. The gates were therefore run, and passed (see
above).

### Test design mapping

This phase changed no behavior, so it added no tests. Its requirement is that
the dependency surfaces compile and their suites pass. Evidence is the four
suite runs above. Dependent-package testing (claudine, claudine-cli,
darkmatter-lsp) was not run because this phase changed none of their inputs.

### Cross-OS

No code changed, so this phase carries no cross-OS risk and no cross-check was
run. The plan (Phase 7) also rules out `just cross-check` for this feature.

## Phase 2

### Execution approach

Phase 2's three tasks were run as three foreground subagents in sequence, not
in parallel. Wave 2's "sparse layers" task needs the canonical key names and
descriptor rows that the "extend style models" task adds, and Wave 3 needs both.
Running them one after another avoided two agents editing `style/mod.rs` and
`parse.rs` at once. Each agent ran only its own focused tests; the area gates
were run once at the end.

### Wave 2, task 1: Extend style models

- Page spacing keys use CSS order: `page.margin-left|right|top|bottom` and
  `page.padding-*`. The Rust fields are `PageStyle::{margin_left, …, padding_bottom}`.
  The old names and their snake-case aliases are removed. An old spelling is
  now an unknown key whose warning suggests the new one
  (`StyleWarning.suggestion`, `StyleWarning::with_suggestion`). The CLI and
  DMLS message reads ``unknown style key `style.page.left-margin`; use `page.margin-left` ``.
  `ul.left-margin` keeps its name.
- The page shorthands `margin padding mx my px py ml mr mt mb pl pr pt pb`
  and every page key are admitted both under `page` and at the top level of
  `style` (`schemas/partials/style.yaml`, `descriptor::SCHEMA`). All-sides and
  vertical shorthands take rows only (integer 0..=65535). Horizontal ones keep
  the length grammar. `page.max-width` must be positive (`LeafType::PositivePageWidth`).
- New typed fields:
  - `PageStyle.theme: Option<ThemePair>`
  - `CodeStyle { theme: Option<ThemePair>, mode: Option<CodeBlockMode> }`
  - `CodeBlockStyle.line_numbers: Option<bool>`
  - `CommonStyle.padding_left` and `padding_right`
  - `HrStyle.padding`, `padding_left`, and `padding_right`
- `hr.margin`, `hr.border`, `hr.emphasis`, and `hr.word-wrap` are removed from
  the schema and the descriptor, so they are now unknown keys.
- Theme fields are declared as `string(suggest(<ThemePair::all()>))`. Runtime
  validation goes through `ThemePair::try_from`, which keeps its alias and case
  acceptance. `StyleParseError::InvalidValue` was added.
  `StyleApplyError::InvalidCodeTheme` was removed because nothing can reach it.
- Tests:
  - L1 `base_style_schema_agrees_with_the_runtime_style_parser` checks 70
    cases against both the shipped schema and the runtime parser. Four known
    gaps are asserted explicitly: a non-theme string and a non-string theme
    pass the schema, and the horizontal-length string arm does not encode the
    grammar.
  - `base_style_theme_suggestions_are_the_canonical_theme_names`
  - three `style_frontmatter` tests
  - the CLI test `removed_page_spacing_name_warns_with_its_replacement`
  - unit tests in walker, descriptor, warning, page, components, hr, and parse
- Six recursive-schema goldens were re-blessed from the live embedded baseline
  (`lib/tests/fixtures/recursive_schema_golden/golden/`).

### Wave 2, task 2: Implement sparse layers

New `style/layer.rs` (public API) and `style/normalize.rs` (crate-private):

```rust
pub enum StyleSourceKind { Document, File, Setters }
pub struct StyleOrigin { pub kind, pub source: String, pub path: String, pub written: String, pub span: Option<StyleSpan>, pub context: FileResolutionContext }
pub enum StyleSetterValue { Raw(String), Parsed(SetterValue) }
pub struct StyleSetter { pub argument: String, pub path: PropertyPath, pub value: StyleSetterValue }   // ::new / ::parsed
StyleLayer::document(value, identity, context, positions /* keyed style.page.x */)
StyleLayer::file(value, argument, context, positions /* keyed page.x */)
StyleLayer::setters(setters, context)
pub enum UnknownStyleKeyPolicy { Warn, Error }
pub struct StylePreparationPolicy { pub document_unknown_keys, pub deny_style }   // ::render(), ::compose(), .with_deny_style()
pub struct StyleDiagnostic { pub code, pub warning, pub origin }
pub struct StyleProblem { pub code, pub message, pub path, pub origin }
pub struct StylePreparationError { pub problems, pub diagnostics }
PreparedStyle: value(), style(), into_style(), retained(), origin(path), origins(), diagnostics()
pub fn prepare_style_layers(layers: &[StyleLayer], schema: Option<&EffectiveSchema>, policy: StylePreparationPolicy) -> Result<PreparedStyle, StylePreparationError>
```

- Codes:
  - `dm.schema.unknown_key` and `dm.schema.type_mismatch` for setters
  - `dm.schema.missing_required` for the merged style
  - new style codes: `dm.style.invalid_value`, `dm.style.conflict`,
    `dm.style.empty_file`, `dm.style.invalid_root`, `dm.style.invalid_setter`
- The document path (`parse::from_json_value`) now deserializes the normalized
  value:
  - top-level and page shorthands take effect;
  - `page.alignment` broadcasts to every component, `hr.alignment` included;
  - two spellings of one field in a document fail with the new
    `StyleParseError::Conflict`.
  - The descriptor rows of the newly effective shorthands and top-level keys
    took their target field's `sub_spec`.
- Tests: 29 in `lib/tests/l1/style_layers.rs`, in five groups: normalization,
  alignment, merging, admission, and sources. They use only the public API.

### Wave 3: Wire effective styles

- `apply_prepared_style(page, &PreparedStyle, StyleAssetResolver, &PresentationDefaults)`
  lowers a prepared style onto a page.
- `StyleAssetResolver::for_target(OutputTarget)` reads local CSS only for
  Html and MarkdownPlus. `passive()` validates a stylesheet value without
  reading it.
- A stylesheet resolves through `FileReference` against the winning field's
  `StyleOrigin.context`. `bespoke.rs`'s `current_dir` fallback is gone, along
  with its allowance in `context_construction_guard.rs`.
  `apply_bespoke_style` now takes `(…, assets, &FileResolutionContext)`
  instead of `source_path`.
- New errors: `StyleApplyError::StylesheetReference` keeps the typed
  `FileReferenceError`, and `StylesheetContextMissing` was added.
- New `style/presentation.rs` adds `PresentationDefaults::{new, from_env,
  from_snapshot, from_context}`. Its `resolve(&StyleFrontmatter) -> Presentation`
  resolves in this order:
  - style beats the `THEME`/`CODE_THEME` defaults from the request snapshot;
  - a present environment value is read with `from_str_or_default`;
  - an absent prose theme is OneHalf;
  - an absent code theme is derived from the effective prose theme;
  - the code-block mode defaults to inverse;
  - line numbers are read from `code-block.line-numbers`.
- Component `padding-left` and `padding-right` lower after `padding` into
  `ComponentPolicy.layout.padding` for terminal and browser. This also covers
  `hr` padding, `CommonStyle::to_css_overlay`, and local-style merges. A
  side-only override keeps the inherited vertical padding.
- The CLI now builds pages through `render::PageInputs { document_context,
  presentation }`, with the environment read from `request.snapshot()`.
  `ResolvedTheme` was removed.
  - The old flags (`--theme`, `--code-theme`, `--code-block`,
    `--line-numbers`) still win, because they are applied as a top layer.
  - `Cli.code_block` became `Option` and lost its clap default; `schema
    about` falls back to inverse.
  - Claims still drive the other old flags until Phase 4/5.
- Tests: 24 in `lib/tests/l1/style_effective_lowering.rs`, in three groups:
  `padding::` (8, including `hr_padding_left_moves_the_rule_four_cells_in`),
  `presentation::` (6), and `stylesheet_origins::` (10, including that
  terminal, plain, markdown, and json never read CSS, and that HTTP(S) stays a
  link). There are also unit tests in `presentation.rs` and `bespoke.rs`.
- Observable changes:
  - a terminal or plain render no longer reads, or fails on, a missing local
    stylesheet;
  - with stdin input, a relative stylesheet resolves against the launch
    context;
  - a local-style `padding` on `hyperlinks` or `images` now reaches the CSS
    overlay.
- Docs updated: `docs/topics/layouts-and-style.md`, `docs/rendering/style.md`,
  and `.claude/skills/darkmatter/rendering.md`.

### Input-robustness smell grep

I searched `layer.rs`, `normalize.rs`, and `presentation.rs` for
`#[serde(default)]`, `unwrap_or_default()`, `filter_map(..as_str)`, and
`.ok()`. There is one hit: `normalize.rs::broadcast_alignment`'s `.ok()`. It
is intentional. An unparseable alignment is not broadcast, and the
`page.alignment` value itself is still validated and rejected. The full
matrix audit is part of Wave 7 (Phase 5).

### Unrelated working-tree change

`simplified-schema/docs/topics/types/container-types.md` is modified in the
working tree. No agent in this phase edited it. It was left untouched and is
not listed as a Phase 2 file.

### Requirement-to-test mapping (Phase 2)

| Requirement | Tests |
|---|---|
| Rule 18 names, removed spellings, suggestions | `style_frontmatter::removed_page_spacing_names_warn_with_their_replacement`, the CLI test `layout_style_frontmatter::removed_page_spacing_name_warns_with_its_replacement`, walker unit tests |
| Schema, runtime, and typed agreement (Ruling 6) | `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser` (70 cases, 4 asserted gaps), `base_style_theme_suggestions_are_the_canonical_theme_names` |
| Vertical u16 rows, positive cap, all-sides validity | agreement cases plus page/length unit tests (`margin: '10%'` fails, `mx: '10%'` passes, `my: '2ch'` fails, `max-width: 0` fails) |
| Rules 8–10: normalization, precedence, conflicts, sparse merge | `style_layers::normalization::*`, `alignment::*`, `merging::*` |
| Rules 11–12: admission, custom keys, warnings | `style_layers::admission::*`, `sources::*` |
| Rule 16: empty style file | `style_layers::sources::a_file_must_supply_a_non_null_admitted_value`, `an_admitted_custom_file_value_counts_as_supplied` |
| Rule 17: stylesheet origins and target-aware reads | `style_effective_lowering::stylesheet_origins::*` (10) |
| Side padding and HR padding lowering | `style_effective_lowering::padding::*` (8) |
| Theme, mode, and line-number defaults after merge | `style_effective_lowering::presentation::*` (6) and `presentation.rs` unit tests |

These are all L1 tests through the public `darkmatter::style` API. They use
explicit widths and pinned terminal profiles, and they anchor file contexts in
temp dirs.

### Gates (macOS, run once after every subagent finished)

| Command | Result |
|---|---|
| `darkmatter: just test` | 14960 run, 14960 passed, 10 skipped, 118 s, exit 0; no failures and no reruns |
| `darkmatter: just lint` | exit 0 (includes the zed-dmls wasm32-wasip2 check) |
| `just check-tier-coverage darkmatter` (repo root) | 0 stranded, exit 0 |
| changed L2 files: `nextest -p darkmatter -p darkmatter-cli --features terminal-tests -E 'test(layout_policy) \| test(frontmatter_tables)' --run-ignored all` with `BISCUIT_TEST_LEVEL=2` | 14 passed |
| `cargo check -p darkmatter -p darkmatter-cli --tests --benches`, plus `--features browser-tests,terminal-tests` | OK (Wave 3 agent) |
| dependent packages: `cargo check -p claudine -p claudine-cli --tests` | OK. A grep found no use of `darkmatter::style` or the old page keys outside the darkmatter area, so no claudine tests were selected. darkmatter-lsp's suite is part of the area's `just test`. |

The gate-skip check was
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
It returned `packages: [darkmatter, darkmatter-cli, darkmatter-lsp,
test-toolkit, claudine, simplified-schema]` with `change_class: package`, so
the gates ran. `simplified-schema` comes from the unrelated
`container-types.md` edit.

### Skipped or not covered

- The browser tier (`just test-browser`) was not run. Its callers were only
  compile-checked. The browser geometry assertions are migrated in Phase 5 and
  run in Phase 7.
- No cross-OS check was run. The plan (Phase 7) rules out `just cross-check`
  for this feature. The Windows risk is noted: the inlined stylesheet's
  `data-darkmatter-source` path now comes from `FileReference::resolve_detailed`
  without canonicalization, and no test compares its spelling.

## Phase 3

### Execution approach

The two Wave 4 tasks ran as two foreground subagents **one after the other**,
not in parallel. Both edit the `darkmatter-cli` crate. Two agents building it
at once would block each other on cargo's lock, and each would see the other's
half-finished, uncompilable state. The argument task ran first because it
defines `RawStyleSetter` and `PreparedRenderOptions`, which the loader uses.
Each agent ran only its own focused tests. The area gates ran once at the end.

### Scope boundary for this phase

The plan's argument task lists "removals". Removing the 38 root presentation
flags, root `--save`, `--list-themes`, and `render --max-width` now would break
two things. The render path still reads them through `CliStyleClaims`, and
Phase 4 rewires it. About 80 L1 assertions (`layout_flags.rs`,
`layout_alignment.rs`, `layout_fill.rs`, `mermaid_features.rs`, and others) and
the L2 and browser suites use them, and Phase 5 Wave 7 migrates those. So this
phase built the new surface alongside the old one. It made only the removals
that already have a working replacement: feature splicing, and the global
`--code-block`, which now exists only as `schema about --code-block`. See
`decisions_for_review`.

### Wave 4, task 2: Refactor render arguments

- `RenderOptions` (`cli/src/args/render.rs`) has no clap defaults:
  - `args: Vec<OsString>` (`[ARGS]...`, `num_args = 0..`)
  - `literal_args: Vec<OsString>` (`last = true`, filled only after `--`)
  - `style: Vec<OsString>` (`--style <FILE>`, repeatable)
  - `mermaid: bool`, moved here from `Cli`
  - the retained fields `output`, `show`, `max_width`, `indent`, `deny`, `features`

  clap 4.6.1 needed no workaround: `md render -h` shows
  `Usage: md render [OPTIONS] [ARGS]... [-- [INPUT]...]`.
- The new public API:

  ```rust
  pub struct RawStyleSetter { pub argument: String, pub key: String, pub value: String }
  pub struct RenderInput { pub input: Option<PathBuf>, pub setters: Vec<RawStyleSetter> }
  pub enum RenderArgsError { MultipleInputs(Vec<OsString>) }      // -> clap::Error, exit 2
  pub fn classify_render_args(args: &[OsString], literal_args: &[OsString]) -> Result<RenderInput, RenderArgsError>
  pub fn setter_hint(input: &OsStr) -> Option<String>             // "stlye.ul.x=4" -> "style.ul.x=4"
  impl RenderOptions { pub fn has_explicit_arguments(&self) -> Vec<&'static str> }  // destructures every field
  pub struct PreparedRenderOptions { input, setters, styles, policy, show, indent, mermaid, features, output }  // args/render_prepare.rs
  impl PreparedRenderOptions { pub fn prepare(&RenderOptions) -> Result<Self, clap::Error> }
  ```
- `validate_subcommand_usage` uses `has_explicit_arguments`. Each of these
  now exits 2:
  - `md --feature x render doc.md`
  - `md style.margin=4 toc doc.md`
  - `md --style s.yaml render doc.md`
  - `md --mermaid render doc.md`
  - `md --max-width 30 render doc.md`

  `Cli::command_or_render` no longer splices features.
- `--code-block` is `SchemaTarget::About { code_block: CodeBlockArg }`, with
  default `inverse`. The render path no longer reads it. A document's
  `style.page.code.mode` still applies.
- `run_render` builds `PreparedRenderOptions` and reads `input` and `mermaid`
  from it. Setters and `--style` are classified and collected but **not yet
  applied**. Phase 4 wires them in. `docs/cli/render.md` marks them
  **planned**.
- Root help:
  - It shows the commands, then a "Shared controls" section (`--width`, `-v`,
    `--magic-root`, `--completions`), then a "Render" section that lists
    `--style` only.
  - It ends with the sentence `` `md [ARGS] [OPTIONS]` is `md render [ARGS] [OPTIONS]`; see `md render --help`. ``
  - The other render fields are hidden on the root command only, through
    root `mut_arg`. The old presentation flags, `--save`, and `--list-themes`
    are hidden too.
  - `md render --help` still lists every render option.

### Wave 4, task 1: Load style files

- The library export is
  `darkmatter::markdown::compose::expression::data::{parse_local_document, LocalDataDocument}`.
  - It wraps the existing `decode_local` and `parse_text`, so the reader
    rules are unchanged: JSON and JSON5 keep the last duplicate key, and YAML
    and TOML reject duplicates.
  - It accepts tree formats only.
  - `positions(paths)` uses the existing `SourceLocator`. A position marks
    where the value starts.
- Schema selection lives in `cli/src/commands/schema/validate.rs`.
  - The core is `effective_document_schema(schemas, &Markdown, context, discover_triggers)`.
  - The render entry is `select_document_schema(&Markdown, Option<&OpenedArgument>, &MdRequest) -> SelectedDocumentSchema { schema, trigger_findings }`.
  - The baseline comes from the request snapshot's environment, and
    triggers are discovered in the document's context (the launch context
    for stdin). The document is never composed.
  - Completion's `document_effective` now goes through the same core, with
    unchanged semantics.
- `cli/src/render_style.rs` (`pub mod render_style`) prints nothing:
  - `classify_style_argument(&OsStr) -> Result<StyleFileArgument>` does no I/O.
  - `load_style_file(&OsStr, &MdRequest) -> Result<LoadedStyleFile { layer, path }>`
    opens the file through `io::open_argument`. The layer's context is
    `OpenedArgument::document_context`.
  - `setter_layer(&[RawStyleSetter], &MdRequest)` parses each key with
    `PropertyPath` and uses the launch context.
  - `prepare_cli_style_inputs(document, opened, styles, setters, request) -> CliStyleInputs { schema, layers, trigger_findings }`.
  - `CliStyleInputs::prepare(document_layer, policy) -> Result<PreparedStyle>`
    and `CliStyleInputs::warnings(&PreparedStyle) -> Vec<Diagnostic>`.
- Failures are run errors with exit 1:

  | Case | Code | Location |
  |---|---|---|
  | missing file, invalid reference, non-Unicode argument, URL (`unsupported-remote`, refused before any lookup) | `dm.input.read_failure`, with `DocumentArgumentError` kept in the chain | none |
  | `-`, inline `{…}` (hints `style="{…}"`), unsupported extension, empty file part, bad offset path | `dm.input.read_failure` | none |
  | invalid UTF-8, malformed content | `dm.data.malformed` | file, line and column |
  | offset missing or not a mapping | `dm.style.invalid_root` | file |
  | bad setter path (`style..x=1`) | `dm.style.invalid_setter` | none |
  | library problems | the problem's own code | file and line for a file; none for a setter |

  Preparation failures go through `ClassifiedFailure::report`.
  `output/report.rs::failure_diagnostics` became `pub` so L1 tests can read
  a failure's records.
- The five `dm.style.*` codes were **not** added to `RENDER_DIAGNOSTIC_CODES`.
  That list holds warnings `--deny` can block, and these codes are always
  errors (see `decisions_for_review`).

### Requirement-to-test mapping (Phase 3)

| Requirement | Tests |
|---|---|
| Rule 2: positional split and classification through the real parser | `args::render::tests::{a_bare_file_is_the_input, a_command_name_after_the_terminator_is_a_file, setters_sit_on_either_side_of_the_input, a_setter_looking_file_is_an_input_with_dot_slash_or_after_the_terminator, a_whole_style_value_is_one_setter, a_value_is_split_at_the_first_equals_only, an_empty_value_is_still_a_setter, only_style_or_a_style_dot_prefix_makes_a_setter, two_inputs_are_a_usage_error, setter_hint_suggests_the_style_spelling}` |
| Rule 3: native tokens kept intact | `a_non_unicode_token_is_kept_native` (unix and windows), `a_non_ascii_unicode_token_is_kept_intact`, `render_style::tests::a_non_unicode_argument_is_an_invalid_reference` |
| Rule 4: root render arguments before a subcommand, generated from clap | `every_root_render_argument_before_a_subcommand_is_a_usage_error` (fails for an unlisted field), `spelled_root_render_arguments_are_rejected_before_a_subcommand`, `shared_controls_before_a_subcommand_are_accepted`, `render_options_have_no_defaults`, L1 `feature_flag::root_feature_before_the_render_subcommand_exits_with_usage_error`, `render_basic::render_features_are_never_combined_across_levels` |
| Rule 5: `schema about --code-block`, the render flag gone | `args::cli::tests::{schema_about_code_block_defaults_to_inverse, schema_about_accepts_code_block_flag, code_block_is_an_unknown_argument_outside_schema_about}`, L1 `render_basic::{render_accepts_every_frontmatter_code_mode, render_rejects_an_invalid_frontmatter_code_mode, render_code_block_flag_is_an_unknown_argument}`, L2 `level2_cli_code_theme_overrides_style_page_code_theme` |
| Root help versus `render --help` | `render_help_lists_every_render_option_and_root_help_does_not`, L1 `help::root_help_points_at_render_help_for_render_options` |
| Rule 14: reference and offset adapter | `render_style::tests::{only_an_implicit_relative_reference_splits_at_its_first_at, a_refused_argument_fails_before_any_lookup, an_inline_object_names_the_inline_setter}`; L1 `render_style_files::` reference cases (whole file, offset, nested, escaped dot, array index, `@` in a file name, magic root, missing and non-mapping offset, `-`, empty file part, remote, unsupported extensions, missing file) |
| Rule 15: loaders (JSON, JSON5, YAML/YML, TOML) | L1 `render_style_files::` loader controls and failures (duplicate keys per format, trailing garbage, second YAML document, invalid UTF-8); library `data::parse::tests::{a_local_document_keeps_the_reader_rules_and_positions_each_value, a_local_document_is_a_tree_format_only}` |
| Rule 16: empty file, root shape | L1 `render_style_files::` `{}`, `{ table: { bad: 1 } }`, YAML `page.color:` null, `[1]` and `42` roots |
| Rule 17: file-relative origin of `page.stylesheet` | L1 `render_style_files::` precedence test asserting the first file's directory survives a later unrelated file |
| Rules 11–12: schema selection from the document, setter codes | L1 `render_style_files::` schema-selection tests (`$schema: { style: { foo: string } }`, stdin) and setter tests (`dm.schema.unknown_key`, `dm.schema.type_mismatch`, `dm.style.invalid_setter`, no line or column) |
| Diagnostics keep codes and file locations | L1 `render_style_files::` record and location tests through `ClassifiedFailure` / `failure_diagnostics` |

All 33 new `render_style_files` tests anchor their requests and fixtures in temp
dirs and run in under 0.4 s each.

### Input-robustness smell grep

The new code in `render_style.rs` and `data/parse.rs` has no
`#[serde(default)]`. Every `unwrap_or_default` in `parse.rs` was already there.
The hits in `render_style.rs` are deliberate:
- `filter_map`s drop notices that have no code, and keys with no proven
  position;
- one `unwrap_or_default` is on optional message text;
- one `.ok()?` turns a line number that does not fit `u32` into "no position".

The full matrix audit stays with Phase 5 Wave 7.

### Gates (macOS, run once after both subagents finished)

| Command | Result |
|---|---|
| `darkmatter: just test` | 15019 run, 15019 passed, 10 skipped, 112.5 s, none at or above 5 s; no reruns |
| `darkmatter: just lint` | exit 0 (includes the zed-dmls wasm32-wasip2 check) |
| `just check-tier-coverage darkmatter` | 0 stranded |
| changed L2 test: `BISCUIT_TEST_LEVEL=2 cargo nextest run -p darkmatter-cli --features terminal-tests -E 'test(code_theme_overrides_style_page_code_theme)' --run-ignored all` | 1 passed |
| `cargo check -p darkmatter -p darkmatter-cli --examples --benches` | OK |
| dependents: `cargo check -p claudine -p claudine-cli --tests` | OK. The library change only adds public items. Other dependents (research, biscuit-icon-cli, playa-cli, sniff-cli, messenger) use only the darkmatter library and were not rebuilt. |

The gate-skip check was
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
It returned `packages: [darkmatter, darkmatter-cli, darkmatter-lsp,
test-toolkit, claudine, simplified-schema]` with `change_class: package`, so
the gates ran. Phase 2's changes are still uncommitted in the working tree.
The Phase 3 file lists in the frontmatter come from this phase's agents and
were checked against `git status`.

### Skipped or not covered

- Setters and `--style` are accepted and prepared by the new module, but not
  yet applied when rendering (Phase 4).
- Trigger-file schema selection for a style preparation has no test. A
  git-repository fixture is needed. The document's `$schema` governing
  selection is tested, including from stdin.
- No cross-OS run. Plan Phase 7 rules out `just cross-check` for this feature.
  Windows-specific risk: the classifier treats `\` as a path separator except
  where it forms a property-path escape (`\.`, `\\`). The Windows-path test
  `themes\user@host.yaml` is a parser test, not a real-path test.
- The browser tier was not run; this phase changed no browser code.
