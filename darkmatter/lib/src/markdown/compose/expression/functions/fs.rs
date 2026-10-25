//! Context-aware handler bodies: filesystem, path, skill, and document
//! functions that read a `ResolutionContext`.

use super::*;

/// Constructs an [`ExpressionError::Other`] for `function` with `message`.
pub(super) fn expression_other(function: &'static str, message: String) -> ExpressionError {
    ExpressionError::Other {
        function: function.to_string(),
        message,
    }
}

/// The bound text of a required (non-null) `string` or `file` parameter.
///
/// Binding guarantees the value is text, so a miss is a Darkmatter defect;
/// it is reported as a contract violation instead of a panic.
pub(super) fn required_text<'a>(
    function: &'static str,
    call: &'a BoundCall<'_>,
    index: usize,
) -> Result<&'a str, ExpressionError> {
    call.args().text(index).ok_or_else(|| ExpressionError::ContractViolation {
        function: function.to_string(),
        message: format!("argument {index} did not bind to text"),
    })
}

pub(super) fn file_reference_error(
    function: &'static str,
    raw: &str,
    ctx: &ResolutionContext,
    kind: FileRefFailure,
    source: Option<biscuit_file::FileReferenceError>,
) -> ExpressionError {
    let caller = ctx.active_caller_file_provenance.as_ref().map(|provenance| {
        Arc::new(super::super::CallerFileDiagnosticProvenance {
            property: provenance.property.clone(),
            origin: provenance.origin.clone(),
            candidate: provenance.candidate.clone(),
            candidate_provenance: provenance.candidate_provenance,
        })
    });
    let (reference, cwd) = ctx
        .active_caller_file_provenance
        .as_ref()
        .map(|provenance| (provenance.reference.clone(), provenance.origin.cwd().to_path_buf()))
        .unwrap_or_else(|| (raw.to_string(), ctx.cwd.clone()));
    ExpressionError::FileReference(FileReferenceDiagnostic {
        function,
        reference,
        kind,
        cwd,
        fallback_dir: ctx.file_ref_fallback_dir.clone(),
        source: source.map(Arc::new),
        caller,
    })
}

pub(super) fn resolve_arg(
    function: &'static str,
    raw: &str,
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, ExpressionError> {
    let file_ref = parse_file_ref(function, raw, ctx)?;
    // Per D2 there is no launch-area fallback for a nested-document
    // reference — only repository and authoring-document candidates participate.
    resolve_document_file_ref(&file_ref, &ctx.cwd, &ctx.file_resolution_context)
    .map_err(|e| {
        file_reference_error(
            function,
            raw,
            ctx,
            FileRefFailure::classify(&e),
            Some(e),
        )
    })
}

pub(super) fn parse_file_ref(
    function: &'static str,
    raw: &str,
    ctx: &ResolutionContext,
) -> Result<biscuit_file::FileReference, ExpressionError> {
    biscuit_file::FileReference::new(&normalize_path_arg(raw)).map_err(|e| {
        file_reference_error(function, raw, ctx, FileRefFailure::classify(&e), Some(e))
    })
}

pub(super) fn reject_remote_url(function: &'static str, raw: &str) -> Result<(), ExpressionError> {
    if is_remote_url(raw) {
        return Err(expression_other(
            function,
            format!("{function}() does not accept HTTP(S) URLs"),
        ));
    }
    Ok(())
}

/// `absolute(file) -> file` — the absolute path a reference names.
///
/// An existing file returns its matched path. A missing local target returns
/// the resolver's first candidate for it, lexically normalized: the path a
/// later write to that reference would create. `absolute` therefore does not
/// prove existence; [`file_exists_fn`] does. The result uses the platform's
/// native separators.
///
/// ## Examples
///
/// With the document in `/r/prompts` inside repository `/r`:
///
/// - `absolute("notes.md")` → `/r/prompts/notes.md` when that file exists,
///   else `/r/notes.md` when only the repository copy exists.
/// - `absolute("./sub/new.md")` with no such file → `/r/prompts/sub/new.md`.
///
/// ## Errors
///
/// - An HTTP(S) URL is rejected without being fetched.
/// - A recursive (`%`) reference with no match is a `not_found` file-reference
///   error: its candidates are search directories, not a file path.
/// - An invalid reference, a tree escape, a missing home/vault/repository
///   anchor, or a candidate probe I/O failure is a typed file-reference error.
pub fn absolute_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    reject_remote_url("absolute", raw)?;
    let file_ref = parse_file_ref("absolute", raw, ctx)?;
    let typed = |e: biscuit_file::FileReferenceError| {
        file_reference_error("absolute", raw, ctx, FileRefFailure::classify(&e), Some(e))
    };
    let path = if file_ref.class().recursive {
        resolve_document_file_ref(&file_ref, &ctx.cwd, &ctx.file_resolution_context)
            .map_err(typed)?
            .ok_or_else(|| {
                file_reference_error("absolute", raw, ctx, FileRefFailure::NotFound, None)
            })?
    } else {
        resolve_document_file_ref_target(&file_ref, &ctx.cwd, &ctx.file_resolution_context)
            .map_err(typed)?
    };
    Ok(Value::String(path.to_string_lossy().to_string()))
}

/// `file_exists(value: unknown | null) -> bool` — whether a reference names an existing
/// file.
///
/// A well-formed reference with no matching regular file (including one that
/// names a directory) is `false`, and so is `null`: an optional reference that
/// was never supplied names no file. The argument is inspected, never
/// converted: a number, boolean, array, or object is `false`, so `42` does not
/// name a file called `42`. Only text can be `true`, which is what lets a
/// `file_exists(x)` guard prove `x` a present string.
///
/// Text that is not a well-formed reference ([`is_well_formed_reference`]:
/// `""`, `https://`) is `false` before any filesystem or network access; it
/// can never name a file, so it is never tried as a local path.
///
/// ## Errors
///
/// A failure to answer is never reported as absence. A tree escape, a missing
/// home/vault/repository anchor, and a candidate probe I/O failure (a
/// permission error, for example) are typed file-reference errors. An HTTP(S)
/// URL is `true` once fetched; a denied or failed fetch is an error, and so is
/// any URL in a local-only context (no remote runtime).
pub fn file_exists_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    // The parameter is preservation-only (`unknown | null`), so this is the
    // caller's original value: only text names a file (spec decision 82).
    let Some(Value::String(raw)) = call.args().get(0) else {
        return Ok(Value::Bool(false));
    };
    let raw = raw.as_str();
    let normalized = normalize_path_arg(raw);
    let Ok(file_ref) = biscuit_file::FileReference::new(&normalized) else {
        return Ok(Value::Bool(false));
    };
    if file_ref.class().kind == biscuit_file::FileReferenceKind::Url {
        return match ctx.fetch_remote_text(&normalized) {
            Ok(Some(_)) => Ok(Value::Bool(true)),
            Ok(None) => Err(expression_other("file_exists", format!(
                "file_exists() cannot read remote URL {raw:?}: this resolution \
                 context is local-only (no remote runtime)"
            ))),
            Err(message) => Err(expression_other("file_exists", message)),
        };
    }
    resolve_document_file_ref(&file_ref, &ctx.cwd, &ctx.file_resolution_context)
        .map(|found| Value::Bool(found.is_some()))
        .map_err(|e| file_reference_error("file_exists", raw, ctx, FileRefFailure::classify(&e), Some(e)))
}

/// `has_command(cmd: string | null) -> bool` — pure existence/executability
/// probe for a host command. Reports whether `cmd` is on `PATH` or is an
/// existing executable path; it **never executes** the command. `null` and
/// empty text read as `false`; a number or boolean arrives converted to text,
/// so `has_command(true)` looks for a command named `true`.
///
/// Only two input shapes can probe: a bare name (`PATH` search) and an
/// absolute path (exists + executable, delegated to `which`). Any other
/// path-shaped input — `./mytool`, `bin/mytool`, `~/bin/mytool`, or a Windows
/// drive-relative `C:mytool` — reads as `false`: it is neither tilde-expanded
/// nor resolved against the resolution context or the process CWD.
pub fn has_command_fn(call: &BoundCall<'_>, _ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(Value::Bool(call.args().text(0).is_some_and(command_exists)))
}

/// Whether `raw` names a command [`has_command_fn`] finds; `false` for empty
/// text. Shared with `can_execute`, which checks it before any shell probe.
pub(super) fn command_exists(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    // `which::which` resolves relative paths against the process CWD, which
    // would make document truth depend on where the composer was launched —
    // reject them before probing. The multi-component test mirrors `which`'s
    // own separator detection and also catches Windows drive-relative inputs
    // (`C:mytool` parses as Prefix + Normal but is not absolute).
    let path = std::path::Path::new(raw);
    if !path.is_absolute() && path.components().count() > 1 {
        return false;
    }
    which::which(raw).is_ok()
}

/// `relative(file) -> file | Error::InvalidFilePath`
///
/// The returned path uses forward slashes so it can be persisted in Markdown
/// without platform-dependent text drift.
pub fn relative_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let abs = match resolve_arg("relative", raw, ctx)? {
        Some(p) => p,
        None => {
            return Err(file_reference_error(
                "relative",
                raw,
                ctx,
                FileRefFailure::NotFound,
                None,
            ));
        }
    };
    Ok(Value::String(make_portable_relative_in_context(
        &abs,
        &ctx.cwd,
        &ctx.file_resolution_context,
    )))
}

/// Resolves a raw path string to an absolute path shape through the shared
/// FS-path rules.
///
/// Rejects HTTP(S) URLs. Otherwise delegates entirely to `FileReference`
/// classification and its ordered candidate plan (D1/D3): an existing target
/// resolves to its matched path, and a missing target takes the plan's first
/// candidate as its shape. Path-component functions thus operate on missing
/// files and directories without a private prefix grammar or an existence
/// probe. Both existing and missing targets are shaped by the shared
/// [`resolve_document_file_ref_shape`], so a shape can never disagree with an
/// execution-time resolution on classification, candidate order, or
/// document-first anchoring.
pub(super) fn resolve_path_shape(
    name: &'static str,
    raw: &str,
    ctx: &ResolutionContext,
) -> Result<PathBuf, ExpressionError> {
    resolve_anchored_path_shape(name, raw, ctx).map(|shape| shape.path)
}

/// A resolved path shape and whether its authored reference was absolute.
pub(super) struct PathShape {
    path: PathBuf,
    absolute_input: bool,
}

/// [`resolve_path_shape`] plus whether the authored reference was absolute,
/// for the functions that return a path built from their input.
pub(super) fn resolve_anchored_path_shape(
    name: &'static str,
    raw: &str,
    ctx: &ResolutionContext,
) -> Result<PathShape, ExpressionError> {
    reject_remote_url(name, raw)?;
    let file_ref = parse_file_ref(name, raw, ctx)?;
    // The shared shaper builds the same candidate plan execution probes
    // (document-first for implicit references), so a missing target's shape
    // is the first shared candidate rather than a source-first join.
    let path = resolve_document_file_ref_shape(&file_ref, &ctx.cwd, &ctx.file_resolution_context)
        .map_err(|e| file_reference_error(name, raw, ctx, FileRefFailure::classify(&e), Some(e)))?;
    Ok(PathShape {
        path,
        absolute_input: file_ref.class().kind == biscuit_file::FileReferenceKind::Absolute,
    })
}

/// Renders a path a function built from its input.
///
/// An absolute input keeps its absolute anchor, in portable spelling. Its
/// repository- or document-relative display form would be re-resolved
/// document-directory first by a later reader and could name a different
/// file. Any other input keeps the display form ([`path_display_components`]).
pub(super) fn render_built_path(path: &Path, absolute_input: bool, ctx: &ResolutionContext) -> String {
    if absolute_input {
        biscuit_file::to_portable_string(path)
    } else {
        make_portable_relative_in_context(path, &ctx.cwd, &ctx.file_resolution_context)
    }
}

/// Parses a stem against the indexed grammar, returning the base name, parsed
/// index, and the original zero-padding width of the index.
pub(super) fn indexed_stem_info(stem: &str) -> Option<(String, u64, usize)> {
    let last_hyphen = stem.rfind('-')?;
    if last_hyphen == 0 {
        return None;
    }
    if stem.as_bytes().get(last_hyphen - 1) == Some(&b'-') {
        return None;
    }
    let base = &stem[..last_hyphen];
    let digits = &stem[last_hyphen + 1..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let width = digits.len();
    let index = digits.parse::<u64>().ok()?;
    Some((base.to_string(), index, width))
}

/// Formats an indexed stem, preserving the original zero-padding width.
pub(super) fn format_indexed_stem(base: &str, index: u64, width: usize) -> String {
    format!("{base}-{index:0>width$}")
}

/// Splits a resolved path into its display directory components and basename.
///
/// The display shape follows the same `relative(file)` policy used elsewhere:
/// repo-root relative, base-dir relative, `~`-aliased, or absolute. Components
/// are returned with `/` separators in mind.
pub(super) fn path_display_components(
    path: &Path,
    base_dir: &Path,
    request_context: &biscuit_file::FileResolutionContext,
) -> (Vec<String>, String) {
    let rel = make_portable_relative_in_context(path, base_dir, request_context);
    let trimmed = rel.strip_prefix('/').unwrap_or(&rel).to_string();
    match trimmed.rfind('/') {
        Some(pos) => {
            let dir_part = &trimmed[..pos];
            let dirs = if dir_part.is_empty() {
                Vec::new()
            } else {
                dir_part.split('/').map(|s| s.to_string()).collect()
            };
            (dirs, trimmed[pos + 1..].to_string())
        }
        None => (Vec::new(), trimmed),
    }
}

/// `is_indexed_file(file) -> bool` — true when the filename stem matches the
/// indexed grammar (`base-NNN`); `null` is `false`.
pub fn is_indexed_file_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Bool(false));
    };
    let path = resolve_path_shape("is_indexed_file", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    let stem = file_stem(&base);
    Ok(Value::Bool(parse_indexed_stem(&stem).is_some()))
}

/// `file_index(file) -> number` — the parsed index, or `-1` when non-indexed.
pub fn file_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("file_index", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    let stem = file_stem(&base);
    let index = parse_indexed_stem(&stem)
        .map(|i| i.index as i64)
        .unwrap_or(-1);
    Ok(Value::Number(index.into()))
}

/// `increment_file_index(file) -> string` — bumps the numeric suffix, starting
/// at `2` for non-indexed files. Preserves zero-padding width for indexed stems.
///
/// The input is resolved first (an existing file, else its first candidate)
/// and only the filename changes, so the result sits in the resolved parent
/// directory. An absolute input returns an absolute path; any other input
/// returns its display form.
pub fn increment_file_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let PathShape { path, absolute_input } =
        resolve_anchored_path_shape("increment_file_index", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    let ext = file_extension(&base);
    let stem = file_stem(&base);
    let new_stem = if let Some((base_name, index, width)) = indexed_stem_info(&stem) {
        let next = index.saturating_add(1);
        format_indexed_stem(&base_name, next, width)
    } else {
        format!("{stem}-2")
    };
    let new_base = if ext.is_empty() {
        new_stem
    } else {
        format!("{new_stem}.{ext}")
    };
    let out = path
        .parent()
        .map(|p| p.join(&new_base))
        .unwrap_or_else(|| PathBuf::from(&new_base));
    Ok(Value::String(render_built_path(&out, absolute_input, ctx)))
}

/// `decrement_file_index(file) -> string` — decrements the numeric suffix,
/// clamped at `0`. Non-indexed files start at `0` and preserve no padding.
///
/// Resolves its input and renders its result as [`increment_file_index_fn`]
/// does.
pub fn decrement_file_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let PathShape { path, absolute_input } =
        resolve_anchored_path_shape("decrement_file_index", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    let ext = file_extension(&base);
    let stem = file_stem(&base);
    let new_stem = if let Some((base_name, index, width)) = indexed_stem_info(&stem) {
        let next = index.saturating_sub(1);
        format_indexed_stem(&base_name, next, width)
    } else {
        format!("{stem}-0")
    };
    let new_base = if ext.is_empty() {
        new_stem
    } else {
        format!("{new_stem}.{ext}")
    };
    let out = path
        .parent()
        .map(|p| p.join(&new_base))
        .unwrap_or_else(|| PathBuf::from(&new_base));
    Ok(Value::String(render_built_path(&out, absolute_input, ctx)))
}

#[derive(Clone, Copy)]
pub(super) enum IndexEndpoint {
    First,
    Last,
}

pub(super) fn find_index_endpoint(
    name: &'static str,
    call: &BoundCall<'_>,
    ctx: &ResolutionContext,
    endpoint: IndexEndpoint,
) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let PathShape {
        path: input,
        absolute_input,
    } = resolve_anchored_path_shape(name, raw, ctx)?;
    let Some(parent) = input.parent() else {
        return Ok(Value::String(render_built_path(&input, absolute_input, ctx)));
    };
    let filename = input
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let input_stem = file_stem(&filename);
    let input_extension = file_extension(&filename);
    let family_base = indexed_stem_info(&input_stem)
        .map(|(base, _, _)| base)
        .unwrap_or(input_stem);

    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Value::String(render_built_path(&input, absolute_input, ctx)));
        }
        Err(error) => {
            return Err(expression_other(
                name,
                format!("failed to scan {}: {error}", parent.display()),
            ));
        }
    };
    let mut candidates = Vec::<(Option<u64>, String)>::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            expression_other(name, format!("failed to read {}: {error}", parent.display()))
        })?;
        let actual_name = entry.file_name().to_string_lossy().into_owned();
        if file_extension(&actual_name) != input_extension {
            continue;
        }
        let stem = file_stem(&actual_name);
        let ordinal = if stem == family_base {
            None
        } else if let Some((base, index, _)) = indexed_stem_info(&stem) {
            if base != family_base {
                continue;
            }
            Some(index)
        } else {
            continue;
        };
        candidates.push((ordinal, actual_name));
    }
    candidates.sort();
    let selected = match endpoint {
        IndexEndpoint::First => candidates.first(),
        IndexEndpoint::Last => candidates.last(),
    };
    let chosen = selected
        .map(|(_, filename)| parent.join(filename))
        .unwrap_or(input);
    Ok(Value::String(render_built_path(&chosen, absolute_input, ctx)))
}

/// Returns the lowest-indexed existing member of a file's index family,
/// scanning the input's resolved parent directory. With no member present the
/// input itself is returned. An absolute input returns an absolute path; any
/// other input returns its display form.
pub fn find_first_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    find_index_endpoint("find_first_index", call, ctx, IndexEndpoint::First)
}

/// Returns the highest-indexed existing member of a file's index family,
/// resolved and rendered as [`find_first_index_fn`] is.
pub fn find_last_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    find_index_endpoint("find_last_index", call, ctx, IndexEndpoint::Last)
}

/// `basename(file) -> string` — the final path component including extension.
pub fn basename_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("basename", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(base))
}

/// `basename_without_index(file) -> string` — removes an indexed suffix from
/// the stem. Non-indexed basenames pass through unchanged.
pub fn basename_without_index_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("basename_without_index", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    let stem = file_stem(&base);
    let ext = file_extension(&base);
    let unindexed = match indexed_stem_info(&stem) {
        Some((base_name, _, _)) => base_name,
        None => stem,
    };
    let out = if ext.is_empty() {
        unindexed
    } else {
        format!("{unindexed}.{ext}")
    };
    Ok(Value::String(out))
}

/// `dirname(file) -> string` — the directory portion of a path.
///
/// An absolute input returns its absolute parent (portable spelling), even
/// inside the repository or the document directory, so a schema-materialized
/// caller value can derive a sibling path (`dirname(absolute(spec)) +
/// '/review-1.md'`) that stays beside it instead of being re-resolved against
/// the document. Any other input returns the directory portion of its display
/// path (`dirname("sub/x.md")` is `sub`).
pub fn dirname_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let PathShape { path, absolute_input } = resolve_anchored_path_shape("dirname", raw, ctx)?;
    // A path outside every projection root (repository, base directory, home)
    // has no relative display form; its absolute parent is the only anchor
    // that still resolves when another file name is appended to it.
    let projected = make_portable_relative_in_context(
        &path,
        &ctx.cwd,
        &ctx.file_resolution_context,
    );
    if absolute_input || Path::new(&projected).is_absolute() {
        return Ok(Value::String(
            path.parent()
                .map(biscuit_file::to_portable_string)
                .unwrap_or_default(),
        ));
    }
    let (dirs, _) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(if dirs.is_empty() {
        String::new()
    } else {
        dirs.join("/")
    }))
}

/// `ext(file) -> string` — the final extension without the leading dot, or an
/// empty string when there is none.
pub fn ext_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("ext", raw, ctx)?;
    let (_, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(file_extension(&base)))
}

/// `parent_dir(file) -> string` — the directory segment immediately above the
/// basename, or an empty string when there is none.
pub fn parent_dir_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("parent_dir", raw, ctx)?;
    let (dirs, _) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(dirs.last().cloned().unwrap_or_default()))
}

/// `file_trailing(file) -> string` — the last directory segment plus the
/// basename, or just the basename when there is no directory.
pub fn file_trailing_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("file_trailing", raw, ctx)?;
    let (dirs, base) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(match dirs.last() {
        Some(d) => format!("{d}/{base}"),
        None => base,
    }))
}

/// `dir_leading(file) -> string` — the directory path before the last segment,
/// or an empty string when there is no leading directory.
pub fn dir_leading_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let path = resolve_path_shape("dir_leading", raw, ctx)?;
    let (dirs, _) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
    Ok(Value::String(if dirs.len() <= 1 {
        String::new()
    } else {
        dirs[..dirs.len() - 1].join("/")
    }))
}

/// `join(left, right) -> string` — joins two path strings, normalizing leading
/// and duplicate separators. Validates the result through the shared FS-path
/// rules and rejects HTTP(S) arguments.
pub fn join_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let (Some(left_raw), Some(right_raw)) = (call.args().text(0), call.args().text(1)) else {
        return Ok(Value::Null);
    };
    if is_remote_url(left_raw) || is_remote_url(right_raw) {
        return Err(expression_other(
            "join",
            "join() does not accept HTTP(S) URLs".to_string(),
        ));
    }
    let left = resolve_path_shape("join", left_raw, ctx)?;
    let right = right_raw.trim_start_matches(['/', '\\']);
    let joined = left.join(right);
    let joined_str = joined.to_string_lossy().to_string();
    let validated = resolve_path_shape("join", &joined_str, ctx)?;
    Ok(Value::String(
        make_portable_relative_in_context(
            &validated,
            &ctx.cwd,
            &ctx.file_resolution_context,
        ),
    ))
}

/// Backslash-escapes the characters CommonMark treats as active in link text.
///
/// The backslash itself is escaped first, and is not optional: the two-argument
/// `link(target, desc)` takes its description straight from the caller, who can
/// supply a literal `\`. CommonMark honors a backslash before ASCII punctuation
/// inside link text, so an unescaped `\\server\share\[x]` would lose separators
/// and reopen the very bracket escaping this function performs.
///
/// The one-argument arm cannot reach that case: its description and its
/// destination project the same path, so a description that fell back to a
/// native spelling implies `portable_destination` already rejected the
/// destination.
///
/// Whitespace is preserved. The sibling
/// [`collapse_and_escape`](escape::collapse_and_escape) collapses it
/// deliberately, which is right for provider prose and wrong for a path.
pub(super) fn escape_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// Returns `true` when a link destination needs angle-bracket wrapping to stay
/// CommonMark-safe.
pub(super) fn destination_needs_wrapping(dest: &str) -> bool {
    dest.chars().any(|c| {
        c.is_ascii_control()
            || c == ' '
            || c == '\t'
            || c == '('
            || c == ')'
            || c == '<'
            || c == '>'
    })
}

/// Formats a Markdown inline link `[text](destination)` with text and
/// destination escaping.
pub(super) fn format_markdown_link(text: &str, destination: &str) -> String {
    format!("[{}]({})", escape_link_text(text), markdown_destination(destination))
}

/// Formats a Markdown inline link whose text is one code span.
///
/// The label is fenced by [`renderable::markdown::code_span`] and is otherwise
/// literal: `[`, `]`, and `\` are not escaped, because a code span binds
/// tighter than link brackets and gives backslashes no meaning. An empty label
/// yields `[](destination)`, the same as `link(target, "")`.
pub(super) fn format_markdown_code_link(text: &str, destination: &str) -> String {
    format!(
        "[{}]({})",
        renderable::markdown::code_span(text),
        markdown_destination(destination)
    )
}

/// Spells a link destination, wrapping it in angle brackets when CommonMark
/// would otherwise end or split it.
pub(super) fn markdown_destination(destination: &str) -> String {
    if destination_needs_wrapping(destination) {
        let inner = destination.replace('<', "\\<").replace('>', "\\>");
        format!("<{inner}>")
    } else {
        destination.to_string()
    }
}

/// Renders a resolved local path as a `link()`-family destination.
///
/// ## Errors
///
/// [`ExpressionError`] when the path has no faithful portable spelling (a
/// Windows UNC, device, or unreducible verbatim path). `link()` has no
/// compose-report warning channel, and falling back to the raw argument is not
/// an option: it may be a magic (`@`), repository (`&`/`^`), or source-relative
/// reference that
/// names a different file once the composed document moves.
pub(super) fn portable_destination(function: &'static str, path: &Path) -> Result<String, ExpressionError> {
    biscuit_file::try_portable_string(path).ok_or_else(|| {
        expression_other(
            function,
            format!(
                "{function}() cannot render {} as a Markdown destination: no faithful portable spelling exists, and CommonMark would consume the backslash escapes in its native form.",
                path.display()
            ),
        )
    })
}

/// The raw label and destination shared by `link()` and `code_link()`, before
/// either spells the label as Markdown.
pub(super) struct LinkParts {
    label: String,
    destination: String,
}

/// Resolves the `link(file)` / `link(target, desc)` argument shapes for
/// `function`, which names the caller in every error.
///
/// Returns `Ok(None)` when a null argument propagates to a null result.
pub(super) fn resolve_link_parts(
    function: &'static str,
    call: &BoundCall<'_>,
    ctx: &ResolutionContext,
) -> Result<Option<LinkParts>, ExpressionError> {
    let args = call.args();
    // Overloads in catalog order: `(file)`, then `(target, desc)`.
    match call.overload() {
        0 => {
            let Some(raw) = args.text(0) else {
                return Ok(None);
            };
            if is_remote_url(raw) {
                return Err(expression_other(
                    function,
                    format!(
                        "{function}() one-argument form does not accept HTTP(S) URLs; use {function}(target, desc)"
                    ),
                ));
            }
            let path = resolve_path_shape(function, raw, ctx)?;
            let label = make_portable_relative_in_context(
                &path,
                &ctx.cwd,
                &ctx.file_resolution_context,
            );
            let destination = portable_destination(function, &path)?;
            Ok(Some(LinkParts { label, destination }))
        }
        _ => {
            let (Some(target_raw), Some(label)) = (args.text(0), args.text(1)) else {
                return Ok(None);
            };
            let label = label.to_string();
            let destination = if is_remote_url(target_raw) {
                url::Url::parse(target_raw).map_err(|e| {
                    expression_other(
                        function,
                        format!("{function}() invalid URL {target_raw:?}: {e}"),
                    )
                })?;
                target_raw.to_string()
            } else {
                let path = resolve_path_shape(function, target_raw, ctx)?;
                portable_destination(function, &path)?
            };
            Ok(Some(LinkParts { label, destination }))
        }
    }
}

/// `link(file)` / `link(target, desc)` — emits a Markdown inline link.
///
/// - One argument: resolves a local file and uses `relative(file)` as the link
///   text and the resolved absolute path as the destination. HTTP(S) URLs are
///   rejected because a description is required.
/// - Two arguments: `target` may be a local file reference or an HTTP(S) URL;
///   `desc` must be a string and is used as the link text.
pub fn link_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(resolve_link_parts("link", call, ctx)?.map_or(Value::Null, |parts| {
        Value::String(format_markdown_link(&parts.label, &parts.destination))
    }))
}

/// `code_link(file)` / `code_link(target, desc)` — emits a Markdown inline link
/// whose text is inline code, such as `` [`plans/foo.md`](/abs/plans/foo.md) ``.
///
/// Arguments, null handling, destinations, and errors are `link()`'s; only the
/// label differs (see [`format_markdown_code_link`]).
pub fn code_link_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(resolve_link_parts("code_link", call, ctx)?.map_or(Value::Null, |parts| {
        Value::String(format_markdown_code_link(&parts.label, &parts.destination))
    }))
}

/// Validates that a skill name is a single basename component.
pub(super) fn validate_skill_name(name: &str) -> bool {
    let path = Path::new(name);
    let mut components = path.components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

/// Checks whether a direct child directory named `name` exists under any root.
pub(super) fn skill_exists_in_roots(roots: &[PathBuf], name: &str) -> bool {
    roots.iter().any(|root| root.join(name).is_dir())
}

/// `has_skill(name)` — `true` when a direct child directory named `name` exists
/// in any user-scoped or local-scoped skill root for the executing agent;
/// `null` is `false`.
pub fn has_skill_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(name) = call.args().text(0) else {
        return Ok(Value::Bool(false));
    };
    // `ctx.area` is `""` at a monorepo root, so no skill is the answer, not an error.
    if name.is_empty() {
        return Ok(Value::Bool(false));
    }
    if !validate_skill_name(name) {
        return Err(expression_other(
            "has_skill",
            "has_skill() skill name must be a basename without path separators".to_string(),
        ));
    }
    let agent = ctx.agent();
    let home_dir = ctx.home_dir().unwrap_or_else(|| PathBuf::from("."));
    let local_root = ctx
        .file_resolution_context
        .repository_root()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    let roots = SkillRoots::new(home_dir, local_root).roots_for_agent(&agent);
    Ok(Value::Bool(skill_exists_in_roots(&roots, name)))
}

/// `has_local_skill(name)` — `true` when a direct child directory named `name`
/// exists in any local-scoped skill root for the executing agent; `null` is
/// `false`.
pub fn has_local_skill_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(name) = call.args().text(0) else {
        return Ok(Value::Bool(false));
    };
    // `ctx.area` is `""` at a monorepo root, so no skill is the answer, not an error.
    if name.is_empty() {
        return Ok(Value::Bool(false));
    }
    if !validate_skill_name(name) {
        return Err(expression_other(
            "has_local_skill",
            "has_local_skill() skill name must be a basename without path separators"
                .to_string(),
        ));
    }
    let agent = ctx.agent();
    let local_root = ctx
        .file_resolution_context
        .repository_root()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    let roots = SkillRoots::new(PathBuf::from("."), local_root).local_roots_for_agent(&agent);
    Ok(Value::Bool(skill_exists_in_roots(&roots, name)))
}

/// Loads a Markdown file via the resolution context. `Err` if the path is
/// invalid or unreadable.
pub(super) fn load_markdown(
    raw: &str,
    ctx: &ResolutionContext,
    fname: &'static str,
) -> Result<Markdown, ExpressionError> {
    // HTTP(S) arguments read from the run's remote-fetch cache instead of disk,
    // mirroring how `::file`/`::code` directives resolve remote targets.
    if is_remote_url(raw) {
        return match ctx
            .fetch_remote_text(raw)
            .map_err(|message| expression_other(fname, message))?
        {
            Some(body) => Markdown::try_from_content(body).map_err(|e| {
                expression_other(fname, format!("{fname}() failed to parse {raw:?}: {e}"))
            }),
            None => Err(file_reference_error(
                fname,
                raw,
                ctx,
                FileRefFailure::RemoteNotEnabled,
                None,
            )),
        };
    }
    let path = resolve_arg(fname, raw, ctx)?.ok_or_else(|| {
        file_reference_error(fname, raw, ctx, FileRefFailure::NotFound, None)
    })?;
    // A reference that resolved but cannot be read (permissions, a race delete)
    // is still a file-reference failure: surface it through the shared
    // diagnostic carrying the typed `Io` cause, not an opaque `Other` string, so
    // it renders identically to a missing reference and `frontmatter()` /
    // `markdown_title()` / siblings all inherit the same report.
    let content = std::fs::read_to_string(&path).map_err(|e| {
        file_reference_error(
            fname,
            raw,
            ctx,
            FileRefFailure::NotFound,
            Some(biscuit_file::FileReferenceError::Io {
                path: path.clone(),
                source: e,
            }),
        )
    })?;
    Markdown::try_from_content(content)
        .map_err(|e| expression_other(fname, format!("{fname}() failed to parse {raw:?}: {e}")))
}

/// `frontmatter(file)` → object; `frontmatter(file, prop)` → value | null.
pub fn frontmatter_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let md = load_markdown(raw, ctx, "frontmatter")?;
    let map = md.frontmatter().as_map();
    match call.args().text(1) {
        None => {
            let obj: serde_json::Map<String, Value> =
                map.iter().map(|(k, v)| (k.clone(), stored_value(v))).collect();
            Ok(Value::Object(obj))
        }
        Some(prop) => Ok(map.get(prop).map(stored_value).unwrap_or(Value::Null)),
    }
}

/// `try_frontmatter(file) -> { error, ok, value }` — `frontmatter(file)` as a
/// value instead of a failure.
///
/// A file that reads and parses gives `{ ok: true, value: <frontmatter>,
/// error: null }`. A file that is missing, unreadable, or whose frontmatter
/// does not parse (invalid YAML, a duplicate key) gives `{ ok: false, value:
/// null, error: <message> }`, so a document can report the problem instead of
/// failing its composition. Parsing is `frontmatter()`'s: an opening fence
/// that never closes is body text, which reads as empty frontmatter.
pub fn try_frontmatter_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let (ok, value, error) = match load_markdown(raw, ctx, "try_frontmatter") {
        Ok(md) => {
            let object: serde_json::Map<String, Value> = md
                .frontmatter()
                .as_map()
                .iter()
                .map(|(key, value)| (key.clone(), stored_value(value)))
                .collect();
            (true, Value::Object(object), Value::Null)
        }
        Err(error) => (false, Value::Null, Value::String(error.to_string())),
    };
    let mut outcome = serde_json::Map::new();
    outcome.insert("error".to_string(), error);
    outcome.insert("ok".to_string(), Value::Bool(ok));
    outcome.insert("value".to_string(), value);
    Ok(Value::Object(outcome))
}

/// `find_files(pattern) -> file[] | Error` — every file a glob reference
/// matches, as absolute portable paths in native order.
///
/// The pattern is a [`GlobReference`](biscuit_file::GlobReference) authored
/// in this document: its prefix (`&`, `^`, `@`, `~`, `./`, bare, …) picks the
/// roots single-file resolution uses for that sigil, and the result merges
/// every root's matches, most local root first, then shallowest, then
/// component-wise. A bare pattern searches the document's folder, then the
/// repository root. `*` stays within one segment and `**` crosses segments;
/// a pattern without a wildcard names one path under each root. No hidden,
/// ignored, or underscore filter applies, so a `&**/…` walk from the
/// repository root also visits build output.
///
/// Directory symlinks are not followed. A file symlink a bare, `./`, or `../`
/// pattern matches whose target leaves the file tree is left out and reported
/// as a [`GLOB_SKIPPED_SYMLINK_CODE`](crate::markdown::compose::ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE)
/// warning. A `null` pattern lists nothing; a pattern that cannot be parsed or
/// rooted is an [`ExpressionError::GlobReference`].
pub fn find_files_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Array(Vec::new()));
    };
    let glob_error = |source| ExpressionError::GlobReference {
        function: "find_files",
        source: Arc::new(source),
    };
    // A URL keeps its `//`, so the glob reference rejects it as remote.
    let pattern = if is_remote_url(raw) { raw.to_string() } else { normalize_path_arg(raw) };
    let globs = biscuit_file::GlobReference::new([pattern]).map_err(glob_error)?;
    let listing = globs.list_files(&ctx.file_context()).map_err(glob_error)?;
    ctx.glob_warnings.record("interpolation", &listing);
    Ok(Value::Array(
        listing
            .matches
            .iter()
            .map(|path| Value::String(biscuit_file::to_portable_string(path)))
            .collect(),
    ))
}

/// A value read from another document's frontmatter, with stored literal
/// tokens decoded to the text they hold. The result is expression output, so
/// it is data either way; a malformed token stays raw, as every other reader
/// keeps it, rather than failing the reading expression.
pub(super) fn stored_value(value: &Value) -> Value {
    crate::markdown::literal_token::decode_literal_tokens(value)
        .unwrap_or_else(|_| value.clone())
}

/// `markdown_body_empty(file) -> bool | Error` — body has only whitespace;
/// `null` is `false`.
pub fn markdown_body_empty_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Bool(false));
    };
    let md = load_markdown(raw, ctx, "markdown_body_empty")?;
    Ok(Value::Bool(md.content().trim().is_empty()))
}

/// `markdown_title(file) -> string | null | Error` — frontmatter `title`,
/// else first H1. Multiple H1s: first wins, warning to STDERR.
pub fn markdown_title_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let md = load_markdown(raw, ctx, "markdown_title")?;
    if let Some(Value::String(t)) = md.frontmatter().as_map().get("title").map(stored_value) {
        return Ok(Value::String(t));
    }
    let h1s: Vec<String> = md
        .content()
        .lines()
        .filter_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string()))
        .collect();
    match h1s.as_slice() {
        [] => Ok(Value::Null),
        [single] => Ok(Value::String(single.clone())),
        [first, ..] => {
            eprintln!("markdown_title(): multiple H1 headings in {raw:?}; using the first");
            Ok(Value::String(first.clone()))
        }
    }
}

/// `validate_schema(file)` / `validate_schema(file, obj)` -> bool | Error —
/// whether a document's frontmatter is valid against its declared schema.
///
/// The two-argument form validates the combined page: `obj` merged over the
/// document's frontmatter with the explicit-override merge `::file set=`
/// uses (nested objects merge; supplied values, arrays, and explicit `null`
/// replace). The combined page exists only in memory, so neither the file nor
/// `obj` changes, and an empty `obj` answers exactly as the one-argument form.
/// A page (combined or not) that declares no `$schema` is valid; `null` for
/// `file` passes through.
///
/// ## Errors
///
/// A file that cannot be loaded keeps its file-reference error, and a schema
/// that cannot be resolved or compiled is an [`ExpressionError::Other`]; an
/// invalid page is `false`, never an error.
pub fn validate_schema_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let md = load_markdown(raw, ctx, "validate_schema")?;
    let combined = match call.args().object(1) {
        Some(overrides) if !overrides.is_empty() => {
            let base = Value::Object(md.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect());
            match crate::markdown::compose::context::effective_state::deep_merge_override(
                &base,
                &Value::Object(overrides.clone()),
            ) {
                Value::Object(merged) => Some(Markdown::with_frontmatter(
                    crate::markdown::Frontmatter::from_map(merged.into_iter().collect()),
                    md.content(),
                )),
                // An object merged over an object is always an object.
                _ => None,
            }
        }
        _ => None,
    };
    let page = combined.as_ref().unwrap_or(&md);
    if !page.frontmatter().as_map().contains_key("$schema") {
        return Ok(Value::Bool(true));
    }
    let schemas = DarkmatterSchemas::new(ctx.file_context());
    let report = schemas
        .validate(page)
        .map_err(|e| expression_other(
            "validate_schema",
            format!("validate_schema() error for {raw:?}: {e}"),
        ))?;
    Ok(Value::Bool(report.valid))
}

