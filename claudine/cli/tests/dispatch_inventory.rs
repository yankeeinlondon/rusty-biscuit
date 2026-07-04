//! Mechanical dispatch inventory for the `claudine-cli` crate (provider-metadata
//! Phase A2, `features/2026-07-02-provider-metadata/design/pipeline-dry.md`).
//!
//! Scans every `.rs` file under `cli/src/` for per-variant `Provider` dispatch
//! and regenerates the committed inventory at
//! `claudine/docs/providers/dispatch-inventory.json`. The drift test
//! byte-compares the regenerated document against the committed file, so a
//! plain `just test` run fails whenever dispatch sites change without a
//! matching inventory update. Integration tests under `cli/tests/` are out of
//! scope by design.
//!
//! Regenerate after intentional dispatch changes:
//!
//! ```text
//! CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory
//! ```
//!
//! ## Pattern forms (pattern-set v1)
//!
//! Extends the lib guard's three forms (`no_unauthorized_match_provider_in_lib`
//! in `lib/src/provider/tests.rs`) with the dominant CLI forms named in
//! `design/pipeline-dry.md`. Every `Provider::<Variant>` occurrence in
//! non-comment, non-literal source is classified into exactly one form:
//!
//! - `match-provider` — `match expr { Provider::X => .. }`; one record per
//!   `match` expression, listing every variant named in its arm patterns.
//! - `matches-macro` — `matches!(expr, Provider::X | ..)`; one record per call.
//! - `eq-comparison` / `ne-comparison` — `expr == Provider::X` / `!=`, either
//!   operand order.
//! - `tuple-array` — `[(Provider::X, ..), ..]` fact tables; one record per
//!   array literal.
//! - `let-pattern` — `let Provider::X` destructuring (`if let`, `while let`,
//!   `let .. else`).
//! - `provider-arm` — `Provider::X =>` arms outside a recognized `match` body
//!   (defensive residual; matches the lib guard's standalone-arm form).
//! - `direct-ref` — any other variant reference (fixture arrays, constructor
//!   arguments, function call operands, ..). Not dispatch-shaped by itself but
//!   recorded so the inventory total reconciles against a raw grep.
//!
//! Because classification is total, `totals.provider_refs` equals the raw
//! occurrence count of `Provider::<KnownVariant>` in `cli/src/**` minus hits
//! inside comments and string/char literals. Lookalike enums (`TtsProvider`,
//! `HostTtsProvider`, ..) and lowercase associated functions
//! (`Provider::parse_cli_name`, ..) are excluded by the variant-name filter.
//!
//! ## Notes
//!
//! Known limitations of the line-oriented lexer (an inventory, not a compiler):
//!
//! - Code produced by macros is invisible; `macro_rules!` bodies are classified
//!   by textual shape. Matches-style macros other than `matches!` (for example
//!   `assert_matches!`) classify their interior variants as `direct-ref`.
//! - `Some(Provider::X)` inside `if let` classifies as `direct-ref` because the
//!   `let` keyword is not adjacent to the variant path.
//! - Inline `#[cfg(test)]` modules inside `src/**` files are inventoried and
//!   are not distinguishable by path; only `src/**/tests/` directories receive
//!   the path-based `exempt_candidate` tag. The Phase I guard, not this
//!   inventory, decides final test exemptions.
//! - `exempt_candidate` mirrors the blanket-exemption list in
//!   `design/pipeline-dry.md` (`commands/wrap/profile/*.rs`, the clap mapping
//!   in `main.rs`, test paths). Those sites are still recorded because they
//!   feed the Phase D WrapperProfile disposition table.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

const PATTERN_SET_VERSION: u32 = 2;
const BLESS_ENV: &str = "CLAUDINE_UPDATE_INVENTORY";
const REGEN_COMMAND: &str =
    "CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory";

const CLASS_CONDITIONAL: &str = "conditional";
const CLASS_REFERENCE: &str = "reference";

const FORM_MATCH: &str = "match-provider";
const FORM_MATCHES: &str = "matches-macro";
const FORM_EQ: &str = "eq-comparison";
const FORM_NE: &str = "ne-comparison";
const FORM_TUPLE: &str = "tuple-array";
const FORM_LET: &str = "let-pattern";
const FORM_ARM: &str = "provider-arm";
const FORM_DIRECT: &str = "direct-ref";

/// One inventory record: a dispatch site (or grouped construct) in `cli/src`.
#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    /// Repo-relative path, forward slashes on every platform.
    path: String,
    /// 1-based line of the construct anchor (`match` / `matches!` / `[` /
    /// first occurrence for per-line forms).
    line: usize,
    form: &'static str,
    /// `conditional` when the construct varies behavior by provider (branch
    /// forms, plus `tuple-array` fact tables — a per-provider lookup table is
    /// a data-driven branch and a catalog-data candidate); `reference` when
    /// the code merely names a provider (fixtures, constructor arguments).
    /// Derived from `form`; Phase D's disposition table and Phase I's guard
    /// allow-list filter on this field (Checkpoint A ruling, 2026-07-04).
    dispatch_class: &'static str,
    /// Deduplicated variant names in the construct, sorted alphabetically.
    providers: Vec<String>,
    /// True when the path falls under the guard's blanket-exemption candidates
    /// (`commands/wrap/profile/*.rs`, `main.rs`, `src/**/tests/` paths).
    exempt_candidate: bool,
}

#[derive(Serialize)]
struct Totals {
    sites: usize,
    provider_refs: usize,
    by_form: BTreeMap<&'static str, usize>,
    by_dispatch_class: BTreeMap<&'static str, usize>,
}

#[derive(Serialize)]
struct Inventory {
    tool: &'static str,
    pattern_set_version: u32,
    scanned_root: &'static str,
    regenerate: &'static str,
    forms: BTreeMap<&'static str, &'static str>,
    dispatch_classes: BTreeMap<&'static str, &'static str>,
    totals: Totals,
    sites: Vec<Site>,
}

fn form_descriptions() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (FORM_MATCH, "match expression with Provider::X arm patterns (one record per match)"),
        (FORM_MATCHES, "matches!(expr, Provider::X | ..) macro call (one record per call)"),
        (FORM_EQ, "expr == Provider::X comparison"),
        (FORM_NE, "expr != Provider::X comparison"),
        (FORM_TUPLE, "[(Provider::X, ..)] tuple-array fact table (one record per array)"),
        (FORM_LET, "let Provider::X destructuring (if let / while let / let-else)"),
        (FORM_ARM, "Provider::X => arm outside a recognized match body"),
        (FORM_DIRECT, "any other Provider::X variant reference"),
    ])
}

/// Derive the record's `dispatch_class` from its pattern form. Every form
/// except `direct-ref` varies behavior by provider identity: the branch forms
/// obviously, and `tuple-array` because a per-provider fact table is a
/// data-driven branch (its production instance is the stderr noise-suppression
/// table, a `DisplayPolicy` catalog candidate).
fn dispatch_class(form: &'static str) -> &'static str {
    if form == FORM_DIRECT {
        CLASS_REFERENCE
    } else {
        CLASS_CONDITIONAL
    }
}

fn dispatch_class_descriptions() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            CLASS_CONDITIONAL,
            "behavior varies by provider (branch forms + tuple-array fact tables); \
             the Phase D disposition-table / Phase I guard universe",
        ),
        (
            CLASS_REFERENCE,
            "code merely names a provider (fixtures, constructor arguments); \
             nothing to migrate",
        ),
    ])
}

fn is_ident(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// Replace comments and string/char literals with spaces (newlines preserved)
/// so classification only sees code shape and line numbers stay accurate.
/// Nested block comments and raw strings (`r#".."#`, `br".."`) are handled;
/// non-ASCII char literals are blanked to their closing quote.
fn sanitize(src: &str) -> Vec<u8> {
    let bytes = src.as_bytes();
    let mut out = bytes.to_vec();
    let blank = |out: &mut [u8], from: usize, to: usize| {
        for slot in out.iter_mut().take(to).skip(from) {
            if *slot != b'\n' {
                *slot = b' ';
            }
        }
    };

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
        } else if (b == b'r' || b == b'b')
            && (i == 0 || !is_ident(bytes[i - 1]))
            && let Some((content_start, hashes)) = raw_string_open(bytes, i)
        {
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
        } else if b == b'\'' {
            if let Some(end) = char_literal_end(bytes, i) {
                blank(&mut out, i, end);
                i = end;
            } else {
                // Lifetime or loop label: leave untouched.
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// If a raw (byte) string opens at `i` (`r"`, `r#"`, `br"`, ..), return the
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
    let mut hashes = 0;
    while bytes.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    if bytes.get(j) == Some(&b'"') {
        Some((j + 1, hashes))
    } else {
        None
    }
}

/// End offset (exclusive) of a char literal opening at `i`, or `None` when the
/// quote starts a lifetime/label instead.
fn char_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    match bytes.get(i + 1) {
        Some(b'\\') => {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j] != b'\'' {
                j += 1;
            }
            Some((j + 1).min(bytes.len()))
        }
        Some(&c) if c >= 0x80 => {
            // Non-ASCII char literal such as 'é'.
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'\'' {
                j += 1;
            }
            Some((j + 1).min(bytes.len()))
        }
        Some(_) if bytes.get(i + 2) == Some(&b'\'') => Some(i + 3),
        _ => None,
    }
}

/// A `Provider::<Variant>` occurrence in sanitized source.
struct Occurrence {
    start: usize,
    end: usize,
    variant: String,
}

fn occurrences(s: &[u8], variants: &[String]) -> Vec<Occurrence> {
    const NEEDLE: &[u8] = b"Provider::";
    let mut found = Vec::new();
    let mut i = 0;
    while i + NEEDLE.len() <= s.len() {
        if &s[i..i + NEEDLE.len()] == NEEDLE && (i == 0 || !is_ident(s[i - 1])) {
            let ident_start = i + NEEDLE.len();
            let mut j = ident_start;
            while j < s.len() && is_ident(s[j]) {
                j += 1;
            }
            let ident = std::str::from_utf8(&s[ident_start..j]).unwrap_or("");
            if variants.iter().any(|v| v == ident) {
                found.push(Occurrence {
                    start: i,
                    end: j,
                    variant: ident.to_string(),
                });
            }
            i = j;
        } else {
            i += 1;
        }
    }
    found
}

fn skip_ws(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Offset of the last non-whitespace byte strictly before `i`.
fn prev_non_ws(s: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    while j > 0 {
        j -= 1;
        if !s[j].is_ascii_whitespace() {
            return Some(j);
        }
    }
    None
}

/// Spans of `matches!(..)` invocations: (keyword offset, open, close).
struct MacroSpan {
    kw: usize,
    open: usize,
    close: usize,
}

fn matches_macro_spans(s: &[u8]) -> Vec<MacroSpan> {
    const KW: &[u8] = b"matches";
    let mut spans = Vec::new();
    let mut i = 0;
    while i + KW.len() < s.len() {
        let bounded = &s[i..i + KW.len()] == KW
            && (i == 0 || !is_ident(s[i - 1]))
            && !is_ident(s[i + KW.len()]);
        if bounded {
            let bang = skip_ws(s, i + KW.len());
            if s.get(bang) == Some(&b'!') {
                let open = skip_ws(s, bang + 1);
                if let Some(&delim) = s.get(open)
                    && let Some(close) = matching_delimiter(s, open, delim)
                {
                    spans.push(MacroSpan { kw: i, open, close });
                    i = open + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    spans
}

/// Offset of the delimiter closing the one opening at `open`, counting nesting
/// of the same delimiter kind only (same-kind pairs are balanced in valid code).
fn matching_delimiter(s: &[u8], open: usize, delim: u8) -> Option<usize> {
    let closer = match delim {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let mut depth = 0usize;
    let mut i = open;
    while i < s.len() {
        if s[i] == delim {
            depth += 1;
        } else if s[i] == closer {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// A `match` expression: keyword offset, body span, and the arm pattern
/// regions (pattern + optional guard, up to each depth-0 `=>`).
struct MatchBlock {
    kw: usize,
    pattern_regions: Vec<(usize, usize)>,
}

fn match_blocks(s: &[u8]) -> Vec<MatchBlock> {
    const KW: &[u8] = b"match";
    let mut blocks = Vec::new();
    let mut i = 0;
    while i + KW.len() < s.len() {
        let bounded = &s[i..i + KW.len()] == KW
            && (i == 0 || !is_ident(s[i - 1]))
            && !is_ident(s[i + KW.len()]);
        if bounded
            && let Some(body_open) = scrutinee_body_open(s, i + KW.len())
            && let Some(body_close) = matching_delimiter(s, body_open, b'{')
        {
            blocks.push(MatchBlock {
                kw: i,
                pattern_regions: pattern_regions(s, body_open + 1, body_close),
            });
        }
        i += 1;
    }
    blocks
}

/// Scan the scrutinee after a `match` keyword for the `{` opening the match
/// body: the first `{` at paren/bracket depth zero. Bails on `;` or a stray
/// closing `}` (malformed or non-expression context).
fn scrutinee_body_open(s: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 0i32;
    while i < s.len() {
        match s[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(i),
            b';' => return None,
            b'}' if depth == 0 => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split a match body into arm pattern regions. Each region runs from the end
/// of the previous arm's body to the next depth-0 `=>` and therefore includes
/// any `if` guard. Arm bodies (block or expression) are skipped, so variants
/// referenced inside them classify independently.
fn pattern_regions(s: &[u8], body_start: usize, body_end: usize) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut region_start = body_start;
    let mut depth = 0i32;
    let mut i = body_start;
    while i < body_end {
        match s[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0
                && s.get(i + 1) == Some(&b'>')
                && (i == 0 || !matches!(s[i - 1], b'=' | b'!' | b'<' | b'>')) =>
            {
                regions.push((region_start, i));
                i = skip_arm_body(s, i + 2, body_end);
                region_start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    regions
}

/// Advance past one arm body starting just after `=>`, returning the offset of
/// the next arm's pattern region start.
fn skip_arm_body(s: &[u8], i: usize, body_end: usize) -> usize {
    let mut j = skip_ws(s, i);
    if j >= body_end {
        return body_end;
    }
    if s[j] == b'{' {
        match matching_delimiter(s, j, b'{') {
            Some(close) if close < body_end => {
                j = skip_ws(s, close + 1);
                if j < body_end && s[j] == b',' {
                    j += 1;
                }
                return j;
            }
            _ => return body_end,
        }
    }
    // Expression body: ends at the next comma at local depth zero (or the
    // match body's closing brace, which drives local depth negative).
    let mut depth = 0i32;
    while j < body_end {
        match s[j] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth < 0 {
                    return j;
                }
            }
            b',' if depth == 0 => return j + 1,
            _ => {}
        }
        j += 1;
    }
    body_end
}

/// Spans of `[(Provider::X, ..)]` tuple arrays: (open bracket, close bracket).
fn tuple_array_spans(s: &[u8], occs: &[Occurrence]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for occ in occs {
        let Some(paren) = prev_non_ws(s, occ.start) else {
            continue;
        };
        if s[paren] != b'(' {
            continue;
        }
        let Some(bracket) = prev_non_ws(s, paren) else {
            continue;
        };
        if s[bracket] != b'[' {
            continue;
        }
        if let Some(close) = matching_delimiter(s, bracket, b'[') {
            spans.push((bracket, close));
        }
    }
    spans
}

fn eq_ne_adjacent(s: &[u8], occ: &Occurrence) -> Option<&'static str> {
    let classify = |a: u8, b: u8| match (a, b) {
        (b'=', b'=') => Some(FORM_EQ),
        (b'!', b'=') => Some(FORM_NE),
        _ => None,
    };
    if let Some(j) = prev_non_ws(s, occ.start)
        && j >= 1
        && let Some(form) = classify(s[j - 1], s[j])
        // Exclude `<=` / `>=` / `==` chains resolving to other tokens.
        && (j < 2 || !matches!(s[j - 2], b'<' | b'>' | b'=' | b'!'))
    {
        return Some(form);
    }
    let j = skip_ws(s, occ.end);
    if j + 1 < s.len()
        && let Some(form) = classify(s[j], s[j + 1])
    {
        return Some(form);
    }
    None
}

fn is_let_pattern(s: &[u8], start: usize) -> bool {
    let Some(j) = prev_non_ws(s, start) else {
        return false;
    };
    j >= 2 && &s[j - 2..=j] == b"let" && (j == 2 || !is_ident(s[j - 3]))
}

/// Forward heuristic for `Provider::X => ` arms outside recognized match
/// bodies: skip `| Provider::Y` alternations and up to three tuple/slice
/// closers, then require `=>`.
fn looks_like_arm(s: &[u8], end: usize) -> bool {
    let mut i = end;
    let mut closers = 0;
    loop {
        i = skip_ws(s, i);
        match s.get(i) {
            Some(b'|') if s.get(i + 1) != Some(&b'|') => {
                i = skip_ws(s, i + 1);
                let path_start = i;
                while i < s.len() && (is_ident(s[i]) || s[i] == b':') {
                    i += 1;
                }
                if i == path_start {
                    return false;
                }
            }
            Some(b')') | Some(b']') if closers < 3 => {
                closers += 1;
                i += 1;
            }
            Some(b'=') if s.get(i + 1) == Some(&b'>') => return true,
            _ => return false,
        }
    }
}

/// 1-based line number of byte offset `pos`.
fn line_of(line_starts: &[usize], pos: usize) -> usize {
    line_starts.partition_point(|&start| start <= pos)
}

fn classify(
    occ: &Occurrence,
    s: &[u8],
    macros: &[MacroSpan],
    blocks: &[MatchBlock],
    tuples: &[(usize, usize)],
) -> (&'static str, usize) {
    if let Some(span) = macros
        .iter()
        .find(|m| occ.start > m.open && occ.start < m.close)
    {
        return (FORM_MATCHES, span.kw);
    }
    if let Some(form) = eq_ne_adjacent(s, occ) {
        return (form, occ.start);
    }
    if let Some(&(open, _)) = tuples
        .iter()
        .find(|&&(open, close)| occ.start > open && occ.start < close)
    {
        return (FORM_TUPLE, open);
    }
    for block in blocks {
        if block
            .pattern_regions
            .iter()
            .any(|&(a, z)| occ.start >= a && occ.start < z)
        {
            return (FORM_MATCH, block.kw);
        }
    }
    if is_let_pattern(s, occ.start) {
        return (FORM_LET, occ.start);
    }
    if looks_like_arm(s, occ.end) {
        return (FORM_ARM, occ.start);
    }
    (FORM_DIRECT, occ.start)
}

/// Blanket-exemption candidates per `design/pipeline-dry.md`: the per-provider
/// profile impl files, the clap mapping in `main.rs`, and test paths.
fn exempt_candidate(rel_path: &str) -> bool {
    if rel_path == "claudine/cli/src/main.rs" {
        return true;
    }
    if let Some(rest) = rel_path.strip_prefix("claudine/cli/src/commands/wrap/profile/")
        && !rest.contains('/')
    {
        return true;
    }
    rel_path.contains("/tests/")
}

/// Scan one file's source, returning its grouped sites and the raw count of
/// classified `Provider::<Variant>` occurrences.
fn scan_source(rel_path: &str, src: &str, variants: &[String]) -> (Vec<Site>, usize) {
    let sanitized = sanitize(src);
    let s = sanitized.as_slice();
    let occs = occurrences(s, variants);
    if occs.is_empty() {
        return (Vec::new(), 0);
    }
    let macros = matches_macro_spans(s);
    let blocks = match_blocks(s);
    let tuples = tuple_array_spans(s, &occs);

    let mut line_starts = vec![0usize];
    for (i, &b) in s.iter().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }

    let mut groups: BTreeMap<(usize, &'static str), BTreeSet<String>> = BTreeMap::new();
    for occ in &occs {
        let (form, anchor) = classify(occ, s, &macros, &blocks, &tuples);
        groups
            .entry((line_of(&line_starts, anchor), form))
            .or_default()
            .insert(occ.variant.clone());
    }

    let exempt = exempt_candidate(rel_path);
    let sites = groups
        .into_iter()
        .map(|((line, form), providers)| Site {
            path: rel_path.to_string(),
            line,
            form,
            dispatch_class: dispatch_class(form),
            providers: providers.into_iter().collect(),
            exempt_candidate: exempt,
        })
        .collect();
    (sites, occs.len())
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn variant_names() -> Vec<String> {
    claudine::provider::PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(|p| format!("{p:?}"))
        .collect()
}

fn generate_inventory() -> Inventory {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_root = manifest_dir.join("src");
    let variants = variant_names();

    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(
        !files.is_empty(),
        "no .rs files found under {} — scan root is broken",
        src_root.display()
    );

    let mut sites = Vec::new();
    let mut provider_refs = 0usize;
    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
        let rel = file
            .strip_prefix(manifest_dir)
            .expect("scanned file lives under the manifest dir")
            .to_string_lossy()
            .replace('\\', "/");
        let rel_repo = format!("claudine/cli/{rel}");
        let (file_sites, refs) = scan_source(&rel_repo, &content, &variants);
        sites.extend(file_sites);
        provider_refs += refs;
    }
    sites.sort();

    let mut by_form: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_dispatch_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    for site in &sites {
        *by_form.entry(site.form).or_default() += 1;
        *by_dispatch_class.entry(site.dispatch_class).or_default() += 1;
    }

    assert!(
        provider_refs > 0,
        "zero Provider references found in cli/src — scanner or variant list is broken"
    );

    Inventory {
        tool: "claudine-cli/tests/dispatch_inventory.rs",
        pattern_set_version: PATTERN_SET_VERSION,
        scanned_root: "claudine/cli/src",
        regenerate: REGEN_COMMAND,
        forms: form_descriptions(),
        dispatch_classes: dispatch_class_descriptions(),
        totals: Totals {
            sites: sites.len(),
            provider_refs,
            by_form,
            by_dispatch_class,
        },
        sites,
    }
}

fn inventory_file_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli crate has a parent package area")
        .join("docs/providers/dispatch-inventory.json")
}

fn render(inventory: &Inventory) -> String {
    let mut json = serde_json::to_string_pretty(inventory).expect("inventory serializes");
    json.push('\n');
    json
}

/// Drift test: the committed inventory must byte-match a fresh regeneration.
/// Set `CLAUDINE_UPDATE_INVENTORY=1` to bless the committed file instead.
#[test]
fn dispatch_inventory_matches_committed_file() {
    let inventory = generate_inventory();
    let generated = render(&inventory);
    let path = inventory_file_path();

    if std::env::var_os(BLESS_ENV).is_some_and(|v| v == "1") {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
        }
        fs::write(&path, &generated)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
        eprintln!("blessed dispatch inventory: {}", path.display());
        return;
    }

    let committed = fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            panic!(
                "failed to read committed inventory {}: {e}\nbless it with: {REGEN_COMMAND}",
                path.display()
            )
        })
        .replace("\r\n", "\n");

    if committed != generated {
        let divergence = committed
            .lines()
            .zip(generated.lines())
            .position(|(a, b)| a != b)
            .map(|idx| {
                format!(
                    "first divergence at line {}:\n  committed: {}\n  generated: {}",
                    idx + 1,
                    committed.lines().nth(idx).unwrap_or(""),
                    generated.lines().nth(idx).unwrap_or(""),
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "documents share a prefix but differ in length \
                     (committed {} lines, generated {} lines)",
                    committed.lines().count(),
                    generated.lines().count(),
                )
            });
        panic!(
            "dispatch inventory drift: {} no longer matches cli/src.\n{divergence}\n\
             If the dispatch change is intentional, regenerate with:\n  {REGEN_COMMAND}",
            path.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Classifier unit tests (fixture sources, never scanned from disk)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod classifier_tests {
    use super::*;

    fn scan(src: &str) -> (Vec<Site>, usize) {
        scan_source("claudine/cli/src/fixture.rs", src, &variant_names())
    }

    #[test]
    fn match_block_groups_arm_variants_into_one_site() {
        let src = "fn f(p: Provider) -> u8 {\n    match p {\n        Provider::Claude => 1,\n        Provider::Codex | Provider::Gemini => 2,\n        _ => 0,\n    }\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 3);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].form, FORM_MATCH);
        assert_eq!(sites[0].line, 2);
        assert_eq!(sites[0].providers, ["Claude", "Codex", "Gemini"]);
    }

    #[test]
    fn variants_in_arm_bodies_classify_independently() {
        let src = "fn f(p: Provider) -> Provider {\n    match p {\n        Provider::Claude => Provider::Codex,\n        other => other,\n    }\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 2);
        let forms: Vec<_> = sites.iter().map(|s| s.form).collect();
        assert!(forms.contains(&FORM_MATCH));
        assert!(forms.contains(&FORM_DIRECT));
    }

    #[test]
    fn multiline_matches_macro_is_one_site_at_the_macro_line() {
        let src = "fn f(p: Provider) -> bool {\n    matches!(\n        p,\n        Provider::Codex | Provider::Gemini\n    )\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 2);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].form, FORM_MATCHES);
        assert_eq!(sites[0].line, 2);
        assert_eq!(sites[0].providers, ["Codex", "Gemini"]);
    }

    #[test]
    fn eq_and_ne_comparisons_both_operand_orders() {
        let src = "fn f(p: Provider) -> bool {\n    p == Provider::Claude\n        && Provider::Codex != p\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 2);
        let forms: Vec<_> = sites.iter().map(|s| (s.form, s.line)).collect();
        assert!(forms.contains(&(FORM_EQ, 2)));
        assert!(forms.contains(&(FORM_NE, 3)));
    }

    #[test]
    fn tuple_array_groups_all_elements() {
        let src = "const T: [(Provider, u8); 2] = [\n    (Provider::Claude, 1),\n    (Provider::Codex, 2),\n];\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 2);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].form, FORM_TUPLE);
        assert_eq!(sites[0].providers, ["Claude", "Codex"]);
    }

    #[test]
    fn let_pattern_and_direct_ref() {
        let src = "fn f(p: Provider) {\n    if let Provider::Goose = p {}\n    let _fixture = [Provider::Claude, Provider::Codex];\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 3);
        let forms: Vec<_> = sites.iter().map(|s| s.form).collect();
        assert!(forms.contains(&FORM_LET));
        assert!(forms.contains(&FORM_DIRECT));
    }

    #[test]
    fn comments_strings_and_lookalikes_are_ignored() {
        let src = "// Provider::Claude in a comment\nfn f() {\n    let _s = \"Provider::Codex\";\n    let _t = TtsProvider::Say;\n    let _u = Provider::parse_cli_name;\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 0);
        assert!(sites.is_empty());
    }

    #[test]
    fn nested_match_assigns_arms_to_inner_match() {
        let src = "fn f(p: Provider, q: Provider) -> u8 {\n    match p {\n        Provider::Claude => match q {\n            Provider::Codex => 1,\n            _ => 2,\n        },\n        _ => 0,\n    }\n}\n";
        let (sites, refs) = scan(src);
        assert_eq!(refs, 2);
        assert_eq!(sites.len(), 2);
        assert!(sites.iter().all(|s| s.form == FORM_MATCH));
        let lines: Vec<_> = sites.iter().map(|s| s.line).collect();
        assert_eq!(lines, [2, 3]);
    }

    #[test]
    fn exempt_candidate_paths() {
        assert!(exempt_candidate("claudine/cli/src/main.rs"));
        assert!(exempt_candidate(
            "claudine/cli/src/commands/wrap/profile/codex.rs"
        ));
        assert!(exempt_candidate(
            "claudine/cli/src/commands/wrap/profile/tests/apply.rs"
        ));
        assert!(!exempt_candidate("claudine/cli/src/commands/wrap/mod.rs"));
    }
}
