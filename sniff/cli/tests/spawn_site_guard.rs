//! Structural guards for Level 1 integration tests: raw `sniff` spawns, and
//! PATH escapes that must carry a call-site reason.

#[path = "common/source_scan.rs"]
mod source_scan;

use source_scan::{is_ident, line_at, sanitize};

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FORM_CARGO_BIN: &str = "cargo_bin";
const FORM_BIN_EXE: &str = "bin_exe";
const ESCAPE_FAKE_ONLY: &str = "fake_only_path";
const ESCAPE_HOST_PATH: &str = "host_path";
const EXCLUDED_PREFIXES: &[&str] = &["level2_", "real_"];
const REPORT_FILE: &str = "sniff-spawn-site-burn-down.jsonl";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    file: String,
    line: usize,
    form: &'static str,
}

struct AllowEntry {
    file: &'static str,
    reason: &'static str,
}

const SPAWN_ALLOWLIST: &[AllowEntry] = &[];

fn tests_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn excluded(relative: &str) -> bool {
    relative == "spawn_site_guard.rs"
        || relative.starts_with("common/")
        || relative.rsplit('/').next().is_some_and(|name| {
            EXCLUDED_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
}

fn collect_rust_files(root: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_files(&path, output)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            output.push(path);
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("scanned path must be below tests root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn spawn_sites(source: &str) -> Vec<(usize, &'static str)> {
    spawn_sites_with_forms(source, true, true)
}

fn spawn_sites_with_forms(
    source: &str,
    detect_cargo_bin: bool,
    detect_bin_exe: bool,
) -> Vec<(usize, &'static str)> {
    let sanitized = sanitize(source);
    let bytes = sanitized.as_bytes();
    let mut sites = Vec::new();
    if detect_cargo_bin {
        find_calls(
            source,
            bytes,
            b"cargo_bin",
            false,
            FORM_CARGO_BIN,
            &mut sites,
        );
    }
    if detect_bin_exe {
        find_calls(source, bytes, b"bin_exe", true, FORM_BIN_EXE, &mut sites);
    }
    sites.sort_unstable();
    sites
}

fn find_calls(
    source: &str,
    sanitized: &[u8],
    name: &[u8],
    requires_bang: bool,
    form: &'static str,
    sites: &mut Vec<(usize, &'static str)>,
) {
    let mut search = 0;
    while search + name.len() <= sanitized.len() {
        let Some(found) = sanitized[search..]
            .windows(name.len())
            .position(|window| window == name)
            .map(|offset| search + offset)
        else {
            break;
        };
        search = found + name.len();
        if found > 0 && is_ident(sanitized[found - 1]) {
            continue;
        }
        if sanitized.get(search).is_some_and(|byte| is_ident(*byte)) {
            continue;
        }
        if !requires_bang
            && sanitized[..found]
                .iter()
                .rposition(|byte| !byte.is_ascii_whitespace())
                .is_none_or(|last| last < 1 || &sanitized[last - 1..=last] != b"::")
        {
            continue;
        }

        let mut cursor = search;
        while sanitized.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if requires_bang {
            if sanitized.get(cursor) != Some(&b'!') {
                continue;
            }
            cursor += 1;
            while sanitized.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                cursor += 1;
            }
        }
        if sanitized.get(cursor) != Some(&b'(') || !first_argument_is_sniff(source, cursor + 1) {
            continue;
        }
        sites.push((line_at(source, found), form));
    }
}

fn first_argument_is_sniff(source: &str, mut cursor: usize) -> bool {
    let bytes = source.as_bytes();
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    bytes.get(cursor..cursor + 7) == Some(b"\"sniff\"")
}

/// A `SniffCommandBuilder` PATH escape and the reason recorded beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EscapeSite {
    file: String,
    line: usize,
    form: &'static str,
    reason: Option<String>,
}

fn escape_sites(file: &str, source: &str) -> Vec<EscapeSite> {
    escape_sites_with_forms(file, source, true, true)
}

fn escape_sites_with_forms(
    file: &str,
    source: &str,
    detect_fake_only: bool,
    detect_host: bool,
) -> Vec<EscapeSite> {
    let sanitized = sanitize(source);
    let bytes = sanitized.as_bytes();
    let mut sites = Vec::new();
    if detect_fake_only {
        find_method_calls(
            source,
            bytes,
            b"fake_only_path",
            ESCAPE_FAKE_ONLY,
            &mut sites,
        );
    }
    if detect_host {
        find_method_calls(source, bytes, b"host_path", ESCAPE_HOST_PATH, &mut sites);
    }
    sites.sort_unstable();
    sites
        .into_iter()
        .map(|(line, form)| EscapeSite {
            file: file.to_string(),
            line,
            form,
            reason: justification(source, &sanitized, line),
        })
        .collect()
}

fn path_escape_sites(source: &str) -> Vec<(usize, &'static str)> {
    escape_sites("scanned.rs", source)
        .into_iter()
        .map(|site| (site.line, site.form))
        .collect()
}

fn find_method_calls(
    source: &str,
    sanitized: &[u8],
    name: &[u8],
    form: &'static str,
    sites: &mut Vec<(usize, &'static str)>,
) {
    let mut search = 0;
    while search + name.len() <= sanitized.len() {
        let Some(found) = sanitized[search..]
            .windows(name.len())
            .position(|window| window == name)
            .map(|offset| search + offset)
        else {
            break;
        };
        search = found + name.len();
        if found > 0 && is_ident(sanitized[found - 1]) {
            continue;
        }
        if sanitized.get(search).is_some_and(|byte| is_ident(*byte)) {
            continue;
        }
        let receiver = sanitized[..found]
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace());
        if receiver.is_none_or(|last| sanitized[last] != b'.') {
            continue;
        }
        let mut cursor = search;
        while sanitized.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if sanitized.get(cursor) != Some(&b'(') {
            continue;
        }
        sites.push((line_at(source, found), form));
    }
}

/// The `//` comment attached to `line`: its own trailing comment, else the
/// contiguous comment block ending at the nearest preceding non-blank line.
fn justification(source: &str, sanitized: &str, line: usize) -> Option<String> {
    if let Some(reason) = line_comment(source, sanitized, line) {
        return Some(reason);
    }
    let blank = |number: usize| {
        source
            .lines()
            .nth(number - 1)
            .is_some_and(|text| text.trim().is_empty())
    };
    let mut candidate = line.saturating_sub(1);
    while candidate >= 1 && blank(candidate) {
        candidate -= 1;
    }
    let mut block = Vec::new();
    while candidate >= 1 {
        let Some(text) = whole_line_comment(source, sanitized, candidate) else {
            break;
        };
        block.push(text);
        candidate -= 1;
    }
    block.reverse();
    (!block.is_empty()).then(|| block.join(" "))
}

/// A comment occupying the whole line, so a trailing comment on unrelated code
/// above cannot be borrowed as the escape's reason.
fn whole_line_comment(source: &str, sanitized: &str, line: usize) -> Option<String> {
    sanitized
        .lines()
        .nth(line - 1)
        .filter(|blanked| blanked.trim().is_empty())?;
    line_comment(source, sanitized, line)
}

/// Comment text on `line`, accepted only when the sanitized source shows that
/// nothing but the comment follows — so a `"// …"` string literal is not a reason.
fn line_comment(source: &str, sanitized: &str, line: usize) -> Option<String> {
    let original = source.lines().nth(line - 1)?;
    let blanked = sanitized.lines().nth(line - 1)?;
    let start = original.find("//")?;
    if !blanked.get(start..)?.trim().is_empty() {
        return None;
    }
    let text = original[start..].trim_start_matches('/').trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn unjustified(sites: &[EscapeSite]) -> Vec<String> {
    sites
        .iter()
        .filter(|site| site.reason.is_none())
        .map(|site| format!("{}:{} {}", site.file, site.line, site.form))
        .collect()
}

fn scanned_sources(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)
        .map_err(|error| format!("failed to scan {}: {error}", root.display()))?;
    files.sort();
    let mut sources = Vec::new();
    for file in files {
        let relative = relative(root, &file);
        if excluded(&relative) {
            continue;
        }
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {relative}: {error}"))?;
        sources.push((relative, source));
    }
    Ok(sources)
}

fn scan(root: &Path) -> Result<Vec<Site>, String> {
    let mut sites = Vec::new();
    for (relative, source) in scanned_sources(root)? {
        sites.extend(spawn_sites(&source).into_iter().map(|(line, form)| Site {
            file: relative.clone(),
            line,
            form,
        }));
    }
    Ok(sites)
}

fn scan_path_escapes(root: &Path) -> Result<Vec<EscapeSite>, String> {
    let mut sites = Vec::new();
    for (relative, source) in scanned_sources(root)? {
        sites.extend(escape_sites(&relative, &source));
    }
    Ok(sites)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Reconciliation {
    unlisted: Vec<String>,
    stale: Vec<String>,
    unexplained: Vec<String>,
    counts: BTreeMap<String, usize>,
}

fn reconcile(sites: &[Site], allowlist: &[AllowEntry]) -> Reconciliation {
    let allowed: BTreeSet<&str> = allowlist.iter().map(|entry| entry.file).collect();
    let mut result = Reconciliation::default();
    for site in sites {
        if allowed.contains(site.file.as_str()) {
            *result.counts.entry(site.file.clone()).or_default() += 1;
        } else {
            result
                .unlisted
                .push(format!("{}:{} {}", site.file, site.line, site.form));
        }
    }
    for entry in allowlist {
        if !sites.iter().any(|site| site.file == entry.file) {
            result.stale.push(entry.file.to_string());
        }
        if entry.reason.trim().is_empty() {
            result.unexplained.push(entry.file.to_string());
        }
    }
    result
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Record<'a> {
    File {
        file: &'a str,
        reason: &'a str,
        sites: usize,
    },
    Total {
        files: usize,
        sites: usize,
        scanned_sites: usize,
    },
}

fn report(sites: &[Site], result: &Reconciliation) -> String {
    let mut output = String::new();
    for entry in SPAWN_ALLOWLIST {
        if let Some(count) = result.counts.get(entry.file) {
            output.push_str(
                &serde_json::to_string(&Record::File {
                    file: entry.file,
                    reason: entry.reason,
                    sites: *count,
                })
                .unwrap(),
            );
            output.push('\n');
        }
    }
    output.push_str(
        &serde_json::to_string(&Record::Total {
            files: result.counts.len(),
            sites: result.counts.values().sum(),
            scanned_sites: sites.len(),
        })
        .unwrap(),
    );
    output.push('\n');
    output
}

#[test]
fn l1_tests_spawn_sniff_through_the_fixture_builder_or_an_explicit_entry() {
    let sites = scan(&tests_root()).expect("spawn-site population must scan");
    let result = reconcile(&sites, SPAWN_ALLOWLIST);
    let artifact = report(&sites, &result);
    eprintln!(
        "sniff spawn-site burn-down: {} raw sites across {} allow-listed files ({} scanned)",
        result.counts.values().sum::<usize>(),
        result.counts.len(),
        sites.len()
    );
    for entry in SPAWN_ALLOWLIST {
        if let Some(count) = result.counts.get(entry.file) {
            eprintln!("  {}: {count} — {}", entry.file, entry.reason);
        }
    }
    let path = test_toolkit::stage_dir().join(REPORT_FILE);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(error) = fs::write(&path, artifact) {
        eprintln!("warning: could not write {}: {error}", path.display());
    }

    assert!(
        result.unlisted.is_empty(),
        "raw sniff spawns outside the fixture builder:\n{}",
        result.unlisted.join("\n")
    );
    assert!(
        result.stale.is_empty(),
        "stale SPAWN_ALLOWLIST entries:\n{}",
        result.stale.join("\n")
    );
    assert!(
        result.unexplained.is_empty(),
        "SPAWN_ALLOWLIST entries without reasons:\n{}",
        result.unexplained.join("\n")
    );
}

#[test]
fn detector_finds_all_raw_spawn_forms_and_ignores_prose_and_strings() {
    assert_eq!(
        spawn_sites("let c = assert_cmd::Command::cargo_bin(\"sniff\").unwrap();\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(
        spawn_sites("let p = assert_cmd::cargo::cargo_bin ( \"sniff\" );\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(
        spawn_sites("let p = biscuit_test_harness::bin_exe! ( \"sniff\" );\n"),
        [(1, FORM_BIN_EXE)]
    );
    assert!(spawn_sites("//! `Command::cargo_bin(\"sniff\")` is raw.\n").is_empty());
    assert!(spawn_sites("/* bin_exe!(\"sniff\") */\n").is_empty());
    assert!(spawn_sites("let prose = r#\"Command::cargo_bin(\"sniff\")\"#;\n").is_empty());
    assert!(spawn_sites("Command::cargo_bin(\"other\");\n").is_empty());
    assert!(spawn_sites("fixture.command_std().spawn().unwrap();\n").is_empty());
}

#[test]
fn restored_detector_rejects_a_violation_that_a_neutered_detector_misses() {
    let source = "let c = assert_cmd::Command::cargo_bin(\"sniff\").unwrap();\n";
    let file = |sites: Vec<(usize, &'static str)>| {
        sites
            .into_iter()
            .map(|(line, form)| Site {
                file: "new_test.rs".to_string(),
                line,
                form,
            })
            .collect::<Vec<_>>()
    };

    let neutered = reconcile(&file(spawn_sites_with_forms(source, false, true)), &[]);
    assert!(
        neutered.unlisted.is_empty(),
        "neutered detector misses the site"
    );
    let restored = reconcile(&file(spawn_sites(source)), &[]);
    assert_eq!(restored.unlisted, ["new_test.rs:1 cargo_bin"]);
}

#[test]
fn l1_path_escapes_carry_a_call_site_reason() {
    let sites = scan_path_escapes(&tests_root()).expect("path-escape population must scan");
    assert!(
        !sites.is_empty(),
        "the PATH-escape detector found no live sites; it has stopped observing the suite"
    );
    eprintln!("sniff PATH escapes: {} sites", sites.len());
    for site in &sites {
        eprintln!(
            "  {}:{} {} — {}",
            site.file,
            site.line,
            site.form,
            site.reason.as_deref().unwrap_or("<no reason>")
        );
    }
    let missing = unjustified(&sites);
    assert!(
        missing.is_empty(),
        "PATH escapes without a call-site reason naming the tool observed or the \
         absence proved:\n{}",
        missing.join("\n")
    );
}

#[test]
fn escape_detector_finds_both_path_escapes_and_ignores_prose_and_strings() {
    assert_eq!(
        path_escape_sites("let c = fixture.command_builder().fake_only_path();\n"),
        [(1, ESCAPE_FAKE_ONLY)]
    );
    assert_eq!(
        path_escape_sites("builder\n    .host_path ()\n    .build();\n"),
        [(2, ESCAPE_HOST_PATH)]
    );
    assert!(path_escape_sites("//! `.host_path()` widens the child PATH.\n").is_empty());
    assert!(path_escape_sites("/* .fake_only_path() */\n").is_empty());
    assert!(path_escape_sites("let prose = \"builder.host_path()\";\n").is_empty());
    assert!(path_escape_sites("pub fn host_path(mut self) -> Self { self }\n").is_empty());
    assert!(path_escape_sites("builder.host_path_entries();\n").is_empty());
    assert!(path_escape_sites("builder.fake_only_path;\n").is_empty());
}

#[test]
fn escape_justification_accepts_adjacent_comments_and_rejects_string_lookalikes() {
    let reason = |source: &str| {
        escape_sites("a.rs", source)
            .into_iter()
            .next()
            .expect("one escape site")
            .reason
    };

    assert_eq!(
        reason("builder.host_path(); // git is the tool under observation\n"),
        Some("git is the tool under observation".to_string())
    );
    assert_eq!(
        reason("// no host executable may be probed\nbuilder.fake_only_path();\n"),
        Some("no host executable may be probed".to_string())
    );
    assert_eq!(
        reason("// git is the tool under observation\n\n\nbuilder.host_path();\n"),
        Some("git is the tool under observation".to_string())
    );
    assert_eq!(
        reason("// git is the tool\n// under observation\nbuilder.host_path();\n"),
        Some("git is the tool under observation".to_string())
    );
    assert_eq!(reason("builder.host_path(); //\n"), None);
    assert_eq!(
        reason("let banner = \"// git is the tool\";\nbuilder.host_path();\n"),
        None
    );
    assert_eq!(
        reason("let banner = 1; // unrelated\nbuilder.host_path();\n"),
        None
    );
}

#[test]
fn restored_escape_detector_rejects_an_unjustified_site_that_a_neutered_detector_misses() {
    let source = "fixture.command_builder().host_path().build();\n";

    let neutered = unjustified(&escape_sites_with_forms("new_test.rs", source, true, false));
    assert!(
        neutered.is_empty(),
        "neutered detector misses the unjustified escape"
    );
    let restored = unjustified(&escape_sites("new_test.rs", source));
    assert_eq!(restored, ["new_test.rs:1 host_path"]);
}

#[test]
fn reconciliation_rejects_stale_and_unexplained_entries() {
    let live = [Site {
        file: "live.rs".to_string(),
        line: 4,
        form: FORM_CARGO_BIN,
    }];
    let entries = [
        AllowEntry {
            file: "live.rs",
            reason: "",
        },
        AllowEntry {
            file: "gone.rs",
            reason: "removed",
        },
    ];
    let result = reconcile(&live, &entries);
    assert_eq!(result.stale, ["gone.rs"]);
    assert_eq!(result.unexplained, ["live.rs"]);
}
