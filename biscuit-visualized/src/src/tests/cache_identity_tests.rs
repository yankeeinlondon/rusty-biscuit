//! D41, D63, D70, AC-C6a: the font part of the Mermaid cache key
//! ([`FontChain::cache_identity`]).
//!
//! Each test changes one input and checks whether the identity changes. The
//! installed faces are files in a temp directory (see [`FontDir`]) whenever a
//! test needs file metadata or must prove a file's contents are not read; the
//! fonts are patched copies of the bundled Inter and Noto Emoji, as in
//! `font_chain_tests`.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

use super::font_chain_tests::{MERMAID_STACK, NOTO_EMOJI_FILE, renamed, table};
use crate::fonts::FontChain;
use crate::fonts::bundled::INTER_REGULAR;
use crate::fonts::query::{FontQuery, GenericFamily};

/// The generic answers every chain here uses: `sans-serif` names `Anter`, so
/// every stack and generic resolves to the `Anter` face and no query reaches
/// the installed sans-serif rule (which reads every installed face's tables).
const GENERICS: &[(GenericFamily, &str)] = &[(GenericFamily::SansSerif, "Anter")];

fn stacks() -> Vec<FontQuery> {
    vec![
        FontQuery::from_css(MERMAID_STACK, 400, false),
        FontQuery::from_css(MERMAID_STACK, 700, false),
    ]
}

fn anter() -> Vec<u8> {
    renamed(INTER_REGULAR, "Inter", "Anter")
}

/// A face no stack resolves to: reachable only as a fallback.
fn zebra() -> Vec<u8> {
    renamed(NOTO_EMOJI_FILE, "Noto Emoji", "Zebra Mono")
}

fn fixed_time() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

/// Font files in a temp directory, each written with modification time
/// [`fixed_time`].
struct FontDir(TempDir);

impl FontDir {
    fn new() -> Self {
        FontDir(TempDir::new().expect("temp dir"))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.path().join(name);
        std::fs::write(&path, bytes).expect("writes the font");
        set_modified(&path, fixed_time());
        path
    }
}

fn set_modified(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(time))
        .expect("sets the modification time");
}

fn identity_of(paths: &[&Path]) -> u64 {
    FontChain::injected_files(paths, GENERICS, None).cache_identity(&stacks())
}

/// A copy of `font` whose `name` table holds only `family` (name IDs 1, 2, 4,
/// and 6, Windows Unicode English), so the family can have any length.
fn with_family(font: &[u8], family: &str) -> Vec<u8> {
    let postscript: String = family.chars().filter(|c| !c.is_whitespace()).collect();
    let names = [
        (1u16, family.to_string()),
        (2, "Regular".to_string()),
        (4, format!("{family} Regular")),
        (6, format!("{postscript}-Regular")),
    ];
    let mut records = Vec::new();
    let mut strings = Vec::new();
    for (name_id, value) in &names {
        let encoded: Vec<u8> = value.encode_utf16().flat_map(u16::to_be_bytes).collect();
        for field in [3u16, 1, 0x0409, *name_id, encoded.len() as u16, strings.len() as u16] {
            records.extend(field.to_be_bytes());
        }
        strings.extend(encoded);
    }
    let mut name_table = Vec::new();
    for field in [0u16, names.len() as u16, 6 + records.len() as u16] {
        name_table.extend(field.to_be_bytes());
    }
    name_table.extend(records);
    name_table.extend(strings);

    let old = table(font, b"name");
    let mut bytes = font.to_vec();
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    let offset = bytes.len() as u32;
    bytes.extend(&name_table);
    let count = u16::from_be_bytes([font[4], font[5]]) as usize;
    let record = (0..count)
        .map(|i| 12 + 16 * i)
        .find(|at| u32::from_be_bytes(font[at + 8..at + 12].try_into().unwrap()) as usize == old.start)
        .expect("the name table record");
    bytes[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
    bytes[record + 12..record + 16].copy_from_slice(&(name_table.len() as u32).to_be_bytes());
    bytes
}

// ------------------------------------------------- faces hashed by contents

/// AC-C6a: a resolved face's contents are hashed, so changing them changes
/// the identity even with the same size and modification time.
#[test]
fn changing_a_resolved_faces_contents_changes_the_identity() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let zebra_path = dir.write("zebra.ttf", &zebra());
    let before = identity_of(&[&anter_path, &zebra_path]);

    let mut patched = anter();
    let checksum = table(&patched, b"head").start + 8;
    patched[checksum] ^= 0xFF;
    dir.write("anter.ttf", &patched);

    assert_ne!(identity_of(&[&anter_path, &zebra_path]), before);
}

// ------------------------------------------ faces identified by file metadata

/// AC-C6a, D70: changing only the size of a file no stack resolves to
/// changes the identity.
#[test]
fn changing_only_another_files_size_changes_the_identity() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let zebra_path = dir.write("zebra.ttf", &zebra());
    let before = identity_of(&[&anter_path, &zebra_path]);

    let mut grown = zebra();
    grown.extend([0; 4]);
    dir.write("zebra.ttf", &grown);

    assert_ne!(identity_of(&[&anter_path, &zebra_path]), before);
}

/// AC-C6a, D70: changing only the modification time of a file no stack
/// resolves to changes the identity.
#[test]
fn changing_only_another_files_modification_time_changes_the_identity() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let zebra_path = dir.write("zebra.ttf", &zebra());
    let before = identity_of(&[&anter_path, &zebra_path]);

    set_modified(&zebra_path, fixed_time() + Duration::from_secs(86_400));

    assert_ne!(identity_of(&[&anter_path, &zebra_path]), before);
}

/// AC-C6a, D70: a file no stack resolves to is never read for the key.
/// Replacing it, after the chain loaded it, with same-size garbage and the
/// same modification time leaves the identity unchanged (D70's accepted
/// residual), and only the resolved and bundled faces' contents are hashed.
#[test]
fn another_files_contents_are_never_read() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let zebra_path = dir.write("zebra.ttf", &zebra());
    let pristine = identity_of(&[&anter_path, &zebra_path]);
    let chain = FontChain::injected_files(&[&anter_path, &zebra_path], GENERICS, None);

    dir.write("zebra.ttf", &vec![0xAB; zebra().len()]);
    let identity = chain.cache_identity(&stacks());

    assert_eq!(identity, pristine);
    assert_eq!(chain.contents_hashed(), chain.bundled_ids().len() + 1);
}

// ---------------------------------------- the faces the chain actually uses

/// AC-C6a, D63: changing only `MERMAID_FONT` changes the identity.
#[test]
fn changing_only_mermaid_font_changes_the_identity() {
    let (anter, zebra) = (anter(), zebra());

    let unset = FontChain::injected(&[&anter, &zebra], GENERICS, None).cache_identity(&stacks());
    let pinned = FontChain::injected(&[&anter, &zebra], GENERICS, Some("Inter")).cache_identity(&stacks());

    assert_ne!(unset, pinned);
}

/// AC-C6a, D63: changing only the installed font set changes the identity.
#[test]
fn changing_only_the_installed_font_set_changes_the_identity() {
    let (anter, zebra) = (anter(), zebra());

    let fewer = FontChain::injected(&[&anter], GENERICS, None).cache_identity(&stacks());
    let more = FontChain::injected(&[&anter, &zebra], GENERICS, None).cache_identity(&stacks());

    assert_ne!(fewer, more);
}

/// AC-C6a, D63: changing only the fallback order changes the identity. Of
/// two fallback faces, `Yebra` sorts first until it is renamed in place to
/// `Zebrb`, with the same size and modification time, so the font-set hash
/// is unchanged and only the order differs.
#[test]
fn changing_only_the_fallback_order_changes_the_identity() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let zebra_path = dir.write("zebra.ttf", &zebra());
    let yebra_path = dir.write("yebra.ttf", &renamed(&zebra(), "Zebra", "Yebra"));
    let paths: [&Path; 3] = [&anter_path, &zebra_path, &yebra_path];
    let before = FontChain::injected_files(&paths, GENERICS, None);

    dir.write("yebra.ttf", &renamed(&zebra(), "Zebra", "Zebrb"));
    let after = FontChain::injected_files(&paths, GENERICS, None);

    assert_eq!(before.font_set_hash(&stacks()), after.font_set_hash(&stacks()));
    assert_ne!(before.cache_identity(&stacks()), after.cache_identity(&stacks()));
}

/// AC-C6a, D41: a font a family's own CSS requests (`courier`) is covered
/// although no stack names it: installing a `Courier` face changes the
/// identity, without its contents being hashed.
#[test]
fn a_css_requested_courier_face_is_covered_without_naming_it() {
    let dir = FontDir::new();
    let anter_path = dir.write("anter.ttf", &anter());
    let courier_path = dir.write("courier.ttf", &with_family(INTER_REGULAR, "Courier"));
    assert!(!MERMAID_STACK.to_lowercase().contains("courier"));
    let without = FontChain::injected_files(&[&anter_path], GENERICS, None);
    let with = FontChain::injected_files(&[&anter_path, &courier_path], GENERICS, None);
    let courier = with
        .resolve(&FontQuery::from_css("courier", 400, false))
        .expect("a face");
    assert_eq!(with.database().face(courier).expect("face").families[0].0, "Courier");

    assert_ne!(without.cache_identity(&stacks()), with.cache_identity(&stacks()));
    assert_eq!(with.contents_hashed(), with.bundled_ids().len() + 1);
}

// --------------------------------------------------- once per process (D41)

/// AC-C6a, D41: font contents are hashed on the first call only; a second
/// call (the next render) returns the memoized identity.
#[test]
fn contents_are_hashed_once_across_two_calls() {
    let (anter, zebra) = (anter(), zebra());
    let chain = FontChain::injected(&[&anter, &zebra], GENERICS, None);

    let first = chain.cache_identity(&stacks());
    let hashed = chain.contents_hashed();
    let second = chain.cache_identity(&stacks());

    assert_eq!(first, second);
    assert!(hashed > 0);
    assert_eq!(chain.contents_hashed(), hashed);
}
