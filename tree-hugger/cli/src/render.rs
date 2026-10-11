//! The shared declaration renderer for the commands (`functions`,
//! `types`, `symbols`, `classes`) and of `refs`, fed from v2 records.
//!
//! Every line is `Prose` markup (inside a biscuit-terminal `UnorderedList`
//! for listings) or a biscuit-terminal `Table` row, so color, emphasis and
//! OSC 8 links degrade with the terminal's capabilities and plain output is
//! the same text without escapes. Interactive output wraps at the terminal
//! width with a hanging indent; plain and redirected output never wraps.
//! Source-derived text always goes through [`literal`], so a control
//! sequence or markup in a name, signature or comment is shown, never acted
//! on.
//!
//! The listing, outline and `classes` routines write to a [`Layout`], not to
//! bytes: [`TerminalLayout`] renders each line for the terminal, while the
//! `--format prose` and `--format markdown` exports (`prose.rs`,
//! `markdown.rs`) write the same markup unrendered or as CommonMark. All three
//! therefore share buffering, grouping, ordering, outline ownership and class
//! sections.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::{
    InlineProse, LineBreaks, Prose, Table, TableCellContent, TableColumn, Terminal, TerminalRenderable, UnorderedList,
    WordWrap, strip_escape_codes,
};
use tree_hugger::{
    FieldData, FunctionData, ParameterData, ProgrammingLanguage, SymbolId, SymbolKind,
    SymbolKindData, SymbolKindV2, SymbolRecord, Visibility, find_package_root,
};

use crate::classes::ClassSummary;
use crate::order::{SortOrder, compare, kind_rank, portable_path_cmp};
use crate::output::SignatureMode;
use crate::report::portable_relative;
use crate::{color_forced, display_path, file_url};

/// A width no rendered line reaches, so unwrapped output never wraps.
const NO_WRAP_WIDTH: u32 = 1_000_000;

/// The narrowest a wrapped line is laid out, however deep its indent.
const MIN_WRAP_WIDTH: u32 = 20;

/// Entries of a type's field or variant list shown by `--signatures short`
/// before the rest is summarized as `… +N`.
const SHORT_LIST_ENTRIES: usize = 4;

/// Indents [`Renderer`] keeps a laid-out terminal for; the listings never
/// go deeper.
const INDENTS: usize = 16;

/// The line a report with no declaration shows.
pub(crate) const NO_SYMBOLS: &str = "<dim>(no symbols)</dim>";

/// Where a declaration listing is written. The listing routines decide
/// content, order and nesting; a layout decides only presentation. Every
/// `markup` argument is Prose markup whose generated tags close within each
/// line; lines after the first are the item's doc lines.
pub(crate) trait Layout {
    /// A heading: `level` 1 is a package section, 2 a module or file group
    /// or a file's outline or class header, 3 a file inside a module group.
    fn heading(&mut self, level: u8, indent: usize, markup: &str) -> io::Result<()>;
    /// One outline row: a declaration, an `[in …]` context or a class
    /// section title.
    fn row(&mut self, row: Row<'_>) -> io::Result<()>;
    /// Declarations as bullets at `indent`, nested `depth` levels deep.
    fn list(&mut self, indent: usize, depth: usize, items: &[String]) -> io::Result<()>;
    /// A line that is neither heading nor declaration: `(no symbols)`.
    fn note(&mut self, indent: usize, markup: &str) -> io::Result<()>;
    fn blank(&mut self) -> io::Result<()>;
    /// The writer underneath, flushed at chunk ends.
    fn writer(&mut self) -> &mut dyn Write;
}

/// One row of an outline or `classes` report.
pub(crate) struct Row<'a> {
    /// Nesting below the file's top-level rows.
    pub(crate) depth: usize,
    /// Terminal indent of the first line.
    pub(crate) indent: usize,
    /// Terminal indent of wrapped continuation lines, beyond `indent`.
    pub(crate) hanging: u32,
    pub(crate) markup: &'a str,
    /// Terminal indent of each doc line on its own line; `None` keeps the
    /// doc lines inside the row's wrapped text, flush with its first line.
    pub(crate) doc_indent: Option<usize>,
}

/// [`Layout`] for the terminal: each line rendered as the renderer's
/// terminal allows.
pub(crate) struct TerminalLayout<'r, 'w> {
    renderer: &'r Renderer,
    out: &'w mut dyn Write,
}

impl<'r, 'w> TerminalLayout<'r, 'w> {
    pub(crate) fn new(renderer: &'r Renderer, out: &'w mut dyn Write) -> Self {
        Self { renderer, out }
    }
}

impl Layout for TerminalLayout<'_, '_> {
    fn heading(&mut self, _level: u8, indent: usize, markup: &str) -> io::Result<()> {
        self.renderer.line(self.out, indent, markup)
    }

    fn row(&mut self, row: Row<'_>) -> io::Result<()> {
        let Some(doc_indent) = row.doc_indent else {
            return self.renderer.hanging_line(self.out, row.indent, row.hanging, row.markup);
        };
        let mut lines = row.markup.split('\n');
        self.renderer.hanging_line(self.out, row.indent, row.hanging, lines.next().unwrap_or_default())?;
        for doc in lines {
            self.renderer.line(self.out, doc_indent, doc)?;
        }
        Ok(())
    }

    fn list(&mut self, indent: usize, _depth: usize, items: &[String]) -> io::Result<()> {
        let mut list = UnorderedList::empty();
        for item in items {
            // The list gives each item its hanging indent and wraps it at the
            // renderer's width (never, when that width is unbounded).
            list.add(Prose::new(item.as_str()).with_line_breaks(LineBreaks::Hard).with_word_wrap(WordWrap::None));
        }
        self.renderer.list(self.out, indent, &list)
    }

    fn note(&mut self, indent: usize, markup: &str) -> io::Result<()> {
        self.renderer.line(self.out, indent, markup)
    }

    fn blank(&mut self) -> io::Result<()> {
        writeln!(self.out)
    }

    fn writer(&mut self) -> &mut dyn Write {
        self.out
    }
}

/// One file's selected records, ready to render.
pub(crate) struct FileListing {
    /// The file as analyzed.
    pub(crate) file: PathBuf,
    pub(crate) records: Vec<SymbolRecord>,
    /// The file's unit as [`ListingStream`] writes it, rendered ahead (on
    /// the worker that analyzed the file); `records` is then empty.
    rendered: Option<Vec<u8>>,
}

impl FileListing {
    pub(crate) fn new(file: PathBuf, records: Vec<SymbolRecord>) -> Self {
        Self { file, records, rendered: None }
    }

    /// A file whose unit was rendered ahead into `bytes`, for a stream whose
    /// units are files ([`ListingStream::units_are_files`]) by a layout that
    /// carries no state from one unit to the next, so the bytes equal what
    /// the stream would write. Only a file with records has one.
    pub(crate) fn rendered(file: PathBuf, bytes: Vec<u8>) -> Self {
        Self { file, records: Vec::new(), rendered: Some(bytes) }
    }

    fn has_symbols(&self) -> bool {
        self.rendered.is_some() || !self.records.is_empty()
    }
}

/// Ordering and grouping of a declaration listing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ListingOptions {
    pub(crate) group_by_file: bool,
    pub(crate) group_by_module: bool,
    pub(crate) sort: SortOrder,
}

/// How a [`Renderer`] writes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RenderOptions {
    /// Color, emphasis and links where the terminal supports them.
    pub(crate) styled: bool,
    /// `--comments`.
    pub(crate) show_comments: bool,
    /// The width to wrap at, or `None` to never wrap.
    pub(crate) width: Option<u32>,
    pub(crate) signatures: SignatureMode,
}

/// Builds the Prose markup of declarations and renders it as terminal
/// text; the exports write the same markup through their own [`Layout`].
pub(crate) struct Renderer {
    term: Terminal,
    /// `term` laid out at each indent up to [`INDENTS`] when wrapping, so
    /// [`Self::term_at`] does not copy the terminal per line.
    indented: Vec<Terminal>,
    styled: bool,
    show_comments: bool,
    width: Option<u32>,
    signatures: SignatureMode,
    display_root: Option<PathBuf>,
}

/// A record with the display path it is listed under.
struct Listed<'a> {
    path: String,
    record: &'a SymbolRecord,
}

impl Renderer {
    /// A renderer writing as `options` says, with paths shown relative to
    /// `display_root`.
    pub(crate) fn new(options: RenderOptions, display_root: Option<PathBuf>) -> Self {
        let RenderOptions {
            styled,
            show_comments,
            width,
            signatures,
        } = options;
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
        term.fixed_width = Some(width.unwrap_or(NO_WRAP_WIDTH));
        let indented = match width {
            Some(width) => (0..=INDENTS).map(|indent| wrapped_at(&term, width, indent)).collect(),
            None => Vec::new(),
        };
        Self {
            term,
            indented,
            styled,
            show_comments,
            width,
            signatures,
            display_root,
        }
    }

    /// A renderer for the Prose and Markdown exports: its markup always
    /// carries styles and source-location links, independent of the
    /// terminal running `hug`; each export's [`Layout`] keeps or drops them.
    pub(crate) fn for_export(show_comments: bool, signatures: SignatureMode, display_root: Option<PathBuf>) -> Self {
        let mut term = Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .osc_link_support(true)
            .build();
        term.fixed_width = Some(NO_WRAP_WIDTH);
        Self {
            term,
            indented: Vec::new(),
            styled: true,
            show_comments,
            width: None,
            signatures,
            display_root,
        }
    }

    /// The terminal content indented by `indent` columns is laid out on.
    fn term_at(&self, indent: usize) -> Cow<'_, Terminal> {
        match (self.width, self.indented.get(indent)) {
            (Some(_), Some(term)) => Cow::Borrowed(term),
            (Some(width), None) => Cow::Owned(wrapped_at(&self.term, width, indent)),
            (None, _) => Cow::Borrowed(&self.term),
        }
    }

    /// How a line wraps: with a hanging indent of `hanging` columns when
    /// wrapping, else never.
    fn wrap(&self, hanging: u32) -> WordWrap {
        match self.width {
            Some(_) => WordWrap::default().with_hanging_indent(hanging),
            None => WordWrap::None,
        }
    }

    /// One package's listing, or one unit of it (see [`ListingStream`]).
    pub(crate) fn package_listing(
        &self,
        out: &mut dyn Layout,
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
            return out.note(0, NO_SYMBOLS);
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
                    out.heading(2, 0, &format!("<b>{}</b>", literal(module_title(&module))))?;
                    for (file, listed) in grouped(listed, |item| item.path.clone()) {
                        out.heading(3, 2, &format!("<underline>{}</underline>", literal(&file)))?;
                        self.symbol_list(out, &listed, false)?;
                    }
                    out.blank()?;
                }
            }
            (true, false) => {
                for (module, listed) in grouped(listed, |item| module_key(&item.path)) {
                    out.heading(2, 0, &format!("<b>{}</b>", literal(module_title(&module))))?;
                    self.symbol_list(out, &listed, true)?;
                    out.blank()?;
                }
            }
            (false, true) => {
                for (file, listed) in grouped(listed, |item| item.path.clone()) {
                    out.heading(2, 0, &format!("<b>{}</b>", literal(&file)))?;
                    self.symbol_list(out, &listed, false)?;
                    out.blank()?;
                }
            }
            (false, false) => self.symbol_list(out, &listed, true)?,
        }
        Ok(())
    }

    /// One bullet per record: the declaration in the signature mode (see
    /// [`Self::declaration`]) and location (`path:line:col`, or `[line:col]` inside a
    /// file group).
    fn symbol_list(&self, out: &mut dyn Layout, listed: &[&Listed<'_>], include_file: bool) -> io::Result<()> {
        let items: Vec<String> = listed
            .iter()
            .map(|item| {
                let record = item.record;
                let start = &record.source.declaration_span.start;
                let location = if include_file {
                    format!("{}:{}:{}", item.path, start.line, start.column)
                } else {
                    format!("[{}:{}]", start.line, start.column)
                };
                format!(
                    "{} <dim>{}</dim>{}",
                    self.declaration(record, true),
                    self.link(&record.source.file_path, start.line, &literal(&location)),
                    self.doc_lines(record.docs.raw_doc.as_deref()),
                )
            })
            .collect();
        out.list(2, 0, &items)
    }

    /// The `classes` report of one file: the file header, then each
    /// container with its static and instance methods and fields.
    pub(crate) fn classes(
        &self,
        out: &mut dyn Layout,
        file: &Path,
        language: ProgrammingLanguage,
        summaries: &[ClassSummary],
    ) -> io::Result<()> {
        let header = literal(&self.display(file));
        out.heading(
            2,
            0,
            &format!(
                "<b>{}</b> (<dim>{}</dim>)",
                self.link(file, 1, &header),
                literal(&language.to_string())
            ),
        )?;
        for summary in summaries {
            let class = &summary.container;
            let start = &class.source.declaration_span.start;
            let location = format!("[{}:{}]", start.line, start.column);
            let markup = format!(
                "<magenta>{}</magenta> <b>{}</b> <dim>{}</dim>{}",
                SymbolKind::from(class.kind),
                literal(&class.identity.name),
                self.link(&class.source.file_path, start.line, &location),
                self.doc_lines(class.docs.raw_doc.as_deref()),
            );
            out.row(Row {
                depth: 0,
                indent: 2,
                hanging: 2,
                markup: &markup,
                doc_indent: Some(4),
            })?;
            self.method_section(out, "Static Methods", &summary.static_methods)?;
            self.method_section(out, "Instance Methods", &summary.instance_methods)?;
            self.field_section(out, "Static Fields", &summary.static_fields, language)?;
            self.field_section(out, "Instance Fields", &summary.instance_fields, language)?;
        }
        out.blank()
    }

    /// `--format table`: one row per record, in the given order, with the
    /// Location, Kind, Visibility, Name and (unless `--signatures none`)
    /// Signature columns. Only the Signature column wraps. The records are
    /// native (not `trim_record`ed): Location shows each path relative to
    /// `scan_root` and links the native path, so a name that is not valid
    /// Unicode never links to its replacement spelling.
    pub(crate) fn table(&self, out: &mut dyn Write, records: &[&SymbolRecord], scan_root: &Path) -> io::Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let unwrapped = |header: &str| TableColumn::new(header).with_word_wrap(WordWrap::None);
        let mut columns = vec![
            unwrapped("Location"),
            unwrapped("Kind"),
            unwrapped("Visibility"),
            unwrapped("Name"),
        ];
        let with_signature = self.signatures != SignatureMode::None;
        if with_signature {
            columns.push(TableColumn::new("Signature"));
        }
        let rows: Vec<Vec<TableCellContent>> = records
            .iter()
            .map(|record| {
                let line = record.source.declaration_span.start.line;
                let relative = portable_relative(&record.source.file_path, scan_root);
                let location = format!("{}:{line}", self.display(Path::new(&relative)));
                let linked = self.link(&record.source.file_path, line, &literal(&location));
                let location: TableCellContent = if linked.starts_with("<a ") {
                    InlineProse::new(linked).into()
                } else {
                    neutralized(&location).into()
                };
                let visibility = visibility(record).map(|visibility| visibility.to_string()).unwrap_or_default();
                let mut row = vec![
                    location,
                    SymbolKind::from(record.kind).to_string().into(),
                    visibility.into(),
                    neutralized(&record.identity.name).into(),
                ];
                if with_signature {
                    row.push(neutralized(&signature_cell(record, self.signatures)).into());
                }
                row
            })
            .collect();
        let table = Table::new().with_columns(columns).with_data(rows);
        self.write_indented(out, 0, &table.render(&table_terminal(&table, &self.term)))?;
        writeln!(out)
    }

    /// The terminal this renderer lays out on.
    pub(crate) fn terminal(&self) -> &Terminal {
        &self.term
    }

    /// The directory `file` is listed under by `--group-by-module`.
    pub(crate) fn module_of(&self, file: &Path) -> String {
        module_key(&self.display(file))
    }

    /// The `--group-by-module` heading of `--tree`: the directory of the
    /// file listed next.
    pub(crate) fn module_heading(&self, out: &mut dyn Layout, file: &Path) -> io::Result<()> {
        out.heading(2, 0, &format!("<b>{}</b>", literal(module_title(&self.module_of(file)))))
    }

    /// The line a report with no declaration prints.
    pub(crate) fn no_symbols(&self, out: &mut dyn Write) -> io::Result<()> {
        self.line(out, 0, NO_SYMBOLS)
    }

    /// `--tree`: one file's outline. Each record is nested under the
    /// selected record its `relations.container` names (by ID, so
    /// out-of-body Rust `impl` and Go receiver methods join their type, and
    /// repeated `impl` blocks share it). A member whose container was not
    /// selected or is in another file is listed under a marked contextual
    /// heading, which is not itself a listed declaration; a member with no
    /// same-file container is headed by its in-file owner path
    /// (`[in other::W]` for `impl other::W`, never the local `W`).
    /// A type row ends with its declared bases ([`Self::declared_bases`]).
    /// `name_prefix` is the prefix the records' qualified names carry.
    /// Siblings are ordered by `sort`. `in_module` says the file heading sits
    /// under a `--group-by-module` heading. Returns whether anything was
    /// written.
    pub(crate) fn outline(
        &self,
        out: &mut dyn Layout,
        file: &Path,
        records: &[SymbolRecord],
        name_prefix: &str,
        sort: SortOrder,
        in_module: bool,
    ) -> io::Result<bool> {
        if records.is_empty() {
            return Ok(false);
        }
        let display = self.display(file);
        let outline = Outline::new(records, name_prefix, &display, sort);
        let header = self.link(file, 1, &literal(&display));
        out.heading(if in_module { 3 } else { 2 }, 0, &format!("<b>{header}</b>"))?;
        let mut written = vec![false; records.len()];
        for root in &outline.roots {
            match root {
                Root::Record(position) => self.outline_node(out, &outline, *position, 1, &mut written)?,
                Root::Context { kind, name, members } => {
                    let kind = kind.map(|kind| format!("{} ", SymbolKind::from(kind))).unwrap_or_default();
                    out.row(Row {
                        depth: 0,
                        indent: 2,
                        hanging: 2,
                        markup: &format!("<dim>[in {}{}]</dim>", literal(&kind), literal(name)),
                        doc_indent: None,
                    })?;
                    for member in members {
                        self.outline_node(out, &outline, *member, 2, &mut written)?;
                    }
                }
            }
        }
        out.blank()?;
        Ok(true)
    }

    /// One outline row and its members; top-level rows are at `depth` 1.
    fn outline_node(
        &self,
        out: &mut dyn Layout,
        outline: &Outline<'_>,
        position: usize,
        depth: usize,
        written: &mut [bool],
    ) -> io::Result<()> {
        if std::mem::replace(&mut written[position], true) {
            return Ok(());
        }
        let record = &outline.records[position];
        let start = &record.source.declaration_span.start;
        let location = format!("[{}:{}]", start.line, start.column);
        let markup = format!(
            "{}{} <dim>{}</dim>{}",
            self.declaration(record, false),
            self.declared_bases(record),
            self.link(&record.source.file_path, start.line, &literal(&location)),
            self.doc_lines(record.docs.raw_doc.as_deref()),
        );
        out.row(Row {
            depth: depth - 1,
            indent: 2 * depth,
            hanging: 4,
            markup: &markup,
            doc_indent: None,
        })?;
        for child in &outline.children[position] {
            self.outline_node(out, outline, *child, depth + 1, written)?;
        }
        Ok(())
    }

    /// ` extends A, B; implements C` for a type row of `--tree`: the
    /// record's classified and unclassified bases as written,
    /// in source order, empty lists left out. Only type kinds get one, so a
    /// Rust method's trait relation never heads its row. A list the
    /// `--signatures full` header already writes is not repeated.
    fn declared_bases(&self, record: &SymbolRecord) -> String {
        if !SymbolKind::from(record.kind).is_type() {
            return String::new();
        }
        let shows_header = self.signatures == SignatureMode::Full && record.source.signature_text.is_some();
        let relations = &record.relations;
        let clauses: Vec<String> = [
            ("extends", &relations.extends, BaseList::Extends),
            ("implements", &relations.implements, BaseList::Implements),
            ("unclassified bases", &relations.unclassified_bases, BaseList::Unclassified),
        ]
        .into_iter()
        .filter_map(|(keyword, bases, list)| {
            let names: Vec<String> = bases.iter()
                .filter(|base| !(shows_header && header_writes(record, list, base)))
                .map(|base| literal(&base.name)).collect();
            (!names.is_empty()).then(|| format!("{keyword} {}", names.join(", ")))
        })
        .collect();
        if clauses.is_empty() {
            String::new()
        } else {
            format!(" {}", clauses.join("; "))
        }
    }

    /// A class section's title row, `Static Methods (2)`, above its items.
    fn section_title(&self, out: &mut dyn Layout, title: &str, len: usize) -> io::Result<()> {
        out.row(Row {
            depth: 1,
            indent: 4,
            hanging: 2,
            markup: &format!("<yellow>{title}</yellow> ({len})"),
            doc_indent: None,
        })
    }

    fn method_section(&self, out: &mut dyn Layout, title: &str, methods: &[SymbolRecord]) -> io::Result<()> {
        if methods.is_empty() {
            return Ok(());
        }
        self.section_title(out, title, methods.len())?;
        let items: Vec<String> = methods
            .iter()
            .map(|method| {
                let start = &method.source.declaration_span.start;
                let location = format!("[{}:{}]", start.line, start.column);
                let visibility = match visibility(method) {
                    Some(visibility) => format!("<i>{visibility}</i> "),
                    None => String::new(),
                };
                let text = match shown(method, self.signatures, true) {
                    Shown::Header(header) => literal(&header),
                    Shown::Named(named) => format!("{visibility}{}", literal(&named)),
                };
                format!(
                    "<green>{text}</green> <dim>{}</dim>{}",
                    self.link(&method.source.file_path, start.line, &location),
                    self.doc_lines(method.docs.raw_doc.as_deref()),
                )
            })
            .collect();
        out.list(6, 2, &items)
    }

    fn field_section(
        &self,
        out: &mut dyn Layout,
        title: &str,
        fields: &[FieldData],
        language: ProgrammingLanguage,
    ) -> io::Result<()> {
        if fields.is_empty() {
            return Ok(());
        }
        self.section_title(out, title, fields.len())?;
        let items: Vec<String> = fields
            .iter()
            .map(|field| {
                let visibility = match field.visibility.and_then(Option::<Visibility>::from) {
                    Some(visibility) => format!("<i>{visibility}</i> "),
                    None => String::new(),
                };
                format!(
                    "{visibility}<cyan>{}</cyan>{}",
                    literal(&format_field(&field.name, field.type_text.as_deref(), language)),
                    self.doc_lines(field.docs.as_deref()),
                )
            })
            .collect();
        out.list(6, 2, &items)
    }

    /// `path` as listed: relative to the display root, `/`-separated.
    pub(crate) fn display(&self, path: &Path) -> String {
        display_path(path, self.display_root.as_deref())
    }

    /// `text` as an OSC 8 link to `path` at `line` when the terminal follows
    /// links, else `text` alone. `text` must already be escaped.
    pub(crate) fn link(&self, path: &Path, line: u32, text: &str) -> String {
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
    pub(crate) fn doc_lines(&self, doc: Option<&str>) -> String {
        self.trimmed_doc(doc)
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| format!("\n<dim>{}</dim>", literal(line)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn list(&self, out: &mut dyn Write, indent: usize, list: &UnorderedList) -> io::Result<()> {
        let rendered = list.render(&self.term_at(indent));
        self.write_indented(out, indent, &rendered)
    }

    pub(crate) fn line(&self, out: &mut dyn Write, indent: usize, markup: &str) -> io::Result<()> {
        self.hanging_line(out, indent, 2, markup)
    }

    /// A line whose wrapped continuation lines are indented by `hanging`
    /// columns more than the first.
    fn hanging_line(&self, out: &mut dyn Write, indent: usize, hanging: u32, markup: &str) -> io::Result<()> {
        let rendered = Prose::new(markup)
            .with_line_breaks(LineBreaks::Hard)
            .with_word_wrap(self.wrap(hanging))
            .render(&self.term_at(indent));
        self.write_indented(out, indent, &rendered)
    }

    /// A declaration as listed, without its location: in `--signatures
    /// full`, the header as written (falling back to the short form when
    /// none was extracted); otherwise the visibility, the label and the
    /// name with the mode's signature. `with_container` appends the
    /// `[in <kind> <container>]` suffix of flat listings.
    pub(crate) fn declaration(&self, record: &SymbolRecord, with_container: bool) -> String {
        match shown(record, self.signatures, with_container) {
            Shown::Header(header) => format!("<b>{}</b>", literal(&header)),
            Shown::Named(named) => {
                let label = literal(&label(record));
                let label = match color_tag_for_kind(SymbolKind::from(record.kind)) {
                    Some(tag) => format!("<{tag}>{label}</{tag}>"),
                    None => label,
                };
                let visibility = match visibility(record) {
                    Some(visibility) => format!("<i>{visibility}</i> "),
                    None => String::new(),
                };
                format!("{visibility}{label} <b>{}</b>", literal(&named))
            }
        }
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

/// `term` wrapping at `width` less `indent` columns, but never narrower than
/// [`MIN_WRAP_WIDTH`].
fn wrapped_at(term: &Terminal, width: u32, indent: usize) -> Terminal {
    let mut term = term.clone();
    let indent = u32::try_from(indent).unwrap_or(u32::MAX);
    term.fixed_width = Some(width.saturating_sub(indent).max(MIN_WRAP_WIDTH));
    term
}

/// The `functions`, `types` and `symbols` listing, written as files arrive:
/// one flat list (or file and directory groups), split into package sections
/// when the scan's files span more than one package.
///
/// Files must arrive in [`listing_order`]. Each arriving file joins the
/// current unit, and a unit is rendered as soon as a file of another unit
/// arrives, so only the current unit is held:
///
/// - `--sort file` without groups, or `--group-by-file` alone: one file,
///   written on arrival (and possibly rendered ahead, see
///   [`FileListing::rendered`])
/// - `--group-by-module`: one directory
/// - `--sort name|kind` without groups, or `--prelude`: one package (a
///   prelude file lists records declared in other files)
///
/// Sections are decided from the package count before the first file is
/// analyzed, never from declarations found later, so no layout decision
/// holds output beyond the current unit.
pub(crate) struct ListingStream<'r> {
    renderer: &'r Renderer,
    options: ListingOptions,
    per_package: bool,
    sections: bool,
    current: Option<OpenPackage>,
    /// Flat output only: whether any declaration was written.
    wrote_any: bool,
}

/// The terminal `table` renders on: `term`, or, when `term` is too narrow for
/// the table's non-wrapping columns, `term` without a width limit, so the
/// table keeps its natural width and the terminal wraps it. Rendering at the
/// narrow width would print biscuit-terminal's "could not be rendered" notice
/// in place of the table.
pub(crate) fn table_terminal<'a>(table: &Table, term: &'a Terminal) -> Cow<'a, Terminal> {
    match table.plan_widths(term.width()) {
        Ok(_) => Cow::Borrowed(term),
        Err(_) => {
            let mut term = term.clone();
            term.fixed_width = Some(NO_WRAP_WIDTH);
            Cow::Owned(term)
        }
    }
}

struct OpenPackage {
    root: PathBuf,
    unit: Option<String>,
    pending: Vec<FileListing>,
    has_symbols: bool,
}

/// Orders `files` for [`ListingStream`] and pairs each with its package
/// root: by package root, then (with `--group-by-module`) directory, then
/// display path. Returns the ordered files and the number of packages.
pub(crate) fn listing_order(
    renderer: &Renderer,
    files: &[PathBuf],
    git_root: &Path,
    options: ListingOptions,
) -> (Vec<(PathBuf, PathBuf)>, usize) {
    // Package discovery reads manifests, so it is asked once per directory
    // for this run only.
    let mut roots: HashMap<&Path, PathBuf> = HashMap::new();
    let mut keyed: Vec<(PathBuf, String, String, PathBuf)> = files
        .iter()
        .map(|file| {
            let start = file.parent().unwrap_or(file.as_path());
            let display = renderer.display(file);
            let module = if options.group_by_module {
                module_key(&display)
            } else {
                String::new()
            };
            let root = roots.entry(start).or_insert_with(|| find_package_root(start, git_root)).clone();
            (root, module, display, file.clone())
        })
        .collect();
    keyed.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| portable_path_cmp(&left.1, &right.1))
            .then_with(|| portable_path_cmp(&left.2, &right.2))
    });
    let mut packages = 0;
    let mut previous: Option<&PathBuf> = None;
    for (package, ..) in &keyed {
        if previous != Some(package) {
            packages += 1;
            previous = Some(package);
        }
    }
    let ordered = keyed.into_iter().map(|(package, _, _, file)| (package, file)).collect();
    (ordered, packages)
}

impl<'r> ListingStream<'r> {
    /// A stream over files from `packages` package roots, sectioned when
    /// there are two or more. `per_package` holds each package whole
    /// (required by `--prelude`).
    pub(crate) fn new(renderer: &'r Renderer, options: ListingOptions, packages: usize, per_package: bool) -> Self {
        let per_package =
            per_package || (!options.group_by_file && !options.group_by_module && options.sort != SortOrder::File);
        Self {
            renderer,
            options,
            per_package,
            sections: packages > 1,
            current: None,
            wrote_any: false,
        }
    }

    /// Whether each unit is one file, so a unit is complete when its file
    /// arrives.
    pub(crate) fn units_are_files(&self) -> bool {
        !self.per_package && !self.options.group_by_module
    }

    /// Adds the next file's listing, writing whatever it completes.
    pub(crate) fn push(&mut self, out: &mut dyn Layout, package_root: &Path, listing: FileListing) -> io::Result<()> {
        if self.current.as_ref().is_none_or(|open| open.root != package_root) {
            self.close_package(out)?;
            self.open_package(out, package_root.to_path_buf())?;
        }
        let unit = if self.per_package {
            None
        } else {
            let display = self.renderer.display(&listing.file);
            Some(if self.options.group_by_module {
                module_key(&display)
            } else {
                display
            })
        };
        if self.current.as_ref().is_some_and(|open| open.unit != unit) {
            self.flush_unit(out)?;
        }
        let Some(open) = self.current.as_mut() else {
            unreachable!("a package was opened above");
        };
        open.unit = unit;
        open.has_symbols |= listing.has_symbols();
        open.pending.push(listing);
        if self.units_are_files() {
            // Written now, not when the next file arrives, so a flush right
            // after this delivery reaches the output.
            self.flush_unit(out)?;
        }
        Ok(())
    }

    /// Writes the last unit and the `(no symbols)` footer of a flat listing
    /// that had no declarations.
    pub(crate) fn finish(mut self, out: &mut dyn Layout) -> io::Result<()> {
        self.close_package(out)?;
        if self.sections || self.wrote_any {
            Ok(())
        } else {
            out.note(0, NO_SYMBOLS)
        }
    }

    fn open_package(&mut self, out: &mut dyn Layout, root: PathBuf) -> io::Result<()> {
        if self.sections {
            self.section_heading(out, &root)?;
        }
        self.current = Some(OpenPackage {
            root,
            unit: None,
            pending: Vec::new(),
            has_symbols: false,
        });
        Ok(())
    }

    fn close_package(&mut self, out: &mut dyn Layout) -> io::Result<()> {
        self.flush_unit(out)?;
        let Some(open) = self.current.take() else {
            return Ok(());
        };
        if self.sections {
            self.section_end(out, open.has_symbols)
        } else {
            self.wrote_any |= open.has_symbols;
            Ok(())
        }
    }

    /// Renders the current unit; a unit without declarations writes nothing.
    fn flush_unit(&mut self, out: &mut dyn Layout) -> io::Result<()> {
        let Some(open) = self.current.as_mut() else {
            return Ok(());
        };
        if let [FileListing { rendered: Some(bytes), .. }] = open.pending.as_slice() {
            out.writer().write_all(bytes)?;
        } else if open.pending.iter().any(FileListing::has_symbols) {
            let listings: Vec<&FileListing> = open.pending.iter().collect();
            self.renderer.package_listing(out, &listings, self.options)?;
        }
        open.pending.clear();
        Ok(())
    }

    fn section_heading(&self, out: &mut dyn Layout, root: &Path) -> io::Result<()> {
        out.blank()?;
        let heading = literal(&self.renderer.display(root));
        out.heading(1, 0, &format!("<blue><b>{heading}</b></blue>"))?;
        out.blank()
    }

    fn section_end(&self, out: &mut dyn Layout, has_symbols: bool) -> io::Result<()> {
        if !has_symbols {
            out.note(0, NO_SYMBOLS)?;
        }
        out.blank()
    }
}

/// One file's records arranged for `--tree`.
struct Outline<'a> {
    records: &'a [SymbolRecord],
    /// The selected members of each record, ordered.
    children: Vec<Vec<usize>>,
    roots: Vec<Root<'a>>,
}

/// A top-level outline entry.
enum Root<'a> {
    Record(usize),
    /// Members whose container is not listed: not selected, or declared in
    /// another file.
    Context {
        kind: Option<SymbolKindV2>,
        name: &'a str,
        members: Vec<usize>,
    },
}

impl<'a> Outline<'a> {
    fn new(records: &'a [SymbolRecord], name_prefix: &str, display: &str, sort: SortOrder) -> Self {
        let index: HashMap<&SymbolId, usize> = records
            .iter()
            .enumerate()
            .map(|(position, record)| (&record.id, position))
            .collect();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); records.len()];
        let mut roots: Vec<Root<'a>> = Vec::new();
        let mut contexts: HashMap<(Option<&SymbolId>, &str), usize> = HashMap::new();
        for (position, record) in records.iter().enumerate() {
            let container = record.relations.container.as_ref();
            let parent = container
                .and_then(|container| container.id.as_ref())
                .and_then(|id| index.get(id).copied())
                .filter(|parent| *parent != position);
            // Containment links only containers declared in this file; a
            // member of a type declared elsewhere (`impl Elsewhere { .. }`)
            // still names its container.
            let context = match container {
                Some(container) => Some((container.id.as_ref(), container.name.as_str())),
                None => record
                    .identity
                    .container_name
                    .as_deref()
                    .map(|name| (None, unresolved_owner(record, name_prefix, name))),
            };
            match (parent, context) {
                (Some(parent), _) => children[parent].push(position),
                (None, Some(key)) => {
                    let slot = *contexts.entry(key).or_insert_with(|| {
                        roots.push(Root::Context {
                            kind: record.identity.container_kind,
                            name: key.1,
                            members: Vec::new(),
                        });
                        roots.len() - 1
                    });
                    if let Root::Context { members, .. } = &mut roots[slot] {
                        members.push(position);
                    }
                }
                (None, None) => roots.push(Root::Record(position)),
            }
        }

        // A containment cycle would leave records unreachable from any
        // root; they are listed at the top level rather than dropped.
        let mut reachable = vec![false; records.len()];
        let mut stack: Vec<usize> = roots
            .iter()
            .flat_map(|root| match root {
                Root::Record(position) => vec![*position],
                Root::Context { members, .. } => members.clone(),
            })
            .collect();
        while let Some(position) = stack.pop() {
            if !std::mem::replace(&mut reachable[position], true) {
                stack.extend(&children[position]);
            }
        }
        for (position, reached) in reachable.iter().enumerate() {
            if !reached {
                roots.push(Root::Record(position));
            }
        }

        let by_order = |left: &usize, right: &usize| compare(sort, (display, &records[*left]), (display, &records[*right]));
        for members in &mut children {
            members.sort_by(by_order);
        }
        for root in &mut roots {
            if let Root::Context { members, .. } = root {
                members.sort_by(by_order);
            }
        }
        // A contextual heading sorts by its container's name and kind, and
        // by its first member's position.
        let key = |root: &Root<'a>| -> (&'a str, u8, &'a SymbolRecord) {
            match root {
                Root::Record(position) => {
                    let record = &records[*position];
                    (record.identity.name.as_str(), kind_rank(record.kind), record)
                }
                Root::Context { kind, name, members } => {
                    let first = &records[members[0]];
                    (name, kind.map_or(kind_rank(first.kind), kind_rank), first)
                }
            }
        };
        roots.sort_by(|left, right| {
            let (left_name, left_rank, left_record) = key(left);
            let (right_name, right_rank, right_record) = key(right);
            let primary = match sort {
                SortOrder::File => Ordering::Equal,
                SortOrder::Name => left_name.cmp(right_name),
                SortOrder::Kind => left_rank.cmp(&right_rank),
            };
            primary.then_with(|| compare(SortOrder::File, (display, left_record), (display, right_record)))
        });
        Self {
            records,
            children,
            roots,
        }
    }
}

/// The heading of a member with no same-file container: its in-file owner
/// path (the qualified name without the prefix and the member's own name)
/// when that qualifies `container_name` further, else `container_name`.
/// Two members of `impl a::W` and `impl b::W` are distinct owners and never
/// share one `[in W]` heading.
fn unresolved_owner<'a>(record: &'a SymbolRecord, name_prefix: &str, container_name: &'a str) -> &'a str {
    const SEPARATORS: [char; 3] = ['.', ':', '\\'];
    if record.identity.is_namespace_rooted {
        return container_name;
    }
    record
        .identity
        .qualified_name
        .strip_prefix(name_prefix)
        .and_then(|in_file| in_file.strip_suffix(record.identity.name.as_str()))
        .map(|path| path.trim_end_matches(SEPARATORS))
        .filter(|path| {
            path.strip_suffix(container_name)
                .is_some_and(|qualifier| qualifier.ends_with(SEPARATORS))
        })
        .unwrap_or(container_name)
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
pub(crate) fn label(record: &SymbolRecord) -> String {
    let keyword_names_it = matches!(
        record.kind,
        SymbolKindV2::Type | SymbolKindV2::Variable | SymbolKindV2::Field | SymbolKindV2::Constant
    );
    match &record.identity.keyword {
        Some(keyword) if keyword_names_it => keyword.clone(),
        _ => SymbolKind::from(record.kind).to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
enum BaseList {
    Extends,
    Implements,
    Unclassified,
}

/// Whether this entry is already written in the type's own header.
/// Rust impls and Go interface embeds come from outside that header; Swift
/// source spans distinguish header conformances from extension conformances.
/// A text search would hide `impl Display for W` behind `struct W<T: Display>`.
fn header_writes(record: &SymbolRecord, list: BaseList, base: &tree_hugger::SymbolRef) -> bool {
    if record.language == ProgrammingLanguage::Swift && matches!(list, BaseList::Implements) {
        return base.source_span.as_ref().is_some_and(|span| {
            span.start_byte >= record.source.declaration_span.start_byte
                && span.end_byte <= record.source.declaration_span.end_byte
        });
    }
    !matches!(
        (record.language, list),
        (ProgrammingLanguage::Rust, BaseList::Implements) | (ProgrammingLanguage::Go, _)
    )
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

/// What a declaration line shows in a signature mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Shown {
    /// `--signatures full`: the declaration header as written, which
    /// already carries its keyword and visibility.
    Header(String),
    /// The name with the mode's signature, shown after the visibility and
    /// label.
    Named(String),
}

/// `record` in `mode`; `with_container` appends the member's
/// `[in <kind> <container>]` suffix.
pub(crate) fn shown(record: &SymbolRecord, mode: SignatureMode, with_container: bool) -> Shown {
    let suffix = if with_container {
        container_suffix(record).unwrap_or_default()
    } else {
        String::new()
    };
    let name = &record.identity.name;
    match mode {
        SignatureMode::Full => match record.source.signature_text.as_deref() {
            Some(header) => Shown::Header(format!("{}{suffix}", one_line(header))),
            None => Shown::Named(format!("{name}{}{suffix}", compact_signature(record))),
        },
        SignatureMode::Short => Shown::Named(format!("{name}{}{suffix}", compact_signature(record))),
        SignatureMode::None => Shown::Named(format!("{name}{suffix}")),
    }
}

/// The signature column of `--format table` in `mode`: the header as
/// written for `full`, the compact form without the name for `short`.
pub(crate) fn signature_cell(record: &SymbolRecord, mode: SignatureMode) -> String {
    match (mode, record.source.signature_text.as_deref()) {
        (SignatureMode::None, _) => String::new(),
        (SignatureMode::Full, Some(header)) => one_line(header),
        _ => compact_signature(record).trim_start().to_string(),
    }
}

/// The compact signature that follows the name: `(param: T, ...) -> R` for
/// callables, ` { field: T, ... }` or ` { Variant, ... }` for types, else
/// nothing.
fn compact_signature(record: &SymbolRecord) -> String {
    let language = record.language;
    let kind = SymbolKind::from(record.kind);
    if kind.is_function() {
        match &record.kind_data {
            SymbolKindData::Function(function) => format_function(function, language),
            _ if record.visibility.is_synthetic => String::new(),
            _ => "()".to_string(),
        }
    } else if kind.is_type() {
        match &record.kind_data {
            SymbolKindData::Type(data) if !data.fields.is_empty() => short_list(
                data.fields
                    .iter()
                    .map(|field| format_field(&field.name, field.type_text.as_deref(), language)),
                data.fields.len(),
            ),
            SymbolKindData::Enum(data) if !data.variants.is_empty() => short_list(
                data.variants.iter().map(|variant| {
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
                }),
                data.variants.len(),
            ),
            _ => String::new(),
        }
    } else {
        String::new()
    }
}

/// ` { a, b, c, d }`, or ` { a, b, c, d, … +N }` past four entries.
fn short_list(entries: impl Iterator<Item = String>, len: usize) -> String {
    let mut shown: Vec<String> = entries.take(SHORT_LIST_ENTRIES).collect();
    if len > SHORT_LIST_ENTRIES {
        shown.push(format!("… +{}", len - SHORT_LIST_ENTRIES));
    }
    format!(" {{ {} }}", shown.join(", "))
}

/// ` [in <container kind> <container>]` for a member kind declared in a
/// container.
fn container_suffix(record: &SymbolRecord) -> Option<String> {
    let is_member_kind = matches!(
        SymbolKind::from(record.kind),
        SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Function
            | SymbolKind::Field
            | SymbolKind::Property
            | SymbolKind::Constant
            | SymbolKind::EnumVariant
            | SymbolKind::Parameter
    );
    let container = record.identity.container_name.as_deref().filter(|_| is_member_kind)?;
    Some(match record.identity.container_kind {
        Some(container_kind) => format!(" [in {} {container}]", SymbolKind::from(container_kind)),
        None => format!(" [in {container}]"),
    })
}

/// A multi-line declaration header on one line: each line break, with the
/// indentation around it, becomes one space. Other spacing is kept.
fn one_line(text: &str) -> String {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let mut joined = lines.next().unwrap_or_default().to_string();
    for line in lines {
        joined.push(' ');
        joined.push_str(line);
    }
    joined
}

/// `text` as literal `Prose` content: every control character (C0, DEL and
/// C1; tab included) is shown as its `\u{..}` escape, then the Prose markup
/// characters are escaped. `Prose::escape_text` alone passes ANSI CSI and
/// OSC sequences through, so source text could otherwise restyle the
/// terminal, rewrite the screen or plant a link.
pub(crate) fn literal(text: &str) -> String {
    Prose::escape_text(&neutralized(text))
}

/// `text` with every control character shown as its `\u{..}` escape, for
/// output that is not Prose markup (table text cells).
pub(crate) fn neutralized(text: &str) -> String {
    if !text.chars().any(char::is_control) {
        return text.to_string();
    }
    text.chars()
        .map(|character| {
            if character.is_control() {
                character.escape_unicode().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

fn format_function(function: &FunctionData, language: ProgrammingLanguage) -> String {
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
    format!("({}){return_part}", parameters.join(", "))
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
