//! Link Normalization operation for the compose pipeline.
//!
//! Converts the absolute destinations [`link_resolve`](super::link_resolve)
//! wrote back into portable references in the Finalization stage, through
//! [`biscuit_file::PortablePath`] and its default strategy. Runs only on the
//! root document.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::markdown::Markdown;
use crate::markdown::compose::util::source_link_context;
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeWarning};
use crate::markdown::reference::{
    ReferenceKind, ReferenceTarget,
    html::{
        extract_html_audio, extract_html_iframes, extract_html_images, extract_html_link_tags,
        extract_html_links, extract_html_script_blocks, extract_html_sources,
        extract_html_videos,
    },
    local::{extract_markdown_images, extract_markdown_links},
};
use crate::markdown::types::MarkdownResult;
use biscuit_file::{
    FileResolutionContext, Finding, PathIdentity, PortabilityPreference, PortablePath,
    try_portable_string,
};

const STAGE: &str = "link_normalization";

/// Splits a link destination into its path and the `#fragment`, `?query`, or
/// `:line` / `:line-line` suffix that stays with the link layer.
///
/// `destination` is the parsed destination, never raw Markdown, so the only
/// colon treated as a suffix is a trailing run of line digits: a Windows drive
/// colon (`C:/x.md`) is part of the path. The `?` of a verbatim `\\?\` prefix
/// is not a query.
fn split_suffix(destination: &str) -> (&str, &str) {
    let prefix_len = [r"\\?\", "//?/"]
        .iter()
        .find(|prefix| destination.starts_with(**prefix))
        .map_or(0, |prefix| prefix.len());
    let path_end = destination[prefix_len..]
        .find(['#', '?'])
        .map_or(destination.len(), |index| prefix_len + index);
    let path = &destination[..path_end];
    let line_start = path.rfind(':').filter(|&colon| {
        let mut lines = path[colon + 1..].splitn(2, '-');
        let is_line = |part: Option<&str>| {
            part.is_some_and(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        };
        is_line(lines.next()) && lines.next().is_none_or(|end| is_line(Some(end)))
    });
    let split = line_start.unwrap_or(path_end);
    (&destination[..split], &destination[split..])
}

/// Re-spells `target` under the context's own spelling of an anchor that
/// contains it.
///
/// [`link_resolve`](super::link_resolve) canonicalizes what it writes
/// (`/private/var/…` on macOS), while the context keeps the spelling the
/// request was opened with (`/var/…`). `PortablePath` compares paths
/// lexically, so without this a target inside the document's tree would look
/// unrelated to it.
fn in_context_spelling(target: &Path, ctx: &FileResolutionContext) -> PathBuf {
    let Ok(canonical_target) = std::fs::canonicalize(target) else {
        return target.to_path_buf();
    };
    let canonical_target = PathIdentity::new(&canonical_target);
    let anchors = [Some(ctx.base_dir()), Some(ctx.cwd()), ctx.repository_root(), ctx.home_dir()];
    for anchor in anchors.into_iter().flatten() {
        if PathIdentity::new(target).starts_with(&PathIdentity::new(anchor)) {
            return target.to_path_buf();
        }
        let Ok(canonical_anchor) = std::fs::canonicalize(anchor) else {
            continue;
        };
        if let Some(rest) = canonical_target.strip_prefix(&PathIdentity::new(&canonical_anchor)) {
            return rest.iter().fold(anchor.to_path_buf(), |path, name| path.join(name));
        }
    }
    target.to_path_buf()
}

/// The spelling Darkmatter writes for a `PortablePath` result.
///
/// A composed document is Darkmatter source again, and composing it would
/// evaluate a leading `{{VAR}}` as an expression. The literal form
/// `{{{VAR}}}` composes back to `{{VAR}}`, which link resolution then reads as
/// the environment anchor, so a compose of the output reproduces it.
fn compose_spelling(reference: &str, strategy: &PortabilityPreference) -> String {
    if *strategy == PortabilityPreference::EnvRootedPath
        && let Some(rest) = reference.strip_prefix("{{")
        && let Some((name, tail)) = rest.split_once("}}")
    {
        return format!("{{{{{{{name}}}}}}}{tail}");
    }
    reference.to_string()
}

/// Normalizes absolute link destinations back into portable references.
///
/// This operation is the inverse of [`link_resolve`](super::link_resolve::link_resolve).
/// It runs during the Finalization phase on the root document only, with the
/// root document's file-resolution context, so relative results are relative
/// to the composed document wherever the link was authored.
///
/// Each absolute destination goes through `PortablePath` with the default
/// strategy: a nearby relative link, then the repository root (`&`), a
/// portable environment variable, the home directory (`~`), and the absolute
/// path last. Portable variables are those the request environment's
/// `PORTABLE_ENV_VARIABLES` declares plus
/// [`ComposeOptions::with_portable_env`]; there is no built-in set.
///
/// A `#fragment`, `?query`, or `:line` suffix is split off first and
/// reattached. Relative and sigil destinations are left alone: link
/// resolution already made every destination it could resolve absolute.
///
/// A destination is left byte-identical, with a warning, when evaluation fails
/// or when only the absolute fallback matched and the destination has no
/// faithful portable spelling (a Windows UNC, device, or unreducible verbatim
/// path). This stage runs after transclusion, so preserving authored text
/// cannot retarget the link, unlike
/// [`link_resolve`](super::link_resolve::link_resolve), which errors instead.
pub fn normalize_links(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let source = options.source.clone();
    let content = markdown.content();

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
    records.extend(extract_html_script_blocks(content, &source));

    let mut to_normalize = Vec::new();
    for record in records {
        let ReferenceTarget::LocalPath { raw } = &record.target else {
            continue;
        };
        let Some(destination) = raw.to_str().map(str::to_string) else {
            continue;
        };
        if !Path::new(split_suffix(&destination).0).is_absolute() {
            continue;
        }
        match record.kind {
            ReferenceKind::Hyperlink
            | ReferenceKind::Image
            | ReferenceKind::HtmlVideo
            | ReferenceKind::HtmlAudio
            | ReferenceKind::HtmlSource
            | ReferenceKind::HtmlIframe
            | ReferenceKind::ScriptImport
            | ReferenceKind::CssImport
            | ReferenceKind::FontImport => to_normalize.push((record, destination)),
            _ => {}
        }
    }

    if to_normalize.is_empty() {
        return Ok(());
    }

    // Sort by span start descending for safe in-place replacement
    to_normalize.sort_by_key(|(r, _)| std::cmp::Reverse(r.origin.span.start));

    // One context for the whole document; without one `PortablePath` captures
    // the process directory, home, and environment itself.
    let ctx = source_link_context(options);
    let mut new_content = content.to_string();
    let mut applied_count = 0;
    let mut reported_names = BTreeSet::new();

    for (record, destination) in to_normalize {
        let (path_text, suffix) = split_suffix(&destination);
        let target = match &ctx {
            Some(ctx) => in_context_spelling(Path::new(path_text), ctx),
            None => PathBuf::from(path_text),
        };
        let mut portable =
            PortablePath::from_path(target).with_portable_env(options.portable_env.iter().cloned());
        if let Some(ctx) = &ctx {
            portable = portable.with_ctx(ctx);
        }

        let result = match portable.file_reference() {
            Ok(result) => result,
            Err(error) => {
                report_invalid_names(error.findings(), &mut reported_names, report);
                let headline = error.to_string();
                let headline = headline.lines().next().unwrap_or_default();
                report.add_warning(ComposeWarning::new(
                    STAGE,
                    format!(
                        "the destination <blue>{destination}</blue> was left exactly as authored: {headline}"
                    ),
                ));
                continue;
            }
        };
        report_invalid_names(result.findings(), &mut reported_names, report);

        // The absolute fallback rewrites nothing: the destination already is
        // the absolute path.
        if *result.strategy() == PortabilityPreference::AbsolutePath {
            if try_portable_string(Path::new(path_text)).is_none() {
                report.add_warning(ComposeWarning::new(
                    STAGE,
                    format!(
                        "the destination <blue>{destination}</blue> has no faithful portable spelling and no relative, repository, environment, or home reference reaches it; it was left exactly as authored."
                    ),
                ));
            }
            continue;
        }

        let replacement = format!(
            "{}{suffix}",
            compose_spelling(result.reference().raw(), result.strategy())
        );
        if let Some((start, end)) = super::find_target_range(&new_content, &record, &destination) {
            new_content.replace_range(start..end, &replacement);
            applied_count += 1;
        }
    }

    report.link_normalizations_applied += applied_count;
    if applied_count > 0 {
        *markdown.content_mut() = new_content;
    }

    Ok(())
}

/// Warns once per invalid `PORTABLE_ENV_VARIABLES` / `with_portable_env` name.
///
/// Other findings are not repeated here: a missing or non-file target is
/// reference validation's to report.
fn report_invalid_names(
    findings: &[Finding],
    reported: &mut BTreeSet<String>,
    report: &mut ComposeReport,
) {
    for finding in findings {
        if let Finding::InvalidPortableVariableName { name } = finding
            && reported.insert(name.clone())
        {
            report.add_warning(ComposeWarning::new(
                STAGE,
                format!(
                    "<b>{name}</b> is not a valid portable variable name (A-Z, 0-9, and _ only); it was ignored."
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeReport;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    fn options_with_repo(source: &Path) -> ComposeOptions {
        let repo = source
            .ancestors()
            .find(|path| path.join(".git").exists())
            .expect("test repository root");
        let base = source.parent().expect("source parent");
        let context = biscuit_file::FileResolutionContext::new(base)
            .with_repository_root(repo);
        ComposeOptions::new()
            .with_source_file(source)
            .with_file_resolution_context(context)
    }

    /// A request snapshot outside any repository whose `cwd` is a directory
    /// that is neither the target's nor an ancestor of it, so no relative
    /// preference can claim a target under `root`.
    fn detached_options(root: &Path, env: HashMap<String, String>) -> ComposeOptions {
        let elsewhere = root.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let snapshot = biscuit_file::FileResolutionContext::new(&elsewhere)
            .without_home_dir()
            .with_env(env);
        ComposeOptions::new().with_file_resolution_context(snapshot)
    }

    /// A variable's value as a user would export it. A canonical Windows path
    /// is verbatim (`\\?\C:\…`), and `{{VAR}}/rest` cannot resolve under a
    /// verbatim value (see the `os` skill, Windows path spelling).
    fn env_value(path: &Path) -> String {
        biscuit_file::to_portable_string(path)
    }

    fn normalize(content: &str, options: &ComposeOptions) -> (String, ComposeReport) {
        let mut md = Markdown::new(content);
        let mut report = ComposeReport::new();
        normalize_links(&mut md, options, &mut report).unwrap();
        (md.content().to_string(), report)
    }

    fn link_warnings(report: &ComposeReport) -> Vec<&str> {
        report
            .warnings
            .iter()
            .filter(|warning| warning.stage == STAGE)
            .map(|warning| warning.message.as_str())
            .collect()
    }

    /// A repository with `docs/source.md` and the named files under `assets/`.
    struct RepoFixture {
        _dir: tempfile::TempDir,
        repo: PathBuf,
        source: PathBuf,
    }

    impl RepoFixture {
        fn new(assets: &[&str]) -> Self {
            let dir = tempdir().unwrap();
            let repo = dir.path().join("repo");
            fs::create_dir_all(repo.join(".git")).unwrap();
            fs::create_dir_all(repo.join("docs")).unwrap();
            fs::create_dir_all(repo.join("assets")).unwrap();
            let source = repo.join("docs").join("source.md");
            fs::write(&source, "").unwrap();
            for asset in assets {
                fs::write(repo.join("assets").join(asset), "x").unwrap();
            }
            Self { _dir: dir, repo, source }
        }

        /// The destination `link_resolve` writes: canonical, portable text.
        fn absolute(&self, relative: &str) -> String {
            let path = relative.split('/').fold(self.repo.clone(), |path, name| path.join(name));
            biscuit_file::to_portable_string(&fs::canonicalize(path).unwrap())
        }
    }

    #[test]
    fn a_peer_directory_target_becomes_a_relative_link() {
        let fixture = RepoFixture::new(&["image.png"]);
        let content = format!("![img]({})\n", fixture.absolute("assets/image.png"));

        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(output, "![img](../assets/image.png)\n");
        assert_eq!(report.link_normalizations_applied, 1);
        assert!(link_warnings(&report).is_empty(), "{:?}", report.warnings);
    }

    /// The document's own directory is the route's origin, whatever the
    /// document's name looks like: an extensionless source (`docs/README`)
    /// is not a directory, and a dotted directory name (`v1.2`) is not a file.
    /// Two levels up is not a default relative shape, so the repository root
    /// anchors it.
    #[test]
    fn a_distant_target_in_the_repository_is_repository_rooted() {
        let fixture = RepoFixture::new(&["image.png"]);
        let docs = fixture.repo.join("docs").join("v1.2");
        fs::create_dir_all(&docs).unwrap();

        for source_name in ["README", "guide.md"] {
            let source_file = docs.join(source_name);
            fs::write(&source_file, "").unwrap();
            let content = format!("![img]({})\n", fixture.absolute("assets/image.png"));

            let (output, report) = normalize(&content, &options_with_repo(&source_file));

            assert_eq!(output, "![img](&assets/image.png)\n", "source {source_name}");
            assert_eq!(report.link_normalizations_applied, 1, "source {source_name}");
        }
    }

    #[test]
    fn deep_and_same_directory_targets() {
        let fixture = RepoFixture::new(&[]);
        let docs = fixture.repo.join("docs").join("deep").join("nested").join("dir");
        let images = fixture.repo.join("assets").join("images");
        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&images).unwrap();
        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();
        fs::write(images.join("image.png"), "png").unwrap();
        fs::write(docs.join("sibling.md"), "md").unwrap();
        let content = format!(
            "[img]({})\n[sibling]({})",
            fixture.absolute("assets/images/image.png"),
            fixture.absolute("docs/deep/nested/dir/sibling.md"),
        );

        let (output, report) = normalize(&content, &options_with_repo(&source_file));

        assert_eq!(output, "[img](&assets/images/image.png)\n[sibling](./sibling.md)");
        assert_eq!(report.link_normalizations_applied, 2);
    }

    #[test]
    fn a_target_under_home_is_home_rooted() {
        let home = dirs::home_dir().expect("Has home dir");
        let target = home.join("some_file.txt");
        let content = format!("[file]({})\n", biscuit_file::to_portable_string(&target));

        let (output, report) = normalize(&content, &ComposeOptions::new());

        assert_eq!(output, "[file](~/some_file.txt)\n");
        assert_eq!(report.link_normalizations_applied, 1);
    }

    /// The written spelling is the triple-brace literal: a composed document
    /// is Darkmatter source again, and `{{{VAR}}}` composes back to the
    /// `{{VAR}}` anchor link resolution reads.
    #[test]
    fn a_declared_portable_variable_is_written_as_an_interpolation_literal() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("config.json"), "{}").unwrap();
        let env = HashMap::from([(
            "PROJECT_ROOT".to_string(),
            env_value(&project),
        )]);
        let content = format!(
            "<a href=\"{}\">config</a>\n",
            biscuit_file::to_portable_string(&project.join("config.json"))
        );

        let options = detached_options(&root, env).with_portable_env(["PROJECT_ROOT"]);
        let (output, report) = normalize(&content, &options);

        assert_eq!(output, "<a href=\"{{{PROJECT_ROOT}}}/config.json\">config</a>\n");
        assert_eq!(report.link_normalizations_applied, 1);
        assert!(link_warnings(&report).is_empty(), "{:?}", report.warnings);
    }

    /// `PROJECT_ROOT` and `DOCS_BASE` were built-in anchors once; now a
    /// variable is portable only when something declares it.
    #[test]
    fn no_variable_is_portable_unless_declared() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("config.json"), "{}").unwrap();
        let value = env_value(&project);
        let env = HashMap::from([
            ("PROJECT_ROOT".to_string(), value.clone()),
            ("DOCS_BASE".to_string(), value),
        ]);
        let destination = biscuit_file::to_portable_string(&project.join("config.json"));
        let content = format!("[config]({destination})\n");

        let (output, report) = normalize(&content, &detached_options(&root, env));

        assert_eq!(output, content, "the absolute fallback rewrites nothing");
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(link_warnings(&report).is_empty(), "{:?}", report.warnings);
    }

    /// `PORTABLE_ENV_VARIABLES` is read from the request's captured
    /// environment, not the process, and joins the option's names.
    #[test]
    fn the_captured_environment_declares_and_supplies_variables() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let config = root.join("config");
        let notes = root.join("notes");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&notes).unwrap();
        fs::write(config.join("a.json"), "{}").unwrap();
        fs::write(notes.join("b.md"), "").unwrap();
        let env = HashMap::from([
            ("PORTABLE_ENV_VARIABLES".to_string(), " CAPTURED_ROOT , bad-name".to_string()),
            ("CAPTURED_ROOT".to_string(), env_value(&config)),
            ("NOTES".to_string(), env_value(&notes)),
        ]);
        let content = format!(
            "[a]({})\n[b]({})\n",
            biscuit_file::to_portable_string(&config.join("a.json")),
            biscuit_file::to_portable_string(&notes.join("b.md")),
        );

        let options = detached_options(&root, env).with_portable_env(["NOTES"]);
        let (output, report) = normalize(&content, &options);

        assert_eq!(output, "[a]({{{CAPTURED_ROOT}}}/a.json)\n[b]({{{NOTES}}}/b.md)\n");
        assert_eq!(report.link_normalizations_applied, 2);
        let warnings = link_warnings(&report);
        assert_eq!(warnings.len(), 1, "one warning per invalid name: {warnings:?}");
        assert!(warnings[0].contains("bad-name"), "{warnings:?}");
    }

    #[test]
    fn the_deepest_portable_variable_wins() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let parent = root.join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("config.json"), "{}").unwrap();
        let env = HashMap::from([
            ("USER".to_string(), env_value(&parent)),
            ("USER_NAME".to_string(), env_value(&child)),
        ]);
        let content = format!(
            "[config]({})",
            biscuit_file::to_portable_string(&child.join("config.json"))
        );

        let options = detached_options(&root, env).with_portable_env(["USER", "USER_NAME"]);
        let (output, _) = normalize(&content, &options);

        assert_eq!(output, "[config]({{{USER_NAME}}}/config.json)");
    }

    #[test]
    fn css_font_and_script_destinations_normalize() {
        let fixture = RepoFixture::new(&["styles.css", "font.woff2", "app.js"]);
        let content = format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n<link rel=\"preload\" as=\"font\" href=\"{}\">\n<script src=\"{}\"></script>",
            fixture.absolute("assets/styles.css"),
            fixture.absolute("assets/font.woff2"),
            fixture.absolute("assets/app.js"),
        );

        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(
            output,
            "<link rel=\"stylesheet\" href=\"../assets/styles.css\">\n<link rel=\"preload\" as=\"font\" href=\"../assets/font.woff2\">\n<script src=\"../assets/app.js\"></script>"
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn angle_bracket_and_quoted_destinations_keep_their_delimiters() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let source_file = repo.join("source.md");
        fs::write(&source_file, "").unwrap();
        fs::write(repo.join("with (parens).md"), "").unwrap();
        fs::write(repo.join("single_quotes.md"), "").unwrap();
        let abs_parens = fs::canonicalize(repo.join("with (parens).md")).unwrap();
        let abs_quotes = fs::canonicalize(repo.join("single_quotes.md")).unwrap();
        let content = format!(
            "[link](<{}>)\n<img src='{}'>\n<a href=\"{}\" data-alt='{}'>link</a>",
            biscuit_file::to_portable_string(&abs_parens),
            biscuit_file::to_portable_string(&abs_quotes),
            biscuit_file::to_portable_string(&abs_quotes),
            biscuit_file::to_portable_string(&abs_quotes)
        );

        let (output, report) = normalize(&content, &options_with_repo(&source_file));

        assert_eq!(
            output,
            format!(
                "[link](<./with (parens).md>)\n<img src='./single_quotes.md'>\n<a href=\"./single_quotes.md\" data-alt='{}'>link</a>",
                biscuit_file::to_portable_string(&abs_quotes)
            ),
            "only destinations change; a data attribute is not a link"
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn spaced_html_attributes_normalize() {
        let fixture = RepoFixture::new(&["image.png", "movie.mp4", "styles.css"]);
        let content = format!(
            "<a href = \"{}\">link</a>\n<img src = \"{}\">\n<video src = \"{}\"></video>\n<link href = \"{}\">",
            fixture.absolute("assets/image.png"),
            fixture.absolute("assets/image.png"),
            fixture.absolute("assets/movie.mp4"),
            fixture.absolute("assets/styles.css"),
        );

        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(
            output,
            "<a href = \"../assets/image.png\">link</a>\n<img src = \"../assets/image.png\">\n<video src = \"../assets/movie.mp4\"></video>\n<link href = \"../assets/styles.css\">"
        );
        assert_eq!(report.link_normalizations_applied, 4);
    }

    /// The suffix layer stays in Darkmatter: each suffix is split from the
    /// parsed destination and reattached to the portable reference.
    #[test]
    fn fragment_query_and_line_suffixes_are_reattached() {
        let fixture = RepoFixture::new(&["x.md", "x.rs"]);
        let x_md = fixture.absolute("assets/x.md");
        let x_rs = fixture.absolute("assets/x.rs");
        let content = format!(
            "[a]({x_md}#install)\n[b]({x_md}?plain=1)\n[c]({x_rs}:42)\n[d]({x_rs}:3-7)\n[e]({x_md}?q=1#frag)\n"
        );

        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(
            output,
            "[a](../assets/x.md#install)\n[b](../assets/x.md?plain=1)\n[c](../assets/x.rs:42)\n[d](../assets/x.rs:3-7)\n[e](../assets/x.md?q=1#frag)\n"
        );
        assert_eq!(report.link_normalizations_applied, 5);
    }

    #[test]
    fn split_suffix_only_takes_trailing_line_numbers_after_a_colon() {
        assert_eq!(split_suffix("/a/x.md#h"), ("/a/x.md", "#h"));
        assert_eq!(split_suffix("/a/x.md?q#h"), ("/a/x.md", "?q#h"));
        assert_eq!(split_suffix("/a/x.rs:42"), ("/a/x.rs", ":42"));
        assert_eq!(split_suffix("/a/x.rs:3-7"), ("/a/x.rs", ":3-7"));
        assert_eq!(split_suffix("/a/x.rs:42#h"), ("/a/x.rs", ":42#h"));
        // Not line suffixes: a drive colon, a non-numeric tail, an open range.
        assert_eq!(split_suffix("C:/a/x.md"), ("C:/a/x.md", ""));
        assert_eq!(split_suffix("/a/b:c.md"), ("/a/b:c.md", ""));
        assert_eq!(split_suffix("/a/x.rs:3-"), ("/a/x.rs:3-", ""));
        assert_eq!(split_suffix("/a/x.rs:"), ("/a/x.rs:", ""));
        // A verbatim prefix's `?` is not a query.
        assert_eq!(split_suffix(r"\\?\C:\a\x.md"), (r"\\?\C:\a\x.md", ""));
        assert_eq!(split_suffix(r"\\?\C:\a\x.md#h"), (r"\\?\C:\a\x.md", "#h"));
        assert_eq!(split_suffix(r"\\?\C:\a\x.rs:9"), (r"\\?\C:\a\x.rs", ":9"));
        assert_eq!(split_suffix("//?/C:/a/x.md?q"), ("//?/C:/a/x.md", "?q"));
    }

    /// Normalizing an already-normalized document changes nothing.
    #[test]
    fn normalizing_twice_changes_nothing_the_second_time() {
        let fixture = RepoFixture::new(&["image.png", "x.md"]);
        let content = format!(
            "![img]({})\n[x]({}#h)\n",
            fixture.absolute("assets/image.png"),
            fixture.absolute("assets/x.md"),
        );
        let options = options_with_repo(&fixture.source);

        let (once, _) = normalize(&content, &options);
        let (twice, report) = normalize(&once, &options);

        assert_eq!(once, "![img](../assets/image.png)\n[x](../assets/x.md#h)\n");
        assert_eq!(twice, once);
        assert_eq!(report.link_normalizations_applied, 0);
    }

    /// A relative or sigil destination is what link resolution could not make
    /// absolute; normalization has no target for it and leaves it alone.
    #[test]
    fn relative_and_sigil_destinations_are_left_alone() {
        let fixture = RepoFixture::new(&["image.png"]);
        let content = "[a](../assets/image.png)\n[b](&assets/image.png)\n[c](~/x.md)\n[d](missing.md)\n";

        let (output, report) = normalize(content, &options_with_repo(&fixture.source));

        assert_eq!(output, content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(link_warnings(&report).is_empty(), "{:?}", report.warnings);
    }

    /// A target nothing portable reaches keeps its absolute destination
    /// silently: the absolute path is a faithful spelling of itself.
    #[test]
    fn the_absolute_fallback_keeps_the_destination_without_a_warning() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let other = root.join("other");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("x.md"), "").unwrap();
        let content = format!(
            "[x]({})\n",
            biscuit_file::to_portable_string(&other.join("x.md"))
        );

        let (output, report) = normalize(&content, &detached_options(&root, HashMap::new()));

        assert_eq!(output, content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(link_warnings(&report).is_empty(), "{:?}", report.warnings);
    }

    /// A probe failure (here a path through a regular file) is never read as
    /// a missing file: the destination is kept and the failure reported.
    #[cfg(unix)]
    #[test]
    fn an_evaluation_failure_keeps_the_destination_and_warns() {
        let fixture = RepoFixture::new(&["image.png"]);
        let destination = format!("{}/x.md", fixture.absolute("assets/image.png"));
        let content = format!("[x]({destination})\n");

        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(output, content);
        assert_eq!(report.link_normalizations_applied, 0);
        let warnings = link_warnings(&report);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("left exactly as authored"), "{warnings:?}");
        assert!(warnings[0].contains(&destination), "{warnings:?}");
    }

    /// A canonical destination (`/private/var/…` on macOS) still routes from a
    /// context spelled the way the request was opened (`/var/…`).
    #[test]
    fn a_canonical_destination_routes_from_a_lexical_context() {
        let fixture = RepoFixture::new(&["image.png"]);
        let lexical = fixture.repo.join("assets").join("image.png");
        let canonical = fixture.absolute("assets/image.png");
        let options = options_with_repo(&fixture.source);

        for destination in [canonical, biscuit_file::to_portable_string(&lexical)] {
            let (output, _) = normalize(&format!("![i]({destination})"), &options);
            assert_eq!(output, "![i](../assets/image.png)", "{destination}");
        }
    }

    #[test]
    fn remote_urls_are_not_touched() {
        let content = "[link](https://example.com/page) and ![img](http://cdn.example.com/img.png)";

        let (output, report) = normalize(content, &ComposeOptions::new());

        assert_eq!(output, content);
        assert_eq!(report.link_normalizations_applied, 0);
    }

    /// Every way a verbatim component stops meaning itself, in the spelling an
    /// author can actually put in a document.
    ///
    /// Which names survive losing `\\?\` is `biscuit-file`'s rule (its
    /// `survives_without_verbatim_prefix` tests cover each one); these
    /// fixtures prove the rule reaches this stage through every anchor.
    #[cfg(windows)]
    fn unsafe_component_fixtures() -> Vec<(&'static str, String)> {
        vec![
            ("literal dot", ".".to_string()),
            ("literal dot-dot", "..".to_string()),
            ("reserved DOS name", "CON".to_string()),
            ("reserved DOS name with extension", "com1.txt".to_string()),
            ("trailing dot", "trailing.".to_string()),
            ("trailing space", "trailing ".to_string()),
            ("invalid Win32 character", "a*b".to_string()),
            // 128 astral scalar values are 256 UTF-16 units, which is what
            // Windows measures.
            ("overlong in UTF-16 units", "🦀".repeat(128)),
        ]
    }

    /// Normalizes `content` and asserts the destination survived
    /// byte-identical with a preservation warning.
    #[cfg(windows)]
    fn assert_preserved_with_warning(content: &str, options: &ComposeOptions, label: &str) {
        let (output, report) = normalize(content, options);

        assert_eq!(output, content, "{label}: destination was rewritten");
        assert_eq!(report.link_normalizations_applied, 0, "{label}");
        assert!(
            link_warnings(&report)
                .iter()
                .any(|warning| warning.contains("left exactly as authored")),
            "{label}: expected a preservation warning, got: {:?}",
            report.warnings
        );
    }

    /// A Windows snapshot outside any repository whose `cwd` is a directory
    /// beside `root`'s contents, so only the named anchor can claim a target.
    #[cfg(windows)]
    fn env_options(root: &Path) -> ComposeOptions {
        let env = HashMap::from([(
            "PROJECT_ROOT".to_string(),
            env_value(&root),
        )]);
        detached_options(root, env).with_portable_env(["PROJECT_ROOT"])
    }

    /// As [`env_options`], but the anchor is the home directory.
    #[cfg(windows)]
    fn home_options(home: &Path) -> ComposeOptions {
        let elsewhere = home.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let snapshot = biscuit_file::FileResolutionContext::new(&elsewhere)
            .with_home_dir(home)
            .with_env(HashMap::new());
        ComposeOptions::new().with_file_resolution_context(snapshot)
    }

    /// Rewriting `\\?\C:\…\repo\assets\.\image.png` relative to a document
    /// inside the repository would drop the namespace and, once re-read as an
    /// ordinary path, the literal `.` directory: a different location. Every
    /// category must survive byte-identical instead, with a warning.
    #[cfg(windows)]
    #[test]
    fn repo_anchor_preserves_every_unsafe_category() {
        for (label, unsafe_name) in unsafe_component_fixtures() {
            let fixture = RepoFixture::new(&[]);
            // `canonicalize` yields the verbatim spelling on Windows, so the
            // destination below is a genuine descendant of the repository root.
            let verbatim_repo = fs::canonicalize(&fixture.repo).unwrap();
            let destination =
                format!(r"{}\assets\{unsafe_name}\image.png", verbatim_repo.display());
            assert!(
                try_portable_string(Path::new(&destination)).is_none(),
                "{label}: fixture must be one `dunce` declines"
            );

            let content = format!("<img src=\"{destination}\">\n");
            assert_preserved_with_warning(&content, &options_with_repo(&fixture.source), label);
        }
    }

    /// The other half of the same rule: an anchored descendant whose only
    /// problem is length still normalizes, end to end.
    #[cfg(windows)]
    #[test]
    fn anchored_over_max_path_destination_still_normalizes() {
        let fixture = RepoFixture::new(&[]);
        let verbatim_repo = fs::canonicalize(&fixture.repo).unwrap();
        let long_name = "a".repeat(250);
        let destination = format!(r"{}\assets\{long_name}\image.png", verbatim_repo.display());
        assert!(
            try_portable_string(Path::new(&destination)).is_none(),
            "fixture must exceed MAX_PATH or it proves nothing"
        );

        let content = format!("<img src=\"{destination}\">\n");
        let (output, report) = normalize(&content, &options_with_repo(&fixture.source));

        assert_eq!(output, format!("<img src=\"../assets/{long_name}/image.png\">\n"));
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[cfg(windows)]
    #[test]
    fn env_anchor_preserves_every_unsafe_category() {
        for (label, unsafe_name) in unsafe_component_fixtures() {
            let dir = tempdir().unwrap();
            let root = fs::canonicalize(dir.path()).unwrap();
            let destination = format!(r"{}\docs\{unsafe_name}\f.md", root.display());
            assert!(
                try_portable_string(Path::new(&destination)).is_none(),
                "{label}: fixture must be one `dunce` declines"
            );

            let content = format!("<a href=\"{destination}\">f</a>\n");
            assert_preserved_with_warning(&content, &env_options(&root), label);
        }
    }

    #[cfg(windows)]
    #[test]
    fn home_anchor_preserves_every_unsafe_category() {
        for (label, unsafe_name) in unsafe_component_fixtures() {
            let dir = tempdir().unwrap();
            let home = fs::canonicalize(dir.path()).unwrap();
            let destination = format!(r"{}\docs\{unsafe_name}\f.md", home.display());
            assert!(
                try_portable_string(Path::new(&destination)).is_none(),
                "{label}: fixture must be one `dunce` declines"
            );

            let content = format!("<a href=\"{destination}\">f</a>\n");
            assert_preserved_with_warning(&content, &home_options(&home), label);
        }
    }

    /// The success control for [`env_anchor_preserves_every_unsafe_category`]:
    /// without it the gate cannot be told apart from a blanket refusal to
    /// anchor anything `try_portable_string` declined.
    #[cfg(windows)]
    #[test]
    fn env_anchored_over_max_path_destination_still_normalizes() {
        let dir = tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let long_name = "a".repeat(250);
        let destination = format!(r"{}\docs\{long_name}\f.md", root.display());
        assert!(
            try_portable_string(Path::new(&destination)).is_none(),
            "fixture must exceed MAX_PATH or it proves nothing"
        );

        let content = format!("<a href=\"{destination}\">f</a>\n");
        let (output, report) = normalize(&content, &env_options(&root));

        assert_eq!(
            output,
            format!("<a href=\"{{{{{{PROJECT_ROOT}}}}}}/docs/{long_name}/f.md\">f</a>\n")
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    /// The success control for [`home_anchor_preserves_every_unsafe_category`].
    #[cfg(windows)]
    #[test]
    fn home_anchored_over_max_path_destination_still_normalizes() {
        let dir = tempdir().unwrap();
        let home = fs::canonicalize(dir.path()).unwrap();
        let long_name = "a".repeat(250);
        let destination = format!(r"{}\docs\{long_name}\f.md", home.display());
        assert!(
            try_portable_string(Path::new(&destination)).is_none(),
            "fixture must exceed MAX_PATH or it proves nothing"
        );

        let content = format!("<a href=\"{destination}\">f</a>\n");
        let (output, report) = normalize(&content, &home_options(&home));

        assert_eq!(output, format!("<a href=\"~/docs/{long_name}/f.md\">f</a>\n"));
        assert_eq!(report.link_normalizations_applied, 1);
    }

    /// Finalization runs after transclusion, so an authored destination it
    /// cannot portabilize is left exactly as written and reported: the
    /// warn-and-preserve half of the stage-specific policy whose other half is
    /// `link_resolve`'s error.
    ///
    /// The fixture is an HTML anchor because CommonMark consumes the backslash
    /// escapes in a Markdown destination. It is a verbatim path rather than a
    /// UNC one because probing `\\server\share` blocks on SMB name resolution
    /// for tens of seconds.
    #[cfg(windows)]
    #[test]
    fn declined_absolute_destination_is_preserved_and_warned() {
        let dir = tempdir().unwrap();
        let options = detached_options(dir.path(), HashMap::new());
        let content = "<a href=\"\\\\?\\C:\\repo\\.\\docs\\f.md\">f</a>\n";

        let (output, report) = normalize(content, &options);

        assert_eq!(output, content);
        assert_eq!(report.link_normalizations_applied, 0);
        assert!(
            link_warnings(&report)
                .iter()
                .any(|warning| warning.contains(r"\\?\C:\repo\.\docs\f.md")),
            "expected a preserved-destination warning, got: {:?}",
            report.warnings
        );
    }
}
