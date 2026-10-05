//! The one terminal renderer of the declaration commands (`functions`,
//! `types`, `symbols`, `classes`), fed from v2 records.
//!
//! Every line is `Prose` markup inside a biscuit-terminal `UnorderedList`, so
//! color, emphasis and OSC 8 links degrade with the terminal's capabilities
//! and `--plain` is the same text without escapes. Lines never wrap: the
//! terminal width is pinned far beyond any line (wrapping arrives with the
//! later report work).

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::{
    Prose, Terminal, TerminalRenderable, UnorderedList, WordWrap, strip_escape_codes,
};
use tree_hugger::{
    FieldData, FunctionData, ParameterData, ProgrammingLanguage, SymbolKind, SymbolKindData,
    SymbolKindV2, SymbolRecord, Visibility, find_package_root,
};

use crate::classes::ClassSummary;
use crate::order::{SortOrder, compare, portable_path_cmp};
use crate::{color_forced, display_path, file_url};

/// A width no rendered line reaches, so the list never wraps.
const NO_WRAP_WIDTH: u32 = 1_000_000;

/// One file's selected records, ready to render.
pub(crate) struct FileListing {
    /// The file as analyzed.
    pub(crate) file: PathBuf,
    pub(crate) records: Vec<SymbolRecord>,
}

/// Ordering and grouping of a declaration listing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ListingOptions {
    pub(crate) group_by_file: bool,
    pub(crate) group_by_module: bool,
    pub(crate) sort: SortOrder,
}

/// Renders records as terminal text.
pub(crate) struct Renderer {
    term: Terminal,
    styled: bool,
    show_comments: bool,
    display_root: Option<PathBuf>,
}

/// A record with the display path it is listed under.
struct Listed<'a> {
    path: String,
    record: &'a SymbolRecord,
}

impl Renderer {
    /// A renderer that styles output when `styled` (color, emphasis, links
    /// where the terminal supports them) and otherwise prints plain text.
    pub(crate) fn new(styled: bool, show_comments: bool, display_root: Option<PathBuf>) -> Self {
        let mut term = if !styled {
            Terminal::builder()
                .is_tty(false)
                .color_depth(ColorDepth::None)
                .osc_link_support(false)
                .build()
        } else if color_forced() {
            // Forced color may run without a real TTY (CI, captured output,
            // tests); an optimistic terminal guarantees color and OSC 8.
            Terminal::new_optimistic(NO_WRAP_WIDTH)
        } else {
            Terminal::new()
        };
        term.fixed_width = Some(NO_WRAP_WIDTH);
        Self {
            term,
            styled,
            show_comments,
            display_root,
        }
    }

    /// The `functions`, `types` and `symbols` listing: one flat list (or file
    /// and directory groups), split into packages when more than one package
    /// has declarations.
    pub(crate) fn listing(
        &self,
        out: &mut dyn Write,
        files: &[FileListing],
        git_root: &Path,
        options: ListingOptions,
    ) -> io::Result<()> {
        let mut packages: BTreeMap<PathBuf, Vec<&FileListing>> = BTreeMap::new();
        for listing in files {
            let start = listing.file.parent().unwrap_or(listing.file.as_path());
            packages
                .entry(find_package_root(start, git_root))
                .or_default()
                .push(listing);
        }
        let packages_with_symbols = packages
            .values()
            .filter(|listings| listings.iter().any(|listing| !listing.records.is_empty()))
            .count();

        if packages.len() <= 1 || packages_with_symbols <= 1 {
            let all: Vec<&FileListing> = files.iter().collect();
            return self.package_listing(out, &all, options);
        }
        for (package_root, listings) in packages {
            writeln!(out)?;
            let heading = Prose::escape_text(&self.display(&package_root));
            self.line(out, 0, &format!("<blue><b>{heading}</b></blue>"))?;
            writeln!(out)?;
            self.package_listing(out, &listings, options)?;
            writeln!(out)?;
        }
        Ok(())
    }

    fn package_listing(
        &self,
        out: &mut dyn Write,
        files: &[&FileListing],
        options: ListingOptions,
    ) -> io::Result<()> {
        let mut listed: Vec<Listed<'_>> = files
            .iter()
            .flat_map(|listing| listing.records.iter())
            .map(|record| Listed {
                path: self.display(&record.source.file_path),
                record,
            })
            .collect();
        if listed.is_empty() {
            return self.line(out, 0, "<dim>(no symbols)</dim>");
        }
        listed.sort_by(|left, right| {
            compare(
                options.sort,
                (&left.path, left.record),
                (&right.path, right.record),
            )
        });

        let listed: Vec<&Listed<'_>> = listed.iter().collect();
        match (options.group_by_module, options.group_by_file) {
            (true, true) => {
                for (module, listed) in grouped(listed, |item| module_key(&item.path)) {
                    self.line(out, 0, &format!("<b>{}</b>", Prose::escape_text(module_title(&module))))?;
                    for (file, listed) in grouped(listed, |item| item.path.clone()) {
                        self.line(out, 2, &format!("<underline>{}</underline>", Prose::escape_text(&file)))?;
                        self.symbol_list(out, &listed, false)?;
                    }
                    writeln!(out)?;
                }
            }
            (true, false) => {
                for (module, listed) in grouped(listed, |item| module_key(&item.path)) {
                    self.line(out, 0, &format!("<b>{}</b>", Prose::escape_text(module_title(&module))))?;
                    self.symbol_list(out, &listed, true)?;
                    writeln!(out)?;
                }
            }
            (false, true) => {
                for (file, listed) in grouped(listed, |item| item.path.clone()) {
                    self.line(out, 0, &format!("<b>{}</b>", Prose::escape_text(&file)))?;
                    self.symbol_list(out, &listed, false)?;
                    writeln!(out)?;
                }
            }
            (false, false) => self.symbol_list(out, &listed, true)?,
        }
        Ok(())
    }

    /// One bullet per record: visibility, label, name with its compact
    /// signature, and location (`path:line:col`, or `[line:col]` inside a
    /// file group).
    fn symbol_list(&self, out: &mut dyn Write, listed: &[&Listed<'_>], include_file: bool) -> io::Result<()> {
        let mut list = UnorderedList::empty();
        for item in listed {
            let record = item.record;
            let start = &record.source.declaration_span.start;
            let location = if include_file {
                format!("{}:{}:{}", item.path, start.line, start.column)
            } else {
                format!("[{}:{}]", start.line, start.column)
            };
            let kind = SymbolKind::from(record.kind);
            let label = Prose::escape_text(&label(record));
            let label = match color_tag_for_kind(kind) {
                Some(tag) => format!("<{tag}>{label}</{tag}>"),
                None => label,
            };
            let visibility = match visibility(record) {
                Some(visibility) => format!("<i>{visibility}</i> "),
                None => String::new(),
            };
            let markup = format!(
                "{visibility}{label} <b>{}</b> <dim>{}</dim>{}",
                Prose::escape_text(&name_with_signature(record)),
                self.link(&record.source.file_path, start.line, &location),
                self.doc_lines(record.docs.raw_doc.as_deref()),
            );
            list.add(Prose::new(markup).with_word_wrap(WordWrap::None));
        }
        self.list(out, 2, &list)
    }

    /// The `classes` report of one file: the file header, then each
    /// container with its static and instance methods and fields.
    pub(crate) fn classes(
        &self,
        out: &mut dyn Write,
        file: &Path,
        language: ProgrammingLanguage,
        summaries: &[ClassSummary],
    ) -> io::Result<()> {
        let header = Prose::escape_text(&self.display(file));
        self.line(
            out,
            0,
            &format!(
                "<b>{}</b> (<dim>{}</dim>)",
                self.link(file, 1, &header),
                Prose::escape_text(&language.to_string())
            ),
        )?;
        for summary in summaries {
            let class = &summary.container;
            let start = &class.source.declaration_span.start;
            let location = format!("[{}:{}]", start.line, start.column);
            self.line(
                out,
                2,
                &format!(
                    "<magenta>{}</magenta> <b>{}</b> <dim>{}</dim>",
                    SymbolKind::from(class.kind),
                    Prose::escape_text(&class.identity.name),
                    self.link(&class.source.file_path, start.line, &location),
                ),
            )?;
            if let Some(doc) = self.trimmed_doc(class.docs.raw_doc.as_deref()) {
                for line in doc {
                    self.line(out, 4, &format!("<dim>{}</dim>", Prose::escape_text(line)))?;
                }
            }
            self.method_section(out, "Static Methods", &summary.static_methods)?;
            self.method_section(out, "Instance Methods", &summary.instance_methods)?;
            self.field_section(out, "Static Fields", &summary.static_fields, language)?;
            self.field_section(out, "Instance Fields", &summary.instance_fields, language)?;
        }
        writeln!(out)
    }

    fn method_section(&self, out: &mut dyn Write, title: &str, methods: &[SymbolRecord]) -> io::Result<()> {
        if methods.is_empty() {
            return Ok(());
        }
        self.line(out, 4, &format!("<yellow>{title}</yellow> ({})", methods.len()))?;
        let mut list = UnorderedList::empty();
        for method in methods {
            let start = &method.source.declaration_span.start;
            let location = format!("[{}:{}]", start.line, start.column);
            let visibility = match visibility(method) {
                Some(visibility) => format!("<i>{visibility}</i> "),
                None => String::new(),
            };
            let markup = format!(
                "{visibility}<green>{}</green> <dim>{}</dim>{}",
                Prose::escape_text(&name_with_signature(method)),
                self.link(&method.source.file_path, start.line, &location),
                self.doc_lines(method.docs.raw_doc.as_deref()),
            );
            list.add(Prose::new(markup).with_word_wrap(WordWrap::None));
        }
        self.list(out, 6, &list)
    }

    fn field_section(
        &self,
        out: &mut dyn Write,
        title: &str,
        fields: &[FieldData],
        language: ProgrammingLanguage,
    ) -> io::Result<()> {
        if fields.is_empty() {
            return Ok(());
        }
        self.line(out, 4, &format!("<yellow>{title}</yellow> ({})", fields.len()))?;
        let mut list = UnorderedList::empty();
        for field in fields {
            let visibility = match field.visibility.and_then(Option::<Visibility>::from) {
                Some(visibility) => format!("<i>{visibility}</i> "),
                None => String::new(),
            };
            let markup = format!(
                "{visibility}<cyan>{}</cyan>{}",
                Prose::escape_text(&format_field(&field.name, field.type_text.as_deref(), language)),
                self.doc_lines(field.docs.as_deref()),
            );
            list.add(Prose::new(markup).with_word_wrap(WordWrap::None));
        }
        self.list(out, 6, &list)
    }

    /// `path` as listed: relative to the display root, `/`-separated.
    pub(crate) fn display(&self, path: &Path) -> String {
        display_path(path, self.display_root.as_deref())
    }

    /// `text` as an OSC 8 link to `path` at `line` when the terminal follows
    /// links, else `text` alone. `text` must already be escaped.
    fn link(&self, path: &Path, line: u32, text: &str) -> String {
        match file_url(path).filter(|_| self.term.osc_link_support && self.styled) {
            Some(url) => format!("<a href={}>{text}</a>", Prose::quoted_attr(&format!("{url}#L{line}"))),
            None => text.to_string(),
        }
    }

    /// The doc comment's non-blank lines, trimmed, when `--comments` is on.
    fn trimmed_doc<'a>(&self, doc: Option<&'a str>) -> Option<Vec<&'a str>> {
        if !self.show_comments {
            return None;
        }
        let lines: Vec<&str> = doc?.trim().lines().map(str::trim).filter(|line| !line.is_empty()).collect();
        (!lines.is_empty()).then_some(lines)
    }

    /// Doc lines as continuation lines of a list item: the list's hanging
    /// indent aligns them under the item text.
    fn doc_lines(&self, doc: Option<&str>) -> String {
        self.trimmed_doc(doc)
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| format!("\n<dim>{}</dim>", Prose::escape_text(line)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn list(&self, out: &mut dyn Write, indent: usize, list: &UnorderedList) -> io::Result<()> {
        let rendered = list.render(&self.term);
        self.write_indented(out, indent, &rendered)
    }

    fn line(&self, out: &mut dyn Write, indent: usize, markup: &str) -> io::Result<()> {
        let rendered = Prose::new(markup).with_word_wrap(WordWrap::None).render(&self.term);
        self.write_indented(out, indent, &rendered)
    }

    fn write_indented(&self, out: &mut dyn Write, indent: usize, rendered: &str) -> io::Result<()> {
        // Emphasis SGR is not gated by color depth, so plain output strips
        // whatever escapes remain.
        let rendered = if self.styled {
            rendered.to_string()
        } else {
            strip_escape_codes(rendered.to_string())
        };
        let padding = " ".repeat(indent);
        if rendered.is_empty() {
            // An empty heading (the scan root's own package) still takes a line.
            return writeln!(out);
        }
        for line in rendered.lines() {
            writeln!(out, "{padding}{line}")?;
        }
        Ok(())
    }
}

/// Splits sorted items into groups by `key`, keeping each group's order and
/// ordering the groups by their portable path.
fn grouped<'a, 'b>(
    items: impl IntoIterator<Item = &'b Listed<'a>>,
    key: impl Fn(&Listed<'a>) -> String,
) -> Vec<(String, Vec<&'b Listed<'a>>)>
where
    'a: 'b,
{
    let mut groups: Vec<(String, Vec<&Listed<'a>>)> = Vec::new();
    let mut positions: HashMap<String, usize> = HashMap::new();
    for item in items {
        let group = key(item);
        let position = *positions.entry(group.clone()).or_insert_with(|| {
            groups.push((group, Vec::new()));
            groups.len() - 1
        });
        groups[position].1.push(item);
    }
    groups.sort_by(|(left, _), (right, _)| portable_path_cmp(left, right));
    groups
}

/// The directory part of a display path (`""` at the root).
fn module_key(path: &str) -> String {
    path.rsplit_once('/').map(|(directory, _)| directory.to_string()).unwrap_or_default()
}

fn module_title(module: &str) -> &str {
    if module.is_empty() { "(root)" } else { module }
}

/// The word before a declaration's name: its declaring keyword for the
/// kinds whose syntax names them (`struct`, `let`, `const`, ...), else the
/// kind.
fn label(record: &SymbolRecord) -> String {
    let keyword_names_it = matches!(
        record.kind,
        SymbolKindV2::Type | SymbolKindV2::Variable | SymbolKindV2::Field | SymbolKindV2::Constant
    );
    match &record.identity.keyword {
        Some(keyword) if keyword_names_it => keyword.clone(),
        _ => SymbolKind::from(record.kind).to_string(),
    }
}

fn visibility(record: &SymbolRecord) -> Option<Visibility> {
    record.visibility.level.into()
}

/// Prose color tag for a symbol kind.
pub(crate) fn color_tag_for_kind(kind: SymbolKind) -> Option<&'static str> {
    match kind {
        SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor => Some("green"),
        SymbolKind::Type | SymbolKind::TypeAlias | SymbolKind::Class | SymbolKind::Interface => {
            Some("magenta")
        }
        SymbolKind::Enum | SymbolKind::EnumVariant | SymbolKind::Field | SymbolKind::Property => {
            Some("cyan")
        }
        SymbolKind::Trait | SymbolKind::Namespace | SymbolKind::Module => Some("yellow"),
        SymbolKind::Variable | SymbolKind::Parameter => Some("blue"),
        SymbolKind::Macro => Some("red"),
        SymbolKind::Constant => Some("bright-blue"),
        SymbolKind::Unknown => None,
    }
}

/// The name with its compact signature, and the container it is declared
/// in for members:
///
/// - functions and methods: `name(param: T, ...) -> R` (a synthetic prelude
///   export, which has no signature of its own, is the bare name)
/// - types: `name { field: T, ... }` or `name { Variant, ... }`
/// - members: `... [in <container kind> <container>]`
pub(crate) fn name_with_signature(record: &SymbolRecord) -> String {
    let name = &record.identity.name;
    let language = record.language;
    let kind = SymbolKind::from(record.kind);
    let base = if kind.is_function() {
        match &record.kind_data {
            SymbolKindData::Function(function) => format_function(name, function, language),
            _ if record.visibility.is_synthetic => name.clone(),
            _ => format!("{name}()"),
        }
    } else if kind.is_type() {
        match &record.kind_data {
            SymbolKindData::Type(data) if !data.fields.is_empty() => {
                let fields: Vec<String> = data
                    .fields
                    .iter()
                    .map(|field| format_field(&field.name, field.type_text.as_deref(), language))
                    .collect();
                format!("{name} {{ {} }}", fields.join(", "))
            }
            SymbolKindData::Enum(data) if !data.variants.is_empty() => {
                let variants: Vec<String> = data
                    .variants
                    .iter()
                    .map(|variant| {
                        if !variant.tuple_fields.is_empty() {
                            format!("{}({})", variant.name, variant.tuple_fields.join(", "))
                        } else if !variant.struct_fields.is_empty() {
                            let fields: Vec<String> = variant
                                .struct_fields
                                .iter()
                                .map(|field| match &field.type_text {
                                    Some(ty) => format!("{}: {ty}", field.name),
                                    None => field.name.clone(),
                                })
                                .collect();
                            format!("{} {{ {} }}", variant.name, fields.join(", "))
                        } else {
                            variant.name.clone()
                        }
                    })
                    .collect();
                format!("{name} {{ {} }}", variants.join(", "))
            }
            _ => name.clone(),
        }
    } else {
        name.clone()
    };

    let is_member_kind = matches!(
        kind,
        SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Function
            | SymbolKind::Field
            | SymbolKind::Property
            | SymbolKind::Constant
            | SymbolKind::EnumVariant
            | SymbolKind::Parameter
    );
    match record.identity.container_name.as_deref() {
        Some(container) if is_member_kind => match record.identity.container_kind {
            Some(container_kind) => {
                format!("{base} [in {} {container}]", SymbolKind::from(container_kind))
            }
            None => format!("{base} [in {container}]"),
        },
        _ => base,
    }
}

fn format_function(name: &str, function: &FunctionData, language: ProgrammingLanguage) -> String {
    let parameters: Vec<String> = function
        .parameters
        .iter()
        .map(|parameter| format_parameter(parameter, language))
        .collect();
    let return_part = match &function.return_info.type_text {
        Some(ty) => match language {
            ProgrammingLanguage::Go => format!(" {ty}"),
            ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => format!(": {ty}"),
            _ => format!(" -> {ty}"),
        },
        None => String::new(),
    };
    format!("{name}({}){return_part}", parameters.join(", "))
}

fn format_parameter(parameter: &ParameterData, language: ProgrammingLanguage) -> String {
    let mut text = String::new();
    if parameter.is_variadic {
        match language {
            ProgrammingLanguage::Python => text.push('*'),
            ProgrammingLanguage::Go
            | ProgrammingLanguage::JavaScript
            | ProgrammingLanguage::TypeScript => text.push_str("..."),
            _ => {}
        }
    }
    text.push_str(&parameter.name);
    if let Some(ty) = &parameter.type_text {
        match language {
            ProgrammingLanguage::Go => {
                text.push(' ');
                text.push_str(ty);
            }
            _ => {
                text.push_str(": ");
                text.push_str(ty);
            }
        }
    }
    if let Some(default) = &parameter.default_value {
        text.push_str(" = ");
        text.push_str(default);
    }
    text
}

fn format_field(name: &str, type_text: Option<&str>, language: ProgrammingLanguage) -> String {
    match type_text {
        Some(ty) if language == ProgrammingLanguage::Go => format!("{name} {ty}"),
        Some(ty) => format!("{name}: {ty}"),
        None => name.to_string(),
    }
}
