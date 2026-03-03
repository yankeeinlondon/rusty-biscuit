//! Samsung Smart TV control.
//!
//! Provides a homelab-native interface over the generated schematic REST and
//! WebSocket clients. REST (port 8001) handles device info, logs, and app
//! launching. WebSocket (port 8002) handles remote control key sending.

use schematic_schema::samsung_smart_tv::{
    GetDeviceInfoRequest, GetServerLogsRequest, LaunchApplicationByIdRequest,
    LaunchApplicationByNameRequest, SamsungSmartTv as RestClient,
};
use schematic_schema::samsung_smart_tv_remote_ws::{
    RemoteControlClient, RemoteControlConnectionParams, SamsungSmartTvRemoteWs,
};
use schematic_schema::ws_shared::{WsClientOptions, WsError};

// Re-export response types for consumers
pub use schematic_schema::samsung_smart_tv_remote_ws::{
    SamsungRemoteCommandAction, SamsungRemoteControlCommand, SamsungRemoteControlParams,
    SamsungRemoteEnvelope, SamsungRemoteEventName, SamsungRemoteKnownAction,
    SamsungRemoteKnownEvent, SamsungRemoteKnownMethod, SamsungRemoteKnownType,
    SamsungRemoteMethod, SamsungRemoteType,
};

pub use schematic_schema::samsung_smart_tv::{SamsungDeviceInfo, SamsungDeviceInfoResponse};

/// Default Samsung Smart TV REST API port.
pub const DEFAULT_REST_PORT: u16 = 8001;

/// Default Samsung Smart TV WebSocket API port.
pub const DEFAULT_WS_PORT: u16 = 8002;

/// Client name sent in the WebSocket connection (base64-encoded "homelab").
const CLIENT_NAME_B64: &str = "aG9tZWxhYg==";

/// Errors from the Samsung Smart TV API.
#[derive(Debug, thiserror::Error)]
pub enum SamsungTvError {
    /// REST API error
    #[error("Samsung TV API error: {0}")]
    Api(#[from] schematic_schema::shared::SchematicError),

    /// WebSocket error
    #[error("Samsung TV WebSocket error: {0}")]
    WebSocket(#[from] WsError),

    /// TV rejected the remote connection
    #[error("Samsung TV unauthorized: remote access denied")]
    Unauthorized,

    /// Connection timed out waiting for channel connect
    #[error("Samsung TV connection timed out")]
    ConnectionTimeout,
}

/// Samsung Smart TV client for REST and WebSocket control.
pub struct SamsungTv {
    rest_client: RestClient,
    ws_client: SamsungSmartTvRemoteWs,
    host: String,
    rest_port: u16,
    ws_port: u16,
}

impl SamsungTv {
    /// Creates a new Samsung TV client.
    pub fn new(host: impl Into<String>, rest_port: u16, ws_port: u16) -> Self {
        let host = host.into();
        let rest_base = format!("http://{}:{}", host, rest_port);
        let ws_base = format!("wss://{}:{}", host, ws_port);
        Self {
            rest_client: RestClient::with_base_url(rest_base),
            ws_client: SamsungSmartTvRemoteWs::with_base_url(ws_base),
            host,
            rest_port,
            ws_port,
        }
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

    // ── REST: Device ─────────────────────────────────────────────────

    /// Get device information (model, firmware, network details).
    pub async fn get_device_info(&self) -> Result<SamsungDeviceInfoResponse, SamsungTvError> {
        Ok(self
            .rest_client
            .request::<SamsungDeviceInfoResponse>(GetDeviceInfoRequest {})
            .await?)
    }

    /// Get server logs from the TV.
    pub async fn get_server_logs(&self) -> Result<String, SamsungTvError> {
        Ok(self
            .rest_client
            .request_text(GetServerLogsRequest {})
            .await?)
    }

    // ── REST: Apps ───────────────────────────────────────────────────

    /// Launch an application by its ID.
    pub async fn launch_app_by_id(&self, app_id: &str) -> Result<(), SamsungTvError> {
        self.rest_client
            .request_empty(LaunchApplicationByIdRequest::new(app_id.to_string()))
            .await?;
        Ok(())
    }

    /// Launch an application by its name.
    pub async fn launch_app_by_name(&self, app_name: &str) -> Result<(), SamsungTvError> {
        self.rest_client
            .request_empty(LaunchApplicationByNameRequest::new(app_name.to_string()))
            .await?;
        Ok(())
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
}

/// Builds a remote control key command for the Samsung TV.
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
