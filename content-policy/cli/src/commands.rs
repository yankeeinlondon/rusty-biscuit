//! Subcommand execution: read the document, call the library, print.
//!
//! Every error is returned as a message for stderr and exit code `1`; a
//! produced report or plan is exit `0` whatever it says.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use content_policy::{
    EvaluationContext, FileAdapter, RenewalContext, apply_renewal, evaluate_document,
    plan_renewal,
};

use crate::args::{CheckArgs, RenewArgs};
use crate::output::{TextOutput, needs_action};

pub fn check(args: &CheckArgs) -> Result<String, String> {
    let bytes = read(&args.document)?;
    let at = args
        .at
        .map_or_else(Utc::now, |date| date.and_time(chrono::NaiveTime::MIN).and_utc());
    let context = EvaluationContext::new(at)
        .with_options(args.config.options())
        .with_document(label(&args.document))
        .with_files(Arc::new(FileAdapter::new()), base_dir(&args.document));
    let report = evaluate_document(&bytes, &context).map_err(|error| error.to_string())?;
    Ok(if args.needs_action {
        format!("{}\n", needs_action(&report))
    } else if args.output.json {
        format!("{}\n", report.to_json())
    } else {
        TextOutput::new(args.output.plain).report(&report)
    })
}

pub fn renew(args: &RenewArgs) -> Result<String, String> {
    let bytes = read(&args.document)?;
    let today = args.today.unwrap_or_else(|| Utc::now().date_naive());
    let context = with_update_date(RenewalContext::new(today), args.on)
        .with_options(args.config.options())
        .with_document(label(&args.document))
        .with_files(Arc::new(FileAdapter::new()), base_dir(&args.document));
    let plan = plan_renewal(&bytes, &context).map_err(|error| error.to_string())?;
    if args.write {
        apply_renewal(&args.document, &plan).map_err(|error| error.to_string())?;
    }
    Ok(if args.output.json {
        format!("{}\n", plan.to_json())
    } else {
        TextOutput::new(args.output.plain).plan(&plan, args.write)
    })
}

fn with_update_date(context: RenewalContext, on: Option<NaiveDate>) -> RenewalContext {
    match on {
        Some(date) => context.on(date),
        None => context,
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// The directory `FileChanged` paths resolve from: the document's own. The
/// adapter resolves a relative spelling from the current directory, which is
/// also the document tree root outside a repository.
fn base_dir(document: &Path) -> PathBuf {
    match document.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// The document identity as the user spelled it; never canonicalized.
fn label(path: &Path) -> String {
    path.display().to_string()
}
