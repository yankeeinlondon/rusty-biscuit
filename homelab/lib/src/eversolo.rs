//! Eversolo DMP-A8 music streamer control.
//!
//! Provides a homelab-native interface over the generated schematic client.

use schematic_schema::eversolo::{
    DeviceGetModelRequest, Eversolo as EversoloClient, MusicGetInputOutputListRequest,
    MusicGetStateRequest, MusicPlayLastRequest, MusicPlayNextRequest, MusicPlayOrPauseRequest,
    MusicSeekToRequest, MusicSetInputRequest, MusicSetMuteRequest, MusicSetOutputRequest,
    MusicSetVolumeRequest, PowerGetOptionsRequest, PowerSetOptionRequest,
    RemoteInputTextRequest, RemoteSendKeyRequest, SystemChangeVuDisplayRequest,
    SystemGetKnobBrightnessRequest, SystemGetScreenBrightnessRequest,
    SystemGetSpectrumModeListRequest, SystemGetVuModeListRequest,
    SystemSetKnobBrightnessRequest, SystemSetScreenBrightnessRequest,
    SystemSetSpectrumModeRequest, SystemSetVuModeRequest,
};

// Re-export response types for consumers
pub use schematic_schema::eversolo::{
    BrightnessResponse, DisplayModeListResponse, GetModelResponse, GetStateResponse,
    InputOutputListResponse, PowerOptionsResponse, StatusResponse,
};

/// Default Eversolo API port.
pub const DEFAULT_PORT: u16 = 9529;

/// Errors from the Eversolo API.
#[derive(Debug, thiserror::Error)]
pub enum EversoloError {
    /// Eversolo API error
    #[error("Eversolo API error: {0}")]
    Api(#[from] schematic_schema::shared::SchematicError),
}

/// Eversolo DMP-A8 music streamer client.
pub struct Eversolo {
    client: EversoloClient,
    host: String,
    port: u16,
}

impl Eversolo {
    /// Creates a new Eversolo client.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        let host = host.into();
        let base_url = format!("http://{}:{}", host, port);
        Self {
            client: EversoloClient::with_base_url(base_url),
            host,
            port,
        }
    }

    /// Returns the host address.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port number.
    pub fn port(&self) -> u16 {
        self.port
    }

    // ── Device ──────────────────────────────────────────────────────

    /// Get device model, firmware, and network information.
    pub async fn get_model(&self) -> Result<GetModelResponse, EversoloError> {
        Ok(self
            .client
            .request::<GetModelResponse>(DeviceGetModelRequest {})
            .await?)
    }

    // ── Music Control ───────────────────────────────────────────────

    /// Get current playback state, track info, and volume.
    pub async fn get_state(&self) -> Result<GetStateResponse, EversoloError> {
        Ok(self
            .client
            .request::<GetStateResponse>(MusicGetStateRequest {})
            .await?)
    }

    /// Toggle play/pause on the current track.
    pub async fn play_or_pause(&self) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(MusicPlayOrPauseRequest {})
            .await?)
    }

    /// Skip to the next track.
    pub async fn play_next(&self) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(MusicPlayNextRequest {})
            .await?)
    }

    /// Go back to the previous track.
    pub async fn play_previous(&self) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(MusicPlayLastRequest {})
            .await?)
    }

    /// Seek to a position in the current track.
    pub async fn seek_to(&self, time_ms: i64) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(MusicSeekToRequest::new().with_time(time_ms))
            .await?)
    }

    // ── Audio ───────────────────────────────────────────────────────

    /// List available audio inputs and outputs.
    pub async fn get_inputs_outputs(
        &self,
    ) -> Result<InputOutputListResponse, EversoloError> {
        Ok(self
            .client
            .request::<InputOutputListResponse>(MusicGetInputOutputListRequest {})
            .await?)
    }

    /// Set the active audio input.
    pub async fn set_input(
        &self,
        tag: &str,
    ) -> Result<InputOutputListResponse, EversoloError> {
        Ok(self
            .client
            .request::<InputOutputListResponse>(
                MusicSetInputRequest::new().with_tag(tag.to_string()),
            )
            .await?)
    }

    /// Set the active audio output.
    pub async fn set_output(
        &self,
        tag: &str,
    ) -> Result<InputOutputListResponse, EversoloError> {
        Ok(self
            .client
            .request::<InputOutputListResponse>(
                MusicSetOutputRequest::new().with_tag(tag.to_string()),
            )
            .await?)
    }

    /// Set the absolute volume level.
    pub async fn set_volume(&self, level: i64) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(MusicSetVolumeRequest::new().with_volume(level))
            .await?)
    }

    /// Set or clear mute state.
    #[allow(non_snake_case)]
    pub async fn set_mute(&self, muted: bool) -> Result<StatusResponse, EversoloError> {
        let value = if muted { 1 } else { 0 };
        Ok(self
            .client
            .request::<StatusResponse>(MusicSetMuteRequest::new().with_isMute(value))
            .await?)
    }

    // ── Power ───────────────────────────────────────────────────────

    /// Get available power options (shutdown, reboot, etc.).
    pub async fn get_power_options(&self) -> Result<PowerOptionsResponse, EversoloError> {
        Ok(self
            .client
            .request::<PowerOptionsResponse>(PowerGetOptionsRequest {})
            .await?)
    }

    /// Execute a power action by tag.
    pub async fn set_power_option(
        &self,
        tag: &str,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                PowerSetOptionRequest::new().with_tag(tag.to_string()),
            )
            .await?)
    }

    // ── Remote ──────────────────────────────────────────────────────

    /// Send a remote control key command.
    pub async fn send_key(&self, key: &str) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                RemoteSendKeyRequest::new().with_key(key.to_string()),
            )
            .await?)
    }

    /// Send text input to the device.
    pub async fn input_text(&self, text: &str) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                RemoteInputTextRequest::new().with_text(text.to_string()),
            )
            .await?)
    }

    // ── Display ─────────────────────────────────────────────────────

    /// Get current screen brightness level.
    pub async fn get_screen_brightness(&self) -> Result<BrightnessResponse, EversoloError> {
        Ok(self
            .client
            .request::<BrightnessResponse>(SystemGetScreenBrightnessRequest {})
            .await?)
    }

    /// Set screen brightness level.
    pub async fn set_screen_brightness(
        &self,
        index: i64,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                SystemSetScreenBrightnessRequest::new().with_index(index),
            )
            .await?)
    }

    /// Get current knob LED brightness level.
    pub async fn get_knob_brightness(&self) -> Result<BrightnessResponse, EversoloError> {
        Ok(self
            .client
            .request::<BrightnessResponse>(SystemGetKnobBrightnessRequest {})
            .await?)
    }

    /// Set knob LED brightness level.
    pub async fn set_knob_brightness(
        &self,
        index: i64,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                SystemSetKnobBrightnessRequest::new().with_index(index),
            )
            .await?)
    }

    /// List available VU meter display modes.
    pub async fn get_vu_modes(&self) -> Result<DisplayModeListResponse, EversoloError> {
        Ok(self
            .client
            .request::<DisplayModeListResponse>(SystemGetVuModeListRequest {})
            .await?)
    }

    /// Set the VU meter display mode.
    pub async fn set_vu_mode(
        &self,
        index: i64,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(SystemSetVuModeRequest::new().with_index(index))
            .await?)
    }

    /// List available spectrum display modes.
    pub async fn get_spectrum_modes(
        &self,
    ) -> Result<DisplayModeListResponse, EversoloError> {
        Ok(self
            .client
            .request::<DisplayModeListResponse>(SystemGetSpectrumModeListRequest {})
            .await?)
    }

    /// Set the spectrum display mode.
    #[allow(non_snake_case)]
    pub async fn set_spectrum_mode(
        &self,
        index: i64,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                SystemSetSpectrumModeRequest::new().with_index(index),
            )
            .await?)
    }

    /// Toggle VU display open/close.
    #[allow(non_snake_case)]
    pub async fn change_vu_display(
        &self,
        open_type: i64,
    ) -> Result<StatusResponse, EversoloError> {
        Ok(self
            .client
            .request::<StatusResponse>(
                SystemChangeVuDisplayRequest::new().with_openType(open_type),
            )
            .await?)
    }
}
