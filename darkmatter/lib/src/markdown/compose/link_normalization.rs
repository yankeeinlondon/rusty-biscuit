//! Link Normalization operation for the compose pipeline.
//!
//! Converts absolute paths back to portable forms in the Finalization stage.
//! Runs only on the root document.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeSource};
use crate::markdown::reference::{
    ReferenceKind, ReferenceTarget,
    html::{
        extract_html_audio, extract_html_iframes, extract_html_images, extract_html_link_tags,
        extract_html_links, extract_html_sources, extract_html_videos,
    },
    local::{extract_markdown_images, extract_markdown_links},
};
use crate::markdown::types::MarkdownResult;
use biscuit_file::{to_portable_string, try_portable_string};

/// A Windows-prefix-agnostic path identity for `starts_with`, `strip_prefix`,
/// and relative-path arithmetic.
///
/// Path identity and path rendering are different contracts, and this is the
/// identity one: a key is never emitted as document text. A repository root
/// short enough for `dunce` to reduce can hold a descendant that is too long
/// for it, so routing either operand through [`to_portable_string`] would spell
/// the two differently and silently stop normalizing a path that really is
/// inside the repository.
///
/// Keys compare equal across the spellings of one location: `C:\x` and
/// `\\?\C:\x` (a drive letter is case-insensitive), `\\server\share\x` and
/// `\\?\UNC\server\share\x`. Device (`\\.\`) and unrecognized verbatim
/// (`\\?\Volume{…}`) namespaces keep their own prefix text, because neither has
/// an equivalent spelling in another namespace.
///
/// Only the drive letter is case-folded. Windows compares whole paths
/// case-insensitively, so a root recorded as `C:\repo` still fails to match a
/// destination canonicalized as `C:\Repo\…`. That gap predates this key and is
/// deliberately not widened here.
///
/// The suffix is split into components out of the raw platform units, and a key
/// is never turned back into a [`Path`]. Two separate reasons, both
/// load-bearing:
///
/// - [`Path::components`] drops a `.` and reads `..` as a parent hop. Under a
///   verbatim prefix both are ordinary directory names, so re-parsing would let
///   `strip_prefix` hand back a remainder naming a different file.
/// - [`OsStr`] is not UTF-8 on Windows. A key built through `to_string_lossy`
///   maps two paths differing only in unpaired surrogates onto one
///   U+FFFD-bearing identity, which is enough for the wrong anchor to match and
///   for a destination to be rewritten as some other path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ComparisonKey {
    /// The namespace-independent root: `""`, `"C:"`, or `\\server\share`.
    root: OsString,
    /// Whether a separator follows the root, separating `C:\a` from `C:a`.
    rooted: bool,
    components: Vec<OsString>,
}

impl ComparisonKey {
    fn starts_with(&self, base: &ComparisonKey) -> bool {
        self.root == base.root
            && self.rooted == base.rooted
            && base.components.len() <= self.components.len()
            && self.components[..base.components.len()] == base.components[..]
    }

    fn strip_prefix(&self, base: &ComparisonKey) -> Option<&[OsString]> {
        self.starts_with(base)
            .then(|| &self.components[base.components.len()..])
    }

    fn parent(&self) -> ComparisonKey {
        let mut parent = self.clone();
        parent.components.pop();
        parent
    }
}

#[cfg(windows)]
fn comparison_key(path: &Path) -> ComparisonKey {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Component, Prefix};

    const SEPARATOR: u16 = b'\\' as u16;
    const ALT_SEPARATOR: u16 = b'/' as u16;

    let units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let (root, verbatim, prefix_units) = match path.components().next() {
        Some(Component::Prefix(prefix)) => {
            let consumed = prefix.as_os_str().encode_wide().count();
            let (root, verbatim) = match prefix.kind() {
                Prefix::Disk(drive) => (drive_root(drive), false),
                Prefix::VerbatimDisk(drive) => (drive_root(drive), true),
                Prefix::UNC(server, share) => (unc_root(server, share), false),
                Prefix::VerbatimUNC(server, share) => (unc_root(server, share), true),
                Prefix::DeviceNS(_) => (prefix.as_os_str().to_os_string(), false),
                Prefix::Verbatim(_) => (prefix.as_os_str().to_os_string(), true),
            };
            (root, verbatim, consumed)
        }
        _ => (OsString::new(), false, 0),
    };

    let is_separator =
        |unit: u16| unit == SEPARATOR || (!verbatim && unit == ALT_SEPARATOR);
    let rooted = units.get(prefix_units).copied().is_some_and(is_separator);
    let components = units[prefix_units..]
        .split(|unit| is_separator(*unit))
        .filter(|segment| !segment.is_empty())
        .map(OsString::from_wide)
        // A `.` is a self-reference only outside a verbatim namespace; inside
        // one it is a directory whose name happens to be a dot.
        .filter(|segment| verbatim || segment != ".")
        .collect();

    ComparisonKey {
        root,
        rooted,
        components,
    }
}

#[cfg(windows)]
fn drive_root(drive: u8) -> OsString {
    OsString::from(format!("{}:", drive.to_ascii_uppercase() as char))
}

#[cfg(windows)]
fn unc_root(server: &OsStr, share: &OsStr) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let mut units: Vec<u16> = r"\\".encode_utf16().collect();
    units.extend(server.encode_wide());
    units.push(b'\\' as u16);
    units.extend(share.encode_wide());
    OsString::from_wide(&units)
}

/// Off Windows there is no namespace to be agnostic about, and
/// [`Path::components`] is both faithful and non-lossy: `/` is the only
/// separator and `.` is never a literal name.
#[cfg(not(windows))]
fn comparison_key(path: &Path) -> ComparisonKey {
    use std::path::Component;

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
            other => components.push(other.as_os_str().to_os_string()),
        }
    }

    ComparisonKey {
        root: OsString::new(),
        rooted: path.has_root(),
        components,
    }
}

/// Whether `component` still names the same thing once a Windows namespace
/// prefix is dropped.
///
/// [`try_portable_string`] declining answers a question about the *whole*
/// path, and an anchored replacement asks a narrower one: it emits only the
/// names below the anchor, without any prefix. A descendant that merely exceeds
/// `MAX_PATH` becomes a short, ordinary relative path and stays eligible, while
/// a literal `.`, `..`, reserved DOS name, or trailing dot or space is re-read
/// as something else the moment it is spelled without `\\?\`.
///
/// The rules mirror `dunce`'s own component checks, minus its whole-path length
/// test, so the two cannot disagree about what a legacy spelling preserves.
#[cfg(windows)]
fn survives_namespace_removal(components: &[OsString]) -> bool {
    components.iter().all(|component| {
        // Non-Unicode is legal on disk but cannot be written into a document
        // without U+FFFD substitution, so it is never a faithful replacement.
        let Some(name) = component.to_str() else {
            return false;
        };
        if name == "." || name == ".." || name.chars().count() > 255 {
            return false;
        }
        if name.ends_with('.') || name.ends_with(' ') {
            return false;
        }
        if name
            .bytes()
            .any(|byte| matches!(byte, 0..=31 | b'<' | b'>' | b':' | b'"' | b'/' | b'\\' | b'|' | b'?' | b'*'))
        {
            return false;
        }
        !is_reserved_dos_name(name)
    })
}

/// `CON`, `con.txt`, and `con.. .txt` are all the DOS console device.
#[cfg(windows)]
fn is_reserved_dos_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "AUX", "NUL", "PRN", "CON", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let Some(stem) = Path::new(name).file_stem().and_then(OsStr::to_str) else {
        return false;
    };
    let stem = stem.trim_end_matches([' ', '.']);
    stem.len() <= 4 && RESERVED.iter().any(|name| stem.eq_ignore_ascii_case(name))
}

/// Off Windows the guard's other operand is always false, because
/// [`try_portable_string`] never declines there.
#[cfg(not(windows))]
fn survives_namespace_removal(_components: &[OsString]) -> bool {
    true
}

/// Renders anchor-relative components as portable text, or `""` when the
/// destination *is* the anchor.
///
/// The components carry no prefix by construction, so [`to_portable_string`]
/// cannot decline here.
fn render_components(components: &[OsString]) -> String {
    let mut path = PathBuf::new();
    for component in components {
        path.push(component);
    }
    to_portable_string(&path)
}

/// Renders `up` parent hops followed by `forward` names, as
/// [`compute_relative_path`] returns them.
fn render_relative(up: usize, forward: &[OsString]) -> String {
    let mut components: Vec<OsString> = vec![OsString::from(".."); up];
    components.extend_from_slice(forward);
    let rendered = render_components(&components);
    if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    }
}

/// Normalizes absolute path links back into portable forms.
///
/// This operation is the inverse of [`link_resolve`](super::link_resolve::link_resolve).
/// It runs during the Finalization phase on the root document only.
///
/// Rules applied in order:
/// 1. **Same-repo**: If path is inside the same git repo as the document, make it relative.
/// 2. **Home-dir**: If path is under HOME, use `~/` prefix.
/// 3. **ENV-var**: If path is under a whitelisted environment variable, use `${VAR}/` prefix.
///
/// A destination with no faithful portable spelling (a Windows UNC, device, or
/// unreducible verbatim path) is left byte-identical and reported as a warning,
/// in two cases: no anchor applied, or an anchor applied but dropping the
/// namespace prefix would not have preserved every remaining component. This
/// stage runs after transclusion, so preserving authored text cannot retarget
/// the link — unlike [`link_resolve`](super::link_resolve::link_resolve), which
/// errors instead.
///
/// Anchoring is tried before either check, because a declined absolute spelling
/// that is still inside the repository usually *does* have a safe relative form
/// — an over-`MAX_PATH` descendant of a short root is the motivating case — and
/// must be normalized rather than warned about.
pub fn normalize_links(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let source = options.source.clone();
    let content = markdown.content();

    // 3.5 Extract absolute path references
    let mut records = Vec::new();
    records.extend(extract_markdown_links(content, &source));
    records.extend(extract_markdown_images(content, &source));
    records.extend(extract_html_links(content, &source));
    records.extend(extract_html_images(content, &source));
    records.extend(extract_html_videos(content, &source));
    records.extend(extract_html_audio(content, &source));
    records.extend(extract_html_sources(content, &source));
    records.extend(extract_html_iframes(content, &source));
    records.extend(extract_html_link_tags(content, &source));
    records.extend(crate::markdown::reference::html::extract_html_script_blocks(content, &source));

    let mut to_normalize = Vec::new();

    for record in records {
        let mut abs_path = None;
        let mut raw_abs = None;
        if let ReferenceTarget::RemoteUrl { .. } = &record.target {
            continue;
        }
        if let ReferenceTarget::LocalPath { raw } = &record.target {
            if raw.is_absolute() {
                raw_abs = Some(raw.clone());
            } else if let ComposeSource::File(path) = &source
                && let Some(parent) = path.parent()
            {
                let joined = parent.join(raw);
                raw_abs = std::fs::canonicalize(&joined).ok().or(Some(joined));
            }
        }
        if let Some(raw) = raw_abs
            && raw.is_absolute()
        {
            abs_path = Some(raw.clone());
        }

        if let Some(abs_path) = abs_path {
            match record.kind {
                ReferenceKind::Hyperlink
                | ReferenceKind::Image
                | ReferenceKind::HtmlVideo
                | ReferenceKind::HtmlAudio
                | ReferenceKind::HtmlSource
                | ReferenceKind::HtmlIframe
                | ReferenceKind::ScriptImport
                | ReferenceKind::CssImport
                | ReferenceKind::FontImport => {
                    to_normalize.push((record, abs_path));
                }
                _ => {}
            }
        }
    }

    if to_normalize.is_empty() {
        return Ok(());
    }

    // Sort by span start descending for safe in-place replacement
    to_normalize.sort_by_key(|(r, _)| std::cmp::Reverse(r.origin.span.start));

    let mut new_content = content.to_string();
    let mut applied_count = 0;

    let base_file = match &source {
        ComposeSource::File(path) => match std::fs::canonicalize(path) {
            Ok(p) => Some(p),
            Err(e) => {
                report.add_warning(crate::markdown::compose::ComposeWarning::new(
                    "link_normalization",
                    format!(
                        "Failed to canonicalize source path '{}': {}",
                        path.display(),
                        e
                    ),
                ));
                return Ok(());
            }
        },
        _ => None,
    };

    let base_dir = base_file
        .as_ref()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let base_file = base_file.as_deref().map(comparison_key);
    let git_root = match options.file_resolution_context.as_ref() {
        Some(context) => context.repository_root().map(Path::to_path_buf),
        None => base_dir.as_ref().and_then(|d| super::find_git_root_from(d)),
    }
    .map(|r| std::fs::canonicalize(&r).unwrap_or(r))
    .map(|r| comparison_key(&r));
    let home = match options.file_resolution_context.as_ref() {
        Some(context) => context.home_dir().map(Path::to_path_buf),
        None => dirs::home_dir(),
    }
    .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
    .map(|path| comparison_key(&path));

    for (record, abs_path) in to_normalize {
        let resolved_abs = std::fs::canonicalize(&abs_path).unwrap_or_else(|_| abs_path.clone());
        let comparable_abs = comparison_key(&resolved_abs);
        // Every anchor arm emits the destination without its namespace prefix.
        // When the whole path had no faithful portable spelling, that removal
        // has to be proved component by component before any arm may write text.
        let namespace_declined = try_portable_string(&resolved_abs).is_none();
        let audit =
            |components: &[OsString]| !namespace_declined || survives_namespace_removal(components);
        let mut replacement = None;
        let mut anchor_rejected = false;

        // 3.6 Same-repo rule
        if let Some(ref repo) = git_root
            && comparable_abs.starts_with(repo)
            && let Some(ref doc_path) = base_file
        {
            let (up, forward) = compute_relative_path(doc_path, &comparable_abs);

            // Only `forward` is audited: the `..` hops are this pipeline's own
            // parent navigation, not names copied out of the destination.
            if audit(&forward) {
                replacement = Some(render_relative(up, &forward));
            } else {
                anchor_rejected = true;
            }
        }

        if replacement.is_none()
            && !anchor_rejected
            && let Some(ref h) = home
            && let Some(rel) = comparable_abs.strip_prefix(h)
        {
            if audit(rel) {
                replacement = Some(format!("~/{}", render_components(rel)));
            } else {
                anchor_rejected = true;
            }
        }

        // 3.8 ENV-var rule
        if replacement.is_none() && !anchor_rejected {
            let whitelist = options.effective_env_path_whitelist();
            let mut best_var = None;
            let mut longest_len = 0;

            for var_name in whitelist {
                let val = match options.file_resolution_context.as_ref() {
                    Some(context) => context.env().get(&var_name).cloned(),
                    None => std::env::var(&var_name).ok(),
                };
                if let Some(val) = val {
                    let var_path = PathBuf::from(val);
                    let var_path = std::fs::canonicalize(&var_path).unwrap_or(var_path);
                    let var_path = comparison_key(&var_path);
                    if comparable_abs.starts_with(&var_path) {
                        let depth = var_path.components.len();
                        if best_var.is_none() || depth > longest_len {
                            longest_len = depth;
                            best_var = Some((var_name, var_path));
                        }
                    }
                }
            }

            if let Some((var_name, var_path)) = best_var
                && let Some(rel) = comparable_abs.strip_prefix(&var_path)
            {
                if audit(rel) {
                    // 3.9 Emit warning
                    let msg = format!(
                        "the path <blue>{}</blue> was found to be an offset of the <b>{}</b> environment variable and will use this abstraction.",
                        abs_path.display(),
                        var_name
                    );
                    report.add_warning(crate::markdown::compose::ComposeWarning::new(
                        "link_normalization",
                        msg,
                    ));

                    replacement = Some(format!("${{{}}}/{}", var_name, render_components(rel)));
                } else {
                    anchor_rejected = true;
                }
            }
        }

        if replacement.is_none() {
            if anchor_rejected {
                report.add_warning(crate::markdown::compose::ComposeWarning::new(
                    "link_normalization",
                    format!(
                        "the destination <blue>{}</blue> anchors inside the repository, home, or an environment variable, but writing it without its Windows namespace prefix would not preserve every component; it was left exactly as authored.",
                        abs_path.display()
                    ),
                ));
            } else if namespace_declined || try_portable_string(&abs_path).is_none() {
                report.add_warning(crate::markdown::compose::ComposeWarning::new(
                    "link_normalization",
                    format!(
                        "the destination <blue>{}</blue> has no faithful portable spelling and no repository, home, or environment anchor applies; it was left exactly as authored.",
                        abs_path.display()
                    ),
                ));
            }
        }

        if let Some(new_target) = replacement
            && let Some((start, end)) =
                super::find_target_range(&new_content, &record, &abs_path.to_string_lossy())
        {
            new_content.replace_range(start..end, &new_target);
            applied_count += 1;
        }
    }

    report.link_normalizations_applied += applied_count;
    if applied_count > 0 {
        *markdown.content_mut() = new_content;
    }

    Ok(())
}

/// Splits the route from `from`'s directory to `to` into parent hops and the
/// names below the point where the two diverge.
///
/// The halves are returned separately rather than pre-joined because only
/// `forward` carries names copied out of the destination; that is the slice
/// [`survives_namespace_removal`] must audit, and a joined path would leave the
/// generated `..` hops indistinguishable from a literal `..` directory name.
fn compute_relative_path(
    from: &ComparisonKey,
    to: &ComparisonKey,
) -> (usize, Vec<OsString>) {
    let from = strip_macos_private(from);
    let to = strip_macos_private(to);

    let from_dir = if from
        .components
        .last()
        .is_some_and(|name| Path::new(name).extension().is_some())
    {
        from.parent()
    } else {
        from
    };

    let common = from_dir
        .components
        .iter()
        .zip(to.components.iter())
        .take_while(|(base, target)| base == target)
        .count();

    (
        from_dir.components.len() - common,
        to.components[common..].to_vec(),
    )
}

/// macOS canonicalizes `/tmp` and `/var` under `/private`, so a document and
/// its destination can disagree about that leading component depending on which
/// of them was canonicalized. Dropping it keeps the two comparable.
fn strip_macos_private(key: &ComparisonKey) -> ComparisonKey {
    let mut key = key.clone();
    if key.rooted
        && key.components.len() > 1
        && key.components.first().is_some_and(|first| first == "private")
    {
        key.components.remove(0);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeReport;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_links_same_repo() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();
        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();
        let target_file = assets.join("image.png");
        fs::write(&target_file, "png").unwrap();
        let abs_path = std::fs::canonicalize(&target_file).unwrap();
        let content = format!("![img]({})\n", biscuit_file::to_portable_string(&abs_path));
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("../assets/image.png"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_home_dir() {
        let home = dirs::home_dir().expect("Has home dir");
        let target = home.join("some_file.txt");
        let content = format!("[file]({})\n", biscuit_file::to_portable_string(&target));
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new();
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("~/some_file.txt"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_env_var() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().join("project");
        fs::create_dir_all(&project_root).unwrap();
        let target = project_root.join("config.json");
        fs::write(&target, "{}").unwrap();
        let abs_path = match std::fs::canonicalize(&target) {
            Ok(p) => {
                if p.to_string_lossy().starts_with("/private/") {
                    PathBuf::from(&p.to_string_lossy()[8..])
                } else {
                    p
                }
            }
            Err(_) => target.clone(),
        };
        let canonical_root = std::fs::canonicalize(&project_root).unwrap();
        let mut env = std::collections::HashMap::new();
        env.insert(
            "PROJECT_ROOT".to_string(),
            canonical_root.to_string_lossy().into_owned(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(&project_root)
            .without_home_dir()
            .with_env(env);
        let content = format!(
            "<a href=\"{}\">config</a>\n",
            biscuit_file::to_portable_string(&abs_path)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["PROJECT_ROOT".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("${PROJECT_ROOT}/config.json"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn link_normalization_reuses_snapshot_environment() {
        let dir = tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let target = root.join("config.json");
        fs::write(&target, "{}").unwrap();
        let content = format!(
            "[config]({})\n",
            biscuit_file::to_portable_string(&target)
        );
        let mut md = Markdown::new(&content);
        let mut env = std::collections::HashMap::new();
        env.insert("CAPTURED_ROOT".to_string(), root.display().to_string());
        let snapshot = biscuit_file::FileResolutionContext::new(&root)
            .without_home_dir()
            .with_env(env);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["CAPTURED_ROOT".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(md.content().contains("${CAPTURED_ROOT}/config.json"));
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_css_font_script() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_css = assets.join("styles.css");
        let target_font = assets.join("font.woff2");
        let target_script = assets.join("app.js");
        fs::write(&target_css, "").unwrap();
        fs::write(&target_font, "").unwrap();
        fs::write(&target_script, "").unwrap();

        let abs_css = std::fs::canonicalize(&target_css).unwrap();
        let abs_font = std::fs::canonicalize(&target_font).unwrap();
        let abs_script = std::fs::canonicalize(&target_script).unwrap();

        let content = format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n<link rel=\"preload\" as=\"font\" href=\"{}\">\n<script src=\"{}\"></script>",
            biscuit_file::to_portable_string(&abs_css),
            biscuit_file::to_portable_string(&abs_font),
            biscuit_file::to_portable_string(&abs_script)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("../assets/styles.css"),
            "CSS failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("../assets/font.woff2"),
            "Font failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("../assets/app.js"),
            "Script failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn test_normalize_links_deep_nesting() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();

        let docs = repo.join("docs").join("deep").join("nested").join("dir");
        let assets = repo.join("assets").join("images");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_file = assets.join("image.png");
        fs::write(&target_file, "png").unwrap();

        let same_dir_file = docs.join("sibling.md");
        fs::write(&same_dir_file, "md").unwrap();

        let abs_img = std::fs::canonicalize(&target_file).unwrap();
        let abs_sibling = std::fs::canonicalize(&same_dir_file).unwrap();

        let content = format!(
            "[img]({})\n[sibling]({})",
            biscuit_file::to_portable_string(&abs_img),
            biscuit_file::to_portable_string(&abs_sibling)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(md.content().contains("../../../../assets/images/image.png"));
        assert!(md.content().contains("sibling.md") || md.content().contains("./sibling.md"));
        assert_eq!(report.link_normalizations_applied, 2);
    }

    #[test]
    fn test_normalize_links_env_var_specificity() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        let target = child.join("config.json");
        fs::write(&target, "{}").unwrap();

        let abs_path = std::fs::canonicalize(&target).unwrap_or(target);
        let abs_parent = std::fs::canonicalize(&parent).unwrap_or(parent);
        let abs_child = std::fs::canonicalize(&child).unwrap_or_else(|_| child.clone());

        let mut env = std::collections::HashMap::new();
        env.insert(
            "USER".to_string(),
            abs_parent.to_string_lossy().into_owned(),
        );
        env.insert(
            "USER_NAME".to_string(),
            abs_child.to_string_lossy().into_owned(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(&child)
            .without_home_dir()
            .with_env(env);

        let content = format!(
            "[config]({})",
            biscuit_file::to_portable_string(&abs_path)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["USER".to_string(), "USER_NAME".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        // Should use the longer match USER_NAME
        assert!(
            md.content().contains("${USER_NAME}/config.json"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_edge_cases() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();

        let source_file = repo.join("source.md");
        fs::write(&source_file, "").unwrap();

        let parens_file = repo.join("with (parens).md");
        let quotes_file = repo.join("single_quotes.md");
        fs::write(&parens_file, "").unwrap();
        fs::write(&quotes_file, "").unwrap();

        let abs_parens = std::fs::canonicalize(&parens_file).unwrap();
        let abs_quotes = std::fs::canonicalize(&quotes_file).unwrap();

        let content = format!(
            "[link](<{}>)\n<img src='{}'>\n<a href=\"{}\" data-alt='{}'>link</a>",
            biscuit_file::to_portable_string(&abs_parens),
            biscuit_file::to_portable_string(&abs_quotes),
            biscuit_file::to_portable_string(&abs_quotes),
            biscuit_file::to_portable_string(&abs_quotes)
        );

        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("(<with (parens).md>)"),
            "Parens failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("'single_quotes.md'"),
            "Quotes failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"single_quotes.md\""),
            "Mixed failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn test_normalize_links_html_spaced_attributes() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_img = assets.join("image.png");
        let target_video = assets.join("movie.mp4");
        let target_css = assets.join("styles.css");
        fs::write(&target_img, "png").unwrap();
        fs::write(&target_video, "video").unwrap();
        fs::write(&target_css, "body {}").unwrap();

        let abs_img = std::fs::canonicalize(&target_img).unwrap();
        let abs_video = std::fs::canonicalize(&target_video).unwrap();
        let abs_css = std::fs::canonicalize(&target_css).unwrap();

        let content = format!(
            "<a href = \"{}\">link</a>\n<img src = \"{}\">\n<video src = \"{}\"></video>\n<link href = \"{}\">",
            biscuit_file::to_portable_string(&abs_img),
            biscuit_file::to_portable_string(&abs_img),
            biscuit_file::to_portable_string(&abs_video),
            biscuit_file::to_portable_string(&abs_css)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("../assets/image.png"),
            "Spaced anchor href failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/image.png\""),
            "Spaced img src failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/movie.mp4\""),
            "Spaced video src failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/styles.css\""),
            "Spaced link href failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 4);
    }

    /// Both spellings of one share must land on one key, or a document under
    /// `\\server\share` stops normalizing the moment something canonicalizes a
    /// destination into the verbatim namespace.
    ///
    /// Equality alone would not prove the pipeline works, so the prefix tests
    /// and the relative arithmetic run across the two spellings as well. The
    /// pair is not driven through [`normalize_links`] because every anchor arm
    /// canonicalizes, and `canonicalize` against a `\\server\share` path blocks
    /// on SMB name resolution for tens of seconds.
    #[cfg(windows)]
    #[test]
    fn comparison_key_equates_legacy_and_verbatim_unc() {
        assert_eq!(
            comparison_key(Path::new(r"\\server\share\x")),
            comparison_key(Path::new(r"\\?\UNC\server\share\x"))
        );

        let legacy_root = comparison_key(Path::new(r"\\server\share\repo"));
        let verbatim_child =
            comparison_key(Path::new(r"\\?\UNC\server\share\repo\docs\f.md"));
        assert!(verbatim_child.starts_with(&legacy_root));
        assert_eq!(
            verbatim_child.strip_prefix(&legacy_root).map(render_components),
            Some("docs/f.md".to_string())
        );

        let legacy_doc = comparison_key(Path::new(r"\\server\share\repo\assets\a.md"));
        let (up, forward) = compute_relative_path(&legacy_doc, &verbatim_child);
        assert_eq!(render_relative(up, &forward), "../docs/f.md");

        // Equal as identities, declined as text: the pair a UNC destination
        // reaches Finalization with, and the reason equating them cannot be
        // done by rendering both.
        assert!(try_portable_string(Path::new(r"\\server\share\x")).is_none());
        assert!(try_portable_string(Path::new(r"\\?\UNC\server\share\x")).is_none());
    }

    /// A key is an identity, so two paths that differ only in an unpaired
    /// surrogate must not share one.
    ///
    /// `to_string_lossy` maps both onto the same U+FFFD-bearing text, which is
    /// enough for one path to match the other's anchor and be rewritten as a
    /// destination naming a different file.
    #[cfg(windows)]
    #[test]
    fn comparison_key_keeps_unpaired_surrogates_distinct() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let with_trailing_unit = |unit: u16| {
            let units: Vec<u16> = r"C:\repo\".encode_utf16().chain([unit]).collect();
            PathBuf::from(OsString::from_wide(&units))
        };
        let first = with_trailing_unit(0xD800);
        let second = with_trailing_unit(0xD801);

        assert_ne!(first, second);
        assert_eq!(
            first.to_string_lossy(),
            second.to_string_lossy(),
            "fixture must be one a lossy key would collapse, or it proves nothing"
        );

        let first_key = comparison_key(&first);
        let second_key = comparison_key(&second);
        assert_ne!(first_key, second_key);
        assert!(!first_key.starts_with(&second_key));
        assert!(first_key.starts_with(&comparison_key(Path::new(r"C:\repo"))));
    }

    /// A repository root short enough for `dunce` to reduce, holding a
    /// descendant too long for it to reduce.
    ///
    /// This is the case that separates identity from rendering: the descendant
    /// declines portabilization outright, so had either operand been spelled
    /// through `to_portable_string` the prefix test would compare `C:/r`
    /// against `\\?\C:\r\…` and silently refuse to normalize a path that is
    /// genuinely inside the repository.
    #[cfg(windows)]
    #[test]
    fn safe_repo_root_contains_declined_long_verbatim_descendant() {
        let repo = Path::new(r"C:\r");
        let doc = Path::new(r"C:\r\docs\a.md");
        let long_a = "a".repeat(150);
        let long_b = "b".repeat(150);
        let descendant =
            PathBuf::from(format!(r"\\?\C:\r\assets\{long_a}\{long_b}\image.png"));
        assert!(
            descendant.as_os_str().len() > 260,
            "fixture must exceed MAX_PATH or `dunce` would reduce it"
        );

        assert!(try_portable_string(repo).is_some());
        assert!(
            try_portable_string(&descendant).is_none(),
            "the descendant must be one `dunce` declines, or this proves nothing"
        );

        let repo_key = comparison_key(repo);
        let doc_key = comparison_key(doc);
        let descendant_key = comparison_key(&descendant);
        assert!(descendant_key.starts_with(&repo_key));

        let (up, forward) = compute_relative_path(&doc_key, &descendant_key);
        assert!(
            survives_namespace_removal(&forward),
            "length alone must not disqualify an anchored replacement"
        );
        assert_eq!(
            render_relative(up, &forward),
            format!("../assets/{long_a}/{long_b}/image.png")
        );
    }

    /// Each way a verbatim component stops meaning itself once `\\?\` is gone.
    ///
    /// `dunce` declines all of them, and the anchored arms must reach the same
    /// verdict from the components alone — over-`MAX_PATH`, the one decline
    /// that *is* recoverable below an anchor, is pinned above.
    #[cfg(windows)]
    #[test]
    fn unsafe_components_do_not_survive_namespace_removal() {
        for unsafe_name in [".", "..", "CON", "con.txt", "com1", "trailing.", "trailing "] {
            let components = vec![OsString::from(unsafe_name), OsString::from("f.md")];
            assert!(
                !survives_namespace_removal(&components),
                "{unsafe_name} must not be written without its `\\\\?\\` prefix"
            );
        }

        for safe_name in ["assets", "console", "com", "com10", "a.b.c", "..leading"] {
            let components = vec![OsString::from(safe_name), OsString::from("f.md")];
            assert!(
                survives_namespace_removal(&components),
                "{safe_name} is an ordinary name and must stay eligible"
            );
        }
    }

    /// The repository anchor must not become a way around the decline.
    ///
    /// Rewriting `\\?\C:\…\repo\.\assets\image.png` relative to a document
    /// inside the repository drops both the namespace and, if the remainder is
    /// re-parsed as an ordinary Windows path, the literal `.` directory — so
    /// the emitted link names `assets\image.png`, a different location. The
    /// authored text must survive byte-identical instead, with a warning.
    #[cfg(windows)]
    #[test]
    fn anchored_unsafe_verbatim_destination_is_preserved_and_warned() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        // `canonicalize` yields the verbatim spelling on Windows, so the
        // destination below is a genuine descendant of the repository root.
        let verbatim_repo = std::fs::canonicalize(&repo).unwrap();
        let destination = format!(r"{}\.\assets\image.png", verbatim_repo.display());
        assert!(
            try_portable_string(Path::new(&destination)).is_none(),
            "fixture must be one `dunce` declines"
        );

        let content = format!("<img src=\"{destination}\">\n");
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert_eq!(md.content(), content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(
            report.warnings.iter().any(|w| w.stage == "link_normalization"
                && w.message.contains("would not preserve every component")),
            "expected an anchored-preservation warning, got: {:?}",
            report.warnings
        );
    }

    /// The other half of the same rule: an anchored descendant whose only
    /// problem is length still normalizes, end to end.
    #[cfg(windows)]
    #[test]
    fn anchored_over_max_path_destination_still_normalizes() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let verbatim_repo = std::fs::canonicalize(&repo).unwrap();
        let long_name = "a".repeat(250);
        let destination = format!(r"{}\assets\{long_name}\image.png", verbatim_repo.display());
        assert!(
            try_portable_string(Path::new(&destination)).is_none(),
            "fixture must exceed MAX_PATH or it proves nothing"
        );

        let content = format!("<img src=\"{destination}\">\n");
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content()
                .contains(&format!("../assets/{long_name}/image.png")),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    /// The environment anchor takes the same route as the repository one, and
    /// needs its own regression: it reaches `strip_prefix` through a different
    /// arm and, unlike the repository rule, does not require a source file.
    #[cfg(windows)]
    #[test]
    fn env_anchored_unsafe_verbatim_destination_is_preserved_and_warned() {
        let dir = tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let destination = format!(r"{}\.\docs\f.md", root.display());
        assert!(try_portable_string(Path::new(&destination)).is_none());

        let mut env = std::collections::HashMap::new();
        env.insert(
            "PROJECT_ROOT".to_string(),
            root.to_string_lossy().into_owned(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(&root)
            .without_home_dir()
            .with_env(env);

        let content = format!("<a href=\"{destination}\">f</a>\n");
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["PROJECT_ROOT".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert_eq!(md.content(), content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(
            report.warnings.iter().any(|w| w.stage == "link_normalization"
                && w.message.contains("would not preserve every component")),
            "expected an anchored-preservation warning, got: {:?}",
            report.warnings
        );
    }

    /// Finalization runs after transclusion, so an authored destination it
    /// cannot portabilize is left exactly as written and reported — the
    /// warn-and-preserve half of the stage-specific policy whose other half is
    /// `link_resolve`'s error.
    ///
    /// The fixture is an HTML anchor rather than a Markdown destination because
    /// CommonMark consumes the backslash escapes in the latter, so a native
    /// Windows spelling cannot survive being authored there in the first place.
    /// It is a verbatim path rather than the UNC one because `canonicalize`
    /// against `\\server\share` blocks on SMB name resolution for tens of
    /// seconds; UNC's decline is pinned in
    /// [`comparison_key_equates_legacy_and_verbatim_unc`] instead.
    #[cfg(windows)]
    #[test]
    fn declined_absolute_destination_is_preserved_and_warned() {
        let dir = tempdir().unwrap();
        let snapshot = biscuit_file::FileResolutionContext::new(dir.path())
            .without_home_dir()
            .with_env(std::collections::HashMap::new());
        let content = "<a href=\"\\\\?\\C:\\repo\\.\\docs\\f.md\">f</a>\n";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert_eq!(md.content(), content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(
            report.warnings.iter().any(|w| {
                w.stage == "link_normalization" && w.message.contains(r"\\?\C:\repo\.\docs\f.md")
            }),
            "expected a preserved-destination warning, got: {:?}",
            report.warnings
        );
    }

    #[test]
    fn test_normalize_links_preserves_remote_urls() {
        let content = "[link](https://example.com/page) and ![img](http://cdn.example.com/img.png)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new();
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("https://example.com/page"),
            "HTTPS URL was modified. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("http://cdn.example.com/img.png"),
            "HTTP URL was modified. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 0);
    }
}
