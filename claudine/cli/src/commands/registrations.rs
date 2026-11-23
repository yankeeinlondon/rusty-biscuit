//! The hidden `claudine registrations` command: lists, prints, or publishes
//! the Darkmatter host registration compiled into this binary
//! ([`RegistrationInventory::claudine`]). Maintainers' guide:
//! `claudine/docs/cli/registrations.md`.
//!
//! `main.rs` dispatches it before Darkmatter's configuration is read and
//! before first-run setup, so it works without a Claudine configuration.
//! Listing and printing read only the embedded inventory; `publish` writes
//! only the registration tree under the captured home.

use biscuit_hash::xx_hash_bytes;
use clap::Args;
use color_eyre::eyre::{Report, Result, eyre};
use serde::Serialize;

use claudine::composition::registration::{PublishedFile, RegistrationInventory, publish};

use crate::{log, request};

/// The positional token that publishes. No inventory path can equal it:
/// every published path is a `.yaml` or `.json` file name.
const PUBLISH: &str = "publish";

/// Arguments for `claudine registrations`.
#[derive(Debug, Args)]
pub struct RegistrationsArgs {
    /// A published path exactly as listed (prints that file with its
    /// contents), or `publish` (installs the registration). Omit to list.
    #[arg(value_name = "FILEPATH|publish")]
    pub target: Option<String>,
}

/// One printed file. Field order is the JSON key order; `contents` is
/// present only when one file is printed (`file_details`).
#[derive(Serialize)]
struct FileView<'a> {
    kind: &'static str,
    filepath: &'a str,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<&'a str>,
}

impl<'a> FileView<'a> {
    fn of(file: &'a PublishedFile) -> Self {
        Self {
            kind: file.kind().as_str(),
            filepath: file.path(),
            hash: format!("{:016x}", xx_hash_bytes(file.bytes())),
            name: file.name(),
            contents: None,
        }
    }
}

/// Runs the command; every refusal is an error, so the exit status is
/// nonzero and stdout stays empty.
pub fn run(args: &RegistrationsArgs) -> Result<()> {
    let inventory = RegistrationInventory::claudine();
    match args.target.as_deref() {
        None => print_json(&listing(inventory)),
        Some(PUBLISH) => {
            let home = request::snapshot()
                .home()
                .ok_or_else(|| eyre!("no home directory is known, so the registration cannot be published"))?;
            publish(home, inventory).map_err(Report::new)?;
            print_json(&listing(inventory))
        }
        Some(path) => {
            let file = inventory
                .file(path)
                .ok_or_else(|| eyre!("`{path}` is not a published registration path; run `claudine registrations` for the list"))?;
            let contents = file
                .text()
                .ok_or_else(|| eyre!("`{path}` is not UTF-8 text"))?;
            print_json(&FileView { contents: Some(contents), ..FileView::of(file) })
        }
    }
}

/// The `file[]` listing, in the inventory's path order.
fn listing(inventory: &RegistrationInventory) -> Vec<FileView<'_>> {
    inventory.files().iter().map(FileView::of).collect()
}

fn print_json(value: &impl Serialize) -> Result<()> {
    log::data(&serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_publish_token_is_not_a_published_path() {
        assert!(RegistrationInventory::claudine().file(PUBLISH).is_none());
    }

    #[test]
    fn a_listed_file_omits_name_and_contents_when_absent() {
        let file = PublishedFile::new(
            "schemas/a.yaml",
            claudine::composition::registration::FileKind::Library,
            None::<&str>,
            b"kind: schema\n".as_slice(),
        );
        let value = serde_json::to_value(FileView::of(&file)).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "kind": "schema",
                "filepath": "schemas/a.yaml",
                "hash": format!("{:016x}", xx_hash_bytes(b"kind: schema\n")),
            })
        );
    }
}
