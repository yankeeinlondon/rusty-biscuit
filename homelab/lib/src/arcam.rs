use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use thiserror::Error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::network::Host;

/// Errors that can occur when communicating with the Arcam PA240/PA410/PA720.
#[derive(Debug, Error)]
pub enum ArcamError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid response: expected at least 7 bytes (St Zn Cc Ac Dl Data Et), got {0} bytes: {1}")]
    ResponseTooShort(usize, String),

    #[error("Invalid response: missing start byte 0x21, got: {0}")]
    BadStartByte(String),

    #[error("Zone invalid (answer code 0x82, raw: {0})")]
    ZoneInvalid(String),

    #[error("Command not recognised by amplifier (answer code 0x83, raw: {0})")]
    CommandNotRecognised(String),

    #[error("Parameter not recognised by amplifier (answer code 0x84, raw: {0})")]
    ParameterNotRecognised(String),

    #[error("Invalid data length (answer code 0x86, raw: {0})")]
    InvalidDataLength(String),

    #[error("Unknown answer code 0x{answer_code:02X} (raw: {raw})")]
    UnknownAnswerCode { answer_code: u8, raw: String },

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// A parsed Arcam protocol response frame.
///
/// Frame format: `St(0x21) Zn Cc Ac Dl Data... Et(0x0D)`
#[derive(Debug)]
pub struct ArcamResponse {
    /// Zone number (typically 0x01).
    pub zone: u8,
    /// Command code echoed back.
    pub command: u8,
    /// Answer code: 0x00 = status update (success).
    pub answer_code: u8,
    /// Payload data bytes.
    pub data: Vec<u8>,
    /// Full raw response bytes for diagnostics.
    pub raw: Vec<u8>,
}

/// Complete system status obtained via System Status (0x5D) command.
///
/// This command triggers the amp to send individual status responses for all parameters.
#[derive(Debug, Default)]
pub struct ArcamSystemStatus {
    /// Power state: true = on, false = standby
    pub power: bool,
    /// Mute state: true = muted, false = unmuted
    pub muted: bool,
    /// Software version string (e.g., "1.2")
    pub software_version: Option<String>,
    /// Friendly name (PA720/PA240 only)
    pub friendly_name: Option<String>,
    /// IP address as string (PA720/PA240 only)
    pub ip_address: Option<String>,
    /// Seconds until auto-standby (0-14400)
    pub timeout_counter: Option<u16>,
    /// Auto shutdown setting (0=Disabled, 1=20min, 2=30min, 3=1hr, 4=2hr, 5=4hr)
    pub auto_shutdown: Option<u8>,
    /// Input signal detected
    pub input_detect: Option<bool>,
    /// System model name (e.g., "PA240")
    pub model: Option<String>,
    /// Amplifier mode raw byte (PA240 only): 1=Stereo, 2=Bridged, 3=DualMono
    pub amplifier_mode: Option<u8>,
}

/// Format raw bytes as a hex string for diagnostics (e.g. `"21 01 00 00 01 01 0D"`).
fn hex_dump(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Send a raw command frame and read the response.
///
/// `cmd_bytes` should be the complete PA240/PA410/PA720 command frame.
/// Returns the raw response bytes on success.
async fn send_raw(ip_addr: &str, cmd_bytes: &[u8]) -> Result<Vec<u8>, ArcamError> {
    // Arcam PA240/PA410/PA720 IP control default port is 50000
    let addr = format!("{}:50000", ip_addr);
    let mut sock = TcpStream::connect(addr).await?;
    sock.write_all(cmd_bytes).await?;

    // Read up to some reasonable max (answers are short)
    let mut buf = vec![0u8; 64];
    let n = sock.read(&mut buf).await?;
    buf.truncate(n);
    Ok(buf)
}

/// Parse a raw response into a structured [`ArcamResponse`].
///
/// Arcam response frame: `St(0x21) Zn Cc Ac Dl Data[Dl bytes] Et(0x0D)`
/// Minimum valid response is 7 bytes (header + 1 data byte + trailer).
fn parse_response(raw: Vec<u8>) -> Result<ArcamResponse, ArcamError> {
    if raw.len() < 7 {
        return Err(ArcamError::ResponseTooShort(raw.len(), hex_dump(&raw)));
    }
    if raw[0] != 0x21 {
        return Err(ArcamError::BadStartByte(hex_dump(&raw)));
    }

    let zone = raw[1];
    let command = raw[2];
    let answer_code = raw[3];
    let data_len = raw[4] as usize;
    // Data bytes start at index 5, followed by Et at index 5 + data_len
    let data = raw[5..5 + data_len.min(raw.len().saturating_sub(6))].to_vec();

    Ok(ArcamResponse {
        zone,
        command,
        answer_code,
        data,
        raw,
    })
}

/// Send a command and parse the response, returning an error for non-zero answer codes.
async fn send_command(ip_addr: &str, cmd_bytes: &[u8]) -> Result<ArcamResponse, ArcamError> {
    let raw = send_raw(ip_addr, cmd_bytes).await?;
    let resp = parse_response(raw)?;

    match resp.answer_code {
        0x00 => Ok(resp),
        0x82 => Err(ArcamError::ZoneInvalid(hex_dump(&resp.raw))),
        0x83 => Err(ArcamError::CommandNotRecognised(hex_dump(&resp.raw))),
        0x84 => Err(ArcamError::ParameterNotRecognised(hex_dump(&resp.raw))),
        0x86 => Err(ArcamError::InvalidDataLength(hex_dump(&resp.raw))),
        code => Err(ArcamError::UnknownAnswerCode {
            answer_code: code,
            raw: hex_dump(&resp.raw),
        }),
    }
}

/// Request the current power state.
///
/// Returns `Ok(true)` if ON, `Ok(false)` if standby/off.
async fn request_power_state(ip_addr: &str) -> Result<bool, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x00(Power) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip_addr, &cmd).await?;
    // Data[0]: 0x00 = standby, 0x01 = on
    Ok(resp.data.first().copied() == Some(0x01))
}

/// Power ON the amplifier.
async fn power_on(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x00(Power) Dl=0x01 Data=0x01(On) CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x01, 0x0D];
    send_command(ip_addr, &cmd).await?;
    Ok(())
}

/// Power OFF (standby) the amplifier.
async fn power_off(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x00(Power) Dl=0x01 Data=0x00(Standby) CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x00, 0x0D];
    send_command(ip_addr, &cmd).await?;
    Ok(())
}

/// Query whether mute is active.
///
/// Returns `Ok(true)` if muted, `Ok(false)` if un-muted.
async fn get_mute_status(ip: &str) -> Result<bool, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x0E(Mute) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    // Data[0]: 0x00 = muted, 0x01 = unmuted
    Ok(resp.data.first().copied() == Some(0x00))
}

/// Mute the amplifier (speaker outputs).
async fn mute_on(ip: &str) -> Result<(), ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x0E(Mute) Dl=0x01 Data=0x00(Mute) CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0x00, 0x0D];
    send_command(ip, &cmd).await?;
    Ok(())
}

/// Unmute the amplifier.
async fn mute_off(ip: &str) -> Result<(), ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x0E(Mute) Dl=0x01 Data=0x01(Unmute) CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0x01, 0x0D];
    send_command(ip, &cmd).await?;
    Ok(())
}

/// Query the amplifier's current mode (raw byte).
///
/// The SH305E Issue 3 protocol document claims 0-indexed values
/// (`0`=Stereo, `1`=Bridged, `2`=Dual Mono) but real-world testing
/// shows the firmware returns 1-indexed values:
/// `1`=Stereo, `2`=Bridged, `3`=Dual Mono.
async fn get_amplifier_mode(ip: &str) -> Result<u8, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x61(AmpMode) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x61, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    Ok(resp.data.first().copied().unwrap_or(0))
}

/// Send a heartbeat to check connectivity and reset the EuP standby timer.
///
/// Returns `true` if the amplifier responds with "Alive".
async fn heartbeat(ip: &str) -> Result<bool, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x25(Heartbeat) Dl=0x01 Data=0xF0(Ping) CR
    let cmd = [0x21, 0x01, 0x25, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    // Data[0]: 0x00 = alive
    Ok(resp.data.first().copied() == Some(0x00))
}

/// Query the system model name (e.g. "PA240", "PA720", "PA410").
///
/// Returns the model as a trimmed ASCII string (max 10 characters).
async fn get_system_model(ip: &str) -> Result<String, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x5E(SystemModel) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x5E, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    Ok(String::from_utf8_lossy(&resp.data).trim().to_string())
}

/// Query the auto-shutdown timer setting.
///
/// Returns the raw byte value:
/// - `0x00` = Disabled
/// - `0x01` = 20 min
/// - `0x02` = 30 min
/// - `0x03` = 1 hour
/// - `0x04` = 2 hours
/// - `0x05` = 4 hours
async fn get_auto_shutdown(ip: &str) -> Result<u8, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x58(AutoShutdown) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x58, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    Ok(resp.data.first().copied().unwrap_or(0))
}

/// Set the auto-shutdown timer setting.
///
/// Valid values:
/// - `0x00` = Disabled
/// - `0x01` = 20 min
/// - `0x02` = 30 min
/// - `0x03` = 1 hour
/// - `0x04` = 2 hours
///
/// Note: 4 hours (0x05) is not recommended as it may exceed network timeout.
async fn set_auto_shutdown(ip: &str, value: u8) -> Result<u8, ArcamError> {
    if value > 0x04 {
        return Err(ArcamError::InvalidParameter(format!(
            "Invalid auto-shutdown value: {}. Valid values: 0x00-0x04",
            value
        )));
    }
    // Frame: ! Zone=0x01 Cc=0x58(AutoShutdown) Dl=0x01 Data=value CR
    let cmd = [0x21, 0x01, 0x58, 0x01, value, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    Ok(resp.data.first().copied().unwrap_or(0))
}

/// Query the timeout counter (seconds remaining until auto-standby).
///
/// Returns seconds remaining as a big-endian u16 (range 0–14400).
async fn get_timeout_counter(ip: &str) -> Result<u16, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x55(TimeoutCounter) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x55, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    let hi = resp.data.first().copied().unwrap_or(0);
    let lo = resp.data.get(1).copied().unwrap_or(0);
    Ok(u16::from_be_bytes([hi, lo]))
}

/// Query lifter temperature (PA720/PA240 only).
///
/// Sensor: 0xF0 = sensor 1, 0xF1 = sensor 2
/// Returns temperature in degrees Celsius.
async fn get_lifter_temperature(ip: &str, sensor: u8) -> Result<u8, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x56(LifterTemp) Dl=0x01 Data=sensor CR
    let cmd = [0x21, 0x01, 0x56, 0x01, sensor, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    // Response: [sensor_id, temperature]
    Ok(resp.data.get(1).copied().unwrap_or(0))
}

/// Query output temperature.
///
/// Sensor: 0xF0 = sensor 1, 0xF1 = sensor 2
/// Returns temperature in degrees Celsius.
async fn get_output_temperature(ip: &str, sensor: u8) -> Result<u8, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x57(OutputTemp) Dl=0x01 Data=sensor CR
    let cmd = [0x21, 0x01, 0x57, 0x01, sensor, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    // Response: [sensor_id, temperature]
    Ok(resp.data.get(1).copied().unwrap_or(0))
}

/// Query friendly name (PA720/PA240 only).
async fn get_friendly_name(ip: &str) -> Result<String, ArcamError> {
    // Frame: ! Zone=0x01 Cc=0x53(FriendlyName) Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x53, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;
    Ok(String::from_utf8_lossy(&resp.data).trim().to_string())
}

/// Set friendly name (PA720/PA240 only).
///
/// Name must be uppercase A-Z, digits 0-9, and space only. Max 10 characters.
async fn set_friendly_name(ip: &str, name: &str) -> Result<String, ArcamError> {
    let name_bytes = name.as_bytes();
    if name_bytes.len() > 10 {
        return Err(ArcamError::InvalidDataLength(
            "friendly name exceeds 10 characters".to_string(),
        ));
    }
    let mut cmd = vec![0x21, 0x01, 0x53, name_bytes.len() as u8];
    cmd.extend_from_slice(name_bytes);
    cmd.push(0x0D);
    let resp = send_command(ip, &cmd).await?;
    Ok(String::from_utf8_lossy(&resp.data).trim().to_string())
}

/// Query all system status parameters via System Status (0x5D) command.
///
/// This triggers the amp to send individual status responses for all parameters:
/// Power, Software Version, Mute, Friendly Name*, IP Address*, Timeout Counter,
/// Lifter Temperature*, Output Temperature, Auto Shutdown, Input Detect,
/// System Model, Amplifier Mode**.
///
/// *PA720/PA240 only. **PA240 only.
async fn get_system_status(ip: &str) -> Result<ArcamSystemStatus, ArcamError> {
    use tokio::time::timeout;

    let addr = format!("{}:50000", ip);
    let mut sock = TcpStream::connect(addr).await?;

    // Send System Status command: ! Zone=0x01 Cc=0x5D Dl=0x01 Data=0xF0(Query) CR
    let cmd = [0x21, 0x01, 0x5D, 0x01, 0xF0, 0x0D];
    sock.write_all(&cmd).await?;

    let mut status = ArcamSystemStatus::default();
    let mut buf = vec![0u8; 64];

    // Read initial acknowledgment and subsequent status messages
    // Each frame is at least 7 bytes, we read with short timeouts between frames
    loop {
        match timeout(Duration::from_millis(300), sock.read(&mut buf)).await {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(n)) => {
                let frame = &buf[..n];
                if let Ok(resp) = parse_response(frame.to_vec()) {
                    match resp.command {
                        // Power (0x00): 0x00 = standby, 0x01 = on
                        0x00 => status.power = resp.data.first().copied() == Some(0x01),
                        // Mute (0x0E): 0x00 = muted, 0x01 = unmuted
                        0x0E => status.muted = resp.data.first().copied() == Some(0x00),
                        // Software Version (0x04): 2 bytes major.minor
                        0x04 => {
                            if resp.data.len() >= 2 {
                                status.software_version =
                                    Some(format!("{}.{}", resp.data[0], resp.data[1]));
                            }
                        }
                        // Friendly Name (0x53): ASCII string
                        0x53 => {
                            let name = String::from_utf8_lossy(&resp.data).trim().to_string();
                            if !name.is_empty() {
                                status.friendly_name = Some(name);
                            }
                        }
                        // IP Address (0x54): 4 bytes
                        0x54 => {
                            if resp.data.len() >= 4 {
                                status.ip_address = Some(format!(
                                    "{}.{}.{}.{}",
                                    resp.data[0], resp.data[1], resp.data[2], resp.data[3]
                                ));
                            }
                        }
                        // Timeout Counter (0x55): 2 bytes big-endian
                        0x55 => {
                            if resp.data.len() >= 2 {
                                status.timeout_counter = Some(u16::from_be_bytes([
                                    resp.data[0],
                                    resp.data[1],
                                ]));
                            }
                        }
                        // Auto Shutdown (0x58): 1 byte
                        0x58 => status.auto_shutdown = resp.data.first().copied(),
                        // Input Detect (0x5A): 0x00 = no signal, 0x01 = signal present
                        0x5A => status.input_detect = Some(resp.data.first().copied() == Some(0x01)),
                        // System Model (0x5E): ASCII string
                        0x5E => {
                            let model = String::from_utf8_lossy(&resp.data).trim().to_string();
                            if !model.is_empty() {
                                status.model = Some(model);
                            }
                        }
                        // Amplifier Mode (0x61): 1 byte (PA240 only)
                        0x61 => status.amplifier_mode = resp.data.first().copied(),
                        // Skip the ack response for 0x5D itself
                        0x5D => {}
                        _ => {}
                    }
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }

    Ok(status)
}

/// Convert an auto-shutdown byte value to a human-readable label.
pub fn auto_shutdown_label(value: u8) -> &'static str {
    match value {
        0x00 => "Off",
        0x01 => "20 min",
        0x02 => "30 min",
        0x03 => "1 hour",
        0x04 => "2 hours",
        0x05 => "4 hours",
        _ => "Unknown",
    }
}

/// Convert an auto-shutdown byte value to seconds.
///
/// Returns the timeout in seconds, or 0 if disabled.
pub fn auto_shutdown_seconds(value: u8) -> u32 {
    match value {
        0x00 => 0,       // Disabled
        0x01 => 20 * 60, // 20 min
        0x02 => 30 * 60, // 30 min
        0x03 => 60 * 60, // 1 hour
        0x04 => 2 * 60 * 60, // 2 hours
        _ => 0,
    }
}

/// **Arcam** struct is used to power on and off (as well
/// as request the current power state for) an Arcam PA240/PA410/PA720
/// amplifier.
///
/// This leverages the [Arcam Serial over IP API](https://www.arcam.co.uk/ugc/tor/PA240/Custom%20Installation%20Notes/RS232_PA720_PA240_PA410_SH305E_3.pdf).
pub struct Arcam {
    host: Host,
}

impl Arcam {
    /// Create a new Arcam struct with a "host" (IP address
    /// or DNS name).)
    pub fn new(host: Host) -> Self {
        Arcam { host }
    }

    /// Send a signal to the Arcam amplifier to turn the
    /// power on.
    pub async fn power_on(&self) -> Result<(), ArcamError> {
        let addr = self.host_addr();
        power_on(&addr).await
    }

    /// Send a signal to the Arcam amplifier to turn the
    /// power on.
    pub async fn power_off(&self) -> Result<(), ArcamError> {
        let addr = self.host_addr();
        power_off(&addr).await
    }

    /// Query the Arcam amplifier on what power state it
    /// is currently in (true = on).
    pub async fn request_power_state(&self) -> Result<bool, ArcamError> {
        let addr = self.host_addr();
        request_power_state(&addr).await
    }

    /// Query whether mute is active.
    /// Returns `Ok(true)` if muted, `Ok(false)` if un-muted.
    pub async fn get_mute_status(&self) -> Result<bool, ArcamError> {
        let addr = self.host_addr();
        get_mute_status(&addr).await
    }

    /// Mute the amplifier (speaker outputs).
    pub async fn mute_on(&self) -> Result<(), ArcamError> {
        let addr = self.host_addr();
        mute_on(&addr).await
    }

    /// Unmute the amplifier.
    pub async fn mute_off(&self) -> Result<(), ArcamError> {
        let addr = self.host_addr();
        mute_off(&addr).await
    }

    /// Query the amplifier's current mode (raw byte).
    ///
    /// Real-world testing shows the firmware returns 1-indexed values:
    /// `1`=Stereo, `2`=Bridged, `3`=Dual Mono.
    pub async fn get_amplifier_mode(&self) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        get_amplifier_mode(&addr).await
    }

    /// Send a heartbeat to check connectivity and reset the EuP standby timer.
    ///
    /// On the PA series, sending any command (including heartbeat) while the amp
    /// is reachable keeps the network interface active during standby. Calling
    /// this periodically prevents the amp from powering down its network port.
    ///
    /// Returns `true` if the amplifier responds with "Alive".
    pub async fn heartbeat(&self) -> Result<bool, ArcamError> {
        let addr = self.host_addr();
        heartbeat(&addr).await
    }

    /// Query the system model name (e.g. "PA240", "PA720", "PA410").
    pub async fn get_system_model(&self) -> Result<String, ArcamError> {
        let addr = self.host_addr();
        get_system_model(&addr).await
    }

    /// Query the auto-shutdown timer setting.
    ///
    /// Returns the raw byte: 0x00=Disabled, 0x01=20min, 0x02=30min,
    /// 0x03=1hr, 0x04=2hr, 0x05=4hr.
    pub async fn get_auto_shutdown(&self) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        get_auto_shutdown(&addr).await
    }

    /// Set the auto-shutdown timer setting.
    ///
    /// Valid values:
    /// - `0x00` = Disabled
    /// - `0x01` = 20 min
    /// - `0x02` = 30 min
    /// - `0x03` = 1 hour
    /// - `0x04` = 2 hours
    ///
    /// Note: 4 hours (0x05) is not recommended as it may exceed network timeout.
    pub async fn set_auto_shutdown(&self, value: u8) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        set_auto_shutdown(&addr, value).await
    }

    /// Query seconds remaining until auto-standby (0–14400).
    pub async fn get_timeout_counter(&self) -> Result<u16, ArcamError> {
        let addr = self.host_addr();
        get_timeout_counter(&addr).await
    }

    /// Query all system status parameters via System Status (0x5D) command.
    ///
    /// This is more efficient than querying individual parameters as it retrieves
    /// all status information in a single network round-trip.
    ///
    /// Returns a struct containing: power, mute, software_version, friendly_name,
    /// ip_address, timeout_counter, auto_shutdown, input_detect, model, amplifier_mode.
    pub async fn get_system_status(&self) -> Result<ArcamSystemStatus, ArcamError> {
        let addr = self.host_addr();
        get_system_status(&addr).await
    }

    /// Query lifter temperature (PA720/PA240 only).
    ///
    /// Sensor 1 (0xF0) is the primary sensor. Returns temperature in Celsius.
    pub async fn get_lifter_temperature(&self, sensor: u8) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        get_lifter_temperature(&addr, sensor).await
    }

    /// Query output temperature.
    ///
    /// Sensor 1 (0xF0) is the primary sensor. Returns temperature in Celsius.
    pub async fn get_output_temperature(&self, sensor: u8) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        get_output_temperature(&addr, sensor).await
    }

    /// Query friendly name (PA720/PA240 only).
    pub async fn get_friendly_name(&self) -> Result<String, ArcamError> {
        let addr = self.host_addr();
        get_friendly_name(&addr).await
    }

    /// Set friendly name (PA720/PA240 only).
    ///
    /// Name must be uppercase A-Z, digits 0-9, and space only. Max 10 characters.
    pub async fn set_friendly_name(&self, name: &str) -> Result<String, ArcamError> {
        let addr = self.host_addr();
        set_friendly_name(&addr, name).await
    }

    /// Send a raw command frame and return the parsed response.
    ///
    /// Useful for diagnostics and debugging protocol issues.
    pub async fn send_command(&self, cmd_bytes: &[u8]) -> Result<ArcamResponse, ArcamError> {
        let addr = self.host_addr();
        send_command(&addr, cmd_bytes).await
    }

    fn host_addr(&self) -> String {
        match &self.host {
            Host::V4(addr) => addr.to_string(),
            Host::V6(addr) => format!("[{addr}]"),
            Host::Dns(name) => name.clone(),
        }
    }
}

impl From<Host> for Arcam {
    fn from(host: Host) -> Self {
        Self::new(host)
    }
}

impl From<Ipv4Addr> for Arcam {
    fn from(host: Ipv4Addr) -> Self {
        Self::new(Host::V4(host))
    }
}

impl From<std::net::Ipv6Addr> for Arcam {
    fn from(host: std::net::Ipv6Addr) -> Self {
        Self::new(Host::V6(host))
    }
}

impl From<String> for Arcam {
    fn from(host: String) -> Self {
        Self::from(host.as_str())
    }
}

impl From<&str> for Arcam {
    fn from(host: &str) -> Self {
        if let Ok(ipv4) = host.parse::<Ipv4Addr>() {
            return Self::new(Host::V4(ipv4));
        }
        if let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>() {
            return Self::new(Host::V6(ipv6));
        }
        Self::new(Host::Dns(host.to_string()))
    }
}

impl From<IpAddr> for Arcam {
    fn from(host: IpAddr) -> Self {
        match host {
            IpAddr::V4(addr) => Self::from(addr),
            IpAddr::V6(addr) => Self::from(addr),
        }
    }
}
