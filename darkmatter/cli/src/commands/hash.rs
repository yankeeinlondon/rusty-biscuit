//! `md hash` subcommand implementation.

use crate::args::Cli;
use crate::io::{DocumentArgumentError, load_markdown, parse_argument, read_markdown_text};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_hash::xx_hash;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use clap::CommandFactory as _;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::output::command::{CommandValue, ComparisonData, HashData, HashMode, HashValueData, code_document};
use darkmatter::markdown::compose::expression::Severity;
use darkmatter::markdown::hash::{
    Change, ComputedHash, DEFAULT_HASH_PROPERTY, LAST_UPDATED_KEY, MdHashKind, MdHashOptions, StoredHash,
    select_kind,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::fs::{ReferenceEntry, collect_markdown_files, resolve_entry_in_context};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// The flags that select what `md hash` computes.
#[derive(Debug, Clone, Copy)]
pub struct HashFlags {
    /// `--kind`.
    pub kind: Option<MdHashKind>,
    /// `--body`.
    pub body: bool,
    /// `--frontmatter`.
    pub frontmatter: bool,
    /// `--save`.
    pub save: bool,
    /// `--diff`.
    pub diff: bool,
    /// `--strict`.
    pub strict: bool,
}

type HashValue = CommandValue<HashData>;

/// A hash result's formatted text, which is the same in every formatted output.
fn hash_value(data: HashData, text: String) -> HashValue {
    let terminal = text.clone();
    CommandValue::new(data, code_document(None, text.trim_end()))
        .with_terminal(move |_| terminal.clone())
        .with_markdown(text)
}

/// Hash a markdown document's frontmatter and/or body.
///
/// The CLI owns environment and flag parsing; the library performs all
/// computation from an explicit [`MdHashOptions`] bundle. Bare invocation
/// reports the computed hash and exits `0`. `--diff` reports differences and
/// exits `1` when any are found or nothing is stored; `--save` writes the hash
/// back and exits `0`. Operational errors exit `1` too.
///
/// When the input is a directory, recursively finds all markdown files, hashes
/// each in parallel, and produces a single aggregate hash. Directory mode is
/// bare-hash only: `--save`, `--diff`, and the `structured`/`detailed` kinds are
/// rejected with a usage error.
#[instrument(skip_all)]
pub fn run_hash(input: Option<&PathBuf>, flags: HashFlags, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let mode = match (flags.save, flags.diff) {
        (true, _) => HashMode::Save,
        (_, true) => HashMode::Diff,
        _ => HashMode::Compute,
    };
    let file = input.filter(|path| path.to_str() != Some("-")).map(|path| path.display().to_string());
    let skeleton = HashData { mode, file: file.clone(), kind: None, computed: None, stored: None, comparison: None, saved: false };
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::typed(&skeleton))?;
    let result = hash_result(input, flags, skeleton, request);
    run.finish(result)
}

fn hash_result(input: Option<&PathBuf>, flags: HashFlags, base: HashData, request: &MdRequest) -> Result<Completed<HashValue>> {
    let HashFlags { kind, body, frontmatter, save, diff, strict } = flags;
    let options = resolve_hash_options(kind, body, frontmatter, strict);

    // A file argument opens once, here, so single-document mode, `--diff`, and
    // `--save` all act on the entry that decided the mode.
    let opened = match input {
        Some(path) if path.to_str() != Some("-") => match open_hash_argument(path, request)? {
            ReferenceEntry::Directory(directory) => {
                return run_hash_directory(&directory, &options, save, diff, base);
            }
            ReferenceEntry::File(file) => Some(file),
        },
        _ => None,
    };

    if save {
        let resolved = opened
            .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;
        let (source, md) = read_markdown_text(&resolved)?;
        let stored = md.stored_hash(&options)?;
        return run_hash_save(
            &md,
            &source,
            &resolved,
            stored.as_ref(),
            &options,
            chrono::Utc::now(),
            base,
        );
    }

    let md = match &opened {
        Some(file) => Markdown::try_from(file.as_path())
            .wrap_err_with(|| format!("Failed to read file: {:?}", file))?,
        None => load_markdown(input, request)?,
    };
    let stored = md.stored_hash(&options)?;

    if diff {
        return run_hash_diff(&md, stored.as_ref(), &options, base);
    }
    // Bare mode: select the kind (forced, else stored, else simple).
    let selected = select_kind(stored.as_ref().map(|s| s.kind), options.forced_kind);
    let computed = md.compute_hash(selected, &options);
    let text = computed_hash_text(&computed)?;
    let data = HashData {
        kind: Some(selected),
        computed: Some(HashValueData::from(&computed)),
        stored: stored.as_ref().map(HashValueData::from),
        ..base
    };
    Ok(Completed::success(hash_value(data, text)))
}

/// Resolves [`MdHashOptions`] from flags and the `HASH_PROPERTY` /
/// `HASH_IGNORE_PROPERTIES` environment variables.
///
/// `--kind` is the single forced-kind input; `--body` / `--frontmatter` are
/// shorthands the CLI maps to `Body` / `Fm`. `HASH_IGNORE_PROPERTIES` is additive
/// and can never un-ignore the active hash property or `last_updated`.
fn resolve_hash_options(
    kind: Option<MdHashKind>,
    body: bool,
    frontmatter: bool,
    strict: bool,
) -> MdHashOptions {
    let property = std::env::var("HASH_PROPERTY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|trimmed| !trimmed.is_empty())
        .unwrap_or_else(|| DEFAULT_HASH_PROPERTY.to_string());

    let extra_ignored = std::env::var("HASH_IGNORE_PROPERTIES")
        .ok()
        .map(|raw| parse_ignore_properties(&raw, &property))
        .unwrap_or_default();

    let forced_kind = match (kind, body, frontmatter) {
        (Some(k), _, _) => Some(k),
        (None, true, _) => Some(MdHashKind::Body),
        (None, _, true) => Some(MdHashKind::Fm),
        (None, false, false) => None,
    };

    MdHashOptions {
        property,
        extra_ignored,
        forced_kind,
        strict,
    }
}

/// Parses `HASH_IGNORE_PROPERTIES` as a CSV list of extra ignored properties:
/// trims each entry, drops empties, and excludes the always-managed `property`
/// and `last_updated` keys so they never leak into the stored `ignored` list.
fn parse_ignore_properties(raw: &str, property: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter(|entry| *entry != property && *entry != LAST_UPDATED_KEY)
        .map(String::from)
        .collect()
}

/// A computed hash's text: the flat string for `fm`/`body`/`simple`/`structured`,
/// or the nested YAML object for `detailed`.
fn computed_hash_text(computed: &ComputedHash) -> Result<String> {
    match computed.flat_string() {
        Some(flat) => Ok(format!("{flat}\n")),
        None => {
            let ComputedHash::Detailed(detailed) = computed else {
                unreachable!("only detailed hashes lack a flat string form");
            };
            serde_yaml_ng::to_string(detailed).wrap_err("Failed to serialize detailed hash")
        }
    }
}

/// `md hash --diff`: a kind-aware explanation, failing when the document differs
/// from its stored hash or has none.
fn run_hash_diff(md: &Markdown, stored: Option<&StoredHash>, options: &MdHashOptions, base: HashData) -> Result<Completed<HashValue>> {
    let Some(stored) = stored else {
        return Ok(Completed::failure(hash_value(base, "No stored hash to compare against\n".to_string())).with_diagnostics(vec![
            crate::output::output_diagnostic("dm.hash.no_stored_hash", Severity::Error, "No stored hash to compare against"),
        ]));
    };

    // Comparison first: a malformed stored value must fail before anything is
    // reported. Both calls hash the document under the same stored-policy
    // identity, so `--diff` computes that artifact twice.
    let comparison = md.compare_hash(stored, options)?;
    let explanation = md.explain_hash_diff(stored, options)?;
    let changed = comparison.frontmatter_changed || comparison.body_changed;
    let computed = md.compute_hash(stored.kind, options);
    let data = HashData {
        kind: Some(stored.kind),
        computed: Some(HashValueData::from(&computed)),
        stored: Some(HashValueData::from(stored)),
        comparison: Some(ComparisonData::from(&comparison)),
        ..base
    };
    let value = hash_value(data, format!("{}\n", explanation.render()));
    Ok(if changed {
        let diagnostic = crate::output::output_diagnostic("dm.hash.changed", Severity::Error, "The document differs from its stored hash");
        Completed::failure(value).with_diagnostics(vec![diagnostic])
    } else {
        Completed::success(value)
    })
}

/// `md hash --save`: writes the computed hash back into the document's
/// frontmatter and reports what changed. Always succeeds when the write does,
/// whether or not a write was needed.
fn run_hash_save(
    md: &Markdown,
    source: &str,
    resolved: &std::path::Path,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
    now: chrono::DateTime<chrono::Utc>,
    base: HashData,
) -> Result<Completed<HashValue>> {
    // A first baseline has nothing to compare against, so it has no explanation.
    // Computed here, before the write below, so it describes the in-memory
    // document against its *previous* stored hash.
    let explanation = match stored {
        Some(stored) => Some(md.explain_hash_diff(stored, options)?),
        None => None,
    };

    let stamp = md.stamp_baseline(source, options, now, Change::Detect)?;
    let saved = stamp.text.is_some();

    if let Some(written) = stamp.text {
        std::fs::write(resolved, written)
            .wrap_err_with(|| format!("Failed to write hash to {:?}", resolved))?;
    }

    let text = match explanation {
        Some(explanation) => format!("{}\n", explanation.render()),
        None => format!("No stored hash found; wrote initial {} baseline\n", stamp.decision.kind),
    };
    let computed = md.compute_hash(stamp.decision.kind, options);
    let data = HashData {
        kind: Some(stamp.decision.kind),
        computed: Some(HashValueData::from(&computed)),
        stored: stored.map(HashValueData::from),
        saved,
        ..base
    };
    Ok(Completed::success(hash_value(data, text)))
}

/// Opens a hash argument as the file or directory it names, through the same
/// reference grammar and candidate precedence as every file argument.
///
/// The first planned candidate that exists decides: a file there is hashed
/// as one document even when a later candidate is a directory, and a
/// directory there is hashed as a tree even when a later candidate is a file
/// (so `&docs` is the repository's `docs/`). A candidate that cannot be probed
/// stops the search with the `io` failure class rather than letting a later
/// candidate answer. A recursive (`%`) reference names files only.
///
/// ## Errors
///
/// A [`DocumentArgumentError`] when the argument is not valid reference
/// syntax (a relative path leaving the launch repository is
/// `InvalidReference`), a candidate cannot be probed, or nothing exists;
/// checked before any directory is read.
fn open_hash_argument(argument: &Path, request: &MdRequest) -> Result<ReferenceEntry> {
    let reference = parse_argument(argument)?;
    let raw = || argument.to_string_lossy().into_owned();
    match resolve_entry_in_context(&reference, request.launch_context()?) {
        Ok(Some(entry)) => Ok(entry),
        Ok(None) => Err(DocumentArgumentError::no_match(raw()).into()),
        Err(source) => Err(DocumentArgumentError::new(raw(), source).into()),
    }
}

/// Directory-aggregate hashing. Bare-hash only: `--save`, `--diff`, and the
/// `structured`/`detailed` kinds are rejected with a usage error.
fn run_hash_directory(
    path: &std::path::Path,
    options: &MdHashOptions,
    save: bool,
    diff: bool,
    base: HashData,
) -> Result<Completed<HashValue>> {
    let usage = |message: &str| Cli::command().error(clap::error::ErrorKind::ArgumentConflict, message);
    if save || diff {
        return Err(usage("--save and --diff are not supported when hashing a directory").into());
    }
    if matches!(
        options.forced_kind,
        Some(MdHashKind::Structured | MdHashKind::Detailed)
    ) {
        return Err(usage("--kind structured and --kind detailed are not supported when hashing a directory").into());
    }

    let body_only = options.forced_kind == Some(MdHashKind::Body);
    let frontmatter_only = options.forced_kind == Some(MdHashKind::Fm);

    // Directory mode is bare-hash only, so the kind is always one with a flat
    // string form. Computing through `compute_hash` (rather than the legacy
    // `Markdown::hash`) applies the managed-key and `HASH_IGNORE_PROPERTIES`
    // ignore policy, so an aggregate is stable when a file only gains `hash` /
    // `last_updated` and honors extra ignored properties.
    let kind = if body_only {
        MdHashKind::Body
    } else if frontmatter_only {
        MdHashKind::Fm
    } else {
        MdHashKind::Simple
    };

    let mut paths = collect_markdown_files(path)?;
    paths.sort();

    // Propagate per-file load/parse failures with path context rather than
    // hashing an empty document in their place. A single unreadable or
    // malformed file must fail the whole aggregate so `md hash <dir>` honors
    // the operational-error exit-code contract instead of producing a false
    // baseline.
    let per_file_hashes: Vec<String> = paths
        .par_iter()
        .map(|p| {
            let md = Markdown::try_from(p.as_path())
                .wrap_err_with(|| format!("Failed to read file: {}", p.display()))?;
            Ok(md
                .compute_hash(kind, options)
                .flat_string()
                .expect("simple/fm/body always have a flat string form"))
        })
        .collect::<Result<Vec<String>>>()?;

    let combined = per_file_hashes.join("\n");
    let aggregate = xx_hash(&combined);

    let flat = if body_only || frontmatter_only {
        format!("{:016x}", aggregate)
    } else {
        // Split each per-file "fm-body" hash, aggregate fm and body separately
        let (fm_parts, body_parts): (Vec<&str>, Vec<&str>) = per_file_hashes
            .iter()
            .filter_map(|h| h.split_once('-'))
            .unzip();

        let fm_aggregate = xx_hash(&fm_parts.join("\n"));
        let body_aggregate = xx_hash(&body_parts.join("\n"));
        format!("{:016x}-{:016x}", fm_aggregate, body_aggregate)
    };

    let data = HashData {
        mode: HashMode::Directory,
        kind: Some(kind),
        computed: Some(HashValueData { kind, value: serde_json::Value::String(flat.clone()), ignored: None }),
        ..base
    };
    Ok(Completed::success(hash_value(data, format!("{flat}\n"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_save_stamps_last_updated_with_the_utc_date() {
        let instant: chrono::DateTime<chrono::Utc> = "2026-09-28T23:30:00Z".parse().unwrap();
        let east_of_utc = chrono::FixedOffset::east_opt(10 * 3600).unwrap();
        assert_eq!(
            instant.with_timezone(&east_of_utc).date_naive().to_string(),
            "2026-09-29"
        );
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("doc.md");
        // A stale stored hash of the same kind is a content change, which bumps
        // `last_updated`.
        std::fs::write(
            &file,
            "---\ntitle: T\nhash: 1111111111111111-2222222222222222\nlast_updated: 2020-01-01\n---\n# H\n\nBody.\n",
        )
        .unwrap();
        let (source, md) = read_markdown_text(&file).unwrap();
        let options = MdHashOptions::default();
        let stored = md.stored_hash(&options).unwrap();

        let base = HashData { mode: HashMode::Save, file: None, kind: None, computed: None, stored: None, comparison: None, saved: false };
        run_hash_save(&md, &source, &file, stored.as_ref(), &options, instant, base).unwrap();

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(
            written.contains("last_updated: 2026-09-28\n"),
            "expected the UTC date stamp in:\n{written}"
        );
    }
}
