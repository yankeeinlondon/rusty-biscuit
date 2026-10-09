//! Content fingerprints: the `<scheme>:<hex>` values a `FileChanged` rule
//! stores in its property.
//!
//! A fingerprint is evidence stored in the document, unlike the renewal plan
//! fingerprint, which never is. Its scheme names the comparison algorithm, so
//! a value under a scheme this library does not know is never proof of change
//! or freshness.

use std::fmt;

use crate::diagnostic::DiagnosticCode;
use crate::grammar::RuleError;

/// A fingerprint scheme this library computes. Both hash with BLAKE3 through
/// `biscuit-hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FingerprintScheme {
    /// `blake3-lf`, the default: the bytes after every CRLF pair becomes LF,
    /// so a Windows checkout with `core.autocrlf` fingerprints like macOS and
    /// Linux. A lone CR is data and is kept.
    Blake3Lf,
    /// `blake3`: the raw bytes, for binary files where CRLF is data.
    Blake3,
}

impl FingerprintScheme {
    /// The scheme a first capture writes.
    pub const DEFAULT: Self = Self::Blake3Lf;

    #[allow(missing_docs)]
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blake3Lf => "blake3-lf",
            Self::Blake3 => "blake3",
        }
    }

    /// Parses a scheme name. Names are case-sensitive.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Blake3Lf, Self::Blake3]
            .into_iter()
            .find(|scheme| scheme.as_str() == name)
    }

    /// The fingerprint of `bytes` under this scheme, such as
    /// `blake3-lf:` followed by 64 lowercase hex digits.
    #[must_use]
    pub fn fingerprint(self, bytes: &[u8]) -> String {
        let digest = match self {
            Self::Blake3 => biscuit_hash::blake3_hash_bytes(bytes),
            Self::Blake3Lf => biscuit_hash::blake3_hash_bytes(&crlf_to_lf(bytes)),
        };
        let mut text = String::with_capacity(self.as_str().len() + 1 + 64);
        text.push_str(self.as_str());
        text.push(':');
        for byte in digest {
            text.push_str(&format!("{byte:02x}"));
        }
        text
    }
}

impl fmt::Display for FingerprintScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn crlf_to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(bytes.len());
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            continue;
        }
        normalized.push(*byte);
    }
    normalized
}

/// A stored fingerprint value of the valid shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoredFingerprint<'a> {
    Known(FingerprintScheme),
    /// A well-formed value under a scheme this library does not compute,
    /// such as `sha256:…`.
    Unrecognized(&'a str),
}

/// Reads a stored fingerprint string. The value must match
/// `^[a-z0-9][a-z0-9-]*:[0-9a-f]+$`, and a recognized scheme must carry 64 hex
/// digits. Uppercase hex is rejected rather than folded: renewal writes
/// lowercase, and a folded match would hide a hand-edited typo.
pub(crate) fn classify(text: &str) -> Result<StoredFingerprint<'_>, RuleError> {
    let malformed = || RuleError {
        code: DiagnosticCode::InvalidFingerprint,
        message: format!(
            "`{text}` is not a content fingerprint; expected `<scheme>:<lowercase hex>`, such \
             as `blake3-lf:` followed by 64 hex digits; remove the value and run `policy renew` \
             to capture it"
        ),
    };
    let (scheme, hex) = text.split_once(':').ok_or_else(malformed)?;
    let scheme_ok = scheme
        .bytes()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    let hex_ok = !hex.is_empty() && hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
    if !(scheme_ok && hex_ok) {
        return Err(malformed());
    }
    match FingerprintScheme::parse(scheme) {
        Some(known) if hex.len() == 64 => Ok(StoredFingerprint::Known(known)),
        Some(known) => Err(RuleError {
            code: DiagnosticCode::InvalidFingerprint,
            message: format!(
                "`{text}` has {} hex digits; a `{known}` fingerprint has 64",
                hex.len()
            ),
        }),
        None => Ok(StoredFingerprint::Unrecognized(scheme)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "9f2c41aa9f2c41aa9f2c41aa9f2c41aa9f2c41aa9f2c41aa9f2c41aa9f2c41aa";

    #[test]
    fn blake3_lf_ignores_crlf_and_blake3_does_not() {
        let lf = b"line one\nline two\n";
        let crlf = b"line one\r\nline two\r\n";
        let lone_cr = b"line one\rline two\n";
        let lf_scheme = FingerprintScheme::Blake3Lf;
        assert_eq!(lf_scheme.fingerprint(lf), lf_scheme.fingerprint(crlf));
        assert_ne!(lf_scheme.fingerprint(lf), lf_scheme.fingerprint(lone_cr));
        let raw = FingerprintScheme::Blake3;
        assert_ne!(raw.fingerprint(lf), raw.fingerprint(crlf));
        assert!(lf_scheme.fingerprint(lf).starts_with("blake3-lf:"));
        assert_eq!(raw.fingerprint(lf).len(), "blake3:".len() + 64);
        // `blake3` of LF-only bytes equals `blake3-lf` of the same bytes' digest.
        assert_eq!(raw.fingerprint(lf)[7..], lf_scheme.fingerprint(crlf)[10..]);
    }

    #[test]
    fn crlf_normalization_keeps_lone_carriage_returns() {
        assert_eq!(crlf_to_lf(b"a\r\nb\rc\r\r\n"), b"a\nb\rc\r\n");
        assert_eq!(crlf_to_lf(b"\r"), b"\r");
    }

    #[test]
    fn stored_values_classify_by_shape() {
        assert_eq!(
            classify(&format!("blake3-lf:{HEX}")),
            Ok(StoredFingerprint::Known(FingerprintScheme::Blake3Lf))
        );
        assert_eq!(
            classify(&format!("blake3:{HEX}")),
            Ok(StoredFingerprint::Known(FingerprintScheme::Blake3))
        );
        assert_eq!(classify("sha256:ab"), Ok(StoredFingerprint::Unrecognized("sha256")));
        assert_eq!(classify("x9-1:0"), Ok(StoredFingerprint::Unrecognized("x9-1")));
        for text in [
            "",
            "blake3-lf",
            ":abc",
            "blake3-lf:",
            "blake3-lf:zz",
            "BLAKE3:ab",
            "-x:ab",
            "blake3-lf:9F2C",
            "blake3 :ab",
            "blake3-lf:ab cd",
            "blake3-lf:ab\n",
            "blake3-lf:abc", // recognized scheme, wrong length
            &format!("blake3-lf:{HEX}0"),
        ] {
            let error = classify(text).expect_err(text);
            assert_eq!(error.code, DiagnosticCode::InvalidFingerprint, "{text:?}");
        }
    }
}
