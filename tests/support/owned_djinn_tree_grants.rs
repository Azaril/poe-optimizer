//! Reviewed allocation supply only. Usage ownership and effective inputs stay open.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_normalize::NormalizationPolicy,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/djinn-tree-grants")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn bindings() -> Value {
    read("bindings.json")
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b = bindings();
    let dependencies: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.schema.len(), 6);
    assert_eq!(m.owners.len(), 2);
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(a["registry_last_issued_before"], 0x32d1);
    assert_eq!(a["registry_last_issued_after"], 0x32d5);
    assert_eq!(a["allocated_definitions"], 4);
    for field in ["before", "definitions", "registry", "roles"] {
        assert_eq!(b[field], a[field]);
    }
    assert_eq!(dependencies["source"]["input"], a["before"]);
    assert_eq!(dependencies["source"]["definitions"], a["definitions"]);
    let names: Vec<_> = a["artifact_sha256"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["bindings", "dependencies", "migration"]);
    for name in names {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(
            a["artifact_sha256"][name],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert!(
        !fs::read(data().join("authoring.json"))
            .unwrap()
            .contains(&b'\r')
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 15);
    let mut paths = BTreeSet::new();
    for pin in pins {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    for pin in a["source_witness_files"].as_array().unwrap() {
        assert!(pins.contains(pin));
    }
    let rows = b["families"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let schema = json!(m.schema);
    for (index, family) in rows.iter().enumerate() {
        let prior = dependencies["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["value"]["id"] == family["passive"])
            .unwrap();
        let mut expected = prior.clone();
        let declarations = &mut expected["value"]["schema"]["value"]["declarations"];
        for (field, slot) in [("grants", "grant"), ("skill_grants", "supply")] {
            assert!(
                declarations[field]["members"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(declarations[field]["closure"]["kind"], "partial");
            declarations[field]["members"] = json!([family[slot]]);
        }
        assert_eq!(schema[index], json!({"kind":"definition","value":expected}));
        let supply = &schema[2 + index * 2]["value"]["value"];
        let grant = &schema[3 + index * 2]["value"]["value"];
        assert_eq!(schema[2 + index * 2]["kind"], "slot");
        assert_eq!(schema[2 + index * 2]["value"]["kind"], "skill_grant");
        assert_eq!(supply["id"], family["supply"]);
        assert_eq!(supply["schema"]["value"]["skill"], family["skill"]);
        assert!(
            supply["schema"]["value"]["outputs"]["members"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            supply["schema"]["value"]["outputs"]["closure"]["kind"],
            "partial"
        );
        assert_eq!(grant["id"], family["grant"]);
        assert_eq!(
            grant["schema"]["value"]["provider_roles"],
            json!(["allocation"])
        );
        assert_eq!(
            grant["schema"]["value"]["target"],
            json!({"kind":"skill","value":family["supply"]})
        );
        let owner = &m.owners[index];
        assert!(!owner.programs.is_complete());
        assert_eq!(
            json!(owner.owner),
            json!({"kind":"definition","value":{"kind":"passive_node","value":family["passive"]}})
        );
        assert_eq!(owner.programs.members.len(), 1);
        let program = json!(owner.programs.members[0]);
        assert_eq!(program["id"], family["program"]);
        assert_eq!(program["context"], "actor");
        assert!(program["reads"].as_array().unwrap().is_empty());
        assert_eq!(program["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(program["nodes"][0]["id"], "raw-level");
        let literal = &program["nodes"][0]["expression"];
        assert_eq!(literal["kind"], "literal");
        assert_eq!(literal["value"]["kind"], "quantity");
        assert_eq!(literal["value"]["value"]["value"].as_f64(), Some(1.0));
        let unit = &literal["value"]["value"]["unit"];
        assert_eq!(
            dependencies["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| d["value"]["id"] == *unit
                    && d["value"]["schema"]["value"]["dimension"] == "count")
                .count(),
            1
        );
        assert_eq!(
            program["nodes"][1],
            json!({"id":"provided","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}})
        );
        assert_eq!(
            program["effects"],
            json!([
                {"id":"raw-grant-level","when":null,"effect":{"kind":"project_skill_parameter","skill":family["supply"],"parameter":family["raw_level"],"value":"raw-level"}},
                {"id":"tree-grant","when":null,"effect":{"kind":"activate_grant","slot":family["grant"],"enabled":"provided"}}
            ])
        );
        // No quality, effective-level, actor-stat or action-output producer.
        for field in ["raw_level", "raw_quality"] {
            let slot = dependencies["slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["value"]["id"] == family[field])
                .unwrap();
            assert_eq!(
                slot["value"]["schema"]["value"]["skill_input"],
                "authored_or_projected"
            );
            assert_eq!(
                slot["value"]["schema"]["value"]["presence"],
                "required_once"
            );
        }
    }
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = Vec::new();
    for (path, bytes, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let raw = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(raw.len() as u64, proof[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), proof[hash]);
        reports.push(raw);
    }
    assert!(reports[0] == reports[1], "both complete JIT reports agree");
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(report["source_revision"], a["source_revision"]);
    assert_eq!(report["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(
        report["evidence"]["manifest_sha256"],
        a["source_manifest_sha256"]
    );
    assert_eq!(report["evidence"]["files"], a["source_witness_files"]);
    assert_eq!(
        report["evidence"]["tree_sha256"],
        a["source_tree_json_sha256"]
    );
    assert_eq!(report["evidence"]["native_parity"], false);
    assert_eq!(report["evidence"]["no_business_wrappers"], true);
    assert_eq!(report["evidence"]["complete_loads_per_jit"], 13);
    assert_eq!(report["evidence"]["originals"], a["original_sources"]);
    for original in a["original_sources"].as_array().unwrap() {
        let bytes = fs::read(
            root()
                .join("tests/fixtures/builds/breadth-20260908")
                .join(original["name"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
    let expected_names = [
        "original",
        "remove-sand-allocation",
        "remove-sand-saved-tree-group",
        "disable-sand-manual",
        "level-one-sand-manual",
        "disable-sand-bidding",
        "remove-water-allocation",
        "remove-water-saved-tree-group",
        "disable-water-manual",
        "level-one-water-manual",
        "disable-water-bidding",
        "sand-main-minion-two",
        "archived-manual-changes",
    ];
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_names.len());
    let b = bindings();
    for (case, name) in cases.iter().zip(expected_names) {
        assert_eq!(case["name"], name);
        for flag in [
            "original_functions_preserved",
            "saved_instances_preserved",
            "selected_state_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(case["state"][flag], true);
        }
        for mode in ["MAIN", "CALCS"] {
            let grants = case["state"]["modes"][mode]["grants"].as_array().unwrap();
            for family in b["families"].as_array().unwrap() {
                let matched: Vec<_> = grants
                    .iter()
                    .filter(|g| g["source_node_id"] == family["node_id"])
                    .collect();
                if name == format!("remove-{}-allocation", family["key"].as_str().unwrap()) {
                    assert!(matched.is_empty());
                    continue;
                }
                assert_eq!(matched.len(), 1);
                let grant = matched[0];
                assert_eq!(grant["source_node_exact"], true);
                let fields = json!({"level":1,"noSupports":true,"skillId":family["source_effect"],"source":format!("Tree:{}",family["node_id"].as_u64().unwrap())});
                assert_eq!(grant["fields"], fields);
                assert_eq!(grant["node_grants"], json!([fields]));
            }
        }
    }
    assert_eq!(cases[0]["xml_sha256"], a["original_sources"][4]["sha256"]);
}

fn preserve(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, m: &OwnedReleaseMigrationInput) {
    let old = prior.input();
    let new = next.input();
    let mut expected = old.recipe.schema.clone();
    expected.release = m.release.clone();
    for row in &m.schema {
        match row {
            SchemaExtensionEntry::Definition(d) => {
                let entry = expected
                    .definitions
                    .iter_mut()
                    .find(|x| x.address() == d.address())
                    .unwrap();
                *entry = d.clone();
            }
            SchemaExtensionEntry::Slot(s) => {
                assert!(!expected.slots.iter().any(|x| x.address() == s.address()));
                expected.slots.push(s.clone());
            }
        }
    }
    expected
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    expected.slots.sort_by_cached_key(SlotDescriptor::address);
    assert!(
        expected == new.recipe.schema,
        "only two passive revisions and four declared slots"
    );
    let mut expected = old.recipe.rules.clone();
    expected.definitions = next.receipt().definitions.clone();
    for row in &m.owners {
        let owner = expected
            .owners
            .iter_mut()
            .find(|x| x.owner == row.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, row.programs.closure);
        assert!(owner.programs.members.is_empty());
        owner.programs.members = row.programs.members.clone();
    }
    assert!(
        expected == new.recipe.rules,
        "all old programs, tables, receivers and application registry remain exact"
    );
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32d5);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 4
    );
    assert_eq!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()],
        old.recipe.registry.entries
    );
    assert_eq!(new.query_sets, old.query_sets);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    let old_value = json!(old);
    let mut restored = json!(new);
    // Checked release assembly authenticates each dependency before these exact
    // rebind locations are restored. No recursive identity stripping is used.
    for path in [
        "/recipe/schema",
        "/recipe/rules",
        "/recipe/registry",
        "/recipe/routing/definitions",
        "/mapping/definitions",
        "/mapping/registry",
        "/roles/definitions",
        "/roles/mapping",
        "/rewards/definitions",
        "/rewards/mapping",
        "/items/definitions",
        "/item_source/item_lines",
        "/tree/definitions",
        "/tree/registry",
        "/tree/mapping",
        "/tree/normalization",
        "/normalization/gem_quality/value/definitions",
        "/normalization/gem_inputs/definitions",
        "/normalization/direct_skill_inputs/definitions",
        "/normalization/direct_skill_inputs/roles",
        "/normalization/gem_inventory/definitions",
        "/normalization/gem_inventory/roles",
        "/normalization/gem_inventory/scalar_inputs",
        "/normalization/gem_inventory/usage_inputs",
        "/normalization/usage_inputs/definitions",
        "/normalization/usage_inputs/roles",
        "/normalization/usage_inputs/scalar_inputs",
        "/normalization/support_origin_order/roles",
        "/normalization/payload_inventory/roles",
        "/normalization/skill_inventory/roles",
        "/normalization/skill_inventory/direct_inputs",
        "/normalization/configuration_reward_inventory/reward_policy",
        "/normalization/equipment_membership/definitions",
        "/normalization/equipment_membership/item_lines",
        "/normalization/equipment_membership/item_source",
        "/normalization/item_modifier_membership/definitions",
        "/normalization/item_modifier_membership/item_lines",
        "/normalization/item_modifier_membership/item_source",
        "/normalization/item_parameter_inputs/definitions",
        "/normalization/item_parameter_inputs/item_lines",
        "/normalization/item_parameter_inputs/item_source",
        "/normalization/passive_socket_membership/definitions",
        "/normalization/passive_socket_membership/mapping",
        "/normalization/passive_socket_membership/item_lines",
        "/normalization/passive_socket_membership/item_source",
        "/normalization/passive_socket_membership/equipment",
    ] {
        *restored
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("new {path}")) = old_value
            .pointer(path)
            .unwrap_or_else(|| panic!("old {path}"))
            .clone();
    }
    for list in [
        "/normalization/gem_inventory/primary_dispositions",
        "/normalization/direct_skill_inputs/dispositions",
    ] {
        let count = old_value.pointer(list).unwrap().as_array().unwrap().len();
        assert_eq!(
            restored.pointer(list).unwrap().as_array().unwrap().len(),
            count
        );
        for index in 0..count {
            for field in ["definitions", "roles"] {
                let path = format!("{list}/{index}/reference_action/{field}");
                *restored.pointer_mut(&path).unwrap() = old_value.pointer(&path).unwrap().clone();
            }
        }
    }
    // Preserve finite f64 values through their actual typed policy representation.
    let policy: NormalizationPolicy =
        serde_json::from_value(restored["normalization"].clone()).unwrap();
    assert_eq!(policy, old.normalization);
    restored["normalization"] = old_value["normalization"].clone();
    assert_eq!(new.provenance.len(), old.provenance.len() + 1);
    assert_eq!(new.provenance[..old.provenance.len()], old.provenance);
    restored["provenance"].as_array_mut().unwrap().pop();
    assert!(
        restored == old_value,
        "every other prior release field is unchanged"
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "roles", "normalization"] {
        assert_eq!(receipt[field], a[field]);
    }
    let dependencies: Value = read("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    for row in definitions {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let slots: Vec<SlotDescriptor> = serde_json::from_value(dependencies["slots"].clone()).unwrap();
    for row in slots {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    preserve(prior, &migrated, &migration);
    let mut full = migrated.input().clone();
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-djinn-tree-grants"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-djinn-tree-grants-v1",
            &(a, bindings(), dependencies),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.provenance.pop();
    assert!(
        restored == *migrated.input(),
        "only authenticated source provenance appended"
    );
    next
}
