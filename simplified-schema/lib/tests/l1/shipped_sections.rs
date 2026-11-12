//! Deferred payload sections (`shipped::accept_deferred`): an artifact whose
//! payload members each sit on their own line is accepted by its header and
//! member list alone, and each member is checked, decoded, and validated the
//! first time its owner reads it.
//!
//! The reader under test is `accept_deferred` with `Sections::take` and
//! `Section::get`. Default outcome for an invalid document layout, header,
//! or member list: the whole artifact is refused before any section is read.
//! Listed exception: a section whose JSON or record is invalid refuses only
//! that section (the others still serve), with the section's pointer and its
//! line and column in the artifact.

use serde::Serialize;
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{
    self, Acceptance, AcceptedArtifact, ArtifactDescriptor, FallbackReason, RecordError, Refusal, Section, Sections,
};

const SOURCES: &[EmbeddedDocument<'static>] = &[EmbeddedDocument { path: "host/a.yaml", text: "a: 1\n" }];

fn descriptor() -> ArtifactDescriptor<'static> {
    ArtifactDescriptor { owner: "test-host", preparation_revision: 1, sources: SOURCES, dependencies: &[], configuration: &[] }
}

#[derive(Serialize)]
struct Payload {
    names: Vec<String>,
    limit: u32,
}

struct Loaded {
    names: Section<Vec<String>>,
    limit: Section<u32>,
}

fn generated() -> String {
    let payload = Payload { names: vec!["b".into(), "a".into()], limit: 7 };
    String::from_utf8(shipped::generate(&descriptor(), &payload).expect("the payload encodes")).unwrap()
}

fn take(_: &Acceptance<'_>, sections: &mut Sections) -> Result<Loaded, RecordError> {
    Ok(Loaded { names: sections.take("names")?, limit: sections.take("limit")? })
}

fn load(text: &str) -> Result<AcceptedArtifact<Loaded>, Refusal> {
    shipped::accept_deferred(&descriptor(), Some(text.as_bytes()), take)
}

fn names(loaded: &Loaded) -> Result<&Vec<String>, &Refusal> {
    loaded.names.get(|_, names: Vec<String>| {
        if names.is_empty() { Err(RecordError::new("", "at least one name")) } else { Ok(names) }
    })
}

fn limit(loaded: &Loaded) -> Result<&u32, &Refusal> {
    loaded.limit.get(|_, limit: u32| Ok(limit))
}

#[test]
fn each_payload_member_is_written_on_its_own_line() {
    let text = generated();
    assert!(text.starts_with("{\n\"header\": {\n  \"wire_version\": 1,"), "{text}");
    assert!(text.ends_with("\"payload\": {\n\"names\":[\"b\",\"a\"],\n\"limit\":7\n}\n}\n"), "{text}");
}

#[test]
fn a_section_is_decoded_only_when_first_read_and_keeps_authored_order() {
    let accepted = load(&generated()).expect("the control is accepted");
    let loaded = accepted.payload();
    assert!(!loaded.names.is_decoded() && !loaded.limit.is_decoded());
    assert_eq!(limit(loaded), Ok(&7));
    assert!(!loaded.names.is_decoded(), "reading one section decodes no other");
    assert_eq!(names(loaded).map(|names| names.as_slice()), Ok(["b".to_string(), "a".to_string()].as_slice()));
}

#[test]
fn a_malformed_section_refuses_only_itself_at_its_line_and_column() {
    let text = generated().replace("\"names\":[\"b\",\"a\"]", "\"names\":[\"b\",1]");
    let accepted = load(&text).expect("the header and member list are accepted");
    let refusal = names(accepted.payload()).expect_err("the section is refused");
    assert_eq!(refusal.reason, FallbackReason::MalformedArtifact);
    assert_eq!(refusal.pointer.as_deref(), Some("/payload/names/1"));
    let line = text.lines().position(|line| line.starts_with("\"names\"")).unwrap() + 1;
    assert_eq!(refusal.line, Some(line), "{refusal}");
    assert_eq!(refusal.column, Some(text.lines().nth(line - 1).unwrap().find('1').unwrap() + 1), "{refusal}");
    assert_eq!(limit(accepted.payload()), Ok(&7), "the other section still serves");
}

#[test]
fn a_repeated_key_inside_a_section_refuses_that_section() {
    let payload = serde_json::json!({ "names": ["a"], "limit": 1 });
    let mut text = String::from_utf8(shipped::generate(&descriptor(), &payload).unwrap()).unwrap();
    text = text.replace("\"limit\":1", "\"limit\":{\"x\":1,\"x\":2}");
    let accepted = load(&text).expect("the member list is accepted");
    let refusal = limit(accepted.payload()).expect_err("the repeated key is refused");
    assert_eq!((refusal.reason, refusal.pointer.as_deref()), (FallbackReason::MalformedArtifact, Some("/payload/limit/x")));
}

#[test]
fn a_record_a_section_validator_rejects_is_unusable() {
    let text = generated().replace("\"names\":[\"b\",\"a\"]", "\"names\":[]");
    let accepted = load(&text).expect("accepted");
    let refusal = names(accepted.payload()).expect_err("an empty list is refused by the validator");
    assert_eq!((refusal.reason, refusal.pointer.as_deref()), (FallbackReason::UnusableRecord, Some("/payload/names")));
}

#[cfg(feature = "work-counters")]
#[test]
fn a_refused_section_is_counted_once() {
    use simplified_schema::work_counters::{Work, snapshot};
    let text = generated().replace("\"limit\":7", "\"limit\":\"7\"");
    let accepted = load(&text).expect("accepted");
    let before = snapshot();
    assert!(limit(accepted.payload()).is_err() && limit(accepted.payload()).is_err());
    assert_eq!(snapshot().since(&before).get(Work::ArtifactFallbackMalformed), 1);
}

/// Each row edits the generated text once; every one refuses the whole
/// artifact before any section is read.
macro_rules! refused {
    ($($name:ident: $edit:expr => $reason:ident at $pointer:expr;)*) => {$(
        #[test]
        fn $name() {
            let edit: fn(String) -> String = $edit;
            let refusal = load(&edit(generated())).err().expect("the artifact is refused");
            assert_eq!(refusal.reason, FallbackReason::$reason, "{refusal}");
            assert_eq!(refusal.pointer.as_deref(), $pointer, "{refusal}");
        }
    )*};
}

refused! {
    a_pretty_printed_payload_is_not_the_layout: |text| {
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        String::from_utf8(serde_json::to_vec_pretty(&value).unwrap()).unwrap() + "\n"
    } => MalformedArtifact at None;
    a_member_line_without_its_comma: |text| text.replace("\"a\"],\n", "\"a\"]\n") => MalformedArtifact at None;
    a_comma_after_the_last_member: |text| text.replace("\"limit\":7\n", "\"limit\":7,\n") => MalformedArtifact at None;
    content_after_the_document: |text| text + "{}\n" => MalformedArtifact at None;
    a_missing_final_line_feed: |text| text.trim_end().to_string() => MalformedArtifact at None;
    an_unquoted_member_key: |text| text.replace("\"limit\":7", "limit:7") => MalformedArtifact at None;
    a_repeated_member: |text| text.replace("\"limit\":7", "\"limit\":7,\n\"limit\":8") => MalformedArtifact at Some("/payload/limit");
    an_unknown_member: |text| text.replace("\"limit\":7", "\"limit\":7,\n\"extra\":1") => UnusableRecord at Some("/payload/extra");
    a_missing_member: |text| text.replace(",\n\"limit\":7", "") => UnusableRecord at Some("/payload/limit");
    a_header_digest_mismatch: |text| text.replace(&shipped::Digest::of(b"a: 1\n").to_string(), "0000000000000000") => SourceMismatch at Some("/header/sources/0/digest");
    a_malformed_header_member: |text| text.replace("\"wire_version\": 1", "\"wire_version\": \"1\"") => MalformedArtifact at Some("/header/wire_version");
}
