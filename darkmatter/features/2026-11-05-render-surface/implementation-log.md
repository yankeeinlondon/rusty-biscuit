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
    - |-
        The old hidden root presentation flags (`--margin`, `--fill`, `--theme`,
        and the rest) still work in this phase, because Phase 5 migrates their
        tests. They now travel as one transitional value (`LegacyPresentation`)
        that the render applies over the effective style, so a flag still wins over
        the document, a style file, and a setter. `md render --max-width` goes the
        same way instead of being applied last. The plan says page builders never
        read `Cli`; they don't: the flags are read once at dispatch. The alternative
        was to translate each old flag into a setter now, but the fill flags have no
        setter equivalent (an accepted loss in the spec), so that would have changed
        behavior ahead of the test migration. Phase 5 deletes `LegacyPresentation`,
        `style_claims.rs`, and the `legacy` field together with the flags.
    - |-
        Records about a style file or a setter must not take the document's path
        (the previous phase asked for this). The handoff suggested exempting the
        setter codes the way `dm.setter.invalid` is exempt, but `dm.schema.*` codes
        are shared with document problems, which do need the document's path. So a
        failure now says whether its records are already located
        (`ClassifiedFailure::located`), and the report sink skips the fallback for
        those. Undoing this means going back to a code list, which would wrongly
        strip the document from document schema problems.
    - |-
        Render now selects the document's schema on every run, so a document's
        own unknown style key is judged by its schema. When that selection fails
        (for example, a `$schema` that points at a missing file), a render with no
        setter and no `--style` keeps rendering with the built-in style rules and
        logs the failure only at debug level, as rendering did before. A render
        that has setters or style files fails instead, because their values cannot
        be judged. The alternative, failing every render of such a document, would
        make `md doc.md` stop working for documents that render today. This lives in
        `render_style.rs::prepare_cli_style_inputs`.
    - |-
        Markdown and data outputs print the source and never read the document's
        style. So a problem in the document's own `style:` (a bad value, two
        spellings of one key) does not stop them; a warning that `--deny` made an
        error still does. The CLI layers are checked on every target, as the spec
        requires. To do that, a failed merge is retried with the CLI layers alone.
        A formatted Markdown output prints only the CLI layers' warnings, as it
        printed none of the document's before. A data output reports both. The
        alternative was to fail Markdown and data outputs on a broken document
        style, which the spec's "keep the existing target-dependent handling" rules
        out.
    - |-
        A warning about the document's own style now carries its authored line
        and column (the key's position in the frontmatter, as a document line). It
        used to carry only the file. One existing test that pinned the null
        position was updated (`a_render_style_warning_names_the_document`). Flow
        mappings (`page: { bogus: 1 }`) still give no position.
    - |-
        `md clean` and `md set`, when they print a rendered page (terminal or
        HTML), render it with the document's own style only. The old root layout
        flags no longer reach them (`md --margin 4 clean doc.md` used to indent the
        cleaned output). The plan's Rule 20 asks for this.
    - |-
        Pages are built on a terminal made from the resolved output options
        (`Terminal::from_output_options`) instead of a fresh host detection
        (`Terminal::default()`), as Rule 6 asks. The render itself already used the
        output profile, and image protocol detection still happens inside the
        renderer, so no test output changed. The CLI's L2 real-terminal tests pass.
    - |-
        `--deny` failures for style now carry the blocked warnings as error
        records, with their own codes (`dm.style.deprecated` for a body inline-rule
        style). The message reads "the style was rejected by `--deny CODE`: ...".
        Before, the failure was an untyped error with a generic code.
    - |-
        Schema-trigger findings from the render's schema selection are now
        reported (to stderr, or in a data output's `diagnostics`). If a `--style`
        file then fails to load, the findings are not included in that failure's
        report; selection runs first, but the findings are delivered only after
        every style file has been read. Merging them into the failure is a small
        change in `commands/render.rs::prepare_style` if the reviewer wants it.
    - |-
        With no input and a terminal on stdin, both spellings exit 1. A formatted
        output writes the root help to stderr. A data output writes one failure
        value with code `dm.input.read_failure`. Before, `md` printed the help to
        stdout and exited 0, and `md render` failed with an untyped error. Whether
        stdin is a terminal is now part of `OutputEnvironment` (`stdin_is_terminal`),
        so an in-process test can set it. The plan's single PTY wiring test is part
        of Phase 5's coverage task and was not written here.
    - |-
        `md list-themes` writes a bare array for JSON, JSON5, and YAML; TOML has
        no top-level array, so it writes the list under `value`, as `md get` does
        for a list. Each entry uses the dark-mode description, as `--list-themes`
        did. Formatted outputs end with a line naming `style.page.theme=NAME` and
        `style.page.code.theme=NAME`.
    - |-
        Render completion shows only `style` and the keys under it, using the
        document's schema, through a new library mode (`PathCommand::Render`).
        clap calls the same completer for words after `--`, so the completer checks
        the recovered line for `--` and then offers files only. The old hidden
        presentation flags are still offered by clap's completion when a prefix
        like `--mar` is typed, until Phase 5 removes them; the removed `--save` and
        `--list-themes` are not offered. A real-shell L2 test typed `style.pa<Tab>`
        and expected one match. Since Phase 2 added top-level `style.padding*`
        shorthands, that prefix has several matches, so the test now types
        `style.pag`.
    - |-
        Phase 5 ran its tasks out of the plan's wave order. First the layout tests
        were migrated to setters while the old flags still worked. Then the flags and
        the claim machinery were removed (Wave 8). The input-robustness matrix and the
        alias-parity tests (Wave 7) came last. The plan put Wave 8 after all of
        Wave 7. Doing parity last meant its clap-generated option inventory was the
        final one, without `--max-width` or the old flags. Otherwise it would have
        needed representatives for options that were about to be deleted. The end
        state is the same. Nothing needs redoing.
    - |-
        `images` gained `padding`, `padding-left`, and `padding-right` in the
        embedded style schema and the runtime descriptor. The image style model
        already had and applied these fields, but the schema did not admit them. So
        `--fill-images pad=N` / `indent=N` had no style replacement, and the spec says
        every retained fill behavior must have one. Alternative: accept the loss and
        document it. Reversing this would mean removing the three keys, re-blessing
        six recursive-schema golden fixtures, and deleting
        `layout_fill::images_padding_moves_a_lone_image_as_the_old_fill_did`.
    - |-
        The removed `--page-bg-color` flag accepted `30,30,35` (an RGB triple) and
        `currentColor`. The style color grammar does not; the same colors are written
        `#1e1e23` and `current`. I kept the grammar unchanged, and recorded this as a
        spelling loss, not a capability loss. Widening it would change every color
        field in frontmatter as well, which the spec did not ask for. If the reviewer
        wants the old spellings, widen the shared style color parser and add
        schema/parser agreement cases. No other code depends on this choice.
    - |-
        `render::apply_presentation` now calls `apply_prepared_style` directly.
        Previously, a stylesheet with no recorded origin quietly fell back to the
        document's context. Now it fails with `StylesheetContextMissing`. Style
        preparation always records an origin for a winning `page.stylesheet`, so the
        fallback should be unreachable, and an error is safer than resolving in a
        guessed context. Restoring the fallback would be a small change in
        `apply_prepared_style`.
    - |-
        Two defects were found by the input matrix and fixed in
        `lib/src/style/layer.rs`:
        (1) An unknown member inside a union's object arm, e.g.
        `table.border: { weight: thick, glow: 1 }`, is now treated like any other
        unknown key: a style file warns and drops it, a setter fails with
        `dm.schema.unknown_key` at `table.border.glow`, and a document warns. Before
        the fix it was a whole-value constraint error, and documents dropped it
        silently.
        (2) A style schema problem's message is now prefixed with
        `` `style.<path>`: `` unless the message already names the path.
        Alternative for (2): put the path only in a structured field. Reversing either
        means editing `admit_pass`/`undeclared_members` or `StyleProblem`'s `Display`,
        and the matching `style_layer_matrix` cells.
    - |-
        A failed data output with `--show` still opens its failure value, as
        `docs/cli/output-and-width.md` documents and the code does. The plan's input
        matrix says by default that a malformed style "opens no artifact". I read
        that as being about the rendered page. So the no-artifact assertions use
        formatted outputs, and data-output failures are tested without `--show`. If
        the plan's literal reading wins, `ReportSink::finish_with` must skip opening
        on failure, and the documented behavior changes.
    - |-
        Library test helpers that chain the `apply_*_style` functions (the
        structural gate, parity tests, cutover reference, bench, and
        tree-features characterization) still pass
        `FileResolutionContext::new(std::env::temp_dir())` to `apply_bespoke_style`
        rather than a request-derived context or `apply_prepared_style`. Switching
        them to full preparation would change what those tests prove, because
        preparation validates differently from `from_frontmatter`. The context is
        explicit and never the process directory. Changing it later touches only
        those helpers.
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
    Phase 5 finished all of Wave 7 and Wave 8. The old root presentation
    flags, `md render --max-width`, `cli/src/style_claims.rs`,
    `lib/src/style/cli_claims.rs`, and every `*StyleOverrides` mask are gone.
    `apply_*_style` take no mask. `render::apply_presentation` is one
    `apply_prepared_style` call.

    For Phase 6 (documentation):
    - Partial doc fixes already landed: `docs/cli/render.md` has the full
      presentation-flag migration table, a fill sub-table, and a "Themes and line
      numbers" section. `docs/rendering/style.md` has one precedence section,
      "Later Style Layers Win, Field by Field". Examples that would now fail were
      fixed in `output-and-width.md`, `mermaid.md`, and `layouts-and-style.md`.
      Review these rather than rewriting them.
    - `docs/rendering/style.md` still says "sub-spec #N" in many places, which
      predates this feature. Docs must not name specs; consider cleaning it.
    - `images` now accepts `padding`/`padding-left`/`padding-right`; document it
      with the other component keys.
    - Two style-preparation behaviors changed and need docs:
      - an unknown member of a union's object arm (e.g. `table.border.glow`) is an
        unknown key (warned in a file or document, an error for a setter);
      - schema problems are prefixed with `` `style.<path>`: ``.
    - Color spellings `30,30,35` and `currentColor` (old `--page-bg-color`) are
      not accepted; use `#1e1e23` and `current`. The migration table should say
      so if it doesn't.
    - The working tree was committed by the pipeline mid-phase (commit
      `498f7cbbf`), so `git status` alone does not show Phase 5's files; use the
      `*_during_phase_5` frontmatter lists.
source_files_during_phase_4:
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/completion.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/output.rs
    - darkmatter/cli/src/args/path_completion.rs
    - darkmatter/cli/src/args/render.rs
    - darkmatter/cli/src/args/render_prepare.rs
    - darkmatter/cli/src/artifact.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/list_themes.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/io/mod.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/src/output/command.rs
    - darkmatter/cli/src/output/mod.rs
    - darkmatter/cli/src/output/report.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/src/render_style.rs
    - darkmatter/cli/src/style_claims.rs
    - darkmatter/cli/schemas/output/list-themes.yaml
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/cli/tests/l1/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/cli/tests/l1/clean_schema.rs
    - darkmatter/cli/tests/l1/compose_report_diagnostics.rs
    - darkmatter/cli/tests/l1/feature_flag.rs
    - darkmatter/cli/tests/l1/help.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/mermaid_features.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - darkmatter/cli/tests/l1/path_completion.rs
    - darkmatter/cli/tests/l1/render_dispatch.rs
    - darkmatter/cli/tests/level2/level2_path_completion.rs
docs_updated_during_phase_4:
    - darkmatter/cli/README.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/cli/delta.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/output-and-width.md
    - darkmatter/docs/cli/render.md
    - darkmatter/docs/rendering/mermaid.md
    - darkmatter/docs/rendering/style.md
    - darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_4:
    - darkmatter/docs/cli/list-themes.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/cleanup.md
    - .claude/skills/darkmatter/cli.md
source_files_during_phase_5:
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/completion.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/parsers.rs
    - darkmatter/cli/src/args/render.rs
    - darkmatter/cli/src/args/render/tests.rs
    - darkmatter/cli/src/args/render_prepare.rs
    - darkmatter/cli/src/args/wrappers.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/lib.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/src/style_claims.rs
    - darkmatter/cli/tests/common/mod.rs
    - darkmatter/cli/tests/common/style.rs
    - darkmatter/cli/tests/l1/code_block.rs
    - darkmatter/cli/tests/l1/feature_flag.rs
    - darkmatter/cli/tests/l1/help.rs
    - darkmatter/cli/tests/l1/layout_alignment.rs
    - darkmatter/cli/tests/l1/layout_fill.rs
    - darkmatter/cli/tests/l1/layout_flags.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/mermaid_features.rs
    - darkmatter/cli/tests/l1/output_envelope.rs
    - darkmatter/cli/tests/l1/render_alias_parity.rs
    - darkmatter/cli/tests/l1/render_basic.rs
    - darkmatter/cli/tests/l1/render_style_matrix.rs
    - darkmatter/cli/tests/l1/render_style_wiring.rs
    - darkmatter/cli/tests/level2/level2_code_block_styling.rs
    - darkmatter/cli/tests/level2/level2_disclosure_blocks.rs
    - darkmatter/cli/tests/level2/level2_frontmatter_images.rs
    - darkmatter/cli/tests/level2/level2_frontmatter_tables.rs
    - darkmatter/cli/tests/level2/level2_horizontal_rules.rs
    - darkmatter/cli/tests/level2/level2_layout_dimensions.rs
    - darkmatter/cli/tests/level2/level2_ordered_lists.rs
    - darkmatter/cli/tests/level2/level2_render_no_input.rs
    - darkmatter/cli/tests/level2/main.rs
    - darkmatter/lib/benches/render_pipeline_steps.rs
    - darkmatter/lib/src/layout/error.rs
    - darkmatter/lib/src/layout/page/tests.rs
    - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
    - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
    - darkmatter/lib/src/style/apply.rs
    - darkmatter/lib/src/style/bespoke.rs
    - darkmatter/lib/src/style/cli_claims.rs
    - darkmatter/lib/src/style/coverage_tests.rs
    - darkmatter/lib/src/style/descriptor.rs
    - darkmatter/lib/src/style/layer.rs
    - darkmatter/lib/src/style/mod.rs
    - darkmatter/lib/tests/browser/browser_render.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
    - darkmatter/lib/tests/l1/cutover_reference.rs
    - darkmatter/lib/tests/l1/disclosure_render_targets.rs
    - darkmatter/lib/tests/l1/horizontal_rule_integration.rs
    - darkmatter/lib/tests/l1/layout_snapshots.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/style_frontmatter.rs
    - darkmatter/lib/tests/l1/style_frontmatter_parity.rs
    - darkmatter/lib/tests/l1/style_layer_matrix.rs
    - darkmatter/lib/tests/l1/tree_features_characterization.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs
    - darkmatter/schemas/partials/style.yaml
docs_updated_during_phase_5:
    - darkmatter/docs/cli/developing.md
    - darkmatter/docs/cli/output-and-width.md
    - darkmatter/docs/cli/render.md
    - darkmatter/docs/rendering/code-highlighting.md
    - darkmatter/docs/rendering/mermaid.md
    - darkmatter/docs/rendering/style.md
    - darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/darkmatter/rendering.md
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

## Phase 4

### Execution approach

All three Phase 4 tasks were done in this session, one after another, with no
subagents. Every task edits the `darkmatter-cli` crate, and Wave 6's two tasks
both depend on Wave 5's new dispatch, so parallel agents would have blocked on
cargo's lock and seen each other's half-built state. Focused tests ran after
each task; the area gates ran once at the end.

### Wave 5: Unify render dispatch

- **One entry for both spellings.** `commands::render::dispatch_render(&RenderOptions, &Cli, &MdRequest, &OutputEnvironment)`:
  - builds `PreparedRenderOptions` and attaches `LegacyPresentation::from_cli(cli, options.max_width)`;
  - resolves the output once (`resolve_output` + `source_markdown_width`);
  - handles no input (below);
  - calls `run_render(&PreparedRenderOptions, &ResolvedOutput, &MdRequest)`, which takes no `Cli`.

  `run_subcommand`'s `Render` arm passes `OutputEnvironment::detect()`.
  `main.rs` no longer has its own implicit branches: the root `--save`
  (implicit clean), the root `--list-themes`, and the "no input → help on
  stdout, exit 0" branch are gone. Only `--completions`, help, and version run
  before dispatch, and the removed-policy-flag check is unchanged.
- **Removed root flags.** `Cli.save` and `Cli.list_themes` were deleted, so
  `md doc.md --save` and `md --list-themes` are clap unknown-argument errors
  (exit 2). `validate_subcommand_usage` no longer special-cases `--save`.
- **Style preparation before any output** (`Render::prepare_style`), in this order:
  1. `prepare_cli_style_inputs` selects the schema and reads every `--style`
     file and the setters.
  2. The selection's trigger findings are delivered, to the sink or stderr.
  3. `render_style::document_style_layer` builds the document's `style:` layer
     from the original frontmatter, before cleanup. Its positions are shifted
     to document lines.
  4. `render_style::prepare_render_style` merges everything.
  5. The deny checks run (below), then the warnings are reported.

  Cleanup with `--indent` runs only after this, and the style never enters it.
- **Target rules** (Rule 13). A page target (terminal, plain, HTML,
  MarkdownPlus) fails on any problem. A source target (Markdown, data) does
  not fail on a problem in the document's own `style:`, but a warning that
  `--deny` made an error still fails it. In that case preparation runs again
  with the CLI layers only, so their problems and warnings are still found.
  A formatted source output prints only the CLI layers' warnings. A data
  output reports every warning.
- **Deny.**
  - `--deny style`: preparation refuses unknown and deprecated keys in every
    layer. The body's inline-rule styles (`--- { style: waves }`) are now
    refused in `Render::stop_if_denied`.
  - `--deny <code>` (one warning code) blocks every warning record with that
    code, including the inline-rule deprecations.

  The failure's records are the blocked warnings, with severity changed to
  error.
- **Page builders** (Rules 6, 20):
  - `render::terminal_page(input_path, &PageInputs, &PagePresentation, &Terminal)`
  - `artifact::html_page(&PageInputs, &PagePresentation)`
  - `render::apply_presentation(page, &PageInputs, &PagePresentation, target)`

  `PagePresentation { style: &PreparedStyle, features, mermaid, legacy }`.
  The terminal is `Terminal::from_output_options(&output.options)`, in render
  and in `document_value`. The stylesheet resolves in the context of the
  winning `page.stylesheet` origin. `apply_style_frontmatter`,
  `style_diagnostics`, `stop_if_style_code_denied`, `log_style_warnings`, and
  `with_cli_presentation` were removed. `document_value` (`md clean`, `md set`)
  uses `render::document_style`, which prepares the document layer alone: no
  root flag and no CLI layer reaches an edit's rendering. Its `cli` parameter
  is gone, and so is `clean_value`'s and `set_value`'s.
- **Records the document's path must not be stamped on** (previous phase's
  item 1).
  - New `ClassifiedFailure::located` and `located_from`. `located_from` keeps
    the wrapped report as `Error::source`, so `DocumentArgumentError` still
    renders its status block.
  - New `output::report::failure_records`, which returns the records and a
    `located` flag.
  - `ReportSink::failure` and `CommandRun::finish` skip
    `with_document_fallback` for a located failure.
  - Every failure from `render_style.rs`, and every load or setter failure
    that `prepare_cli_style_inputs` wraps, is located. So a missing style file
    and a setter both keep `source: null`.

  The handoff suggested exempting the codes instead. That does not work:
  `dm.schema.*` codes are also used by document records, which do need the
  fallback.
- **Trigger findings** (previous phase's item 2) are delivered by render. A
  data output adds `finding_diagnostic` rows. A formatted output writes
  `emit_warnings` rows to stderr.
- **Schema selection failure.** A render with no CLI layer falls back to the
  built-in descriptor (`SelectedDocumentSchema::none()`), with a debug-log
  line only. A render with CLI layers fails.
- **No input** (Rule 21). `OutputEnvironment` gained `stdin_is_terminal`,
  which is detected without reading stdin.
  - A formatted output writes `Cli::command().render_help()` to stderr and
    returns `ReportedExit(1)`.
  - A data output writes one failure envelope with code `dm.input.read_failure`.
  - Redirected stdin, including empty stdin, still goes through
    `load_opened_document`.
- **Setter hint** (from the Phase 3 handoff). When an input does not resolve
  and `setter_hint` matches it, `with_setter_hint` attaches the hint to its
  `DocumentArgumentError`. It is added to the message (`; to set a style,
  write \`style.…\``) and replaces the status block's hint.
- **Rule 3.** `io::parse_argument` uses `to_str()`. A non-Unicode argument is
  a typed `InvalidReference` (`InvalidSyntax`) and is never opened under its
  lossy spelling.
- **Transitional.** `style_claims::LegacyPresentation { claims, theme, code_theme, line_numbers }`
  carries the old hidden root flags until Phase 5 removes them.
  `md render --max-width` now goes into `claims.max_width` instead of a
  trailing `with_max_width`. `render::apply_cli_layout_flags(page, &Cli)`
  remains only for test callers.

### Wave 6, task 1: Add theme command

- `Command::ListThemes { output }` and `commands/list_themes.rs`. The data
  value is a `DataShape::Bare` array of `ThemeEntry { name, description }`,
  in `ThemePair::all()` order, using the dark descriptions.
  - The Markdown is a code-span list followed by setter guidance.
  - The terminal output uses `UnorderedList` and `Prose`.
  - The guidance line names `style.page.theme=NAME` and
    `style.page.code.theme=NAME`.
- New `COMMAND_LEAVES` row, `Cli::requests_data_output` arm, and
  `cli/schemas/output/list-themes.yaml`. The schema's first arm is
  `value: theme[]`; TOML writes the array under `[[value]]`.
- `md clean --save`, `md set --save`, `md code-block --theme`, and
  `md schema about --code-block` keep their own definitions and tests.

### Wave 6, task 2: Connect style completion

- Library: new `PathCommand::Render`. It spells keys as setters, like
  `Compose`, and `command_can_spell` keeps only `style` and the paths under
  it. A private `PathCommand::assigns` replaces the `== Compose` checks.
- CLI:
  - `args::path_completion::complete_render_word` is the `ARGS` completer, so
    it serves both spellings. `DocumentSite::for_render` takes the one
    non-setter word as the input and the style setters as earlier paths, and
    uses `SchemaPolicy::Edit`, the same baseline and trigger selection that
    render uses.
  - `CommandLine` records `escaped`: clap calls the `ARGS` completer even
    after `--`, so a word after `--` completes files only.
  - `--style` gets `complete_style_files` (directories, plus `json`, `json5`,
    `yaml`, `yml`, and `toml` files).
  - Completion never opens a `--style` file.

### Requirement-to-test mapping (Phase 4)

| Requirement | Tests |
|---|---|
| Both spellings give equal bytes, stderr, and status on every target (terminal, plain, HTML, Markdown, MarkdownPlus, and the four data formats) | `render_dispatch::both_spellings_print_the_same_bytes_for_every_target` |
| CLI layers reach the page; precedence is document < file < setter, whatever the order on the line | `render_dispatch::a_setter_and_a_style_file_move_the_page_and_the_later_layer_wins` |
| A style failure gives exit 1, no stdout, and no artifact opened (`MD_DRY_RUN=fail`) on every target; a data failure gives one value, empty stderr, and a setter record with null `source` | `render_dispatch::a_bad_setter_fails_every_target_before_anything_is_printed_or_opened` |
| Style-file failures and warnings name the style file, never the document | `render_dispatch::{a_style_file_failure_names_the_style_file_never_the_document, a_style_file_warning_is_reported_on_every_target_and_names_its_file}` |
| Target rules for authored style; CLI style never in source output | `render_dispatch::{a_formatted_source_output_shows_no_warning_of_the_document_style, a_source_output_carries_no_cli_style_and_ignores_a_broken_document_style}`, existing `output_envelope::a_style_warning_*`, `output_commands::render_deny_takes_the_style_codes_it_reports` |
| `--deny style` and `--deny dm.style.deprecated` block inline-rule styles | `render_dispatch::denying_style_blocks_an_inline_rule_style_and_a_warning_code_blocks_on_its_own` |
| Trigger findings delivered by render (data and formatted) | `render_dispatch::render_reports_an_invalid_trigger_its_schema_selection_skipped` |
| Document style warnings keep their authored position | `compose_report_diagnostics::a_render_style_warning_names_the_document` (now line 3, column 3) |
| Setter hint | `render_dispatch::a_misspelled_setter_suggests_the_style_spelling` |
| Redirected and empty stdin; `-` | `render_dispatch::redirected_stdin_renders_in_both_spellings_even_when_empty` |
| No input with a terminal on stdin: both spellings and both families exit 1 | `render_dispatch::no_input::both_spellings_fail_with_status_one_for_every_family` (in process, injected `stdin_is_terminal`) |
| Rule 3, non-Unicode input | `render_dispatch::a_non_unicode_input_is_an_invalid_reference_not_a_lossy_file` (unix) |
| Root `--save` and `--list-themes` are unknown arguments | `clean::root_save_is_an_unknown_argument_that_writes_nothing`, `output_commands::{the_removed_root_save_is_a_usage_error_even_with_a_data_output, root_list_themes_is_unknown}` |
| `md clean --save` keeps the behavior the root shorthand tests covered | `clean::test_clean_save_cleans_in_place_and_reports_delta`, `clean_fidelity::{clean_save_writes_a_changed_document, clean_save_leaves_a_declined_document_untouched}`, `clean_schema::test_clean_save_uses_default_schema_state_and_repairs` |
| `md list-themes`: order, every data format, Markdown and plain controls, schema conformance | `output_commands::{conforms_list_themes_json_is_every_theme_in_canonical_order, conforms_list_themes_in_every_other_data_format, list_themes_markdown_lists_every_theme_with_the_setter_guidance, help_list_themes, list_themes_takes_no_input, every_leaf_has_a_schema_file_and_a_conformance_test, every_schema_file_shares_one_diagnostic_type}`, `help::test_list_themes`, unit tests in `commands/list_themes.rs` |
| Completion: `style.` → `style.page.`, `style.ul.` → `padding-left=`, enum and theme values, custom, open, and broken schemas, no setter after `--`, `--style` file filter, no removed flags, no style file opened and no command run | `path_completion::{render_offers_only_style_setters_and_files_in_both_spellings, render_walks_the_style_schema_to_keys_and_values, render_reads_the_documents_own_style_declaration, render_after_the_terminator_offers_files_and_no_setter, style_offers_only_style_files_and_directories, render_offers_no_removed_root_flag, render_completion_opens_no_style_file_and_runs_nothing}`; library `schemas::completion::tests::render_offers_only_style_and_spells_each_key_as_a_setter` |
| Legacy flags still win over the effective style | `render::tests::a_legacy_flag_wins_over_the_effective_style_and_an_omitted_one_leaves_it`; existing `layout_*`, `mermaid_features`, `feature_flag` tests, migrated to the new page-builder API |

Process tests run 0.05–2 s each. The slowest is
`a_bad_setter_fails_every_target…`, with 8 spawns, at about 2 s.

### Input-robustness matrix and smell grep

No new file or configuration reader was added in this phase. The style-file
loaders and layer validators are Phase 2 and Phase 3 code, and their full
matrix belongs to Phase 5 Wave 7. I grepped the new and changed code
(`commands/render.rs`, `render_style.rs`, `render.rs`, `list_themes.rs`, and
the completion changes) for `#[serde(default)]`, `unwrap_or_default()`,
`filter_map(.. as_str())`, and swallowed `.ok()`. Nothing was found on a
load-bearing parse:
- `document_style_layer` reads an absent `style:` as `Value::Null`, which
  preparation already treats as an empty document layer (Phase 2 behavior).
- `complete_render_word` uses `.ok()`, but completion is advisory by contract.

### Gates (macOS, run serially by this session; no subagents)

| Command | Result |
|---|---|
| `darkmatter: just test`, first run | 13236 passed, then stopped (fail-fast) on 1 failure: `spawn_site_guard` caught `.env("MD_DRY_RUN")` in the new `render_dispatch.rs`. In the same run the single-test spelling-parity check took 7.8 s, over the 5 s budget. Fixed: `MD_DRY_RUN` is now set through `command_builder().application_input`, and the parity check became one test per target with a guard. |
| `darkmatter: just test`, second run | 15055 run, 15055 passed, 10 skipped, 132.6 s |
| `darkmatter: just lint` | exit 0 (run before and after the test fixes; includes the zed-dmls wasm32-wasip2 check) |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `cargo check -p darkmatter -p darkmatter-cli --examples --benches` | OK |
| `cargo check -p darkmatter -p darkmatter-cli --tests --features terminal-tests` | OK |
| `just _test_l2 darkmatter-cli --features terminal-tests` | 77 of 82 passed. The 5 `level2_path_completion::a_tab_round_trip_in_*` tests failed. The cause is Phase 2, not this phase: the top-level `style.padding*` shorthands made `style.pa<Tab>` ambiguous. The tests now type `style.pag`. Rerun: 6 of 6 passed. |
| dependents: `cargo check -p claudine -p claudine-cli --tests` | OK. claudine-cli uses only `darkmatter_cli::approval`, and no crate outside darkmatter matches on `PathCommand`. |

The gate-skip check was
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
It returned `packages: [darkmatter, darkmatter-cli, test-toolkit]` with
`change_class: package`, so the gates ran.

### Skipped or not covered

- The library's L2 suite, the DMLS L2 suite, and the browser tier were not
  run. This phase changed no library rendering code, only one completion enum
  variant in the library, and no browser code.
- The plan's single controlled-PTY wiring test for no input was not written.
  It is in Phase 5 Wave 7's scope ("no-input TTY"). The in-process test
  covers the decision.
- If a `--style` file fails to load, the trigger findings are not included in
  that failure's report (see `decisions_for_review`).
- No cross-OS run. Plan Phase 7 rules out `just cross-check` for this
  feature. The changes with OS-specific behavior:
  - `parse_argument` now refuses a non-Unicode argument instead of opening its
    lossy spelling. That test is unix-only.
  - The completion file filter matches extensions case-insensitively.

## Phase 5

### Execution approach

Four foreground subagents ran **one after another**, because every task
compiles the `darkmatter-cli` crate. They did not run in the plan's
wave order:

1. Wave 7 "Migrate layout coverage", while the old flags still worked
2. Wave 8 "Retire style claims", CLI half (flags, `style_claims.rs`, `--max-width`)
3. Wave 8, library half (`cli_claims.rs`, the override masks, lib tests and benches)
4. Wave 7 "Cover input failures"
5. Wave 7 "Cover alias parity"

The parity inventory is generated from clap, so it was written last. By then
the removed flags and `--max-width` were gone, and it covers the final option set. Each agent
ran only its own scoped tests; the area gates ran once at the end, with no
agent running.

### Wave 7: Migrate layout coverage

- Every CLI L1 and L2 test that used an old flag now uses setters or a
  `style={…}` object:
  - L1: `layout_flags`, `layout_alignment`, `layout_fill`,
    `layout_style_frontmatter`, `output_envelope`, `render_basic`,
    `code_block`, `feature_flag`, `mermaid_features`
  - L2: `level2_code_block_styling`, `level2_layout_dimensions`,
    `level2_frontmatter_images`, `level2_disclosure_blocks`,
    `level2_frontmatter_tables`, `level2_horizontal_rules`,
    `level2_ordered_lists`
- New `cli/tests/common/style.rs` builds a page in process the way `md` does:
  clap, then `PreparedRenderOptions`, then `prepare_cli_style_inputs` plus the
  document layer, then `apply_presentation`. Each run gets a temp-dir request.
  It also holds the ten-component broadcast helpers.
- What the migrated tests cover:
  - every broadcast target;
  - each fill mode (`full`, `pad`, `indent`, `max`, `explicit`);
  - `pad`/`indent` resetting inherited vertical padding, with a negative control;
  - `indent` with the side named (left, right, both);
  - HR placement at 50 columns (`style.hr.padding-left=4` gives 4 cells in, 46 wide).

  The old "flag beats frontmatter" tests are now "a setter beats the document
  field by field, and an unrelated document field survives".
- **Product fix:** `images` had no `padding`, `padding-left`, or
  `padding-right` in the schema or descriptor, although `ImageStyle` carries
  and applies them, so `--fill-images pad=N` could not be expressed. They were
  added, and the plain output now matches the old flag byte for byte. New
  test: `layout_fill::images_padding_moves_a_lone_image_as_the_old_fill_did`.
  The descriptor count went from 204 to 207, and six
  `recursive_schema_golden` fixtures were re-blessed. Their only content change
  is the three keys.
- **Spelling loss, recorded for review:** the old `--page-bg-color` parser
  accepted `30,30,35` and `currentColor`. The style color grammar does not;
  write `#1e1e23` and `current` instead.

### Wave 8: Retire style claims

- CLI deleted:
  - every hidden root presentation field and its aliases in `args/cli.rs`
  - `RenderOptions.max_width`
  - `PageBackgroundArg`, `PageAlignmentArg`, `CliFill`, `parse_cli_fill`,
    `parse_max_width`, `parse_cli_length`
  - `cli/src/style_claims.rs` (`LegacyPresentation`)
  - the `legacy` fields of `PreparedRenderOptions` and `PagePresentation`
  - `render::apply_cli_layout_flags`

  `apply_presentation` is now one call to `apply_prepared_style`. No entry was
  added to `REMOVED_FLAGS`. `dispatch_render` still takes `&Cli`, but only for
  the shared `--width` and `-v`; `run_render` and the page builders read no `Cli`.
- Library deleted:
  - `lib/src/style/cli_claims.rs`: `CliStyleClaims`, `FillClaim`,
    `apply_cli_claims`, and every `*_overrides_from_claims`
  - `PageStyleOverrides`, `ComponentStyleOverrides`, `ListStyleOverrides`,
    `HrStyleOverrides`, `DisclosureStyleOverrides`, `BespokeStyleOverrides`

  `apply_{page,component,list,hr,disclosure}_style(page, &StyleFrontmatter)`
  and `apply_bespoke_style(page, style, assets, context)` take no mask.
  Callers were updated: the structural gate, the parity tests, the bench, the
  browser and L2 suites, and six lib L1 files. 11 tests that only proved a mask
  suppressed a field were deleted. The two lib tests of "CLI beats frontmatter"
  became two-layer `prepare_style_layers` tests (`style_frontmatter::{setter_layer_overrides_document_left_margin_field_by_field, code_theme_setter_layer_overrides_document_code_theme}`).
- Removal tests:
  - `args::cli::tests::removed_render_flags_are_unknown_arguments`: all 49
    removed spellings and aliases, through `md doc.md …` and
    `md render doc.md …`, each `UnknownArgument` with exit 2
  - `the_root_declares_only_shared_controls_besides_render_options`: the guard
  - spawned `render_basic::removed_render_flags_are_unknown_arguments`
- Repository search: no claim or mask type name remains outside
  `features/`/`fixes/`. Old flag spellings remain only in removal tests,
  migration-test comments, the `docs/cli/render.md` migration table, one
  sentence of `docs/topics/layouts-and-style.md` pointing to that table, and
  the removal-test description in `.claude/skills/darkmatter/cli.md`. The
  `--fill`/`--theme` hits in `biscuit-terminal` belong to `bt`'s own flags.
- Docs: `docs/cli/render.md` has a "Presentation flags" migration table for
  every removed flag and alias, a fill sub-table, and a "Themes and line
  numbers" section. The stale `md FILE --save` shorthand row is fixed.
  `docs/rendering/style.md` replaces the claim and mask sections with one
  precedence section, "Later Style Layers Win, Field by Field", which has a
  Mermaid flow. The integration order now matches `apply_prepared_style`; the
  old lists had HR after color, which was wrong. Failing examples were fixed in
  `output-and-width.md`, `mermaid.md`, and `layouts-and-style.md`.

### Wave 7: Cover input failures

New tests:
- `lib/tests/l1/style_layer_matrix.rs`: 111 field-shape cells at
  `prepare_style_layers`, one `#[test]` each
- `cli/tests/l1/render_style_matrix.rs`: 22 in-process loader and CLI-layer tests
- `cli/tests/l1/render_style_wiring.rs`: 20 subprocess tests, including control,
  malformed, and semantic failure per loader (JSON, JSON5, YAML, TOML), via
  `loader_wiring!`

| Matrix row | Where covered |
|---|---|
| CLI style root | `style_layer_matrix::root_*`; `style_layers::admission::a_cli_root_must_be_a_mapping` |
| Page rows and shorthands | `style_layer_matrix::rows_*` (full walk on `margin-top`; siblings `mb`, `padding-top`, `my`, `py`, `padding`; `margin: '10%'` fails) |
| Horizontal lengths and cap | `style_layer_matrix::length_*` (full walk on `ml`; `mx`, `padding-right`, `table.padding-left`), `cap_*`; `render_style_wiring::the_output_width_still_bounds_a_valid_cap` |
| Enum, theme, bool | `style_layer_matrix::` theme walk (alias `Visual_Studio_Dark` passes); `code.mode`, `code.theme`, `alignment`, `line-numbers` control + invalid |
| Compound border | `style_layer_matrix::border_*`; existing `style_layers::merging::*` for partial objects and scalar→object |
| Admitted array / open metadata | `style_layer_matrix::` `tags: number[]` cells (`[]` replaces, `[]` alone is an empty file, element paths `tags.1`/`tags.0`); existing `an_open_object_takes_only_a_whole_value` |
| Required | existing `required_style_fields_may_come_from_two_layers`; new `required_from_a_document_satisfies_a_sparse_file`, null cases |
| Unknown | `style_layer_matrix::` invalid sibling and setter-unknown cells; `render_style_wiring::denying_style_blocks_a_style_file_unknown_key` |
| JSON / JSON5 / YAML / TOML | `render_style_matrix::{json_*, json5_*, yaml_*, toml_*}` plus existing `render_style_files::` loader cases; per-loader subprocess `render_style_wiring::{json,json5,yaml,toml}_loader::*` |
| Reference/offset adapter | existing `render_style_files::` and unit tests; `render_style_wiring::a_remote_style_file_makes_no_request` (mock server, zero requests) |

Other cells the plan names:
- CSS taken from an earlier file, with an unrelated later file
- stdin Markdown with local `--style` files
- document versus launch versus file CSS origins
- source, data, terminal, and plain never read CSS
- HTML and MarkdownPlus read local CSS, and a remote stylesheet stays a link
- a malformed style reads no CSS and opens nothing
- no expression or shell runs, with sentinel files
- repeated setters, and a repeated `style=` replacing the whole assignment
- empty file versus inline `{}`
- an invalid overridden file
- a custom open schema
- a setter's location is its argument plus the canonical path, with no position

All are in `render_style_wiring` or `render_style_matrix`.

**Defects found and fixed** (`lib/src/style/layer.rs`):

- **A. An unknown member of a union's object arm.** For example
  `table.border: { weight: thick, glow: 1 }`. It was a whole-value
  `dm.schema.constraint`: a file failed instead of warning, a setter had the
  wrong code, path, and origin, and a document dropped `glow` silently.
  - Fix: `admit_pass` asks `declared_shape` for the undeclared members of a
    failing object (new `undeclared_members`), records them as unknown keys,
    and judges what remains.
  - Audit: every border consumer (including `code-block`) and every loader
    feed the same `admit` path, and all their cells pass.
- **B. A schema problem did not name its property.** `StyleProblem`'s
  `Display` now prefixes `` `style.<path>`: `` unless the message already
  names the path.
  - Audit: runtime and conflict messages already carried the path.

Smell grep over `layer.rs`, `normalize.rs`, `render_style.rs`, and the
`parse_local_document` path: no defect.
- `normalize.rs:290` `.ok()?` is the alignment broadcast only. The value is
  still rejected (`alignment_unknown_fails`).
- The `render_style.rs` `filter_map`s, `unwrap_or_default`, and `.ok()?` are
  the ones the Phase 3 log already lists.
- `#[serde(default)]` on `PageStyle`, `CodeStyle`, `CommonStyle`, and
  `ComponentBorderObject` maps absent and null to `None`, which is correct.
  CLI layers strip nulls (Rule 10), and a document null means absent.

### Wave 7: Cover alias parity

- `cli/tests/l1/render_alias_parity.rs` has 42 tests.
  - **Option inventory:** built from `RenderOptions::augment_args`, with one
    representative row per id and value (every `--output` target and
    `--feature` name), plus `OVERRIDES` for conflict cases.
  - **Guards:**
    - `every_render_argument_has_a_representative`
    - `every_root_argument_is_classified` (render, shared, root-only, help)
    - `root_only_arguments_are_not_render_arguments`
  - **Setters:** generated from the effective schema through `path_candidates`
    (358 paths) and typed by `declared_shape`; 80 sampled per (type, last key),
    with family guards.
  - **Comparison:** in process. Both spellings must give the same parse result
    or error kind and exit, shared controls, `PreparedRenderOptions` (as Debug
    text), and effective style (merged value, retained keys, projection,
    origins, warnings).
  - **Spawned rows**, one `#[test]` per row, each list guarded:
    - an option before a subcommand exits 2 (8)
    - early actions `--help`, `--version`, `--completions zsh`, `COMPLETE=zsh` (4), plus `-V`
    - styled byte parity for terminal, plain, html, and markdown-plus, with
      pinned width and environment (4)
    - stdin forms: empty and nonempty, with and without `-`, plain and json,
      `--width` alone, `-v` alone (10)
    - command-name files `-- toc` and `./toc` in both spellings (4), with
      stdin closed and a 5 s timeout
- `cli/tests/level2/level2_render_no_input.rs` is the plan's single PTY wiring
  test. It runs in a detached tmux pane (no focus) and checks that stdin is a
  terminal. For both `md` and `md render`, stdout is empty, the exit is 1, and
  stderr holds the same root help.
- No spelling divergence was found, so there was no product change.

### Requirement-to-test mapping (Phase 5)

| Requirement | Tests |
|---|---|
| Alias parity of parse, prepared value, and effective style for every render option and sampled setter, generated from clap and the schema, with guards | `render_alias_parity::{every_render_argument_has_a_representative, every_root_argument_is_classified, every_style_setter_family_decides_the_same_in_both_spellings, …}` |
| Every nonshared render option before a subcommand exits 2 | `args::render::tests::every_root_render_argument_before_a_subcommand_is_a_usage_error` (in process); `render_alias_parity` spawned rows (8) |
| Help, version, and completion are separate early actions | `render_alias_parity` early-action rows (4) plus `-V` |
| Styled byte parity for terminal, plain, HTML, and MarkdownPlus | `render_alias_parity::styled_*_bytes_agree`; unstyled per target in `render_dispatch` |
| No-input TTY (PTY wiring), redirected empty and nonempty stdin, `-`, shared-controls-only, command-name inputs | `level2_render_no_input::level2_render_no_input_on_a_terminal_prints_root_help_in_both_spellings`; `render_alias_parity` stdin and command-name rows |
| Input robustness matrix | the "Cover input failures" table above |
| Layout behavior retained through setters (broadcasts, every fill mode, side naming, padding reset, HR placement) | the migrated `layout_*` L1 files and the seven L2 files listed above |
| Removed flags are unknown arguments (exit 2), with no `REMOVED_FLAGS` entry | `args::cli::tests::{removed_render_flags_are_unknown_arguments, the_root_declares_only_shared_controls_besides_render_options}`, `render_basic::removed_render_flags_are_unknown_arguments`, `help::*`, `args::render::tests` help checks |
| A layer beats the document field by field (replaces the claim precedence) | `style_frontmatter::{setter_layer_overrides_document_left_margin_field_by_field, code_theme_setter_layer_overrides_document_code_theme}`, `layout_style_frontmatter::*` |

### Gates (macOS, run by this session after every subagent finished)

| Command | Result |
|---|---|
| `darkmatter: just test` | 15172 run, 15172 passed, 10 skipped, 152.4 s; no test at or above 5 s; no reruns |
| `darkmatter: just lint` | exit 0 (includes the zed-dmls wasm32-wasip2 check) |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `cargo check -p darkmatter -p darkmatter-cli --examples --benches` | OK |
| `just _test_l2 darkmatter-cli --features terminal-tests` (full CLI L2) | 83 passed |
| `just _test_l2 darkmatter --features terminal-tests` (full library L2) | 18 passed |
| browser (subagent, scoped): `just test-browser browser_component_ browser_page_ browser_table_center browser_local_image browser_translucent browser_list_left` | 13 passed, headless Chrome |
| dependents: `cargo check -p claudine -p claudine-cli --tests` | OK. No package outside `darkmatter/` uses a removed or changed symbol (repository grep for the `apply_*_style` functions, the claim types, and the masks). |

**Gate-skip check.** Midway through this phase the pipeline's commit process
committed the working tree, including all of Phase 5's code:
`498f7cbbf feat(darkmatter): complete layered render surface and CLI migration`
and `da0c57a72 planning(darkmatter): record Phase 4 close …`. Both were signed
by the author. This session staged or committed nothing.

When the check ran, `git status --porcelain --untracked-files=all` was
empty, so
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
printed nothing; it had no files to classify. The gates were run anyway,
above. Run on one of this phase's files (`darkmatter/cli/src/render.rs`),
the calculator returns `packages: [darkmatter-cli, test-toolkit]`, so
skipping was never an option.

### Skipped or not covered

- The full browser tier was not run. Only the browser tests that use the
  edited `style_page_doc` helper ran (13, all passed).
- The terminal target on a real color TTY has no spelling-parity test.
  Through a pipe it is byte-identical to plain, which is covered.
- No cross-OS run, as in the earlier phases. Plan Phase 7 is the
  validation phase. This phase's OS exposure is mostly test code. The
  production changes are layer admission logic, schema keys, and flag
  removal, none of them path or OS specific.
- `render_alias_parity.rs` is 852 lines. That is over the advisory 500-line
  `just lint-files` cap, which `just lint` does not enforce.
