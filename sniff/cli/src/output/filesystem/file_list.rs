//! Rendering core for the filtered verbose file-association list.
//!
//! `sniff files --association <cat> -v` lists every captured matching path
//! after the summary table. This module owns the pure pieces of that list:
//!
//! - [`reversible_label`] textualizes a native path so no control byte or
//!   non-Unicode unit can inject report lines or terminal sequences, while
//!   distinct native spellings stay distinguishable.
//! - [`escape_for_prose`] and [`escape_href`] apply the Prose markup and
//!   attribute escaping on top of the label, in that order.
//! - [`link_target`] turns a native path into a `file://` URL via
//!   `url::Url::from_file_path` (the same primitive `biscuit-terminal`
//!   delegates to), returning `None` on any representability failure.
//! - [`render_file_list`] sorts the captured paths and renders them as a
//!   `Prose`-item unordered list with OSC8 hyperlinks.
//!
//! Escape layering is fixed: the reversible label is built first (no ESC byte
//! survives it, so `Prose::escape_text`'s ANSI pass-through can never fire),
//! markup escaping is applied second, and the hyperlink `href` is built from
//! the native target and attribute-escaped separately.
//!
//! ## Label notation
//!
//! | Native value | Label |
//! | ------------ | ----- |
//! | readable Unicode (above U+009F) | as-is |
//! | newline / CR / tab | `\n` / `\r` / `\t` |
//! | escape (both platforms) | `\x1B` |
//! | Unix C0/DEL byte, invalid Unix byte | `\xNN` (two uppercase hex digits) |
//! | C1 control code point (valid UTF-8, U+0080–U+009F) | `\u{XXXX}` |
//! | Windows C0/DEL/C1 unit, lone surrogate | `\u{XXXX}` (four uppercase hex digits) |
//! | literal backslash (incl. Windows separator) | `\\` |
//! | Unix `/` separator | `/` (readable) |
//!
//! One precision relative to the plan's ruling 5: a C1 *code point* on Unix
//! (valid UTF-8, e.g. U+0085 as `C2 85`) renders `\u{0085}`, not `\x85`.
//! `\xNN` is reserved for native bytes, because a C1 scalar spelled `\xNN`
//! would collide with the invalid byte of the same value and break the
//! spec's distinguishability requirement; `\u{XXXX}` is the ruling's own
//! collision-free notation. Both notations round-trip exactly (see the
//! decoder-backed tests below).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

/// Textualizes a native path into its reversible label.
///
/// The label keeps readable Unicode and directory segments, spells controls
/// visibly, and doubles every literal backslash (so a name that already looks
/// like an escape, e.g. a literal `\n` two-character spelling, stays distinct
/// from a real newline). See the [module documentation](self) for the exact
/// notation.
pub(super) fn reversible_label(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        label_from_bytes(path.as_os_str().as_bytes())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        label_from_units(&path.as_os_str().encode_wide().collect::<Vec<_>>())
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().into_owned()
    }
}

/// Builds the reversible label for a Unix-native path from its raw bytes.
///
/// Platform-neutral on purpose (pure function of bytes) so every OS can unit
/// test the Unix notation without constructing non-UTF-8 on-disk names.
#[cfg(any(unix, test))]
fn label_from_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            // Includes ESC, which spells `\x1B` through the same format.
            0x00..=0x1f | 0x7f => {
                let _ = write!(out, "\\x{byte:02X}");
            }
            // Printable ASCII, including the `/` separator, stays readable.
            0x20..=0x7e => out.push(byte as char),
            _ => match utf8_scalar_at(bytes, i) {
                Some((ch, len)) => {
                    if ('\u{80}'..='\u{9f}').contains(&ch) {
                        let _ = write!(out, "\\u{{{:04X}}}", ch as u32);
                    } else {
                        out.push(ch);
                    }
                    i += len;
                    continue;
                }
                None => {
                    let _ = write!(out, "\\x{byte:02X}");
                }
            },
        }
        i += 1;
    }
    out
}

/// Decodes the UTF-8 scalar starting at `start`, returning it with its byte
/// length, or `None` when the bytes there are not a well-formed sequence
/// (orphan continuation, overlong lead, out-of-range lead, truncated tail).
#[cfg(any(unix, test))]
fn utf8_scalar_at(bytes: &[u8], start: usize) -> Option<(char, usize)> {
    let len = match bytes[start] {
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => return None,
    };
    let end = start.checked_add(len)?;
    let slice = bytes.get(start..end)?;
    let ch = std::str::from_utf8(slice).ok()?.chars().next()?;
    Some((ch, len))
}

/// Builds the reversible label for a Windows-native path from its UTF-16
/// code units.
///
/// Platform-neutral on purpose (pure function of units) so every OS can unit
/// test the Windows notation without illegal on-disk names. Paired surrogates
/// decode to their readable astral scalar; unpaired ones keep the distinct
/// `\u{XXXX}` notation.
#[cfg(any(windows, test))]
fn label_from_units(units: &[u16]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        match unit {
            0x5c => out.push_str("\\\\"),
            0x0a => out.push_str("\\n"),
            0x0d => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x1b => out.push_str("\\x1B"),
            0x00..=0x1f | 0x7f..=0x9f => {
                let _ = write!(out, "\\u{{{:04X}}}", unit);
            }
            0xd800..=0xdbff => {
                let paired = units
                    .get(i + 1)
                    .is_some_and(|next| (0xdc00..=0xdfff).contains(next));
                if paired {
                    let high = u32::from(unit - 0xd800) << 10;
                    let low = u32::from(units[i + 1] - 0xdc00);
                    // Astral scalars sit above U+009F and stay readable.
                    let ch = char::from_u32(0x10000 + high + low).expect("valid surrogate pair");
                    out.push(ch);
                    i += 2;
                    continue;
                }
                let _ = write!(out, "\\u{{{:04X}}}", unit);
            }
            0xdc00..=0xdfff => {
                let _ = write!(out, "\\u{{{:04X}}}", unit);
            }
            _ => out.push(char::from_u32(u32::from(unit)).expect("non-surrogate BMP scalar")),
        }
        i += 1;
    }
    out
}

/// Escapes a reversible label for the Prose/InlineProse grammar (layering
/// step 2): `< > { * _ [ ] ( ) \` and backslash are backslash-escaped.
///
/// The label has no ESC byte left at this point, so the underlying
/// `Prose::escape_text` ANSI pass-through cannot fire. `&` needs no escape —
/// the grammar has no entities.
pub(super) fn escape_for_prose(label: &str) -> String {
    Prose::escape_text(label)
}

/// Quotes a hyperlink URL for a Prose `href` attribute (layering step 3),
/// so a quote or `>` in the value cannot terminate the attribute.
pub(super) fn escape_href(url: &str) -> String {
    Prose::quoted_attr(url)
}

/// Builds the OSC8 hyperlink destination for one captured path.
///
/// Relative paths join onto `root` (the owning package root or effective
/// base the paths are relative to); absolute paths stay absolute and are
/// never re-rooted. The join is lexical — no `exists()` probe, no
/// canonicalize — and any representability failure returns `None` so the
/// caller can silently fall back to the escaped label without a link.
pub(super) fn link_target(root: &Path, path: &Path) -> Option<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    url::Url::from_file_path(absolute).ok().map(String::from)
}

/// Renders the `Files:` section for a filtered verbose association report.
///
/// Paths sort by native `PathBuf` order before formatting. Each entry is one
/// `Prose` list item — a linked, dim-directory/bold-name label when
/// [`link_target`] succeeds, or the plain styled label when it fails
/// (silently: no diagnostic, no link). An empty slice renders nothing at all,
/// heading included.
pub(super) fn render_file_list(files: &[PathBuf], root: &Path, term: &Terminal) -> String {
    if files.is_empty() {
        return String::new();
    }
    let mut sorted: Vec<&PathBuf> = files.iter().collect();
    sorted.sort();

    let mut list = UnorderedList::empty();
    for path in sorted {
        list.add(Prose::new(file_list_item_markup(path, root)));
    }

    let mut out = String::new();
    // The section already ends with one blank line after the table (or the
    // incomplete-scan notice), which separates the heading from it.
    writeln!(out, "Files:").unwrap();
    writeln!(out, "{}", list.render(term)).unwrap();
    out
}

/// Builds the Prose markup for one file-list entry, applying the fixed
/// escape layering: reversible label, then markup escaping, with the href
/// built from the native target and attribute-escaped separately.
fn file_list_item_markup(path: &Path, root: &Path) -> String {
    let label = reversible_label(path);
    let body = match split_label_dir_name(&label) {
        Some((dir, name)) => format!(
            "<blue><dim>{}</dim><b>{}</b></blue>",
            escape_for_prose(dir),
            escape_for_prose(name)
        ),
        None => format!("<blue><b>{}</b></blue>", escape_for_prose(&label)),
    };
    match link_target(root, path) {
        Some(url) => format!("<a href={}>{body}</a>", escape_href(&url)),
        None => body,
    }
}

/// Splits a reversible label into (directory incl. trailing separator, name)
/// for the dim/bold styling. Splitting the *label* (not the native path)
/// keeps the styled parts concatenating back to exactly the full label, even
/// for root-only parents like `/x.png`.
#[cfg(unix)]
fn split_label_dir_name(label: &str) -> Option<(&str, &str)> {
    let idx = label.rfind('/')?;
    let name = &label[idx + 1..];
    if name.is_empty() {
        return None;
    }
    Some((&label[..idx + 1], name))
}

/// Windows variant: the native separator is `\`, which the label doubles, so
/// the boundary is the last `\\` sequence — with `/` also recognized for
/// stored paths that carried forward slashes.
#[cfg(windows)]
fn split_label_dir_name(label: &str) -> Option<(&str, &str)> {
    const SEP: &str = "\\\\";
    let back = label.rfind(SEP).map(|i| i + SEP.len());
    let fwd = label.rfind('/').map(|i| i + 1);
    let split = back.max(fwd)?;
    let name = &label[split..];
    if name.is_empty() {
        return None;
    }
    Some((&label[..split], name))
}

#[cfg(not(any(unix, windows)))]
fn split_label_dir_name(label: &str) -> Option<(&str, &str)> {
    label.rfind('/').map(|idx| (&label[..idx + 1], &label[idx + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::components::renderable::TerminalRenderable;

    /// Decodes a Unix label back to raw bytes; the round-trip twin of
    /// `label_from_bytes` used by the reversibility tests.
    fn decode_bytes_label(label: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chars = label.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                continue;
            }
            match chars.next().expect("label backslash always escaped") {
                '\\' => out.push(b'\\'),
                'n' => out.push(b'\n'),
                'r' => out.push(b'\r'),
                't' => out.push(b'\t'),
                'x' => {
                    let mut hex = String::new();
                    hex.push(chars.next().unwrap());
                    hex.push(chars.next().unwrap());
                    out.push(u8::from_str_radix(&hex, 16).unwrap());
                }
                'u' => {
                    assert_eq!(chars.next().unwrap(), '{');
                    let mut hex = String::new();
                    loop {
                        let c = chars.next().unwrap();
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                    let cp = u32::from_str_radix(&hex, 16).unwrap();
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(char::from_u32(cp).unwrap().encode_utf8(&mut buf).as_bytes());
                }
                other => panic!("unexpected escape {other:?}"),
            }
        }
        out
    }

    /// Decodes a Windows label back to UTF-16 code units; the round-trip
    /// twin of `label_from_units`.
    fn decode_units_label(label: &str) -> Vec<u16> {
        let mut out = Vec::new();
        let mut chars = label.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                let mut buf = [0u16; 2];
                out.extend_from_slice(c.encode_utf16(&mut buf));
                continue;
            }
            match chars.next().expect("label backslash always escaped") {
                '\\' => out.push(0x5c),
                'n' => out.push(0x0a),
                'r' => out.push(0x0d),
                't' => out.push(0x09),
                'x' => {
                    let mut hex = String::new();
                    hex.push(chars.next().unwrap());
                    hex.push(chars.next().unwrap());
                    out.push(u16::from_str_radix(&hex, 16).unwrap());
                }
                'u' => {
                    assert_eq!(chars.next().unwrap(), '{');
                    let mut hex = String::new();
                    loop {
                        let c = chars.next().unwrap();
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                    let cp = u32::from_str_radix(&hex, 16).unwrap();
                    if let Some(ch) = char::from_u32(cp) {
                        let mut buf = [0u16; 2];
                        out.extend_from_slice(ch.encode_utf16(&mut buf));
                    } else {
                        // Lone surrogate: a single unpaired unit.
                        out.push(u16::try_from(cp).unwrap());
                    }
                }
                other => panic!("unexpected escape {other:?}"),
            }
        }
        out
    }

    // -----------------------------------------------------------------
    // Task 2.1 — reversible labels
    // -----------------------------------------------------------------

    #[test]
    fn bytes_label_keeps_readable_text_and_unix_separators() {
        assert_eq!(label_from_bytes(b"assets/logo.svg"), "assets/logo.svg");
        assert_eq!(label_from_bytes(b"a/b/c.png"), "a/b/c.png");
        let unicode = "héllo→😀.png".as_bytes();
        assert_eq!(label_from_bytes(unicode), "héllo→😀.png");
    }

    #[test]
    fn bytes_label_spells_controls_visibly() {
        assert_eq!(label_from_bytes(b"a\nb"), "a\\nb");
        assert_eq!(label_from_bytes(b"a\rb"), "a\\rb");
        assert_eq!(label_from_bytes(b"a\tb"), "a\\tb");
        assert_eq!(label_from_bytes(b"a\x1bb"), "a\\x1Bb");
        assert_eq!(label_from_bytes(b"a\x00b"), "a\\x00b");
        assert_eq!(label_from_bytes(b"a\x0bb"), "a\\x0Bb");
        assert_eq!(label_from_bytes(b"a\x7fb"), "a\\x7Fb");
    }

    #[test]
    fn bytes_label_distinguishes_literal_escape_lookalikes() {
        // A real newline byte vs a filename containing the two characters `\n`.
        assert_eq!(label_from_bytes(b"re\nport"), "re\\nport");
        assert_eq!(label_from_bytes(b"re\\nport"), "re\\\\nport");
        // A literal backslash doubles, so `\\n` cannot be read as a newline.
        assert_ne!(label_from_bytes(b"re\nport"), label_from_bytes(b"re\\nport"));
        // And a literal `\x1B` spelling stays distinct from a real ESC byte.
        assert_ne!(label_from_bytes(b"e\x1b"), label_from_bytes(b"e\\x1B"));
        assert_eq!(label_from_bytes(b"e\\x1B"), "e\\\\x1B");
    }

    #[test]
    fn bytes_label_preserves_invalid_bytes_hex_escaped() {
        assert_eq!(label_from_bytes(&[0x61, 0xff, 0x62]), "a\\xFFb");
        assert_eq!(label_from_bytes(&[0x80]), "\\x80");
        // Overlong encodings stay byte-faithful rather than decoding.
        assert_eq!(label_from_bytes(&[0xc0, 0xaf]), "\\xC0\\xAF");
        // Truncated multi-byte tail escapes byte-wise.
        assert_eq!(label_from_bytes(&[0xe2, 0x82]), "\\xE2\\x82");
        // Distinct invalid bytes stay distinguishable.
        assert_ne!(label_from_bytes(&[0xfe]), label_from_bytes(&[0xff]));
    }

    #[test]
    fn bytes_label_escapes_c1_code_points_without_byte_collision() {
        // U+0085 (NEL) as valid UTF-8 must not collide with the invalid byte
        // 0x85: the scalar uses the `\u{}` notation, the byte uses `\xNN`.
        assert_eq!(label_from_bytes(&[0x61, 0xc2, 0x85, 0x62]), "a\\u{0085}b");
        assert_eq!(label_from_bytes(&[0x61, 0x85, 0x62]), "a\\x85b");
        assert_ne!(
            label_from_bytes(&[0x61, 0xc2, 0x85, 0x62]),
            label_from_bytes(&[0x61, 0x85, 0x62])
        );
    }

    #[test]
    fn bytes_label_contains_no_control_bytes_or_escapes() {
        let hostile: Vec<u8> = (0u8..=255).collect();
        let label = label_from_bytes(&hostile);
        assert!(
            label.chars().all(|c| !c.is_control()),
            "control char leaked: {label:?}"
        );
        assert!(!label.contains('\x1b'));
    }

    #[test]
    fn bytes_label_round_trips_every_single_byte() {
        for byte in 0u8..=255 {
            let input = [b'a', byte, b'z'];
            let label = label_from_bytes(&input);
            assert_eq!(decode_bytes_label(&label), input, "byte {byte:#04x}");
        }
    }

    #[test]
    fn bytes_label_round_trips_mixed_sequences() {
        let cases: Vec<Vec<u8>> = vec![
            b"plain/path.png".to_vec(),
            vec![0xf0, 0x9f, 0x98, 0x80, b'.', b'p', b'n', b'g'],
            vec![b'\\', b'n', 0xff, 0xc2, 0x85, 0x0a, b'\\'],
            (0u8..=255).collect(),
        ];
        for case in cases {
            let label = label_from_bytes(&case);
            assert_eq!(decode_bytes_label(&label), case, "case {case:?}");
        }
    }

    #[test]
    fn units_label_doubles_windows_separators() {
        // Native `assets\logo.svg` renders with a doubled separator.
        let units: Vec<u16> = "assets\\logo.svg".encode_utf16().collect();
        assert_eq!(label_from_units(&units), "assets\\\\logo.svg");
        let units: Vec<u16> = "a\\b\\c.png".encode_utf16().collect();
        assert_eq!(label_from_units(&units), "a\\\\b\\\\c.png");
        // Forward slashes (legal in stored paths) stay readable.
        let units: Vec<u16> = "a/b.png".encode_utf16().collect();
        assert_eq!(label_from_units(&units), "a/b.png");
    }

    #[test]
    fn units_label_spells_controls_visibly() {
        let mk = |s: &str| s.encode_utf16().collect::<Vec<_>>();
        assert_eq!(label_from_units(&mk("a\nb")), "a\\nb");
        assert_eq!(label_from_units(&mk("a\rb")), "a\\rb");
        assert_eq!(label_from_units(&mk("a\tb")), "a\\tb");
        // ESC keeps the spec's `\x1B` spelling even on the units path.
        let esc = vec![u16::from(b'a'), 0x1b, u16::from(b'b')];
        assert_eq!(label_from_units(&esc), "a\\x1Bb");
        // Other C0/DEL/C1 controls use the `\u{}` notation.
        let bel = vec![u16::from(b'a'), 0x07, u16::from(b'b')];
        assert_eq!(label_from_units(&bel), "a\\u{0007}b");
        let del = vec![u16::from(b'a'), 0x7f, u16::from(b'b')];
        assert_eq!(label_from_units(&del), "a\\u{007F}b");
        let nel = vec![u16::from(b'a'), 0x85, u16::from(b'b')];
        assert_eq!(label_from_units(&nel), "a\\u{0085}b");
    }

    #[test]
    fn units_label_keeps_lone_surrogates_distinct_and_readable_astral() {
        let lone_high = vec![u16::from(b'a'), 0xd800, u16::from(b'b')];
        assert_eq!(label_from_units(&lone_high), "a\\u{D800}b");
        let lone_low = vec![u16::from(b'a'), 0xdc00, u16::from(b'b')];
        assert_eq!(label_from_units(&lone_low), "a\\u{DC00}b");
        // A paired surrogate pair (U+1F600) decodes to the readable scalar.
        let paired = vec![u16::from(b'a'), 0xd83d, 0xde00, u16::from(b'b')];
        assert_eq!(label_from_units(&paired), "a😀b");
        // Distinct lone surrogates stay distinguishable, and a literal
        // `\u{D800}` text spelling doubles its backslash.
        let mk = |s: &str| s.encode_utf16().collect::<Vec<_>>();
        assert_eq!(label_from_units(&mk(r"a\u{D800}b")), "a\\\\u{D800}b");
        assert_ne!(
            label_from_units(&mk(r"a\u{D800}b")),
            label_from_units(&lone_high)
        );
    }

    #[test]
    fn units_label_contains_no_control_chars() {
        let mut hostile: Vec<u16> = (0u16..=0x2ff).collect();
        hostile.extend([0xd800, 0xdc00, 0xdbff, 0xdfff, 0xfffd, 0xffff]);
        let label = label_from_units(&hostile);
        assert!(
            label.chars().all(|c| !c.is_control()),
            "control char leaked: {label:?}"
        );
        assert!(!label.contains('\x1b'));
    }

    #[test]
    fn units_label_round_trips_arbitrary_units() {
        let mut cases: Vec<Vec<u16>> = vec![
            "assets\\logo.svg".encode_utf16().collect(),
            vec![0x07, 0x1b, 0x7f, 0x85, 0xa0],
            vec![0xd83d, 0xde00], // paired
        ];
        // Every lone surrogate, each distinguishable after round-trip.
        for surrogate in [0xd800, 0xdbff, 0xdc00, 0xdfff] {
            cases.push(vec![u16::from(b'x'), surrogate, u16::from(b'y')]);
        }
        // A broad sweep including controls and printables.
        cases.push((0u16..=0x300).collect());
        for case in cases {
            let label = label_from_units(&case);
            assert_eq!(decode_units_label(&label), case, "case {case:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn reversible_label_reads_native_bytes_on_unix() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(OsString::from_vec(vec![b'a', 0xff, 0x62]));
        assert_eq!(reversible_label(&path), "a\\xFFb");
    }

    #[cfg(windows)]
    #[test]
    fn reversible_label_reads_native_units_on_windows() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        let path = PathBuf::from(OsString::from_wide(&[u16::from(b'a'), 0xd800, u16::from(b'b')]));
        assert_eq!(reversible_label(&path), "a\\u{D800}b");
    }

    // -----------------------------------------------------------------
    // Task 2.2 — markup and attribute escaping
    // -----------------------------------------------------------------

    #[test]
    fn escape_for_prose_renders_markup_sensitive_labels_literally() {
        use biscuit_terminal::components::prose::InlineProse;
        let term = Terminal::new_optimistic(200);
        for label in [
            "a<b>b&c</b>",
            "&amp;",
            "[x](y)",
            "*bold*",
            "_it_",
            r"esc\ape\\x1B",
            "\"quoted\"",
            "{brace}",
            "`tick`",
            "re\\nreal_vs_\\x1Besc",
        ] {
            let rendered = InlineProse::new(escape_for_prose(label)).render(&term);
            assert_eq!(rendered, label, "label {label:?}");
        }
    }

    #[test]
    fn escape_for_prose_survives_styling_wrappers() {
        use biscuit_terminal::components::prose::InlineProse;
        use biscuit_terminal::prelude::strip_escape_codes;
        let term = Terminal::new_optimistic(200);
        for label in ["a<b>c&d[e](f)*g*_h_", r"lit\\n\name"] {
            let markup = format!("<blue><dim>dir/</dim><b>{}</b></blue>", escape_for_prose(label));
            let rendered = InlineProse::new(markup).render(&term);
            assert_eq!(strip_escape_codes(&rendered), format!("dir/{label}"), "label {label:?}");
        }
    }

    #[test]
    fn escape_href_cannot_be_terminated_by_quotes_or_angle_brackets() {
        use biscuit_terminal::components::prose::InlineProse;
        let term = Terminal::new_optimistic(200);
        for url in [
            "file:///a\"b",
            "file:///a>b",
            "file:///a'<b",
            "file:///a\"b'c>d",
            "file:///plain/path",
        ] {
            let markup = format!("<a href={}>lbl</a>", escape_href(url));
            let rendered = InlineProse::new(markup).render(&term);
            let destinations = osc8_destinations(&rendered);
            assert_eq!(destinations, vec![url.to_string()], "url {url:?}");
        }
    }

    /// Extracts the OSC8 destinations from a rendered string.
    fn osc8_destinations(rendered: &str) -> Vec<String> {
        rendered
            .split("\x1b]8;;")
            .skip(1)
            .map(|rest| rest.split("\x1b\\").next().unwrap_or("").to_string())
            .filter(|dest| !dest.is_empty())
            .collect()
    }

    // -----------------------------------------------------------------
    // Task 2.3 — link targets
    // -----------------------------------------------------------------

    #[test]
    fn link_target_joins_relative_and_keeps_absolute_platform_neutral() {
        // Platform-neutral: build the root from the live cwd so the path is
        // absolute on every OS without hard-coding a Unix spelling.
        let root = std::env::current_dir().unwrap();
        let relative = Path::new("lib/assets/logo.svg");
        let url = link_target(&root, relative).expect("representable");
        let parsed = url::Url::parse(&url).unwrap();
        assert_eq!(parsed.scheme(), "file");
        assert_eq!(
            parsed,
            url::Url::from_file_path(root.join(relative)).unwrap(),
            "relative joins onto the root"
        );

        // An absolute stored path stays absolute and is not re-rooted.
        let absolute = std::env::temp_dir().join("elsewhere/x.png");
        let url = link_target(&root, &absolute).expect("representable");
        assert_eq!(
            url::Url::parse(&url).unwrap(),
            url::Url::from_file_path(&absolute).unwrap(),
            "absolute stays absolute"
        );
    }

    #[test]
    fn link_target_never_probes_the_filesystem() {
        // The path does not exist; the link is still built (removed files
        // remain listed and linked after discovery).
        let root = std::env::temp_dir().join("definitely/not/here");
        let url = link_target(&root, Path::new("ghost.png")).expect("representable");
        assert!(url.starts_with("file://"), "{url}");
    }

    #[test]
    fn link_target_returns_none_when_result_is_not_absolute() {
        // A relative root joined with a relative path cannot form a file URL;
        // the caller falls back to the label without a link.
        assert_eq!(link_target(Path::new("rel"), Path::new("a.png")), None);
    }

    #[cfg(unix)]
    #[test]
    fn link_target_percent_encodes_non_utf8_unix_bytes() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(OsString::from_vec(vec![b'a', 0xff, 0x62]));
        let url = link_target(Path::new("/work"), &path).expect("faithful byte encoding");
        assert_eq!(url, "file:///work/a%FFb");
    }

    #[cfg(unix)]
    #[test]
    fn link_target_encodes_spaces_and_unicode_on_unix() {
        let url = link_target(Path::new("/work"), Path::new("as sets/ünïcode.png"))
            .expect("representable");
        assert_eq!(url, "file:///work/as%20sets/%C3%BCn%C3%AFcode.png");
    }

    #[cfg(windows)]
    #[test]
    fn link_target_encodes_drive_verbatim_and_unc_paths() {
        use std::path::PathBuf;
        // Plain drive.
        let url = link_target(&PathBuf::from(r"C:\work"), Path::new(r"lib\a.png"));
        assert_eq!(url.as_deref(), Some("file:///C:/work/lib/a.png"));
        // Verbatim drive: the \\?\ prefix is consumed by the URL encoder.
        let url = link_target(&PathBuf::from(r"\\?\C:\work"), Path::new(r"a.png"));
        assert_eq!(url.as_deref(), Some("file:///C:/work/a.png"));
        // UNC share.
        let url = link_target(&PathBuf::from(r"\\server\share"), Path::new(r"a b.png"));
        assert_eq!(url.as_deref(), Some("file://server/share/a%20b.png"));
        // A non-Unicode component (lone surrogate) errs -> per-entry fallback.
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        let bad = PathBuf::from(OsString::from_wide(&[
            u16::from(b'a'), 0xd800, u16::from(b'.'), u16::from(b'p'),
            u16::from(b'n'), u16::from(b'g'),
        ]));
        assert_eq!(link_target(&PathBuf::from(r"C:\work"), &bad), None);
    }

    // -----------------------------------------------------------------
    // Task 2.4 — entry and list renderer
    // -----------------------------------------------------------------

    /// Strips renderer styling so list assertions see label text only.
    fn visible(rendered: &str) -> String {
        biscuit_terminal::prelude::strip_escape_codes(rendered)
    }

    fn paths_from_strs(specs: &[&str]) -> Vec<PathBuf> {
        specs.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn file_list_sorts_paths_in_native_order() {
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&[
            "screenshots/example.png",
            "assets/logo.svg",
            "assets/banner.png",
            "Zebra.png",
        ]);
        let out = render_file_list(&files, Path::new("/work"), &term);
        let text = visible(&out);
        // `Zebra.png` sorts first: PathBuf order is byte-wise per component,
        // and uppercase `Z` precedes lowercase `a`.
        let zebra = text.find("- Zebra.png").expect("zebra listed");
        let banner = text.find("- assets/banner.png").expect("banner listed");
        let logo = text.find("- assets/logo.svg").expect("logo listed");
        let shot = text.find("- screenshots/example.png").expect("screenshot listed");
        assert!(zebra < banner && banner < logo && logo < shot, "native PathBuf order:\n{text}");
    }

    #[test]
    fn file_list_marks_heading_and_bullets() {
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&["assets/banner.png"]);
        let out = render_file_list(&files, Path::new("/work"), &term);
        assert!(out.contains("Files:"), "{out}");
        let text = visible(&out);
        assert!(text.contains("- assets/banner.png"), "{text}");
    }

    #[test]
    fn file_list_links_each_entry_with_osc8() {
        // The root must be absolute on every OS (a Unix-spelled `/work` is
        // relative on Windows), so anchor on the live working directory.
        let root = std::env::current_dir().unwrap();
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&["assets/banner.png", "logo.svg"]);
        let out = render_file_list(&files, &root, &term);
        let mut sorted: Vec<&PathBuf> = files.iter().collect();
        sorted.sort();
        let expected: Vec<String> = sorted
            .iter()
            .map(|path| link_target(&root, path).expect("representable"))
            .collect();
        assert_eq!(osc8_destinations(&out), expected);
    }

    #[test]
    fn file_list_empty_renders_nothing() {
        let term = Terminal::new_optimistic(200);
        assert_eq!(render_file_list(&[], Path::new("/work"), &term), "");
    }

    #[test]
    fn file_list_lists_each_captured_path_once() {
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&[
            "a.png",
            "b/c.png",
            "b/d.png",
            "e.png",
        ]);
        let text = visible(&render_file_list(&files, Path::new("/work"), &term));
        for path in &files {
            let label = reversible_label(path);
            let occurrences = text.match_indices(&label).count();
            assert_eq!(occurrences, 1, "{label} in:\n{text}");
        }
    }

    #[test]
    fn file_list_falls_back_to_label_silently_when_target_fails() {
        // A relative root cannot anchor a file URL, so every entry's target
        // fails: labels still render, with no OSC8, no panic, no diagnostic.
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&["assets/banner.png", "logo.svg"]);
        let out = render_file_list(&files, Path::new("not-absolute"), &term);
        assert!(!out.contains("\x1b]8;"), "no hyperlinks:\n{out:?}");
        let text = visible(&out);
        assert!(text.contains("- assets/banner.png"), "{text}");
        assert!(text.contains("- logo.svg"), "{text}");
    }

    #[cfg(windows)]
    #[test]
    fn file_list_falls_back_per_entry_for_non_unicode_windows_values() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        let term = Terminal::new_optimistic(200);
        let good = PathBuf::from("ok.png");
        let bad = PathBuf::from(OsString::from_wide(&[
            u16::from(b'a'), 0xd800, u16::from(b'.'), u16::from(b'p'),
            u16::from(b'n'), u16::from(b'g'),
        ]));
        let out = render_file_list(&[good, bad], Path::new(r"C:\work"), &term);
        let destinations = osc8_destinations(&out);
        assert_eq!(destinations, vec!["file:///C:/work/ok.png".to_string()]);
        let text = visible(&out);
        assert!(text.contains("- ok.png"), "{text}");
        assert!(text.contains(r"- a\u{D800}.png"), "{text}");
    }

    #[test]
    fn file_list_rich_and_plain_labels_agree_after_stripping() {
        // Rich render (OSC8 + SGR) stripped of styling shows exactly the
        // labels; plain mode is the same strip applied by `emit_text`.
        let term = Terminal::new_optimistic(200);
        let files = paths_from_strs(&[
            "assets/banner.png",
            "we\\ird name.txt",
            "sp ace.png",
            "üñí.png",
        ]);
        let out = render_file_list(&files, Path::new("/work"), &term);
        let text = visible(&out);
        let mut expected = String::from("Files:\n");
        // Native PathBuf order over these four: PathBuf comparison is by
        // components/bytes; compute it the same way the renderer does.
        let mut sorted: Vec<&PathBuf> = files.iter().collect();
        sorted.sort();
        for path in sorted {
            expected.push_str(&format!("- {}\n", reversible_label(path)));
        }
        assert_eq!(text, expected);
    }

    #[test]
    fn file_list_label_cannot_inject_lines_or_terminal_sequences() {
        #[cfg(unix)]
        let files = {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;
            // Filename bytes containing newline and ESC.
            vec![PathBuf::from(OsString::from_vec(vec![
                b'a', 0x0a, 0x1b, b']', b'8', b';', b';', b'e', b'v', b'i', b'l', b'b',
            ]))]
        };
        #[cfg(windows)]
        let files = {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;
            vec![PathBuf::from(OsString::from_wide(&[
                u16::from(b'a'), 0x0a, 0x1b, u16::from(b']'), u16::from(b'8'),
                u16::from(b';'), u16::from(b';'), u16::from(b'e'), u16::from(b'v'),
                u16::from(b'i'), u16::from(b'l'),
            ]))]
        };
        // An absolute root on every OS, so the entry is linked rather than
        // falling back to the no-link label.
        let root = std::env::current_dir().unwrap();
        let term = Terminal::new_optimistic(200);
        let out = render_file_list(&files, &root, &term);
        // One item means exactly one bullet line: the newline inside the
        // filename cannot add report lines. The only OSC8 sequences are the
        // renderer's own open/close pair for that one link — the filename's
        // ESC cannot open a second one.
        assert_eq!(out.matches("\x1b]8;").count(), 2, "raw render:\n{out:?}");
        let raw_bullet_lines = out.lines().filter(|l| l.contains("- ")).count();
        assert_eq!(raw_bullet_lines, 1, "raw render:\n{out:?}");
        // After stripping, the controls are visibly spelled and no raw
        // escape byte remains in the label.
        let text = visible(&out);
        assert!(
            text.contains(r"a\n\x1B]8;;evil"),
            "controls spelled visibly: {text:?}"
        );
        assert!(!text.contains('\x1b'), "{text:?}");
    }
}
