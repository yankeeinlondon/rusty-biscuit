//! Network section output formatting.

pub fn print_network_section(network: &sniff::NetworkInfo) {
    println!("=== Network ===");
    if network.permission_denied {
        println!("Permission denied - unable to enumerate interfaces");
    } else {
        if let Some(ref primary) = network.primary_interface {
            println!("Primary interface: {}", primary);
        }
        println!("Interfaces: {}", network.interfaces.len());
        for iface in &network.interfaces {
            let status = if iface.flags.is_up { "UP" } else { "DOWN" };
            let loopback = if iface.flags.is_loopback {
                " (loopback)"
            } else {
                ""
            };
            println!("  {} [{}]{}", iface.name, status, loopback);
            for ip in &iface.ipv4_addresses {
                println!("    IPv4: {}", ip);
            }
            for ip in &iface.ipv6_addresses {
                println!("    IPv6: {}", ip);
            }
        }

        // Print aggregated IP addresses (only if there are any)
        if !network.ip_addresses.v4.is_empty() {
            println!();
            println!("All IPv4 Addresses ({}):", network.ip_addresses.v4.len());
            for addr in &network.ip_addresses.v4 {
                println!("  {} ({})", addr.address, addr.interface);
            }
        }

        if !network.ip_addresses.v6.is_empty() {
            println!();
            println!("All IPv6 Addresses ({}):", network.ip_addresses.v6.len());
            for addr in &network.ip_addresses.v6 {
                println!("  {} ({})", addr.address, addr.interface);
            }
        }
    }
    println!();
}
