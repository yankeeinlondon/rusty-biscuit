use std::collections::BTreeSet;

use biscuit_icon::Icon;
use biscuit_icon::cache::{IconCache, SetInfo};
use biscuit_icon::catalog;
use biscuit_icon::iconify::IconifyClient;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

use crate::args::{CacheAction, Commands};

const ONLINE_ICON_LIMIT: usize = 20;

/// Builds an Iconify client respecting the `ICONIFY_BASE_URL` env var.
fn client_from_env() -> IconifyClient {
    if let Ok(base) = std::env::var("ICONIFY_BASE_URL") {
        IconifyClient::with_base(base)
    } else {
        IconifyClient::new()
    }
}

/// Runs the resolved command.
pub async fn run(command: Commands, nerd: bool) -> Result<()> {
    let client = client_from_env();
    run_with_client(command, nerd, &client).await
}

/// Runs the resolved command with an injectable Iconify client (used in tests).
pub async fn run_with_client(command: Commands, nerd: bool, client: &IconifyClient) -> Result<()> {
    match command {
        Commands::Icons { filter, from } => icons(filter, from, nerd, client).await,
        Commands::Sets { filter } => sets(filter, client).await,
        Commands::Cache { action: CacheAction::Clear } => {
            IconCache::open_default()?.clear()?;
            println!("cache cleared");
            Ok(())
        }
        Commands::Completions { .. } => Ok(()), // handled in main before dispatch
    }
}

async fn icons(filter: Option<String>, from: Option<String>, nerd: bool, client: &IconifyClient) -> Result<()> {
    let term = Terminal::new();
    let needle = filter.unwrap_or_default();

    let allowed: BTreeSet<String> = from
        .as_ref()
        .map(|s| s.split(',').map(str::trim).filter(|p| !p.is_empty()).map(String::from).collect())
        .unwrap_or_default();

    let cache = IconCache::open_default()?;

    // A `prefix:name` filter is a direct lookup+render, but `--from` still applies.
    if needle.contains(':') {
        if !catalog_allowed_prefix(&needle, &allowed) {
            return Err(eyre!(
                "{needle:?} is not in the allowed set; try `icon --from <csv> <prefix:name>`"
            ));
        }
        let icon = lookup_icon(&needle, &cache, client).await?;
        println!("{}  {needle}", render_icon(&icon, &term, nerd));
        return Ok(());
    }

    let offline = catalog::offline_icons(&cache, &needle, &allowed)?;

    for id in &offline {
        match lookup_icon(id, &cache, client).await {
            Ok(icon) => println!("{}  {id}", render_icon(&icon, &term, nerd)),
            Err(err) => eprintln!("{id}: {err}"),
        }
    }

    // Skip online search when there is no filter (empty query is not supported
    // by the Iconify search endpoint).
    if needle.is_empty() {
        if offline.is_empty() {
            return Err(eyre!("no icons available offline"));
        }
        return Ok(());
    }

    // Paginate through online search results and merge with offline results.
    match client.search_icons(&needle).await {
        Ok(hits) => {
            let seen: std::collections::HashSet<_> = offline.iter().cloned().collect();
            let new_hits: Vec<_> = hits
                .into_iter()
                .filter(|id| catalog_allowed_prefix(id, &allowed) && !seen.contains(id))
                .collect();
            if offline.is_empty() && new_hits.is_empty() {
                return Err(eyre!(
                    "no icons match {needle:?}; try `icon <prefix:name>` to fetch directly"
                ));
            }
            for id in new_hits {
                match Icon::iconify_with(&id, &cache, client).await {
                    Ok(icon) => println!("{}  {id}", render_icon(&icon, &term, nerd)),
                    Err(err) => eprintln!("{id}: {err}"),
                }
            }
        }
        Err(err) => {
            if offline.is_empty() {
                return Err(eyre!(
                    "no offline icons match {needle:?} and the online catalog is unavailable: {err}"
                ));
            }
        }
    }

    Ok(())
}

async fn sets(filter: Option<String>, client: &IconifyClient) -> Result<()> {
    let needle = filter.unwrap_or_default().to_lowercase();
    let cache = IconCache::open_default()?;

    let mut offline = catalog::offline_sets(&cache, &needle)?;
    offline.sort_by(|a, b| a.prefix.cmp(&b.prefix));

    let online = client.fetch_collections().await;

    match online {
        Ok(sets) => {
            // Cache every fetched collection so the full catalog is available
            // for later offline filters, then filter only for presentation.
            for (prefix, title, license) in &sets {
                let license_str = license.as_ref().and_then(|l| {
                    if l.spdx.is_empty() {
                        None
                    } else {
                        Some(l.spdx.clone())
                    }
                });
                let license_title = license.as_ref().and_then(|l| {
                    if l.title.is_empty() {
                        None
                    } else {
                        Some(l.title.clone())
                    }
                });
                let license_url = license.as_ref().and_then(|l| l.url.clone());
                let info = SetInfo {
                    prefix: prefix.clone(),
                    title: title.clone(),
                    license: license_str,
                    license_title,
                    license_url,
                };
                if let Err(err) = cache.put_set(&info) {
                    tracing::warn!("failed to cache set metadata for {prefix}: {err}");
                }
            }
            for (prefix, title, license) in sets {
                if needle.is_empty()
                    || prefix.to_lowercase().contains(&needle)
                    || title.to_lowercase().contains(&needle)
                {
                    let license_str = license.as_ref().and_then(|l| {
                        if l.spdx.is_empty() {
                            None
                        } else {
                            Some(l.spdx.clone())
                        }
                    });
                    let license_title = license.as_ref().and_then(|l| {
                        if l.title.is_empty() {
                            None
                        } else {
                            Some(l.title.clone())
                        }
                    });
                    let license_url = license.as_ref().and_then(|l| l.url.clone());
                    let info = SetInfo {
                        prefix: prefix.clone(),
                        title: title.clone(),
                        license: license_str,
                        license_title,
                        license_url,
                    };
                    offline.retain(|s| s.prefix != prefix);
                    offline.push(info);
                }
            }
        }
        Err(err) => {
            if offline.is_empty() {
                return Err(eyre!(
                    "no offline set listings available and the network catalog is unreachable: {err}"
                ));
            }
        }
    }

    offline.sort_by(|a, b| a.prefix.cmp(&b.prefix));
    for set in offline {
        println!("{}\t{}", set.prefix, set.title);
    }
    Ok(())
}

/// Resolves an identifier to an [`Icon`], preferring the embedded domain
/// catalog so offline listings do not trigger network fetches.
async fn lookup_icon(id: &str, cache: &IconCache, client: &IconifyClient) -> Result<Icon> {
    if let Some(icon) = biscuit_icon::domain::icon_for_id(id) {
        Ok(icon)
    } else {
        Ok(Icon::iconify_with(id, cache, client).await?)
    }
}

fn render_icon(icon: &Icon, term: &Terminal, nerd: bool) -> String {
    icon.clone().nerd_font(nerd).render(term)
}

fn catalog_allowed_prefix(id: &str, allowed: &BTreeSet<String>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    id.split_once(':')
        .map(|(prefix, _)| allowed.contains(prefix))
        .unwrap_or(false)
}
