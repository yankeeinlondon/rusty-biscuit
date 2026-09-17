//! `IcmpSendEcho2` / `Icmp6SendEcho2` (native Windows).
//!
//! The ICMP helper API needs no privilege, matches replies to requests itself,
//! and returns within the supplied timeout, so no socket or subprocess is used.

use std::io;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, ICMPV6_ECHO_REPLY_LH, IP_DEST_HOST_UNREACHABLE, IP_DEST_NET_UNREACHABLE,
    IP_DEST_PORT_UNREACHABLE, IP_DEST_PROT_UNREACHABLE, IP_DEST_UNREACHABLE,
    IP_HOP_LIMIT_EXCEEDED, IP_REQ_TIMED_OUT, IP_SUCCESS, Icmp6CreateFile, Icmp6ParseReplies,
    Icmp6SendEcho2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho2,
};
use windows::Win32::Networking::WinSock::{AF_INET6, IN6_ADDR, IN6_ADDR_0, SOCKADDR_IN6, SOCKADDR_IN6_0};

use super::{EchoOutcome, IcmpError, scope_index};
use crate::network::ScopedIpAddr;

const REQUEST_DATA: [u8; 16] = *b"sniff-icmp-probe";
const ERROR_ACCESS_DENIED: u32 = 5;

/// An open ICMP helper handle, closed on drop.
struct IcmpHandle(HANDLE);

impl Drop for IcmpHandle {
    fn drop(&mut self) {
        // SAFETY: the handle came from `IcmpCreateFile`/`Icmp6CreateFile` and is
        // closed exactly once.
        let _ = unsafe { IcmpCloseHandle(self.0) };
    }
}

#[derive(Default)]
pub(super) struct WindowsProbe {
    v4: Option<IcmpHandle>,
    v6: Option<IcmpHandle>,
}

impl WindowsProbe {
    pub(super) fn echo(
        &mut self,
        target: &ScopedIpAddr,
        timeout: Duration,
    ) -> Result<EchoOutcome, IcmpError> {
        // The API counts whole milliseconds; round a sub-millisecond remainder
        // up so the wait is never shorter than asked. Budgets cap at 60 s.
        let timeout_ms = u32::try_from(timeout.as_micros().div_ceil(1000))
            .map_err(|error| IcmpError::InvalidBudget(error.to_string()))?;
        // Aligned for the reply structures, which hold pointers.
        let mut reply = vec![0_u64; 64];
        let reply_len = u32::try_from(reply.len() * size_of::<u64>()).unwrap_or(u32::MAX);
        let started = Instant::now();

        let status = match target.address() {
            IpAddr::V4(address) => {
                let handle = open(&mut self.v4, false)?;
                // SAFETY: request and reply buffers are valid for the given lengths
                // for the duration of this synchronous call.
                let count = unsafe {
                    IcmpSendEcho2(
                        handle,
                        None,
                        None,
                        None,
                        u32::from_ne_bytes(address.octets()),
                        REQUEST_DATA.as_ptr().cast(),
                        REQUEST_DATA.len() as u16,
                        None,
                        reply.as_mut_ptr().cast(),
                        reply_len,
                        timeout_ms,
                    )
                };
                if count == 0 {
                    return classify_failure(target, io::Error::last_os_error());
                }
                // SAFETY: a non-zero count means the buffer starts with a reply.
                unsafe { (*reply.as_ptr().cast::<ICMP_ECHO_REPLY>()).Status }
            }
            IpAddr::V6(address) => {
                let scope = scope_index(target)?;
                let handle = open(&mut self.v6, true)?;
                let source = SOCKADDR_IN6 {
                    sin6_family: AF_INET6,
                    ..Default::default()
                };
                let destination = SOCKADDR_IN6 {
                    sin6_family: AF_INET6,
                    sin6_addr: IN6_ADDR {
                        u: IN6_ADDR_0 {
                            Byte: address.octets(),
                        },
                    },
                    Anonymous: SOCKADDR_IN6_0 {
                        sin6_scope_id: scope,
                    },
                    ..Default::default()
                };
                // SAFETY: as above; both sockaddrs outlive the call.
                let count = unsafe {
                    Icmp6SendEcho2(
                        handle,
                        None,
                        None,
                        None,
                        &source,
                        &destination,
                        REQUEST_DATA.as_ptr().cast(),
                        REQUEST_DATA.len() as u16,
                        None,
                        reply.as_mut_ptr().cast(),
                        reply_len,
                        timeout_ms,
                    )
                };
                if count == 0 {
                    return classify_failure(target, io::Error::last_os_error());
                }
                // SAFETY: the buffer holds the reply `Icmp6SendEcho2` wrote.
                if unsafe { Icmp6ParseReplies(reply.as_mut_ptr().cast(), reply_len) } == 0 {
                    return classify_failure(target, io::Error::last_os_error());
                }
                // SAFETY: a parsed reply starts the buffer.
                unsafe { (*reply.as_ptr().cast::<ICMPV6_ECHO_REPLY_LH>()).Status }
            }
        };

        if status == IP_SUCCESS {
            Ok(EchoOutcome::Reply {
                round_trip: started.elapsed(),
            })
        } else {
            Ok(EchoOutcome::NoReply)
        }
    }
}

fn open(slot: &mut Option<IcmpHandle>, ipv6: bool) -> Result<HANDLE, IcmpError> {
    if let Some(handle) = slot {
        return Ok(handle.0);
    }
    // SAFETY: plain handle-returning calls with no arguments.
    let opened = unsafe {
        if ipv6 {
            Icmp6CreateFile()
        } else {
            IcmpCreateFile()
        }
    };
    let handle = opened.map_err(|error| {
        let source = io::Error::from_raw_os_error(error.code().0 & 0xffff);
        if source.raw_os_error() == i32::try_from(ERROR_ACCESS_DENIED).ok() {
            IcmpError::NotPermitted {
                detail: format!("opening the ICMP helper failed: {error}"),
            }
        } else {
            IcmpError::Send {
                target: if ipv6 { "IPv6" } else { "IPv4" }.to_string(),
                source,
            }
        }
    })?;
    Ok(slot.insert(IcmpHandle(handle)).0)
}

/// A zero reply count is a timeout or an unreachable target unless the error
/// is a local failure.
fn classify_failure(target: &ScopedIpAddr, error: io::Error) -> Result<EchoOutcome, IcmpError> {
    let code = error.raw_os_error().and_then(|code| u32::try_from(code).ok());
    match code {
        Some(
            IP_REQ_TIMED_OUT
            | IP_DEST_HOST_UNREACHABLE
            | IP_DEST_NET_UNREACHABLE
            | IP_DEST_PORT_UNREACHABLE
            | IP_DEST_PROT_UNREACHABLE
            | IP_DEST_UNREACHABLE
            | IP_HOP_LIMIT_EXCEEDED,
        ) => Ok(EchoOutcome::NoReply),
        Some(ERROR_ACCESS_DENIED) => Err(IcmpError::NotPermitted {
            detail: format!("sending an ICMP echo to {target} was denied: {error}"),
        }),
        _ => Err(IcmpError::Send {
            target: target.to_string(),
            source: error,
        }),
    }
}
