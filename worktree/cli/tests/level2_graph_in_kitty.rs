//! Level 2 tests for the `wt list` graph as Kitty draws it.
//!
//! Each test starts a private, unfocused Kitty at a fixed cell size
//! (`KittyInstance`), runs `wt list` from the main checkout under `script` so
//! the bytes `wt` sent are recorded, and checks three records against each
//! other: the text Kitty holds (`kitty @ get-text`), the image `wt`
//! transmitted, and a screenshot of the window. The sizing rule itself is
//! covered at L1 in biscuit-terminal's `git_graph` tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use base64::Engine;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::kitty::KittyInstance;
use image::RgbaImage;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Branches forked from `main`, each in its own worktree with its own commit,
/// so the base view has one lane per branch.
const BRANCHES: [&str; 5] = ["alpha", "bravo", "charlie", "delta", "echo"];

/// A pixel counts as drawn when a channel is brighter than this. Kitty's
/// default background (`--config NONE`) is black.
const DRAWN: u8 = 48;

/// Screenshot edges may differ from the transmitted image by antialiasing and
/// the capture's color conversion.
const EDGE_TOLERANCE_PX: i64 = 3;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

/// A main checkout on `main` with a linked worktree per branch in
/// [`BRANCHES`]. `main` gains a commit after each fork, so every lane leaves
/// the default lane at a different commit.
struct Fixture {
    parent: tempfile::TempDir,
    main: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let parent = tempfile::tempdir().expect("create parent temp dir");
        let main = parent.path().join("main");
        fs::create_dir_all(&main).unwrap();
        fs::create_dir_all(parent.path().join("home")).unwrap();

        run_git(&main, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("gc.auto", "0"),
            ("core.fsmonitor", "false"),
            ("core.commitGraph", "false"),
        ] {
            run_git(&main, &["config", key, value]);
        }
        let commit = |dir: &Path, file: &str, message: &str| {
            fs::write(dir.join(file), message).unwrap();
            run_git(dir, &["add", "."]);
            run_git(dir, &["commit", "-m", message]);
        };
        commit(&main, "main.txt", "main 1");
        commit(&main, "main.txt", "main 2");
        for branch in BRANCHES {
            let worktree = parent.path().join(format!("wt-{branch}"));
            run_git(&main, &["worktree", "add", worktree.to_str().unwrap(), "-b", branch]);
            commit(&worktree, &format!("{branch}.txt"), &format!("work on {branch}"));
            commit(&main, "main.txt", &format!("main after {branch}"));
        }
        Self { parent, main }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.parent.path().join(name)
    }
}

/// What `wt` sent for the graph: the Kitty APC's `c=` columns, the rows it
/// then moved the cursor down (`CSI <rows> B`), and the decoded PNG.
struct Transmitted {
    columns: u32,
    rows: u32,
    png: RgbaImage,
}

/// Pulls the graph image out of a `script` recording of `wt list`.
fn parse_transmitted(recording: &[u8]) -> Transmitted {
    let text = String::from_utf8_lossy(recording);
    let start = text.find("\x1b_G").unwrap_or_else(|| panic!("wt sent no Kitty image.\n{text:.2000}"));
    let mut rest = &text[start..];
    let mut columns = None;
    let mut payload = String::new();
    while let Some(body) = rest.strip_prefix("\x1b_G") {
        let end = body.find("\x1b\\").expect("unterminated Kitty APC");
        let (params, data) = body[..end].split_once(';').expect("Kitty APC without payload");
        for param in params.split(',') {
            if let Some(value) = param.strip_prefix("c=") {
                columns = Some(value.parse::<u32>().expect("numeric c="));
            }
            assert!(!param.starts_with("r="), "wt should let Kitty derive the rows: {params}");
        }
        payload.push_str(data);
        rest = &body[end + 2..];
    }
    // `CSI u` restores the cursor to the image's top row; `CSI <rows> B` then
    // skips the rows the image covers.
    let rows = rest
        .strip_prefix("\x1b[u\x1b[")
        .and_then(|after| after.split_once('B'))
        .and_then(|(rows, _)| rows.parse::<u32>().ok())
        .unwrap_or_else(|| panic!("no cursor move after the image: {:?}", &rest[..rest.len().min(40)]));
    let png = base64::engine::general_purpose::STANDARD.decode(payload).expect("base64 image payload");
    Transmitted {
        columns: columns.expect("Kitty APC without c="),
        rows,
        png: image::load_from_memory(&png).expect("PNG image payload").to_rgba8(),
    }
}

/// Bounding box `(left, top, right, bottom)`, exclusive at the far edges.
type BoundingBox = (i64, i64, i64, i64);

/// The drawn pixels' bounding box inside `region`, after `color` maps a pixel
/// to what it looks like on a black background.
fn drawn_box(image: &RgbaImage, region: BoundingBox, color: impl Fn(&image::Rgba<u8>) -> [u8; 3]) -> Option<BoundingBox> {
    let (left, top, right, bottom) = region;
    let mut found: Option<BoundingBox> = None;
    for y in top.max(0)..bottom.min(image.height() as i64) {
        for x in left.max(0)..right.min(image.width() as i64) {
            if color(image.get_pixel(x as u32, y as u32)).iter().any(|&c| c > DRAWN) {
                let (l, t, r, b) = found.unwrap_or((x, y, x + 1, y + 1));
                found = Some((l.min(x), t.min(y), r.max(x + 1), b.max(y + 1)));
            }
        }
    }
    found
}

fn over_black(pixel: &image::Rgba<u8>) -> [u8; 3] {
    let [r, g, b, a] = pixel.0;
    let blend = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
    [blend(r), blend(g), blend(b)]
}

fn opaque(pixel: &image::Rgba<u8>) -> [u8; 3] {
    [pixel.0[0], pixel.0[1], pixel.0[2]]
}

/// Splits `get-text` lines into screen rows: Kitty returns a soft-wrapped
/// line whole. Every glyph `wt list` prints here is one cell wide.
fn screen_rows(text: &str, columns: u32) -> Vec<String> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            rows.push(String::new());
        }
        rows.extend(chars.chunks(columns as usize).map(|row| row.iter().collect::<String>()));
    }
    rows
}

/// One `wt list` in a private Kitty window `columns` × `lines` cells.
struct GraphRun {
    columns: u32,
    lines: u32,
    /// Cell size in device pixels, as Kitty reports it (`CSI 16 t`).
    cell: (u32, u32),
    /// The visible screen, one string per row.
    screen: Vec<String>,
    /// Screen plus scrollback, one string per row.
    all: Vec<String>,
    transmitted: Transmitted,
    instance: KittyInstance,
}

impl GraphRun {
    fn new(fixture: &Fixture, columns: u32, lines: u32) -> Self {
        let instance = KittyInstance::launch(columns, lines).expect("launch a private Kitty");
        let mut harness = instance.harness();

        // `open` hands the pane this process's environment, so the host
        // terminal's `TERM_PROGRAM` would decide the image protocol.
        let recording = fixture.path("wt-list.rec");
        let script = fixture.path("run.sh");
        fs::write(
            &script,
            format!(
                "cd '{main}'\n\
                 printf '\\033[16t'; IFS=';t' read -rs -d t -t 2 _ ch cw\n\
                 clear\n\
                 script -q '{rec}' env -u TERM_PROGRAM HOME='{home}' XDG_CACHE_HOME='{home}/cache' '{wt}' list\n\
                 echo \"cell=${{cw}}x${{ch}} wt-done\"\n",
                main = fixture.main.display(),
                rec = recording.display(),
                home = fixture.path("home").display(),
                wt = biscuit_test_harness::bin_exe!("wt").display(),
            ),
        )
        .unwrap();
        harness
            .send_text(format!("bash '{}'\n", script.display()).as_bytes())
            .expect("send the run script");

        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let frame = harness.capture().expect("kitty get-text");
            if frame.plain.contains("wt-done") {
                break;
            }
            assert!(Instant::now() < deadline, "wt list never finished.\n{}", frame.plain);
            std::thread::sleep(Duration::from_millis(100));
        }
        let frame = harness.capture_extent("screen").expect("kitty get-text");
        let cell = frame
            .plain
            .lines()
            .find_map(|line| line.trim().strip_prefix("cell="))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|size| size.split_once('x'))
            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .unwrap_or_else(|| panic!("Kitty did not report its cell size.\n{}", frame.plain));
        let all = screen_rows(&harness.capture_extent("all").expect("kitty get-text --extent=all").plain, columns);

        Self {
            columns,
            lines,
            cell,
            screen: screen_rows(&frame.plain, columns),
            all,
            transmitted: parse_transmitted(&fs::read(&recording).expect("read the recording")),
            instance,
        }
    }

    fn screen_row(&self, needle: &str) -> usize {
        self.screen
            .iter()
            .position(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no {needle:?} on screen:\n{}", self.screen.join("\n")))
    }

    /// The table and legend survive the image, in a real terminal: every
    /// worktree row, with its column borders where the header has them.
    fn assert_table_intact(&self) {
        let rows = &self.all;
        let all = self.all.join("\n");
        let header = rows
            .iter()
            .position(|row| row.contains("Worktree") && row.contains("Branch") && row.contains('│'))
            .unwrap_or_else(|| panic!("no table header:\n{all}"));
        let borders = |row: &str| -> Vec<usize> {
            row.chars().enumerate().filter(|(_, c)| *c == '│').map(|(i, _)| i).collect()
        };
        let expected = borders(&rows[header]);
        let mut names = vec!["base repo".to_string()];
        names.extend(BRANCHES.iter().map(|branch| format!("wt-{branch}")));
        for (offset, name) in names.iter().enumerate() {
            let row = rows[header + 2 + offset].as_str();
            assert!(row.contains(&format!("○ {name}")), "row {offset} should be {name}: {row:?}\n{all}");
            assert_eq!(borders(row), expected, "{name}'s borders: {row:?}\n{all}");
        }
        assert!(rows[header + 2 + names.len()].starts_with('└'), "table bottom:\n{all}");
        assert!(all.contains("parent deleted"), "legend:\n{all}");
    }

    /// The image `wt` asked for and the rows it left for it, measured on the
    /// screen text and on the pixels Kitty drew. Returns the hidden-lane count
    /// from the elision notice, if there is one.
    fn assert_graph_drawn(&self) -> Option<usize> {
        let (cell_w, cell_h) = self.cell;
        let image = &self.transmitted;
        assert!(
            image.columns <= self.columns,
            "the graph's {} columns must fit the {}-column window",
            image.columns,
            self.columns
        );
        assert_eq!(
            image.png.width(),
            image.columns * cell_w,
            "the PNG is rasterized at the graph's columns, so Kitty draws it unscaled"
        );
        // Kitty derives the rows from `c=` and the PNG's aspect ratio; `wt`
        // must reserve the same number or text overlaps the image.
        assert_eq!(image.rows, image.png.height().div_ceil(cell_h), "reserved rows vs Kitty's rows");

        // Screen text: the image hangs below the legend, and the next text is
        // the notice (or the done marker), at most one blank row after it.
        let legend = self.screen_row("parent deleted");
        let next = (legend + 1..self.screen.len())
            .find(|&row| !self.screen[row].trim().is_empty())
            .unwrap_or_else(|| panic!("nothing after the graph:\n{}", self.screen.join("\n")));
        let band = (next - legend - 1) as u32;
        assert!(
            band == image.rows || band == image.rows + 1,
            "{band} rows between the legend and the next text for a {}-row image:\n{}",
            image.rows,
            self.screen.join("\n")
        );

        // Pixels: the drawn part of the transmitted PNG must appear exactly
        // where the band starts, and nothing may be drawn elsewhere in it.
        let png_box = drawn_box(&image.png, (0, 0, i64::from(image.png.width()), i64::from(image.png.height())), over_black)
            .expect("the transmitted graph is not blank");
        let shot_path = self.instance_screenshot_path();
        let (screen_box, shot) = self.wait_for_drawn_band(legend, next, &shot_path);
        let (x0, y0) = self.grid_origin(&shot);
        let top = y0 + (legend as i64 + 1) * i64::from(cell_h);
        let expected = (png_box.0 + x0, png_box.1 + top, png_box.2 + x0, png_box.3 + top);
        let close = |a: i64, b: i64| (a - b).abs() <= EDGE_TOLERANCE_PX;
        assert!(
            close(screen_box.0, expected.0)
                && close(screen_box.1, expected.1)
                && close(screen_box.2, expected.2)
                && close(screen_box.3, expected.3),
            "Kitty drew the graph at {screen_box:?}, but the transmitted image belongs at {expected:?} \
             (cell {cell_w}x{cell_h}, {}x{} image, {} rows). Screenshot: {}",
            image.columns,
            image.rows,
            image.rows,
            shot_path.display()
        );
        // Kept on failure for the message above.
        let _ = fs::remove_file(&shot_path);

        self.screen[next]
            .trim()
            .strip_suffix(" not shown")
            .and_then(|notice| notice.split_whitespace().next())
            .map(|count| count.parse().expect("numeric hidden-lane count"))
    }

    fn instance_screenshot_path(&self) -> PathBuf {
        std::env::temp_dir().join(format!(
            "wt-graph-{}x{}-{}.png",
            self.columns,
            self.lines,
            std::process::id()
        ))
    }

    /// The cell grid's top-left pixel in a window screenshot: equal side
    /// borders, and the grid flush with the bottom border.
    fn grid_origin(&self, shot: &RgbaImage) -> (i64, i64) {
        let (cell_w, cell_h) = self.cell;
        let grid_w = i64::from(self.columns * cell_w);
        let border = (i64::from(shot.width()) - grid_w) / 2;
        assert!(
            (0..=4).contains(&border),
            "a {}-px screenshot does not hold a {grid_w}-px grid; is Kitty's window the requested size?",
            shot.width()
        );
        (border, i64::from(shot.height()) - border - i64::from(self.lines * cell_h))
    }

    /// Screenshots the window until the band between `legend` and `next`
    /// holds drawn pixels (Kitty draws on its next frame), and returns their
    /// bounding box and the screenshot.
    fn wait_for_drawn_band(&self, legend: usize, next: usize, path: &Path) -> (BoundingBox, RgbaImage) {
        let cell_h = i64::from(self.cell.1);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.instance.screenshot(path).expect("screenshot the Kitty window");
            let shot = image::open(path).expect("read the screenshot").to_rgba8();
            let (x0, y0) = self.grid_origin(&shot);
            let band = (
                x0,
                y0 + (legend as i64 + 1) * cell_h,
                x0 + i64::from(self.columns * self.cell.0),
                y0 + next as i64 * cell_h,
            );
            if let Some(found) = drawn_box(&shot, band, opaque) {
                return (found, shot);
            }
            assert!(
                Instant::now() < deadline,
                "nothing drawn where the graph belongs. Screenshot: {} (a capture without window \
                 contents means the calling terminal lacks the Screen Recording permission)",
                path.display()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// A short window: the base view's graph is capped at half the rows, so
/// lanes are left out and the notice counts them; the image Kitty draws fills
/// exactly the rows `wt` reserved.
#[test]
#[serial(level2_terminal)]
fn level2_graph_height_cap_elides_lanes_in_a_short_kitty_window() {
    require_level!(Level::L2, KittyInstance::can_launch(), Backend::Kitty);

    let fixture = Fixture::new();
    let run = GraphRun::new(&fixture, 100, 32);

    assert!(run.transmitted.rows <= run.lines / 2, "{} rows exceed half of {}", run.transmitted.rows, run.lines);
    run.assert_table_intact();
    let hidden = run.assert_graph_drawn().unwrap_or_else(|| {
        panic!("the capped graph should end with an elision notice:\n{}", run.screen.join("\n"))
    });
    assert!((1..BRANCHES.len()).contains(&hidden), "{hidden} hidden lanes of {}", BRANCHES.len());
}

/// A narrow window: the graph is clamped to the window's columns and drawn
/// at the transmitted size, with the table intact above it.
#[test]
#[serial(level2_terminal)]
fn level2_graph_fits_a_narrow_kitty_window() {
    require_level!(Level::L2, KittyInstance::can_launch(), Backend::Kitty);

    let fixture = Fixture::new();
    let run = GraphRun::new(&fixture, 56, 60);

    assert!(run.transmitted.rows <= run.lines / 2, "{} rows exceed half of {}", run.transmitted.rows, run.lines);
    run.assert_table_intact();
    run.assert_graph_drawn();
}
