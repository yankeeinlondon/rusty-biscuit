# `md` File Arguments and CLI Conventions

Detail behind the "CLI orientation" section of [SKILL.md](SKILL.md).

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

Rendering flags are presentation policy. `CliStyleClaims` captures explicitly
supplied global CLI style claims so command handlers can merge them with
frontmatter without mistaking defaults for user intent. Keep parsing in
`cli/src/args`, command execution in `cli/src/commands`, and output in shared
renderable components.
