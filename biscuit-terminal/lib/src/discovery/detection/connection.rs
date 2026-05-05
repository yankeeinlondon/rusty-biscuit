use std::env;

use serde::{Deserialize, Serialize};

/// Client information for a host which was established over an SSH connection.
///
/// Parsed from the `SSH_CLIENT` environment variable which has the format:
/// `<client_ip> <client_port> <server_port>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshClient {
    /// The IP address or DNS name of the client connecting
    pub host: String,

    /// The port which the host is communicating back to the client on
    pub source_port: u32,

    /// The port the client used to connect to the host (typically 22)
    pub server_port: u32,

    /// The TTY path for the SSH session (from `SSH_TTY`)
    pub tty_path: Option<String>,
}

/// Client information for a host which was established over a Mosh connection.
///
/// Mosh (Mobile Shell) provides a more resilient remote connection that
/// handles intermittent connectivity and roaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoshClient {
    /// The connection string from `MOSH_CONNECTION`
    pub connection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Connection {
    /// This terminal connection is a local connection
    Local,
    /// This terminal is using a SSH connection
    SshClient(SshClient),
    MoshClient(MoshClient),
}

/// Detect whether the terminal session is a remote connection (SSH, Mosh) or local.
///
/// ## Detection Strategy
///
/// 1. Check `SSH_CLIENT` environment variable for SSH connections
/// 2. Check `MOSH_CONNECTION` for Mosh connections
/// 3. Default to `Connection::Local` if no remote indicators
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{detect_connection, Connection};
///
/// match detect_connection() {
///     Connection::Local => println!("Running locally"),
///     Connection::SshClient(ssh) => println!("SSH from {}", ssh.host),
///     Connection::MoshClient(mosh) => println!("Mosh connection: {}", mosh.connection),
/// }
/// ```
pub fn detect_connection() -> Connection {
    // Check for Mosh first (it also sets SSH_CLIENT sometimes)
    if let Ok(mosh_conn) = env::var("MOSH_CONNECTION")
        && !mosh_conn.is_empty()
    {
        return Connection::MoshClient(MoshClient {
            connection: mosh_conn,
        });
    }

    // Check for SSH connection
    // SSH_CLIENT format: "client_ip client_port server_port"
    if let Ok(ssh_client) = env::var("SSH_CLIENT") {
        let parts: Vec<&str> = ssh_client.split_whitespace().collect();
        if parts.len() >= 3
            && let (Ok(source_port), Ok(server_port)) =
                (parts[1].parse::<u32>(), parts[2].parse::<u32>())
        {
            let tty_path = env::var("SSH_TTY").ok();
            return Connection::SshClient(SshClient {
                host: parts[0].to_string(),
                source_port,
                server_port,
                tty_path,
            });
        }
    }

    Connection::Local
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_connection_returns_variant() {
        let _ = detect_connection();
    }

    #[test]
    fn test_connection_serialize_local() {
        let conn = Connection::Local;
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("Local"));
    }

    #[test]
    fn test_ssh_client_clone() {
        let ssh = SshClient {
            host: "192.168.1.1".to_string(),
            source_port: 22,
            server_port: 22,
            tty_path: Some("/dev/pts/0".to_string()),
        };
        let cloned = ssh.clone();
        assert_eq!(cloned.host, ssh.host);
    }

    #[test]
    fn test_mosh_client_clone() {
        let mosh = MoshClient {
            connection: "mosh://test".to_string(),
        };
        let cloned = mosh.clone();
        assert_eq!(cloned.connection, mosh.connection);
    }
}
