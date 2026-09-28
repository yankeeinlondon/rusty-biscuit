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

/// The branch [`Fixture::merged`] merges into `main`, named as in the first
/// observed merged-branch graph. Long neighboring labels are proven at L1.
const MERGED: &str = "fix/wt-ux";

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

fn git_output(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git should be installed");
    assert!(output.status.success(), "git {args:?} failed in {repo:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A commit on `tree` with `parents` (first parent first), made without
/// touching a checkout; returns its full SHA.
fn commit_on(repo: &Path, tree: &str, parents: &[&str], message: &str) -> String {
    let mut args = vec!["commit-tree", "-m", message];
    for parent in parents {
        args.extend(["-p", *parent]);
    }
    args.push(tree);
    git_output(repo, &args)
}

/// `count` commits after `from`, oldest first.
fn chain_on(repo: &Path, tree: &str, from: &str, prefix: &str, count: usize) -> Vec<String> {
    let mut chain: Vec<String> = Vec::with_capacity(count);
    for n in 1..=count {
        let parent = chain.last().map_or(from, String::as_str).to_string();
        chain.push(commit_on(repo, tree, &[&parent], &format!("{prefix}{n}")));
    }
    chain
}

/// The observed-shape history's commit counts, the same in the L1
/// (`observed_sparse_lanes`) and perf (`GraphFixture::observed_sparse_lanes`)
/// builders.
const SPARSE_SCHEMA_COMMITS: usize = 55;
const SPARSE_WT_UX_BEFORE_MERGE: usize = 8;
const SPARSE_WT_UX_AFTER_MERGE: usize = 64;
const SPARSE_WT_UX_AFTER_SYNC: usize = 3;
const SPARSE_SNIFF_COMMITS: usize = 94;

/// The observed-shape history's named commits, by full SHA.
struct SparseLanes {
    d2: String,
    w1: String,
    m103: String,
    m104: String,
    b1: String,
}

/// A main checkout on `main` with linked worktrees.
struct Fixture {
    parent: tempfile::TempDir,
    main: PathBuf,
    /// Worktree directory names; the table lists them after `base repo`.
    worktrees: Vec<String>,
    /// Set by [`Fixture::sparse_lanes`].
    sparse: Option<SparseLanes>,
}

impl Fixture {
    fn init() -> Self {
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
        let fixture = Self {
            parent,
            main,
            worktrees: Vec::new(),
            sparse: None,
        };
        fixture.commit(&fixture.main, "main.txt", "main 1");
        fixture.commit(&fixture.main, "main.txt", "main 2");
        fixture
    }

    /// One lane per branch in [`BRANCHES`]. `main` gains a commit after each
    /// fork, so every lane leaves the default lane at a different commit.
    fn new() -> Self {
        let mut fixture = Self::init();
        for branch in BRANCHES {
            let worktree = fixture.add_worktree(&format!("wt-{branch}"), branch);
            fixture.commit(&worktree, &format!("{branch}.txt"), &format!("work on {branch}"));
            fixture.commit(&fixture.main, "main.txt", &format!("main after {branch}"));
        }
        fixture
    }

    /// A worktree whose branch is merged into `main` with a merge
    /// commit: the graph's merge edge, at the commit tagged `main`.
    ///
    /// ```text
    /// main 1 - main 2 - main 3 ----- Merge (main)
    ///                \              /
    ///                 m1 - m2 -----   (MERGED, wt-merged)
    /// ```
    fn merged() -> Self {
        let mut fixture = Self::init();
        let merged = fixture.add_worktree("wt-merged", MERGED);
        fixture.commit(&merged, "merged.txt", "merged 1");
        fixture.commit(&merged, "merged.txt", "merged 2");
        fixture.commit(&fixture.main, "main.txt", "main 3");
        run_git(&fixture.main, &["merge", "--no-ff", "--no-edit", MERGED]);
        fixture
    }

    /// The history `wt list` drew sparsely on 2026-09-27, with a worktree per
    /// branch and each branch's recorded parent:
    ///
    /// - `main`: `main 1` (`r`), `main 2` (`d1`), `d2..d12`, `M103` (merges
    ///   `W1`), `d13`, `M104` (merges `fix/sniff`); `origin/main` = `main`.
    /// - `feat/schema-enhancement` (parent `main`) forks at `d2`: 55 commits.
    /// - `fix/wt-ux` (parent `main`) forks at `d5`: `w1..w8` (`W1` = `w8`),
    ///   64 more after `M103`, merges `main` back at `B1`, then 3 more.
    /// - `fix/sniff` (parent `fix/wt-ux`) forks at `W1`: 94 commits.
    fn sparse_lanes() -> Self {
        let mut fixture = Self::init();
        let repo = fixture.main.clone();
        let tree = git_output(&repo, &["rev-parse", "HEAD^{tree}"]);
        let mut d = vec![git_output(&repo, &["rev-parse", "HEAD~1"]), git_output(&repo, &["rev-parse", "HEAD"])];
        let more = chain_on(&repo, &tree, &d[1], "d", 11);
        d.extend(more);
        let schema = chain_on(&repo, &tree, &d[2], "s", SPARSE_SCHEMA_COMMITS);
        let before = chain_on(&repo, &tree, &d[5], "w", SPARSE_WT_UX_BEFORE_MERGE);
        let w1 = before.last().unwrap().clone();
        let m103 = commit_on(&repo, &tree, &[&d[12], &w1], "Merge pull request #103 from fix/wt-ux");
        let sniff = chain_on(&repo, &tree, &w1, "n", SPARSE_SNIFF_COMMITS);
        let after = chain_on(&repo, &tree, &w1, "w", SPARSE_WT_UX_AFTER_MERGE);
        let d13 = commit_on(&repo, &tree, &[&m103], "d13");
        let m104 = commit_on(&repo, &tree, &[&d13, sniff.last().unwrap()], "Merge pull request #104 from fix/sniff");
        let b1 = commit_on(&repo, &tree, &[after.last().unwrap(), &m104], "Merge branch 'main' into fix/wt-ux");
        let synced = chain_on(&repo, &tree, &b1, "x", SPARSE_WT_UX_AFTER_SYNC);

        run_git(&repo, &["update-ref", "refs/heads/main", &m104]);
        run_git(&repo, &["update-ref", "refs/remotes/origin/main", &m104]);
        run_git(&repo, &["reset", "-q", "--hard", "main"]);
        for (directory, branch, tip, parent) in [
            ("wt-schema", "feat/schema-enhancement", schema.last().unwrap(), "main"),
            ("wt-ux", "fix/wt-ux", synced.last().unwrap(), "main"),
            ("wt-sniff", "fix/sniff", sniff.last().unwrap(), "fix/wt-ux"),
        ] {
            run_git(&repo, &["branch", branch, tip]);
            let worktree = fixture.path(directory);
            run_git(&repo, &["worktree", "add", "-q", worktree.to_str().unwrap(), branch]);
            fixture.worktrees.push(directory.to_string());
            fixture.record_parent(branch, parent);
        }
        fixture.sparse = Some(SparseLanes {
            d2: d[2].clone(),
            w1,
            m103,
            m104,
            b1,
        });
        fixture
    }

    /// The fork-origin store `wt list` reads under the fixture's `HOME` and
    /// `XDG_CACHE_HOME` (see [`GraphRun::new`]).
    fn fork_store(&self) -> PathBuf {
        let real = worktree::fork_origin::fork_origin_path(&self.main).expect("fork store path");
        let home = self.path("home");
        let root = if cfg!(target_os = "macos") {
            home.join("Library").join("Caches")
        } else {
            home.join("cache")
        };
        root.join("worktree").join(real.file_name().expect("store file name"))
    }

    fn record_parent(&self, branch: &str, parent: &str) {
        let origin = worktree::fork_origin::ForkOrigin {
            base_branch: parent.to_string(),
            base_sha: git_output(&self.main, &["rev-parse", parent]),
            created_at: 1,
        };
        worktree::fork_origin::record(&self.fork_store(), branch, origin).expect("record the fork origin");
    }

    fn add_worktree(&mut self, directory: &str, branch: &str) -> PathBuf {
        let worktree = self.path(directory);
        run_git(&self.main, &["worktree", "add", worktree.to_str().unwrap(), "-b", branch]);
        self.worktrees.push(directory.to_string());
        worktree
    }

    fn commit(&self, dir: &Path, file: &str, message: &str) {
        fs::write(dir.join(file), message).unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", message]);
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

/// Kitty is never scheduled in CI, so this warning is the only trace of a
/// pixel check that did not run; `.config/nextest.toml` shows this binary's
/// output on success so it is seen.
fn warn_pixels_unproven(reason: &str) {
    let test = std::thread::current().name().unwrap_or("level2_graph_in_kitty").to_string();
    eprintln!("WARNING: {test}: pixel check skipped: {reason}");
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
    worktrees: Vec<String>,
    /// Where the screenshot and the transmitted PNG are kept for inspection;
    /// `None` deletes the screenshot once it passes.
    evidence: Option<String>,
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
            worktrees: fixture.worktrees.clone(),
            evidence: None,
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
        names.extend(self.worktrees.iter().cloned());
        // Rows follow the parent tree, so only the block's membership is fixed.
        let block = &rows[header + 2..header + 2 + names.len()];
        for name in &names {
            let matching: Vec<&String> = block.iter().filter(|row| row.contains(&format!("○ {name} "))).collect();
            assert_eq!(matching.len(), 1, "one row for {name}:\n{all}");
            assert_eq!(borders(matching[0]), expected, "{name}'s borders: {:?}\n{all}", matching[0]);
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

        self.assert_pixels_match(legend, next);

        // "Some history is not shown" may follow the image instead.
        self.screen[next]
            .trim()
            .strip_suffix(" not shown")
            .and_then(|notice| notice.strip_suffix(" more worktrees").or_else(|| notice.strip_suffix(" more worktree")))
            .map(|count| count.parse().expect("numeric hidden-lane count"))
    }

    /// The drawn part of the transmitted PNG must appear exactly where the
    /// band below `legend` starts, and nothing may be drawn elsewhere in it.
    /// Skipped with a warning when the screenshot cannot show what Kitty drew;
    /// the text and APC checks above still hold.
    fn assert_pixels_match(&self, legend: usize, next: usize) {
        let (cell_w, cell_h) = self.cell;
        let image = &self.transmitted;
        if !biscuit_test_harness::screen_capture_permitted() {
            warn_pixels_unproven("this process lacks macOS Screen Recording permission (grant it to the terminal running the tests)");
            return;
        }
        let png_box = drawn_box(&image.png, (0, 0, i64::from(image.png.width()), i64::from(image.png.height())), over_black)
            .expect("the transmitted graph is not blank");
        let shot_path = self.instance_screenshot_path();
        let Some((screen_box, shot)) = self.wait_for_drawn_band(legend, next, &shot_path) else {
            warn_pixels_unproven(&format!(
                "the screenshot holds no window contents, not even the table Kitty reports on screen, so Kitty had not \
                 drawn its window. Screenshot: {}",
                shot_path.display()
            ));
            return;
        };
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
        // Kept on failure for the message above, and as evidence when asked.
        if self.evidence.is_none() {
            let _ = fs::remove_file(&shot_path);
        }
    }

    fn instance_screenshot_path(&self) -> PathBuf {
        let name = match &self.evidence {
            Some(label) => format!("wt-graph-{label}-{}x{}-screenshot.png", self.columns, self.lines),
            None => format!("wt-graph-{}x{}-{}.png", self.columns, self.lines, std::process::id()),
        };
        std::env::temp_dir().join(name)
    }

    /// Keeps this run's screenshot and writes the transmitted PNG beside it,
    /// both named for `label`, and prints their paths.
    fn keep_evidence(&mut self, label: &str) {
        self.evidence = Some(label.to_string());
        let png = std::env::temp_dir().join(format!("wt-graph-{label}-{}x{}-transmitted.png", self.columns, self.lines));
        self.transmitted.png.save(&png).expect("save the transmitted PNG");
        eprintln!(
            "evidence: transmitted {} and screenshot {}",
            png.display(),
            self.instance_screenshot_path().display()
        );
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
    /// bounding box and the screenshot. `None` when the last screenshot shows
    /// nothing even where the table is, which proves nothing about the graph.
    fn wait_for_drawn_band(&self, legend: usize, next: usize, path: &Path) -> Option<(BoundingBox, RgbaImage)> {
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
                return Some((found, shot));
            }
            if Instant::now() >= deadline {
                let table = (band.0, y0, band.2, band.1);
                assert!(
                    drawn_box(&shot, table, opaque).is_none(),
                    "Kitty drew the table but nothing where the graph belongs. Screenshot: {}",
                    path.display()
                );
                return None;
            }
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

/// A merged branch's lane, at both window sizes: the table stays intact, the
/// image fits the window and the rows `wt` reserved, no lane is left out, and
/// nothing is reported as not shown. The screenshot and
/// the transmitted PNG are kept in the temp directory for visual inspection;
/// the layout itself (merge parents, labels, overlaps) is proven at L1 by
/// `gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags`.
#[test]
#[serial(level2_terminal)]
fn level2_graph_draws_a_merged_branch_in_kitty() {
    require_level!(Level::L2, KittyInstance::can_launch(), Backend::Kitty);

    let fixture = Fixture::merged();
    // Both windows' text and transmitted images are checked (and kept) before
    // either screenshot, so a capture failure still leaves both PNGs.
    let runs: Vec<GraphRun> = [(100, 32), (56, 60)]
        .into_iter()
        .map(|(columns, lines)| {
            let mut run = GraphRun::new(&fixture, columns, lines);
            run.keep_evidence("merged");
            assert!(run.transmitted.rows <= run.lines / 2, "{} rows exceed half of {}", run.transmitted.rows, run.lines);
            let all = run.all.join("\n");
            assert!(!all.contains("Some history is not shown"), "a merged branch is complete history:\n{all}");
            run.assert_table_intact();
            run
        })
        .collect();
    for run in &runs {
        assert_eq!(run.assert_graph_drawn(), None, "no lane is left out:\n{}", run.screen.join("\n"));
    }
}

/// The observed sparse-lanes history in a 200×60 window: the table stays
/// intact, the image fits the window and the rows `wt` reserved, no lane is
/// left out, and the only notice is "Some history is not shown" (for
/// `fix/sniff`'s undrawn fork). The screenshot and the transmitted PNG are
/// kept in the temp directory for inspection of the lane density and the
/// merge edge; the plan itself is proven at L1 by
/// `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`.
#[test]
#[serial(level2_terminal)]
fn level2_graph_restores_lane_density_in_kitty() {
    require_level!(Level::L2, KittyInstance::can_launch(), Backend::Kitty);

    let fixture = Fixture::sparse_lanes();
    let mut run = GraphRun::new(&fixture, 200, 60);
    run.keep_evidence("sparse");
    assert!(run.transmitted.rows <= run.lines / 2, "{} rows exceed half of {}", run.transmitted.rows, run.lines);
    let all = run.all.join("\n");
    assert!(all.contains("Some history is not shown"), "fix/sniff's fork is not drawn:\n{all}");
    assert!(!all.contains("more worktree"), "no lane is left out:\n{all}");
    run.assert_table_intact();
    assert_eq!(run.assert_graph_drawn(), None, "no lane is left out:\n{}", run.screen.join("\n"));
}

/// The observed-shape fixture is the history the plan describes (E1): merge
/// parents, `origin/main`, own-commit counts, and the recorded parents `wt`
/// will read.
#[test]
#[serial(level2_terminal)]
fn level2_sparse_lanes_fixture_has_the_observed_topology() {
    require_level!(Level::L2, KittyInstance::can_launch(), Backend::Kitty);

    let fixture = Fixture::sparse_lanes();
    let sparse = fixture.sparse.as_ref().expect("sparse-lanes shas");
    let repo = &fixture.main;
    let at = |rev: &str| git_output(repo, &["rev-parse", rev]);
    let parents = |merge: &str| git_output(repo, &["rev-list", "--parents", "-n", "1", merge]);
    let count = |range: &str| git_output(repo, &["rev-list", "--count", range]);

    assert_eq!(at("main"), sparse.m104);
    assert_eq!(at("refs/remotes/origin/main"), sparse.m104);
    assert_eq!(parents(&sparse.m103).split(' ').nth(2), Some(sparse.w1.as_str()));
    assert_eq!(parents(&sparse.m104).split(' ').nth(2), Some(at("fix/sniff").as_str()));
    assert_eq!(parents(&sparse.b1).split(' ').nth(2), Some(sparse.m104.as_str()));
    assert_eq!(git_output(repo, &["merge-base", "main", "feat/schema-enhancement"]), sparse.d2);
    assert_eq!(count("main..feat/schema-enhancement"), SPARSE_SCHEMA_COMMITS.to_string());
    assert_eq!(count(&format!("{}..fix/sniff", sparse.w1)), SPARSE_SNIFF_COMMITS.to_string());
    assert_eq!(count("main..fix/wt-ux"), (SPARSE_WT_UX_AFTER_MERGE + 1 + SPARSE_WT_UX_AFTER_SYNC).to_string());
    assert_eq!(count(&format!("{}..{}", sparse.d2, sparse.w1)), (3 + SPARSE_WT_UX_BEFORE_MERGE).to_string(), "d3..d5 and w1..w8");
    assert_eq!(git_output(repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 4);

    let store = worktree::fork_origin::ForkOriginStore::load_from(&fixture.fork_store());
    assert_eq!(store.get("fix/sniff").map(|fork| fork.base_branch.as_str()), Some("fix/wt-ux"));
    assert_eq!(store.get("fix/wt-ux").map(|fork| fork.base_branch.as_str()), Some("main"));
    assert_eq!(store.get("feat/schema-enhancement").map(|fork| fork.base_branch.as_str()), Some("main"));
}
