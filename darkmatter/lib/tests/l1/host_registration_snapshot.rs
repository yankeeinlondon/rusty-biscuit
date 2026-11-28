//! `darkmatter::config::registration::bundle`: a format 3 `darkmatter-host`
//! manifest loaded into one verified snapshot. The format version checked
//! before the closed envelope, digest-table agreement, member path rules,
//! expression catalogs' derived facts companions, digest verification with
//! its single retry over a republished manifest, the revision's independence
//! from `files` order and install location, the publisher's admission of a
//! previous format 2 manifest, and parity between the installed, pre-read,
//! and embedded readers.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::migration_receipt::content_hash;
use darkmatter::config::registration::HOST_REGISTRATION_CODE;
use darkmatter::config::registration::bundle::{
    AdmittedManifest, SnapshotRefusal, VerifiedSnapshot, admit_installed_manifest, admit_manifest, load_snapshot,
    load_snapshot_with, verify_read,
};

const A: &str = "./schemas/a.yaml";
const B: &str = "./schemas/b.yaml";
/// An expression catalog (`kind: expression-schema`).
const CATALOG: &str = "./functions/project.yaml";
/// [`CATALOG`]'s derived facts companion.
const FACTS: &str = "./functions/project.facts.json";
const LIFECYCLE: &str = "./globals.lifecycle.json";

const EMBEDDED_NAME: &str = "acme";

/// The exact refusal of a manifest declaring format `version`.
fn unsupported(version: i128) -> String {
    format!(
        "unsupported host registration format version {version}; update Darkmatter and the host tool, then republish \
         the registration"
    )
}

/// A temporary bundle under `host/`, beside an empty home and working
/// directory; nothing is read from the process.
struct Bundle {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Bundle {
    /// A valid bundle: two schema files and an expression catalog in
    /// `files`, a lifecycle artifact, the catalog's facts companion, and a
    /// manifest listing their digests.
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

    /// Writes an expression catalog at `relative`.
    fn write_catalog(&self, relative: &str) {
        self.write(relative, &format!("kind: expression-schema\n# {relative}\n"));
    }

    /// Writes every default member, its content tagged with `release`.
    fn write_members(&self, release: &str) {
        for member in [A, B] {
            self.write(member, &format!("kind: schema\n# {member} from release {release}\n"));
        }
        self.write(CATALOG, &format!("kind: expression-schema\n# {CATALOG} from release {release}\n"));
        for member in [LIFECYCLE, FACTS] {
            self.write(member, &format!("{{ \"member\": \"{member}\", \"release\": \"{release}\" }}\n"));
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

    fn manifest_on_disk(&self) -> String {
        std::fs::read_to_string(self.manifest()).unwrap()
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

    fn admit(&self) -> Result<AdmittedManifest, Box<SnapshotRefusal>> {
        admit_manifest(&self.manifest(), &std::fs::read(self.manifest()).unwrap())
    }

    /// Every file below `host/` but the manifest, keyed by bundle path.
    fn files_on_disk(&self) -> BTreeMap<String, Arc<[u8]>> {
        fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Arc<[u8]>>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
                let kind = entry.file_type().unwrap();
                if kind.is_dir() {
                    walk(&entry.path(), &path, out);
                } else if kind.is_file() && path != "manifest.yaml" {
                    out.insert(path, Arc::from(std::fs::read(entry.path()).unwrap()));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.host(), "", &mut out);
        out
    }

    /// The bundle verified from memory as a host would embed it: the
    /// manifest and every file below `host/`.
    fn embedded(&self) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
        let manifest = std::fs::read(self.manifest()).unwrap();
        VerifiedSnapshot::from_embedded(EMBEDDED_NAME, "manifest.yaml", &manifest, &self.files_on_disk())
    }
}

/// Checks the shape every refusal shares.
fn assert_refusal_in(refusal: &SnapshotRefusal, manifest: &Path) {
    assert_eq!(refusal.code, HOST_REGISTRATION_CODE, "{refusal:#?}");
    assert_eq!(refusal.file, manifest, "{refusal:#?}");
    assert!(refusal.line.is_some() && refusal.column.is_some(), "located: {refusal:#?}");
}

/// The embedded reader refuses the bundle where the installed reader does,
/// with the same message.
fn assert_embedded_parity(bundle: &Bundle) {
    assert_embedded_parity_with(bundle, &bundle.files_on_disk());
}

/// [`assert_embedded_parity`] with the embedded member bytes `members`.
fn assert_embedded_parity_with(bundle: &Bundle, members: &BTreeMap<String, Arc<[u8]>>) {
    let manifest = std::fs::read(bundle.manifest()).unwrap();
    let read = *bundle.load().expect_err("the installed reader refuses");
    let embedded = *VerifiedSnapshot::from_embedded(EMBEDDED_NAME, "manifest.yaml", &manifest, members)
        .expect_err("the embedded reader refuses");
    assert_eq!(
        (&embedded.message, &embedded.member, embedded.line, embedded.column, embedded.unsupported_format),
        (&read.message, &read.member, read.line, read.column, read.unsupported_format),
    );
}

/// The member entries of a manifest: `files`, `lifecycle`, and the extra
/// `digests` keys (the derived companions).
#[derive(Clone)]
struct Members {
    files: Vec<String>,
    lifecycle: String,
    companions: Vec<String>,
}

impl Default for Members {
    fn default() -> Self {
        Self {
            files: [A, B, CATALOG].iter().map(ToString::to_string).collect(),
            lifecycle: LIFECYCLE.to_string(),
            companions: vec![FACTS.to_string()],
        }
    }
}

impl Members {
    /// `files` replaced, the default companion kept.
    fn with_files(files: &[&str]) -> Self {
        Self { files: files.iter().map(ToString::to_string).collect(), ..Self::default() }
    }

    fn companions(mut self, companions: &[&str]) -> Self {
        self.companions = companions.iter().map(ToString::to_string).collect();
        self
    }

    fn all(&self) -> Vec<String> {
        let mut all = self.files.clone();
        all.push(self.lifecycle.clone());
        all.extend(self.companions.iter().cloned());
        all
    }
}

/// The digest of the file `reference` names below `host`; a placeholder for
/// a file that is not there (a member refused before it is read).
fn digest_of(host: &Path, reference: &str) -> String {
    std::fs::read(host.join(reference)).map_or_else(|_| "0".repeat(64), |bytes| content_hash(&bytes))
}

fn manifest_for(host: &Path, members: &Members) -> String {
    let mut text = String::from("kind: darkmatter-host\nformat_version: 3\nhost: acme\nversion: \"1.2.0\"\n");
    if members.files.is_empty() {
        text.push_str("files: []\n");
    } else {
        text.push_str("files:\n");
        for file in &members.files {
            text.push_str(&format!("    - '{file}'\n"));
        }
    }
    text.push_str(&format!("lifecycle: '{}'\n", members.lifecycle));
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

/// `text` without the `digests` entry for `key`.
fn without_digest(text: &str, key: &str) -> String {
    let entry = format!("'{key}': ");
    text.lines().filter(|line| !line.contains(&entry)).map(|line| format!("{line}\n")).collect()
}

/// The `digests` entry line for `key` in `text`.
fn digest_line(text: &str, key: &str) -> String {
    let entry = format!("'{key}': ");
    text.lines().find(|line| line.contains(&entry)).unwrap_or_else(|| panic!("`{key}` in {text}")).to_string()
}

/// The `files` entry line for `reference` in a manifest written by
/// [`manifest_for`].
fn files_line(text: &str, reference: &str) -> usize {
    line_of(text, &format!("- '{reference}'"))
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

// ── control ─────────────────────────────────────────────────────────────────

#[test]
fn a_valid_bundle_loads_into_one_verified_snapshot() {
    let bundle = Bundle::new();
    let snapshot = bundle.loaded();
    let manifest_text = bundle.manifest_on_disk();

    assert_eq!((snapshot.host(), snapshot.version()), ("acme", "1.2.0"));
    assert_eq!(snapshot.manifest(), bundle.manifest());
    assert_eq!(snapshot.manifest_hash(), content_hash(manifest_text.as_bytes()));
    assert_eq!(snapshot.bundle_root(), bundle.host());
    let files: Vec<&str> = snapshot.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(files, ["schemas/a.yaml", "schemas/b.yaml", "functions/project.yaml"]);
    assert_eq!(snapshot.sorted_files(), ["functions/project.yaml", "schemas/a.yaml", "schemas/b.yaml"]);
    assert_eq!(snapshot.lifecycle().path, "globals.lifecycle.json");
    let [pair] = snapshot.companions() else { panic!("one companion: {:?}", snapshot.companions()) };
    assert_eq!((pair.catalog.path.as_str(), pair.facts.path.as_str()), ("functions/project.yaml", "functions/project.facts.json"));
    assert_eq!((pair.facts.field.as_str(), pair.facts.reference.as_str()), ("files[2]", FACTS), "listed at its catalog's entry");
    assert_eq!(pair.facts.digest, digest_of(&bundle.host(), FACTS));
    let members: Vec<&str> = snapshot.members().map(|member| member.path.as_str()).collect();
    assert_eq!(
        members,
        ["schemas/a.yaml", "schemas/b.yaml", "functions/project.yaml", "globals.lifecycle.json", "functions/project.facts.json"]
    );

    let keys: Vec<&str> = snapshot.bytes().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["functions/project.facts.json", "functions/project.yaml", "globals.lifecycle.json", "schemas/a.yaml", "schemas/b.yaml"]
    );
    let facts = std::fs::read(bundle.host().join("functions/project.facts.json")).unwrap();
    assert_eq!(snapshot.member_bytes("functions/project.facts.json").map(|bytes| &bytes[..]), Some(&facts[..]));

    let second = &snapshot.files()[1];
    assert_eq!((second.field.as_str(), second.reference.as_str()), ("files[1]", B));
    assert_eq!(second.at.as_ref().map(|at| at.line), Some(files_line(&manifest_text, B)));
    assert_eq!(second.location, bundle.host().join("schemas/b.yaml"));

    let read: Vec<&Path> = snapshot.dependencies().iter().map(|dependency| dependency.path.as_path()).collect();
    assert_eq!(read[0], bundle.manifest(), "the manifest is read first");
    assert_eq!(read.len(), 6, "the manifest and five members, once each: {read:?}");
    assert!(snapshot.candidates().contains(&bundle.host().join("functions/project.facts.json")));
    assert_eq!(snapshot.revision().len(), 64);
}

#[test]
fn verifying_already_read_bytes_gives_the_snapshot_the_reader_gives() {
    let bundle = Bundle::new();
    let manifest = std::fs::read(bundle.manifest()).unwrap();

    let verified = verify_read(&bundle.manifest(), &manifest, &bundle.context(), &bundle.files_on_disk())
        .unwrap_or_else(|refusal| panic!("the bytes verify: {refusal:#?}"));

    assert_eq!(verified, bundle.loaded());
}

#[test]
fn verifying_already_read_bytes_refuses_a_missing_or_mismatched_member_at_its_entry() {
    let bundle = Bundle::new();
    let manifest = std::fs::read(bundle.manifest()).unwrap();
    let verify = |members: &BTreeMap<String, Arc<[u8]>>| {
        *verify_read(&bundle.manifest(), &manifest, &bundle.context(), members).expect_err("the bytes are refused")
    };

    for (path, field, reference) in [("schemas/b.yaml", "files[1]", B), ("functions/project.facts.json", "files[2]", FACTS)] {
        let mut missing = bundle.files_on_disk();
        missing.remove(path);
        let refusal = verify(&missing);
        assert_refusal_in(&refusal, &bundle.manifest());
        assert_eq!(refusal.member.as_deref(), Some(field), "{path}");
        assert_eq!(refusal.message, format!("`{reference}` is missing from the bundle"));

        let mut changed = bundle.files_on_disk();
        changed.insert(path.to_string(), Arc::from(&b"# edited\n"[..]));
        let refusal = verify(&changed);
        assert_refusal_in(&refusal, &bundle.manifest());
        assert_eq!(refusal.member.as_deref(), Some(field), "{path}");
        assert!(refusal.message.starts_with(&format!("`{reference}` does not match its digest")), "{}", refusal.message);
    }
}

#[test]
fn an_embedded_bundle_verifies_the_same_members_and_revision() {
    let bundle = Bundle::new();
    let installed = bundle.loaded();
    let embedded = bundle.embedded().unwrap_or_else(|refusal| panic!("embedded: {refusal:#?}"));
    assert_eq!(embedded.revision(), installed.revision());
    assert_eq!(embedded.bytes(), installed.bytes());
    let paths = |snapshot: &VerifiedSnapshot| -> Vec<(String, String)> {
        snapshot.companions().iter().map(|pair| (pair.catalog.path.clone(), pair.facts.path.clone())).collect()
    };
    assert_eq!(paths(&embedded), paths(&installed));
}

#[test]
fn loading_one_bundle_twice_gives_equal_snapshots() {
    let bundle = Bundle::new();
    assert_eq!(bundle.loaded(), bundle.loaded());
}

// ── revision ────────────────────────────────────────────────────────────────

#[test]
fn reordering_files_keeps_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    bundle.publish(&Members::with_files(&[CATALOG, B, A]));
    let after = bundle.loaded();
    assert_ne!(before.manifest_hash(), after.manifest_hash(), "the manifest bytes changed");
    assert_eq!(before.revision(), after.revision());
    assert_eq!(after.sorted_files(), before.sorted_files());
    assert_eq!(after.companions()[0].facts.path, before.companions()[0].facts.path);
}

#[test]
fn a_copied_bundle_keeps_the_revision() {
    let bundle = Bundle::new();
    let copy = bundle.root.join("elsewhere/deeper/host-copy");
    let mut relatives: Vec<String> = bundle.files_on_disk().into_keys().collect();
    relatives.push("manifest.yaml".to_string());
    for relative in relatives {
        let target = copy.join(&relative);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(bundle.host().join(&relative), target).unwrap();
    }
    let copied = load_snapshot(&copy.join("manifest.yaml"), &bundle.context()).unwrap();
    let original = bundle.loaded();
    assert_eq!(copied.revision(), original.revision());
    assert_eq!(copied.bundle_root(), copy);
    assert_eq!(copied.companions()[0].facts.location, copy.join("functions/project.facts.json"));
}

#[test]
fn changing_one_member_byte_changes_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    for member in ["schemas/b.yaml", "functions/project.facts.json"] {
        bundle.write(member, &format!("{{ \"changed\": \"{member}\" }}\n"));
        bundle.publish(&Members::default());
        let after = bundle.loaded();
        assert_ne!(after.revision(), before.revision(), "{member}");
    }
}

#[test]
fn a_companion_digest_alone_changes_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    bundle.write("functions/project.facts.json", "{ \"facts\": 2 }\n");
    bundle.publish(&Members::default());
    let after = bundle.loaded();
    assert_eq!(after.files(), before.files(), "every listed member is unchanged");
    assert_eq!(after.lifecycle(), before.lifecycle());
    assert_ne!(after.revision(), before.revision());
}

#[test]
fn a_companion_path_changes_the_revision() {
    let bundle = Bundle::new();
    let before = bundle.loaded();
    // The same catalog and facts bytes under a name whose companion differs.
    let catalog = std::fs::read(bundle.host().join("functions/project.yaml")).unwrap();
    let facts = std::fs::read(bundle.host().join("functions/project.facts.json")).unwrap();
    std::fs::write(bundle.host().join("functions/project.v2.yaml"), catalog).unwrap();
    std::fs::write(bundle.host().join("functions/project.v2.facts.json"), facts).unwrap();
    bundle.publish(&Members::with_files(&[A, B, "./functions/project.v2.yaml"]).companions(&["./functions/project.v2.facts.json"]));
    let after = bundle.loaded();
    assert_eq!(after.companions()[0].facts.path, "functions/project.v2.facts.json");
    assert_eq!(after.companions()[0].facts.digest, before.companions()[0].facts.digest);
    assert_ne!(after.revision(), before.revision());
}

// ── format version, checked before the shape ────────────────────────────────

#[test]
fn an_unsupported_integer_format_is_refused_before_anything_else() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_on_disk();
    // Members that are not there prove nothing was probed.
    std::fs::remove_file(bundle.host().join("schemas/a.yaml")).unwrap();
    for version in [1, 2, 4] {
        let text = valid.replace("format_version: 3\n", &format!("format_version: {version}\n"));
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.message, unsupported(version), "{version}");
        assert_eq!((refusal.line, refusal.column), (Some(line_of(&text, "format_version:")), Some(17)), "{version}: at the value");
        assert_eq!(refusal.member, None, "{version}");
        assert_eq!(refusal.unsupported_format, Some(version), "{version}");
        assert_eq!(refusal.candidates, [bundle.manifest()], "{version}: no member was looked for");
    }
}

#[test]
fn a_version_less_manifest_with_a_format_1_field_is_format_1_at_that_field() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_on_disk();
    let text = valid.replace("format_version: 3\n", "identify: ./schemas/a.yaml\n");
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.message, unsupported(1));
    assert_eq!(refusal.unsupported_format, Some(1));
    assert_eq!(refusal.line, Some(line_of(&text, "identify:")));
    assert_eq!(refusal.candidates, [bundle.manifest()], "no member was looked for");
}

#[test]
fn a_format_1_manifest_is_unsupported() {
    let bundle = Bundle::new();
    let legacy = "kind: darkmatter-host\nformat_version: 1\nhost: acme\nversion: 1.2.0\nidentify: ./schemas/a.yaml\n\
                  triggers: []\nglobals: ./schemas/b.yaml\nlifecycle: ./globals.lifecycle.json\nfunctions: []\n";
    bundle.write_manifest(legacy);
    let refusal = bundle.refused();
    assert_eq!(refusal.message, unsupported(1));
    assert_eq!(refusal.unsupported_format, Some(1));
    assert_eq!(refusal.line, Some(2), "at `format_version`");
}

#[test]
fn a_wrong_typed_format_version_is_judged_by_the_contract() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_on_disk();
    for value in ["\"3\"", "3.0"] {
        let text = valid.replace("format_version: 3\n", &format!("format_version: {value}\n"));
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.unsupported_format, None, "{value}: {refusal:#?}");
        assert_eq!(refusal.line, Some(line_of(&text, "format_version:")), "{value}: {refusal:#?}");
        assert!(!refusal.message.contains("unsupported host registration"), "{value}: {}", refusal.message);
    }
    bundle.write_manifest(&replace_field(&valid, "format_version", ""));
    let refusal = bundle.refused();
    assert_eq!(refusal.unsupported_format, None);
    assert!(refusal.message.contains("`format_version` is required"), "{}", refusal.message);
}

// ── the closed envelope ─────────────────────────────────────────────────────

#[test]
fn every_field_is_required() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for field in ["kind", "format_version", "host", "version", "files", "lifecycle", "digests"] {
        bundle.write_manifest(&replace_field(&valid, field, ""));
        let refusal = bundle.refused();
        assert_eq!(refusal.unsupported_format, None, "{field}");
        assert!(refusal.message.contains(&format!("`{field}` is required")), "{field}: {}", refusal.message);
    }
}

#[test]
fn functions_is_an_unknown_field_even_when_empty() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for functions in ["functions: []\n", &format!("functions:\n    - catalog: '{CATALOG}'\n      facts: '{FACTS}'\n")] {
        let text = replace_field(&valid, "lifecycle", &format!("lifecycle: '{LIFECYCLE}'\n{functions}"));
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.unsupported_format, None);
        assert!(refusal.message.contains("`functions` is not a manifest field"), "{functions:?}: {}", refusal.message);
        assert_eq!(refusal.line, Some(line_of(&text, "functions:")), "{functions:?}: {refusal:#?}");
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
fn a_format_1_field_beside_format_3_is_an_unknown_field() {
    let bundle = Bundle::new();
    bundle.write_manifest(&(bundle.manifest_text(&Members::default()) + "identify: ./schemas/a.yaml\n"));
    let refusal = bundle.refused();
    assert_eq!(refusal.unsupported_format, None);
    assert!(refusal.message.contains("`identify` is not a manifest field"), "{}", refusal.message);
}

#[test]
fn an_empty_files_list_is_accepted_with_the_lifecycle_digest_still_required() {
    let bundle = Bundle::new();
    let empty = Members::with_files(&[]).companions(&[]);
    bundle.publish(&empty);
    let snapshot = bundle.loaded();
    assert!(snapshot.files().is_empty() && snapshot.companions().is_empty());
    assert_eq!(snapshot.bytes().len(), 1, "only the lifecycle artifact");

    let text = replace_field(&bundle.manifest_text(&empty), "digests", "digests: {}\n");
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("lifecycle"));
    assert_eq!(refusal.line, Some(line_of(&text, "lifecycle:")));
    assert!(refusal.message.contains("no entry in `digests`"), "{}", refusal.message);
}

#[test]
fn files_must_be_a_list_of_references() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let cases = [
        ("files: null\n", "files:"),
        ("files: ./schemas/a.yaml\n", "files:"),
        ("files:\n    - 7\n", "- 7"),
        ("files:\n    - \"\"\n", "- \"\""),
    ];
    for (replacement, at) in cases {
        let text = replace_field(&valid, "files", replacement);
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
    for (reference, field) in [(B, "files[1]"), (CATALOG, "files[2]"), (LIFECYCLE, "lifecycle")] {
        let text = without_digest(&valid, reference);
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.member.as_deref(), Some(field), "{reference}");
        assert_eq!(refusal.message, format!("`{reference}` has no entry in `digests`; every member lists its digest"));
    }
    let text = without_digest(&valid, B);
    bundle.write_manifest(&text);
    assert_eq!(bundle.refused().line, Some(files_line(&text, B)));
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
fn digests_is_a_map_of_strings() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let a_line = digest_line(&valid, A);
    let cases = [
        ("digests null", replace_field(&valid, "digests", "digests: null\n"), "digests:"),
        ("digests a list", replace_field(&valid, "digests", "digests: []\n"), "digests:"),
        ("digests a string", replace_field(&valid, "digests", "digests: abc\n"), "digests:"),
        ("a null digest", valid.replace(&a_line, &format!("    '{A}': null")), "'./schemas/a.yaml':"),
        ("a number digest", valid.replace(&a_line, &format!("    '{A}': 7")), "'./schemas/a.yaml':"),
        ("a list digest", valid.replace(&a_line, &format!("    '{A}': []")), "'./schemas/a.yaml':"),
    ];
    for (case, text, at) in cases {
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.unsupported_format, None, "{case}");
        assert_eq!(refusal.line, Some(line_of(&text, at)), "{case}: {refusal:#?}");
    }
}

#[test]
fn a_digest_is_64_lowercase_hex_digits() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    for reference in [A, FACTS] {
        let digest = digest_of(&bundle.host(), reference);
        for bad in [digest.to_uppercase(), digest[..63].to_string(), format!("{}g", &digest[..63])] {
            let text = valid.replace(&digest, &bad);
            bundle.write_manifest(&text);
            let refusal = bundle.refused();
            assert_eq!(refusal.line, Some(line_of(&text, &format!("'{reference}':"))), "{reference} {bad}: {refusal:#?}");
        }
    }
}

#[test]
fn a_valid_digest_of_other_bytes_is_a_mismatch_at_the_entry() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let text = valid.replace(&digest_of(&bundle.host(), A), &digest_of(&bundle.host(), B));
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[0]"));
    assert_eq!(refusal.line, Some(files_line(&text, A)));
    assert!(refusal.message.starts_with(&format!("`{A}` does not match its digest")), "{}", refusal.message);
}

// ── member paths ────────────────────────────────────────────────────────────

/// Publishes the default members with `files[1]` replaced by `member`, and
/// returns the refusal, which must be located at that entry.
fn refused_member(bundle: &Bundle, member: &str) -> SnapshotRefusal {
    let members = Members::with_files(&[A, member, CATALOG]);
    let text = bundle.manifest_text(&members);
    bundle.write_manifest(&text);
    let refusal = *bundle.load().expect_err(member);
    assert_refusal_in(&refusal, &bundle.manifest());
    assert_eq!(refusal.member.as_deref(), Some("files[1]"), "{member}: {refusal:#?}");
    assert_eq!(refusal.line, Some(files_line(&text, member)), "{member}");
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
    bundle.publish(&Members::with_files(&["./schemas/console.yaml", "./schemas/com10.yaml", CATALOG]));
    bundle.loaded();
}

#[test]
fn in_bundle_dot_traversal_is_resolved() {
    let bundle = Bundle::new();
    bundle.publish(&Members::with_files(&["./schemas/../schemas/a.yaml", "../host/schemas/b.yaml", CATALOG]));
    let snapshot = bundle.loaded();
    let files: Vec<&str> = snapshot.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(files, ["schemas/a.yaml", "schemas/b.yaml", "functions/project.yaml"]);
    let default = {
        bundle.publish(&Members::default());
        bundle.loaded()
    };
    assert_eq!(snapshot.revision(), default.revision(), "the revision reads normalized paths");
}

#[test]
fn a_reentered_catalog_derives_its_companion_with_the_authored_prefix() {
    let bundle = Bundle::new();
    let catalog = "../host/functions/project.yaml";
    let facts = "../host/functions/project.facts.json";
    bundle.publish(&Members::with_files(&[A, B, catalog]).companions(&[facts]));
    let snapshot = bundle.loaded();
    let [pair] = snapshot.companions() else { panic!("one companion: {:?}", snapshot.companions()) };
    assert_eq!((pair.facts.reference.as_str(), pair.facts.path.as_str()), (facts, "functions/project.facts.json"));
    bundle.publish(&Members::default());
    assert_eq!(snapshot.revision(), bundle.loaded().revision(), "the revision reads normalized paths");
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
    bundle.publish(&Members::with_files(&[A, "./linked/c.yaml", CATALOG]));
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("`linked`") && refusal.message.contains("junction"), "{}", refusal.message);
}

// ── derived companions ──────────────────────────────────────────────────────

#[test]
fn a_yml_catalog_derives_the_same_companion_name() {
    let bundle = Bundle::new();
    bundle.write_catalog("functions/project.yml");
    bundle.publish(&Members::with_files(&[A, B, "./functions/project.yml"]));
    let snapshot = bundle.loaded();
    let [pair] = snapshot.companions() else { panic!("one companion: {:?}", snapshot.companions()) };
    assert_eq!((pair.catalog.path.as_str(), pair.facts.path.as_str()), ("functions/project.yml", "functions/project.facts.json"));
    assert_eq!(pair.facts.reference, FACTS);
}

#[test]
fn a_catalog_name_keeps_every_earlier_dot() {
    let bundle = Bundle::new();
    bundle.write_catalog("functions/a.b.yaml");
    bundle.write("functions/a.b.facts.json", "{}\n");
    bundle.publish(&Members::with_files(&[A, B, "./functions/a.b.yaml"]).companions(&["./functions/a.b.facts.json"]));
    let snapshot = bundle.loaded();
    let [pair] = snapshot.companions() else { panic!("one companion: {:?}", snapshot.companions()) };
    assert_eq!((pair.facts.reference.as_str(), pair.facts.path.as_str()), ("./functions/a.b.facts.json", "functions/a.b.facts.json"));
    assert_eq!(snapshot.member_bytes("functions/a.b.facts.json").map(|bytes| &bytes[..]), Some(&b"{}\n"[..]));
}

#[test]
fn a_catalog_without_a_yaml_extension_is_refused_at_its_entry() {
    let bundle = Bundle::new();
    for catalog in ["./functions/project.json", "./functions/project"] {
        bundle.write(catalog, "{ \"kind\": \"expression-schema\" }\n");
        let text = bundle.manifest_text(&Members::with_files(&[A, B, catalog]).companions(&[]));
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.member.as_deref(), Some("files[2]"), "{catalog}");
        assert_eq!(refusal.line, Some(files_line(&text, catalog)), "{catalog}");
        assert!(refusal.message.contains("must end in `.yaml` or `.yml`"), "{catalog}: {}", refusal.message);
        assert_embedded_parity(&bundle);
    }
}

#[test]
fn a_same_stem_yaml_and_yml_catalog_pair_is_refused() {
    let bundle = Bundle::new();
    bundle.write_catalog("functions/project.yml");
    let text = bundle.manifest_text(&Members::with_files(&[A, B, CATALOG, "./functions/project.yml"]));
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[3]"));
    assert_eq!(refusal.line, Some(files_line(&text, "./functions/project.yml")));
    assert!(refusal.message.contains(&format!("`{FACTS}`")), "{}", refusal.message);
    assert!(refusal.message.contains("no two catalogs may share one"), "{}", refusal.message);
    assert_embedded_parity(&bundle);
}

#[test]
fn a_companion_listed_in_files_is_refused() {
    let bundle = Bundle::new();
    for (files, catalog_field) in [([A, CATALOG, FACTS], "files[1]"), ([A, FACTS, CATALOG], "files[2]")] {
        let text = bundle.manifest_text(&Members::with_files(&files).companions(&[]));
        bundle.write_manifest(&text);
        let refusal = bundle.refused();
        assert_eq!(refusal.member.as_deref(), Some(catalog_field), "{files:?}");
        assert_eq!(refusal.line, Some(files_line(&text, CATALOG)), "{files:?}");
        assert!(refusal.message.contains("never listed in `files`"), "{files:?}: {}", refusal.message);
        assert_embedded_parity(&bundle);
    }
}

#[test]
fn a_missing_companion_digest_is_refused_at_the_catalog_without_a_retry() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_on_disk();
    let text = without_digest(&valid, FACTS);
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert_eq!(refusal.line, Some(files_line(&text, CATALOG)));
    assert_eq!(
        refusal.message,
        format!("`{CATALOG}` is an expression catalog, so `digests` must list its facts companion `{FACTS}`")
    );
    assert_eq!(refusal.unsupported_format, None);
    assert_embedded_parity(&bundle);

    // Not a publish in progress: a changed manifest is not read again.
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![text, valid], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 1, "the manifest is read once");
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
}

#[test]
fn a_missing_catalog_digest_is_refused_at_the_catalog() {
    let bundle = Bundle::new();
    let text = without_digest(&bundle.manifest_on_disk(), CATALOG);
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert_eq!(refusal.message, format!("`{CATALOG}` has no entry in `digests`; every member lists its digest"));
}

#[test]
fn a_missing_companion_file_is_refused_watched_and_retried_only_over_a_changed_manifest() {
    let bundle = Bundle::new();
    let one = bundle.manifest_on_disk();
    std::fs::remove_file(bundle.host().join("functions/project.facts.json")).unwrap();
    let facts = bundle.host().join("functions/project.facts.json");

    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert_eq!(refusal.line, Some(files_line(&one, CATALOG)));
    assert_eq!(refusal.message, format!("`{FACTS}` is missing from the bundle"));
    assert!(refusal.candidates.contains(&facts), "the companion is watched: {:?}", refusal.candidates);

    // An unchanged manifest is read once more, then refused; the companion
    // stays a candidate.
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one.clone()], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 2, "read again once, then refused");
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert!(refusal.candidates.contains(&facts), "{:?}", refusal.candidates);

    // A changed manifest that still names it: one retry only.
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one.clone(), format!("{one}# republished\n")], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 2, "one retry only");
    assert_eq!(refusal.message, format!("`{FACTS}` is missing from the bundle"));

    // A changed manifest that no longer names the catalog loads alone.
    let two = bundle.manifest_text(&Members::with_files(&[A, B]).companions(&[]));
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one, two.clone()], &count);
    let snapshot = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap();
    assert_eq!(count.get(), 2);
    assert_eq!(snapshot.manifest_hash(), content_hash(two.as_bytes()));
    assert!(snapshot.companions().is_empty());
    assert!(snapshot.candidates().contains(&facts), "the first attempt's candidates are kept");

    // The embedded reader refuses it at the same entry, in its own words.
    bundle.write_manifest(&bundle.manifest_text(&Members::default()));
    let read = *bundle.load().unwrap_err();
    let embedded = *bundle.embedded().unwrap_err();
    assert_eq!((&embedded.member, embedded.line, embedded.column), (&read.member, read.line, read.column));
    assert_eq!(embedded.message, format!("`{FACTS}` is missing from the embedded bundle"));
}

#[test]
fn a_mismatched_companion_is_refused_at_its_catalog() {
    let bundle = Bundle::new();
    bundle.write("functions/project.facts.json", "{ \"edited\": true }\n");
    let refusal = bundle.refused();
    let text = bundle.manifest_on_disk();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert_eq!(refusal.line, Some(files_line(&text, CATALOG)));
    assert!(refusal.message.starts_with(&format!("`{FACTS}` does not match its digest")), "{}", refusal.message);
    assert!(refusal.dependencies.iter().any(|dependency| dependency.path == bundle.host().join("functions/project.facts.json")));

    let embedded = *bundle.embedded().unwrap_err();
    assert_eq!((&embedded.member, embedded.line, embedded.column), (&refusal.member, refusal.line, refusal.column));
    assert!(embedded.message.starts_with(&format!("`{FACTS}` does not match its digest")), "{}", embedded.message);
}

#[cfg(unix)]
#[test]
fn a_companion_reached_through_a_symbolic_link_is_refused() {
    let bundle = Bundle::new();
    let facts = bundle.host().join("functions/project.facts.json");
    std::fs::rename(&facts, bundle.host().join("real.facts.json")).unwrap();
    std::os::unix::fs::symlink(bundle.host().join("real.facts.json"), &facts).unwrap();
    bundle.publish(&Members::default());
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"));
    assert!(
        refusal.message.contains("passes through `functions/project.facts.json`") && refusal.message.contains("symbolic link"),
        "{}",
        refusal.message
    );
    assert!(!refusal.dependencies.iter().any(|dependency| dependency.path == facts), "never read");
}

#[test]
fn a_companion_differing_from_a_member_only_in_case_is_refused() {
    let bundle = Bundle::new();
    let upper = "./functions/Project.facts.json";
    // On a file system that ignores case this rewrites the companion too;
    // the refusal comes before any companion read either way.
    bundle.write(upper, "kind: schema\n");
    let text = bundle.manifest_text(&Members::with_files(&[A, B, CATALOG, upper]));
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[2]"), "at the catalog that derives it");
    assert!(
        refusal.message.contains("`functions/project.facts.json` differs only in letter case from `functions/Project.facts.json`"),
        "{}",
        refusal.message
    );
    // A file system that ignores case holds one file for both names.
    let mut members = bundle.files_on_disk();
    members.insert("functions/Project.facts.json".to_string(), Arc::from(&b"kind: schema\n"[..]));
    assert_embedded_parity_with(&bundle, &members);
}

#[test]
fn a_companion_shaped_digest_for_a_non_catalog_is_refused_at_its_key() {
    let bundle = Bundle::new();
    let stray = "./schemas/a.facts.json";
    bundle.write(stray, "{}\n");
    let text = bundle.manifest_text(&Members::default().companions(&[FACTS, stray]));
    bundle.write_manifest(&text);
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("digests../schemas/a.facts.json"));
    assert_eq!(refusal.line, Some(line_of(&text, &format!("'{stray}':"))));
    assert!(refusal.message.contains("no member entry names and no expression catalog derives"), "{}", refusal.message);
    assert_embedded_parity(&bundle);
}

#[test]
fn a_mutation_catalog_derives_no_companion_at_the_snapshot() {
    let bundle = Bundle::new();
    let mutations = "./functions/actions.yaml";
    bundle.write(mutations, "kind: mutation-schema\n");
    bundle.publish(&Members::with_files(&[A, B, CATALOG, mutations]));
    let snapshot = bundle.loaded();
    let catalogs: Vec<&str> = snapshot.companions().iter().map(|pair| pair.catalog.path.as_str()).collect();
    assert_eq!(catalogs, ["functions/project.yaml"], "its kind is judged when it is prepared, not here");
    assert!(snapshot.member_bytes("functions/actions.yaml").is_some());

    // So a companion-shaped digest for it names no member.
    let stray = "./functions/actions.facts.json";
    bundle.write(stray, "{}\n");
    bundle.publish(&Members::with_files(&[A, B, CATALOG, mutations]).companions(&[FACTS, stray]));
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("digests../functions/actions.facts.json"));
}

// ── watches ─────────────────────────────────────────────────────────────────

#[test]
fn a_missing_member_is_refused_with_every_named_companion_watched() {
    let bundle = Bundle::new();
    bundle.write_catalog("functions/other.yaml");
    bundle.write("functions/other.facts.json", "{}\n");
    bundle.publish(&Members::with_files(&[A, B, CATALOG, "./functions/other.yaml"]).companions(&[FACTS, "./functions/other.facts.json"]));
    std::fs::remove_file(bundle.host().join("schemas/a.yaml")).unwrap();
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[0]"));
    assert_eq!(refusal.message, format!("`{A}` is missing from the bundle"));
    for watched in ["schemas/a.yaml", "schemas/b.yaml", "globals.lifecycle.json", "functions/project.facts.json", "functions/other.facts.json"] {
        assert!(refusal.candidates.contains(&bundle.host().join(watched)), "{watched}: {:?}", refusal.candidates);
    }
    assert!(
        !refusal.dependencies.iter().any(|dependency| dependency.path.ends_with("project.facts.json")),
        "watched, not read"
    );
}

// ── digest verification and the retry ──────────────────────────────────────

#[test]
fn a_mismatched_member_under_an_unchanged_manifest_is_refused_at_its_entry() {
    let bundle = Bundle::new();
    bundle.write("schemas/b.yaml", "kind: schema\n# edited\n");
    let refusal = bundle.refused();
    let text = bundle.manifest_on_disk();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert_eq!(refusal.line, Some(files_line(&text, B)));
    assert!(refusal.message.contains("does not match its digest"), "{}", refusal.message);
    assert!(refusal.dependencies.iter().any(|dependency| dependency.path == bundle.host().join("schemas/b.yaml")));
}

#[test]
fn a_missing_member_is_refused_and_watched() {
    let bundle = Bundle::new();
    std::fs::remove_file(bundle.host().join("schemas/b.yaml")).unwrap();
    let refusal = bundle.refused();
    assert_eq!(refusal.member.as_deref(), Some("files[1]"));
    assert!(refusal.message.contains("missing"), "{}", refusal.message);
    assert!(refusal.candidates.contains(&bundle.host().join("schemas/b.yaml")), "{:?}", refusal.candidates);
    assert!(refusal.candidates.contains(&bundle.host().join("functions/project.facts.json")), "every member is watched");
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
    for member in snapshot.members() {
        assert_eq!(member.digest, digest_of(&bundle.host(), &member.reference), "release two's digest");
        let bytes = snapshot.member_bytes(&member.path).unwrap();
        assert!(String::from_utf8_lossy(bytes).contains("release") && String::from_utf8_lossy(bytes).contains("two"));
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
    // Mid-publish: every member is release two's but the companion, still
    // release one's.
    bundle.write(FACTS, &format!("{{ \"member\": \"{FACTS}\", \"release\": \"one\" }}\n"));
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one.clone(), two.clone()], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert_eq!(count.get(), 2, "one retry only");
    assert_eq!(refusal.member.as_deref(), Some("files[2]"), "release two's check of the companion: {refusal:#?}");
    assert!(refusal.message.starts_with(&format!("`{FACTS}` does not match")), "{}", refusal.message);

    // Each release alone, against the same mixed files, is refused too.
    for text in [one, two] {
        bundle.write_manifest(&text);
        bundle.refused();
    }
}

#[test]
fn a_retry_keeps_both_attempts_inputs() {
    let (bundle, one, _) = two_releases();
    let other = Members::with_files(&[A, "./schemas/c.yaml", CATALOG]);
    bundle.write("schemas/c.yaml", "# c\n");
    let two = bundle.manifest_text(&other);
    std::fs::remove_file(bundle.host().join("schemas/c.yaml")).unwrap();
    let count = Cell::new(0);
    let reader = sequenced(bundle.manifest(), vec![one, two], &count);
    let refusal = load_snapshot_with(&bundle.manifest(), &bundle.context(), &reader).unwrap_err();
    assert!(refusal.message.contains("missing"), "{}", refusal.message);
    for watched in ["schemas/b.yaml", "schemas/c.yaml", "functions/project.facts.json"] {
        assert!(refusal.candidates.contains(&bundle.host().join(watched)), "{watched}: {:?}", refusal.candidates);
    }
}

// ── admission without member reads ──────────────────────────────────────────

#[test]
fn admission_lists_the_members_without_reading_them() {
    let bundle = Bundle::new();
    bundle.write("schemas/a.yaml", "# changed after publication\n");
    std::fs::remove_file(bundle.host().join("schemas/b.yaml")).unwrap();
    std::fs::remove_file(bundle.host().join("functions/project.facts.json")).unwrap();
    bundle.load().expect_err("the verifying reader refuses the mismatch");

    let admitted = bundle.admit().unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    assert_eq!((admitted.host(), admitted.version(), admitted.format_version()), ("acme", "1.2.0", 3));
    let paths: Vec<&str> = admitted.members().map(|member| member.path.as_str()).collect();
    assert_eq!(
        paths,
        ["schemas/a.yaml", "schemas/b.yaml", "functions/project.yaml", "globals.lifecycle.json", "functions/project.facts.json"]
    );
}

/// One edit per shared validator (format version, envelope, digest map,
/// YAML document, member path): admission refuses exactly where the
/// verifying reader does, located at the same place.
#[test]
fn admission_refuses_what_the_verifying_reader_refuses() {
    let bundle = Bundle::new();
    let valid = bundle.manifest_text(&Members::default());
    let a_line = digest_line(&valid, A);
    let a_digest = digest_of(&bundle.host(), A);
    let cases = [
        ("version empty", valid.replace("version: \"1.2.0\"", "version: \"\"")),
        ("version omitted", replace_field(&valid, "version", "")),
        ("format 1", valid.replace("format_version: 3", "format_version: 1")),
        ("format 4", valid.replace("format_version: 3", "format_version: 4")),
        ("format string", valid.replace("format_version: 3", "format_version: \"3\"")),
        ("format absent", replace_field(&valid, "format_version", "")),
        ("functions empty", replace_field(&valid, "lifecycle", &format!("lifecycle: '{LIFECYCLE}'\nfunctions: []\n"))),
        ("digests null", replace_field(&valid, "digests", "digests: null\n")),
        ("digest nonhex", valid.replace(&a_line, &format!("    '{A}': \"{}\"", "z".repeat(64)))),
        ("digest uppercase", valid.replace(&a_digest, &a_digest.to_uppercase())),
        ("digest missing", without_digest(&valid, B)),
        ("stray digest", format!("{valid}    './schemas/c.yaml': \"{}\"\n", "a".repeat(64))),
        ("duplicate digest key", format!("{valid}{a_line}\n")),
        ("second document", format!("{valid}---\nkind: other\n")),
        ("unprefixed member", bundle.manifest_text(&Members::with_files(&[A, "schemas/b.yaml", CATALOG]))),
        ("escaping member", bundle.manifest_text(&Members::with_files(&[A, "../x.yaml", CATALOG]))),
    ];
    for (case, text) in cases {
        bundle.write_manifest(&text);
        let read = *bundle.load().expect_err(case);
        let admitted = *bundle.admit().expect_err(case);
        assert_eq!(
            (admitted.message, admitted.line, admitted.column, admitted.member, admitted.unsupported_format),
            (read.message, read.line, read.column, read.member, read.unsupported_format),
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
    bundle.publish(&Members::with_files(&[A, "../host/schemas/b.yaml", CATALOG]));
    bundle.loaded();

    let refusal = *bundle.admit().expect_err("lexically, `..` leaves the bundle");

    assert_eq!(refusal.member.as_deref(), Some("files[1]"), "{refusal:#?}");
    assert!(refusal.message.contains("leaves the bundle"), "{}", refusal.message);
}

/// Installed admission resolves every member category (`files`, an
/// expression catalog in them, `lifecycle`, and the catalog's companion) as
/// the verifying reader does: a spelling that climbs out and back in is the
/// in-bundle member, and one that ends outside the bundle is refused at its
/// entry, where the reader refuses it.
#[test]
fn installed_admission_resolves_every_member_category_as_the_reader_does() {
    let bundle = Bundle::new();
    let reentered = Members {
        files: vec![A.to_string(), "../host/schemas/b.yaml".to_string(), "../host/functions/project.yaml".to_string()],
        lifecycle: "../host/globals.lifecycle.json".to_string(),
        companions: vec!["../host/functions/project.facts.json".to_string()],
    };
    bundle.publish(&reentered);
    let bytes = std::fs::read(bundle.manifest()).unwrap();
    let loaded = bundle.loaded();

    let admitted = admit_installed_manifest(&bundle.manifest(), &bytes, &bundle.context())
        .unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    let paths: Vec<&str> = admitted.members().map(|member| member.path.as_str()).collect();
    assert_eq!(
        paths,
        ["schemas/a.yaml", "schemas/b.yaml", "functions/project.yaml", "globals.lifecycle.json", "functions/project.facts.json"]
    );
    let read: Vec<&str> = loaded.members().map(|member| member.path.as_str()).collect();
    assert_eq!(read, paths, "the reader names the same members");

    for field in ["files[1]", "files[2]", "lifecycle"] {
        let mut escaping = Members::default();
        match field {
            "files[1]" => escaping.files[1] = "../schemas/b.yaml".to_string(),
            "files[2]" => {
                escaping.files[2] = "../functions/project.yaml".to_string();
                escaping.companions = vec!["../functions/project.facts.json".to_string()];
            }
            _ => escaping.lifecycle = "../globals.lifecycle.json".to_string(),
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

// ── the frozen format 2 manifest ────────────────────────────────────────────

/// A request context with an empty home and working directory.
fn empty_context(dir: &Path) -> FileResolutionContext {
    std::fs::create_dir_all(dir.join("home")).unwrap();
    std::fs::create_dir_all(dir.join("work")).unwrap();
    FileResolutionContext::from_snapshot(dir.join("work"), Some(dir.join("home")), HashMap::new())
}

#[test]
fn a_frozen_format_2_manifest_is_admitted_for_cleanup() {
    let manifest = biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_registration_v2/darkmatter/manifest.yaml");
    let bytes = std::fs::read(&manifest).unwrap();

    let admitted = admit_manifest(&manifest, &bytes).unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    assert_eq!((admitted.host(), admitted.version(), admitted.format_version()), ("claudine", "0.1.0", 2));
    let text = String::from_utf8(bytes).unwrap();
    let mut expected: Vec<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- \"./").map(|rest| rest.trim_end_matches('"').to_string()))
        .collect();
    assert_eq!(expected.len(), 15, "the fixture lists fifteen files");
    expected.push("globals.lifecycle.json".to_string());
    let paths: Vec<&str> = admitted.members().map(|member| member.path.as_str()).collect();
    assert_eq!(paths, expected, "its files, then its lifecycle; it has no function pairs");
}

#[test]
fn a_frozen_format_2_manifest_is_never_loaded() {
    let manifest = biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_registration_v2/darkmatter/manifest.yaml");
    let bytes = std::fs::read(&manifest).unwrap();
    let line = line_of(&String::from_utf8(bytes.clone()).unwrap(), "format_version:");
    let dir = tempfile::tempdir().unwrap();
    let context = empty_context(dir.path());
    let bundle_dir = manifest.parent().unwrap();
    let members: BTreeMap<String, Arc<[u8]>> = ["globals.lifecycle.json", "schemas/action.yaml"]
        .into_iter()
        .map(|path| (path.to_string(), Arc::from(std::fs::read(bundle_dir.join(path)).unwrap())))
        .collect();

    let refusals = [
        ("installed admission", *admit_installed_manifest(&manifest, &bytes, &context).unwrap_err()),
        ("load", *load_snapshot(&manifest, &context).unwrap_err()),
        ("verify_read", *verify_read(&manifest, &bytes, &context, &members).unwrap_err()),
        ("embedded", *VerifiedSnapshot::from_embedded("claudine", "manifest.yaml", &bytes, &members).unwrap_err()),
    ];
    for (reader, refusal) in refusals {
        assert_eq!(refusal.message, unsupported(2), "{reader}");
        assert_eq!(refusal.unsupported_format, Some(2), "{reader}");
        assert_eq!(refusal.line, Some(line), "{reader}: at `format_version`");
        assert_eq!(refusal.member, None, "{reader}");
    }
    let loaded = *load_snapshot(&manifest, &context).unwrap_err();
    assert_eq!(loaded.candidates, [manifest], "no member was looked for");
}

#[test]
fn a_format_2_manifest_with_function_pairs_admits_its_pair_members() {
    let bundle = Bundle::new();
    let host = bundle.host();
    let mut text = String::from("kind: darkmatter-host\nformat_version: 2\nhost: acme\nversion: \"1.2.0\"\n");
    text.push_str(&format!("files:\n    - '{A}'\n    - '{B}'\nlifecycle: '{LIFECYCLE}'\n"));
    text.push_str(&format!("functions:\n    - catalog: '{CATALOG}'\n      facts: '{FACTS}'\ndigests:\n"));
    for member in [A, B, LIFECYCLE, CATALOG, FACTS] {
        text.push_str(&format!("    '{member}': \"{}\"\n", digest_of(&host, member)));
    }
    bundle.write_manifest(&text);

    let admitted = bundle.admit().unwrap_or_else(|refusal| panic!("admitted: {refusal:#?}"));

    assert_eq!(admitted.format_version(), 2);
    let members: Vec<(&str, &str)> = admitted.members().map(|member| (member.field.as_str(), member.path.as_str())).collect();
    assert_eq!(
        members,
        [
            ("files[0]", "schemas/a.yaml"),
            ("files[1]", "schemas/b.yaml"),
            ("lifecycle", "globals.lifecycle.json"),
            ("functions[0].catalog", "functions/project.yaml"),
            ("functions[0].facts", "functions/project.facts.json"),
        ]
    );
    let refusal = bundle.refused();
    assert_eq!(refusal.unsupported_format, Some(2), "the loader refuses it");
}

#[test]
fn a_malformed_format_2_manifest_confers_no_cleanup_authority() {
    let manifest = biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_registration_v2/darkmatter/manifest.yaml");
    let valid = std::fs::read_to_string(&manifest).unwrap();
    let digest = "73347213b879e94fe6fc409e460f322fd03825d329e82d43b8163b35d367901a";
    assert!(valid.contains(digest), "the fixture's lifecycle digest");
    let cases = [
        ("an unknown field", format!("{valid}extra: true\n")),
        ("an uppercase digest", valid.replace(digest, &digest.to_uppercase())),
        ("a short digest", valid.replace(digest, &digest[..63])),
        ("no functions", replace_field(&valid, "functions", "")),
        ("an empty host", valid.replace("host: claudine", "host: \"\"")),
        ("a function pair with an extra key", valid.replace("functions: []", "functions:\n    - catalog: ./a.yaml\n      facts: ./a.facts.json\n      extra: 1")),
    ];
    for (case, text) in cases {
        let refusal = *admit_manifest(&manifest, text.as_bytes()).expect_err(case);
        assert_eq!(refusal.unsupported_format, None, "{case}");
        assert!(refusal.message.starts_with("the format 2 manifest is malformed"), "{case}: {}", refusal.message);
    }
}
