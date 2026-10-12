//! The body rules, each planned as bounded source edits against one parse.
//!
//! Every rule reads the shared [`Model`] and proposes [`SourceEdit`]s; a rule
//! that cannot apply safely to a structure (a tab where a column matters, a
//! directive line next to the boundary, an expression inside a table) leaves
//! that structure as authored rather than guessing. The fidelity comparison
//! in `body` is the backstop for anything a rule gets wrong.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::{Alignment, Event};

use super::edits::SourceEdit;
use super::protect::{
    Lines, Protection, blank_line_prefix, content_start, depth_base, is_blank_at_depth, is_blank_line,
};
use super::rules::{ListInteriorPadding, ListItemPadding, ResolvedMarkdownRules};
use super::structure::{Block, BlockKind, Container, Model};

/// The separator a joined line break needs between `previous` and `next`:
/// nothing beside a spaceless-script letter or a zero-width space, otherwise
/// one space.
pub(super) fn join_separator(previous: Option<char>, next: Option<char>) -> &'static str {
    let spaceless = |c: Option<char>| c.is_some_and(|c| c == '\u{200B}' || super::reflow::is_spaceless_letter(c));
    if spaceless(previous) || spaceless(next) { "" } else { " " }
}

/// The change to one list item's marker and the spaces after it.
#[derive(Debug, Clone, Default)]
struct MarkerChange {
    number: Option<String>,
    padding_delta: isize,
}

#[derive(Debug, Clone, Copy)]
struct GapPlan {
    depth: usize,
    desired: usize,
    allow_removal: bool,
}

struct Planner<'a, 'v> {
    source: &'a str,
    lines: &'a Lines,
    protection: &'a Protection,
    events: &'a [(Event<'v>, Range<usize>)],
    model: &'a Model,
    rules: &'a ResolvedMarkdownRules,
    edits: Vec<SourceEdit>,
    markers: HashMap<usize, MarkerChange>,
    gaps: BTreeMap<usize, GapPlan>,
    /// Lines `unwrap` joins onto the line above.
    joined: std::collections::HashSet<usize>,
}

/// Plans every enabled body rule except fixed-width re-wrapping, which runs as
/// a second stage on this stage's output.
pub(super) fn plan(
    source: &str,
    lines: &Lines,
    protection: &Protection,
    events: &[(Event<'_>, Range<usize>)],
    model: &Model,
    rules: &ResolvedMarkdownRules,
) -> Vec<SourceEdit> {
    let mut planner = Planner {
        source,
        lines,
        protection,
        events,
        model,
        rules,
        edits: Vec::new(),
        markers: HashMap::new(),
        gaps: BTreeMap::new(),
        joined: std::collections::HashSet::new(),
    };
    // Joins are planned first: a line they remove no longer constrains how
    // its list may be re-indented.
    planner.unwrap();
    planner.default_language();
    planner.ordered_lists();
    planner.indentation();
    planner.emit_markers();
    planner.list_padding();
    planner.list_item_padding();
    planner.list_interior_padding();
    planner.emit_gaps();
    planner.table_layout();
    planner.edits
}

impl Planner<'_, '_> {
    fn line_of(&self, offset: usize) -> usize {
        self.lines.index_of(offset)
    }

    /// The last line inside `range` with content in it. A parser range can end
    /// partway into the next line, covering only that line's indentation;
    /// such a line is not part of the range's content.
    fn last_content_line(&self, range: &Range<usize>) -> usize {
        let first = self.line_of(range.start);
        let mut last = self.line_of(range.end.saturating_sub(1).max(range.start));
        while last > first {
            let line = self.lines.get(last);
            let covered = &self.source[line.start..range.end.clamp(line.start, line.end)];
            if self.is_whitespace_line(last) || covered.trim().is_empty() {
                last -= 1;
            } else {
                break;
            }
        }
        last
    }

    /// Whether a protected line inside `range`, or directly after it (past
    /// blank lines), sits behind a container prefix and would therefore change
    /// container if the lines around it moved. A directive line belongs to the
    /// view parse of no block, so one indented into the last item of a list
    /// lies just past that list's range.
    fn has_prefixed_protected_line(&self, range: &Range<usize>) -> bool {
        let mut end = range.end;
        let mut next = self.last_content_line(range) + 1;
        while next < self.lines.len() {
            let line = self.lines.get(next);
            if !(is_blank_line(self.source, line) || self.protection.is_line_protected(line.start)) {
                break;
            }
            end = line.next;
            next += 1;
        }
        self.protection.lines.iter().any(|line| {
            range.start <= line.start && line.start < end && !self.source[line.start..].starts_with("::")
        })
    }

    fn default_language(&mut self) {
        let language = self.rules.default_language.value.as_str();
        if language.is_empty() {
            return;
        }
        let bytes = self.source.as_bytes();
        for fence in &self.model.fences {
            if !fence.info_empty {
                continue;
            }
            let mut start = fence.start;
            while start < bytes.len() && bytes[start] == b' ' {
                start += 1;
            }
            let Some(&fence_char) = bytes.get(start) else { continue };
            if fence_char != b'`' && fence_char != b'~' {
                continue;
            }
            let run = bytes[start..].iter().take_while(|byte| **byte == fence_char).count();
            if self.protection.touches_inline(&(start..start + run + 1)) {
                continue;
            }
            self.edits.push(SourceEdit::insert(start + run, language));
        }
    }

    /// Renumbers ordered lists that are neither uniform nor sequential. A
    /// marker that changes width gives or takes the difference from the spaces
    /// after it, so the item's content column, and every line that depends on
    /// it, stays put.
    fn ordered_lists(&mut self) {
        if !self.rules.ordered_lists.value {
            return;
        }
        for list in &self.model.lists {
            if list.start.is_none() || list.items.is_empty() {
                continue;
            }
            let numbers: Option<Vec<(u64, usize)>> = list
                .items
                .iter()
                .map(|item| {
                    let marker = &self.model.items[*item].marker;
                    let digits = self.source.get(marker.start..marker.end.saturating_sub(1))?;
                    Some((digits.parse::<u64>().ok()?, digits.len()))
                })
                .collect();
            let Some(numbers) = numbers else { continue };
            let first = numbers[0].0;
            let uniform = numbers.iter().all(|(number, _)| *number == first);
            let sequential = numbers.iter().enumerate().all(|(index, (number, _))| *number == first + index as u64);
            if uniform || sequential {
                continue;
            }

            let mut changes = Vec::new();
            let mut feasible = true;
            for (index, item_index) in list.items.iter().enumerate() {
                let item = &self.model.items[*item_index];
                let renumbered = (first + index as u64).to_string();
                let (_, old_width) = numbers[index];
                let width_delta = renumbered.len() as isize - old_width as isize;
                let single_line = self.line_of(item.marker.start) == self.last_content_line(&item.range);
                let padding_delta = match item.padding {
                    _ if width_delta == 0 => 0,
                    Some(padding) if padding <= 4 => {
                        let compensated = padding as isize - width_delta;
                        if (1..=4).contains(&compensated) {
                            -width_delta
                        } else if single_line {
                            0
                        } else {
                            feasible = false;
                            break;
                        }
                    }
                    _ if single_line && item.padding.is_some() => 0,
                    _ => {
                        feasible = false;
                        break;
                    }
                };
                changes.push((*item_index, renumbered, padding_delta));
            }
            if !feasible {
                continue;
            }
            for (item, number, padding_delta) in changes {
                let change = self.markers.entry(item).or_default();
                change.number = Some(number);
                change.padding_delta += padding_delta;
            }
        }
    }

    /// Re-indents each list nested directly in an item to the requested step
    /// from its parent's marker, clamped to the band CommonMark reads as the
    /// same tree: at least the parent's content column, at most three past it.
    /// When the step lies beyond that band, the parent's marker padding grows
    /// (up to four spaces) so the requested column becomes valid.
    fn indentation(&mut self) {
        let Some(step) = self.rules.indentation.value.spaces() else { return };
        let mut line_delta: HashMap<(usize, usize), isize> = HashMap::new();
        let mut added_padding: HashMap<usize, isize> = HashMap::new();

        for list in &self.model.lists {
            let Some(parent_index) = list.parent_item else { continue };
            let Some(first_index) = list.items.first() else { continue };
            let parent = &self.model.items[parent_index];
            let depth = list.depth;
            let (Some(parent_marker), Some(parent_content)) = (parent.marker_column, parent.content_column()) else {
                continue;
            };
            let first_line = self.line_of(self.model.items[*first_index].marker.start);
            let list_lines = first_line..=self.last_content_line(&list.range);
            if self.has_prefixed_protected_line(&list.range)
                || self.crosses_multiline_inline(&list.range)
                || !self.lines_have_plain_indent(list_lines, depth)
            {
                continue;
            }

            let parent_line = self.line_of(parent.marker.start);
            let parent_shift = line_delta.get(&(parent_line, depth)).copied().unwrap_or(0);
            let mut new_content =
                parent_content as isize + parent_shift + added_padding.get(&parent_index).copied().unwrap_or(0);
            let target = (parent_marker as isize + parent_shift + step as isize).max(new_content);
            let target = if target > new_content + 3 {
                let extra = target - (new_content + 3);
                let padding = parent.padding.unwrap_or(usize::MAX) as isize;
                let already = self.markers.get(&parent_index).map_or(0, |change| change.padding_delta);
                let parent_lines = parent_line..=self.last_content_line(&parent.range);
                if padding + already + extra <= 4
                    && !self.has_prefixed_protected_line(&parent.range)
                    && !self.crosses_multiline_inline(&parent.range)
                    && self.lines_have_plain_indent(parent_lines.clone(), depth)
                {
                    self.markers.entry(parent_index).or_default().padding_delta += extra;
                    *added_padding.entry(parent_index).or_default() += extra;
                    for line in parent_lines.skip(1) {
                        if self.indent_at(line, depth).is_some_and(|indent| indent >= parent_content) {
                            *line_delta.entry((line, depth)).or_default() += extra;
                        }
                    }
                    new_content += extra;
                    target
                } else {
                    new_content + 3
                }
            } else {
                target
            };

            // Moving left must not lower the last item's content column to a
            // later block of the parent item, which would then join that item.
            let target = match (list.items.last().map(|item| &self.model.items[*item]), self.later_parent_indent(list, parent, depth, &line_delta)) {
                (Some(last), Some(later)) => match (last.marker_column, last.content_column()) {
                    (Some(marker), Some(content)) => target.max(later + 1 - (content - marker) as isize),
                    _ => target,
                },
                _ => target,
            };
            if target > new_content + 3 {
                continue;
            }

            // Every item's marker goes to the target column; items of one list
            // may be authored at different columns, so each moves by its own
            // delta, together with its own lines. A lazy continuation line
            // keeps its authored indentation: the parser strips it only up to
            // the containers it still satisfies, so moving it would change
            // code-span text on that line.
            let mut moves: Vec<(usize, usize, isize, usize)> = Vec::new();
            let mut safe = true;
            for item_index in &list.items {
                let item = &self.model.items[*item_index];
                let (Some(marker_column), Some(content)) = (item.marker_column, item.content_column()) else {
                    continue;
                };
                let marker_line = self.line_of(item.marker.start);
                let current = marker_column as isize + line_delta.get(&(marker_line, depth)).copied().unwrap_or(0);
                let delta = target - current;
                if delta == 0 {
                    continue;
                }
                // Moving left must not bring the item's content column down to
                // a lazy line, which would then lose indentation inside any
                // code span continued on it.
                let last = self.last_content_line(&item.range);
                safe &= (marker_line + 1..=last).all(|line| {
                    self.is_whitespace_line(line)
                        || self.joined.contains(&line)
                        || self.indent_at(line, depth).is_none_or(|indent| {
                            indent >= content || (indent as isize) < content as isize + delta
                        })
                });
                moves.push((marker_line, last, delta, content));
            }
            if !safe {
                continue;
            }
            for (marker_line, last, delta, content) in moves {
                for line in marker_line..=last {
                    if line == marker_line || self.indent_at(line, depth).is_some_and(|indent| indent >= content) {
                        *line_delta.entry((line, depth)).or_default() += delta;
                    }
                }
            }
        }

        let mut deltas: Vec<((usize, usize), isize)> = line_delta.into_iter().filter(|(_, delta)| *delta != 0).collect();
        deltas.sort_unstable();
        for ((line_index, depth), delta) in deltas {
            let line = self.lines.get(line_index);
            // A quote's empty `>` line moves with its quote. A whitespace-only
            // line moves only inside a code block, where its spaces past the
            // strip column are code; an empty line stays empty.
            let in_code = self.model.code_blocks.iter().any(|code| code.start < line.start && line.start < code.end);
            if (self.is_whitespace_line(line_index) && (!in_code || line.start == line.end))
                || self.protection.is_line_protected(line.start)
            {
                continue;
            }
            let Some(base) = depth_base(self.source, line, depth) else { continue };
            let run = self.source[base..line.end].bytes().take_while(|byte| *byte == b' ').count();
            let width = (run as isize + delta).max(0) as usize;
            self.edits.push(SourceEdit::reindent(base..base + run, " ".repeat(width)));
        }
    }

    /// Whether every non-blank, unprotected line in `lines` has space-only
    /// indentation at `depth`.
    fn lines_have_plain_indent(&self, lines: std::ops::RangeInclusive<usize>, depth: usize) -> bool {
        lines.into_iter().all(|index| {
            let line = self.lines.get(index);
            if self.is_whitespace_line(index) || self.protection.is_line_protected(line.start) {
                return true;
            }
            match depth_base(self.source, line, depth) {
                Some(base) => !self.source[base..line.end].trim_start_matches(' ').starts_with('\t'),
                // A lazy line has no column of its own.
                None => !self.source[line.start..line.end].starts_with('\t'),
            }
        })
    }

    /// Whether an expression (or a literal) that spans lines overlaps
    /// `range`. Re-indenting or re-spacing lines inside one would change its
    /// spelling, so the structure is left as authored.
    fn crosses_multiline_inline(&self, range: &Range<usize>) -> bool {
        self.protection.inline.iter().any(|span| {
            span.start < range.end
                && range.start < span.end
                && self.source[span.clone()].contains(['\n', '\r'])
        })
    }

    /// The (already shifted) indentation of the first line after `list` in its
    /// parent item: the start of the block that follows the list there, the
    /// only line that could join the list's last item.
    fn later_parent_indent(
        &self,
        list: &super::structure::ListModel,
        parent: &super::structure::ItemModel,
        depth: usize,
        line_delta: &HashMap<(usize, usize), isize>,
    ) -> Option<isize> {
        let after = self.last_content_line(&list.range) + 1;
        let last = self.last_content_line(&parent.range);
        let line = (after..=last).find(|line| !self.is_whitespace_line(*line))?;
        if self.protection.is_line_protected(self.lines.get(line).start) {
            return None;
        }
        self.indent_at(line, depth)
            .map(|indent| indent as isize + line_delta.get(&(line, depth)).copied().unwrap_or(0))
    }

    fn is_whitespace_line(&self, index: usize) -> bool {
        let line = self.lines.get(index);
        self.source[line.start..line.end].trim().is_empty()
    }

    /// The authored indentation of `line` at block-quote `depth`, in spaces.
    fn indent_at(&self, line: usize, depth: usize) -> Option<usize> {
        let line = self.lines.get(line);
        let base = depth_base(self.source, line, depth)?;
        Some(self.source[base..line.end].bytes().take_while(|byte| *byte == b' ').count())
    }

    fn emit_markers(&mut self) {
        let mut changes: Vec<(usize, MarkerChange)> = std::mem::take(&mut self.markers).into_iter().collect();
        changes.sort_unstable_by_key(|(item, _)| *item);
        for (item_index, change) in changes {
            let item = &self.model.items[item_index];
            if self.protection.touches_inline(&(item.marker.start..item.marker.end + item.padding.unwrap_or(0))) {
                continue;
            }
            let marker = &self.source[item.marker.clone()];
            let new_marker = match &change.number {
                Some(number) => format!("{number}{}", &marker[marker.len() - 1..]),
                None => marker.to_string(),
            };
            match item.padding {
                Some(padding) if change.padding_delta != 0 => {
                    let spaces = (padding as isize + change.padding_delta).max(1) as usize;
                    self.edits.push(SourceEdit::replace(
                        item.marker.start..item.marker.end + padding,
                        format!("{new_marker}{}", " ".repeat(spaces)),
                    ));
                }
                _ if new_marker != marker => {
                    self.edits.push(SourceEdit::replace(item.marker.clone(), new_marker));
                }
                _ => {}
            }
        }
    }

    fn want_gap(&mut self, anchor: Option<usize>, depth: usize, desired: usize, allow_removal: bool) {
        if let Some(anchor) = anchor {
            self.gaps.entry(anchor).or_insert(GapPlan { depth, desired, allow_removal });
        }
    }

    /// One blank line before and after each list whose container is the
    /// document or a block quote (blank lines there never change tight/loose).
    fn list_padding(&mut self) {
        if !self.rules.list_padding.value {
            return;
        }
        let model = self.model;
        for list in &model.lists {
            if !matches!(list.container, Container::Document | Container::BlockQuote) {
                continue;
            }
            if list.previous.is_some() {
                let anchor = list.items.first().map(|item| self.line_of(model.items[*item].marker.start));
                self.want_gap(anchor, list.depth, 1, true);
            }
            if let Some(next) = &list.next {
                let anchor = Some(self.line_of(next.range.start));
                self.want_gap(anchor, list.depth, 1, true);
            }
        }
    }

    fn list_item_padding(&mut self) {
        let desired = match self.rules.list_item_padding.value {
            ListItemPadding::Preserve => return,
            ListItemPadding::None => 0,
            ListItemPadding::One => 1,
        };
        let model = self.model;
        for list in &model.lists {
            for pair in list.items.windows(2) {
                let previous = &model.items[pair[0]];
                let next = &model.items[pair[1]];
                // An HTML block or a table ends only at a blank line or a new
                // block; removing the blank could pull the next item into it.
                let allow_removal = !matches!(
                    previous.children.last().map(|block| block.kind),
                    Some(BlockKind::Html | BlockKind::Table)
                );
                let anchor = Some(self.line_of(next.marker.start));
                self.want_gap(anchor, list.depth, desired, allow_removal);
            }
        }
    }

    fn list_interior_padding(&mut self) {
        let value = self.rules.list_interior_padding.value;
        if value == ListInteriorPadding::Preserve {
            return;
        }
        let model = self.model;
        for item in &model.items {
            let depth = model.lists.get(item.list).map_or(0, |list| list.depth);
            let first_line = self.line_of(item.marker.start);
            for pair in item.children.windows(2) {
                let (previous, next) = (&pair[0], &pair[1]);
                let anchor = self.line_of(next.range.start);
                if anchor <= first_line {
                    continue;
                }
                let desired = match value {
                    ListInteriorPadding::None => 0,
                    ListInteriorPadding::ExceptNestedLists if matches!(next.kind, BlockKind::List { .. }) => 0,
                    _ => 1,
                };
                self.want_gap(Some(anchor), depth, desired, can_interrupt(previous, next));
            }
        }
    }

    /// Turns the boundary plans into whole-line insertions and deletions.
    fn emit_gaps(&mut self) {
        let gaps = std::mem::take(&mut self.gaps);
        for (anchor, plan) in gaps {
            let anchor_line = self.lines.get(anchor);
            if anchor == 0 || self.protection.is_line_protected(anchor_line.start) {
                continue;
            }
            let mut first_blank = anchor;
            while first_blank > 0 {
                let line = self.lines.get(first_blank - 1);
                if self.protection.is_line_protected(line.start) || !is_blank_at_depth(self.source, line, plan.depth) {
                    break;
                }
                first_blank -= 1;
            }
            // A directive line on either side of the gap, no block above it,
            // or an unclosed fence above it (the blank lines would be code)
            // leaves the boundary as written.
            if first_blank == 0 || self.protection.is_line_protected(self.lines.get(first_blank - 1).start) {
                continue;
            }
            let above = first_blank - 1;
            if self.model.unclosed_fences.iter().any(|code| {
                self.line_of(code.start) <= above && above <= self.line_of(code.end.saturating_sub(1).max(code.start))
            }) {
                continue;
            }
            let count = anchor - first_blank;
            if count == plan.desired
                || self.crosses_multiline_inline(&(self.lines.get(first_blank - 1).start..anchor_line.next))
            {
                continue;
            }
            if count > plan.desired {
                if plan.desired == 0 && !plan.allow_removal {
                    continue;
                }
                let start = self.lines.get(first_blank + plan.desired).start;
                self.edits.push(SourceEdit::replace(start..anchor_line.start, ""));
            } else {
                let Some(prefix) = blank_line_prefix(self.source, anchor_line, plan.depth) else { continue };
                let terminator = match self.lines.terminator(self.source, anchor - 1) {
                    "" => "\n",
                    terminator => terminator,
                };
                self.edits.push(SourceEdit::insert(anchor_line.start, format!("{prefix}{terminator}")));
            }
        }
    }

    /// Pads each table's cells to one width per column and rewrites the
    /// delimiter row to match, keeping every row's container prefix.
    fn table_layout(&mut self) {
        if !self.rules.table_layout.value {
            return;
        }
        let model = self.model;
        'tables: for table in &model.tables {
            if table.rows.is_empty()
                || self.protection.line_within(&table.range)
                || self.protection.touches_inline(&table.range)
            {
                continue;
            }
            let columns = table.alignments.len();
            let mut rows: Vec<(Range<usize>, Vec<&str>)> = Vec::new();
            for row in &table.rows {
                let line = self.lines.get(self.line_of(row.range.start));
                let prefix = &self.source[line.start..row.range.start];
                if prefix.contains('\t') || row.cells.len() != columns || row.range.end > line.next {
                    continue 'tables;
                }
                let first_cell = row.cells.first().map_or(row.range.start, |cell| cell.start);
                let lead = &self.source[row.range.start..first_cell.max(row.range.start)];
                let last_cell = row.cells.iter().map(|cell| cell.end).max().unwrap_or(row.range.start);
                let tail = self.source.get(last_cell.min(line.end)..line.end).unwrap_or("");
                if !matches!(lead.trim(), "" | "|") || !matches!(tail.trim(), "" | "|") {
                    continue 'tables;
                }
                let cells = row
                    .cells
                    .iter()
                    .map(|cell| if cell.is_empty() { "" } else { self.source[cell.clone()].trim_matches([' ', '\t']) })
                    .collect();
                rows.push((row.range.start..line.end, cells));
            }

            let head_line_index = self.line_of(table.rows[0].range.start);
            if head_line_index + 1 >= self.lines.len() {
                continue;
            }
            let head_line = self.lines.get(head_line_index);
            let head_prefix = &self.source[head_line.start..table.rows[0].range.start];
            let delimiter_line = self.lines.get(head_line_index + 1);
            let delimiter_text = &self.source[delimiter_line.start..delimiter_line.end];
            if self.protection.is_line_protected(delimiter_line.start) || !delimiter_text.starts_with(head_prefix) {
                continue;
            }

            let widths: Vec<usize> = (0..columns)
                .map(|column| {
                    rows.iter()
                        .map(|(_, cells)| UnicodeWidthStr::width(cells[column]))
                        .max()
                        .unwrap_or(0)
                        .max(3)
                })
                .collect();
            for (range, cells) in &rows {
                let padded: Vec<String> = cells
                    .iter()
                    .zip(&widths)
                    .map(|(cell, width)| format!("{cell}{}", " ".repeat(width - UnicodeWidthStr::width(*cell))))
                    .collect();
                let rebuilt = format!("| {} |", padded.join(" | "));
                if self.source[range.clone()] != rebuilt {
                    self.edits.push(SourceEdit::replace(range.clone(), rebuilt));
                }
            }
            let delimiters: Vec<String> = table
                .alignments
                .iter()
                .zip(&widths)
                .map(|(alignment, width)| match alignment {
                    Alignment::None => "-".repeat(*width),
                    Alignment::Left => format!(":{}", "-".repeat(width - 1)),
                    Alignment::Right => format!("{}:", "-".repeat(width - 1)),
                    Alignment::Center => format!(":{}:", "-".repeat(width - 2)),
                })
                .collect();
            let rebuilt = format!("| {} |", delimiters.join(" | "));
            let range = delimiter_line.start + head_prefix.len()..delimiter_line.end;
            if self.source[range.clone()] != rebuilt {
                self.edits.push(SourceEdit::replace(range, rebuilt));
            }
        }
    }

    /// Joins each paragraph soft break with the next line, through that
    /// line's container prefix. A break next to a removed directive line or
    /// inside an expression is left alone.
    fn unwrap(&mut self) {
        if !self.rules.joins_soft_breaks() {
            return;
        }
        let bytes = self.source.as_bytes();
        for run in &self.model.runs {
            for index in run.events.clone() {
                if !matches!(self.events[index].0, Event::SoftBreak) {
                    continue;
                }
                let line_break = self.events[index].1.start;
                let Some((_, next)) = self.events.get(index + 1) else { continue };
                if next.start <= line_break || self.line_of(next.start) != self.line_of(line_break) + 1 {
                    continue;
                }
                let next_start = content_start(self.source, next.start, self.lines.get(self.line_of(next.start)).start);
                let floor = index.checked_sub(1).map_or(0, |previous| self.events[previous].1.end.min(line_break));
                let mut join_start = line_break;
                while join_start > floor && matches!(bytes[join_start - 1], b' ' | b'\t') {
                    join_start -= 1;
                }
                let range = join_start..next_start;
                if self.protection.line_within(&(line_break..next_start)) || self.protection.touches_inline(&range) {
                    continue;
                }
                let separator = join_separator(
                    self.source[..join_start].chars().next_back(),
                    self.source[next_start..].chars().next(),
                );
                self.edits.push(SourceEdit::replace(range, separator));
                self.joined.insert(self.line_of(next_start));
            }
        }
    }
}

/// Whether `next` can follow `previous` inside an item with no blank line and
/// still be its own block. A paragraph never can (it would continue the line
/// above), and a block quote directly after another joins it.
fn can_interrupt(previous: &Block, next: &Block) -> bool {
    let next_interrupts = match next.kind {
        BlockKind::List { ordered, start, first_item_empty } => !first_item_empty && (!ordered || start == Some(1)),
        BlockKind::FencedCode | BlockKind::BlockQuote | BlockKind::Heading { setext: false } => true,
        _ => false,
    };
    let previous_ends = matches!(
        previous.kind,
        BlockKind::Paragraph
            | BlockKind::FencedCode
            | BlockKind::List { .. }
            | BlockKind::Heading { .. }
    ) || matches!(previous.kind, BlockKind::BlockQuote);
    // Two block quotes with no blank line between them are one quote.
    let merges = matches!((previous.kind, next.kind), (BlockKind::BlockQuote, BlockKind::BlockQuote));
    next_interrupts && previous_ends && !merges
}
