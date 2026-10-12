use serde::Deserialize;

use super::manifest::is_valid_id;

const DEFAULT_TIMEOUT_MS: u64 = 25;
const MAX_TIMEOUT_MS: u64 = 100;
const MAX_CACHE_TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptSegmentContribution {
    pub name: String,
    pub summary: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub cache_ttl_ms: u64,
}

impl PromptSegmentContribution {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !is_valid_id(&self.name) {
            return Err("name must be a valid plugin identifier");
        }
        if self.summary.trim().is_empty() || self.summary.chars().any(char::is_control) {
            return Err("summary must be visible text");
        }
        if !(1..=MAX_TIMEOUT_MS).contains(&self.timeout_ms) {
            return Err("timeout_ms must be between 1 and 100");
        }
        if self.cache_ttl_ms > MAX_CACHE_TTL_MS {
            return Err("cache_ttl_ms must not exceed 300000");
        }
        Ok(())
    }
}

const fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_a_short_uncached_budget() {
        let segment: PromptSegmentContribution =
            toml::from_str("name = \"node\"\nsummary = \"Node version\"\n").unwrap();

        assert_eq!(segment.timeout_ms, 25);
        assert_eq!(segment.cache_ttl_ms, 0);
        assert_eq!(segment.validate(), Ok(()));
    }

    #[test]
    fn rejects_a_prompt_blocking_timeout() {
        let segment = PromptSegmentContribution {
            name: "slow".into(),
            summary: "Too slow".into(),
            timeout_ms: 500,
            cache_ttl_ms: 0,
        };

        assert_eq!(
            segment.validate(),
            Err("timeout_ms must be between 1 and 100")
        );
    }
}
