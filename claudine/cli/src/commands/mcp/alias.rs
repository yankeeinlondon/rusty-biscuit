use claudine::mcp::catalog::McpCatalogStore;
use color_eyre::eyre::Result;
use inquire::Text;
use serde_json::json;

use super::{AliasArgs, AliasCompatibilityCommand, prompt_for_server_query, render_json_or_text};

pub(super) fn run_alias(args: AliasArgs, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;

    match args.compatibility {
        Some(AliasCompatibilityCommand::Add(add)) => {
            catalog.add_alias(&catalog.resolve(&add.name)?.id.clone(), &add.alias)?;
            catalog.save()?;
            return render_json_or_text(
                json_output,
                json!({ "action": "add", "id": add.name, "alias": add.alias }),
                "Alias added.".to_string(),
            );
        }
        Some(AliasCompatibilityCommand::Remove(remove)) => {
            catalog.remove_alias(&remove.alias)?;
            catalog.save()?;
            return render_json_or_text(
                json_output,
                json!({ "action": "remove", "alias": remove.alias }),
                format!("Alias `{}` removed.", remove.alias),
            );
        }
        None => {}
    }

    let name = match args.name {
        Some(name) => name,
        None => prompt_for_server_query(&catalog, "Choose the MCP server to alias:")?,
    };
    let alias = match args.alias {
        Some(alias) => alias,
        None => Text::new("Alias to add:").prompt()?,
    };

    let server_id = catalog.resolve(&name)?.id.clone();
    catalog.add_alias(&server_id, &alias)?;
    catalog.save()?;

    render_json_or_text(
        json_output,
        json!({ "action": "add", "id": server_id, "alias": alias }),
        format!("Alias `{alias}` added to `{server_id}`."),
    )
}
