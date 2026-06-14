//! Analysis pipeline for god-file detection.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::FileReference;
use rayon::prelude::*;
use tree_sitter::Node;

use crate::file::tree_file::TreeFile;
use crate::scanner::collect_files;
use crate::shared::{ProgrammingLanguage, SymbolKind, SymbolKindV2, SymbolRecord};

use super::constants::{MANY_MEMBERS_THRESHOLD, MAX_BLOCKS, MIN_BLOCK_SLOC, MODERATE_MIN_SLOC};
use super::types::{
    ContainerCallout, GodAnalysis, KindHistogram, MemberSummary, RefactorHint, RiskBand,
    SymbolBlock,
};

/// Lazily-evaluated god-file scanner rooted at a directory.
#[derive(Debug)]
pub struct GodFiles {
    root: PathBuf,
    candidates: OnceCell<Vec<PathBuf>>,
    analysis: OnceCell<Vec<GodAnalysis>>,
}

impl GodFiles {
    /// Construct a scanner rooted at `dir`. No I/O occurs until
    /// `candidates()` or `analysis()` is called.
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            root: dir.as_ref().to_path_buf(),
            candidates: OnceCell::new(),
            analysis: OnceCell::new(),
        }
    }

    /// All candidate god files (>= 400 *physical* lines), discovered by a
    /// cheap parallel newline scan. Cached after first call.
    pub fn candidates(&self) -> &Vec<PathBuf> {
        self.candidates
            .get_or_init(|| discover_candidates(&self.root))
    }

    /// Full analysis of every candidate. Populates `candidates()` first if
    /// needed, then parses only candidates in parallel and re-filters by
    /// effective SLOC. Cached after first call.
    pub fn analysis(&self) -> &Vec<GodAnalysis> {
        self.analysis.get_or_init(|| {
            let candidates = self.candidates().to_vec();
            let root = self.root.clone();

            let mut analyses: Vec<GodAnalysis> = candidates
                .into_par_iter()
                .filter_map(|path| analyze_single_file(&path, &root))
                .collect();

            // Sort: High risk first, then descending effective SLOC,
            // then deterministic path order.
            analyses.sort_by(|a, b| {
                let band_order = match (a.risk, b.risk) {
                    (RiskBand::High, RiskBand::Moderate) => std::cmp::Ordering::Less,
                    (RiskBand::Moderate, RiskBand::High) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                };
                band_order
                    .then_with(|| b.effective_sloc.cmp(&a.effective_sloc))
                    .then_with(|| a.relative_path.cmp(&b.relative_path))
            });

            analyses
        })
    }
}

/// Discover files under `root` that have at least `MODERATE_MIN_SLOC` physical
/// lines. Files are sorted deterministically by path.
fn discover_candidates(root: &Path) -> Vec<PathBuf> {
    let files = match collect_files(root, &[], &[], None) {
        Ok(files) => files,
        Err(crate::TreeHuggerError::NoSourceFiles { .. }) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    // Filter to programming languages and count newlines in parallel.
    let mut with_lines: Vec<(PathBuf, usize)> = files
        .into_par_iter()
        .filter_map(|path| {
            let language = ProgrammingLanguage::from_path(&path)?;
            if !language.is_programming_language() {
                return None;
            }
            let physical_lines = count_physical_lines(&path).ok()?;
            if physical_lines >= MODERATE_MIN_SLOC {
                Some((path, physical_lines))
            } else {
                None
            }
        })
        .collect();

    // Deterministic ordering.
    with_lines.sort_by(|a, b| a.0.cmp(&b.0));
    with_lines.into_iter().map(|(path, _)| path).collect()
}

/// Count physical lines (newlines) in a file without parsing.
fn count_physical_lines(path: &Path) -> std::io::Result<usize> {
    let bytes = fs::read(path)?;
    // Count newline characters; if the file does not end with a newline,
    // the last line is still counted by adding 1 when there is content.
    let newline_count = memchr::memchr_iter(b'\n', &bytes).count();
    if bytes.is_empty() {
        Ok(0)
    } else if bytes.last() == Some(&b'\n') {
        Ok(newline_count)
    } else {
        Ok(newline_count + 1)
    }
}

/// Analyze a single candidate file, returning `None` if it drops below the
/// moderate threshold.
fn analyze_single_file(path: &Path, root: &Path) -> Option<GodAnalysis> {
    let physical_lines = count_physical_lines(path).ok()?;
    let language = ProgrammingLanguage::from_path(path)?;

    let (effective_sloc, parsed_data) = match TreeFile::new(path) {
        Ok(tree_file) => {
            let source = tree_file.source().to_string();
            let metrics = compute_sloc_metrics(&source, Some(&tree_file));
            let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
            let records = tree_file.symbol_records().unwrap_or_default();
            let imports = tree_file.imported_symbols().unwrap_or_default();

            let blocks_data = compute_symbol_blocks(&source, &records, &comment_ranges);
            let top_level = collect_top_level(&records);
            let top_level_symbol_count = top_level.len();
            let kind_histogram = build_kind_histogram(&top_level);
            let max_nesting_depth = compute_max_nesting_depth(&tree_file);
            let import_fan_out = imports.len();
            let todo_fixme_count = count_todo_fixme(&source, &comment_ranges);
            let comment_density = compute_comment_density(physical_lines, &source, &comment_ranges);

            let signals = SignalBundle {
                effective_sloc: metrics.effective_sloc,
                top_level_symbol_count,
                import_fan_out,
                max_nesting_depth,
                comment_density,
            };
            let refactor_hints = synthesize_hints(&signals, &blocks_data.blocks);

            (
                metrics.effective_sloc,
                Some(ParsedData {
                    blocks: blocks_data.blocks,
                    blocks_truncated: blocks_data.truncated,
                    top_level_symbol_count,
                    kind_histogram,
                    max_nesting_depth,
                    import_fan_out,
                    todo_fixme_count,
                    comment_density,
                    refactor_hints,
                }),
            )
        }
        Err(_) => {
            // Unparseable fallback: band using physical lines.
            (physical_lines, None)
        }
    };

    if effective_sloc < MODERATE_MIN_SLOC {
        return None;
    }

    let risk = RiskBand::from_sloc(effective_sloc)?;
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();

    // Build the resolvable reference from an absolute path so the OSC8
    // hyperlink target is correct regardless of the process working directory
    // when the scan root was given as a relative or explicit directory.
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let file = FileReference::new(absolute.to_str().unwrap_or("")).ok()?;

    // Candidates that failed to parse are still reported, banded by physical
    // line count, with a diagnostic note explaining the degraded analysis.
    let note = if parsed_data.is_none() {
        Some(format!(
            "could not parse as {language}; banded by physical line count only"
        ))
    } else {
        None
    };

    let (
        blocks,
        blocks_truncated,
        top_level_symbol_count,
        kind_histogram,
        max_nesting_depth,
        import_fan_out,
        todo_fixme_count,
        comment_density,
        refactor_hints,
    ) = match parsed_data {
        Some(data) => (
            data.blocks,
            data.blocks_truncated,
            data.top_level_symbol_count,
            data.kind_histogram,
            data.max_nesting_depth,
            data.import_fan_out,
            data.todo_fixme_count,
            data.comment_density,
            data.refactor_hints,
        ),
        None => (
            Vec::new(),
            0,
            0,
            KindHistogram::default(),
            0,
            0,
            0,
            0.0,
            Vec::new(),
        ),
    };

    Some(GodAnalysis {
        file,
        relative_path,
        language,
        physical_lines,
        effective_sloc,
        risk,
        blocks,
        blocks_truncated,
        top_level_symbol_count,
        kind_histogram,
        max_nesting_depth,
        import_fan_out,
        todo_fixme_count,
        comment_density,
        refactor_hints,
        note,
    })
}

/// Parsed analysis fields produced when TreeFile succeeds.
struct ParsedData {
    blocks: Vec<SymbolBlock>,
    blocks_truncated: usize,
    top_level_symbol_count: usize,
    kind_histogram: KindHistogram,
    max_nesting_depth: usize,
    import_fan_out: usize,
    todo_fixme_count: usize,
    comment_density: f32,
    refactor_hints: Vec<RefactorHint>,
}

/// Raw signals used to synthesize refactor hints.
struct SignalBundle {
    effective_sloc: usize,
    top_level_symbol_count: usize,
    import_fan_out: usize,
    max_nesting_depth: usize,
    comment_density: f32,
}

/// SLOC metrics for a single file.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct SlocMetrics {
    physical_lines: usize,
    blank_lines: usize,
    comment_only_lines: usize,
    effective_sloc: usize,
}

/// Compute SLOC metrics from source text and optional parsed tree.
///
/// Effective SLOC = physical − blank − comment_only. A line containing both
/// code and a trailing comment counts as code (not comment-only).
fn compute_sloc_metrics(source: &str, tree_file: Option<&TreeFile>) -> SlocMetrics {
    let physical_lines = count_physical_lines_from_source(source);

    let comment_ranges = tree_file
        .and_then(|tf| tf.comment_ranges().ok())
        .unwrap_or_default();

    let coverage = build_comment_coverage(&comment_ranges);

    let mut blank_lines = 0usize;
    let mut comment_only_lines = 0usize;

    let mut start = 0usize;
    for (end, _) in source.match_indices('\n') {
        classify_line(
            &source[start..end],
            start,
            &coverage,
            &mut blank_lines,
            &mut comment_only_lines,
        );
        start = end + 1;
    }

    if start < source.len() {
        classify_line(
            &source[start..],
            start,
            &coverage,
            &mut blank_lines,
            &mut comment_only_lines,
        );
    }

    let effective_sloc = physical_lines
        .saturating_sub(blank_lines)
        .saturating_sub(comment_only_lines);

    SlocMetrics {
        physical_lines,
        blank_lines,
        comment_only_lines,
        effective_sloc,
    }
}

/// Effective SLOC over a byte range, using the file-level comment coverage.
///
/// Counts physical lines within `[start_byte, end_byte)` and subtracts blank
/// and comment-only lines. `coverage` is indexed by absolute byte position so
/// a comment span that ends mid-line still classifies that line correctly.
fn effective_sloc_for_range(
    source: &str,
    start_byte: usize,
    end_byte: usize,
    coverage: &[bool],
) -> usize {
    let end_byte = end_byte.min(source.len());
    if start_byte >= end_byte {
        return 0;
    }

    let mut physical = 0usize;
    let mut blank = 0usize;
    let mut comment_only = 0usize;

    let segment = &source[start_byte..end_byte];
    let mut line_start = start_byte;
    for (rel_end, _) in segment.match_indices('\n') {
        let abs_end = start_byte + rel_end;
        physical += 1;
        classify_line(
            &source[line_start..abs_end],
            line_start,
            coverage,
            &mut blank,
            &mut comment_only,
        );
        line_start = abs_end + 1;
    }
    if line_start < end_byte {
        physical += 1;
        classify_line(
            &source[line_start..end_byte],
            line_start,
            coverage,
            &mut blank,
            &mut comment_only,
        );
    }

    physical.saturating_sub(blank).saturating_sub(comment_only)
}

fn count_physical_lines_from_source(source: &str) -> usize {
    if source.is_empty() {
        0
    } else {
        let newline_count = source.matches('\n').count();
        if source.ends_with('\n') {
            newline_count
        } else {
            newline_count + 1
        }
    }
}

fn build_comment_coverage(comment_ranges: &[(usize, usize)]) -> Vec<bool> {
    if comment_ranges.is_empty() {
        return Vec::new();
    }
    let max_end = comment_ranges
        .iter()
        .map(|(_, end)| *end)
        .max()
        .unwrap_or(0);
    let mut cov = vec![false; max_end];
    for (start, end) in comment_ranges {
        for i in *start..(*end).min(cov.len()) {
            cov[i] = true;
        }
    }
    cov
}

/// Classify a single line as blank, comment-only, or effective.
fn classify_line(
    line: &str,
    line_start_byte: usize,
    coverage: &[bool],
    blank_lines: &mut usize,
    comment_only_lines: &mut usize,
) {
    if line.trim().is_empty() {
        *blank_lines += 1;
        return;
    }

    if !coverage.is_empty() && is_line_comment_only(line, line_start_byte, coverage) {
        *comment_only_lines += 1;
    }
}

/// Returns true when every non-whitespace byte on the line falls inside a
/// comment span.
fn is_line_comment_only(line: &str, line_start_byte: usize, coverage: &[bool]) -> bool {
    for (i, byte) in line.bytes().enumerate() {
        if !byte.is_ascii_whitespace() {
            let pos = line_start_byte + i;
            if pos >= coverage.len() || !coverage[pos] {
                return false;
            }
        }
    }
    true
}

/// Count physical lines that overlap with at least one comment range.
fn compute_comment_density(
    physical_lines: usize,
    source: &str,
    comment_ranges: &[(usize, usize)],
) -> f32 {
    if physical_lines == 0 {
        return 0.0;
    }
    let comment_lines = count_comment_lines(source, comment_ranges);
    let density = comment_lines as f32 / physical_lines as f32;
    density.clamp(0.0, 1.0)
}

/// Count source lines that touch a comment span.
fn count_comment_lines(source: &str, comment_ranges: &[(usize, usize)]) -> usize {
    if comment_ranges.is_empty() {
        return 0;
    }
    let coverage = build_comment_coverage(comment_ranges);

    let mut count = 0usize;
    let mut start = 0usize;
    for (end, _) in source.match_indices('\n') {
        if line_overlaps_comment(&source[start..end], start, &coverage) {
            count += 1;
        }
        start = end + 1;
    }
    if start < source.len() && line_overlaps_comment(&source[start..], start, &coverage) {
        count += 1;
    }
    count
}

fn line_overlaps_comment(line: &str, line_start_byte: usize, coverage: &[bool]) -> bool {
    for (i, _) in line.bytes().enumerate() {
        let pos = line_start_byte + i;
        if pos < coverage.len() && coverage[pos] {
            return true;
        }
    }
    false
}

/// Count TODO/FIXME/HACK/XXX markers inside comment text.
///
/// Counts every case-insensitive marker occurrence, not the number of comment
/// nodes containing a marker: a single block comment with `TODO`, `FIXME`,
/// `HACK`, and `XXX` reports 4, and a marker repeated within one comment is
/// counted each time.
fn count_todo_fixme(source: &str, comment_ranges: &[(usize, usize)]) -> usize {
    let mut count = 0usize;
    for (start, end) in comment_ranges {
        if let Some(text) = source.get(*start..*end) {
            let upper = text.to_uppercase();
            for marker in ["TODO", "FIXME", "HACK", "XXX"] {
                count += upper.matches(marker).count();
            }
        }
    }
    count
}

/// Compute symbol blocks from the parsed symbol records.
///
/// Block SLOC is *effective* SLOC over the block span: comment-only and blank
/// lines inside a symbol body are excluded, using the file-level comment
/// coverage derived from the parse tree.
fn compute_symbol_blocks(
    source: &str,
    records: &[SymbolRecord],
    comment_ranges: &[(usize, usize)],
) -> BlocksResult {
    let coverage = build_comment_coverage(comment_ranges);

    let mut blocks: Vec<SymbolBlock> = records
        .iter()
        .filter_map(|rec| {
            let (start_byte, end_byte, start_line, end_line) =
                record_span_bytes(rec).unwrap_or((0, 0, 0, 0));
            let sloc = effective_sloc_for_range(source, start_byte, end_byte, &coverage);
            if sloc < MIN_BLOCK_SLOC {
                return None;
            }
            let doc_summary = rec
                .docs
                .raw_doc
                .as_ref()
                .and_then(|doc| doc.lines().find(|line| !line.trim().is_empty()))
                .map(|line| line.trim().to_string());

            let kind = SymbolKind::from(rec.kind);

            Some(SymbolBlock {
                name: rec.identity.name.clone(),
                kind,
                start_line,
                end_line,
                sloc,
                doc_summary,
                many_members: None,
            })
        })
        .collect();

    // Attach container call-outs to block entries that are structural containers.
    attach_many_members(source, records, &coverage, &mut blocks);

    // Rank descending by SLOC.
    blocks.sort_by(|a, b| b.sloc.cmp(&a.sloc).then_with(|| a.name.cmp(&b.name)));

    let truncated = blocks.len().saturating_sub(MAX_BLOCKS);
    blocks.truncate(MAX_BLOCKS);

    BlocksResult { blocks, truncated }
}

/// Result of computing ranked symbol blocks.
struct BlocksResult {
    blocks: Vec<SymbolBlock>,
    truncated: usize,
}

/// A symbol span as `(start_byte, end_byte, start_line, end_line)`.
type SpanBytes = (usize, usize, usize, usize);

/// Returns the full declaration byte span for a symbol record, falling back to
/// the name span only when no declaration span was recorded.
///
/// The declaration span covers the complete symbol block — the declaration line
/// plus its body — which is what block SLOC, displayed line ranges, and
/// ownership containment all require. The body span alone omits the declaration
/// line, under-counting a symbol by one or more lines and collapsing a multiline
/// declaration without a recognized body down to its name token.
fn record_span_bytes(rec: &SymbolRecord) -> Option<SpanBytes> {
    let span = &rec.source.declaration_span;
    if span.start_byte != 0 || span.end_byte != 0 {
        return Some((
            span.start_byte as usize,
            span.end_byte as usize,
            span.start.line as usize,
            span.end.line as usize,
        ));
    }
    let span = rec.source.name_span.as_ref()?;
    Some((
        span.start_byte as usize,
        span.end_byte as usize,
        span.start.line as usize,
        span.end.line as usize,
    ))
}

/// Returns true for member kinds that should be excluded from structural
/// member counts.
fn is_non_structural_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Variable | SymbolKind::Field | SymbolKind::Parameter
    )
}

/// Attach `ContainerCallout` to any block that represents a container with
/// more than `MANY_MEMBERS_THRESHOLD` *direct* structural members.
fn attach_many_members(
    source: &str,
    records: &[SymbolRecord],
    coverage: &[bool],
    blocks: &mut [SymbolBlock],
) {
    // Container spans, used both to bind a block to its exact record and to
    // reject members owned by a deeper (nested) container.
    let container_spans: Vec<(&SymbolRecord, SpanBytes)> = records
        .iter()
        .filter(|r| is_container_kind(r.kind))
        .filter_map(|r| record_span_bytes(r).map(|span| (r, span)))
        .collect();

    for block in blocks.iter_mut() {
        // Bind by identity *and* span. Two same-named containers in different
        // scopes have distinct line ranges, so the line span disambiguates them;
        // matching on `(name, kind)` alone would bind both blocks to the first.
        let Some(&(_, container_span)) = container_spans.iter().find(|(r, span)| {
            r.identity.name == block.name
                && SymbolKind::from(r.kind) == block.kind
                && span.2 == block.start_line
                && span.3 == block.end_line
        }) else {
            continue;
        };

        let (container_start, container_end, _, _) = container_span;

        let members: Vec<&SymbolRecord> = records
            .iter()
            .filter(|m| {
                let Some((m_start, m_end, _, _)) = record_span_bytes(m) else {
                    return false;
                };
                if is_non_structural_kind(SymbolKind::from(m.kind)) {
                    return false;
                }
                if !(m_start > container_start && m_end <= container_end) {
                    return false;
                }
                // Direct ownership only: reject members enclosed by a tighter
                // container nested within this one (e.g. methods of a nested
                // class), so an outer container is not inflated by descendants.
                !container_spans.iter().any(|(_, (c_start, c_end, _, _))| {
                    let (c_start, c_end) = (*c_start, *c_end);
                    let strictly_inside = c_start >= container_start
                        && c_end <= container_end
                        && (c_start > container_start || c_end < container_end);
                    strictly_inside && m_start > c_start && m_end <= c_end
                })
            })
            .collect();

        if members.len() > MANY_MEMBERS_THRESHOLD {
            let mut summaries: Vec<MemberSummary> = members
                .iter()
                .filter_map(|m| {
                    let (m_start, m_end, _, _) = record_span_bytes(m)?;
                    let sloc = effective_sloc_for_range(source, m_start, m_end, coverage);
                    Some(MemberSummary {
                        name: m.identity.name.clone(),
                        kind: SymbolKind::from(m.kind),
                        sloc,
                    })
                })
                .collect();
            summaries.sort_by(|a, b| b.sloc.cmp(&a.sloc).then_with(|| a.name.cmp(&b.name)));
            summaries.truncate(MAX_BLOCKS);
            block.many_members = Some(ContainerCallout {
                member_count: members.len(),
                members: summaries,
            });
        }
    }
}

/// Returns true for kinds that can act as structural containers.
fn is_container_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Class
            | SymbolKindV2::Interface
            | SymbolKindV2::Enum
            | SymbolKindV2::Trait
            | SymbolKindV2::Type
            | SymbolKindV2::Module
            | SymbolKindV2::Namespace
    )
}

/// Collect the records with no owning symbol — the file's top-level symbols.
///
/// A symbol is top-level only when it has neither a recorded container name
/// (`module_path`) nor a declaration span enclosed by another symbol. Relying
/// on `module_path` alone over-reports: the parse pass does not consistently
/// populate it for locals and parameters, so those would masquerade as
/// file-scope symbols. Span containment is the authoritative corrective signal
/// — a parameter or local always sits inside its owning function's declaration
/// span — while genuine file-scope variables remain enclosed by nothing and
/// stay counted.
fn collect_top_level(records: &[SymbolRecord]) -> Vec<&SymbolRecord> {
    let enclosed = compute_enclosed(records);
    records
        .iter()
        .enumerate()
        .filter(|(i, r)| r.identity.module_path.is_none() && !enclosed[*i])
        .map(|(_, r)| r)
        .collect()
}

/// For each record (indexed by position), whether another record's declaration
/// span strictly encloses it.
///
/// Symbol spans are laminar (AST-derived: any two are disjoint or one contains
/// the other), so a single sweep with a stack of enclosing spans decides
/// containment in `O(n log n)` — the same near-linear shape the analysis pass
/// requires for files with tens of thousands of symbols.
fn compute_enclosed(records: &[SymbolRecord]) -> Vec<bool> {
    let mut enclosed = vec![false; records.len()];

    // (record index, span). Records without a usable span cannot be proven
    // enclosed and are left at the default `false` (treated as file-scope).
    let mut spans: Vec<(usize, SpanBytes)> = records
        .iter()
        .enumerate()
        .filter_map(|(i, r)| record_span_bytes(r).map(|s| (i, s)))
        .collect();

    // Start ascending, end descending: wider (enclosing) spans sort before the
    // narrower spans they contain on a tie.
    spans.sort_by(|a, b| a.1.0.cmp(&b.1.0).then_with(|| b.1.1.cmp(&a.1.1)));

    let mut stack: Vec<SpanBytes> = Vec::new();
    for &(idx, (start, end, _, _)) in &spans {
        // Pop ancestors that do not strictly enclose the current span. Equal
        // spans are treated as siblings (strictness on at least one bound), so
        // a degenerate duplicate never claims to contain the original.
        while let Some(&(top_start, top_end, _, _)) = stack.last() {
            let encloses =
                top_start <= start && top_end >= end && (top_start < start || top_end > end);
            if encloses {
                break;
            }
            stack.pop();
        }
        if !stack.is_empty() {
            enclosed[idx] = true;
        }
        stack.push((start, end, 0, 0));
    }

    enclosed
}

/// Build a deterministic histogram from top-level symbols.
fn build_kind_histogram(top_level: &[&SymbolRecord]) -> KindHistogram {
    let mut map: BTreeMap<SymbolKind, usize> = BTreeMap::new();
    for rec in top_level {
        *map.entry(SymbolKind::from(rec.kind)).or_insert(0) += 1;
    }
    KindHistogram(map)
}

/// Compute the deepest nesting of control-flow constructs from a single walk
/// of the concrete syntax tree.
///
/// Symbol-span containment cannot see this signal: `if`/`for`/`while`/`match`/
/// `try` constructs are not extracted symbols, so a function packed with eight
/// nested `if` blocks would otherwise report the trivial depth of its symbol
/// records. Walking the tree-sitter tree and counting only control-flow
/// construct nodes (not their body blocks, and not function or class scopes)
/// yields a depth that grows with nested branching and stays flat for
/// sequential branches.
fn compute_max_nesting_depth(tree_file: &TreeFile) -> usize {
    let language = tree_file.language;
    let mut max_depth = 0usize;
    // Iterative DFS carrying the control-flow depth of each node's parent, so a
    // deep tree cannot overflow the call stack.
    let mut stack: Vec<(Node, usize)> = vec![(tree_file.root_node(), 0)];
    while let Some((node, depth)) = stack.pop() {
        let child_depth = if is_control_flow_nesting(language, node.kind()) {
            let here = depth + 1;
            max_depth = max_depth.max(here);
            here
        } else {
            depth
        };
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push((child, child_depth));
        }
    }
    max_depth
}

/// Returns true for syntax-tree node kinds that introduce a control-flow
/// nesting level in `language`'s grammar.
///
/// Classification is per-language because the same node kind means different
/// things across grammars: Scala's `try_expression` is a real `try { } catch`
/// block, while Rust's and Swift's `try_expression` is the postfix `?` /
/// prefix `try` operator and must not count. The kind sets are derived from the
/// pinned grammars' `node-types.json`.
///
/// Each set matches the construct node itself (`if_statement`, `for_statement`,
/// …) and not its body block, so each nesting level is counted once. Clause and
/// arm nodes (`else_clause`, `elif_clause`, `case_statement`, `catch_clause`)
/// are children of their construct and excluded — counting them would double the
/// depth of a single branch. Ternary `conditional_expression` is excluded: the
/// metric measures block nesting, not expression nesting.
fn is_control_flow_nesting(language: ProgrammingLanguage, kind: &str) -> bool {
    match language {
        ProgrammingLanguage::Rust => matches!(
            kind,
            "if_expression"
                | "match_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "try_block"
        ),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "for_in_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
                | "with_statement"
        ),
        ProgrammingLanguage::Go => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "expression_switch_statement"
                | "type_switch_statement"
                | "select_statement"
        ),
        ProgrammingLanguage::Python => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "match_statement"
                | "try_statement"
                | "with_statement"
        ),
        ProgrammingLanguage::Java => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "while_statement"
                | "do_statement"
                | "switch_expression"
                | "try_statement"
                | "try_with_resources_statement"
        ),
        ProgrammingLanguage::Php => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
                | "match_expression"
        ),
        // Perl block forms plus statement-modifier (postfix) and `*_simple`
        // forms; both express conditional/looping execution in this grammar.
        ProgrammingLanguage::Perl => matches!(
            kind,
            "if_statement"
                | "unless_statement"
                | "while_statement"
                | "until_statement"
                | "for_statement_1"
                | "for_statement_2"
                | "if_simple_statement"
                | "unless_simple_statement"
                | "while_simple_statement"
                | "until_simple_statement"
                | "for_simple_statement"
                | "when_simple_statement"
        ),
        ProgrammingLanguage::Bash => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "c_style_for_statement"
                | "while_statement"
                | "case_statement"
        ),
        ProgrammingLanguage::Zsh => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "c_style_for_statement"
                | "terse_for_statement"
                | "while_statement"
                | "repeat_statement"
                | "case_statement"
                | "select_statement"
        ),
        ProgrammingLanguage::C => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "seh_try_statement"
        ),
        ProgrammingLanguage::Cpp => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "for_range_loop"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
                | "seh_try_statement"
        ),
        ProgrammingLanguage::CSharp => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "switch_expression"
                | "try_statement"
        ),
        ProgrammingLanguage::Swift => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "repeat_while_statement"
                | "switch_statement"
                | "do_statement"
                | "guard_statement"
        ),
        ProgrammingLanguage::Scala => matches!(
            kind,
            "if_expression"
                | "for_expression"
                | "while_expression"
                | "do_while_expression"
                | "match_expression"
                | "try_expression"
        ),
        ProgrammingLanguage::Lua => matches!(
            kind,
            "if_statement" | "for_statement" | "while_statement" | "repeat_statement"
        ),
    }
}

/// Synthesize refactor hints from signals and ranked blocks.
fn synthesize_hints(signals: &SignalBundle, blocks: &[SymbolBlock]) -> Vec<RefactorHint> {
    let mut hints = Vec::new();

    if signals.effective_sloc > 0 && !blocks.is_empty() {
        let top = &blocks[0];
        // Share is the dominant block against the *file's* effective SLOC. Summed
        // block SLOC double-counts overlapping spans (a container plus its own
        // methods), which understates the share; file SLOC is the honest base.
        let share = (top.sloc as f32 / signals.effective_sloc as f32).min(1.0);
        if share >= 0.5 {
            hints.push(RefactorHint::DominatedBySingleSymbol {
                name: top.name.clone(),
                share,
            });
        }
    }

    if signals.top_level_symbol_count >= 10 {
        hints.push(RefactorHint::ManyUnrelatedTopLevel {
            count: signals.top_level_symbol_count,
        });
    }

    if signals.max_nesting_depth >= 5 {
        hints.push(RefactorHint::DeeplyNested {
            depth: signals.max_nesting_depth,
        });
    }

    if signals.import_fan_out >= 20 {
        hints.push(RefactorHint::HighCoupling {
            import_fan_out: signals.import_fan_out,
        });
    }

    if signals.comment_density >= 0.4 {
        hints.push(RefactorHint::LowCodeDensity {
            comment_density: signals.comment_density,
        });
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn candidate_discovery_excludes_sub_400_lines() {
        let dir = TempDir::new().unwrap();
        let small_content = "line\n".repeat(399);
        temp_file(&dir, "small.rs", &small_content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert!(
            candidates.is_empty(),
            "sub-400 line file should be excluded"
        );
    }

    #[test]
    fn candidate_discovery_includes_400_lines() {
        let dir = TempDir::new().unwrap();
        let medium_content = "line\n".repeat(400);
        temp_file(&dir, "medium.rs", &medium_content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert_eq!(candidates.len(), 1, "400-line file should be included");
        assert!(candidates[0].ends_with("medium.rs"));
    }

    #[test]
    fn candidate_discovery_is_deterministic() {
        let dir = TempDir::new().unwrap();
        let content = "line\n".repeat(500);
        temp_file(&dir, "zeta.rs", &content);
        temp_file(&dir, "alpha.rs", &content);
        temp_file(&dir, "beta.rs", &content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        let names: Vec<_> = candidates
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, vec!["alpha.rs", "beta.rs", "zeta.rs"]);
    }

    #[test]
    fn candidate_discovery_no_parse_required() {
        let dir = TempDir::new().unwrap();
        let bad_content = "not valid rust at all\n".repeat(400);
        temp_file(&dir, "invalid.rs", &bad_content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert_eq!(
            candidates.len(),
            1,
            "invalid syntax should not prevent discovery"
        );
    }

    #[test]
    fn candidate_discovery_filters_non_programming_files() {
        let dir = TempDir::new().unwrap();
        let content = "line\n".repeat(500);
        temp_file(&dir, "readme.md", &content);
        temp_file(&dir, "data.json", &content);
        temp_file(&dir, "script.rs", &content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].ends_with("script.rs"));
    }

    #[test]
    fn candidates_returns_stable_cached_reference() {
        // Repeated `candidates()` calls must hand back the same cached Vec, not
        // re-run the newline scan each time.
        let dir = TempDir::new().unwrap();
        temp_file(&dir, "big.rs", &"line\n".repeat(500));

        let god_files = GodFiles::new(dir.path());
        let first = god_files.candidates();
        let second = god_files.candidates();
        assert!(
            std::ptr::eq(first, second),
            "candidates() must return the same cached reference"
        );
    }

    #[test]
    fn analysis_returns_stable_cached_reference_and_populates_candidates() {
        let dir = TempDir::new().unwrap();
        temp_file(&dir, "big.py", &"x = 1\n".repeat(500));

        let god_files = GodFiles::new(dir.path());
        // `analysis()` populates `candidates()` implicitly: after analysis the
        // candidate set is available without a separate discovery pass.
        let first = god_files.analysis();
        assert_eq!(
            god_files.candidates().len(),
            1,
            "analysis() should have populated the candidate set"
        );
        let second = god_files.analysis();
        assert!(
            std::ptr::eq(first, second),
            "analysis() must return the same cached reference"
        );
    }

    #[test]
    fn candidate_discovery_screens_on_physical_not_effective_lines() {
        // Guard against discovery growing a parse-based screen. A 450-line file
        // that is *entirely* comments has 0 effective SLOC, so if discovery
        // parsed and filtered by effective SLOC (or symbol presence) it would be
        // excluded. It must still surface as a candidate (physical-line screen),
        // and only the parsing `analysis()` pass may drop it.
        let dir = TempDir::new().unwrap();
        let comment_only = "# comment line\n".repeat(450);
        temp_file(&dir, "comments_only.py", &comment_only);

        let god_files = GodFiles::new(dir.path());
        assert_eq!(
            god_files.candidates().len(),
            1,
            "comment-only file must be discovered by the physical-line screen"
        );
        assert!(
            god_files.analysis().is_empty(),
            "0 effective SLOC file must be dropped by the parsing analysis pass"
        );
    }

    #[test]
    fn candidate_discovery_empty_directory() {
        let dir = TempDir::new().unwrap();
        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidate_discovery_no_trailing_newline() {
        let dir = TempDir::new().unwrap();
        let mut content = "line\n".repeat(399);
        content.push_str("last line");
        temp_file(&dir, "edge.rs", &content);

        let god_files = GodFiles::new(dir.path());
        let candidates = god_files.candidates();
        assert_eq!(
            candidates.len(),
            1,
            "400 physical lines without trailing newline"
        );
    }

    #[test]
    fn physical_line_counter_counts_correctly() {
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "count.rs", "a\nb\nc");
        assert_eq!(count_physical_lines(&path).unwrap(), 3);

        let path2 = temp_file(&dir, "count2.rs", "a\nb\nc\n");
        assert_eq!(count_physical_lines(&path2).unwrap(), 3);

        let path3 = temp_file(&dir, "count3.rs", "");
        assert_eq!(count_physical_lines(&path3).unwrap(), 0);

        let path4 = temp_file(&dir, "count4.rs", "single");
        assert_eq!(count_physical_lines(&path4).unwrap(), 1);
    }

    // ========================================================================
    // Phase 4: Effective SLOC and band boundary tests
    // ========================================================================

    #[test]
    fn analysis_drops_sub_400_effective_sloc() {
        let dir = TempDir::new().unwrap();
        // 399 code lines + 1 blank line = 400 physical, 399 effective
        let mut content = String::new();
        for i in 0..399 {
            content.push_str(&format!("x{} = {}\n", i, i));
        }
        content.push('\n');
        temp_file(&dir, "small.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert!(analysis.is_empty(), "399 effective SLOC should be dropped");
    }

    #[test]
    fn analysis_classifies_400_as_moderate() {
        let dir = TempDir::new().unwrap();
        let content = "x = 1\n".repeat(400);
        temp_file(&dir, "medium.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert_eq!(analysis[0].risk, RiskBand::Moderate);
        assert_eq!(analysis[0].effective_sloc, 400);
    }

    #[test]
    fn analysis_classifies_999_as_moderate() {
        let dir = TempDir::new().unwrap();
        let content = "x = 1\n".repeat(999);
        temp_file(&dir, "medium_high.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert_eq!(analysis[0].risk, RiskBand::Moderate);
        assert_eq!(analysis[0].effective_sloc, 999);
    }

    #[test]
    fn analysis_classifies_1000_as_high() {
        let dir = TempDir::new().unwrap();
        let content = "x = 1\n".repeat(1000);
        temp_file(&dir, "high.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert_eq!(analysis[0].risk, RiskBand::High);
        assert_eq!(analysis[0].effective_sloc, 1000);
    }

    #[test]
    fn analysis_demotes_physical_high_to_effective_moderate() {
        let dir = TempDir::new().unwrap();
        // 1000 physical lines but 600 are comments → 400 effective (moderate)
        let mut content = String::new();
        for i in 0..400 {
            content.push_str(&format!("x{} = {}\n", i, i));
        }
        for _ in 0..600 {
            content.push_str("# comment line\n");
        }
        temp_file(&dir, "demoted.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert_eq!(analysis[0].risk, RiskBand::Moderate);
        assert_eq!(analysis[0].effective_sloc, 400);
    }

    #[test]
    fn analysis_drops_physical_moderate_effective_sub_400() {
        let dir = TempDir::new().unwrap();
        // 450 physical lines but 100 are comments and 50 blank → 300 effective (dropped)
        let mut content = String::new();
        for i in 0..300 {
            content.push_str(&format!("x{} = {}\n", i, i));
        }
        for _ in 0..50 {
            content.push('\n');
        }
        for _ in 0..100 {
            content.push_str("# comment line\n");
        }
        temp_file(&dir, "dropped.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert!(
            analysis.is_empty(),
            "300 effective SLOC should be dropped even with 450 physical"
        );
    }

    #[test]
    fn sloc_counts_blank_lines() {
        let dir = TempDir::new().unwrap();
        let mut content = String::new();
        for i in 0..300 {
            content.push_str(&format!("x{} = {}\n", i, i));
        }
        for _ in 0..100 {
            content.push('\n');
        }
        for _ in 0..100 {
            content.push_str("# comment\n");
        }
        let path = temp_file(&dir, "blank.py", &content);

        let source = fs::read_to_string(&path).unwrap();
        let tree_file = TreeFile::new(&path).unwrap();
        let metrics = compute_sloc_metrics(&source, Some(&tree_file));

        assert_eq!(metrics.physical_lines, 500);
        assert_eq!(metrics.blank_lines, 100);
        assert_eq!(metrics.comment_only_lines, 100);
        assert_eq!(metrics.effective_sloc, 300);
    }

    #[test]
    fn sloc_mixed_code_comment_counts_as_code() {
        let dir = TempDir::new().unwrap();
        // Line with code + trailing comment should count as effective
        let mut content = String::new();
        for i in 0..300 {
            content.push_str(&format!("x{} = {}  # comment\n", i, i));
        }
        let path = temp_file(&dir, "mixed.py", &content);

        let source = fs::read_to_string(&path).unwrap();
        let tree_file = TreeFile::new(&path).unwrap();
        let metrics = compute_sloc_metrics(&source, Some(&tree_file));

        assert_eq!(metrics.physical_lines, 300);
        assert_eq!(metrics.blank_lines, 0);
        assert_eq!(metrics.comment_only_lines, 0);
        assert_eq!(metrics.effective_sloc, 300);
    }

    #[test]
    fn analysis_sorts_high_before_moderate() {
        let dir = TempDir::new().unwrap();
        temp_file(&dir, "moderate.py", &"x = 1\n".repeat(400));
        temp_file(&dir, "high.py", &"x = 1\n".repeat(1000));

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 2);
        assert_eq!(analysis[0].risk, RiskBand::High);
        assert_eq!(analysis[1].risk, RiskBand::Moderate);
    }

    #[test]
    fn analysis_sorts_by_descending_sloc_within_band() {
        let dir = TempDir::new().unwrap();
        temp_file(&dir, "a.py", &"x = 1\n".repeat(500));
        temp_file(&dir, "b.py", &"x = 1\n".repeat(600));

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 2);
        assert_eq!(analysis[0].effective_sloc, 600);
        assert_eq!(analysis[1].effective_sloc, 500);
    }

    #[test]
    fn analysis_unparseable_fallback_uses_physical_lines() {
        let dir = TempDir::new().unwrap();
        let mut content = Vec::new();
        for _ in 0..600 {
            content.extend_from_slice(b"x = 1\n");
        }
        // Inject invalid UTF-8 so TreeFile::new fails with an Io error.
        content[3] = 0xFF;

        let path = dir.path().join("invalid.py");
        std::fs::write(&path, &content).unwrap();

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert_eq!(analysis[0].risk, RiskBand::Moderate);
        assert_eq!(analysis[0].effective_sloc, 600);
        // Unparseable candidates carry a diagnostic note and empty signals.
        assert!(
            analysis[0].note.is_some(),
            "unparseable candidate should carry a diagnostic note"
        );
        assert!(analysis[0].blocks.is_empty());
    }

    // ========================================================================
    // Phase 5: Structural signals, blocks, and refactor hints
    // ========================================================================

    #[test]
    fn block_ranking_sorts_descending_by_sloc() {
        let dir = TempDir::new().unwrap();
        let mut source = String::new();
        source.push_str("def small():\n");
        for i in 0..15 {
            source.push_str(&format!("    x{} = {}\n", i, i));
        }
        source.push_str("def big():\n");
        for i in 0..30 {
            source.push_str(&format!("    y{} = {}\n", i, i));
        }
        let path = temp_file(&dir, "rank.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        let names: Vec<_> = result.blocks.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"small"), "missing small; got {:?}", names);
        assert!(names.contains(&"big"), "missing big; got {:?}", names);
        assert_eq!(result.blocks[0].name, "big");
        assert_eq!(result.blocks[1].name, "small");
    }

    #[test]
    fn block_floor_excludes_tiny_symbols() {
        let dir = TempDir::new().unwrap();
        let mut source = String::new();
        source.push_str("def tiny():\n    pass\n");
        source.push_str("def big():\n");
        for i in 0..30 {
            source.push_str(&format!("    y{} = {}\n", i, i));
        }
        let path = temp_file(&dir, "floor.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].name, "big");
        assert_eq!(result.truncated, 0);
    }

    #[test]
    fn block_cap_truncates_at_max_blocks() {
        let dir = TempDir::new().unwrap();
        let mut source = String::new();
        source.push('\n');
        for i in 0..12 {
            source.push_str(&format!("def fn{}():\n", i));
            for _ in 0..20 {
                source.push_str("    x = 1\n");
            }
        }
        let path = temp_file(&dir, "cap.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);
        assert_eq!(result.blocks.len(), MAX_BLOCKS);
        assert_eq!(result.truncated, 12 - MAX_BLOCKS);
    }

    #[test]
    fn block_sloc_excludes_comment_only_lines() {
        // A documented block with mostly comment lines must rank below a
        // code-heavy block, and its SLOC must reflect effective lines only.
        let dir = TempDir::new().unwrap();
        let mut source = String::new();
        source.push_str("def documented():\n");
        for _ in 0..30 {
            source.push_str("    # explanatory comment line\n");
        }
        for i in 0..5 {
            source.push_str(&format!("    x{} = {}\n", i, i));
        }
        source.push_str("def lean():\n");
        for i in 0..20 {
            source.push_str(&format!("    y{} = {}\n", i, i));
        }
        let path = temp_file(&dir, "comments.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        // `documented` has 36 physical lines but only ~5 effective code lines.
        // With comment-only lines excluded it falls below MIN_BLOCK_SLOC and is
        // dropped; under the old snippet-reparse path it would have counted the
        // comments as code (~35 SLOC) and ranked at the top.
        assert!(
            !result.blocks.iter().any(|b| b.name == "documented"),
            "comment-heavy block should fall below the SLOC floor, got {:?}",
            result
                .blocks
                .iter()
                .map(|b| (&b.name, b.sloc))
                .collect::<Vec<_>>()
        );
        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].name, "lean");
    }

    #[test]
    fn many_members_excludes_variables_fields_parameters() {
        let source = r#"
class BigClass:
    def method_a(self):
        a = 1
        b = 2
        c = 3
        d = 4
        e = 5
        f = 6
        g = 7
        h = 8
        i = 9
        j = 10
        k = 11
        l = 12
        m = 13
        n = 14
        o = 15

    field_a = 1
    field_b = 2
    field_c = 3
    field_d = 4
    field_e = 5
    field_f = 6
    field_g = 7
    field_h = 8
    field_i = 9
    field_j = 10
"#;
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "bigclass.py", source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let class = records
            .iter()
            .find(|r| r.identity.name == "BigClass")
            .expect("class found");
        let class_span = record_span_bytes(class).expect("class span");

        let structural_members: Vec<_> = records
            .iter()
            .filter(|m| {
                let Some(m_span) = record_span_bytes(m) else {
                    return false;
                };
                m_span.0 > class_span.0
                    && m_span.1 < class_span.1
                    && !is_non_structural_kind(SymbolKind::from(m.kind))
            })
            .collect();

        // Only method_a should be a structural member; fields are excluded.
        assert_eq!(structural_members.len(), 1);
        assert_eq!(structural_members[0].identity.name, "method_a");
    }

    /// Find the callout attached to the first block matching `name`.
    fn callout_for<'a>(blocks: &'a [SymbolBlock], name: &str) -> Option<&'a ContainerCallout> {
        blocks
            .iter()
            .find(|b| b.name == name)
            .and_then(|b| b.many_members.as_ref())
    }

    #[test]
    fn many_members_counts_direct_members_not_descendants() {
        // `Outer` has only two direct members (one method and a nested class),
        // while `Inner` has eleven methods. Counting descendants would push
        // `Outer` over the threshold; only direct ownership must be counted.
        let mut source = String::from("class Outer:\n");
        source.push_str("    def only_method(self):\n");
        for i in 0..16 {
            source.push_str(&format!("        x{i} = {i}\n"));
        }
        source.push_str("    class Inner:\n");
        for i in 0..11 {
            source.push_str(&format!(
                "        def m{i}(self):\n            return {i}\n"
            ));
        }
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "nested.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        assert!(
            callout_for(&result.blocks, "Outer").is_none(),
            "Outer has 2 direct members; descendants must not inflate it"
        );
        let inner = callout_for(&result.blocks, "Inner").expect("Inner callout present");
        assert_eq!(inner.member_count, 11);
    }

    #[test]
    fn many_members_binds_duplicate_container_names_independently() {
        // Two top-level `class Dup` definitions: the first owns eleven methods,
        // the second owns two. Binding by `(name, kind)` alone would give both
        // blocks the first definition's member list; binding by span keeps them
        // independent.
        let mut source = String::from("class Dup:\n");
        for i in 0..11 {
            source.push_str(&format!("    def a{i}(self):\n        return {i}\n"));
        }
        source.push_str("class Dup:\n");
        for i in 0..2 {
            source.push_str(&format!("    def b{i}(self):\n"));
            for j in 0..10 {
                source.push_str(&format!("        y{j} = {j}\n"));
            }
        }
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "dup.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        let dup_callouts: Vec<usize> = result
            .blocks
            .iter()
            .filter(|b| b.name == "Dup")
            .map(|b| b.many_members.as_ref().map_or(0, |c| c.member_count))
            .collect();
        assert_eq!(
            dup_callouts.len(),
            2,
            "both Dup definitions should surface as blocks, got {dup_callouts:?}"
        );
        // Exactly one definition crosses the threshold (the 11-member one); the
        // 2-member definition must not inherit the first's member list.
        assert!(
            dup_callouts.contains(&11),
            "the 11-member Dup must be called out, got {dup_callouts:?}"
        );
        assert!(
            dup_callouts.contains(&0),
            "the 2-member Dup must not be called out, got {dup_callouts:?}"
        );
    }

    #[test]
    fn histogram_is_deterministic() {
        let dir = TempDir::new().unwrap();
        let mut content = String::new();
        for i in 0..10 {
            content.push_str(&format!("def f{}():\n    pass\n", i));
        }
        for i in 0..5 {
            content.push_str(&format!("class C{}:\n    pass\n", i));
        }
        // Pad to 400 effective lines
        while content.lines().count() < 450 {
            content.push_str("x = 1\n");
        }
        temp_file(&dir, "hist.py", &content);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        let histogram = &analysis[0].kind_histogram;
        let keys: Vec<_> = histogram.0.keys().copied().collect();
        // Deterministic ordering: BTreeMap keys sorted by SymbolKind Ord.
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn top_level_excludes_locals_and_parameters() {
        // Public-boundary reproduction: ten top-level functions, each with one
        // parameter and many local assignments. The locals and parameters have
        // no recorded container, so the old `module_path.is_none()` test
        // counted all of them as top-level (hundreds of phantom Variables and
        // Parameters) and emitted a wildly inflated ManyUnrelatedTopLevel hint.
        let dir = TempDir::new().unwrap();
        let mut source = String::new();
        for f in 0..10 {
            source.push_str(&format!("def f{f}(arg):\n"));
            for v in 0..44 {
                source.push_str(&format!("    local_{f}_{v} = arg + {v}\n"));
            }
        }
        temp_file(&dir, "functions.py", &source);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1, "file should report as a god file");
        let report = &analysis[0];

        assert_eq!(
            report.top_level_symbol_count, 10,
            "only the ten functions are top-level; locals and parameters are not"
        );
        let kinds: Vec<_> = report.kind_histogram.0.keys().copied().collect();
        assert_eq!(
            kinds,
            vec![SymbolKind::Function],
            "histogram must contain only the top-level functions, got {:?}",
            report.kind_histogram.0
        );
        assert!(
            report
                .refactor_hints
                .iter()
                .any(|h| matches!(h, RefactorHint::ManyUnrelatedTopLevel { count: 10 })),
            "hint count must be the ten functions, got {:?}",
            report.refactor_hints
        );
    }

    #[test]
    fn top_level_keeps_file_scope_variables() {
        // File-scope (module-level) variables have no enclosing symbol and must
        // remain counted as top-level alongside functions.
        let mut source = String::new();
        for i in 0..12 {
            source.push_str(&format!("CONST_{i} = {i}\n"));
        }
        source.push_str("def helper(x):\n    return x\n");
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "globals.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let top_level = collect_top_level(&records);

        let var_count = top_level
            .iter()
            .filter(|r| SymbolKind::from(r.kind) == SymbolKind::Variable)
            .count();
        assert_eq!(
            var_count, 12,
            "all twelve file-scope variables are top-level"
        );
        assert!(
            top_level.iter().any(|r| r.identity.name == "helper"),
            "the top-level function must remain counted"
        );
        // `helper`'s parameter and any function-body locals are enclosed.
        assert!(
            !top_level.iter().any(|r| r.identity.name == "x"),
            "the function parameter must be excluded"
        );
    }

    #[test]
    fn todo_fixme_count_in_comments() {
        let source = r#"
# TODO: fix this
x = 1
# FIXME: also this
# normal comment
# HACK: workaround
# XXX: legacy
"#;
        let comment_ranges = vec![(1, 17), (20, 38), (39, 55), (56, 76), (77, 93)];
        assert_eq!(count_todo_fixme(source, &comment_ranges), 4);
    }

    #[test]
    fn todo_fixme_counts_every_marker_in_one_comment() {
        // A single block comment carrying several distinct markers plus a
        // repeat must count every occurrence (case-insensitive), not collapse
        // to one per comment node.
        let source = "/* TODO refactor; fixme the leak; HACK retry; xxx legacy; todo again */";
        let comment_ranges = vec![(0, source.len())];
        // TODO/todo ×2, FIXME ×1, HACK ×1, XXX ×1 = 5
        assert_eq!(count_todo_fixme(source, &comment_ranges), 5);
    }

    #[test]
    fn comment_density_clamped() {
        let source = "# comment\n";
        let ranges = vec![(0, 10)];
        assert_eq!(compute_comment_density(1, source, &ranges), 1.0);
        assert_eq!(compute_comment_density(0, source, &ranges), 0.0);
    }

    #[test]
    fn refactor_hint_dominated_by_single_symbol() {
        let blocks = vec![
            SymbolBlock {
                name: "big".into(),
                kind: SymbolKind::Function,
                start_line: 1,
                end_line: 100,
                sloc: 80,
                doc_summary: None,
                many_members: None,
            },
            SymbolBlock {
                name: "small".into(),
                kind: SymbolKind::Function,
                start_line: 101,
                end_line: 120,
                sloc: 20,
                doc_summary: None,
                many_members: None,
            },
        ];
        let signals = SignalBundle {
            effective_sloc: 100,
            top_level_symbol_count: 2,
            import_fan_out: 0,
            max_nesting_depth: 1,
            comment_density: 0.0,
        };
        let hints = synthesize_hints(&signals, &blocks);
        assert!(hints.iter().any(
            |h| matches!(h, RefactorHint::DominatedBySingleSymbol { name, .. } if name == "big")
        ));
    }

    #[test]
    fn dominated_share_uses_file_sloc_not_overlapping_blocks() {
        // A container block plus its nested method blocks overlap, so the summed
        // block SLOC (180 + 30 + 30 = 240) exceeds the file's effective SLOC
        // (200). Dividing by the sum would understate the share (180/240 = 0.75);
        // it must divide by file SLOC (180/200 = 0.90).
        let blocks = vec![
            SymbolBlock {
                name: "Container".into(),
                kind: SymbolKind::Class,
                start_line: 1,
                end_line: 200,
                sloc: 180,
                doc_summary: None,
                many_members: None,
            },
            SymbolBlock {
                name: "m1".into(),
                kind: SymbolKind::Method,
                start_line: 5,
                end_line: 60,
                sloc: 30,
                doc_summary: None,
                many_members: None,
            },
            SymbolBlock {
                name: "m2".into(),
                kind: SymbolKind::Method,
                start_line: 61,
                end_line: 120,
                sloc: 30,
                doc_summary: None,
                many_members: None,
            },
        ];
        let signals = SignalBundle {
            effective_sloc: 200,
            top_level_symbol_count: 1,
            import_fan_out: 0,
            max_nesting_depth: 2,
            comment_density: 0.0,
        };
        let hints = synthesize_hints(&signals, &blocks);
        let share = hints
            .iter()
            .find_map(|h| match h {
                RefactorHint::DominatedBySingleSymbol { share, .. } => Some(*share),
                _ => None,
            })
            .expect("dominance hint present");
        assert!(
            (share - 0.9).abs() < 1e-6,
            "share should be top.sloc / file effective SLOC (0.90), got {share}"
        );
    }

    #[test]
    fn refactor_hint_many_unrelated_top_level() {
        let signals = SignalBundle {
            effective_sloc: 500,
            top_level_symbol_count: 12,
            import_fan_out: 0,
            max_nesting_depth: 1,
            comment_density: 0.0,
        };
        let hints = synthesize_hints(&signals, &[]);
        assert!(
            hints
                .iter()
                .any(|h| matches!(h, RefactorHint::ManyUnrelatedTopLevel { count: 12 }))
        );
    }

    #[test]
    fn refactor_hint_deeply_nested() {
        let signals = SignalBundle {
            effective_sloc: 500,
            top_level_symbol_count: 1,
            import_fan_out: 0,
            max_nesting_depth: 6,
            comment_density: 0.0,
        };
        let hints = synthesize_hints(&signals, &[]);
        assert!(
            hints
                .iter()
                .any(|h| matches!(h, RefactorHint::DeeplyNested { depth: 6 }))
        );
    }

    #[test]
    fn refactor_hint_high_coupling() {
        let signals = SignalBundle {
            effective_sloc: 500,
            top_level_symbol_count: 1,
            import_fan_out: 25,
            max_nesting_depth: 1,
            comment_density: 0.0,
        };
        let hints = synthesize_hints(&signals, &[]);
        assert!(
            hints
                .iter()
                .any(|h| matches!(h, RefactorHint::HighCoupling { import_fan_out: 25 }))
        );
    }

    #[test]
    fn refactor_hint_low_code_density() {
        let signals = SignalBundle {
            effective_sloc: 500,
            top_level_symbol_count: 1,
            import_fan_out: 0,
            max_nesting_depth: 1,
            comment_density: 0.5,
        };
        let hints = synthesize_hints(&signals, &[]);
        assert!(hints.iter().any(|h| matches!(
            h,
            RefactorHint::LowCodeDensity {
                comment_density: 0.5
            }
        )));
    }

    /// `(extension, source, expected_depth)`. Every supported language is
    /// exercised with its grammar-specific loop, conditional, match, and
    /// exception forms so the per-language node-kind sets in
    /// `is_control_flow_nesting` cannot silently lose a construct. Each `deep`
    /// case nests distinct constructs; each `flat`/sequential case keeps them
    /// side by side so nesting must not accumulate. Function and class scopes do
    /// not count — only branching/looping/match/exception constructs do.
    const NESTING_DEPTH_CASES: &[(&str, &str, usize)] = &[
        // Python: one function, eight nested `if` blocks → depth 8.
        (
            "py",
            "def f(x):\n    if x > 0:\n        if x > 1:\n            if x > 2:\n                if x > 3:\n                    if x > 4:\n                        if x > 5:\n                            if x > 6:\n                                if x > 7:\n                                    return x\n",
            8,
        ),
        // Python: eight *sequential* ifs in one function → depth 1.
        (
            "py",
            "def f(x):\n    if x == 0:\n        return 0\n    if x == 1:\n        return 1\n    if x == 2:\n        return 2\n    if x == 3:\n        return 3\n",
            1,
        ),
        // Python: with / for / match / while / try → depth 5.
        (
            "py",
            "def f(x):\n    with a() as b:\n        for i in x:\n            match i:\n                case 0:\n                    while i:\n                        try:\n                            pass\n                        except Exception:\n                            pass\n",
            5,
        ),
        // Rust: nested for / if / while inside a function → depth 3.
        (
            "rs",
            "fn f(n: i32) {\n    for i in 0..n {\n        if i > 0 {\n            while i < n {\n                break;\n            }\n        }\n    }\n}\n",
            3,
        ),
        // Rust: a single function with no control flow → depth 0.
        ("rs", "fn f() {\n    let x = 1;\n    let y = 2;\n}\n", 0),
        // Rust: match arm body holds a loop holding an if → depth 3.
        (
            "rs",
            "fn f(x: i32) {\n    match x {\n        _ => {\n            loop {\n                if x > 0 {}\n            }\n        }\n    }\n}\n",
            3,
        ),
        // Rust: the `?` operator is `try_expression`, not a nesting block → 0.
        (
            "rs",
            "fn f() -> Result<(), ()> {\n    let _ = g()?;\n    Ok(())\n}\n",
            0,
        ),
        // Rust: a `try { }` block (`try_block`) holding an if → depth 2.
        (
            "rs",
            "fn f() {\n    let _: Result<(), ()> = try {\n        if true {}\n    };\n}\n",
            2,
        ),
        // JavaScript: nested for / if / try → depth 3.
        (
            "js",
            "function f(items) {\n  for (const i of items) {\n    if (i) {\n      try {\n        use(i);\n      } catch (e) {}\n    }\n  }\n}\n",
            3,
        ),
        // JavaScript: do / with / if → depth 3.
        (
            "js",
            "function f(o, x) {\n  do {\n    with (o) {\n      if (x) {}\n    }\n  } while (false);\n}\n",
            3,
        ),
        // TypeScript: for-of / if / while → depth 3.
        (
            "ts",
            "function f(items: number[]) {\n  for (const i of items) {\n    if (i) {\n      while (i) {}\n    }\n  }\n}\n",
            3,
        ),
        // TypeScript: switch / do-while → depth 2.
        (
            "ts",
            "function f(n: number) {\n  switch (n) {\n    case 1:\n      do {} while (n > 0);\n  }\n}\n",
            2,
        ),
        // Go: for / if / expression-switch → depth 3.
        (
            "go",
            "func f(n int) {\n\tfor i := 0; i < n; i++ {\n\t\tif i > 0 {\n\t\t\tswitch i {\n\t\t\tcase 1:\n\t\t\t}\n\t\t}\n\t}\n}\n",
            3,
        ),
        // Go: select / type-switch → depth 2.
        (
            "go",
            "func f(ch chan int, x interface{}) {\n\tselect {\n\tcase <-ch:\n\t\tswitch x.(type) {\n\t\tcase int:\n\t\t}\n\t}\n}\n",
            2,
        ),
        // Java: try-with-resources / enhanced-for / if → depth 3.
        (
            "java",
            "class C {\n  void f(java.util.List<String> xs) throws Exception {\n    try (var r = open()) {\n      for (var x : xs) {\n        if (x != null) {}\n      }\n    }\n  }\n}\n",
            3,
        ),
        // Java: switch / while / do → depth 3.
        (
            "java",
            "class C {\n  void f(int n) {\n    switch (n) {\n      case 1:\n        while (n > 0) {\n          do {} while (n > 0);\n        }\n    }\n  }\n}\n",
            3,
        ),
        // PHP: foreach / if / while → depth 3.
        (
            "php",
            "<?php\nfunction f($items) {\n  foreach ($items as $i) {\n    if ($i) {\n      while ($i) {}\n    }\n  }\n}\n",
            3,
        ),
        // PHP: switch / match-expression → depth 2.
        (
            "php",
            "<?php\nfunction f($x) {\n  switch ($x) {\n    case 1:\n      $y = match($x) { 1 => 1, default => 2 };\n      break;\n  }\n}\n",
            2,
        ),
        // Perl: block if / unless / while → depth 3.
        (
            "pl",
            "if ($x) {\n  unless ($y) {\n    while ($z) {}\n  }\n}\n",
            3,
        ),
        // Perl: until block holding a foreach (`for_statement_2`) → depth 2.
        ("pl", "until ($done) {\n  for my $i (@list) {}\n}\n", 2),
        // Perl: C-style for (`for_statement_1`) holding an if → depth 2.
        (
            "pl",
            "for (my $i = 0; $i < 10; $i++) {\n  if ($i) {}\n}\n",
            2,
        ),
        // Perl: postfix statement modifier inside a block → depth 2.
        ("pl", "if ($x) {\n  print 1 unless $y;\n}\n", 2),
        // Perl: a lone postfix `if` modifier (`if_simple_statement`) → depth 1.
        ("pl", "print 1 if $c;\n", 1),
        // Bash: if / for / while → depth 3.
        (
            "sh",
            "if true; then\n  for x in a b; do\n    while true; do :; done\n  done\nfi\n",
            3,
        ),
        // Bash: case holding a C-style for → depth 2.
        (
            "sh",
            "case $x in\n  a)\n    for ((i=0;i<3;i++)); do :; done\n  ;;\nesac\n",
            2,
        ),
        // Zsh: for / if / while → depth 3.
        (
            "zsh",
            "for x in a b; do\n  if true; then\n    while true; do :; done\n  fi\ndone\n",
            3,
        ),
        // Zsh: repeat holding a case → depth 2.
        (
            "zsh",
            "repeat 3; do\n  case $x in\n    a) :;;\n  esac\ndone\n",
            2,
        ),
        // Zsh: select holding an if → depth 2.
        (
            "zsh",
            "select x in a b; do\n  if true; then :; fi\ndone\n",
            2,
        ),
        // C: for / if / while → depth 3.
        (
            "c",
            "void f(int n) {\n  for (int i = 0; i < n; i++) {\n    if (i) {\n      while (i) {}\n    }\n  }\n}\n",
            3,
        ),
        // C: switch holding a do-while → depth 2.
        (
            "c",
            "void f(int n) {\n  switch (n) {\n    case 1:\n      do {} while (0);\n  }\n}\n",
            2,
        ),
        // C++: range-for / if / while → depth 3.
        (
            "cpp",
            "void f(std::vector<int> v) {\n  for (auto x : v) {\n    if (x) {\n      while (x) {}\n    }\n  }\n}\n",
            3,
        ),
        // C++: try holding a switch → depth 2.
        (
            "cpp",
            "void f(int n) {\n  try {\n    switch (n) {\n      case 1:\n        break;\n    }\n  } catch (...) {}\n}\n",
            2,
        ),
        // C#: foreach / if / while → depth 3.
        (
            "cs",
            "class C {\n  void F(int[] xs) {\n    foreach (var x in xs) {\n      if (x > 0) {\n        while (x > 0) {}\n      }\n    }\n  }\n}\n",
            3,
        ),
        // C#: try holding a switch statement → depth 2.
        (
            "cs",
            "class C {\n  void F(int n) {\n    try {\n      switch (n) {\n        case 1:\n          break;\n      }\n    } catch {}\n  }\n}\n",
            2,
        ),
        // Swift: for / if / while → depth 3.
        (
            "swift",
            "func f(items: [Int]) {\n  for i in items {\n    if i > 0 {\n      while i > 0 {}\n    }\n  }\n}\n",
            3,
        ),
        // Swift: repeat-while holding a switch → depth 2.
        (
            "swift",
            "func f(n: Int) {\n  repeat {\n    switch n {\n    case 1: break\n    default: break\n    }\n  } while n > 0\n}\n",
            2,
        ),
        // Swift: guard else-block holding a for → depth 2.
        (
            "swift",
            "func f(x: Int?) {\n  guard let y = x else {\n    for _ in 0..<1 {}\n    return\n  }\n  _ = y\n}\n",
            2,
        ),
        // Scala: for / if / while → depth 3.
        (
            "scala",
            "object O {\n  def f(items: List[Int]): Unit = {\n    for (i <- items) {\n      if (i > 0) {\n        while (i > 0) {}\n      }\n    }\n  }\n}\n",
            3,
        ),
        // Scala: match arm holding a try (`try_expression` here is a real block) → depth 2.
        (
            "scala",
            "object O {\n  def f(x: Int): Unit = {\n    x match {\n      case _ =>\n        try { g() } catch { case e: Exception => () }\n    }\n  }\n}\n",
            2,
        ),
        // Scala: do-while holding an if → depth 2.
        (
            "scala",
            "object O {\n  def f(): Unit = {\n    var n = 0\n    do { if (n > 0) {} } while (n < 3)\n  }\n}\n",
            2,
        ),
        // Lua: for / if / while → depth 3.
        (
            "lua",
            "function f(items)\n  for _, i in ipairs(items) do\n    if i then\n      while i do end\n    end\n  end\nend\n",
            3,
        ),
        // Lua: repeat holding an if → depth 2.
        (
            "lua",
            "function f()\n  repeat\n    if x then end\n  until done\nend\n",
            2,
        ),
    ];

    #[test]
    fn nesting_depth_counts_control_flow_not_symbols() {
        let dir = TempDir::new().unwrap();
        for (ext, source, expected_depth) in NESTING_DEPTH_CASES {
            let path = temp_file(&dir, &format!("nest.{ext}"), source);
            let tree_file =
                TreeFile::new(&path).unwrap_or_else(|e| panic!("{ext}: TreeFile::new failed: {e}"));
            let depth = compute_max_nesting_depth(&tree_file);
            assert_eq!(
                depth, *expected_depth,
                "{ext}: control-flow nesting depth mismatch for source:\n{source}"
            );
        }
    }

    #[test]
    fn deeply_nested_control_flow_emits_hint() {
        // Public-boundary reproduction: a function with eight nested `if`
        // blocks must surface `DeeplyNested`. Symbol-span nesting reported a
        // trivial depth here and emitted no hint.
        let dir = TempDir::new().unwrap();
        let mut source = String::from("def handler(value):\n");
        for level in 0..8 {
            let indent = "    ".repeat(level + 1);
            source.push_str(&format!("{indent}if value > {level}:\n"));
        }
        let deepest = "    ".repeat(9);
        source.push_str(&format!("{deepest}return value\n"));
        // Pad to a reportable god file (>= 400 effective SLOC).
        for i in 0..420 {
            source.push_str(&format!("x{i} = {i}\n"));
        }
        temp_file(&dir, "deep.py", &source);

        let god_files = GodFiles::new(dir.path());
        let analysis = god_files.analysis();
        assert_eq!(analysis.len(), 1);
        assert!(
            analysis[0].max_nesting_depth >= 8,
            "expected control-flow depth >= 8, got {}",
            analysis[0].max_nesting_depth
        );
        assert!(
            analysis[0]
                .refactor_hints
                .iter()
                .any(|h| matches!(h, RefactorHint::DeeplyNested { .. })),
            "deeply nested control flow must emit a DeeplyNested hint"
        );
    }

    /// Synthetic flat record with a disjoint declaration span; nothing nests.
    fn flat_record(start_byte: u32) -> SymbolRecord {
        use crate::shared::{
            AnalysisPass, DocsFacet, IdentityFacet, ProvenanceFacet, SchemaVersion, SemanticFacet,
            SourceFacet, SymbolId, SymbolKindData, TextPoint, TextSpan, TypeFacet, VisibilityFacet,
        };
        SymbolRecord {
            schema_version: SchemaVersion::V2_0,
            id: SymbolId(String::new()),
            language: ProgrammingLanguage::Rust,
            kind: SymbolKindV2::Function,
            identity: IdentityFacet {
                name: String::new(),
                display_name: String::new(),
                qualified_name: None,
                module_path: None,
                stable_key: String::new(),
            },
            source: SourceFacet {
                declaration_span: TextSpan {
                    start: TextPoint::default(),
                    end: TextPoint::default(),
                    start_byte,
                    end_byte: start_byte + 5,
                },
                ..SourceFacet::default()
            },
            visibility: VisibilityFacet::default(),
            docs: DocsFacet::default(),
            attributes: Vec::new(),
            modifiers: Vec::new(),
            type_info: TypeFacet::default(),
            relations: Default::default(),
            semantics: SemanticFacet::default(),
            provenance: ProvenanceFacet {
                extractor: String::new(),
                extractor_version: String::new(),
                parse_pass: AnalysisPass::Parse.as_str().to_string(),
                created_at_epoch_ms: 0,
                updated_at_epoch_ms: 0,
            },
            kind_data: SymbolKindData::Unknown,
            extensions: Default::default(),
        }
    }

    /// Performance regression for review-7: span-containment processing must
    /// stay near-linear. `compute_enclosed` (which decides top-level ownership)
    /// is the remaining symbol-span sweep — a naive containment check is
    /// `O(symbols^2)` (each span compared against every prior span), so a flat
    /// file with tens of thousands of symbols — exactly the large files this
    /// command targets — would stall for over a minute. The stack sweep is
    /// `O(n log n)`.
    ///
    /// This is the bounded CI guard the review calls for: the ceiling is
    /// calibrated to the symbol count, so a reintroduced quadratic path blows
    /// past it without parsing a giant fixture. A wall-clock *ratio* across
    /// sizes is too noisy under parallel test execution to assert here — track
    /// the precise slope with a Criterion benchmark instead.
    #[test]
    fn enclosure_stays_near_linear_on_flat_symbols() {
        use std::time::{Duration, Instant};

        let flat: Vec<SymbolRecord> = (0..50_000).map(|i| flat_record(i * 10)).collect();

        // Min over repeats: robust to scheduler noise under parallel test runs.
        let mut best = Duration::MAX;
        for _ in 0..3 {
            let start = Instant::now();
            let enclosed = compute_enclosed(&flat);
            best = best.min(start.elapsed());
            assert!(
                enclosed.iter().all(|&e| !e),
                "disjoint flat spans must enclose nothing"
            );
        }

        assert!(
            best < Duration::from_secs(2),
            "50k flat symbols took {best:?}; expected sub-second (quadratic regression?)"
        );
    }

    // ========================================================================
    // Per-language comment classification (Level 1)
    //
    // Every supported language must subtract comment-only lines from effective
    // SLOC and keep mixed code+comment lines as code. The regression that
    // motivated these: Perl's comments query was empty, so `# comment` lines
    // were counted as code and inflated effective SLOC.
    // ========================================================================

    /// `(extension, source, expected_comment_only, expected_effective)`. Each
    /// source carries comment-only lines and at least one line mixing code with
    /// a trailing comment, which must count as code.
    const COMMENT_CLASSIFICATION_CASES: &[(&str, &str, usize, usize)] = &[
        (
            "rs",
            "// alpha\n// beta\nfn helper() {}\nconst VALUE: i32 = 1; // trailing\n",
            2,
            2,
        ),
        (
            "go",
            "// alpha\n// beta\npackage main\nvar value = 1 // trailing\n",
            2,
            2,
        ),
        ("py", "# alpha\n# beta\nx = 1\ny = 2  # trailing\n", 2, 2),
        (
            "pl",
            "# alpha\n# beta\nmy $x = 1;\nmy $y = 2; # trailing\n",
            2,
            2,
        ),
        ("sh", "# alpha\n# beta\nx=1\ny=2 # trailing\n", 2, 2),
        ("zsh", "# alpha\n# beta\nx=1\ny=2 # trailing\n", 2, 2),
        (
            "lua",
            "-- alpha\n-- beta\nlocal x = 1\nlocal y = 2 -- trailing\n",
            2,
            2,
        ),
        (
            "c",
            "// alpha\n// beta\nint helper(void) { return 0; }\nint value = 1; // trailing\n",
            2,
            2,
        ),
        (
            "cpp",
            "// alpha\n// beta\nint helper(void) { return 0; }\nint value = 1; // trailing\n",
            2,
            2,
        ),
        (
            "cs",
            "// alpha\n// beta\nint x = 1;\nint y = 2; // trailing\n",
            2,
            2,
        ),
        (
            "java",
            "// alpha\n// beta\nclass Helper {}\nint value = 1; // trailing\n",
            2,
            2,
        ),
        (
            "js",
            "// alpha\n// beta\nlet x = 1;\nlet y = 2; // trailing\n",
            2,
            2,
        ),
        (
            "ts",
            "// alpha\n// beta\nlet x = 1;\nlet y = 2; // trailing\n",
            2,
            2,
        ),
        (
            "scala",
            "// alpha\n// beta\nval x = 1\nval y = 2 // trailing\n",
            2,
            2,
        ),
        (
            "swift",
            "// alpha\n// beta\nlet x = 1\nlet y = 2 // trailing\n",
            2,
            2,
        ),
        // PHP comments only parse inside `<?php`; the opening tag is one code line.
        (
            "php",
            "<?php\n// alpha\n// beta\n$x = 1;\n$y = 2; // trailing\n",
            2,
            3,
        ),
    ];

    #[test]
    fn comment_only_lines_excluded_for_every_language() {
        let dir = TempDir::new().unwrap();
        for (ext, source, expected_comment_only, expected_effective) in COMMENT_CLASSIFICATION_CASES
        {
            let path = temp_file(&dir, &format!("case.{ext}"), source);
            let tree_file =
                TreeFile::new(&path).unwrap_or_else(|e| panic!("{ext}: TreeFile::new failed: {e}"));
            let metrics = compute_sloc_metrics(tree_file.source(), Some(&tree_file));

            assert_eq!(
                metrics.comment_only_lines, *expected_comment_only,
                "{ext}: comment-only lines miscounted (effective={}, physical={})",
                metrics.effective_sloc, metrics.physical_lines
            );
            assert_eq!(
                metrics.effective_sloc, *expected_effective,
                "{ext}: effective SLOC miscounted (comment_only={}, blank={})",
                metrics.comment_only_lines, metrics.blank_lines
            );
        }
    }

    #[test]
    fn perl_comment_only_file_is_dropped() {
        // Public-boundary reproduction of the Perl false positive: a 450-line
        // file of `# comment` lines has zero effective SLOC and must not be
        // reported. Before the comments query fix, every line counted as code
        // and surfaced a spurious moderate-risk god file.
        let dir = TempDir::new().unwrap();
        let comment_only = "# comment line\n".repeat(450);
        temp_file(&dir, "comments_only.pl", &comment_only);

        let god_files = GodFiles::new(dir.path());
        assert!(
            god_files.analysis().is_empty(),
            "comment-only Perl file must be dropped (0 effective SLOC)"
        );
    }

    #[test]
    fn perl_pod_only_file_is_dropped() {
        // Perl POD is documentation, not code, but the grammar parses it as a
        // separate `pod_statement` rather than a hash comment. A 450-line POD
        // block must contribute zero effective SLOC and be dropped, not surface
        // as a spurious moderate-risk god file.
        let dir = TempDir::new().unwrap();
        let mut content = String::from("=pod\n\n");
        for i in 0..450 {
            content.push_str(&format!("Documentation paragraph line {i}.\n\n"));
        }
        content.push_str("=cut\n");
        temp_file(&dir, "pod_only.pl", &content);

        let god_files = GodFiles::new(dir.path());
        assert!(
            god_files.analysis().is_empty(),
            "POD-only Perl file must be dropped (0 effective SLOC)"
        );
    }

    #[test]
    fn perl_pod_excluded_from_effective_sloc() {
        // POD documentation lines are comment-only; the surrounding `my` lines
        // are the only effective code.
        let source = "my $x = 1;\n=pod\n\nDocumentation.\nMore docs.\n\n=cut\nmy $y = 2;\n";
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "mixed_pod.pl", source);

        let tree_file = TreeFile::new(&path).unwrap();
        let metrics = compute_sloc_metrics(tree_file.source(), Some(&tree_file));
        assert_eq!(
            metrics.effective_sloc, 2,
            "only the two `my` lines are code (comment_only={}, blank={})",
            metrics.comment_only_lines, metrics.blank_lines
        );
    }

    #[test]
    fn perl_pod_todo_markers_are_counted() {
        // Debt markers inside POD documentation are still debt signals, so the
        // POD range must reach `count_todo_fixme`.
        let source = "=pod\n\nTODO document this. FIXME handle the edge case.\n\n=cut\n";
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "pod_markers.pl", source);

        let tree_file = TreeFile::new(&path).unwrap();
        let ranges = tree_file.comment_ranges().unwrap();
        assert_eq!(
            count_todo_fixme(tree_file.source(), &ranges),
            2,
            "TODO and FIXME inside POD must both count"
        );
    }

    // ========================================================================
    // Declaration-span block boundaries (Level 1)
    //
    // Block SLOC and displayed ranges use the full declaration span (the
    // declaration line plus body), not the body span alone.
    // ========================================================================

    #[test]
    fn block_includes_declaration_line_at_floor() {
        // `edge` has a one-line declaration plus 14 effective body lines: 15
        // SLOC total, exactly at MIN_BLOCK_SLOC. Measuring the body alone (14)
        // drops it below the floor; the full declaration span keeps it.
        assert_eq!(MIN_BLOCK_SLOC, 15, "test assumes the floor is 15");
        let dir = TempDir::new().unwrap();
        let mut source = String::from("def edge():\n");
        for i in 0..14 {
            source.push_str(&format!("    x{i} = {i}\n"));
        }
        let path = temp_file(&dir, "floor.py", &source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let comment_ranges = tree_file.comment_ranges().unwrap_or_default();
        let result = compute_symbol_blocks(tree_file.source(), &records, &comment_ranges);

        let edge = result
            .blocks
            .iter()
            .find(|b| b.name == "edge")
            .unwrap_or_else(|| {
                panic!(
                    "edge block missing; got {:?}",
                    result
                        .blocks
                        .iter()
                        .map(|b| (&b.name, b.sloc))
                        .collect::<Vec<_>>()
                )
            });
        assert_eq!(edge.sloc, 15, "block SLOC must span the declaration line");
        // Displayed range starts at the declaration, not the first body line.
        // TextPoint lines are 1-based, so the `def edge():` line is line 1.
        assert_eq!(edge.start_line, 1, "start line must be the `def` line");
    }

    #[test]
    fn multiline_declaration_without_body_uses_declaration_span() {
        // A multiline type alias has no body block, so `body_span` is None and
        // the declaration would collapse to its name token under the old
        // body-first policy. `record_span_bytes` must report the full
        // declaration span instead.
        let source = "type LongAlias =\n  | \"a\"\n  | \"b\"\n  | \"c\";\n";
        let dir = TempDir::new().unwrap();
        let path = temp_file(&dir, "alias.ts", source);

        let tree_file = TreeFile::new(&path).unwrap();
        let records = tree_file.symbol_records().unwrap();
        let alias = records
            .iter()
            .find(|r| r.identity.name == "LongAlias")
            .expect("LongAlias record present");

        assert!(
            alias.source.body_span.is_none(),
            "precondition: type alias has no recognized body span"
        );
        let (_, _, start_line, end_line) =
            record_span_bytes(alias).expect("declaration span present");
        assert!(
            end_line > start_line,
            "multiline declaration must not collapse to a single name line \
             (start={start_line}, end={end_line})"
        );
    }
}
