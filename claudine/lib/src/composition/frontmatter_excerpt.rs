//! Focused frontmatter YAML for an error-attached source excerpt.
//!
//! When a composition error is rooted in a prompt file's YAML frontmatter, the
//! report is far easier to act on if it shows the lines involved. A
//! [`FrontmatterExcerpt`] holds only the focused regions of the block: each
//! located line with [`EXCERPT_CONTEXT_LINES`] of context either side, plus the
//! header lines that enclose it (a `$schema:` parent, the `- …` line opening a
//! union arm). It renders each region as a [`CodeBlock`] numbered with its real
//! source lines, joined by a `⋮` elision line. When nothing is locatable, no
//! excerpt is captured; the whole block renders only when the focused regions
//! already cover every line of it.
//!
//! Line *selection* comes from `biscuit-terminal`'s
//! [`SourceContext::focused_line_regions`]; this module resolves Claudine's
//! property paths (sequence indexes, re-rooted task paths, `$schema`
//! declarations) to the lines it focuses.
//!
//! The excerpt is attached to an error at the render boundary (see
//! `CompositionError::enrich_frontmatter`) and appended after the primary
//! diagnostic by the CLI's error walker. Rendering is withheld in non-TTY
//! output to avoid exposing frontmatter into pipes, logs, and CI — the same
//! privacy posture the inline-compose / sequence mismatch diagnostic uses.

use std::path::PathBuf;

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::errors::SourceContext;
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::CodeBlock;
use darkmatter::markdown::dsl::CodeBlockMeta;

/// Lines of context shown above and below each focused line (ruling D2; there
/// is deliberately no configuration surface).
pub const EXCERPT_CONTEXT_LINES: usize = 3;

/// The focused regions of a frontmatter block, ready to render as YAML
/// [`CodeBlock`]s appended to an error report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterExcerpt {
    /// Non-adjacent runs of source lines, ascending.
    regions: Vec<ExcerptRegion>,
    /// Whether stderr was a TTY when the excerpt was captured. Gates rendering:
    /// non-TTY output omits the excerpt entirely.
    stderr_is_tty: bool,
}

/// One contiguous run of frontmatter source lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExcerptRegion {
    /// The run's source text, without a trailing line-ending.
    text: String,
    /// 1-based source-file line of the run's first line.
    start_line: usize,
    /// 1-based source-file lines to highlight within the run.
    highlighted: Vec<usize>,
}

impl FrontmatterExcerpt {
    /// Capture the excerpt for one dotted frontmatter property
    /// (e.g. `"success.message"`, `"initialize.stack[0].action[1].set"`).
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block or the
    /// property cannot be located in it.
    pub fn capture(source_text: &str, property: &str, stderr_is_tty: bool) -> Option<Self> {
        Self::capture_properties(source_text, &[property], stderr_is_tty)
    }

    /// Capture the union of the excerpts for several dotted frontmatter
    /// properties. Properties that cannot be located are skipped.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block or none
    /// of the properties can be located.
    pub fn capture_properties(
        source_text: &str,
        properties: &[impl AsRef<str>],
        stderr_is_tty: bool,
    ) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)?;
        let lines = properties
            .iter()
            .filter_map(|property| locate_property_line(&block, property.as_ref()))
            .collect();
        Self::focus(&block, lines, stderr_is_tty)
    }

    /// Capture the excerpt for properties a schema problem names.
    ///
    /// Each property is shown where the document sets it (an exact frontmatter
    /// key, when one exists) and where `$schema` declares it: under an inline
    /// `$schema` mapping, or in every arm of a `$schema` union.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block or no
    /// property is found in either place.
    pub fn capture_schema_properties(
        source_text: &str,
        properties: &[impl AsRef<str>],
        stderr_is_tty: bool,
    ) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)?;
        let located = located_paths(&block);
        let lines = properties
            .iter()
            .flat_map(|property| {
                let property = property.as_ref();
                located
                    .iter()
                    .filter(move |(path, _)| path == property || declares(path, property))
                    .map(|(_, line)| *line)
            })
            .collect();
        Self::focus(&block, lines, stderr_is_tty)
    }

    /// Capture an excerpt for a schema-body parse failure, mapping a byte span
    /// within the offending property's type-and-constraint string to a source
    /// line.
    ///
    /// `property` is the dotted schema-property path (e.g. `"$schema.spec"`) the
    /// typed `SchemaError::Grammar` failure was attributed to. The byte span is
    /// an offset **into that property's type-and-constraint string** (not the
    /// document), so it is mapped to a source line by counting the line-breaks
    /// the span crosses inside the property's value region. For the usual
    /// single-line type string (`spec: file(required, match(...))`) the span
    /// crosses no line-break and the highlight lands on the property line; for a
    /// multi-line YAML block scalar it advances to the continuation line the span
    /// points into.
    ///
    /// When `property` is `None`, or the property cannot be located, the
    /// highlight falls back to the `$schema` parent line.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block or
    /// neither the property nor `$schema` can be located.
    pub fn capture_schema_span(
        source_text: &str,
        property: Option<&str>,
        span_start: usize,
        stderr_is_tty: bool,
    ) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)?;
        let line = property
            .and_then(|p| locate_property_line(&block, p))
            .map(|line| line + value_line_offset(&block, line, span_start))
            .or_else(|| locate_property_line(&block, "$schema"));
        Self::focus(&block, line.into_iter().collect(), stderr_is_tty)
    }

    /// Capture the excerpt around a 1-based source line.
    ///
    /// Used where the error carries a document line rather than a property: a
    /// YAML parse error with a location, or `FrontmatterFenceMismatch`, whose
    /// offending token is the delimiter itself. The block is the `---`
    /// frontmatter, or else a matched dash-only (`----`+) near-miss fence pair.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has neither block, or `line` is outside it.
    pub fn capture_line(source_text: &str, line: usize, stderr_is_tty: bool) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)
            .or_else(|| capture_near_miss_frontmatter_block(source_text))?;
        Self::focus(&block, vec![line], stderr_is_tty)
    }

    /// Focus `block` (whose line 1 is the source file's line 1) on `lines`.
    fn focus(block: &str, lines: Vec<usize>, stderr_is_tty: bool) -> Option<Self> {
        let context = SourceContext::with_frontmatter(
            PathBuf::new(),
            PathBuf::new(),
            block,
            Some(0..block.len()),
        );
        let source_lines: Vec<&str> = block.lines().collect();
        let regions = context
            .focused_line_regions(&lines, EXCERPT_CONTEXT_LINES)?
            .into_iter()
            .map(|region| ExcerptRegion {
                text: source_lines[region.start_line - 1..region.end_line].join("\n"),
                start_line: region.start_line,
                highlighted: region.highlighted,
            })
            .collect();
        Some(Self {
            regions,
            stderr_is_tty,
        })
    }

    /// Render the excerpt as a trailing appendix for an error report.
    ///
    /// Returns an empty string in non-TTY output (privacy gating). Otherwise
    /// returns a blank-line separator and one YAML [`CodeBlock`] per region,
    /// numbered with source lines, with a `⋮` line between regions. SGR and
    /// OSC 8 escapes are stripped when the terminal has no color depth, so
    /// redirected / `NO_COLOR` output stays plain text.
    pub fn render_appendix(&self, term: &Terminal) -> String {
        if !self.stderr_is_tty {
            return String::new();
        }
        let mut rendered = String::new();
        for (idx, region) in self.regions.iter().enumerate() {
            let mut meta = CodeBlockMeta {
                line_numbering: true,
                ..CodeBlockMeta::default()
            };
            for line in &region.highlighted {
                meta.highlight.add_line(*line);
            }
            let block = CodeBlock::yaml(region.text.clone())
                .with_meta(meta)
                .with_start_line(region.start_line)
                .render(term);
            if idx > 0 {
                rendered.push('\n');
                rendered.push_str(&" ".repeat(gutter_column(&block)));
                rendered.push_str("⋮\n");
            }
            rendered.push_str(block.trim_end_matches('\n'));
        }
        let body = if matches!(term.color_depth, ColorDepth::None) {
            strip_escape_codes(&rendered)
        } else {
            rendered
        };
        format!("\n\n{body}")
    }
}

#[cfg(test)]
impl FrontmatterExcerpt {
    /// Test-only accessor for the first highlighted line.
    pub fn highlight_line(&self) -> Option<usize> {
        self.highlighted_lines().first().copied()
    }

    /// Test-only accessor for every highlighted line, ascending.
    pub fn highlighted_lines(&self) -> Vec<usize> {
        self.regions
            .iter()
            .flat_map(|region| region.highlighted.iter().copied())
            .collect()
    }

    /// Test-only accessor for each region's inclusive source-line span.
    pub fn line_spans(&self) -> Vec<(usize, usize)> {
        self.regions
            .iter()
            .map(|region| (region.start_line, region.start_line + region.text.lines().count() - 1))
            .collect()
    }
}

/// The display column of the line-number gutter's `│` in a rendered block, so
/// an elision line can sit in it. `0` when the block draws no gutter (a
/// terminal without color renders a plain fence).
fn gutter_column(rendered_block: &str) -> usize {
    strip_escape_codes(rendered_block)
        .lines()
        .find_map(|row| row.chars().position(|c| c == '│'))
        .unwrap_or(0)
}

/// Whether the located `path` is a `$schema` declaration of `property`: under
/// an inline `$schema` mapping (`$schema.spec`) or in a union arm
/// (`$schema[1].spec`).
fn declares(path: &str, property: &str) -> bool {
    let Some(rest) = path.strip_prefix("$schema") else {
        return false;
    };
    let rest = match rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
        Some((index, tail)) if index.parse::<usize>().is_ok() => tail,
        _ => rest,
    };
    rest.strip_prefix('.') == Some(property)
}

/// Capture the frontmatter block **including** its `---` delimiter lines.
///
/// Unlike [`capture_frontmatter_yaml`](super::mismatch::capture_frontmatter_yaml),
/// which returns only the interior, this keeps the delimiters so a rendered
/// [`CodeBlock`]'s 1-based line numbers line up with the source file (whose
/// frontmatter always opens on line 1). A single trailing line-ending after the
/// closing delimiter is stripped; all interior line-endings are preserved.
///
/// ## Returns
///
/// `None` when `text` has no well-formed frontmatter block (no leading `---`
/// line, or no closing `---` line).
pub fn capture_frontmatter_block(text: &str) -> Option<String> {
    let mut lines = text.split_inclusive('\n');

    let opening = lines.next()?;
    if opening.trim() != "---" {
        return None;
    }

    let mut end = opening.len();
    for line in lines {
        end += line.len();
        if line.trim() == "---" {
            let block = &text[..end];
            return Some(block.strip_suffix('\n').unwrap_or(block).to_string());
        }
    }

    None
}

/// Capture a near-miss frontmatter block **including** its `----`+ delimiter lines.
///
/// This is the counterpart to [`capture_frontmatter_block`] for the malformed
/// dash-only fence case. It recognizes a matched pair of four-or-more dash
/// fences (`----`/`-----`/...) and returns the full block, delimiters included,
/// so line 1 in the captured block equals line 1 in the source file.
///
/// ## Returns
///
/// `None` when `text` has no matched near-miss dash-only fence pair at the top
/// of the document.
pub fn capture_near_miss_frontmatter_block(text: &str) -> Option<String> {
    let mut lines = text.split_inclusive('\n');

    let opening = lines.next()?;
    let opening_trimmed = opening.trim();
    if !is_dash_only_fence(opening_trimmed) || opening_trimmed.len() < 4 {
        return None;
    }

    let mut end = opening.len();
    for line in lines {
        end += line.len();
        if line.trim() == opening_trimmed {
            let block = &text[..end];
            return Some(block.strip_suffix('\n').unwrap_or(block).to_string());
        }
    }

    None
}

/// Returns `true` when `text` is non-empty and contains only `-` characters.
fn is_dash_only_fence(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b == b'-')
}

/// Locate the 1-based line of a dotted frontmatter property within a captured
/// `block` (delimiters included).
///
/// Resolves nested keys and sequence indexes by indentation. A semantic path
/// whose document root differs from its authored YAML root may match a unique
/// suffix; ambiguous suffixes are refused. Blank and comment lines are skipped.
///
/// ## Examples
///
/// ```ignore
/// // block line 1 is `---`; `success.message` resolves to its file line.
/// assert_eq!(locate_property_line(block, "success.message"), Some(16));
/// ```
pub fn locate_property_line(block: &str, dotted_property: &str) -> Option<usize> {
    let located = located_paths(block);
    if let Some((_, line)) = located.iter().find(|(path, _)| path == dotted_property) {
        return Some(*line);
    }

    let mut suffix = dotted_property;
    while let Some((_, remainder)) = suffix.split_once('.') {
        suffix = remainder;
        let matches = located
            .iter()
            .filter(|(path, _)| path == suffix || path.ends_with(&format!(".{suffix}")))
            .collect::<Vec<_>>();
        if let [(_, line)] = matches.as_slice() {
            return Some(*line);
        }
    }
    None
}

/// Every key and sequence item in `block` as a dotted path (`a.b[0].c`) with
/// its 1-based line.
///
/// A `- key: …` line yields both the item (`a[0]`) and its first key
/// (`a[0].key`); the item's later keys sit at the key's column, so they are
/// its siblings (`a[0].other`), not its children.
fn located_paths(block: &str) -> Vec<(String, usize)> {
    let mut parents: Vec<(isize, String)> = Vec::new();
    let mut indexes = std::collections::HashMap::<(isize, String), usize>::new();
    let mut located = Vec::<(String, usize)>::new();

    for (idx, line) in block.lines().enumerate() {
        if is_blank_or_comment(line) || line.trim() == "---" {
            continue;
        }
        let indent = indent_of(line);
        while parents
            .last()
            .is_some_and(|(parent_indent, _)| *parent_indent >= indent)
        {
            parents.pop();
        }
        let parent = parents.last().map_or("", |(_, path)| path.as_str()).to_string();
        let trimmed = line.trim_start();

        if trimmed == "-" || trimmed.starts_with("- ") {
            let counter = indexes.entry((indent, parent.clone())).or_default();
            let item = format!("{parent}[{counter}]");
            *counter += 1;
            located.push((item.clone(), idx + 1));
            parents.push((indent, item.clone()));

            let rest = &trimmed[1..];
            let body = rest.trim_start();
            if let Some(key) = key_name(body) {
                let key_col = indent + 1 + (rest.len() - body.len()) as isize;
                let path = join_property(&item, key);
                located.push((path.clone(), idx + 1));
                parents.push((key_col, path));
            }
        } else if let Some(key) = key_name(trimmed) {
            let path = join_property(&parent, key);
            located.push((path.clone(), idx + 1));
            parents.push((indent, path));
        }
    }
    located
}

fn join_property(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}.{key}")
    }
}

/// Map a byte offset within a property's type-and-constraint string to the
/// number of source line-breaks it crosses, so a schema-grammar `span` can
/// advance the highlight onto the right continuation line of a multi-line value.
///
/// `property_line` is the 1-based source line of the `key:` entry inside `block`.
/// The property's value text is everything after the first `:` on that line plus
/// any deeper-indented continuation lines (a YAML block scalar). The returned
/// offset is `0` for the common single-line type string, since its value text
/// holds no line-break before `span_start`.
fn value_line_offset(block: &str, property_line: usize, span_start: usize) -> usize {
    let lines: Vec<&str> = block.lines().collect();
    let Some(key_idx) = property_line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return 0;
    };
    let key_indent = indent_of(lines[key_idx]);

    // Reconstruct the value text exactly as the schema lexer saw it: the inline
    // remainder after `key:`, then each deeper-indented continuation line joined
    // by the `\n` the lexer's span counts against.
    let inline = lines[key_idx]
        .split_once(':')
        .map_or("", |(_, rest)| rest.trim_start());
    let mut value = String::from(inline);
    for line in &lines[key_idx + 1..] {
        if is_blank_or_comment(line) {
            value.push('\n');
            continue;
        }
        if indent_of(line) <= key_indent {
            break;
        }
        value.push('\n');
        value.push_str(line.trim_start());
    }

    // Clamp to a char boundary at or below `span_start` so a span landing mid
    // UTF-8 sequence (e.g. a non-ASCII description) never panics the slice.
    let mut end = span_start.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].matches('\n').count()
}

/// The byte-width of a line's leading spaces, as a signed value for comparison
/// against the synthetic `-1` root indent.
fn indent_of(line: &str) -> isize {
    (line.len() - line.trim_start().len()) as isize
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Extract the mapping key from a `key: value` line, or `None` when the line is
/// not a plain mapping entry (e.g. a list item or a bare scalar).
fn key_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    let (key, _) = trimmed.split_once(':')?;
    let key = key.trim();
    Some(key.trim_matches(|c| c == '"' || c == '\''))
}

#[cfg(test)]
mod tests;
