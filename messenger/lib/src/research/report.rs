//! Report models built from the catalog.
//!
//! A report reads only a published catalog ([`CatalogView`]), so it always
//! reflects one verified snapshot and stays available when today's validation
//! would fail (an expired override, stale research). It groups facts into the spec's report sections,
//! reports freshness against the caller's `today`, lists implementation gaps,
//! and produces the two per-adapter handoffs:
//!
//! - **Truncation:** for each of the seven chat adapters, every payload
//!   surface Messenger emits, the constraints that apply to it (field and
//!   aggregate bounds), whether each is enforceable or the specific reasons it
//!   is not, where the service can truncate or transform content despite a
//!   successful response, and richer researched surfaces Messenger does not
//!   emit. A surface without a researched bound reports its coverage state and
//!   gap, never "unlimited".
//! - **Diagnostics:** each adapter's response envelopes and error inventory,
//!   which signatures are executable, related constraints, and the adapter's
//!   assessed error handling.
//!
//! Nothing here changes `CapabilitySet` or delivery behavior.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::model::{AdapterId, Category, Date, PlatformId, State, Surface};
use super::validate::CoverageState;

/// The read model of a published `catalog.json`: the fields reports need,
/// parsed from the catalog's bytes. It carries eligibility as reason codes
/// only, so no executable type can be built from a file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CatalogView {
    pub platforms: Vec<PlatformView>,
    pub implementation: Vec<AssessmentView>,
}

impl CatalogView {
    /// Parses published catalog bytes.
    ///
    /// ## Errors
    ///
    /// The JSON error for bytes that are not a catalog.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    fn platform(&self, platform: PlatformId) -> Option<&PlatformView> {
        self.platforms.iter().find(|entry| entry.platform_id == platform)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PlatformView {
    pub platform_id: PlatformId,
    pub last_updated: Date,
    pub refresh_due: Date,
    pub interfaces: Vec<InterfaceView>,
    pub coverage: Vec<CoverageView>,
    pub facts: Vec<FactView>,
    pub constraints: Vec<EligibilityView>,
    pub gaps: Vec<Value>,
}

impl PlatformView {
    /// Past its refresh date on `today`: still inspectable, never invalid.
    pub fn is_stale(&self, today: &Date) -> bool {
        today >= &self.refresh_due
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InterfaceView {
    pub interface_id: String,
    pub adapters: Vec<AdapterId>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CoverageView {
    pub interface: String,
    pub categories: Vec<CellView>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CellView {
    pub category: Category,
    pub state: CoverageState,
    pub records: usize,
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FactView {
    pub array: String,
    pub id: String,
    pub interface: Option<String>,
    pub state: Option<State>,
    pub evidence: Vec<String>,
    pub gap: Option<String>,
    pub record: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EligibilityView {
    /// `eligible` or `ineligible`.
    pub eligibility: String,
    pub id: String,
    #[serde(default)]
    pub reasons: Vec<ReasonView>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReasonView {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AssessmentView {
    pub id: String,
    pub adapter: AdapterId,
    pub category: Category,
    /// `{"state": "current", …}` or `{"state": "unassessed", "reason": …}`.
    pub state: Value,
    pub gap: Option<GapView>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GapView {
    pub facts: Vec<String>,
    pub reason: String,
}

/// The payload surfaces each chat adapter emits today, with Messenger's
/// native locator. Implementation knowledge from the Phase 1 surface
/// inventory (`spikes/surface-inventory/inventory.md`); it must change with the
/// adapter code it describes.
pub fn emitted_surfaces(adapter: AdapterId) -> &'static [(Surface, &'static str)] {
    match adapter {
        AdapterId::Discord => &[
            (Surface::Body, "content"),
            (Surface::Summary, "content"),
            (Surface::RichDescription, "embeds[0].description"),
            (Surface::AttachmentDescription, "attachments[n].description"),
            (Surface::Filename, "attachments[n].filename"),
        ],
        AdapterId::DiscordWebhook => &[
            (Surface::Body, "/content (inside payload_json with files)"),
            (Surface::Summary, "/content"),
            (Surface::RichDescription, "/embeds/0/description"),
            (Surface::AttachmentDescription, "/attachments/{i}/description"),
            (Surface::Filename, "/attachments/{i}/filename"),
        ],
        AdapterId::Slack => &[(Surface::Body, "/text (chat.postMessage)")],
        AdapterId::SlackWebhook => &[(Surface::Body, "/text")],
        AdapterId::Telegram => &[(Surface::Body, "sendMessage /text")],
        AdapterId::WhatsApp => &[(Surface::Body, "/text/body")],
        AdapterId::Signal => &[(Surface::Body, "params.message")],
    }
}

/// Narrows a report. `None` fields do not filter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Filter {
    pub platform: Option<PlatformId>,
    pub interface: Option<String>,
    pub operation: Option<String>,
}

impl Filter {
    fn platform(&self, platform: PlatformId) -> bool {
        self.platform.is_none_or(|wanted| wanted == platform)
    }

    fn interface(&self, interface: Option<&str>) -> bool {
        self.interface.as_deref().is_none_or(|wanted| interface == Some(wanted))
    }

    fn operations(&self, operations: &[String]) -> bool {
        self.operation.as_deref().is_none_or(|wanted| operations.iter().any(|op| op == wanted))
    }
}

/// A report section: facts from a fixed set of frontmatter arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Versions,
    Constraints,
    Formatting,
    Images,
    Attribution,
    Location,
    Expression,
    Interactivity,
    Errors,
    Eligibility,
    Capabilities,
}

impl SectionKind {
    pub const ALL: &'static [SectionKind] = &[
        SectionKind::Versions,
        SectionKind::Constraints,
        SectionKind::Formatting,
        SectionKind::Images,
        SectionKind::Attribution,
        SectionKind::Location,
        SectionKind::Expression,
        SectionKind::Interactivity,
        SectionKind::Errors,
        SectionKind::Eligibility,
        SectionKind::Capabilities,
    ];

    pub fn title(self) -> &'static str {
        match self {
            SectionKind::Versions => "API versions and release chronology",
            SectionKind::Constraints => "Constraints",
            SectionKind::Formatting => "Formatting profiles and text bindings",
            SectionKind::Images => "Image bindings",
            SectionKind::Attribution => "Attribution",
            SectionKind::Location => "Location and geolocation",
            SectionKind::Expression => "Expressive effects",
            SectionKind::Interactivity => "Inbound text, questions, and forms",
            SectionKind::Errors => "Error envelopes, errors, and recovery evidence",
            SectionKind::Eligibility => "Eligibility requirements",
            SectionKind::Capabilities => "Attachments, addressing, receipts, delivery controls, and rate limits",
        }
    }

    /// `(array, summary fields)` for each array the section draws from.
    fn arrays(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            SectionKind::Versions => &[
                ("api_versions", &["subject", "name", "versioning", "latest_stable", "previews"]),
                ("chronology", &["subject", "name", "version", "stability", "release_date", "release_date_state"]),
            ],
            SectionKind::Constraints => &[(
                "constraints",
                &["surface", "native_locator", "kind", "value", "unit", "measurement_stage", "enforced_by", "overflow_behavior", "aggregation_scope"],
            )],
            SectionKind::Formatting => &[
                ("format_profiles", &["family", "dialect", "auto_interpretation", "malformed_behavior"]),
                ("text_bindings", &["surface", "native_locator", "representation", "content_role", "packaging", "visibility", "profiles"]),
            ],
            SectionKind::Images => &[(
                "image_bindings",
                &["role", "fidelity", "native_locator", "placement", "placement_control", "collection", "min_items", "max_items", "supplied_by"],
            )],
            SectionKind::Attribution => &[(
                "attribution_bindings",
                &["role", "native_fields", "control", "scope", "inclusion", "override_behavior", "changes_sender"],
            )],
            SectionKind::Location => &[
                ("location_bindings", &["role", "subject", "mode", "origin", "inclusion", "representation", "delivery", "presentation"]),
                ("author_geolocation", &["exposure"]),
            ],
            SectionKind::Expression => &[(
                "expression_bindings",
                &["mechanism", "support", "api_availability", "control", "scope", "intent", "intent_fidelity"],
            )],
            SectionKind::Interactivity => &[
                ("inbound_bindings", &["event", "mechanism", "reach", "content", "reuses_send_identity"]),
                ("question_bindings", &["kind", "native", "mechanism", "answer_type", "cancellation", "responses", "fidelity", "companion_interface"]),
                ("form_bindings", &["container", "question_kinds", "submission", "absent_vs_empty", "cancellation", "navigation", "companion_interface"]),
            ],
            SectionKind::Errors => &[
                ("envelopes", &["origin", "body_format", "code_locator", "message_locator", "warnings_locator", "retry_after_locator"]),
                ("errors", &["envelope", "phase", "outcome", "category", "related_facts", "delivery_certainty", "recovery", "replay_safety", "remediation"]),
            ],
            SectionKind::Eligibility => &[("eligibility", &["kind", "description"])],
            SectionKind::Capabilities => &[
                ("attachment_bindings", &["media_kinds", "upload_mechanisms", "native_locator", "filename_control"]),
                ("addressing", &["destination_kinds", "reply_support", "thread_support", "returns_identifier"]),
                ("receipts", &["acceptance", "identifier_locator", "delivered_state", "read_state"]),
                ("delivery_controls", &["control", "support", "native_fields"]),
                ("rate_limits", &["scope", "basis", "rate", "per_seconds", "burst", "retry_after_unit", "idempotency"]),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub platform_id: PlatformId,
    pub array: String,
    pub id: String,
    pub interface: Option<String>,
    pub operations: Vec<String>,
    pub state: Option<State>,
    /// `field=value` pairs of the section's summary fields, in field order.
    pub summary: String,
    pub evidence: Vec<String>,
    pub gap: Option<String>,
    /// For constraints: whether it reached the executable projection, and
    /// every reason it did not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ineligible_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    pub kind: SectionKind,
    pub title: &'static str,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Freshness {
    pub platform_id: PlatformId,
    pub last_updated: Date,
    pub refresh_due: Date,
    /// Accepted but past its refresh date: still inspectable, never invalid.
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageRow {
    pub platform_id: PlatformId,
    pub interface: String,
    pub category: Category,
    pub state: CoverageState,
    pub records: usize,
    pub gap: Option<String>,
}

/// A `requires_messenger_update` item: from an assessment outcome or a
/// document gap of that kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImplementationGapRow {
    pub platform_id: PlatformId,
    pub source: String,
    pub adapter: Option<AdapterId>,
    pub category: Option<Category>,
    pub facts: Vec<String>,
    pub reason: String,
}

/// One constraint applying to an emitted surface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SurfaceConstraint {
    pub id: String,
    pub operation: Option<String>,
    pub kind: Option<String>,
    pub value: Option<u64>,
    pub unit: Option<String>,
    pub stage: Option<String>,
    pub enforced_by: Option<String>,
    pub overflow_behavior: Option<String>,
    /// The constraint bounds several surfaces together.
    pub aggregate: bool,
    pub executable: bool,
    pub reasons: Vec<String>,
    pub gap: Option<String>,
}

/// Why a surface is or is not enforceable.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Enforceability {
    /// At least one executable bound applies.
    Enforceable,
    /// Bounds are researched but none is executable.
    NotEnforceable { reasons: Vec<String> },
    /// No constraint names this surface: not unlimited, unresearched.
    NoResearchedBound { coverage: Option<CoverageState>, gap: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SurfaceHandoff {
    pub surface: Surface,
    pub messenger_locator: &'static str,
    pub enforceability: Enforceability,
    pub constraints: Vec<SurfaceConstraint>,
}

/// A researched surface or image role Messenger does not emit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnimplementedSurface {
    pub array: String,
    pub id: String,
    pub surface: String,
}

/// A bound whose overflow the service handles by truncating, splitting, or
/// transforming content: loss despite a successful response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceSideLoss {
    pub id: String,
    pub surface: String,
    pub overflow_behavior: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TruncationHandoff {
    pub adapter: AdapterId,
    pub platform_id: PlatformId,
    /// The sending interface mapped to the adapter; `None` means the
    /// accepted research does not map it (itself an unresolved question).
    pub interface: Option<String>,
    pub surfaces: Vec<SurfaceHandoff>,
    pub service_side_loss: Vec<ServiceSideLoss>,
    pub unimplemented_surfaces: Vec<UnimplementedSurface>,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorHandoff {
    pub id: String,
    pub category: Option<String>,
    pub outcome: Option<String>,
    pub state: Option<State>,
    /// A known record with an explicit signature; candidate matches never are.
    pub executable_signature: bool,
    pub related_facts: Vec<String>,
    pub recovery: Option<String>,
    pub replay_safety: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssessedHandling {
    pub assessment: String,
    pub state: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiagnosticHandoff {
    pub adapter: AdapterId,
    pub platform_id: PlatformId,
    pub interface: Option<String>,
    pub coverage: Option<CoverageState>,
    pub gap: Option<String>,
    pub envelopes: Vec<String>,
    pub errors: Vec<ErrorHandoff>,
    /// Messenger's assessed handling (category `errors`); empty means unassessed.
    pub implementation: Vec<AssessedHandling>,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub filter: Filter,
    pub today: Date,
    pub freshness: Vec<Freshness>,
    pub coverage: Vec<CoverageRow>,
    pub sections: Vec<Section>,
    pub implementation_gaps: Vec<ImplementationGapRow>,
    pub truncation: Vec<TruncationHandoff>,
    pub diagnostics: Vec<DiagnosticHandoff>,
}

/// Builds the report for `filter` on `today`.
pub fn build(catalog: &CatalogView, filter: &Filter, today: &Date) -> Report {
    let platforms: Vec<&PlatformView> =
        catalog.platforms.iter().filter(|platform| filter.platform(platform.platform_id)).collect();

    let freshness = platforms
        .iter()
        .map(|platform| Freshness {
            platform_id: platform.platform_id,
            last_updated: platform.last_updated.clone(),
            refresh_due: platform.refresh_due.clone(),
            stale: platform.is_stale(today),
        })
        .collect();

    let coverage = platforms
        .iter()
        .flat_map(|platform| {
            platform
                .coverage
                .iter()
                .filter(|interface| filter.interface(Some(&interface.interface)))
                .flat_map(move |interface| {
                    interface.categories.iter().map(move |category| CoverageRow {
                        platform_id: platform.platform_id,
                        interface: interface.interface.clone(),
                        category: category.category,
                        state: category.state,
                        records: category.records,
                        gap: category.gap.clone(),
                    })
                })
        })
        .collect();

    let sections = SectionKind::ALL
        .iter()
        .map(|kind| Section {
            kind: *kind,
            title: kind.title(),
            rows: platforms.iter().flat_map(|platform| section_rows(platform, *kind, filter)).collect(),
        })
        .collect();

    let mut implementation_gaps: Vec<ImplementationGapRow> = Vec::new();
    for outcome in &catalog.implementation {
        let Some(gap) = &outcome.gap else { continue };
        let platform_id = outcome.adapter.platform();
        if !filter.platform(platform_id) {
            continue;
        }
        implementation_gaps.push(ImplementationGapRow {
            platform_id,
            source: format!("assessment {}", outcome.id),
            adapter: Some(outcome.adapter),
            category: Some(outcome.category),
            facts: gap.facts.clone(),
            reason: gap.reason.clone(),
        });
    }
    for platform in &platforms {
        for gap in &platform.gaps {
            if gap["kind"].as_str() != Some("requires_messenger_update") {
                continue;
            }
            if !filter.interface(gap["interface"].as_str()) {
                continue;
            }
            implementation_gaps.push(ImplementationGapRow {
                platform_id: platform.platform_id,
                source: format!("gap {}", gap["id"].as_str().unwrap_or_default()),
                adapter: None,
                category: gap["category"].as_str().and_then(|c| serde_json::from_value(Value::from(c)).ok()),
                facts: strings(&gap["facts"]),
                reason: gap["question"].as_str().unwrap_or_default().to_string(),
            });
        }
    }

    let adapters: Vec<AdapterId> = AdapterId::ALL
        .iter()
        .copied()
        .filter(|adapter| filter.platform(adapter.platform()))
        .collect();
    let truncation = adapters
        .iter()
        .filter_map(|adapter| {
            let platform = catalog.platform(adapter.platform())?;
            let handoff = truncation_handoff(platform, *adapter);
            filter.interface(handoff.interface.as_deref()).then_some(handoff)
        })
        .collect();
    let diagnostics = adapters
        .iter()
        .filter_map(|adapter| {
            let platform = catalog.platform(adapter.platform())?;
            let handoff = diagnostic_handoff(catalog, platform, *adapter);
            filter.interface(handoff.interface.as_deref()).then_some(handoff)
        })
        .collect();

    Report {
        filter: filter.clone(),
        today: today.clone(),
        freshness,
        coverage,
        sections,
        implementation_gaps,
        truncation,
        diagnostics,
    }
}

/// The schema spelling of a coverage state, e.g. `investigated_gap`.
pub fn coverage_name(state: CoverageState) -> &'static str {
    match state {
        CoverageState::Researched => "researched",
        CoverageState::NotApplicable => "not_applicable",
        CoverageState::InvestigatedGap => "investigated_gap",
        CoverageState::OpenGap => "open_gap",
        CoverageState::Missing => "missing",
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

/// The operations a record is scoped to (`operation` or `operations`).
fn operations(record: &Value) -> Vec<String> {
    match record.get("operation").and_then(Value::as_str) {
        Some(operation) => vec![operation.to_string()],
        None => strings(&record["operations"]),
    }
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Array(items) if items.is_empty() => None,
        Value::Array(items) => Some(items.iter().filter_map(scalar).collect::<Vec<_>>().join("+")),
        Value::Object(_) => None,
        other => Some(other.to_string()),
    }
}

fn section_rows(platform: &PlatformView, kind: SectionKind, filter: &Filter) -> Vec<Row> {
    let mut rows = Vec::new();
    for (array, fields) in kind.arrays() {
        for fact in platform.facts.iter().filter(|fact| fact.array == *array) {
            let operations = operations(&fact.record);
            let document_wide = fact.interface.is_none() && operations.is_empty();
            if !document_wide && (!filter.interface(fact.interface.as_deref()) || !filter.operations(&operations)) {
                continue;
            }
            if document_wide && (filter.interface.is_some() || filter.operation.is_some()) {
                continue;
            }
            let summary = fields
                .iter()
                .filter_map(|field| scalar(&fact.record[*field]).map(|value| format!("{field}={value}")))
                .collect::<Vec<_>>()
                .join(", ");
            let (executable, ineligible_reasons) = if *array == "constraints" {
                constraint_eligibility(platform, &fact.id)
            } else {
                (None, Vec::new())
            };
            rows.push(Row {
                platform_id: platform.platform_id,
                array: fact.array.clone(),
                id: fact.id.clone(),
                interface: fact.interface.clone(),
                operations,
                state: fact.state,
                summary,
                evidence: fact.evidence.clone(),
                gap: fact.gap.clone(),
                executable,
                ineligible_reasons,
            });
        }
    }
    rows
}

fn constraint_eligibility(platform: &PlatformView, id: &str) -> (Option<bool>, Vec<String>) {
    match platform.constraints.iter().find(|entry| entry.id == id) {
        Some(entry) if entry.eligibility == "eligible" => (Some(true), Vec::new()),
        Some(entry) => (Some(false), entry.reasons.iter().map(|reason| reason.reason.clone()).collect()),
        None => (None, Vec::new()),
    }
}

fn adapter_interface(platform: &PlatformView, adapter: AdapterId) -> Option<String> {
    platform
        .interfaces
        .iter()
        .find(|interface| interface.adapters.contains(&adapter))
        .map(|interface| interface.interface_id.clone())
}

fn category_coverage(platform: &PlatformView, interface: &str, category: Category) -> (Option<CoverageState>, Option<String>) {
    platform
        .coverage
        .iter()
        .find(|entry| entry.interface == interface)
        .and_then(|entry| entry.categories.iter().find(|cell| cell.category == category))
        .map_or((None, None), |cell| (Some(cell.state), cell.gap.clone()))
}

fn facts_in<'a>(platform: &'a PlatformView, array: &'a str, interface: &'a str) -> impl Iterator<Item = &'a FactView> {
    platform
        .facts
        .iter()
        .filter(move |fact| fact.array == array && fact.interface.as_deref() == Some(interface))
}

fn truncation_handoff(platform: &PlatformView, adapter: AdapterId) -> TruncationHandoff {
    let interface = adapter_interface(platform, adapter);
    let mut unresolved = Vec::new();
    let Some(interface_id) = interface.clone() else {
        unresolved.push(format!("no accepted {} interface maps adapter {adapter}", platform.platform_id));
        return TruncationHandoff {
            adapter,
            platform_id: platform.platform_id,
            interface,
            surfaces: Vec::new(),
            service_side_loss: Vec::new(),
            unimplemented_surfaces: Vec::new(),
            unresolved,
        };
    };
    let emitted = emitted_surfaces(adapter);
    let constraints: Vec<&FactView> = facts_in(platform, "constraints", &interface_id).collect();

    let mut surfaces = Vec::new();
    for (surface, locator) in emitted {
        let name = surface.as_str();
        let mut applying: Vec<SurfaceConstraint> = Vec::new();
        for fact in &constraints {
            let record = &fact.record;
            let direct = record["surface"].as_str() == Some(name);
            let member = record["members"]
                .as_array()
                .is_some_and(|members| members.iter().any(|m| m["surface"].as_str() == Some(name)));
            if !direct && !member {
                continue;
            }
            let (executable, reasons) = constraint_eligibility(platform, &fact.id);
            applying.push(SurfaceConstraint {
                id: fact.id.clone(),
                operation: record["operation"].as_str().map(str::to_string),
                kind: scalar(&record["kind"]),
                value: record["value"].as_u64(),
                unit: scalar(&record["unit"]),
                stage: scalar(&record["measurement_stage"]),
                enforced_by: scalar(&record["enforced_by"]),
                overflow_behavior: scalar(&record["overflow_behavior"]),
                aggregate: member && !direct,
                executable: executable == Some(true),
                reasons,
                gap: fact.gap.clone(),
            });
        }
        let enforceability = if applying.iter().any(|c| c.executable && c.kind.as_deref() != Some("recommended_max")) {
            Enforceability::Enforceable
        } else if applying.is_empty() {
            let (coverage, gap) = category_coverage(platform, &interface_id, Category::Constraints);
            unresolved.push(format!(
                "{name}: no researched bound for this surface (constraints coverage: {}{})",
                coverage.map_or("missing", coverage_name),
                gap.as_deref().map(|gap| format!(", gap {gap}")).unwrap_or_default()
            ));
            Enforceability::NoResearchedBound { coverage, gap }
        } else {
            let mut reasons: Vec<String> = applying
                .iter()
                .flat_map(|c| {
                    if c.reasons.is_empty() {
                        vec![format!("{}: advisory only", c.id)]
                    } else {
                        c.reasons.iter().map(|reason| format!("{}: {reason}", c.id)).collect()
                    }
                })
                .collect();
            reasons.sort();
            reasons.dedup();
            unresolved.push(format!("{name}: not enforceable ({})", reasons.join("; ")));
            Enforceability::NotEnforceable { reasons }
        };
        surfaces.push(SurfaceHandoff {
            surface: *surface,
            messenger_locator: locator,
            enforceability,
            constraints: applying,
        });
    }

    let mut service_side_loss = Vec::new();
    for fact in &constraints {
        let behavior = fact.record["overflow_behavior"].as_str().unwrap_or_default();
        match behavior {
            "truncate" | "split" | "transform" => service_side_loss.push(ServiceSideLoss {
                id: fact.id.clone(),
                surface: scalar(&fact.record["surface"]).unwrap_or_default(),
                overflow_behavior: behavior.to_string(),
            }),
            "unknown" => unresolved.push(format!("{}: overflow behavior unknown", fact.id)),
            _ => {}
        }
    }

    let emitted_names: Vec<&str> = emitted.iter().map(|(surface, _)| surface.as_str()).collect();
    let mut unimplemented_surfaces: Vec<UnimplementedSurface> = facts_in(platform, "text_bindings", &interface_id)
        .filter_map(|fact| {
            let surface = fact.record["surface"].as_str()?;
            (!emitted_names.contains(&surface)).then(|| UnimplementedSurface {
                array: fact.array.clone(),
                id: fact.id.clone(),
                surface: surface.to_string(),
            })
        })
        .collect();
    unimplemented_surfaces.extend(facts_in(platform, "image_bindings", &interface_id).map(|fact| UnimplementedSurface {
        array: fact.array.clone(),
        id: fact.id.clone(),
        surface: format!("image:{}", fact.record["role"].as_str().unwrap_or("unknown")),
    }));

    TruncationHandoff {
        adapter,
        platform_id: platform.platform_id,
        interface,
        surfaces,
        service_side_loss,
        unimplemented_surfaces,
        unresolved,
    }
}

fn diagnostic_handoff(catalog: &CatalogView, platform: &PlatformView, adapter: AdapterId) -> DiagnosticHandoff {
    let interface = adapter_interface(platform, adapter);
    let implementation: Vec<AssessedHandling> = catalog
        .implementation
        .iter()
        .filter(|outcome| outcome.adapter == adapter && outcome.category == Category::Errors)
        .map(|outcome| AssessedHandling {
            assessment: outcome.id.clone(),
            state: outcome.state.clone(),
        })
        .collect();
    let mut unresolved = Vec::new();
    let Some(interface_id) = interface.clone() else {
        unresolved.push(format!("no accepted {} interface maps adapter {adapter}", platform.platform_id));
        return DiagnosticHandoff {
            adapter,
            platform_id: platform.platform_id,
            interface,
            coverage: None,
            gap: None,
            envelopes: Vec::new(),
            errors: Vec::new(),
            implementation,
            unresolved,
        };
    };
    let (coverage, gap) = category_coverage(platform, &interface_id, Category::Errors);
    let envelopes: Vec<String> = facts_in(platform, "envelopes", &interface_id).map(|fact| fact.id.clone()).collect();
    let errors: Vec<ErrorHandoff> = facts_in(platform, "errors", &interface_id)
        .map(|fact| {
            let record = &fact.record;
            ErrorHandoff {
                id: fact.id.clone(),
                category: scalar(&record["category"]),
                outcome: scalar(&record["outcome"]),
                state: fact.state,
                executable_signature: fact.state == Some(State::Known) && record["signature"].is_object(),
                related_facts: strings(&record["related_facts"]),
                recovery: scalar(&record["recovery"]),
                replay_safety: scalar(&record["replay_safety"]),
            }
        })
        .collect();
    if envelopes.is_empty() {
        unresolved.push(format!(
            "no researched response envelope (errors coverage: {}{})",
            coverage.map_or("missing", coverage_name),
            gap.as_deref().map(|gap| format!(", gap {gap}")).unwrap_or_default()
        ));
    }
    for error in errors.iter().filter(|error| !error.executable_signature) {
        unresolved.push(format!("{}: no executable signature", error.id));
    }
    if implementation.is_empty() {
        unresolved.push(format!("{adapter} error handling is unassessed"));
    }
    for handling in &implementation {
        if handling.state["state"].as_str() == Some("unassessed") {
            let reason = handling.state["reason"].as_str().unwrap_or("unknown");
            unresolved.push(format!("{}: unassessed ({reason})", handling.assessment));
        }
    }
    DiagnosticHandoff {
        adapter,
        platform_id: platform.platform_id,
        interface,
        coverage,
        gap,
        envelopes,
        errors,
        implementation,
        unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_chat_adapter_declares_a_body_surface() {
        for adapter in AdapterId::ALL {
            let surfaces = emitted_surfaces(*adapter);
            assert!(surfaces.iter().any(|(surface, _)| *surface == Surface::Body), "{adapter}");
            let mut names: Vec<&str> = surfaces.iter().map(|(surface, _)| surface.as_str()).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), surfaces.len(), "{adapter} lists a surface twice");
        }
    }
}
