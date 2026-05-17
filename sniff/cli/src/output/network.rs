//! Network section output formatting.

use biscuit_terminal::{
    components::{
        compose::Compose,
        list::UnorderedList,
        prose::Prose,
        renderable::{RenderableTerminalContent, TerminalRenderable},
    },
    terminal::Terminal,
    utils::layout::Margin,
};

pub fn render_network_section(network: &sniff::NetworkInfo, verbose: u8) -> String {
    let terminal = Terminal::new();
    let document = build_network_document(network, verbose);
    let mut out = document.display(&terminal).to_string();
    out.push('\n');
    out
}

fn build_network_document(network: &sniff::NetworkInfo, verbose: u8) -> Compose {
    let mut document = Compose::default();

    document.add_prose(Prose::new("<bold><underline>Network</underline></bold>"));
    document.add_text("\n");

    if network.permission_denied {
        document.add_prose(Prose::new(
            "<red>Interface enumeration was denied by the host OS.</red>",
        ));
        document.add_text("\n");
        document.add_unordered_list(build_summary_list(network, verbose));
        return document;
    }

    document.add_unordered_list(build_summary_list(network, verbose));

    if verbose == 0 {
        return document;
    }

    let interfaces = build_interface_blocks(network, verbose);
    if !interfaces.is_empty() {
        document.add_text("\n");
        document.add_prose(Prose::new("<bold>Interfaces</bold>"));
        document.add_text("\n");
        for (index, block) in interfaces.into_iter().enumerate() {
            if index > 0 {
                document.add_text("\n");
            }
            document.add_unordered_list(block);
        }
    }

    document
}

fn build_summary_list(network: &sniff::NetworkInfo, verbose: u8) -> UnorderedList {
    let mut list = UnorderedList::empty();
    let active_non_loopback = network
        .interfaces
        .iter()
        .filter(|interface| interface.flags.is_up && !interface.flags.is_loopback)
        .count();

    list.add(Prose::new(format!(
        "<bold>Primary interface:</bold> {}",
        network
            .primary_interface
            .as_deref()
            .unwrap_or("Not detected")
    )));

    if let Some(wan_ip) = &network.wan_ip_address {
        list.add(Prose::new(format!("<bold>WAN IP address:</bold> {wan_ip}")));
    }

    list.add(Prose::new(format!(
        "<bold>Active interfaces:</bold> {} of {}",
        active_non_loopback,
        network.interfaces.len()
    )));

    if !network.ip_addresses.v4.is_empty() {
        let summary = if let Some(summary) = summarize_active_ipv4_addresses(network) {
            summary
        } else {
            format!("{} total", network.ip_addresses.v4.len())
        };
        list.add(Prose::new(format!("<bold>IPv4:</bold> {summary}")));
    }

    if !network.ip_addresses.v6.is_empty() {
        let summary = if let Some(summary) = summarize_active_ipv6_addresses(network) {
            summary
        } else {
            format!("{} total", network.ip_addresses.v6.len())
        };
        list.add(Prose::new(format!("<bold>IPv6:</bold> {summary}")));
    }

    if verbose > 1 {
        list.add(Prose::new(format!(
            "<bold>Total local addresses:</bold> {}",
            network.ip_addresses.v4.len() + network.ip_addresses.v6.len()
        )));
    }

    list
}

fn build_interface_blocks(network: &sniff::NetworkInfo, verbose: u8) -> Vec<UnorderedList> {
    network
        .interfaces
        .iter()
        .filter(|interface| verbose > 1 || interface.flags.is_up || interface.flags.is_loopback)
        .map(|interface| {
            let mut block = UnorderedList::empty();
            let label = format!(
                "<bold>{}</bold> <dim>({})</dim>",
                interface.name,
                format_interface_status(interface, network.primary_interface.as_deref())
            );
            block.add(Prose::new(label));

            if let Some(mac_address) = &interface.mac_address
                && verbose > 1
            {
                block.add(indented_line(format!(
                    "<bold>MAC address:</bold> {mac_address}"
                )));
            }

            if !interface.ipv4_addresses.is_empty() {
                block.add(indented_line(format!(
                    "<bold>IPv4:</bold> {}",
                    interface
                        .ipv4_addresses
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }

            if !interface.ipv6_addresses.is_empty() {
                block.add(indented_line(format!(
                    "<bold>IPv6:</bold> {}",
                    interface
                        .ipv6_addresses
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }

            block
        })
        .collect()
}

fn indented_line(content: String) -> RenderableTerminalContent {
    Prose::new(content)
        .with_left_margin(Margin::Chars(2))
        .into()
}

fn format_interface_status(
    interface: &sniff::network::NetworkInterface,
    primary_interface: Option<&str>,
) -> String {
    let mut states = Vec::with_capacity(4);

    states.push(if interface.flags.is_up {
        "up".to_string()
    } else {
        "down".to_string()
    });

    if interface.flags.is_running {
        states.push("running".to_string());
    }

    if interface.flags.is_loopback {
        states.push("loopback".to_string());
    }

    if primary_interface == Some(interface.name.as_str()) {
        states.push("primary".to_string());
    }

    states.join(", ")
}

fn summarize_active_ipv4_addresses(network: &sniff::NetworkInfo) -> Option<String> {
    summarize_active_interface_addresses(network, |interface| {
        interface
            .ipv4_addresses
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    })
}

fn summarize_active_ipv6_addresses(network: &sniff::NetworkInfo) -> Option<String> {
    summarize_active_interface_addresses(network, |interface| {
        interface
            .ipv6_addresses
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    })
}

fn summarize_active_interface_addresses<F>(
    network: &sniff::NetworkInfo,
    address_fn: F,
) -> Option<String>
where
    F: Fn(&sniff::network::NetworkInterface) -> Vec<String>,
{
    let summaries: Vec<_> = network
        .interfaces
        .iter()
        .filter(|interface| interface.flags.is_up && !interface.flags.is_loopback)
        .filter_map(|interface| {
            let addresses = address_fn(interface);
            if addresses.is_empty() {
                None
            } else {
                Some((interface.name.as_str(), addresses))
            }
        })
        .collect();

    if summaries.is_empty() {
        return None;
    }

    if summaries.len() == 1 {
        return summaries
            .into_iter()
            .next()
            .map(|(_, addresses)| addresses.join(", "));
    }

    Some(
        summaries
            .into_iter()
            .map(|(name, addresses)| format!("{name} {}", addresses.join(", ")))
            .collect::<Vec<_>>()
            .join("; "),
    )
}
