//! Every alternative of the imported `BetaResponseStreamEvent` decodes as
//! itself, with its payload, when its documented `type` is on the wire.
//!
//! The union is an `anyOf` with `discriminator: type` in the vendored spec.
//! Decoded first-match, every event that carries an audio event's few fields
//! (51 of 53) became an audio event.

use schematic_definitions::openai::*;
use serde_json::{Map, Value};

/// Calls `$visit!(Alternative)` for every variant of the enum, in declaration
/// order. The exhaustive `match` in [`variant_name`] fails to compile when the
/// enum gains a variant this list lacks.
macro_rules! stream_event_alternatives {
    ($visit:ident) => {
        $visit! {
            BetaResponseAudioDeltaEvent, BetaResponseAudioDoneEvent, BetaResponseAudioTranscriptDeltaEvent,
            BetaResponseAudioTranscriptDoneEvent, BetaResponseCodeInterpreterCallCodeDeltaEvent,
            BetaResponseCodeInterpreterCallCodeDoneEvent, BetaResponseCodeInterpreterCallCompletedEvent,
            BetaResponseCodeInterpreterCallInProgressEvent, BetaResponseCodeInterpreterCallInterpretingEvent,
            BetaResponseCompletedEvent, BetaResponseContentPartAddedEvent, BetaResponseContentPartDoneEvent,
            BetaResponseCreatedEvent, BetaResponseErrorEvent, BetaResponseFileSearchCallCompletedEvent,
            BetaResponseFileSearchCallInProgressEvent, BetaResponseFileSearchCallSearchingEvent,
            BetaResponseFunctionCallArgumentsDeltaEvent, BetaResponseFunctionCallArgumentsDoneEvent,
            BetaResponseInProgressEvent, BetaResponseFailedEvent, BetaResponseIncompleteEvent,
            BetaResponseOutputItemAddedEvent, BetaResponseOutputItemDoneEvent,
            BetaResponseReasoningSummaryPartAddedEvent, BetaResponseReasoningSummaryPartDoneEvent,
            BetaResponseReasoningSummaryTextDeltaEvent, BetaResponseReasoningSummaryTextDoneEvent,
            BetaResponseReasoningTextDeltaEvent, BetaResponseReasoningTextDoneEvent, BetaResponseRefusalDeltaEvent,
            BetaResponseRefusalDoneEvent, BetaResponseTextDeltaEvent, BetaResponseTextDoneEvent,
            BetaResponseWebSearchCallCompletedEvent, BetaResponseWebSearchCallInProgressEvent,
            BetaResponseWebSearchCallSearchingEvent, BetaResponseImageGenCallCompletedEvent,
            BetaResponseImageGenCallGeneratingEvent, BetaResponseImageGenCallInProgressEvent,
            BetaResponseImageGenCallPartialImageEvent, BetaResponseMCPCallArgumentsDeltaEvent,
            BetaResponseMCPCallArgumentsDoneEvent, BetaResponseMCPCallCompletedEvent, BetaResponseMCPCallFailedEvent,
            BetaResponseMCPCallInProgressEvent, BetaResponseMCPListToolsCompletedEvent,
            BetaResponseMCPListToolsFailedEvent, BetaResponseMCPListToolsInProgressEvent,
            BetaResponseOutputTextAnnotationAddedEvent, BetaResponseQueuedEvent,
            BetaResponseCustomToolCallInputDeltaEvent, BetaResponseCustomToolCallInputDoneEvent
        }
    };
}

fn variant_name(event: &BetaResponseStreamEvent) -> &'static str {
    macro_rules! arms {
        ($($alternative:ident),*) => {
            match event {
                $(BetaResponseStreamEvent::$alternative(_) => stringify!($alternative),)*
            }
        };
    }
    stream_event_alternatives!(arms)
}

/// One value per alternative, in order: its name and a JSON value that the
/// alternative's own struct accepts, carrying `tag` as `type` and a distinct
/// value in each top-level string or integer member it accepts one in.
fn payloads(tag_of: impl Fn(&str) -> String) -> Vec<(&'static str, Value)> {
    fn distinct<T: serde::Serialize + serde::de::DeserializeOwned + Default>(tag: String) -> Value {
        let Value::Object(mut object) = serde_json::to_value(T::default()).expect("serializes") else {
            panic!("an event serializes as an object")
        };
        object.insert("type".into(), Value::String(tag));
        // A defaulted nested union has an empty tag, which no longer decodes.
        for (member, value) in object.iter_mut() {
            if value.get("type") == Some(&Value::String(String::new())) {
                *value = nested_union(member);
            }
        }
        let keys: Vec<String> = object.keys().filter(|key| *key != "type").cloned().collect();
        for (index, key) in keys.into_iter().enumerate() {
            let marker = match &object[&key] {
                Value::String(_) => Value::String(format!("{key}-{index}")),
                Value::Number(number) if number.is_i64() => Value::from(index as i64 + 7),
                _ => continue,
            };
            // A member typed as an enum or a constrained alias rejects the
            // marker; it keeps its default.
            let mut candidate: Map<String, Value> = object.clone();
            candidate.insert(key.clone(), marker);
            if serde_json::from_value::<T>(Value::Object(candidate.clone())).is_ok() {
                object = candidate;
            }
        }
        Value::Object(object)
    }

    macro_rules! collect {
        ($($alternative:ident),*) => {
            vec![$((stringify!($alternative), distinct::<$alternative>(tag_of(stringify!($alternative)))),)*]
        };
    }
    stream_event_alternatives!(collect)
}

/// A documented value for a stream event member that holds a union.
fn nested_union(member: &str) -> Value {
    let value = match member {
        "part" => serde_json::to_value(BetaOutputContent::BetaOutputTextContent(BetaOutputTextContent {
            type_: "output_text".into(),
            text: "hi".into(),
            ..Default::default()
        })),
        "item" => serde_json::to_value(BetaOutputItem::BetaOutputMessage(BetaOutputMessage {
            id: "msg_1".into(),
            type_: "message".into(),
            role: "assistant".into(),
            status: "completed".into(),
            ..Default::default()
        })),
        other => panic!("no documented value for the union member `{other}`"),
    };
    value.expect("serializes")
}

/// The block of the component `name` in the vendored spec, without its header.
fn component<'a>(spec: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("    {name}:");
    let mut lines = spec.lines().skip_while(|line| *line != header);
    assert!(lines.next().is_some(), "the spec declares component `{name}`");
    lines.take_while(|line| line.is_empty() || line.starts_with("     ")).collect()
}

/// The component names of `name`'s `anyOf` alternatives.
fn any_of(spec: &str, name: &str) -> Vec<String> {
    component(spec, name)
        .into_iter()
        .filter_map(|line| line.trim().strip_prefix("- $ref: \"#/components/schemas/")?.strip_suffix('"').map(str::to_string))
        .collect()
}

/// The single documented value of `name`'s `type` property.
fn documented_type(spec: &str, name: &str) -> String {
    let block = component(spec, name);
    let start = block.iter().position(|line| *line == "        type:").unwrap_or_else(|| panic!("`{name}` declares `type`"));
    let property = block[start + 1..].iter().take_while(|line| line.starts_with("          "));
    let mut values = property.skip_while(|line| line.trim() != "enum:").skip(1).map_while(|line| line.trim().strip_prefix("- "));
    let value = values.next().unwrap_or_else(|| panic!("`{name}` documents one `type` value"));
    assert_eq!(values.next(), None, "`{name}` documents one `type` value");
    value.trim_matches('"').to_string()
}

#[test]
fn stream_event_alternatives_match_the_vendored_spec_and_decode_as_themselves() {
    let spec = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("../specs/openai/openapi.yaml"))
        .expect("read the vendored OpenAI spec");
    let declared = any_of(&spec, "BetaResponseStreamEvent");
    let payloads = payloads(|name| documented_type(&spec, name));

    let listed: Vec<&str> = payloads.iter().map(|(name, _)| *name).collect();
    assert_eq!(listed, declared, "the guard lists exactly the spec's `anyOf` alternatives, in order");
    assert_eq!(listed.len(), 53);

    let mut misdecoded = Vec::new();
    for (name, payload) in &payloads {
        let decoded: BetaResponseStreamEvent =
            serde_json::from_value(payload.clone()).unwrap_or_else(|error| panic!("{name} does not decode: {error}"));
        let retained = serde_json::to_value(&decoded).expect("serializes");
        if variant_name(&decoded) != *name || retained != *payload {
            misdecoded.push(format!("{name} -> {} ({retained})", variant_name(&decoded)));
        }
    }
    assert!(misdecoded.is_empty(), "{} alternatives decode wrongly:\n{}", misdecoded.len(), misdecoded.join("\n"));
}

#[test]
fn stream_event_rejects_an_undocumented_or_missing_type() {
    let unknown = serde_json::from_str::<BetaResponseStreamEvent>(r#"{"type":"response.future","sequence_number":1}"#)
        .expect_err("an undocumented type is not some other event");
    let missing = serde_json::from_str::<BetaResponseStreamEvent>(r#"{"delta":"hi","sequence_number":1}"#)
        .expect_err("an event without a type is not an audio delta");

    assert!(unknown.to_string().contains("unknown `type` value `response.future` for `BetaResponseStreamEvent`"), "{unknown}");
    assert!(missing.to_string().contains("missing field `type`"), "{missing}");
}

#[test]
fn stream_event_text_delta_keeps_its_identity_and_log_probabilities() {
    let line = r#"{"type":"response.output_text.delta","item_id":"i","output_index":0,"content_index":0,"delta":"hi","sequence_number":3,"logprobs":[]}"#;

    let event: BetaResponseStreamEvent = serde_json::from_str(line).expect("the review's text event decodes");

    let BetaResponseStreamEvent::BetaResponseTextDeltaEvent(delta) = event else {
        panic!("expected a text delta, got {}", variant_name(&event))
    };
    assert_eq!((delta.item_id.as_str(), delta.delta.as_str(), delta.sequence_number), ("i", "hi", 3));
    assert!(delta.logprobs.is_empty());
}
