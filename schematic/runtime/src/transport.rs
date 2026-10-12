//! Connection state: the HTTP client every send goes through.
//!
//! The runtime builds every HTTP client itself, always with the library's
//! redirect following disabled, and never accepts one from a caller. A
//! [`SharedTransport`] lets several clients share one connection pool; the
//! settings that can only exist once per pool (proxy, TLS roots, invalid
//! certificate acceptance, connect timeout) then belong to the transport, and
//! a client attached to it may not set them again.

use std::error::Error as _;
use std::time::Duration;

use crate::error::BuildError;
use crate::response::FailureCategory;

/// A connection pool shared by several clients.
///
/// Cloning is cheap and shares the pool. Two clients attached to the same
/// transport reuse each other's idle connections to the same origin.
#[derive(Debug, Clone)]
pub struct SharedTransport {
    pub(crate) http: reqwest::Client,
}

impl SharedTransport {
    /// A builder for a shared transport.
    pub fn builder() -> SharedTransportBuilder {
        SharedTransportBuilder::default()
    }
}

/// Builds a [`SharedTransport`].
#[derive(Debug, Clone, Default)]
pub struct SharedTransportBuilder {
    settings: ConnectionSettings,
}

impl SharedTransportBuilder {
    /// Sends every request through the proxy at `url`.
    #[must_use]
    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.settings.proxy = Some(url.into());
        self
    }

    /// Trusts `certificate` (PEM or DER) as a root, in addition to the
    /// platform's roots.
    #[must_use]
    pub fn add_root_certificate(mut self, certificate: impl Into<Vec<u8>>) -> Self {
        self.settings.roots.push(certificate.into());
        self
    }

    /// Accepts any server certificate, including self-signed and expired
    /// ones. Only for devices that cannot present a valid certificate.
    #[must_use]
    pub fn danger_accept_invalid_certificates(mut self) -> Self {
        self.settings.accept_invalid_certificates = true;
        self
    }

    /// Bounds how long establishing a connection may take.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.settings.connect_timeout = Some(timeout);
        self
    }

    /// Builds the transport.
    ///
    /// ## Errors
    ///
    /// Returns [`BuildError`] when the proxy URL or a certificate is invalid,
    /// or the HTTP client cannot be initialized.
    pub fn build(self) -> Result<SharedTransport, BuildError> {
        Ok(SharedTransport {
            http: self.settings.build()?,
        })
    }
}

/// The settings that live on the HTTP client itself rather than on a request.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConnectionSettings {
    pub(crate) proxy: Option<String>,
    pub(crate) roots: Vec<Vec<u8>>,
    pub(crate) accept_invalid_certificates: bool,
    pub(crate) connect_timeout: Option<Duration>,
}

impl ConnectionSettings {
    /// The first option set here, by its builder name; used to refuse
    /// settings that an attached shared transport owns.
    pub(crate) fn first_set_option(&self) -> Option<&'static str> {
        if self.proxy.is_some() {
            Some("proxy")
        } else if !self.roots.is_empty() {
            Some("add_root_certificate")
        } else if self.accept_invalid_certificates {
            Some("danger_accept_invalid_certificates")
        } else if self.connect_timeout.is_some() {
            Some("connect_timeout")
        } else {
            None
        }
    }

    /// Builds an HTTP client that never follows redirects.
    pub(crate) fn build(&self) -> Result<reqwest::Client, BuildError> {
        let mut builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
        if let Some(proxy) = &self.proxy {
            let proxy = reqwest::Proxy::all(proxy.as_str()).map_err(|_| BuildError::InvalidProxy)?;
            builder = builder.proxy(proxy);
        }
        for root in &self.roots {
            let certificate = if root.starts_with(b"-----BEGIN") {
                reqwest::Certificate::from_pem(root)
            } else {
                reqwest::Certificate::from_der(root)
            }
            .map_err(|_| BuildError::InvalidCertificate)?;
            builder = builder.add_root_certificate(certificate);
        }
        if self.accept_invalid_certificates {
            builder = builder.tls_danger_accept_invalid_certs(true);
        }
        if let Some(timeout) = self.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }
        builder.build().map_err(|_| {
            // The TLS backend parses root certificates only here, so a build
            // that succeeds without them blames a certificate.
            let without_roots = Self {
                roots: Vec::new(),
                ..self.clone()
            };
            if !self.roots.is_empty() && without_roots.build().is_ok() {
                BuildError::InvalidCertificate
            } else {
                BuildError::Transport
            }
        })
    }
}

/// The sanitized category of a library error raised before a response head.
///
/// Only the error's kind and source types are inspected; its text, which can
/// contain the unredacted URL, is never read.
pub(crate) fn classify(error: &reqwest::Error) -> FailureCategory {
    if chain_has_tls_error(error.source()) {
        FailureCategory::Tls
    } else if error.is_timeout() {
        FailureCategory::Timeout
    } else if error.is_connect() {
        FailureCategory::Connect
    } else {
        FailureCategory::Other
    }
}

/// Whether a `rustls::Error` occurs anywhere below `source`.
///
/// The handshake error arrives wrapped in `io::Error`s, and
/// `io::Error::source` skips the error it wraps, so each `io::Error` is also
/// opened through `get_ref`.
fn chain_has_tls_error(mut source: Option<&(dyn std::error::Error + 'static)>) -> bool {
    while let Some(current) = source {
        if current.downcast_ref::<rustls::Error>().is_some() {
            return true;
        }
        if let Some(wrapped) = current.downcast_ref::<std::io::Error>().and_then(std::io::Error::get_ref)
            && chain_has_tls_error(Some(wrapped))
        {
            return true;
        }
        source = current.source();
    }
    false
}
