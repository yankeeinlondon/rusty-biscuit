//! Git's ref-update lines: framing, field validation, and how each update is
//! classified.

use gix::hash::Kind;
use rstest::rstest;

use crate::ci_push::{InputError, RefUpdate, SkipReason, TriggerEvent, parse_updates};

const A: &str = "1111111111111111111111111111111111111111";
const B: &str = "2222222222222222222222222222222222222222";
const ZERO: &str = "0000000000000000000000000000000000000000";

fn parse(input: &str) -> Result<Vec<RefUpdate>, InputError> {
    parse_updates(input.as_bytes(), Kind::Sha1)
}

fn destinations(updates: &[RefUpdate]) -> Vec<(usize, &str)> {
    updates.iter().map(|update| (update.line, update.remote_ref.as_str())).collect()
}

#[rstest]
#[case::lf("refs/heads/a {A} refs/heads/a {ZERO}\nrefs/heads/b {B} refs/heads/b {A}\n")]
#[case::crlf("refs/heads/a {A} refs/heads/a {ZERO}\r\nrefs/heads/b {B} refs/heads/b {A}\r\n")]
#[case::no_final_newline("refs/heads/a {A} refs/heads/a {ZERO}\nrefs/heads/b {B} refs/heads/b {A}")]
fn every_line_ending_gives_the_same_updates(#[case] template: &str) {
    let input = template.replace("{A}", A).replace("{B}", B).replace("{ZERO}", ZERO);
    let updates = parse(&input).unwrap();
    assert_eq!(destinations(&updates), [(1, "refs/heads/a"), (2, "refs/heads/b")]);
    assert_eq!(updates[1].local_oid.to_string(), B);
    assert_eq!(updates[1].remote_oid.to_string(), A);
    assert!(updates[0].remote_oid.is_null());
}

#[test]
fn blank_and_whitespace_only_lines_are_ignored_but_counted() {
    let input = format!("\n  \t\nrefs/heads/a {A} refs/heads/a {ZERO}\n\r\n");
    assert_eq!(destinations(&parse(&input).unwrap()), [(3, "refs/heads/a")]);
}

#[test]
fn empty_input_is_an_empty_push() {
    assert_eq!(parse("").unwrap(), []);
}

#[test]
fn fields_may_be_separated_by_any_ascii_whitespace() {
    let input = format!("refs/heads/a\t{A}  refs/heads/a \t{ZERO}\n");
    assert_eq!(destinations(&parse(&input).unwrap()), [(1, "refs/heads/a")]);
}

#[rstest]
#[case::three_fields(format!("refs/heads/a {A} refs/heads/a"), "has 3 fields")]
#[case::five_fields(format!("refs/heads/a {A} refs/heads/a {ZERO} extra"), "has 5 fields")]
#[case::short_local_oid(format!("refs/heads/a 1234 refs/heads/a {ZERO}"), "local oid \"1234\" is not 40 lowercase hex digits")]
#[case::uppercase_oid(format!("refs/heads/a {} refs/heads/a {ZERO}", A.replace('1', "A")), "local oid")]
#[case::non_hex_oid(format!("refs/heads/a {A} refs/heads/a {}", "g".repeat(40)), "remote oid")]
#[case::sha256_length_in_a_sha1_repository(format!("refs/heads/a {}1 refs/heads/a {ZERO}", "a".repeat(63)), "local oid")]
#[case::short_remote_ref(format!("refs/heads/a {A} main {ZERO}"), "remote ref \"main\" is not a full ref name")]
#[case::invalid_remote_ref(format!("refs/heads/a {A} refs/heads/a..b {ZERO}"), "is not a full ref name")]
fn a_malformed_line_names_its_line_number(#[case] line: String, #[case] problem: &str) {
    let input = format!("refs/heads/ok {A} refs/heads/ok {ZERO}\n{line}\n");
    let error = parse(&input).unwrap_err();
    assert_eq!(error.line, 2, "{error}");
    assert!(error.problem.contains(problem), "{error}");
    assert_eq!(error.text, line);
}

#[test]
fn a_crlf_malformed_line_is_quoted_without_its_line_ending() {
    let error = parse("refs/heads/a 1234 refs/heads/a 0\r\n").unwrap_err();
    assert_eq!(error.text, "refs/heads/a 1234 refs/heads/a 0");
}

#[test]
fn a_non_utf8_line_is_malformed() {
    let mut input = format!("refs/heads/a {A} refs/heads/a {ZERO}\n").into_bytes();
    input.extend_from_slice(b"refs/heads/\xff ");
    input.extend_from_slice(format!("{A} refs/heads/b {ZERO}\n").as_bytes());
    let error = parse_updates(&input, Kind::Sha1).unwrap_err();
    assert_eq!(error.line, 2);
    assert_eq!(error.problem, "is not valid UTF-8");
}

#[test]
fn a_second_update_of_one_destination_is_malformed() {
    let input = format!("refs/heads/a {A} refs/heads/x {ZERO}\nrefs/heads/b {B} refs/heads/x {ZERO}\n");
    let error = parse(&input).unwrap_err();
    assert_eq!(error.line, 2);
    assert_eq!(error.problem, "updates refs/heads/x again (line 1 already updates it)");
    assert!(error.to_string().starts_with("ref update line 2 updates refs/heads/x again"));
}

#[rstest]
#[case::branch(format!("refs/heads/a {A} refs/heads/a {B}"), None)]
#[case::branch_created(format!("refs/heads/a {A} refs/heads/a {ZERO}"), None)]
#[case::deletion(format!("(delete) {ZERO} refs/heads/a {B}"), Some(SkipReason::Deletion))]
#[case::deleted_tag(format!("(delete) {ZERO} refs/tags/v1 {B}"), Some(SkipReason::Deletion))]
#[case::tag(format!("refs/tags/v1 {A} refs/tags/v1 {ZERO}"), Some(SkipReason::NotABranch))]
#[case::notes(format!("refs/notes/ci-local/scope {A} refs/notes/ci-local/scope {B}"), Some(SkipReason::NotABranch))]
fn only_a_branch_update_is_reviewed(#[case] line: String, #[case] skip: Option<SkipReason>) {
    let update = parse(&line).unwrap().remove(0);
    assert_eq!(update.skip_reason(), skip);
    assert_eq!(update.is_reviewed(), skip.is_none());
}

#[test]
fn a_renamed_refspec_keeps_both_branch_names() {
    let update = parse(&format!("refs/heads/feature {A} refs/heads/other {ZERO}")).unwrap().remove(0);
    assert_eq!(update.local_branch(), Some("feature"));
    assert_eq!(update.remote_branch(), Some("other"));
    assert_eq!(update.label(), "refs/heads/feature -> refs/heads/other");
}

#[rstest]
#[case::head("HEAD")]
#[case::revision("HEAD~1")]
#[case::object_id(A)]
fn a_source_that_is_not_a_branch_has_no_local_branch(#[case] source: &str) {
    let update = parse(&format!("{source} {A} refs/heads/other {ZERO}")).unwrap().remove(0);
    assert_eq!(update.local_branch(), None);
    assert_eq!(update.remote_branch(), Some("other"));
    assert!(update.is_reviewed());
}

#[rstest]
#[case::main("refs/heads/main", TriggerEvent::Push, "push")]
#[case::feature("refs/heads/feature", TriggerEvent::PullRequest, "pull_request")]
#[case::main_lookalike("refs/heads/main2", TriggerEvent::PullRequest, "pull_request")]
fn main_triggers_a_push_and_every_other_branch_a_pull_request(
    #[case] destination: &str,
    #[case] event: TriggerEvent,
    #[case] spelling: &str,
) {
    let update = parse(&format!("refs/heads/x {A} {destination} {ZERO}")).unwrap().remove(0);
    assert_eq!(update.event(), event);
    assert_eq!(update.event().as_str(), spelling);
}
