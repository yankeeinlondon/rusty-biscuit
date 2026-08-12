use crate::args::LayoutArgs;
use crate::commands::mermaid::{build_mermaid_diagram, display_mermaid};
use crate::commands::shared::{print_example_command_with_terminal, terminal_for_render};
use crate::commands::{CliContext, Run};
use clap::Args as ClapArgs;
use std::io::Write;

/// Example data for erd --example
const ERD_EXAMPLE_ENTITIES: &[&str] = &[
    "Customer {\n        int id PK\n        string name\n        string email\n    }",
    "Order {\n        int id PK\n        date orderDate\n        int customerId FK\n    }",
    "Product {\n        int id PK\n        string name\n        decimal price\n    }",
    "OrderItem {\n        int orderId FK\n        int productId FK\n        int quantity\n    }",
];
const ERD_EXAMPLE_RELATIONSHIPS: &[&str] = &[
    "Customer ||--o{ Order : places",
    "Order ||--|{ OrderItem : contains",
    "Product ||--o{ OrderItem : \"ordered in\"",
];
const ERD_EXAMPLE_CMD: &str = "bt erd --title \"E-Commerce Schema\" \\\n  --entity \"Customer { int id PK }\" \\\n  --entity \"Order { int id PK }\" \\\n  \"Customer ||--o{ Order : places\"";

/// Render an entity relationship diagram (ERD)
#[derive(ClapArgs, Debug, Clone)]
#[command(after_long_help = "\nRelationship Syntax:\n  ||--o{   One to many\n")]
pub struct ErdArgs {
    #[arg(long, short = 't')]
    pub title: Option<String>,

    #[arg(long, short = 'w')]
    pub width: Option<crate::types::WidthSpec>,

    #[command(flatten)]
    pub layout: LayoutArgs,

    #[arg(long, short = 'E', action = clap::ArgAction::Append)]
    pub entity: Vec<String>,

    #[arg(long)]
    pub inverse: bool,

    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long)]
    pub meta: bool,

    #[arg(value_name = "RELATIONSHIPS", required_unless_present = "example")]
    pub relationships: Vec<String>,
}

impl Run for ErdArgs {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        let _ = std::io::stdout().flush();

        let (entities, relationships, eff_title): (Vec<String>, Vec<String>, Option<&str>) =
            if self.example {
                (
                    ERD_EXAMPLE_ENTITIES.iter().map(|s| s.to_string()).collect(),
                    ERD_EXAMPLE_RELATIONSHIPS
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    Some("E-Commerce Schema"),
                )
            } else {
                (self.entity, self.relationships, self.title.as_deref())
            };

        if relationships.is_empty() && entities.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No relationships or entities provided. Use format: \"Entity1 ||--o{{ Entity2 : label\""
            ));
        }

        let mut lines = vec!["erDiagram".to_string()];

        for entity in &entities {
            lines.push(format!("    {}", entity));
        }

        for rel in &relationships {
            lines.push(format!("    {}", rel));
        }

        use crate::commands::mermaid::with_mermaid_frontmatter_title;
        let instructions = with_mermaid_frontmatter_title(&lines.join("\n"), eff_title);

        if ctx.json {
            let output = serde_json::json!({
                "type": "erd",
                "inverse": self.inverse,
                "title": eff_title,
                "entities": entities,
                "relationships": relationships,
                "instructions": instructions,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        let width_str = self.width.as_ref().map(|w| w.to_string());
        let diagram = build_mermaid_diagram(
            &instructions,
            self.inverse,
            width_str.as_deref(),
            &self.layout,
        )?;
        let terminal = terminal_for_render(ctx.plain);
        display_mermaid(
            &diagram,
            &instructions,
            "ERD",
            &self.layout,
            self.meta,
            false,
            &terminal,
        )?;

        if self.example {
            print_example_command_with_terminal(ERD_EXAMPLE_CMD, &terminal);
        }

        Ok(())
    }
}
