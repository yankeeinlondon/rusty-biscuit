//! Layout engine for choice option lists.
//!
//! [`ChoiceLayout`] computes the screen position of every visible
//! option. Supports vertical layout (one item per row) and horizontal
//! layout (items packed left-to-right, wrapping to new rows).

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
/// Computes the screen position of every visible option. In vertical mode
/// each option occupies one full row; in horizontal mode multiple options
/// may share a row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChoiceLayout {
    /// Rect for every option that could be rendered (in option order).
    pub item_rects: Vec<ChoiceItemRect>,
    /// For each row, the inclusive range of `item_rects` indices that
    /// sit on that row. Used for horizontal navigation.
    pub row_ranges: Vec<(usize, usize)>,
}

impl ChoiceLayout {
    /// Builds a vertical layout where every visible option occupies one full
    /// row within `area`.
    ///
    /// The returned rects map each visible index to its screen row.
    pub fn vertical(visible_indices: &[usize], area: Rect) -> Self {
        let mut item_rects = Vec::with_capacity(visible_indices.len());
        let mut row_ranges = Vec::with_capacity(visible_indices.len());

        for (i, &idx) in visible_indices.iter().enumerate() {
            item_rects.push(ChoiceItemRect {
                x: area.x,
                y: area.y + i as u16,
                width: area.width,
                height: 1,
                option_index: idx,
            });
            row_ranges.push((i, i));
        }

        Self {
            item_rects,
            row_ranges,
        }
    }

    /// Extra horizontal gap (in cells) inserted between items on the
    /// same row. Each item's width already includes a trailing blank;
    /// the gap adds breathing room beyond that so options don't feel
    /// cramped.
    const HORIZONTAL_GAP: u16 = 1;

    /// Builds a horizontal layout where options are packed left-to-right,
    /// wrapping to new rows when the next item would exceed `area.width`.
    ///
    /// `item_widths` must have the same length as `visible_indices` and
    /// contains the rendered width of each option (including indicator,
    /// separator, label, trailing blank, and badge).
    ///
    /// `row_height` is the vertical increment between layout rows. Pass `1`
    /// when hotkey badges are hidden; pass `2` (or more) when badges
    /// occupy a sub-row below each option so that successive option rows
    /// do not collide with the badge row.
    ///
    /// [`HORIZONTAL_GAP`] cells of padding are inserted between adjacent
    /// items on the same row (but not after the last item).
    pub fn horizontal(
        visible_indices: &[usize],
        item_widths: &[u16],
        area: Rect,
        row_height: u16,
    ) -> Self {
        assert_eq!(visible_indices.len(), item_widths.len());
        assert!(row_height >= 1);

        let mut item_rects = Vec::with_capacity(visible_indices.len());
        let mut row_ranges = Vec::new();

        let area_right = area.x + area.width;
        let mut x = area.x;
        let mut y = area.y;
        let mut current_row_start = 0;

        for (i, (&idx, &width)) in visible_indices.iter().zip(item_widths.iter()).enumerate() {
            // If this item plus the inter-item gap would overflow the
            // current row, start a new row. The gap is only needed when
            // there is already an item on this row (x > area.x).
            // The first item in a row always fits even if wider than the row.
            let needed = if x > area.x {
                Self::HORIZONTAL_GAP + width
            } else {
                width
            };
            if x + needed > area_right && x > area.x {
                row_ranges.push((current_row_start, i.saturating_sub(1)));
                current_row_start = i;
                x = area.x;
                y += row_height;
            }

            item_rects.push(ChoiceItemRect {
                x,
                y,
                width,
                height: row_height,
                option_index: idx,
            });

            x += width + Self::HORIZONTAL_GAP;
        }

        if current_row_start < item_rects.len() {
            row_ranges.push((current_row_start, item_rects.len() - 1));
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

    /// Returns the number of rows in the layout.
    pub fn row_count(&self) -> usize {
        self.row_ranges.len()
    }

    /// Returns the row index (0-based) that contains the given option index.
    pub fn row_of(&self, option_index: usize) -> Option<usize> {
        let item_idx = self
            .item_rects
            .iter()
            .position(|rect| rect.option_index == option_index)?;
        self.row_ranges
            .iter()
            .position(|(start, end)| *start <= item_idx && item_idx <= *end)
    }

    /// Returns the column (x coordinate) of the given option index.
    pub fn col_of(&self, option_index: usize) -> Option<u16> {
        self.item_rects
            .iter()
            .find(|rect| rect.option_index == option_index)
            .map(|rect| rect.x)
    }

    /// Returns the option index of the item in `row` whose column is
    /// closest to `target_col`. Returns `None` if the row is out of bounds
    /// or empty.
    pub fn closest_in_row(&self, row: usize, target_col: u16) -> Option<usize> {
        let (range_start, range_end) = *self.row_ranges.get(row)?;
        let mut best = None;
        let mut best_dist = u16::MAX;
        for i in range_start..=range_end {
            let rect = &self.item_rects[i];
            let dist = rect.x.abs_diff(target_col);
            if dist < best_dist {
                best_dist = dist;
                best = Some(rect.option_index);
            }
        }
        best
    }

    /// Returns the option index of the last item in `row`.
    pub fn last_in_row(&self, row: usize) -> Option<usize> {
        let (_start, end) = *self.row_ranges.get(row)?;
        self.item_rects.get(end).map(|rect| rect.option_index)
    }

    /// Returns the option index of the first item in `row`.
    pub fn first_in_row(&self, row: usize) -> Option<usize> {
        let (start, _end) = *self.row_ranges.get(row)?;
        self.item_rects.get(start).map(|rect| rect.option_index)
    }
}

/// Navigates to the closest item in the adjacent row.
///
/// `delta` of `-1` means the row above; `1` means the row below.
/// Returns `None` if the layout is empty, the current hover is not found,
/// or no enabled option exists in the target row.
pub fn navigate_row<V: Clone + PartialEq>(
    layout: &ChoiceLayout,
    options: &[super::choose::ChoiceOption<V>],
    current_hover: usize,
    delta: i32,
) -> Option<usize> {
    if layout.is_empty() {
        return None;
    }

    let current_row = layout.row_of(current_hover)?;
    let row_count = layout.row_count();
    if row_count == 0 {
        return None;
    }

    let target_row = if delta < 0 {
        if current_row == 0 {
            row_count - 1
        } else {
            current_row - 1
        }
    } else {
        if current_row + 1 >= row_count {
            0
        } else {
            current_row + 1
        }
    };

    let current_col = layout.col_of(current_hover).unwrap_or(0);
    let target_idx = layout
        .closest_in_row(target_row, current_col)
        .or_else(|| layout.last_in_row(target_row))?;

    if options.get(target_idx).is_some_and(|o| !o.disabled) {
        Some(target_idx)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_layout_one_item_per_row() {
        let area = Rect::new(0, 0, 20, 10);
        let visible = vec![0, 1, 2];
        let layout = ChoiceLayout::vertical(&visible, area);

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
        let visible = vec![10, 20];
        let layout = ChoiceLayout::vertical(&visible, area);

        assert_eq!(layout.item_rects[0].x, 5);
        assert_eq!(layout.item_rects[0].y, 3);
        assert_eq!(layout.item_rects[1].x, 5);
        assert_eq!(layout.item_rects[1].y, 4);
        assert_eq!(layout.item_rects[0].width, 30);
        assert_eq!(layout.item_rects[0].option_index, 10);
    }

    #[test]
    fn empty_layout() {
        let area = Rect::new(0, 0, 10, 5);
        let visible: Vec<usize> = vec![];
        let layout = ChoiceLayout::vertical(&visible, area);
        assert!(layout.is_empty());
        assert!(layout.row_ranges.is_empty());
    }

    #[test]
    fn horizontal_layout_packs_left_to_right() {
        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // Widths: 6, 8, 7 + HORIZONTAL_GAP(1) between items:
        //   row 0: item0(x=0,w=6), item1(x=7,w=8)  (6+1+8=15 ≤ 20)
        //   row 1: item2 wraps because 15+1+7=23 > 20
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);

        assert_eq!(layout.row_count(), 2);
        assert_eq!(layout.row_ranges, vec![(0, 1), (2, 2)]);

        assert_eq!(layout.item_rects[0].option_index, 0);
        assert_eq!(layout.item_rects[0].x, 0);
        assert_eq!(layout.item_rects[0].y, 0);

        assert_eq!(layout.item_rects[1].option_index, 1);
        assert_eq!(layout.item_rects[1].x, 7); // 6 + HORIZONTAL_GAP(1)
        assert_eq!(layout.item_rects[1].y, 0);

        assert_eq!(layout.item_rects[2].option_index, 2);
        assert_eq!(layout.item_rects[2].x, 0);
        assert_eq!(layout.item_rects[2].y, 1);
    }

    #[test]
    fn horizontal_layout_wraps_when_exceeds_width() {
        let area = Rect::new(0, 0, 10, 5);
        let visible = vec![0, 1, 2, 3];
        // Widths: 6, 6, 6, 6 -> each row fits one item
        let widths = vec![6, 6, 6, 6];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);

        assert_eq!(layout.row_count(), 4);
        for i in 0..4 {
            assert_eq!(layout.item_rects[i].x, 0);
            assert_eq!(layout.item_rects[i].y, i as u16);
            assert_eq!(layout.item_rects[i].option_index, i);
        }
    }

    #[test]
    fn horizontal_layout_first_item_always_fits() {
        let area = Rect::new(0, 0, 5, 5);
        let visible = vec![0, 1];
        // Widths: 10, 3 -> first item wider than row but still placed
        let widths = vec![10, 3];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);

        assert_eq!(layout.row_count(), 2);
        assert_eq!(layout.item_rects[0].x, 0);
        assert_eq!(layout.item_rects[0].width, 10);
        assert_eq!(layout.item_rects[1].x, 0);
        assert_eq!(layout.item_rects[1].width, 3);
    }

    #[test]
    fn row_of_finds_correct_row() {
        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // 6+8=14, 14+7=21 > 20 -> option 2 wraps to row 1
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);

        assert_eq!(layout.row_of(0), Some(0));
        assert_eq!(layout.row_of(1), Some(0));
        assert_eq!(layout.row_of(2), Some(1));
        assert_eq!(layout.row_of(99), None);
    }

    #[test]
    fn closest_in_row_finds_nearest_column() {
        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // 6+8=14, 14+7=21 > 20 -> option 2 on row 1 at x=0
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);

        // Row 1 has only option 2 at x=0. Closest to any column is option 2.
        assert_eq!(layout.closest_in_row(1, 5), Some(2));
        assert_eq!(layout.closest_in_row(1, 0), Some(2));
    }

    #[test]
    fn navigate_row_moves_to_adjacent_row() {
        use crate::components::choose::ChoiceOption;

        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // 6+8=14, 14+7=21 > 20 -> option 2 on row 1
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);
        let options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Alpha", "alpha"),
            ChoiceOption::new("b", "Beta", "beta"),
            ChoiceOption::new("c", "Gamma", "gamma"),
        ];

        // From option 0 (row 0), down goes to row 1 -> option 2.
        assert_eq!(navigate_row(&layout, &options, 0, 1), Some(2));
        // From option 2 (row 1), up goes to row 0 -> closest column is option 0.
        assert_eq!(navigate_row(&layout, &options, 2, -1), Some(0));
    }

    #[test]
    fn navigate_row_wraps_around() {
        use crate::components::choose::ChoiceOption;

        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // 6+8=14, 14+7=21 > 20 -> option 2 on row 1
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);
        let options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Alpha", "alpha"),
            ChoiceOption::new("b", "Beta", "beta"),
            ChoiceOption::new("c", "Gamma", "gamma"),
        ];

        // From row 0, up wraps to last row.
        assert_eq!(navigate_row(&layout, &options, 0, -1), Some(2));
        // From row 1, down wraps to first row.
        assert_eq!(navigate_row(&layout, &options, 2, 1), Some(0));
    }

    #[test]
    fn horizontal_layout_navigation_does_not_visit_badge_rows() {
        // Phase 6 invariant: hotkey badges in horizontal mode are
        // rendered on the screen row immediately below the option, but
        // the layout itself only contains option rows. Up/Down
        // navigation reads from `layout.row_ranges` and therefore must
        // skip the badge sub-rows entirely. This fixture pins that
        // navigation rolls option-row → option-row even when several
        // option rows would visually share screen space with badge
        // bands.
        use crate::components::choose::ChoiceOption;

        let area = Rect::new(0, 0, 30, 6);
        let visible = vec![0, 1, 2, 3];
        // Each option is 12 cells wide; row width 30 ⇒ two options per
        // row, two rows total. row_height=2 to reserve badge sub-rows.
        let widths = vec![12, 12, 12, 12];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 2);
        assert_eq!(layout.row_count(), 2);

        let options: Vec<ChoiceOption<String>> = (0..4)
            .map(|i| ChoiceOption::new(format!("o{i}"), format!("Opt {i}"), format!("v{i}")))
            .collect();

        // Down-arrow from the first row drops to row 1 (option 2 or
        // 3), never to a badge row that doesn't exist in the layout.
        let next = navigate_row(&layout, &options, 0, 1).expect("next row");
        assert!(next == 2 || next == 3);
        // Up-arrow from row 1 goes back to row 0, again never to a
        // badge sub-row.
        let prev = navigate_row(&layout, &options, next, -1).expect("prev row");
        assert!(prev == 0 || prev == 1);
    }

    #[test]
    fn navigate_row_skips_disabled() {
        use crate::components::choose::ChoiceOption;

        let area = Rect::new(0, 0, 20, 5);
        let visible = vec![0, 1, 2];
        // 6+8=14, 14+7=21 > 20 -> option 2 on row 1
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 1);
        let mut options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Alpha", "alpha"),
            ChoiceOption::new("b", "Beta", "beta"),
            ChoiceOption::new("c", "Gamma", "gamma"),
        ];
        options[2].disabled = true;

        // Target row has only disabled option -> None.
        assert_eq!(navigate_row(&layout, &options, 0, 1), None);
    }

    #[test]
    fn horizontal_layout_with_row_height_2_reserves_badge_space() {
        let area = Rect::new(0, 0, 20, 10);
        let visible = vec![0, 1, 2];
        // Widths: 6, 8, 7 -> row 0: items 0,1 (6+8=14 ≤ 20), row 1: item 2
        let widths = vec![6, 8, 7];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 2);

        assert_eq!(layout.row_count(), 2);

        // Row 0 items at y=0.
        assert_eq!(layout.item_rects[0].y, 0);
        assert_eq!(layout.item_rects[0].height, 2);
        assert_eq!(layout.item_rects[1].y, 0);
        assert_eq!(layout.item_rects[1].height, 2);

        // Row 1 item at y=2 (skipping y=1 badge sub-row).
        assert_eq!(layout.item_rects[2].y, 2);
        assert_eq!(layout.item_rects[2].height, 2);
    }

    #[test]
    fn horizontal_layout_three_rows_with_row_height_2() {
        let area = Rect::new(0, 0, 10, 10);
        let visible = vec![0, 1, 2, 3];
        // Widths: 6,6,6,6 → each row fits one item, 4 rows total.
        let widths = vec![6, 6, 6, 6];
        let layout = ChoiceLayout::horizontal(&visible, &widths, area, 2);

        assert_eq!(layout.row_count(), 4);
        assert_eq!(layout.item_rects[0].y, 0);
        assert_eq!(layout.item_rects[1].y, 2);
        assert_eq!(layout.item_rects[2].y, 4);
        assert_eq!(layout.item_rects[3].y, 6);
    }
}
