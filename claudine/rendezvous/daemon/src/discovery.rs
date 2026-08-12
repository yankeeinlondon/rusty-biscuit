//! mDNS service discovery for the rendezvous mesh.
//!
//! Daemons publish a `_agentgrid._udp.local` service record carrying
//! their node ID (hex-encoded public key) plus QUIC port. Browsing
//! daemons receive `ServiceResolved` events for every peer announcing
//! the same service type and forward [`DiscoveredPeer`] messages onto
//! the registry over an mpsc channel.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// mDNS service type advertised by every rendezvous daemon.
pub const SERVICE_TYPE: &str = "_agentgrid._udp.local.";

/// TXT-record key used to carry the node's hex-encoded public key.
pub const NODE_ID_TXT_KEY: &str = "node_id";

/// Errors that surface while configuring or driving mDNS discovery.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// Creating the underlying [`ServiceDaemon`] failed.
    #[error("failed to create mDNS service daemon: {0}")]
    Daemon(#[from] mdns_sd::Error),
}

/// A peer observed via mDNS browsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredPeer {
    /// Hex-encoded `node_id` lifted from the peer's TXT record.
    pub node_id: String,
    /// Socket address (`ip:port`) the peer is reachable at.
    pub socket_addr: SocketAddr,
}

/// Live mDNS browser handle. Drops gracefully shut down the daemon
/// and its background task.
pub struct DiscoveryHandle {
    daemon: ServiceDaemon,
    instance_fullname: String,
    discovery_rx: Option<mpsc::UnboundedReceiver<DiscoveredPeer>>,
    browse_task: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl std::fmt::Debug for DiscoveryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryHandle")
            .field("instance_fullname", &self.instance_fullname)
            .finish_non_exhaustive()
    }
}

impl DiscoveryHandle {
    /// Take the discovered-peers receiver. Returns `None` after the
    /// first call.
    pub fn take_rx(&mut self) -> Option<mpsc::UnboundedReceiver<DiscoveredPeer>> {
        self.discovery_rx.take()
    }

    /// Service fullname this handle registered for itself (mostly
    /// useful in tests so callers can filter their own announcements
    /// when they are exercising both ends in the same process).
    #[must_use]
    pub fn instance_fullname(&self) -> &str {
        &self.instance_fullname
    }

    /// Stop publishing and browsing. Safe to call multiple times.
    pub async fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = self.daemon.unregister(&self.instance_fullname);
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
        if let Some(task) = self.browse_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
        }
    }
}

impl Drop for DiscoveryHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = self.daemon.unregister(&self.instance_fullname);
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
        if let Some(task) = self.browse_task.take() {
            task.abort();
        }
    }
}

/// Spin up mDNS publishing + browsing for the local daemon.
///
/// The local QUIC endpoint must be bound before this is called so the
/// announced port matches reality. `instance_name` should be unique
/// enough to avoid collisions on the LAN — the node ID hex prefix is
/// a good default.
pub fn start(
    instance_name: &str,
    node_id_hex: &str,
    quic_port: u16,
    advertise_ips: Vec<IpAddr>,
) -> Result<DiscoveryHandle, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;

    let host_label = format!("{instance_name}.local.");
    let mut txt = HashMap::new();
    txt.insert(NODE_ID_TXT_KEY.to_string(), node_id_hex.to_string());

    let mut service = ServiceInfo::new(
        SERVICE_TYPE,
        instance_name,
        &host_label,
        advertise_ips.as_slice(),
        quic_port,
        Some(txt),
    )?;
    // Enable address auto-discovery so the OS-assigned hostname's
    // additional IPs are advertised when the explicit list is empty.
    service = service.enable_addr_auto();
    daemon.register(service)?;

    let instance_fullname = format!("{instance_name}.{SERVICE_TYPE}");
    let receiver = daemon.browse(SERVICE_TYPE)?;
    let (tx, rx) = mpsc::unbounded_channel();
    let self_fullname = instance_fullname.clone();
    let shutdown = Arc::new(AtomicBool::new(false));

    let browse_task = {
        let shutdown = Arc::clone(&shutdown);
        tokio::task::spawn_blocking(move || {
            loop {
                match receiver.recv_timeout(Duration::from_millis(500)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let fullname = info.get_fullname().to_string();
                        if fullname == self_fullname {
                            continue;
                        }
                        let node_id = info
                            .get_property_val_str(NODE_ID_TXT_KEY)
                            .unwrap_or_default()
                            .to_string();
                        let port = info.get_port();
                        for addr in info.get_addresses() {
                            let sock = SocketAddr::new(*addr, port);
                            let peer = DiscoveredPeer {
                                node_id: node_id.clone(),
                                socket_addr: sock,
                            };
                            if tx.send(peer).is_err() {
                                return;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => {
                        if shutdown.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                }
            }
        })
    };

    Ok(DiscoveryHandle {
        daemon,
        instance_fullname,
        discovery_rx: Some(rx),
        browse_task: Some(browse_task),
        shutdown,
    })
}
