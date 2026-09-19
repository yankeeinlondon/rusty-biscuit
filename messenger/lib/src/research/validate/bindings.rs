//! Binding rules: SR-FORMAT, SR-IMAGE, SR-ATTRIBUTION, SR-LOCATION,
//! SR-EXPRESSION.

use std::collections::BTreeMap;

use super::{Index, push};
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    ApiAvailability, AttributionRole, AttributionScope, ConstraintKind, ContentRole, DefaultState,
    ExpressionMechanism, ExpressionScope, Fidelity, FieldRelationshipKind, ImageCollection, Intent,
    LocationMode, LocationRole, LocationSubject, PlacementControl, PlatformDocument, State,
    SuppliedBy, Support, YesNo,
};

pub(super) fn check(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    format(d, index, findings);
    images(d, index, findings);
    attribution(d, index, findings);
    location(d, findings);
    expression(d, index, findings);
}

// ---- SR-FORMAT --------------------------------------------------------------

fn format(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (p, profile) in d.format_profiles.iter().enumerate() {
        for (construct, support) in profile.constructs.iter() {
            let pointer = format!("/format_profiles/{p}/constructs/{construct}");
            if support.state == State::Known && support.support.is_none() {
                push(
                    findings,
                    Rule::Format,
                    format!("{pointer}/support"),
                    &profile.id,
                    format!("known construct {construct} names its support; the {} family implies none", profile.family),
                );
            }
            if support.state != State::Known && support.support.is_some() {
                push(
                    findings,
                    Rule::Format,
                    format!("{pointer}/support"),
                    &profile.id,
                    format!("{} construct {construct} carries no support value", support.state),
                );
            }
        }
    }
    for (b, binding) in d.text_bindings.iter().enumerate() {
        let base = format!("/text_bindings/{b}");
        if let Some(selector) = &binding.selector {
            match &selector.default {
                Some(default) if !selector.values.contains(default) => push(
                    findings,
                    Rule::Format,
                    format!("{base}/selector/default"),
                    &binding.id,
                    format!("selector default {default} is not one of its allowed values"),
                ),
                Some(_) if selector.default_state != DefaultState::Documented => push(
                    findings,
                    Rule::Format,
                    format!("{base}/selector/default_state"),
                    &binding.id,
                    "a selector default is documented",
                ),
                None if selector.default_state == DefaultState::Documented => push(
                    findings,
                    Rule::Format,
                    format!("{base}/selector/default"),
                    &binding.id,
                    "a documented selector default names its value",
                ),
                _ => {}
            }
        }
        for (r, relationship) in binding.relationships.iter().enumerate() {
            let pointer = format!("{base}/relationships/{r}");
            let Some(target) = index.text_bindings.get(relationship.target.as_str()) else { continue };
            match relationship.kind {
                FieldRelationshipKind::MutuallyExclusive => {
                    let symmetric = target.relationships.iter().any(|back| {
                        back.kind == FieldRelationshipKind::MutuallyExclusive && back.target == binding.id
                    });
                    if !symmetric {
                        push(
                            findings,
                            Rule::Format,
                            pointer,
                            &binding.id,
                            format!("mutually_exclusive with {} is not declared on both sides", target.id),
                        );
                    }
                }
                FieldRelationshipKind::FallbackFor
                    if target.content_role != ContentRole::Primary || binding.content_role == ContentRole::Primary =>
                {
                    push(
                        findings,
                        Rule::Format,
                        pointer,
                        &binding.id,
                        format!("fallback_for links a fallback to primary content; {} is {}", target.id, target.content_role),
                    );
                }
                _ => {}
            }
        }
    }
}

// ---- SR-IMAGE ---------------------------------------------------------------

fn images(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (c, coverage) in d.role_coverage.iter().enumerate() {
        for (role, cell) in coverage.roles.iter() {
            let pointer = format!("/role_coverage/{c}/roles/{role}/bindings");
            for id in &cell.bindings {
                let Some(binding) = index.image_bindings.get(id.as_str()) else { continue };
                if binding.role != role {
                    push(
                        findings,
                        Rule::Image,
                        pointer.clone(),
                        id,
                        format!("binding has role {} but is listed under {role}", binding.role),
                    );
                }
                if binding.interface != coverage.interface {
                    push(
                        findings,
                        Rule::Image,
                        pointer.clone(),
                        id,
                        format!("binding belongs to {}, not {}", binding.interface, coverage.interface),
                    );
                }
            }
        }
    }
    for (b, binding) in d.image_bindings.iter().enumerate() {
        let base = format!("/image_bindings/{b}");
        let provider_controlled =
            binding.supplied_by == SuppliedBy::ProviderDerived || binding.collection == ImageCollection::ProviderSelected;
        if provider_controlled && binding.placement_control == PlacementControl::Explicit {
            push(
                findings,
                Rule::Image,
                format!("{base}/placement_control"),
                &binding.id,
                "provider-derived or provider-selected placement is never explicit caller control",
            );
        }
        if let (Some(min), Some(max)) = (binding.min_items, binding.max_items)
            && min > max
        {
            push(findings, Rule::Image, format!("{base}/min_items"), &binding.id, format!("min_items {min} exceeds max_items {max}"));
        }
        for shared in &binding.shared_constraints {
            let Some(constraint) = index.constraints.get(shared.as_str()) else { continue };
            if constraint.kind == ConstraintKind::ItemCountMax
                && constraint.knowledge.state == State::Known
                && let (Some(limit), Some(max)) = (constraint.value, binding.max_items)
                && max > limit
            {
                push(
                    findings,
                    Rule::Image,
                    format!("{base}/max_items"),
                    &binding.id,
                    format!("max_items {max} exceeds shared constraint {} ({limit})", constraint.id),
                );
            }
        }
    }
}

// ---- SR-ATTRIBUTION ---------------------------------------------------------

fn attribution(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (a, binding) in d.attribution_bindings.iter().enumerate() {
        let base = format!("/attribution_bindings/{a}");
        let identity_role = matches!(
            binding.role,
            AttributionRole::SenderIdentity | AttributionRole::DelegatedAuthor | AttributionRole::ForwardedOrigin
        );
        if binding.changes_sender == YesNo::Yes && !identity_role {
            push(
                findings,
                Rule::Attribution,
                format!("{base}/changes_sender"),
                &binding.id,
                format!("a {} label never changes the authenticated sender", binding.role),
            );
        }
        if matches!(binding.scope, AttributionScope::Account | AttributionScope::Application)
            && let Some(image) = binding.image_binding.as_deref().and_then(|id| index.image_bindings.get(id))
            && image.supplied_by == SuppliedBy::CallerSupplied
        {
            push(
                findings,
                Rule::Attribution,
                format!("{base}/image_binding"),
                &binding.id,
                format!("a {}-scoped avatar cannot bind the per-message slot {}", binding.scope, image.id),
            );
        }
    }
}

// ---- SR-LOCATION ------------------------------------------------------------

fn location(d: &PlatformDocument, findings: &mut Findings) {
    for (l, binding) in d.location_bindings.iter().enumerate() {
        let base = format!("/location_bindings/{l}");
        if binding.role == LocationRole::SharedPlace && binding.subject == LocationSubject::Author {
            push(findings, Rule::Location, format!("{base}/subject"), &binding.id, "a shared place is not the author's location");
        }
        if binding.role == LocationRole::LiveLocation && binding.mode != LocationMode::Live {
            push(findings, Rule::Location, format!("{base}/mode"), &binding.id, "a live location uses mode: live");
        }
    }
    let mut per_interface: BTreeMap<&str, usize> = BTreeMap::new();
    for (g, answer) in d.author_geolocation.iter().enumerate() {
        let count = per_interface.entry(answer.interface.as_str()).or_default();
        *count += 1;
        if *count > 1 {
            push(
                findings,
                Rule::Location,
                format!("/author_geolocation/{g}"),
                &answer.id,
                format!("{} already has an author-geolocation answer", answer.interface),
            );
        }
    }
}

// ---- SR-EXPRESSION ----------------------------------------------------------

fn expression(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (e, binding) in d.expression_bindings.iter().enumerate() {
        let base = format!("/expression_bindings/{e}");
        if binding.api_availability == ApiAvailability::AppOnly
            && matches!(binding.support, Some(Support::Supported | Support::Conditional))
        {
            push(
                findings,
                Rule::Expression,
                format!("{base}/support"),
                &binding.id,
                "app-only behavior is a research lead, never API support",
            );
        }
        let content_mechanism = matches!(
            binding.mechanism,
            ExpressionMechanism::Reaction | ExpressionMechanism::EmojiContent | ExpressionMechanism::StickerContent
        );
        if content_mechanism
            && matches!(
                binding.scope,
                ExpressionScope::MessageBubble | ExpressionScope::SelectedText | ExpressionScope::Conversation
            )
        {
            push(
                findings,
                Rule::Expression,
                format!("{base}/scope"),
                &binding.id,
                format!("a {} is not a presentation effect with {} scope", binding.mechanism, binding.scope),
            );
        }
        if (binding.intent == Intent::Unmapped) != (binding.intent_fidelity == Fidelity::Unmapped) {
            push(
                findings,
                Rule::Expression,
                format!("{base}/intent_fidelity"),
                &binding.id,
                "intent and intent_fidelity are unmapped together",
            );
        }
        if binding.intent_fidelity == Fidelity::Exact {
            let evidenced = binding.knowledge.state == State::Known
                && binding.native_identifier.is_some()
                && !binding.knowledge.evidence.is_empty()
                && !index.secondary_only(&binding.knowledge.evidence);
            if !evidenced {
                push(
                    findings,
                    Rule::Expression,
                    format!("{base}/intent_fidelity"),
                    &binding.id,
                    "an exact intent mapping names the native effect and cites non-secondary evidence for its meaning",
                );
            }
        }
    }
}
