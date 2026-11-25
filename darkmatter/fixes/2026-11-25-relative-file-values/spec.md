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
reviewed: true
reviewed-by: codex/gpt-6.2-sol
reviewed-on: "2026-11-25"
review-iterations: 0
status: human-in-the-loop
needs-rulings: true
required-rulings:
    - Choose the spelling for repository values that need an explicit anchor.
    - Decide what evidence is required before emitting an implicit relative path.
    - Decide which base a missing implicit relative path is planned under.
---

# File values are relative by default

Darkmatter binds a caller-supplied local `file` to an absolute path so a
prompt can find the caller's file wherever the prompt lives. The absolute
path is what the body prints and what expressions and lifecycle state see
(measured with `md compose` and `claudine compose` on 2026-11-25). Functions
such as `dirname` and the index functions carry that path into generated
text. Prompts work around this by joining `parent_dir(spec)` and
`basename(spec)`, which drops higher directories when the file is deeply
nested.

With the prompt launched from the repository root, `spec=feat/x/spec.md`,
and `review-2.md` beside the spec, expressions produce these values.

| Expression | Today | After this change |
| --- | --- | --- |
| `{{ spec }}` | `/home/me/repo/feat/x/spec.md` | `feat/x/spec.md` |
| `{{ dirname(spec) }}` | `/home/me/repo/feat/x` | `feat/x` |
| `{{ find_last_index(dirname(spec) + '/review-1.md') }}` | `/home/me/repo/feat/x/review-2.md` | `feat/x/review-2.md` |
| `{{ contains(spec, 'repo') }}` | `true` | `false` |
| `{{ absolute_path(spec) }}` | named `absolute` | `/home/me/repo/feat/x/spec.md` |

## Decisions

The author set the direction on 2026-11-25. The remaining choices are under
[Open Questions](#open-questions).

### Use one portable spelling in expressions and output

Apply the spelling rule to bound local files, schema-selected local files
(including defaults and members of lists and objects), and the path-producing
functions listed below. Preserve existing union selection, including an
ambiguous `file | string` value that stays as written. Ordinary strings,
nulls, and HTTP(S) values keep their current behavior.

- Prefer an implicit relative path from the launch repository root, such as
  `feat/x/spec.md`, when it can select the same file. A file beside a prompt
  follows the same rule. The launch repository remains the base when the
  prompt is stored elsewhere. The same preference applies to directories.
  Emit `.` for the repository root only when it can denote that root in the
  receiving context, including when followed by a filename. The grammar
  treats `.` and `./name.md` as explicit relative, so they can otherwise
  name the prompt's directory and its files.
- For a target outside the launch repository, try a declared portable
  environment root, then `~/`, then a faithful absolute spelling. Apply
  this fallback when the run starts outside every repository, even if the
  prompt belongs to a checkout. That checkout supplies the prompt's authored
  `&` references, but a repository-relative output would be looked up from
  the launch directory. Apply the same fallback to a target known to belong
  to a second checkout, including a nested checkout identified at source
  admission.
- Emit forward slashes where they preserve the target. On Unix a backslash
  can be part of the filename, so keep a faithful native spelling when
  needed. If no expression string can faithfully name the target, report a
  typed file-reference failure with the original reference, selected target
  where available, and authored location. Reuse a registered code where its
  contract fits. A new code also needs typed details, DMLS validation and
  completion, and a documented lifecycle `failure` example.
- Treat a generated spelling such as `{{NOTES}}/spec.md` as data through both
  frontmatter interpolation passes, repeated preparation, child forwarding,
  and body rendering. A file operation can expand that variable through the
  reference grammar using the captured request environment.

Expressions, prepared frontmatter, and returned composition state see the
same text. Retain the selected target and authored origin for intact file
values where needed for schema checks, access decisions, diagnostics, and
child handoff. Track file selection by occurrence so a plain string with the
same contents does not acquire file identity. Keep the raw caller value for
fresh preparation against another schema.

The alternative, keeping the absolute path inside expressions and converting
only when a value is printed, was considered and rejected by the author on
2026-11-25: a test such as `contains(spec, 'fixes')` would read the
worktree's path, and text built from a path would print absolute unless its
property were declared `file`. Neither is something an author would expect.

Bind caller inputs before the first frontmatter interpolation. Bind
document-authored values and defaults when their effective schema and value
are available. Preserve the pipeline's evaluation order. An early expression
that depends on a pending schema-selected file follows the existing deferral
rules, including the existing failure when the dependency cannot be resolved.
Once bound, the value is stable across repeated schema preparation.

Concatenation produces ordinary text. If a nearer base contains
`feat/x/review-1.md`, the expression `dirname(spec) + '/review-1.md'` can
name that nearer file when `dirname(spec)` is `feat/x`. Use
`join(dirname(spec), 'review-1.md')` to construct a sibling in the selected
directory and evaluate the spelling of the resulting path independently.
When building raw text for a process, request the native path at the final
handoff.

### Resolve implicit relative paths from the launch context

Composition file references, command file arguments, and their completion
use the following ordered bases for implicit relative paths. The first
match wins.

1. Launch directory
2. Package root at launch, when available
3. Package area root at launch, when available
4. Repository root at launch, when available

Deduplicate equal bases in that order. Outside a repository, use only the
launch directory. Use the captured launch context even when Claudine changes
its process working directory to the repository root. For example, from
`darkmatter/`, `features/foobar.md` can find
`darkmatter/features/foobar.md` or `features/foobar.md` at the repository
root. To reach `claudine/features/foobar.md`, include `claudine/` in the
reference.

An authored `./plan.md` starts beside the composing document. An explicit
relative caller argument starts at its captured authoring directory.
Inside a second checkout, authored `./`, `../`, `&`, and `^` follow that
document's tree. `@` continues to use the launch scope. Home, environment,
vault, URL, and absolute forms keep their existing meanings. After expanding
environment variables, classify the resulting path with the shared grammar.

Use the launch order for direct implicit references, implicit `%` searches,
and implicit composition globs such as those in `find_files` and
`::file-links`. Preserve each consumer's search and result ordering within
and across bases. Use the launch tree boundary for launch-relative lookup
and the source tree boundary for explicit document-relative lookup. Preserve
the existing lexical and symlink containment checks and typed failures.
A miss can advance to a later base. A containment, permission, or I/O error
keeps the resolver's existing stopping rule.

Use this policy for composed file values, file-reading directives and
functions, lifecycle file targets, and their preflight and editor projections.
A `file` default continues to use its existing schema authoring context for
explicit forms. Keep document-link destinations document-relative. Keep
schema imports and schema-root selection, configuration and permission
references, registered bundle members, and other biscuit-file consumers on
their existing lookup contracts. Make launch lookup an explicit choice in the
shared resolver so a change to composition cannot silently change which
permissions file is read.

For DMLS, use the launch context when the host supplies one. In a standalone
editor session, use the configured workspace context for suggestions and
syntax checks. A finding that depends on the actual launch directory should
only be reported as definite when that directory is known.

> **Review note.** The launch order deliberately changes the earlier
> document-directory rule for implicit relative composition references.
> Convert authored neighbors to `./plan.md`, and use the path from a launch
> base to find a project input. Claudine's current design page asks for
> explicit author approval before it is edited. Confirm the agreed wording
> when updating that page.

### Preserve target selection and access

Select or plan local targets through biscuit-file's file-reference resolver.
An eager file must exist and satisfy its schema constraints. A caller's
lazy local file keeps the first planned target without a probe, even if
the file is missing and a later base has a file with the same name. Apply
that planning rule to document-authored and defaulted lazy files when
binding them. Preserve the existing refusal for recursive lazy inputs,
which have no single planned file. A recursive reference in a context that
allows searching uses the matched file or its existing typed miss.

Check schema constraints such as `match(...)` against the selected target in
its original context. A portable spelling does not grant file access. Use
the selected target and authored location for permissions, read-set recording,
and typed errors. Preflight and execution must agree about the target.
Keep passive validation and editor analysis free of new probes and expression
execution.

A file passed intact through a sequence task, lifecycle directive, or
transclusion carries its spelling, selected target, and authored origin,
including when the child belongs to another checkout. Each child starts
with its own frontmatter and receives the parent state under the shared
inheritance rule, subject to schema `isolated`. An inline sequence and the
same sequence in a referenced file behave alike. A lazy target stays bound
after another step creates it.

### Cover path-producing functions and link labels

Apply the spelling rule to `dirname`, `join`, `find_files`,
`find_first_index`, `find_last_index`, `increment_file_index`, and
`decrement_file_index` in Darkmatter's
[expression catalog](darkmatter/schemas/partials/functions.yaml). Darkmatter's
[filesystem expression handlers](darkmatter/lib/src/markdown/compose/expression/functions/fs.rs)
select the target and construct the result. Apply the spelling rule to that
result even when the input was absolute. When a function accepts a directory
operand, resolve it as a directory. A regular-file probe would miss an
existing directory at a later base. Retain the component semantics of
`basename`, `parent_dir`, `ext`, `file_trailing`, and `dir_leading`.

Preserve the match order, deduplication, warnings, and empty result of
`find_files`. Preserve the index functions' selection and missing-target
rules, including the ability to name a future sibling. When an endpoint
function finds no family member and returns the input target, apply the
same spelling rule to that target.

A one-argument `link` or `code_link` uses the portable path as its label,
with inline-code formatting for `code_link`. A two-argument call uses the
supplied label. Keep existing Markdown escaping and URL handling. Send
the absolute local destination through the existing link pipeline so terminal
hyperlinks reach the target and Markdown output can use a sibling link such
as `./plan.md`.

### Rename the explicit path functions

Rename `absolute(file)` to `absolute_path(file)`. It returns a native absolute
path. Keep the existing first-candidate behavior for a missing non-recursive
file and the typed errors for a recursive miss or unsupported URL. An
explicit native result survives assignment or forwarding as a whole value,
including through a `file` field. Use it at the final handoff to a process
that requires a native path. A relative spelling works as a raw process
argument only when the process starts at the corresponding base. A process
argument can receive `~/` or `{{VAR}}` literally.

Rename `relative(file)` to `relative_path(file)`. It explicitly requests
the default portable spelling and keeps the existing match and error behavior
for unresolved file inputs. Both functions keep their null behavior and
accept directories returned by `dirname`, so
`absolute_path(dirname(spec))` works when the receiving process needs an
absolute directory.

Remove the old names from the runtime and catalog together, including any
aliases. Composition and DMLS report `dm.expression.unknown_function` for
the retired names. Completion and hover show the new signatures and examples.

## Terms and shared implementation

The three packages use different words for the same things. These are the
terms for new docs, skills, diagnostics, and identifiers.

| Term | Meaning | Replaces |
| --- | --- | --- |
| explicit relative path | `./x.md`, `../x.md`: starts at its authoring directory | |
| implicit relative path | `feat/x.md`: nothing in front, joined to each eligible base in order | "bare path", "bare reference" |
| base | a directory an implicit relative path is joined to | "root" and "candidate" where they mean this |
| launch directory | where the command was invoked, or the directory the library caller supplies | "request directory", "invoking cwd", "caller's CWD" |
| portable path | the selected reference spelling for a known target; it never climbs with `../` | "display path", "presentation path", "repository-relative path" where it means the value |

Biscuit-file owns the launch base order and the spelling strategy. Extend its
[`PortablePath`](biscuit-file/lib/src/file_reference/portable/evaluate.rs)
API, which generates reference spellings from a target, to support the
expression policy using the captured context. Preserve its existing default
for link and other callers. `biscuit_file::to_portable_string` only turns
separators into forward slashes, which is not what "portable path" means
here; rename it so its name says what it does.

Use the shared ordering in resolution, completion, and the interactive
chooser. Use the shared spelling for file values, expression results,
completion insertions, and chooser submissions.

Today at least a dozen routines turn a path into text by removing a
repository root, each with its own fallback: the `relative` function, `link`
labels, completion candidates (`CANDIDATE_STRATEGY`), the chooser's
`path_label`, the autocomplete window's `path_label`, DMLS's `display_path`
and two `relativize` helpers, the reference report's `relative_display`, and
Claudine's doctor report. List every such routine in Darkmatter, DMLS, and
Claudine, replace each with the shared one, and record any that is kept with
the reason. A report label or editor diagnostic that deliberately uses its
own display base is such a reason. Markdown link normalization keeps the
document's base, because a link is read from its document and a value is not.

## Migration and documentation

- Audit live composition references in shipped documents, prompts, skills,
  and fixtures. Include directives, defaults, read-side expressions, globs,
  and lifecycle targets. As of 2026-11-25, thirty-six `::file` and `::code`
  directives in seventeen Markdown files use the implicit form, none of them
  under `prompts/`. Convert authored neighbors to `./name.md` or an
  explicit relative glob. Check migrated targets from a different launch
  directory.
- Update live expression callers and examples, including
  `prompts/feature.md`. Remove path assembly workarounds whose purpose was
  portable prose. Check lifecycle targets, shell commands, and process
  arguments so they still reach the same files. Preserve historical feature
  and fix documents.
- The grammar is described in three places that disagree about the implicit
  relative form. Make `biscuit-file/docs/topics/file-references.md` the one
  description of the forms, the bases, the portable path, and the terms
  above. Darkmatter's and Claudine's file-reference topics state only what
  each adds (how a value is bound, what a caller may pass, completion and
  the chooser, the policy each entry point uses) and point to that page for
  the rest; they do not restate the grammar. Show a document neighbor, a
  repository file reached from a package area, a prompt outside the
  repository, and a shell handoff. Update affected
  README examples, command and error docs, and repo skills. Publish updated
  caller skills using their package-area recipes.
- Claudine completion and its chooser may list files under any launch base,
  including the repository root. A sibling package area's file uses its
  usable repository-relative spelling or the agreed pinned spelling. The
  completion insertion, chooser label and submission, and prepared value
  agree for a given target. Accepting a candidate must still select the file
  that satisfied the input schema.

## Verification

- Through `md compose` and Claudine direct and sequence composition, bind
  the same file with implicit relative, absolute, and home-relative inputs.
  Launch from the root and from a subdirectory. Check expression values,
  exported frontmatter, path function results, and the target later read or
  written.
- Cover an authored file, a schema default, nested lists and objects, a
  plain string matching the spelling of a bound file, and an ambiguous
  `file | string`. Verify eager missing errors and schema matching. Check
  that a lazy first candidate stays bound when it is missing, a later file
  already exists, and a subsequent step writes to the selected location.
- Check every launch base, including duplicates and missing package
  anchors. Exercise a launch-directory shadow as well as package and
  package-area shadows. Verify preflight, runtime, completion, and chooser
  parity. Cover a source neighbor reached via `./`, an implicit glob and
  recursive reference, a symlink escape, and an unchanged configuration or
  schema import.
- From a package area, pass a prompt and a `spec` that each exist in that
  area and in a sibling area. Check that the area's own files are chosen,
  that a file existing only at the repository root is still found, and that
  the area-relative spelling of a file existing only in the sibling area
  fails with `no-match`.
- Launch with a prompt outside the repository, a child in a second checkout
  including a nested checkout, and from outside every checkout. Confirm
  the selected file across transclusion, lifecycle forwarding, and
  equivalent inline and referenced sequences. Include an `isolated` field
  and a test that the source's authored `&` still follows its own checkout.
- Check directory lookup when a nearer and later base have the same
  directory name, including a root file and the result of `dirname`.
  Verify `join` constructs a sibling of the selected directory, and cover
  new index siblings and the no-family result.
- Check environment-root, home, and absolute fallbacks, including the
  protection of generated variable spelling from a second interpolation.
  Export the portable value, read it back with the same launch context, and
  verify it names the original target.
  Cover Windows native absolute output and a Unix file with a literal
  backslash. Confirm that `find_files` keeps its ordering and skip warnings,
  and that reusable results open the listed files using the approved
  shadow rule.
- Check both link labels, Markdown punctuation, URL behavior, terminal
  hyperlink targets, and sibling-relative Markdown destinations. Test a
  root-based shell handoff and a subdirectory handoff using `absolute_path`.
  Verify the new names and retired-name diagnostic in composition and DMLS.
- Verify the chosen spelling strategy does not make passive validation
  effectful. Use a focused probe-count regression to enforce the approved
  I/O rule for file listings. A separate timing spike is unnecessary for
  the probe-free recommendation.

## Open Questions

### Which spelling should pin a repository target?

From `/work/repo/tools`, suppose `/work/repo/tools/feat/x.md` shadows
`/work/repo/feat/x.md`. The draft calls for using `&/feat/x.md` for the
repository file, but authored `&` follows the source document's repository.
A prompt in `/work/templates` can therefore look for
`/work/templates/feat/x.md` when it consumes or builds on the emitted
string. Holding the target when forwarding an intact value helps, but
concatenation and serialized output can only use the spelling they receive.
The same issue applies when a root directory would otherwise print as `.`.

1. **Use the existing anchored fallbacks (recommended).** When an implicit
   spelling cannot be used, emit `{{WORKSPACE}}/feat/x.md` if a declared
   variable names the workspace, then try `~/`, then an absolute path.
   Apply the same rule to directories and retain the meaning of authored
   `&`. This works across sources with the captured environment and needs
   no new syntax. A file inside the repository can print an absolute path
   if it is shadowed and no portable anchor applies.
2. **Add an explicit launch-repository reference form.** A new prefix could
   express “the checkout this run started in,” including for a directory
   and for a string built by concatenation. The result stays portable even
   when nearer files shadow it. The new form needs a grammar decision,
   completion and diagnostic support, and a documented missing-root error.
   Processes outside Darkmatter would still need `absolute_path`.

The existing fallbacks address the wrong-checkout case without changing
authored references or expanding the language.

### How should formatting establish that a nearer base will not win?

From `/work/repo/tools`, a listing can already know that it found
`/work/repo/feat/x.md` without knowing whether
`/work/repo/tools/feat/x.md` exists. A file probe also cannot establish
whether the corresponding directory would win in a directory lookup. The
draft requires both a reliably reusable spelling and no extra probes per
listing result.

1. **Check the first path component under each nearer base, once per run
   (recommended).** For `darkmatter/features/x/spec.md` launched from
   `darkmatter/`, ask whether `darkmatter/darkmatter` exists. If it does
   not, neither a file nor a directory can shadow anything spelled
   `darkmatter/…`, so every such value prints short, and a listing of 500
   files costs one check. Only when that first component does exist under
   a nearer base is the full path checked, and the pinned form used on a
   real shadow. The checks respect the same read approvals and tree
   boundaries and join the read set. Passive validation makes no checks.
2. **Use only the candidate plan and observations already made.** Emit
   `feat/x.md` when its only candidate is the selected target, or existing
   observations show the target wins. This adds no I/O. From any launch
   below the repository root there is more than one candidate and nothing
   has observed the nearer ones, so nearly every value prints in the pinned
   form (in practice an absolute path) even though no shadow exists.

The reviewer recommended the second option to keep formatting free of I/O.
The author's stated case is a session launched from a package area, where
the second option gives back the absolute paths this fix exists to remove.

### Where is a missing implicit relative path planned?

Today a missing lazy file is planned under the first base without a probe.
With the launch directory first, a prompt launched from `darkmatter/` that
builds `dirname(spec) + '/review-3.md'` gets
`darkmatter/features/x/review-3.md`, which exists under no base. Planned
under the launch directory, a lifecycle step would create
`darkmatter/darkmatter/features/x/review-3.md`.

1. **Plan under the first base where the path's containing directory exists
   (recommended).** `darkmatter/darkmatter/features/x/` does not exist, so
   the launch directory is skipped; `darkmatter/features/x/` exists at the
   repository root, so the file is planned there. `features/new.md` typed
   from `darkmatter/` is still planned in `darkmatter/features/`. When no
   base has the directory, the launch directory is used as today. This
   costs one directory check per base, only on a miss.
2. **Keep the first base.** No checks. A new file's path must be built with
   `join` on a resolved directory or pinned with `absolute_path`; a path
   built with `+` from a launch below the repository root is written to the
   wrong place without an error.
