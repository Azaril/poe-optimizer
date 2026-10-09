//! Pure retained-proof validation shared with the optional PoB acquisition tests.
//! Reading authenticated source bytes here never constructs or executes Lua.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

fn root() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if directory.join("data/owned").is_dir() {
        directory
    } else {
        directory.join("../..")
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn check_semantics(evidence: &Value) {
    assert_eq!(evidence["schema_version"], 1);
    assert_eq!(
        evidence["kind"],
        "pinned-constructed-minion-intrinsic-source-selection"
    );
    assert_eq!(
        evidence["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(
        evidence["scope"],
        json!({"source_selection_only":true,"native_numerical_coverage":false,"all_actions":false,"normal_import_only":true})
    );
    let catalog = &evidence["catalog"];
    assert!(catalog["skill_count"].as_u64().unwrap() > 1000);
    assert!(catalog["raw_tables"].as_u64().unwrap() > 10000);
    assert!(catalog["raw_rows"].as_u64().unwrap() > catalog["raw_tables"].as_u64().unwrap());
    let selected = &catalog["selected"];
    assert_eq!(selected["selected_roots_have_no_metatable"], true);
    for name in ["summon_callbacks", "basic_callbacks", "profile_callbacks"] {
        assert_eq!(
            selected[name],
            json!({}),
            "new selected callback independent of graph aliases"
        );
    }
    assert_eq!(
        catalog["metatable_policy"],
        "only-original-skillStatMapMeta-on-statMap"
    );
    for row in catalog["metatables"].as_array().unwrap() {
        assert!(row["path"].as_str().unwrap().ends_with("/statMap"));
    }
    assert_eq!(selected["summon_id"], "SummonSkeletalSnipersPlayer");
    assert_eq!(selected["basic_id"], "MinionMeleeBow");
    assert_eq!(selected["profile_id"], "RaisedSkeletonSniper");
    assert_eq!(
        selected["summon"],
        json!({"minionList":{"[1]":"RaisedSkeletonSniper"},"minionUses":null,"minionHasItemSet":null,"parts":null,"addFlags":null})
    );
    assert_eq!(
        selected["basic"],
        json!({"parts":null,"baseFlags":null,"addFlags":null,"weaponTypes":null})
    );
    assert_eq!(
        selected["basic_stat_sets"],
        json!({"1":{"baseFlags":{"attack":true,"projectile":true},"parts":null,"skillFlags":null,"direct_callbacks":{}}})
    );
    assert_eq!(
        selected["profile"],
        json!({"skillList":{"[1]":"MinionMeleeBow","[2]":"GasShotSkeletonSniperMinion"},"weaponType1":"Bow","weaponType2":null,"hostile":null,"modList":{},"damage":1.15,"damageSpread":0.3,"attackTime":1.5,"critChance":5,"baseDamageIgnoresAttackSpeed":true})
    );
    let modifiers = catalog["skill_data_modifiers"].as_array().unwrap();
    assert!(!modifiers.is_empty());
    for modifier in modifiers {
        assert!(!modifier["path"].as_str().unwrap().is_empty());
        assert_ne!(
            modifier["key"], "minionUseBowAndQuiver",
            "new constructed inheritance supplier"
        );
    }
    let callbacks = catalog["callbacks"].as_array().unwrap();
    assert!(!callbacks.is_empty());
    for callback in callbacks {
        let path = callback["path"].as_str().unwrap();
        // A callback inherited through the reviewed statMap metatable copies
        // existing global mappings; it is not a skill execution callback.
        if path.starts_with("skills/SummonSkeletalSnipersPlayer/")
            || path.starts_with("skills/MinionMeleeBow/")
            || path.starts_with("minions/RaisedSkeletonSniper/")
        {
            assert!(
                path.ends_with("/statMap/<metatable>/__index"),
                "new selected source callback: {path}"
            );
            assert_eq!(callback["source"]["path"], "src/Modules/Data.lua");
            assert_eq!(callback["source"]["line"], 914);
            assert_eq!(callback["source"]["end_line"], 924);
        }
    }
    let source_metadata = catalog["source_metadata"].as_array().unwrap();
    for row in source_metadata {
        let path = row["path"].as_str().unwrap();
        if path.ends_with("/addFlags") {
            assert_ne!(
                row["value"]["unarmed"], true,
                "new unarmed source replacement"
            );
        }
        if path.ends_with("/skillFlag") {
            assert_ne!(row["value"], "unarmed", "new mapped source replacement");
        }
    }
    let inventory = &evidence["static_inventory"];
    let inheritance: Vec<_> = inventory["references"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["name"] == "minionUseBowAndQuiver")
        .cloned()
        .collect();
    assert_eq!(inheritance,json!([
        {"path":"src/Modules/CalcActiveSkill.lua","line":986,"name":"minionUseBowAndQuiver","quoted":false},
        {"path":"src/Modules/CalcPerform.lua","line":1107,"name":"minionUseBowAndQuiver","quoted":false}
    ]).as_array().unwrap().clone());
    let factories = inventory["factory_keys"].as_array().unwrap();
    assert!(factories.len() > 200);
    let mut parameters = 0;
    for factory in factories {
        assert_ne!(
            factory["key"], "minionUseBowAndQuiver",
            "new statically declared inheritance supplier"
        );
        if factory["key"] == "<original makeSkillDataMod parameter>" {
            parameters += 1;
            assert_eq!(factory["path"], "src/Modules/Data.lua");
            assert_eq!(factory["line"], 70);
        }
    }
    assert_eq!(parameters, 1);
    assert!(!inventory["dynamic_writes"].as_array().unwrap().is_empty());
    assert_eq!(
        evidence["selected_pass_corroboration"],
        json!({"path":"data/owned/poe2/3887ae68/minion-preconversion-source/report.json","sha256":"7a723a4eece7b297737006b08d1dc61ec0047d13cfd2b9fbfeaee94eb52397cb","bytes":4041068,"role":"corroboration-not-supplier-completeness"})
    );
}

pub fn check(evidence: &Value, full: bool) {
    check_semantics(evidence);
    let root = root();
    let bytes =
        std::fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let text = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
    assert_eq!(evidence["source_manifest_sha256"], digest(text.as_bytes()));
    let manifest: Value = serde_json::from_str(&text).unwrap();
    let files: BTreeMap<_, _> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["path"].as_str().unwrap().ends_with(".lua"))
        .map(|row| {
            (
                row["path"].as_str().unwrap().to_owned(),
                row["sha256"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(evidence["files"], json!(files));
    if full {
        for (path, expected) in files {
            let text =
                std::fs::read_to_string(root.join("vendor/path-of-building-poe2").join(path))
                    .unwrap()
                    .replace("\r\n", "\n");
            assert_eq!(digest(text.as_bytes()), expected);
        }
        let pin = &evidence["selected_pass_corroboration"];
        let bytes = std::fs::read(root.join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(digest(&bytes), pin["sha256"]);
    }
}

#[allow(dead_code)]
pub fn invalid_controls(original: &Value) -> Vec<(&'static str, Value)> {
    let mut controls = Vec::new();
    let mut changed = original.clone();
    changed["catalog"]["selected"]["basic_callbacks"]["preDamageFunc"] = json!({"path":"aliased"});
    controls.push(("aliased selected callback", changed));
    let mut changed = original.clone();
    changed["catalog"]["selected"]["selected_roots_have_no_metatable"] = json!(false);
    controls.push(("selected root metatable", changed));
    let mut changed = original.clone();
    changed["catalog"]["metatable_policy"] = json!("arbitrary");
    controls.push(("unreviewed nested modifier behavior", changed));
    let mut changed = original.clone();
    changed["catalog"]["selected"]["summon"]["minionUses"] = json!({"Weapon 1":true});
    controls.push(("parent equipment replacement", changed));
    let mut changed = original.clone();
    changed["catalog"]["selected"]["summon"]["minionHasItemSet"] = json!(true);
    controls.push(("item-set replacement", changed));
    let mut changed = original.clone();
    changed["catalog"]["selected"]["summon"]["minionList"]["[1]"] = json!("RaisedSkeleton");
    controls.push(("Iron Mass eligible profile", changed));
    let mut changed = original.clone();
    changed["catalog"]["selected"]["basic_stat_sets"]["1"]["baseFlags"]["unarmed"] = json!(true);
    controls.push(("unarmed basic source", changed));
    let mut changed = original.clone();
    changed["catalog"]["skill_data_modifiers"].as_array_mut().unwrap().push(json!({"path":"skills/AnySupport/nested/ExtraSkillMod/value/mod","key":"minionUseBowAndQuiver"}));
    controls.push(("nested inheritance supplier", changed));
    let mut changed = original.clone();
    changed["catalog"]["callbacks"].as_array_mut().unwrap().push(json!({"path":"skills/MinionMeleeBow/preDamageFunc","source":{"path":"src/Data/Skills/minion.lua","line":117,"end_line":117,"sha256":"new"}}));
    controls.push(("selected source callback", changed));
    let mut changed = original.clone();
    changed["catalog"]["source_metadata"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"skillStatMap/new/skillFlag","value":"unarmed"}));
    controls.push(("mapped source replacement", changed));
    let mut changed = original.clone();
    changed["static_inventory"]["factory_keys"].as_array_mut().unwrap().push(json!({"path":"src/Modules/ModParser.lua","line":1,"key":"minionUseBowAndQuiver","kind":"list-record"}));
    controls.push(("parser inheritance supplier", changed));
    controls
}
