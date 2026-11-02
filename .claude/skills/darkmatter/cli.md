# `md` File Arguments and CLI Conventions

Detail behind the "CLI orientation" section of [SKILL.md](SKILL.md).

`args::RenderOptions` defines the arguments shared by top-level default
rendering (`Cli::render`, flattened) and `Command::Render`. After the help and
`--save` shortcuts, `Cli::command_or_render` supplies the default command and
combines top-level and explicit-render features once. Both forms enter the
same `run_subcommand` branch and `run_render(&RenderOptions, &Cli, &MdRequest)`;
render policy is resolved there. Keep render argument definitions and
preparation shared, including `--indent`.

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
and exits `64` for assignments or `--schema` with any YAML input; the
baseline is loaded only when some input is not YAML. The YAML route
(`validate.rs::validate_yaml`) loads with `LoadedYamlDocument::load` (failure
→ `FileOutcome::YamlLoadError`, exit 3, JSON `error: "yaml_parse"` with
`line`/`column`) and validates with `validate_loaded_yaml_document` (error →
exit 2; problems → 1; `NoSchema` → the Markdown no-schema record). A
`KindCheck` problem's JSON `kind` is `kind_check`. Wiring tests:
`cli/tests/l1/schema_validate_yaml.rs`; the matrices stay in-process in lib.
A YAML file with Markdown `---` fences is two documents and exits 3; a
leading `---` alone is a document marker. A root-`$schema` file with no
`kind` (an informal schema file) is judged as data against its own
`$schema`, so such a file can fail here, and DMLS reports the same failure.

A custom baseline path (`schema validate --schema`, `BASELINE_SCHEMA`, and
`compose --baseline-schema`) is a launch-context file argument too, resolved
with `resolve_file_path`; it never falls back when it fails. Baseline
selection order is in [schema.md](schema.md#runtime-baseline).

Rendering flags are presentation policy. `CliStyleClaims` captures explicitly
supplied global CLI style claims so command handlers can merge them with
frontmatter without mistaking defaults for user intent. Keep parsing in
`cli/src/args`, command execution in `cli/src/commands`, and output in shared
renderable components.

`--feature` (`args::Feature`, top level and on `render`, lists combined) is
lowered in one place, `render::apply_features`, for both HTML and terminal
output (`render::terminal_page` mirrors `html_page`, so tests drive the real
lowering in-process). Names: `embed-mermaid-font`, the three shared Mermaid
fallback names (mapped onto `biscuit_visualized::components::MermaidFallbackFeature`
by `Feature::mermaid_fallback()` and folded into
`DarkmatterPage::with_mermaid_fallback_policy`; inert without `--mermaid`), and
`no-mermaid-rendering` (lowers to `MermaidMode::Text`, beating `--mermaid`
and `embed-mermaid-font`). `--mermaid` itself selects `MermaidMode::Automatic`.
Convention: the `cli` skill; user docs: `darkmatter/docs/cli/render.md#features`.
