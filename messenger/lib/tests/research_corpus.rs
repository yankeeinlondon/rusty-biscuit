//! Passive corpus for the provider research contract.
//!
//! Validates every shipped contract artifact (roster, schemas, platform
//! documents) and every fixture under `tests/fixtures/research/` through
//! Darkmatter's library schema validation, the same code path as
//! `md schema validate`. Semantic rules that SimplifiedSchema cannot express
//! are Rust-owned (`docs/research/platforms/_rules.md`); their fixtures must
//! stay schema-valid here so the rule cannot silently move into the schema.
//!
//! Fixture conventions (see `tests/fixtures/research/README.md`):
//! - `negative/schema/*` carry `# expect-problem: <pointer or text>`;
//! - `negative/semantic/*` carry `# expect-rule: SR-*` and a matching stem.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationReport};
use messenger::ProviderKind;
use serde_json::Value;

/// `messenger/lib`, read at run time: nextest's archive runs (the WSL2 CI
/// leg) remap `CARGO_MANIFEST_DIR` onto the extracted workspace, while the
/// compile-time `env!` value names the builder's checkout.
fn lib_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn messenger_dir() -> PathBuf {
    lib_dir()
        .parent()
        .expect("messenger/lib has a parent")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    lib_dir().join("tests/fixtures/research")
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
}

fn load(path: &Path) -> Markdown {
    Markdown::try_from(path).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// One validator cache per test process, so each schema compiles once.
fn schemas() -> &'static DarkmatterSchemas {
    static SCHEMAS: OnceLock<DarkmatterSchemas> = OnceLock::new();
    SCHEMAS.get_or_init(DarkmatterSchemas::new)
}

/// Resolved schemas by canonical `$schema` path. `DarkmatterSchemas::validate`
/// re-resolves a schema's imports for every document (about 100 ms each in
/// debug builds), which pushed these tests toward nextest's termination
/// limit under full-suite load; the validator itself is the same.
fn effective(markdown: &Markdown, schema: &Path) -> Arc<EffectiveSchema> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Arc<EffectiveSchema>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let key = schema.canonicalize().unwrap_or_else(|e| panic!("resolve {}: {e}", schema.display()));
    if let Some(found) = cache.lock().expect("cache").get(&key) {
        return Arc::clone(found);
    }
    let resolved = schemas()
        .effective_for(markdown)
        .unwrap_or_else(|e| panic!("schema error for {}: {e}", schema.display()))
        .expect("a declared schema");
    let resolved = Arc::new(resolved);
    cache.lock().expect("cache").insert(key, Arc::clone(&resolved));
    resolved
}

/// Validates one file; a schema that fails to load is a test failure, not a
/// validation problem.
fn validate(path: &Path) -> ValidationReport {
    let markdown = load(path);
    let declared = markdown
        .frontmatter()
        .get::<String>("$schema")
        .ok()
        .flatten()
        .unwrap_or_else(|| panic!("{} declares no $schema, so validation would be vacuous", path.display()));
    let schema = path.parent().expect("parent").join(declared);
    let mut frontmatter: serde_json::Map<String, Value> =
        markdown.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    frontmatter.remove("$schema");
    effective(&markdown, &schema).validate(&Value::Object(frontmatter))
}

fn describe(report: &ValidationReport) -> String {
    report
        .problems
        .iter()
        .map(|p| format!("{} {}", p.path, p.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn header_value(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).expect("read fixture");
    let prefix = format!("# {key}: ");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix).map(|v| v.trim().to_string()))
}

fn frontmatter_value(path: &Path, key: &str) -> Value {
    load(path)
        .frontmatter()
        .get::<Value>(key)
        .unwrap_or_else(|e| panic!("{key} in {}: {e}", path.display()))
        .unwrap_or_else(|| panic!("{} has no {key}", path.display()))
}

fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is not an array"))
        .iter()
        .map(|v| v.as_str().expect("string item"))
        .collect()
}

// ---- shipped artifacts ---------------------------------------------------

#[test]
fn shipped_roster_validates_against_its_schema() {
    let report = validate(&messenger_dir().join("docs/platforms.yaml"));
    assert!(report.valid, "roster: {}", describe(&report));
}

/// The roster's adapter IDs are persisted identifiers: they must be exactly
/// the chat `ProviderKind::as_str()` spellings, each mapped once, while
/// research-only companions map none and non-chat providers stay excluded.
#[test]
fn shipped_roster_maps_every_chat_adapter_exactly_once() {
    let roster = messenger_dir().join("docs/platforms.yaml");
    let platforms = frontmatter_value(&roster, "platforms");
    let excluded = frontmatter_value(&roster, "excluded");
    let cap = frontmatter_value(&roster, "curated_source_cap")
        .as_u64()
        .expect("numeric cap");

    let chat_adapters: BTreeSet<&str> = [
        ProviderKind::Discord,
        ProviderKind::DiscordWebhook,
        ProviderKind::Slack,
        ProviderKind::SlackWebhook,
        ProviderKind::Telegram,
        ProviderKind::WhatsApp,
        ProviderKind::Signal,
    ]
    .into_iter()
    .map(ProviderKind::as_str)
    .collect();

    let mut active = BTreeSet::new();
    let mut mapped: BTreeMap<String, usize> = BTreeMap::new();
    let mut sending_interfaces = 0;
    for platform in platforms.as_array().expect("platforms array") {
        let id = platform["platform_id"].as_str().expect("platform_id");
        assert_eq!(platform["status"], "active", "{id} status");
        assert_eq!(platform["file"], format!("{id}.md"), "{id} file");
        active.insert(id.to_string());

        let curated = platform["curated_sources"].as_array().expect("curated sources");
        assert!(curated.len() as u64 <= cap, "{id}: {} curated sources exceed cap {cap}", curated.len());

        for interface in platform["interfaces"].as_array().expect("interfaces") {
            let adapters = strings(interface, "adapters");
            match interface["role"].as_str() {
                Some("sending_adapter") => {
                    sending_interfaces += 1;
                    assert!(!adapters.is_empty(), "{id}: sending interface without adapter");
                }
                Some("research_only") => {
                    assert!(adapters.is_empty(), "{id}: research-only interface maps {adapters:?}");
                }
                other => panic!("{id}: unexpected role {other:?}"),
            }
            for adapter in adapters {
                *mapped.entry(adapter.to_string()).or_default() += 1;
            }
        }
    }

    let expected_platforms: BTreeSet<String> = ["discord", "slack", "telegram", "whatsapp", "signal"]
        .map(String::from)
        .into();
    assert_eq!(active, expected_platforms);
    assert_eq!(sending_interfaces, 7);
    assert_eq!(
        mapped.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        chat_adapters
    );
    assert!(mapped.values().all(|count| *count == 1), "adapter mapped twice: {mapped:?}");

    let excluded_subjects: BTreeSet<&str> = excluded
        .as_array()
        .expect("excluded array")
        .iter()
        .map(|e| e["subject"].as_str().expect("subject"))
        .collect();
    for subject in [
        "email",
        ProviderKind::Desktop.as_str(),
        ProviderKind::Apns.as_str(),
        ProviderKind::Fcm.as_str(),
    ] {
        assert!(excluded_subjects.contains(subject), "{subject} is not explicitly excluded");
    }
}

/// Every roster document either declares the research schema and validates,
/// or is unmigrated legacy prose whose prompt delegates to the shared fleet
/// instructions instead of carrying its own copy of the contract. The legacy
/// allowance ends with the first publication (`publication.json`); from then
/// on `typed::shipped_publication_is_accepted_and_drift_free` holds every
/// document to accepted scope.
#[test]
fn shipped_platform_documents_validate_or_delegate_to_the_fleet() {
    let dir = messenger_dir().join("docs/research/platforms");
    let published = messenger_dir().join("docs/research/publication.json").exists();
    for id in ["discord", "slack", "telegram", "whatsapp", "signal"] {
        let path = dir.join(format!("{id}.md"));
        let markdown = load(&path);
        let declared = markdown.frontmatter().get::<String>("$schema").ok().flatten();
        if declared.is_some() {
            let report = validate(&path);
            assert!(report.valid, "{id}: {}", describe(&report));
            continue;
        }
        assert!(!published, "{id} is legacy prose, but the accepted baseline is already published");
        let prompt: String = markdown
            .frontmatter()
            .get("prompt")
            .expect("prompt")
            .unwrap_or_else(|| panic!("{id} has neither $schema nor a delegating prompt"));
        assert!(
            prompt.contains("messenger/docs/research/platforms/_fleet.md"),
            "{id} prompt does not delegate to the fleet instructions"
        );
        assert!(prompt.contains("Pass 2"), "{id} prompt must be reconciliation-only");
        assert!(prompt.contains(&format!("platform_id: {id}")), "{id} prompt names another platform");
    }
}

#[test]
fn rules_document_lists_every_rule_the_semantic_corpus_names() {
    let rules = fs::read_to_string(messenger_dir().join("docs/research/platforms/_rules.md"))
        .expect("read _rules.md");
    for path in files_in(&fixtures_dir().join("negative/semantic")) {
        let rule = header_value(&path, "expect-rule")
            .unwrap_or_else(|| panic!("{} has no expect-rule header", path.display()));
        assert!(rules.contains(&format!("| {rule} |")), "{rule} is not documented in _rules.md");
    }
}

// ---- fixture corpus ------------------------------------------------------

fn assert_all_valid(dir: &str, min: usize) {
    let files = files_in(&fixtures_dir().join(dir));
    assert!(files.len() >= min, "{dir} corpus shrank to {} fixtures", files.len());
    for path in files {
        let report = validate(&path);
        assert!(report.valid, "{}: {}", path.display(), describe(&report));
    }
}

#[test]
fn contract_fixtures_validate() {
    assert_all_valid("contract", 40);
}

#[test]
fn interaction_fixtures_validate() {
    assert_all_valid("interaction", 12);
}

#[test]
fn diagnostic_fixtures_validate() {
    assert_all_valid("diagnostics", 6);
}

/// The accepted-fleet publication baseline (`lifecycle/fleet/`): one
/// document per roster platform.
#[test]
fn fleet_fixtures_validate() {
    assert_all_valid("lifecycle/fleet", 5);
}

/// The ported pilots are the schema-version-1 freeze gate: all four must
/// pass, and the investigated-gap fixture proves every category can record
/// an investigated gap.
#[test]
fn freeze_gate_fixtures_validate() {
    let contract = fixtures_dir().join("contract");
    for name in [
        "pilot-discord.md",
        "pilot-telegram.md",
        "pilot-slack.md",
        "pilot-signal.md",
        "gap-investigated.md",
    ] {
        let report = validate(&contract.join(name));
        assert!(report.valid, "{name}: {}", describe(&report));
    }

    let gap_fixture = contract.join("gap-investigated.md");
    let coverage = frontmatter_value(&gap_fixture, "coverage");
    let categories = coverage[0]["categories"].as_object().expect("categories");
    assert_eq!(categories.len(), 16, "coverage matrix is not complete");
    assert!(categories.values().all(|cell| cell["status"] == "gap"));
}

#[test]
fn schema_negative_fixtures_fail_at_their_declared_problem() {
    let files = files_in(&fixtures_dir().join("negative/schema"));
    assert!(files.len() >= 25, "schema-negative corpus shrank to {}", files.len());
    for path in files {
        let expected = header_value(&path, "expect-problem")
            .unwrap_or_else(|| panic!("{} has no expect-problem header", path.display()));
        let report = validate(&path);
        assert!(!report.valid, "{} unexpectedly validated", path.display());
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.path.contains(&expected) || p.message.contains(&expected)),
            "{} failed for another reason (expected {expected}): {}",
            path.display(),
            describe(&report)
        );
    }
}

/// Rules owned by `research::validate::{identity, constraints, coverage}`
/// and overrides; the remaining rules run in the second half.
const FIRST_HALF_RULES: &[&str] = &[
    "sr-roster", "sr-curated", "sr-unique", "sr-ref", "sr-evidence", "sr-strict-scalars", "sr-state-value",
    "sr-condition", "sr-applicability", "sr-aggregate", "sr-kind-unit", "sr-enforceable", "sr-coverage",
    "sr-gap", "sr-change", "sr-override",
];

fn assert_semantic_fixtures(first_half: bool) {
    let mut checked = 0;
    for path in files_in(&fixtures_dir().join("negative/semantic")) {
        let rule = header_value(&path, "expect-rule").expect("expect-rule header");
        let stem = path.file_name().and_then(|n| n.to_str()).expect("file name");
        let prefix = rule.to_lowercase();
        assert!(stem.starts_with(&format!("{prefix}--")), "{stem} does not start with its rule code {rule}");
        if FIRST_HALF_RULES.contains(&prefix.as_str()) != first_half {
            continue;
        }
        let report = validate(&path);
        assert!(report.valid, "{stem} must stay schema-valid: {}", describe(&report));
        checked += 1;
    }
    assert!(checked >= 30, "semantic-negative corpus half shrank to {checked}");
}

#[test]
fn semantic_negative_fixtures_for_identity_and_constraint_rules_are_schema_valid() {
    assert_semantic_fixtures(true);
}

#[test]
fn semantic_negative_fixtures_for_binding_interaction_and_error_rules_are_schema_valid() {
    assert_semantic_fixtures(false);
}

// ---- passivity -----------------------------------------------------------

/// Validation must be passive: no effect engine, no network attempt, and no
/// mutation of any validated file. The allowlist fixture carries an
/// expression-shaped `prompt` that would spawn a process if evaluated. The
/// sample covers every schema (roster, document, overrides, mappings) and
/// both negative kinds; the per-directory tests validate the full corpus.
#[test]
fn validation_is_passive() {
    let root = fixtures_dir();
    let mut paths = vec![messenger_dir().join("docs/platforms.yaml")];
    for name in [
        "composition-allowlist.md",
        "pilot-discord.md",
        "pilot-telegram.md",
        "pilot-slack.md",
        "pilot-signal.md",
        "overrides-valid.yaml",
        "mappings-valid.yaml",
    ] {
        paths.push(root.join("contract").join(name));
    }
    for dir in ["negative/schema", "negative/semantic"] {
        paths.extend(files_in(&root.join(dir)).into_iter().take(3));
    }
    let before: Vec<Vec<u8>> = paths.iter().map(|p| fs::read(p).expect("read")).collect();
    let engines = darkmatter::effects::engine_build_count();
    let network = darkmatter::effects::network_attempt_count();

    for path in &paths {
        let _ = validate(path);
    }

    assert_eq!(darkmatter::effects::engine_build_count(), engines, "validation built an effect engine");
    assert_eq!(darkmatter::effects::network_attempt_count(), network, "validation attempted network access");
    for (path, bytes) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).expect("reread"), bytes, "{} changed", path.display());
    }
}

// ---- sanitization --------------------------------------------------------

const SECRET_MARKERS: &[&str] = &[
    "xoxb-",
    "xoxp-",
    "xapp-",
    "xoxe.",
    "Bearer ",
    "-----BEGIN",
    "hooks.slack.com/services/T",
    "api.telegram.org/bot1",
    "api.telegram.org/bot2",
    "api.telegram.org/bot5",
    "api.telegram.org/bot6",
    "api.telegram.org/bot7",
];

const ESCAPE_SPELLINGS: &[&str] = &["\\e[", "\\x1b", "\\x1B", "\\u001b", "\\u001B", "\\033", "\\u009b", "\\u009B"];

/// Returns why `text` is unsafe to store as a fixture, or `None`.
fn unsafe_content(text: &str) -> Option<String> {
    if let Some(c) = text.chars().find(|c| c.is_control() && !matches!(c, '\n' | '\t' | '\r')) {
        return Some(format!("control character U+{:04X}", c as u32));
    }
    if let Some(esc) = ESCAPE_SPELLINGS.iter().find(|e| text.contains(**e)) {
        return Some(format!("escaped control sequence {esc}"));
    }
    if let Some(marker) = SECRET_MARKERS.iter().find(|m| text.contains(**m)) {
        return Some(format!("credential marker {marker:?}"));
    }
    // A webhook URL with a numeric ID carries its secret token after it;
    // templates spell the ID `{webhook_id}`.
    if let Some(at) = text.find("/webhooks/")
        && text[at + "/webhooks/".len()..].starts_with(|c: char| c.is_ascii_digit())
    {
        return Some("webhook URL with a concrete ID".to_string());
    }
    // Telegram bot tokens: 8-10 digits, a colon, then 35 token characters.
    let bytes = text.as_bytes();
    for (i, _) in text.match_indices(':') {
        let digits = bytes[..i].iter().rev().take_while(|b| b.is_ascii_digit()).count();
        let tail = bytes[i + 1..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_' || **b == b'-')
            .count();
        if (8..=10).contains(&digits) && tail >= 35 {
            return Some("bot-token shape".to_string());
        }
    }
    // E.164 phone numbers: a plus sign followed by ten or more digits.
    for (i, _) in text.match_indices('+') {
        if bytes[i + 1..].iter().take_while(|b| b.is_ascii_digit()).count() >= 10 {
            return Some("phone-number shape".to_string());
        }
    }
    None
}

#[test]
fn sanitization_scanner_rejects_known_unsafe_shapes() {
    let unsafe_samples = [
        "token: xoxb-0000-0000-placeholder",
        "Authorization: Bearer abc",
        "https://discord.com/api/webhooks/123456/secret",
        "https://api.telegram.org/bot123456789:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/sendMessage",
        "123456789:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "call +14155550100",
        "red \u{1b}[31mtext",
        "body: \"\\u001b[2J\"",
        "\u{7}bell",
    ];
    for sample in unsafe_samples {
        assert!(unsafe_content(sample).is_some(), "scanner missed {sample:?}");
    }
    let safe_samples = [
        "POST https://discord.com/api/v10/webhooks/{webhook_id}/{webhook_token}?wait=true",
        "{\"member\":{\"user\":{\"id\":\"USER_ID\"}}}",
        "retry after 35: see https://docs.example.com/limits",
        "min: \"0.17\"",
    ];
    for sample in safe_samples {
        assert_eq!(unsafe_content(sample), None, "false positive on {sample:?}");
    }
}

/// Criterion 14: fixtures and shipped contract files hold no credentials,
/// recipient identifiers, or control sequences, and captured provider text
/// stays bounded.
#[test]
fn research_corpus_is_sanitized() {
    const MAX_PAYLOAD_BYTES: usize = 1024;
    let root = fixtures_dir();
    let research = messenger_dir().join("docs/research/platforms");
    let mut paths = vec![messenger_dir().join("docs/platforms.yaml")];
    for name in ["_schema.yaml", "_types.yaml", "_overrides.schema.yaml", "_rules.md", "_fleet.md"] {
        paths.push(research.join(name));
    }
    for dir in ["contract", "interaction", "diagnostics", "lifecycle/fleet", "negative/schema", "negative/semantic"] {
        paths.extend(files_in(&root.join(dir)));
    }
    for path in &paths {
        let text = fs::read_to_string(path).expect("utf-8 fixture");
        if let Some(reason) = unsafe_content(&text) {
            panic!("{}: {reason}", path.display());
        }
    }

    for dir in ["contract", "interaction", "diagnostics"] {
        for path in files_in(&root.join(dir)) {
            let markdown = load(&path);
            for (key, field) in [("error_fixtures", "body"), ("interaction_fixtures", "payload")] {
                let Ok(Some(Value::Array(records))) = markdown.frontmatter().get::<Value>(key) else {
                    continue;
                };
                for record in records {
                    let text = record[field].as_str().unwrap_or_default();
                    assert!(
                        text.len() <= MAX_PAYLOAD_BYTES,
                        "{} {}: {field} is {} bytes",
                        path.display(),
                        record["id"],
                        text.len()
                    );
                }
            }
        }
    }
}

// ---- typed loading and semantic validation (feature `research`) ----------

/// The same corpus through `messenger::research`: the typed loader (which
/// runs the Darkmatter schema pass itself) and the Rust-owned semantic
/// rules. Fixture headers drive expectations:
/// - `# expect-rule: SR-*` — every finding carries that rule;
/// - `# validate-scope: accepted` — clean as a fragment, rejected only when
///   judged as accepted research (full roster coverage, investigated gaps);
/// - `# expect-ineligible: <fact> <reason>` — valid, but the fact never
///   reaches the executable projection.
#[cfg(feature = "research")]
mod typed {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use darkmatter::markdown::Markdown;
    use messenger::research::canonical::schema_fingerprint;
    use messenger::research::generate::{check, generate};
    use messenger::research::model::{Date, Mappings, Overrides, PlatformDocument, PlatformId, Roster};
    use messenger::research::paths::CATALOG;
    use messenger::research::publish::Options;
    use messenger::research::{
        Context, Diagnostic, DocumentValidation, Loaded, Loader, RepoPath, Rule, Scope, ValidatedDocument,
        Workspace, validate_document, validate_mappings, validate_overrides, validate_roster,
    };

    use super::{files_in, fixtures_dir, header_value, messenger_dir};

    pub(super) fn repo_root() -> std::path::PathBuf {
        messenger_dir().parent().expect("repository root").to_path_buf()
    }

    pub(super) fn loader() -> &'static Loader {
        static LOADER: OnceLock<Loader> = OnceLock::new();
        LOADER.get_or_init(|| Loader::new(Workspace::new(repo_root()).expect("absolute root")))
    }

    pub(super) fn shipped_roster() -> &'static Roster {
        static ROSTER: OnceLock<Roster> = OnceLock::new();
        ROSTER.get_or_init(|| {
            let loaded = loader().load_roster(&messenger_dir().join("docs/platforms.yaml")).expect("roster");
            assert!(loaded.is_clean(), "shipped roster: {:?}", loaded.diagnostics);
            loaded.record.expect("typed roster")
        })
    }

    pub(super) fn today() -> Date {
        Date::parse("2026-09-17").expect("date")
    }

    pub(super) fn current_schema_fingerprint() -> String {
        let read = |name: &str| {
            std::fs::read_to_string(messenger_dir().join("docs/research/platforms").join(name)).expect("schema")
        };
        schema_fingerprint(&read("_schema.yaml"), &read("_types.yaml"))
    }

    pub(super) fn load_document(path: &Path) -> Loaded<PlatformDocument> {
        loader().load_document(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    pub(super) fn validate(path: &Path, scope: Scope) -> DocumentValidation {
        let context = Context { roster: Some(shipped_roster()), scope };
        validate_document(&load_document(path), &context)
    }

    pub(super) fn validated(name: &str) -> ValidatedDocument {
        let result = validate(&fixtures_dir().join(name), Scope::Fragment);
        assert!(result.diagnostics.is_empty(), "{name}: {}", show(&result.diagnostics));
        result.validated.expect("validated")
    }

    /// Companion documents for cross-file fixtures: the facts named by the
    /// override and mapping fixtures live in these contract documents.
    pub(super) fn companions() -> &'static [ValidatedDocument] {
        static COMPANIONS: OnceLock<Vec<ValidatedDocument>> = OnceLock::new();
        COMPANIONS.get_or_init(|| {
            vec![validated("contract/constraints-field.md"), validated("contract/bindings-mode-switch.md")]
        })
    }

    pub(super) fn show(diagnostics: &[Diagnostic]) -> String {
        diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n  ")
    }

    fn rules(diagnostics: &[Diagnostic]) -> BTreeSet<Rule> {
        diagnostics.iter().map(|d| d.rule).collect()
    }

    fn assert_clean_documents(dir: &str, min: usize) {
        let files: Vec<_> = files_in(&fixtures_dir().join(dir))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
        assert!(files.len() >= min, "{dir} shrank to {}", files.len());
        for path in files {
            let result = validate(&path, Scope::Fragment);
            assert!(result.diagnostics.is_empty(), "{}:\n  {}", path.display(), show(&result.diagnostics));
            assert!(result.validated.is_some(), "{} did not validate", path.display());
        }
    }

    /// The fleet fixture is complete accepted research, not a fragment.
    #[test]
    fn fleet_documents_pass_the_semantic_rules_as_accepted_research() {
        let files = files_in(&fixtures_dir().join("lifecycle/fleet"));
        assert_eq!(files.len(), 5);
        for path in files {
            let result = validate(&path, Scope::Accepted);
            assert!(result.diagnostics.is_empty(), "{}:\n  {}", path.display(), show(&result.diagnostics));
        }
    }

    #[test]
    fn contract_documents_pass_the_semantic_rules() {
        assert_clean_documents("contract", 38);
    }

    #[test]
    fn interaction_documents_pass_the_semantic_rules() {
        assert_clean_documents("interaction", 12);
    }

    #[test]
    fn diagnostic_documents_pass_the_semantic_rules() {
        assert_clean_documents("diagnostics", 6);
    }

    #[test]
    fn shipped_roster_is_a_complete_accepted_roster() {
        let loaded = loader().load_roster(&messenger_dir().join("docs/platforms.yaml")).expect("roster");
        let diagnostics = validate_roster(&loaded, Scope::Accepted);
        assert!(diagnostics.is_empty(), "{}", show(&diagnostics));
    }

    #[test]
    fn positive_roster_overrides_and_mappings_fixtures_are_clean() {
        let contract = fixtures_dir().join("contract");
        for name in ["roster-cap-10.yaml", "roster-refresh-interval-max.yaml"] {
            let roster = loader().load_roster(&contract.join(name)).expect("roster");
            let diagnostics = validate_roster(&roster, Scope::Fragment);
            assert!(diagnostics.is_empty(), "{name}: {}", show(&diagnostics));
        }

        let companions = companions();
        let documents: Vec<&ValidatedDocument> = companions.iter().collect();
        let overrides = loader().load_overrides(&contract.join("overrides-valid.yaml")).expect("overrides");
        let diagnostics = validate_overrides(&overrides, &documents, &current_schema_fingerprint(), &today());
        assert!(diagnostics.is_empty(), "overrides-valid: {}", show(&diagnostics));

        let mappings = loader().load_mappings(&contract.join("mappings-valid.yaml")).expect("mappings");
        let diagnostics = validate_mappings(&mappings, &documents);
        assert!(diagnostics.is_empty(), "mappings-valid: {}", show(&diagnostics));
    }

    /// Validates one semantic-negative fixture by its kind and returns the
    /// findings, checking the scope and eligibility headers along the way.
    fn semantic_findings(path: &Path) -> Vec<Diagnostic> {
        let name = path.file_name().and_then(|n| n.to_str()).expect("name").to_string();
        let schema = std::fs::read_to_string(path).expect("read");
        let companions = companions();
        let documents: Vec<&ValidatedDocument> = companions.iter().collect();
        if schema.contains("platforms.schema.yaml") {
            let loaded: Loaded<Roster> = loader().load(path).expect("roster");
            return validate_roster(&loaded, Scope::Fragment);
        }
        if schema.contains("_overrides.schema.yaml") {
            let loaded: Loaded<Overrides> = loader().load(path).expect("overrides");
            return validate_overrides(&loaded, &documents, &current_schema_fingerprint(), &today());
        }
        if schema.contains("implementation/_schema.yaml") {
            let loaded: Loaded<Mappings> = loader().load(path).expect("mappings");
            return validate_mappings(&loaded, &documents);
        }
        let fragment = validate(path, Scope::Fragment);
        if let Some(ineligible) = header_value(path, "expect-ineligible") {
            let (fact, reason) = ineligible.split_once(' ').expect("fact and reason");
            assert!(fragment.diagnostics.is_empty(), "{name} must validate:\n  {}", show(&fragment.diagnostics));
            let eligibility = fragment.validated.expect("validated").eligibility();
            let entry = eligibility.iter().find(|e| e.id() == fact).unwrap_or_else(|| panic!("{name}: no {fact}"));
            assert!(entry.executable().is_none(), "{name}: {fact} reached the executable projection");
            assert!(
                entry.reasons().iter().any(|r| r.code() == reason),
                "{name}: {fact} is ineligible for {:?}, not {reason}",
                entry.reasons()
            );
            return Vec::new();
        }
        if header_value(path, "validate-scope").as_deref() == Some("accepted") {
            assert!(
                fragment.diagnostics.is_empty(),
                "{name} must be clean as a fragment:\n  {}",
                show(&fragment.diagnostics)
            );
            return validate(path, Scope::Accepted).diagnostics;
        }
        fragment.diagnostics
    }

    fn assert_semantic_rules(prefixes: &[&str], min: usize) {
        let mut checked = 0;
        for path in files_in(&fixtures_dir().join("negative/semantic")) {
            let stem = path.file_name().and_then(|n| n.to_str()).expect("name").to_string();
            if !prefixes.iter().any(|p| stem.starts_with(&format!("{p}--"))) {
                continue;
            }
            checked += 1;
            let expected = header_value(&path, "expect-rule").expect("expect-rule");
            let diagnostics = semantic_findings(&path);
            if header_value(&path, "expect-ineligible").is_some() {
                continue;
            }
            assert!(!diagnostics.is_empty(), "{stem} produced no finding");
            let found = rules(&diagnostics);
            let expected_rule = Rule::from_code(&expected).unwrap_or_else(|| panic!("{expected} is not a rule"));
            let accepted = header_value(&path, "validate-scope").as_deref() == Some("accepted");
            if accepted {
                assert!(found.contains(&expected_rule), "{stem}: expected {expected}, got\n  {}", show(&diagnostics));
            } else {
                assert_eq!(
                    found,
                    BTreeSet::from([expected_rule]),
                    "{stem}: expected only {expected}, got\n  {}",
                    show(&diagnostics)
                );
            }
            for diagnostic in &diagnostics {
                assert!(
                    !diagnostic.path.as_str().contains('\\') && !diagnostic.path.as_str().starts_with('/'),
                    "{stem}: non-portable path {}",
                    diagnostic.path
                );
            }
        }
        assert!(checked >= min, "{prefixes:?}: only {checked} fixtures");
    }

    #[test]
    fn identity_rules_reject_their_fixtures() {
        assert_semantic_rules(&["sr-roster", "sr-curated", "sr-unique", "sr-ref", "sr-evidence", "sr-strict-scalars"], 17);
    }

    #[test]
    fn constraint_rules_reject_their_fixtures() {
        assert_semantic_rules(
            &["sr-state-value", "sr-condition", "sr-applicability", "sr-aggregate", "sr-kind-unit", "sr-enforceable"],
            12,
        );
    }

    #[test]
    fn coverage_rules_reject_their_fixtures() {
        assert_semantic_rules(&["sr-coverage", "sr-gap", "sr-change", "sr-override", "sr-mapping"], 14);
    }

    #[test]
    fn binding_rules_reject_their_fixtures() {
        assert_semantic_rules(&["sr-format", "sr-image", "sr-attribution", "sr-location", "sr-expression"], 14);
    }

    #[test]
    fn interaction_rules_reject_their_fixtures() {
        assert_semantic_rules(&["sr-interactivity"], 9);
    }

    #[test]
    fn error_rules_reject_their_fixtures() {
        assert_semantic_rules(&["sr-envelope", "sr-match", "sr-match-overlap", "sr-origin-phase", "sr-fixtures"], 8);
    }

    /// The typed loader and every rule family are as passive as schema
    /// validation: no effect engine, no network attempt, no file mutation.
    #[test]
    fn typed_loading_and_validation_are_passive() {
        let root = fixtures_dir();
        let mut documents = Vec::new();
        for name in ["composition-allowlist.md", "pilot-discord.md", "pilot-telegram.md", "pilot-slack.md", "pilot-signal.md"] {
            documents.push(root.join("contract").join(name));
        }
        documents.extend(files_in(&root.join("diagnostics")));
        documents.extend(files_in(&root.join("interaction")).into_iter().take(4));
        let others = [
            messenger_dir().join("docs/platforms.yaml"),
            root.join("contract/overrides-valid.yaml"),
            root.join("contract/mappings-valid.yaml"),
        ];
        let all: Vec<_> = documents.iter().chain(others.iter()).cloned().collect();
        let before: Vec<Vec<u8>> = all.iter().map(|p| std::fs::read(p).expect("read")).collect();
        let engines = darkmatter::effects::engine_build_count();
        let network = darkmatter::effects::network_attempt_count();

        for path in &documents {
            let result = validate(path, Scope::Fragment);
            assert!(result.diagnostics.is_empty(), "{}", show(&result.diagnostics));
            let _ = validate(path, Scope::Accepted);
        }
        let _ = validate_roster(&loader().load_roster(&others[0]).expect("roster"), Scope::Accepted);
        let companions = companions();
        let refs: Vec<&ValidatedDocument> = companions.iter().collect();
        let overrides = loader().load_overrides(&others[1]).expect("overrides");
        let _ = validate_overrides(&overrides, &refs, &current_schema_fingerprint(), &today());
        let mappings = loader().load_mappings(&others[2]).expect("mappings");
        let _ = validate_mappings(&mappings, &refs);

        assert_eq!(darkmatter::effects::engine_build_count(), engines, "research validation built an effect engine");
        assert_eq!(darkmatter::effects::network_attempt_count(), network, "research validation attempted network access");
        for (path, bytes) in all.iter().zip(before) {
            assert_eq!(std::fs::read(path).expect("reread"), bytes, "{} changed", path.display());
        }
    }

    /// Adapter IDs are persisted identifiers (receipts, routes): the typed
    /// vocabulary must be exactly the chat `ProviderKind::as_str()` values.
    #[test]
    fn adapter_ids_are_the_persisted_provider_kind_spellings() {
        use messenger::ProviderKind;
        use messenger::research::model::AdapterId;
        let chat = [
            ProviderKind::Discord,
            ProviderKind::DiscordWebhook,
            ProviderKind::Slack,
            ProviderKind::SlackWebhook,
            ProviderKind::Telegram,
            ProviderKind::WhatsApp,
            ProviderKind::Signal,
        ];
        let typed: Vec<&str> = AdapterId::ALL.iter().map(|a| a.as_str()).collect();
        let persisted: Vec<&str> = chat.into_iter().map(ProviderKind::as_str).collect();
        assert_eq!(typed, persisted);
    }

    /// Every semantic fixture belongs to exactly one rule-family test above.
    #[test]
    fn every_semantic_fixture_is_exercised_by_a_rule_family() {
        let families = [
            "sr-roster", "sr-curated", "sr-unique", "sr-ref", "sr-evidence", "sr-strict-scalars", "sr-state-value",
            "sr-condition", "sr-applicability", "sr-aggregate", "sr-kind-unit", "sr-enforceable", "sr-coverage",
            "sr-gap", "sr-change", "sr-override", "sr-mapping", "sr-format", "sr-image", "sr-attribution",
            "sr-location", "sr-expression", "sr-interactivity", "sr-envelope", "sr-match", "sr-match-overlap",
            "sr-origin-phase", "sr-fixtures",
        ];
        for path in files_in(&fixtures_dir().join("negative/semantic")) {
            let stem = path.file_name().and_then(|n| n.to_str()).expect("name");
            let (prefix, _) = stem.split_once("--").expect("rule prefix");
            assert!(families.contains(&prefix), "{stem} is not covered by a rule-family test");
        }
    }

    // ---- the published baseline ------------------------------------------

    /// Why the research tree under `root` is not an accepted, drift-free
    /// published baseline; `None` before its first publication, when there is
    /// nothing to hold to accepted scope yet.
    ///
    /// Once published, every active roster document must declare the schema
    /// (legacy prose no longer qualifies) and pass the Accepted-scope rules,
    /// and `generate --check` must find neither drift nor a hand edit.
    fn published_baseline_findings(root: &Path) -> Option<Vec<String>> {
        let loader = Loader::new(Workspace::new(root).expect("absolute root"));
        let workspace = loader.workspace();
        if !workspace.manifest().exists() {
            return None;
        }
        let mut findings = Vec::new();
        let roster = loader.load_roster(&workspace.roster()).expect("roster");
        let Some(record) = roster.record.as_ref().filter(|_| roster.is_clean()) else {
            return Some(vec![format!("roster: {}", show(&roster.diagnostics))]);
        };
        for platform in record.active_platforms() {
            let id = platform.platform_id;
            let path = workspace.document(id);
            let declared = Markdown::try_from(path.as_path())
                .ok()
                .and_then(|markdown| markdown.frontmatter().get::<String>("$schema").ok().flatten());
            if declared.is_none() {
                findings.push(format!("{id}: legacy prose (no $schema) after publication"));
                continue;
            }
            let loaded = match loader.load_document(&path) {
                Ok(loaded) => loaded,
                Err(error) => {
                    findings.push(format!("{id}: {error}"));
                    continue;
                }
            };
            let result = validate_document(&loaded, &Context { roster: Some(record), scope: Scope::Accepted });
            findings.extend(result.diagnostics.iter().map(|d| format!("{id}: {d}")));
        }
        match check(&loader, &today()) {
            Ok(None) => {}
            Ok(Some(drift)) => findings.push(format!("generate --check: {drift:?}")),
            Err(error) => findings.push(format!("generate --check: {error}")),
        }
        Some(findings)
    }

    /// Vacuous until the accepted baseline is published; see
    /// `published_baseline_findings` and the fixture-tree tests below for
    /// the post-publication branch.
    #[test]
    fn shipped_publication_is_accepted_and_drift_free() {
        let Some(findings) = published_baseline_findings(&repo_root()) else {
            let catalog = loader().workspace().resolve(&RepoPath::from_portable(CATALOG));
            assert!(!catalog.exists(), "{CATALOG} exists without a publication manifest");
            eprintln!("VACUOUS: no publication.json yet; the accepted-baseline guard activates on first publication");
            return;
        };
        assert!(findings.is_empty(), "published baseline:\n  {}", findings.join("\n  "));
    }

    /// A temporary repository holding the shipped schemas, roster, and fleet
    /// prompt, with the accepted-fleet fixture published through `generate`.
    /// Overrides and mappings are omitted: they name facts in the shipped
    /// documents, which the fixture replaces.
    fn published_fixture_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = Workspace::new(dir.path()).expect("absolute root");
        let source = loader().workspace();
        let inputs: [fn(&Workspace) -> PathBuf; 6] = [
            Workspace::roster,
            Workspace::roster_schema,
            Workspace::document_schema,
            Workspace::types_schema,
            Workspace::overrides_schema,
            Workspace::fleet_prompt,
        ];
        for input in inputs {
            let to = input(&target);
            std::fs::create_dir_all(to.parent().expect("parent")).expect("mkdir");
            std::fs::copy(input(source), to).expect("copy shipped input");
        }
        for platform in PlatformId::ALL {
            let fixture = fixtures_dir().join(format!("lifecycle/fleet/{platform}.md"));
            let text = std::fs::read_to_string(fixture)
                .expect("fleet fixture")
                .replace("$schema: ../../../../../../docs/research/platforms/_schema.yaml", "$schema: ./_schema.yaml");
            std::fs::write(target.document(*platform), text).expect("write document");
        }
        generate(&Loader::new(target), &BTreeMap::new(), &today(), Options::default()).expect("fixture publishes");
        dir
    }

    fn findings_mentioning(root: &Path, needle: &str) -> Vec<String> {
        let findings = published_baseline_findings(root).expect("the fixture is published");
        assert!(!findings.is_empty(), "the guard accepted a defective baseline");
        findings.into_iter().filter(|f| f.contains(needle)).collect()
    }

    #[test]
    fn published_baseline_guard_accepts_a_clean_published_fleet() {
        let tree = published_fixture_tree();
        let findings = published_baseline_findings(tree.path()).expect("the fixture is published");
        assert!(findings.is_empty(), "{}", findings.join("\n  "));
    }

    #[test]
    fn published_baseline_guard_rejects_legacy_prose() {
        let tree = published_fixture_tree();
        let legacy = "---\nprompt: |-\n    Refresh the Discord research document (`platform_id: discord`) per\n    \
                      `messenger/docs/research/platforms/_fleet.md`, Pass 2.\n---\n# Discord\n";
        std::fs::write(Workspace::new(tree.path()).expect("root").document(PlatformId::Discord), legacy).expect("write");
        let legacy_findings = findings_mentioning(tree.path(), "legacy prose");
        assert_eq!(legacy_findings, vec!["discord: legacy prose (no $schema) after publication".to_string()]);
    }

    #[test]
    fn published_baseline_guard_rejects_drifted_generated_artifacts() {
        let tree = published_fixture_tree();
        let catalog = Workspace::new(tree.path()).expect("root").resolve(&RepoPath::from_portable(CATALOG));
        let original = std::fs::read_to_string(&catalog).expect("catalog");

        // Stale: an input changed after publication, so regeneration differs.
        let roster = tree.path().join("messenger/docs/platforms.yaml");
        let text = std::fs::read_to_string(&roster).expect("roster");
        assert!(text.contains("refresh_interval_days: 30"), "edit target not found");
        std::fs::write(&roster, text.replacen("refresh_interval_days: 30", "refresh_interval_days: 45", 1)).expect("write");
        let stale = findings_mentioning(tree.path(), "generate --check");
        assert!(stale.iter().any(|f| f.contains("Changed") && f.contains(CATALOG)), "{stale:?}");
        std::fs::write(&roster, text).expect("restore roster");
        assert_eq!(published_baseline_findings(tree.path()), Some(Vec::new()));

        // Hand-edited: the published catalog no longer matches its manifest.
        std::fs::write(&catalog, original.replace("\"discord\"", "\"Discord\"")).expect("write");
        let edited = findings_mentioning(tree.path(), "generate --check");
        assert!(edited.iter().any(|f| f.contains(CATALOG) && f.contains("does not match")), "{edited:?}");
    }
}
