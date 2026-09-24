//! Contracts for `scripts/kache-config-merge.py`
//! (fixes/2026-09-23-ensuring-kache-support, spec §2).
//!
//! The merge is the one writer of the kache store pin (the cache table's
//! local_store and ignore_env keys) and of the Cargo-home activation (the
//! build table's rustc-wrapper key), so its observable contract is what
//! `just init` leans on: every other key and comment survives, a repeated
//! write is a byte-identical no-op (the Phase 2 daemon-restart trigger
//! compares file content), a malformed file is rejected with the original
//! untouched, and a dated backup appears only when the content actually
//! changed. Every test runs the real script through its normal command-line
//! invocation against copies of the shipped fixtures under
//! `scripts/fixtures/kache/`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

fn script() -> PathBuf {
    repo_root().join("scripts/kache-config-merge.py")
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("scripts/fixtures/kache").join(name)
}

/// `python3` everywhere this suite runs, `python` where only the Windows
/// spelling exists.
fn python() -> Command {
    for name in ["python3", "python"] {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Command::new(name);
        }
    }
    panic!("neither python3 nor python is on PATH; the merge helper needs one");
}

fn merge(file: &Path, args: &[&str]) -> Output {
    let mut command = python();
    command.arg(script()).arg(file).args(args);
    command.output().expect("merge helper must spawn")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "kache-merge-contract.{}.{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }

    fn copy_fixture(&self, name: &str) -> PathBuf {
        let target = self.0.join(name);
        fs::copy(fixture(name), &target).unwrap_or_else(|error| {
            panic!("copy fixture {name}: {error}");
        });
        target
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn parse(path: &Path) -> toml::Value {
    let text = fs::read_to_string(path).expect("read merged file");
    toml::from_str(&text)
        .unwrap_or_else(|error| panic!("{} must parse as TOML: {error}", path.display()))
}

/// The shipped fixtures the corpus must hold their shape against. The
/// malformed fixture is excluded — it exists to be rejected.
fn corpus_fixtures() -> Vec<&'static str> {
    vec![
        "config-with-cache.toml",
        "config-without-cache.toml",
        "cargo-config.toml",
        "config-inline-comments.toml",
    ]
}

/// The keys the kache write targets; their lines keep everything but the
/// value.
const TARGET_KEYS: [&str; 2] = ["local_store", "ignore_env"];

/// The trailing comment of a `key = value` line, including the whitespace
/// before its `#`; a `#` inside a basic or literal string does not count.
fn trailing_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, char) in line.char_indices() {
        match quote {
            Some('"') if escaped => escaped = false,
            Some('"') if char == '\\' => escaped = true,
            Some(open) if char == open => quote = None,
            Some(_) => {}
            None if char == '"' || char == '\'' => quote = Some(char),
            None if char == '#' => return &line[line[..index].trim_end().len()..],
            None => {}
        }
    }
    ""
}

/// A fixture line survives the merge: verbatim, or — for a targeted key —
/// with its indentation, key, `=` spacing, and trailing comment intact.
fn assert_line_survives(name: &str, merged: &str, line: &str) {
    let targeted = TARGET_KEYS.iter().any(|key| {
        line.trim_start()
            .strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    });
    if !targeted {
        assert!(
            merged.contains(line),
            "{name}: line lost by the merge: {line:?}\nmerged:\n{merged}"
        );
        return;
    }
    let prefix = &line[..=line.find('=').expect("targeted line has '='")];
    let comment = trailing_comment(line);
    assert!(
        merged
            .lines()
            .any(|merged_line| merged_line.starts_with(prefix) && merged_line.ends_with(comment)),
        "{name}: targeted line lost its prefix or comment {comment:?}: {line:?}\nmerged:\n{merged}"
    );
}

/// Passive corpus: every shipped fixture survives the kache config write —
/// the file still parses, the requested keys hold their values, and the
/// fixture's own keys and comments are all still present.
#[test]
fn corpus_every_fixture_holds_its_shape_through_the_kache_write() {
    for name in corpus_fixtures() {
        let scratch = Scratch::new("corpus");
        let target = scratch.copy_fixture(name);
        let output = merge(
            &target,
            &["cache", "local_store", "/Volumes/coding/kache", "ignore_env", "true"],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: exit 0, stderr:\n{}",
            stderr(&output)
        );
        let value = parse(&target);
        let cache = value
            .get("cache")
            .and_then(|c| c.as_table())
            .unwrap_or_else(|| panic!("{name}: [cache] must be a table"));
        assert_eq!(
            cache.get("local_store").and_then(|v| v.as_str()),
            Some("/Volumes/coding/kache"),
            "{name}: local_store value"
        );
        assert_eq!(
            cache.get("ignore_env").and_then(|v| v.as_bool()),
            Some(true),
            "{name}: ignore_env value"
        );

        let merged = fs::read_to_string(&target).unwrap();
        for line in fs::read_to_string(fixture(name)).unwrap().lines() {
            assert_line_survives(name, &merged, line);
        }
    }
}

/// End-to-end with the real invocation path: an existing hand-tuned `[cache]`
/// gains `local_store` and `ignore_env` while every comment and key line
/// survives byte-for-byte, and the previous file is kept as a dated backup.
#[test]
fn existing_cache_table_is_merged_in_place_with_a_dated_backup() {
    let scratch = Scratch::new("in-place");
    let target = scratch.copy_fixture("config-with-cache.toml");
    let original = fs::read_to_string(&target).unwrap();

    let output = merge(
        &target,
        &["cache", "local_store", "/Volumes/coding/kache", "ignore_env", "true"],
    );
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));

    let merged = fs::read_to_string(&target).unwrap();
    assert!(
        merged.contains("local_store = \"/Volumes/coding/kache\""),
        "the stale value is replaced:\n{merged}"
    );
    assert!(!merged.contains("\"/old/location\""));
    for line in original.lines() {
        assert_line_survives("config-with-cache.toml", &merged, line);
    }

    let backups: Vec<_> = fs::read_dir(&scratch.0)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("config-with-cache.toml.bak-"))
        .collect();
    assert_eq!(backups.len(), 1, "exactly one dated backup");
    assert_eq!(
        fs::read_to_string(backups[0].path()).unwrap(),
        original,
        "the backup holds the pre-write content"
    );
}

/// Persisted values must survive a write/read/write round trip unchanged: a
/// second run is a byte-identical no-op that adds no second backup — the
/// Phase 2 restart trigger compares file content, so a no-op write must be
/// recognizable as one.
#[test]
fn repeated_write_is_a_byte_identical_no_op() {
    let scratch = Scratch::new("idempotent");
    let target = scratch.copy_fixture("config-with-cache.toml");
    let args = ["cache", "local_store", "/Volumes/coding/kache", "ignore_env", "true"];

    assert_eq!(merge(&target, &args).status.code(), Some(0));
    let first = fs::read_to_string(&target).unwrap();

    let second = merge(&target, &args);
    assert_eq!(second.status.code(), Some(0));
    assert!(
        stdout(&second).contains("already holds"),
        "the no-op is reported: {}",
        stdout(&second)
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        first,
        "the second write changes nothing"
    );

    let backups = |dir: &Path| {
        fs::read_dir(dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .map(|e| e.file_name().to_string_lossy().starts_with("config-with-cache.toml.bak-"))
                    .unwrap_or(false)
            })
            .count()
    };
    assert_eq!(backups(&scratch.0), 1, "the no-op adds no backup");
}

fn backup_count(dir: &Path, name: &str) -> usize {
    let prefix = format!("{name}.bak-");
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .count()
}

/// Review-1 finding #8: replacing a targeted key's value keeps that line's
/// indentation, `=` spacing, and trailing comment, and a `#` inside the old
/// quoted value is not mistaken for the comment's start.
#[test]
fn inline_comments_on_targeted_keys_survive_a_changing_write() {
    let scratch = Scratch::new("inline-comments");
    let target = scratch.copy_fixture("config-inline-comments.toml");
    let original = fs::read_to_string(&target).unwrap();

    let output = merge(
        &target,
        &[
            "cache",
            "local_store",
            "/Volumes/coding/kache",
            "ignore_env",
            "true",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr:\n{}",
        stderr(&output)
    );

    let merged = fs::read_to_string(&target).unwrap();
    let expected: String = original
        .replace(
            "  local_store = \"/old/#1 store\"   # retain this placement reason",
            "  local_store = \"/Volumes/coding/kache\"   # retain this placement reason",
        )
        .replace(
            "ignore_env = false # intentional for old setup",
            "ignore_env = true # intentional for old setup",
        );
    assert_ne!(expected, original, "the fixture holds both targeted lines");
    assert_eq!(merged, expected, "only the two values changed");

    let value = parse(&target);
    let cache = value.get("cache").and_then(|c| c.as_table()).unwrap();
    assert_eq!(
        cache.get("local_store").and_then(|v| v.as_str()),
        Some("/Volumes/coding/kache")
    );
    assert_eq!(
        cache.get("ignore_env").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(backup_count(&scratch.0, "config-inline-comments.toml"), 1);
}

/// With a trailing comment on each targeted line, a write of the values the
/// file already holds is a byte-identical no-op with no backup — both on the
/// untouched fixture and after a changing write.
#[test]
fn inline_comments_do_not_defeat_the_no_op_detection() {
    let scratch = Scratch::new("inline-noop");
    let target = scratch.copy_fixture("config-inline-comments.toml");
    let original = fs::read(&target).unwrap();

    let same = merge(
        &target,
        &[
            "cache",
            "local_store",
            "/old/#1 store",
            "ignore_env",
            "false",
        ],
    );
    assert_eq!(same.status.code(), Some(0), "stderr:\n{}", stderr(&same));
    assert!(stdout(&same).contains("already holds"), "{}", stdout(&same));
    assert_eq!(
        fs::read(&target).unwrap(),
        original,
        "current values change nothing"
    );
    assert_eq!(backup_count(&scratch.0, "config-inline-comments.toml"), 0);

    let args = [
        "cache",
        "local_store",
        "/Volumes/coding/kache",
        "ignore_env",
        "true",
    ];
    assert_eq!(merge(&target, &args).status.code(), Some(0));
    let first = fs::read(&target).unwrap();
    let second = merge(&target, &args);
    assert_eq!(second.status.code(), Some(0));
    assert!(
        stdout(&second).contains("already holds"),
        "{}",
        stdout(&second)
    );
    assert_eq!(
        fs::read(&target).unwrap(),
        first,
        "the repeat changes nothing"
    );
    assert_eq!(backup_count(&scratch.0, "config-inline-comments.toml"), 1);
}

/// A literal-string value holding `#` keeps its trailing comment too; a
/// multi-line value is refused with the original untouched rather than
/// half-rewritten.
#[test]
fn literal_strings_are_edited_and_multi_line_values_refused() {
    let scratch = Scratch::new("quote-styles");
    let literal = scratch.0.join("literal.toml");
    fs::write(&literal, "[cache]\nlocal_store = '/lit/#x' # why\n").unwrap();
    let output = merge(&literal, &["cache", "local_store", "/new"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr:\n{}",
        stderr(&output)
    );
    assert_eq!(
        fs::read_to_string(&literal).unwrap(),
        "[cache]\nlocal_store = \"/new\" # why\n"
    );

    let multi_line = scratch.0.join("multi-line.toml");
    let original = "[cache]\nlocal_store = \"\"\"\n/old\"\"\"\n";
    fs::write(&multi_line, original).unwrap();
    let output = merge(&multi_line, &["cache", "local_store", "/new"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a multi-line value is refused"
    );
    assert!(
        stderr(&output).contains("multi-line"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read_to_string(&multi_line).unwrap(), original);
}

/// A file with no `[cache]` table gets one appended at the end; the tables
/// that existed are untouched.
#[test]
fn missing_cache_table_is_appended() {
    let scratch = Scratch::new("append");
    let target = scratch.copy_fixture("config-without-cache.toml");
    let output = merge(&target, &["cache", "local_store", "/x", "ignore_env", "true"]);
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));

    let value = parse(&target);
    assert_eq!(
        value.get("build").and_then(|b| b.get("jobs")).and_then(|j| j.as_integer()),
        Some(8),
        "pre-existing tables survive"
    );
    let text = fs::read_to_string(&target).unwrap();
    assert!(
        text.trim_end().ends_with("ignore_env = true"),
        "the appended table is last:\n{text}"
    );
    assert!(text.contains("[net]"));
}

/// The activation write: `[build] rustc-wrapper = "kache"` lands beside the
/// existing keys and comments of a Cargo home config.
#[test]
fn cargo_activation_write_preserves_build_table() {
    let scratch = Scratch::new("cargo");
    let target = scratch.copy_fixture("cargo-config.toml");
    let output = merge(&target, &["build", "rustc-wrapper", "kache"]);
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));

    let value = parse(&target);
    let build = value.get("build").and_then(|b| b.as_table()).unwrap();
    assert_eq!(build.get("rustc-wrapper").and_then(|v| v.as_str()), Some("kache"));
    let text = fs::read_to_string(&target).unwrap();
    assert!(text.contains("hand-tuned"), "comments survive:\n{text}");
    assert!(
        text.contains("rustflags = [\"-Z\", \"threads=8\"]"),
        "existing keys survive:\n{text}"
    );
}

/// A malformed file is rejected with a non-zero exit and the original is
/// left untouched — no backup, no merge temp file, nothing but the error.
#[test]
fn malformed_file_is_rejected_untouched() {
    let scratch = Scratch::new("malformed");
    let target = scratch.copy_fixture("config-malformed.toml");
    let original = fs::read(&target).unwrap();

    let output = merge(&target, &["cache", "local_store", "/x"]);
    assert_ne!(output.status.code(), Some(0), "malformed input cannot pass");
    assert!(
        stderr(&output).contains("kache-config-merge: error="),
        "the error is named: {}",
        stderr(&output)
    );
    assert_eq!(fs::read(&target).unwrap(), original, "original untouched");

    let mut entries: Vec<String> = fs::read_dir(&scratch.0)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec!["config-malformed.toml".to_string()],
        "no backup and no temp litter: {entries:?}"
    );
}

/// An absent file is created with the table (fresh-host case), including its
/// parent directories.
#[test]
fn absent_file_is_created_with_parent_directories() {
    let scratch = Scratch::new("absent");
    let target = scratch.0.join("deep/config.toml");
    let output = merge(&target, &["cache", "local_store", "/x", "ignore_env", "true"]);
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));
    let value = parse(&target);
    assert_eq!(
        value
            .get("cache")
            .and_then(|c| c.get("ignore_env"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        stdout(&output).contains("created with"),
        "creation is reported: {}",
        stdout(&output)
    );
}

/// Representation variants: a store path with spaces stays a correctly
/// quoted TOML string, and `false`/`true` are booleans, not strings.
#[test]
fn value_variants_round_trip() {
    let scratch = Scratch::new("variants");
    let target = scratch.copy_fixture("config-without-cache.toml");
    let output = merge(&target, &["cache", "local_store", "/Volumes/My Disk/kache", "ignore_env", "false"]);
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));
    let value = parse(&target);
    let cache = value.get("cache").and_then(|c| c.as_table()).unwrap();
    assert_eq!(
        cache.get("local_store").and_then(|v| v.as_str()),
        Some("/Volumes/My Disk/kache")
    );
    assert_eq!(cache.get("ignore_env").and_then(|v| v.as_bool()), Some(false));
}

/// CRLF files keep their CRLF endings — "preserve every other key and
/// comment" includes the bytes they were written with.
#[test]
fn crlf_endings_are_preserved() {
    let scratch = Scratch::new("crlf");
    let target = scratch.0.join("crlf.toml");
    let original = fs::read(fixture("config-with-cache.toml")).unwrap();
    // The fixtures are LF; rebuild the same bytes with CRLF endings.
    let mut crlf = Vec::with_capacity(original.len());
    for byte in &original {
        if *byte == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(*byte);
    }
    fs::write(&target, &crlf).unwrap();

    let output = merge(&target, &["cache", "local_store", "/x"]);
    assert_eq!(output.status.code(), Some(0), "stderr:\n{}", stderr(&output));

    let merged = fs::read(&target).unwrap();
    let lfs = merged.iter().filter(|b| **b == b'\n').count();
    let crlfs = merged.windows(2).filter(|w| w == b"\r\n").count();
    assert_eq!(lfs, crlfs, "every newline is CRLF in the merged file");
}

/// Injection-shaped table and key names are rejected before anything is
/// written, and a bare argument list is a usage error.
#[test]
fn hostile_names_and_bare_arguments_are_rejected() {
    let scratch = Scratch::new("reject");
    let target = scratch.copy_fixture("config-with-cache.toml");
    let original = fs::read(&target).unwrap();

    let bad_table = merge(&target, &["a b", "k", "v"]);
    assert_ne!(bad_table.status.code(), Some(0));

    let bad_key = merge(&target, &["cache", "k = injected", "v"]);
    assert_ne!(bad_key.status.code(), Some(0));
    assert_eq!(fs::read(&target).unwrap(), original, "original untouched");
}

#[test]
fn odd_argument_count_is_a_usage_error() {
    let scratch = Scratch::new("usage");
    let target = scratch.copy_fixture("config-with-cache.toml");
    let output = merge(&target, &["cache", "lonely"]);
    assert_eq!(output.status.code(), Some(2));
}
