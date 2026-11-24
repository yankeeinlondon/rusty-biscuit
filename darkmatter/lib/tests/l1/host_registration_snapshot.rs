//! `darkmatter::config::registration::bundle`: a format 2 `darkmatter-host`
//! manifest loaded into one verified snapshot. The closed envelope, the
//! format 1 upgrade warning, digest-table agreement, member path rules,
//! digest verification with its single retry over a republished manifest,
//! and the revision's independence from `files` order and install location.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::migration_receipt::content_hash;
use darkmatter::config::registration::HOST_REGISTRATION_CODE;
use darkmatter::config::registration::bundle::{
    SnapshotRefusal, VerifiedSnapshot, admit_installed_manifest, admit_manifest, load_snapshot, load_snapshot_with, verify_read,
};

const FILES: [&str; 2] = ["./schemas/a.yaml", "./schemas/b.yaml"];
const LIFECYCLE: &str = "./globals.lifecycle.json";
const CATALOG: &str = "./functions/catalog.yaml";
const FACTS: &str = "./functions/facts.json";

/// A temporary bundle under `host/`, beside an empty home and working
/// directory; nothing is read from the process.
struct Bundle {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Bundle {
    /// A valid bundle: two schema files, a lifecycle artifact, one function
    /// pair, and a manifest listing their digests.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        let bundle = Self { _dir: dir, root };
        bundle.write_members("one");
        bundle.publish(&Members::default());
        bundle
    }

    fn host(&self) -> PathBuf {
        self.root.join("host")
    }

    fn manifest(&self) -> PathBuf {
        self.host().join("manifest.yaml")
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.host().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Writes every default member, its content tagged with `release`.
    fn write_members(&self, release: &str) {
        for member in FILES.into_iter().chain([LIFECYCLE, CATALOG, FACTS]) {
            self.write(member, &format!("# {member} from release {release}\n"));
        }
    }

    /// The manifest text for `members`, with digests of the files on disk.
    fn manifest_text(&self, members: &Members) -> String {
        manifest_for(&self.host(), members)
    }

    fn publish(&self, members: &Members) {
        let text = self.manifest_text(members);
        std::fs::write(self.manifest(), text).unwrap();
    }

    fn write_manifest(&self, text: &str) {
        std::fs::write(self.manifest(), text).unwrap();
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.root.join("home")), HashMap::new())
    }

    fn load(&self) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
        load_snapshot(&self.manifest(), &self.context())
    }

    fn loaded(&self) -> VerifiedSnapshot {
        self.load().unwrap_or_else(|refusal| panic!("the bundle loads: {refusal:#?}"))
    }

    /// The refusal of a load, which must be located in the manifest.
    fn refused(&self) -> SnapshotRefusal {
        let refusal = *self.load().expect_err("the bundle is refused");
        assert_refusal_in(&refusal, &self.manifest());
        refusal
    }
}

/// Checks the shape every refusal shares.
fn assert_refusal_in(refusal: &SnapshotRefusal, manifest: &Path) {
    assert_eq!(refusal.code, HOST_REGISTRATION_CODE, "{refusal:#?}");
    assert_eq!(refusal.file, manifest, "{refusal:#?}");
    assert!(refusal.line.is_some() && refusal.column.is_some(), "located: {refusal:#?}");
}

/// The member entries of a manifest.
#[derive(Clone)]
struct Members {
    files: Vec<String>,
    lifecycle: String,
    functions: Vec<(String, String)>,
}

impl Default for Members {
    fn default() -> Self {
        Self {
            files: FILES.iter().map(ToString::to_string).collect(),
            lifecycle: LIFECYCLE.to_string(),
            functions: vec![(CATALOG.to_string(), FACTS.to_string())],
        }
    }
}

impl Members {
    fn with_files(files: &[&str]) -> Self {
        Self { files: files.iter().map(ToString::to_string).collect(), ..Self::default() }
    }

    fn all(&self) -> Vec<String> {
        let mut all = self.files.clone();
        all.push(self.lifecycle.clone());
        for (catalog, facts) in &self.functions {
            all.push(catalog.clone());
            all.push(facts.clone());
        }
        all
    }
}

/// The digest of the file `reference` names below `host`; a placeholder for
/// a file that is not there (a member refused before it is read).
fn digest_of(host: &Path, reference: &str) -> String {
    std::fs::read(host.join(reference)).map_or_else(|_| "0".repeat(64), |bytes| content_hash(&bytes))
}

fn manifest_for(host: &Path, members: &Members) -> String {
    let mut text = String::from("kind: darkmatter-host\nformat_version: 2\nhost: acme\nversion: \"1.2.0\"\n");
    if members.files.is_empty() {
        text.push_str("files: []\n");
    } else {
        text.push_str("files:\n");
        for file in &members.files {
            text.push_str(&format!("    - '{file}'\n"));
        }
    }
    text.push_str(&format!("lifecycle: '{}'\n", members.lifecycle));
    if members.functions.is_empty() {
        text.push_str("functions: []\n");
    } else {
        text.push_str("functions:\n");
        for (catalog, facts) in &members.functions {
            text.push_str(&format!("    - catalog: '{catalog}'\n      facts: '{facts}'\n"));
        }
    }
    text.push_str("digests:\n");
    for member in members.all() {
        text.push_str(&format!("    '{member}': \"{}\"\n", digest_of(host, &member)));
    }
    text
}

/// The 1-based line of the first line of `text` containing `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|line| line.contains(needle)).map(|index| index + 1).unwrap_or_else(|| panic!("`{needle}` in {text}"))
}

/// `text` with the top-level `field` (and its indented continuation lines)
/// replaced by `replacement` lines; an empty replacement removes it.
fn replace_field(text: &str, field: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in text.lines() {
        if line.starts_with(&format!("{field}:")) {
            out.push_str(replacement);
            skipping = true;
        } else if skipping && line.starts_with(' ') {
        } else {
            skipping = false;
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// ── control ─────────────────────────────────────────────────────────────────

#[test]
fn a_valid_bundle_loads_into_one_verified_snapshot() {
    let bundle = Bundle::new();
    let snapshot = bundle.loaded();
    let manifest_text = std::fs::read_to_string(bundle.manifest()).unwrap();

    assert_eq!((snapshot.host(), snapshot.version()), ("acme", "1.2.0"));
    assert_eq!(snapshot.manifest(), bundle.manifest());
    assert_eq!(snapshot.manifest_hash(), content_hash(manifest_text.as_bytes()));
    assert_eq!(snapshot.bundle_root(), bundle.host());
    let files: Vec<&str> = snapshot.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(files, ["schemas/a.yaml", "schemas/b.yaml"]);
    assert_eq!(snapshot.sorted_files(), ["schemas/a.yaml", "schemas/b.yaml"]);
    assert_eq!(snapshot.lifecycle().path, "globals.lifecycle.json");
    let [pair] = snapshot.functions() else { panic!("one pair: {:?}", snapshot.functions()) };
    assert_eq!((pair.catalog.path.as_str(), pair.facts.path.as_str()), ("functions/catalog.yaml", "functions/facts.json"));
    assert_eq!(pair.facts.field, "functions[0].facts");

    let keys: Vec<&str> = snapshot.bytes().keys().map(String::as_str).collect();
    assert_eq!(keys, ["functions/catalog.yaml", "functions/facts.json", "globals.lifecycle.json", "schemas/a.yaml", "schemas/b.yaml"]);
    let a = std::fs::read(bundle.host().join("schemas/a.yaml")).unwrap();
    assert_eq!(snapshot.member_bytes("schemas/a.yaml").map(|bytes| &bytes[..]), Some(&a[..]));

    let second = &snapshot.files()[1];
    assert_eq!((second.field.as_str(), second.reference.as_str()), ("files[1]", "./schemas/b.yaml"));
    assert_eq!(second.at.as_ref().map(|at| at.line), Some(line_of(&manifest_text, "- './schemas/b.yaml'")));
    assert_eq!(second.location, bundle.host().join("schemas/b.yaml"));

    let read: Vec<&Path> = snapshot.dependencies().iter().map(|dependency| dependency.path.as_path()).collect();
    assert_eq!(read[0], bundle.manifest(), "the manifest is read first");
    assert_eq!(read.len(), 6, "the manifest and five members, once each: {read:?}");
    assert!(snapshot.candidates().contains(&bundle.host().join("functions/facts.json")));
    assert_eq!(snapshot.revision().len(), 64);
}

/// Every default member's bytes as on disk, keyed by bundle path.
fn read_members(bundle: &Bundle) -> BTreeMap<String, std::sync::Arc<[u8]>> {
    Members::default()
        .all()
        .iter()
        .map(|reference| {
            let path = reference.trim_start_matches("./").to_string();
            let bytes = std::fs::read(bundle.host().join(&path)).unwrap();
            (path, std::sync::Arc::from(bytes))
        })
        .collect()
}

#[test]
fn verifying_already_read_bytes_gives_the_snapshot_the_reader_gives() {
    let bundle = Bundle::new();
    let manifest = std::fs::read(bundle.manifest()).unwrap();

    let verified = verify_read(&bundle.manifest(), &manifest, &bundle.context(), &read_members(&bundle))
        .unwrap_or_else(|refusal| panic!("the bytes verify: {refusal:#?}"));

    assert_eq!(verified, bundle.loaded());
}

#[test]
fn verifying_already_read_bytes_refuses_a_missing_or_mismatched_member_at_its_entry() {
    let bundle = Bundle::new();
    let manifest = std::fs::read(bundle.manifest()).unwrap();
    let verify = |members: &BTreeMap<String, std::sync::Arc<[u8]>>| {
        *verify_read(&bundle.manifest(), &manifest, &bundle.context(), members).expect_err("the bytes are refused")
    };

    let mut missing = read_members(&bundle);
    missing.remove("schemas/b.yaml");
    let refusal = verify(&missing);
    assert_refusal_in(&refusal, &bundle.manifest());
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("is missing from the bundle"), "{}", refusal.message);

    let mut changed = read_members(&bundle);
    changed.insert("schemas/b.yaml".to_string(), std::sync::Arc::from(&b"# edited\n"[..]));
    let refusal = verify(&changed);
    assert_refusal_in(&refusal, &bundle.manifest());
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("does not match its digest"), "{}", refusal.message);
}

#[test]
fn loading_one_bundle_twice_gives_equal_snapshots() {
    let bundle = Bundle::new();
    assert_eq!(bundle.loaded(), bundle.loaded());
}

#[test]
fn reordering_files_keeps_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    bundle.publish(&Members::with_files(&["./schemas/b.yaml", "./schemas/a.yaml"]));
    let after = bundle.loaded();
    assert_ne!(before.manifest_hash(), after.manifest_hash(), "the manifest bytes changed");
    assert_eq!(before.revision(), after.revision());
    assert_eq!(after.sorted_files(), before.sorted_files());
}

#[test]
fn a_copied_bundle_keeps_the_revision() {
    let bundle = Bundle::new();
    let copy = bundle.root.join("elsewhere/deeper/host-copy");
    for relative in ["manifest.yaml", "schemas/a.yaml", "schemas/b.yaml", "globals.lifecycle.json", "functions/catalog.yaml", "functions/facts.json"] {
        let target = copy.join(relative);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(bundle.host().join(relative), target).unwrap();
    }
    let copied = load_snapshot(&copy.join("manifest.yaml"), &bundle.context()).unwrap();
    let original = bundle.loaded();
    assert_eq!(copied.revision(), original.revision());
    assert_eq!(copied.bundle_root(), copy);
}

#[test]
fn changing_one_member_byte_changes_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    bundle.write("schemas/b.yaml", "# changed\n");
    bundle.publish(&Members::default());
    assert_ne!(bundle.loaded().revision(), before.revision());
}

// ── format 1 ────────────────────────────────────────────────────────────────

#[test]
fn a_format_1_manifest_asks_for_the_hosts_setup() {
    let bundle = Bundle::new();
    let legacy = "kind: darkmatter-host\nformat_version: 1\nhost: acme\nversion: 1.2.0\nidentify: ./schemas/a.yaml\n\
                  triggers: []\nglobals: ./schemas/b.yaml\nlifecycle: ./globals.lifecycle.json\nfunctions: []\n";
    bundle.write_manifest(legacy);
    let refusal = bundle.refused();
    assert!(refusal.legacy_format, "{refusal:#?}");
    assert_eq!(refusal.line, Some(2), "at `format_version`: {refusal:#?}");
    assert!(refusal.message.contains("format 2"), "{}", refusal.message);
    assert!(refusal.message.contains("`acme setup`") && refusal.message.contains("update `acme`"), "{}", refusal.message);
    assert_eq!(refusal.candidates, [bundle.manifest()], "no member was looked for");
}

#[test]
fn a_manifest_with_format_1_fields_and_no_version_is_format_1() {
    let bundle = Bundle::new();
    bundle.write_manifest("kind: darkmatter-host\nhost: acme\nidentify: ./schemas/a.yaml\n");
    let refusal = bundle.refused();
    assert!(refusal.legacy_format, "{refusal:#?}");
    assert_eq!(refusal.line, Some(3), "at the first format 1 field: {refusal:#?}");
}

// ── the closed envelope ─────────────────────────────────────────────────────

#[test]
fn every_field_is_required() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for field in ["kind", "format_version", "host", "version", "files", "lifecycle", "functions", "digests"] {
        bundle.write_manifest(&replace_field(&valid, field, ""));
        let refusal = bundle.refused();
        assert!(!refusal.legacy_format);
        assert!(refusal.message.contains(&format!("`{field}` is required")), "{field}: {}", refusal.message);
    }
}

#[test]
fn an_unknown_field_is_refused_at_its_key() {
    let bundle = Bundle::new();
    let text = bundle.manifest_text(&Members::default()) + "extra: true\n";
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert!(refusal.message.contains("`extra` is not a manifest field"), "{}", refusal.message);
    assert_eq!(refusal.line, Some(line_of(&text, "extra:")));
}

#[test]
fn a_format_1_field_beside_format_2_is_an_unknown_field() {
    let bundle = Bundle::new();
    bundle.write_manifest(&(bundle.manifest_text(&Members::default()) + "identify: ./schemas/a.yaml\n"));
    let refusal = bundle.refused();
    assert!(!refusal.legacy_format);
    assert!(refusal.message.contains("`identify` is not a manifest field"), "{}", refusal.message);
}

#[test]
fn format_version_is_the_integer_2() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for value in ["2.0", "\"2\"", "3"] {
        bundle.write_manifest(&valid.replace("format_version: 2\n", &format!("format_version: {value}\n")));
        let refusal = bundle.refused();
        assert_eq!(refusal.line, Some(2), "{value} at `format_version`: {refusal:#?}");
        assert!(!refusal.legacy_format, "{value}");
    }
}

#[test]
fn empty_files_and_functions_lists_are_accepted() {
    let bundle = Bundle::new();
    bundle.publish(&Members { files: Vec::new(), functions: Vec::new(), ..Members::default() });
    let snapshot = bundle.loaded();
    assert!(snapshot.files().is_empty() && snapshot.functions().is_empty());
    assert_eq!(snapshot.bytes().len(), 1, "only the lifecycle artifact");
}

#[test]
fn files_and_functions_must_be_lists() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let cases = [
        ("files", "files: null\n", "files:"),
        ("files", "files: ./schemas/a.yaml\n", "files:"),
        ("files", "files:\n    - 7\n", "- 7"),
        ("files", "files:\n    - \"\"\n", "- \"\""),
        ("functions", "functions: null\n", "functions:"),
        ("functions", "functions:\n    - catalog: ./functions/catalog.yaml\n", "- catalog:"),
        ("functions", "functions:\n    - catalog: ./functions/catalog.yaml\n      facts: ./functions/facts.json\n      extra: 1\n", "extra:"),
    ];
    for (field, replacement, at) in cases {
        let text = replace_field(&valid, field, replacement);
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.line, Some(line_of(&text, at)), "{replacement:?}: {refusal:#?}");
    }
}

#[test]
fn host_and_version_are_nonempty_strings() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for (from, to) in [("host: acme\n", "host: \"\"\n"), ("host: acme\n", "host: 7\n"), ("version: \"1.2.0\"\n", "version: \"\"\n")] {
        let text = valid.replace(from, to);
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.line, Some(line_of(&text, to.trim_end())), "{to:?}: {refusal:#?}");
    }
}

// ── digests ─────────────────────────────────────────────────────────────────

#[test]
fn a_member_without_a_digest_is_refused_at_its_entry() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let text: String = valid.lines().filter(|line| !line.contains("'./schemas/b.yaml': ")).map(|line| format!("{line}\n")).collect();
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert_eq!(refusal.line, Some(line_of(&text, "- './schemas/b.yaml'")));
    assert!(refusal.message.contains("no entry in `digests`"), "{}", refusal.message);
}

#[test]
fn a_digest_without_a_member_is_refused_at_its_key() {
    let bundle = Bundle::new();
    let text = bundle.manifest_text(&Members::default()) + &format!("    './schemas/c.yaml': \"{}\"\n", "a".repeat(64));
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("digests../schemas/c.yaml"));
    assert_eq!(refusal.line, Some(line_of(&text, "'./schemas/c.yaml':")));
}

#[test]
fn a_digest_is_64_lowercase_hex_digits() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let digest = digest_of(&bundle.host(), "./schemas/a.yaml");
    for bad in [digest.to_uppercase(), digest[..63].to_string(), format!("{}g", &digest[..63])] {
        let text = valid.replace(&digest, &bad);
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.line, Some(line_of(&text, "'./schemas/a.yaml':")), "{bad}: {refusal:#?}");
    }
}

// ── member paths ────────────────────────────────────────────────────────────

/// Publishes the default members with `files[1]` replaced by `member`, and
/// returns the refusal, which must be located at that entry.
fn refused_member(bundle: &Bundle, member: &str) -> SnapshotRefusal {
    let members = Members::with_files(&["./schemas/a.yaml", member]);
    let text = bundle.manifest_text(&members);
    bundle.write_manifest(&text);
    let refusal = *bundle.load().expect_err(member);
    assert_refusal_in(&refusal, &bundle.manifest());
    assert_eq!(refusal.member.as_deref(), Some("files[1]"), "{member}: {refusal:#?}");
    assert_eq!(refusal.line, Some(line_of(&text, &format!("- '{member}'"))), "{member}");
    refusal
}

#[test]
fn a_member_path_is_written_portably() {
    let bundle = Bundle::new();
    bundle.write("schemas/CON.yaml", "x\n");
    let cases = [
        ("schemas/b.yaml", "must start with `./` or `../`"),
        ("/schemas/b.yaml", "must start with `./` or `../`"),
        ("./schemas\\b.yaml", "forward slashes"),
        ("./schemas/CON.yaml", "device name"),
        ("./schemas/con", "device name"),
        ("./schemas/lpt9.txt", "device name"),
        ("./schemas/b.yaml.", "ends with `.`"),
        ("./schemas//b.yaml", "empty path segment"),
        ("./schemas/", "empty path segment"),
        ("./schemas/b c.yaml", "outside"),
        ("./schémas/b.yaml", "outside"),
        ("./schemas/..", "names a directory"),
        ("../x.yaml", "leaves the bundle"),
        ("./schemas/../../x.yaml", "leaves the bundle"),
    ];
    for (member, expected) in cases {
        let refusal = refused_member(&bundle, member);
        assert!(refusal.message.contains(expected), "{member}: {}", refusal.message);
    }
}

#[test]
fn a_device_name_inside_a_longer_name_is_allowed() {
    let bundle = Bundle::new();
    bundle.write("schemas/console.yaml", "x\n");
    bundle.write("schemas/com10.yaml", "x\n");
    bundle.publish(&Members::with_files(&["./schemas/console.yaml", "./schemas/com10.yaml"]));
    bundle.loaded();
}

#[test]
fn in_bundle_dot_traversal_is_resolved() {
    let bundle = Bundle::new();
    bundle.publish(&Members::with_files(&["./schemas/../schemas/a.yaml", "../host/schemas/b.yaml"]));
    let snapshot = bundle.loaded();
    let files: Vec<&str> = snapshot.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(files, ["schemas/a.yaml", "schemas/b.yaml"]);
    let default = {
        bundle.publish(&Members::default());
        bundle.loaded()
    };
    assert_eq!(snapshot.revision(), default.revision(), "the revision reads normalized paths");
}

#[test]
fn two_entries_naming_one_file_are_refused() {
    let bundle = Bundle::new();
    let refusal = refused_member(&bundle, "./schemas/x/../a.yaml");
    assert!(refusal.message.contains("`files[0]`"), "{}", refusal.message);

    let members = Members { lifecycle: "./schemas/../schemas/a.yaml".to_string(), ..Members::default() };
    bundle.write_manifest(&bundle.manifest_text(&members));
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("lifecycle"), "{refusal:#?}");
}

#[test]
fn names_that_differ_only_in_case_are_refused() {
    let bundle = Bundle::new();
    bundle.write("Schemas/c.yaml", "x\n");
    let refusal = refused_member(&bundle, "./Schemas/c.yaml");
    assert!(refusal.message.contains("letter case"), "{}", refusal.message);
    bundle.write("schemas/A.yaml", "x\n");
    let refusal = refused_member(&bundle, "./schemas/A.yaml");
    assert!(refusal.message.contains("`schemas/A.yaml`"), "{}", refusal.message);
}

#[test]
fn a_remote_member_is_refused_without_a_fetch() {
    let bundle = Bundle::new();
    let refusal = refused_member(&bundle, "https://example.invalid/schemas/b.yaml");
    assert!(refusal.message.contains("remote"), "{}", refusal.message);
}

/// Links `link` (a directory below the bundle) to `target`: a symbolic link
/// on Unix, a junction on Windows, which needs no privilege.
fn link_directory(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "a junction is created");
    }
}

#[test]
fn a_linked_directory_inside_the_bundle_is_refused() {
    let bundle = Bundle::new();
    bundle.write("real/c.yaml", "x\n");
    link_directory(&bundle.host().join("real"), &bundle.host().join("linked"));
    bundle.publish(&Members::with_files(&["./schemas/a.yaml", "./linked/c.yaml"]));
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("`linked`") && refusal.message.contains("junction"), "{}", refusal.message);
}

// ── digest verification and the retry ──────────────────────────────────────

#[test]
fn a_mismatched_member_under_an_unchanged_manifest_is_refused_at_its_entry() {
    let bundle = Bundle::new();
    bundle.write("functions/facts.json", "{ \"edited\": true }\n");
    let refusal = bundle.refused();
    let text = std::fs::read_to_string(bundle.manifest()).unwrap();
    assert_eq!(refusal.member.as_deref(), Some("functions[0].facts"));
    assert_eq!(refusal.line, Some(line_of(&text, "facts: './functions/facts.json'")));
    assert!(refusal.message.contains("does not match its digest"), "{}", refusal.message);
    assert!(refusal.dependencies.iter().any(|dependency| dependency.path == bundle.host().join("functions/facts.json")));
}

#[test]
fn a_missing_member_is_refused_and_watched() {
    let bundle = Bundle::new();
    std::fs::remove_file(bundle.host().join("schemas/b.yaml")).unwrap();
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("missing"), "{}", refusal.message);
    assert!(refusal.candidates.contains(&bundle.host().join("schemas/b.yaml")), "{:?}", refusal.candidates);
    assert!(refusal.candidates.contains(&bundle.host().join("functions/facts.json")), "every member is watched");
}

/// A reader returning each manifest read's bytes from `manifests` in turn
/// (the last repeated) and every other file from disk, counting manifest
/// reads.
fn sequenced(manifest: PathBuf, manifests: Vec<String>, count: &Cell<usize>) -> impl Fn(&Path) -> io::Result<Vec<u8>> + '_ {
    move |path: &Path| {
        if path == manifest {
            let index = count.get().min(manifests.len() - 1);
            count.set(count.get() + 1);
            Ok(manifests[index].clone().into_bytes())
        } else {
            std::fs::read(path)
        }
    }
}

/// A bundle whose members on disk are release `two`, with the manifest
/// texts of release `one` and release `two`.
fn two_releases() -> (Bundle, String, String) {
    let bundle = Bundle::new();
    let one = bundle.manifest_text(&Members::default());
    bundle.write_members("two");
    let two = bundle.manifest_text(&Members::default());
    bundle.write_manifest(&two);
    assert_ne!(one, two);
    (bundle, one, two)
}

#[test]
fn a_republished_manifest_is_verified_again_and_used_alone() {
    let (bundle, one, two) = two_releases();
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one, two.clone()], &count);
    let snapshot = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap();
    assert_eq!(count.get(), 2, "the manifest is read twice");
    assert_eq!(snapshot.manifest_hash(), content_hash(two.as_bytes()));
    for member in snapshot.files() {
        assert_eq!(member.digest, digest_of(&bundle.host(), &member.reference), "release two's digest");
        let bytes = snapshot.member_bytes(&member.path).unwrap();
        assert!(String::from_utf8_lossy(bytes).contains("release two"));
    }
    assert_eq!(snapshot, bundle.loaded(), "the same snapshot as a load of release two alone");
    let manifest = snapshot.dependencies().iter().find(|dependency| dependency.path == bundle.manifest()).unwrap();
    assert_eq!(manifest.content_hash, content_hash(two.as_bytes()), "the latest read's hash");
}

#[test]
fn an_unchanged_manifest_is_not_retried() {
    let (bundle, one, _) = two_releases();
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 2, "read again once, then refused");
    assert_refusal_in(&refusal, &bundle.manifest());
    assert_eq!(refusal.member.as_deref(), Some("files[0]"));
    assert!(refusal.message.contains("does not match its digest"), "{}", refusal.message);
}

#[test]
fn members_of_two_releases_are_never_combined() {
    let (bundle, one, two) = two_releases();
    // Mid-publish: `a.yaml` is release two's, `b.yaml` still release one's.
    bundle.write("schemas/b.yaml", "# ./schemas/b.yaml from release one\n");
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one.clone(), two.clone()], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 2, "one retry only");
    assert_eq!(refusal.member.as_deref(), Some("files[1]"), "release two's check of `b.yaml`: {refusal:#?}");

    // Each release alone, against the same mixed files, is refused too.
    for text in [one, two] {
        bundle.write_manifest(&text);
        bundle.refused();
    }
}

#[test]
fn a_retry_keeps_both_attempts_inputs() {
    let (bundle, one, _) = two_releases();
    let other = Members::with_files(&["./schemas/a.yaml", "./schemas/c.yaml"]);
    bundle.write("schemas/c.yaml", "# c\n");
    let two = bundle.manifest_text(&other);
    std::fs::remove_file(bundle.host().join("schemas/c.yaml")).unwrap();
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one, two], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert!(refusal.message.contains("missing"), "{}", refusal.message);
    for watched in ["schemas/b.yaml", "schemas/c.yaml"] {
        assert!(refusal.candidates.contains(&bundle.host().join(watched)), "{watched}: {:?}", refusal.candidates);
    }
}

// ── admission without member reads ──────────────────────────────────────────

impl Bundle {
    fn admit(&self) -> Result<darkmatter::config::registration::bundle::AdmittedManifest, Box<SnapshotRefusal>> {
        admit_manifest(&self.manifest(), &std::fs::read(self.manifest()).unwrap())
    }
}

#[test]
fn admission_lists_the_members_without_reading_them() {
    let bundle = Bundle::new();
    bundle.write("schemas/a.yaml", "# changed after publication\n");
    std::fs::remove_file(bundle.host().join("schemas/b.yaml")).unwrap();
    bundle.load().expect_err("the verifying reader refuses the mismatch");

    let admitted = bundle.admit().unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    assert_eq!((admitted.host(), admitted.version()), ("acme", "1.2.0"));
    let paths: Vec<&str> = admitted.members().map(|member| member.path.as_str()).collect();
    assert_eq!(
        paths,
        ["schemas/a.yaml", "schemas/b.yaml", "globals.lifecycle.json", "functions/catalog.yaml", "functions/facts.json"]
    );
}

/// One edit per shared validator (envelope, digest map, YAML document,
/// member path): admission refuses exactly where the verifying reader does,
/// located at the same place.
#[test]
fn admission_refuses_what_the_verifying_reader_refuses() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let digest_line = valid.lines().find(|line| line.contains("'./schemas/a.yaml':")).unwrap().to_string();
    let cases = [
        ("version empty", valid.replace("version: \"1.2.0\"", "version: \"\"")),
        ("version omitted", replace_field(&valid, "version", "")),
        ("format 1", valid.replace("format_version: 2", "format_version: 1")),
        ("digests null", replace_field(&valid, "digests", "digests: null\n")),
        ("digest nonhex", valid.replace(&digest_line, &format!("    './schemas/a.yaml': \"{}\"", "z".repeat(64)))),
        ("duplicate digest key", format!("{valid}{digest_line}\n")),
        ("second document", format!("{valid}---\nkind: other\n")),
        ("unprefixed member", bundle.manifest_text(&Members::with_files(&["./schemas/a.yaml", "schemas/b.yaml"]))),
        ("escaping member", bundle.manifest_text(&Members::with_files(&["./schemas/a.yaml", "../x.yaml"]))),
    ];
    for (case, text) in cases {
        bundle.write_manifest(&text);
        let read = *bundle.load().expect_err(case);
        let admitted = *bundle.admit().expect_err(case);
        assert_eq!(
            (admitted.message, admitted.line, admitted.column, admitted.member, admitted.legacy_format),
            (read.message, read.line, read.column, read.member, read.legacy_format),
            "{case}"
        );
    }
}

/// Admission applies `.` and `..` to the spelling, so a path that climbs
/// out of the bundle folder and back in, which the installed reader
/// resolves, is refused: admission is never the more permissive reader.
#[test]
fn admission_refuses_a_path_that_climbs_out_and_back_in() {
    let bundle = Bundle::new();
    bundle.publish(&Members::with_files(&["./schemas/a.yaml", "../host/schemas/b.yaml"]));
    bundle.loaded();

    let refusal = *bundle.admit().expect_err("lexically, `..` leaves the bundle");

    assert_eq!(refusal.member.as_deref(), Some("files[1]"), "{refusal:#?}");
    assert!(refusal.message.contains("leaves the bundle"), "{}", refusal.message);
}

/// Installed admission resolves every member category (`files`,
/// `lifecycle`, each function's `catalog` and `facts`) as the verifying
/// reader does: a spelling that climbs out and back in is the in-bundle
/// member, and one that ends outside the bundle is refused at its entry,
/// where the reader refuses it.
#[test]
fn installed_admission_resolves_every_member_category_as_the_reader_does() {
    let bundle = Bundle::new();
    let reentered = Members {
        files: vec!["./schemas/a.yaml".to_string(), "../host/schemas/b.yaml".to_string()],
        lifecycle: "../host/globals.lifecycle.json".to_string(),
        functions: vec![("../host/functions/catalog.yaml".to_string(), "../host/functions/facts.json".to_string())],
    };
    bundle.publish(&reentered);
    let bytes = std::fs::read(bundle.manifest()).unwrap();
    let loaded = bundle.loaded();

    let admitted = admit_installed_manifest(&bundle.manifest(), &bytes, &bundle.context())
        .unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    let paths: Vec<&str> = admitted.members().map(|member| member.path.as_str()).collect();
    assert_eq!(
        paths,
        ["schemas/a.yaml", "schemas/b.yaml", "globals.lifecycle.json", "functions/catalog.yaml", "functions/facts.json"]
    );
    let read: Vec<&str> = loaded.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(read, paths[..2], "the reader names the same members");

    let fields = ["files[1]", "lifecycle", "functions[0].catalog", "functions[0].facts"];
    for (index, field) in fields.into_iter().enumerate() {
        let mut escaping = Members::default();
        match index {
            0 => escaping.files[1] = "../schemas/b.yaml".to_string(),
            1 => escaping.lifecycle = "../globals.lifecycle.json".to_string(),
            2 => escaping.functions[0].0 = "../functions/catalog.yaml".to_string(),
            _ => escaping.functions[0].1 = "../functions/facts.json".to_string(),
        }
        bundle.publish(&escaping);
        let bytes = std::fs::read(bundle.manifest()).unwrap();
        let read = *bundle.load().expect_err(field);
        let admitted = *admit_installed_manifest(&bundle.manifest(), &bytes, &bundle.context()).expect_err(field);
        assert_eq!(admitted.member.as_deref(), Some(field), "{admitted:#?}");
        assert!(admitted.message.contains("leaves the bundle"), "{field}: {}", admitted.message);
        assert_eq!((admitted.message, admitted.line, admitted.member), (read.message, read.line, read.member), "{field}");
    }
}
