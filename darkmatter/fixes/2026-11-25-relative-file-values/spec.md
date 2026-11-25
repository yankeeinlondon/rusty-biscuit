---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
reviewed: false
reviewed-by: codex/gpt-6.2-sol
reviewed-on: "2026-11-25"
review-iterations: 0
status: draft-spec
needs-rulings: false
---

# File values are relative by default

Darkmatter binds a caller-supplied local `file` to an absolute path so that a
prompt can refer to the caller's file wherever the prompt lives. Today the
absolute spelling also appears in rendered text. Functions such as `dirname`
and the index functions carry that spelling forward. When a prompt asks an
agent to record a path in a file, the generated text can name a particular
worktree or home directory. Prompts currently work around this by assembling
a path from pieces such as
`{{ parent_dir(spec) }}/{{ basename(spec) }}`, which omits the higher directories
when the target is more deeply nested.

With the prompt launched from the repository root, `spec=feat/x/spec.md`, and
`review-2.md` present beside the spec, the output changes as follows.

| Expression | Today | After this change |
| --- | --- | --- |
| `{{ spec }}` | `/home/me/repo/feat/x/spec.md` | `feat/x/spec.md` |
| `{{ dirname(spec) }}` | `/home/me/repo/feat/x` | `feat/x` |
| `{{ find_last_index(dirname(spec) + '/review-1.md') }}` | `/home/me/repo/feat/x/review-2.md` | `feat/x/review-2.md` |
| `{{ absolute_path(spec) }}` | named `absolute` | `/home/me/repo/feat/x/spec.md` |

## Decisions

The author ruled on every decision below on 2026-11-25.

### Use portable spelling for local file and directory values

Rendering a bound local `file`, a local `file` selected by the schema from a
document value or default, or a path-producing function result uses the same
output rule. Apply the rule to file entries in arrays and nested objects as
well. Schema-selected values follow the existing union rules, so an ambiguous
`file | string` input stays as written. Ordinary strings, nulls, and HTTP(S)
URLs keep their current behavior. A recursive search that resolves to a local
file uses the spelling of its selected target.

- For a target in the request's repository, emit an implicit relative path from that
  repository's root, such as `feat/x/spec.md`. This holds for a file beside
  the composing document too: the author ruled on 2026-11-25 that a value is
  never `./review-2.md`, because the agent or shell that receives it is not in
  that document's directory. A directory function returning
  the repository root emits `.`. The request's repository is the common base
  for this output even when the prompt lives outside it. Where the request has
  no repository, use the composing source's repository if one was established
  during source admission.
- For a target outside that output root, or when there is no repository root,
  try a declared portable environment root, then `~/`, then the absolute path.
  A target known to belong to a second checkout follows the same fallback
  because a path relative to that checkout could point elsewhere for the
  reader. Use forward slashes where they preserve the target's spelling.
  Preserve a faithful absolute spelling as the fallback if a portable
  spelling would be ambiguous. Treat emitted environment-root text as data
  so a spelling such as `{{NOTES}}/spec.md` does not trigger interpolation
  again in the same composition.
- Apply this order to the identified target. The biscuit-file
  [`PortablePath`](biscuit-file/lib/src/file_reference/portable/evaluate.rs)
  API generates file-reference spellings. Its shared default strategy serves
  links as well and includes nearby routes and authored forms. Use an explicit
  strategy for expression output, with a path-only operation where needed to
  format an already selected target without probing it. Output never climbs
  via `../` or `../../`.

### Expressions see the portable spelling

The portable spelling is the value an expression works with, not a form
applied when the value is printed. With `spec=feat/x/spec.md`:

| Expression | Result |
| --- | --- |
| `{{ dirname(spec) + '/review-1.md' }}` | `feat/x/review-1.md` |
| `{{ contains(spec, 'fixes') }}` | tests `feat/x/spec.md`, never the worktree's path |
| `{{ starts_with(spec, '/home/me/repo/') }}` | `false` |

A path an author builds from another path is ordinary text. It names a file
by being looked up again under the rule below, so Darkmatter does not track
which file a piece of text came from. Prepared frontmatter and state returned
to a library caller carry the portable spelling too.

The alternative, keeping the absolute path inside expressions and converting
at output, was rejected: a test such as `contains(spec, 'fixes')` would read
the worktree's path, and text built from a path would print absolute unless
its property were declared `file`. Neither is something an author would
expect.

### An implicit relative path is looked up from roots, not from the document

A path with nothing in front of it (`feat/x/review-1.md`: no `./`, `/`, `~`,
or sigil) is the implicit relative form. It is looked up in this order, and
the first match wins:

1. the launch directory
2. the package root
3. the package area root
4. the repository root

Outside a repository only the launch directory applies. The order is most
local first, so `README.md` typed in `darkmatter/docs/` is the one in that
folder, which is also what shell completion offers.

All four bases come from the run: the launch directory, then its package,
package area, and repository. They do
not come from the document that wrote the path, so a prompt behaves the same
wherever it is stored. A prompt at `~/.claudine/prompts/review.md`, run from
`/home/me/repo` with `spec=feat/x/spec.md`, builds `feat/x/review-2.md` and
finds it in `/home/me/repo`. The `@` form already takes its roots from the
run.

Sibling package areas are never bases. Run from `darkmatter/` with
`claudine compose prompts/review.md spec=features/foobar.md`, both arguments
are looked up under `darkmatter/` (the launch directory and the package area)
and then at the repository root; a
`claudine/features/foobar.md` is not considered. The agent starts at the
repository root, so the value of `spec` is `darkmatter/features/foobar.md`.

A document in a second repository therefore cannot use the implicit form to
reach a file in its own repository; it writes `./` or `&`.

The folder of the document that wrote the path is no longer a base. A file
beside the document is written `./plan.md`, which is the only form that is
relative to the document. Today the implicit form tries the document's folder
first and then the document's repository root, and tries neither the package,
the package area, nor the launch directory.

This is what makes the portable spelling safe to compute with. A file beside
a prompt can no longer be chosen in place of the repository file that a value
names, because the prompt's folder is not searched.

The explicit forms are unchanged: `./` and `../` are relative to the document
that wrote them, and `&`, `^`, `@`, `~`, `{{VAR}}`, and absolute paths keep
their meaning.

When a file exists at the repository-root path and a different file exists at
the same path under the package or package area, the portable spelling of the
repository file is written with the repository anchor (`&/feat/x/spec.md`), so
that the spelling still names it.

### Preserve file selection and access rules

Resolve authored references through the existing file-reference grammar,
using their existing origin. A caller value uses the captured launch or
authoring context, and a schema default uses the context that already applies
to that default. A lazy caller file keeps its first planned target even if
the file is missing. When output needs a target for a document-authored lazy
file, use its source context and the existing path-selection order, including
the first planned target for a missing non-recursive reference. Eager files
must still exist under the existing schema rules. Keep validation-only
operations passive. Formatting an already known target preserves it without
another lookup, repository discovery, or probe per item in a file listing.
File reads, grants, read-set recording, and typed errors continue to use the
selected target and its authored origin.

A `file` value passed whole to a child through a sequence task, lifecycle
directive, or transclusion arrives as the same portable spelling and is looked
up by the same rule, so it names the same file in the child. Each child
starts with its own frontmatter and receives parent state under the shared
inheritance rule, with `isolated` as the schema-controlled exception. An
inline sequence and the same sequence in a referenced file behave alike.

### Cover path-producing functions and link labels

In Darkmatter's [expression catalog](darkmatter/schemas/partials/functions.yaml),
apply the output rule to `dirname`, `join`, `find_files`, `find_first_index`,
`find_last_index`, `increment_file_index`, and `decrement_file_index`.
`dirname` and `join` produce paths even though their signatures return text.
Functions that extract pieces, such as `basename`, `parent_dir` (the immediate
directory name), and `ext`, keep those meanings. `find_files` keeps its
match order, deduplication, warnings, and empty result. Index functions keep
their index and missing-file rules, including paths to siblings that have
yet to be created.

`link` and `code_link` produce Markdown. A one-argument call uses the portable
path as its label, with inline-code formatting for `code_link`. A two-argument
call uses the supplied label and keeps existing Markdown escaping and URL
handling. Local links carry the absolute destination into the existing link
pipeline, which can resolve links for terminal output and normalize them for
Markdown. For example, a Markdown link to a sibling can still be written as
`./plan.md`. Keep ordinary link normalization rules independent of file-value
output.

### Rename the explicit path functions

Rename `absolute(file)` to `absolute_path(file)`. It requests a native
absolute path, including when used for output. Preserve its existing
resolution behavior, including the first candidate for a missing
non-recursive target and a typed error for a recursive miss or an unsupported
URL. Use it at the final handoff when a shell command, subprocess argument, or
other consumer requires a native path. A command may run from the launch
directory or an explicitly configured working directory. An implicit relative path only
works for such a consumer when the base is known to be the output root, and
`~/` and `{{VAR}}` are Darkmatter reference forms that a process argument may
receive literally.

Rename `relative(file)` to `relative_path(file)`. It explicitly requests the
same spelling as default portable output, with the same fallbacks. Keep its
existing matching and error behavior for unresolved inputs. Both renamed
functions keep their null behavior. Apply `absolute_path` after the final
path transformation when absolute output is required, for example
`absolute_path(dirname(spec))`.

Remove the old names from the catalog and runtime together, with no aliases.
Composition and DMLS report `dm.expression.unknown_function` on the old names.
Show the new names, signatures, and examples in completion and hover.

### This narrows an earlier ruling

Claudine's file-referencing page records a ruling that, during composition,
the working directory is the folder of the file being composed, with an
exception: a file passed in by the caller is resolved from the directory the
caller ran the command in.

That ruling still holds for the explicit forms (`./`, `../`) and for the
exception. It no longer holds for the implicit relative form inside a
document, which until now tried the document's folder and then the
document's repository root. The page is rewritten to say so.

## Nomenclature

The three packages use different words for the same things. These are the
terms from now on; the others are removed from docs, skills, diagnostics, and
identifiers.

| Term | Meaning | Replaces |
| --- | --- | --- |
| explicit relative path | `./x.md`, `../x.md`: relative to the document that wrote it | |
| implicit relative path | `feat/x.md`: nothing in front, looked up from the bases | "bare path", "bare reference" |
| base | a directory an implicit relative path is joined to | "root" and "candidate" where they mean this |
| launch directory | the directory the command was run in, or the one a library caller names | "request directory", "invoking cwd", "caller's CWD" |
| portable path | the text that names a file as a value: the implicit relative path from the repository root when the file is in the repository, otherwise `~/…` or absolute | "display path", "presentation path", "repository-relative path" where it means the value |

`biscuit_file::to_portable_string` only turns separators into forward
slashes, which is not what "portable path" means here. It is renamed so that
its name says what it does.

## One implementation

Each of these exists once, in biscuit-file, and every caller uses it.

- **The bases of the implicit relative form.** Resolution, completion's
  search, and the interactive chooser read the same ordered list.
- **The portable path of a file.** Today at least a dozen routines turn a path
  into text by removing a repository root, each with its own fallback: the
  `relative` function, `link` labels, completion candidates
  (`CANDIDATE_STRATEGY`), the chooser's `path_label`, the autocomplete
  window's `path_label`, DMLS's `display_path` and two `relativize` helpers,
  the reference report's `relative_display`, and Claudine's doctor report.
  The implementation lists every such routine in Darkmatter, DMLS, and
  Claudine, replaces each with the shared one, and records any it keeps with
  the reason.

Link normalization keeps its own preference order, because a link is read
from its document and a value is not.

## Documentation

The grammar is described in three places that disagree about the implicit
relative form: biscuit-file's page says the working directory and then the
repository root, Darkmatter's says the document's folder and then the
repository root, and Claudine's says the current working directory and then
spends a section on whose directory that is.

- `biscuit-file/docs/topics/file-references.md` is the one description of the
  forms, the bases, and the portable path, and it holds the table of terms
  above.
- Darkmatter's and Claudine's file-referencing pages state only what each
  adds (how a value is bound, what the caller may pass, completion and the
  chooser) and point to that page for the rest. They do not restate the
  grammar.
- Each page gives a worked example for the cases an author meets: a neighbor
  file, a file elsewhere in the repository, a file passed in from a package
  area, a prompt kept outside the repository, and a path handed to a shell
  command.
- The caller skills and repo skills for the three packages are updated from
  those pages in the same change.

## Scope

- Darkmatter owns file values, path functions, link labels, and the lookup of
  the implicit relative form (with biscuit-file, which defines the reference
  grammar). The file-reference documentation in both packages states the new
  bases.
- Thirty-six `::file` and `::code` directives in seventeen Markdown files use
  the implicit form today, none of them under `prompts/`. Each one that names
  a neighbor becomes `./name.md`; test fixtures that rely on the document's
  folder are updated the same way.
- Update live expression callers and examples, including prompts, the
  expression catalog and fixtures, DMLS, and Darkmatter and Claudine
  documentation and skills. Replace workarounds such as
  `parent_dir(spec) + '/' + basename(spec)` when the goal is portable prose.
  Audit uses that feed lifecycle target fields, shell commands, or other
  process arguments so they still reach the same file.
- Verify `md compose` and Claudine's direct and sequence composition paths.
  Share Darkmatter's output rule and preserve the file's origin through the
  existing handoff. Update host integration wherever the chosen output
  boundary requires it.
- Claudine's completion of file values keeps its own list of spellings
  (`CANDIDATE_STRATEGY` in the schema-completion candidates) and offers a
  file in a sibling package area as `../claudine/…`. It uses the value
  spelling instead, so a completed candidate is spelled exactly as the value
  would be recorded: `claudine/features/x/spec.md`. Completion and its
  chooser may still list files anywhere under the repository root, because
  the root is a base; a sibling is reached only by its path from the root.
- The interactive chooser that opens for an unresolved `key=value` file shares
  completion's search but has a third spelling routine for its labels
  (`path_label` in the schema-interactive command) and records the chosen file
  as an absolute path. Its labels and its recorded value use the value
  spelling too, leaving one routine for all three.
- Existing feature and fix documents that store absolute paths are historical
  records and retain their original text.

## Verification

- Bind the same file with an implicit relative argument from the repository root,
  an absolute argument, and a home-relative argument. Check identical output
  for `spec`, `dirname`, and the index functions. Repeat from a subdirectory
  using an argument that names the same file from that launch directory.
- Cover a document-authored file, a schema default, a file inside a list or
  object, a plain string, and an ambiguous `file | string` value. Verify that
  a lazy missing target stays usable for a future write and that an eager
  missing target still produces its existing error.
- Use a file beside the prompt, a same-name local shadow, and a prompt outside
  the request's repository. Check both displayed text and the target used by
  a subsequent read or write. The file beside the prompt displays as its implicit
  repository-root path. Verify inheritance across transclusion,
  lifecycle forwarding, and equivalent inline and referenced sequences,
  including an `isolated` property.
- Check the environment-root, home, and absolute fallbacks, including a
  target in a second checkout and a path reached from a deeply nested prompt.
  Cover native Windows separators in explicit output and forward slashes in
  portable output.
- Check that `find_files` changes spellings while preserving match order and
  skip warnings. Check that its results still open the listed files.
- Check the default and explicit labels for both link functions, including a
  label with Markdown punctuation. Verify the absolute local target when the
  terminal renders a hyperlink and the existing sibling-relative Markdown
  link behavior. URL handling stays the same.
- Verify that a root-based shell example can use the rendered portable path, and
  that a command launched in a subdirectory can use `absolute_path` for the
  same target. Check the new function names in composition and editor help
  and the registered unknown-function error for both retired names.
- Check that `contains`, `starts_with`, and concatenation work on the portable
  spelling, and that exported frontmatter carries it.
- For the implicit form, check each base in order with a file that exists at
  only that base, and check that a file beside the document is not found
  without `./`. Check that a repository file shadowed under the package is
  spelled with `&/` and still read.
- From a package area, pass a prompt and a `spec` that each exist in that
  area and in a sibling area. Check that the area's own files are chosen, that
  a file existing only at the repository root is still found, and that the
  value of `spec` is the repository-root path.
- From a package area, complete a `spec` value whose only match is in a
  sibling area. Check that the candidate is the repository-root path, that
  accepting it resolves, and that the area-relative spelling of that sibling
  file fails with `no-match`.
- Choose a file in the interactive chooser and check that its label, the
  completion candidate for the same file, and the value the prompt receives
  are the same text.
- Run a prompt stored outside the repository that builds a sibling path from
  its input, reads it with `frontmatter`, and writes it with a lifecycle step.
