//! AC29 / R37 migration guard for the nesting the clean break removed
//! (feature `2026-09-09-more-context`, ruling R33): `current.ctx.` and
//! `current.env.` may appear only where `ALLOWLIST` expects them.
//!
//! Scope rule. Starting at the workspace root — the nearest ancestor of this
//! crate's `CARGO_MANIFEST_DIR` whose `Cargo.toml` declares `[workspace]` —
//! every regular file is scanned whose extension is `rs`, `md`, `toml`,
//! `yaml`, `yml`, or `json`, plus every regular file under a directory named
//! `prompts` regardless of extension. That covers code and tests, shipped
//! prompts, schemas and templates, user docs, READMEs, and the
//! `.claude/skills` tree. Directories named `target`, `node_modules`,
//! `features`, or `fixes` are skipped at any depth, as is every dot-directory
//! other than `.claude` (`.git`, `.gitnexus`, ...); symlinks are never
//! followed. Skipping `features/` and `fixes/` excludes planning documents —
//! specs, plans, reviews, logs, decisions, and their `_completed` and
//! `_unscheduled` lifecycle directories — which are neither code, shipped
//! prompts, user docs, nor skills; every in-flight spec the migration list
//! names lives there.
//!
//! Each occurrence of either literal counts once, so a line naming both
//! spellings counts twice. The scan must equal `ALLOWLIST` exactly: a file
//! outside it with any occurrence is a new use of the removed spelling, and
//! an entry whose count moved is stale. An allowlisted occurrence is a
//! negative test proving the spelling is rejected, or documentation / error
//! text explaining the removal; the one exception is annotated inline. This
//! file lists itself because its documentation names the spellings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const NEEDLES: [&str; 2] = ["current.ctx.", "current.env."];
const EXTENSIONS: [&str; 6] = ["rs", "md", "toml", "yaml", "yml", "json"];
const SKIPPED_DIRS: [&str; 4] = ["target", "node_modules", "features", "fixes"];

/// Workspace-relative `/`-separated path, expected occurrence count.
const ALLOWLIST: &[(&str, usize)] = &[
    // Skills explaining the clean break (R29–R33).
    (".claude/skills/claudine/SKILL.md", 2),
    (".claude/skills/claudine/composition.md", 2),
    (".claude/skills/claudine/lifecycle.md", 2),
    (".claude/skills/darkmatter/compose.md", 2),
    // Exception: a Rust field chain, `current.env.push(..)` on a local named
    // `current`, not the document spelling. The literal scan cannot tell
    // them apart.
    ("claudine/cli/tests/level2_lifecycle_control.rs", 1),
    // Negative test: `{{current.ctx.branch}}` fails under Claudine.
    ("claudine/lib/src/composition/lifecycle/context/tests.rs", 2),
    // Doc comment naming the contract that replaced the nesting.
    ("claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs", 1),
    // Negative test through the `md` binary.
    ("darkmatter/cli/tests/compose_lazy_roots.rs", 2),
    // User docs: "There is no nesting".
    ("darkmatter/docs/topics/darkmatter-expressions.md", 2),
    // Negative test: the nesting names no capability and no eager requirement.
    ("darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs", 2),
    // Doc comment on the eager-requirement scanner.
    ("darkmatter/lib/src/markdown/compose/context/capture/groups.rs", 1),
    // Doc comment plus negative test on the lazy-root member lookup.
    ("darkmatter/lib/src/markdown/compose/context/current.rs", 4),
    // `LazyRootMemberUnknown` docs and error text.
    ("darkmatter/lib/src/markdown/compose/expression/error.rs", 4),
    // Negative test through the compose pipeline.
    ("darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs", 2),
    // This guard: the module doc, `NEEDLES`, the annotations above, and the
    // scope fixture below.
    ("darkmatter/lib/tests/current_root_migration_guard.rs", 16),
];

fn workspace_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    let mut dir = manifest_dir.as_path();
    loop {
        let manifest = dir.join("Cargo.toml");
        if std::fs::read_to_string(&manifest).is_ok_and(|text| text.contains("[workspace]")) {
            return dir.to_path_buf();
        }
        dir = dir
            .parent()
            .expect("a workspace Cargo.toml above CARGO_MANIFEST_DIR");
    }
}

fn is_skipped_dir(name: &str) -> bool {
    SKIPPED_DIRS.contains(&name) || (name.starts_with('.') && name != ".claude")
}

fn has_scanned_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| EXTENSIONS.contains(&extension))
}

/// 1-based line of every occurrence, per file, keyed by `/`-separated path
/// relative to `root`. Files without an occurrence are absent.
fn scan(root: &Path) -> BTreeMap<String, Vec<usize>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![(root.to_path_buf(), false)];
    while let Some((dir, under_prompts)) = pending.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()));
        for entry in entries {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry.file_type().expect("file type");
            if kind.is_dir() {
                if !is_skipped_dir(&name) {
                    pending.push((path, under_prompts || name == "prompts"));
                }
            } else if kind.is_file() && (under_prompts || has_scanned_extension(&path)) {
                let text = std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
                let text = String::from_utf8_lossy(&text);
                let lines: Vec<usize> = text
                    .lines()
                    .enumerate()
                    .flat_map(|(index, line)| {
                        let hits: usize = NEEDLES.iter().map(|needle| line.matches(needle).count()).sum();
                        std::iter::repeat_n(index + 1, hits)
                    })
                    .collect();
                if !lines.is_empty() {
                    let relative = path.strip_prefix(root).expect("under the root");
                    let key = relative
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    found.insert(key, lines);
                }
            }
        }
    }
    found
}

#[test]
fn the_removed_nesting_appears_only_where_the_allowlist_expects() {
    let allowed: BTreeMap<&str, usize> = ALLOWLIST.iter().copied().collect();
    assert_eq!(allowed.len(), ALLOWLIST.len(), "duplicate allowlist entry");

    let found = scan(&workspace_root());
    let mut problems = Vec::new();
    for (path, lines) in &found {
        match allowed.get(path.as_str()) {
            None => problems.push(format!("new occurrence outside the allowlist: {path} at lines {lines:?}")),
            Some(&expected) if expected != lines.len() => problems.push(format!(
                "stale allowlist count for {path}: expected {expected}, found {} at lines {lines:?}",
                lines.len()
            )),
            Some(_) => {}
        }
    }
    for (path, expected) in ALLOWLIST {
        if !found.contains_key(*path) {
            problems.push(format!("stale allowlist entry: {path} expects {expected} occurrences but has none"));
        }
    }
    assert!(problems.is_empty(), "AC29 migration guard:\n  {}", problems.join("\n  "));
}

/// The scanner counts every occurrence and honors the scope rule, so the
/// guard above cannot pass vacuously.
#[test]
fn the_scan_counts_occurrences_and_honors_the_scope_rule() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("src/a.rs", "let x = \"{{ current.ctx.a }} {{ current.env.B }}\";\n\n// current.ctx.c\n");
    write("prompts/format.tmpl", "{{ current.env.HOME }}\n");
    write(".claude/skills/topic/SKILL.md", "no `current.ctx.x` nesting\n");
    write("notes.txt", "current.ctx.ignored: extension out of scope\n");
    write("features/2026-01-01-x/spec.md", "current.ctx.ignored: planning document\n");
    write("fixes/_completed/y/spec.md", "current.env.ignored: planning document\n");
    write("target/debug/build.rs", "current.ctx.ignored: build output\n");
    write(".git/COMMIT_EDITMSG", "current.ctx.ignored: dot-directory\n");
    write("clean.md", "current_env.HOME and ctx.current are fine\n");

    let found = scan(root.path());

    assert_eq!(
        found,
        BTreeMap::from([
            (".claude/skills/topic/SKILL.md".to_string(), vec![1]),
            ("prompts/format.tmpl".to_string(), vec![1]),
            ("src/a.rs".to_string(), vec![1, 1, 3]),
        ]),
    );
}
