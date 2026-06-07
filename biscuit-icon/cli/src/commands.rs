use biscuit_icon::Icon;
use biscuit_icon::cache::IconCache;
use biscuit_icon::iconify::IconifyClient;
use biscuit_icon::render::NerdFontMode;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

use crate::args::{CacheAction, Commands};

/// Runs the resolved command.
pub async fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Icons { filter, from } => icons(filter, from).await,
        Commands::Sets { filter } => sets(filter).await,
        Commands::Cache { action: CacheAction::Clear } => {
            IconCache::open_default()?.clear()?;
            println!("cache cleared");
            Ok(())
        }
        Commands::Completions { .. } => Ok(()), // handled in main before dispatch
    }
}

async fn icons(filter: Option<String>, _from: Option<String>) -> Result<()> {
    let term = Terminal::new();
    let needle = filter.unwrap_or_default();
    // A `prefix:name` filter is a direct lookup+render.
    if needle.contains(':') {
        let icon = Icon::iconify(&needle).await?;
        println!("{}  {needle}", icon.render_terminal(&term, NerdFontMode::Off));
        return Ok(());
    }
    // Otherwise list matching cached icons.
    let cache = IconCache::open_default()?;
    let hits = cache.search_names(&needle)?;
    if hits.is_empty() {
        return Err(eyre!("no cached icons match {needle:?}; try `icon <prefix:name>` to fetch"));
    }
    for id in hits {
        let icon = Icon::iconify(&id).await?;
        println!("{}  {id}", icon.render_terminal(&term, NerdFontMode::Off));
    }
    Ok(())
}

async fn sets(filter: Option<String>) -> Result<()> {
    let client = IconifyClient::new();
    let needle = filter.unwrap_or_default().to_lowercase();
    for (prefix, title) in client.fetch_collections().await? {
        if needle.is_empty()
            || prefix.to_lowercase().contains(&needle)
            || title.to_lowercase().contains(&needle)
        {
            println!("{prefix}\t{title}");
        }
    }
    Ok(())
}
