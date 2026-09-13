//! Caller-injected operands for the fixed GetSpecList/GetLoadoutByName algorithms.
//!
//! This policy can be supplied independently or through a game-data package.
//! Its validation bounds metadata only; it neither authenticates source nor
//! evaluates patterns or requires every reached tree version to have a display
//! entry. Package validation additionally binds its latest version to startup.
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{collections::BTreeMap, fmt};

pub const BUILD_LOADOUT_POLICY_SCHEMA_VERSION: u32 = 1;
const MAX_VERSIONS: usize = 4096;
const MAX_STRING_BYTES: usize = 4096;
const MAX_TOTAL_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildLoadoutPolicy {
    pub schema_version: u32,
    pub default_title: String,
    pub latest_tree_version: String,
    #[serde(deserialize_with = "version_displays")]
    pub tree_version_display: BTreeMap<String, String>,
    pub version_prefix: String,
    pub version_suffix: String,
    pub single_link_pattern: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BuildLoadoutPolicyError {
    #[error("unsupported build loadout policy schema {actual}")]
    UnsupportedSchema { actual: u32 },
    #[error("build loadout policy exceeds the version row limit")]
    TooManyVersions,
    #[error("build loadout policy text exceeds the per-string byte limit")]
    TextTooLong,
    #[error("build loadout policy exceeds the aggregate text byte limit")]
    TextBudgetExceeded,
}

impl BuildLoadoutPolicy {
    /// Validate before preparing or cloning caller-owned metadata. Empty strings,
    /// embedded NULs and uncompiled patterns retain their source-time semantics.
    pub fn validate(&self) -> Result<(), BuildLoadoutPolicyError> {
        if self.schema_version != BUILD_LOADOUT_POLICY_SCHEMA_VERSION {
            return Err(BuildLoadoutPolicyError::UnsupportedSchema {
                actual: self.schema_version,
            });
        }
        if self.tree_version_display.len() > MAX_VERSIONS {
            return Err(BuildLoadoutPolicyError::TooManyVersions);
        }
        let mut bytes = 0usize;
        let mut text = |value: &str| {
            if value.len() > MAX_STRING_BYTES {
                return Err(BuildLoadoutPolicyError::TextTooLong);
            }
            bytes = bytes
                .checked_add(value.len())
                .ok_or(BuildLoadoutPolicyError::TextBudgetExceeded)?;
            if bytes > MAX_TOTAL_TEXT_BYTES {
                return Err(BuildLoadoutPolicyError::TextBudgetExceeded);
            }
            Ok(())
        };
        for value in [
            &self.default_title,
            &self.latest_tree_version,
            &self.version_prefix,
            &self.version_suffix,
            &self.single_link_pattern,
        ] {
            text(value)?;
        }
        for (version, display) in &self.tree_version_display {
            text(version)?;
            text(display)?;
        }
        Ok(())
    }
}

fn version_displays<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Displays;
    impl<'de> de::Visitor<'de> for Displays {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("unique tree-version display keys")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some(key) = source.next_key::<String>()? {
                if result.contains_key(&key) {
                    return Err(de::Error::custom("duplicate tree-version display key"));
                }
                if result.len() == MAX_VERSIONS {
                    return Err(de::Error::custom("too many tree-version display rows"));
                }
                result.insert(key, source.next_value()?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Displays)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> BuildLoadoutPolicy {
        BuildLoadoutPolicy {
            schema_version: BUILD_LOADOUT_POLICY_SCHEMA_VERSION,
            default_title: "Untitled by caller".into(),
            latest_tree_version: "caller-current".into(),
            tree_version_display: BTreeMap::from([("caller-old".into(), "Earlier".into())]),
            version_prefix: "<".into(),
            version_suffix: "> ".into(),
            single_link_pattern: "%((%w+)%)".into(),
        }
    }

    #[test]
    fn caller_policy_roundtrips_without_bundled_defaults() {
        let policy = fixture();
        policy.validate().unwrap();
        let encoded = serde_json::to_string(&policy).unwrap();
        let decoded: BuildLoadoutPolicy = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, policy);
        assert!(!decoded.tree_version_display.contains_key("caller-current"));
    }

    #[test]
    fn semantic_failures_remain_lazy() {
        let mut policy = fixture();
        policy.default_title.clear();
        policy.latest_tree_version.clear();
        policy.version_prefix = "\0".into();
        policy.version_suffix.clear();
        policy.single_link_pattern = "[".into();
        policy.tree_version_display.clear();
        policy.validate().unwrap();
    }

    #[test]
    fn schema_and_required_shape_are_checked() {
        let mut policy = fixture();
        policy.schema_version += 1;
        assert_eq!(
            policy.validate(),
            Err(BuildLoadoutPolicyError::UnsupportedSchema { actual: 2 })
        );
        let mut encoded = serde_json::to_value(policy).unwrap();
        encoded
            .as_object_mut()
            .unwrap()
            .remove("single_link_pattern");
        assert!(serde_json::from_value::<BuildLoadoutPolicy>(encoded).is_err());
        let mut encoded = serde_json::to_value(fixture()).unwrap();
        encoded["unknown"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<BuildLoadoutPolicy>(encoded).is_err());
        let encoded = serde_json::to_string(&fixture()).unwrap();
        let encoded = encoded.replace(
            "\"caller-old\":\"Earlier\"",
            "\"caller-old\":\"Earlier\",\"caller-old\":\"Other\"",
        );
        assert!(serde_json::from_str::<BuildLoadoutPolicy>(&encoded).is_err());
    }

    #[test]
    fn all_text_operands_and_version_entries_share_bounds() {
        for index in 0..7 {
            let mut policy = fixture();
            let oversized = "x".repeat(MAX_STRING_BYTES + 1);
            match index {
                0 => policy.default_title = oversized,
                1 => policy.latest_tree_version = oversized,
                2 => policy.version_prefix = oversized,
                3 => policy.version_suffix = oversized,
                4 => policy.single_link_pattern = oversized,
                5 => policy.tree_version_display = BTreeMap::from([(oversized, String::new())]),
                6 => policy.tree_version_display = BTreeMap::from([(String::new(), oversized)]),
                _ => unreachable!(),
            }
            assert_eq!(policy.validate(), Err(BuildLoadoutPolicyError::TextTooLong));
        }
        let mut policy = fixture();
        policy.tree_version_display = (0..257)
            .map(|index| (index.to_string(), "x".repeat(MAX_STRING_BYTES)))
            .collect();
        assert_eq!(
            policy.validate(),
            Err(BuildLoadoutPolicyError::TextBudgetExceeded)
        );
        policy.tree_version_display = (0..=MAX_VERSIONS)
            .map(|index| (index.to_string(), String::new()))
            .collect();
        assert_eq!(
            policy.validate(),
            Err(BuildLoadoutPolicyError::TooManyVersions)
        );
    }
}
