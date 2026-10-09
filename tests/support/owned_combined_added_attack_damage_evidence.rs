//! Retained source-domain proof. This pure checker never hosts Lua.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
fn root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if p.join("data/owned").is_dir() {
        p
    } else {
        p.join("../..")
    }
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn selection() -> Value {
    let b = std::fs::read(
        root().join("data/owned/poe2/3887ae68/minion-attack-selection/source-evidence.json"),
    )
    .unwrap();
    assert_eq!(
        digest(&b),
        "a2daf1e5c6c058ae2ce32f434feee325d06344a96a5ccbf29ed7320df2c63ec3"
    );
    serde_json::from_slice(&b).unwrap()
}
fn table_projection(v: &Value) -> Value {
    match v {
        Value::Array(rows) => Value::Object(
            rows.iter()
                .enumerate()
                .map(|(i, v)| (format!("[{}]", i + 1), table_projection(v)))
                .collect(),
        ),
        Value::Object(rows) => Value::Object(
            rows.iter()
                .map(|(k, v)| (k.clone(), table_projection(v)))
                .collect(),
        ),
        _ => v.clone(),
    }
}
pub fn check_semantics(v: &Value) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["kind"],
        "pinned-constructed-intrinsic-added-damage-domain"
    );
    assert_eq!(
        v["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(
        v["scope"],
        json!({"normal_import_only":true,"imported_party_admitted":false,"requires_accounted_input":true,"exact_intrinsic_basic_source":true,"other_profiles_admitted":false,"all_actions":false,"finite_numeric_domain":[1.0,1.15],"game_rounding_intent":false})
    );
    let prior = selection();
    let c = &v["catalog"];
    for name in [
        "skill_count",
        "raw_tables",
        "raw_rows",
        "callbacks",
        "aliases",
        "metatables",
        "selected",
    ] {
        assert_eq!(
            c[name], prior["catalog"][name],
            "changed complete constructed {name} inventory"
        );
    }
    let rows = c["added_modifiers"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "only two dormant original stat-map recipes");
    for (row, stat) in rows.iter().zip([
        "active_skill_added_damage_+%_final",
        "added_damage_+%_final",
    ]) {
        assert_eq!(row["path"], format!("skillStatMap/{stat}/[1]"));
        assert_eq!(
            row["record"],
            json!({"name":"AddedDamage","type":"MORE","flags":0,"keywordFlags":0})
        );
    }
    assert_eq!(
        c["stat_consumers"],
        json!([]),
        "mapped additions have no constructed skill or support consumer"
    );
    let full: Value = serde_json::from_slice(
        &std::fs::read(
            root().join("data/owned/poe2/3887ae68/minion-life-source/source-vectors.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        c["full_profile"],
        table_projection(&full["definitions"]["profile"])
    );
    assert_eq!(c["full_profile"]["damage"], 1.15);
    assert!(c["full_profile"].get("damageFixup").is_none());
    assert!(v["high_precision_mods"].is_object());
    for channel in ["AddedDamage", "AddedPhysicalDamage"] {
        assert!(v["high_precision_mods"].get(channel).is_none());
    }
    assert!(v["cache_census"]["raw_tables"].as_u64().unwrap() > c["raw_tables"].as_u64().unwrap());
    assert!(v["cache_census"]["raw_rows"].as_u64().unwrap() > c["raw_rows"].as_u64().unwrap());
    for row in v["extra_stats"].as_array().unwrap() {
        let key = row["record"]["value"]["key"].as_str().unwrap();
        assert!(!matches!(
            key,
            "added_damage_+%_final" | "active_skill_added_damage_+%_final"
        ));
    }
    for row in v["extra_stat_factories"].as_array().unwrap() {
        let key = row["key"].as_str().unwrap();
        assert!(!matches!(
            key,
            "added_damage_+%_final" | "active_skill_added_damage_+%_final"
        ));
    }
    let expected = json!([
     {"path":"src/Classes/ModStore.lua","line":747,"literal":"Added"},
     {"path":"src/Classes/ModStore.lua","line":750,"literal":"Added"},
     {"path":"src/Classes/ModStore.lua","line":759,"literal":"Added"},
     {"path":"src/Classes/ModStore.lua","line":760,"literal":"Added"},
     {"path":"src/Data/SkillStatMap.lua","line":1016,"literal":"added_damage_+%_final"},
     {"path":"src/Data/SkillStatMap.lua","line":1017,"literal":"AddedDamage"},
     {"path":"src/Data/SkillStatMap.lua","line":1019,"literal":"active_skill_added_damage_+%_final"},
     {"path":"src/Data/SkillStatMap.lua","line":1020,"literal":"AddedDamage"},
     {"path":"src/Data/StatDescriptions/gem_stat_descriptions.lua","line":6013,"literal":"added_damage_+%_final"},
      {"path":"src/Data/StatDescriptions/gem_stat_descriptions.lua","line":33012,"literal":"added_damage_+%_final"},
      {"path":"src/Data/StatDescriptions/skill_stat_descriptions.lua","line":9193,"literal":"active_skill_added_damage_+%_final"},
      {"path":"src/Data/StatDescriptions/skill_stat_descriptions.lua","line":40563,"literal":"active_skill_added_damage_+%_final"},
      {"path":"src/Modules/CalcActiveSkill.lua","line":709,"literal":"AddedDamage"},
     {"path":"src/Modules/CalcOffence.lua","line":4134,"literal":"Added"},
     {"path":"src/Modules/CalcOffence.lua","line":4134,"literal":"AddedDamage"}
    ]);
    assert_eq!(
        v["references"], expected,
        "new literal/dynamic family spelling requires review"
    );
    assert_eq!(
        v["selection_dependency"],
        json!({"path":"data/owned/poe2/3887ae68/minion-attack-selection/source-evidence.json","sha256":"a2daf1e5c6c058ae2ce32f434feee325d06344a96a5ccbf29ed7320df2c63ec3"})
    );
    assert_eq!(
        v["selected_pass_corroboration"],
        prior["selected_pass_corroboration"]
    );
}
pub fn check(v: &Value, full: bool) {
    check_semantics(v);
    let r = root();
    let manifest =
        std::fs::read(r.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(v["source_manifest_sha256"], digest(&manifest));
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    let files: BTreeMap<_, _> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["path"].as_str().unwrap().ends_with(".lua"))
        .map(|r| {
            (
                r["path"].as_str().unwrap().to_owned(),
                r["sha256"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(v["files"], json!(files));
    assert_eq!(v["module_order"], selection()["module_order"]);
    if full {
        for (path, expected) in files {
            let text = std::fs::read_to_string(r.join("vendor/path-of-building-poe2").join(path))
                .unwrap()
                .replace("\r\n", "\n");
            assert_eq!(digest(text.as_bytes()), expected);
        }
        let p = &v["selected_pass_corroboration"];
        let b = std::fs::read(r.join(p["path"].as_str().unwrap())).unwrap();
        assert_eq!(p["bytes"], b.len());
        assert_eq!(p["sha256"], digest(&b));
    }
}
