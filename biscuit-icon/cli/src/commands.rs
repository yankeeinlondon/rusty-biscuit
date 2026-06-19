use std::collections::BTreeSet;
use std::io::IsTerminal;

use biscuit_icon::Icon;
use biscuit_icon::cache::{IconCache, SetInfo};
use biscuit_icon::catalog::{self, IconMeta};
use biscuit_icon::iconify::IconifyClient;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

use crate::args::{CacheAction, Commands, ShowFlags};
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
pub async fn run(command: Commands, nerd: bool, verbose: u8) -> Result<()> {
    let client = client_from_env();
    run_with_client(command, nerd, verbose, &client).await
}

/// Runs the resolved command with an injectable Iconify client (used in tests).
pub async fn run_with_client(
    command: Commands,
    nerd: bool,
    verbose: u8,
    client: &IconifyClient,
) -> Result<()> {
    match command {
        Commands::Show { ids, show } => {
            let ShowFlags { from, svg, code_block, css, meta, list, pick } = show;
            run_show(ids, from, svg, code_block, css, meta, list, pick, nerd, client).await
        }
        Commands::Sets { filter } => sets(filter, client).await,
        Commands::Cache { action: CacheAction::List } => {
            let cache = IconCache::open_default()?;
            let term = build_terminal();
            cache_list(&cache, &term, nerd)
        }
        Commands::Cache {
            action: CacheAction::Clear { filter },
        } => {
            let cache = IconCache::open_default()?;
            match filter {
                Some(f) => {
                    let count = cache.clear_filtered(&f)?;
                    println!("{count} icon(s) cleared");
                }
                None => {
                    cache.clear()?;
                    println!("cache cleared");
                }
            }
            Ok(())
        }
        Commands::Domain { arg, show } => domain(arg, show, verbose, nerd).await,
        Commands::Completions { .. } => Ok(()), // handled in main before dispatch
    }
}

/// Validate that at most one format flag is set.
fn check_format_exclusivity(svg: bool, code_block: bool, css: bool) -> Result<()> {
    let count = [svg, code_block, css].iter().filter(|&&b| b).count();
    if count > 1 {
        return Err(eyre!(
            "--svg, --code-block, and --css are mutually exclusive"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_show(
    ids: Vec<String>,
    from: Option<String>,
    svg: bool,
    code_block: bool,
    css: bool,
    meta: bool,
    list: bool,
    pick: bool,
    nerd: bool,
    client: &IconifyClient,
) -> Result<()> {
    check_format_exclusivity(svg, code_block, css)?;

    let term = build_terminal();

    let allowed: BTreeSet<String> = from
        .as_ref()
        .map(|s| s.split(',').map(str::trim).filter(|p| !p.is_empty()).map(String::from).collect())
        .unwrap_or_default();

    let cache = IconCache::open_default()?;

    // When no ids are given, list offline icons only.
    if ids.is_empty() {
        return show_offline_list(&cache, &allowed, &term, nerd, client).await;
    }

    let fmt = Format { svg, code_block, css, nerd };

    // Single id without colon → inexact-match dispatch (4.6).
    if ids.len() == 1 && !ids[0].contains(':') {
        return show_inexact_match(&ids[0], &allowed, &cache, client, &term, &fmt, meta, list, pick)
            .await;
    }

    // One or more explicit ids (contain ':') → direct lookups (4.1, 4.7).
    let resolved = resolve_ids(&ids, &allowed, &cache, client).await?;
    emit_resolved(&resolved, &cache, &term, &fmt, meta)
}

async fn show_offline_list(
    cache: &IconCache,
    allowed: &BTreeSet<String>,
    term: &Terminal,
    nerd: bool,
    client: &IconifyClient,
) -> Result<()> {
    let offline = catalog::offline_icons(cache, "", allowed)?;
    let fmt = Format { svg: false, code_block: false, css: false, nerd };
    let mut errors = Vec::new();
    for id in &offline {
        match lookup_icon(id, cache, client).await {
            Ok(icon) => println!("{}", fmt.render_list_line(&icon, id, term)),
            Err(err) => {
                eprintln!("{id}: {err}");
                errors.push(id.clone());
            }
        }
    }
    if offline.is_empty() {
        return Err(eyre!("no icons available offline"));
    }
    if !errors.is_empty() {
        return Err(eyre!("{} offline icon(s) could not be rendered", errors.len()));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Format {
    svg: bool,
    code_block: bool,
    css: bool,
    nerd: bool,
}

impl Format {
    fn render(&self, icon: &Icon, term: &Terminal) -> String {
        if self.svg {
            return icon.svg();
        }
        if self.css {
            return icon.css();
        }
        if self.code_block {
            return render_code_block(icon);
        }
        icon.clone().nerd_font(self.nerd).render(term)
    }

    /// Renders `icon` for a one-per-line list and appends the icon's id,
    /// unless the rendered output is just the id itself (the text fallback
    /// used when the icon has no glyph and the terminal cannot inline an
    /// image). In that fallback case the line is the id alone — printing
    /// `<id>  <id>` is a tell that the visual is missing.
    fn render_list_line(&self, icon: &Icon, id: &str, term: &Terminal) -> String {
        let rendered = self.render(icon, term);
        let visible = biscuit_terminal::utils::escape_codes::strip_escape_codes(&rendered);
        if visible.trim() == id {
            id.to_string()
        } else {
            format!("{rendered}  {id}")
        }
    }
}

fn render_code_block(icon: &Icon) -> String {
    let svg = icon.svg();
    let md_content = format!("```svg\n{svg}\n```");
    let md = darkmatter::markdown::Markdown::new(md_content);
    let opts = darkmatter::markdown::output::TerminalOptions::default();
    md.as_terminal(opts).unwrap_or(svg)
}

async fn resolve_ids(
    ids: &[String],
    allowed: &BTreeSet<String>,
    cache: &IconCache,
    client: &IconifyClient,
) -> Result<Vec<(String, Icon)>> {
    let mut resolved = Vec::with_capacity(ids.len());
    for id in ids {
        if !id.contains(':') {
            return Err(eyre!("{id:?} is not a valid icon identifier; expected `prefix:name`"));
        }
        if !catalog_allowed_prefix(id, allowed) {
            return Err(eyre!(
                "{id:?} is not in the allowed set; try `icon --from <csv> <prefix:name>`"
            ));
        }
        match lookup_icon(id, cache, client).await {
            Ok(icon) => resolved.push((id.clone(), icon)),
            Err(err) => {
                // 4.7: print error + suggestions
                eprintln!("{id}: {err}");
                if let Some((_prefix, name)) = id.split_once(':') {
                    match catalog::suggestions(cache, name) {
                        Ok(suggs) if !suggs.is_empty() => {
                            eprintln!("suggestions:");
                            for s in &suggs {
                                eprintln!("  {s}");
                            }
                        }
                        _ => {}
                    }
                }
                return Err(eyre!("icon does not exist: {id}"));
            }
        }
    }
    Ok(resolved)
}

fn emit_resolved(
    resolved: &[(String, Icon)],
    cache: &IconCache,
    term: &Terminal,
    fmt: &Format,
    meta: bool,
) -> Result<()> {
    if meta {
        emit_meta_table(resolved, cache, term, fmt)
    } else if resolved.len() == 1 {
        let (_, icon) = &resolved[0];
        println!("{}", fmt.render(icon, term));
        Ok(())
    } else {
        emit_display_table(resolved, term, fmt)
    }
}

fn emit_display_table(
    resolved: &[(String, Icon)],
    term: &Terminal,
    fmt: &Format,
) -> Result<()> {
    let data: Vec<Vec<TableCellContent>> = resolved
        .iter()
        .map(|(id, icon)| {
            vec![id.clone().into(), fmt.render(icon, term).into()]
        })
        .collect();

    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("ID"),
            TableColumn::new("Display"),
        ])
        .with_data(data)
        .alternate_background_color();

    println!("{}", table.render(term));
    Ok(())
}

fn emit_meta_table(
    resolved: &[(String, Icon)],
    cache: &IconCache,
    term: &Terminal,
    fmt: &Format,
) -> Result<()> {
    let mut rows_meta: Vec<(String, Icon, IconMeta)> = Vec::with_capacity(resolved.len());
    for (id, icon) in resolved {
        let m = catalog::icon_meta(cache, icon)?;
        rows_meta.push((id.clone(), icon.clone(), m));
    }

    let has_author = rows_meta.iter().any(|(_, _, m)| m.author.is_some());
    let has_license = rows_meta.iter().any(|(_, _, m)| m.license.is_some());

    let mut columns = vec![
        TableColumn::new("Set"),
        TableColumn::new("Icon"),
    ];

    let mut base_data: Vec<Vec<TableCellContent>> = Vec::with_capacity(resolved.len());
    for (_, icon, m) in &rows_meta {
        let mut row = vec![
            m.set.clone().into(),
            m.icon.clone().into(),
        ];
        row.push(m.categories.clone().into());
        row.push(m.tags.clone().into());
        if has_author {
            row.push(m.author.clone().unwrap_or_default().into());
        }
        if has_license {
            row.push(m.license.clone().unwrap_or_default().into());
        }
        row.push(fmt.render(icon, term).into());
        base_data.push(row);
    }

    columns.push(TableColumn::new("Categories"));
    columns.push(TableColumn::new("Tags"));
    if has_author {
        columns.push(TableColumn::new("Author"));
    }
    if has_license {
        columns.push(TableColumn::new("License"));
    }
    columns.push(TableColumn::new("Display"));

    let table = Table::new()
        .with_columns(columns)
        .with_data(base_data)
        .alternate_background_color();

    println!("{}", table.render(term));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn show_inexact_match(
    needle: &str,
    allowed: &BTreeSet<String>,
    cache: &IconCache,
    client: &IconifyClient,
    term: &Terminal,
    fmt: &Format,
    meta: bool,
    list: bool,
    pick: bool,
) -> Result<()> {
    const MAX_RESULTS: usize = 100;
    const CONCURRENCY: usize = 10;

    let offline = catalog::offline_icons(cache, needle, allowed)?;

    let mut all_ids: Vec<String> = offline.clone();
    let mut online_total: Option<usize> = None;

    if !needle.is_empty() {
        let allowed_vec: Vec<String> = allowed.iter().cloned().collect();
        let prefixes = if allowed_vec.is_empty() { None } else { Some(allowed_vec.as_slice()) };

        if let Ok((hits, total)) = client.search_icons(needle, Some(MAX_RESULTS), prefixes).await {
            let seen: std::collections::HashSet<_> = offline.iter().cloned().collect();
            for id in hits {
                if !seen.contains(&id) {
                    all_ids.push(id);
                }
            }
            online_total = Some(total);
        }
    }

    if all_ids.is_empty() {
        return Err(eyre!("no icons match {needle:?}"));
    }

    let is_tty = std::io::stdout().is_terminal();

    if pick && !is_tty {
        return Err(eyre!("--pick requires an interactive terminal"));
    }

    // Non-TTY or --list: list every match one per line.
    if list || !is_tty {
        let result = list_matches(&all_ids, cache, client, term, fmt, meta, CONCURRENCY).await;
        if let Some(total) = online_total && total > MAX_RESULTS {
            println!(
                "… {} more result(s) available online; use a more specific filter",
                total - MAX_RESULTS
            );
        }
        return result;
    }

    // TTY + exactly 1 match → auto-render.
    if all_ids.len() == 1 {
        let id = &all_ids[0];
        let icon = lookup_icon(id, cache, client).await?;
        if meta {
            let resolved = vec![(id.clone(), icon)];
            return emit_meta_table(&resolved, cache, term, fmt);
        }
        println!("{}", fmt.render(&icon, term));
        return Ok(());
    }

    // TTY + ≥2 matches → picker.
    if pick || is_tty {
        return launch_picker(&all_ids, cache, client, term, fmt, meta).await;
    }

    list_matches(&all_ids, cache, client, term, fmt, meta, CONCURRENCY).await
}

async fn list_matches(
    ids: &[String],
    cache: &IconCache,
    client: &IconifyClient,
    term: &Terminal,
    fmt: &Format,
    meta: bool,
    concurrency: usize,
) -> Result<()> {
    let mut resolved = Vec::new();
    let mut errors = Vec::new();

    for chunk in ids.chunks(concurrency) {
        let mut handles = Vec::with_capacity(chunk.len());
        for id in chunk {
            let id = id.clone();
            let cache = cache.clone();
            let client = client.clone();
            handles.push(tokio::spawn(async move {
                let icon = lookup_icon(&id, &cache, &client).await;
                (id, icon)
            }));
        }
        for handle in handles {
            let (id, result) = handle.await.map_err(|e| eyre!("task join failed: {e}"))?;
            match result {
                Ok(icon) => {
                    if !meta {
                        println!("{}", fmt.render_list_line(&icon, &id, term));
                    }
                    resolved.push((id, icon));
                }
                Err(err) => {
                    eprintln!("{id}: {err}");
                    errors.push(id);
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(eyre!("{} icon(s) could not be fetched", errors.len()));
    }

    if meta {
        return emit_meta_table(&resolved, cache, term, fmt);
    }

    Ok(())
}

async fn launch_picker(
    ids: &[String],
    cache: &IconCache,
    client: &IconifyClient,
    term: &Terminal,
    fmt: &Format,
    meta: bool,
) -> Result<()> {
    use biscuit_tui::prelude::*;

    let options: Vec<ChoiceOption> = ids
        .iter()
        .map(|id| ChoiceOption::new(id.as_str(), id.as_str(), id.clone()))
        .collect();

    let state = ChooseManyState::from_options(options);
    let component = ChooseMany::new();

    match biscuit_tui::core::run_standalone(component, state, None) {
        Ok(picked) => {
            if picked.is_empty() {
                return Ok(());
            }
            let mut resolved = Vec::with_capacity(picked.len());
            for id in &picked {
                let icon = lookup_icon(id, cache, client).await?;
                resolved.push((id.to_string(), icon));
            }
            if meta {
                return emit_meta_table(&resolved, cache, term, fmt);
            }
            if resolved.len() == 1 {
                let (_, icon) = &resolved[0];
                println!("{}", fmt.render(icon, term));
                return Ok(());
            }
            emit_display_table(&resolved, term, fmt)
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::Interrupted {
                std::process::exit(130);
            }
            if err.kind() == std::io::ErrorKind::ConnectionAborted {
                std::process::exit(130);
            }
            Err(eyre!("picker failed: {err}"))
        }
    }
}

async fn domain(arg: Option<String>, show: ShowFlags, verbose: u8, nerd: bool) -> Result<()> {
    let term = build_terminal();
    let fmt = Format {
        svg: show.svg,
        code_block: show.code_block,
        css: show.css,
        nerd,
    };
    check_format_exclusivity(fmt.svg, fmt.code_block, fmt.css)?;
    match arg {
        None => domain_sets_table(&term),
        Some(a) => {
            if let Some((set, _variant)) = a.split_once(':') {
                if !biscuit_icon::domain::is_domain_set(set) {
                    return Err(eyre!("not a curated enum"));
                }
                match biscuit_icon::domain::domain_icon(&a) {
                    Some(icon) => {
                        println!("{}", fmt.render(&icon, &term));
                        Ok(())
                    }
                    None => Err(eyre!("not a curated enum")),
                }
            } else if fmt.svg || fmt.code_block || fmt.css {
                Err(eyre!(
                    "--svg, --code-block, and --css only apply to the single-icon form `icon domain <set>:<variant>`; got `icon domain <set>` (variants table)"
                ))
            } else {
                match biscuit_icon::domain::domain_variants(&a) {
                    Some(variants) => {
                        domain_variants_table(&variants, &term, &fmt, verbose > 0)
                    }
                    None => {
                        let sets = biscuit_icon::domain::domain_sets();
                        let matches: Vec<_> = sets
                            .into_iter()
                            .filter(|(name, _)| name.to_lowercase().contains(&a.to_lowercase()))
                            .collect();
                        if matches.is_empty() {
                            return Err(eyre!("no domain set matches {a:?}"));
                        }
                        for (name, _) in matches {
                            println!("{name}");
                        }
                        Ok(())
                    }
                }
            }
        }
    }
}

fn domain_sets_table(term: &Terminal) -> Result<()> {
    let sets = biscuit_icon::domain::domain_sets();
    let data: Vec<Vec<TableCellContent>> = sets
        .into_iter()
        .map(|(name, count)| {
            vec![name.to_string().into(), TableCellContent::Integer(count as i64)]
        })
        .collect();

    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Domain Set"),
            TableColumn::new("Variant Count"),
        ])
        .with_data(data)
        .alternate_background_color();

    println!("{}", table.render(term));
    Ok(())
}

fn domain_variants_table(
    variants: &[biscuit_icon::domain::DomainVariant],
    term: &Terminal,
    fmt: &Format,
    verbose: bool,
) -> Result<()> {
    use biscuit_icon::Icon;
    use biscuit_terminal::components::prose::Prose;

    let data: Vec<Vec<TableCellContent>> = variants
        .iter()
        .map(|v| {
            let mut row: Vec<TableCellContent> = vec![v.name.clone().into()];
            // The `Icon` cell renders the variant through a small-cell
            // ladder: Unicode/Nerd Font glyph when available, otherwise a
            // 1-cell-wide inline image, otherwise the iconify id in dim
            // style. Inline images are constrained to 1 cell so the
            // surrounding table grid is not destroyed.
            let icon_cell = biscuit_icon::domain::icon_for_id(v.iconify_id)
                .map(|icon: Icon| render_table_icon_cell(&icon, term, fmt))
                .unwrap_or_else(|| {
                    TableCellContent::StyledProse(Box::new(Prose::new("<dim>—</dim>")))
                });
            row.push(icon_cell);
            if verbose {
                row.push(v.iconify_id.to_string().into());
            }
            row
        })
        .collect();

    let mut columns = vec![
        TableColumn::new("Variant"),
        TableColumn::new("Icon"),
    ];
    if verbose {
        columns.push(TableColumn::new("Iconify ID"));
    }

    let table = Table::new()
        .with_columns(columns)
        .with_data(data)
        .alternate_background_color();

    println!("{}", table.render(term));
    Ok(())
}

/// Renders one icon into a single table cell. The ladder is:
/// 1. Unicode glyph (always preferred — single character, fits any cell)
/// 2. Nerd Font glyph (when `fmt.nerd` is set and the icon has one)
/// 3. 1-cell-wide inline image (when the terminal supports it; uses
///    [`Icon::render_in_cell`] so the image does not overflow the cell)
/// 4. Dimmed iconify id (so the user still sees *something* identifying
///    the icon, even in a plain xterm without an image protocol)
fn render_table_icon_cell(icon: &Icon, term: &Terminal, fmt: &Format) -> TableCellContent {
    use biscuit_terminal::components::prose::Prose;

    if icon.unicode_char().is_some()
        || (fmt.nerd && icon.nerd_font_char().is_some())
    {
        return TableCellContent::Text(fmt.render(icon, term));
    }

    #[cfg(feature = "image")]
    {
        if let Ok(rendered) = icon.render_in_cell(term, 1) {
            return TableCellContent::Text(rendered);
        }
    }
    let _ = term;

    // Fall back to the iconify id in dim style so the cell is still
    // informative. The id is also available as a dedicated column with
    // `--verbose`, but losing the visual reference is worse than a hint
    // at the identifier.
    TableCellContent::StyledProse(Box::new(Prose::new(format!(
        "<dim>{}</dim>",
        Prose::escape_text(icon.id())
    ))))
}

fn cache_list(cache: &IconCache, term: &Terminal, nerd: bool) -> Result<()> {
    let icons = cache.list_icons()?;
    if icons.is_empty() {
        println!("no cached icons");
        return Ok(());
    }

    let show_display = nerd
        || icons.iter().any(|ci| has_domain_glyph(&format!("{}:{}", ci.prefix, ci.name)))
        || has_image_support(term);

    let mut rows = Vec::with_capacity(icons.len());
    for ci in &icons {
        let meta = catalog::cached_icon_meta(cache, ci)?;
        let mut row = vec![
            meta.set.clone().into(),
            meta.icon.clone().into(),
        ];
        if show_display {
            let id = format!("{}:{}", ci.prefix, ci.name);
            let display = biscuit_icon::domain::icon_for_id(&id)
                .map(|icon| icon.clone().nerd_font(nerd).render(term))
                .unwrap_or_default();
            row.push(display.into());
        }
        row.push(meta.categories.clone().into());
        row.push(meta.tags.clone().into());
        rows.push(row);
    }

    let mut columns = vec![
        TableColumn::new("Set"),
        TableColumn::new("Icon"),
    ];
    if show_display {
        columns.push(TableColumn::new("Display"));
    }
    columns.push(TableColumn::new("Categories"));
    columns.push(TableColumn::new("Tags"));

    let table = Table::new()
        .with_columns(columns)
        .with_data(rows)
        .alternate_background_color();

    println!("{}", table.render(term));
    Ok(())
}

fn has_domain_glyph(id: &str) -> bool {
    biscuit_icon::domain::icon_for_id(id)
        .and_then(|icon| icon.unicode_char().or(icon.nerd_font_char()))
        .is_some()
}

fn has_image_support(term: &Terminal) -> bool {
    #[cfg(feature = "image")]
    {
        use biscuit_terminal::discovery::detection::ImageSupport;
        !matches!(term.image_support, ImageSupport::None)
    }
    #[cfg(not(feature = "image"))]
    {
        let _ = term;
        false
    }
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

/// Builds a [`SetInfo`] from a [`CollectionInfo`], mapping license and
/// metadata fields.
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
    let tags = if info.tags.is_empty() {
        None
    } else {
        Some(info.tags.join(", "))
    };
    SetInfo {
        prefix: info.prefix.clone(),
        title: info.title.clone(),
        license: license_str,
        license_title,
        license_url,
        total: info.total,
        author_name: info.author_name.clone(),
        author_url: info.author_url.clone(),
        tags,
        category: info.category.clone(),
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

fn build_terminal() -> Terminal {
    if let (Ok(w), Ok(h)) = (
        std::env::var("BISCUIT_TERM_WIDTH"),
        std::env::var("BISCUIT_TERM_HEIGHT"),
    ) && let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
        return Terminal::builder().width(w).height(h).build();
    }
    Terminal::new()
}

fn catalog_allowed_prefix(id: &str, allowed: &BTreeSet<String>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    id.split_once(':')
        .map(|(prefix, _)| allowed.contains(prefix))
        .unwrap_or(false)
}
