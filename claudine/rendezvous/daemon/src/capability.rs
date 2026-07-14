//! Host-capability detection and broadcast.
//!
//! Fills this host's `capability/{node_id}` register (see
//! [`crate::register::RegisterStore`]) with the hardware and identity
//! fields from the host-capability-broadcast spec, using the `sniff`
//! detection library. The register then reaches every mesh peer through
//! normal document sync, giving the future scheduler its placement
//! inputs.
//!
//! Cadence follows the ratified rulings: detection at daemon startup
//! plus an hourly re-check, with all writes going through the store's
//! write-on-change path so an unchanged host never touches the
//! document. `available_storage` is quantized to whole GB with 5%
//! hysteresis so a volatile number cannot churn the register.

use serde_json::{Map as JsonMap, Value as JsonValue, json};
use sniff::hardware::detect_hardware_with_request;
use sniff::os::{OsType, detect_os_with_request};
use sniff::request::{HardwareRequest, OsRequest};

use crate::register::{RegisterError, RegisterStore};

/// Version stamp for the capability field schema. Readers ignore
/// unknown fields and treat missing fields as "capability unknown"
/// (ratified D5), so this only needs to bump on incompatible
/// *reinterpretations* of an existing field.
pub const CAPABILITY_SCHEMA_VERSION: i64 = 1;

/// Detect this host's capability fields. Detection failures degrade to
/// omitting the affected fields (missing = "unknown" per the ratified
/// schema rules), never to an error — a partially-described host is
/// still schedulable by whatever it *did* report.
#[must_use]
pub fn detect_capability_fields(node_id: &str) -> JsonMap<String, JsonValue> {
    let mut fields = JsonMap::new();
    fields.insert("schema_version".into(), json!(CAPABILITY_SCHEMA_VERSION));
    fields.insert("id".into(), json!(node_id));

    match detect_os_with_request(&OsRequest::summary()) {
        Ok(os) => {
            fields.insert("name".into(), json!(os.hostname));
            fields.insert("os".into(), json!(os_slug(os.os_type)));
            fields.insert("os_version".into(), json!(os.version));
        }
        Err(err) => {
            tracing::warn!(
                target: "rendezvous_daemon::capability",
                %err,
                "os detection failed; capability register omits os fields",
            );
        }
    }

    let request = HardwareRequest {
        include_cpu: true,
        include_memory: true,
        include_storage: true,
        include_gpu: true,
        include_audio: false,
    };
    match detect_hardware_with_request(&request) {
        Ok(hw) => {
            fields.insert("arch".into(), json!(hw.cpu.arch));
            fields.insert("cpu_cores".into(), json!(hw.cpu.logical_cores as i64));
            fields.insert("memory".into(), json!(hw.memory.total_bytes as i64));
            let simd = &hw.cpu.simd;
            for (name, enabled) in [
                ("sse", simd.sse),
                ("sse2", simd.sse2),
                ("sse3", simd.sse3),
                ("ssse3", simd.ssse3),
                ("sse4_1", simd.sse4_1),
                ("sse4_2", simd.sse4_2),
                ("avx", simd.avx),
                ("avx2", simd.avx2),
                ("avx512f", simd.avx512f),
                ("avx512vl", simd.avx512vl),
                ("avx512bw", simd.avx512bw),
                ("neon", simd.neon),
            ] {
                fields.insert(name.into(), json!(enabled));
            }
            fields.insert("gpu".into(), json!(gpu_slug(&hw.gpu)));
            if let Some(bytes) = root_available_bytes(&hw.storage) {
                fields.insert("available_storage".into(), json!(quantize_gb(bytes)));
            }
        }
        Err(err) => {
            tracing::warn!(
                target: "rendezvous_daemon::capability",
                %err,
                "hardware detection failed; capability register omits hardware fields",
            );
        }
    }

    fields
}

/// Run one detection pass and write any changes into the local
/// capability register. Returns `true` when the register changed.
pub fn refresh_capabilities(store: &RegisterStore) -> Result<bool, RegisterError> {
    let doc_id = store.local_capability_id();
    let mut fields = detect_capability_fields(doc_id.owner_node_id());
    apply_storage_hysteresis(store, &doc_id, &mut fields)?;
    store.upsert_local_fields(&doc_id, &fields)
}

/// Hysteresis for the one genuinely volatile field: keep the register's
/// current `available_storage` unless the newly detected value moved by
/// more than 5% (and at least 1 GB, which quantization already
/// guarantees). Write-on-change then sees an unchanged value and leaves
/// the document untouched.
fn apply_storage_hysteresis(
    store: &RegisterStore,
    doc_id: &rendezvous_core::DocumentId,
    fields: &mut JsonMap<String, JsonValue>,
) -> Result<(), RegisterError> {
    let Some(new_gb) = fields.get("available_storage").and_then(JsonValue::as_i64) else {
        return Ok(());
    };
    let current_gb = store
        .deep_value(doc_id)?
        .and_then(|v| v.get("available_storage").and_then(JsonValue::as_i64));
    if let Some(current) = current_gb {
        let threshold = (current / 20).max(1);
        if (new_gb - current).abs() < threshold {
            fields.insert("available_storage".into(), json!(current));
        }
    }
    Ok(())
}

/// Spec enum value for the host OS.
fn os_slug(os_type: OsType) -> &'static str {
    match os_type {
        OsType::MacOS => "macOS",
        OsType::Linux => "Linux",
        OsType::Windows => "Windows",
        _ => "other",
    }
}

/// Spec enum value for the GPU: `none`, `metal`, `nvidia`, or `other`.
fn gpu_slug(gpus: &[sniff::hardware::GpuInfo]) -> &'static str {
    if gpus.is_empty() {
        return "none";
    }
    let haystack = |gpu: &sniff::hardware::GpuInfo| {
        format!(
            "{} {} {}",
            gpu.backend,
            gpu.vendor.as_deref().unwrap_or_default(),
            gpu.name
        )
        .to_ascii_lowercase()
    };
    if gpus.iter().any(|g| haystack(g).contains("nvidia")) {
        "nvidia"
    } else if gpus.iter().any(|g| haystack(g).contains("metal")) {
        "metal"
    } else {
        "other"
    }
}

/// Available bytes on the root volume (`/` on Unix, the system drive on
/// Windows), falling back to the largest non-removable volume.
fn root_available_bytes(volumes: &[sniff::hardware::StorageInfo]) -> Option<u64> {
    let root = volumes.iter().find(|v| {
        let mount = v.mount_point.to_string_lossy();
        mount == "/" || mount.eq_ignore_ascii_case("c:\\") || mount.eq_ignore_ascii_case("c:")
    });
    root.or_else(|| {
        volumes
            .iter()
            .filter(|v| !v.is_removable)
            .max_by_key(|v| v.total_bytes)
    })
    .map(|v| v.available_bytes)
}

/// Whole-GB quantization: coarse enough that normal filesystem churn
/// never produces a new value.
fn quantize_gb(bytes: u64) -> i64 {
    (bytes / 1_000_000_000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use rendezvous_core::NodeIdentity;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn quantize_is_whole_gb() {
        assert_eq!(quantize_gb(0), 0);
        assert_eq!(quantize_gb(999_999_999), 0);
        assert_eq!(quantize_gb(1_000_000_000), 1);
        assert_eq!(quantize_gb(512_345_678_901), 512);
    }

    #[test]
    fn detection_produces_schema_and_identity_fields() {
        let fields = detect_capability_fields("node-test");
        assert_eq!(fields["schema_version"], serde_json::json!(1));
        assert_eq!(fields["id"], serde_json::json!("node-test"));
        // Host-dependent fields exist on any machine the suite runs on.
        assert!(fields.contains_key("os"), "os field missing: {fields:?}");
        assert!(fields.contains_key("cpu_cores"));
        assert!(fields.contains_key("memory"));
    }

    #[test]
    fn refresh_writes_once_then_holds_steady() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("caps.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([3u8; 32]));
        let store = RegisterStore::new(storage, identity).expect("store");

        let first = refresh_capabilities(&store).expect("first refresh");
        assert!(first, "first refresh must populate the register");

        // A second pass on an (essentially) unchanged host writes
        // nothing: static fields are equal and available_storage is
        // held by quantization + hysteresis.
        let second = refresh_capabilities(&store).expect("second refresh");
        assert!(!second, "unchanged host must not touch the register");

        let value = store
            .deep_value(&store.local_capability_id())
            .expect("read")
            .expect("present");
        assert_eq!(value["schema_version"], serde_json::json!(1));
    }

    #[test]
    fn hysteresis_holds_small_storage_movement() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("caps.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([4u8; 32]));
        let store = RegisterStore::new(storage, identity).expect("store");
        let doc_id = store.local_capability_id();

        let mut seed = JsonMap::new();
        seed.insert("available_storage".into(), serde_json::json!(500));
        store.upsert_local_fields(&doc_id, &seed).expect("seed");

        // 2% movement: held at the current value.
        let mut fields = JsonMap::new();
        fields.insert("available_storage".into(), serde_json::json!(510));
        apply_storage_hysteresis(&store, &doc_id, &mut fields).expect("hysteresis");
        assert_eq!(fields["available_storage"], serde_json::json!(500));

        // 10% movement: passes through.
        let mut fields = JsonMap::new();
        fields.insert("available_storage".into(), serde_json::json!(550));
        apply_storage_hysteresis(&store, &doc_id, &mut fields).expect("hysteresis");
        assert_eq!(fields["available_storage"], serde_json::json!(550));
    }
}
