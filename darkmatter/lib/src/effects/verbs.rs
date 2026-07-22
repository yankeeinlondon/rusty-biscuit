//! Verb implementations for the side-effect engine.

use crate::effects::EffectEngine;
use crate::effects::error::EffectError;
use crate::effects::fs_write::{atomic_write_guarded, ensure_within};
use crate::markdown::FrontmatterMap;
use crate::markdown::Markdown;
use crate::markdown::hash::MdHashOptions;
use biscuit_file::file_reference::fetch::{FetchPolicy, HostPattern};
use serde_json::Value;
use std::path::PathBuf;

impl EffectEngine {
    /// Resolves a (normalized) path argument against the mutation root.
    fn resolve(&self, raw: &str) -> Result<PathBuf, EffectError> {
        let normalized = normalize_path_arg(raw);
        let joined = std::path::Path::new(&normalized);
        let abs = if joined.is_absolute() {
            joined.to_path_buf()
        } else {
            self.mutation_root().join(joined)
        };
        Ok(abs)
    }

    /// Loads a Markdown file for mutation.
    fn load(&self, raw: &str) -> Result<(PathBuf, Markdown), EffectError> {
        let path = self.resolve(raw)?;
        let content = std::fs::read_to_string(&path).map_err(|source| EffectError::Io {
            path: path.clone(),
            source,
        })?;
        let md = Markdown::try_from_content(content)
            .map_err(|e| EffectError::Markdown(e.to_string()))?;
        Ok((path, md))
    }

    /// Serializes, optionally re-hashes, and atomically writes the document.
    fn save(&self, raw_path: &str, md: &Markdown) -> Result<(), EffectError> {
        let path = self.resolve(raw_path)?;
        let serialized = if self.auto_rehash() && md.frontmatter().as_map().contains_key("hash") {
            let opts = MdHashOptions::default();
            let decision = md
                .plan_hash_save(None, &opts)
                .map_err(|e| EffectError::Markdown(e.to_string()))?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            md.apply_hash_save(&decision, &opts, &today)
                .unwrap_or_else(|| md.as_string())
        } else {
            md.as_string()
        };
        atomic_write_guarded(self.mutation_root(), &path, serialized.as_bytes())
    }

    /// `set(key, value)` → prior value (or null).
    ///
    /// Writes `key` into an in-memory `state` map and returns the value it
    /// replaced. Unlike [`set_frontmatter`](Self::set_frontmatter), this never
    /// touches disk: it mutates the caller's map so an orchestrator can carry
    /// runtime state across lifecycle actions and loop iterations without a
    /// file write-back.
    ///
    /// The value is stored verbatim, so its JSON type is preserved (a boolean
    /// stays a boolean, an array stays an array).
    ///
    /// ## Errors
    ///
    /// Returns [`EffectError::InvalidKey`] when `key` is empty or contains a
    /// `.` (dotted-path nesting is not supported; keys are top-level only).
    /// Which top-level keys an orchestrator forbids (e.g. Claudine's reserved
    /// sequence keys) is the caller's policy and is intentionally not enforced
    /// here.
    pub fn set(
        &self,
        state: &mut FrontmatterMap,
        key: &str,
        value: Value,
    ) -> Result<Value, EffectError> {
        if key.is_empty() || key.contains('.') {
            return Err(EffectError::InvalidKey(key.to_string()));
        }
        let prior = state.get(key).cloned().unwrap_or(Value::Null);
        state.insert(key.to_string(), value);
        Ok(prior)
    }

    /// `set_frontmatter(file, prop, value)` → prior value (or null).
    pub fn set_frontmatter(
        &self,
        file: &str,
        prop: &str,
        value: Value,
    ) -> Result<Value, EffectError> {
        let (_, mut md) = self.load(file)?;
        let prior = md
            .frontmatter()
            .as_map()
            .get(prop)
            .cloned()
            .unwrap_or(Value::Null);
        md.frontmatter_mut()
            .as_map_mut()
            .insert(prop.to_string(), value);
        self.save(file, &md)?;
        Ok(prior)
    }

    /// `merge_frontmatter(file, obj)` → merged object (shallow).
    pub fn merge_frontmatter(&self, file: &str, obj: Value) -> Result<Value, EffectError> {
        let incoming = obj.as_object().ok_or(EffectError::PropertyType {
            op: "merge_frontmatter",
            prop: "<obj>".to_string(),
        })?;
        let (_, mut md) = self.load(file)?;
        {
            let map = md.frontmatter_mut().as_map_mut();
            for (k, v) in incoming {
                map.insert(k.clone(), v.clone());
            }
        }
        self.save(file, &md)?;
        let merged: serde_json::Map<String, Value> = md
            .frontmatter()
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Value::Object(merged))
    }

    /// `delete_frontmatter(file, prop)` → removed value (or null).
    pub fn delete_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
        let (_, mut md) = self.load(file)?;
        let removed = md
            .frontmatter_mut()
            .as_map_mut()
            .shift_remove(prop)
            .unwrap_or(Value::Null);
        self.save(file, &md)?;
        Ok(removed)
    }

    /// `increment_frontmatter(file, prop)` → new number (missing → 1).
    pub fn increment_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
        self.bump(file, prop, 1)
    }

    /// `decrement_frontmatter(file, prop)` → new number (missing → -1).
    pub fn decrement_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
        self.bump(file, prop, -1)
    }

    fn bump(&self, file: &str, prop: &str, delta: i64) -> Result<Value, EffectError> {
        let (_, mut md) = self.load(file)?;
        let current = md.frontmatter().as_map().get(prop).cloned();
        let n: i64 = match current {
            None | Some(Value::Null) => 0,
            Some(Value::Number(num)) => num.as_i64().ok_or(EffectError::PropertyType {
                op: "increment_frontmatter",
                prop: prop.to_string(),
            })?,
            Some(Value::String(s)) => {
                s.trim()
                    .parse::<i64>()
                    .map_err(|_| EffectError::PropertyType {
                        op: "increment_frontmatter",
                        prop: prop.to_string(),
                    })?
            }
            Some(_) => {
                return Err(EffectError::PropertyType {
                    op: "increment_frontmatter",
                    prop: prop.to_string(),
                });
            }
        };
        let next = Value::Number((n + delta).into());
        md.frontmatter_mut()
            .as_map_mut()
            .insert(prop.to_string(), next.clone());
        self.save(file, &md)?;
        Ok(next)
    }

    /// `append_frontmatter(file, prop, value)` → new array.
    pub fn append_frontmatter(
        &self,
        file: &str,
        prop: &str,
        value: Value,
    ) -> Result<Value, EffectError> {
        self.array_mutate(file, prop, value, true)
    }

    /// `prepend_frontmatter(file, prop, value)` → new array.
    pub fn prepend_frontmatter(
        &self,
        file: &str,
        prop: &str,
        value: Value,
    ) -> Result<Value, EffectError> {
        self.array_mutate(file, prop, value, false)
    }

    fn array_mutate(
        &self,
        file: &str,
        prop: &str,
        value: Value,
        append: bool,
    ) -> Result<Value, EffectError> {
        let (_, mut md) = self.load(file)?;
        let mut arr = match md.frontmatter().as_map().get(prop).cloned() {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) => a,
            Some(_) => {
                return Err(EffectError::PropertyType {
                    op: if append {
                        "append_frontmatter"
                    } else {
                        "prepend_frontmatter"
                    },
                    prop: prop.to_string(),
                });
            }
        };
        if append {
            arr.push(value);
        } else {
            arr.insert(0, value);
        }
        let new_value = Value::Array(arr);
        md.frontmatter_mut()
            .as_map_mut()
            .insert(prop.to_string(), new_value.clone());
        self.save(file, &md)?;
        Ok(new_value)
    }

    /// `ensure_file(file)` → absolute path; creates an empty file if missing.
    pub fn ensure_file(&self, file: &str) -> Result<String, EffectError> {
        self.ensure_file_inner(file, None)
    }

    /// `ensure_file(file, content)` → absolute path; writes `content` only when
    /// creating a missing file (existing files are left unchanged).
    pub fn ensure_file_with_content(
        &self,
        file: &str,
        content: &str,
    ) -> Result<String, EffectError> {
        self.ensure_file_inner(file, Some(content))
    }

    fn ensure_file_inner(&self, file: &str, content: Option<&str>) -> Result<String, EffectError> {
        let path = self.resolve(file)?;
        // Verify containment up front (covers both the create and no-op paths).
        let cleaned = ensure_within(self.mutation_root(), &path)?;
        if !cleaned.exists() {
            atomic_write_guarded(
                self.mutation_root(),
                &cleaned,
                content.unwrap_or("").as_bytes(),
            )?;
        }
        Ok(cleaned.to_string_lossy().to_string())
    }

    /// `ensure_dir(dir)` → absolute path (`mkdir -p`).
    pub fn ensure_dir(&self, dir: &str) -> Result<String, EffectError> {
        let path = self.resolve(dir)?;
        let cleaned = ensure_within(self.mutation_root(), &path)?;
        std::fs::create_dir_all(&cleaned).map_err(|source| EffectError::Io {
            path: cleaned.clone(),
            source,
        })?;
        Ok(cleaned.to_string_lossy().to_string())
    }

    /// `append_line(file, text)` → absolute path.
    pub fn append_line(&self, file: &str, text: &str) -> Result<String, EffectError> {
        let path = self.resolve(file)?;
        let cleaned = ensure_within(self.mutation_root(), &path)?;
        let mut existing = std::fs::read_to_string(&cleaned).unwrap_or_default();
        existing.push_str(text);
        existing.push('\n');
        atomic_write_guarded(self.mutation_root(), &cleaned, existing.as_bytes())?;
        Ok(cleaned.to_string_lossy().to_string())
    }

    /// `append_jsonl(file, obj)` → absolute path.
    pub fn append_jsonl(&self, file: &str, obj: Value) -> Result<String, EffectError> {
        let line = serde_json::to_string(&obj).map_err(|e| EffectError::Markdown(e.to_string()))?;
        self.append_line(file, &line)
    }

    /// `http_post(url, body)` -> object with `status` and `body`.
    pub fn http_post(&self, url: &str, body: impl Into<Vec<u8>>) -> Result<Value, EffectError> {
        #[cfg(feature = "effects-instrumentation")]
        crate::effects::record_network_attempt();
        let url = url::Url::parse(url).map_err(|e| EffectError::InvalidUrl(e.to_string()))?;
        let mut policy = FetchPolicy::deny_all();
        for host in self.allowed_hosts() {
            policy = policy.allow(HostPattern::Exact(host.clone()));
        }

        let client = biscuit_file::file_reference::fetch::PolicyClient::new()
            .map_err(|e| EffectError::Network(e.to_string()))?;
        let response = biscuit_file::file_reference::fetch::post_blocking(
            &client,
            &url,
            &policy,
            body.into(),
        )
        .map_err(|e| match e {
            biscuit_file::FetchError::PolicyDenied { host } => EffectError::HostNotAllowed(host),
            other => EffectError::Network(other.to_string()),
        })?;

        let response_body = String::from_utf8_lossy(&response.body).to_string();
        Ok(serde_json::json!({
            "status": response.status,
            "body": response_body,
        }))
    }
}

/// Strips a leading `file://` and collapses doubled `/` (engine-local copy of
/// the expression-engine normalization, kept here to avoid coupling).
fn normalize_path_arg(raw: &str) -> String {
    let stripped = raw.strip_prefix("file://").unwrap_or(raw);
    let mut out = String::with_capacity(stripped.len());
    let mut prev_slash = false;
    for ch in stripped.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::effects::EffectEngine;
    use crate::markdown::FrontmatterMap;
    use serde_json::{Value, json};

    #[test]
    fn set_returns_prior_value_and_mutates_in_memory() {
        let eng = EffectEngine::builder().build();
        let mut state = FrontmatterMap::new();

        // First write of an absent key returns null.
        let prior = eng.set(&mut state, "count", json!(1)).unwrap();
        assert_eq!(prior, Value::Null);
        assert_eq!(state.get("count"), Some(&json!(1)));

        // Overwriting returns the value that was replaced.
        let prior = eng.set(&mut state, "count", json!(2)).unwrap();
        assert_eq!(prior, json!(1));
        assert_eq!(state.get("count"), Some(&json!(2)));
    }

    #[test]
    fn set_preserves_whole_value_types() {
        let eng = EffectEngine::builder().build();
        let mut state = FrontmatterMap::new();

        eng.set(&mut state, "flag", json!(true)).unwrap();
        eng.set(&mut state, "list", json!([1, 2, 3])).unwrap();
        eng.set(&mut state, "obj", json!({"a": 1})).unwrap();

        assert_eq!(state.get("flag"), Some(&Value::Bool(true)));
        assert_eq!(state.get("list"), Some(&json!([1, 2, 3])));
        assert_eq!(state.get("obj"), Some(&json!({"a": 1})));
    }

    #[test]
    fn set_performs_no_filesystem_write() {
        // A file-less mutation root would make any disk write fail; `set` must
        // not touch it. The temp dir starts and stays empty.
        let dir = tempfile::TempDir::new().unwrap();
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();
        let mut state = FrontmatterMap::new();

        eng.set(&mut state, "ready", json!(true)).unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert!(entries.is_empty(), "set must not write to disk");
    }

    #[test]
    fn set_rejects_empty_and_dotted_keys() {
        let eng = EffectEngine::builder().build();
        let mut state = FrontmatterMap::new();

        let empty = eng.set(&mut state, "", json!(1)).unwrap_err();
        assert!(matches!(empty, crate::effects::EffectError::InvalidKey(k) if k.is_empty()));

        let dotted = eng.set(&mut state, "a.b", json!(1)).unwrap_err();
        assert!(matches!(dotted, crate::effects::EffectError::InvalidKey(k) if k == "a.b"));

        // A rejected write leaves the map untouched.
        assert!(state.is_empty());
    }

    #[test]
    fn set_frontmatter_writes_and_rehashes() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("d.md");
        std::fs::write(&file, "---\ntitle: T\nhash: stale\n---\nBody\n").unwrap();
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();

        let prior = eng
            .set_frontmatter("d.md", "status", json!("in-progress"))
            .unwrap();
        assert_eq!(prior, Value::Null); // status did not exist before

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("status: in-progress"));
        assert!(written.contains("title: T"));
        // hash was recomputed (no longer the literal "stale").
        assert!(!written.contains("hash: stale"));
    }

    #[test]
    fn frontmatter_mutation_verbs() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("d.md");
        std::fs::write(&file, "---\nphase: 1\ntags: [a]\n---\nBody\n").unwrap();
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();

        assert_eq!(eng.increment_frontmatter("d.md", "phase").unwrap(), json!(2));
        assert_eq!(eng.decrement_frontmatter("d.md", "phase").unwrap(), json!(1));
        assert_eq!(
            eng.append_frontmatter("d.md", "tags", json!("b")).unwrap(),
            json!(["a", "b"])
        );
        assert_eq!(
            eng.prepend_frontmatter("d.md", "tags", json!("z")).unwrap(),
            json!(["z", "a", "b"])
        );
        let merged = eng
            .merge_frontmatter("d.md", json!({"owner": "ken"}))
            .unwrap();
        assert_eq!(merged["owner"], json!("ken"));
        let removed = eng.delete_frontmatter("d.md", "owner").unwrap();
        assert_eq!(removed, json!("ken"));
    }

    #[test]
    fn http_post_denies_hosts_by_default() {
        let eng = EffectEngine::builder().build();

        let err = eng
            .http_post("https://example.com/hook", b"{}")
            .unwrap_err();

        assert!(matches!(err, crate::effects::EffectError::HostNotAllowed(_)));
    }
}
