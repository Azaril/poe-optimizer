//! Offline Warrior source correspondence. Numerical owners and every deferred
//! gameplay/input obligation remain open; this is not a build evaluator.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_item_lines::{ItemLineRule, OwnedItemLinePolicy},
    owned_item_source::{
        ItemRuleSourceLayout, ItemRuleSourceRole, ItemSourceConditionalMember, ItemSourceDialect,
        ItemSourceLayoutPolicy,
    },
    owned_normalize::{
        GemInventoryPolicy, GeneratedSkillInputPolicy, GeneratedSkillInputRule,
        PrimaryGemInputDisposition, equipment_membership_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/warrior-generated-correspondence")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
pub fn check_authored() {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let a: Value = read("authoring.json");
    assert_eq!(json!(migration.before), a["before"]);
    assert_eq!(migration.schema_version, 5);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v27"
    );
    assert_eq!(migration.schema.len(), 16);
    assert!(
        migration.tables.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    assert_eq!(migration.owners.len(), 2);
    assert!(migration.owners.iter().all(|o| !o.programs.is_complete()));
    let schema = json!(migration.schema);
    let actor = schema
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["kind"] == "definition"
                && row["value"]["value"]["id"]["key"] == "def.0000000000003366"
        })
        .unwrap();
    let declarations = &actor["value"]["value"]["schema"]["value"]["declarations"];
    // Actor-level authored parameters/sockets are not supported by the current
    // schema. Their required empty representation is not numerical coverage.
    for field in ["parameters", "sockets"] {
        assert_eq!(
            declarations[field],
            json!({"members":[],"closure":{"kind":"complete"}})
        );
    }
    for field in ["choices", "grants", "actors", "skill_grants", "outputs"] {
        assert_eq!(declarations[field]["closure"]["kind"], "partial");
    }
    let d: PrimaryGemInputDisposition = read("disposition.json");
    let g: GeneratedSkillInputRule = read("generated-input.json");
    assert_eq!(d.physical.gem, g.gem);
    for (left, right) in [
        (&d.physical.game_id, &g.game_id),
        (&d.physical.variant_id, &g.variant_id),
        (&d.physical.skill_id, &g.skill_id),
        (&d.physical.name_spec, &g.name_spec),
    ] {
        assert_eq!(left, right);
    }
    let r = json!(d.reference_action);
    assert_eq!(r["kind"], "pob_physical_singleton_minion_actions_v1");
    assert_eq!(r["primary"], json!(g.skill));
    assert_eq!(r["minion"]["source_id"], "RaisedSkeletonWarriors");
    assert_eq!(r["actions"].as_array().unwrap().len(), 1);
    assert_eq!(r["actions"][0]["skill_id"], "MinionMeleeStep");
    let dependencies: Value = read("dependencies.json");
    assert_eq!(
        r["actions"][0]["output"],
        dependencies["child_disposition"]["output"]
    );
    assert_eq!(dependencies["role"]["materialization"]["kind"], "physical");
    let scalar = &dependencies["physical_scalar"]["parameters"];
    assert_eq!(scalar.as_array().unwrap().len(), 2);
    assert_eq!(scalar[0]["slot"], json!(d.physical.corrupted));
    assert_eq!(scalar[1]["slot"], json!(d.physical.corruption_level));
    assert_eq!(a["allocated_keys"].as_array().unwrap().len(), 13);
    assert_eq!(a["allocated_keys"][12], "def.000000000000336a");
}
pub fn check_source() {
    let packet: Value = read("source-vectors.json");
    for file in packet["capture_helpers"].as_array().unwrap() {
        let bytes = fs::read(root().join(file["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), file["sha256"]);
    }
    for (file, field) in [
        (
            "crates/poe-optimizer-pob/tests/support/warrior_generated_source.lua",
            "observer_sha256",
        ),
        (
            "crates/poe-optimizer-pob/tests/support/warrior_generated_source.rs",
            "test_sha256",
        ),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(root().join(file)).unwrap())),
            packet[field]
        );
    }
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&manifest)),
        packet["manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["upstream_revision"], packet["source_revision"]);
    for file in packet["files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(file));
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(file["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            file["sha256"]
        );
    }
    let mut raw = None;
    for report in packet["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), report["sha256"]);
        assert_eq!(bytes.len() as u64, report["bytes"]);
        if let Some(prior) = &raw {
            assert_eq!(prior, &bytes);
        } else {
            raw = Some(bytes.clone());
        }
        let source: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(source["attempted_full_loads"], 16);
        assert_eq!(source["cases"].as_array().unwrap().len(), 8);
        assert!(
            source["cases"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["uninstrumented_equal"] == true)
        );
        assert_eq!(source["files"], packet["files"]);
        assert_eq!(source["observer_sha256"], packet["observer_sha256"]);
        assert_eq!(source["test_sha256"], packet["test_sha256"]);
        for observation in packet["observations"].as_array().unwrap() {
            assert_eq!(
                source
                    .pointer(observation["pointer"].as_str().unwrap())
                    .unwrap(),
                &observation["value"]
            );
        }
    }
    assert!(raw.is_some());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source();
    let dependencies: Value = read("dependencies.json");
    let mut migration: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(prior.receipt().input, migration.before);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x335d);
    for descriptor in dependencies["schema"].as_array().unwrap() {
        assert!(
            json!(prior.input().recipe.schema.definitions)
                .as_array()
                .unwrap()
                .contains(descriptor)
        );
    }
    for descriptor in dependencies["slots"].as_array().unwrap() {
        assert!(
            json!(prior.input().recipe.schema.slots)
                .as_array()
                .unwrap()
                .contains(descriptor)
        );
    }
    assert!(
        json!(prior.input().roles.roles)
            .as_array()
            .unwrap()
            .contains(&dependencies["role"])
    );
    assert!(
        json!(prior.normalization().gem_inputs)
            .get("gems")
            .unwrap()
            .as_array()
            .unwrap()
            .contains(&dependencies["physical_scalar"])
    );
    // Authoring is tied to an exact predecessor; no runtime compatibility branch.
    migration.before = prior.receipt().input;
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut full = migrated.input().clone();
    let definitions = migrated.receipt().definitions.clone();
    let roles = migrated.receipt().roles;
    let mut disposition: Value = read("disposition.json");
    disposition["reference_action"]["definitions"] = json!(definitions);
    disposition["reference_action"]["roles"] = json!(roles);
    let disposition: PrimaryGemInputDisposition = serde_json::from_value(disposition).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        ..
    }) = &mut full.normalization.gem_inventory
    else {
        panic!("physical proof inventory")
    };
    assert!(
        !primary_dispositions
            .iter()
            .any(|d| d.physical.gem == disposition.physical.gem)
    );
    primary_dispositions.push(disposition.clone());
    let generated: GeneratedSkillInputRule = read("generated-input.json");
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows, .. }) =
        &mut full.normalization.generated_skill_inputs
    else {
        panic!("same generated input policy")
    };
    assert!(!rows.iter().any(|r| r.gem == generated.gem));
    rows.push(generated);
    let item_rules: Vec<ItemLineRule> = read("item-rules.json");
    for row in &item_rules {
        assert!(!full.items.rules.iter().any(|r| r.id == row.id));
        full.items.rules.push(row.clone());
    }
    full.items.version = key("warrior-grant-source-lines-v1");
    let items = OwnedItemLinePolicy::new(
        full.items.clone(),
        migrated.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    full.item_source.item_lines = *items.identity();
    full.item_source.version = key("warrior-grant-source-frame-v1");
    let conditions: Vec<ItemSourceConditionalMember> = read("source-conditions.json");
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut full.item_source.dialect
    else {
        panic!("reviewed attributed source")
    };
    for row in conditions {
        assert!(
            !single_modifier_conditions
                .iter()
                .any(|r| r.rule == row.rule)
        );
        single_modifier_conditions.push(row);
    }
    for row in item_rules {
        full.item_source.rule_layouts.push(ItemRuleSourceLayout {
            rule: row.id,
            role: ItemRuleSourceRole::Unresolved,
        });
    }
    let source = ItemSourceLayoutPolicy::new(
        full.item_source.clone(),
        &items,
        migrated.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    // Rebind only the explicit source commitments of existing checked adapters.
    let mut normalization = json!(full.normalization);
    for policy in [
        "equipment_membership",
        "item_modifier_membership",
        "item_parameter_inputs",
        "passive_socket_membership",
    ] {
        assert!(normalization[policy].get("item_lines").is_some());
        assert!(normalization[policy].get("item_source").is_some());
        normalization[policy]["item_lines"] = json!(items.identity());
        normalization[policy]["item_source"] = json!(source.identity());
    }
    full.normalization = serde_json::from_value(normalization).unwrap();
    let equipment = equipment_membership_identity(
        full.normalization.equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut normalization = json!(full.normalization);
    normalization["passive_socket_membership"]["equipment"] = json!(equipment);
    full.normalization = serde_json::from_value(normalization).unwrap();
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            migrated.assembled().registry(),
            migrated.assembled().schema(),
            migrated.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("warrior-generated-correspondence"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-warrior-generated-correspondence-v1",
            &(
                migration,
                read::<Value>("authoring.json"),
                read::<Value>("disposition.json"),
                read::<Value>("generated-input.json"),
                read::<Value>("item-rules.json"),
                read::<Value>("source-conditions.json"),
                read::<Value>("source-vectors.json"),
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    let mut normalization = json!(restored.normalization);
    let old_normalization = json!(migrated.normalization());
    let removed = normalization["gem_inventory"]["primary_dispositions"]
        .as_array_mut()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(removed, json!(disposition));
    let removed = normalization["generated_skill_inputs"]["rows"]
        .as_array_mut()
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(removed, read::<Value>("generated-input.json"));
    for policy in [
        "equipment_membership",
        "item_modifier_membership",
        "item_parameter_inputs",
        "passive_socket_membership",
    ] {
        for field in ["item_lines", "item_source"] {
            normalization[policy][field] = old_normalization[policy][field].clone();
        }
    }
    normalization["passive_socket_membership"]["equipment"] =
        old_normalization["passive_socket_membership"]["equipment"].clone();
    assert_eq!(
        normalization, old_normalization,
        "exact policy inverse; no extra dispositions or defaults"
    );
    let mut items = restored.items.clone();
    for row in read::<Vec<ItemLineRule>>("item-rules.json")
        .into_iter()
        .rev()
    {
        assert_eq!(items.rules.pop().unwrap(), row);
    }
    items.version = migrated.input().items.version.clone();
    assert_eq!(items, migrated.input().items);
    let mut item_source = restored.item_source.clone();
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        single_modifier_conditions,
        ..
    } = &mut item_source.dialect
    else {
        unreachable!()
    };
    for condition in read::<Vec<ItemSourceConditionalMember>>("source-conditions.json")
        .into_iter()
        .rev()
    {
        assert_eq!(single_modifier_conditions.pop().unwrap(), condition);
    }
    for row in read::<Vec<ItemLineRule>>("item-rules.json")
        .into_iter()
        .rev()
    {
        assert_eq!(
            item_source.rule_layouts.pop().unwrap(),
            ItemRuleSourceLayout {
                rule: row.id,
                role: ItemRuleSourceRole::Unresolved
            }
        );
    }
    item_source.version = migrated.input().item_source.version.clone();
    item_source.item_lines = migrated.input().item_source.item_lines;
    assert_eq!(item_source, migrated.input().item_source);
    restored.normalization = migrated.input().normalization.clone();
    restored.items = items;
    restored.item_source = item_source;
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        migrated.input().tree.as_ref().unwrap().content
    );
    restored.tree = migrated.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(
        restored,
        *migrated.input(),
        "all other full release data and identities survive exactly"
    );
    assert_component(&next);
    assert_eq!(prior.input().query_sets, next.input().query_sets);
    assert_eq!(prior.input().roles.roles, next.input().roles.roles);
    assert_eq!(
        prior.input().recipe.rules.contribution_queries,
        next.input().recipe.rules.contribution_queries
    );
    assert_eq!(
        prior.input().recipe.rules.receivers,
        next.input().recipe.rules.receivers
    );
    next
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    for row in migration.schema {
        match row {
            poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(d) => {
                assert!(endpoint.input().recipe.schema.definitions.contains(&d))
            }
            poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Slot(s) => {
                assert!(endpoint.input().recipe.schema.slots.contains(&s))
            }
        }
    }
    for owner in migration.owners {
        assert!(endpoint.input().recipe.rules.owners.contains(&owner));
        assert!(!owner.programs.is_complete());
    }
}
