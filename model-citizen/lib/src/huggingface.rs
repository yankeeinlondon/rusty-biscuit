//! HuggingFace Hub integration for model search and download.
//!
//! This module provides functionality to search for models on HuggingFace
//! and download GGUF variants with progress tracking.

use crate::gguf::quantization_from_filename;
use crate::{ModelCitizenError, QuantizationType};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DOWNLOAD_RETRY_WINDOW: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
enum DownloadAttemptError {
    Retryable(ModelCitizenError),
    Fatal(ModelCitizenError),
}

/// A GGUF variant available for download from HuggingFace.
#[derive(Debug, Clone)]
pub struct GgufVariant {
    /// Filename of the GGUF file.
    pub filename: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Detected quantization type.
    pub quantization: QuantizationType,
    /// Download URL.
    pub download_url: String,
}

impl GgufVariant {
    /// Returns the size formatted as a human-readable string.
    #[must_use]
    pub fn size_display(&self) -> String {
        const GB: u64 = 1024 * 1024 * 1024;
        const MB: u64 = 1024 * 1024;

        if self.size_bytes >= GB {
            format!("{:.1} GB", self.size_bytes as f64 / GB as f64)
        } else if self.size_bytes >= MB {
            format!("{:.1} MB", self.size_bytes as f64 / MB as f64)
        } else {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        }
    }

    /// Returns an estimated RAM requirement based on quantization.
    ///
    /// This is a rough estimate - actual RAM usage depends on context length
    /// and other factors.
    #[must_use]
    pub fn estimated_ram(&self) -> String {
        // Rough multiplier: loaded model is ~1.1-1.2x file size for inference
        let ram_bytes = (self.size_bytes as f64 * 1.15) as u64;
        const GB: u64 = 1024 * 1024 * 1024;

        format!("{:.1} GB", ram_bytes as f64 / GB as f64)
    }
}

/// Result of a HuggingFace model search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Repository ID (e.g., "TheBloke/Llama-2-7B-GGUF").
    pub repo_id: String,
    /// Author/organization name.
    pub author: Option<String>,
    /// Number of downloads.
    pub downloads: u64,
    /// Number of likes.
    pub likes: u64,
    /// Available GGUF variant count.
    pub variant_count: usize,
    /// ISO 8601 creation timestamp (e.g., "2024-01-15T10:30:00.000Z").
    pub created_at: Option<String>,
    /// ISO 8601 last-modified timestamp.
    pub last_modified: Option<String>,
    /// Tags from the HuggingFace API (format indicators, task types, etc.).
    pub tags: Vec<String>,
    /// Pipeline tag indicating the primary task (e.g., "text-generation", "image-text-to-text").
    pub pipeline_tag: Option<String>,
}

impl SearchResult {
    /// Whether the repo contains GGUF files (based on tags).
    #[must_use]
    pub fn has_gguf(&self) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case("gguf"))
    }

    /// Whether the repo contains SafeTensors files (based on tags).
    #[must_use]
    pub fn has_safetensors(&self) -> bool {
        self.tags.iter().any(|t| t == "safetensors")
    }
}

/// Sort order for HuggingFace model search results.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by download count (most downloaded first).
    #[default]
    Downloads,
    /// Sort by like count (most liked first).
    Likes,
    /// Sort by trending score.
    Trending,
    /// Sort by creation date (newest first).
    Created,
    /// Sort by last modified date (most recently updated first).
    Modified,
}

impl SortOrder {
    /// Returns the API query parameter value for this sort order.
    #[must_use]
    pub fn as_api_param(self) -> &'static str {
        match self {
            Self::Downloads => "downloads",
            Self::Likes => "likes",
            Self::Trending => "trendingScore",
            Self::Created => "createdAt",
            Self::Modified => "lastModified",
        }
    }

    /// Returns a human-readable label for display.
    #[must_use]
    pub fn display_label(self) -> &'static str {
        match self {
            Self::Downloads => "downloads",
            Self::Likes => "likes",
            Self::Trending => "trending score",
            Self::Created => "creation date",
            Self::Modified => "last modified",
        }
    }
}

/// Client for HuggingFace Hub operations.
///
/// Provides search and download functionality for GGUF models.
pub struct HuggingFaceClient {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Base URL for API.
    base_url: String,
    /// Optional API token for authenticated requests.
    token: Option<String>,
}

impl Default for HuggingFaceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HuggingFaceClient {
    /// Creates a new HuggingFace client.
    ///
    /// Attempts to load API token from environment variables:
    /// - `HF_TOKEN`
    /// - `HUGGING_FACE_API_KEY`
    /// - `HF_API_KEY`
    #[must_use]
    pub fn new() -> Self {
        let token = std::env::var("HF_TOKEN")
            .or_else(|_| std::env::var("HUGGING_FACE_API_KEY"))
            .or_else(|_| std::env::var("HF_API_KEY"))
            .ok();

        Self {
            client: reqwest::Client::new(),
            base_url: "https://huggingface.co".to_string(),
            token,
        }
    }

    /// Creates a client with a custom token.
    #[must_use]
    pub fn with_token(token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://huggingface.co".to_string(),
            token: Some(token.into()),
        }
    }

    /// Searches for models on HuggingFace.
    ///
    /// When `query` is `None`, returns models sorted by `sort` with no text filter.
    /// When a query is provided, it is passed directly to the API as-is.
    ///
    /// ## Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn search_models(
        &self,
        query: Option<&str>,
        limit: usize,
        sort: SortOrder,
    ) -> Result<Vec<SearchResult>, ModelCitizenError> {
        self.search_models_with_filter(query, limit, sort, None).await
    }

    /// Searches for GGUF models on HuggingFace.
    ///
    /// Uses server-side GGUF filtering (`filter=gguf`) so the results are
    /// immediately suitable for GGUF download flows.
    ///
    /// ## Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn search_gguf_models(
        &self,
        query: Option<&str>,
        limit: usize,
        sort: SortOrder,
    ) -> Result<Vec<SearchResult>, ModelCitizenError> {
        self.search_models_with_filter(query, limit, sort, Some("gguf"))
            .await
    }

    async fn search_models_with_filter(
        &self,
        query: Option<&str>,
        limit: usize,
        sort: SortOrder,
        filter: Option<&str>,
    ) -> Result<Vec<SearchResult>, ModelCitizenError> {
        let mut url = format!(
            "{}/api/models?limit={}&sort={}&full=false",
            self.base_url,
            limit,
            sort.as_api_param()
        );

        if let Some(q) = query {
            url.push_str(&format!("&search={}", urlencoding::encode(q)));
        }
        if let Some(filter) = filter {
            url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
        }

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(ModelCitizenError::network(format!(
                "HuggingFace API error: {}",
                response.status()
            )));
        }

        let models: Vec<serde_json::Value> = response.json().await?;

        let results: Vec<SearchResult> = models
            .into_iter()
            .filter_map(|m| {
                let repo_id = m.get("modelId")?.as_str()?.to_string();
                let author = m.get("author").and_then(|a| a.as_str()).map(String::from);
                let downloads = m.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0);
                let likes = m.get("likes").and_then(|l| l.as_u64()).unwrap_or(0);
                let created_at = m
                    .get("createdAt")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let last_modified = m
                    .get("lastModified")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let tags = m
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let pipeline_tag = m
                    .get("pipeline_tag")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Some(SearchResult {
                    repo_id,
                    author,
                    downloads,
                    likes,
                    variant_count: 0, // Will be populated when listing variants
                    created_at,
                    last_modified,
                    tags,
                    pipeline_tag,
                })
            })
            .collect();

        Ok(results)
    }

    /// Lists available GGUF variants for a repository.
    ///
    /// ## Arguments
    ///
    /// * `repo_id` - Repository ID (e.g., "TheBloke/Llama-2-7B-GGUF")
    ///
    /// ## Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn list_variants(
        &self,
        repo_id: &str,
    ) -> Result<Vec<GgufVariant>, ModelCitizenError> {
        let url = format!("{}/api/models/{}/tree/main", self.base_url, repo_id);

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(ModelCitizenError::network(format!(
                "HuggingFace API error: {}",
                response.status()
            )));
        }

        let files: Vec<serde_json::Value> = response.json().await?;

        let variants: Vec<GgufVariant> = files
            .into_iter()
            .filter_map(|f| {
                let filename = f.get("path")?.as_str()?.to_string();

                // Only include GGUF files
                if !filename.to_lowercase().ends_with(".gguf") {
                    return None;
                }

                let size_bytes = f.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let quantization = quantization_from_filename(Path::new(&filename));

                let download_url = format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    repo_id, filename
                );

                Some(GgufVariant {
                    filename,
                    size_bytes,
                    quantization,
                    download_url,
                })
            })
            .collect();

        Ok(variants)
    }

    /// Downloads a GGUF file with progress callback.
    ///
    /// ## Arguments
    ///
    /// * `repo_id` - Repository ID
    /// * `filename` - GGUF filename to download
    /// * `dest_dir` - Destination directory
    /// * `progress` - Progress callback receiving (downloaded_bytes, total_bytes)
    ///
    /// ## Errors
    ///
    /// Returns an error if the download fails.
    pub async fn download<F>(
        &self,
        repo_id: &str,
        filename: &str,
        dest_dir: &Path,
        mut progress: F,
    ) -> Result<PathBuf, ModelCitizenError>
    where
        F: FnMut(u64, u64),
    {
        let started_at = Instant::now();
        let mut attempt: u32 = 0;

        loop {
            match self
                .download_once(repo_id, filename, dest_dir, &mut progress)
                .await
            {
                Ok(path) => return Ok(path),
                Err(DownloadAttemptError::Fatal(err)) => return Err(err),
                Err(DownloadAttemptError::Retryable(err)) => {
                    let elapsed = started_at.elapsed();
                    if elapsed >= DOWNLOAD_RETRY_WINDOW {
                        return Err(ModelCitizenError::network(format!(
                            "{err} (retry window of {}s exhausted)",
                            DOWNLOAD_RETRY_WINDOW.as_secs()
                        )));
                    }

                    attempt = attempt.saturating_add(1);
                    let delay = Self::retry_delay(attempt);
                    let remaining = DOWNLOAD_RETRY_WINDOW.saturating_sub(elapsed);
                    tokio::time::sleep(delay.min(remaining)).await;
                }
            }
        }
    }

    async fn download_once<F>(
        &self,
        repo_id: &str,
        filename: &str,
        dest_dir: &Path,
        progress: &mut F,
    ) -> Result<PathBuf, DownloadAttemptError>
    where
        F: FnMut(u64, u64),
    {
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, filename
        );

        let dest_path = Self::destination_path(dest_dir, filename);
        let tmp_path = Self::temp_download_path(dest_dir, filename);
        let mut resume_from = std::fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0);

        if let Some(parent) = tmp_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(ModelCitizenError::from)
                .map_err(DownloadAttemptError::Fatal)?;
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(ModelCitizenError::from)
                .map_err(DownloadAttemptError::Fatal)?;
        }

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }

        let response = request.send().await.map_err(Self::classify_reqwest_error)?;
        let status = response.status();

        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && resume_from > 0 {
            // The remote file may already be complete locally, or the partial is invalid.
            if let Some(total_size) = Self::content_range_total(response.headers())
                && total_size == resume_from
            {
                std::fs::rename(&tmp_path, &dest_path)
                    .map_err(ModelCitizenError::from)
                    .map_err(DownloadAttemptError::Fatal)?;
                progress(total_size, total_size);
                return Ok(dest_path);
            }

            let _ = std::fs::remove_file(&tmp_path);
            return Err(DownloadAttemptError::Retryable(ModelCitizenError::network(
                "Partial download out of sync; restarting from scratch",
            )));
        }

        if !status.is_success() {
            let err = ModelCitizenError::network(format!(
                "Download failed: {}",
                status
            ));
            return if Self::is_retryable_status(status) {
                Err(DownloadAttemptError::Retryable(err))
            } else {
                Err(DownloadAttemptError::Fatal(err))
            };
        }

        let append_mode = resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
        if !append_mode {
            resume_from = 0;
        }

        let remaining = response.content_length();
        let total_size = if append_mode {
            Self::content_range_total(response.headers())
                .or_else(|| remaining.map(|n| resume_from.saturating_add(n)))
                .unwrap_or_else(|| resume_from.max(1))
        } else {
            remaining.unwrap_or(1)
        };

        // Download to temp file and then atomically rename.
        let mut file = if append_mode {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&tmp_path)
                .map_err(ModelCitizenError::from)
                .map_err(DownloadAttemptError::Fatal)?
        } else {
            std::fs::File::create(&tmp_path)
                .map_err(ModelCitizenError::from)
                .map_err(DownloadAttemptError::Fatal)?
        };
        let mut downloaded: u64 = resume_from;

        use futures::StreamExt;
        use std::io::Write;

        progress(downloaded, total_size);

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(Self::classify_reqwest_error)?;
            file.write_all(&chunk)
                .map_err(ModelCitizenError::from)
                .map_err(DownloadAttemptError::Fatal)?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total_size);
        }

        std::fs::rename(&tmp_path, &dest_path)
            .map_err(ModelCitizenError::from)
            .map_err(DownloadAttemptError::Fatal)?;

        Ok(dest_path)
    }

    /// Returns the temporary download path (`<filename>.tmp`) used for resumable downloads.
    #[must_use]
    pub fn temp_download_path(dest_dir: &Path, filename: &str) -> PathBuf {
        dest_dir.join(format!("{filename}.tmp"))
    }

    /// Returns the final destination path for a downloaded file.
    #[must_use]
    pub fn destination_path(dest_dir: &Path, filename: &str) -> PathBuf {
        dest_dir.join(filename)
    }

    /// Returns the current size of a partial download, if one exists.
    ///
    /// ## Errors
    ///
    /// Returns an error if metadata cannot be read for reasons other than file-not-found.
    pub fn partial_download_size(
        dest_dir: &Path,
        filename: &str,
    ) -> Result<Option<u64>, ModelCitizenError> {
        let tmp_path = Self::temp_download_path(dest_dir, filename);
        match std::fs::metadata(tmp_path) {
            Ok(meta) => Ok(Some(meta.len())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(ModelCitizenError::from(err)),
        }
    }

    fn retry_delay(attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(4);
        Duration::from_secs((1_u64 << shift).min(15))
    }

    fn is_retryable_status(status: reqwest::StatusCode) -> bool {
        status.is_server_error()
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
            || status == reqwest::StatusCode::REQUEST_TIMEOUT
    }

    fn is_retryable_reqwest_error(err: &reqwest::Error) -> bool {
        err.is_timeout() || err.is_connect() || err.is_request() || err.is_body()
    }

    fn classify_reqwest_error(err: reqwest::Error) -> DownloadAttemptError {
        let retryable = Self::is_retryable_reqwest_error(&err);
        let mapped = ModelCitizenError::network(err.to_string());
        if retryable {
            DownloadAttemptError::Retryable(mapped)
        } else {
            DownloadAttemptError::Fatal(mapped)
        }
    }

    fn content_range_total(headers: &reqwest::header::HeaderMap) -> Option<u64> {
        let value = headers.get(reqwest::header::CONTENT_RANGE)?;
        let as_str = value.to_str().ok()?;
        let (_, total) = as_str.rsplit_once('/')?;
        if total == "*" {
            return None;
        }
        total.parse::<u64>().ok()
    }

    /// Cleans up incomplete download files.
    ///
    /// Call this on Ctrl+C or error to remove .tmp files.
    pub fn cleanup_temp_files(dir: &Path) -> Result<usize, ModelCitizenError> {
        let mut count = 0;

        let entries = std::fs::read_dir(dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "tmp")
                && std::fs::remove_file(&path).is_ok()
            {
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gguf_variant_size_display() {
        let variant = GgufVariant {
            filename: "test.gguf".to_string(),
            size_bytes: 4_500_000_000,
            quantization: QuantizationType::Q4Km,
            download_url: String::new(),
        };
        assert_eq!(variant.size_display(), "4.2 GB");

        let small = GgufVariant {
            filename: "test.gguf".to_string(),
            size_bytes: 500_000_000,
            quantization: QuantizationType::Q4Km,
            download_url: String::new(),
        };
        assert_eq!(small.size_display(), "476.8 MB");
    }

    #[test]
    fn gguf_variant_estimated_ram() {
        let variant = GgufVariant {
            filename: "test.gguf".to_string(),
            size_bytes: 4_000_000_000,
            quantization: QuantizationType::Q4Km,
            download_url: String::new(),
        };
        // 4GB * 1.15 = 4.6GB
        assert!(variant.estimated_ram().contains("4."));
    }

    #[test]
    fn client_new_without_token() {
        // Clear env vars for test (unsafe in Rust 2024)
        let client = HuggingFaceClient::new();
        assert_eq!(client.base_url, "https://huggingface.co");
    }

    #[test]
    fn client_with_token() {
        let client = HuggingFaceClient::with_token("test_token");
        assert_eq!(client.token, Some("test_token".to_string()));
    }

    #[test]
    fn search_result_fields() {
        let result = SearchResult {
            repo_id: "TheBloke/Llama-2-7B-GGUF".to_string(),
            author: Some("TheBloke".to_string()),
            downloads: 100_000,
            likes: 500,
            variant_count: 10,
            created_at: Some("2024-01-15T10:30:00.000Z".to_string()),
            last_modified: Some("2024-06-20T14:00:00.000Z".to_string()),
            tags: vec!["gguf".to_string(), "text-generation".to_string()],
            pipeline_tag: Some("text-generation".to_string()),
        };

        assert_eq!(result.repo_id, "TheBloke/Llama-2-7B-GGUF");
        assert_eq!(result.author, Some("TheBloke".to_string()));
        assert!(result.has_gguf());
        assert!(!result.has_safetensors());
    }

    #[test]
    fn sort_order_default_is_downloads() {
        assert_eq!(SortOrder::default(), SortOrder::Downloads);
    }

    #[test]
    fn sort_order_api_params() {
        assert_eq!(SortOrder::Downloads.as_api_param(), "downloads");
        assert_eq!(SortOrder::Likes.as_api_param(), "likes");
        assert_eq!(SortOrder::Trending.as_api_param(), "trendingScore");
        assert_eq!(SortOrder::Created.as_api_param(), "createdAt");
        assert_eq!(SortOrder::Modified.as_api_param(), "lastModified");
    }

    #[test]
    fn sort_order_display_labels() {
        assert_eq!(SortOrder::Downloads.display_label(), "downloads");
        assert_eq!(SortOrder::Likes.display_label(), "likes");
        assert_eq!(SortOrder::Trending.display_label(), "trending score");
        assert_eq!(SortOrder::Created.display_label(), "creation date");
        assert_eq!(SortOrder::Modified.display_label(), "last modified");
    }

    #[test]
    fn cleanup_temp_files_returns_zero_for_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let count = HuggingFaceClient::cleanup_temp_files(temp_dir.path()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn cleanup_temp_files_removes_tmp_files() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create some temp files
        std::fs::write(temp_dir.path().join("model.gguf.tmp"), b"temp").unwrap();
        std::fs::write(temp_dir.path().join("other.gguf.tmp"), b"temp").unwrap();
        std::fs::write(temp_dir.path().join("keep.gguf"), b"keep").unwrap();

        let count = HuggingFaceClient::cleanup_temp_files(temp_dir.path()).unwrap();
        assert_eq!(count, 2);

        // Verify .tmp files are gone
        assert!(!temp_dir.path().join("model.gguf.tmp").exists());
        assert!(!temp_dir.path().join("other.gguf.tmp").exists());
        // Verify .gguf file is kept
        assert!(temp_dir.path().join("keep.gguf").exists());
    }

    #[test]
    fn retryable_status_detection() {
        assert!(HuggingFaceClient::is_retryable_status(
            reqwest::StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(HuggingFaceClient::is_retryable_status(
            reqwest::StatusCode::BAD_GATEWAY
        ));
        assert!(!HuggingFaceClient::is_retryable_status(
            reqwest::StatusCode::NOT_FOUND
        ));
    }

    #[test]
    fn retry_delay_caps_at_fifteen_seconds() {
        assert_eq!(HuggingFaceClient::retry_delay(1), Duration::from_secs(1));
        assert_eq!(HuggingFaceClient::retry_delay(2), Duration::from_secs(2));
        assert_eq!(HuggingFaceClient::retry_delay(5), Duration::from_secs(15));
        assert_eq!(HuggingFaceClient::retry_delay(8), Duration::from_secs(15));
    }

    #[test]
    fn temp_download_path_appends_tmp_suffix() {
        let path = HuggingFaceClient::temp_download_path(Path::new("/tmp/models"), "foo.gguf");
        assert_eq!(path, Path::new("/tmp/models/foo.gguf.tmp"));
    }

    #[test]
    fn partial_download_size_reports_existing_tmp_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let tmp_path = HuggingFaceClient::temp_download_path(temp_dir.path(), "model.gguf");
        std::fs::write(&tmp_path, [0_u8; 16]).unwrap();

        let size = HuggingFaceClient::partial_download_size(temp_dir.path(), "model.gguf").unwrap();
        assert_eq!(size, Some(16));
    }

    #[test]
    fn partial_download_size_returns_none_when_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let size = HuggingFaceClient::partial_download_size(temp_dir.path(), "missing.gguf").unwrap();
        assert_eq!(size, None);
    }
}
