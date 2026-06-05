# God Files — Draft Specification

> Status: **Draft** · Area: `tree-hugger` (lib + cli) · Date: 2026-04-05

## 1. Overview

A high-performance facility for scanning a directory tree and identifying
**"god files"** — oversized source files that are strong candidates for
refactoring. The feature ships in two layers:

- **Library** (`tree-hugger-lib`): a lazily-evaluated `GodFiles` struct that
  discovers candidates and produces a structured `GodAnalysis` per file.
- **CLI** (`tree-hugger-cli`, binary `hug`): a `god-files` subcommand that
  renders the analysis grouped by risk.

The design optimizes for speed on large trees by splitting cheap discovery
(line counting, no parse) from expensive analysis (parse + symbol extraction),
running both in parallel and analyzing only the small set of candidates.

## 2. Definitions & Risk Bands

A "god file" is a source file whose **effective lines of code (SLOC)** cross a
risk threshold. SLOC excludes blank lines and comment-only lines.

| Band            | Effective SLOC | Meaning                                |
|-----------------|----------------|----------------------------------------|
| (not a candidate) | `< 400`      | Ignored / dropped.                     |
| **Moderate risk** | `400 – 999`  | Potential problem.                     |
| **High risk**     | `>= 1000`    | High-risk refactoring candidate.       |

Scope is limited to **programming languages** only. All 16 currently-supported
tree-hugger grammars are programming languages (Rust, JavaScript, TypeScript,
Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua). CSS and
HTML are explicitly *not* programming languages, but they are also not currently
supported grammars, so no file is excluded today. To future-proof against
markup grammars being added, the library gains an
`ProgrammingLanguage::is_programming_language() -> bool` predicate (returns
`true` for all current variants); the candidate walk filters on it.

## 3. Decisions Log (resolved during clarification)

These decisions are settled and drive the design below.

1. **LOC metric.** Candidate screening uses raw **physical line count**
   (newline count, zero parsing) for maximum speed. **Effective SLOC**
   (physical − blank − comment-only lines, derived from the parse tree) is the
   **authoritative** metric that decides final risk classification.
2. **Re-filter (both bands).** After SLOC is computed, files are reclassified at
   *both* boundaries: a file under 400 effective lines is **dropped**; a file
   whose physical lines were `>= 1000` but whose SLOC is `< 1000` is **demoted**
   high → moderate. The reported number always matches the risk reasoning.
3. **Cache API — interior mutability (`&self`).** `OnceCell`-backed caches so
   `.candidates(&self) -> &Vec<PathBuf>` and `.analysis(&self) -> &Vec<GodAnalysis>`
   keep `&self` signatures and lazily fill on first call.
4. **Two-phase, parallel performance model.** `candidates()` counts newlines in
   parallel with no parse; `analysis()` parses only the candidate subset in
   parallel. Candidate files are read twice (count, then parse) — negligible
   versus parsing the whole tree.
5. **Rendering — biscuit-terminal `Prose`.** The report is built with `Prose`;
   the markup in this spec maps 1:1 and yields capability-aware styling and OSC8
   hyperlinks without hand-emitted escape codes.
6. **Largest-blocks selection — top-N + floor.** List the largest symbols by
   SLOC, capped at `MAX_BLOCKS` (default 8) and only those `>= MIN_BLOCK_SLOC`
   (default 15), with a `+N more` note when truncated.
7. **"Many children" call-out — fixed absolute threshold.** A container symbol
   (class/impl/trait/etc.) with more than `MANY_MEMBERS_THRESHOLD` (default 10)
   members gets a child sub-list. Variable/field/parameter members are excluded
   from the count (they do not explain structure).

## 4. Library Design

### 4.1 Module placement

New module `tree-hugger/lib/src/god_files/` (sibling to `analysis/`), re-exported
from `lib.rs`. Public surface: `GodFiles`, `GodAnalysis`, `RiskBand`,
`SymbolBlock`, `ContainerCallout`, `KindHistogram`, `RefactorHint`.

### 4.2 `GodFiles`

```rust
/// Lazily-evaluated god-file scanner rooted at a directory.
pub struct GodFiles {
    root: PathBuf,
    candidates: OnceCell<Vec<PathBuf>>,
    analysis: OnceCell<Vec<GodAnalysis>>,
}

impl GodFiles {
    /// Construct a scanner rooted at `dir`. No I/O occurs until
    /// `candidates()` or `analysis()` is called.
    pub fn new(dir: impl AsRef<Path>) -> Self;

    /// All candidate god files (>= 400 *physical* lines), discovered by a
    /// cheap parallel newline scan. Cached after first call.
    pub fn candidates(&self) -> &Vec<PathBuf>;

    /// Full analysis of every candidate. Populates `candidates()` first if
    /// needed, then parses only candidates in parallel and re-filters by
    /// effective SLOC. Cached after first call.
    pub fn analysis(&self) -> &Vec<GodAnalysis>;
}
```

- `OnceCell` (single-threaded `std::cell::OnceCell`) is sufficient because the
  caches are filled once behind `&self` within a command. If a `Sync` variant is
  later required (sharing a `GodFiles` across threads), swap to `OnceLock`.
- The expensive *inner* work (walking, counting, parsing) uses `rayon`
  internally; the `OnceCell` only guards the memoized result.
- `candidates()` returning `&Vec<PathBuf>` matches the requested
  `.candidates() -> &Vec<Path>` API (using owned `PathBuf` rather than borrowed
  `Path`, which cannot be returned from a freshly-built cache).

### 4.3 `GodAnalysis` and supporting types

```rust
pub struct GodAnalysis {
    /// Resolvable reference to the file (drives the OSC8 hyperlink).
    pub file: FileReference,          // from biscuit-file
    /// Path relative to the scan root, for display.
    pub relative_path: PathBuf,
    pub language: ProgrammingLanguage,

    pub physical_lines: usize,        // raw newline count (screening metric)
    pub effective_sloc: usize,        // physical - blank - comment_only (authoritative)
    pub risk: RiskBand,               // decided by effective_sloc

    /// Largest symbol blocks (top-N by SLOC, >= MIN_BLOCK_SLOC), pre-sorted desc.
    pub blocks: Vec<SymbolBlock>,
    /// Count of blocks omitted by the top-N cap (for the "+N more" note).
    pub blocks_truncated: usize,

    // --- Signal: structural shape ---
    pub top_level_symbol_count: usize,
    pub kind_histogram: KindHistogram, // e.g. { Trait: 5, Type: 12, Function: 40 }

    // --- Signal: complexity ---
    pub max_nesting_depth: usize,

    // --- Signal: coupling & debt ---
    pub import_fan_out: usize,          // count of imported symbols / import stmts
    pub todo_fixme_count: usize,        // TODO|FIXME|HACK|XXX in comments
    pub comment_density: f32,           // comment_lines / physical_lines (0.0..=1.0)

    // --- Signal: derived guidance ---
    pub refactor_hints: Vec<RefactorHint>,
}

pub enum RiskBand { Moderate, High }   // sub-400 files never produce a GodAnalysis

pub struct SymbolBlock {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub end_line: usize,
    pub sloc: usize,                    // effective lines within the block span
    pub doc_summary: Option<String>,    // first line of doc_comment, for context
    /// Present when member_count > MANY_MEMBERS_THRESHOLD.
    pub many_members: Option<ContainerCallout>,
}

pub struct ContainerCallout {
    pub member_count: usize,            // structural members only (no vars/fields)
    /// Sub-list of notable members (capped, same MAX_BLOCKS policy).
    pub members: Vec<MemberSummary>,
}

pub struct MemberSummary { pub name: String, pub kind: SymbolKind, pub sloc: usize }

pub struct KindHistogram(pub BTreeMap<SymbolKind, usize>);

pub enum RefactorHint {
    DominatedBySingleSymbol { name: String, share: f32 },   // one symbol >> rest
    ManyUnrelatedTopLevel { count: usize },                 // split by responsibility
    DeeplyNested { depth: usize },                          // extract / flatten
    HighCoupling { import_fan_out: usize },                 // blast radius warning
    LowCodeDensity { comment_density: f32 },                // mostly comments/docs
}
```

### 4.4 Algorithm

**Phase 1 — `candidates()` (cheap, parallel, no parse):**

1. `scanner::collect_files(root, &[], &[], None)` enumerates source files,
   honoring `.gitignore` and the default fixture exclusions, restricted to
   supported (programming-language) extensions.
2. In parallel (`rayon`), for each file: read bytes, count newlines with
   `memchr::memchr_iter(b'\n', ...)`, keep files with `>= 400` physical lines.
3. Sort deterministically (by path); store in the cache.

**Phase 2 — `analysis()` (expensive, parallel, candidates only):**

1. Ensure `candidates()` is populated.
2. In parallel, for each candidate: `TreeFile::new(path)` → derive:
   - **Effective SLOC.** Walk the parse tree's comment nodes (per-language
     `comments.scm`) to mark comment-only lines; mark blank lines from source;
     `effective_sloc = physical − blank − comment_only`. (A line containing both
     code and a trailing comment counts as code.)
   - **Re-filter:** drop if `effective_sloc < 400`; otherwise band by SLOC.
   - **Blocks.** From `symbols()` / `symbol_records()`, compute per-symbol SLOC
     over its span, rank desc, apply top-N + floor, set `blocks_truncated`.
   - **Structural shape.** `top_level_symbol_count` = symbols with no container;
     `kind_histogram` tallies `SymbolKind` across top-level symbols.
   - **Max nesting depth.** Deepest block-nesting via a single tree walk.
   - **Coupling/debt.** `import_fan_out` = `imported_symbols().len()`;
     `todo_fixme_count` from comment text; `comment_density` from comment lines.
   - **Container call-outs.** For each container symbol, count structural members
     (exclude `Variable`/`Field`/`Parameter`); attach `ContainerCallout` when
     `> MANY_MEMBERS_THRESHOLD`.
   - **Refactor hints.** Rule-based synthesis from the signals above.
3. Sort results: **High before Moderate**, then by `effective_sloc` desc within
   each band; store in the cache.

### 4.5 Constants (tunable, documented)

| Constant                  | Default | Purpose                                  |
|---------------------------|---------|------------------------------------------|
| `MODERATE_MIN_SLOC`       | 400     | Lower bound of moderate band.            |
| `HIGH_MIN_SLOC`           | 1000    | Lower bound of high band.                |
| `MAX_BLOCKS`              | 8       | Cap on listed symbol blocks per file.    |
| `MIN_BLOCK_SLOC`          | 15      | Floor below which a symbol is not listed.|
| `MANY_MEMBERS_THRESHOLD`  | 10      | Member count above which a container is called out. |

## 5. CLI Design

### 5.1 Subcommand

```
hug god-files [DIR] [--high-risk]
```

- `DIR` (optional positional): directory to scan. Defaults to the current
  working directory.
- `--high-risk`: filter output to high-risk files only (moderate section and
  its heading line are suppressed; the report heading still reports both counts).
- Honors the existing global output flags (`--plain`, `--json`) where applicable;
  `--json` emits the `Vec<GodAnalysis>` as structured JSON.

### 5.2 Scanning

Reuses `scanner::collect_files`; the `god-files` handler constructs
`GodFiles::new(dir)` and calls `.analysis()`.

### 5.3 Rendering (biscuit-terminal `Prose`)

Risk bands render as two sections, **High risk first**, then **Moderate risk**.
Empty bands are omitted. Markup below is literal Prose source.

**Report heading** (margin-top: 1):

```
- There are <yellow>{moderate_count}</yellow> files with moderate risk of being considered _god files_
- There are <red>{high_count}</red> files with <b>high risk</b> of being considered _god files_
```

**Section heading** (margin-top: 1, margin-bottom: 1):

```
<b><uu>{section}</uu></b>
```

where `{section}` is `High risk` or `Moderate risk`.

**File list item** (one per file; `<red>` for high, `<yellow>` for moderate):

```
- the <a href={file-ref}>{rel-path}</a> file is <b><red|yellow>{sloc}</red|yellow></b> lines of code
```

**Nested under each file item:**

- `- the largest blocks in this file are composed by these symbols:`
  - one line per `SymbolBlock`: kind + name + line span + SLOC, with the
    `doc_summary` appended inline (dimmed) for context when present.
  - when `many_members` is set, a `(N members)` note plus an indented
    sub-list of notable members.
  - a final `- …and {blocks_truncated} more` line when truncated.
- a compact **signals** line: `top-level symbols: {n} · max depth: {d} ·
  imports: {f} · TODO/FIXME: {t} · comments: {density%}`.
- each `RefactorHint` as a short dimmed bullet (e.g.
  `- likely refactor: one symbol holds {share%} of the code — extract methods`).

### 5.4 Example output (illustrative)

```
- There are 3 files with moderate risk of being considered god files
- There are 1 files with high risk of being considered god files

High risk

- the src/engine/render.rs file is 1342 lines of code
  - the largest blocks in this file are composed by these symbols:
    - impl Renderer  [88–612]  524 sloc  (31 members)
      - fn render_tree  [120–410]  290 sloc
      - fn layout_pass  [430–540]  110 sloc
      - …and 29 more
    - fn fold_node  [640–910]  270 sloc — folds a render node to terminal text
    - …and 4 more
  - top-level symbols: 6 · max depth: 9 · imports: 41 · TODO/FIXME: 3 · comments: 12%
  - likely refactor: `impl Renderer` holds 39% of the code — split by responsibility
  - likely refactor: high coupling (41 imports) — expect wide refactor blast radius
```

## 6. Performance

- Candidate screening is memory-bandwidth bound (newline counting via `memchr`)
  and embarrassingly parallel; it never invokes tree-sitter.
- Only candidates are parsed. Because god files are rare, the costly parse runs
  on a small subset.
- Both phases use `rayon`; the `OnceCell` caches guarantee single computation.
- Reuse of tree-hugger's in-process / persistent parse cache is permitted but
  not required for correctness.

## 7. Edge Cases

- **Empty / no candidates:** report heading shows `0` / `0`; both sections
  omitted. Exit success.
- **Unparseable candidate:** if `TreeFile::new` fails, fall back to physical
  lines for banding, emit the file with empty `blocks`/signals and a diagnostic
  note; do not abort the whole scan.
- **Files with code+comment on one line:** counted as code (not comment-only).
- **Symlinks / non-UTF8 / binary:** excluded by the scanner's normal filters.
- **`--high-risk` with zero high-risk files:** heading still prints both counts;
  no section body.

## 8. Testing

- **Unit (lib):** SLOC computation (blank/comment/mixed lines) per language;
  band classification and re-filter (drop + demote) boundaries at 399/400/999/
  1000 SLOC; block ranking with floor + cap + truncation count; container
  call-out threshold; refactor-hint rules. Fixtures under the standard
  `fixtures/` tree (excluded from scans by default).
- **Lazy/cache behavior:** `candidates()` populated implicitly by `analysis()`;
  repeated calls return the same cached reference.
- **CLI integration (`assert_cmd` + `predicates`):** `hug god-files <dir>`
  grouping/order; `--high-risk` filtering; `--json` shape; default-CWD behavior;
  `--plain` strips styling.
- **Performance smoke:** assert candidate screening does not parse (e.g. via a
  counter / timing on a large fixture tree).

## 9. Open Questions / Future Work

- Should thresholds and `MANY_MEMBERS_THRESHOLD` be CLI-overridable
  (`--moderate`, `--high`, `--members`)? Deferred; constants for v1.
- Per-language SLOC nuance (e.g. Python docstrings as comments vs. code) — start
  with comment-node detection; refine if corpus shows skew.
- Optional severity sort by a blended score (SLOC × depth × coupling) instead of
  pure SLOC — deferred.
```
