//! Formatter opinions: which candidate a row selects, and the indentation it
//! states.
//!
//! A row selects the first readable, parseable candidate that declares the
//! formatter. Unreadable and unparseable candidates, and declarations of the
//! wrong type, are diagnosed and skipped. Existing candidates after the
//! selected one are reported as shadowed. A selected candidate that is
//! disabled, names only a shared preset, or has no document for the file's
//! language gives no opinion and still shadows the rest. Values the file
//! leaves unstated, or states invalidly, take the row's default, so a
//! formatter opinion always states both values.

use std::num::NonZeroU8;
use std::path::PathBuf;

use globset::GlobBuilder;

use super::catalog::{Declaration, FormatterRow};
use super::editorconfig::is_extension_wide;
use super::inputs::{Inputs, Loaded, Node};
use super::{Diagnostics, IndentAuthority, IndentOpinion, IndentStyle, Target, TargetKind};

/// The row's opinion for `target`, or `None` when no candidate gives one.
pub(super) fn row_opinion(
    row: &FormatterRow,
    inputs: &Inputs,
    target: &Target,
    diagnostics: &mut Diagnostics,
) -> Option<IndentOpinion> {
    for executable in row.executable {
        if inputs.executable_exists(executable) {
            diagnostics.push(executable, None, "is an executable configuration; Sniff does not run it");
        }
    }

    let mut selected: Option<(&str, Option<IndentOpinion>)> = None;
    for candidate in row.candidates {
        let loaded = inputs.get(candidate.path);
        if let Some((selected_path, _)) = &selected {
            let declares = match (loaded, candidate.declaration) {
                (Loaded::Absent, _) => false,
                (Loaded::Document(document), Declaration::Key(key)) => lookup(document, key).is_some(),
                _ => true,
            };
            if declares {
                diagnostics.push(candidate.path, None, format!("is shadowed by `{selected_path}`"));
            }
            continue;
        }
        let declaration = match loaded {
            Loaded::Absent => continue,
            Loaded::Unreadable(reason) | Loaded::Unparseable(reason) => {
                diagnostics.push(candidate.path, None, format!("{reason}; skipped"));
                continue;
            }
            Loaded::EditorConfig(_) => continue,
            Loaded::Documents(documents) => Declared::Documents(documents),
            Loaded::Document(document) => match candidate.declaration {
                Declaration::Bare => Declared::Node(document, String::new()),
                Declaration::Key(key) => match lookup(document, key) {
                    None => continue,
                    Some(node) => Declared::Node(node, format!("{}.", key.join("."))),
                },
            },
        };
        let mut reader = Reader {
            path: candidate.path,
            prefix: String::new(),
            target,
            diagnostics: &mut *diagnostics,
            via: Vec::new(),
        };
        let outcome = match declaration {
            Declared::Documents(documents) => clang_format(documents, &mut reader),
            Declared::Node(node, prefix) => {
                reader.prefix = prefix;
                declared_mapping(row, node, &mut reader)
            }
        };
        let via = std::mem::take(&mut reader.via);
        match outcome {
            Outcome::Skipped => continue,
            Outcome::NoOpinion => selected = Some((candidate.path, None)),
            Outcome::Opinion(layer) => {
                let opinion = IndentOpinion {
                    source: row.authority,
                    path: Some(PathBuf::from(candidate.path)),
                    via,
                    indent_using: Some(layer.style.unwrap_or(row.default_style)),
                    indent: Some(layer.width.unwrap_or_else(|| {
                        NonZeroU8::new(row.default_width).expect("catalog widths are positive")
                    })),
                };
                selected = Some((candidate.path, Some(opinion)));
            }
        }
    }
    selected.and_then(|(_, opinion)| opinion)
}

enum Declared<'a> {
    Node(&'a Node, String),
    Documents(&'a [Node]),
}

/// What one selected-or-skipped candidate yields.
enum Outcome {
    Opinion(Layer),
    /// Declares the formatter but gives no opinion; still shadows later
    /// candidates.
    NoOpinion,
    /// A malformed declaration: diagnosed, and the row moves on.
    Skipped,
}

/// Values one layer of a configuration states; `None` falls through to the
/// layer below, then to the row default.
#[derive(Debug, Clone, Copy, Default)]
struct Layer {
    style: Option<IndentStyle>,
    width: Option<NonZeroU8>,
}

impl Layer {
    /// `self` over `lower`, property by property.
    fn over(self, lower: Layer) -> Layer {
        Layer { style: self.style.or(lower.style), width: self.width.or(lower.width) }
    }
}

fn lookup<'a>(document: &'a Node, key: &[&str]) -> Option<&'a Node> {
    key.iter().try_fold(document, |node, segment| node.get(segment))
}

fn declared_mapping(row: &FormatterRow, node: &Node, reader: &mut Reader<'_>) -> Outcome {
    if row.authority == IndentAuthority::Prettier
        && let Node::String(preset) = node
    {
        reader.report(None, format!("names the shared preset `{preset}`, which Sniff does not load"));
        return Outcome::NoOpinion;
    }
    if node.as_map().is_none() {
        let key = reader.prefix.trim_end_matches('.').to_string();
        let message = format!("must be a mapping, found {}; skipped", node.describe());
        reader.diagnostics.push(reader.path, (!key.is_empty()).then_some(key.as_str()), message);
        return Outcome::Skipped;
    }
    match row.authority {
        IndentAuthority::Rustfmt => rustfmt(node, reader),
        IndentAuthority::Prettier | IndentAuthority::Oxfmt => prettier_like(node, reader),
        IndentAuthority::Biome => biome(node, reader),
        IndentAuthority::Deno => deno(node, reader),
        IndentAuthority::Dprint => dprint(node, reader),
        IndentAuthority::Ruff => ruff(node, reader),
        IndentAuthority::Tombi => tombi(node, reader),
        IndentAuthority::Taplo => taplo(node, reader),
        IndentAuthority::Yamlfmt => yamlfmt(node, reader),
        IndentAuthority::Stylua => stylua(node, reader),
        IndentAuthority::SwiftFormat => swift_format(node, reader),
        IndentAuthority::Rubocop => rubocop(node, reader),
        IndentAuthority::Csharpier => csharpier(node, reader),
        IndentAuthority::ClangFormat
        | IndentAuthority::EditorConfig
        | IndentAuthority::Convention
        | IndentAuthority::Default => Outcome::NoOpinion,
    }
}

/// Reads settings from one candidate, diagnosing invalid values and recording
/// the keys it used in `via`.
struct Reader<'a> {
    path: &'a str,
    /// The declaration's key path, ending in `.` (`prettier.`), or empty.
    prefix: String,
    target: &'a Target,
    diagnostics: &'a mut Diagnostics,
    via: Vec<String>,
}

impl Reader<'_> {
    fn full_key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    fn report(&mut self, key: Option<&str>, reason: impl Into<String>) {
        let key = key.map(|key| self.full_key(key));
        self.diagnostics.push(self.path, key.as_deref(), reason);
    }

    fn used(&mut self, key: &str) {
        let key = self.full_key(key);
        if !self.via.contains(&key) {
            self.via.push(key);
        }
    }

    fn invalid(&mut self, key: &str, expected: &str, found: &Node) {
        self.report(Some(key), format!("must be {expected}, found {}; ignored", found.describe()));
    }

    /// A nested mapping. Absent is `None`; any other type is diagnosed and
    /// treated as absent.
    fn mapping<'n>(&mut self, node: Option<&'n Node>, key: &str) -> Option<&'n Node> {
        let node = node?;
        if node.as_map().is_some() {
            Some(node)
        } else {
            self.invalid(key, "a mapping", node);
            None
        }
    }

    fn width(&mut self, node: Option<&Node>, key: &str) -> Option<NonZeroU8> {
        let node = node?;
        match node {
            Node::Int(value) if (1..=255).contains(value) => {
                self.used(key);
                NonZeroU8::new(u8::try_from(*value).ok()?)
            }
            _ => {
                self.invalid(key, "an integer from 1 through 255", node);
                None
            }
        }
    }

    fn boolean(&mut self, node: Option<&Node>, key: &str) -> Option<bool> {
        let node = node?;
        match node {
            Node::Bool(value) => {
                self.used(key);
                Some(*value)
            }
            _ => {
                self.invalid(key, "a boolean", node);
                None
            }
        }
    }

    /// A boolean that selects tabs when true.
    fn uses_tabs(&mut self, node: Option<&Node>, key: &str) -> Option<IndentStyle> {
        self.boolean(node, key).map(|tabs| if tabs { IndentStyle::Tabs } else { IndentStyle::Spaces })
    }

    /// A string naming one of two styles, compared exactly.
    fn style(&mut self, node: Option<&Node>, key: &str, spaces: &str, tabs: &str) -> Option<IndentStyle> {
        let node = node?;
        match node {
            Node::String(value) if value == spaces => {
                self.used(key);
                Some(IndentStyle::Spaces)
            }
            Node::String(value) if value == tabs => {
                self.used(key);
                Some(IndentStyle::Tabs)
            }
            _ => {
                self.invalid(key, &format!("`{spaces}` or `{tabs}`"), node);
                None
            }
        }
    }

    /// A glob field: a non-empty string, or a non-empty list of them. One bad
    /// element invalidates the whole field.
    fn globs(&mut self, node: &Node, key: &str) -> Option<Vec<String>> {
        let patterns: Option<Vec<String>> = match node {
            Node::String(pattern) if !pattern.is_empty() => Some(vec![pattern.clone()]),
            Node::List(items) if !items.is_empty() => items
                .iter()
                .map(|item| match item {
                    Node::String(pattern) if !pattern.is_empty() => Some(pattern.clone()),
                    _ => None,
                })
                .collect(),
            _ => None,
        };
        let Some(patterns) = patterns else {
            self.invalid(key, "a non-empty glob or a non-empty list of non-empty globs", node);
            return None;
        };
        for pattern in &patterns {
            if let Err(error) = override_glob(pattern) {
                self.report(Some(key), format!("holds the invalid glob `{pattern}`: {error}; ignored"));
                return None;
            }
        }
        Some(patterns)
    }

    fn unsupported(&mut self, node: &Node, key: &str, reason: &str) {
        if node.get(key).is_some() {
            self.report(Some(key), reason.to_string());
        }
    }
}

const NOT_FOLLOWED: &str = "is not followed; the file's own settings and the formatter's defaults apply";
const OVERRIDES_NOT_APPLIED: &str = "path overrides are not applied";

fn rustfmt(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    Outcome::Opinion(Layer {
        style: reader.uses_tabs(node.get("hard_tabs"), "hard_tabs"),
        width: reader.width(node.get("tab_spaces"), "tab_spaces"),
    })
}

/// Prettier and oxfmt: `useTabs`, `tabWidth`, and matching
/// `overrides[].options` applied in file order.
fn prettier_like(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    let mut layer = Layer {
        style: reader.uses_tabs(node.get("useTabs"), "useTabs"),
        width: reader.width(node.get("tabWidth"), "tabWidth"),
    };
    let Some(overrides) = node.get("overrides") else {
        return Outcome::Opinion(layer);
    };
    let Node::List(entries) = overrides else {
        reader.invalid("overrides", "a list", overrides);
        return Outcome::Opinion(layer);
    };
    for (index, entry) in entries.iter().enumerate() {
        let key = format!("overrides[{index}]");
        if let Some(entry_layer) = override_entry(entry, &key, reader) {
            layer = entry_layer.over(layer);
        }
    }
    Outcome::Opinion(layer)
}

/// One override entry's options when the entry is valid and matches.
fn override_entry(entry: &Node, key: &str, reader: &mut Reader<'_>) -> Option<Layer> {
    if entry.as_map().is_none() {
        reader.invalid(key, "a mapping", entry);
        return None;
    }
    let files_key = format!("{key}.files");
    let Some(files) = entry.get("files") else {
        reader.report(Some(key), "has no `files`; ignored");
        return None;
    };
    let files = reader.globs(files, &files_key)?;
    let exclude_key = format!("{key}.excludeFiles");
    let excludes = match entry.get("excludeFiles") {
        None => Vec::new(),
        Some(node) => reader.globs(node, &exclude_key)?,
    };
    let options_key = format!("{key}.options");
    let options = match entry.get("options") {
        None => None,
        Some(node) if node.as_map().is_some() => Some(node),
        Some(node) => {
            reader.invalid(&options_key, "a mapping", node);
            return None;
        }
    };

    // An extension lookup has no real path: an entry that depends on one is
    // left to path lookup, whole.
    if matches!(reader.target.kind, TargetKind::Extension(_))
        && !files.iter().chain(&excludes).all(|pattern| is_extension_wide(pattern))
    {
        return None;
    }
    let path = reader.target.match_path();
    let included = files.iter().any(|pattern| override_matches(pattern, &path));
    let excluded = excludes.iter().any(|pattern| override_matches(pattern, &path));
    if !included || excluded {
        return None;
    }
    let options = options?;
    Some(Layer {
        style: reader.uses_tabs(options.get("useTabs"), &format!("{options_key}.useTabs")),
        width: reader.width(options.get("tabWidth"), &format!("{options_key}.tabWidth")),
    })
}

fn override_glob(pattern: &str) -> Result<globset::GlobMatcher, globset::Error> {
    GlobBuilder::new(pattern).literal_separator(true).backslash_escape(true).build().map(|glob| glob.compile_matcher())
}

/// Prettier's rule: a pattern without `/` matches the basename at any depth.
fn override_matches(pattern: &str, path: &str) -> bool {
    let Ok(matcher) = override_glob(pattern) else {
        return false;
    };
    let subject = if pattern.contains('/') { path } else { path.rsplit('/').next().unwrap_or(path) };
    matcher.is_match(subject)
}

fn biome(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "extends", NOT_FOLLOWED);
    reader.unsupported(node, "overrides", OVERRIDES_NOT_APPLIED);
    let file_type = reader.target.file_type.as_deref().unwrap_or_default();
    let language = match file_type {
        "json" | "jsonc" => "json",
        "css" => "css",
        "graphql" | "gql" => "graphql",
        _ => "javascript",
    };

    let global = reader.mapping(node.get("formatter"), "formatter");
    let (global_layer, global_enabled) = biome_formatter(global, "formatter", reader);
    let language_formatter_key = format!("{language}.formatter");
    let language_block = reader.mapping(node.get(language), language);
    let language_formatter = language_block
        .and_then(|block| reader.mapping(block.get("formatter"), &language_formatter_key));
    let (language_layer, language_enabled) = biome_formatter(language_formatter, &language_formatter_key, reader);

    // The language's own switch wins over the global one.
    if language_enabled.or(global_enabled) == Some(false) {
        return Outcome::NoOpinion;
    }
    Outcome::Opinion(language_layer.over(global_layer))
}

fn biome_formatter(block: Option<&Node>, key: &str, reader: &mut Reader<'_>) -> (Layer, Option<bool>) {
    let Some(block) = block else {
        return (Layer::default(), None);
    };
    let enabled = reader.boolean(block.get("enabled"), &format!("{key}.enabled"));
    let layer = Layer {
        style: reader.style(block.get("indentStyle"), &format!("{key}.indentStyle"), "space", "tab"),
        width: reader.width(block.get("indentWidth"), &format!("{key}.indentWidth")),
    };
    (layer, enabled)
}

fn deno(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    let Some(fmt) = reader.mapping(node.get("fmt"), "fmt") else {
        return Outcome::Opinion(Layer::default());
    };
    let flat = Layer {
        style: reader.uses_tabs(fmt.get("useTabs"), "fmt.useTabs"),
        width: reader.width(fmt.get("indentWidth"), "fmt.indentWidth"),
    };
    // Legacy `fmt.options` applies only where the flat key is absent.
    let legacy = match reader.mapping(fmt.get("options"), "fmt.options") {
        Some(options) => Layer {
            style: if fmt.get("useTabs").is_none() {
                reader.uses_tabs(options.get("useTabs"), "fmt.options.useTabs")
            } else {
                None
            },
            width: if fmt.get("indentWidth").is_none() {
                reader.width(options.get("indentWidth"), "fmt.options.indentWidth")
            } else {
                None
            },
        },
        None => Layer::default(),
    };
    Outcome::Opinion(flat.over(legacy))
}

fn dprint(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "extends", NOT_FOLLOWED);
    reader.unsupported(node, "overrides", OVERRIDES_NOT_APPLIED);
    let global = Layer {
        style: reader.uses_tabs(node.get("useTabs"), "useTabs"),
        width: reader.width(node.get("indentWidth"), "indentWidth"),
    };
    let plugin = match reader.target.file_type.as_deref() {
        Some("json" | "jsonc") => "json",
        _ => "typescript",
    };
    let block = match reader.mapping(node.get(plugin), plugin) {
        Some(block) => Layer {
            style: reader.uses_tabs(block.get("useTabs"), &format!("{plugin}.useTabs")),
            width: reader.width(block.get("indentWidth"), &format!("{plugin}.indentWidth")),
        },
        None => Layer::default(),
    };
    Outcome::Opinion(block.over(global))
}

fn ruff(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    let width = reader.width(node.get("indent-width"), "indent-width");
    let style = reader
        .mapping(node.get("format"), "format")
        .and_then(|format| reader.style(format.get("indent-style"), "format.indent-style", "space", "tab"));
    Outcome::Opinion(Layer { style, width })
}

fn tombi(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "overrides", OVERRIDES_NOT_APPLIED);
    let rules = reader
        .mapping(node.get("format"), "format")
        .and_then(|format| reader.mapping(format.get("rules"), "format.rules"));
    let Some(rules) = rules else {
        return Outcome::Opinion(Layer::default());
    };
    let style = reader.style(rules.get("indent-style"), "format.rules.indent-style", "space", "tab");
    // `indent-width` counts spaces; tabs keep the formatter's default width.
    let width = if style == Some(IndentStyle::Tabs) {
        None
    } else {
        reader.width(rules.get("indent-width"), "format.rules.indent-width")
    };
    Outcome::Opinion(Layer { style, width })
}

fn taplo(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "rule", "`[[rule]]` tables are not applied");
    let Some(formatting) = reader.mapping(node.get("formatting"), "formatting") else {
        return Outcome::Opinion(Layer::default());
    };
    let key = "formatting.indent_string";
    let Some(indent) = formatting.get("indent_string") else {
        return Outcome::Opinion(Layer::default());
    };
    let layer = match indent {
        Node::String(text) if !text.is_empty() && text.chars().all(|c| c == ' ') => match u8::try_from(text.len()) {
            Ok(width) => {
                reader.used(key);
                Layer { style: Some(IndentStyle::Spaces), width: NonZeroU8::new(width) }
            }
            Err(_) => {
                reader.invalid(key, "at most 255 spaces", indent);
                Layer::default()
            }
        },
        Node::String(text) if !text.is_empty() && text.chars().all(|c| c == '\t') => {
            reader.used(key);
            Layer { style: Some(IndentStyle::Tabs), width: None }
        }
        _ => {
            reader.invalid(key, "a string of only spaces or only tabs", indent);
            Layer::default()
        }
    };
    Outcome::Opinion(layer)
}

fn yamlfmt(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    let width = reader
        .mapping(node.get("formatter"), "formatter")
        .and_then(|formatter| reader.width(formatter.get("indent"), "formatter.indent"));
    Outcome::Opinion(Layer { style: None, width })
}

fn stylua(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    Outcome::Opinion(Layer {
        style: reader.style(node.get("indent_type"), "indent_type", "Spaces", "Tabs"),
        width: reader.width(node.get("indent_width"), "indent_width"),
    })
}

fn swift_format(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    let tab_width = reader.width(node.get("tabWidth"), "tabWidth");
    let Some(indentation) = node.get("indentation") else {
        return Outcome::Opinion(Layer::default());
    };
    let key = "indentation";
    let entries = indentation.as_map().unwrap_or_default();
    let [(kind, count)] = entries else {
        reader.invalid(key, "exactly one of `{\"spaces\": N}` or `{\"tabs\": N}`", indentation);
        return Outcome::Opinion(Layer::default());
    };
    let count_key = format!("indentation.{kind}");
    let layer = match kind.as_str() {
        "spaces" => {
            let width = reader.width(Some(count), &count_key);
            Layer { style: width.map(|_| IndentStyle::Spaces), width }
        }
        "tabs" => match reader.width(Some(count), &count_key) {
            Some(tabs) => {
                let per_tab = tab_width.map_or(8, NonZeroU8::get);
                match u8::try_from(u16::from(tabs.get()) * u16::from(per_tab)) {
                    Ok(width) => Layer { style: Some(IndentStyle::Tabs), width: NonZeroU8::new(width) },
                    Err(_) => {
                        reader.report(Some(&count_key), "times `tabWidth` exceeds 255 columns; ignored");
                        Layer::default()
                    }
                }
            }
            None => Layer::default(),
        },
        _ => {
            reader.invalid(key, "exactly one of `{\"spaces\": N}` or `{\"tabs\": N}`", indentation);
            Layer::default()
        }
    };
    Outcome::Opinion(layer)
}

fn rubocop(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "inherit_from", NOT_FOLLOWED);
    reader.unsupported(node, "inherit_gem", NOT_FOLLOWED);
    let style = reader
        .mapping(node.get("Layout/IndentationStyle"), "Layout/IndentationStyle")
        .and_then(|cop| reader.style(cop.get("EnforcedStyle"), "Layout/IndentationStyle.EnforcedStyle", "spaces", "tabs"));
    let width = reader
        .mapping(node.get("Layout/IndentationWidth"), "Layout/IndentationWidth")
        .and_then(|cop| reader.width(cop.get("Width"), "Layout/IndentationWidth.Width"));
    Outcome::Opinion(Layer { style, width })
}

fn csharpier(node: &Node, reader: &mut Reader<'_>) -> Outcome {
    reader.unsupported(node, "overrides", OVERRIDES_NOT_APPLIED);
    Outcome::Opinion(Layer {
        style: reader.uses_tabs(node.get("useTabs"), "useTabs"),
        width: reader.width(node.get("indentSize"), "indentSize"),
    })
}

/// clang-format's built-in presets, compared case-insensitively.
const CLANG_PRESETS: &[(&str, u8)] =
    &[("LLVM", 2), ("Google", 2), ("Chromium", 2), ("Mozilla", 2), ("GNU", 2), ("WebKit", 4), ("Microsoft", 4)];

/// clang-format: the document without `Language`, then the target
/// language's document, then the selected preset.
fn clang_format(documents: &[Node], reader: &mut Reader<'_>) -> Outcome {
    let language = match reader.target.file_type.as_deref() {
        Some("m" | "mm") => "ObjC",
        Some("proto") => "Proto",
        _ => "Cpp",
    };
    let mut base: Option<&Node> = None;
    let mut specific: Option<&Node> = None;
    let mut mappings = 0;
    for (index, document) in documents.iter().enumerate() {
        if document.as_map().is_none() {
            reader.report(None, format!("document {} must be a mapping, found {}; skipped", index + 1, document.describe()));
            continue;
        }
        mappings += 1;
        match document.get("Language") {
            None => {
                if base.is_some() {
                    reader.report(None, format!("document {} repeats the document without `Language`; ignored", index + 1));
                } else {
                    base = Some(document);
                }
            }
            Some(Node::String(name)) => {
                if name == language {
                    if specific.is_some() {
                        reader.report(Some("Language"), format!("document {} repeats `Language: {name}`; ignored", index + 1));
                    } else {
                        specific = Some(document);
                    }
                }
            }
            Some(other) => {
                reader.report(Some("Language"), format!("in document {} must be a string, found {}; ignored", index + 1, other.describe()));
            }
        }
    }
    if mappings == 0 {
        reader.report(None, "has no mapping document; skipped");
        return Outcome::Skipped;
    }
    if base.is_none() && specific.is_none() {
        return Outcome::NoOpinion;
    }

    let mut layer = Layer::default();
    let mut preset: Option<(&str, u8)> = None;
    for document in [specific, base].into_iter().flatten() {
        let this = Layer {
            style: clang_use_tab(document.get("UseTab"), reader),
            width: reader.width(document.get("IndentWidth"), "IndentWidth"),
        };
        layer = layer.over(this);
        if preset.is_none()
            && let Some(node) = document.get("BasedOnStyle")
        {
            preset = clang_preset(node, reader);
        }
    }
    let width = preset.map_or(2, |(_, width)| width);
    Outcome::Opinion(layer.over(Layer { style: Some(IndentStyle::Spaces), width: NonZeroU8::new(width) }))
}

fn clang_preset(node: &Node, reader: &mut Reader<'_>) -> Option<(&'static str, u8)> {
    let Node::String(name) = node else {
        reader.invalid("BasedOnStyle", "a preset name", node);
        return None;
    };
    match CLANG_PRESETS.iter().find(|(preset, _)| preset.eq_ignore_ascii_case(name)) {
        Some(preset) => {
            reader.used("BasedOnStyle");
            Some(*preset)
        }
        None => {
            reader.report(Some("BasedOnStyle"), format!("`{name}` is not a built-in preset Sniff knows; LLVM defaults apply"));
            None
        }
    }
}

fn clang_use_tab(node: Option<&Node>, reader: &mut Reader<'_>) -> Option<IndentStyle> {
    let node = node?;
    let style = match node {
        Node::String(mode) if mode == "Never" => Some(IndentStyle::Spaces),
        Node::String(mode)
            if matches!(mode.as_str(), "ForIndentation" | "ForContinuationAndIndentation" | "AlignWithSpaces" | "Always") =>
        {
            Some(IndentStyle::Tabs)
        }
        _ => None,
    };
    if style.is_some() {
        reader.used("UseTab");
    } else {
        reader.invalid(
            "UseTab",
            "one of `Never`, `ForIndentation`, `ForContinuationAndIndentation`, `AlignWithSpaces`, or `Always`",
            node,
        );
    }
    style
}
