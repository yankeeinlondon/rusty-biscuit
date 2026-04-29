//! Layout engine for choice option lists.
//!
//! [`ChoiceLayout`] computes the screen position of every visible
//! option. Currently supports vertical layout (one item per row);
//! horizontal layout will be added in Phase 6.

use ratatui::layout::Rect;

/// Screen rectangle for a single choice option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceItemRect {
    /// Column offset of the top-left corner.
    pub x: u16,
    /// Row offset of the top-left corner.
    pub y: u16,
    /// Width in cells.
    pub width: u16,
    /// Height in cells (always 1 for vertical mode).
    pub height: u16,
    /// Index into the original options vector.
    pub option_index: usize,
}

/// Layout engine for choice lists.
///
/// Computes the screen position of every option. In vertical mode
/// each option occupies one full row; in horizontal mode (Phase 6)
/// multiple options may share a row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChoiceLayout {
    /// Rect for every option that could be rendered (in option order).
    pub item_rects: Vec<ChoiceItemRect>,
    /// For each row, the inclusive range of `item_rects` indices that
    /// sit on that row. Used for horizontal navigation.
    pub row_ranges: Vec<(usize, usize)>,
}

impl ChoiceLayout {
    /// Builds a vertical layout where every option occupies one full
    /// row within `area`.
    ///
    /// The returned rects are in source option order; callers must
    /// apply scroll offset themselves when rendering.
    pub fn vertical(options_count: usize, area: Rect) -> Self {
        let mut item_rects = Vec::with_capacity(options_count);
        let mut row_ranges = Vec::with_capacity(options_count);

        for i in 0..options_count {
            item_rects.push(ChoiceItemRect {
                x: area.x,
                y: area.y + i as u16,
                width: area.width,
                height: 1,
                option_index: i,
            });
            row_ranges.push((i, i));
        }

        Self {
            item_rects,
            row_ranges,
        }
    }

    /// Returns the number of options in the layout.
    pub fn len(&self) -> usize {
        self.item_rects.len()
    }

    /// Returns `true` when the layout contains no options.
    pub fn is_empty(&self) -> bool {
        self.item_rects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_layout_one_item_per_row() {
        let area = Rect::new(0, 0, 20, 10);
        let layout = ChoiceLayout::vertical(3, area);

        assert_eq!(layout.len(), 3);
        assert_eq!(layout.row_ranges, vec![(0, 0), (1, 1), (2, 2)]);

        assert_eq!(layout.item_rects[0].y, 0);
        assert_eq!(layout.item_rects[1].y, 1);
        assert_eq!(layout.item_rects[2].y, 2);

        assert_eq!(layout.item_rects[0].width, 20);
        assert_eq!(layout.item_rects[0].height, 1);
    }

    #[test]
    fn vertical_layout_with_offset_area() {
        let area = Rect::new(5, 3, 30, 10);
        let layout = ChoiceLayout::vertical(2, area);

        assert_eq!(layout.item_rects[0].x, 5);
        assert_eq!(layout.item_rects[0].y, 3);
        assert_eq!(layout.item_rects[1].x, 5);
        assert_eq!(layout.item_rects[1].y, 4);
        assert_eq!(layout.item_rects[0].width, 30);
    }

    #[test]
    fn empty_layout() {
        let area = Rect::new(0, 0, 10, 5);
        let layout = ChoiceLayout::vertical(0, area);
        assert!(layout.is_empty());
        assert!(layout.row_ranges.is_empty());
    }
}
