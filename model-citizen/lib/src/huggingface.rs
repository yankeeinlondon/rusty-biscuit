//! HuggingFace Hub integration for model search and download.
//!
//! This module provides functionality to search for GGUF models on HuggingFace
//! and download them with progress tracking.

use crate::gguf::quantization_from_filename;
use crate::{ModelCitizenError, QuantizationType};
use std::path::{Path, PathBuf};

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

    /// Searches for GGUF models on HuggingFace.
    ///
    /// ## Arguments
    ///
    /// * `query` - Search query (e.g., "llama 3 gguf")
    /// * `limit` - Maximum number of results to return
    ///
    /// ## Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn search_models(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, ModelCitizenError> {
        // Add "gguf" to query if not present
        let search_query = if query.to_lowercase().contains("gguf") {
            query.to_string()
        } else {
            format!("{} gguf", query)
        };

        let url = format!(
            "{}/api/models?search={}&limit={}&full=false",
            self.base_url,
            urlencoding::encode(&search_query),
            limit
        );

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

                Some(SearchResult {
                    repo_id,
                    author,
                    downloads,
                    likes,
                    variant_count: 0, // Will be populated when listing variants
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
    pub async fn list_variants(&self, repo_id: &str) -> Result<Vec<GgufVariant>, ModelCitizenError> {
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
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, filename
        );

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(ModelCitizenError::network(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let total_size = response
            .content_length()
            .ok_or_else(|| ModelCitizenError::network("Unknown content length"))?;

        // Create destination path with .tmp suffix
        std::fs::create_dir_all(dest_dir)?;
        let dest_path = dest_dir.join(filename);
        let tmp_path = dest_dir.join(format!("{}.tmp", filename));

        // Download to temp file
        let mut file = std::fs::File::create(&tmp_path)?;
        let mut downloaded: u64 = 0;

        use std::io::Write;
        use futures::StreamExt;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ModelCitizenError::network(e.to_string()))?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total_size);
        }

        // Rename temp file to final destination
        std::fs::rename(&tmp_path, &dest_path)?;

        Ok(dest_path)
    }

    /// Cleans up incomplete download files.
    ///
    /// Call this on Ctrl+C or error to remove .tmp files.
    pub fn cleanup_temp_files(dir: &Path) -> Result<usize, ModelCitizenError> {
        let mut count = 0;

        let entries = std::fs::read_dir(dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "tmp") {
                if std::fs::remove_file(&path).is_ok() {
                    count += 1;
                }
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
        };

        assert_eq!(result.repo_id, "TheBloke/Llama-2-7B-GGUF");
        assert_eq!(result.author, Some("TheBloke".to_string()));
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
}
