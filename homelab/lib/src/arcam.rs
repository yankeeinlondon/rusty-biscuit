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

    #[error("Invalid response for power state")]
    InvalidResponse,

    #[error("Invalid response for mute status")]
    InvalidMuteStatusResponse,

    #[error("Invalid response for amplifier mode")]
    InvalidAmplifierModeResponse,
}

/// Send a raw command frame and optionally read a response.
///
/// `cmd_bytes` should be the complete PA240/PA410/PA720 command frame.
/// Returns the raw response bytes on success.
async fn send_command(ip_addr: &str, cmd_bytes: &[u8]) -> Result<Vec<u8>, ArcamError> {
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

/// Request the current power state.
///
/// Returns `Ok(true)` if ON, `Ok(false)` if standby/off.
async fn request_power_state(ip_addr: &str) -> Result<bool, ArcamError> {
    // Frame: ! 0x01 0x00 [len=1] 0xF0 CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip_addr, &cmd).await?;

    // Typical response: ! 0x01 0x00 <AnswerCode> 0x01 <state> CR
    // State byte is usually the last before CR
    if resp.len() >= 6 {
        let state = resp[resp.len() - 2];
        return Ok(state == 0x01);
    }

    Err(ArcamError::InvalidResponse)
}

/// Power ON the amplifier.
async fn power_on(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x00 [len=1] 0x01 CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x01, 0x0D];
    let _ = send_command(ip_addr, &cmd).await?;
    Ok(())
}

/// Power OFF (standby) the amplifier.
async fn power_off(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x00 [len=1] 0x00 CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x00, 0x0D];
    let _ = send_command(ip_addr, &cmd).await?;
    Ok(())
}

/// Query whether mute is active.
/// Returns `Ok(true)` if muted, `Ok(false)` if un-muted.
async fn get_mute_status(ip: &str) -> Result<bool, ArcamError> {
    // Frame: ! 0x01 0x0E 0x01 0xF0 CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;

    // Typical response: ! 01 0E AC 01 <status> CR
    if resp.len() >= 6 {
        let status = resp[resp.len() - 2];
        return Ok(status == 0x00); // 0x00 = muted
    }
    Err(ArcamError::InvalidMuteStatusResponse)
}

/// Mute the amplifier (speaker outputs).
async fn mute_on(ip: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x0E 0x01 0x00 CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0x00, 0x0D];
    let _ = send_command(ip, &cmd).await?;
    Ok(())
}

/// Unmute the amplifier.
async fn mute_off(ip: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x0E 0x01 0x01 CR
    let cmd = [0x21, 0x01, 0x0E, 0x01, 0x01, 0x0D];
    let _ = send_command(ip, &cmd).await?;
    Ok(())
}

/// Query the amplifier’s current mode:
/// returns:
///   0 => Stereo
///   1 => Bridged
///   2 => Dual Mono
async fn get_amplifier_mode(ip: &str) -> Result<u8, ArcamError> {
    // Frame: ! 0x01 0x61 0x01 0xF0 CR
    let cmd = [0x21, 0x01, 0x61, 0x01, 0xF0, 0x0D];
    let resp = send_command(ip, &cmd).await?;

    // Typical response: ! 01 61 AC 01 <mode> CR
    if resp.len() >= 6 {
        let mode = resp[resp.len() - 2];
        return Ok(mode);
    }
    Err(ArcamError::InvalidAmplifierModeResponse)
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

    /// Query the amplifier’s current mode:
    /// returns:
    ///   0 => Stereo
    ///   1 => Bridged
    ///   2 => Dual Mono
    pub async fn get_amplifier_mode(&self) -> Result<u8, ArcamError> {
        let addr = self.host_addr();
        get_amplifier_mode(&addr).await
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
        Self::new(Host::Dns(host))
    }
}

impl From<&str> for Arcam {
    fn from(host: &str) -> Self {
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
