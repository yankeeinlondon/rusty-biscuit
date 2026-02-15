use std::io;
use std::net::{IpAddr, Ipv4Addr};
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

/// Query the amplifier's current mode.
///
/// Returns `0` = Stereo, `1` = Bridged, `2` = Dual Mono.
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

    /// Query the amplifier's current mode.
    ///
    /// Returns `0` = Stereo, `1` = Bridged, `2` = Dual Mono.
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
