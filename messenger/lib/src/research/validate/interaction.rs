//! Interaction rules (SR-INTERACTIVITY): inbound reach, typed questions,
//! form packaging, companion interfaces, and answer-fixture replay.

use super::replay::{InteractionReplay, replay_interaction};
use super::{Index, push};
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    AnswerType, Direction, FormContainer, InboundContent, InteractionExpectation, InterfaceRole,
    Nativeness, PlatformDocument, QuestionKind, QuestionMechanism, Responses, Submission,
};

pub(super) fn check(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    inbound(d, index, findings);
    questions(d, index, findings);
    forms(d, index, findings);
    fixtures(d, index, findings);
}

/// A callback-only or receive-only companion cannot claim general text.
fn inbound(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (i, binding) in d.inbound_bindings.iter().enumerate() {
        let Some(interface) = index.interfaces.get(binding.interface.as_str()) else { continue };
        let general_text = matches!(binding.content, InboundContent::FullText | InboundContent::PartialText);
        if interface.direction == Direction::CallbackOnly && general_text {
            push(
                findings,
                Rule::Interactivity,
                format!("/inbound_bindings/{i}/content"),
                &binding.id,
                format!("callback-only {} delivers interactions, not general conversation text", interface.interface_id),
            );
        }
    }
}

fn companion(
    pointer: String,
    subject: &str,
    companion: Option<&str>,
    index: &Index<'_>,
    findings: &mut Findings,
) {
    let Some(companion) = companion else { return };
    let declared = index
        .interfaces
        .get(companion)
        .is_some_and(|interface| interface.role == InterfaceRole::ResearchOnly);
    if !declared {
        push(
            findings,
            Rule::Interactivity,
            pointer,
            subject,
            format!("companion {companion} is not a declared research-only interface"),
        );
    }
}

fn questions(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (q, question) in d.question_bindings.iter().enumerate() {
        let base = format!("/question_bindings/{q}");
        let id = question.id.as_str();
        companion(format!("{base}/companion_interface"), id, question.companion_interface.as_deref(), index, findings);

        let expected = match question.kind {
            QuestionKind::Confirmation => AnswerType::Boolean,
            QuestionKind::SingleChoice => AnswerType::OptionId,
            QuestionKind::MultipleChoice => AnswerType::OptionIdArray,
            QuestionKind::TextInput => AnswerType::String,
        };
        if question.mechanism == QuestionMechanism::LinkButton {
            if question.answer_type != AnswerType::None || question.answer_locator.is_some() {
                push(
                    findings,
                    Rule::Interactivity,
                    format!("{base}/answer_type"),
                    id,
                    "a link button opens a URL and returns no answer",
                );
            }
        } else if question.answer_type != expected {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/answer_type"),
                id,
                format!("a {} question answers with {expected}, not {}", question.kind, question.answer_type),
            );
        }
        if question.kind == QuestionKind::Confirmation
            && question.mechanism != QuestionMechanism::LinkButton
            && question.confirmation_mapping.is_none()
        {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/confirmation_mapping"),
                id,
                "a confirmation maps its affirmative and negative answers",
            );
        }
        let interpreted = matches!(
            question.mechanism,
            QuestionMechanism::FreeTextInterpretation | QuestionMechanism::ReactionInterpretation
        );
        if interpreted && question.native == Nativeness::Native {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/native"),
                id,
                format!("{} is application-managed, never a native structured reply", question.mechanism),
            );
        }
        if let (Some(min), Some(max)) = (question.min_selections, question.max_selections)
            && min > max
        {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/min_selections"),
                id,
                format!("min_selections {min} exceeds max_selections {max}"),
            );
        }
        if question.responses == Responses::AggregateOnly && question.responder_locator.is_some() {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/responder_locator"),
                id,
                "aggregate-only results identify no responder",
            );
        }
    }
}

fn forms(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (f, form) in d.form_bindings.iter().enumerate() {
        let base = format!("/form_bindings/{f}");
        companion(format!("{base}/companion_interface"), &form.id, form.companion_interface.as_deref(), index, findings);
        if form.container == FormContainer::MessageControls
            && matches!(form.submission, Submission::SingleEvent | Submission::Both)
        {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/submission"),
                &form.id,
                "grouped message controls submit per field; only a native container submits in one event",
            );
        }
        if let Some(max) = form.max_fields
            && form.fields.len() as u64 > max
        {
            push(
                findings,
                Rule::Interactivity,
                format!("{base}/fields"),
                &form.id,
                format!("{} fields exceed max_fields {max}", form.fields.len()),
            );
        }
        for (n, field) in form.fields.iter().enumerate() {
            if !form.question_kinds.contains(&field.kind) {
                push(
                    findings,
                    Rule::Interactivity,
                    format!("{base}/fields/{n}/kind"),
                    &form.id,
                    format!("field {} is a {} but the form lists no such kind", field.id, field.kind),
                );
            }
        }
    }
}

/// Replays `answer` fixtures against their question bindings.
fn fixtures(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (x, fixture) in d.interaction_fixtures.iter().enumerate() {
        if fixture.expect != InteractionExpectation::Answer {
            continue;
        }
        let Some(question) = index.question_bindings.get(fixture.binding.as_str()) else { continue };
        let pointer = format!("/interaction_fixtures/{x}");
        match replay_interaction(question, &fixture.payload) {
            InteractionReplay::Answer(answer) => {
                if let Some(expected) = &fixture.expected_answer {
                    let expected: Option<serde_json::Value> = serde_json::from_str(expected).ok();
                    if expected.as_ref() != Some(&answer) {
                        push(
                            findings,
                            Rule::Interactivity,
                            format!("{pointer}/expected_answer"),
                            &fixture.id,
                            "replayed answer differs from expected_answer",
                        );
                    }
                }
            }
            InteractionReplay::NotReplayable => {}
            InteractionReplay::NoAnswer => push(
                findings,
                Rule::Interactivity,
                format!("{pointer}/payload"),
                &fixture.id,
                format!("payload carries no {} answer at the binding's answer locator", question.answer_type),
            ),
        }
    }
}
