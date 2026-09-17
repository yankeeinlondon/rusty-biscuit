//! `route print` parser.
//!
//! Only the headings of the output are localized, so the parser keys on
//! structure: `=` separator lines split the output into blocks, and a block
//! whose first line ends with `:` is a route list. Route lists alternate
//! active, persistent per table, and only active routes are in effect. A
//! gateway that is not an address (`On-link`, in any language) is no gateway.

use std::net::{Ipv4Addr, Ipv6Addr};

use super::{DefaultGateways, LowestMetric, scoped_ipv6_gateway};

pub(super) fn default_gateways(output: &str) -> DefaultGateways {
    let mut v4 = LowestMetric::new();
    let mut v6 = LowestMetric::new();

    for rows in active_route_lists(output) {
        let mut rows = rows.iter();
        while let Some(row) = rows.next() {
            let cols: Vec<&str> = row.split_whitespace().collect();
            // Network Destination, Netmask, Gateway, Interface, Metric. A
            // localized `On-link` may be more than one word.
            if cols.len() >= 5 && cols[0] == "0.0.0.0" && cols[1] == "0.0.0.0" {
                if let Ok(metric) = cols[cols.len() - 1].parse::<u32>() {
                    let gateway = (cols.len() == 5)
                        .then(|| cols[2].parse::<Ipv4Addr>().ok())
                        .flatten()
                        .filter(|gateway| !gateway.is_unspecified());
                    v4.offer(metric, gateway);
                }
                continue;
            }
            // If, Metric, Network Destination, Gateway. A destination too long
            // for its column wraps the gateway onto the next line.
            if cols.len() >= 3 && cols[2] == "::/0" {
                let (Ok(interface), Ok(metric)) = (cols[0].parse::<u32>(), cols[1].parse::<u32>())
                else {
                    continue;
                };
                let gateway_column = if cols.len() == 3 {
                    rows.next().map(|next| next.trim().to_string())
                } else {
                    Some(cols[3..].join(" "))
                };
                let gateway = gateway_column
                    .and_then(|column| column.parse::<Ipv6Addr>().ok())
                    .and_then(|gateway| scoped_ipv6_gateway(gateway, || interface.to_string()));
                v6.offer(metric, gateway);
            }
        }
    }

    DefaultGateways {
        v4: v4.gateway(),
        v6: v6.gateway(),
    }
}

fn active_route_lists(output: &str) -> Vec<Vec<&str>> {
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '=') {
            blocks.push(Vec::new());
        } else if let Some(block) = blocks.last_mut() {
            block.push(line);
        }
    }

    blocks
        .into_iter()
        .filter(|block| {
            block
                .iter()
                .map(|line| line.trim())
                .find(|line| !line.is_empty())
                .is_some_and(|heading| heading.ends_with(':'))
        })
        .step_by(2)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE_PRINT: &str = include_str!("fixtures/windows_route_print.txt");
    const ROUTE_PRINT_ON_LINK: &str = include_str!("fixtures/windows_route_print_on_link.txt");
    const ROUTE_PRINT_NO_DEFAULT: &str =
        include_str!("fixtures/windows_route_print_no_default.txt");

    #[test]
    fn picks_the_lowest_metric_active_default_route_for_both_families() {
        // v4: metric 25 beats 281, the first of two metric-25 routes wins, and
        // the metric-1 persistent route is not in effect. v6: If 12 metric 25
        // beats If 18, and the metric-1 persistent `::/0` is not in effect.
        let gateways = default_gateways(ROUTE_PRINT);
        assert_eq!(gateways.v4, Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(
            gateways.v6.map(|g| g.to_string()),
            Some("fe80::a8bb:ccff:fedd:eeff%12".to_string())
        );
    }

    #[test]
    fn on_link_default_routes_have_no_gateway() {
        let gateways = default_gateways(ROUTE_PRINT_ON_LINK);
        assert_eq!(gateways, DefaultGateways::default());
    }

    #[test]
    fn persistent_only_default_routes_are_not_in_effect() {
        let gateways = default_gateways(ROUTE_PRINT_NO_DEFAULT);
        assert_eq!(gateways, DefaultGateways::default());
        assert_eq!(default_gateways(""), DefaultGateways::default());
    }

    #[test]
    fn a_wrapped_gateway_column_is_read_from_the_next_line() {
        let output = "\
===========================================================================
Active Routes:
 If Metric Network Destination      Gateway
  7     10 ::/0
                                    2001:db8::fffe
===========================================================================
Persistent Routes:
  None
";
        assert_eq!(
            default_gateways(output).v6.map(|g| g.to_string()),
            Some("2001:db8::fffe".to_string())
        );
    }

    #[test]
    fn localized_headings_do_not_change_the_result() {
        let german = ROUTE_PRINT
            .replace("Active Routes:", "Aktive Routen:")
            .replace("Persistent Routes:", "Ständige Routen:")
            .replace("On-link", "Auf Verbindung");
        assert_eq!(default_gateways(&german), default_gateways(ROUTE_PRINT));

        let german_on_link = ROUTE_PRINT_ON_LINK
            .replace("Active Routes:", "Aktive Routen:")
            .replace("Persistent Routes:", "Ständige Routen:")
            .replace("On-link", "Auf Verbindung");
        assert_eq!(default_gateways(&german_on_link), DefaultGateways::default());
    }
}
