use std::collections::BTreeSet;

use biscuit_icon::Icon;
use biscuit_icon::cache::{IconCache, SetInfo};
use biscuit_icon::catalog;
use biscuit_icon::iconify::IconifyClient;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

use crate::args::{CacheAction, Commands};
use crate::sets_table::{SetRow, render_sets};

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
    const MAX_RESULTS: usize = 100;
    const CONCURRENCY: usize = 10;

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
    let mut errors = Vec::new();

    for id in &offline {
        match lookup_icon(id, &cache, client).await {
            Ok(icon) => println!("{}  {id}", render_icon(&icon, &term, nerd)),
            Err(err) => {
                eprintln!("{id}: {err}");
                errors.push(id.clone());
            }
        }
    }

    // Skip online search when there is no filter (empty query is not supported
    // by the Iconify search endpoint).
    if needle.is_empty() {
        if offline.is_empty() {
            return Err(eyre!("no icons available offline"));
        }
        if !errors.is_empty() {
            return Err(eyre!("{} offline icon(s) could not be rendered", errors.len()));
        }
        return Ok(());
    }

    let allowed_vec: Vec<String> = allowed.iter().cloned().collect();
    let prefixes = if allowed_vec.is_empty() { None } else { Some(allowed_vec.as_slice()) };

    // Paginate through online search results and merge with offline results.
    match client.search_icons(&needle, Some(MAX_RESULTS), prefixes).await {
        Ok((hits, total)) => {
            let seen: std::collections::HashSet<_> = offline.iter().cloned().collect();
            let new_hits: Vec<_> = hits
                .into_iter()
                .filter(|id| !seen.contains(id))
                .collect();
            if offline.is_empty() && new_hits.is_empty() {
                return Err(eyre!(
                    "no icons match {needle:?}; try `icon <prefix:name>` to fetch directly"
                ));
            }

            for chunk in new_hits.chunks(CONCURRENCY) {
                let mut handles = Vec::with_capacity(chunk.len());
                for id in chunk {
                    let id = id.clone();
                    let cache = cache.clone();
                    let client = client.clone();
                    handles.push(tokio::spawn(async move {
                        (id.clone(), Icon::iconify_with(&id, &cache, &client).await)
                    }));
                }
                for handle in handles {
                    let (id, result) = handle.await.map_err(|e| eyre!("task join failed: {e}"))?;
                    match result {
                        Ok(icon) => println!("{}  {id}", render_icon(&icon, &term, nerd)),
                        Err(err) => {
                            eprintln!("{id}: iconify fetch failed: {err}");
                            errors.push(id);
                        }
                    }
                }
            }

            if total > MAX_RESULTS {
                println!(
                    "… {} more result(s) available online; use a more specific filter",
                    total - MAX_RESULTS
                );
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

    if !errors.is_empty() {
        return Err(eyre!("{} icon(s) could not be fetched", errors.len()));
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
            for info in &sets {
                let set_info = set_info_from_collection(info);
                if let Err(err) = cache.put_set(&set_info) {
                    tracing::warn!("failed to cache set metadata for {}: {err}", info.prefix);
                }
            }
            for info in sets {
                if needle.is_empty()
                    || info.prefix.to_lowercase().contains(&needle)
                    || info.title.to_lowercase().contains(&needle)
                {
                    let set_info = set_info_from_collection(&info);
                    offline.retain(|s| s.prefix != info.prefix);
                    offline.push(set_info);
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

    // A successful online fetch that matches nothing leaves no rows. Preserve
    // the command's no-result error contract rather than rendering an empty
    // table. (The offline-fallback branch above already errors when the network
    // is down and no offline rows exist.)
    if offline.is_empty() {
        return Err(eyre!("no icon sets match {needle:?}"));
    }

    let prefixes: Vec<String> = offline.iter().map(|s| s.prefix.clone()).collect();
    let counts = cache.cached_icon_counts(&prefixes)?;

    let rows: Vec<SetRow> = offline
        .into_iter()
        .map(|s| SetRow {
            prefix: s.prefix.clone(),
            title: s.title,
            total: s.total,
            cached: counts.get(&s.prefix).copied().unwrap_or(0),
        })
        .collect();

    let term = if let (Ok(w), Ok(h)) = (
        std::env::var("BISCUIT_TERM_WIDTH"),
        std::env::var("BISCUIT_TERM_HEIGHT"),
    ) {
        if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
            Terminal::builder().width(w).height(h).build()
        } else {
            Terminal::new()
        }
    } else {
        Terminal::new()
    };
    let output = render_sets(&rows, &term);
    println!("{output}");
    Ok(())
}

/// Builds a [`SetInfo`] from a [`CollectionInfo`], mapping license fields.
fn set_info_from_collection(info: &biscuit_icon::iconify::CollectionInfo) -> SetInfo {
    let license_str = info.license.as_ref().and_then(|l| {
        if l.spdx.is_empty() {
            None
        } else {
            Some(l.spdx.clone())
        }
    });
    let license_title = info.license.as_ref().and_then(|l| {
        if l.title.is_empty() {
            None
        } else {
            Some(l.title.clone())
        }
    });
    let license_url = info.license.as_ref().and_then(|l| l.url.clone());
    SetInfo {
        prefix: info.prefix.clone(),
        title: info.title.clone(),
        license: license_str,
        license_title,
        license_url,
        total: info.total,
    }
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
