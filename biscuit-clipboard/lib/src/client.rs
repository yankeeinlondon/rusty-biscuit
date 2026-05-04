use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config;

const HEADER_CLIPPER: &str = "x-clipper";

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("service not running")]
    ServiceNotRunning,

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("service error: {0}")]
    Service(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntrySummary {
    pub id: String,
    pub timestamp: String,
    pub content_type: String,
    pub preview: String,
    pub size_bytes: usize,
}

#[derive(Debug)]
pub enum ServiceStatus {
    Running { pid: u32, port: u16 },
    Stopped,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct HealthResponse {
    status: String,
    entries: usize,
}

#[derive(Deserialize)]
struct HistoryResponse {
    entries: Vec<EntrySummary>,
}

#[derive(Deserialize)]
struct SetResponse {
    id: String,
}

pub struct ClipperClient {
    client: reqwest::Client,
    base_url: Option<String>,
}

impl Default for ClipperClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipperClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url: None,
        }
    }

    pub async fn ensure_running(&mut self) -> Result<(), ClientError> {
        if self.try_connect().await.is_ok() {
            return Ok(());
        }

        self.spawn_service()?;
        self.poll_until_ready().await
    }

    pub async fn get_current(&self) -> Result<Option<String>, ClientError> {
        let url = format!("{}/current", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("text/plain") {
            let text = resp.text().await?;
            Ok(Some(text))
        } else {
            let summary: EntrySummary = resp.json().await?;
            Ok(Some(summary.preview))
        }
    }

    pub async fn set_text(&self, text: &str) -> Result<String, ClientError> {
        let url = format!("{}/set", self.base_url());
        let body = serde_json::json!({ "text": text });
        let resp = self
            .client
            .post(&url)
            .header(HEADER_CLIPPER, "1")
            .json(&body)
            .send()
            .await?;

        let result: SetResponse = resp.json().await?;
        Ok(result.id)
    }

    pub async fn get_history(&self) -> Result<Vec<EntrySummary>, ClientError> {
        let url = format!("{}/history", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;
        let history: HistoryResponse = resp.json().await?;
        Ok(history.entries)
    }

    pub async fn get_latest(&self) -> Result<EntrySummary, ClientError> {
        let url = format!("{}/history/latest", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ClientError::Service("no entries".to_string()));
        }

        resp.json().await.map_err(ClientError::Http)
    }

    pub async fn clear_history(&self) -> Result<(), ClientError> {
        let url = format!("{}/history", self.base_url());
        self.client
            .delete(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await?;
        Ok(())
    }

    pub async fn stop_service(&self) -> Result<(), ClientError> {
        let pid = config::read_pid_file()
            .ok_or_else(|| ClientError::Service("no PID file found".to_string()))?;

        if !config::is_pid_alive(pid) {
            return Err(ClientError::Service("service not running".to_string()));
        }

        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = pid;
        }

        if let Ok(dir) = config::runtime_dir() {
            let _ = std::fs::remove_file(dir.join(config::PID_FILENAME));
            let _ = std::fs::remove_file(dir.join(config::PORT_FILENAME));
        }

        Ok(())
    }

    pub fn service_status(&self) -> ServiceStatus {
        match (config::read_pid_file(), config::read_port_file()) {
            (Some(pid), Some(port)) => {
                if config::is_pid_alive(pid) {
                    ServiceStatus::Running { pid, port }
                } else {
                    ServiceStatus::Stopped
                }
            }
            _ => ServiceStatus::Stopped,
        }
    }

    async fn try_connect(&mut self) -> Result<(), ClientError> {
        let port = config::read_port_file()
            .ok_or(ClientError::ServiceNotRunning)?;

        let pid = config::read_pid_file()
            .ok_or(ClientError::ServiceNotRunning)?;

        if !config::is_pid_alive(pid) {
            return Err(ClientError::ServiceNotRunning);
        }

        self.base_url = Some(format!("http://127.0.0.1:{port}"));
        self.health_check().await
    }

    async fn health_check(&self) -> Result<(), ClientError> {
        let url = format!("{}/health", self.base_url());
        let resp = self
            .client
            .get(&url)
            .header(HEADER_CLIPPER, "1")
            .send()
            .await
            .map_err(|e| ClientError::Connection(e.to_string()))?;

        let has_fingerprint = resp
            .headers()
            .get(HEADER_CLIPPER)
            .is_some_and(|v| v == "1");

        if !has_fingerprint {
            return Err(ClientError::ServiceNotRunning);
        }

        resp.error_for_status_ref()
            .map_err(ClientError::Http)?;

        Ok(())
    }

    fn spawn_service(&self) -> Result<(), ClientError> {
        let port = config::configured_port();

        let binary = find_clipper_binary();

        std::process::Command::new(&binary)
            .arg("--port")
            .arg(port.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                ClientError::Service(format!("Failed to start clipper ({binary}): {e}"))
            })?;

        Ok(())
    }

    async fn poll_until_ready(&mut self) -> Result<(), ClientError> {
        let start = Instant::now();
        let mut delay = Duration::from_millis(50);
        let max_delay = Duration::from_millis(500);
        let timeout = Duration::from_secs(5);

        while start.elapsed() < timeout {
            tokio::time::sleep(delay).await;
            delay = delay.saturating_mul(2).min(max_delay);

            if let Some(port) = config::read_port_file() {
                self.base_url = Some(format!("http://127.0.0.1:{port}"));
                if self.health_check().await.is_ok() {
                    return Ok(());
                }
            }
        }

        Err(ClientError::ServiceNotRunning)
    }

    fn base_url(&self) -> &str {
        self.base_url.as_deref().unwrap_or("")
    }
}

fn find_clipper_binary() -> String {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let clipper = dir.join("clipper");
        if clipper.exists() {
            return clipper.to_string_lossy().into_owned();
        }
    }
    "clipper".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let _client = ClipperClient::new();
    }

    #[test]
    fn test_service_status_stopped() {
        let client = ClipperClient::new();
        assert!(matches!(client.service_status(), ServiceStatus::Stopped));
    }

    #[test]
    fn test_entry_summary_serialization() {
        let entry = EntrySummary {
            id: "a1b2c3d4".to_string(),
            timestamp: "2026-05-02T14:32:01Z".to_string(),
            content_type: "text".to_string(),
            preview: "Hello world".to_string(),
            size_bytes: 42,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: EntrySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.preview, deserialized.preview);
    }

    #[test]
    fn test_find_clipper_binary_returns_string() {
        let binary = find_clipper_binary();
        assert!(!binary.is_empty());
    }
}
