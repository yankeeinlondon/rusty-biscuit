//! Level-2 terminal verification for the `icon` CLI.
//!
//! These tests drive the real `icon` binary inside a tmux/Kitty/WezTerm pane
//! and assert on the actual escape sequences and visible text the terminal
//! emits.

use std::path::PathBuf;

use biscuit_test_harness::{
    CapturedFrame, TerminalHarness, wait_for_prompt,
};
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
#[cfg(feature = "image")]
use biscuit_test_harness::wezterm::WezTermHarness;
#[cfg(feature = "image")]
use image::GenericImageView;
use serial_test::serial;
use test_toolkit::{Level, require_level};

static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Returns the absolute path to the `icon` binary under test.
fn icon_bin() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("../../target/debug/icon")
        .canonicalize()
        .expect("icon binary should be built by cargo test")
}

/// Returns a `PATH` value that includes the directory containing the `icon`
/// binary, preserving the caller's `PATH` so the shell can find standard
/// utilities.
fn path_with_icon_bin() -> String {
    let bin_dir = icon_bin().parent().unwrap().to_path_buf();
    let existing = std::env::var("PATH").unwrap_or_default();
    format!("{existing}:{}", bin_dir.display())
}

/// Runs an `icon` command with an isolated `$HOME` and a dead Iconify endpoint
/// so tests never trigger live network searches.
fn run_icon(harness: &mut TmuxHarness, args: &str) -> CapturedFrame {
    let home = tempfile::tempdir().unwrap();
    let cmd = format!(
        "HOME='{}' PATH='{}' ICONIFY_BASE_URL='http://127.0.0.1:1' icon {}\n",
        home.path().display(),
        path_with_icon_bin(),
        args
    );
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    let _ = wait_for_prompt(harness);
    harness.capture().expect("capture failed")
}

// ------------------------------------------------------------------
// Unicode glyph output
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_unicode_glyph_renders_in_terminal() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // "grinning" matches the built-in Emoji::Happy icon.
    let frame = run_icon(harness, "icons grinning");
    assert!(
        frame.plain.contains('\u{1F600}'),
        "expected Unicode grinning face in visible output; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("fluent-emoji-flat:grinning-face"),
        "expected icon identifier in visible output; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Nerd Font output
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_nerd_font_glyph_renders_with_flag() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // DevOps::Github maps to "uil:github" and defines a Nerd Font glyph.
    let frame = run_icon(harness, "--nerd icons github");
    assert!(
        frame.plain.contains('\u{f09b}'),
        "expected Nerd Font github glyph in visible output; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("uil:github"),
        "expected icon identifier in visible output; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Text fallback shows the icon identifier
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_text_fallback_shows_identifier() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // Os::Apple has no glyph, so it must fall back to its Iconify id.
    let frame = run_icon(harness, "icons apple");
    assert!(
        frame.plain.contains("ic:baseline-apple"),
        "expected icon identifier as text fallback; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Image-protocol fallback (requires the `image` feature)
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
#[cfg(feature = "image")]
fn level2_image_protocol_fallback_renders_graphics() {
    let wezterm_available = WezTermHarness::available();
    require_level!(Level::L2, wezterm_available, "WezTerm");

    static SHARED_WEZTERM: biscuit_test_harness::shared::SharedHarness<
        biscuit_test_harness::wezterm::WezTermHarness,
    > = biscuit_test_harness::shared::SharedHarness::new();

    let mut guard = SHARED_WEZTERM.get_or_init(|| {
        biscuit_test_harness::wezterm::WezTermHarness::shared_or_spawn()
            .expect("attach/spawn WezTerm")
    });
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // Pre-populate the cache with a bright red icon so the pixel assertion
    // is image-specific — a red rectangle is unmistakable against any terminal
    // background and cannot be confused with shell text.
    let home = tempfile::tempdir().unwrap();
    {
        let cache_dir = home.path().join(".cache").join("biscuit-icon");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
        cache
            .put(
                "test",
                "red-witness",
                &biscuit_icon::IconBody::new(r#"<rect width="24" height="24" fill="red"/>"#, 24, 24),
            )
            .unwrap();
    }

    let baseline_result = harness.capture_window_png().expect("png capture call failed");
    let capture_available = baseline_result.is_some();
    require_level!(Level::L2, capture_available, "window capture (screen recording permission)");
    let baseline_png = baseline_result.unwrap();

    let cmd = format!(
        "HOME='{}' PATH='{}' icon icons test:red-witness\n",
        home.path().display(),
        path_with_icon_bin(),
    );
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    let _ = wait_for_prompt(harness);

    let after_png = harness
        .capture_window_png()
        .expect("png capture call failed")
        .expect("capture was available at baseline but not after render");

    let baseline = image::load_from_memory(&baseline_png).expect("decode baseline png");
    let after = image::load_from_memory(&after_png).expect("decode after png");

    // Build a mask of bright red pixels that appear in the after image but
    // were not present in the baseline, then look for a contiguous block.
    // A rendered icon produces a solid rectangle; command text or UI chrome
    // does not.
    let mut new_red = vec![vec![false; after.width() as usize]; after.height() as usize];
    for y in 0..after.height() {
        for x in 0..after.width() {
            let after_pixel = after.get_pixel(x, y);
            if after_pixel[0] > 200 && after_pixel[1] < 50 && after_pixel[2] < 50 {
                let baseline_pixel = if x < baseline.width() && y < baseline.height() {
                    baseline.get_pixel(x, y)
                } else {
                    after_pixel
                };
                let baseline_red = baseline_pixel[0] > 200
                    && baseline_pixel[1] < 50
                    && baseline_pixel[2] < 50;
                if !baseline_red {
                    new_red[y as usize][x as usize] = true;
                }
            }
        }
    }

    let mut visited = vec![vec![false; after.width() as usize]; after.height() as usize];
    let mut found_component = false;
    for y in 0..after.height() {
        for x in 0..after.width() {
            if !new_red[y as usize][x as usize] || visited[y as usize][x as usize] {
                continue;
            }

            let mut stack = vec![(x, y)];
            let mut size = 0;
            visited[y as usize][x as usize] = true;

            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                for (dx, dy) in [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx >= 0
                        && nx < after.width() as i32
                        && ny >= 0
                        && ny < after.height() as i32
                    {
                        let nx = nx as u32;
                        let ny = ny as u32;
                        if new_red[ny as usize][nx as usize]
                            && !visited[ny as usize][nx as usize]
                        {
                            visited[ny as usize][nx as usize] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }

            if size >= 50 {
                found_component = true;
                break;
            }
        }
        if found_component {
            break;
        }
    }

    assert!(
        found_component,
        "expected a contiguous block of at least 50 bright red pixels in WezTerm capture after rendering icon; image may not have rendered"
    );
}

// ------------------------------------------------------------------
// Listing alignment
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_listing_includes_multiple_names() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    let frame = run_icon(harness, "icons arrow");
    assert!(
        frame.plain.contains("mdi:arrow-left-circle"),
        "expected arrow-left-circle in listing; got:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("mdi:arrow-right-circle"),
        "expected arrow-right-circle in listing; got:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// `sets` table layout
// ------------------------------------------------------------------

/// Seeds an isolated `$HOME` cache with set metadata so an offline `icon sets`
/// run renders deterministic rows. `cached` seeds N icon rows per prefix so the
/// `Cached` column has known values.
fn seed_sets(
    home: &std::path::Path,
    sets: &[(&str, &str, Option<usize>)],
    cached: &[(&str, usize)],
) {
    let cache_dir = home.join(".cache").join("biscuit-icon");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cache = biscuit_icon::cache::IconCache::open_at(cache_dir.join("icons.db")).unwrap();
    for (prefix, title, total) in sets {
        cache
            .put_set(&biscuit_icon::cache::SetInfo {
                prefix: (*prefix).into(),
                title: (*title).into(),
                license: None,
                license_title: None,
                license_url: None,
                total: *total,
            })
            .unwrap();
    }
    for (prefix, count) in cached {
        for i in 0..*count {
            cache
                .put(prefix, &format!("icon{i}"), &biscuit_icon::IconBody::new("<path/>", 24, 24))
                .unwrap();
        }
    }
}

/// Runs `icon sets <filter>` against an isolated, offline cache at a fixed
/// terminal size so the chosen layout is deterministic.
fn run_sets(
    harness: &mut TmuxHarness,
    home: &std::path::Path,
    filter: &str,
    width: u32,
    height: u32,
) -> CapturedFrame {
    // Invoke the freshly-built binary by absolute path: a globally-installed
    // `icon` earlier on `$PATH` would otherwise shadow it and run stale code.
    let cmd = format!(
        "HOME='{}' ICONIFY_BASE_URL='http://127.0.0.1:1' \
         BISCUIT_TERM_WIDTH={width} BISCUIT_TERM_HEIGHT={height} '{}' sets {filter}\n",
        home.display(),
        icon_bin().display(),
    );
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    let _ = wait_for_prompt(harness);
    harness.capture().expect("capture failed")
}

#[test]
#[serial(level2_terminal)]
fn level2_sets_single_table_renders() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    let home = tempfile::tempdir().unwrap();
    // The "zz" token matches no built-in prefix, so only these rows appear.
    seed_sets(
        home.path(),
        &[
            ("zzalpha", "ZZ Alpha", None),
            ("zzbeta", "ZZ Beta", Some(42)),
            ("zztest", "ZZ Test", Some(1_234_567)),
        ],
        &[("zzbeta", 2)],
    );

    // 70 cols < MIN_SPLIT_WIDTH and 20 rows of height keep a single table.
    let frame = run_sets(harness, home.path(), "zz", 70, 20);

    assert!(
        frame.plain.contains('│') && frame.plain.contains('─'),
        "expected box-drawing borders in real terminal; got:\n{}",
        frame.plain
    );

    // Headers render left-to-right in column order.
    let header = frame
        .plain
        .lines()
        .find(|l| {
            l.contains("Set") && l.contains("Prefix") && l.contains("Total") && l.contains("Cached")
        })
        .unwrap_or_else(|| panic!("header row not found in:\n{}", frame.plain));
    let pos = |needle: &str| header.find(needle).expect("header column present");
    assert!(
        pos("Set") < pos("Prefix") && pos("Prefix") < pos("Total") && pos("Total") < pos("Cached"),
        "columns out of order in header: {header:?}",
    );

    // The large total is rendered with thousands separators in the real terminal.
    assert!(
        frame.plain.contains("1,234,567"),
        "expected thousands-separated total; got:\n{}",
        frame.plain
    );

    // The second data row (zzbeta) carries an alternating-row background SGR.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let striped = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, p)| p.contains('│') && p.contains("zzbeta"))
        .and_then(|(i, _)| raw_lines.get(i).map(|r| (*r).to_string()))
        .unwrap_or_else(|| panic!("striped zzbeta row not found in:\n{}", frame.plain));
    assert!(
        striped.contains("\x1b[48;2;")
            || striped.contains("\x1b[48:2:")
            || striped.contains("\x1b[48;5;"),
        "expected alternating-row background SGR; raw row: {striped:?}",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_sets_split_table_renders() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    let home = tempfile::tempdir().unwrap();
    let seeded: Vec<(String, String, Option<usize>)> = (0..6)
        .map(|i| (format!("zz{i}"), format!("ZZ {i}"), Some(i * 100)))
        .collect();
    let seeded_refs: Vec<(&str, &str, Option<usize>)> = seeded
        .iter()
        .map(|(p, t, total)| (p.as_str(), t.as_str(), *total))
        .collect();
    seed_sets(home.path(), &seeded_refs, &[]);

    // Wide enough for two tables, short enough that 6 rows exceed the budget.
    let frame = run_sets(harness, home.path(), "zz", 110, 6);

    // Two tables side by side: a header-rule line carries both an inner-right
    // tee (end of the left table) and an inner-left tee (start of the right).
    assert!(
        frame.plain.lines().any(|l| l.contains('┤') && l.contains('├')),
        "expected side-by-side split layout; got:\n{}",
        frame.plain
    );

    for i in 0..6 {
        assert!(
            frame.plain.contains(&format!("zz{i}")),
            "expected zz{i} in split output; got:\n{}",
            frame.plain
        );
    }

    // Column-major split: zz0 (top of left table) and zz3 (top of right table)
    // share the first data row, with zz0 to the left of zz3.
    let first_row = frame
        .plain
        .lines()
        .find(|l| l.contains("zz0") && l.contains("zz3"))
        .unwrap_or_else(|| panic!("expected zz0 and zz3 on one row; got:\n{}", frame.plain));
    assert!(
        first_row.find("zz0").unwrap() < first_row.find("zz3").unwrap(),
        "expected column-major order (zz0 left of zz3): {first_row:?}",
    );
}

// ------------------------------------------------------------------
// Styled errors
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_styled_error_emits_sgr_red() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    // An extra colon is rejected by the identifier parser and rendered as a
    // Prose-styled error.
    let frame = run_icon(harness, "icons mdi:home:extra");
    assert!(
        frame.raw.contains("\x1b[31m") || frame.raw.contains("\x1b[91m"),
        "expected SGR red in styled error output; raw:\n{}",
        frame.raw
    );
    assert!(
        frame.plain.contains("Error:"),
        "expected 'Error:' label in output; got:\n{}",
        frame.plain
    );
}
