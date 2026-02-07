use std::io;
use std::net::Ipv4Addr;
use thiserror::Error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::network::Host;

/// Errors that can occur when communicating with the Arcam PA240.
#[derive(Debug, Error)]
pub enum ArcamError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid response for power state")]
    InvalidResponse,
}

/// Send a raw command frame and optionally read a response.
///
/// `cmd_bytes` should be the complete PA240 command frame.
/// Returns the raw response bytes on success.
async fn send_pa240_command(ip_addr: &str, cmd_bytes: &[u8]) -> Result<Vec<u8>, ArcamError> {
    // Arcam PA240 IP control default port is 50000
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
    let resp = send_pa240_command(ip_addr, &cmd).await?;

    // Typical response: ! 0x01 0x00 <AnswerCode> 0x01 <state> CR
    // State byte is usually the last before CR
    if resp.len() >= 6 {
        let state = resp[resp.len() - 2];
        return Ok(state == 0x01);
    }

    Err(ArcamError::InvalidResponse)
}

/// Power ON the PA240.
async fn power_on(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x00 [len=1] 0x01 CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x01, 0x0D];
    let _ = send_pa240_command(ip_addr, &cmd).await?;
    Ok(())
}

/// Power OFF (standby) the PA240.
async fn power_off(ip_addr: &str) -> Result<(), ArcamError> {
    // Frame: ! 0x01 0x00 [len=1] 0x00 CR
    let cmd = [0x21, 0x01, 0x00, 0x01, 0x00, 0x0D];
    let _ = send_pa240_command(ip_addr, &cmd).await?;
    Ok(())
}

pub struct Arcam {
    host: Host,
}

impl Arcam {
    pub fn new(host: Host) -> Self {
        Arcam { host: host }
    }

    pub async fn power_on() -> Result<(), ArcamError> {
        todo!()
    }

    pub async fn power_off() -> Result<(), ArcamError> {
        todo!()
    }


}
