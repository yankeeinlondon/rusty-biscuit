//! Datagram ICMP sockets (macOS, Linux, WSL2).

use std::io::{self, Read};
use std::net::{IpAddr, SocketAddrV4, SocketAddrV6};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::packet::{self, MAX_REPLY_LEN, TOKEN_LEN};
use super::{EchoOutcome, IcmpError, scope_index};
use crate::network::ScopedIpAddr;

#[derive(Default)]
pub(super) struct UnixProbe {
    v4: Option<Socket>,
    v6: Option<Socket>,
    sequence: u16,
}

impl UnixProbe {
    pub(super) fn echo(
        &mut self,
        target: &ScopedIpAddr,
        timeout: Duration,
    ) -> Result<EchoOutcome, IcmpError> {
        let started = Instant::now();
        let deadline = started + timeout;

        let (ipv6, address) = match target.address() {
            IpAddr::V4(address) => (false, SockAddr::from(SocketAddrV4::new(address, 0))),
            IpAddr::V6(address) => {
                let scope = scope_index(target)?;
                (true, SockAddr::from(SocketAddrV6::new(address, 0, 0, scope)))
            }
        };
        let slot = if ipv6 { &mut self.v6 } else { &mut self.v4 };
        let socket = match slot {
            Some(socket) => socket,
            None => slot.insert(open(ipv6)?),
        };

        self.sequence = self.sequence.wrapping_add(1);
        let sequence = self.sequence;
        let token = packet::token();
        // The identifier is advisory: Linux replaces it with the socket's port.
        let identifier = (std::process::id() & 0xffff) as u16;
        let request = packet::echo_request(ipv6, identifier, sequence, &token);

        match socket.send_to(&request, &address) {
            Ok(_) => {}
            Err(error) if is_unreachable(&error) => return Ok(EchoOutcome::NoReply),
            Err(error) if is_permission(&error) => return Err(not_permitted(ipv6, &error)),
            Err(source) => {
                return Err(IcmpError::Send {
                    target: target.to_string(),
                    source,
                });
            }
        }

        let socket = &*socket;
        wait_for_reply(deadline, |remaining, buffer| {
            socket.set_read_timeout(Some(remaining))?;
            let mut reader = socket;
            reader.read(buffer)
        }, |datagram| packet::is_echo_reply(ipv6, datagram, sequence, &token))
        .map(|replied| {
            if replied {
                EchoOutcome::Reply {
                    round_trip: started.elapsed(),
                }
            } else {
                EchoOutcome::NoReply
            }
        })
        .map_err(|source| IcmpError::Send {
            target: target.to_string(),
            source,
        })
    }
}

/// Reads datagrams until one matches or `deadline` passes.
///
/// `receive` must return within the remaining time it is given; timeouts,
/// interrupts, unrelated datagrams, and ICMP unreachable reports keep waiting
/// or end as no reply. Any other read error is returned.
fn wait_for_reply(
    deadline: Instant,
    mut receive: impl FnMut(Duration, &mut [u8]) -> io::Result<usize>,
    mut matches: impl FnMut(&[u8]) -> bool,
) -> io::Result<bool> {
    let mut buffer = [0_u8; MAX_REPLY_LEN + TOKEN_LEN];
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(false);
        }
        // A zero socket timeout means "block forever".
        let remaining = (deadline - now).max(Duration::from_micros(1));
        match receive(remaining, &mut buffer) {
            Ok(len) => {
                if matches(&buffer[..len]) {
                    return Ok(true);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
                ) || is_unreachable(&error) => {}
            Err(error) => return Err(error),
        }
    }
}

fn open(ipv6: bool) -> Result<Socket, IcmpError> {
    let (domain, protocol) = if ipv6 {
        (Domain::IPV6, Protocol::ICMPV6)
    } else {
        (Domain::IPV4, Protocol::ICMPV4)
    };
    Socket::new(domain, Type::DGRAM, Some(protocol)).map_err(|error| {
        if is_permission(&error) {
            not_permitted(ipv6, &error)
        } else {
            IcmpError::Send {
                target: if ipv6 { "IPv6" } else { "IPv4" }.to_string(),
                source: error,
            }
        }
    })
}

/// The target cannot be reached: the outcome is no reply, not a local failure.
fn is_unreachable(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(libc::ENETUNREACH | libc::EHOSTUNREACH | libc::EHOSTDOWN)
    )
}

fn is_permission(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::EACCES | libc::EPERM))
}

fn not_permitted(ipv6: bool, error: &io::Error) -> IcmpError {
    let family = if ipv6 { "ICMPv6" } else { "ICMP" };
    let hint = if cfg!(target_os = "linux") {
        "; allow unprivileged ICMP by including this process's group in the \
         `net.ipv4.ping_group_range` sysctl (it governs IPv4 and IPv6)"
    } else {
        ""
    };
    IcmpError::NotPermitted {
        detail: format!("opening a datagram {family} socket failed: {error}{hint}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn a_noisy_socket_ends_as_no_reply_at_the_deadline() {
        let deadline = Instant::now() + 40 * MS;
        let mut reads = 0;
        let replied = wait_for_reply(
            deadline,
            |remaining, buffer| {
                reads += 1;
                std::thread::sleep(remaining.min(2 * MS));
                buffer[0] = 0xff;
                Ok(1)
            },
            |_| false,
        )
        .unwrap();
        assert!(!replied);
        assert!(reads > 1, "unrelated datagrams keep the wait going");
        assert!(Instant::now() < deadline + 20 * MS, "bounded by the deadline");
    }

    #[test]
    fn interrupts_and_unreachable_reports_keep_waiting_for_the_match() {
        let mut script = vec![
            Err(io::Error::from(io::ErrorKind::Interrupted)),
            Err(io::Error::from_raw_os_error(libc::EHOSTUNREACH)),
            Err(io::Error::from(io::ErrorKind::WouldBlock)),
            Ok(3),
        ]
        .into_iter();
        let replied = wait_for_reply(
            Instant::now() + Duration::from_secs(5),
            |_, _| script.next().unwrap(),
            |datagram| datagram.len() == 3,
        )
        .unwrap();
        assert!(replied);
    }

    #[test]
    fn a_read_failure_is_an_error() {
        let error = wait_for_reply(
            Instant::now() + Duration::from_secs(5),
            |_, _| Err(io::Error::from_raw_os_error(libc::EBADF)),
            |_| true,
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EBADF));
    }

    #[test]
    fn a_passed_deadline_never_reads() {
        let replied = wait_for_reply(
            Instant::now() - MS,
            |_, _| panic!("must not read after the deadline"),
            |_| true,
        )
        .unwrap();
        assert!(!replied);
    }

    #[test]
    fn socket_errors_are_classified() {
        for errno in [libc::ENETUNREACH, libc::EHOSTUNREACH, libc::EHOSTDOWN] {
            assert!(is_unreachable(&io::Error::from_raw_os_error(errno)));
        }
        for errno in [libc::EACCES, libc::EPERM] {
            let error = io::Error::from_raw_os_error(errno);
            assert!(is_permission(&error));
            assert!(matches!(
                not_permitted(false, &error),
                IcmpError::NotPermitted { .. }
            ));
        }
        assert!(!is_unreachable(&io::Error::from_raw_os_error(libc::EADDRNOTAVAIL)));
        assert!(!is_permission(&io::Error::from_raw_os_error(libc::EADDRNOTAVAIL)));
    }
}
