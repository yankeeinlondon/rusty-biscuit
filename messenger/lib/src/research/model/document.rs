//! Authored platform research documents (`docs/research/platforms/{platform}.md`).
//!
//! These DTOs mirror `_schema.yaml` and `_types.yaml` (schema version 1)
//! field for field. They are authored records: nothing here is executable
//! until `research::validate` derives an eligible projection.

use serde::{Deserialize, Serialize};

use super::common::{
    AdapterId, Category, Condition, Date, Fidelity, Knowledge, PlatformId, Source, Stage, Support,
    Surface, Unit, YesNo, string_enum,
};

/// One platform research document's frontmatter.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformDocument {
    pub schema_version: u32,
    pub platform_id: PlatformId,
    pub created: Date,
    pub last_updated: Date,
    pub agent: String,
    pub model: String,

    pub sources: Vec<Source>,
    pub interfaces: Vec<Interface>,
    pub coverage: Vec<InterfaceCoverage>,

    pub api_versions: Vec<ApiVersionFinding>,
    pub chronology: Vec<ChronologyEntry>,

    pub constraints: Vec<Constraint>,
    pub format_profiles: Vec<FormatProfile>,
    pub text_bindings: Vec<TextBinding>,
    pub image_bindings: Vec<ImageBinding>,
    pub role_coverage: Vec<RoleCoverage>,
    pub attachment_bindings: Vec<AttachmentBinding>,
    pub addressing: Vec<AddressingFact>,
    pub receipts: Vec<ReceiptFact>,

    pub attribution_bindings: Vec<AttributionBinding>,
    pub location_bindings: Vec<LocationBinding>,
    pub author_geolocation: Vec<AuthorGeolocation>,
    pub expression_bindings: Vec<ExpressionBinding>,

    pub delivery_controls: Vec<DeliveryControl>,
    pub eligibility: Vec<EligibilityRequirement>,
    pub rate_limits: Vec<RateLimit>,

    pub envelopes: Vec<Envelope>,
    pub errors: Vec<ErrorRecord>,
    pub error_fixtures: Vec<ErrorFixture>,

    pub inbound_bindings: Vec<InboundBinding>,
    pub question_bindings: Vec<QuestionBinding>,
    pub form_bindings: Vec<FormBinding>,
    pub interaction_fixtures: Vec<InteractionFixture>,

    pub changes: Vec<Change>,
    pub gaps: Vec<Gap>,
    pub requires_messenger_update: bool,
    pub reason: Option<String>,

    /// Composition metadata allowlisted by the schema; never interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<serde_json::Value>,
}

// ---- interfaces ------------------------------------------------------------

string_enum! {
    pub enum RelationshipKind {
        ReceivesEventsFor => "receives_events_for",
        RespondsToInteractionsFrom => "responds_to_interactions_from",
        RequiresCompanionInterface => "requires_companion_interface",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    pub kind: RelationshipKind,
    pub target: String,
    pub prerequisites: Option<Vec<String>>,
    pub authorization: Option<String>,
}

string_enum! {
    pub enum InterfaceRole {
        SendingAdapter => "sending_adapter",
        ResearchOnly => "research_only",
    }
}

string_enum! {
    pub enum Classification {
        Official => "official",
        Community => "community",
    }
}

string_enum! {
    pub enum Direction {
        SendOnly => "send_only",
        ReceiveOnly => "receive_only",
        CallbackOnly => "callback_only",
        Bidirectional => "bidirectional",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub interface_id: String,
    pub role: InterfaceRole,
    pub api_identity: String,
    pub classification: Classification,
    pub direction: Direction,
    pub adapters: Vec<AdapterId>,
    pub operations: Vec<String>,
    pub endpoint_template: Option<String>,
    pub api_version: Option<String>,
    pub sdk: Option<String>,
    pub sdk_version: Option<String>,
    pub bridge: Option<String>,
    pub bridge_version: Option<String>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
}

string_enum! {
    pub enum CoverageStatus {
        Researched => "researched",
        NotApplicable => "not_applicable",
        Gap => "gap",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageCell {
    pub status: CoverageStatus,
    pub gap: Option<String>,
    pub explanation: Option<String>,
}

/// Every category is a required key, so an omitted category cannot read as
/// unrestricted support.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryMatrix {
    pub versions: CoverageCell,
    pub constraints: CoverageCell,
    pub formatting: CoverageCell,
    pub text_bindings: CoverageCell,
    pub images: CoverageCell,
    pub attachments: CoverageCell,
    pub addressing: CoverageCell,
    pub receipts: CoverageCell,
    pub attribution: CoverageCell,
    pub location: CoverageCell,
    pub expression: CoverageCell,
    pub interactivity: CoverageCell,
    pub delivery_controls: CoverageCell,
    pub eligibility: CoverageCell,
    pub rate_limits: CoverageCell,
    pub errors: CoverageCell,
}

impl CategoryMatrix {
    /// The cell for one category.
    pub fn cell(&self, category: Category) -> &CoverageCell {
        match category {
            Category::Versions => &self.versions,
            Category::Constraints => &self.constraints,
            Category::Formatting => &self.formatting,
            Category::TextBindings => &self.text_bindings,
            Category::Images => &self.images,
            Category::Attachments => &self.attachments,
            Category::Addressing => &self.addressing,
            Category::Receipts => &self.receipts,
            Category::Attribution => &self.attribution,
            Category::Location => &self.location,
            Category::Expression => &self.expression,
            Category::Interactivity => &self.interactivity,
            Category::DeliveryControls => &self.delivery_controls,
            Category::Eligibility => &self.eligibility,
            Category::RateLimits => &self.rate_limits,
            Category::Errors => &self.errors,
        }
    }

    /// All sixteen cells in category order.
    pub fn cells(&self) -> impl Iterator<Item = (Category, &CoverageCell)> {
        Category::ALL.iter().map(|category| (*category, self.cell(*category)))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceCoverage {
    pub interface: String,
    pub categories: CategoryMatrix,
}

// ---- versions --------------------------------------------------------------

string_enum! {
    /// Keeps provider API versions apart from SDK and bridge releases.
    pub enum VersionSubject {
        ProviderApi => "provider_api",
        Sdk => "sdk",
        Bridge => "bridge",
    }
}

string_enum! {
    /// `unversioned` is an established finding; `unresearched` is not.
    pub enum Versioning {
        Versioned => "versioned",
        Unversioned => "unversioned",
        Unresearched => "unresearched",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiVersionFinding {
    pub id: String,
    pub interface: String,
    pub subject: VersionSubject,
    pub name: Option<String>,
    pub versioning: Versioning,
    pub latest_stable: Option<String>,
    pub previews: Option<Vec<String>>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum Stability {
        Stable => "stable",
        Preview => "preview",
        Deprecated => "deprecated",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChronologyEntry {
    pub id: String,
    pub interface: String,
    pub subject: VersionSubject,
    pub name: Option<String>,
    pub version: String,
    pub stability: Stability,
    pub release_date: Option<Date>,
    pub release_period: Option<String>,
    pub release_date_state: super::common::State,
    pub knowledge: Knowledge,
}

// ---- constraints -----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub surface: Surface,
    pub native_locator: String,
    pub repeated: Option<bool>,
}

string_enum! {
    pub enum ConstraintKind {
        HardMax => "hard_max",
        HardMin => "hard_min",
        RecommendedMax => "recommended_max",
        AggregateMax => "aggregate_max",
        ItemCountMax => "item_count_max",
        ItemCountMin => "item_count_min",
        PayloadBytesMax => "payload_bytes_max",
    }
}

impl ConstraintKind {
    /// Kinds whose unit is `items`.
    pub fn is_count(self) -> bool {
        matches!(self, ConstraintKind::ItemCountMax | ConstraintKind::ItemCountMin)
    }

    /// Kinds measured in raw request bytes.
    pub fn is_payload(self) -> bool {
        matches!(self, ConstraintKind::PayloadBytesMax)
    }

    /// Lower bounds.
    pub fn is_lower_bound(self) -> bool {
        matches!(self, ConstraintKind::HardMin | ConstraintKind::ItemCountMin)
    }
}

string_enum! {
    pub enum Enforcer {
        Service => "service",
        Sdk => "sdk",
        Bridge => "bridge",
    }
}

string_enum! {
    pub enum OverflowBehavior {
        Reject => "reject",
        Truncate => "truncate",
        Split => "split",
        Transform => "transform",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum AggregationScope {
        Message => "message",
        Request => "request",
        RichObject => "rich_object",
        Item => "item",
    }
}

/// A scoped bound. Logical key `(platform_id, interface, operation, id)`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub surface: Surface,
    pub native_locator: String,
    pub kind: ConstraintKind,
    pub value: Option<u64>,
    pub unit: Unit,
    pub measurement_stage: Stage,
    pub enforced_by: Option<Enforcer>,
    pub recommended_by: Option<String>,
    pub overflow_behavior: OverflowBehavior,
    pub overflow_details: Option<String>,
    #[serde(default)]
    pub overflow_errors: Vec<String>,
    pub applies_when: Vec<Condition>,
    pub aggregation_scope: Option<AggregationScope>,
    #[serde(default)]
    pub members: Vec<Member>,
    pub knowledge: Knowledge,
}

// ---- formatting and text bindings -----------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructSupport {
    pub state: super::common::State,
    pub support: Option<Support>,
    pub syntax: Option<String>,
    pub restriction: Option<String>,
}

string_enum! {
    /// The fourteen constructs every format profile answers.
    pub enum Construct {
        Emphasis => "emphasis",
        Strong => "strong",
        Strikethrough => "strikethrough",
        Underline => "underline",
        InlineCode => "inline_code",
        FencedCode => "fenced_code",
        Links => "links",
        Images => "images",
        Headings => "headings",
        Lists => "lists",
        BlockQuotes => "block_quotes",
        Tables => "tables",
        Spoilers => "spoilers",
        Mentions => "mentions",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Constructs {
    pub emphasis: ConstructSupport,
    pub strong: ConstructSupport,
    pub strikethrough: ConstructSupport,
    pub underline: ConstructSupport,
    pub inline_code: ConstructSupport,
    pub fenced_code: ConstructSupport,
    pub links: ConstructSupport,
    pub images: ConstructSupport,
    pub headings: ConstructSupport,
    pub lists: ConstructSupport,
    pub block_quotes: ConstructSupport,
    pub tables: ConstructSupport,
    pub spoilers: ConstructSupport,
    pub mentions: ConstructSupport,
}

impl Constructs {
    pub fn get(&self, construct: Construct) -> &ConstructSupport {
        match construct {
            Construct::Emphasis => &self.emphasis,
            Construct::Strong => &self.strong,
            Construct::Strikethrough => &self.strikethrough,
            Construct::Underline => &self.underline,
            Construct::InlineCode => &self.inline_code,
            Construct::FencedCode => &self.fenced_code,
            Construct::Links => &self.links,
            Construct::Images => &self.images,
            Construct::Headings => &self.headings,
            Construct::Lists => &self.lists,
            Construct::BlockQuotes => &self.block_quotes,
            Construct::Tables => &self.tables,
            Construct::Spoilers => &self.spoilers,
            Construct::Mentions => &self.mentions,
        }
    }

    /// All fourteen constructs in schema order.
    pub fn iter(&self) -> impl Iterator<Item = (Construct, &ConstructSupport)> {
        Construct::ALL.iter().map(|construct| (*construct, self.get(*construct)))
    }
}

string_enum! {
    pub enum FixtureProvenance {
        DocumentationExample => "documentation_example",
        Observed => "observed",
        Constructed => "constructed",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormatFixture {
    pub id: String,
    pub input: String,
    pub interpretation: String,
    pub provenance: FixtureProvenance,
    #[serde(default)]
    pub evidence: Vec<String>,
}

string_enum! {
    pub enum FormatFamily {
        PlainText => "plain_text",
        Markdown => "markdown",
        MarkdownSubset => "markdown_subset",
        Html => "html",
        HtmlSubset => "html_subset",
        ProviderMarkup => "provider_markup",
    }
}

string_enum! {
    pub enum AutoInterpretation {
        Links => "links",
        Mentions => "mentions",
        LinksAndMentions => "links_and_mentions",
        None => "none",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum MalformedBehavior {
        Reject => "reject",
        Literal => "literal",
        Strip => "strip",
        Transform => "transform",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormatProfile {
    pub id: String,
    pub family: FormatFamily,
    pub dialect: Option<String>,
    pub dialect_version: Option<String>,
    pub constructs: Constructs,
    pub escaping: Option<String>,
    pub nesting: Option<String>,
    pub newlines: Option<String>,
    pub auto_interpretation: AutoInterpretation,
    pub malformed_behavior: MalformedBehavior,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub fixtures: Vec<FormatFixture>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum DefaultState {
        Documented => "documented",
        Absent => "absent",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub field: String,
    pub values: Vec<String>,
    pub default: Option<String>,
    pub default_state: DefaultState,
}

string_enum! {
    pub enum FieldRelationshipKind {
        MutuallyExclusive => "mutually_exclusive",
        RequiredTogether => "required_together",
        OptionalCompanion => "optional_companion",
        FallbackFor => "fallback_for",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRelationship {
    pub kind: FieldRelationshipKind,
    pub target: String,
}

string_enum! {
    pub enum Representation {
        Text => "text",
        Markup => "markup",
        Entities => "entities",
        StructuredBlocks => "structured_blocks",
    }
}

string_enum! {
    pub enum ContentRole {
        Primary => "primary",
        NotificationFallback => "notification_fallback",
        AccessibilityFallback => "accessibility_fallback",
        CompatibilityFallback => "compatibility_fallback",
    }
}

string_enum! {
    pub enum Packaging {
        SingleFieldModeSwitch => "single_field_mode_switch",
        SeparateFields => "separate_fields",
        StructuredObject => "structured_object",
        SeparateOperations => "separate_operations",
        PlainOnly => "plain_only",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Visibility {
        VisibleContent => "visible_content",
        NotificationOnly => "notification_only",
        AccessibilityOnly => "accessibility_only",
        NotDisplayed => "not_displayed",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub surface: Surface,
    pub native_locator: String,
    pub representation: Representation,
    pub content_role: ContentRole,
    pub packaging: Packaging,
    pub visibility: Visibility,
    pub profiles: Vec<String>,
    pub selector: Option<Selector>,
    pub entity_offset_unit: Option<Unit>,
    #[serde(default)]
    pub relationships: Vec<FieldRelationship>,
    pub precedence: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub knowledge: Knowledge,
}

// ---- images ----------------------------------------------------------------

string_enum! {
    pub enum ImageRole {
        InlineContent => "inline_content",
        Attachment => "attachment",
        Primary => "primary",
        Thumbnail => "thumbnail",
        Accessory => "accessory",
        AuthorIcon => "author_icon",
        FooterIcon => "footer_icon",
        LinkPreview => "link_preview",
    }
}

string_enum! {
    pub enum Placement {
        InlineAnchor => "inline_anchor",
        StandaloneMedia => "standalone_media",
        CardMain => "card_main",
        CardThumbnail => "card_thumbnail",
        SectionAccessory => "section_accessory",
        AuthorArea => "author_area",
        FooterArea => "footer_area",
        Preview => "preview",
    }
}

string_enum! {
    pub enum PlacementControl {
        Explicit => "explicit",
        ProviderSelected => "provider_selected",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ImageSubmission {
        MarkupReference => "markup_reference",
        RequestField => "request_field",
        MultipartPart => "multipart_part",
        UploadThenReference => "upload_then_reference",
        SeparateOperation => "separate_operation",
    }
}

string_enum! {
    pub enum ImageSource {
        RemoteUrl => "remote_url",
        UploadBytes => "upload_bytes",
        ProviderMediaId => "provider_media_id",
        LocalPath => "local_path",
    }
}

string_enum! {
    pub enum ImageCollection {
        Single => "single",
        OrderedList => "ordered_list",
        Album => "album",
        Gallery => "gallery",
        Carousel => "carousel",
        ProviderSelected => "provider_selected",
    }
}

string_enum! {
    pub enum MultipleImages {
        RepeatedObjects => "repeated_objects",
        ArrayField => "array_field",
        MultipleRequests => "multiple_requests",
        NotSupported => "not_supported",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ReceiptGranularity {
        Single => "single",
        PerItem => "per_item",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum CaptionScope {
        PerItem => "per_item",
        Shared => "shared",
        None => "none",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Animation {
        Supported => "supported",
        Unsupported => "unsupported",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum SuppliedBy {
        CallerSupplied => "caller_supplied",
        ProviderDerived => "provider_derived",
        AccountScope => "account_scope",
        ApplicationScope => "application_scope",
    }
}

/// One binding per (native slot, role).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub container_surface: Option<Surface>,
    pub role: ImageRole,
    pub fidelity: Fidelity,
    pub native_locator: String,
    pub placement: Placement,
    pub placement_control: PlacementControl,
    pub submissions: Vec<ImageSubmission>,
    pub sources: Vec<ImageSource>,
    pub collection: ImageCollection,
    pub multiple_images: MultipleImages,
    pub ordering: YesNo,
    pub atomic: YesNo,
    pub receipts: ReceiptGranularity,
    pub min_items: Option<u64>,
    pub max_items: Option<u64>,
    #[serde(default)]
    pub shared_constraints: Vec<String>,
    pub caption_binding: Option<String>,
    pub caption_scope: Option<CaptionScope>,
    pub alt_text_binding: Option<String>,
    pub mime_types: Option<Vec<String>>,
    pub animation: Option<Animation>,
    pub transformations: Option<String>,
    pub supplied_by: SuppliedBy,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCell {
    pub state: super::common::State,
    pub support: Option<Support>,
    pub fidelity: Option<Fidelity>,
    pub bindings: Vec<String>,
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleMatrix {
    pub inline_content: RoleCell,
    pub attachment: RoleCell,
    pub primary: RoleCell,
    pub thumbnail: RoleCell,
    pub accessory: RoleCell,
    pub author_icon: RoleCell,
    pub footer_icon: RoleCell,
    pub link_preview: RoleCell,
}

impl RoleMatrix {
    pub fn get(&self, role: ImageRole) -> &RoleCell {
        match role {
            ImageRole::InlineContent => &self.inline_content,
            ImageRole::Attachment => &self.attachment,
            ImageRole::Primary => &self.primary,
            ImageRole::Thumbnail => &self.thumbnail,
            ImageRole::Accessory => &self.accessory,
            ImageRole::AuthorIcon => &self.author_icon,
            ImageRole::FooterIcon => &self.footer_icon,
            ImageRole::LinkPreview => &self.link_preview,
        }
    }

    /// All eight roles in schema order.
    pub fn iter(&self) -> impl Iterator<Item = (ImageRole, &RoleCell)> {
        ImageRole::ALL.iter().map(|role| (*role, self.get(*role)))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCoverage {
    pub interface: String,
    pub roles: RoleMatrix,
    pub unmapped_native_roles: Option<Vec<String>>,
}

// ---- attachments, addressing, receipts -------------------------------------

string_enum! {
    pub enum MediaKind {
        Image => "image",
        Video => "video",
        Audio => "audio",
        Voice => "voice",
        Document => "document",
        Animation => "animation",
        Sticker => "sticker",
        Other => "other",
    }
}

string_enum! {
    pub enum UploadMechanism {
        MultipartPart => "multipart_part",
        UploadThenReference => "upload_then_reference",
        RemoteUrl => "remote_url",
        ProviderMediaId => "provider_media_id",
        Base64Field => "base64_field",
        LocalPath => "local_path",
    }
}

string_enum! {
    pub enum FilenameControl {
        Caller => "caller",
        Provider => "provider",
        None => "none",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub media_kinds: Vec<MediaKind>,
    pub upload_mechanisms: Vec<UploadMechanism>,
    pub native_locator: Option<String>,
    pub mime_restrictions: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub caption_binding: Option<String>,
    pub alt_text_binding: Option<String>,
    pub filename_control: Option<FilenameControl>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum DestinationKind {
        Channel => "channel",
        DirectMessage => "direct_message",
        Group => "group",
        Thread => "thread",
        User => "user",
        PhoneNumber => "phone_number",
        Chat => "chat",
        WebhookBound => "webhook_bound",
        Other => "other",
    }
}

string_enum! {
    pub enum ReturnsIdentifier {
        Always => "always",
        Conditional => "conditional",
        Never => "never",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddressingFact {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub destination_kinds: Vec<DestinationKind>,
    pub reply_support: Option<Support>,
    pub thread_support: Option<Support>,
    pub reference_fields: Option<Vec<String>>,
    pub returns_identifier: ReturnsIdentifier,
    pub identifier_locator: Option<String>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum Acceptance {
        SynchronousResponse => "synchronous_response",
        None => "none",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum StatusChannel {
        Synchronous => "synchronous",
        AsynchronousEvent => "asynchronous_event",
        Unavailable => "unavailable",
        Unknown => "unknown",
    }
}

/// Accepted is not delivered; asynchronous states name their mechanism.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFact {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub acceptance: Acceptance,
    pub identifier_locator: Option<String>,
    pub delivered_state: StatusChannel,
    pub read_state: StatusChannel,
    pub status_mechanism: Option<String>,
    pub status_interface: Option<String>,
    pub warnings_locator: Option<String>,
    pub knowledge: Knowledge,
}

// ---- attribution, location, expression -------------------------------------

string_enum! {
    pub enum AttributionRole {
        SenderIdentity => "sender_identity",
        SenderDisplayName => "sender_display_name",
        SenderAvatar => "sender_avatar",
        ContentAuthor => "content_author",
        Signature => "signature",
        ForwardedOrigin => "forwarded_origin",
        DelegatedAuthor => "delegated_author",
    }
}

string_enum! {
    pub enum AttributionControl {
        PlatformDerived => "platform_derived",
        AccountConfigured => "account_configured",
        ApplicationConfigured => "application_configured",
        CallerSupplied => "caller_supplied",
        Mixed => "mixed",
    }
}

string_enum! {
    pub enum AttributionScope {
        Account => "account",
        Application => "application",
        Webhook => "webhook",
        Conversation => "conversation",
        Message => "message",
    }
}

string_enum! {
    pub enum AttributionInclusion {
        Always => "always",
        Optional => "optional",
        Conditional => "conditional",
        Unavailable => "unavailable",
    }
}

string_enum! {
    pub enum OverrideBehavior {
        Honored => "honored",
        Rejected => "rejected",
        Ignored => "ignored",
        Conditional => "conditional",
        NotApplicable => "not_applicable",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttributionBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub role: AttributionRole,
    pub native_fields: Vec<String>,
    pub control: AttributionControl,
    pub scope: AttributionScope,
    pub inclusion: AttributionInclusion,
    pub override_behavior: OverrideBehavior,
    pub changes_sender: YesNo,
    pub image_binding: Option<String>,
    pub format_profile: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub provenance_presentation: Option<String>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum LocationFieldName {
        Latitude => "latitude",
        Longitude => "longitude",
        CoordinateReferenceSystem => "coordinate_reference_system",
        Accuracy => "accuracy",
        Label => "label",
        Address => "address",
        Timestamp => "timestamp",
        Heading => "heading",
        LiveDuration => "live_duration",
        Other => "other",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocationField {
    pub name: LocationFieldName,
    pub locator: String,
    pub required: bool,
    pub unit: Option<String>,
    pub constraint: Option<String>,
}

string_enum! {
    pub enum LocationRole {
        AuthorLocation => "author_location",
        SharedPlace => "shared_place",
        LiveLocation => "live_location",
    }
}

string_enum! {
    pub enum LocationSubject {
        Author => "author",
        Device => "device",
        Place => "place",
        Unspecified => "unspecified",
    }
}

string_enum! {
    pub enum LocationMode {
        Static => "static",
        Live => "live",
    }
}

string_enum! {
    pub enum LocationAssociation {
        PlatformAsserted => "platform_asserted",
        CallerAsserted => "caller_asserted",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum LocationOrigin {
        CallerSupplied => "caller_supplied",
        DeviceDerived => "device_derived",
        AccountDerived => "account_derived",
        PlatformDerived => "platform_derived",
    }
}

string_enum! {
    pub enum LocationInclusion {
        Always => "always",
        WhenSupplied => "when_supplied",
        Conditional => "conditional",
        Unavailable => "unavailable",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum LocationRepresentation {
        StructuredFields => "structured_fields",
        DedicatedOperation => "dedicated_operation",
        StructuredObject => "structured_object",
        TextFallback => "text_fallback",
    }
}

string_enum! {
    pub enum LocationDelivery {
        SameMessage => "same_message",
        ReplacesText => "replaces_text",
        SeparateMessage => "separate_message",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum LocationPresentation {
        Map => "map",
        PlaceCard => "place_card",
        Link => "link",
        Text => "text",
        Unknown => "unknown",
    }
}

/// Subject and origin describe provider behavior only; they never authorize
/// Messenger to discover or transmit the host's location.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocationBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub role: LocationRole,
    pub subject: LocationSubject,
    pub mode: LocationMode,
    pub association: LocationAssociation,
    pub origin: LocationOrigin,
    pub inclusion: LocationInclusion,
    pub representation: LocationRepresentation,
    #[serde(default)]
    pub fields: Vec<LocationField>,
    pub delivery: LocationDelivery,
    pub presentation: LocationPresentation,
    pub coexists_with: Option<Vec<String>>,
    pub live_behavior: Option<String>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum GeolocationExposure {
        AutomaticallyExposed => "automatically_exposed",
        OptionallySupplied => "optionally_supplied",
        Unavailable => "unavailable",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorGeolocation {
    pub id: String,
    pub interface: String,
    pub exposure: GeolocationExposure,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum ExpressionMechanism {
        MessageEffect => "message_effect",
        TextEffect => "text_effect",
        Reaction => "reaction",
        EmojiContent => "emoji_content",
        StickerContent => "sticker_content",
    }
}

string_enum! {
    pub enum ApiAvailability {
        Api => "api",
        AppOnly => "app_only",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ExpressionControl {
        CallerChosen => "caller_chosen",
        ContentTriggered => "content_triggered",
        ProviderSelected => "provider_selected",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ExpressionScope {
        SelectedText => "selected_text",
        MessageBubble => "message_bubble",
        Conversation => "conversation",
        ProviderDefined => "provider_defined",
        NotApplicable => "not_applicable",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ExpressionDiscovery {
        FixedDocumented => "fixed_documented",
        DynamicCatalog => "dynamic_catalog",
        NotApplicable => "not_applicable",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Intent {
        Emphasis => "emphasis",
        Celebration => "celebration",
        Joy => "joy",
        Affection => "affection",
        Anger => "anger",
        Sadness => "sadness",
        Surprise => "surprise",
        Unmapped => "unmapped",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionBinding {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub mechanism: ExpressionMechanism,
    pub support: Option<Support>,
    pub api_availability: ApiAvailability,
    pub native_identifier: Option<String>,
    pub native_fields: Option<Vec<String>>,
    pub control: ExpressionControl,
    pub scope: ExpressionScope,
    pub discovery: ExpressionDiscovery,
    pub intent: Intent,
    pub intent_fidelity: Fidelity,
    pub rendering: Option<String>,
    pub coexistence: Option<String>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

// ---- delivery controls, eligibility, rate limits ----------------------------

string_enum! {
    pub enum DeliveryControlKind {
        SilentDelivery => "silent_delivery",
        LinkPreview => "link_preview",
        MentionControl => "mention_control",
        Edit => "edit",
        Delete => "delete",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryControl {
    pub id: String,
    pub interface: String,
    pub operation: String,
    pub control: DeliveryControlKind,
    pub support: Option<Support>,
    pub native_fields: Option<Vec<String>>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum EligibilityKind {
        AuthenticationScheme => "authentication_scheme",
        RequiredScope => "required_scope",
        Membership => "membership",
        Registration => "registration",
        ConversationWindow => "conversation_window",
        TemplatePrerequisite => "template_prerequisite",
        AccountType => "account_type",
        Other => "other",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EligibilityRequirement {
    pub id: String,
    pub interface: String,
    pub kind: EligibilityKind,
    pub description: String,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum RateScope {
        Global => "global",
        Route => "route",
        Resource => "resource",
        Conversation => "conversation",
        Account => "account",
        Application => "application",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum RateBasis {
        Documented => "documented",
        Observed => "observed",
        Dynamic => "dynamic",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum RetryAfterUnit {
        Seconds => "seconds",
        Milliseconds => "milliseconds",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Idempotency {
        Supported => "supported",
        Unsupported => "unsupported",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimit {
    pub id: String,
    pub interface: String,
    pub operations: Vec<String>,
    pub scope: RateScope,
    pub basis: RateBasis,
    pub rate: Option<u64>,
    pub per_seconds: Option<f64>,
    pub burst: Option<u64>,
    pub retry_after_locator: Option<String>,
    pub retry_after_unit: Option<RetryAfterUnit>,
    #[serde(default)]
    pub errors: Vec<String>,
    pub idempotency: Idempotency,
    pub knowledge: Knowledge,
}

// ---- errors and diagnostics ------------------------------------------------

string_enum! {
    pub enum Origin {
        Service => "service",
        Sdk => "sdk",
        Bridge => "bridge",
        Transport => "transport",
    }
}

string_enum! {
    pub enum BodyFormat {
        Json => "json",
        PlainText => "plain_text",
        Empty => "empty",
        SdkError => "sdk_error",
        JsonRpc => "json_rpc",
        Unknown => "unknown",
    }
}

/// Where an interface's responses carry status, codes, and diagnostics.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub id: String,
    pub interface: String,
    pub operations: Vec<String>,
    pub origin: Origin,
    pub body_format: BodyFormat,
    pub success_discriminator: Option<String>,
    pub code_locator: Option<String>,
    pub subcode_locator: Option<String>,
    pub message_locator: Option<String>,
    pub field_errors_locator: Option<String>,
    pub warnings_locator: Option<String>,
    pub correlation_id_locator: Option<String>,
    pub retry_after_locator: Option<String>,
    pub retry_after_unit: Option<RetryAfterUnit>,
    pub knowledge: Knowledge,
}

impl Envelope {
    /// Every locator the envelope defines, in field order.
    pub fn locators(&self) -> impl Iterator<Item = &str> {
        [
            &self.success_discriminator,
            &self.code_locator,
            &self.subcode_locator,
            &self.message_locator,
            &self.field_errors_locator,
            &self.warnings_locator,
            &self.correlation_id_locator,
            &self.retry_after_locator,
        ]
        .into_iter()
        .filter_map(|locator| locator.as_deref())
    }
}

/// A conjunction: every present predicate must hold.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSignature {
    pub http_status: Option<u16>,
    pub native_code_number: Option<i64>,
    pub native_code_string: Option<String>,
    pub native_subcode: Option<String>,
    pub discriminator_locator: Option<String>,
    pub discriminator_equals: Option<String>,
    pub exact_text_token: Option<String>,
    pub sdk_error_variant: Option<String>,
}

impl MatchSignature {
    /// The present predicates as comparable `(name, value)` pairs.
    pub fn predicates(&self) -> Vec<(&'static str, String)> {
        let mut predicates = Vec::new();
        let mut push = |name: &'static str, value: Option<String>| {
            if let Some(value) = value {
                predicates.push((name, value));
            }
        };
        push("http_status", self.http_status.map(|v| v.to_string()));
        push("native_code_number", self.native_code_number.map(|v| v.to_string()));
        push("native_code_string", self.native_code_string.clone());
        push("native_subcode", self.native_subcode.clone());
        push("discriminator_locator", self.discriminator_locator.clone());
        push("discriminator_equals", self.discriminator_equals.clone());
        push("exact_text_token", self.exact_text_token.clone());
        push("sdk_error_variant", self.sdk_error_variant.clone());
        predicates
    }
}

string_enum! {
    pub enum ErrorPhase {
        BeforeSubmission => "before_submission",
        Response => "response",
        DeliveryEvent => "delivery_event",
    }
}

string_enum! {
    pub enum ErrorOutcome {
        Failure => "failure",
        Warning => "warning",
    }
}

string_enum! {
    pub enum ErrorCategory {
        Authentication => "authentication",
        Permission => "permission",
        InvalidDestination => "invalid_destination",
        InvalidContent => "invalid_content",
        ContentTooLarge => "content_too_large",
        UnsupportedFeature => "unsupported_feature",
        DeliveryPolicy => "delivery_policy",
        RateLimited => "rate_limited",
        QuotaExhausted => "quota_exhausted",
        Conflict => "conflict",
        ServiceUnavailable => "service_unavailable",
        Transport => "transport",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum DeliveryCertainty {
        NotSubmitted => "not_submitted",
        Rejected => "rejected",
        Accepted => "accepted",
        Partial => "partial",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Recovery {
        RetryCandidate => "retry_candidate",
        AfterCorrection => "after_correction",
        DoNotRetry => "do_not_retry",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum ReplaySafety {
        Safe => "safe",
        Unsafe => "unsafe",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Remediation {
        RefreshCredentials => "refresh_credentials",
        GrantPermission => "grant_permission",
        CorrectDestination => "correct_destination",
        ShortenContent => "shorten_content",
        ChangeFormat => "change_format",
        Wait => "wait",
        CheckService => "check_service",
        ContactOperator => "contact_operator",
        None => "none",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorRecord {
    pub id: String,
    pub interface: String,
    pub operations: Vec<String>,
    pub envelope: String,
    pub origin: Origin,
    pub phase: ErrorPhase,
    pub outcome: ErrorOutcome,
    pub category: ErrorCategory,
    /// Executable signature; only a `known` error may carry one.
    #[serde(rename = "match")]
    pub signature: Option<MatchSignature>,
    /// Research lead for an `unknown` error; never executable.
    pub candidate_match: Option<MatchSignature>,
    #[serde(default)]
    pub related_facts: Vec<String>,
    pub affected_fields: Option<Vec<String>>,
    pub delivery_certainty: DeliveryCertainty,
    pub recovery: Recovery,
    pub replay_safety: ReplaySafety,
    pub recovery_prerequisites: Option<String>,
    pub remediation: Remediation,
    pub remediation_note: Option<String>,
    pub documentation: Option<String>,
    pub diagnostic_fields: Option<Vec<String>>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    #[serde(default)]
    pub fixtures: Vec<String>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum CaptureProvenance {
        Captured => "captured",
        ConstructedFromDocs => "constructed_from_docs",
        ObservedInRepo => "observed_in_repo",
    }
}

string_enum! {
    pub enum FixtureExpectation {
        Match => "match",
        NoMatch => "no_match",
        Unknown => "unknown",
    }
}

/// A sanitized response replayed against a scope's executable signatures.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorFixture {
    pub id: String,
    pub envelope: String,
    pub operation: String,
    pub provenance: CaptureProvenance,
    pub expect: FixtureExpectation,
    pub expected_error: Option<String>,
    pub http_status: Option<u16>,
    #[serde(default)]
    pub headers: Vec<String>,
    pub body: Option<String>,
    pub sdk_variant: Option<String>,
    pub note: Option<String>,
}

// ---- interactivity ---------------------------------------------------------

string_enum! {
    pub enum RepeatSubmissions {
        Allowed => "allowed",
        Rejected => "rejected",
        ReplacesPrevious => "replaces_previous",
        Unknown => "unknown",
    }
}

/// Durations carry their unit in the field name.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lifecycle {
    pub ack_deadline_ms: Option<u64>,
    pub response_token_seconds: Option<u64>,
    pub answer_window_seconds: Option<u64>,
    pub repeat_submissions: Option<RepeatSubmissions>,
    pub closable: Option<YesNo>,
    pub expiry: Option<String>,
    pub requires_user_action: Option<YesNo>,
    pub proactive: Option<YesNo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InboundPayload {
    pub text: Option<String>,
    pub sender: Option<String>,
    pub conversation: Option<String>,
    pub message_ref: Option<String>,
    pub event_id: Option<String>,
    pub timestamp: Option<String>,
    pub edits: Option<String>,
    pub deletions: Option<String>,
}

string_enum! {
    pub enum InboundMechanism {
        PushWebhook => "push_webhook",
        PersistentConnection => "persistent_connection",
        LongPoll => "long_poll",
        Poll => "poll",
        LocalEventStream => "local_event_stream",
        None => "none",
    }
}

string_enum! {
    pub enum InboundReach {
        Direct => "direct",
        Groups => "groups",
        Mentions => "mentions",
        Replies => "replies",
        CommandsOnly => "commands_only",
        InteractionsOnly => "interactions_only",
        None => "none",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum InboundContent {
        FullText => "full_text",
        PartialText => "partial_text",
        MetadataOnly => "metadata_only",
        None => "none",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InboundBinding {
    pub id: String,
    pub interface: String,
    pub event: String,
    pub mechanism: InboundMechanism,
    pub reach: Vec<InboundReach>,
    pub content: InboundContent,
    pub payload: Option<InboundPayload>,
    pub authentication: Option<String>,
    pub ack_deadline_ms: Option<u64>,
    pub redelivery: Option<YesNo>,
    pub ordering: Option<YesNo>,
    pub dedup_locator: Option<String>,
    pub replay: Option<String>,
    pub retention: Option<String>,
    pub prerequisites: Option<Vec<String>>,
    pub reuses_send_identity: YesNo,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationMapping {
    pub affirmative: String,
    pub negative: String,
}

string_enum! {
    pub enum QuestionKind {
        Confirmation => "confirmation",
        SingleChoice => "single_choice",
        MultipleChoice => "multiple_choice",
        TextInput => "text_input",
    }
}

string_enum! {
    pub enum Nativeness {
        Native => "native",
        ApplicationManaged => "application_managed",
    }
}

string_enum! {
    pub enum QuestionMechanism {
        Buttons => "buttons",
        LinkButton => "link_button",
        SelectMenu => "select_menu",
        NativePoll => "native_poll",
        TextInput => "text_input",
        FreeTextInterpretation => "free_text_interpretation",
        ReactionInterpretation => "reaction_interpretation",
        Other => "other",
    }
}

string_enum! {
    pub enum AnswerType {
        Boolean => "boolean",
        OptionId => "option_id",
        OptionIdArray => "option_id_array",
        String => "string",
        None => "none",
    }
}

string_enum! {
    pub enum OptionSource {
        Static => "static",
        Dynamic => "dynamic",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Cancellation {
        DistinctOutcome => "distinct_outcome",
        Indistinguishable => "indistinguishable",
        NotAvailable => "not_available",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Responses {
        Attributable => "attributable",
        Anonymous => "anonymous",
        AggregateOnly => "aggregate_only",
        Conditional => "conditional",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum AnswerValidation {
        Provider => "provider",
        Application => "application",
        Both => "both",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum AnswerVisibility {
        Public => "public",
        Private => "private",
        Ephemeral => "ephemeral",
        Conditional => "conditional",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionBinding {
    pub id: String,
    pub interface: String,
    pub kind: QuestionKind,
    pub native: Nativeness,
    pub mechanism: QuestionMechanism,
    pub operation: String,
    pub native_locator: String,
    pub companion_interface: Option<String>,
    pub answer_event: Option<String>,
    pub answer_locator: Option<String>,
    pub answer_type: AnswerType,
    pub confirmation_mapping: Option<ConfirmationMapping>,
    pub option_id_locator: Option<String>,
    pub options: Option<OptionSource>,
    pub option_label_constraint: Option<String>,
    pub option_value_constraint: Option<String>,
    pub min_selections: Option<u64>,
    pub max_selections: Option<u64>,
    pub defaults: Option<Support>,
    pub cancellation: Cancellation,
    pub responder_locator: Option<String>,
    pub correlation_locator: Option<String>,
    pub opaque_state_constraint: Option<String>,
    pub responses: Responses,
    pub validation: AnswerValidation,
    pub visibility: AnswerVisibility,
    pub lifecycle: Option<Lifecycle>,
    pub fidelity: Fidelity,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormField {
    pub id: String,
    pub kind: QuestionKind,
    pub required: bool,
    pub native_locator: String,
    pub default: Option<String>,
}

string_enum! {
    pub enum FormContainer {
        Modal => "modal",
        MessageControls => "message_controls",
        NativeForm => "native_form",
        SequentialApplication => "sequential_application",
        ExternalForm => "external_form",
        None => "none",
    }
}

string_enum! {
    pub enum Submission {
        SingleEvent => "single_event",
        PerFieldEvents => "per_field_events",
        Both => "both",
        SequentialApplication => "sequential_application",
        External => "external",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum AbsentVsEmpty {
        Distinguishable => "distinguishable",
        Indistinguishable => "indistinguishable",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum FormValidation {
        Field => "field",
        CrossField => "cross_field",
        Both => "both",
        None => "none",
        Unknown => "unknown",
    }
}

string_enum! {
    pub enum Navigation {
        SinglePage => "single_page",
        NativeMultiPage => "native_multi_page",
        NotApplicable => "not_applicable",
        Unknown => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FormBinding {
    pub id: String,
    pub interface: String,
    pub container: FormContainer,
    pub opened_by: Option<String>,
    pub question_kinds: Vec<QuestionKind>,
    #[serde(default)]
    pub fields: Vec<FormField>,
    pub max_fields: Option<u64>,
    pub submission: Submission,
    pub partial_updates: Option<YesNo>,
    pub absent_vs_empty: AbsentVsEmpty,
    pub cancellation: Cancellation,
    pub conditional_fields: Option<Support>,
    pub validation: FormValidation,
    pub navigation: Navigation,
    pub companion_interface: Option<String>,
    pub lifecycle: Option<Lifecycle>,
    #[serde(default)]
    pub applies_when: Vec<Condition>,
    pub knowledge: Knowledge,
}

string_enum! {
    pub enum InteractionExpectation {
        Answer => "answer",
        Cancelled => "cancelled",
        Dismissed => "dismissed",
        Timeout => "timeout",
        NoAnswer => "no_answer",
        StaleOption => "stale_option",
        AggregateOnly => "aggregate_only",
        NotAnAnswer => "not_an_answer",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionFixture {
    pub id: String,
    pub binding: String,
    pub provenance: CaptureProvenance,
    pub payload: String,
    pub expect: InteractionExpectation,
    /// The canonical answer as JSON text.
    pub expected_answer: Option<String>,
    pub note: Option<String>,
}

// ---- lifecycle -------------------------------------------------------------

string_enum! {
    pub enum GapKind {
        Research => "research",
        RequiresMessengerUpdate => "requires_messenger_update",
        Vocabulary => "vocabulary",
    }
}

string_enum! {
    pub enum GapStatus {
        Open => "open",
        Investigated => "investigated",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    pub id: String,
    pub kind: GapKind,
    pub status: GapStatus,
    pub question: String,
    pub interface: Option<String>,
    pub category: Option<Category>,
    pub facts: Vec<String>,
    #[serde(default)]
    pub searches: Vec<String>,
    #[serde(default)]
    pub inspected_sources: Vec<String>,
    pub unresolved_reason: Option<String>,
    pub blocked_decision: Option<String>,
    pub next_investigation: String,
}

string_enum! {
    pub enum ChangeKind {
        Added => "added",
        Changed => "changed",
        Removed => "removed",
        Unresolved => "unresolved",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    pub facts: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub summary: String,
}
