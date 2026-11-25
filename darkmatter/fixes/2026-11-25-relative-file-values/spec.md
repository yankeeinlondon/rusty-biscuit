# File values are relative by default

A `file` value that reaches a document from outside it is turned into an
absolute path when it is bound, and every path function keeps the form it is
given. One relative argument therefore becomes absolute everywhere downstream.

Composing a prompt with `spec=feat/x/spec.md` from the repository root gives:

| Expression | Today |
| --- | --- |
| `{{ spec }}` | `/home/me/repo/feat/x/spec.md` |
| `{{ dirname(spec) }}` | `/home/me/repo/feat/x` |
| `{{ find_last_index(dirname(spec) + '/review-1.md') }}` | `/home/me/repo/feat/x/review-2.md` |
| `{{ relative(spec) }}` | `feat/x/spec.md` |

A path written in the document itself stays as written:
`{{ dirname("feat/x/spec.md") }}` is `feat/x`. Only the bound input differs.

The absolute form then reaches committed files whenever a prompt tells an
agent to record `{{ spec }}`. Forty-three feature and fix documents in this
repository carry an absolute path in a frontmatter property, naming a worktree
that has since been removed or a home directory on another machine. Prompts
work around it by spelling paths from pieces
(`{{ parent_dir(spec) }}/{{ basename(spec) }}`).

## Decisions

Ruled by the author on 2026-11-25.

### 1. A file value renders as its portable path

Every `file` value an expression produces is the portable path of its target:
inside a repository, the bare path from the repository root
(`feat/x/spec.md`). This holds for a bound input, a schema default, and the
result of every function that returns a file or a directory, whatever form the
input had. The sentence "an absolute input returns an absolute path" is
removed from each function it appears in, and `find_files` returns relative
paths.

Outside a repository the existing preference order decides
(`biscuit_file::PortabilityPreference::DEFAULT_STRATEGY`): a declared
environment root, then `~/`, and the absolute path only as the last resort.

Authors do not call a function to get this. A document that needs a
relative path writes `{{ spec }}`.

### 2. `link(file)` displays the relative path and targets the absolute one

An OSC 8 hyperlink works only with an absolute target, so `link(file)` keeps
one. The text it displays is the portable path. When the second argument is
given, the display is exactly that argument.

The implementation must check whether any other function hands a path to
something that requires the absolute form (a terminal, a URL, a process
started in another directory) and list each one it finds here. `link` is the
only one known.

### 3. `absolute_path(file)` and `relative_path(file)`

`absolute(file)` is renamed `absolute_path(file)`. It is the one way to ask
for the absolute form.

`relative(file)` is renamed `relative_path(file)`. Its description states what
the result is relative to (the repository root inside a repository), replacing
"a best-effort relative path from the document's base directory".

There are no aliases: the old names stop resolving, and DMLS reports them as
unknown functions. Eighteen calls in five files under `prompts/`, the package
`docs/` trees, and `.claude/skills/` are updated in the same change.

### 4. No route climbs two levels

A portable path never takes the form `../../x.md`. The default preference
order already excludes it (`ParentDir` is not in `DEFAULT_STRATEGY`) and
already tries the bare repository-root form before any `./` or `../` route.
The change adds a test that pins both facts for file values, so a later edit
to the order cannot reintroduce the form.

### 5. Values come from the repository root; `./file.md` is for links

A path an expression produces is handed to an agent or a shell that is not in
the composing document's directory, so it is always the bare repository-root
path, even when the file sits beside that document. A review prompt in
`prompts/_reviews/` gives `darkmatter/features/x/review-2.md`, never
`./review-2.md`.

A link written inside a document is read from that document, so a file in the
same directory is linked as `./plan.md`. Link normalization keeps its existing
rules and is not changed here.

## Scope

- Darkmatter: the binding of `file`-typed inputs, the path functions in
  `darkmatter/schemas/partials/functions.yaml` and their implementations,
  `link`, and the two renames.
- DMLS: completion, hover, and diagnostics for the renamed functions.
- Claudine: no change of its own; `claudine compose` shows the same absolute
  input today and takes the fix through Darkmatter.
- Callers: the prompts that spell a path from pieces to avoid the absolute
  form are simplified to `{{ spec }}`.
- Docs: the expression-function reference and the file-reference topic page
  state the default and show `absolute_path` as the exception.

Existing feature and fix documents that carry an absolute path are left as
they are.

## Verification

- With a relative argument, an absolute argument, and a `~/` argument naming
  the same file inside a repository, `{{ spec }}`, `{{ dirname(spec) }}`, and
  each index function give the same repository-relative result.
- The same document composed from the repository root and from a subdirectory
  gives the same paths.
- A document that names a file in its own directory gets the bare
  repository-root path from an expression, not `./file.md`.
- Outside a repository, a file under the home directory renders with `~/`.
- `link(file)` in a terminal that supports OSC 8 emits an absolute target and
  displays the relative path; with a second argument it displays that
  argument.
- `absolute(…)` and `relative(…)` fail composition with an unknown-function
  error, and DMLS marks them.
- A shell step that receives `{{ spec }}` and runs from the repository root
  still finds the file.
