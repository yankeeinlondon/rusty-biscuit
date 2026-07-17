//! Structural drift guards for the two composition seams pinned by Phase 1 of
//! `features/2026-07-13-proxy-with/plan.md`.
//!
//! Both guards answer a question behavioral tests cannot: *did a new semantic
//! owner appear?* A test can prove that the composer we call behaves correctly;
//! only a structural scan can prove that no **second** composer — or second
//! optional proxy-target channel — grew alongside it.
//!
//! - [`compose_with_allowlist_holds_the_line`] enumerates every production
//!   caller of Darkmatter's `compose_with`. The feature's R1 bans a second
//!   composition implementation; Phase 5 retired the two
//!   `harness_orch::prompt::materialize_harness_prompt` arms — that function now
//!   routes through `composition::prepare::prepare_document` — so
//!   [`COMPOSE_WITH_ALLOWLIST`] shrank by exactly those two.
//! - [`subtree_compose_baseline_holds_the_line`] enumerates every production
//!   entry point to Darkmatter's event-time subtree composition (DM2). Phase 4
//!   routes `proxy.with` values through the executor's existing one; the guard
//!   is what proves no *second* interpolator grew to serve them.
//! - [`no_new_optional_proxy_target_channel`] enumerates every
//!   `Option<PathBuf>`-shaped proxy-target carrier. Two exist today; the spec's
//!   R1 bans a second channel, and Phase 6 drives
//!   [`PROXY_TARGET_CHANNEL_BASELINE`] to zero.
//!
//! ## Site identity
//!
//! Sites are keyed on `{parent_module}::{file_stem}::{enclosing_item}` — stable
//! under line moves and reformatting, unlike a line number, and specific enough
//! to name one semantic owner, unlike a substring ban. A site that moves within
//! its function does not fail the guard; a site that appears in a **new**
//! function does.
//!
//! ## Relationship to `dispatch_inventory.rs`
//!
//! That guard is the site-level model this one follows (allowlist entries
//! carrying a reason, staleness failing as loudly as a new site). It scans for
//! a different construct and predates this file, so it carries its own lexer;
//! the two are deliberately independent test binaries.
//!
//! ## Scanner scope
//!
//! Production `.rs` under `lib/src/` and `cli/src/` only. Comments, string and
//! char literals, and `#[cfg(test)]` module bodies are blanked before matching,
//! so prose mentioning `compose_with` cannot trip the guard. Whole-file test
//! modules (`tests.rs`, `tests/`) are excluded by path.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// A production call site of Darkmatter's `compose_with`, keyed by site
/// identity, with the number of calls expected in that function.
///
/// Every entry is a **sanctioned semantic owner of composition**. Adding one
/// means adding a composer; the reason must say why that is not a duplicate of
/// `composition::prepare`.
const COMPOSE_WITH_ALLOWLIST: &[AllowedSite] = &[
    AllowedSite {
        site: "composition::prepare::prepare_direct_with_prompt",
        calls: 1,
        reason: "the canonical direct-document composer (the sanctioned owner); \
                 `prepare_direct` and the canonical service's every entry \
                 delegate here",
    },
    AllowedSite {
        site: "composition::prepare::prepare_inline",
        calls: 1,
        reason: "the canonical inline-compose composer (the sanctioned owner)",
    },
    AllowedSite {
        site: "system_prompt::prepare::compose_prompt_markdown",
        calls: 1,
        reason: "system-prompt documents compose outside the prompt pipeline; \
                 not a prompt composer",
    },
    AllowedSite {
        site: "wrap::overlay::materialize_passthrough_harness_seed",
        calls: 1,
        reason: "passthrough seed materialization for the harness overlay",
    },
];

/// Every `Option<PathBuf>`-shaped proxy-target carrier in production today.
///
/// **Empty, and it stays empty.** The spec's R1 bans an optional proxy-target
/// channel: a proxy whose consumption is optional is a proxy that can be
/// silently dropped, which is the motivating bug. Phase 6 collapsed the two
/// that existed — the loop engine's `LoopExecutionResult::init_proxy_target`
/// and the pipeline's `route_initialize::init_proxy_target` — into the one
/// `DocumentTransition`, whose `Proxy` variant a caller cannot ignore without
/// a compile error.
const PROXY_TARGET_CHANNEL_BASELINE: &[AllowedSite] = &[];

/// Every production entry point to Darkmatter's event-time subtree composition
/// (DM2).
///
/// This is the counterpart to [`COMPOSE_WITH_ALLOWLIST`] one layer down: that
/// guard bans a second *document* composer, this one bans a second
/// *interpolator*. Behavioral tests can show that `proxy.with` resolves
/// correctly; only a structural scan can show it does so by delegating to the
/// shared substrate rather than through a bespoke Claudine grammar grown
/// alongside it.
const SUBTREE_COMPOSE_BASELINE: &[AllowedSite] = &[
    AllowedSite {
        site: "composition::interpolation_conformance::dm2_render",
        calls: 1,
        reason: "the conformance harness that pins DM1/DM2 agreement; not a \
                 production evaluation path",
    },
    AllowedSite {
        site: "composition::preflight::resolve_shell_command_expr",
        calls: 1,
        reason: "pre-flight shell-command resolution — early-binding only, \
                 before any event fires",
    },
    AllowedSite {
        site: "lifecycle::executor::resolve_string_value",
        calls: 1,
        reason: "THE event-time interpolator for every lifecycle surface, \
                 `proxy.with` values included. A second site here would be a \
                 second interpolation grammar.",
    },
];

/// One allowlisted site: a stable identity, an expected occurrence count, and
/// the justification a reviewer needs to judge a change to this list.
struct AllowedSite {
    site: &'static str,
    calls: usize,
    reason: &'static str,
}

/// A scanned occurrence: its site identity and the location a human needs to
/// go look at it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Found {
    site: String,
    path: String,
    line: usize,
}

/// One raw match from a finder, before module/enclosing-item resolution.
struct Occurrence {
    offset: usize,
    /// Appended to the resolved `{module}::{enclosing_item}` identity. A named
    /// declaration (a channel) contributes its binding name; an unnamed
    /// construct (a call) contributes nothing.
    name: Option<String>,
}

#[test]
fn compose_with_allowlist_holds_the_line() {
    let found = scan_all(find_compose_with_calls);
    assert_baseline(&found, COMPOSE_WITH_ALLOWLIST, "compose_with call site", "R1 bans a second composition implementation. A new `compose_with` caller is a \
         new semantic owner of composition: either route it through \
         `claudine::composition::prepare` or add it to `COMPOSE_WITH_ALLOWLIST` \
         with a reason saying why it is not a duplicate.");
}

#[test]
fn subtree_compose_baseline_holds_the_line() {
    let found = scan_all(find_subtree_compose_calls);
    assert_baseline(
        &found,
        SUBTREE_COMPOSE_BASELINE,
        "SubtreeCompose entry point",
        "Event-time interpolation has one owner: `lifecycle::executor::resolve_string_value`. \
         A new `SubtreeCompose::new` site is a second interpolator — route the value through \
         the executor's existing resolution instead, or add the site to \
         `SUBTREE_COMPOSE_BASELINE` with a reason saying why it is not a duplicate grammar.",
    );
}

#[test]
fn no_new_optional_proxy_target_channel() {
    let found = scan_all(find_proxy_target_channels);
    assert_baseline(&found, PROXY_TARGET_CHANNEL_BASELINE, "optional proxy-target channel", "R1 bans a second optional proxy-target channel: an `Option`-shaped target \
         can be silently dropped by a caller that forgets to consume it, which is \
         the motivating bug. Return the typed document transition instead.");
}

/// Compare scanned sites against a seeded baseline, failing on a new site, a
/// stale entry, and a changed occurrence count alike.
///
/// A stale entry fails as loudly as a new site: a baseline that still names a
/// deleted owner silently stops guarding anything.
fn assert_baseline(found: &[Found], baseline: &[AllowedSite], what: &str, guidance: &str) {
    let failures = check_baseline(found, baseline, what);
    assert!(
        failures.is_empty(),
        "{what} drift guard failed:\n  {}\n\n{guidance}",
        failures.join("\n  ")
    );
}

/// The comparison itself, returning one message per drift. Split out from
/// [`assert_baseline`] so the guard's own failure modes are unit-testable
/// without a production source file to violate them.
fn check_baseline(found: &[Found], baseline: &[AllowedSite], what: &str) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();

    for site in unique_sites(found) {
        if !baseline.iter().any(|a| a.site == site) {
            let at = locations(found, &site);
            failures.push(format!("NEW {what} `{site}` at {at} is not in the baseline"));
        }
    }

    for allowed in baseline {
        let actual = found.iter().filter(|f| f.site == allowed.site).count();
        if actual == 0 {
            failures.push(format!(
                "STALE baseline entry `{}` matches no live {what} — delete it \
                 (recorded reason: {})",
                allowed.site, allowed.reason
            ));
        } else if actual != allowed.calls {
            let at = locations(found, allowed.site);
            failures.push(format!(
                "COUNT CHANGED for `{}`: baseline expects {}, found {actual} at {at}",
                allowed.site, allowed.calls
            ));
        }
    }

    failures
}

fn unique_sites(found: &[Found]) -> BTreeSet<String> {
    found.iter().map(|f| f.site.clone()).collect()
}

fn locations(found: &[Found], site: &str) -> String {
    found
        .iter()
        .filter(|f| f.site == site)
        .map(|f| format!("{}:{}", f.path, f.line))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run `finder` over every production source file and return all occurrences,
/// each resolved to a `{module}::{enclosing_item}[::{name}]` identity.
fn scan_all(finder: impl Fn(&[u8]) -> Vec<Occurrence>) -> Vec<Found> {
    let area = area_root();
    let mut out = Vec::new();
    for root in ["lib/src", "cli/src"] {
        for file in rust_files(&area.join(root)) {
            let rel = file
                .strip_prefix(&area)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if is_test_path(&rel) {
                continue;
            }
            let src = fs::read_to_string(&file).expect("read source");
            let sanitized = sanitize(&src);
            for found in finder(&sanitized) {
                let module = module_identity(&rel);
                let item = enclosing_item(&sanitized, found.offset)
                    .unwrap_or_else(|| "<file-scope>".to_string());
                let suffix = found.name.map(|n| format!("::{n}")).unwrap_or_default();
                out.push(Found {
                    site: format!("{module}::{item}{suffix}"),
                    path: rel.clone(),
                    line: line_of(&sanitized, found.offset),
                });
            }
        }
    }
    out.sort();
    out
}

/// The `claudine/` package-area root (this test binary lives in `claudine/cli`).
fn area_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/cli has a parent")
        .to_path_buf()
}

/// Test-only sources: whole-file test modules and test directories. Inline
/// `#[cfg(test)]` modules are handled by the sanitizer instead.
fn is_test_path(rel: &str) -> bool {
    rel.ends_with("/tests.rs") || rel.contains("/tests/")
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

/// `{parent_dir}::{file_stem}` — the stable two-segment module identity the
/// plan's baseline is written in (e.g. `composition::prepare`). A `mod.rs`
/// takes its directory's name, matching how the module is referred to in Rust.
fn module_identity(rel: &str) -> String {
    let parts: Vec<&str> = rel.trim_end_matches(".rs").split('/').collect();
    let (stem, parent) = match parts.as_slice() {
        [.., parent, "mod"] => (*parent, parts.get(parts.len().wrapping_sub(3)).copied()),
        [.., parent, stem] => (*stem, Some(*parent)),
        _ => return rel.to_string(),
    };
    match parent {
        Some(p) if p != "src" => format!("{p}::{stem}"),
        _ => stem.to_string(),
    }
}

/// Every `.compose_with(` method call. A call has no declared name, so its
/// identity is its enclosing function alone.
fn find_compose_with_calls(src: &[u8]) -> Vec<Occurrence> {
    find_all(src, b".compose_with(")
        .into_iter()
        .map(|offset| Occurrence { offset, name: None })
        .collect()
}

/// Every `SubtreeCompose::new(` constructor call — the entry point to DM2.
fn find_subtree_compose_calls(src: &[u8]) -> Vec<Occurrence> {
    find_all(src, b"SubtreeCompose::new(")
        .into_iter()
        .map(|offset| Occurrence { offset, name: None })
        .collect()
}

/// Byte offsets of every declaration whose type is `Option<..PathBuf>` and
/// whose binding name names a proxy target.
///
/// Matches both semantic shapes a channel can take: a struct field
/// (`pub init_proxy_target: Option<PathBuf>`) and a local binding
/// (`let mut init_proxy_target: Option<std::path::PathBuf>`). A struct-literal
/// initializer (`init_proxy_target: None`) carries no type annotation and so is
/// correctly not a declaration. `Option<&Path>` parameters are borrowed views of
/// a channel, not the channel itself, and are out of scope by the same rule.
fn find_proxy_target_channels(src: &[u8]) -> Vec<Occurrence> {
    let text = String::from_utf8_lossy(src);
    let mut out = Vec::new();
    for (offset, _) in text.match_indices("Option<") {
        // Require the sole generic argument to be a path-typed `PathBuf`, so a
        // nested generic (`Option<Vec<PathBuf>>`) is not a target channel.
        let Some(close) = text[offset..].find('>') else {
            continue;
        };
        let inner = text[offset + "Option<".len()..offset + close].trim();
        if !inner.ends_with("PathBuf") || inner.contains('<') {
            continue;
        }
        // Walk back over `: ` to the identifier being declared.
        let Some(colon) = text[..offset].rfind(':') else {
            continue;
        };
        if text[colon + 1..offset].trim() != "" {
            continue;
        }
        let name: String = text[..colon]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let lower = name.to_ascii_lowercase();
        if lower.contains("proxy") && lower.contains("target") {
            out.push(Occurrence {
                offset,
                name: Some(name),
            });
        }
    }
    out
}

fn find_all(src: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= src.len() {
        if &src[i..i + needle.len()] == needle {
            out.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// The innermost `fn` / `struct` / `impl`-associated item whose body encloses
/// `offset`, as a `::`-joined path (e.g. `LoopExecutionResult::init_proxy_target`
/// resolves through its enclosing `struct`).
///
/// Returns the item **name**; the caller prefixes the module. Nested `fn`s
/// resolve to the innermost, which is what a site identity wants.
fn enclosing_item(src: &[u8], offset: usize) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (kw, len) in [("fn ", 3usize), ("struct ", 7), ("enum ", 5)] {
        for start in find_all(src, kw.as_bytes()) {
            if start > offset {
                break;
            }
            // Require the keyword to start a token.
            if start > 0 && is_ident_byte(src[start - 1]) {
                continue;
            }
            let name_start = skip_ws(src, start + len);
            let mut name_end = name_start;
            while name_end < src.len() && is_ident_byte(src[name_end]) {
                name_end += 1;
            }
            if name_end == name_start {
                continue;
            }
            // Find this item's brace body and test containment.
            let Some(brace) = next_brace(src, name_end) else {
                continue;
            };
            let Some(close) = matching_brace(src, brace) else {
                continue;
            };
            if brace < offset && offset < close {
                let span = close - brace;
                let name = String::from_utf8_lossy(&src[name_start..name_end]).to_string();
                if best.as_ref().is_none_or(|(b, _)| span < *b) {
                    best = Some((span, name));
                }
            }
        }
    }
    best.map(|(_, name)| name)
}

/// The next `{` that opens an item body, skipping a generic/where/param list.
/// Returns `None` if a `;` (a declaration without a body) comes first.
fn next_brace(src: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    let mut depth = 0i32;
    while i < src.len() {
        match src[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth <= 0 => return Some(i),
            b';' if depth <= 0 => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

fn matching_brace(src: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, b) in src.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn skip_ws(src: &[u8], mut i: usize) -> usize {
    while i < src.len() && src[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

fn line_of(src: &[u8], offset: usize) -> usize {
    src[..offset].iter().filter(|b| **b == b'\n').count() + 1
}

/// Blank comments, string/char literals, and `#[cfg(test)]` module bodies to
/// spaces, preserving newlines so line numbers stay accurate.
///
/// Matching runs on the result, so prose or a fixture string naming
/// `compose_with` cannot trip a guard.
fn sanitize(src: &str) -> Vec<u8> {
    let bytes = src.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            blank(&mut out, start, i);
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            let start = i;
            let mut depth = 1;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            blank(&mut out, start, i);
        } else if (b == b'r' || b == b'b') && (i == 0 || !is_ident_byte(bytes[i - 1])) {
            match raw_string_open(bytes, i) {
                Some((content_start, hashes)) => {
                    let start = i;
                    i = content_start;
                    while i < bytes.len() {
                        if bytes[i] == b'"'
                            && bytes[i + 1..].len() >= hashes
                            && bytes[i + 1..].iter().take(hashes).all(|&h| h == b'#')
                        {
                            i += 1 + hashes;
                            break;
                        }
                        i += 1;
                    }
                    blank(&mut out, start, i);
                }
                None => i += 1,
            }
        } else if b == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            blank(&mut out, start, i);
        } else if b == b'\'' {
            match char_literal_end(bytes, i) {
                Some(end) => {
                    blank(&mut out, i, end);
                    i = end;
                }
                // A lifetime or loop label, not a literal.
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    blank_cfg_test_modules(&mut out);
    out
}

fn blank(out: &mut [u8], from: usize, to: usize) {
    for slot in out.iter_mut().take(to).skip(from) {
        if *slot != b'\n' {
            *slot = b' ';
        }
    }
}

/// If a raw (byte) string opens at `i` (`r"`, `r#"`, `br#"`, ..), return the
/// offset just past the opening quote and the hash count.
fn raw_string_open(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    let mut j = i;
    if bytes.get(j) == Some(&b'b') {
        j += 1;
    }
    if bytes.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let hashes_start = j;
    while bytes.get(j) == Some(&b'#') {
        j += 1;
    }
    if bytes.get(j) != Some(&b'"') {
        return None;
    }
    Some((j + 1, j - hashes_start))
}

/// The offset just past a char literal opening at `i`, or `None` when `'` opens
/// a lifetime/label instead.
fn char_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    if bytes.get(j) == Some(&b'\\') {
        j += 2;
    } else {
        // Advance one UTF-8 scalar.
        j += 1;
        while j < bytes.len() && (bytes[j] & 0xC0) == 0x80 {
            j += 1;
        }
    }
    (bytes.get(j) == Some(&b'\'')).then_some(j + 1)
}

/// Blank the body of every inline `#[cfg(test)] mod NAME { .. }`. Runs on
/// already-sanitized bytes, so a `#[cfg(test)]` inside a comment cannot trigger
/// blanking.
fn blank_cfg_test_modules(bytes: &mut [u8]) {
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b'#' && bytes.get(i + 1) == Some(&b'[') {
            let Some(close) = matching_bracket(bytes, i + 1) else {
                i += 1;
                continue;
            };
            if attr_is_cfg_test(&bytes[i..=close]) {
                let mut j = skip_ws(bytes, close + 1);
                if bytes[j..].starts_with(b"mod") && bytes.get(j + 3).is_some_and(|b| !is_ident_byte(*b)) {
                    let mut k = skip_ws(bytes, j + 3);
                    while k < n && is_ident_byte(bytes[k]) {
                        k += 1;
                    }
                    k = skip_ws(bytes, k);
                    if bytes.get(k) == Some(&b'{')
                        && let Some(body_close) = matching_brace(bytes, k)
                    {
                        for slot in bytes.iter_mut().take(body_close).skip(k + 1) {
                            if *slot != b'\n' {
                                *slot = b' ';
                            }
                        }
                        i = body_close + 1;
                        continue;
                    }
                }
                j = skip_ws(bytes, j);
                let _ = j;
            }
            i = close + 1;
            continue;
        }
        i += 1;
    }
}

fn matching_bracket(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// True when `attr` (`#` through the closing `]`) is a `#[cfg(test)]` gate.
/// Deliberately conservative: `#[cfg(feature = "test-utils")]` is not a gate.
fn attr_is_cfg_test(attr: &[u8]) -> bool {
    let text = std::str::from_utf8(attr).unwrap_or("");
    let inner = text.trim_start_matches("#[").trim_end_matches(']').trim();
    let Some(pred) = inner.strip_prefix("cfg(") else {
        return false;
    };
    let pred = pred.trim_start();
    pred == "test" || pred.starts_with("test)") || pred.starts_with("test,") || pred.starts_with("test ")
}

#[cfg(test)]
mod scanner_tests {
    use super::*;

    /// The sanitizer is what makes the guards trustworthy: prose and fixture
    /// strings naming a guarded construct must not register as call sites.
    #[test]
    fn comments_and_strings_do_not_register_as_sites() {
        let src = r#"
            // md.compose_with(options)
            /// A doc comment mentioning .compose_with( too.
            fn real() {
                let fixture = "md.compose_with(x)";
                let _ = fixture;
            }
        "#;
        let sanitized = sanitize(src);
        assert!(
            find_compose_with_calls(&sanitized).is_empty(),
            "commented and quoted `compose_with` must not be scanned as call sites"
        );
    }

    #[test]
    fn a_real_call_registers_with_its_enclosing_fn() {
        let src = r#"
            fn prepare_direct() {
                let (composed, _) = md.compose_with(compose_opts)?;
            }
        "#;
        let sanitized = sanitize(src);
        let found = find_compose_with_calls(&sanitized);
        assert_eq!(found.len(), 1, "the real call is found");
        assert_eq!(
            enclosing_item(&sanitized, found[0].offset).as_deref(),
            Some("prepare_direct"),
            "the site resolves to its enclosing function, not a line number"
        );
    }

    #[test]
    fn cfg_test_module_bodies_are_out_of_scope() {
        let src = r#"
            #[cfg(test)]
            mod tests {
                fn t() {
                    let _ = md.compose_with(opts);
                }
            }
        "#;
        assert!(
            find_compose_with_calls(&sanitize(src)).is_empty(),
            "test-module composition is not production composition"
        );
    }

    /// Both shapes a proxy-target channel takes today — a struct field and a
    /// local binding — must be seen, while a struct-literal initializer (no
    /// type annotation) and a borrowed `Option<&Path>` view must not.
    #[test]
    fn proxy_target_channel_matches_declarations_only() {
        let field = sanitize("struct R { pub init_proxy_target: Option<PathBuf>, }");
        assert_eq!(find_proxy_target_channels(&field).len(), 1, "struct field is a channel");

        let binding = sanitize("fn f() { let mut init_proxy_target: Option<std::path::PathBuf> = None; }");
        assert_eq!(find_proxy_target_channels(&binding).len(), 1, "local binding is a channel");

        let literal = sanitize("fn f() { R { init_proxy_target: None } }");
        assert!(
            find_proxy_target_channels(&literal).is_empty(),
            "a struct-literal initializer declares no channel"
        );

        let borrowed = sanitize("fn f(initial_proxy_target: Option<&Path>) {}");
        assert!(
            find_proxy_target_channels(&borrowed).is_empty(),
            "a borrowed view is not the owning channel"
        );

        let unrelated = sanitize("fn f() { let some_path: Option<PathBuf> = None; }");
        assert!(
            find_proxy_target_channels(&unrelated).is_empty(),
            "an unrelated Option<PathBuf> is not a proxy-target channel"
        );
    }

    fn at(site: &str, line: usize) -> Found {
        Found {
            site: site.to_string(),
            path: "lib/src/composition/prepare.rs".to_string(),
            line,
        }
    }

    const ONE_SITE: &[AllowedSite] = &[AllowedSite {
        site: "composition::prepare::prepare_direct",
        calls: 1,
        reason: "the sanctioned owner",
    }];

    /// The baseline must be exactly satisfied — not merely covered.
    #[test]
    fn a_matching_baseline_reports_no_drift() {
        let found = vec![at("composition::prepare::prepare_direct", 198)];
        assert!(check_baseline(&found, ONE_SITE, "site").is_empty());
    }

    /// The guard's whole purpose: a site in a function nobody sanctioned fails.
    #[test]
    fn a_new_semantic_owner_fails_the_guard() {
        let found = vec![
            at("composition::prepare::prepare_direct", 198),
            at("harness_orch::prompt::sneaky_second_composer", 42),
        ];
        let failures = check_baseline(&found, ONE_SITE, "site");
        assert_eq!(failures.len(), 1, "exactly the new site fails; got {failures:?}");
        assert!(
            failures[0].contains("NEW site") && failures[0].contains("sneaky_second_composer"),
            "the failure must name the new owner and where to find it; got {failures:?}"
        );
    }

    /// A baseline entry that no longer matches any live site stops guarding
    /// anything, so staleness must fail as loudly as a new site. This is how
    /// Phase 5 and Phase 6 are forced to shrink their baselines rather than
    /// leave a dead entry behind.
    #[test]
    fn a_stale_baseline_entry_fails_the_guard() {
        let failures = check_baseline(&[], ONE_SITE, "site");
        assert_eq!(failures.len(), 1, "the stale entry fails; got {failures:?}");
        assert!(
            failures[0].contains("STALE") && failures[0].contains("prepare_direct"),
            "the failure must name the dead entry; got {failures:?}"
        );
    }

    /// A second call appearing inside an already-sanctioned function is still
    /// drift — this is what catches a `materialize_harness_prompt`-shaped
    /// composer growing a third arm.
    #[test]
    fn an_extra_call_in_an_allowed_site_fails_the_guard() {
        let found = vec![
            at("composition::prepare::prepare_direct", 198),
            at("composition::prepare::prepare_direct", 210),
        ];
        let failures = check_baseline(&found, ONE_SITE, "site");
        assert_eq!(failures.len(), 1, "the count change fails; got {failures:?}");
        assert!(
            failures[0].contains("COUNT CHANGED") && failures[0].contains("expects 1, found 2"),
            "the failure must report expected vs actual; got {failures:?}"
        );
    }

    #[test]
    fn module_identity_is_parent_and_stem() {
        assert_eq!(module_identity("lib/src/composition/prepare.rs"), "composition::prepare");
        assert_eq!(
            module_identity("cli/src/commands/wrap/harness_orch/prompt.rs"),
            "harness_orch::prompt"
        );
        assert_eq!(module_identity("cli/src/commands/wrap/overlay.rs"), "wrap::overlay");
    }
}
