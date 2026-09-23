//! Level 2 tests for `sniff repo recent-commits` rendered in a real terminal.
//!
//! The library authors the report as Prose and the CLI renders it once with
//! word wrapping. Level 1 pins the Prose text and a piped render, but only an
//! emulator shows what actually reaches the character grid, and the styling
//! table in `sniff/docs/topics/repo/recent-commits.md` § Styling is a claim
//! about the grid: which *span* carries bold, blue, blue+dim, or italic, and
//! which span carries a hyperlink. Two tests split that:
//!
//! - wrapping, in a narrow pane, because wrapping is a function of width;
//! - the style-to-span mapping and both link targets, in a pane wide enough
//!   to hold the linked header on one row.
//!
//! The mapping test decodes each captured row into cells carrying the SGR
//! attributes and OSC8 target in force at that character, so an assertion
//! names a token and the style it must carry — not merely that some bold byte
//! occurred somewhere in the frame.
//!
//! Skip-clean: when tmux is unavailable the tests print a skip notice and
//! pass. Gated behind `test-fixtures`, which pulls in `biscuit-test-harness`
//! and `common::capture_until`; the `test-l2` recipe enables it.
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use std::path::Path;
use std::time::Duration;
use test_toolkit::{Backend, Level, require_level};

use crate::common;

const RENDER_DEADLINE: Duration = Duration::from_secs(15);
const PANE_COLS: u32 = 60;
const PANE_ROWS: u32 = 50;
const MARKER: &str = "RECENT-COMMITS-DONE";

const DESCRIPTION: &str = "This deliberately long description paragraph must wrap \
across several rows of a sixty column pane without splitting any word in half.";

/// A repository with one verbose conventional commit whose body wraps.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/main.rs")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Test", "test@test.com").unwrap();
    let message = format!("feat(cli): render reports. {DESCRIPTION}\n\n- keep the first bullet");
    repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[])
        .unwrap();
    dir
}

#[test]
fn level2_recent_commits_verbose_report_wraps_styles_and_links_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let repository = repository();
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux spawn_shell failed");
    harness.resize(PANE_COLS, PANE_ROWS).expect("tmux resize failed");

    // The environment prefix lives in the command string so it binds to
    // `sniff`, not to `clear`.
    let binary = cargo_bin("sniff").display().to_string();
    harness
        .send_command_with_env(
            &format!(
                "clear; FORCE_COLOR=1 '{binary}' --base '{}' repo recent-commits -v; printf '\\n%s\\n' {MARKER}",
                repository.path().display()
            ),
            &[],
        )
        .expect("send_command_with_env failed");

    let frame = common::capture_until(&mut harness, RENDER_DEADLINE, |frame| {
        frame.plain.lines().any(|line| line.trim() == MARKER)
            && frame.plain.contains("Files Impacted:")
    });
    let plain = &frame.plain;
    let report_start = plain
        .find("- [")
        .unwrap_or_else(|| panic!("no commit header in pane:\n{plain}"));
    let report = &plain[report_start..];

    // Every word of the description survives whole: a mid-word soft wrap by
    // the terminal would split one into two tokens.
    let report_words: Vec<&str> = report.split_whitespace().collect();
    let expected: Vec<&str> = DESCRIPTION.split_whitespace().collect();
    assert!(
        report_words
            .windows(expected.len())
            .any(|window| window == expected.as_slice()),
        "description must wrap at word boundaries:\n{plain}"
    );
    let description_rows = report
        .lines()
        .filter(|line| {
            expected
                .iter()
                .any(|word| line.split_whitespace().any(|token| token == *word))
        })
        .count();
    assert!(description_rows > 1, "fixture must actually wrap:\n{plain}");

    for evidence in ["] feat(cli) at ", ": render reports", "Details:", "- keep the first bullet"] {
        assert!(report.contains(evidence), "expected {evidence:?} in pane:\n{plain}");
    }
    for leaked in ["<bold>", "<blue>", "<italic>", "\\[", "\\_", "**"] {
        assert!(!report.contains(leaked), "{leaked:?} leaked into pane:\n{plain}");
    }
}

// ---------------------------------------------------------------------------
// Style-to-span mapping
// ---------------------------------------------------------------------------

/// Wide enough that the linked header, whose URL is one unbreakable token,
/// occupies a single row; per-row decoding is what ties a style to a span.
const WIDE_PANE_COLS: u32 = 200;
const LINKED_MARKER: &str = "LINKED-COMMITS-DONE";
/// Noon UTC on a fixed date: the rendered day label stays in 2026 whatever
/// timezone the host judges it in.
const COMMIT_EPOCH_SECONDS: i64 = 1_775_044_800;

/// The fixture repository plus the values the report must derive from it.
struct LinkedRepository {
    dir: tempfile::TempDir,
    short_hash: String,
    commit_url: String,
    file_url: String,
}

/// A repository whose single commit is contained by a locally recorded
/// `refs/remotes/origin/main`.
///
/// Containment is decided from local Git data alone, so writing the
/// remote-tracking ref and a configured remote URL is the whole setup: the
/// report gains a `commit_url` without any network access.
fn linked_repository() -> LinkedRepository {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    repo.remote("origin", "https://github.com/o/r.git").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("src/main.rs")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let when = git2::Time::new(COMMIT_EPOCH_SECONDS, 0);
    let signature = git2::Signature::new("Test", "test@test.com", &when).unwrap();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "feat(cli): render reports. Short body.\n\n- keep the first bullet",
            &tree,
            &[],
        )
        .unwrap();
    repo.reference("refs/remotes/origin/main", oid, true, "fixture remote tip")
        .unwrap();

    // Sniff reports the repository root resolved, which on macOS differs from
    // the `TempDir` path (`/var` is a symlink to `/private/var`).
    let root = dir.path().canonicalize().unwrap();
    LinkedRepository {
        short_hash: oid.to_string().chars().take(7).collect(),
        commit_url: format!("https://github.com/o/r/commit/{oid}"),
        file_url: url::Url::from_file_path(root.join("src/main.rs"))
            .unwrap()
            .to_string(),
        dir,
    }
}

#[test]
fn level2_recent_commits_styles_and_links_map_to_their_spans_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = linked_repository();
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux spawn_shell failed");
    harness
        .resize(WIDE_PANE_COLS, PANE_ROWS)
        .expect("tmux resize failed");

    let binary = cargo_bin("sniff").display().to_string();
    harness
        .send_command_with_env(
            &format!(
                "clear; FORCE_COLOR=1 '{binary}' --base '{}' repo recent-commits -v; printf '\\n%s\\n' {LINKED_MARKER}",
                fixture.dir.path().display()
            ),
            &[],
        )
        .expect("send_command_with_env failed");

    let frame = common::capture_until(&mut harness, RENDER_DEADLINE, |frame| {
        frame.plain.lines().any(|line| line.trim() == LINKED_MARKER)
            && frame.plain.contains("Files Impacted:")
            && frame.plain.contains(&fixture.commit_url)
    });
    let rows: Vec<Row> = frame.raw.lines().map(Row::decode).collect();
    let raw = &frame.raw;

    // Whether the backend can carry OSC8 decides the *form* of both links, not
    // whether they exist. Each branch below pins an exact target either way,
    // and `proofs` keeps the capability gate from quietly proving nothing.
    let osc8 = raw.contains("\u{1b}]8;;");
    let mut proofs: Vec<&str> = Vec::new();

    // -- Header: hash, operation, scope, `at`, time, heading ----------------

    let header = rows
        .iter()
        .find(|row| row.text.starts_with("- ["))
        .unwrap_or_else(|| panic!("no commit header row in pane:\n{raw}"));

    let bold = header.run_spans(|attrs| attrs.bold);
    assert_eq!(
        bold.len(),
        2,
        "the header carries bold on exactly the hash and the time label, got {:?}:\n{raw}",
        header.runs(|attrs| attrs.bold)
    );
    let (hash, time_label) = (&bold[0], &bold[1]);
    assert_eq!(
        header.slice(hash),
        fixture.short_hash,
        "the first bold span is the short hash:\n{raw}"
    );
    let time_label_text = header.slice(time_label).to_string();

    assert_eq!(
        header.runs(|attrs| attrs.italic),
        vec!["at".to_string()],
        "italic covers the word `at` and nothing else:\n{raw}"
    );
    assert_eq!(
        header.runs(|attrs| attrs.fg.is_blue()),
        vec!["feat(cli)".to_string()],
        "blue covers the operation and the parenthesized scope, and stops there:\n{raw}"
    );
    assert_eq!(
        header.runs(|attrs| attrs.dim),
        vec!["cli".to_string()],
        "dim covers the scope alone, so `feat`, `(` and `)` stay blue-not-dim:\n{raw}"
    );

    // The four run assertions above already leave the brackets outside every
    // styled run; what remains is that the hash's URL sits immediately around
    // the bold span rather than anywhere in the frame.
    let (opening, closing) = if osc8 {
        ("- [", "] ".to_string())
    } else {
        ("- [[", format!("]({})] ", fixture.commit_url))
    };
    assert_eq!(
        header.text[..hash.start].to_string(),
        opening,
        "nothing but the unstyled brackets precedes the hash:\n{raw}"
    );
    assert!(
        header.text[hash.end..].starts_with(&closing),
        "expected {closing:?} directly after the hash:\n{raw}"
    );
    if osc8 {
        assert_eq!(
            header.links(),
            vec![(fixture.short_hash.clone(), fixture.commit_url.clone())],
            "the hash, and only the hash, carries the commit URL:\n{raw}"
        );
        proofs.push("hash URL as an OSC8 target on the hash span");
    } else {
        proofs.push("hash URL as the documented [hash](url) fallback beside the hash span");
    }

    // The bold time label is bounded by `at ` before it and the unstyled `: `
    // separator and heading after it.
    let tail = ": render reports";
    assert_eq!(
        header.text[..time_label.start].to_string(),
        format!("{opening}{}{closing}feat(cli) at ", fixture.short_hash),
        "the bold time label begins right after `at `:\n{raw}"
    );
    assert_eq!(
        header.text[time_label.end..].to_string(),
        tail,
        "the bold time label ends right before the `: ` separator:\n{raw}"
    );
    let parts: Vec<&str> = time_label_text.split(' ').collect();
    assert_eq!(
        parts.len(),
        2,
        "the time label is `{{time}} {{day}}`: {time_label_text:?}"
    );
    assert!(
        is_time_of_day(parts[0]),
        "the time label opens with a 12-hour clock: {time_label_text:?}"
    );
    assert!(
        parts[1].contains("2026"),
        "the fixture's fixed commit date reached the label: {time_label_text:?}"
    );

    // -- Section labels, bullets, and the changed file ----------------------

    for label in ["Details:", "Files Impacted:"] {
        let row = rows
            .iter()
            .find(|row| row.text.trim() == label)
            .unwrap_or_else(|| panic!("no {label:?} row in pane:\n{raw}"));
        assert_eq!(
            row.runs(|attrs| attrs.bold),
            vec![label.to_string()],
            "the {label:?} label is bold and its indent is not:\n{raw}"
        );
        assert!(
            row.runs(|attrs| attrs.italic || attrs.dim || attrs.fg.is_blue()).is_empty(),
            "the {label:?} label carries bold alone:\n{raw}"
        );
    }

    let bullet = rows
        .iter()
        .find(|row| row.text.trim() == "- keep the first bullet")
        .unwrap_or_else(|| panic!("no bullet row in pane:\n{raw}"));
    assert!(
        bullet.cells.iter().all(|cell| cell.attrs == Attrs::default()),
        "bullet points are unstyled:\n{raw}"
    );

    let file_row = rows
        .iter()
        .find(|row| row.text.trim_start().starts_with("- added:"))
        .unwrap_or_else(|| panic!("no changed-file row in pane:\n{raw}"));
    assert!(
        file_row.cells.iter().all(|cell| cell.attrs == Attrs::default()),
        "a changed file is a link, not a styled span:\n{raw}"
    );
    if osc8 {
        assert_eq!(
            file_row.links(),
            vec![("src/main.rs".to_string(), fixture.file_url.clone())],
            "the file path carries its own file:// target:\n{raw}"
        );
        proofs.push("file URL as an OSC8 target on the path span");
    } else {
        assert_eq!(
            file_row.text.trim(),
            format!("- added: [src/main.rs]({})", fixture.file_url),
            "the file path degrades to the documented [path](url) fallback:\n{raw}"
        );
        proofs.push("file URL as the documented [path](url) fallback");
    }

    // A capability gate that grew a third, assert-nothing branch would land
    // here rather than pass as a silently weaker test.
    assert_eq!(
        proofs.len(),
        2,
        "both link targets must be proved in whichever form the backend supports"
    );
    eprintln!("level2 link evidence (osc8 = {osc8}): {proofs:?}");
}

/// `6:05pm`, the clock half of a `{time} {day}` label.
fn is_time_of_day(token: &str) -> bool {
    let Some(clock) = token.strip_suffix("am").or_else(|| token.strip_suffix("pm")) else {
        return false;
    };
    let Some((hour, minute)) = clock.split_once(':') else {
        return false;
    };
    (1..=12).contains(&hour.parse::<u32>().unwrap_or(0))
        && minute.len() == 2
        && minute.chars().all(|character| character.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Captured-row decoding
// ---------------------------------------------------------------------------

/// Foreground color in force at a character.
///
/// Only the distinction the styling table makes is modeled: blue covers the
/// 4-bit, bright, and 256-color spellings a terminal may pick for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Fg {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Fg {
    fn is_blue(self) -> bool {
        matches!(self, Fg::Indexed(4 | 12))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct Attrs {
    bold: bool,
    dim: bool,
    italic: bool,
    fg: Fg,
}

#[derive(Clone, Debug)]
struct Cell {
    character: char,
    attrs: Attrs,
    link: Option<String>,
}

/// One captured row as visible text plus the style state at each character.
///
/// tmux re-emits a row's attributes as SGR runs rather than reproducing the
/// byte sequence the program wrote, so assertions read the decoded state, not
/// the escape spelling.
#[derive(Clone, Debug)]
struct Row {
    text: String,
    cells: Vec<Cell>,
    /// Byte offset of each cell in `text`, with `text.len()` as a sentinel.
    offsets: Vec<usize>,
}

/// Byte offsets of a token inside [`Row::text`].
struct Span {
    start: usize,
    end: usize,
}

impl Row {
    fn decode(raw: &str) -> Self {
        let mut attrs = Attrs::default();
        let mut link: Option<String> = None;
        let mut cells = Vec::new();
        let mut characters = raw.chars();
        while let Some(character) = characters.next() {
            if character != '\u{1b}' {
                cells.push(Cell {
                    character,
                    attrs,
                    link: link.clone(),
                });
                continue;
            }
            match characters.next() {
                Some('[') => {
                    let mut parameters = String::new();
                    let mut terminator = None;
                    for character in characters.by_ref() {
                        if character.is_ascii_alphabetic() {
                            terminator = Some(character);
                            break;
                        }
                        parameters.push(character);
                    }
                    if terminator == Some('m') {
                        apply_sgr(&mut attrs, &parameters);
                    }
                }
                Some(']') => {
                    let mut body = String::new();
                    while let Some(character) = characters.next() {
                        match character {
                            '\u{7}' => break,
                            // String Terminator: ESC \
                            '\u{1b}' => {
                                characters.next();
                                break;
                            }
                            _ => body.push(character),
                        }
                    }
                    if let Some(rest) = body.strip_prefix("8;") {
                        let target = rest.split_once(';').map_or("", |(_, url)| url);
                        link = (!target.is_empty()).then(|| target.to_string());
                    }
                }
                _ => {}
            }
        }
        let text: String = cells.iter().map(|cell| cell.character).collect();
        let mut offsets = Vec::with_capacity(cells.len() + 1);
        let mut byte = 0;
        for cell in &cells {
            offsets.push(byte);
            byte += cell.character.len_utf8();
        }
        offsets.push(byte);
        Self {
            text,
            cells,
            offsets,
        }
    }

    /// The maximal runs of consecutive characters whose style satisfies
    /// `selector`, as offsets into [`Row::text`] and in order.
    fn run_spans(&self, selector: impl Fn(&Attrs) -> bool) -> Vec<Span> {
        let mut spans: Vec<Span> = Vec::new();
        for (index, cell) in self.cells.iter().enumerate() {
            if !selector(&cell.attrs) {
                continue;
            }
            let (start, end) = (self.offsets[index], self.offsets[index + 1]);
            match spans.last_mut() {
                Some(span) if span.end == start => span.end = end,
                _ => spans.push(Span { start, end }),
            }
        }
        spans
    }

    /// [`Row::run_spans`] as text, for assertions that name the token.
    fn runs(&self, selector: impl Fn(&Attrs) -> bool) -> Vec<String> {
        self.run_spans(selector)
            .iter()
            .map(|span| self.slice(span).to_string())
            .collect()
    }

    fn slice(&self, span: &Span) -> &str {
        &self.text[span.start..span.end]
    }

    /// The maximal `(text, target)` runs of characters sharing one OSC8 link.
    fn links(&self) -> Vec<(String, String)> {
        let mut links: Vec<(String, String)> = Vec::new();
        for cell in &self.cells {
            let Some(target) = &cell.link else { continue };
            match links.last_mut() {
                Some((text, last)) if last == target => text.push(cell.character),
                _ => links.push((cell.character.to_string(), target.clone())),
            }
        }
        links
    }
}

fn apply_sgr(attrs: &mut Attrs, parameters: &str) {
    let codes: Vec<u16> = if parameters.is_empty() {
        vec![0]
    } else {
        parameters
            .split([';', ':'])
            .map(|code| code.parse::<u16>().unwrap_or(0))
            .collect()
    };
    let mut index = 0;
    while index < codes.len() {
        match codes[index] {
            0 => *attrs = Attrs::default(),
            1 => attrs.bold = true,
            2 => attrs.dim = true,
            3 => attrs.italic = true,
            22 => {
                attrs.bold = false;
                attrs.dim = false;
            }
            23 => attrs.italic = false,
            code @ 30..=37 => attrs.fg = Fg::Indexed((code - 30) as u8),
            code @ 90..=97 => attrs.fg = Fg::Indexed((code - 90 + 8) as u8),
            39 => attrs.fg = Fg::Default,
            38 => match codes.get(index + 1) {
                Some(5) => {
                    if let Some(&color) = codes.get(index + 2) {
                        attrs.fg = Fg::Indexed(color as u8);
                    }
                    index += 2;
                }
                Some(2) => {
                    if let (Some(&r), Some(&g), Some(&b)) = (
                        codes.get(index + 2),
                        codes.get(index + 3),
                        codes.get(index + 4),
                    ) {
                        attrs.fg = Fg::Rgb(r as u8, g as u8, b as u8);
                    }
                    index += 4;
                }
                _ => {}
            },
            _ => {}
        }
        index += 1;
    }
}
