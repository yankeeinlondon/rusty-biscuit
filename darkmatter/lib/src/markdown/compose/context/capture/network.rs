use serde_json::{Map, Value};
use sniff::network::{DefaultGateways, ScopedIpAddr};

use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["tailnet", "gateway", "gateway_v6"];

/// Interface addresses and default gateways observed once for a request.
///
/// Each half is independent: `None` means that half was not observed, which
/// projects the schema's empty value (`false` / `null`) alongside the capture
/// diagnostic recorded where the observation failed or was not supplied.
#[derive(Debug, Clone, Default)]
pub(super) struct NetworkObservation {
    pub(super) addresses: Option<Vec<ScopedIpAddr>>,
    pub(super) gateways: Option<DefaultGateways>,
}

pub(super) fn populate_network(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let network = cap.network.as_ref();
    values.insert(
        "tailnet".into(),
        Value::Bool(
            network
                .and_then(|network| network.addresses.as_ref())
                .is_some_and(|addresses| sniff::network::contains_cgnat_address(addresses)),
        ),
    );
    let gateways = network.and_then(|network| network.gateways.as_ref());
    values.insert(
        "gateway".into(),
        gateways
            .and_then(|gateways| gateways.v4)
            .map_or(Value::Null, |gateway| Value::String(gateway.to_string())),
    );
    values.insert(
        "gateway_v6".into(),
        gateways
            .and_then(|gateways| gateways.v6.as_ref())
            .map_or(Value::Null, |gateway| Value::String(gateway.to_string())),
    );
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::path::PathBuf;

    use super::*;

    fn project(network: Option<NetworkObservation>) -> Map<String, Value> {
        let mut cap = ContextCapture::for_test_base(PathBuf::from("/tmp"), None);
        cap.network = network;
        let mut values = Map::new();
        populate_network(&cap, &mut values);
        values
    }

    fn address(text: &str) -> ScopedIpAddr {
        ScopedIpAddr::new(text.parse::<IpAddr>().expect("fixture address"))
    }

    /// AC9: a fixture interface inside 100.64.0.0/10 is a tailnet; the range
    /// boundaries are exact.
    #[test]
    fn tailnet_follows_the_cgnat_range_of_fixture_addresses() {
        for (addresses, expected) in [
            (vec!["127.0.0.1", "192.168.1.20", "100.101.102.103"], true),
            (vec!["100.64.0.0"], true),
            (vec!["100.127.255.255"], true),
            (vec!["100.63.255.255", "100.128.0.0", "10.0.0.2", "fe80::1"], false),
            (vec![], false),
        ] {
            let values = project(Some(NetworkObservation {
                addresses: Some(addresses.iter().map(|text| address(text)).collect()),
                gateways: None,
            }));
            assert_eq!(values["tailnet"], Value::Bool(expected), "{addresses:?}");
        }
    }

    /// AC13 projection: parsed gateways render as addresses, a link-local v6
    /// gateway keeps its zone, and an absent route is `null`.
    #[test]
    fn gateways_project_addresses_with_zone_or_null() {
        let values = project(Some(NetworkObservation {
            addresses: None,
            gateways: Some(DefaultGateways {
                v4: Some(Ipv4Addr::new(192, 168, 1, 1)),
                v6: Some(
                    ScopedIpAddr::with_scope("fe80::1".parse::<Ipv6Addr>().unwrap(), "en0")
                        .expect("scoped gateway"),
                ),
            }),
        }));
        assert_eq!(values["gateway"], Value::String("192.168.1.1".into()));
        assert_eq!(values["gateway_v6"], Value::String("fe80::1%en0".into()));
        assert_eq!(values["tailnet"], Value::Bool(false));

        let values = project(Some(NetworkObservation {
            addresses: Some(Vec::new()),
            gateways: Some(DefaultGateways::default()),
        }));
        assert_eq!(values["gateway"], Value::Null);
        assert_eq!(values["gateway_v6"], Value::Null);
    }

    #[test]
    fn unobserved_network_projects_schema_empty_values() {
        let values = project(None);
        assert_eq!(values["tailnet"], Value::Bool(false));
        assert_eq!(values["gateway"], Value::Null);
        assert_eq!(values["gateway_v6"], Value::Null);
    }
}
