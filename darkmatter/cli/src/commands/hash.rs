//! `md hash` subcommand implementation.

use crate::io::{DocumentArgumentError, load_markdown, parse_argument, read_markdown_text};
use crate::request::MdRequest;
use biscuit_hash::xx_hash;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::hash::{
    ComputedHash, DEFAULT_HASH_PROPERTY, LAST_UPDATED_KEY, MdHashKind, MdHashOptions, StoredHash,
    select_kind,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::fs::{ReferenceEntry, collect_markdown_files, resolve_entry_in_context};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Hash a markdown document's frontmatter and/or body.
///
/// The CLI owns environment and flag parsing; the library performs all
/// computation from an explicit [`MdHashOptions`] bundle. Bare invocation prints
/// the computed hash and exits `0`. `--diff` reports differences and exits `2`
/// when any are found; `--save` writes the hash back and exits `0`. Operational
/// errors propagate on the eyre path (exit `1`).
///
/// When the input is a directory, recursively finds all markdown files, hashes
/// each in parallel, and produces a single aggregate hash. Directory mode is
/// bare-hash only: `--save`, `--diff`, and the `structured`/`detailed` kinds are
/// rejected with a usage error.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn run_hash(
    input: Option<&PathBuf>,
    kind: Option<MdHashKind>,
    body: bool,
    frontmatter: bool,
    save: bool,
    diff: bool,
    strict: bool,
    request: &MdRequest,
) -> Result<()> {
    let options = resolve_hash_options(kind, body, frontmatter, strict);

    // A file argument opens once, here, so single-document mode, `--diff`, and
    // `--save` all act on the entry that decided the mode.
    let opened = match input {
        Some(path) if path.to_str() != Some("-") => match open_hash_argument(path, request)? {
            ReferenceEntry::Directory(directory) => {
                return run_hash_directory(&directory, &options, save, diff);
            }
            ReferenceEntry::File(file) => Some(file),
        },
        _ => None,
    };

    if save {
        let resolved = opened
            .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;
        let (source, md) = read_markdown_text(&resolved)?;
        let stored = parse_stored_hash(&md, &options)?;
        return run_hash_save(&md, &source, &resolved, stored.as_ref(), &options);
    }

    let md = match &opened {
        Some(file) => Markdown::try_from(file.as_path())
            .wrap_err_with(|| format!("Failed to read file: {:?}", file))?,
        None => load_markdown(input, request)?,
    };
    let stored = parse_stored_hash(&md, &options)?;

    if diff {
        return run_hash_diff(&md, stored.as_ref(), &options);
    }
    // Bare mode: select the kind (forced, else stored, else simple) and print.
    let selected = select_kind(stored.map(|s| s.kind), options.forced_kind);
    let computed = md.compute_hash(selected, &options);
    print_computed_hash(&computed)?;
    Ok(())
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

/// Reads and parses the document's stored hash property, or `None` when the
/// property is absent or null.
fn parse_stored_hash(md: &Markdown, options: &MdHashOptions) -> Result<Option<StoredHash>> {
    match md.frontmatter().as_map().get(options.property.as_str()) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => Ok(Some(StoredHash::parse(value, &options.property)?)),
    }
}

/// Prints a computed hash: the flat string for `fm`/`body`/`simple`/`structured`,
/// or the nested YAML object for `detailed`.
fn print_computed_hash(computed: &ComputedHash) -> Result<()> {
    match computed.flat_string() {
        Some(flat) => println!("{flat}"),
        None => {
            let ComputedHash::Detailed(detailed) = computed else {
                unreachable!("only detailed hashes lack a flat string form");
            };
            let yaml = serde_yaml_ng::to_string(detailed)
                .wrap_err("Failed to serialize detailed hash")?;
            print!("{yaml}");
        }
    }
    Ok(())
}

/// `md hash --diff`: prints a kind-aware explanation and exits `2` when the
/// document differs from its stored hash (or has none).
fn run_hash_diff(
    md: &Markdown,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
) -> Result<()> {
    let Some(stored) = stored else {
        println!("No stored hash to compare against");
        std::process::exit(2);
    };

    // Comparison first: a malformed stored value must fail before anything is
    // printed. Both calls hash the document under the same stored-policy
    // identity, so `--diff` computes that artifact twice.
    let comparison = md.compare_hash(stored, options)?;
    let explanation = md.explain_hash_diff(stored, options)?;
    println!("{}", explanation.render());

    if comparison.frontmatter_changed || comparison.body_changed {
        std::process::exit(2);
    }
    Ok(())
}

/// `md hash --save`: writes the computed hash back into the document's
/// frontmatter and prints an explanation of what changed. Always exits `0` on
/// success, whether or not a write was needed.
fn run_hash_save(
    md: &Markdown,
    source: &str,
    resolved: &std::path::Path,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
) -> Result<()> {
    let decision = md.plan_hash_save(stored, options)?;

    // A first baseline has nothing to compare against, so it has no explanation.
    // Computed here, before the write below, so it describes the in-memory
    // document against its *previous* stored hash.
    let explanation = match stored {
        Some(stored) => Some(md.explain_hash_diff(stored, options)?),
        None => None,
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    if let Some(written) = darkmatter::markdown::hash::apply_hash_save_text(
        source, &decision, options, &today,
    )? {
        std::fs::write(resolved, written)
            .wrap_err_with(|| format!("Failed to write hash to {:?}", resolved))?;
    }

    match explanation {
        Some(explanation) => println!("{}", explanation.render()),
        None => {
            println!(
                "No stored hash found; wrote initial {} baseline",
                decision.kind
            );
        }
    }
    Ok(())
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
) -> Result<()> {
    if save || diff {
        return Err(eyre!(
            "--save and --diff are not supported when hashing a directory"
        ));
    }
    if matches!(
        options.forced_kind,
        Some(MdHashKind::Structured | MdHashKind::Detailed)
    ) {
        return Err(eyre!(
            "--kind structured and --kind detailed are not supported when hashing a directory"
        ));
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

    if body_only || frontmatter_only {
        println!("{:016x}", aggregate);
    } else {
        // Split each per-file "fm-body" hash, aggregate fm and body separately
        let (fm_parts, body_parts): (Vec<&str>, Vec<&str>) = per_file_hashes
            .iter()
            .filter_map(|h| h.split_once('-'))
            .unzip();

        let fm_aggregate = xx_hash(&fm_parts.join("\n"));
        let body_aggregate = xx_hash(&body_parts.join("\n"));
        println!("{:016x}-{:016x}", fm_aggregate, body_aggregate);
    }

    Ok(())
}
