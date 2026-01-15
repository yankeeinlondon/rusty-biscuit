use serde::{Deserialize, Serialize};

/// SIMD instruction set capabilities detected on the CPU.
///
/// Contains boolean flags indicating which SIMD (Single Instruction, Multiple Data)
/// instruction sets are supported by the current processor. This information is
/// useful for determining optimal code paths for vectorized operations.
///
/// ## Examples
///
/// ```
/// use sniff_lib::hardware::SimdCapabilities;
///
/// let caps = SimdCapabilities::detect();
/// if caps.avx2 {
///     println!("AVX2 is supported - can use 256-bit vector operations");
/// }
/// ```
///
/// ## Notes
///
/// - On x86_64, detection uses the `is_x86_feature_detected!` macro
/// - On aarch64, detection uses the `is_aarch64_feature_detected!` macro
/// - On unsupported architectures, all fields default to `false`
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimdCapabilities {
    // x86_64 capabilities
    /// SSE (Streaming SIMD Extensions) - 128-bit registers
    pub sse: bool,
    /// SSE2 - 128-bit integer operations
    pub sse2: bool,
    /// SSE3 - horizontal operations
    pub sse3: bool,
    /// SSSE3 (Supplemental SSE3) - shuffle and alignment
    pub ssse3: bool,
    /// SSE4.1 - blend, dot product, streaming loads
    pub sse4_1: bool,
    /// SSE4.2 - string and text processing
    pub sse4_2: bool,
    /// AVX (Advanced Vector Extensions) - 256-bit registers
    pub avx: bool,
    /// AVX2 - 256-bit integer operations
    pub avx2: bool,
    /// AVX-512 Foundation - 512-bit registers
    pub avx512f: bool,
    /// AVX-512 Vector Length extensions
    pub avx512vl: bool,
    /// AVX-512 Byte and Word instructions
    pub avx512bw: bool,
    /// FMA (Fused Multiply-Add) - combined multiply-add operations
    pub fma: bool,

    // aarch64 capabilities
    /// NEON (ARM Advanced SIMD) - 128-bit registers on ARM
    pub neon: bool,
}

impl SimdCapabilities {
    /// Detects SIMD capabilities for the current CPU.
    ///
    /// Uses architecture-specific intrinsics to query the processor for
    /// supported instruction sets. On unsupported architectures, returns
    /// a default instance with all capabilities set to `false`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::hardware::SimdCapabilities;
    ///
    /// let caps = SimdCapabilities::detect();
    /// println!("SSE2: {}, AVX: {}, AVX2: {}", caps.sse2, caps.avx, caps.avx2);
    /// ```
    #[must_use]
    pub fn detect() -> Self {
        let mut caps = Self::default();

        #[cfg(target_arch = "x86_64")]
        {
            caps.sse = std::arch::is_x86_feature_detected!("sse");
            caps.sse2 = std::arch::is_x86_feature_detected!("sse2");
            caps.sse3 = std::arch::is_x86_feature_detected!("sse3");
            caps.ssse3 = std::arch::is_x86_feature_detected!("ssse3");
            caps.sse4_1 = std::arch::is_x86_feature_detected!("sse4.1");
            caps.sse4_2 = std::arch::is_x86_feature_detected!("sse4.2");
            caps.avx = std::arch::is_x86_feature_detected!("avx");
            caps.avx2 = std::arch::is_x86_feature_detected!("avx2");
            caps.avx512f = std::arch::is_x86_feature_detected!("avx512f");
            caps.avx512vl = std::arch::is_x86_feature_detected!("avx512vl");
            caps.avx512bw = std::arch::is_x86_feature_detected!("avx512bw");
            caps.fma = std::arch::is_x86_feature_detected!("fma");
        }

        #[cfg(target_arch = "aarch64")]
        {
            caps.neon = std::arch::is_aarch64_feature_detected!("neon");
        }

        caps
    }

    /// Returns the highest supported AVX level.
    ///
    /// ## Returns
    ///
    /// - `Some("avx512")` if AVX-512 Foundation is supported
    /// - `Some("avx2")` if AVX2 is supported
    /// - `Some("avx")` if AVX is supported
    /// - `None` if no AVX support
    #[must_use]
    pub fn avx_level(&self) -> Option<&'static str> {
        if self.avx512f {
            Some("avx512")
        } else if self.avx2 {
            Some("avx2")
        } else if self.avx {
            Some("avx")
        } else {
            None
        }
    }

    /// Returns the highest supported SSE level.
    ///
    /// ## Returns
    ///
    /// - `Some("sse4.2")` if SSE4.2 is supported
    /// - `Some("sse4.1")` if SSE4.1 is supported
    /// - `Some("ssse3")` if SSSE3 is supported
    /// - `Some("sse3")` if SSE3 is supported
    /// - `Some("sse2")` if SSE2 is supported
    /// - `Some("sse")` if SSE is supported
    /// - `None` if no SSE support
    #[must_use]
    pub fn sse_level(&self) -> Option<&'static str> {
        if self.sse4_2 {
            Some("sse4.2")
        } else if self.sse4_1 {
            Some("sse4.1")
        } else if self.ssse3 {
            Some("ssse3")
        } else if self.sse3 {
            Some("sse3")
        } else if self.sse2 {
            Some("sse2")
        } else if self.sse {
            Some("sse")
        } else {
            None
        }
    }
}

/// Detects SIMD capabilities for the current CPU.
///
/// This is a convenience function that calls [`SimdCapabilities::detect()`].
///
/// ## Examples
///
/// ```
/// use sniff_lib::hardware::detect_simd;
///
/// let caps = detect_simd();
/// if caps.avx2 {
///     println!("AVX2 supported!");
/// }
/// ```
#[must_use]
pub fn detect_simd() -> SimdCapabilities {
    SimdCapabilities::detect()
}

/// CPU information.
///
/// Contains details about the processor including brand, architecture,
/// logical cores, physical cores when available, and SIMD capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU brand string (e.g., "Intel(R) Core(TM) i7-9750H")
    pub brand: String,
    /// CPU architecture (e.g., "x86_64", "aarch64", "arm64")
    pub arch: String,
    /// Number of logical CPU cores (includes hyperthreading)
    pub logical_cores: usize,
    /// Number of physical CPU cores (None if unavailable)
    pub physical_cores: Option<usize>,
    /// SIMD instruction set capabilities
    pub simd: SimdCapabilities,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_simd_returns_valid_capabilities() {
        let caps = detect_simd();
        // On x86_64, at least SSE2 should be supported (required by the architecture)
        #[cfg(target_arch = "x86_64")]
        {
            assert!(caps.sse);
            assert!(caps.sse2);
        }
        // On aarch64, NEON is typically available
        #[cfg(target_arch = "aarch64")]
        {
            assert!(caps.neon);
        }
    }

    #[test]
    fn test_simd_capabilities_default() {
        let caps = SimdCapabilities::default();
        assert!(!caps.sse);
        assert!(!caps.avx);
        assert!(!caps.neon);
    }

    #[test]
    fn test_simd_capabilities_serialization() {
        let caps = detect_simd();
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: SimdCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps, deserialized);
    }

    #[test]
    fn test_avx_level() {
        let mut caps = SimdCapabilities::default();
        assert_eq!(caps.avx_level(), None);

        caps.avx = true;
        assert_eq!(caps.avx_level(), Some("avx"));

        caps.avx2 = true;
        assert_eq!(caps.avx_level(), Some("avx2"));

        caps.avx512f = true;
        assert_eq!(caps.avx_level(), Some("avx512"));
    }

    #[test]
    fn test_sse_level() {
        let mut caps = SimdCapabilities::default();
        assert_eq!(caps.sse_level(), None);

        caps.sse = true;
        assert_eq!(caps.sse_level(), Some("sse"));

        caps.sse2 = true;
        assert_eq!(caps.sse_level(), Some("sse2"));

        caps.sse3 = true;
        assert_eq!(caps.sse_level(), Some("sse3"));

        caps.ssse3 = true;
        assert_eq!(caps.sse_level(), Some("ssse3"));

        caps.sse4_1 = true;
        assert_eq!(caps.sse_level(), Some("sse4.1"));

        caps.sse4_2 = true;
        assert_eq!(caps.sse_level(), Some("sse4.2"));
    }
}
