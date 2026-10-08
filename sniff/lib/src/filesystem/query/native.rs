//! Lossless serialized forms shared by the usage report.

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

/// A native OS string (path, executable, or process name) with a lossless
/// serialized form.
///
/// Serializes as `{"display": "..."}`, adding
/// `"native": {"encoding": "unix_bytes" | "windows_utf16", "units": [...]}`
/// only when `display` cannot round-trip the value (non-UTF-8 Unix bytes,
/// unpaired Windows surrogates). When `native` is present it is authoritative;
/// `display` is for people and must never drive matching or later filesystem
/// operations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeString(OsString);

impl NativeString {
    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn into_os_string(self) -> OsString {
        self.0
    }

    /// Human-readable form; invalid sequences become U+FFFD.
    pub fn display(&self) -> Cow<'_, str> {
        self.0.to_string_lossy()
    }

    /// Whether `display` alone reproduces the value.
    pub fn is_unicode(&self) -> bool {
        self.0.to_str().is_some()
    }
}

impl From<OsString> for NativeString {
    fn from(value: OsString) -> Self {
        Self(value)
    }
}

impl From<PathBuf> for NativeString {
    fn from(value: PathBuf) -> Self {
        Self(value.into_os_string())
    }
}

impl From<&Path> for NativeString {
    fn from(value: &Path) -> Self {
        Self(value.as_os_str().to_owned())
    }
}

impl From<&str> for NativeString {
    fn from(value: &str) -> Self {
        Self(OsString::from(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Encoding {
    UnixBytes,
    WindowsUtf16,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeUnits {
    encoding: Encoding,
    units: Vec<u32>,
}

#[cfg(unix)]
fn native_units(value: &OsStr) -> NativeUnits {
    use std::os::unix::ffi::OsStrExt;
    NativeUnits {
        encoding: Encoding::UnixBytes,
        units: value.as_bytes().iter().map(|&b| u32::from(b)).collect(),
    }
}

#[cfg(windows)]
fn native_units(value: &OsStr) -> NativeUnits {
    use std::os::windows::ffi::OsStrExt;
    NativeUnits {
        encoding: Encoding::WindowsUtf16,
        units: value.encode_wide().map(u32::from).collect(),
    }
}

fn narrow<T: TryFrom<u32>>(units: &[u32], encoding: &str) -> Result<Vec<T>, String> {
    units
        .iter()
        .map(|&unit| {
            T::try_from(unit).map_err(|_| format!("{encoding} unit {unit} is out of range"))
        })
        .collect()
}

/// Decodes `native` on this host. A foreign encoding is accepted only when it
/// holds valid Unicode, because this OS cannot represent anything else.
fn decode_native(native: &NativeUnits) -> Result<OsString, String> {
    match native.encoding {
        Encoding::UnixBytes => {
            let bytes: Vec<u8> = narrow(&native.units, "unix_bytes")?;
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStringExt;
                Ok(OsString::from_vec(bytes))
            }
            #[cfg(not(unix))]
            {
                String::from_utf8(bytes)
                    .map(OsString::from)
                    .map_err(|_| "unix_bytes value is not representable on this OS".to_string())
            }
        }
        Encoding::WindowsUtf16 => {
            let wide: Vec<u16> = narrow(&native.units, "windows_utf16")?;
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStringExt;
                Ok(OsString::from_wide(&wide))
            }
            #[cfg(not(windows))]
            {
                String::from_utf16(&wide)
                    .map(OsString::from)
                    .map_err(|_| "windows_utf16 value is not representable on this OS".to_string())
            }
        }
    }
}

impl Serialize for NativeString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let lossless = self.is_unicode();
        let mut map = serializer.serialize_map(Some(if lossless { 1 } else { 2 }))?;
        map.serialize_entry("display", &self.display())?;
        if !lossless {
            map.serialize_entry("native", &native_units(&self.0))?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for NativeString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NativeStringVisitor;

        impl<'de> Visitor<'de> for NativeStringVisitor {
            type Value = NativeString;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a native string object with `display` and optional `native`")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut display: Option<String> = None;
                let mut native: Option<NativeUnits> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "display" => {
                            if display.is_some() {
                                return Err(de::Error::duplicate_field("display"));
                            }
                            display = Some(map.next_value()?);
                        }
                        // `null` is rejected rather than read as absent: the
                        // writer omits the key when it is not needed.
                        "native" => {
                            if native.is_some() {
                                return Err(de::Error::duplicate_field("native"));
                            }
                            native = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::unknown_field(other, &["display", "native"]));
                        }
                    }
                }
                let display = display.ok_or_else(|| de::Error::missing_field("display"))?;
                match native {
                    Some(native) => decode_native(&native)
                        .map(NativeString)
                        .map_err(de::Error::custom),
                    None => Ok(NativeString(OsString::from(display))),
                }
            }
        }

        deserializer.deserialize_map(NativeStringVisitor)
    }
}

/// `serde(with)` adapter writing integers as canonical decimal strings, so
/// 64- and 128-bit identifiers survive JSON consumers limited to `f64`.
pub(crate) mod decimal {
    use serde::{Deserialize, Deserializer, Serializer, de};
    use std::fmt::Display;
    use std::str::FromStr;

    pub(crate) fn serialize<S: Serializer, T: Display>(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(value)
    }

    pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
    {
        let text = String::deserialize(deserializer)?;
        parse(&text).map_err(de::Error::custom)
    }

    /// Accepts only `0` or digits without a leading zero, sign, or whitespace,
    /// which `FromStr` for integers would otherwise allow in part.
    pub(crate) fn parse<T: FromStr>(text: &str) -> Result<T, String> {
        let canonical = !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
            && (text == "0" || !text.starts_with('0'));
        if !canonical {
            return Err(format!("`{text}` is not a canonical decimal string"));
        }
        text.parse()
            .map_err(|_| format!("`{text}` is out of range"))
    }

    /// The same encoding for an `Option`, where `null` is the absent value.
    pub(crate) mod option {
        use serde::{Deserialize, Deserializer, Serializer, de};
        use std::fmt::Display;
        use std::str::FromStr;

        pub(crate) fn serialize<S: Serializer, T: Display>(
            value: &Option<T>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match value {
                Some(value) => serializer.collect_str(value),
                None => serializer.serialize_none(),
            }
        }

        pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: Deserializer<'de>,
            T: FromStr,
        {
            Option::<String>::deserialize(deserializer)?
                .map(|text| super::parse(&text).map_err(de::Error::custom))
                .transpose()
        }
    }
}

/// Deserializes an `Option` field that must be present, even when `null`.
///
/// serde reads a missing `Option` field as `None` unless the field names a
/// `deserialize_with` function, so routing through this one makes an absent
/// key an error while `null` stays the explicit "unavailable" value.
pub(crate) fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
