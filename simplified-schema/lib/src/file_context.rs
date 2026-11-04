//! The document-relative file-resolution order shared by the
//! `darkmatter-file` format, `match(...)` globs, and a host's expression
//! functions.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError};

/// Canonical document-backed file-reference resolver shared by the expression
/// path and the schema validator.
///
/// Resolution runs through the request's
/// [`FileResolutionContext`](biscuit_file::FileResolutionContext) derived to
/// `cwd`, so it reads no ambient process state (CWD, `$HOME`, environment, or
/// git root). For a
/// local filesystem reference the candidate order is (D2/D3):
///
/// - **explicit** `./`/`../` → the document directory (`cwd`) only, no fallback;
/// - **implicit** bare paths → `cwd` first, then the repository root;
/// - `~`/`~/…` → the user's home directory only;
/// - `@`/`&`/`^`/`vault:`/`%`/absolute/URL → their existing `FileReference`
///   semantics against the context's configured roots.
///
/// The launch-area fallback the previous two-step resolver consulted is
/// **removed for nested documents** (D2): only repository and authoring-document
/// candidates participate. Magic roots and the repository root live on the
/// context; both surfaces — read-side expression functions and the
/// `darkmatter-file` schema format validator — share this single order.
///
/// Relative references may not leave the tree root unless it is only a
/// fallback to `cwd`.
///
/// ## Returns
///
/// - `Ok(Some(path))` when the reference resolves to a regular file.
/// - `Ok(None)` when the reference is well-formed but no candidate matched.
/// - `Err` when the context is invalid, a relative reference leaves the tree
///   root, or a required anchor cannot be established (missing home, missing
///   interpolation variable, unconfigured vault, or a candidate probe I/O
///   failure).
pub fn resolve_document_file_ref(
    file_ref: &FileReference,
    cwd: &Path,
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    #[cfg(feature = "file")]
    return file_ref.resolve_in_context(&document_file_context(cwd, request_context));
    // Without the `file` feature nothing is probed: the check is unavailable,
    // never silently passed.
    #[cfg(not(feature = "file"))]
    {
        let _ = (cwd, request_context);
        Err(FileReferenceError::ResolutionUnavailable { reference: file_ref.raw().to_string() })
    }
}

/// The context a document in `cwd` resolves through.
///
/// A snapshot already derived for `cwd` is used as is: re-deriving it would
/// drop its source path, its opening anchor (a `~` tree root), and a
/// trusted-external derivation. It is recognized by the directory it names,
/// not its spelling: a source keeps the spelling of the tree it was opened in
/// (macOS `/var`) while its canonical path does not (`/private/var`).
///
/// A `cwd` outside the request's tree is the caller's own document, so it is
/// derived the way compose admits such a source: trusted-external when only
/// that derivation is valid.
pub fn document_file_context(
    cwd: &Path,
    request_context: &biscuit_file::FileResolutionContext,
) -> biscuit_file::FileResolutionContext {
    if request_context.cwd() == cwd || same_directory(request_context.cwd(), cwd) {
        return request_context.clone();
    }
    let ordinary = request_context.for_cwd(cwd);
    if ordinary.validate().is_err() {
        let external = request_context.for_trusted_external_cwd(cwd);
        if external.validate().is_ok() {
            return external;
        }
    }
    ordinary
}

fn same_directory(left: &Path, right: &Path) -> bool {
    match (
        biscuit_file::canonicalize_simplified(left),
        biscuit_file::canonicalize_simplified(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
