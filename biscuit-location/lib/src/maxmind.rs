use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tempfile::NamedTempFile;

use crate::error::LocationError;

const MAXMIND_DOWNLOAD_URL: &str =
    "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&suffix=tar.gz";
const MAXMIND_LICENSE_KEY_ENV: &str = "MAXMIND_LICENSE_KEY";
const MAXMIND_ACCOUNT_ID_ENV: &str = "MAXMIND_ACCOUNT_ID";
const MAXMIND_FILENAME: &str = "GeoLite2-City.mmdb";

/// Resolve credentials from environment variables.
///
/// ## Returns
///
/// `Some((account_id, license_key))` if the license key env var is set,
/// `None` otherwise. The account ID defaults to `"1"` when the env var is
/// not set (MaxMind accepts any non-empty value for the free tier).
pub fn resolve_credentials() -> Option<(String, String)> {
    let license_key = std::env::var(MAXMIND_LICENSE_KEY_ENV).ok()?;
    if license_key.is_empty() {
        return None;
    }
    let account_id = std::env::var(MAXMIND_ACCOUNT_ID_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1".to_string());
    Some((account_id, license_key))
}

/// Download the GeoLite2-City database and place it at the default OS data path.
///
/// Uses atomic file replacement (write to temp, then rename) so that
/// concurrent readers never see a partially-written database.
///
/// ## Errors
///
/// Returns `MissingLicenseKey` if `MAXMIND_LICENSE_KEY` is not set.
/// Returns `DatabaseDownload` for network, extraction, or I/O failures.
pub async fn download_database() -> crate::Result<PathBuf> {
    let (account_id, license_key) =
        resolve_credentials().ok_or(LocationError::MissingLicenseKey)?;

    let dest_dir = dirs::data_dir()
        .map(|d| d.join("biscuit-location"))
        .ok_or_else(|| {
            LocationError::DatabaseDownload("cannot determine OS data directory".into())
        })?;

    download_database_to(&account_id, &license_key, &dest_dir).await
}

/// Download the GeoLite2-City database to a specific directory.
///
/// ## Errors
///
/// Returns `DatabaseDownload` for network, extraction, or I/O failures.
pub async fn download_database_to(
    account_id: &str,
    license_key: &str,
    dest_dir: &Path,
) -> crate::Result<PathBuf> {
    std::fs::create_dir_all(dest_dir).map_err(|e| {
        LocationError::DatabaseDownload(format!(
            "failed to create directory {}: {e}",
            dest_dir.display()
        ))
    })?;

    let url = format!(
        "{url}&account_id={account_id}&license_key={license_key}",
        url = MAXMIND_DOWNLOAD_URL
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| LocationError::DatabaseDownload(format!("HTTP request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(LocationError::DatabaseDownload(format!(
            "MaxMind returned HTTP {status}: {body}"
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| LocationError::DatabaseDownload(format!("failed to read response: {e}")))?;

    let mmdb_bytes = extract_mmdb(&bytes)?;

    let dest_path = dest_dir.join(MAXMIND_FILENAME);
    write_atomic(&dest_path, &mmdb_bytes)?;

    Ok(dest_path)
}

/// Extract the first `.mmdb` file from a `.tar.gz` archive.
fn extract_mmdb(tar_gz_bytes: &[u8]) -> crate::Result<Vec<u8>> {
    let gz = GzDecoder::new(tar_gz_bytes);
    let mut archive = tar::Archive::new(gz);

    for entry_result in archive
        .entries()
        .map_err(|e| LocationError::DatabaseDownload(format!("failed to read tar archive: {e}")))?
    {
        let mut entry = entry_result.map_err(|e| {
            LocationError::DatabaseDownload(format!("failed to read tar entry: {e}"))
        })?;

        let path = entry.path().map_err(|e| {
            LocationError::DatabaseDownload(format!("failed to read entry path: {e}"))
        })?;

        if path.extension().is_some_and(|ext| ext == "mmdb") {
            let mut buf = Vec::with_capacity(70 * 1024 * 1024);
            entry.read_to_end(&mut buf).map_err(|e| {
                LocationError::DatabaseDownload(format!("failed to read mmdb data: {e}"))
            })?;
            return Ok(buf);
        }
    }

    Err(LocationError::DatabaseDownload(
        "no .mmdb file found in archive".into(),
    ))
}

/// Write bytes to a file atomically using a temp file + rename.
fn write_atomic(dest: &Path, data: &[u8]) -> crate::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        LocationError::DatabaseDownload(format!("path has no parent: {}", dest.display()))
    })?;

    let mut tmp = NamedTempFile::new_in(parent)
        .map_err(|e| LocationError::DatabaseDownload(format!("failed to create temp file: {e}")))?;

    use std::io::Write;
    tmp.write_all(data)
        .map_err(|e| LocationError::DatabaseDownload(format!("failed to write temp file: {e}")))?;

    tmp.persist(dest).map_err(|e| {
        LocationError::DatabaseDownload(format!("failed to persist database file: {e}"))
    })?;

    Ok(())
}

/// Try to auto-download the database if it's missing and credentials are available.
///
/// Returns `Ok(Some(path))` if the database was downloaded.
/// Returns `Ok(None)` if credentials are not configured (silent no-op).
/// Returns `Err` if credentials exist but the download fails.
pub async fn maybe_download_if_missing(expected_path: &Path) -> crate::Result<Option<PathBuf>> {
    if expected_path.exists() {
        return Ok(None);
    }

    if resolve_credentials().is_none() {
        return Ok(None);
    }

    let downloaded = download_database().await?;
    Ok(Some(downloaded))
}

/// Check whether auto-download is possible (credentials are configured).
pub fn can_auto_download() -> bool {
    resolve_credentials().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_credentials_returns_none_without_env() {
        let prev_key = std::env::var(MAXMIND_LICENSE_KEY_ENV).ok();
        let prev_account = std::env::var(MAXMIND_ACCOUNT_ID_ENV).ok();
        unsafe { std::env::remove_var(MAXMIND_LICENSE_KEY_ENV) };
        unsafe { std::env::remove_var(MAXMIND_ACCOUNT_ID_ENV) };
        assert!(resolve_credentials().is_none());
        if let Some(v) = prev_key {
            unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, v) };
        }
        if let Some(v) = prev_account {
            unsafe { std::env::set_var(MAXMIND_ACCOUNT_ID_ENV, v) };
        }
    }

    #[test]
    fn resolve_credentials_returns_some_with_key_only() {
        let prev_key = std::env::var(MAXMIND_LICENSE_KEY_ENV).ok();
        let prev_account = std::env::var(MAXMIND_ACCOUNT_ID_ENV).ok();
        unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, "test-key") };
        unsafe { std::env::remove_var(MAXMIND_ACCOUNT_ID_ENV) };

        let (account_id, key) = resolve_credentials().unwrap();
        assert_eq!(key, "test-key");
        assert_eq!(account_id, "1");

        match prev_key {
            Some(v) => unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, v) },
            None => unsafe { std::env::remove_var(MAXMIND_LICENSE_KEY_ENV) },
        }
        if let Some(v) = prev_account {
            unsafe { std::env::set_var(MAXMIND_ACCOUNT_ID_ENV, v) };
        }
    }

    #[test]
    fn resolve_credentials_uses_account_id_when_set() {
        let prev_key = std::env::var(MAXMIND_LICENSE_KEY_ENV).ok();
        let prev_account = std::env::var(MAXMIND_ACCOUNT_ID_ENV).ok();
        unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, "test-key") };
        unsafe { std::env::set_var(MAXMIND_ACCOUNT_ID_ENV, "42") };

        let (account_id, key) = resolve_credentials().unwrap();
        assert_eq!(key, "test-key");
        assert_eq!(account_id, "42");

        match prev_key {
            Some(v) => unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, v) },
            None => unsafe { std::env::remove_var(MAXMIND_LICENSE_KEY_ENV) },
        }
        match prev_account {
            Some(v) => unsafe { std::env::set_var(MAXMIND_ACCOUNT_ID_ENV, v) },
            None => unsafe { std::env::remove_var(MAXMIND_ACCOUNT_ID_ENV) },
        }
    }

    #[test]
    fn resolve_credentials_ignores_empty_key() {
        let prev_key = std::env::var(MAXMIND_LICENSE_KEY_ENV).ok();
        unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, "") };
        assert!(resolve_credentials().is_none());
        match prev_key {
            Some(v) => unsafe { std::env::set_var(MAXMIND_LICENSE_KEY_ENV, v) },
            None => unsafe { std::env::remove_var(MAXMIND_LICENSE_KEY_ENV) },
        }
    }

    #[test]
    fn extract_mmdb_rejects_empty_archive() {
        let result = extract_mmdb(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::DatabaseDownload(_)));
    }

    #[test]
    fn extract_mmdb_rejects_archive_without_mmdb() {
        let mut tar_gz = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut tar_gz, flate2::Compression::none());
            let mut archive = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header.set_path("readme.txt").unwrap();
            header.set_size(5);
            header.set_cksum();
            archive.append(&header, "hello".as_bytes()).unwrap();
            let gz = archive.into_inner().unwrap();
            gz.finish().unwrap();
        }
        let result = extract_mmdb(&tar_gz);
        assert!(result.is_err());
    }

    #[test]
    fn extract_mmdb_finds_mmdb_in_tar_gz() {
        let mmdb_content = b"fake mmdb data for testing";
        let mut tar_gz = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut tar_gz, flate2::Compression::none());
            let mut archive = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header
                .set_path("GeoLite2-City_20240101/GeoLite2-City.mmdb")
                .unwrap();
            header.set_size(mmdb_content.len() as u64);
            header.set_cksum();
            archive.append(&header, mmdb_content.as_slice()).unwrap();
            let gz = archive.into_inner().unwrap();
            gz.finish().unwrap();
        }
        let result = extract_mmdb(&tar_gz).unwrap();
        assert_eq!(result, mmdb_content);
    }

    #[tokio::test]
    async fn write_atomic_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("test.mmdb");
        write_atomic(&dest, b"hello").unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn download_database_to_rejects_bad_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let result = download_database_to("fake", "fake", dir.path()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::DatabaseDownload(_)));
    }
}
