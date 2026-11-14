//! Approval matching, the choices a prompt offers, and the retired store's
//! target grammar the migration still parses.

use std::path::{Path, PathBuf};

use super::request::grant_choices;
use super::*;
use crate::config::network::NetworkPermission;
use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::{ReadLocation, ReadTarget};

fn absolute(relative: &str) -> PathBuf {
    std::env::temp_dir().join("darkmatter-grant-tests").join(relative)
}

fn local(path: &Path) -> ReadLocation {
    ReadLocation::Local(path.to_path_buf())
}

fn url(text: &str) -> url::Url {
    url::Url::parse(text).unwrap()
}

fn remote(text: &str) -> ReadLocation {
    ReadLocation::Remote(url(text))
}

fn once_book(grants: &[(GrantDecision, GrantTarget)]) -> GrantBook {
    let mut book = GrantBook::empty();
    for (decision, target) in grants {
        book.add_once(*decision, target.clone());
    }
    book
}

fn target(kind: ReadKind, location: ReadLocation) -> ReadTarget {
    ReadTarget { function: "load_data".to_string(), kind, location, observed_hash: None }
}

#[test]
fn a_file_approval_matches_only_that_file() {
    let file = absolute("data/a.yaml");
    let book = once_book(&[(GrantDecision::Allow, GrantTarget::File(file.clone()))]);
    assert_eq!(book.decide(&local(&file)), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&local(&absolute("data/b.yaml"))), None);
    assert_eq!(book.decide(&local(&absolute("data"))), None, "a file approval never covers its folder");
}

#[test]
fn a_url_approval_is_exact_including_the_query() {
    let book = once_book(&[(GrantDecision::Allow, GrantTarget::Url(url("https://example.com/t.yaml?v=2")))]);
    assert_eq!(book.decide(&remote("https://example.com/t.yaml?v=2")), Some(GrantDecision::Allow));
    assert_eq!(book.decide(&remote("https://example.com/t.yaml?v=3")), None);
    assert_eq!(book.decide(&remote("https://example.com/t.yaml")), None);
}

#[test]
fn a_deny_for_this_run_beats_an_allow_for_this_run() {
    let file = absolute("data/a.yaml");
    let book = once_book(&[
        (GrantDecision::Allow, GrantTarget::File(file.clone())),
        (GrantDecision::Deny, GrantTarget::File(file.clone())),
    ]);
    assert_eq!(book.decide(&local(&file)), Some(GrantDecision::Deny));
}

#[test]
fn retired_folder_url_any_query_and_site_approvals_are_ignored() {
    let folder = absolute("data");
    let book = once_book(&[
        (GrantDecision::Allow, GrantTarget::Dir(folder.clone())),
        (GrantDecision::Allow, GrantTarget::UrlAnyQuery(url("https://example.com/t.yaml"))),
        (GrantDecision::Allow, GrantTarget::Site { scheme: "https".into(), host: "example.com".into(), port: None }),
    ]);
    assert!(book.once().is_empty(), "{:?}", book.once());
    assert_eq!(book.decide(&local(&folder.join("a.yaml"))), None);
    assert_eq!(book.decide(&remote("https://example.com/t.yaml?v=1")), None);
}

#[test]
fn host_values_allow_http_and_https_on_any_port_for_that_host_only() {
    let book = GrantBook::new(vec!["Example.com".to_string()]);
    assert!(book.network_allows(&remote("https://example.com/a")));
    assert!(book.network_allows(&remote("http://example.com:8080/a")));
    assert!(!book.network_allows(&remote("https://other.example.com/a")));
    assert!(!book.network_allows(&local(&absolute("a"))));
}

#[test]
fn network_allow_allows_every_http_url_and_deny_only_the_hosts() {
    let allow = GrantBook::empty().with_network(NetworkPermission::Allow);
    assert!(allow.network_allows(&remote("https://anything.example/a")));
    let deny = GrantBook::empty().with_network(NetworkPermission::Deny);
    assert!(!deny.network_allows(&remote("https://anything.example/a")));
}

#[test]
fn a_file_read_offers_allow_once_always_allow_and_deny_for_this_run() {
    let file = absolute("data/a.yaml");
    let choices = grant_choices(&target(ReadKind::Content, local(&file)), true);
    let expected = |decision, scope| Grant { decision, target: GrantTarget::File(file.clone()), scope };
    assert_eq!(
        choices,
        [
            expected(GrantDecision::Allow, GrantScope::Once),
            expected(GrantDecision::Allow, GrantScope::Persisted),
            expected(GrantDecision::Deny, GrantScope::Once),
        ]
    );
    let unsaveable = grant_choices(&target(ReadKind::Probe, local(&file)), false);
    assert_eq!(unsaveable, [expected(GrantDecision::Allow, GrantScope::Once), expected(GrantDecision::Deny, GrantScope::Once)]);
}

#[test]
fn a_url_read_offers_only_this_run() {
    let read = url("https://example.com/t.yaml?v=2");
    let choices = grant_choices(&target(ReadKind::Content, ReadLocation::Remote(read.clone())), true);
    assert_eq!(
        choices,
        [
            Grant { decision: GrantDecision::Allow, target: GrantTarget::Url(read.clone()), scope: GrantScope::Once },
            Grant { decision: GrantDecision::Deny, target: GrantTarget::Url(read), scope: GrantScope::Once },
        ]
    );
}

#[test]
fn a_listing_or_glob_is_never_asked_about() {
    assert!(grant_choices(&target(ReadKind::Listing, local(&absolute("data"))), true).is_empty());
    assert!(grant_choices(&target(ReadKind::Listing, ReadLocation::Glob("./*.yaml".to_string())), true).is_empty());
}

#[test]
fn only_file_and_exact_url_approvals_apply_and_only_a_file_allow_persists() {
    let file = GrantTarget::File(absolute("a.yaml"));
    let site = GrantTarget::Site { scheme: "https".into(), host: "example.com".into(), port: None };
    let grant = |decision, target: &GrantTarget, scope| Grant { decision, target: target.clone(), scope };
    assert!(grant(GrantDecision::Allow, &file, GrantScope::Persisted).is_applicable());
    assert!(!grant(GrantDecision::Deny, &file, GrantScope::Persisted).is_applicable(), "no persistent deny");
    assert!(!grant(GrantDecision::Allow, &GrantTarget::Url(url("https://e.com/a")), GrantScope::Persisted).is_applicable());
    assert!(!grant(GrantDecision::Allow, &site, GrantScope::Once).is_applicable());
}

#[test]
fn the_retired_store_grammar_still_parses_every_kind_for_the_migration() {
    let file = absolute("data/a.yaml");
    let text = file.to_str().unwrap();
    assert_eq!(GrantTarget::parse("file", text).unwrap(), GrantTarget::File(file.clone()));
    assert_eq!(GrantTarget::parse("dir", text).unwrap(), GrantTarget::Dir(file.clone()));
    assert_eq!(GrantTarget::parse("url", "https://e.com/a?b=1").unwrap(), GrantTarget::Url(url("https://e.com/a?b=1")));
    assert!(GrantTarget::parse("url-any-query", "https://e.com/a?b=1").is_err(), "written without a query");
    assert_eq!(
        GrantTarget::parse("site", "https://e.com:8443").unwrap(),
        GrantTarget::Site { scheme: "https".into(), host: "e.com".into(), port: Some(8443) }
    );
    assert!(GrantTarget::parse("file", "relative/a.yaml").is_err());
    assert!(GrantTarget::parse("folder", text).is_err());
    assert_eq!(GRANT_STORE_RELATIVE_PATH, [".config", "darkmatter", "schema-read-policy"]);
}
