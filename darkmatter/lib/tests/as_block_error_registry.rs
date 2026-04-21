//! Drift guard: verify every `impl BlockError for X` in `darkmatter/lib/src`
//! has a corresponding downcast arm in [`as_block_error`].
//!
//! Adding a new `BlockError` impl without updating the registry causes that
//! error type to silently fall back to the `Display` chain when routed through
//! [`as_block_error`]. This test parses the source tree at test time and
//! asserts the registry is kept in sync.
//!
//! [`as_block_error`]: darkmatter::markdown::errors::as_block_error

use std::path::Path;

#[test]
fn as_block_error_registry_covers_all_block_error_impls() {
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(
        src_root.exists(),
        "source root {} does not exist — is CARGO_MANIFEST_DIR set correctly?",
        src_root.display()
    );

    let mut impl_names = Vec::new();
    collect_impl_block_error_types(&src_root, &mut impl_names);

    assert!(
        !impl_names.is_empty(),
        "failed to find any impl BlockError in {} — regex may need updating",
        src_root.display()
    );

    let registry_source = include_str!("../src/markdown/errors/mod.rs");
    let mut missing = Vec::new();
    for name in &impl_names {
        if !registry_source.contains(&format!("downcast_ref::<{name}>")) {
            missing.push(name.as_str());
        }
    }

    if !missing.is_empty() {
        panic!(
            "impl BlockError for {{ {} }} found in source but no matching \
             `downcast_ref::<{{}}>` arm in `as_block_error` in \
             darkmatter/lib/src/markdown/errors/mod.rs\n\
             Add the missing arm to restore BlockError rendering for that type.",
            missing.join(", ")
        );
    }
}

fn collect_impl_block_error_types(dir: &Path, out: &mut Vec<String>) {
    let entries = std::fs::read_dir(dir).expect("read_dir failed");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_impl_block_error_types(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content =
                std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("impl") && line.contains("BlockError") && line.contains("for")
                    && let Some(name) = extract_type_name(line)
                {
                    out.push(name);
                }
            }
        }
    }
}

fn extract_type_name(line: &str) -> Option<String> {
    let pattern = "for ";
    let start = line.find(pattern)? + pattern.len();
    let rest = &line[start..];
    let end = rest
        .find(['<', ',', ' ', ';', '{'])
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        Some(name.to_string())
    } else {
        None
    }
}
