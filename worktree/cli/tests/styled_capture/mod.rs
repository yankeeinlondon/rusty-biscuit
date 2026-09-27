//! Parses a styled pane capture (`tmux capture-pane -e`) into per-cell style
//! state, so Level 2 tests can assert the style of one glyph or word rather
//! than searching the raw bytes.
//!
//! tmux re-encodes SGR when it captures: it merges attributes (`2;3m`),
//! resets with `0m`, and may emit a 256-color index for a truecolor value when
//! its server has no RGB support. [`Color::matches`] accepts that index for an
//! expected RGB color when it is the index tmux itself would choose.

#![allow(dead_code)]

/// A cell color as the capture spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// A palette index: `30`–`37` and `90`–`97` map to 0–15, `38;5;n` to `n`.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    /// Whether this captured color is `expected`, allowing the 256-color
    /// downgrade of an RGB value.
    pub fn matches(self, expected: Color) -> bool {
        match (self, expected) {
            (Color::Rgb(..), Color::Rgb(..)) | (Color::Indexed(_), Color::Indexed(_)) => {
                self == expected
            }
            (Color::Indexed(index), Color::Rgb(r, g, b)) => index == nearest_256(r, g, b),
            (Color::Rgb(..), Color::Indexed(_)) => false,
        }
    }
}

/// The SGR state of one cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub strikethrough: bool,
}

impl Style {
    pub fn fg_is(&self, expected: Color) -> bool {
        self.fg.is_some_and(|fg| fg.matches(expected))
    }

    pub fn bg_is(&self, expected: Color) -> bool {
        self.bg.is_some_and(|bg| bg.matches(expected))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

/// One captured screen: rows of styled cells.
pub struct StyledScreen {
    pub rows: Vec<Vec<Cell>>,
}

impl StyledScreen {
    /// Parses `raw`. Style carries across line ends, as it does on screen;
    /// OSC strings (hyperlinks) and non-SGR CSI sequences are dropped.
    pub fn parse(raw: &str) -> Self {
        let mut rows = vec![Vec::new()];
        let mut style = Style::default();
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\x1b' => match chars.next() {
                    Some('[') => {
                        let mut params = String::new();
                        let mut last = None;
                        for next in chars.by_ref() {
                            if ('\x40'..='\x7e').contains(&next) {
                                last = Some(next);
                                break;
                            }
                            params.push(next);
                        }
                        if last == Some('m') {
                            apply_sgr(&mut style, &params);
                        }
                    }
                    Some(']') => {
                        // OSC: ends at BEL or ST (`ESC \`).
                        while let Some(next) = chars.next() {
                            if next == '\x07' {
                                break;
                            }
                            if next == '\x1b' && chars.peek() == Some(&'\\') {
                                chars.next();
                                break;
                            }
                        }
                    }
                    _ => {}
                },
                '\n' => rows.push(Vec::new()),
                '\r' => {}
                _ => rows.last_mut().expect("a row").push(Cell { ch, style }),
            }
        }
        Self { rows }
    }

    /// The visible text of `row`.
    pub fn text(&self, row: usize) -> String {
        self.rows[row].iter().map(|cell| cell.ch).collect()
    }

    /// The visible text of the whole screen.
    pub fn plain(&self) -> String {
        (0..self.rows.len()).map(|row| self.text(row)).collect::<Vec<_>>().join("\n")
    }

    /// The last row whose text contains every needle.
    pub fn row_with(&self, needles: &[&str]) -> usize {
        (0..self.rows.len())
            .rev()
            .find(|&row| {
                let text = self.text(row);
                needles.iter().all(|needle| text.contains(needle))
            })
            .unwrap_or_else(|| panic!("no row contains {needles:?}.\nscreen:\n{}", self.plain()))
    }

    /// The cells of the first occurrence of `needle` in `row`.
    pub fn span(&self, row: usize, needle: &str) -> &[Cell] {
        let cells = &self.rows[row];
        let wanted: Vec<char> = needle.chars().collect();
        let start = cells
            .windows(wanted.len())
            .position(|window| window.iter().map(|cell| cell.ch).eq(wanted.iter().copied()))
            .unwrap_or_else(|| panic!("{needle:?} not in row {row}: {:?}", self.text(row)));
        &cells[start..start + wanted.len()]
    }

    /// Asserts that every cell of `needle` in `row` satisfies `check`.
    pub fn assert_span(&self, row: usize, needle: &str, what: &str, check: impl Fn(&Style) -> bool) {
        for cell in self.span(row, needle) {
            assert!(
                check(&cell.style),
                "{needle:?} should be {what}, but {:?} has {:?}.\nrow: {:?}",
                cell.ch,
                cell.style,
                self.text(row),
            );
        }
    }
}

fn apply_sgr(style: &mut Style, params: &str) {
    // Each `;` group is one attribute, except that `38;5;n` and `38;2;r;g;b`
    // span several; the ITU `38:2::r:g:b` form keeps its parts in one group.
    let groups: Vec<&str> = if params.is_empty() { vec!["0"] } else { params.split(';').collect() };
    let mut index = 0;
    while index < groups.len() {
        let group = groups[index];
        index += 1;
        if group.contains(':') {
            let parts: Vec<u16> = group.split(':').map(|part| part.parse().unwrap_or(0)).collect();
            let color = match parts.get(1) {
                Some(5) => parts.get(2).map(|&n| Color::Indexed(n as u8)),
                // `38:2:<colorspace>:r:g:b` or the short `38:2:r:g:b`.
                Some(2) if parts.len() >= 6 => {
                    Some(Color::Rgb(parts[3] as u8, parts[4] as u8, parts[5] as u8))
                }
                Some(2) if parts.len() == 5 => {
                    Some(Color::Rgb(parts[2] as u8, parts[3] as u8, parts[4] as u8))
                }
                _ => None,
            };
            match parts.first() {
                Some(38) => style.fg = color,
                Some(48) => style.bg = color,
                _ => {}
            }
            continue;
        }
        let code: u16 = group.parse().unwrap_or(0);
        match code {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            9 => style.strikethrough = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            29 => style.strikethrough = false,
            30..=37 => style.fg = Some(Color::Indexed((code - 30) as u8)),
            39 => style.fg = None,
            40..=47 => style.bg = Some(Color::Indexed((code - 40) as u8)),
            49 => style.bg = None,
            90..=97 => style.fg = Some(Color::Indexed((code - 90 + 8) as u8)),
            100..=107 => style.bg = Some(Color::Indexed((code - 100 + 8) as u8)),
            38 | 48 => {
                let number = |at: usize| groups.get(at).and_then(|g| g.parse::<u8>().ok()).unwrap_or(0);
                let color = match groups.get(index).copied() {
                    Some("5") => {
                        let color = Color::Indexed(number(index + 1));
                        index += 2;
                        color
                    }
                    Some("2") => {
                        let color = Color::Rgb(number(index + 1), number(index + 2), number(index + 3));
                        index += 4;
                        color
                    }
                    _ => continue,
                };
                if code == 38 {
                    style.fg = Some(color);
                } else {
                    style.bg = Some(color);
                }
            }
            _ => {}
        }
    }
}

/// The 256-color index tmux picks for an RGB value (`colour_find_rgb`): the
/// nearer of the 6x6x6 cube entry and the gray ramp entry.
fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    const CUBE: [i32; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
    let to_cube = |v: u8| -> usize {
        let v = i32::from(v);
        if v < 48 {
            0
        } else if v < 114 {
            1
        } else {
            ((v - 35) / 40) as usize
        }
    };
    let (qr, qg, qb) = (to_cube(r), to_cube(g), to_cube(b));
    let (cr, cg, cb) = (CUBE[qr], CUBE[qg], CUBE[qb]);
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    if (cr, cg, cb) == (r, g, b) {
        return (16 + 36 * qr + 6 * qg + qb) as u8;
    }
    let average = (r + g + b) / 3;
    let gray_index = if average > 238 { 23 } else { ((average - 3).max(0) / 10) as usize };
    let gray = 8 + 10 * gray_index as i32;
    let distance = |x: i32, y: i32, z: i32| (x - r).pow(2) + (y - g).pow(2) + (z - b).pow(2);
    if distance(gray, gray, gray) < distance(cr, cg, cb) {
        (232 + gray_index) as u8
    } else {
        (16 + 36 * qr + 6 * qg + qb) as u8
    }
}
