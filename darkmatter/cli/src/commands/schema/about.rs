//! `md schema about` implementation.
//!
//! Renders a human-readable reference for the SimplifiedSchema authoring
//! language from the typed descriptor catalog in
//! `darkmatter::markdown::schemas`. The command is documentation-only — it
//! performs no document parsing, no context capture, no `EffectEngine`
//! construction, no file resolution, and no network access. The only
//! observable side effect is printing to stdout.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use color_eyre::eyre::Result;
use darkmatter::markdown::highlighting::CodeBlockMode;
use darkmatter::markdown::highlighting::ColorMode as MarkdownColorMode;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::TerminalOptions;
use darkmatter::markdown::schemas::{
    CoercionRuleDescriptor, InlineObjectRuleDescriptor, SchemaConstraintDescriptor,
    SchemaShapeDescriptor, SchemaTypeDescriptor, ValidationBehaviorDescriptor,
    coercion_rule_descriptors, inline_object_rule_descriptors, schema_constraint_descriptors,
    schema_shape_descriptors, schema_type_descriptors, validation_behavior_descriptors,
};

const LEFT_MARGIN_CH: u32 = 1;
const RIGHT_MARGIN_CH: u32 = 2;
const MIN_REPORT_WIDTH: u32 = 64;

fn markdown_color_mode_from_terminal(mode: TerminalColorMode) -> MarkdownColorMode {
    match mode {
        TerminalColorMode::Light => MarkdownColorMode::Light,
        TerminalColorMode::Dark | TerminalColorMode::Unknown => MarkdownColorMode::Dark,
    }
}

/// Run `md schema about`.
pub fn run_about(verbose: bool, code_block_mode: CodeBlockMode) -> Result<()> {
    let terminal = Terminal::default();
    let mut report = SchemaAboutReport::new(&terminal, code_block_mode);

    report.header()?;
    report.shapes(schema_shape_descriptors())?;
    report.types(schema_type_descriptors())?;
    report.constraints(schema_constraint_descriptors())?;
    if verbose {
        report.inline_object_rules(inline_object_rule_descriptors())?;
        report.coercion_rules(coercion_rule_descriptors())?;
        report.validation_behavior(validation_behavior_descriptors())?;
    } else {
        report.verbose_hint();
    }

    Ok(())
}

struct SchemaAboutReport<'a> {
    terminal: &'a Terminal,
    width: u32,
    previous_blank: bool,
    code_block_mode: CodeBlockMode,
}

impl<'a> SchemaAboutReport<'a> {
    fn new(terminal: &'a Terminal, code_block_mode: CodeBlockMode) -> Self {
        let margin_width = LEFT_MARGIN_CH + RIGHT_MARGIN_CH;
        let width = terminal.width().saturating_sub(margin_width).max(MIN_REPORT_WIDTH);
        Self {
            terminal,
            width,
            previous_blank: true,
            code_block_mode,
        }
    }

    fn header(&mut self) -> Result<()> {
        self.prose("<b><yellow>SimplifiedSchema Language Reference</yellow></b>");
        self.blank();
        self.markdown(
            "A SimplifiedSchema describes the frontmatter a Markdown file expects. Put it under \
             `$schema` to name required fields, allowed types, defaults, and simple constraints. \
             Darkmatter uses that schema when you run `md schema validate` and during compose, so \
             invalid frontmatter fails early and common string values can be coerced before the \
             page is rendered.",
        )?;
        self.blank();
        self.markdown(
            "Use this as a quick authoring reference: what shapes are allowed, which type \
             keywords exist, which constraints can be attached, and what Darkmatter does with \
             values during compose.",
        )?;
        Ok(())
    }

    fn shapes(&mut self, shapes: &[SchemaShapeDescriptor]) -> Result<()> {
        self.heading("Schema Shapes");
        self.markdown(
            "A `$schema` value can use any of these forms. Choose the simplest shape that \
             describes the frontmatter you expect.",
        )?;
        let mut markdown = String::new();
        for shape in shapes {
            markdown.push_str(&format!(
                "- **{}.** {}\n  - Example: `{}`\n",
                escape_markdown(shape.name),
                shape.description,
                shape.example.replace('\n', "\\n")
            ));
        }
        self.blank();
        self.markdown(markdown)?;
        Ok(())
    }

    fn types(&mut self, types: &[SchemaTypeDescriptor]) -> Result<()> {
        self.heading("Type System");
        let rows = types
            .iter()
            .map(|t| {
                vec![
                    prose_cell(format!("<b>{}</b>", escape_prose(t.keyword))),
                    prose_cell(markdown_inline_to_prose(t.description)),
                    prose_cell(format_constraint_names(t.accepted_constraints)),
                ]
            })
            .collect();
        self.table(
            vec![
                TableColumn::new("Type")
                    .with_max_width(14)
                    .with_word_wrap(WordWrap::WrapProse(Some(4), None)),
                TableColumn::new("Meaning")
                    .with_min_width(24)
                    .with_word_wrap(WordWrap::WrapProse(Some(6), None)),
                TableColumn::new("Constraints")
                    .with_min_width(24)
                    .with_word_wrap(WordWrap::BespokeProse(
                        Some(4),
                        vec![' ', ',', '/', ';', '(', ')'],
                        None,
                    )),
            ],
            rows,
        );
        self.type_vocabulary_notes()?;
        Ok(())
    }

    fn type_vocabulary_notes(&mut self) -> Result<()> {
        self.markdown(
            r#"Any of the above types can be made into an array by adding `[]` to the end of the type:

```yaml
$schema:
    # a string type
    foo: string
    # can be made into a string array
    bar: string[]
```

All types listed here, including those with the `[]` array modifier, can add constraints to the type by putting parentheses after the type. Multiple constraints are always delimited by the `;` character. As an example:

```yaml
$schema:
  foo: string(required)
  bar: "number(required; min(0); max(10))"
  baz: "string[](min(1))"
```

In this example, constraints are added to a string, a number, and an array of strings. The next section lists the available constraints.

> **Note:** the _values_ in schemas must be valid YAML strings. In some cases you won't need to put single or double quotes around them, but you can always quote the string and then you don't need to think about it."#,
        )
    }

    fn constraints(&mut self, constraints: &[SchemaConstraintDescriptor]) -> Result<()> {
        self.heading("Constraint Vocabulary");
        self.markdown(
            "Constraints go inside `(...)` after a type, separated by commas or semicolons. \
             They narrow what a value may contain or provide metadata such as defaults.",
        )?;
        let wide = self.width >= 120;
        let use_min_width = if wide { 26 } else { 20 };
        let applies_to_max_width = if wide { 24 } else { 16 };
        let meaning_min_width = if wide { 42 } else { 26 };
        let rows = constraints
            .iter()
            .map(|c| {
                vec![
                    prose_cell(format_constraint_use(c)),
                    prose_cell(format_targets(c.target_types)),
                    prose_cell(format_constraint_meaning(c)),
                ]
            })
            .collect();
        self.table(
            vec![
                TableColumn::new("Write")
                    .with_min_width(use_min_width)
                    .with_word_wrap(WordWrap::BespokeProse(
                        Some(4),
                        vec![' ', ',', '/', ';', '(', ')'],
                        None,
                    )),
                TableColumn::new("Applies To")
                    .with_min_width(12)
                    .with_max_width(applies_to_max_width)
                    .with_word_wrap(WordWrap::BespokeProse(
                        Some(4),
                        vec![' ', ',', '/', ';'],
                        None,
                    )),
                TableColumn::new("Meaning")
                    .with_min_width(meaning_min_width)
                    .with_word_wrap(WordWrap::WrapProse(Some(6), None)),
            ],
            rows,
        );
        Ok(())
    }

    fn inline_object_rules(&mut self, rules: &[InlineObjectRuleDescriptor]) -> Result<()> {
        self.heading("Nested Objects");
        self.markdown(
            "Inline object literals are for nested frontmatter values. Use them when a property \
             is itself an object and you want Darkmatter to validate the fields inside it.",
        )?;
        self.detail_list(
            rules.iter().map(|r| (&r.name, &r.rule, &r.description)),
            "Details",
        )?;
        Ok(())
    }

    fn coercion_rules(&mut self, rules: &[CoercionRuleDescriptor]) -> Result<()> {
        self.heading("Compose-time Coercion");
        self.markdown(
            "When a schema expects a boolean or number, compose can turn common string forms into \
             real values before later compose stages run.",
        )?;
        self.detail_list(
            rules.iter().map(|r| (&r.name, &r.rule, &r.description)),
            "Behavior",
        )?;
        Ok(())
    }

    fn validation_behavior(&mut self, behaviors: &[ValidationBehaviorDescriptor]) -> Result<()> {
        self.heading("Validation Notes");
        self.markdown(
            "These notes explain how Darkmatter applies schemas in real documents.",
        )?;
        self.detail_list(
            behaviors.iter().map(|b| (&b.name, &b.rule, &b.description)),
            "Details",
        )?;
        Ok(())
    }

    fn heading(&mut self, title: &str) {
        self.blank();
        self.prose(format!("<b>{}</b>", title));
        self.blank();
    }

    fn prose<T: Into<String>>(&mut self, text: T) {
        let rendered = Prose::new(text.into())
            .with_word_wrap(WordWrap::WrapProse(Some(8), None))
            .render_in_width(self.terminal, self.width);
        self.block(&rendered);
    }

    fn markdown<T: Into<String>>(&mut self, text: T) -> Result<()> {
        let options = self.markdown_options();
        let rendered = Markdown::from(text.into()).as_terminal(options)?;
        self.block(&rendered);
        Ok(())
    }

    fn markdown_options(&self) -> TerminalOptions {
        let mut options = TerminalOptions::default();
        options.max_width = Some(self.width.min(u16::MAX as u32) as u16);
        options.color_mode = markdown_color_mode_from_terminal(self.terminal.color_mode());
        options.code_block_mode = self.code_block_mode;
        options
    }

    fn detail_list<'b, I>(&mut self, rules: I, detail_label: &str) -> Result<()>
    where
        I: IntoIterator<Item = (&'b &'static str, &'b &'static str, &'b &'static str)>,
    {
        self.blank();
        let mut list = UnorderedList::empty();
        for (name, detail, description) in rules {
            let item = if detail.trim().is_empty() {
                format!(
                    "<b>{}</b>. {}",
                    escape_prose(name),
                    escape_prose(&plain_markdown_text(description))
                )
            } else {
                format!(
                    "<b>{}</b>. {}\n  - {}: {}",
                    escape_prose(name),
                    escape_prose(&plain_markdown_text(description)),
                    escape_prose(detail_label),
                    escape_prose(&plain_markdown_text(detail))
                )
            };
            list.add(
                Prose::new(item)
                    .with_word_wrap(WordWrap::WrapProse(Some(8), Some(2))),
            );
        }
        self.block(&list.render_in_width(self.terminal, self.width));
        self.blank();
        Ok(())
    }

    fn verbose_hint(&mut self) {
        self.blank();
        let status = Status::from_prose("use <blue>--verbose</blue> to see additional details")
            .state(StatusState::Info);
        self.block(&status.render(self.terminal));
    }

    fn table(&mut self, columns: Vec<TableColumn>, rows: Vec<Vec<TableCellContent>>) {
        self.blank();
        let table = Table::new()
            .with_columns(columns)
            .with_data(rows)
            .alternate_background_color();
        self.block(&table.render_in_width(self.terminal, self.width));
        self.blank();
    }

    fn block(&mut self, rendered: &str) {
        for line in rendered.lines() {
            if line.trim().is_empty() {
                self.blank();
            } else {
                println!("{}{}", " ".repeat(LEFT_MARGIN_CH as usize), line);
                self.previous_blank = false;
            }
        }
    }

    fn blank(&mut self) {
        if !self.previous_blank {
            println!();
            self.previous_blank = true;
        }
    }
}

fn prose_cell<T: Into<String>>(text: T) -> TableCellContent {
    Prose::new(text.into())
        .with_word_wrap(WordWrap::WrapProse(Some(6), None))
        .into()
}

fn format_constraint_use(constraint: &SchemaConstraintDescriptor) -> String {
    match constraint.keyword {
        "min" => "min(<dim>number</dim>)".to_string(),
        "max" => "max(<dim>number</dim>)".to_string(),
        "default" => "default(<dim>value</dim>)".to_string(),
        "pattern" => "pattern(<dim>regex</dim>)".to_string(),
        "<members>" => "enum(<dim>member, ...</dim>)".to_string(),
        "match" => "match(<dim>glob, ...</dim>)".to_string(),
        "scheme" => "scheme(<dim>http, https, ...</dim>)".to_string(),
        _ => escape_prose(constraint.form),
    }
}

fn format_constraint_meaning(constraint: &SchemaConstraintDescriptor) -> String {
    match constraint.keyword {
        "min" => {
            "Sets a lower bound.\n\
             - <b>string</b>: minimum length\n\
             - <b>number</b>: minimum value\n\
             - <b>array</b>: minimum item count"
                .to_string()
        }
        "max" => {
            "Sets an upper bound.\n\
             - <b>string</b>: maximum length\n\
             - <b>number</b>: maximum value\n\
             - <b>array</b>: maximum item count"
                .to_string()
        }
        _ => markdown_inline_to_prose(constraint.description),
    }
}

fn format_constraint_names(constraints: &str) -> String {
    split_top_level_constraints(constraints)
        .into_iter()
        .map(|part| {
            part.find('(')
                .map(|index| &part[..index])
                .unwrap_or(part)
                .trim()
        })
        .filter(|part| !part.is_empty())
        .map(escape_prose)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_targets(targets: &str) -> String {
    targets.replace(" | ", ", ")
}

fn escape_prose(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '`' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn plain_markdown_text(text: &str) -> String {
    text.replace('`', "")
}

fn split_top_level_constraints(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0u32;

    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' | ';' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(text[start..].trim());
    parts
}

fn markdown_inline_to_prose(text: &str) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(open) = rest.find('`') {
        rendered.push_str(&escape_prose(&rest[..open]));
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('`') else {
            rendered.push_str(&escape_prose(&rest[open..]));
            return rendered;
        };

        rendered.push_str("<dim>");
        rendered.push_str(&escape_prose(&after_open[..close]));
        rendered.push_str("</dim>");
        rest = &after_open[close + 1..];
    }

    rendered.push_str(&escape_prose(rest));
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
    use darkmatter::markdown::highlighting::{
        CodeHighlighter, ColorMode as MarkdownColorMode, ThemePair,
    };
    use syntect::easy::HighlightLines;
    use syntect::highlighting::Color;

    fn terminal_for_mode(mode: TerminalColorMode) -> Terminal {
        let mut terminal = Terminal::new_optimistic(120);
        terminal.color_mode = mode;
        terminal
    }

    fn fg_sgr(color: Color) -> String {
        format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn bg_sgr(color: Color) -> String {
        format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn one_half_yaml_color(mode: MarkdownColorMode, line: &str, token: &str) -> Color {
        let highlighter = CodeHighlighter::new(ThemePair::OneHalf, mode);
        let syntax = highlighter
            .syntax_set()
            .find_syntax_by_extension("yaml")
            .expect("yaml syntax");
        let mut hl = HighlightLines::new(syntax, highlighter.theme());
        let token_start = line
            .find(token)
            .unwrap_or_else(|| panic!("missing token {token:?} in source line {line:?}"));
        let mut offset = 0;
        hl.highlight_line(line, highlighter.syntax_set())
            .expect("highlight line")
            .into_iter()
            .find_map(|(style, text)| {
                let end = offset + text.len();
                let matched = (offset..end).contains(&token_start);
                offset = end;
                matched.then_some(style.foreground)
            })
            .unwrap_or_else(|| panic!("missing token {token:?} in highlighted line {line:?}"))
    }

    fn one_half_background(mode: MarkdownColorMode) -> Color {
        CodeHighlighter::new(ThemePair::OneHalf, mode)
            .theme()
            .settings
            .background
            .expect("theme background")
    }

    fn render_schema_about_markdown(mode: TerminalColorMode) -> String {
        let terminal = terminal_for_mode(mode);
        let report = SchemaAboutReport::new(&terminal, CodeBlockMode::default());
        let mut options = report.markdown_options();
        options.code_theme = ThemePair::OneHalf;
        options.prose_theme = ThemePair::OneHalf;
        Markdown::from(
            "```yaml\n$schema:\n    # a string type\n    bar: string[]\n```\n",
        )
        .as_terminal(options)
        .expect("schema-about markdown render")
    }

    fn assert_yaml_uses_exact_theme(output: &str, expected_mode: MarkdownColorMode) {
        let background = one_half_background(expected_mode);
        let comment = one_half_yaml_color(expected_mode, "    # a string type", "#");
        let object_key = one_half_yaml_color(expected_mode, "    bar: string[]", "bar");
        let string_value = one_half_yaml_color(expected_mode, "    bar: string[]", "string[]");

        assert!(
            output.contains(&bg_sgr(background)),
            "expected OneHalf {:?} background RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            background.r,
            background.g,
            background.b,
        );
        assert!(
            output.contains(&format!("{}#", fg_sgr(comment)))
                && output.contains(&format!("{} a string type", fg_sgr(comment))),
            "expected OneHalf {:?} comment RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            comment.r,
            comment.g,
            comment.b,
        );
        assert!(
            output.contains(&format!("{}bar", fg_sgr(object_key))),
            "expected OneHalf {:?} YAML key RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            object_key.r,
            object_key.g,
            object_key.b,
        );
        assert!(
            output.contains(&format!("{}string[]", fg_sgr(string_value))),
            "expected OneHalf {:?} YAML scalar RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            string_value.r,
            string_value.g,
            string_value.b,
        );
    }

    #[test]
    fn schema_about_markdown_options_use_detected_terminal_mode() {
        for (terminal_mode, markdown_mode) in [
            (TerminalColorMode::Dark, MarkdownColorMode::Dark),
            (TerminalColorMode::Light, MarkdownColorMode::Light),
            (TerminalColorMode::Unknown, MarkdownColorMode::Dark),
        ] {
            let terminal = terminal_for_mode(terminal_mode);
            let report = SchemaAboutReport::new(&terminal, CodeBlockMode::default());
            let options = report.markdown_options();

            assert_eq!(options.color_mode, markdown_mode);
            assert_eq!(options.color_depth, None);
            assert_eq!(options.max_width, Some(117));
        }
    }

    #[test]
    fn schema_about_markdown_code_uses_inverted_theme_for_both_terminal_modes() {
        let dark_output = render_schema_about_markdown(TerminalColorMode::Dark);
        assert_yaml_uses_exact_theme(&dark_output, MarkdownColorMode::Light);

        let light_output = render_schema_about_markdown(TerminalColorMode::Light);
        assert_yaml_uses_exact_theme(&light_output, MarkdownColorMode::Dark);
    }
}
