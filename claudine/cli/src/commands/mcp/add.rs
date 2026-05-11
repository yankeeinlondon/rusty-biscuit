use std::collections::HashMap;

use chrono::Utc;
use claudine::mcp::catalog::McpCatalogStore;
use claudine::mcp::types::{McpServer, McpServerMetadata, McpTransport, derive_server_id};
use color_eyre::eyre::{Result, eyre};
use inquire::{Confirm, Text};
use serde_json::json;

use crate::log;

use super::{AddArgs, AddKind};

pub(super) fn run_add(args: AddArgs, json_output: bool) -> Result<()> {
    let mut catalog = McpCatalogStore::load()?;
    let server = match args.kind {
        AddKind::Local => prompt_for_local_server()?,
        AddKind::Remote => prompt_for_remote_server()?,
    };
    let added = insert_manual_server(&mut catalog, server)?;
    catalog.save()?;

    if json_output {
        log::data(&serde_json::to_string_pretty(&json!({ "added": added }))?);
    } else {
        log::data(&format!("Added MCP server `{added}`."));
    }

    Ok(())
}

fn prompt_for_local_server() -> Result<McpServer> {
    let name = Text::new("Name (leave empty to auto-derive):").prompt()?;
    let command = Text::new("Command:").prompt()?;
    let args = Text::new("Arguments (space-separated, optional):").prompt()?;
    let env = prompt_for_env_map()?;

    let mut server = McpServer {
        id: String::new(),
        aliases: Vec::new(),
        transport: McpTransport::Stdio,
        command: Some(command),
        args: split_args(&args),
        cwd: None,
        env,
        url: None,
        headers: HashMap::new(),
        enabled_tools: Vec::new(),
        disabled_tools: Vec::new(),
        required: false,
        metadata: McpServerMetadata {
            description: None,
            created_from: Some("manual:user".into()),
            fingerprint: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        provider_overrides: HashMap::new(),
    };
    server.id = derive_server_id(&server, (!name.trim().is_empty()).then_some(name.trim()));
    server.metadata.fingerprint = server.fingerprint();
    Ok(server)
}

fn prompt_for_remote_server() -> Result<McpServer> {
    let name = Text::new("Name (leave empty to auto-derive):").prompt()?;
    let url = Text::new("URL:").prompt()?;
    let env = prompt_for_env_map()?;

    let mut server = McpServer {
        id: String::new(),
        aliases: Vec::new(),
        transport: McpTransport::Http,
        command: None,
        args: Vec::new(),
        cwd: None,
        env,
        url: Some(url),
        headers: HashMap::new(),
        enabled_tools: Vec::new(),
        disabled_tools: Vec::new(),
        required: false,
        metadata: McpServerMetadata {
            description: None,
            created_from: Some("manual:user".into()),
            fingerprint: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        provider_overrides: HashMap::new(),
    };
    server.id = derive_server_id(&server, (!name.trim().is_empty()).then_some(name.trim()));
    server.metadata.fingerprint = server.fingerprint();
    Ok(server)
}

fn prompt_for_env_map() -> Result<HashMap<String, String>> {
    let add_env = Confirm::new("Add environment variables?")
        .with_default(false)
        .prompt()?;
    if !add_env {
        return Ok(HashMap::new());
    }

    let mut env = HashMap::new();
    loop {
        let entry = Text::new("Environment entry (KEY=VALUE, leave empty to finish):").prompt()?;
        if entry.trim().is_empty() {
            break;
        }
        let Some((key, value)) = entry.split_once('=') else {
            return Err(eyre!("environment entries must look like KEY=VALUE"));
        };
        env.insert(key.trim().to_string(), value.to_string());
    }
    Ok(env)
}

fn insert_manual_server(catalog: &mut McpCatalogStore, mut server: McpServer) -> Result<String> {
    let base_id = server.id.clone();
    let fingerprint = server.metadata.fingerprint.clone();

    if let Some(existing) = catalog.find_by_fingerprint(&fingerprint) {
        return Ok(existing.id.clone());
    }

    let mut attempt = 1usize;
    while let Some(existing) = catalog.get_server(&server.id) {
        if existing.metadata.fingerprint == fingerprint {
            return Ok(existing.id.clone());
        }
        attempt += 1;
        server.id = format!("{base_id}-{attempt}");
    }

    let added = server.id.clone();
    catalog.add_server(server);
    Ok(added)
}

fn split_args(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}
