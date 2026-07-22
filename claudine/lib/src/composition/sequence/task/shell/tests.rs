use super::{RunawayTrip, RunawayTripKind, Utf8Stream};
use crate::runaway::CaptureVolumeCap;

/// The documented tie-break: one chunk crossing both limits reports the
/// line limit, because there is no observable "first" within a chunk.
#[test]
fn a_chunk_that_crosses_both_limits_reports_the_line_limit() {
    let cap = CaptureVolumeCap::new(true, 16, 1024);

    let trip = RunawayTrip::from_counters(&cap, 400, 8192);

    assert_eq!(
        trip,
        RunawayTrip {
            kind: RunawayTripKind::Lines,
            observed: 400,
            limit: 16,
        },
    );
}

/// A byte-only breach reports the byte limit with the byte counter.
#[test]
fn a_byte_only_breach_reports_the_byte_limit() {
    let cap = CaptureVolumeCap::new(true, 50_000, 1024);

    let trip = RunawayTrip::from_counters(&cap, 3, 4096);

    assert_eq!(
        trip,
        RunawayTrip {
            kind: RunawayTripKind::Bytes,
            observed: 4096,
            limit: 1024,
        },
    );
}

/// The rendered form is what the runaway diagnostic embeds, so observed
/// and limit must both be legible from it.
#[test]
fn a_trip_displays_observed_count_and_limit() {
    let lines = RunawayTrip {
        kind: RunawayTripKind::Lines,
        observed: 50_417,
        limit: 50_000,
    };
    let bytes = RunawayTrip {
        kind: RunawayTripKind::Bytes,
        observed: 40_960,
        limit: 32_768,
    };

    assert_eq!(lines.to_string(), "50417 lines, limit 50000");
    assert_eq!(bytes.to_string(), "40960 bytes, limit 32768");
}

/// The chunk boundary a fixed-size reader actually produces: mid-character.
#[test]
fn a_code_point_split_across_chunks_is_decoded_whole() {
    let bytes = "é".as_bytes();
    let mut stream = Utf8Stream::default();

    assert_eq!(stream.push(&bytes[..1]), "", "a partial code point is held");
    assert_eq!(stream.push(&bytes[1..]), "é");
    assert_eq!(stream.finish(), "");
}

/// A three-byte character split at both possible boundaries.
#[test]
fn a_three_byte_code_point_survives_either_split() {
    for split in 1..3 {
        let bytes = "日".as_bytes();
        let mut stream = Utf8Stream::default();
        let decoded = format!("{}{}", stream.push(&bytes[..split]), stream.push(&bytes[split..]));
        assert_eq!(decoded, "日", "split after {split} byte(s)");
    }
}

/// Text either side of a held fragment still comes through in one piece.
#[test]
fn complete_text_around_a_held_fragment_is_emitted_immediately() {
    let mut stream = Utf8Stream::default();
    let mut bytes = b"ready ".to_vec();
    bytes.extend_from_slice(&"✅".as_bytes()[..2]);

    assert_eq!(stream.push(&bytes), "ready ");
    assert_eq!(stream.push(&"✅".as_bytes()[2..]), "✅");
}

/// A byte that no continuation can rescue is replaced at once rather than
/// held — otherwise a binary-emitting command would stall the stream.
#[test]
fn a_genuinely_invalid_byte_is_replaced_without_being_held() {
    let mut stream = Utf8Stream::default();

    assert_eq!(stream.push(b"a\xffb"), "a\u{FFFD}b");
    assert_eq!(stream.finish(), "", "nothing was left held");
}

/// An incomplete tail at end of stream is released rather than dropped.
#[test]
fn an_incomplete_tail_is_released_at_end_of_stream() {
    let mut stream = Utf8Stream::default();

    assert_eq!(stream.push(&"€".as_bytes()[..2]), "");
    assert_eq!(stream.finish(), "\u{FFFD}");
}
