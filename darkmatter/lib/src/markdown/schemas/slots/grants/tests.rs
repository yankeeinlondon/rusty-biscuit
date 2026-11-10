//! Grant store parsing and writing, and grant matching, at the module's
//! public API. Every store lives in its own temporary folder.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::*;
use crate::markdown::compose::expression::read_gate::ReadLocation;
use crate::markdown::schemas::slots::codes::SLOT_GRANT_STORE_INVALID_CODE;

/// A temporary folder, its canonical path, and a store file inside it.
struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    store: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let store = root.join("schema-read-policy");
        Self { _dir: dir, root, store }
    }

    fn write(&self, contents: impl AsRef<[u8]>) {
        std::fs::write(&self.store, contents).expect("write store");
    }

    fn load(&self) -> Result<GrantStore, GrantStoreError> {
        GrantStore::load(Some(&self.store))
    }

    /// A folder (created) below the root.
    fn dir(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.root.clone(), |path, part| path.join(part));
        std::fs::create_dir_all(&path).expect("create dir");
        path
    }

    /// A file (created, with its folder) below the root.
    /// Creates `relative` (`/`-separated) under the root, joined by
    /// component so the path is spelled natively on every OS.
    fn file(&self, relative: &str) -> PathBuf {
        let path = relative.split('/').fold(self.root.clone(), |path, part| path.join(part));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(&path, "x").expect("write file");
        path
    }
}

fn local(path: &Path) -> ReadLocation {
    ReadLocation::Local(path.to_path_buf())
}

fn remote(url: &str) -> ReadLocation {
    ReadLocation::Remote(url::Url::parse(url).expect("url"))
}

fn url(text: &str) -> url::Url {
    url::Url::parse(text).expect("url")
}

fn site(scheme: &str, host: &str, port: Option<u16>) -> GrantTarget {
    GrantTarget::Site { scheme: scheme.to_string(), host: host.to_string(), port }
}

fn book_from(fixture: &Fixture, contents: &str) -> GrantBook {
    fixture.write(contents);
    GrantBook::new(fixture.load().expect("store loads"), Vec::new())
}

// ---------------------------------------------------------------------------
// Store location
// ---------------------------------------------------------------------------

#[test]
fn store_path_is_below_home_on_every_os() {
    let home = PathBuf::from("home-root");
    let expected = home.join(".config").join("darkmatter").join("schema-read-policy");
    assert_eq!(GrantStore::path_for_home(Some(&home)), Some(expected));
}

#[test]
fn no_home_has_no_store_and_cannot_persist() {
    assert_eq!(GrantStore::path_for_home(None), None);
    let store = GrantStore::load(None).expect("no home is an empty store");
    assert!(store.grants().is_empty());
    assert_eq!(store.path(), None);
    let mut book = GrantBook::new(store, Vec::new());
    let error = book.persist(GrantDecision::Allow, site("https", "example.com", None)).expect_err("no store");
    assert!(matches!(error, GrantStoreWriteError::NoStore), "{error:?}");
    assert!(book.persisted().is_empty());
}

// ---------------------------------------------------------------------------
// Control row: one line of each of the ten kinds
// ---------------------------------------------------------------------------

#[test]
fn control_row_loads_every_kind_and_decides() {
    let fixture = Fixture::new();
    let allowed_file = fixture.file("allowed/file.yaml");
    let allowed_dir = fixture.dir("allowed-dir");
    let denied_file = fixture.file("denied/file.yaml");
    let denied_dir = fixture.dir("denied-dir");
    let contents = format!(
        "allow file {}\nallow dir {}\nallow url https://a.example/x?v=1\nallow url-any-query https://b.example/y\n\
         allow site https://c.example\ndeny file {}\ndeny dir {}\ndeny url https://d.example/x?v=1\n\
         deny url-any-query https://e.example/y\ndeny site http://f.example:8080\n",
        allowed_file.display(),
        allowed_dir.display(),
        denied_file.display(),
        denied_dir.display(),
    );
    let book = book_from(&fixture, &contents);

    assert_eq!(book.persisted().len(), 10);
    assert!(book.persisted().iter().all(|grant| grant.scope == GrantScope::Persisted));
    let kinds: Vec<_> = book.persisted().iter().map(|grant| (grant.decision, grant.target.kind_word())).collect();
    assert_eq!(
        kinds,
        [
            (GrantDecision::Allow, "file"),
            (GrantDecision::Allow, "dir"),
            (GrantDecision::Allow, "url"),
            (GrantDecision::Allow, "url-any-query"),
            (GrantDecision::Allow, "site"),
            (GrantDecision::Deny, "file"),
            (GrantDecision::Deny, "dir"),
            (GrantDecision::Deny, "url"),
            (GrantDecision::Deny, "url-any-query"),
            (GrantDecision::Deny, "site"),
        ]
    );

    let allow = Some(GrantDecision::Allow);
    let deny = Some(GrantDecision::Deny);
    assert_eq!(book.decide(&local(&allowed_file)), allow);
    assert_eq!(book.decide(&local(&allowed_dir.join("deep/x.md"))), allow);
    assert_eq!(book.decide(&remote("https://a.example/x?v=1")), allow);
    assert_eq!(book.decide(&remote("https://b.example/y?anything")), allow);
    assert_eq!(book.decide(&remote("https://c.example/any/path")), allow);
    assert_eq!(book.decide(&local(&denied_file)), deny);
    assert_eq!(book.decide(&local(&denied_dir.join("x.md"))), deny);
    assert_eq!(book.decide(&remote("https://d.example/x?v=1")), deny);
    assert_eq!(book.decide(&remote("https://e.example/y?z=2")), deny);
    assert_eq!(book.decide(&remote("http://f.example:8080/p")), deny);
    assert_eq!(book.decide(&remote("https://unlisted.example/")), None);
    assert_eq!(book.decide(&local(&fixture.root.join("elsewhere.md"))), None);
}

// ---------------------------------------------------------------------------
// Invalid field shapes: each a located error, and the file grants nothing
// ---------------------------------------------------------------------------

/// Generates one test per invalid line. Each fixture puts a valid deny on
/// line 1, a comment on line 2, and the invalid line on line 3, so the test
/// proves both the 1-based location and that the valid deny is not kept.
macro_rules! invalid_line_cases {
    ($($name:ident => $line:expr,)+) => {
        $(
            #[test]
            fn $name() {
                assert_invalid_third_line($line);
            }
        )+

        #[test]
        fn invalid_line_table_covers_every_shape() {
            let names = [$(stringify!($name)),+];
            assert_eq!(names.len(), 17, "update the count when a shape is added: {names:?}");
        }
    };
}

fn assert_invalid_third_line(line: &str) {
    let fixture = Fixture::new();
    fixture.write(format!("deny site https://kept.example\n# comment\n{line}\nallow site https://after.example\n"));
    let error = fixture.load().expect_err(line);
    assert_eq!(error.path, fixture.store, "{line}");
    assert_eq!(error.line, 3, "{line}: {error}");
    assert_eq!(error.code(), SLOT_GRANT_STORE_INVALID_CODE);
    assert_eq!(error.kind, GrantStoreErrorKind::InvalidLine);
    assert!(error.to_string().contains(":3: "), "{error}");
}

invalid_line_cases! {
    invalid_absent_file_target => "allow file",
    invalid_empty_file_target_after_trim => "allow file    ",
    invalid_absent_url_target => "deny url",
    invalid_absent_kind => "allow",
    invalid_relative_file_path => "allow file relative/x.yaml",
    invalid_relative_dir_path => "deny dir ./schemas",
    invalid_url_with_fragment => "allow url https://example.com/x#part",
    invalid_url_any_query_with_fragment => "allow url-any-query https://example.com/x#part",
    invalid_unparseable_url => "allow url not a url",
    invalid_non_http_url_scheme => "allow url ftp://example.com/x",
    invalid_non_http_url_any_query_scheme => "allow url-any-query file:///etc/passwd",
    invalid_non_http_site_scheme => "allow site ftp://example.com",
    invalid_site_with_path => "allow site https://example.com/docs",
    invalid_site_with_userinfo => "allow site https://user:pw@example.com",
    invalid_unknown_verb => "permit file /x",
    invalid_unknown_kind => "allow folder /x",
    invalid_url_any_query_with_query => "allow url-any-query https://example.com/x?v=1",
}

#[test]
fn invalid_site_with_query() {
    assert_invalid_third_line("allow site https://example.com/?q=1");
}

#[test]
fn invalid_site_with_fragment() {
    assert_invalid_third_line("allow site https://example.com#frag");
}

#[cfg(windows)]
#[test]
fn invalid_drive_less_rooted_path_on_windows() {
    assert_invalid_third_line(r"allow file \foo\bar.yaml");
}

#[test]
fn not_utf8_names_the_first_bad_line() {
    let fixture = Fixture::new();
    fixture.write(b"# ok\nallow site https://a.example\nallow file /\xFF\n".as_slice());
    let error = fixture.load().expect_err("not utf-8");
    assert_eq!(error.kind, GrantStoreErrorKind::NotUtf8);
    assert_eq!(error.line, 3);
    assert_eq!(error.code(), SLOT_GRANT_STORE_INVALID_CODE);
}

#[test]
fn unreadable_store_is_a_whole_file_error() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(&fixture.store).expect("a folder where the file should be");
    let error = fixture.load().expect_err("unreadable");
    assert_eq!(error.kind, GrantStoreErrorKind::Unreadable);
    assert_eq!(error.line, 0);
    assert_eq!(error.path, fixture.store);
    assert_eq!(error.code(), SLOT_GRANT_STORE_INVALID_CODE);
}

// ---------------------------------------------------------------------------
// Accepted inputs
// ---------------------------------------------------------------------------

#[test]
fn missing_file_is_an_empty_store_that_keeps_its_path() {
    let fixture = Fixture::new();
    let store = fixture.load().expect("missing is empty");
    assert!(store.grants().is_empty());
    assert_eq!(store.path(), Some(fixture.store.as_path()));
}

#[test]
fn empty_file_is_an_empty_store() {
    let fixture = Fixture::new();
    fixture.write("");
    assert!(fixture.load().expect("empty").grants().is_empty());
}

#[test]
fn comments_blank_lines_crlf_and_bom_are_accepted() {
    let fixture = Fixture::new();
    fixture.write("\u{FEFF}# leading comment\r\n\r\n   \r\n   # indented comment\r\n  allow   site   https://a.example  \r\ndeny url https://b.example/x\r\n");
    let store = fixture.load().expect("loads");
    assert_eq!(
        store.grants(),
        [
            Grant { decision: GrantDecision::Allow, target: site("https", "a.example", None), scope: GrantScope::Persisted },
            Grant {
                decision: GrantDecision::Deny,
                target: GrantTarget::Url(url("https://b.example/x")),
                scope: GrantScope::Persisted
            },
        ]
    );
}

#[test]
fn paths_with_spaces_are_the_rest_of_the_line() {
    let fixture = Fixture::new();
    let spaced = fixture.file("with space/a file.yaml");
    let book = book_from(&fixture, &format!("allow file   {}  \n", spaced.display()));
    assert_eq!(book.persisted()[0].target, GrantTarget::File(spaced.clone()));
    assert_eq!(book.decide(&local(&spaced)), Some(GrantDecision::Allow));
}

#[test]
fn stored_paths_are_canonicalized_when_they_exist() {
    let fixture = Fixture::new();
    let real = fixture.file("real/x.yaml");
    let spelled = fixture.root.join("real").join("..").join("real").join("x.yaml");
    let missing = fixture.root.join("missing").join("..").join("y.yaml");
    let book = book_from(&fixture, &format!("allow file {}\nallow file {}\n", spelled.display(), missing.display()));
    assert_eq!(book.persisted()[0].target, GrantTarget::File(real));
    assert_eq!(book.persisted()[1].target, GrantTarget::File(missing));
}

#[test]
fn duplicate_lines_are_kept_once() {
    let fixture = Fixture::new();
    let store = {
        fixture.write("allow site https://a.example\nallow site https://a.example\nallow  site https://a.example:443\n");
        fixture.load().expect("loads")
    };
    assert_eq!(store.grants().len(), 1);
}

#[test]
fn allow_and_deny_for_the_same_target_deny() {
    let fixture = Fixture::new();
    let book = book_from(&fixture, "allow site https://a.example\ndeny site https://a.example\n");
    assert_eq!(book.persisted().len(), 2);
    assert_eq!(book.decide(&remote("https://a.example/x")), Some(GrantDecision::Deny));
}

// ---------------------------------------------------------------------------
// Matching per target kind
// ---------------------------------------------------------------------------

fn once_book(decision: GrantDecision, target: GrantTarget) -> GrantBook {
    let mut book = GrantBook::empty();
    book.add_once(decision, target);
    book
}

#[test]
fn file_grant_matches_only_that_file() {
    let book = once_book(GrantDecision::Allow, GrantTarget::File(PathBuf::from("/a/b.yaml")));
    assert_eq!(book.decide(&ReadLocation::Local(PathBuf::from("/a/b.yaml"))), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&ReadLocation::Local(PathBuf::from("/a/b.yaml.bak"))), None);
    assert_eq!(book.decide(&ReadLocation::Local(PathBuf::from("/a"))), None);
    assert_eq!(book.decide(&remote("https://a.example/a/b.yaml")), None);
}

#[test]
fn dir_grant_is_recursive_by_component_not_string_prefix() {
    let fixture = Fixture::new();
    let dir = fixture.dir("a/b");
    let book = once_book(GrantDecision::Allow, GrantTarget::Dir(dir.clone()));
    assert_eq!(book.decide(&local(&dir)), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&local(&dir.join("c/d/e.md"))), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&local(&fixture.root.join("a/bc/e.md"))), None);
    assert_eq!(book.decide(&local(&fixture.root.join("a"))), None);
}

#[test]
fn url_grant_is_exact_including_query() {
    let book = once_book(GrantDecision::Allow, GrantTarget::Url(url("https://a.example/x?v=1")));
    assert_eq!(book.decide(&remote("https://a.example/x?v=1")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("HTTPS://A.EXAMPLE:443/x?v=1")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://a.example/x?v=2")), None);
    assert_eq!(book.decide(&remote("https://a.example/x")), None);
    assert_eq!(book.decide(&remote("http://a.example/x?v=1")), None);
}

#[test]
fn url_any_query_grant_ignores_only_the_query() {
    let book = once_book(GrantDecision::Allow, GrantTarget::UrlAnyQuery(url("https://a.example/x")));
    assert_eq!(book.decide(&remote("https://a.example/x")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://a.example/x?v=9&w")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://a.example:443/x?v=1")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://a.example/x/y")), None);
    assert_eq!(book.decide(&remote("https://a.example:8443/x")), None);
    assert_eq!(book.decide(&remote("http://a.example/x")), None);
}

#[test]
fn site_grant_matches_scheme_host_and_port() {
    let book = once_book(GrantDecision::Allow, site("https", "h.example", None));
    assert_eq!(book.decide(&remote("https://h.example:443/x")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://h.example/any?q")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("http://h.example/x")), None);
    assert_eq!(book.decide(&remote("https://sub.h.example/x")), None);
    assert_eq!(book.decide(&remote("https://h.example:8443/x")), None);

    let ported = once_book(GrantDecision::Allow, site("http", "h.example", Some(8080)));
    assert_eq!(ported.decide(&remote("http://h.example:8080/x")), Some(GrantDecision::Allow));
    assert_eq!(ported.decide(&remote("http://h.example/x")), None);
}

#[test]
fn site_line_with_default_port_equals_one_without() {
    let fixture = Fixture::new();
    let book = book_from(&fixture, "allow site https://h.example:443/\n");
    assert_eq!(book.persisted()[0].target, site("https", "h.example", None));
    assert_eq!(book.persisted()[0].target.to_string(), "https://h.example");
}

// ---------------------------------------------------------------------------
// Deny wins across scopes
// ---------------------------------------------------------------------------

#[test]
fn persisted_deny_beats_once_allow() {
    let fixture = Fixture::new();
    let mut book = book_from(&fixture, "deny site https://a.example\n");
    book.add_once(GrantDecision::Allow, GrantTarget::Url(url("https://a.example/x")));
    assert_eq!(book.decide(&remote("https://a.example/x")), Some(GrantDecision::Deny));
}

#[test]
fn once_deny_beats_persisted_allow() {
    let fixture = Fixture::new();
    let dir = fixture.dir("data");
    let mut book = book_from(&fixture, &format!("allow dir {}\n", dir.display()));
    let secret = dir.join("secret.yaml");
    book.add_once(GrantDecision::Deny, GrantTarget::File(secret.clone()));
    assert_eq!(book.decide(&local(&secret)), Some(GrantDecision::Deny));
    assert_eq!(book.decide(&local(&dir.join("open.yaml"))), Some(GrantDecision::Allow));
}

#[test]
fn deny_beats_a_host_value() {
    let mut book = GrantBook::new(GrantStore::empty(), vec!["a.example".to_string()]);
    book.add_once(GrantDecision::Deny, GrantTarget::UrlAnyQuery(url("https://a.example/secret")));
    assert_eq!(book.decide(&remote("https://a.example/secret?x")), Some(GrantDecision::Deny));
    assert_eq!(book.decide(&remote("https://a.example/open")), Some(GrantDecision::Allow));
}

// ---------------------------------------------------------------------------
// `--allow host=` values
// ---------------------------------------------------------------------------

#[test]
fn host_values_allow_http_and_https_on_any_port_for_that_host_only() {
    let book = GrantBook::new(GrantStore::empty(), vec!["Docs.Example".to_string()]);
    for location in ["http://docs.example/x", "https://docs.example/y?q", "https://docs.example:8443/z", "http://DOCS.example:81/"] {
        assert_eq!(book.decide(&remote(location)), Some(GrantDecision::Allow), "{location}");
    }
    for location in ["https://sub.docs.example/", "https://example/", "ftp://docs.example/x", "https://docs.example.evil/"] {
        assert_eq!(book.decide(&remote(location)), None, "{location}");
    }
    assert_eq!(book.decide(&ReadLocation::Local(PathBuf::from("/docs.example"))), None);
}

// ---------------------------------------------------------------------------
// Glob coverage
// ---------------------------------------------------------------------------

fn glob(path: &Path, tail: &str) -> ReadLocation {
    ReadLocation::Glob(format!("{}/{tail}", path.display()))
}

#[test]
fn glob_is_allowed_by_a_dir_holding_its_literal_folders() {
    let fixture = Fixture::new();
    let dir = fixture.dir("schemas");
    let book = once_book(GrantDecision::Allow, GrantTarget::Dir(dir.clone()));
    assert_eq!(book.decide(&glob(&dir, "**/*.yaml")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&glob(&dir, "nested/*.md")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&glob(&fixture.root, "schemas*/x.yaml")), None, "wildcard above the dir");
    assert_eq!(book.decide(&glob(&fixture.root, "**/*.yaml")), None, "reaches beyond the dir");
    assert_eq!(book.decide(&glob(&dir, "../**")), None, "parent component");
    assert_eq!(book.decide(&ReadLocation::Glob("schemas/**/*.yaml".to_string())), None, "relative");
    assert_eq!(book.decide(&ReadLocation::Glob("^**/*.yaml".to_string())), None, "sigil");
}

#[test]
fn glob_is_never_allowed_by_file_url_or_site_grants() {
    let fixture = Fixture::new();
    let file = fixture.file("one.yaml");
    let mut book = once_book(GrantDecision::Allow, GrantTarget::File(file.clone()));
    book.add_once(GrantDecision::Allow, site("https", "a.example", None));
    assert_eq!(book.decide(&ReadLocation::Glob(file.display().to_string())), None);
}

#[test]
fn glob_is_denied_by_any_deny_its_matches_could_reach() {
    let fixture = Fixture::new();
    let dir = fixture.dir("data");
    let private = fixture.dir("data/private");
    let mut book = GrantBook::empty();
    book.add_once(GrantDecision::Allow, GrantTarget::Dir(dir.clone()));
    book.add_once(GrantDecision::Deny, GrantTarget::Dir(private.clone()));
    assert_eq!(book.decide(&glob(&private, "*.yaml")), Some(GrantDecision::Deny), "inside the deny");
    assert_eq!(book.decide(&glob(&dir, "**/*.yaml")), Some(GrantDecision::Deny), "reaches into the deny");
    assert_eq!(book.decide(&glob(&dir, "public/*.yaml")), Some(GrantDecision::Allow), "beside the deny");
    assert_eq!(book.decide(&ReadLocation::Glob("**/*.yaml".to_string())), Some(GrantDecision::Deny), "unknown reach");
    assert_eq!(book.decide(&glob(&dir, "public/../private/*")), Some(GrantDecision::Deny), "parent component");
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

#[test]
fn append_creates_the_parent_folder_and_persists() {
    let fixture = Fixture::new();
    let path = GrantStore::path_for_home(Some(&fixture.root)).expect("store path");
    let mut book = GrantBook::new(GrantStore::load(Some(&path)).expect("missing loads"), Vec::new());
    let data = fixture.dir("data");

    book.persist(GrantDecision::Allow, GrantTarget::Dir(data.clone())).expect("first append");
    assert!(path.is_file());
    assert_eq!(book.decide(&local(&data.join("x.yaml"))), Some(GrantDecision::Allow));

    book.persist(GrantDecision::Deny, GrantTarget::Url(url("https://a.example/x?v=1"))).expect("second append");
    let expected = format!("allow dir {}\ndeny url https://a.example/x?v=1\n", data.display());
    assert_eq!(std::fs::read_to_string(&path).expect("read"), expected);
    assert_eq!(book.persisted().len(), 2);
    assert!(book.persisted().iter().all(|grant| grant.scope == GrantScope::Persisted));
}

#[test]
fn appending_an_existing_grant_is_a_no_op() {
    let fixture = Fixture::new();
    fixture.write("allow site https://a.example\n");
    let before = std::fs::metadata(&fixture.store).expect("metadata").modified().expect("mtime");
    let mut store = fixture.load().expect("loads");
    store.append(GrantDecision::Allow, site("https", "a.example", None)).expect("no-op");
    assert_eq!(std::fs::read_to_string(&fixture.store).expect("read"), "allow site https://a.example\n");
    assert_eq!(std::fs::metadata(&fixture.store).expect("metadata").modified().expect("mtime"), before);
    assert_eq!(store.grants().len(), 1);
}

#[test]
fn append_keeps_existing_bytes_and_adds_a_missing_final_newline() {
    let fixture = Fixture::new();
    fixture.write("# mine\r\nallow site https://a.example");
    let mut store = fixture.load().expect("loads");
    store.append(GrantDecision::Deny, site("http", "b.example", Some(8080))).expect("append");
    assert_eq!(
        std::fs::read_to_string(&fixture.store).expect("read"),
        "# mine\r\nallow site https://a.example\ndeny site http://b.example:8080\n"
    );
}

#[test]
fn append_refuses_a_target_that_would_not_read_back() {
    let fixture = Fixture::new();
    let mut store = fixture.load().expect("loads");
    for path in [fixture.root.join("line\nbreak"), fixture.root.join("trailing ")] {
        let error = store.append(GrantDecision::Deny, GrantTarget::File(path)).expect_err("unrepresentable");
        assert!(matches!(error, GrantStoreWriteError::Unrepresentable { .. }), "{error:?}");
    }
    assert!(!fixture.store.exists());
}

#[test]
fn write_read_write_read_round_trips() {
    let fixture = Fixture::new();
    let file = fixture.file("spaced name/f.yaml");
    let dir = fixture.dir("d");
    let targets = [
        (GrantDecision::Allow, GrantTarget::File(file)),
        (GrantDecision::Deny, GrantTarget::Dir(dir)),
        (GrantDecision::Allow, GrantTarget::Url(url("https://a.example/x?v=1&w=2"))),
        (GrantDecision::Deny, GrantTarget::UrlAnyQuery(url("http://b.example:8080/y"))),
        (GrantDecision::Allow, site("https", "c.example", Some(8443))),
    ];
    let mut first = fixture.load().expect("missing loads");
    for (decision, target) in targets.clone() {
        first.append(decision, target).expect("append");
    }
    let first_read = fixture.load().expect("first read");
    assert_eq!(first_read.grants(), first.grants());
    let first_bytes = std::fs::read(&fixture.store).expect("bytes");

    let second_path = fixture.root.join("second-store");
    let mut second = GrantStore::load(Some(&second_path)).expect("missing loads");
    for grant in first_read.grants() {
        second.append(grant.decision, grant.target.clone()).expect("append");
    }
    let second_read = GrantStore::load(Some(&second_path)).expect("second read");
    assert_eq!(second_read.grants(), first_read.grants());
    assert_eq!(std::fs::read(&second_path).expect("bytes"), first_bytes);
}

#[test]
fn grant_book_is_send_sync_and_clone() {
    fn assert_traits<T: Send + Sync + Clone>() {}
    assert_traits::<GrantBook>();
}
