//! Kitty Unicode placeholders: an image uploaded with a virtual placement
//! (`U=1`) is drawn wherever the terminal finds placeholder cells carrying
//! its ID. The cells are ordinary text, so through tmux they follow the pane
//! when it moves, resizes, or is hidden.
//!
//! Each cell is [`PLACEHOLDER`] followed by two combining marks from
//! [`ROW_COLUMN_DIACRITICS`]: the row, then the column, within the image.
//! The image ID is the cell's 24-bit foreground color. The developer guide
//! is `biscuit-terminal/docs/discovery/graphics-routes.md`.

use crate::discovery::detection::probe::MAX_KITTY_ID;

/// The placeholder character Kitty reserves for image cells.
pub(super) const PLACEHOLDER: char = '\u{10EEEE}';

/// Kitty's row/column diacritics: index `n` marks row or column `n`. Copied
/// in order from kitty's `rowcolumn-diacritics.txt`; its length bounds the
/// grid.
pub(super) const ROW_COLUMN_DIACRITICS: [char; 297] = [
    '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}', '\u{0312}', '\u{033D}', '\u{033E}', '\u{033F}',
    '\u{0346}', '\u{034A}', '\u{034B}', '\u{034C}', '\u{0350}', '\u{0351}', '\u{0352}', '\u{0357}',
    '\u{035B}', '\u{0363}', '\u{0364}', '\u{0365}', '\u{0366}', '\u{0367}', '\u{0368}', '\u{0369}',
    '\u{036A}', '\u{036B}', '\u{036C}', '\u{036D}', '\u{036E}', '\u{036F}', '\u{0483}', '\u{0484}',
    '\u{0485}', '\u{0486}', '\u{0487}', '\u{0592}', '\u{0593}', '\u{0594}', '\u{0595}', '\u{0597}',
    '\u{0598}', '\u{0599}', '\u{059C}', '\u{059D}', '\u{059E}', '\u{059F}', '\u{05A0}', '\u{05A1}',
    '\u{05A8}', '\u{05A9}', '\u{05AB}', '\u{05AC}', '\u{05AF}', '\u{05C4}', '\u{0610}', '\u{0611}',
    '\u{0612}', '\u{0613}', '\u{0614}', '\u{0615}', '\u{0616}', '\u{0617}', '\u{0657}', '\u{0658}',
    '\u{0659}', '\u{065A}', '\u{065B}', '\u{065D}', '\u{065E}', '\u{06D6}', '\u{06D7}', '\u{06D8}',
    '\u{06D9}', '\u{06DA}', '\u{06DB}', '\u{06DC}', '\u{06DF}', '\u{06E0}', '\u{06E1}', '\u{06E2}',
    '\u{06E4}', '\u{06E7}', '\u{06E8}', '\u{06EB}', '\u{06EC}', '\u{0730}', '\u{0732}', '\u{0733}',
    '\u{0735}', '\u{0736}', '\u{073A}', '\u{073D}', '\u{073F}', '\u{0740}', '\u{0741}', '\u{0743}',
    '\u{0745}', '\u{0747}', '\u{0749}', '\u{074A}', '\u{07EB}', '\u{07EC}', '\u{07ED}', '\u{07EE}',
    '\u{07EF}', '\u{07F0}', '\u{07F1}', '\u{07F3}', '\u{0816}', '\u{0817}', '\u{0818}', '\u{0819}',
    '\u{081B}', '\u{081C}', '\u{081D}', '\u{081E}', '\u{081F}', '\u{0820}', '\u{0821}', '\u{0822}',
    '\u{0823}', '\u{0825}', '\u{0826}', '\u{0827}', '\u{0829}', '\u{082A}', '\u{082B}', '\u{082C}',
    '\u{082D}', '\u{0951}', '\u{0953}', '\u{0954}', '\u{0F82}', '\u{0F83}', '\u{0F86}', '\u{0F87}',
    '\u{135D}', '\u{135E}', '\u{135F}', '\u{17DD}', '\u{193A}', '\u{1A17}', '\u{1A75}', '\u{1A76}',
    '\u{1A77}', '\u{1A78}', '\u{1A79}', '\u{1A7A}', '\u{1A7B}', '\u{1A7C}', '\u{1B6B}', '\u{1B6D}',
    '\u{1B6E}', '\u{1B6F}', '\u{1B70}', '\u{1B71}', '\u{1B72}', '\u{1B73}', '\u{1CD0}', '\u{1CD1}',
    '\u{1CD2}', '\u{1CDA}', '\u{1CDB}', '\u{1CE0}', '\u{1DC0}', '\u{1DC1}', '\u{1DC3}', '\u{1DC4}',
    '\u{1DC5}', '\u{1DC6}', '\u{1DC7}', '\u{1DC8}', '\u{1DC9}', '\u{1DCB}', '\u{1DCC}', '\u{1DD1}',
    '\u{1DD2}', '\u{1DD3}', '\u{1DD4}', '\u{1DD5}', '\u{1DD6}', '\u{1DD7}', '\u{1DD8}', '\u{1DD9}',
    '\u{1DDA}', '\u{1DDB}', '\u{1DDC}', '\u{1DDD}', '\u{1DDE}', '\u{1DDF}', '\u{1DE0}', '\u{1DE1}',
    '\u{1DE2}', '\u{1DE3}', '\u{1DE4}', '\u{1DE5}', '\u{1DE6}', '\u{1DFE}', '\u{20D0}', '\u{20D1}',
    '\u{20D4}', '\u{20D5}', '\u{20D6}', '\u{20D7}', '\u{20DB}', '\u{20DC}', '\u{20E1}', '\u{20E7}',
    '\u{20E9}', '\u{20F0}', '\u{2CEF}', '\u{2CF0}', '\u{2CF1}', '\u{2DE0}', '\u{2DE1}', '\u{2DE2}',
    '\u{2DE3}', '\u{2DE4}', '\u{2DE5}', '\u{2DE6}', '\u{2DE7}', '\u{2DE8}', '\u{2DE9}', '\u{2DEA}',
    '\u{2DEB}', '\u{2DEC}', '\u{2DED}', '\u{2DEE}', '\u{2DEF}', '\u{2DF0}', '\u{2DF1}', '\u{2DF2}',
    '\u{2DF3}', '\u{2DF4}', '\u{2DF5}', '\u{2DF6}', '\u{2DF7}', '\u{2DF8}', '\u{2DF9}', '\u{2DFA}',
    '\u{2DFB}', '\u{2DFC}', '\u{2DFD}', '\u{2DFE}', '\u{2DFF}', '\u{A66F}', '\u{A67C}', '\u{A67D}',
    '\u{A6F0}', '\u{A6F1}', '\u{A8E0}', '\u{A8E1}', '\u{A8E2}', '\u{A8E3}', '\u{A8E4}', '\u{A8E5}',
    '\u{A8E6}', '\u{A8E7}', '\u{A8E8}', '\u{A8E9}', '\u{A8EA}', '\u{A8EB}', '\u{A8EC}', '\u{A8ED}',
    '\u{A8EE}', '\u{A8EF}', '\u{A8F0}', '\u{A8F1}', '\u{AAB0}', '\u{AAB2}', '\u{AAB3}', '\u{AAB7}',
    '\u{AAB8}', '\u{AABE}', '\u{AABF}', '\u{AAC1}', '\u{FE20}', '\u{FE21}', '\u{FE22}', '\u{FE23}',
    '\u{FE24}', '\u{FE25}', '\u{FE26}', '\u{10A0F}', '\u{10A38}', '\u{1D185}', '\u{1D186}', '\u{1D187}',
    '\u{1D188}', '\u{1D189}', '\u{1D1AA}', '\u{1D1AB}', '\u{1D1AC}', '\u{1D1AD}', '\u{1D242}', '\u{1D243}',
    '\u{1D244}',
];

/// The most rows or columns a placeholder grid can address.
pub(super) const MAX_GRID_CELLS: u32 = ROW_COLUMN_DIACRITICS.len() as u32;

/// The placeholder grid for image `id` over `columns` x `rows` cells, one
/// string per row. Each row sets the foreground to the ID and resets it to
/// the default (`SGR 39`) at its end, so text after any row keeps its color.
///
/// Returns `None` when the grid cannot be encoded: an empty grid, more
/// than [`MAX_GRID_CELLS`] rows or columns, or an ID that is zero or
/// above [`MAX_KITTY_ID`].
pub(super) fn placeholder_rows(id: u32, columns: u32, rows: u32) -> Option<Vec<String>> {
    let fits = |n: u32| (1..=MAX_GRID_CELLS).contains(&n);
    if !fits(columns) || !fits(rows) || id == 0 || id > MAX_KITTY_ID {
        return None;
    }
    let color = format!("\x1b[38;2;{};{};{}m", id >> 16, (id >> 8) & 0xFF, id & 0xFF);
    let grid = (0..rows as usize)
        .map(|row| {
            let mut line = color.clone();
            for &column in ROW_COLUMN_DIACRITICS.iter().take(columns as usize) {
                line.push(PLACEHOLDER);
                line.push(ROW_COLUMN_DIACRITICS[row]);
                line.push(column);
            }
            line.push_str("\x1b[39m");
            line
        })
        .collect();
    Some(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One cell: the placeholder, then the row and column marks.
    fn cell(row: usize, column: usize) -> String {
        [PLACEHOLDER, ROW_COLUMN_DIACRITICS[row], ROW_COLUMN_DIACRITICS[column]]
            .iter()
            .collect()
    }

    #[test]
    fn a_one_by_one_grid_is_one_cell_between_the_id_color_and_its_reset() {
        assert_eq!(
            placeholder_rows(42, 1, 1).unwrap(),
            vec!["\x1b[38;2;0;0;42m\u{10EEEE}\u{0305}\u{0305}\x1b[39m".to_string()]
        );
    }

    #[test]
    fn a_wide_grid_numbers_its_columns() {
        let rows = placeholder_rows(1, 4, 1).unwrap();
        let expected: String = (0..4).map(|column| cell(0, column)).collect();
        assert_eq!(rows, vec![format!("\x1b[38;2;0;0;1m{expected}\x1b[39m")]);
    }

    #[test]
    fn a_tall_grid_numbers_its_rows_and_resets_color_on_every_row() {
        let rows = placeholder_rows(1, 1, 3).unwrap();
        assert_eq!(rows.len(), 3);
        for (row, line) in rows.iter().enumerate() {
            assert_eq!(line, &format!("\x1b[38;2;0;0;1m{}\x1b[39m", cell(row, 0)));
        }
    }

    #[test]
    fn the_id_is_the_24_bit_foreground() {
        let rows = placeholder_rows(0x12_34_56, 1, 1).unwrap();
        assert!(rows[0].starts_with("\x1b[38;2;18;52;86m"), "{:?}", rows[0]);
        let rows = placeholder_rows(MAX_KITTY_ID, 1, 1).unwrap();
        assert!(rows[0].starts_with("\x1b[38;2;255;255;255m"), "{:?}", rows[0]);
    }

    #[test]
    fn the_table_bounds_rows_and_columns() {
        let grid = placeholder_rows(7, MAX_GRID_CELLS, MAX_GRID_CELLS).unwrap();
        assert_eq!(grid.len(), 297);
        assert!(grid[296].ends_with(&format!("{}\x1b[39m", cell(296, 296))));
        assert_eq!(placeholder_rows(7, MAX_GRID_CELLS + 1, 1), None);
        assert_eq!(placeholder_rows(7, 1, MAX_GRID_CELLS + 1), None);
    }

    #[test]
    fn empty_grids_and_unencodable_ids_are_refused() {
        assert_eq!(placeholder_rows(7, 0, 1), None);
        assert_eq!(placeholder_rows(7, 1, 0), None);
        assert_eq!(placeholder_rows(0, 1, 1), None);
        assert_eq!(placeholder_rows(MAX_KITTY_ID + 1, 1, 1), None);
    }

    #[test]
    fn the_table_matches_kittys_published_ends_and_has_no_duplicates() {
        assert_eq!(ROW_COLUMN_DIACRITICS[0], '\u{0305}');
        assert_eq!(ROW_COLUMN_DIACRITICS[1], '\u{030D}');
        assert_eq!(ROW_COLUMN_DIACRITICS[296], '\u{1D244}');
        let mut sorted = ROW_COLUMN_DIACRITICS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ROW_COLUMN_DIACRITICS.len());
    }
}
