---
name: use-darkmatter
description: How to USE Darkmatter as a caller — running the `md` CLI (render, compose, clean, get/set/rm frontmatter, toc, delta, hash, graph, validate refs, schema validate/detect/about) or calling the `darkmatter` Rust library (Markdown, ComposeRequest, ComposeOptions, RequestSnapshot, compose_with, as_terminal/as_html, fm_get/fm_insert). Also covers authoring Darkmatter documents — `{{{ }}}` expressions, `::file`/`::code` transclusion, `::block when=`, `::map`/`::filter`, `::shell`, `$schema` frontmatter. Use when scripting `md` in a shell or CI, parsing `md --output json`, composing or rendering Markdown from Rust, or writing a document that Darkmatter composes. For changing Darkmatter's own source, load the repo's `darkmatter` skill instead.
---

# Using Darkmatter

Darkmatter is a Markdown system with two parts that callers use:

- the **`md` CLI** (crate `darkmatter-cli`), which renders, composes, tidies,
  and queries Markdown documents;
- the **`darkmatter` library** (crate path `darkmatter/lib` in the
  rusty-biscuit monorepo), which does the same from Rust.

Both run the same **compose pipeline**. It takes a Markdown document with
YAML frontmatter and resolves `{{{ expressions }}}`, conditional blocks,
transclusions (`::file`), shell output (`::shell`), and schema validation.
The output is plain Markdown, which you can then render for a terminal, as
HTML, or as data.

This skill is for **callers**. If you are changing Darkmatter's own source
code inside rusty-biscuit, load the repo's `darkmatter` skill.

## Pick your surface

| You want to… | Read |
|---|---|
| Run `md` by hand or from a script/CI job | [cli.md](cli.md) |
| Call Darkmatter from Rust code | [library.md](library.md) |
| Write a document that composes (expressions, directives, schema) | [authoring.md](authoring.md) |

## Five rules that answer most questions

1. **Compose writes Markdown; render presents it.** `md compose` only writes
   Markdown or data. To see the result styled or as HTML, pipe it:
   `md compose doc.md | md render --output html`.
2. **For a script, use `--output json` and the exit status.** A data output
   writes exactly one JSON value to stdout (including on failure) and nothing
   to stderr. Exit codes: `0` success, `1` the command ran and failed, `2` usage
   error (no JSON printed), `130` interrupted.
3. **Composition has effects; everything else is passive.** Only `md compose`
   runs shell commands, reads data files, and fetches remote content.
   `render`, `get`, `schema validate`, `validate refs`, and tab completion never
   run anything.
4. **Deny-by-default safety.** Network reads are refused unless you pass
   `--allow host=HOST`. Every shell command must be approved: by a
   whitelist file, an interactive prompt, or (in a library) an approval
   handler. With `--output json` nothing is ever prompted, so an unapproved
   command fails the run.
5. **File arguments use one reference grammar:** `./x.md` (relative only), `x.md`
   (relative, then repo root), `&x.md` (repo root; quote it in the shell),
   `^x.md` (package → area → repo root), `@x.md` (magic search roots), `~/x.md`,
   absolute paths, and `https://` (remote). See [cli.md](cli.md#file-references).

## CLI in 30 seconds

```sh
md README.md                              # render for the terminal (= md render README.md)
md compose doc.md                         # composed Markdown to stdout
md compose doc.md name=Ann draft=false    # per-run frontmatter overrides (file unchanged)
md compose doc.md --output json           # {file, outcome, document, diagnostics, summary, ...}
md compose doc.md --deny all              # CI: any warning fails the run
md compose doc.md --shell                 # list the shell commands it would run; runs nothing
md compose doc.md | md render --output html > doc.html
md get doc.md author.name --output plain  # read one frontmatter value
md set doc.md title "New Title" --save    # write one value in place
md schema validate doc.md                 # frontmatter vs. its schema (exit 1 if invalid)
md validate refs doc.md                   # broken links, images, transclusions
md clean doc.md --save                    # reformat without changing meaning
```

## Library in 30 seconds

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
use darkmatter::markdown::output::TerminalOptions;

let md = Markdown::try_from(std::path::Path::new("docs/guide.md"))?;

// A binary snapshots its own process once; a library uses RequestSnapshot::new(dir).
let snapshot = RequestSnapshot::from_process()?;
let request = ComposeRequest::prepare(
    ComposeOptions::new().with_source_file("docs/guide.md"),
    &snapshot,
)?;
let (composed, report) = md.compose_with(&request)?;
println!("{}", composed.as_terminal(TerminalOptions::default())?);
for w in &report.warnings { eprintln!("{:?}: {}", w.code, w.message); }
```

Types live under `darkmatter::markdown::…`. `darkmatter::Markdown` is **not**
re-exported at the crate root, despite what some older doc snippets show.

## Discover instead of guessing

The CLI describes itself. Prefer these over memory:

| Question | Ask `md` |
|---|---|
| Which flags, recoveries, and diagnostic codes exist? | `md compose --help`, `md <cmd> --help` |
| What does the SimplifiedSchema type language contain? | `md schema about` |
| What schema would a set of documents imply? | `md schema detect docs/*.md` |
| Which schema triggers apply to this file? | `md schema triggers doc.md` |
| Which commands/hosts would compose touch? | `md compose doc.md --shell` |
| What keys and values does this document accept? | press Tab after `md compose doc.md style.` (with completions set up) |
| Which theme names can a style use? | `md list-themes` |

## Gotchas

- **The installed `md` can lag the source.** `md` is installed from the repo,
  so when its behavior disagrees with the docs, compare it with a fresh build
  before treating the difference as a bug. To reinstall:
  `just -f darkmatter/justfile install`.
- **`md compose` strips frontmatter** from its output. Add `--fm` to keep it.
- **`md compose` does not wrap prose** unless you pass `--width N`. It ignores
  the terminal width so its output does not depend on where it ran.
- **`md set`/`md clean` print to stdout**. They change the file only with
  `--save`.
- **Setters vs. files.** An argument like `a.b=c.md` is read as a setter. Write
  `./a.b=c.md` to name the file.
- **`&` is a shell operator.** Always quote repo-root references: `'&docs/x.md'`.
- **Relative references cannot climb out of the repository**
  (`failure: invalid-reference`). Use an absolute path or `~`.
- **On Windows PowerShell, `md` is an alias for `mkdir`.** The completion
  setup line in [cli.md](cli.md#shell-completions) removes the alias.
- **Undeclared names are warnings, not errors.** `{{{ colour }}}` with no such
  key renders `""` and adds `dm.expression.undeclared_property`. Write
  `{{{ colour || "none" }}}` when the absence is intended, or pass `--deny all`
  to make the warning fail the run.
- **`::map`/`::filter` sources need a declared type** in `$schema`
  (`people: "{ name: string }[]"`). Without one, composition fails with
  `dm.collection.untyped_*_source`.
- **Effects are not rolled back.** A failed compose prints no document, but
  shell commands and lifecycle actions that already ran stay done. That
  includes `initialize`, which runs before schema validation.
- **Missing and `null` are the same value; `required` asks only whether the
  key was written.** A `default(...)` fills a missing or `null` value, required
  or not, in the composed copy only: the source file and `md set` never gain
  it. See [authoring.md](authoring.md#required-null-and-defaults).

## Canonical documentation

These skill files summarize the docs. The authoritative pages are in the
monorepo under `darkmatter/docs/`. The main checkout is
`/Volumes/coding/personal/rusty-biscuit/darkmatter/docs/`; any worktree has the
same tree.

- `cli/index.md`: every command, the file-reference grammar, property paths,
  and command-line values
- `cli/output-and-width.md`, `cli/data-contracts.md`, `cli/policy.md`: output
  formats, JSON shapes, and `--allow`/`--deny`/`--disable`
- `cli/<command>.md`: a reference page for each command
- `topics/compose-requests.md`, `structs/Markdown.md`, `lib/README.md`: the
  library API
- `topics/darkmatter-expressions.md`: the expression grammar and every
  function
- `inline/*.md`: each directive; `topics/schemas/`: schemas
