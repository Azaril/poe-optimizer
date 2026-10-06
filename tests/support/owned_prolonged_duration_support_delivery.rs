//! Offline action topology and support contribution publication. No final timing law.
#[path = "owned_prolonged_duration_evidence.rs"]
mod evidence;
#[path = "owned_support_delivery_migration.rs"]
mod shared;
use poe_optimizer_core::{
    owned_content::OwnedContentDigest, owned_definitions::*, owned_rules::*, owned_schema::*,
    owned_supports::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "prolonged-duration-support-delivery-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/prolonged-duration-support-delivery")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn rows(v: &Value) -> &[Value] {
    v.as_array().expect("authored array")
}
fn subject(d: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":d["kind"],"value":d}})
}
pub fn authoring_digest() -> OwnedContentDigest {
    shared::authoring_digest(&data(""), "owned-prolonged-duration-support-delivery-v1")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let r = json!(read::<shared::ReceivingFragment>("receiving.json"));
    let p: SupportPreparationInput = read("preparation.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d["source"]["input"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], d["source"][field]);
    }
    assert_eq!(a["allocated_definitions"], 4);
    assert_eq!(a["allocated_slots"], 1);
    assert_eq!(a["new_programs"], 6);
    assert_eq!(a["new_owners"], 2);
    assert_eq!(a["registry_last_issued_before"], 0x32fc);
    assert_eq!(a["registry_last_issued_after"], 0x3301);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
    assert_eq!(
        b["scope"],
        json!({"contribution_factors_only":true,"final_cost_formula":false,
        "final_duration_formula":false,"application_lifetime_transfer":false,
        "reservation_formula":false,"owner_closure":"partial","receiving_fragment_only":true})
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!((m.schema_version, m.contract.schema_version), (5, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!((m.schema.len(), m.owners.len()), (7, 2));
    assert_eq!(b["factor_unit"]["key"], "def.0000000000000001");
    assert_eq!(b["channels"]["cost_factor"]["key"], "def.00000000000032fa");
    assert_eq!(
        b["channels"]["duration_factor"]["key"],
        "def.0000000000003301"
    );
    // The only existing schema changes add the exact same player output to the
    // Skill and its physical supply. Their complete original inventories survive.
    let old = rows(&d["replaced_schema"]);
    assert_eq!(old.len(), 2);
    let schema = json!(m.schema);
    for (index, member_path) in [(0, vec!["declarations", "outputs"]), (1, vec!["outputs"])] {
        let mut restored = schema[index].clone();
        let mut value = &mut restored["value"]["value"]["schema"]["value"];
        for path in member_path {
            value = &mut value[path];
        }
        assert_eq!(value["members"], json!([b["target"]["output"]]));
        value["members"] = json!([]);
        assert_eq!(restored, old[index]);
    }
    let output = &schema[2]["value"]["value"];
    assert_eq!(output["id"], b["target"]["output"]);
    let output_schema = &output["schema"]["value"];
    assert_eq!(output_schema["actor_role"], json!({"kind":"player"}));
    for (field, value) in [
        ("parts", b["target"]["part"].clone()),
        ("modes", b["target"]["mode"].clone()),
        ("stat_sets", b["target"]["stat_sets"][0]["stat_set"].clone()),
    ] {
        assert_eq!(
            output_schema[field],
            json!({"members":[value],"closure":{"kind":"complete"}})
        );
    }
    assert_eq!(output_schema["choices"]["closure"]["kind"], "partial");
    assert_eq!(rows(&b["target"]["stat_sets"]).len(), 1);
    let owners = json!(m.owners);
    for (i, (gem, skill, effect, percent)) in [
        (0x8a4, 0x2b5, "ProlongedDurationSupportPlayer", 30),
        (0x8a5, 0x2b6, "ProlongedDurationSupportPlayerTwo", 35),
    ]
    .into_iter()
    .enumerate()
    {
        let s = &b["supports"][i];
        assert_eq!(s["gem"]["key"], format!("def.{gem:016x}"));
        assert_eq!(s["skill"]["key"], format!("def.{skill:016x}"));
        assert_eq!(s["source_effect"], effect);
        assert_eq!(s["source_duration_more"], percent);
        assert_eq!(s["duration_factor"], 1.0 + f64::from(percent) / 100.0);
        assert_eq!(s["source_mana_multiplier"], 20);
        assert_eq!(s["cost_factor"], 1.2);
        assert_eq!(owners[i]["owner"], subject(&s["gem"]));
        assert_eq!(d["new_owner_ids"][i], owners[i]["owner"]);
        assert_eq!(owners[i]["programs"]["closure"]["kind"], "partial");
        let programs = rows(&owners[i]["programs"]["members"]);
        assert_eq!(programs.len(), 3);
        assert_eq!(programs[0]["id"], s["programs"]["prepared_inputs"]);
        assert_eq!(programs[0]["context"], "support_origin");
        assert_eq!(
            programs[0]["reads"][0]["source"],
            json!({"kind":"gem_level"})
        );
        assert_eq!(
            programs[0]["reads"][1]["source"]["kind"],
            "gem_quality_amount"
        );
        for (index, key) in ["def.00000000000030ae", "def.00000000000030af"]
            .into_iter()
            .enumerate()
        {
            assert_eq!(programs[0]["effects"][index]["effect"]["stat"]["key"], key);
        }
        assert_eq!(programs[1]["id"], s["programs"]["applicability"]);
        assert_eq!(
            programs[1]["nodes"][0]["expression"],
            json!({"kind":"literal","value":{"kind":"boolean","value":true}})
        );
        assert_eq!(
            programs[1]["effects"][0]["effect"]["kind"],
            "support_applicability"
        );
        let delivery = &programs[2];
        assert_eq!(delivery["id"], s["programs"]["delivery"]);
        assert_eq!(delivery["context"], "action");
        assert_eq!(delivery["reads"], json!([]));
        assert_eq!(rows(&delivery["nodes"]).len(), 2);
        assert_eq!(rows(&delivery["effects"]).len(), 2);
        for (index, field) in ["duration_factor", "cost_factor"].into_iter().enumerate() {
            let id = field.replace('_', "-");
            assert_eq!(
                delivery["nodes"][index],
                json!({"id":id,"expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":s[field],"unit":b["factor_unit"]}}}})
            );
            assert_eq!(
                delivery["effects"][index],
                json!({"id":id,"when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"][field],"contribution":"multiply","value":id}})
            );
        }
    }
    assert_eq!(p.supports.len(), 2);
    assert_eq!(json!(p.definitions), a["definitions"]);
    assert_eq!(json!(p.rules), a["rules"]);
    for (i, row) in p.supports.iter().enumerate() {
        assert_eq!(json!(row.gem), b["supports"][i]["gem"]);
        let SchemaState::Known(def) = &row.preparation else {
            panic!("known preparation")
        };
        assert_eq!(
            def.requires,
            Some(SupportTypePredicate::Type(
                OwnedDefinitionKey::new("type.duration").unwrap()
            ))
        );
        assert!(def.excludes.is_none() && def.added_types.is_empty());
        assert!(
            def.is_support
                && !def.is_trigger
                && !def.gems_only
                && !def.from_item
                && !def.ignore_minion_types
        );
    }
    assert_eq!(rows(&r["targets"]).len(), 2);
    for (i, t) in [&b["target"], &b["contrast"]].into_iter().enumerate() {
        let endpoint = &r["targets"][i]["roles"]["members"][0]["endpoints"]["members"][0];
        assert_eq!(
            endpoint,
            &json!({"kind":"action","path":[t["entering_grant"]],"output":t["output"],"selection":{"kind":"all_declared"},"admission":{"kind":"receiving_skill","summoner_path":null}})
        );
        assert_eq!(r["targets"][i]["roles"]["closure"]["kind"], "partial");
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in rows(&a["source_files"]) {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|v| *v == pin)
                .count(),
            1
        );
    }
    evidence::check(&a, &b, &v, false);
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    check_authored();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let provenance = next.input().provenance.last().unwrap();
    assert_eq!(provenance.kind.as_str(), KIND);
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(next.input().recipe.schema.release, m.release);
    for row in m.schema {
        match row {
            SchemaExtensionEntry::Definition(d) => assert_eq!(
                next.input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .filter(|v| **v == d)
                    .count(),
                1
            ),
            SchemaExtensionEntry::Slot(s) => assert_eq!(
                next.input()
                    .recipe
                    .schema
                    .slots
                    .iter()
                    .filter(|v| **v == s)
                    .count(),
                1
            ),
        }
    }
    for owner in m.owners {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|v| **v == owner)
                .count(),
            1
        );
    }
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence::check(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("source-vectors.json"),
        true,
    );
    let next = shared::stage_with_action_topology(prior, &data(""), KIND, authoring_digest());
    assert_endpoint(&next);
    next
}
