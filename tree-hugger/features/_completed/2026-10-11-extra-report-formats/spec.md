---
kind: feature
name: extra-report-formats
date: 2026-10-11
status: draft-spec
reviewed: true
implemented: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 3
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
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
area: tree-hugger
packages:
  - tree-hugger-cli
human_review: false
message_to_agent: |-
  Phase 5 implemented ctags as the documented extended subset after the user's
  explicit instruction to implement this phase. It emits format-2 unsorted
  tags with numeric line addresses and Hug kind/container fields. The
  headless Neovim reader test is `level2_` and runs through `just test-l2`; it
  uses `--headless -u NONE -i NONE` and does not open or focus a window.
  Control characters in a name, recorded container, or scan-relative path
  cause that record to be skipped, a warning to be printed, and exit status 1.

  Phase 4's checklist remains incomplete: shared ListingStream grouping,
  `--tree` layout behavior, semantic Markdown emphasis/headings, and the full
  hostile-input, URL, plain-output and stability coverage still need completion
  before the plan can be considered complete. Phase 5 did not change those
  writers. The author should complete or explicitly defer those items before
  treating Phase 6 validation as final.
depends-on:
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-05-deferred-kind-precision
  - 2026-10-06-symbol-type-relations
completed: true
---

# Spec: TSV, Markdown, Prose and Tags Output for Symbol Reports

## Problem and scope

The `hug` declaration commands (`functions`, `types`, `symbols` and
`classes`) support terminal text, `--format plain|table`, JSON and JSON
Lines. Spreadsheet tools, documentation and pull requests, callers that
render rich text, and editors that read tags files need direct exports.

Extend `--format` with `tsv`, `markdown`, `prose` and `ctags`. The new
formats consume the same selected declarations as existing reports; they do
not extract new symbols or change selector behavior. `imports`, `lint` and
`god-files` retain their existing output. JSON remains the complete structured
representation; these exports are intentionally smaller views.

```bash
hug functions --format tsv > functions.tsv
hug symbols --format markdown --tree > symbols.md
hug --plain classes --format prose > classes.prose
hug symbols --format ctags > tags
```

The tags example assumes the author accepts the contract in
[Open Questions](#open-questions), and writes `tags` at the scan root.
TSV is an escaped interchange format: spreadsheet imports can split its
columns, but literal escapes in cells require decoding to recover tabs and
newlines. No spreadsheet-specific quoting or formula rewriting is added.

## Proposal history and prerequisites

The author split these exports from `2026-10-03-symbol-improvements` on
2026-10-03 and added Prose on 2026-10-05. Scheduled 2026-10-11, last of the
seven features, this work follows:

- `2026-10-03-symbol-filters-and-reports`: selectors, qualified names,
  signature modes, ordering, streaming and the one-output-mode rule.
- `2026-10-05-deferred-kind-precision`: the recorded kinds used by exports.
- `2026-10-06-symbol-type-relations`: declared relations shown by outlines.

The author decided on 2026-10-05 that a missing consumer is not a reason to
defer: formats must exist before callers can adopt them. Completion dates do
not substitute for dependencies being implemented. Unresolved kind or
relation decisions in prerequisites are not silently decided by this export
feature.

**Reader's note:** being last in the sequence does not mean every format
carries every earlier feature's data. The eight TSV columns and editor tags
are bounded projections. Markdown and Prose inherit existing listing and
outline content, including declared relations where those outlines show
them. Metrics, references and full relation objects remain available through
JSON; choosing an export never enables their analysis merely because they
exist in the record schema.

## Shared command contract

The tree-hugger-cli package's [output resolver](../../cli/src/output.rs)
chooses one report mode and rejects conflicting flags. Extend that rule
explicitly rather than treating a new format as a terminal layout by default.

| Combination | Contract |
|---|---|
| New `--format` with `--json`, `--json-full`, `--jsonl` or `--count` | Argument error, exit 2 |
| `--stats` with any new format | Argument error; existing stats output forms stay unchanged |
| `--tree --format markdown` or `prose` | Accepted on functions, types and symbols |
| `--tree --format tsv` or `ctags` | Argument error: rows/tags have no outline layout |
| `classes --tree` | Still an argument error; classes already nests members |
| `--signatures full|short|none` with TSV, Markdown or Prose | Accepted; default short |
| Explicit `--signatures` or `--comments` with ctags | Argument error; tags is navigation output |
| `--comments` with TSV | Argument error; the fixed columns contain no comment field |
| Global `--plain` with any new format | Accepted; affects Prose presentation as described below, leaves other exports unchanged |

New formats are UTF-8, use LF line endings on every OS, contain no ANSI or
OSC 8 sequences, and never wrap at terminal width. `COLUMNS`, redirected
stdout, color forcing and terminal capabilities do not change their bytes.
Control characters in source-derived values must be made inert before output;
escaping markup alone does not remove terminal controls.

Keep current error behavior: report failed files on stderr, emit successful
files, and exit 1 if any file failed. A successful empty selection exits 0;
a scan that finds no source files retains the existing discovery error.
Verbose/cache messages stay on stderr. A closed stdout pipe stops work
quietly with the existing exit 0 behavior; other write failures exit 1.
Callers saving exports must check the exit status, since these formats have
no JSON `errors` array or completion marker. Atomic file replacement is the
caller's responsibility; this feature adds no output-file management flag.

## TSV contract

Emit exactly one header, followed by one row per selected record. An empty
selection emits the header alone. There are exactly eight tab-separated cells
on every row, including trailing empty cells:

```text
file<TAB>line<TAB>column<TAB>kind<TAB>name<TAB>qualified_name<TAB>visibility<TAB>signature
```

`<TAB>` above denotes an actual tab. Values are:

- `file`: the record's declaration file, relative to the scan root, with `/`
  path separators, using the existing report path conversion. A file outside
  that root retains the existing portable fallback; do not invent a path or
  change symbol identity.
- `line`, `column`: the recorded declaration start, both 1-based; do not
  substitute the identifier's span or change the column unit.
- `kind`: the record kind's snake_case spelling, matching JSON and `--kind`;
  never a display keyword or guessed Ctags letter.
- `name`, `qualified_name`: the selected record's names, including the
  scan-root prefix applied by existing query finishing rules.
- `visibility`: its snake_case value, with Unknown represented by an empty
  cell, as in the existing table output. Missing optional values are empty,
  never the strings `null` or `None`.
- `signature`: the same value as the terminal table's Signature cell for the
  chosen signature mode. `none` leaves this cell empty, preserving the
  header; `full` uses the recorded header with the existing short fallback.

Encode each cell in one pass: backslash as `\\`, tab as `\t`, CR as `\r`,
and LF as `\n`. Other control characters use the existing visible `\u{..}`
notation. Do not escape already encoded text again. Unicode, quotes and
ordinary spaces remain unchanged. For example, a literal backslash followed
by `n` encodes as `\\n`, distinguishable from a newline encoded as `\n`.
The decoder recognizes these escapes in one pass; no raw control character
may appear inside a cell.

Files remain in portable path order and `--sort` orders records inside each
file, as with JSON Lines/table output. Grouping flags add no headings or
blank rows and do not alter this order. Stream after each completed batch;
retain at most one file's row set beyond existing scan/prelude buffering.

For `classes`, flatten each selected container and its displayed methods and
constructors into records, then apply the same per-file order. Preserve member
selectors and static/instance filtering. Do not manufacture field records from
class-summary field metadata, which lacks an independent declaration span;
TSV is a record export, while Markdown/Prose retain the class listing's field
sections. Emit a record only once by ID if it is reached through more than one
container. Synthetic prelude records remain visible in TSV, as in JSON.

## Prose contract

The tree-hugger-cli package's [declaration renderer](../../cli/src/render.rs)
uses biscuit-terminal Prose for inline styling and UnorderedList for bullets.
Share declaration text, signature formatting, location labels, grouping,
class sections and outline ownership with that renderer. Extract only the
helpers needed to emit unrendered markup; do not duplicate symbol formatting.

Prose output preserves the current layout: package sections when applicable,
optional file/module groups, outline indentation, static/instance class
sections, and doc lines under `--comments`. Default output does not become
file-grouped just because it is exported. Each generated styling/link tag
closes within its line, so concatenated reports do not leak presentation.

Source names, signatures, comments and paths use the renderer's `literal`
helper, which neutralizes controls before calling the biscuit-terminal
package's [Prose text escaping](../../../biscuit-terminal/lib/src/components/prose/prose.rs).
Attribute values use that package's `Prose::quoted_attr`. No source-derived
markup is interpreted. Include tests for backtick runs as well as tags,
emphasis and link syntax; do not assume all source text is identifier-shaped.

Generate source-location links independently of terminal detection, using the
existing file-URL encoder and `#L<line>` convention. Spaces, Unicode,
parentheses and native Windows paths must survive URL encoding and attribute
quoting. These local file links are meant for local rendering, not portable
navigation in a hosted pull request. With global `--plain`, omit generated
styles and links but retain literal escaping and report layout: this remains
valid Prose input, not raw unescaped source.

**Reader's note:** a plain terminal with OSC 8 disabled renders a Prose link
as `[label](url)`, whereas `--format plain` omits the link entirely. Replace
the draft's impossible direct round-trip invariant with two checks: unwrapped,
ANSI-stripped rendering of styled Prose on an OSC-8-capable terminal matches
`--format plain` text; rendering `--plain --format prose` without styles or
links matches `--format plain` directly. Check matching commands, selectors,
signature modes, comments and grouping in both cases.

Prose supports inline markup, not a serialized document/list structure.
Embedding or rendering the report preserves its textual indentation; it does
not promise semantic nested HTML lists. A structured cross-target document
export is outside this feature.

## Markdown contract

Emit portable CommonMark lists. A flat report has one bullet per declaration;
an outline nests member bullets under the correct container, and `classes`
retains its method/field section hierarchy. Context headings for filtered-out
containers remain context only, never added declarations. Keep overloads
separate and inherit the existing per-file outline ordering and relation
labels, including relations under `--signatures none`.

Use biscuit-terminal UnorderedList or equivalent shared render-tree list
nodes for structure, and Prose-derived inline nodes for the declaration text.
Use headings for existing package/module/file groups with valid blank-line
separation. Indent continuation/comment text under its owning list item;
source comment text is literal content, not an embedded Markdown document.
An empty selection emits a readable `(no symbols)` message using the existing
package-section behavior.

**Reader's note:** folding the whole Prose string through its Markdown
renderer would not reconstruct nested lists. Furthermore, the shared
renderable package's [Markdown text renderer](../../../renderable/src/tree/render/markdown.rs)
currently passes ordinary text nodes through outside table/HTML contexts.
Prose escaping protects its input parser, but that protection is consumed
before Markdown output; it does not guarantee safe Markdown literals.

Reuse the common inline nodes, but apply CommonMark literal escaping to
source text nodes at the Markdown output boundary, not to the finished
Markdown string. This preserves intentional bullets/emphasis while preventing
source text such as `[x](url)`, `**x**`, backticks, `<img>`, `&copy;`, `#`,
`-` and numbered-list prefixes from creating links, HTML, entities or blocks.
A small CLI-owned adapter over source text nodes is in scope; a repository-wide
Markdown renderer change needs its own contract review and is not assumed
here. Golden tests alone do not establish this escaping contract.

Markdown locations are readable labels with no generated file links. This
makes pasted reports useful in docs/PRs without shipping local absolute URLs
or pretending that a hosted renderer can navigate `#L` file links. Prose
retains local navigation. Names, signatures and comments cannot create links
of their own. Markdown drops presentation-only colors/dimming and preserves
semantic emphasis. Global `--plain` does not alter this Markdown contract.

Markdown and Prose inherit listing buffering: one file for outlines/classes
and grouped files, one directory for module groups, and one package for
ungrouped name/kind sorting or prelude expansion. Avoid holding a second copy
of the entire scan to build either export.

## Open Questions

### Which editor tags contract should be supported?

Tags readers differ in accepted escapes and kind/scope conventions. Numeric
addresses and duplicates are supported by the
[documented tags format](https://docs.ctags.io/en/latest/man/tags.5.html),
but kind values depend on the language. An editor navigation claim needs a
named reader and a tested subset, not just a syntactically plausible golden.
Choose before implementing ctags; the recommendation is not author approval.

- **Minimal traditional tags.** Emit name, file and numeric declaration line.
  Pros: smallest contract and easiest basic navigation. Cons: loses kind and
  container information; unusual names/paths still need a stated policy;
  traditional sorted output requires buffering or a different order contract.
- **Documented extended subset (recommended).** Emit numeric addresses, format
  and sort headers, and a reviewed per-language mapping for kind/container
  fields. Pros: retains the information needed to distinguish members and
  same-named declarations, while keeping scope bounded. Cons: mapping and
  reader tests are required; some names or paths may be unsupported and the
  reader must accept unsorted files.
- **Universal Ctags output parity.** Match that tool's language-specific kinds,
  escape extensions and scope conventions. Pros: broader existing-reader
  compatibility. Cons: substantially larger maintenance and test surface,
  coupled to an external tool's language contracts rather than Hug's records.

Recommend the extended subset: kinds and containers are central to these
reports, and broad tool parity is unnecessary for navigation. If accepted,
make the following boundaries concrete in the implementation plan:

- Use numeric declaration-line addresses, never source-derived Ex commands.
  Emit `!_TAG_FILE_FORMAT` value `2` and `!_TAG_FILE_SORTED` value `0`;
  preserve per-file report order even under `--sort name`, which does not
  imply global tag-name order. Retain overloads and duplicate names.
- Resolve paths relative to the scan root. A saved tags file belongs at that
  root; moving it requires caller path rebasing. Preserve `/` separators on
  all OSes. Document outside-root paths and reader behavior explicitly.
- Publish a complete mapping for every supported record kind and language;
  Unknown or imprecise Type must not be guessed into a more specific kind.
  Container fields derive from recorded ownership, never name equality.
  Use genuine container-qualified names rather than scan-root file prefixes.
- Omit synthetic/locationless records and class-summary field-only entries.
  For classes, use selected containers and displayed method/constructor
  records once by ID, preserving their selectors. Empty output retains the
  selected contract's headers.
- Define separate name, filename and extension-value encoding from the tags
  standard. TSV escaping is not automatically valid for these fields. Either
  prove the selected reader accepts escaped unusual names/paths or skip those
  entries with a stderr warning and exit 1, so an incomplete index cannot
  appear successful. Never insert raw tabs/newlines or make up a filename.
- Choose one non-interactive reader fixture that verifies lookup and numeric
  navigation for duplicates, container fields, spaces, Unicode and the chosen
  escaping boundaries. A headless editor or tags-reader library suffices;
  Universal Ctags need not be installed on every host. Do not advertise full
  Universal Ctags compatibility.

## Validation, documentation and completion

Add one golden per format over the existing 16-language `multi/` fixture,
using the established scratch-copy approach because normal CLI discovery
excludes repository fixture directories. Normalize only changing root paths
and IDs where present. Assert exports contain no ANSI/OSC sequences **before**
normalization; ANSI stripping must not conceal a format violation.

Focused cases must verify:

- The output-option table, empty selection, one failed file alongside a
  successful file, stderr separation, closed pipes and ordinary write errors.
- TSV's single header, eight cells including trailing empties, reversible
  escaping, Unicode, visibility/kind spelling and all signature modes.
- Flat, grouped, class and nested reports; filtered-out containers, duplicate
  names in different files and overloads; exact agreement with existing
  selection and class static/instance rules.
- Source control sequences and markup in names, signatures, comments and
  paths; Prose's corrected plain comparisons and attribute-safe links;
  Markdown parsed as CommonMark with expected list nesting and literal text,
  without unintended links/HTML/emphasis. Use an existing parser dependency
  where available; any added test dependency must be documented.
- Stable bytes across worker counts, warm/cold cache and narrow/wide terminal
  widths. Confirm output-format selection adds no references, diagnostics,
  semantics or metrics work beyond the equivalent listing/selectors, and
  containment is requested only for layouts that need it.
- The accepted tags subset and named reader fixture, including the unsorted
  header and duplicate entries, before editor integration is claimed.

Implementation validation uses `just test` and `just lint` in `tree-hugger/`.
If shared terminal behavior changes, also run existing `just test-l2` checks
with the test toolkit, without focusing terminal or browser windows. All four
supported environments remain required: macOS, Linux, native Windows and
WSL2. This feature adds no CI cells or gates and no performance spike; existing
work counters and buffering checks cover the concrete repeated-analysis and
whole-scan retention risks.

Update the CLI README, `tree-hugger/docs/symbol-reports.md`, help/completions
and the tree-hugger skill alongside implementation. Current docs explain the
field/escaping rules, flag combinations, literal rendering, class export
limits, local Prose links and accepted tags-reader subset without referring
to this feature. Update dependency pages if a crate is added. No record/index
schema or extraction/cache revision change is needed for presentation alone.
Review changed renderer/helper comments for behavior drift, especially claims
that formats or outlines are terminal-only.

Implementation is complete, ready for review, when the accepted contracts are
implemented, focused checks pass, all four format goldens are reviewed and
current docs agree. Record the accepted tags choice and any departures in the
implementation log. Leave the directory active; the author closes it after
review.
