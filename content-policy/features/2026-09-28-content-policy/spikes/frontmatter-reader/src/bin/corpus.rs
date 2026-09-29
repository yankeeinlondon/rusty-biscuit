//! Corpus parity: strict spike reader vs Darkmatter's `try_from_content`.
//! Usage: corpus <repo-root>

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use frontmatter_reader_spike::{ReadError, ReadOutcome, parse_strict, read_frontmatter};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if ft.is_dir() {
            if matches!(name.as_ref(), "target" | "node_modules" | ".git") {
                continue;
            }
            walk(&path, out);
        } else if name.ends_with(".md") {
            out.push(path);
        }
    }
}

/// Darkmatter's tab fallback: leading tabs become two spaces each.
fn tab_normalize(yaml: &str) -> String {
    yaml.split('\n')
        .map(|line| {
            let indent: String = line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();
            let rest = &line[indent.len()..];
            format!("{}{rest}", indent.replace('\t', "  "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("repo root"));
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut examples: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut bump = |bucket: String, example: String| {
        *counts.entry(bucket.clone()).or_default() += 1;
        let list = examples.entry(bucket).or_default();
        if list.len() < 30 {
            list.push(example);
        }
    };

    let mut scanned = 0;
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        let starts = bytes
            .strip_prefix(b"\xef\xbb\xbf")
            .unwrap_or(&bytes)
            .starts_with(b"---");
        if !starts {
            continue;
        }
        scanned += 1;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            bump("non-utf8".into(), rel);
            continue;
        };
        let spike = read_frontmatter(&bytes);
        let dm = darkmatter::markdown::Markdown::try_from_content(text);
        match (&spike, &dm) {
            (Ok(ReadOutcome::Found(fm)), Ok(md)) => {
                let dm_map = md.frontmatter().as_map();
                if dm_map == &fm.record {
                    bump("both ok, records equal".into(), rel);
                } else {
                    let diff: Vec<String> = fm
                        .record
                        .iter()
                        .filter(|(k, v)| dm_map.get(*k) != Some(*v))
                        .map(|(k, _)| k.clone())
                        .chain(
                            dm_map
                                .keys()
                                .filter(|k| !fm.record.contains_key(*k))
                                .cloned(),
                        )
                        .collect();
                    bump(
                        "both ok, RECORDS DIFFER".into(),
                        format!("{rel} keys={diff:?}"),
                    );
                }
            }
            (Ok(other), Ok(md)) => {
                let label = match other {
                    ReadOutcome::Unterminated { dot_close } => {
                        format!("both ok, no block: unterminated (dot_close={dot_close})")
                    }
                    ReadOutcome::NoFrontmatter => {
                        "both ok, no block: first line not a fence (e.g. `---x`)".into()
                    }
                    ReadOutcome::NearMissFence => "both ok, near-miss".into(),
                    ReadOutcome::Found(_) => unreachable!(),
                };
                let dm_empty = md.frontmatter().is_empty();
                bump(format!("{label}; dm frontmatter empty={dm_empty}"), rel);
            }
            (Err(e), Err(de)) => {
                let kind = match e {
                    ReadError::Duplicate(_) => "duplicate",
                    ReadError::Yaml(_) => "yaml",
                    ReadError::NotMapping(_) => "not-mapping",
                    ReadError::NotUtf8 => "utf8",
                };
                bump(
                    format!("both fail ({kind})"),
                    format!("{rel}: dm={}", first_line(&de.to_string())),
                );
            }
            (Err(e), Ok(_)) => {
                // Why did Darkmatter recover? Re-run strict parse on its
                // tab-normalized text, then check for protectable expressions.
                let yaml = yaml_of(text);
                let reason = if parse_strict(&tab_normalize(&yaml)).is_ok() {
                    "tab normalization".to_string()
                } else if yaml.contains("$(") || yaml.contains("{{") {
                    let which = match (yaml.contains("$("), yaml.contains("{{")) {
                        (true, true) => "$(...) and {{ }}",
                        (true, false) => "$(...)",
                        _ => "{{ }}",
                    };
                    format!("expression protection: {which}")
                } else {
                    "other".into()
                };
                bump(
                    format!("DM ok, spike fails [{reason}]"),
                    format!("{rel}: {}", first_line(&format!("{e:?}"))),
                );
            }
            (Ok(o), Err(de)) => {
                let kind = matches!(o, ReadOutcome::Found(_))
                    .then_some("found")
                    .unwrap_or("no block");
                bump(
                    format!("spike ok ({kind}), DM fails"),
                    format!("{rel}: {}", first_line(&de.to_string())),
                );
            }
        }
    }

    println!(
        "markdown files: {}  starting with ---: {scanned}\n",
        files.len()
    );
    for (bucket, n) in &counts {
        println!("{n:>5}  {bucket}");
        for ex in &examples[bucket] {
            println!("         - {ex}");
        }
    }
}

fn first_line(s: &str) -> String {
    s.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(160)
        .collect()
}

/// YAML between fences, LF-joined like Darkmatter.
fn yaml_of(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let close = lines
        .iter()
        .skip(1)
        .position(|l| l.trim() == "---")
        .map(|i| i + 1)
        .unwrap_or(1);
    lines[1..close].join("\n")
}
