//! Count transport proof and an isolated public-normalizer fixture.
//! The fixture's remapped IDs and empty declarations grant no gameplay coverage.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

#[path = "../../crates/poe-optimizer-import/tests/support/empty_owned_items.rs"]
mod empty_items;

pub const POLICY: &str = "def.000000000000326a";
pub const PREDECESSOR: &str = "2d307fd317100b5282026a38d91c281a937eb850e275664d8c5ddfc9ee2c6560";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ice-nova-count-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn usage() -> PrimarySkillUsageInput {
    read("usage.json")
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn unpack(path: PathBuf, compressed_limit: u64, raw_limit: u64) -> Vec<u8> {
    assert!(fs::metadata(&path).unwrap().len() <= compressed_limit);
    let compressed = fs::read(path).unwrap();
    assert!(compressed.len() as u64 <= compressed_limit);
    let mut raw = vec![];
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(raw_limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= raw_limit);
    raw
}
pub fn source_report() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| {
        let p: Value = read("source-evidence.json");
        let report = &p["report"];
        let bytes = fs::read(data().join(report["file"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len(), report["compressed_bytes"]);
        assert_eq!(hash(&bytes), report["compressed_sha256"]);
        let raw = unpack(
            data().join("occurrences.json.gz"),
            1024 * 1024,
            6 * 1024 * 1024,
        );
        assert_eq!(raw.len(), 5_226_330);
        assert_eq!(report["raw_bytes"], raw.len());
        assert_eq!(
            hash(&raw),
            "ef5fa7366c5c3c5fc98b31a947f52575dc9aebdf51333104a16aaa192dfea574"
        );
        assert_eq!(report["raw_sha256"], hash(&raw));
        serde_json::from_slice(&raw).unwrap()
    })
}
pub fn check_observation_file_pins(pins: &[Value], manifest: &Value) {
    assert_eq!(pins.len(), 12);
    let mut paths = BTreeSet::new();
    for pin in pins {
        let path = pin["path"].as_str().unwrap();
        assert!(paths.insert(path), "duplicate observation source {path}");
        let matches = rows(&manifest["files"])
            .iter()
            .filter(|entry| entry["path"] == path)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "source must have one manifest entry");
        let source = matches[0];
        assert_eq!(
            pin,
            &json!({"path": source["path"], "sha256": source["sha256"]}),
            "observations pin exactly the manifest path and hash"
        );
    }
}
pub fn authenticate_source(proof: &Value) {
    let expected: Value = read("source-evidence.json");
    assert_eq!(
        proof, &expected,
        "the caller cannot alter evidence commitments"
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        hash(&manifest_bytes),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(proof["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    assert_eq!(proof["archives"].as_array().unwrap().len(), 3);
    assert_eq!(
        rows(&proof["archives"])
            .iter()
            .map(|a| a["source"]["path"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("src/Classes/SkillsTab.lua"),
            json!("src/Modules/CalcDefence.lua"),
            json!("src/Modules/Calcs.lua")
        ]
    );
    for a in rows(&proof["archives"]) {
        assert!(rows(&manifest["files"]).contains(&a["source"]));
        let path = root().join(a["file"].as_str().unwrap());
        let raw = unpack(path.clone(), 1024 * 1024, 1024 * 1024);
        let compressed = fs::read(path).unwrap();
        assert_eq!(compressed.len(), a["compressed_bytes"]);
        assert_eq!(hash(&compressed), a["compressed_sha256"]);
        assert_eq!(raw.len(), a["raw_bytes"]);
        assert_eq!(hash(&raw), a["source"]["sha256"]);
        let text = std::str::from_utf8(&raw).unwrap();
        match a["source"]["path"].as_str().unwrap() {
            "src/Classes/SkillsTab.lua" => {
                assert!(text.contains("count = tonumber(child.attrib.count) or 1"));
            }
            "src/Modules/CalcDefence.lua" => {
                assert!(text.contains("elseif activeSkill.socketGroup.groupCount then"));
                assert!(text.contains("return activeSkill.socketGroup.groupCount, true"));
                assert!(text.contains("return gemData.count or 1, true"));
            }
            "src/Modules/Calcs.lua" => {
                assert!(text.contains("activeSkill.socketGroup.includeInFullDPS"));
                assert!(text.contains("calcs.getActiveSkillCount(activeSkill)"));
            }
            other => panic!("unexpected evidence body {other}"),
        }
    }
    let license = fs::read(root().join(proof["license"]["file"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&license), proof["license"]["sha256"]);
    assert_eq!(license.len(), proof["license"]["bytes"]);
    let report = source_report();
    assert_eq!(report["manifest_sha256"], proof["source_manifest_sha256"]);
    assert_eq!(report["source_revision"], proof["source_revision"]);
    for flag in [
        "native_inventory_authority",
        "native_build_parity",
        "universal_count_inertness_claimed",
        "canonical_parity_lifecycle_selected",
        "full_dps_rows_have_physical_identity",
    ] {
        assert_eq!(report[flag], false);
    }
    check_observation_file_pins(rows(&report["files"]), &manifest);
    assert_eq!(rows(&report["cases"]).len(), 23);
    for (index, source) in rows(&report["original_sources"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            index + 1
        );
        assert_eq!(source["path"], path);
        assert_eq!(
            source["sha256"],
            hash(&fs::read(root().join(path)).unwrap())
        );
    }
    for case in rows(&report["cases"]) {
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            for flag in [
                "source_methods_preserved",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(case["states"][stage][flag], true);
            }
        }
    }
}
pub fn check_authored() {
    let authoring_bytes = fs::read(data().join("authoring.json")).unwrap();
    assert_eq!(authoring_bytes.len(), 793);
    assert_eq!(
        hash(&authoring_bytes),
        "ea2a111345e4cdff58dd7dc6f0ec5db8e5eb332a4bf8821d8a9a3a2428273081"
    );
    let a: Value = serde_json::from_slice(&authoring_bytes).unwrap();
    assert_eq!(a["expected_predecessor"], PREDECESSOR);
    assert_eq!(a["physical_occurrences"], json!([0, 0, 0, 0, 4]));
    assert_eq!(a["usage_inventory"], "pending");
    assert_eq!(a["native_mechanics_coverage"], "partial");
    for key in [
        "new_definitions",
        "new_slots",
        "new_rules",
        "new_tables",
        "new_defaults",
    ] {
        assert_eq!(a[key], 0);
    }
    for (file, pin) in a["artifacts"].as_object().unwrap() {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(bytes.len(), pin["bytes"]);
        assert_eq!(hash(&bytes), pin["sha256"]);
    }
    let row = usage();
    assert_eq!(row.gem.key().as_str(), "def.00000000000007dd");
    assert_eq!(row.primary.key().as_str(), "def.00000000000001df");
    assert_eq!(row.supply.slot.key().as_str(), "def.000000000000327e");
    assert_eq!(row.grant.slot.key().as_str(), "def.000000000000327f");
    assert_eq!(row.policies.len(), 1);
    assert_eq!(row.policies[0].policy.key().as_str(), POLICY);
    assert_eq!(row.policies[0].parameters.len(), 1);
    let reference: PrimarySkillUsageInput = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/sniper-reservation/usage.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(row.policies[0].policy, reference.policies[0].policy);
    assert_eq!(
        row.policies[0].parameters[0].slot,
        reference.policies[0].parameters[0].slot
    );
    let UsageValueSource::ContainingGroupOverride {
        group,
        occurrence,
        fallback_admission,
    } = &row.policies[0].parameters[0].source
    else {
        panic!("count source");
    };
    let UsageValueSource::ContainingGroupOverride {
        group: old_group,
        occurrence: old_occurrence,
        ..
    } = &reference.policies[0].parameters[0].source
    else {
        panic!("reference count source");
    };
    let mut group = group.clone();
    group.id = old_group.id.clone();
    assert_eq!(&group, old_group);
    let mut occurrence = occurrence.clone();
    occurrence.id = old_occurrence.id.clone();
    assert_eq!(&occurrence, old_occurrence);
    let UsageFallbackAdmission::UniqueReviewedPrimary { companions } = fallback_admission else {
        panic!("unique source required");
    };
    assert_eq!(companions.len(), 5);
    let catalog_bytes =
        fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap();
    let proof: Value = read("source-evidence.json");
    assert_eq!(hash(&catalog_bytes), proof["catalog_sha256"]);
    let catalog: Value = serde_json::from_slice(&catalog_bytes).unwrap();
    let checked_catalog =
        SkillIdentityCatalog::new(serde_json::from_value(catalog.clone()).unwrap()).unwrap();
    assert_eq!(
        json!(
            poe_optimizer_core::owned_content::digest_owned(
                "owned-skill-source-catalog-v1",
                checked_catalog.data(),
                64 * 1024 * 1024,
            )
            .unwrap()
        ),
        source_report()["catalog_digest"]
    );
    let mapping: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/compiled/mapping.json")).unwrap(),
    )
    .unwrap();
    for companion in companions {
        let matches: Vec<_> = rows(&catalog["gems"])
            .iter()
            .filter(|c| {
                c["game_id"] == companion.game_id && c["variant_id"] == companion.variant_id
            })
            .collect();
        assert_eq!(matches.len(), 1);
        let c = matches[0];
        assert_eq!(c["primary_effect_id"], companion.skill_id);
        assert_eq!(
            c["name_spec"]
                .as_str()
                .unwrap_or(c["name"].as_str().unwrap()),
            companion.name_spec
        );
        assert_ne!(companion.skill_id, row.skill_id);
        for field in ["effect_list", "additional_effects"] {
            assert!(rows(&c[field]).iter().all(|e| e != &row.skill_id));
        }
        for field in [
            "declared_additional_effects",
            "constructed_additional_effects",
            "declared_additional_stat_sets",
        ] {
            assert!(rows(&c[field]).iter().all(|e| e["id"] != row.skill_id));
        }
        let matches: Vec<_> = rows(&mapping["entries"])
            .iter()
            .filter(|e| {
                e["source"]["value"]["value"]["game_id"]["value"] == companion.game_id
                    && e["source"]["value"]["value"]["variant_id"]["value"] == companion.variant_id
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0]["outcome"]["value"]["target"]["value"]["value"],
            json!(companion.gem)
        );
    }
    let deps: Value = read("dependencies.json");
    let descriptor = rows(&deps["definitions"])
        .iter()
        .find(|d| d["kind"] == "usage_policy")
        .unwrap();
    assert_eq!(descriptor["value"]["id"], json!(row.policies[0].policy));
    assert_eq!(
        descriptor["value"]["schema"]["value"]["targets"],
        json!(["skill"])
    );
    let parameter = rows(&deps["slots"])
        .iter()
        .find(|d| d["kind"] == "parameter")
        .unwrap();
    assert_eq!(
        parameter["value"]["id"],
        json!(row.policies[0].parameters[0].slot)
    );
    assert_eq!(
        parameter["value"]["schema"]["value"]["value"],
        json!({"kind":"integer","value":{"minimum":0,"maximum":4}})
    );
    let gem_schema = rows(&deps["definitions"])
        .iter()
        .find(|d| d["kind"] == "gem")
        .unwrap();
    assert_eq!(gem_schema["value"]["id"], json!(row.gem));
    let decl = &gem_schema["value"]["schema"]["value"]["declarations"];
    assert!(rows(&decl["grants"]["members"]).contains(&json!(row.grant)));
    assert!(rows(&decl["skill_grants"]["members"]).contains(&json!(row.supply)));
    let grant_schema = rows(&deps["slots"])
        .iter()
        .find(|d| d["kind"] == "grant")
        .unwrap();
    assert_eq!(grant_schema["value"]["id"], json!(row.grant));
    assert_eq!(
        grant_schema["value"]["schema"]["value"]["target"],
        json!({"kind":"skill","value":row.supply})
    );
    let supply_schema = rows(&deps["slots"])
        .iter()
        .find(|d| d["kind"] == "skill_grant")
        .unwrap();
    assert_eq!(supply_schema["value"]["id"], json!(row.supply));
    assert_eq!(
        supply_schema["value"]["schema"]["value"]["skill"],
        json!(row.primary)
    );
    authenticate_source(&proof);
}

/// No dependency rebinding or inventory closure is hidden here. Publication
/// authenticates its predecessor, invokes this append, then performs its normal
/// shared normalization transition and complete inverse checks.
pub fn checked_append(policy: &mut NormalizationPolicy) -> Result<(), String> {
    check_authored();
    let expected = usage();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } = policy
        .usage_inputs
        .as_mut()
        .ok_or("existing usage policy missing")?;
    let indices: Vec<_> = physical
        .iter()
        .enumerate()
        .filter(|(_, r)| r.gem == expected.gem)
        .map(|(i, _)| i)
        .collect();
    if indices.len() != 1 {
        return Err("append requires one existing physical source".into());
    }
    let row = &mut physical[indices[0]];
    let mut frame = row.clone();
    frame.policies = expected.policies.clone();
    if frame != expected {
        return Err("existing source/frame/supply differs".into());
    }
    if row
        .policies
        .iter()
        .any(|p| p.policy == expected.policies[0].policy)
    {
        return Err("requested-count policy already present".into());
    }
    row.policies.extend(expected.policies);
    Ok(())
}

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn replace_ids(value: &mut Value, replacements: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(new) = replacements.get(s) {
                *s = new.clone();
            }
        }
        Value::Array(a) => {
            for v in a {
                replace_ids(v, replacements);
            }
        }
        Value::Object(o) => {
            for v in o.values_mut() {
                replace_ids(v, replacements);
            }
        }
        _ => {}
    }
}
pub struct Fixture {
    pub policy: NormalizationPolicy,
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    rewards: OwnedRewardPolicy,
}
impl Fixture {
    pub fn new() -> Self {
        let original = usage();
        let namespace = original.gem.namespace().clone();
        let mut registry = OwnedIdRegistry::empty(namespace.clone(), Default::default()).unwrap();
        let base = registry.identity().unwrap();
        let deps: Value = read("dependencies.json");
        let old_roles: Vec<OwnedGemRoleRow> =
            serde_json::from_value(deps["roles"].clone()).unwrap();
        let mut replacements = BTreeMap::new();
        let mut roles = vec![];
        for old in &old_roles {
            let OwnedPrimarySkill::Known(old_skill) = &old.primary else {
                panic!();
            };
            let gem = registry.allocate_definition::<GemDefinition>().unwrap();
            let skill = registry.allocate_definition::<SkillDefinition>().unwrap();
            replacements.insert(
                old.gem.key().as_str().to_owned(),
                gem.key().as_str().to_owned(),
            );
            replacements.insert(
                old_skill.key().as_str().to_owned(),
                skill.key().as_str().to_owned(),
            );
            roles.push(OwnedGemRoleRow {
                construction: poe_optimizer_import::owned_skill_catalog::OwnedGemSourceConstruction::Unavailable,
                gem,
                primary: OwnedPrimarySkill::Known(skill),
                role: old.role.clone(),
                materialization: old.materialization.clone(),
            });
        }
        let main = old_roles
            .iter()
            .position(|r| r.gem == original.gem)
            .unwrap();
        let owner = SlotOwnerDefId::Gem(roles[main].gem.clone());
        let supply = registry
            .allocate_slot::<SkillGrantSlotDefinition>(owner.clone())
            .unwrap();
        let grant = registry
            .allocate_slot::<GrantSlotDefinition>(owner)
            .unwrap();
        let policy_id = registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let parameter = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(
                policy_id.clone(),
            ))
            .unwrap();
        for (old, new) in [
            (original.supply.slot.key(), supply.slot.key()),
            (original.grant.slot.key(), grant.slot.key()),
            (original.policies[0].policy.key(), policy_id.key()),
            (
                original.policies[0].parameters[0].slot.slot.key(),
                parameter.slot.key(),
            ),
        ] {
            replacements.insert(old.as_str().to_owned(), new.as_str().to_owned());
        }
        let mut row = json!(original);
        replace_ids(&mut row, &replacements);
        let row: PrimarySkillUsageInput = serde_json::from_value(row).unwrap();
        let mut definitions = vec![];
        for role in &roles {
            let OwnedPrimarySkill::Known(skill) = &role.primary else {
                panic!();
            };
            let OwnedGemRole::Known(authored_role) = &role.role else {
                panic!();
            };
            let mut decl = declarations();
            if role.gem == row.gem {
                decl.grants = DeclaredSet::complete(vec![grant.clone()]);
                decl.skill_grants = DeclaredSet::complete(vec![supply.clone()]);
            }
            definitions.push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: role.gem.clone(),
                schema: SchemaState::Known(GemSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(40).unwrap(),
                    },
                    roles: vec![*authored_role],
                    skills: DeclaredSet::complete(vec![skill.clone()]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Optional,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: decl,
                }),
            }));
            definitions.push(DefinitionDescriptor::Skill(DefinitionEntry {
                id: skill.clone(),
                schema: SchemaState::Known(SkillSchema {
                    directly_selectable: false,
                    declarations: declarations(),
                }),
            }));
        }
        let mut decl = declarations();
        decl.parameters = DeclaredSet::complete(vec![parameter.clone()]);
        definitions.push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            id: policy_id,
            schema: SchemaState::Known(UsagePolicySchema {
                targets: vec![UsageTargetKind::Skill],
                declarations: decl,
                scenario_skill_applicability: None,
            }),
        }));
        let slots = vec![
            SlotDescriptor::SkillGrant(DefinitionEntry {
                id: supply.clone(),
                schema: SchemaState::Known(SkillGrantSlotSchema {
                    skill: row.primary.clone(),
                    outputs: DeclaredSet::complete(vec![]),
                    preset_inputs: None,
                }),
            }),
            SlotDescriptor::Grant(DefinitionEntry {
                id: grant,
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(supply),
                }),
            }),
            SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter,
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Integer(IntegerRange {
                        minimum: BoundedInteger::new(0).unwrap(),
                        maximum: BoundedInteger::new(4).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                    skill_input: None,
                }),
            }),
        ];
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("isolated-count-transport-fixture"),
                semantics_version: key("input-only-no-gameplay-proof"),
                definitions,
                slots,
            },
            Default::default(),
        )
        .unwrap();
        let imported_roles: Value = serde_json::from_slice(
            &fs::read(root().join("data/owned/poe2/3887ae68/import/compiled/roles.json")).unwrap(),
        )
        .unwrap();
        let compilation: SkillCatalogReceipt =
            serde_json::from_value(imported_roles["compilation"].clone()).unwrap();
        let mut sources = vec![(row.gem.clone(), row.game_id.clone(), row.variant_id.clone())];
        let UsageValueSource::ContainingGroupOverride {
            fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
            ..
        } = &row.policies[0].parameters[0].source
        else {
            panic!();
        };
        sources.extend(
            companions
                .iter()
                .map(|c| (c.gem.clone(), c.game_id.clone(), c.variant_id.clone())),
        );
        let entries = sources
            .into_iter()
            .map(|(gem, game, variant)| MappingEntry {
                source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                    game_id: SourceComponent::Text(game),
                    variant_id: SourceComponent::Text(variant),
                }),
                outcome: MappingOutcome::Mapped {
                    target: SchemaSubject::Definition(gem.address()),
                    basis: MappingBasis::Exact,
                },
            })
            .collect();
        let mapping = OwnedMappingIndex::new(
            MappingPackageInput {
                schema_version: OWNED_MAPPING_PACKAGE_VERSION,
                namespace: namespace.clone(),
                registry: registry.identity().unwrap(),
                definitions: schema.identity().clone(),
                source: compilation.source.clone(),
                policy_version: compilation.policy.version.clone(),
                entries,
            },
            &registry,
            &schema,
            Default::default(),
        )
        .unwrap();
        let roles = OwnedSkillRoleIndex::new(
            OwnedSkillRolePackageInput {
                reverse_primary: OwnedReversePrimaryCensus::Unavailable {},
                schema_version: OWNED_SKILL_ROLE_VERSION,
                namespace: namespace.clone(),
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                compilation: SkillCatalogReceipt {
                    source: compilation.source.clone(),
                    catalog_digest: compilation.catalog_digest,
                    policy: compilation.policy,
                    base_registry: base,
                    staged_registry: registry.identity().unwrap(),
                    gem_count: roles.len(),
                    skill_count: roles.len(),
                },
                roles,
            },
            &mapping,
            &schema,
            Default::default(),
        )
        .unwrap();
        let seed: Value = serde_json::from_slice(
            &fs::read(
                root().join("data/owned/poe2/3887ae68/import/normalization-policy-seed.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut minimal = serde_json::Map::new();
        for field in [
            "version",
            "namespace",
            "character_level",
            "gem_level",
            "gem_enabled",
            "group_enabled",
            "manual_skill_sources",
            "empty_item_keys",
            "generated_support_prefixes",
            "allocation_attribute",
            "single_active_support_target",
        ] {
            minimal.insert(field.to_owned(), seed[field].clone());
        }
        minimal.insert("equipment_loadouts".into(), json!([]));
        minimal.insert("gem_quality".into(), json!({"kind":"unconverted"}));
        let mut policy: NormalizationPolicy =
            serde_json::from_value(Value::Object(minimal)).unwrap();
        policy.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions: schema.identity().clone(),
            source: compilation.source,
            roles: *roles.identity(),
            catalog: compilation.catalog_digest,
            scalar_inputs: gem_inventory_scalar_inputs_identity(&policy, Default::default())
                .unwrap(),
            physical: vec![row],
            occurrences: vec![],
            scenario: vec![],
            inventory: None,
        });
        let rewards = OwnedRewardPolicy::new(
            RewardPolicyInput {
                schema_version: OWNED_REWARD_POLICY_VERSION,
                namespace,
                version: key("empty-reward-fixture"),
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                rules: vec![],
            },
            &mapping,
            &schema,
            Default::default(),
        )
        .unwrap();
        Self {
            policy,
            registry,
            schema,
            mapping,
            roles,
            rewards,
        }
    }
    pub fn row(&self) -> &PrimarySkillUsageInput {
        let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
            self.policy.usage_inputs.as_ref().unwrap();
        &physical[0]
    }
    pub fn run(&self, xml: &str) -> Result<NormalizedImport, NormalizationError> {
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([0xc9; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        normalize_fresh(
            &evidence,
            *imported.allocator_state(),
            NormalizationArtifacts {
                mappings: &self.mapping,
                registry: &self.registry,
                definitions: &self.schema,
                roles: &self.roles,
                rewards: &self.rewards,
                items: &empty_items::empty_items(&self.schema),
                item_source: &empty_items::empty_item_source(&self.schema),
                tree: None,
            },
            &self.policy,
            &[],
            Default::default(),
        )
    }
}
pub fn gem(row: &PrimarySkillUsageInput, count: Option<&str>) -> String {
    format!(
        r#"<Gem gemId="{}" variantId="{}" skillId="{}" nameSpec="{}" level="17" enabled="true"{}/>"#,
        row.game_id,
        row.variant_id,
        row.skill_id,
        row.name_spec,
        count
            .map(|v| format!(r#" count="{v}""#))
            .unwrap_or_default()
    )
}
pub fn group(gems: &str, attributes: &str) -> String {
    format!(r#"<Skill enabled="true" {attributes}>{gems}</Skill>"#)
}
pub fn xml(groups: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="7"><SkillSet id="7">{groups}</SkillSet></Skills></PathOfBuilding2>"#
    )
}
pub fn values(result: &NormalizedImport) -> Vec<Vec<Option<i64>>> {
    result
        .draft()
        .input()
        .skill_presets
        .members
        .iter()
        .map(|preset| {
            let usage = preset.usage_preferences.as_ref().unwrap();
            assert!(matches!(
                usage.completion,
                DraftListCompletion::Pending { .. }
            ));
            usage
                .members
                .iter()
                .map(|u| {
                    match u
                        .parameters
                        .members
                        .first()
                        .and_then(|p| p.value.to_resolved())
                    {
                        Some(ParameterValue::Integer(v)) => {
                            assert_eq!(u.parameters.members.len(), 1);
                            assert!(matches!(
                                u.parameters.completion,
                                DraftListCompletion::Complete
                            ));
                            Some(v.get())
                        }
                        None => {
                            assert!(u.parameters.members.is_empty());
                            assert!(matches!(
                                u.parameters.completion,
                                DraftListCompletion::Pending { .. }
                            ));
                            None
                        }
                        other => panic!("wrong value {other:?}"),
                    }
                })
                .collect()
        })
        .collect()
}
