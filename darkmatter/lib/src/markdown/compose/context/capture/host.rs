use super::*;
use sniff::os::OsType;

use super::super::format;
use super::snapshot::ContextCapture;

pub(super) const OS_KEYS: &[&str] = &["os", "os_distro", "os_package_manager", "os_version"];
pub(super) const HARDWARE_KEYS: &[&str] = &[
    "memory_total", "memory_used", "memory_avail", "cpu_cores", "cpu_arch",
];
pub(super) const GPU_KEYS: &[&str] = &["gpu"];

pub(super) fn populate_os(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let os_info = cap.os_info.as_ref();

    values.insert(
        "os".into(),
        os_info.map_or(Value::Null, |info| {
            Value::String(
                match info.os_type {
                    OsType::Windows => "Windows",
                    OsType::MacOS => "macOS",
                    OsType::Linux => "Linux",
                    _ => return Value::Null,
                }
                .to_string(),
            )
        }),
    );

    values.insert(
        "os_distro".into(),
        Value::String(
            os_info
                .and_then(|info| info.distribution.clone())
                .unwrap_or_default(),
        ),
    );

    values.insert(
        "os_package_manager".into(),
        os_info
            .and_then(|info| {
                info.system_package_managers
                    .as_ref()
                    .and_then(|spm| spm.primary.as_ref())
                    .map(|pm| Value::String(format!("{pm:?}")))
            })
            .unwrap_or(Value::Null),
    );

    values.insert(
        "os_version".into(),
        Value::String(os_info.map(|info| info.version.clone()).unwrap_or_default()),
    );
}

// ── Hardware context ──────────────────────────────────────────────

pub(super) fn populate_hardware(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let hw = cap.hardware_info.as_ref();

    values.insert(
        "memory_total".into(),
        hw.map_or(Value::Null, |h| {
            Value::String(format::format_bytes(h.memory.total_bytes))
        }),
    );

    values.insert(
        "memory_used".into(),
        hw.map_or(Value::Null, |h| {
            (h.memory.used_bytes * 100)
                .checked_div(h.memory.total_bytes)
                .map_or_else(
                    || Value::String("0%".to_string()),
                    |pct| Value::String(format!("{pct}%")),
                )
        }),
    );

    values.insert(
        "memory_avail".into(),
        hw.map_or(Value::Null, |h| {
            Value::String(format::format_bytes(h.memory.available_bytes))
        }),
    );

    values.insert(
        "cpu_cores".into(),
        hw.map_or(Value::Null, |h| Value::Number(h.cpu.logical_cores.into())),
    );

    values.insert(
        "cpu_arch".into(),
        hw.map_or(Value::Null, |h| Value::String(h.cpu.arch.clone())),
    );

}

pub(super) fn populate_gpu(cap: &ContextCapture, values: &mut Map<String, Value>) {
    values.insert(
        "gpu".into(),
        cap.gpu_names
            .as_ref()
            .map_or(Value::Null, |name| Value::String(name.clone())),
    );
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn gpu_only_population_does_not_require_hardware_capture() {
        let mut cap = ContextCapture::for_test_base(PathBuf::from("/tmp"), None);
        cap.gpu_names = Some("Injected GPU".to_string());
        assert!(cap.hardware_info.is_none());

        let mut values = Map::new();
        populate_gpu(&cap, &mut values);

        assert_eq!(values.get("gpu"), Some(&Value::String("Injected GPU".into())));
        assert!(!values.contains_key("cpu_cores"));
        assert!(!values.contains_key("memory_total"));
    }
}
