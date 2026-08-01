//! `claudine signals` — inspect the compiled signal-detection tables, and
//! replay the research corpus's evidence fixtures through the production
//! engine (`signals check`).
//!
//! Check posture is dev/CI: it reads `docs/research/signals/` from a
//! rusty-biscuit checkout (mirroring `claudine-gen`'s area resolution) and
//! fails when the corpus and the committed tables drift, when evidence is
//! missing (the existence check the sidecar schema defers here), when a
//! declarative record does not fire on its own evidence, when a fired
//! record's extractions do not resolve, or when a fixture fires records
//! that neither cite it as evidence nor appear in the declared exclusions
//! file (`_overlap-exclusions.yaml`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string as portable;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Edges, Length};
use clap::{Args, Subcommand};
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde::Serialize;
use serde_json::Value;

use claudine::signals::{
    DetectionMode, DetectionRecord, ExtractStrategy, ProviderSignalTable, SignalEngine,
    all_detection_tables,
};
use claudine::stream::logs::opencode::{ParsedOpenCodeStderrLine, classify, classify_raw, parse_line};

use crate::log;

/// Declared benign fixture↔record overlaps, sibling of the `_schema.yaml`
/// sidecar (NOT under `fixtures/` — the provenance bijection test rejects
/// non-fixture files there).
const OVERLAP_EXCLUSIONS_FILE: &str = "_overlap-exclusions.yaml";

/// Arguments accepted by the `claudine signals` command.
#[derive(Debug, Args)]
pub struct SignalsArgs {
    #[command(subcommand)]
    pub command: Option<SignalsCommand>,
}

/// Subcommands under `claudine signals`.
#[derive(Debug, Subcommand)]
pub enum SignalsCommand {
    /// Replay every record's evidence fixture through the production
    /// detection engine (requires a rusty-biscuit checkout).
    Check(CheckArgs),
}

/// Arguments accepted by `claudine signals check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Check a single provider slug (the dormant kilo/pi tables are valid
    /// targets too).
    #[arg(long)]
    pub provider: Option<String>,

    /// Emit the machine-readable check report as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Show the table summary or dispatch to `check`.
pub fn run(args: SignalsArgs) -> Result<()> {
    match args.command {
        None => run_summary(),
        Some(SignalsCommand::Check(check)) => run_check(check),
    }
}

/// Compact per-provider summary of the compiled tables. Purely static —
/// works outside a checkout, unlike `check`.
fn run_summary() -> Result<()> {
    let term = crate::log::terminal();
    let columns = vec![
        TableColumn::new("Provider"),
        TableColumn::new("Records").with_alignment(Alignment::Center),
        TableColumn::new("Extractions").with_alignment(Alignment::Center),
        TableColumn::new("Bespoke").with_alignment(Alignment::Center),
        TableColumn::new("Sources"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().margin = Edges::x(Length::ch(1));

    for provider in all_detection_tables() {
        let extractions: usize = provider
            .records
            .iter()
            .map(|record| record.extractions.len())
            .sum();
        let bespoke = provider
            .records
            .iter()
            .filter(|record| record.mode == DetectionMode::Bespoke)
            .count();
        // Per-source breakdown in source declaration order.
        let mut sources: Vec<(&'static str, usize)> = Vec::new();
        for record in provider.records {
            let name: &'static str = record.source.into();
            match sources.iter_mut().find(|(existing, _)| *existing == name) {
                Some((_, count)) => *count += 1,
                None => sources.push((name, 1)),
            }
        }
        let sources = sources
            .iter()
            .map(|(name, count)| format!("{name} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        table.add_row(vec![
            provider.slug.into(),
            provider.records.len().to_string().into(),
            extractions.to_string().into(),
            bespoke.to_string().into(),
            sources.into(),
        ]);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{rendered}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------

/// Per-provider check outcome (also the `--json` payload shape).
#[derive(Debug, Default, Serialize)]
struct ProviderReport {
    provider: String,
    records: usize,
    positives_passed: usize,
    negatives_passed: usize,
    exclusions_applied: Vec<String>,
    failures: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FleetReport {
    providers: Vec<ProviderReport>,
    unused_exclusions: Vec<String>,
    records: usize,
    positives_passed: usize,
    negatives_passed: usize,
    exclusions_applied: usize,
    failures: usize,
}

/// One declared benign overlap: `record` may fire on `fixture` (a fixture
/// that is not its evidence) without failing the check.
#[derive(Debug)]
struct OverlapExclusion {
    /// `<slug>/<file>` relative to the fixtures directory.
    fixture: String,
    record: String,
    reason: String,
}

fn run_check(args: CheckArgs) -> Result<()> {
    let area = resolve_area()?;
    let signals_dir = area.join("docs/research/signals");
    let fixtures_dir = signals_dir.join("fixtures");
    let exclusions = load_exclusions(&signals_dir.join(OVERLAP_EXCLUSIONS_FILE))?;

    let tables: Vec<&'static ProviderSignalTable> = match &args.provider {
        Some(slug) => {
            let table = all_detection_tables()
                .iter()
                .copied()
                .find(|table| table.slug == slug.as_str())
                .ok_or_else(|| {
                    let known = all_detection_tables()
                        .iter()
                        .map(|table| table.slug)
                        .collect::<Vec<_>>()
                        .join(", ");
                    eyre!("unknown provider `{slug}` — expected one of: {known}")
                })?;
            vec![table]
        }
        None => all_detection_tables().to_vec(),
    };

    log::message(&format!(
        "signals check — replaying evidence fixtures from {}",
        portable(&signals_dir)
    ));

    let mut used_exclusions: BTreeSet<usize> = BTreeSet::new();
    let mut providers = Vec::with_capacity(tables.len());
    for table in tables {
        providers.push(check_provider(
            table,
            &signals_dir,
            &fixtures_dir,
            &exclusions,
            &mut used_exclusions,
        )?);
    }

    // A --provider subset legitimately leaves other providers' exclusions
    // untouched, so staleness is only meaningful on a full-fleet run.
    let unused_exclusions = if args.provider.is_none() {
        exclusions
            .iter()
            .enumerate()
            .filter(|(index, _)| !used_exclusions.contains(index))
            .map(|(_, exclusion)| format!("{} / {}", exclusion.fixture, exclusion.record))
            .collect()
    } else {
        Vec::new()
    };

    let fleet = FleetReport {
        records: providers.iter().map(|p| p.records).sum(),
        positives_passed: providers.iter().map(|p| p.positives_passed).sum(),
        negatives_passed: providers.iter().map(|p| p.negatives_passed).sum(),
        exclusions_applied: providers.iter().map(|p| p.exclusions_applied.len()).sum(),
        failures: providers.iter().map(|p| p.failures.len()).sum(),
        unused_exclusions,
        providers,
    };

    if args.json {
        log::data(&serde_json::to_string_pretty(&fleet)?);
    } else {
        render_fleet(&fleet);
    }

    if fleet.failures > 0 {
        bail!("signals check failed with {} failure(s)", fleet.failures);
    }
    // Every compiled record is a bespoke or declarative record that must
    // positively replay on its own evidence (there is no skip path anymore):
    // "verified == shipped" is mechanical. A subset run (`--provider`) is
    // exempt because `records` then counts only the selected provider's rows,
    // which `positives_passed` already matches when clean.
    if fleet.positives_passed != fleet.records {
        bail!(
            "signals check: {} of {} records positively replayed — every compiled record must \
             fire on its own evidence",
            fleet.positives_passed,
            fleet.records,
        );
    }
    Ok(())
}

fn render_fleet(fleet: &FleetReport) {
    for provider in &fleet.providers {
        log::data(&format!(
            "{} — {} records: positives {}, negatives {}, exclusions {}",
            provider.provider,
            provider.records,
            provider.positives_passed,
            provider.negatives_passed,
            provider.exclusions_applied.len(),
        ));
        for info in &provider.exclusions_applied {
            log::data(&format!("  [INFO] exclusion: {info}"));
        }
        for failure in &provider.failures {
            log::data(&format!("  [FAIL] {failure}"));
        }
    }
    for unused in &fleet.unused_exclusions {
        log::data(&format!("[INFO] unused exclusion: {unused}"));
    }
    log::data(&format!(
        "fleet: {} records checked — positives {} passed, negatives {} passed, exclusions {} applied, failures {}",
        fleet.records,
        fleet.positives_passed,
        fleet.negatives_passed,
        fleet.exclusions_applied,
        fleet.failures,
    ));
}

fn check_provider(
    table: &'static ProviderSignalTable,
    signals_dir: &Path,
    fixtures_dir: &Path,
    exclusions: &[OverlapExclusion],
    used_exclusions: &mut BTreeSet<usize>,
) -> Result<ProviderReport> {
    let mut report = ProviderReport {
        provider: table.slug.to_string(),
        records: table.records.len(),
        ..ProviderReport::default()
    };

    // Doc ↔ compiled-table join by record id. The gen drift test should
    // make a mismatch impossible; check defends anyway because everything
    // downstream keys off the join.
    let doc_records = load_doc_evidence(signals_dir, table.slug)?;
    let table_ids: BTreeSet<&str> = table.records.iter().map(|record| record.id).collect();
    for id in doc_records.keys() {
        if !table_ids.contains(id.as_str()) {
            report.failures.push(format!(
                "record `{id}` exists in {}.md but not in the compiled table (regenerate generated.rs)",
                table.slug
            ));
        }
    }
    for record in table.records {
        if !doc_records.contains_key(record.id) {
            report.failures.push(format!(
                "record `{}` exists in the compiled table but not in {}.md (regenerate generated.rs)",
                record.id, table.slug
            ));
        }
    }

    // Fixture payload cache; a fixture that fails to load is reported once.
    let mut payload_cache: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut broken_fixtures: BTreeSet<String> = BTreeSet::new();
    let mut payloads_for = |key: &str, failures: &mut Vec<String>| -> Option<Vec<Value>> {
        if broken_fixtures.contains(key) {
            return None;
        }
        if let Some(payloads) = payload_cache.get(key) {
            return Some(payloads.clone());
        }
        match load_fixture_payloads(fixtures_dir, key, table.slug) {
            Ok(payloads) => {
                payload_cache.insert(key.to_string(), payloads.clone());
                Some(payloads)
            }
            Err(err) => {
                broken_fixtures.insert(key.to_string());
                failures.push(format!("fixture {key}: {err:#}"));
                None
            }
        }
    };

    // Evidence existence + positive replay per record.
    for record in table.records {
        let Some(evidence) = doc_records.get(record.id) else {
            continue; // drift already reported
        };
        if !fixtures_dir.join(evidence).is_file() {
            report.failures.push(format!(
                "record `{}`: evidence fixture {evidence} does not exist",
                record.id
            ));
            continue;
        }
        match record.mode {
            DetectionMode::Bespoke => {
                match bespoke_replayer(record.id) {
                    Some(replay) => {
                        let Some(payloads) = payloads_for(evidence, &mut report.failures) else {
                            continue;
                        };
                        if replay(&payloads) {
                            report.positives_passed += 1;
                        } else {
                            report.failures.push(format!(
                                "record `{}`: bespoke replayer did not detect its evidence {evidence}",
                                record.id
                            ));
                        }
                    }
                    // A bespoke record with no runtime detector wired is a
                    // regression, not a benign skip: it claims a hand-written
                    // emitter that does not exist.
                    None => report.failures.push(format!(
                        "record `{}`: bespoke record has no registered replayer — \
                         wire a runtime detector/replayer",
                        record.id
                    )),
                }
                continue;
            }
            DetectionMode::Declarative => {}
        }

        let Some(payloads) = payloads_for(evidence, &mut report.failures) else {
            continue;
        };
        match replay_positive(table, record, &payloads) {
            PositiveOutcome::Passed => report.positives_passed += 1,
            PositiveOutcome::NeverFired => report.failures.push(format!(
                "record `{}`: never fired on its evidence {evidence}",
                record.id
            )),
            PositiveOutcome::Unresolved(fields) => {
                let detail = fields
                    .iter()
                    .map(|(field, source)| format!("`{field}` (source `{source}`)"))
                    .collect::<Vec<_>>()
                    .join(", ");
                report.failures.push(format!(
                    "record `{}`: fired on {evidence} but extraction(s) did not resolve: {detail}",
                    record.id
                ));
            }
        }
    }

    // Negative overlap: every evidence fixture is replayed against its
    // owner's whole source group; anything else that fires must share the
    // evidence or be declared in the exclusions file.
    let mut owners: BTreeMap<&String, Vec<&'static DetectionRecord>> = BTreeMap::new();
    for record in table.records {
        if let Some(evidence) = doc_records.get(record.id) {
            owners.entry(evidence).or_default().push(record);
        }
    }
    for (fixture, owner_records) in owners {
        let owner_ids: BTreeSet<&str> = owner_records.iter().map(|record| record.id).collect();
        let Some(payloads) = payloads_for(fixture, &mut report.failures) else {
            continue;
        };
        // Unexpected fires dedup across owners: a fixture shared by two
        // owner records replays twice (their version narrowing differs)
        // but each foreign record is one finding.
        let mut unexpected: BTreeSet<&str> = BTreeSet::new();
        // Bespoke negative overlap: a bespoke detector firing on another
        // record's evidence fixture within the same provider × source
        // group is an overlap finding too. Runs outside the owner loop so
        // it never perturbs the `negatives_passed` per-owner tally.
        for record in table.records {
            if record.mode != DetectionMode::Bespoke || owner_ids.contains(record.id) {
                continue;
            }
            if !owner_records.iter().any(|owner| owner.source == record.source) {
                continue;
            }
            let Some(replay) = bespoke_replayer(record.id) else {
                continue;
            };
            if !replay(&payloads) {
                continue;
            }
            match exclusions
                .iter()
                .position(|exclusion| exclusion.fixture == *fixture && exclusion.record == record.id)
            {
                Some(index) => {
                    if used_exclusions.insert(index) {
                        report.exclusions_applied.push(format!(
                            "`{}` fires on {fixture} — {}",
                            record.id, exclusions[index].reason
                        ));
                    }
                }
                None => {
                    unexpected.insert(record.id);
                }
            }
        }
        for owner in owner_records {
            let engine = replay_engine(table, owner);
            let mut owner_clean = true;
            for payload in &payloads {
                for observation in engine.observe_detailed(owner.source, payload) {
                    let fired = observation.record.id;
                    if owner_ids.contains(fired) {
                        continue;
                    }
                    match exclusions.iter().position(|exclusion| {
                        exclusion.fixture == *fixture && exclusion.record == fired
                    }) {
                        Some(index) => {
                            if used_exclusions.insert(index) {
                                report.exclusions_applied.push(format!(
                                    "`{fired}` fires on {fixture} — {}",
                                    exclusions[index].reason
                                ));
                            }
                        }
                        None => {
                            owner_clean = false;
                            unexpected.insert(fired);
                        }
                    }
                }
            }
            if owner_clean {
                report.negatives_passed += 1;
            }
        }
        for fired in unexpected {
            report.failures.push(format!(
                "fixture {fixture}: record `{fired}` fires but the fixture is not its evidence \
                 (add an `{OVERLAP_EXCLUSIONS_FILE}` entry only if this is benign)"
            ));
        }
    }

    Ok(report)
}

enum PositiveOutcome {
    Passed,
    NeverFired,
    /// Fired somewhere, but no firing payload resolved every extraction —
    /// carries the smallest unresolved (field, path) set seen.
    Unresolved(Vec<(&'static str, String)>),
}

fn replay_positive(
    table: &'static ProviderSignalTable,
    record: &'static DetectionRecord,
    payloads: &[Value],
) -> PositiveOutcome {
    let engine = replay_engine(table, record);
    let mut best_unresolved: Option<Vec<(&'static str, String)>> = None;
    for payload in payloads {
        let observations = engine.observe_detailed(record.source, payload);
        let Some(observation) = observations
            .iter()
            .find(|observation| observation.record.id == record.id)
        else {
            continue;
        };
        let unresolved: Vec<(&'static str, String)> = record
            .extractions
            .iter()
            .filter(|spec| !observation.resolved_fields.contains(&spec.field))
            .map(|spec| (spec.field, describe_extract_source(&spec.source)))
            .collect();
        if unresolved.is_empty() {
            return PositiveOutcome::Passed;
        }
        if best_unresolved
            .as_ref()
            .is_none_or(|best| unresolved.len() < best.len())
        {
            best_unresolved = Some(unresolved);
        }
    }
    match best_unresolved {
        Some(fields) => PositiveOutcome::Unresolved(fields),
        None => PositiveOutcome::NeverFired,
    }
}

/// Human-readable one-line description of an extraction strategy for the
/// `signals check` unresolved-field diagnostic.
fn describe_extract_source(source: &ExtractStrategy) -> String {
    match source {
        ExtractStrategy::Path(path) => format!("path `{path}`"),
        ExtractStrategy::Literal(value) => format!("literal `{value}`"),
        ExtractStrategy::Regex { path, pattern } => format!("regex `{pattern}` on `{path}`"),
        ExtractStrategy::StartStopTokens { path, start, stop } => {
            format!("tokens `{start}`..`{stop}` on `{path}`")
        }
    }
}

/// Engine for replaying `record`'s fixtures. Version-scoped records replay
/// with a version their own inclusive bounds admit, so version twins (e.g.
/// the OpenCode legacy / 1.17.8 usage-cap pair) select exactly as
/// production does once the wrapper has observed the provider version;
/// unbounded records replay in union mode.
fn replay_engine(
    table: &'static ProviderSignalTable,
    record: &'static DetectionRecord,
) -> SignalEngine {
    let mut engine = SignalEngine::new(table);
    if let Some(version) = record.since.or(record.until) {
        engine.observe_provider_version(version);
    }
    engine
}

/// Bespoke fixture replayers keyed by record id (a replayer returns true
/// when the evidence payload sequence triggers its detector). Delegates to
/// the lib registry so replay runs the SAME detector code the runtime
/// [`claudine::signals::SignalHub`] chain executes.
fn bespoke_replayer(record_id: &str) -> Option<fn(&[Value]) -> bool> {
    claudine::signals::bespoke_replayer(record_id)
}

// ---------------------------------------------------------------------------
// corpus loading
// ---------------------------------------------------------------------------

/// Walk up from CWD to the claudine package area (the directory holding
/// `docs/providers.yaml`), also accepting an ancestor whose `claudine/`
/// child is the area — the same resolution `claudine-gen` uses.
fn resolve_area() -> Result<PathBuf> {
    let cwd = std::env::current_dir().wrap_err("cannot resolve the current directory")?;
    let mut dir = Some(cwd.as_path());
    while let Some(current) = dir {
        if current.join("docs/providers.yaml").is_file() {
            return Ok(current.to_path_buf());
        }
        let child = current.join("claudine");
        if child.join("docs/providers.yaml").is_file() {
            return Ok(child);
        }
        dir = current.parent();
    }
    bail!(
        "signals check requires a rusty-biscuit checkout — no `docs/providers.yaml` found \
         walking up from {}",
        portable(&cwd)
    )
}

/// Record id → evidence fixture key (`<slug>/<file>`, relative to the
/// fixtures directory) from one provider's sidecar-validated frontmatter.
fn load_doc_evidence(signals_dir: &Path, slug: &str) -> Result<BTreeMap<String, String>> {
    let path = signals_dir.join(format!("{slug}.md"));
    let frontmatter = load_validated_frontmatter(&path)?;
    let records = frontmatter
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("{}: frontmatter has no `records:` list", portable(&path)))?;
    let mut evidence_by_id = BTreeMap::new();
    for record in records {
        let id = record
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("{}: record without a string `id`", portable(&path)))?;
        let evidence = record
            .get("evidence")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                eyre!(
                    "{}: record `{id}` without a string `evidence`",
                    portable(&path)
                )
            })?;
        evidence_by_id.insert(id.to_string(), fixture_key(&path, id, evidence)?);
    }
    Ok(evidence_by_id)
}

/// Document-relative `./fixtures/<slug>/<file>` → `<slug>/<file>`.
fn fixture_key(doc: &Path, record: &str, evidence: &str) -> Result<String> {
    evidence
        .trim_start_matches("./")
        .strip_prefix("fixtures/")
        .map(str::to_string)
        .ok_or_else(|| {
            eyre!(
                "{}: record `{record}` evidence `{evidence}` is not under ./fixtures/",
                portable(doc)
            )
        })
}

/// Reads a signals research doc and validates its frontmatter against the
/// `$schema` sidecar — the CLI-side mirror of claudine-gen's
/// `load_validated_frontmatter` (this CLI never links the generator).
fn load_validated_frontmatter(path: &Path) -> Result<Value> {
    let md = Markdown::try_from(path)
        .map_err(|err| eyre!("failed to read {}: {err}", portable(path)))?;
    let schemas = DarkmatterSchemas::new();
    let effective = schemas
        .effective_for(&md)
        .map_err(|err| eyre!("{}: schema resolution failed: {err}", portable(path)))?
        .ok_or_else(|| {
            eyre!(
                "{}: research document declares no `$schema`",
                portable(path)
            )
        })?;
    let frontmatter = serde_json::to_value(md.frontmatter().as_map())
        .wrap_err_with(|| format!("{}: frontmatter is not JSON-shaped", portable(path)))?;
    let report = effective.validate(&frontmatter);
    if !report.valid {
        let problems = report
            .problems
            .iter()
            .map(|problem| format!("- {}", problem.message))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "{}: frontmatter does not satisfy the sidecar schema:\n{problems}",
            portable(path)
        );
    }
    Ok(frontmatter)
}

/// One fixture → the payload sequence the engine replays, per the corpus
/// payload definition: `.jsonl` is one JSON payload per non-blank line,
/// `.json` a single object (pi's wrapper-synthesized exit shape, goose/kilo
/// sqlite rows), and `.txt` (opencode `stderr_promoted`) runs each line
/// through the production promoted-stderr classifier and serializes the
/// resulting `LogClassification` (glue mode, ratified 2026-07-06).
fn load_fixture_payloads(fixtures_dir: &Path, key: &str, slug: &str) -> Result<Vec<Value>> {
    let path = fixtures_dir.join(key);
    let text = std::fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read {}", portable(&path)))?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    match extension {
        "jsonl" => text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(index, line)| {
                serde_json::from_str(line)
                    .wrap_err_with(|| format!("{}:{}: invalid JSON", portable(&path), index + 1))
            })
            .collect(),
        "json" => {
            Ok(vec![serde_json::from_str(&text).wrap_err_with(|| {
                format!("{}: invalid JSON", portable(&path))
            })?])
        }
        "txt" if slug == "opencode" => Ok(text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let classification = match parse_line(line) {
                    ParsedOpenCodeStderrLine::Structured(record) => classify(&record),
                    ParsedOpenCodeStderrLine::RawText(raw) => classify_raw(&raw),
                };
                classification.to_signal_payload()
            })
            .collect()),
        other => bail!(
            "{}: unsupported fixture extension `{other}` for provider `{slug}`",
            portable(&path)
        ),
    }
}

/// Loads `_overlap-exclusions.yaml`. An absent file is an empty list; a
/// present entry requires a non-empty reason.
fn load_exclusions(path: &Path) -> Result<Vec<OverlapExclusion>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let value = biscuit_file::Yaml::try_from(path)
        .map_err(|err| eyre!("failed to read {}: {err}", portable(path)))?
        .as_json()
        .map_err(|err| eyre!("{}: invalid YAML: {err}", portable(path)))?;
    let entries = value
        .get("exclusions")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            eyre!(
                "{}: expected a top-level `exclusions:` list",
                portable(path)
            )
        })?;
    let mut exclusions = Vec::with_capacity(entries.len());
    for entry in entries {
        let field = |key: &str| -> Result<String> {
            entry
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    eyre!(
                        "{}: exclusion entry missing a non-empty `{key}`",
                        portable(path)
                    )
                })
        };
        exclusions.push(OverlapExclusion {
            fixture: field("fixture")?,
            record: field("record")?,
            reason: field("reason")?,
        });
    }
    Ok(exclusions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_key_normalizes_document_relative_evidence() {
        let doc = Path::new("docs/research/signals/opencode.md");
        assert_eq!(
            fixture_key(doc, "r", "./fixtures/opencode/stream-start-1178.txt").unwrap(),
            "opencode/stream-start-1178.txt"
        );
        assert!(fixture_key(doc, "r", "./elsewhere/file.jsonl").is_err());
    }

    #[test]
    fn antigravity_exit_records_positively_replay_and_no_app_log_records_remain() {
        // Positive verification: both Antigravity `exit` auth records replay
        // clean on their own evidence (positives_passed == records, no
        // failures). The two former app-log records were moved to `gaps`, so
        // the compiled table must carry no `app_log`-source record.
        let area = resolve_area().expect("rusty-biscuit checkout");
        let signals_dir = area.join("docs/research/signals");
        let fixtures_dir = signals_dir.join("fixtures");
        let table = all_detection_tables()
            .iter()
            .copied()
            .find(|table| table.slug == "antigravity")
            .expect("antigravity table");
        let mut used = BTreeSet::new();
        let report = check_provider(table, &signals_dir, &fixtures_dir, &[], &mut used)
            .expect("check_provider succeeds");
        assert!(
            report.failures.is_empty(),
            "unexpected failures: {:?}",
            report.failures
        );
        assert_eq!(
            report.positives_passed, report.records,
            "every antigravity record must positively replay",
        );
        assert!(
            !table
                .records
                .iter()
                .any(|record| record.source == claudine::signals::SignalSource::AppLog),
            "no antigravity record may observe the app_log source (moved to gaps)",
        );
    }

    #[test]
    fn every_compiled_slug_is_checkable() {
        // The check loop iterates the compiled tables; kilo and pi (both now
        // wired providers) must stay present so their corpora remain replayable.
        let slugs: Vec<&str> = all_detection_tables()
            .iter()
            .map(|table| table.slug)
            .collect();
        for slug in ["kilo", "pi"] {
            assert!(slugs.contains(&slug), "{slug} table missing");
        }
    }
}
