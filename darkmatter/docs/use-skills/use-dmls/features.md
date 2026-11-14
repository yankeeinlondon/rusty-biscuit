# What DMLS does in the editor

The features come in layers. Every Markdown file gets layer 0. The higher
layers add support for Darkmatter features on top of it.

## Layer 0: plain Markdown

- **Go to definition** jumps from a link or `#anchor` to the file or heading
  it targets.
- **Find references** lists every link to the heading or document under the
  cursor, across the whole workspace (backlinks).
- **Document symbols** give the heading outline. **Workspace symbols** search
  headings across the workspace by fuzzy match.
- **Document links** make every link and path clickable, including `&`, `^`,
  and `@` references. Each resolves to the file `md compose` would read, even
  when that file is outside the folder you opened.
- **Folding** covers frontmatter, sections, code fences, block quotes,
  lists, and tables.
- **Hover** on a link previews its target: the title, the resolved path, the
  heading, and a summary from the target's frontmatter. The summary is the
  first non-empty `excerpt`, `summary`, `description`, or `about` string.
- **Completion** offers link paths, `#anchors`, and code-fence languages.
- **Diagnostics** report broken links, missing anchors, duplicate headings,
  and unknown fence languages (with a suggested match).

## Layer 1: wiki links

`[[note]]`, `[[note#heading]]`, and `[[note|alias]]` get completion, go to
definition, references, hover, and diagnostics:

- unresolved, ambiguous, or empty links
- a missing heading
- unsupported syntax
- portability collisions (two files that a link could mean on another
  system)

Matching is case-sensitive. It accepts a basename, a path suffix, or a path
from the wiki root, and prefers a file in the same folder. Completion never
inserts `.md`. You can turn wiki links off, or change what completion inserts,
under `[wiki]`.

## Layer 2: frontmatter

Inside the `---` YAML block, DMLS works from the document's **effective
schema**. That schema is built from these layers, each later one overriding
the one before: the built-in baseline, `[schema.extensions]` entries whose
globs match the file, matching trigger files, and the document's own
`$schema`. See [schemas.md](schemas.md).

- **Completion** offers keys (required ones marked), `enum`/`literal`
  members, `suggest(...)` candidates, `file(...)` paths, `style.*` keys, and
  booleans.
- **Hover** shows a key's type, constraints, default, and `-> description`.
  Hovering a `$schema` or `file` *value* says where it resolves, or where the
  file would be created.
- **Diagnostics** report YAML parse errors, wrong types, broken constraints,
  unknown keys (a warning; an error in strict mode), missing required keys
  (strict mode only), malformed references, and bad `style.*` keys. Each is
  ranged on the exact key or value, even deep in nested YAML.
- **Navigation:** go to definition on `$schema` and `file(...)` values.
- **Lifecycle stacks** (`initialize`, `composed`, `failed`, `cancelled`,
  `finalize`) are typed. Their actions complete and validate, and
  expressions inside a stack are checked against that event's globals.
- **Claudine documents** are checked against Claudine's trigger schemas when
  those are in the document's schema roots. See
  [schemas.md](schemas.md#claudine).

## Layer 3: the Darkmatter DSL

DMLS checks everything in this layer without running any of it.

- **Directives** (`::file`, `::code`, `::shell`, `::block`, `::map`,
  `::filter`, disclosures, and others):
  - `::` completes the directive name, and each directive's options and
    their values complete too;
  - hover explains the directive and the target it resolves to;
  - blocks fold;
  - diagnostics report an unknown directive, a malformed option, an unclosed
    block, or a malformed disclosure.
- **Transclusion** (`::file`, `::code`, `prologue`/`epilogue`):
  - targets are clickable and support go to definition;
  - "who transcludes this file?" works as find references;
  - broken targets are flagged, and **transclusion cycles** are detected and
    reported with the chain of files.
- **`::toc-linking`** targets get links and broken-path checks. A fallback
  chain `"a.md | b.md | false"` is read the way composition reads it.
- **`{{{ expressions }}}`:**
  - completion offers frontmatter keys, `fm.<key>`, `ctx.*`, and functions;
  - hover shows the value the expression would resolve to;
  - go to definition jumps to the frontmatter key;
  - diagnostics report malformed expressions and unknown functions. An
    undeclared name gets the `dm.expression.undeclared_property` advisory.
- **Function-call checks** agree with `md compose`. A call that can never
  work is an error, for example `min(flag, 5)` with `flag` declared
  `boolean`. A call that might fail is a warning; set
  `[expressions] strict = true` to warn more.
- **`::map` / `::filter`** get source, option, and callback completion, hover
  on the local variables, and the same `dm.collection.*` problems that
  `md compose` reports.
- **Triple-brace literals** (three braces on each side) are recognized as
  inert. Hover shows the text
  they compose to.
- **Shell:**
  - hovering `::shell`, `::shell-block`, or a `$(…)` value shows whether the
    command is approved, denied, or unknown;
  - a command Darkmatter's built-in blacklist refuses is a
    `dm.security.disallowed_command` error;
  - nothing is ever executed.

## Editing

- **Rename a heading** to update every `#slug` and `[[note#heading]]` link
  that points at it. If two headings have the same text, DMLS refuses rather
  than guess.
- **Rename a file** to update the links that point at it. If anything
  conflicts, nothing changes. This needs client support: it works everywhere
  except Neovim.
- **Code actions:**
  - create a missing file or wiki note;
  - add missing required keys;
  - migrate a deprecated `style:` key;
  - close an unclosed `::block`;
  - wrap a malformed `{{{ … }}}` in a triple-brace literal (three braces on
    each side);
  - turn `foo-bar`, which parses as a subtraction, into `fm['foo-bar']`.

  You can turn each category off under `[code_actions.categories]`.
- **Format Document** runs the same cleanup as `md clean` on the unsaved
  buffer: it repairs the frontmatter and applies the `style.markdown` rules.
  When formatting would change the document's meaning, for example around an
  unclosed `::block`, it declines and shows a warning saying where. There is
  no range or on-type formatting.

## Semantic tokens

DMLS classifies Darkmatter syntax so your theme can mute it:

- `{{{ … }}}` spans carry the `interpolation` modifier;
- `::` keywords carry `directive`, and closers such as `::end-block` also
  carry `closer`;
- wiki-link brackets are `macro.wiki`, and their text is `string.wiki`;
- triple-brace literals also carry `inert`.

The editor recipes are in [install.md](install.md). VS Code and Neovim apply
semantic tokens automatically, Zed needs them turned on, and Helix doesn't
support them. `[semantic_tokens] enable = false` turns them off from the
server side.

## Editor differences

All four editors get the full feature set except for these:

| | VS Code | Zed | Neovim | Helix |
|---|:-:|:-:|:-:|:-:|
| File rename rewrites links | ✅ | ✅ | ❌ | ✅ |
| Semantic tokens | ✅ | opt-in | ✅ | ❌ |
| LSP folding | ✅ | ✅ | line-only | tree-sitter instead |
| Client file watching | ✅ | ✅ | limited on Linux (rescan on save) | ✅ |

Hover is plain Markdown text everywhere, with no images. VS Code renders it
with the most formatting.
