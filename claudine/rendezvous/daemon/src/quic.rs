//! Phase 4 QUIC transport: a thin wrapper around [`quinn::Endpoint`].
//!
//! The endpoint is bidirectional — the same UDP socket accepts inbound
//! connections from other daemons and hosts outbound connections
//! initiated by [`QuicEndpoint::connect`]. TLS is wired up with a
//! self-signed certificate per process; the client side is
//! intentionally permissive for this POC (any presented cert is
//! accepted, since peer authenticity is enforced separately by the
//! signed-envelope layer once data is exchanged).
//!
//! The endpoint owns a background task that drains inbound
//! connections and forwards their handshake result onto a channel so
//! callers (e.g. the peer registry) can react asynchronously.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::crypto::rustls::QuicClientConfig;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Negotiated ALPN protocol identifier. Phase 4 only needs a unique
/// label; later phases can use it to multiplex protocol versions.
pub const ALPN_PROTOCOL: &[u8] = b"agentgrid/0.1";

/// Errors surfaced while constructing or driving the QUIC endpoint.
#[derive(Debug, thiserror::Error)]
pub enum QuicError {
    /// Failed to generate the self-signed certificate the QUIC server
    /// uses for its TLS identity.
    #[error("failed to generate self-signed certificate: {0}")]
    CertGenerate(#[from] rcgen::Error),

    /// rustls rejected the assembled configuration.
    #[error("rustls config error: {0}")]
    Rustls(#[from] rustls::Error),

    /// Building the quinn server config from the rustls bundle failed
    /// (for example, when rustls does not expose a TLS 1.3 group).
    #[error("quinn server config error: {0}")]
    QuinnNoInitialCipherSuite(#[from] quinn::crypto::rustls::NoInitialCipherSuite),

    /// Binding the underlying UDP socket failed.
    #[error("failed to bind QUIC endpoint at {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    /// Outbound QUIC `connect` failed before the handshake completed.
    #[error("quinn connect error: {0}")]
    Connect(#[from] quinn::ConnectError),

    /// The QUIC handshake itself failed.
    #[error("quinn connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),
}

/// Outcome forwarded onto the [`QuicEndpoint::accept_events`] channel
/// for each inbound connection.
#[derive(Debug)]
pub struct InboundConnection {
    /// Remote socket address as observed by the local endpoint.
    pub remote_addr: SocketAddr,
    /// Established quinn connection; the caller is responsible for
    /// driving streams off of it (Phase 4 simply closes it).
    pub connection: quinn::Connection,
}

/// Handle to a running QUIC endpoint.
pub struct QuicEndpoint {
    endpoint: Endpoint,
    local_addr: SocketAddr,
    accept_task: Option<JoinHandle<()>>,
    accept_rx: Option<mpsc::UnboundedReceiver<InboundConnection>>,
}

impl std::fmt::Debug for QuicEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicEndpoint")
            .field("local_addr", &self.local_addr)
            .finish_non_exhaustive()
    }
}

impl QuicEndpoint {
    /// Bind a new QUIC endpoint at `bind_addr`. A `0` port leaves the
    /// OS to pick one; the resolved address is available via
    /// [`QuicEndpoint::local_addr`].
    pub fn bind(bind_addr: SocketAddr) -> Result<Self, QuicError> {
        install_default_crypto_provider();

        let (server_config, client_config) = build_quinn_configs()?;
        let mut endpoint =
            Endpoint::server(server_config, bind_addr).map_err(|source| QuicError::Bind {
                addr: bind_addr,
                source,
            })?;
        endpoint.set_default_client_config(client_config);
        let local_addr = endpoint.local_addr().map_err(|source| QuicError::Bind {
            addr: bind_addr,
            source,
        })?;

        let (tx, rx) = mpsc::unbounded_channel();
        let endpoint_clone = endpoint.clone();
        let accept_task = tokio::spawn(async move {
            run_accept_loop(endpoint_clone, tx).await;
        });

        Ok(Self {
            endpoint,
            local_addr,
            accept_task: Some(accept_task),
            accept_rx: Some(rx),
        })
    }

    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Clone the underlying `quinn::Endpoint` so other tasks can use
    /// it for outbound `connect` calls.
    #[must_use]
    pub fn endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }

    /// Take the inbound-connection receiver. Returns `None` if the
    /// receiver has already been consumed (it can only be taken once,
    /// as it is owned by a single consumer).
    pub fn take_accept_rx(&mut self) -> Option<mpsc::UnboundedReceiver<InboundConnection>> {
        self.accept_rx.take()
    }

    /// Open an outbound connection to `addr`. The TLS handshake uses
    /// the permissive client config and accepts the remote certificate
    /// regardless of its CA chain.
    pub async fn connect(&self, addr: SocketAddr) -> Result<quinn::Connection, QuicError> {
        let connecting = self.endpoint.connect(addr, "rendezvous")?;
        let connection = connecting.await?;
        Ok(connection)
    }

    /// Close the endpoint and await the inbound-accept task. Safe to
    /// call multiple times.
    pub async fn shutdown(&mut self) {
        self.endpoint.close(0u32.into(), b"shutdown");
        if let Some(task) = self.accept_task.take() {
            // Wait at most a second so a misbehaving accept loop does
            // not block daemon shutdown.
            let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
        }
    }
}

impl Drop for QuicEndpoint {
    fn drop(&mut self) {
        self.endpoint.close(0u32.into(), b"drop");
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }
    }
}

async fn run_accept_loop(endpoint: Endpoint, tx: mpsc::UnboundedSender<InboundConnection>) {
    while let Some(incoming) = endpoint.accept().await {
        let tx = tx.clone();
        tokio::spawn(async move {
            let remote_addr = incoming.remote_address();
            match incoming.await {
                Ok(connection) => {
                    let event = InboundConnection {
                        remote_addr,
                        connection,
                    };
                    if tx.send(event).is_err() {
                        tracing::debug!(
                            target: "rendezvous_daemon::quic",
                            %remote_addr,
                            "inbound connection accepted but receiver dropped",
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::quic",
                        %remote_addr,
                        %error,
                        "inbound QUIC handshake failed",
                    );
                }
            }
        });
    }
}

fn build_quinn_configs() -> Result<(ServerConfig, ClientConfig), QuicError> {
    let cert_key = rcgen::generate_simple_self_signed(vec!["rendezvous".to_string()])?;
    let cert_der = CertificateDer::from(cert_key.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert_key.key_pair.serialize_der()));

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der.clone_key())?;
    server_crypto.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];

    let quinn_server_crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?;
    let mut server_config = ServerConfig::with_crypto(Arc::new(quinn_server_crypto));
    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().expect("valid")));
    server_config.transport_config(Arc::new(transport));

    let mut client_crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert))
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![ALPN_PROTOCOL.to_vec()];

    let quinn_client_crypto = QuicClientConfig::try_from(client_crypto)?;
    let client_config = ClientConfig::new(Arc::new(quinn_client_crypto));

    Ok((server_config, client_config))
}

fn install_default_crypto_provider() {
    use std::sync::Once;
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // It is fine if another part of the process already installed
        // a provider — we just need at least one available.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// rustls server-cert verifier that accepts any presented certificate.
///
/// ## Why this is permissive
///
/// Every daemon generates a fresh self-signed certificate on startup.
/// There is no CA or pinned certificate distribution in Phase 4, so the
/// TLS layer cannot authenticate the peer. Instead, peer identity is
/// established by the [`crate::session_log`] signed-envelope layer:
/// each sync delta/snapshot is signed with the peer's long-term Ed25519
/// identity key, and the receiver verifies the signature and checks that
/// the sender owns the document namespace before accepting the payload.
///
/// ## Future hardening
///
/// Replace this verifier before exposing rendezvous to untrusted
/// networks. Options include pinning the peer's certificate or public
/// key after first use (TOFU), requiring invitations to carry a
/// certificate fingerprint, or running a small internal CA for the mesh.
#[derive(Debug)]
struct AcceptAnyServerCert;

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    #[tokio::test]
    async fn endpoint_round_trip() {
        let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let mut server = QuicEndpoint::bind(bind).expect("bind server");
        let mut accept_rx = server.take_accept_rx().expect("rx");
        let client = QuicEndpoint::bind(bind).expect("bind client");

        let server_addr = server.local_addr();
        let connect_task = tokio::spawn(async move { client.connect(server_addr).await });

        let inbound = tokio::time::timeout(Duration::from_secs(5), accept_rx.recv())
            .await
            .expect("accept timeout")
            .expect("inbound channel closed");
        let outbound = connect_task.await.expect("join").expect("connect");

        assert!(inbound.connection.remote_address().port() > 0);
        assert!(outbound.remote_address().port() > 0);
        drop(outbound);
        drop(inbound);
        server.shutdown().await;
    }
}
