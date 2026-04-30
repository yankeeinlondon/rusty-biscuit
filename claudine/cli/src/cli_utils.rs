use biscuit_terminal::components::table::table::TableCellContent;
use chrono::NaiveDate;
use claudine::events::AgenticEvent;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use color_eyre::eyre::{Result, eyre};

pub(crate) fn parse_provider(name: &str) -> Result<Provider> {
    parse_provider_clap(name).map_err(|message| eyre!(message))
}

pub(crate) fn parse_provider_clap(name: &str) -> std::result::Result<Provider, String> {
    if let Some(provider) = Provider::parse_cli_name(name) {
        return Ok(provider);
    }

    let supported = PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(Provider::as_slug)
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("Unknown provider: {name}. Supported: {supported}"))
}

pub(crate) fn parse_naive_date(value: &str) -> std::result::Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| format!("invalid date `{value}`: {error}"))
}

pub(crate) fn event_name_pascal(slug: &str) -> String {
    AgenticEvent::from_slug(slug)
        .map(|event| event.as_pascal_case().to_string())
        .unwrap_or_else(|| slug.to_string())
}

pub(crate) fn bool_indicator(value: bool) -> TableCellContent {
    if value {
        "\u{2705}".into()
    } else {
        "\u{274C}".into()
    }
}
