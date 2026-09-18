//! Identity rules: SR-ROSTER, SR-CURATED, SR-UNIQUE, SR-REF, SR-EVIDENCE,
//! and version findings.

use std::collections::{BTreeMap, BTreeSet};

use super::{Context, Index, Scope, fact_ids, interface_scoped, knowledge_blocks, push};
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    AdapterId, CuratedReview, InterfaceRole, PlatformDocument, PlatformId, Roster, State, Versioning,
};

pub(super) fn check(d: &PlatformDocument, index: &Index<'_>, context: &Context<'_>, findings: &mut Findings) {
    roster_coverage(d, context, findings);
    interfaces(d, findings);
    uniqueness(d, findings);
    references(d, index, findings);
    evidence(d, index, findings);
    versions(d, findings);
}

// ---- SR-ROSTER --------------------------------------------------------------

fn roster_coverage(d: &PlatformDocument, context: &Context<'_>, findings: &mut Findings) {
    let Some(roster) = context.roster else { return };
    let Some(platform) = roster.active_platforms().find(|p| p.platform_id == d.platform_id) else {
        push(
            findings,
            Rule::Roster,
            "/platform_id",
            d.platform_id.as_str(),
            "platform is not an active roster platform",
        );
        return;
    };
    for (position, interface) in d.interfaces.iter().enumerate() {
        let pointer = format!("/interfaces/{position}");
        let Some(expected) = platform.interface(&interface.interface_id) else {
            push(
                findings,
                Rule::Roster,
                pointer,
                &interface.interface_id,
                format!("interface is not in the roster for {}", d.platform_id),
            );
            continue;
        };
        if expected.role != interface.role {
            push(
                findings,
                Rule::Roster,
                format!("{pointer}/role"),
                &interface.interface_id,
                format!("role {} differs from the roster's {}", interface.role, expected.role),
            );
        }
        let authored: BTreeSet<AdapterId> = interface.adapters.iter().copied().collect();
        let rostered: BTreeSet<AdapterId> = expected.adapters.iter().copied().collect();
        if authored != rostered {
            push(
                findings,
                Rule::Roster,
                format!("{pointer}/adapters"),
                &interface.interface_id,
                format!("adapters {} differ from the roster's {}", list(&authored), list(&rostered)),
            );
        }
    }
    if context.scope == Scope::Accepted {
        for expected in &platform.interfaces {
            if !d.interfaces.iter().any(|i| i.interface_id == expected.interface_id) {
                push(
                    findings,
                    Rule::Roster,
                    "/interfaces",
                    &expected.interface_id,
                    "roster interface is missing from the document",
                );
            }
        }
    }
}

/// Document-local interface identity: sending interfaces map adapters of
/// this platform; research-only companions map none.
fn interfaces(d: &PlatformDocument, findings: &mut Findings) {
    for (position, interface) in d.interfaces.iter().enumerate() {
        let pointer = format!("/interfaces/{position}/adapters");
        match interface.role {
            InterfaceRole::SendingAdapter if interface.adapters.is_empty() => push(
                findings,
                Rule::Roster,
                pointer.clone(),
                &interface.interface_id,
                "a sending interface maps at least one adapter",
            ),
            InterfaceRole::ResearchOnly if !interface.adapters.is_empty() => push(
                findings,
                Rule::Roster,
                pointer.clone(),
                &interface.interface_id,
                "a research-only interface maps no adapter",
            ),
            _ => {}
        }
        for adapter in &interface.adapters {
            if adapter.platform() != d.platform_id {
                push(
                    findings,
                    Rule::Roster,
                    pointer.clone(),
                    &interface.interface_id,
                    format!("adapter {adapter} belongs to {}", adapter.platform()),
                );
            }
        }
    }
}

/// Roster file rules (SR-ROSTER, SR-CURATED).
pub(super) fn check_roster(roster: &Roster, scope: Scope, findings: &mut Findings) {
    let mut adapters: BTreeMap<AdapterId, usize> = BTreeMap::new();
    let mut platforms: BTreeMap<PlatformId, usize> = BTreeMap::new();
    let mut interface_ids: BTreeSet<&str> = BTreeSet::new();
    for (p, platform) in roster.platforms.iter().enumerate() {
        let base = format!("/platforms/{p}");
        let id = platform.platform_id.as_str();
        *platforms.entry(platform.platform_id).or_default() += 1;
        if platform.file != format!("{id}.md") {
            push(findings, Rule::Roster, format!("{base}/file"), id, format!("document file must be {id}.md"));
        }
        for (i, interface) in platform.interfaces.iter().enumerate() {
            let pointer = format!("{base}/interfaces/{i}");
            if !interface_ids.insert(interface.interface_id.as_str()) {
                push(findings, Rule::Roster, pointer.clone(), &interface.interface_id, "interface listed twice");
            }
            match interface.role {
                InterfaceRole::SendingAdapter if interface.adapters.is_empty() => push(
                    findings,
                    Rule::Roster,
                    format!("{pointer}/adapters"),
                    &interface.interface_id,
                    "a sending interface maps at least one adapter",
                ),
                InterfaceRole::ResearchOnly if !interface.adapters.is_empty() => push(
                    findings,
                    Rule::Roster,
                    format!("{pointer}/adapters"),
                    &interface.interface_id,
                    "a research-only companion maps no adapter",
                ),
                _ => {}
            }
            for adapter in &interface.adapters {
                *adapters.entry(*adapter).or_default() += 1;
                if adapter.platform() != platform.platform_id {
                    push(
                        findings,
                        Rule::Roster,
                        format!("{pointer}/adapters"),
                        &interface.interface_id,
                        format!("adapter {adapter} belongs to {}", adapter.platform()),
                    );
                }
            }
            for (r, relationship) in interface.relationships.iter().enumerate() {
                if platform.interface(&relationship.target).is_none() {
                    push(
                        findings,
                        Rule::Ref,
                        format!("{pointer}/relationships/{r}/target"),
                        &interface.interface_id,
                        format!("relationship target {} is not an interface of {id}", relationship.target),
                    );
                }
            }
        }
        curated(roster, p, findings);
    }
    for (platform, count) in &platforms {
        if *count > 1 {
            push(findings, Rule::Roster, "/platforms", platform.as_str(), "platform listed twice");
        }
    }
    for (adapter, count) in &adapters {
        if *count > 1 {
            push(findings, Rule::Roster, "/platforms", adapter.as_str(), format!("adapter mapped {count} times"));
        }
    }
    let excluded: BTreeSet<&str> = roster.excluded.iter().map(|e| e.subject.as_str()).collect();
    for platform in roster.active_platforms() {
        if excluded.contains(platform.platform_id.as_str()) {
            push(findings, Rule::Roster, "/excluded", platform.platform_id.as_str(), "an excluded subject is an active platform");
        }
        for interface in &platform.interfaces {
            if excluded.contains(interface.interface_id.as_str()) {
                push(findings, Rule::Roster, "/excluded", &interface.interface_id, "an excluded subject is an active interface");
            }
        }
    }
    if scope == Scope::Accepted {
        for platform in PlatformId::ALL {
            if !roster.active_platforms().any(|p| p.platform_id == *platform) {
                push(findings, Rule::Roster, "/platforms", platform.as_str(), "active roster platform is missing");
            }
        }
        for adapter in AdapterId::ALL {
            if !adapters.contains_key(adapter) {
                push(findings, Rule::Roster, "/platforms", adapter.as_str(), "adapter is not mapped by any sending interface");
            }
        }
    }
}

/// SR-CURATED for one roster platform.
fn curated(roster: &Roster, p: usize, findings: &mut Findings) {
    let platform = &roster.platforms[p];
    let base = format!("/platforms/{p}/curated_sources");
    let id = platform.platform_id.as_str();
    let cap = roster.curated_source_cap as usize;
    if platform.curated_sources.len() > cap {
        push(
            findings,
            Rule::Curated,
            base.clone(),
            id,
            format!("{} curated sources exceed the configured cap of {cap}", platform.curated_sources.len()),
        );
    }
    let mut urls = BTreeSet::new();
    for (s, source) in platform.curated_sources.iter().enumerate() {
        let pointer = format!("{base}/{s}");
        if !urls.insert(source.url.as_str()) {
            push(findings, Rule::Curated, format!("{pointer}/url"), id, "curated URL listed twice");
        }
        for interface in &source.interfaces {
            if platform.interface(interface).is_none() {
                push(
                    findings,
                    Rule::Curated,
                    format!("{pointer}/interfaces"),
                    id,
                    format!("curated source names {interface}, which is not an interface of {id}"),
                );
            }
        }
        if source.review == CuratedReview::Approved && (source.approved_by.is_none() || source.approved_on.is_none()) {
            push(
                findings,
                Rule::Curated,
                pointer,
                id,
                "an approved curated source records approved_by and approved_on",
            );
        }
    }
}

/// SR-ROSTER over the whole fleet: one document per active platform.
pub(super) fn check_fleet(roster: &Roster, documents: &[PlatformId], findings: &mut Findings) {
    for platform in roster.active_platforms() {
        let count = documents.iter().filter(|d| **d == platform.platform_id).count();
        if count != 1 {
            push(
                findings,
                Rule::Roster,
                "/platforms",
                platform.platform_id.as_str(),
                format!("expected exactly one accepted document, found {count}"),
            );
        }
    }
    for document in documents {
        if !roster.active_platforms().any(|p| p.platform_id == *document) {
            push(findings, Rule::Roster, "/platforms", document.as_str(), "document for a platform outside the active roster");
        }
    }
}

// ---- SR-UNIQUE --------------------------------------------------------------

fn uniqueness(d: &PlatformDocument, findings: &mut Findings) {
    duplicates(d.sources.iter().enumerate().map(|(i, s)| (format!("/sources/{i}/id"), s.id.as_str())), "source", findings);
    duplicates(
        d.interfaces.iter().enumerate().map(|(i, s)| (format!("/interfaces/{i}/interface_id"), s.interface_id.as_str())),
        "interface",
        findings,
    );
    duplicates(fact_ids(d).into_iter().map(|(pointer, id)| (format!("{pointer}/id"), id)), "fact", findings);
    duplicates(d.gaps.iter().enumerate().map(|(i, g)| (format!("/gaps/{i}/id"), g.id.as_str())), "gap", findings);
    duplicates(d.changes.iter().enumerate().map(|(i, c)| (format!("/changes/{i}/id"), c.id.as_str())), "change", findings);
    duplicates(
        d.coverage.iter().enumerate().map(|(i, c)| (format!("/coverage/{i}/interface"), c.interface.as_str())),
        "coverage matrix for interface",
        findings,
    );
    duplicates(
        d.role_coverage.iter().enumerate().map(|(i, c)| (format!("/role_coverage/{i}/interface"), c.interface.as_str())),
        "image-role matrix for interface",
        findings,
    );

    // Two records for one scoped bound need distinguishing conditions;
    // records that differ only in conditions are SR-APPLICABILITY's concern.
    // Unit and stage are part of the scope: 2000 scalars and 4000 UTF-8
    // bytes on one field are two simultaneous bounds, not a duplicate.
    let mut scoped: BTreeMap<ScopedBound<'_>, Vec<usize>> = BTreeMap::new();
    for (i, c) in d.constraints.iter().enumerate() {
        scoped
            .entry((
                c.interface.as_str(),
                c.operation.as_str(),
                c.surface.as_str(),
                c.kind.as_str(),
                c.unit.as_str(),
                c.measurement_stage.as_str(),
            ))
            .or_default()
            .push(i);
    }
    for positions in scoped.values() {
        for (n, &later) in positions.iter().enumerate() {
            for &earlier in &positions[..n] {
                let (a, b) = (&d.constraints[earlier], &d.constraints[later]);
                if a.applies_when == b.applies_when && a.knowledge.state == b.knowledge.state {
                    push(
                        findings,
                        Rule::Unique,
                        format!("/constraints/{later}"),
                        &b.id,
                        format!("same scoped bound as {} with no distinguishing condition", a.id),
                    );
                }
            }
        }
    }
}

/// `(interface, operation, surface, kind, unit, stage)`.
type ScopedBound<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str);

fn duplicates<'a>(items: impl Iterator<Item = (String, &'a str)>, what: &str, findings: &mut Findings) {
    let mut seen = BTreeSet::new();
    for (pointer, id) in items {
        if !seen.insert(id) {
            push(findings, Rule::Unique, pointer, id, format!("duplicate {what} ID"));
        }
    }
}

// ---- SR-REF -----------------------------------------------------------------

fn references(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    let mut refs = Refs { index, findings };

    for (pointer, id, interface) in interface_scoped(d) {
        refs.interface(&pointer, id, interface);
    }
    for (i, coverage) in d.coverage.iter().enumerate() {
        refs.interface(&format!("/coverage/{i}/interface"), &coverage.interface, &coverage.interface);
        for (category, cell) in coverage.categories.cells() {
            refs.gap_opt(&format!("/coverage/{i}/categories/{category}/gap"), &coverage.interface, cell.gap.as_deref());
        }
    }
    for (i, coverage) in d.role_coverage.iter().enumerate() {
        refs.interface(&format!("/role_coverage/{i}/interface"), &coverage.interface, &coverage.interface);
        for (role, cell) in coverage.roles.iter() {
            let pointer = format!("/role_coverage/{i}/roles/{role}");
            for binding in &cell.bindings {
                if !index.image_bindings.contains_key(binding.as_str()) {
                    refs.missing(&format!("{pointer}/bindings"), &coverage.interface, "image binding", binding);
                }
            }
            refs.gap_opt(&format!("{pointer}/gap"), &coverage.interface, cell.gap.as_deref());
        }
    }
    for (i, interface) in d.interfaces.iter().enumerate() {
        for (r, relationship) in interface.relationships.iter().enumerate() {
            refs.interface(
                &format!("/interfaces/{i}/relationships/{r}/target"),
                &interface.interface_id,
                &relationship.target,
            );
        }
    }

    for (pointer, id, knowledge) in knowledge_blocks(d) {
        refs.sources(&format!("{pointer}/evidence"), id, &knowledge.evidence);
        refs.gap_opt(&format!("{pointer}/gap"), id, knowledge.gap.as_deref());
        for (c, claim) in knowledge.claims.iter().enumerate() {
            refs.sources(&format!("{pointer}/claims/{c}/evidence"), id, &claim.evidence);
        }
    }
    for (pointer, id, conditions) in super::condition_lists(d) {
        for (c, condition) in conditions.iter().enumerate() {
            refs.gap_opt(&format!("{pointer}/{c}/gap"), id, condition.gap.as_deref());
        }
    }

    for (i, c) in d.constraints.iter().enumerate() {
        let base = format!("/constraints/{i}");
        if let Some(source) = &c.recommended_by
            && !index.sources.contains_key(source.as_str())
        {
            refs.missing(&format!("{base}/recommended_by"), &c.id, "source", source);
        }
        refs.errors(&format!("{base}/overflow_errors"), &c.id, &c.overflow_errors);
    }
    for (i, p) in d.format_profiles.iter().enumerate() {
        let base = format!("/format_profiles/{i}");
        refs.constraints(&format!("{base}/constraints"), &p.id, &p.constraints);
        refs.errors(&format!("{base}/errors"), &p.id, &p.errors);
        for (f, fixture) in p.fixtures.iter().enumerate() {
            refs.sources(&format!("{base}/fixtures/{f}/evidence"), &fixture.id, &fixture.evidence);
        }
    }
    for (i, b) in d.text_bindings.iter().enumerate() {
        let base = format!("/text_bindings/{i}");
        for profile in &b.profiles {
            if !index.profiles.contains_key(profile.as_str()) {
                refs.missing(&format!("{base}/profiles"), &b.id, "format profile", profile);
            }
        }
        refs.constraints(&format!("{base}/constraints"), &b.id, &b.constraints);
        for (r, relationship) in b.relationships.iter().enumerate() {
            if !index.text_bindings.contains_key(relationship.target.as_str()) {
                refs.missing(&format!("{base}/relationships/{r}/target"), &b.id, "text binding", &relationship.target);
            }
        }
    }
    for (i, b) in d.image_bindings.iter().enumerate() {
        let base = format!("/image_bindings/{i}");
        refs.constraints(&format!("{base}/shared_constraints"), &b.id, &b.shared_constraints);
        refs.text_binding_opt(&format!("{base}/caption_binding"), &b.id, b.caption_binding.as_deref());
        refs.text_binding_opt(&format!("{base}/alt_text_binding"), &b.id, b.alt_text_binding.as_deref());
    }
    for (i, b) in d.attachment_bindings.iter().enumerate() {
        let base = format!("/attachment_bindings/{i}");
        refs.constraints(&format!("{base}/constraints"), &b.id, &b.constraints);
        refs.text_binding_opt(&format!("{base}/caption_binding"), &b.id, b.caption_binding.as_deref());
        refs.text_binding_opt(&format!("{base}/alt_text_binding"), &b.id, b.alt_text_binding.as_deref());
    }
    for (i, r) in d.receipts.iter().enumerate() {
        if let Some(target) = &r.status_interface {
            refs.interface(&format!("/receipts/{i}/status_interface"), &r.id, target);
        }
    }
    for (i, b) in d.attribution_bindings.iter().enumerate() {
        let base = format!("/attribution_bindings/{i}");
        refs.constraints(&format!("{base}/constraints"), &b.id, &b.constraints);
        if let Some(image) = &b.image_binding
            && !index.image_bindings.contains_key(image.as_str())
        {
            refs.missing(&format!("{base}/image_binding"), &b.id, "image binding", image);
        }
        if let Some(profile) = &b.format_profile
            && !index.profiles.contains_key(profile.as_str())
        {
            refs.missing(&format!("{base}/format_profile"), &b.id, "format profile", profile);
        }
    }
    for (i, b) in d.location_bindings.iter().enumerate() {
        for (f, field) in b.fields.iter().enumerate() {
            if let Some(constraint) = &field.constraint {
                refs.constraints(&format!("/location_bindings/{i}/fields/{f}/constraint"), &b.id, std::slice::from_ref(constraint));
            }
        }
    }
    for (i, r) in d.rate_limits.iter().enumerate() {
        refs.errors(&format!("/rate_limits/{i}/errors"), &r.id, &r.errors);
    }
    for (i, e) in d.errors.iter().enumerate() {
        let base = format!("/errors/{i}");
        if !index.envelopes.contains_key(e.envelope.as_str()) {
            refs.missing(&format!("{base}/envelope"), &e.id, "envelope", &e.envelope);
        }
        for fixture in &e.fixtures {
            if !index.error_fixtures.contains_key(fixture.as_str()) {
                refs.missing(&format!("{base}/fixtures"), &e.id, "error fixture", fixture);
            }
        }
        for fact in &e.related_facts {
            if !index.facts.contains(fact.as_str()) {
                refs.missing(&format!("{base}/related_facts"), &e.id, "fact", fact);
            }
        }
    }
    for (i, f) in d.error_fixtures.iter().enumerate() {
        let base = format!("/error_fixtures/{i}");
        if !index.envelopes.contains_key(f.envelope.as_str()) {
            refs.missing(&format!("{base}/envelope"), &f.id, "envelope", &f.envelope);
        }
        if let Some(expected) = &f.expected_error
            && !index.errors.contains_key(expected.as_str())
        {
            refs.missing(&format!("{base}/expected_error"), &f.id, "error", expected);
        }
    }
    for (i, q) in d.question_bindings.iter().enumerate() {
        let base = format!("/question_bindings/{i}");
        for (field, value) in [
            ("option_label_constraint", &q.option_label_constraint),
            ("option_value_constraint", &q.option_value_constraint),
            ("opaque_state_constraint", &q.opaque_state_constraint),
        ] {
            if let Some(constraint) = value {
                refs.constraints(&format!("{base}/{field}"), &q.id, std::slice::from_ref(constraint));
            }
        }
    }
    for (i, f) in d.interaction_fixtures.iter().enumerate() {
        let binding = f.binding.as_str();
        if !index.question_bindings.contains_key(binding) && !index.form_bindings.contains_key(binding) {
            refs.missing(&format!("/interaction_fixtures/{i}/binding"), &f.id, "question or form binding", binding);
        }
    }
    for (i, g) in d.gaps.iter().enumerate() {
        if let Some(interface) = &g.interface {
            refs.interface(&format!("/gaps/{i}/interface"), &g.id, interface);
        }
        refs.sources(&format!("/gaps/{i}/inspected_sources"), &g.id, &g.inspected_sources);
    }
    for (i, c) in d.changes.iter().enumerate() {
        refs.sources(&format!("/changes/{i}/evidence"), &c.id, &c.evidence);
    }
}

struct Refs<'i, 'a, 'f> {
    index: &'i Index<'a>,
    findings: &'f mut Findings,
}

impl Refs<'_, '_, '_> {
    fn missing(&mut self, pointer: &str, subject: &str, kind: &str, target: &str) {
        push(self.findings, Rule::Ref, pointer, subject, format!("{kind} {target} does not exist"));
    }

    fn interface(&mut self, pointer: &str, subject: &str, target: &str) {
        if !self.index.interfaces.contains_key(target) {
            self.missing(pointer, subject, "interface", target);
        }
    }

    fn sources(&mut self, pointer: &str, subject: &str, ids: &[String]) {
        for id in ids {
            if !self.index.sources.contains_key(id.as_str()) {
                self.missing(pointer, subject, "source", id);
            }
        }
    }

    fn constraints(&mut self, pointer: &str, subject: &str, ids: &[String]) {
        for id in ids {
            if !self.index.constraints.contains_key(id.as_str()) {
                self.missing(pointer, subject, "constraint", id);
            }
        }
    }

    fn errors(&mut self, pointer: &str, subject: &str, ids: &[String]) {
        for id in ids {
            if !self.index.errors.contains_key(id.as_str()) {
                self.missing(pointer, subject, "error record", id);
            }
        }
    }

    fn gap_opt(&mut self, pointer: &str, subject: &str, gap: Option<&str>) {
        if let Some(gap) = gap
            && !self.index.gaps.contains_key(gap)
        {
            self.missing(pointer, subject, "gap", gap);
        }
    }

    fn text_binding_opt(&mut self, pointer: &str, subject: &str, binding: Option<&str>) {
        if let Some(binding) = binding
            && !self.index.text_bindings.contains_key(binding)
        {
            self.missing(pointer, subject, "text binding", binding);
        }
    }
}

// ---- SR-EVIDENCE ------------------------------------------------------------

fn evidence(d: &PlatformDocument, index: &Index<'_>, findings: &mut Findings) {
    for (i, source) in d.sources.iter().enumerate() {
        if source.url.is_some() == source.location.is_some() {
            push(
                findings,
                Rule::Evidence,
                format!("/sources/{i}"),
                &source.id,
                "a source has exactly one of url or location",
            );
        }
    }
    for (pointer, id, knowledge) in knowledge_blocks(d) {
        if knowledge.state != State::Known {
            continue;
        }
        if knowledge.evidence.is_empty() {
            push(findings, Rule::Evidence, format!("{pointer}/evidence"), id, "a known fact cites at least one source");
        }
        for source in &knowledge.evidence {
            if let Some(source) = index.sources.get(source.as_str())
                && source.retrieved.is_none()
            {
                push(
                    findings,
                    Rule::Evidence,
                    format!("{pointer}/evidence"),
                    id,
                    format!("cited source {} has no retrieved date", source.id),
                );
            }
        }
    }
}

// ---- versions ---------------------------------------------------------------

/// Version findings keep established unversioned APIs apart from
/// unresearched versioning, and previews apart from the stable release.
fn versions(d: &PlatformDocument, findings: &mut Findings) {
    for (i, finding) in d.api_versions.iter().enumerate() {
        let pointer = format!("/api_versions/{i}");
        match finding.versioning {
            Versioning::Unversioned if finding.latest_stable.is_some() || finding.previews.as_ref().is_some_and(|p| !p.is_empty()) => push(
                findings,
                Rule::StateValue,
                pointer,
                &finding.id,
                "an unversioned API records no stable or preview version",
            ),
            Versioning::Unresearched if finding.knowledge.state == State::Known => push(
                findings,
                Rule::StateValue,
                format!("{pointer}/knowledge/state"),
                &finding.id,
                "unresearched versioning is not a known finding",
            ),
            _ => {
                if let (Some(stable), Some(previews)) = (&finding.latest_stable, &finding.previews)
                    && previews.contains(stable)
                {
                    push(
                        findings,
                        Rule::StateValue,
                        format!("{pointer}/previews"),
                        &finding.id,
                        format!("{stable} is listed as both stable and preview"),
                    );
                }
            }
        }
    }
}

fn list(adapters: &BTreeSet<AdapterId>) -> String {
    if adapters.is_empty() {
        return "[]".to_string();
    }
    format!("[{}]", adapters.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", "))
}
