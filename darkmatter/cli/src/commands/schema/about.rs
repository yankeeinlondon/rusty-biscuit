//! `md schema about` implementation.
//!
//! Renders a human-readable reference for the SimplifiedSchema authoring
//! language from the typed descriptor catalog in
//! `darkmatter::markdown::schemas`. The command is documentation-only — it
//! performs no document parsing, no context capture, no `EffectEngine`
//! construction, no file resolution, and no network access. The only
//! observable side effect is printing to stdout.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::schemas::{
    CoercionRuleDescriptor, InlineObjectRuleDescriptor, SchemaConstraintDescriptor,
    SchemaShapeDescriptor, SchemaTypeDescriptor, ValidationBehaviorDescriptor,
    coercion_rule_descriptors, inline_object_rule_descriptors, schema_constraint_descriptors,
    schema_shape_descriptors, schema_type_descriptors, validation_behavior_descriptors,
};

/// Run `md schema about`.
pub fn run_about() -> Result<()> {
    let terminal = Terminal::default();

    print_header(&terminal);
    print_shapes(&terminal, schema_shape_descriptors());
    print_types(&terminal, schema_type_descriptors());
    print_constraints(&terminal, schema_constraint_descriptors());
    print_inline_object_rules(&terminal, inline_object_rule_descriptors());
    print_coercion_rules(&terminal, coercion_rule_descriptors());
    print_validation_behavior(&terminal, validation_behavior_descriptors());
    print_footer(&terminal);

    Ok(())
}

fn print_header(terminal: &Terminal) {
    println!(
        "{}",
        Prose::new(
            "<b><yellow>SimplifiedSchema Language Reference</yellow></b>".to_string(),
        )
        .render(terminal)
    );
    println!(
        "{}",
        Prose::new(
            "<dim>Generated from the typed schema-language descriptor catalog; this is the implementation-bound reference, not hand-maintained prose.</dim>".to_string(),
        )
        .render(terminal)
    );
    println!();
}

fn print_footer(terminal: &Terminal) {
    println!();
    println!(
        "{}",
        Prose::new(
            "<dim>This report is generated from the typed descriptor catalog and reflects the exact type, constraint, and shape surface the parser, converter, and compose-time coercion understand today.</dim>".to_string(),
        )
        .render(terminal)
    );
    println!(
        "{}",
        Prose::new(
            "<dim>`md schema about` is documentation-only: it performs no document parsing, context capture, EffectEngine construction, file resolution, or network access.</dim>".to_string(),
        )
        .render(terminal)
    );
}

fn print_shapes(terminal: &Terminal, shapes: &[SchemaShapeDescriptor]) {
    print_section_header(terminal, "Schema Shapes");
    print_section_intro(
        terminal,
        "A `$schema` value is one of these forms. Every shape compiles to Draft 2020-12 JSON Schema.",
    );
    for s in shapes {
        print!(
            "\n{}",
            Prose::new(format!("<b>{}</b> <dim>({})</dim>", s.name, s.form)).render(terminal)
        );
        println!(
            "{}",
            Prose::new(format!("  <dim>example:</dim> <green>{}</green>", s.example))
                .render(terminal)
        );
        println!("{}", Prose::new(format!("  {}", s.description)).render(terminal));
    }
    println!();
}

fn print_types(terminal: &Terminal, types: &[SchemaTypeDescriptor]) {
    print_section_header(terminal, "Type Vocabulary");
    print_section_intro(
        terminal,
        "Append `[]` to any type for an array of that type, and add `(item_constraints)` between the type and `[]` to constrain items.",
    );
    println!(
        "{}",
        Prose::new(
            "<b>Keyword</b>  <b>Description</b>  <b>Accepted constraints</b>  <b>JSON Schema effect</b>".to_string(),
        )
        .render(terminal)
    );
    for t in types {
        let line = format!(
            "<inverse>{}</inverse>  {}  <dim>{}</dim>  <dim>{}</dim>",
            t.keyword, t.description, t.accepted_constraints, t.json_schema_effect
        );
        println!("{}", Prose::new(line).render(terminal));
    }
    println!();
}

fn print_constraints(terminal: &Terminal, constraints: &[SchemaConstraintDescriptor]) {
    print_section_header(terminal, "Constraint Vocabulary");
    print_section_intro(
        terminal,
        "Constraints are comma- or semicolon-separated inside `(...)`. The same keyword may apply to different types with different effects; the `Target types` column shows the supported combinations.",
    );
    for c in constraints {
        let header = format!(
            "<inverse>{}</inverse> <dim>({})</dim> <dim>— target:</dim> <b>{}</b> <dim>— arity:</dim> <b>{}</b>",
            c.name, c.form, c.target_types, c.argument_arity
        );
        println!("{}", Prose::new(header).render(terminal));
        println!("{}", Prose::new(format!("  {}", c.description)).render(terminal));
        println!(
            "{}",
            Prose::new(format!("  <dim>JSON Schema effect:</dim> {}", c.json_schema_effect))
                .render(terminal)
        );
    }
    println!();
}

fn print_inline_object_rules(terminal: &Terminal, rules: &[InlineObjectRuleDescriptor]) {
    print_section_header(terminal, "Inline Object Rules");
    print_section_intro(
        terminal,
        "Inline object literals declare typed object shapes inside a single type expression. They support arrays, postfix constraints, and arbitrary nesting up to a hard 32-level limit.",
    );
    for r in rules {
        let header = format!("<b>{}</b>", r.name);
        println!("{}", Prose::new(header).render(terminal));
        println!("{}", Prose::new(format!("  <dim>rule:</dim> {}", r.rule)).render(terminal));
        println!("{}", Prose::new(format!("  {}", r.description)).render(terminal));
    }
    println!();
}

fn print_coercion_rules(terminal: &Terminal, rules: &[CoercionRuleDescriptor]) {
    print_section_header(terminal, "Compose-time Coercion");
    print_section_intro(
        terminal,
        "Coercion runs after frontmatter `--set` / `--state` and interpolation, before shell expansion. It writes coerced values back into the frontmatter so downstream compose stages see real booleans and numbers.",
    );
    for r in rules {
        let header = format!("<b>{}</b>", r.name);
        println!("{}", Prose::new(header).render(terminal));
        println!("{}", Prose::new(format!("  <dim>rule:</dim> {}", r.rule)).render(terminal));
        println!("{}", Prose::new(format!("  {}", r.description)).render(terminal));
    }
    println!();
}

fn print_validation_behavior(terminal: &Terminal, behaviors: &[ValidationBehaviorDescriptor]) {
    print_section_header(terminal, "Validation Behaviour");
    print_section_intro(
        terminal,
        "These notes govern how the validator and the compose pipeline interpret a schema beyond the type / constraint surface itself.",
    );
    for b in behaviors {
        let header = format!("<b>{}</b>", b.name);
        println!("{}", Prose::new(header).render(terminal));
        println!("{}", Prose::new(format!("  <dim>rule:</dim> {}", b.rule)).render(terminal));
        println!("{}", Prose::new(format!("  {}", b.description)).render(terminal));
    }
    println!();
}

fn print_section_header(terminal: &Terminal, title: &str) {
    println!();
    println!(
        "{}",
        Prose::new(format!("<b>{}</b>", title)).render(terminal)
    );
    println!(
        "{}",
        Prose::new("<dim>────────────────────────────────────────</dim>".to_string()).render(terminal)
    );
}

fn print_section_intro(terminal: &Terminal, text: &str) {
    println!(
        "{}",
        Prose::new(format!("<dim>{}</dim>", text)).render(terminal)
    );
}
