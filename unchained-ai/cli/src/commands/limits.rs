use std::collections::HashMap;

use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use unchained_ai::primitives::services::agent_status::{
    AgentStatus, AgenticCapLimit, AgenticStatusPlatform,
};

/// Parse a platform name string into an `AgenticStatusPlatform`.
fn parse_platform(name: &str) -> Result<AgenticStatusPlatform> {
    match name.to_lowercase().as_str() {
        "claude" | "claudecode" | "claude-code" | "claude_code" => {
            Ok(AgenticStatusPlatform::ClaudeCode)
        }
        "codex" => Ok(AgenticStatusPlatform::Codex),
        _ => Err(eyre!(
            "Unknown platform: '{}'. Valid options: claude, codex",
            name
        )),
    }
}

/// Render limits as JSON to stdout.
fn render_json(limits: &HashMap<String, AgenticCapLimit>) -> Result<()> {
    let json: HashMap<&str, serde_json::Value> = limits
        .iter()
        .map(|(name, limit)| {
            let value = serde_json::json!({
                "short_cap": {
                    "usage": limit.short_cap_usage,
                    "window": format!("{}", limit.short_cap_window_size),
                    "resets": limit.short_cap_until.0,
                },
                "long_cap": {
                    "usage": limit.long_cap_usage,
                    "window": format!("{}", limit.long_cap_window_size),
                    "resets": limit.long_cap_until.0,
                }
            });
            (name.as_str(), value)
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Render limits as terminal progress bars.
fn render_progress_bars(
    limits: &HashMap<String, AgenticCapLimit>,
    installed: &HashMap<AgenticStatusPlatform, bool>,
) {
    let term = Terminal::default();

    // Header
    println!("Agentic Platform Limits\n");

    // Show installed platforms
    for (platform, is_installed) in installed {
        let status = if *is_installed { "installed" } else { "not found" };
        println!("  {} {}", platform, status);
    }
    println!();

    if limits.is_empty() {
        println!("  No limit data available.");
        return;
    }

    // Render progress bars for each platform
    for (name, limit) in limits {
        println!("  {}", name);

        // Short-term cap
        let short_label = format!("    {} cap", limit.short_cap_window_size);
        let short_bar = Progress::new(limit.short_cap_usage)
            .with_label(short_label)
            .with_bar_width(30);
        print!("{}", short_bar.display(&term));

        // Long-term cap
        let long_label = format!("    {} cap", limit.long_cap_window_size);
        let long_bar = Progress::new(limit.long_cap_usage)
            .with_label(long_label)
            .with_bar_width(30);
        print!("{}", long_bar.display(&term));

        println!();
    }
}

/// Run the limits subcommand.
pub async fn run(platform: Option<String>, json: bool) -> Result<()> {
    let platform_filter = platform.map(|p| parse_platform(&p)).transpose()?;

    let status = AgentStatus::new(None)
        .await
        .map_err(|e| eyre!("Failed to detect agent platforms: {}", e))?;

    if status.available.is_empty() {
        if json {
            println!("{{}}");
        } else {
            println!("No agentic platforms detected.");
        }
        return Ok(());
    }

    let limits = status
        .limits(platform_filter)
        .await
        .map_err(|e| eyre!("Failed to query limits: {}", e))?;

    if json {
        render_json(&limits)?;
    } else {
        render_progress_bars(&limits, &status.installed);
    }

    Ok(())
}
