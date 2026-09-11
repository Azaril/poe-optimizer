//! Reviewed source/IR selection is injected policy, separate from syntax lowering.
//! Matching a policy record binds permission; the independent public source tests
//! and package trust record establish the evidence behind that permission.
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;

const POLICY: &str = include_str!("../modifier_parser_program_policy.json");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    upstream_revision: String,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    source: ParserProgramProvenance,
    program_sha256: String,
    role: ParserProgramRole,
    evidence: String,
}

pub(super) fn bind(
    owner: &ModifierParserData,
    data: ParserProgramData,
) -> Result<ParserProgramPayload> {
    let policy: Policy = serde_json::from_str(POLICY)?;
    bind_policy(owner, data, policy)
}
fn bind_policy(
    owner: &ModifierParserData,
    data: ParserProgramData,
    policy: Policy,
) -> Result<ParserProgramPayload> {
    if policy.schema_version != 1
        || policy.upstream_revision != owner.source.upstream_revision
        || policy.entries.len() > data.programs.len()
    {
        return Err(error("invalid parser program admission policy"));
    }
    let mut admissions = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for entry in policy.entries {
        let matches = data
            .programs
            .iter()
            .filter(|program| program.provenance == entry.source)
            .collect::<Vec<_>>();
        let [program] = matches.as_slice() else {
            return Err(error(
                "program admission policy does not identify exactly one complete function",
            ));
        };
        if !seen.insert(program.callback)
            || program.sha256().map_err(error)? != entry.program_sha256
        {
            return Err(error("duplicate or changed program admission policy body"));
        }
        let permission = ParserProgramAdmission::bind(owner, program, entry.role, entry.evidence)
            .map_err(error)?;
        admissions.insert(program.callback, permission);
    }
    Ok(ParserProgramPayload { data, admissions })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> &'static ModifierParserData {
        static OWNER: std::sync::OnceLock<ModifierParserCatalog> = std::sync::OnceLock::new();
        OWNER
            .get_or_init(|| {
                poe_optimizer_data::game_data::bundled_snapshot()
                    .unwrap()
                    .modifier_parser()
                    .clone()
            })
            .data()
    }
    #[test]
    fn injected_source_policy_reproduces_packaged_admissions() {
        let owner = owner();
        let payload = bind(owner, owner.programs.data.clone()).unwrap();
        assert_eq!(payload, owner.programs);
        assert_eq!(payload.admissions.len(), 4);
        assert_eq!(
            payload
                .admissions
                .values()
                .filter(|a| a.role == ParserProgramRole::Special)
                .count(),
            3
        );
    }
    #[test]
    fn empty_policy_does_not_admit_structurally_generated_programs() {
        let owner = owner();
        let policy = Policy {
            schema_version: 1,
            upstream_revision: owner.source.upstream_revision.clone(),
            entries: Vec::new(),
        };
        let payload = bind_policy(owner, owner.programs.data.clone(), policy).unwrap();
        assert!(payload.admissions.is_empty());
        assert_eq!(payload.data, owner.programs.data);
    }
    #[test]
    fn source_policy_rejects_stale_ir_duplicate_targets_and_missing_bodies() {
        let owner = owner();
        let mut policy: Policy = serde_json::from_str(POLICY).unwrap();
        policy.entries[0].program_sha256 = "0".repeat(64);
        assert!(
            bind_policy(owner, owner.programs.data.clone(), policy)
                .unwrap_err()
                .to_string()
                .contains("changed")
        );
        let mut policy: Policy = serde_json::from_str(POLICY).unwrap();
        let extra: Policy = serde_json::from_str(POLICY).unwrap();
        policy
            .entries
            .push(extra.entries.into_iter().next().unwrap());
        assert!(
            bind_policy(owner, owner.programs.data.clone(), policy)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
        let mut policy: Policy = serde_json::from_str(POLICY).unwrap();
        policy.entries[0].source.function_sha256 = "0".repeat(64);
        assert!(
            bind_policy(owner, owner.programs.data.clone(), policy)
                .unwrap_err()
                .to_string()
                .contains("exactly one")
        );
    }
    #[test]
    fn source_policy_requires_current_version_and_source_revision() {
        for field in ["schema_version", "upstream_revision"] {
            let mut policy: serde_json::Value = serde_json::from_str(POLICY).unwrap();
            policy[field] = if field == "schema_version" {
                serde_json::json!(99)
            } else {
                serde_json::json!("0".repeat(40))
            };
            let owner = owner();
            let policy = serde_json::from_value(policy).unwrap();
            assert!(bind_policy(owner, owner.programs.data.clone(), policy).is_err());
        }
    }
}
