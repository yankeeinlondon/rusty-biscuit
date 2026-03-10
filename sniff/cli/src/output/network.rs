//! Network section output formatting.

use biscuit_terminal::{
    components::{
        list::UnorderedList,
        prose::Prose,
        renderable::Renderable,
        section::{HeadingLevel, Section},
    },
    terminal::Terminal,
};

pub fn print_network_section(network: &sniff::NetworkInfo) {
    let terminal = Terminal::new();
    let document = build_network_document(network);
    print!("{}", document.display(&terminal));
    println!();
}

fn build_network_document(network: &sniff::NetworkInfo) -> Section {
    let mut document = Section::new(HeadingLevel::h1, "Network");

    if network.permission_denied {
        document.push(Prose::new(
            "<red>Interface enumeration was denied by the host OS.</red>",
        ));
        document.push(build_snapshot_section(network));
        return document;
    }

    document.push(Prose::new(
        "Local interface inventory for this host, plus an external WAN lookup when available.",
    ));
    document.push(build_snapshot_section(network));
    document.push(build_interfaces_section(network));
    document
}

fn build_snapshot_section(network: &sniff::NetworkInfo) -> Section {
    let mut section = Section::new(HeadingLevel::h2, "Snapshot");
    let mut list = UnorderedList::empty();

    list.add(Prose::new(format!(
        "<bold>Primary interface:</bold> {}",
        network
            .primary_interface
            .as_deref()
            .unwrap_or("Not detected")
    )));
    list.add(Prose::new(format!(
        "<bold>WAN IP address:</bold> {}",
        network.wan_ip_address.as_deref().unwrap_or("Unavailable")
    )));
    list.add(Prose::new(format!(
        "<bold>Interfaces detected:</bold> {}",
        network.interfaces.len()
    )));
    list.add(Prose::new(format!(
        "<bold>IPv4 addresses:</bold> {}",
        network.ip_addresses.v4.len()
    )));
    list.add(Prose::new(format!(
        "<bold>IPv6 addresses:</bold> {}",
        network.ip_addresses.v6.len()
    )));

    section.push(list);
    section
}

fn build_interfaces_section(network: &sniff::NetworkInfo) -> Section {
    let mut section = Section::new(HeadingLevel::h2, "Interfaces");

    if network.interfaces.is_empty() {
        section.push(Prose::new("No interfaces were returned by the host."));
        return section;
    }

    for interface in &network.interfaces {
        section.push(build_interface_section(
            interface,
            network.primary_interface.as_deref(),
        ));
    }

    section
}

fn build_interface_section(
    interface: &sniff::network::NetworkInterface,
    primary_interface: Option<&str>,
) -> Section {
    let mut section = Section::new(HeadingLevel::h3, &interface.name);
    let mut list = UnorderedList::empty();

    list.add(Prose::new(format!(
        "<bold>Status:</bold> {}",
        format_interface_status(interface, primary_interface)
    )));
    list.add(Prose::new(format!(
        "<bold>MAC address:</bold> {}",
        interface.mac_address.as_deref().unwrap_or("Unavailable")
    )));
    list.add(Prose::new(format!(
        "<bold>IPv4 addresses:</bold> {}",
        format_addresses(
            interface
                .ipv4_addresses
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        )
    )));
    list.add(Prose::new(format!(
        "<bold>IPv6 addresses:</bold> {}",
        format_addresses(
            interface
                .ipv6_addresses
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        )
    )));

    section.push(list);
    section
}

fn format_interface_status(
    interface: &sniff::network::NetworkInterface,
    primary_interface: Option<&str>,
) -> String {
    let mut states = Vec::with_capacity(4);

    states.push(if interface.flags.is_up {
        "<green>up</green>".to_string()
    } else {
        "<red>down</red>".to_string()
    });

    if interface.flags.is_running {
        states.push("<cyan>running</cyan>".to_string());
    }

    if interface.flags.is_loopback {
        states.push("<dim>loopback</dim>".to_string());
    }

    if primary_interface == Some(interface.name.as_str()) {
        states.push("<yellow>primary</yellow>".to_string());
    }

    states.join(", ")
}

fn format_addresses(addresses: Vec<String>) -> String {
    if addresses.is_empty() {
        "None".to_string()
    } else {
        addresses.join(", ")
    }
}
