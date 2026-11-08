# `md` File Arguments and CLI Conventions

Detail behind the "CLI orientation" section of [SKILL.md](SKILL.md).

`args::RenderOptions` defines the arguments shared by top-level default
rendering (`Cli::render`, flattened) and `Command::Render`. Its fields have no
clap defaults, so `RenderOptions::has_explicit_arguments` sees every supplied
one; `commands::validate_subcommand_usage` rejects any root render argument
(setters included) before a subcommand with exit 2, and
`args::render::tests` enumerates the clap definition so a new field without a
root representative fails. Shared controls (`--width`, `-v`, `--debug`,
`--magic-root`) are not render arguments. After the help, version, and
completion actions, `Cli::command_or_render` returns the explicit command or a
clone of the root options; nothing is combined (root `--save` and
`--list-themes` are gone; `md list-themes` is a command leaf). Both forms
enter `commands::render::dispatch_render(options, cli, request,
&OutputEnvironment)`, which builds `args::PreparedRenderOptions` (classified
input/setters, `--style` references, and resolved policy; `cli` supplies
only the shared `--width` and `-v`), resolves the output once, handles no input (`environment.stdin_is_terminal`:
root help on stderr or one data failure value, exit 1), and calls
`run_render(&PreparedRenderOptions, &ResolvedOutput, &MdRequest)`, which never
sees `Cli`. Keep render argument definitions and preparation shared,
including `--indent`.

`Render::run` prepares the style before cleanup and before any output:
`render_style::prepare_render_style` merges `document_style_layer` (the
document's `style:`, positions shifted to document lines) under the CLI
layers. A source target (Markdown, data) never reads the document's style,
so only its denied warnings stop it, while CLI layers are checked on every
target. Trigger findings from the schema selection go to the sink or stderr;
`--deny style` also blocks body inline-rule styles, and `--deny <code>` a
single warning code. Page builders (`render::terminal_page`,
`artifact::html_page`, `render::apply_presentation`) take a
`PagePresentation { style: &PreparedStyle, features, mermaid, legacy }` and
`PageInputs`, never `Cli`; the terminal is `Terminal::from_output_options`.
`commands/render.rs::document_value` (clean, set) uses `render::document_style`,
the document layer alone with no CLI override.

`cli/src/render_style.rs` turns those `--style` references and setters into
`darkmatter::style::StyleLayer`s and prints nothing:
`prepare_cli_style_inputs` selects the Markdown document's schema once
(`commands/schema/validate.rs::select_document_schema`, the same passive
selection completion's `effective_document_schema` uses), reads each file
through the library's `parse_local_document` (the `load_data` readers), and
`CliStyleInputs::prepare` wraps a `StylePreparationError` in a
`ClassifiedFailure` whose records keep the `dm.schema.*`/`dm.style.*` codes.
A file record's `source` is the style file; a setter's is null. Those
failures are `ClassifiedFailure::located` (`located_from` keeps the wrapped
chain, so a `DocumentArgumentError` still renders): `ReportSink::failure` and
`CommandRun::finish` skip `with_document_fallback` for them, so the run's
document is never stamped onto a style-file or setter record. Tests:
`cli/tests/l1/render_style_files.rs`, `cli/tests/l1/render_dispatch.rs`.

Render completion: `args::path_completion::complete_render_word` (the `ARGS`
completer) asks `path_candidates` with `PathCommand::Render`, which offers
only `style` and its descendants as setters (`style.` → `style.page.`, values
after `=`); a word after `--` completes files only, and `--style` completes
JSON/JSON5/YAML/YML/TOML files (`complete_style_files`). It opens only the
input document, never a `--style` file.

Render positionals are two clap fields: `args` (before `--`) and
`literal_args` (`last = true`, only after `--`), so the terminator survives
parsing without scanning argv. `args::classify_render_args` is the one
classifier: a Unicode word whose text before its first `=` is `style` or
starts with `style.` is a `RawStyleSetter` (split once); every other word,
and every `literal_args` word, is the input, kept as a native `OsString`.
Root help uses its own `help_template` and hides the flattened render fields
with root-only `mut_arg`; never hide them in `RenderOptions`, since
`md render --help` lists them all. `--code-block` exists only on
`schema about`.

Every source-file argument opens through `cli/src/io::open_argument` (or
`resolve_file_path` for a context of its own): parse with `FileReference`
first, so a bare `@`/`&`/`^` or `!x.md` is `InvalidReference` even when that
literal file exists (`./@` names it), then resolve in the launch context, and
derive the document context from the returned opening reference
(`OpenedArgument::document_context`). Never join an argument onto the launch
directory, canonicalize or probe the raw text, or fall back from a parse error
to a plain path. Route-specific rules: `code-block` without a flag reads a
file only for one line of reference syntax that resolves; `hash` takes the
first candidate that exists as a file or directory
(`darkmatter::markdown::fs::resolve_entry_in_context`, built on the
resolver's own `resolve_detailed` walk, so an earlier file beats a later
directory and an earlier `Io` probe fails; `find_files` and DMLS's
later-buffer fallback follow the same rule); `edit` creates a miss at its
first candidate; `schema triggers` fails a document outside every repository
with `DocumentOutsideRepository` (`missing-context`). A reader that interprets
references *inside* the opened document passes that document context, never
the launch context: `schema detect`, `schema validate`, and `schema triggers`
canonicalize the opened path (`canonicalize_simplified`) and call
`request.document_context(Some(opened.reference()), &canonical)`, and detection
uses `detect_schema_with_contexts` (library detection is passive and turns a
context that does not admit the document into `string`). User contract:
`darkmatter/docs/cli/index.md#every-file-argument-uses-this-grammar`.

`schema validate` routes an argument as a whole YAML document when its
**resolved** path ends in `.yaml`/`.yml` (ASCII case-insensitive; an
unopened argument by its spelling). A preflight opens every argument first
and raises a usage error (exit 2) for assignments or `--schema` with any YAML input; the
baseline is loaded only when some input is not YAML. The YAML route
(`validate.rs::validate_yaml`) loads with `LoadedYamlDocument::load` (failure
→ `FileOutcome::YamlLoadError`, record `error: "yaml_parse"` with
`line`/`column`) and validates with `validate_loaded_yaml_document` (error →
record `error: "schema"`; `NoSchema` → the Markdown no-schema record); every
failure exits 1, and the record's `error` tells them apart. A `KindCheck`
problem's `kind` is `kind_check`. Wiring tests:
`cli/tests/l1/schema_validate_yaml.rs`; the matrices stay in-process in lib.
A YAML file with Markdown `---` fences is two documents and fails to load; a
leading `---` alone is a document marker. A root-`$schema` file with no
`kind` (an informal schema file) is judged as data against its own
`$schema`, so such a file can fail here, and DMLS reports the same failure.

A custom baseline path (`schema validate --schema`, `BASELINE_SCHEMA`, and
`compose --baseline-schema`) is a launch-context file argument too, resolved
with `resolve_file_path`; it never falls back when it fails. Baseline
selection order is in [schema.md](schema.md#runtime-baseline).

Presentation is style, never a flag: the old root presentation flags and
`md render --max-width` are removed (clap's unknown-argument error, never a
`REMOVED_FLAGS` entry), and `render::apply_presentation` applies only the
prepared effective style (`darkmatter::style::apply_prepared_style`); the
library has no claim or override-mask API. `args::cli::tests::removed_render_flags_are_unknown_arguments`
lists every removed spelling, and
`the_root_declares_only_shared_controls_besides_render_options` makes a new
root argument a deliberate decision. Keep parsing in
`cli/src/args`, command execution in `cli/src/commands`, and output in shared
renderable components.

`--feature` (`args::Feature`, a render option: root form or `render`, never both) is
lowered in one place, `render::apply_features`, for both HTML and terminal
output (`render::terminal_page` mirrors `html_page`, so tests drive the real
lowering in-process). Names: `embed-mermaid-font`, the three shared Mermaid
fallback names (mapped onto `biscuit_visualized::components::MermaidFallbackFeature`
by `Feature::mermaid_fallback()` and folded into
`DarkmatterPage::with_mermaid_fallback_policy`; inert without `--mermaid`), and
`no-mermaid-rendering` (lowers to `MermaidMode::Text`, beating `--mermaid`
and `embed-mermaid-font`). `--mermaid` itself selects `MermaidMode::Automatic`.
Convention: the `cli` skill; user docs: `darkmatter/docs/cli/render.md#features`.

## Property paths and setters

`md get`, `md set`, `md rm`, and `md compose` setters name a value with
`simplified_schema::PropertyPath`, which is the only parser of that grammar
(`only_property_path_splits_a_path_in_the_commands` and its library twin scan for
another). User docs: `docs/cli/index.md#property-paths` and `#command-line-values`.
`md schema validate`'s `key=value` assignments are a separate, older grammar
(`commands/schema/assignment.rs`), not `PropertyPath`.

- A malformed path is a `clap` usage error (exit 2) before the document is read;
  everything else goes through `CommandRun::finish` (exit 1).
- `compose::setters::classify_token` decides setter or input path, for execution
  and completion alike. A token whose key misses the setter grammar is a path, so
  `a..b=1` fails as a second input rather than as a bad setter.
- Values: `SetterValue::parse`, then `bind` through
  `schemas::completion::declared_shape`, which reads the baseline the request
  selected. `md set` and setters share both.
- Setters stay a raw ordered plan (`ComposeOptions::with_setters`, `SetterPlan`),
  bound per document by `apply_setter_plan` right after schemas are prepared, so
  they reach transcluded children and a child's own schema reads them.

## Shell completion of property paths

`COMPLETE=<shell> md -- <words>` is the entry point; `CompleteEnv` answers and
exits before `main` captures its request. `cli/src/completion_shells.rs` owns the
shell side and `cli/src/args/path_completion.rs` the value completers.

- `recover_words()` is the one reader of the line. A completer receives only the
  current word; the engine and the completers both see the recovered words.
- Bash splits at `=` and `:` from version 4 on. `x=<Tab>` has `=` as the current
  word with an empty `$2`; bash 3.2 keeps `x=ce` whole. A reply replaces only the
  last piece, so `bash_reply` strips through the typed `=` or `:` and escapes it.
- `CompleteEnv` removes `COMPLETE` before completers run, so the adapter records
  the word form in a static rather than reading the variable.
- The bash and zsh scripts are the built-in ones with text edits, which
  `every_registration_edit_applies` pins against a `clap_complete` upgrade.
- Elvish and PowerShell keep the built-in adapters. Elvish's Tab is modal (typing
  filters its list until Enter accepts), and it needs its editor module, so its
  L1 script test supplies a stand-in namespace. PowerShell's real Tab test runs in
  ConPTY on native Windows, reading the line through a PSReadLine key handler.
  `print_completions`' PowerShell line removes the `md` alias of `mkdir`.
- Passive and silent: every failure means fewer candidates. `completion_overlay`
  drops only the setter composition would refuse, where `SetterPlan::new` fails
  the whole plan. A misplaced test document therefore does not fail loudly: it
  falls back to the baseline, and `CliProcessFixture::write_file` is relative to
  the workspace, not `cwd()`.
