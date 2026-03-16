use serde::{Deserialize, Serialize};

/// Provider-agnostic token usage counters.
///
/// Fields are optional because not every provider exposes every counter.
/// Reporting ingestion maps these directly to `input_tokens`,
/// `output_tokens`, `total_tokens`, and `cache_read_tokens` columns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NormalizedTokenUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
}

impl NormalizedTokenUsage {
    /// Merge another usage snapshot, preferring non-None values from `other`.
    pub fn merge(&mut self, other: &NormalizedTokenUsage) {
        if other.input.is_some() {
            self.input = other.input;
        }
        if other.output.is_some() {
            self.output = other.output;
        }
        if other.total.is_some() {
            self.total = other.total;
        }
        if other.cache_read.is_some() {
            self.cache_read = other.cache_read;
        }
    }

    /// Accumulate values from `other` (for OpenCode per-step accumulation).
    pub fn accumulate(&mut self, other: &NormalizedTokenUsage) {
        if let Some(v) = other.input {
            *self.input.get_or_insert(0) += v;
        }
        if let Some(v) = other.output {
            *self.output.get_or_insert(0) += v;
        }
        if let Some(v) = other.total {
            *self.total.get_or_insert(0) += v;
        }
        if let Some(v) = other.cache_read {
            *self.cache_read.get_or_insert(0) += v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_other_non_none() {
        let mut base = NormalizedTokenUsage {
            input: Some(100),
            output: Some(200),
            total: Some(300),
            cache_read: None,
        };
        let other = NormalizedTokenUsage {
            input: Some(150),
            output: None,
            total: Some(400),
            cache_read: Some(50),
        };
        base.merge(&other);
        assert_eq!(base.input, Some(150));
        assert_eq!(base.output, Some(200)); // kept, other was None
        assert_eq!(base.total, Some(400));
        assert_eq!(base.cache_read, Some(50));
    }

    #[test]
    fn merge_noop_when_other_all_none() {
        let mut base = NormalizedTokenUsage {
            input: Some(100),
            output: Some(200),
            total: Some(300),
            cache_read: Some(50),
        };
        let other = NormalizedTokenUsage::default();
        base.merge(&other);
        assert_eq!(base.input, Some(100));
        assert_eq!(base.output, Some(200));
        assert_eq!(base.total, Some(300));
        assert_eq!(base.cache_read, Some(50));
    }

    #[test]
    fn accumulate_adds_values() {
        let mut base = NormalizedTokenUsage {
            input: Some(100),
            output: Some(200),
            total: Some(300),
            cache_read: None,
        };
        let step = NormalizedTokenUsage {
            input: Some(50),
            output: Some(75),
            total: Some(125),
            cache_read: Some(10),
        };
        base.accumulate(&step);
        assert_eq!(base.input, Some(150));
        assert_eq!(base.output, Some(275));
        assert_eq!(base.total, Some(425));
        assert_eq!(base.cache_read, Some(10));
    }

    #[test]
    fn accumulate_initializes_from_none() {
        let mut base = NormalizedTokenUsage::default();
        let step = NormalizedTokenUsage {
            input: Some(50),
            output: None,
            total: Some(50),
            cache_read: None,
        };
        base.accumulate(&step);
        assert_eq!(base.input, Some(50));
        assert_eq!(base.output, None);
        assert_eq!(base.total, Some(50));
        assert_eq!(base.cache_read, None);
    }

    #[test]
    fn accumulate_multiple_steps() {
        let mut base = NormalizedTokenUsage::default();
        for i in 1..=3 {
            let step = NormalizedTokenUsage {
                input: Some(i * 10),
                output: Some(i * 20),
                total: Some(i * 30),
                cache_read: None,
            };
            base.accumulate(&step);
        }
        assert_eq!(base.input, Some(60)); // 10 + 20 + 30
        assert_eq!(base.output, Some(120)); // 20 + 40 + 60
        assert_eq!(base.total, Some(180)); // 30 + 60 + 90
        assert_eq!(base.cache_read, None);
    }

    #[test]
    fn serde_round_trip() {
        let usage = NormalizedTokenUsage {
            input: Some(1000),
            output: Some(500),
            total: Some(1500),
            cache_read: Some(200),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let restored: NormalizedTokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage, restored);
    }

    #[test]
    fn serde_skips_none_fields() {
        let usage = NormalizedTokenUsage {
            input: Some(100),
            output: None,
            total: None,
            cache_read: None,
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(!json.contains("output"));
        assert!(!json.contains("total"));
        assert!(!json.contains("cache_read"));
    }
}
