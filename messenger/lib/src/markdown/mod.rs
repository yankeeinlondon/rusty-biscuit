pub mod ast;
pub mod discord;
pub mod parse;
pub mod plain_text;
pub mod slack_mrkdwn;
pub mod telegram_html;

use crate::receipt::ProviderKind;
use ast::RichNode;
use parse::parse_markdown;

/// Render a Markdown string for a specific provider.
#[tracing::instrument(skip_all, fields(provider = %provider, input_len = markdown.len()))]
pub fn render_for_provider(markdown: &str, provider: ProviderKind) -> String {
    tracing::trace!("rendering markdown");
    let nodes = parse_markdown(markdown);
    render_nodes_for_provider(&nodes, provider)
}

/// Render a pre-parsed Markdown AST for a specific provider.
#[tracing::instrument(skip_all, fields(provider = %provider, node_count = nodes.len()))]
pub fn render_nodes_for_provider(nodes: &[RichNode], provider: ProviderKind) -> String {
    tracing::trace!("rendering markdown AST");
    match provider {
        ProviderKind::Discord | ProviderKind::DiscordWebhook => discord::render_discord(nodes),
        ProviderKind::Slack | ProviderKind::SlackWebhook => {
            slack_mrkdwn::render_slack_mrkdwn(nodes)
        }
        ProviderKind::Telegram => telegram_html::render_telegram_html(nodes),
        // Signal and WhatsApp don't support rich text
        ProviderKind::Signal | ProviderKind::WhatsApp => plain_text::render_plain_text(nodes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_for_each_provider() {
        let md = "**bold** and _italic_";

        assert_eq!(
            render_for_provider(md, ProviderKind::Discord),
            "**bold** and *italic*"
        );
        assert_eq!(
            render_for_provider(md, ProviderKind::Slack),
            "*bold* and _italic_"
        );
        assert_eq!(
            render_for_provider(md, ProviderKind::SlackWebhook),
            "*bold* and _italic_"
        );
        assert_eq!(
            render_for_provider(md, ProviderKind::Telegram),
            "<b>bold</b> and <i>italic</i>"
        );
        assert_eq!(
            render_for_provider(md, ProviderKind::Signal),
            "bold and italic"
        );
        assert_eq!(
            render_for_provider(md, ProviderKind::WhatsApp),
            "bold and italic"
        );
    }

    #[test]
    fn round_trip_plain_text_strips_formatting() {
        let md = "**bold** _italic_ ~~strike~~ `code` [link](https://example.com)";
        let plain = render_for_provider(md, ProviderKind::Signal);
        // All formatting should be stripped
        assert!(!plain.contains('*'));
        assert!(!plain.contains('_'));
        assert!(!plain.contains('~'));
        assert!(!plain.contains('`'));
        assert!(plain.contains("bold"));
        assert!(plain.contains("italic"));
    }
}
