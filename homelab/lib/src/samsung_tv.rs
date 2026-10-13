//! Samsung Smart TV control.
//!
//! Provides a homelab-native interface over the generated schematic REST and
//! WebSocket clients. REST (port 8001) handles device info, logs, app
//! launching, status, close, and install. WebSocket (port 8002) handles remote
//! control key sending, app launch with deep-link, installed apps listing, and
//! Art Mode control.

use schematic_schema::samsung_smart_tv::{
    CloseAppRequest, GetAppStatusRequest, GetDeviceInfoRequest, GetServerLogsRequest,
    InstallAppRequest, LaunchApplicationByIdRequest, LaunchApplicationByNameRequest,
    SamsungSmartTv, SamsungSmartTvApi,
};
use schematic_schema::samsung_smart_tv_remote_ws::{
    ArtModeClient, ArtModeConnectionParams, RemoteControlClient, RemoteControlConnectionParams,
    SamsungSmartTvRemoteWs,
};
use schematic_schema::schematic_runtime::{CallOptions, EndpointSpec, NoDeclaredError};
use schematic_schema::ws_shared::{WsClientOptions, WsError};

use crate::rest::{self, RestError};

// Re-export response types for consumers
pub use schematic_schema::samsung_smart_tv::{
    SamsungAppStatusResponse, SamsungDeviceInfo, SamsungDeviceInfoResponse,
};
pub use schematic_schema::samsung_smart_tv_remote_ws::{
    SamsungAppLaunchActionType, SamsungAppLaunchCommand, SamsungAppLaunchData,
    SamsungAppLaunchParams, SamsungArtModeCommand, SamsungArtModeData, SamsungArtModeEmitParams,
    SamsungArtModeEvent, SamsungArtModeRequest, SamsungArtModeStatus, SamsungArtworkItem,
    SamsungConnectedClient, SamsungInstalledApp, SamsungInstalledAppsCommand,
    SamsungInstalledAppsParams, SamsungRemoteCommandAction, SamsungRemoteConnectData,
    SamsungRemoteControlCommand, SamsungRemoteControlParams, SamsungRemoteEnvelope,
    SamsungRemoteEventName, SamsungRemoteKnownAction, SamsungRemoteKnownEvent,
    SamsungRemoteKnownMethod, SamsungRemoteKnownType, SamsungRemoteMethod, SamsungRemoteType,
};

/// Default Samsung Smart TV REST API port.
pub const DEFAULT_REST_PORT: u16 = 8001;

/// Default Samsung Smart TV WebSocket API port.
pub const DEFAULT_WS_PORT: u16 = 8002;

/// Client name sent in the WebSocket connection (base64-encoded "homelab").
const CLIENT_NAME_B64: &str = "aG9tZWxhYg==";

/// Errors from the Samsung Smart TV API.
#[derive(Debug, thiserror::Error)]
pub enum SamsungTvError {
    /// The REST client could not be built, or a REST request failed or was
    /// refused.
    #[error("Samsung TV API error: {0}")]
    Api(#[from] RestError),

    /// WebSocket error
    ///
    /// Boxed: `WsError` is at least 136 bytes, and it is the largest variant
    /// here, so every `Result<_, SamsungTvError>` in this module would carry
    /// that much on its success path too. `From<WsError>` below keeps `?`
    /// working at the call sites.
    #[error("Samsung TV WebSocket error: {0}")]
    WebSocket(Box<WsError>),

    /// TV rejected the remote connection
    #[error("Samsung TV unauthorized: remote access denied")]
    Unauthorized,

    /// Connection timed out waiting for channel connect
    #[error("Samsung TV connection timed out")]
    ConnectionTimeout,

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<WsError> for SamsungTvError {
    fn from(error: WsError) -> Self {
        Self::WebSocket(Box::new(error))
    }
}

/// Samsung Smart TV client for REST and WebSocket control.
pub struct SamsungTv {
    rest_client: SamsungSmartTv,
    ws_client: SamsungSmartTvRemoteWs,
    host: String,
    rest_port: u16,
    ws_port: u16,
}

impl SamsungTv {
    /// Creates a new Samsung TV client (HTTP REST on `rest_port`).
    ///
    /// The REST client uses a 5-second connect timeout and 10-second request
    /// timeout to avoid long waits when the TV is unreachable.
    ///
    /// ## Errors
    ///
    /// Returns [`SamsungTvError::Api`] when `host` does not form a valid base
    /// URL or the HTTP client cannot be initialized.
    pub fn new(
        host: impl Into<String>,
        rest_port: u16,
        ws_port: u16,
    ) -> Result<Self, SamsungTvError> {
        Self::with_https(host, rest_port, ws_port, false)
    }

    /// Creates a new Samsung TV client with optional HTTPS support.
    ///
    /// When `use_https` is `true`, the REST base URL uses `https://` and
    /// the REST client accepts invalid (self-signed) TLS certificates,
    /// which Samsung TVs typically serve on port 8002.
    ///
    /// ## Errors
    ///
    /// Returns [`SamsungTvError::Api`] when `host` does not form a valid base
    /// URL or the HTTP client cannot be initialized.
    pub fn with_https(
        host: impl Into<String>,
        rest_port: u16,
        ws_port: u16,
        use_https: bool,
    ) -> Result<Self, SamsungTvError> {
        let host = host.into();
        let ws_base = format!("wss://{}:{}", host, ws_port);
        let rest_client = rest_builder(&host, rest_port, use_https)
            .build()
            .map_err(RestError::from)?;
        Ok(Self {
            rest_client,
            ws_client: SamsungSmartTvRemoteWs::with_base_url(ws_base),
            host,
            rest_port,
            ws_port,
        })
    }

    /// Returns the host address.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the REST API port.
    pub fn rest_port(&self) -> u16 {
        self.rest_port
    }

    /// Returns the WebSocket API port.
    pub fn ws_port(&self) -> u16 {
        self.ws_port
    }

    async fn send<R>(&self, request: R) -> Result<R::Success, SamsungTvError>
    where
        R: EndpointSpec<Api = SamsungSmartTvApi, Failure = NoDeclaredError>,
    {
        Ok(rest::into_result(
            self.rest_client
                .send(request, &CallOptions::default())
                .await,
        )?)
    }

    // ── REST: Device ─────────────────────────────────────────────────

    /// Get device information (model, firmware, network details).
    pub async fn get_device_info(&self) -> Result<SamsungDeviceInfoResponse, SamsungTvError> {
        self.send(GetDeviceInfoRequest {}).await
    }

    /// Get server logs from the TV.
    pub async fn get_server_logs(&self) -> Result<String, SamsungTvError> {
        self.send(GetServerLogsRequest {}).await
    }

    // ── REST: Apps ───────────────────────────────────────────────────

    /// Launch an application by its ID.
    pub async fn launch_app_by_id(&self, app_id: &str) -> Result<(), SamsungTvError> {
        self.send(LaunchApplicationByIdRequest::new(app_id)).await
    }

    /// Launch an application by its name.
    pub async fn launch_app_by_name(&self, app_name: &str) -> Result<(), SamsungTvError> {
        self.send(LaunchApplicationByNameRequest::new(app_name))
            .await
    }

    /// Get the running status of an application by its ID.
    pub async fn get_app_status(
        &self,
        app_id: &str,
    ) -> Result<SamsungAppStatusResponse, SamsungTvError> {
        self.send(GetAppStatusRequest::new(app_id)).await
    }

    /// Close a running application by its ID.
    pub async fn close_app(&self, app_id: &str) -> Result<(), SamsungTvError> {
        self.send(CloseAppRequest::new(app_id)).await
    }

    /// Install (or reinstall) an application by its ID.
    pub async fn install_app(&self, app_id: &str) -> Result<(), SamsungTvError> {
        self.send(InstallAppRequest::new(app_id)).await
    }

    // ── WebSocket: Remote Control ────────────────────────────────────

    /// Send a single remote key press (fire-and-forget convenience method).
    ///
    /// Opens a WebSocket connection, waits for `ms.channel.connect`,
    /// sends the key command, then closes the connection.
    pub async fn send_key(&self, key: &str) -> Result<(), SamsungTvError> {
        self.send_key_with_token(key, None).await
    }

    /// Send a single remote key press with an optional stored token.
    pub async fn send_key_with_token(
        &self,
        key: &str,
        token: Option<&str>,
    ) -> Result<(), SamsungTvError> {
        let client = self.connect_remote_inner(token).await?;
        let cmd = build_key_command(key);
        client.send(serde_json::to_value(&cmd).unwrap()).await?;
        client.close().await?;
        Ok(())
    }

    /// Open a persistent remote control WebSocket connection.
    ///
    /// Returns the raw `RemoteControlClient` for advanced usage
    /// (e.g., holding keys, sending multiple commands).
    pub async fn connect_remote(&self) -> Result<RemoteControlClient, SamsungTvError> {
        self.connect_remote_inner(None).await
    }

    async fn connect_remote_inner(
        &self,
        token: Option<&str>,
    ) -> Result<RemoteControlClient, SamsungTvError> {
        let encoded_name = CLIENT_NAME_B64.to_string();
        let params = RemoteControlConnectionParams {
            name: encoded_name,
            token: token.map(|t| t.to_string()),
        };
        let options = WsClientOptions::builder()
            .handshake_timeout(std::time::Duration::from_secs(10))
            .request_timeout(std::time::Duration::from_secs(10))
            .build();
        let client = self
            .ws_client
            .connect_remote_control(params, options)
            .await?;
        Ok(client)
    }

    // ── WebSocket: App Control ───────────────────────────────────────

    /// Launch an application via WebSocket with optional deep-link metadata.
    ///
    /// Uses `ms.channel.emit` with the `ed.apps.launch` event. This is the
    /// preferred method for deep-link launching with a `meta_tag` URL.
    pub async fn launch_app_ws(
        &self,
        app_id: &str,
        meta_tag: Option<&str>,
    ) -> Result<(), SamsungTvError> {
        let client = self.connect_remote_inner(None).await?;
        let cmd = build_app_launch_command(app_id, meta_tag);
        client.send(serde_json::to_value(&cmd)?).await?;
        client.close().await?;
        Ok(())
    }

    /// Request the list of installed applications via WebSocket.
    ///
    /// Sends an `ed.installedApp.get` emit command and returns the command
    /// (the actual response arrives asynchronously on the channel).
    pub async fn request_installed_apps(&self) -> Result<(), SamsungTvError> {
        let client = self.connect_remote_inner(None).await?;
        let cmd = build_installed_apps_command();
        client.send(serde_json::to_value(&cmd)?).await?;
        client.close().await?;
        Ok(())
    }

    // ── WebSocket: Art Mode ──────────────────────────────────────────

    /// Open a connection to the Art Mode channel.
    async fn connect_art_mode_inner(
        &self,
        token: Option<&str>,
    ) -> Result<ArtModeClient, SamsungTvError> {
        let params = ArtModeConnectionParams {
            name: CLIENT_NAME_B64.to_string(),
            token: token.map(|t| t.to_string()),
        };
        let options = WsClientOptions::builder()
            .handshake_timeout(std::time::Duration::from_secs(10))
            .request_timeout(std::time::Duration::from_secs(10))
            .build();
        let client = self.ws_client.connect_art_mode(params, options).await?;
        Ok(client)
    }

    /// Send an Art Mode command and close the connection.
    async fn send_art_mode_command(
        &self,
        request: SamsungArtModeRequest,
        extra: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Result<(), SamsungTvError> {
        let client = self.connect_art_mode_inner(None).await?;
        let cmd = build_art_mode_command(request, extra);
        client.send(serde_json::to_value(&cmd)?).await?;
        client.close().await?;
        Ok(())
    }

    /// Get the current Art Mode status (on/off).
    pub async fn get_art_mode_status(&self) -> Result<(), SamsungTvError> {
        self.send_art_mode_command(
            SamsungArtModeRequest::GetArtModeStatus,
            std::collections::BTreeMap::new(),
        )
        .await
    }

    /// Set Art Mode on or off.
    pub async fn set_art_mode(&self, on: bool) -> Result<(), SamsungTvError> {
        let mut extra = std::collections::BTreeMap::new();
        extra.insert(
            "value".to_string(),
            serde_json::Value::String(if on {
                "on".to_string()
            } else {
                "off".to_string()
            }),
        );
        self.send_art_mode_command(SamsungArtModeRequest::SetArtModeStatus, extra)
            .await
    }

    /// Get the currently displayed artwork metadata.
    pub async fn get_current_artwork(&self) -> Result<(), SamsungTvError> {
        self.send_art_mode_command(
            SamsungArtModeRequest::GetCurrentArtwork,
            std::collections::BTreeMap::new(),
        )
        .await
    }

    /// Get the list of available artwork.
    pub async fn get_artwork_list(&self) -> Result<(), SamsungTvError> {
        self.send_art_mode_command(
            SamsungArtModeRequest::GetContentList,
            std::collections::BTreeMap::new(),
        )
        .await
    }

    /// Select an artwork to display.
    pub async fn select_artwork(&self, content_id: &str) -> Result<(), SamsungTvError> {
        let mut extra = std::collections::BTreeMap::new();
        extra.insert(
            "content_id".to_string(),
            serde_json::Value::String(content_id.to_string()),
        );
        self.send_art_mode_command(SamsungArtModeRequest::SelectImage, extra)
            .await
    }

    /// Get the Art Mode display brightness.
    pub async fn get_art_brightness(&self) -> Result<(), SamsungTvError> {
        self.send_art_mode_command(
            SamsungArtModeRequest::GetBrightness,
            std::collections::BTreeMap::new(),
        )
        .await
    }

    /// Set the Art Mode display brightness (0-100).
    pub async fn set_art_brightness(&self, level: u8) -> Result<(), SamsungTvError> {
        let mut extra = std::collections::BTreeMap::new();
        extra.insert(
            "value".to_string(),
            serde_json::Value::String(level.to_string()),
        );
        self.send_art_mode_command(SamsungArtModeRequest::SetBrightness, extra)
            .await
    }
}

// ── Command Builders ─────────────────────────────────────────────────

/// Builds a remote control key command for the Samsung TV.
/// The unvalidated REST client for `{scheme}://{host}:{rest_port}`, with the
/// device timeouts.
///
/// Invalid certificates are accepted only over HTTPS: the TV's certificate is
/// self-signed, and a plain-HTTP client has no certificate to check.
fn rest_builder(host: &str, rest_port: u16, use_https: bool) -> SamsungSmartTv {
    let scheme = if use_https { "https" } else { "http" };
    let builder = SamsungSmartTv::new()
        .base_url(format!("{scheme}://{host}:{rest_port}"))
        .connect_timeout(rest::CONNECT_TIMEOUT)
        .timeout(rest::REQUEST_TIMEOUT);
    if use_https {
        builder.danger_accept_invalid_certificates()
    } else {
        builder
    }
}

fn build_key_command(key: &str) -> SamsungRemoteControlCommand {
    SamsungRemoteControlCommand {
        method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsRemoteControl),
        params: SamsungRemoteControlParams {
            cmd: SamsungRemoteCommandAction::Known(SamsungRemoteKnownAction::Click),
            data_of_cmd: key.to_string(),
            option: "false".to_string(),
            type_of_remote: SamsungRemoteType::Known(SamsungRemoteKnownType::SendRemoteKey),
        },
    }
}

/// Builds an app launch command for `ms.channel.emit`.
fn build_app_launch_command(app_id: &str, meta_tag: Option<&str>) -> SamsungAppLaunchCommand {
    let action_type = if meta_tag.is_some() {
        SamsungAppLaunchActionType::DeepLink
    } else {
        SamsungAppLaunchActionType::NativeLaunch
    };
    SamsungAppLaunchCommand {
        method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
        params: SamsungAppLaunchParams {
            event: "ed.apps.launch".to_string(),
            to: "host".to_string(),
            data: SamsungAppLaunchData {
                action_type,
                app_id: app_id.to_string(),
                meta_tag: meta_tag.map(|s| s.to_string()),
            },
        },
    }
}

/// Builds an installed apps request command.
fn build_installed_apps_command() -> SamsungInstalledAppsCommand {
    SamsungInstalledAppsCommand {
        method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
        params: SamsungInstalledAppsParams {
            event: "ed.installedApp.get".to_string(),
            to: "host".to_string(),
        },
    }
}

/// Builds an Art Mode command with double-encoded data.
fn build_art_mode_command(
    request: SamsungArtModeRequest,
    extra: std::collections::BTreeMap<String, serde_json::Value>,
) -> SamsungArtModeCommand {
    let inner = SamsungArtModeData {
        request,
        id: uuid_v4_simple(),
        extra,
    };
    let data_str = serde_json::to_string(&inner).expect("art mode data serialization");
    SamsungArtModeCommand {
        method: SamsungRemoteMethod::Known(SamsungRemoteKnownMethod::MsChannelEmit),
        params: SamsungArtModeEmitParams {
            event: "art_app_request".to_string(),
            to: "host".to_string(),
            data: data_str,
        },
    }
}

/// Simple UUID v4 generator using random bytes from the OS.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}-0000-4000-8000-{:012x}", nanos, nanos as u64)
}

#[cfg(test)]
mod rest_tests {
    use schematic_schema::schematic_runtime::{BuildError, SharedTransport};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::{SamsungTv, SamsungTvError, rest_builder};
    use crate::rest::RestError;

    const WS_PORT: u16 = 8002;

    fn tv_for(server: &MockServer) -> SamsungTv {
        SamsungTv::new("127.0.0.1", server.address().port(), WS_PORT).expect("client builds")
    }

    /// The first connection option the builder set, revealed by attaching a
    /// shared transport, which refuses connection options by name.
    fn first_connection_option(use_https: bool) -> &'static str {
        let transport = SharedTransport::builder().build().expect("transport");
        match rest_builder("192.0.2.20", 8001, use_https)
            .transport(&transport)
            .build()
        {
            Err(BuildError::TransportOptionConflict { option }) => option,
            other => panic!("expected an option conflict, got {other:?}"),
        }
    }

    #[test]
    fn the_rest_base_url_scheme_follows_use_https() {
        let plain = SamsungTv::with_https("192.0.2.20", 8001, WS_PORT, false).expect("http");
        let secure = SamsungTv::with_https("192.0.2.20", 8002, WS_PORT, true).expect("https");

        assert!(format!("{:?}", plain.rest_client).contains("http://192.0.2.20:8001"));
        assert!(format!("{:?}", secure.rest_client).contains("https://192.0.2.20:8002"));
        assert!(format!("{:?}", plain.rest_client).contains("timeout: 10s"));
    }

    #[test]
    fn invalid_certificates_are_accepted_only_over_https() {
        assert_eq!(
            first_connection_option(true),
            "danger_accept_invalid_certificates"
        );
        // Over HTTP the connect timeout is the first option set, so the
        // invalid-certificate option, checked before it, is unset.
        assert_eq!(first_connection_option(false), "connect_timeout");
    }

    #[tokio::test]
    async fn an_https_client_does_not_speak_plain_http() {
        let server = MockServer::start().await;
        let tv = SamsungTv::with_https("127.0.0.1", server.address().port(), WS_PORT, true)
            .expect("client builds");

        let error = tv
            .get_device_info()
            .await
            .expect_err("TLS to a plain server fails");

        assert!(
            matches!(error, SamsungTvError::Api(RestError::Transport(_))),
            "{error}"
        );
        assert!(
            server
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn get_server_logs_returns_the_text_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/logs/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("line one\nline two\n"))
            .expect(1)
            .mount(&server)
            .await;

        let logs = tv_for(&server).get_server_logs().await.expect("logs");

        assert_eq!(logs, "line one\nline two\n");
    }

    #[tokio::test]
    async fn empty_endpoints_succeed_on_a_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/api/v2/applications/111299001912"))
            .respond_with(ResponseTemplate::new(200).set_body_string("true"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v2/applications/111299001912"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let tv = tv_for(&server);

        tv.launch_app_by_id("111299001912")
            .await
            .expect("launch by id");
        tv.launch_app_by_name("YouTube")
            .await
            .expect("launch by name");
        tv.close_app("111299001912").await.expect("close");
        tv.install_app("111299001912").await.expect("install");
    }

    #[tokio::test]
    async fn empty_endpoints_fail_on_a_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(ResponseTemplate::new(404).set_body_string("no such app"))
            .mount(&server)
            .await;
        let tv = tv_for(&server);

        for result in [
            tv.launch_app_by_id("missing").await,
            tv.launch_app_by_name("missing").await,
            tv.close_app("missing").await,
            tv.install_app("missing").await,
        ] {
            match result {
                Err(SamsungTvError::Api(RestError::Status { status, body })) => {
                    assert_eq!(status, 404);
                    assert_eq!(body, "no such app");
                }
                other => panic!("expected a 404 status error, got {other:?}"),
            }
        }
    }
}
