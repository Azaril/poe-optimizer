//! A source-bound usage policy; readiness remains an authoring fragment.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_normalize::{
        GemInventoryPolicy, OccurrenceUsagePolicy, UsageInputPolicy, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "owned-skill-participation-v1";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/skill-participation")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
pub fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn owner() -> DefinitionRules {
    decode(&read::<Value>("extension.json")["owners"][0])
}

pub fn check_authored() {
    let b: Value = read("bindings.json");
    let e: Value = read("extension.json");
    let r: Value = read("readiness.json");
    assert_eq!(e["schema"].as_array().unwrap().len(), 4);
    assert_eq!(e["owners"].as_array().unwrap().len(), 1);
    assert!(
        e["tables"].as_array().unwrap().is_empty() && e["receivers"].as_array().unwrap().is_empty()
    );
    let o = owner();
    assert!(o.programs.is_complete());
    assert_eq!(o.programs.members.len(), 1);
    let p = &o.programs.members[0];
    assert_eq!(p.context, RuleEntityKind::Skill);
    assert_eq!(p.id.as_str(), "requested-skill-participation");
    assert_eq!(p.reads.len(), 2);
    assert_eq!(p.nodes.len(), 3);
    assert_eq!(p.effects.len(), 1);
    for (i, name) in ["group", "occurrence"].into_iter().enumerate() {
        assert_eq!(p.reads[i].value_type, ComputedValueType::Boolean);
        assert_eq!(
            p.reads[i].source,
            RuleReadSource::Parameter {
                slot: decode(&b[name])
            }
        );
        assert_eq!(
            p.nodes[i].expression,
            RuleExpression::Read {
                input: p.reads[i].id.clone()
            }
        );
    }
    assert_eq!(
        p.nodes[2].expression,
        RuleExpression::All {
            values: p.nodes[..2].iter().map(|n| n.id.clone()).collect()
        }
    );
    assert!(p.effects[0].when.is_none());
    assert_eq!(
        p.effects[0].effect,
        RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat: decode(&b["participation"]),
            value: p.nodes[2].id.clone()
        }
    );
    let mut restored = r["readiness"]["skill_after"].clone();
    assert_eq!(
        restored
            .as_object_mut()
            .unwrap()
            .remove("participation")
            .unwrap(),
        b["participation"]
    );
    assert_eq!(restored, r["readiness"]["skill_before"]);
    assert_eq!(restored["parameters"]["closure"]["kind"], "complete");
    for row in restored["parameters"]["members"].as_array().unwrap() {
        assert_eq!(row["phase"], "execution");
    }
    assert_eq!(r["readiness"]["program"]["phase"], "preparation");
    assert_eq!(r["readiness"]["program"]["role"], "preparation_facts");
    let u: OccurrenceUsagePolicy = read("usage.json");
    assert_eq!(u.policy, decode(&b["policy"]));
    assert_eq!(u.parameters.len(), 2);
    let u = json!(u);
    assert_eq!(u["parameters"][0]["source"]["kind"], "containing_group");
    assert_eq!(u["parameters"][1]["source"]["kind"], "occurrence");
    for (i, name) in ["group", "occurrence"].into_iter().enumerate() {
        assert_eq!(u["parameters"][i]["slot"], b[name]);
        assert_eq!(
            u["parameters"][i]["source"]["value"]["missing"]["kind"],
            "pending"
        );
        assert_eq!(
            u["parameters"][i]["source"]["value"]["tiers"][0]["selectors"],
            json!([{"lane":"attribute","name":"enabled"}])
        );
    }
    source(false);
}

fn projection(report: &Value) -> Value {
    json!(report["cases"].as_array().unwrap()[..5].iter().enumerate().map(|(i,c)| {
        let states=c["observation"]["states"].as_object().unwrap().iter().map(|(stage,s)| {
            assert_eq!(s["outputs_preserved"],true); assert_eq!(s["source_methods_preserved"],true);
            assert_eq!(s["calculation_hook"],false); assert_eq!(s["native_participation_authority"],false);
            let groups:Vec<_>=s["provenance"]["saved"].as_array().unwrap().iter().filter(|g|g["source_ordinal"]==210).collect();assert_eq!(groups.len(),1);
            let modes=s["state"]["modes"].as_object().unwrap().iter().map(|(mode,v)| (mode.clone(),json!({"main":v["main"],"selected_group":v["selected_group"],"default_unarmed_without_source":v["default_unarmed_without_source"]}))).collect::<serde_json::Map<_,_>>();
            (stage.clone(),json!({"saved_group":groups[0],"modes":modes}))
        }).collect::<serde_json::Map<_,_>>();
        json!({"case_index":i,"name":c["name"],"xml_sha256":c["xml_sha256"],"changed_fields":c["changed_fields"],"independent_fresh_repeat_equal":c["independent_fresh_repeat_equal"],"independent_unhooked_equal":c["independent_unhooked_equal"],"states":states})
    }).collect::<Vec<_>>())
}
pub fn source(full: bool) {
    let v: Value = read("source-vectors.json");
    let m = &v["metadata"];
    assert_eq!(m["native_participation_authority"], false);
    assert_eq!(m["native_build_parity"], false);
    assert_eq!(m["no_retry_or_settling"], true);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), m["manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], m["source_revision"]);
    for pin in m["files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["path"] == pin["path"] && x["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert_eq!(v["cases"].as_array().unwrap().len(), 5);
    for (i, c) in v["cases"].as_array().unwrap().iter().enumerate() {
        assert_eq!(c["case_index"], i);
        assert_eq!(c["independent_fresh_repeat_equal"], true);
        assert_eq!(c["independent_unhooked_equal"], true);
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let s = &c["states"][stage];
            let g = &s["saved_group"];
            assert_eq!(g["source_ordinal"], 210);
            assert_eq!(g["preset"], 4);
            assert_eq!(g["loaded_object_exact"], true);
            assert_eq!(g["gems"][0]["source_ordinal"], 211);
            assert_eq!(g["gems"][0]["loaded_object_exact"], true);
            assert_eq!(
                g["gems"][0]["state"]["catalog"]["primary_effect"],
                "SummonSkeletalSnipersPlayer"
            );
            assert_eq!(
                g["attributes"]["enabled"],
                if i == 2 { "false" } else { "true" }
            );
            assert_eq!(
                g["gems"][0]["source"]["attributes"]["enabled"],
                if i == 3 { "false" } else { "true" }
            );
            if i == 2 {
                for mode in ["MAIN", "CALCS"] {
                    assert_eq!(
                        s["modes"][mode]["main"]["identity"]["effect"],
                        "SummonSkeletalSnipersPlayer"
                    );
                }
            }
            if i == 3 {
                for mode in ["MAIN", "CALCS"] {
                    assert_eq!(
                        s["modes"][mode]["main"]["identity"]["effect"],
                        "MeleeUnarmedPlayer"
                    );
                }
            }
        }
    }
    if !full {
        return;
    }
    for (field, path) in [
        ("observer_sha256", "selected_participation_source.lua"),
        ("authentication_sha256", "djinn_provider_source.lua"),
        ("loader_sha256", "sniper_actor_action_source.lua"),
        ("membership_sha256", "authored_skill_membership_source.lua"),
    ] {
        assert_eq!(
            hash(
                &fs::read(
                    root()
                        .join("crates/poe-optimizer-pob/tests/support")
                        .join(path)
                )
                .unwrap()
            ),
            m[field]
        );
    }
    let mut first = None;
    for pin in v["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"]);
        assert_eq!(hash(&bytes), pin["sha256"]);
        if let Some(prior) = &first {
            assert!(prior == &bytes, "JIT report bytes differ");
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut metadata = report.clone();
        metadata.as_object_mut().unwrap().remove("cases");
        assert!(metadata == *m, "source metadata changed");
        assert!(
            projection(&report) == v["cases"],
            "source projection changed"
        );
    }
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    let e: Value = read("extension.json");
    for row in e["schema"].as_array().unwrap() {
        match row["kind"].as_str().unwrap() {
            "definition" => {
                let d: DefinitionDescriptor = decode(&row["value"]);
                assert_eq!(
                    next.input()
                        .recipe
                        .schema
                        .definitions
                        .iter()
                        .filter(|x| **x == d)
                        .count(),
                    1
                );
            }
            "slot" => {
                let s: SlotDescriptor = decode(&row["value"]);
                assert_eq!(
                    next.input()
                        .recipe
                        .schema
                        .slots
                        .iter()
                        .filter(|x| **x == s)
                        .count(),
                    1
                );
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(
        next.input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| **o == owner())
            .count(),
        1
    );
    let b: Value = read("bindings.json");
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        next.normalization().usage_inputs.as_ref().unwrap();
    let row = physical
        .iter()
        .find(|r| r.gem == decode(&b["gem"]))
        .unwrap();
    assert_eq!(row.policies.len(), 2);
    assert_eq!(
        row.policies
            .iter()
            .filter(|p| **p == read::<OccurrenceUsagePolicy>("usage.json"))
            .count(),
        1
    );
    assert_eq!(
        row.policies
            .iter()
            .filter(|p| p.policy == decode(&b["count_policy"]))
            .count(),
        1
    );
    assert!(next.evaluation().is_none());
    assert_eq!(next.receipt().query_rows, 110);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source(true);
    let a: Value = read("authoring.json");
    assert_eq!(a["status"], "ready");
    assert_eq!(a["before"], json!(prior.receipt().input));
    let e: Value = read("extension.json");
    let b: Value = read("bindings.json");
    let m: OwnedReleaseMigrationInput = decode(
        &json!({"schema_version":5,"before":prior.receipt().input,"release":b["release"],"reason":"requested-skill-participation","contract":{"schema_version":6,"schema_semantics_version":prior.input().recipe.schema.semantics_version,"operations_version":"owned-domain-operations-v21","rule_semantics_version":prior.input().recipe.rules.semantics_version},"schema":e["schema"],"owners":e["owners"],"tables":[],"receivers":[],"query_targets":[]}),
    );
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    super::migration_preservation::assert_import_rebindings_only(prior, &migrated);
    let base = migrated.input();
    let mut normalization = base.normalization.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        normalization.usage_inputs.as_mut().unwrap();
    let row = physical
        .iter_mut()
        .find(|r| r.gem == decode(&b["gem"]))
        .unwrap();
    assert_eq!(row.policies.len(), 1);
    assert_eq!(row.policies[0].policy, decode(&b["count_policy"]));
    row.policies.push(read("usage.json"));
    let usage = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current physical inventory")
    };
    *usage_inputs = usage;
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: base.recipe.clone(),
            successor: base.recipe.clone(),
            mapping: base.mapping.clone(),
            roles: base.roles.clone(),
            normalization: base.normalization.clone(),
            rewards: base.rewards.clone(),
            query_sets: base.query_sets.clone(),
            items: base.items.clone(),
            item_source: base.item_source.clone(),
        },
        base.tree.clone().unwrap(),
        normalization,
        Default::default(),
    )
    .unwrap();
    let mut input = base.clone();
    input.normalization = transition.normalization().clone();
    input.tree = transition.tree().map(|t| t.input().clone());
    let payload: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "extension.json",
        "usage.json",
        "readiness.json",
        "source-vectors.json",
    ]
    .iter()
    .map(|n| read(n))
    .collect();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &payload, 2 * 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    // Undo exactly this one appended usage policy and its dependency commitments.
    let mut inverse = next.input().clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        inverse.normalization.usage_inputs.as_mut().unwrap();
    let row = physical
        .iter_mut()
        .find(|r| r.gem == decode(&b["gem"]))
        .unwrap();
    assert_eq!(
        row.policies.pop().unwrap(),
        read::<OccurrenceUsagePolicy>("usage.json")
    );
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut inverse.normalization.gem_inventory
    else {
        unreachable!()
    };
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        usage_inputs: old, ..
    }) = &base.normalization.gem_inventory
    else {
        unreachable!()
    };
    *usage_inputs = *old;
    inverse.tree = base.tree.clone();
    inverse.provenance = base.provenance.clone();
    assert!(
        inverse == *base,
        "only exact usage append and checked tree/provenance commitments"
    );
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let policy = registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    assert_eq!(policy, decode(&b["policy"]));
    for field in ["group", "occurrence"] {
        assert_eq!(
            registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(
                    policy.clone()
                ))
                .unwrap(),
            decode(&b[field])
        );
    }
    assert_eq!(
        registry.allocate_definition::<StatDefinition>().unwrap(),
        decode(&b["participation"])
    );
    assert_eq!(*registry.input(), next.input().recipe.registry);
    // Restore only the four declared allocations, one owner, and checked
    // dependency identities; all unrelated recipe content remains exact.
    let mut restored = next.input().recipe.clone();
    for row in e["schema"].as_array().unwrap() {
        match row["kind"].as_str().unwrap() {
            "definition" => {
                let expected: DefinitionDescriptor = decode(&row["value"]);
                let pos = restored
                    .schema
                    .definitions
                    .iter()
                    .position(|d| *d == expected)
                    .unwrap();
                restored.schema.definitions.remove(pos);
            }
            "slot" => {
                let expected: SlotDescriptor = decode(&row["value"]);
                let pos = restored
                    .schema
                    .slots
                    .iter()
                    .position(|d| *d == expected)
                    .unwrap();
                restored.schema.slots.remove(pos);
            }
            _ => unreachable!(),
        }
    }
    restored.rules.owners.retain(|o| o.owner != owner().owner);
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert!(
        restored == prior.input().recipe,
        "only four new declarations and one new policy owner may change the recipe"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
