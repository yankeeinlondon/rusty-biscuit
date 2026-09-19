//! Error rules: SR-ENVELOPE, SR-MATCH, SR-MATCH-OVERLAP, SR-ORIGIN-PHASE,
//! SR-FIXTURES.

use super::replay::{Classification, Response, classify, compatible, executable, strictly_more_specific};
use super::{Index, push};
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    BodyFormat, DeliveryCertainty, Envelope, ErrorOutcome, ErrorPhase, ErrorRecord,
    FixtureExpectation, MatchSignature, Origin, PlatformDocument, ReplaySafety, State,
};

pub(super) fn check(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (e, envelope) in d.envelopes.iter().enumerate() {
        envelope_locators(e, envelope, findings);
    }
    for (e, error) in d.errors.iter().enumerate() {
        let envelope = index.envelopes.get(error.envelope.as_str()).copied();
        if let Some(envelope) = envelope {
            error_envelope(e, error, envelope, findings);
        }
        signatures(e, error, findings);
        origin_phase(e, error, findings);
    }
    overlap(d, findings);
    fixtures(d, index, findings);
}

// ---- SR-ENVELOPE ------------------------------------------------------------

/// Whether a locator form suits a body format.
fn locator_suits(locator: &str, format: BodyFormat) -> bool {
    if locator == "body_text" {
        format == BodyFormat::PlainText
    } else if locator.starts_with("sdk_variant:") {
        format == BodyFormat::SdkError
    } else if locator.starts_with('/') {
        matches!(format, BodyFormat::Json | BodyFormat::JsonRpc | BodyFormat::Unknown)
    } else {
        true
    }
}

fn envelope_locators(e: usize, envelope: &Envelope, findings: &mut Findings) {
    for locator in envelope.locators() {
        if !locator_suits(locator, envelope.body_format) {
            push(
                findings,
                Rule::Envelope,
                format!("/envelopes/{e}"),
                &envelope.id,
                format!("locator {locator} does not suit a {} body", envelope.body_format),
            );
        }
    }
}

fn error_envelope(e: usize, error: &ErrorRecord, envelope: &Envelope, findings: &mut Findings) {
    let base = format!("/errors/{e}");
    if error.origin != envelope.origin {
        push(
            findings,
            Rule::Envelope,
            format!("{base}/origin"),
            &error.id,
            format!("origin {} differs from envelope {} ({})", error.origin, envelope.id, envelope.origin),
        );
    }
    if error.interface != envelope.interface {
        push(
            findings,
            Rule::Envelope,
            format!("{base}/interface"),
            &error.id,
            format!("envelope {} belongs to {}", envelope.id, envelope.interface),
        );
    }
    for operation in &error.operations {
        if !envelope.operations.contains(operation) {
            push(
                findings,
                Rule::Envelope,
                format!("{base}/operations"),
                &error.id,
                format!("envelope {} does not cover operation {operation}", envelope.id),
            );
        }
    }
    for (field, signature) in [("match", &error.signature), ("candidate_match", &error.candidate_match)] {
        let Some(signature) = signature else { continue };
        for problem in unreadable_predicates(signature, envelope) {
            push(findings, Rule::Envelope, format!("{base}/{field}"), &error.id, problem);
        }
    }
}

/// Predicates that read a locator the envelope does not define.
fn unreadable_predicates(signature: &MatchSignature, envelope: &Envelope) -> Vec<String> {
    let mut problems = Vec::new();
    // A JSON Pointer beneath a defined pointer (an element of a field-error
    // tree, say) reads the same envelope field; `/` is authored as the root.
    let defines = |locator: &str| {
        envelope.locators().any(|defined| {
            defined == locator
                || (defined == "/" && locator.starts_with('/'))
                || (defined.starts_with('/') && locator.starts_with(&format!("{defined}/")))
        })
    };
    if let Some(locator) = &signature.discriminator_locator
        && !defines(locator)
        && locator != "http_status"
    {
        problems.push(format!("discriminator {locator} is not a locator of envelope {}", envelope.id));
    }
    let has_code = envelope.code_locator.is_some() || envelope.warnings_locator.is_some();
    if (signature.native_code_number.is_some() || signature.native_code_string.is_some()) && !has_code {
        problems.push(format!("envelope {} defines no code locator", envelope.id));
    }
    if signature.native_subcode.is_some() && envelope.subcode_locator.is_none() {
        problems.push(format!("envelope {} defines no subcode locator", envelope.id));
    }
    if signature.exact_text_token.is_some()
        && envelope.body_format != BodyFormat::PlainText
        && envelope.message_locator.is_none()
    {
        problems.push(format!("envelope {} has no text to match a token against", envelope.id));
    }
    if signature.sdk_error_variant.is_some() && envelope.body_format != BodyFormat::SdkError {
        problems.push(format!("envelope {} is not an SDK error envelope", envelope.id));
    }
    problems
}

// ---- SR-MATCH ---------------------------------------------------------------

fn signatures(e: usize, error: &ErrorRecord, findings: &mut Findings) {
    let base = format!("/errors/{e}");
    match (error.knowledge.state, &error.signature) {
        (State::Known, None) => push(
            findings,
            Rule::Match,
            format!("{base}/match"),
            &error.id,
            "a known error carries an executable match signature",
        ),
        (state, Some(_)) if state != State::Known => push(
            findings,
            Rule::Match,
            format!("{base}/match"),
            &error.id,
            format!("a {state} error may carry only candidate_match, which is never executable"),
        ),
        _ => {}
    }
    for (field, signature) in [("match", &error.signature), ("candidate_match", &error.candidate_match)] {
        let Some(signature) = signature else { continue };
        if signature.predicates().is_empty() {
            push(findings, Rule::Match, format!("{base}/{field}"), &error.id, "a signature has at least one predicate");
        }
        if signature.discriminator_equals.is_some() && signature.discriminator_locator.is_none() {
            push(
                findings,
                Rule::Match,
                format!("{base}/{field}/discriminator_equals"),
                &error.id,
                "discriminator_equals needs discriminator_locator",
            );
        }
    }
}

// ---- SR-MATCH-OVERLAP -------------------------------------------------------

/// Within one interface, operation, version, origin, and phase, two
/// executable signatures may overlap only when one strictly adds
/// predicates to the other.
fn overlap(d: &PlatformDocument, findings: &mut Findings) {
    for (later, b) in d.errors.iter().enumerate() {
        let Some(sb) = executable(b) else { continue };
        for a in &d.errors[..later] {
            let Some(sa) = executable(a) else { continue };
            let same_scope = a.interface == b.interface
                && a.origin == b.origin
                && a.phase == b.phase
                && a.applies_when == b.applies_when
                && a.operations.iter().any(|op| b.operations.contains(op));
            if same_scope
                && compatible(sa, sb)
                && !strictly_more_specific(sa, sb)
                && !strictly_more_specific(sb, sa)
            {
                push(
                    findings,
                    Rule::MatchOverlap,
                    format!("/errors/{later}/match"),
                    &b.id,
                    format!("signature overlaps {} and neither is strictly more specific", a.id),
                );
            }
        }
    }
}

// ---- SR-ORIGIN-PHASE --------------------------------------------------------

fn origin_phase(e: usize, error: &ErrorRecord, findings: &mut Findings) {
    let pointer = format!("/errors/{e}/delivery_certainty");
    if error.phase == ErrorPhase::BeforeSubmission && error.delivery_certainty != DeliveryCertainty::NotSubmitted {
        push(
            findings,
            Rule::OriginPhase,
            pointer.clone(),
            &error.id,
            format!("a before-submission failure is not_submitted, not {}", error.delivery_certainty),
        );
    }
    if error.outcome == ErrorOutcome::Warning && error.delivery_certainty == DeliveryCertainty::Rejected {
        push(findings, Rule::OriginPhase, pointer.clone(), &error.id, "a warning accompanies a send; it is never a rejection");
    }
    if error.origin == Origin::Transport && error.delivery_certainty == DeliveryCertainty::Rejected {
        push(
            findings,
            Rule::OriginPhase,
            pointer,
            &error.id,
            "a transport failure does not prove the service rejected the message",
        );
    }
    if error.replay_safety == ReplaySafety::Safe
        && !matches!(error.delivery_certainty, DeliveryCertainty::NotSubmitted | DeliveryCertainty::Rejected)
    {
        push(
            findings,
            Rule::OriginPhase,
            format!("/errors/{e}/replay_safety"),
            &error.id,
            format!("replay is not established safe when delivery is {}", error.delivery_certainty),
        );
    }
}

// ---- SR-FIXTURES ------------------------------------------------------------

fn fixtures(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (f, fixture) in d.error_fixtures.iter().enumerate() {
        let Some(envelope) = index.envelopes.get(fixture.envelope.as_str()) else { continue };
        let pointer = format!("/error_fixtures/{f}");
        if !envelope.operations.contains(&fixture.operation) {
            push(
                findings,
                Rule::Fixtures,
                format!("{pointer}/operation"),
                &fixture.id,
                format!("envelope {} does not cover operation {}", envelope.id, fixture.operation),
            );
        }
        let outcome = classify(envelope, &d.errors, &fixture.operation, Response::from(fixture));
        let expected = match fixture.expect {
            FixtureExpectation::Match => match &fixture.expected_error {
                Some(error) => Classification::Matched(error.clone()),
                None => {
                    push(findings, Rule::Fixtures, format!("{pointer}/expected_error"), &fixture.id, "a match fixture names its expected error");
                    continue;
                }
            },
            FixtureExpectation::NoMatch | FixtureExpectation::Unknown => Classification::Unknown,
        };
        if outcome != expected {
            push(
                findings,
                Rule::Fixtures,
                pointer,
                &fixture.id,
                format!("replay classifies as {} but the fixture expects {}", describe(&outcome), describe(&expected)),
            );
        }
    }
    for (e, error) in d.errors.iter().enumerate() {
        for fixture in &error.fixtures {
            if let Some(fixture) = index.error_fixtures.get(fixture.as_str())
                && fixture.envelope != error.envelope
            {
                push(
                    findings,
                    Rule::Fixtures,
                    format!("/errors/{e}/fixtures"),
                    &error.id,
                    format!("fixture {} replays envelope {}, not {}", fixture.id, fixture.envelope, error.envelope),
                );
            }
        }
    }
}

fn describe(classification: &Classification) -> String {
    match classification {
        Classification::Matched(id) => format!("a match for {id}"),
        Classification::Unknown => "unknown".to_string(),
        Classification::Ambiguous(ids) => format!("ambiguous ({})", ids.join(", ")),
    }
}
