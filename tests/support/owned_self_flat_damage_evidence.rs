//! Pure verifier of a nonempty source inventory, never native delivery authority.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
const ADDED: &str = "data/owned/poe2/3887ae68/combined-added-attack-damage/source-evidence.json";
const ENEMY: &str = "data/owned/poe2/3887ae68/enemy-flat-attack-damage/source-evidence.json";
const REVIEWED_CENSUS: &str = "8623c9ac974080cae3a990852c944d859a14c415381bf7c943c7c100d7119c38";
pub const CONTROL_LINES: [&str; 3] = [
    "Minions deal 3 to 7 additional Physical Damage",
    "Minions deal 0 to 0 additional Physical Damage",
    "Minions deal 3 to 7 additional Physical Damage while on Full Life",
];
fn root() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if path.join("data/owned").is_dir() {
        path
    } else {
        path.join("../..")
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fingerprint(value: &Value) -> String {
    digest(&serde_json::to_vec(value).unwrap())
}
pub fn scope() -> Value {
    json!({"channels":["PhysicalMin","PhysicalMax"],"inventory_only":true,"universal_absence":false,
        "native_coverage_authorized":false,"all_candidates_eligible":false,"requires_recipient_and_action_eligibility":true,
        "canonical_unique_object_records":true,"alias_delivery_eligibility_proven":false,
        "imported_party_admitted":false,"positive_game_obtainability_claim":false})
}
pub fn dependencies() -> Value {
    json!([
        {"path":ADDED,"sha256":"6c33fe128a0f6622785e70ef26843d47b8da33cc652f29ccf11246180e34fd45"},
        {"path":ENEMY,"sha256":"02ff224a7ea4842694ea5039aae2db84043245e4aced95445d04418497ecfab6"}
    ])
}
fn dependency(path: &str) -> Value {
    let bytes = std::fs::read(root().join(path)).unwrap();
    let deps = dependencies();
    let row = deps
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == path)
        .unwrap();
    assert_eq!(digest(&bytes), row["sha256"].as_str().unwrap());
    serde_json::from_slice(&bytes).unwrap()
}
pub fn source_files_digest() -> String {
    fingerprint(&dependency(ADDED)["files"])
}

// Classification is of the inner record's filter shape only. Wrapper filters,
// recipient delivery, value scaling and cfg overrides remain separate evidence.
pub fn qualification(record: &Value) -> &'static str {
    if record["type"] != "BASE"
        || record["flags"].as_u64() != Some(0)
        || record["keywordFlags"].as_u64() != Some(0)
    {
        return "other-or-unreviewed";
    }
    let tags: Vec<_> = record
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| key.starts_with('['))
        .map(|(_, v)| v)
        .collect();
    if tags.is_empty() {
        "zero-flags-no-tags"
    } else if tags
        .iter()
        .all(|tag| *tag == &json!({"type":"Condition","var":"FullLife"}))
    {
        "receiving-full-life-candidate"
    } else {
        "other-or-unreviewed"
    }
}
pub fn dynamic_domains() -> Value {
    json!([
        {"domain":"parsed-minion-modifier","path":"src/Modules/CalcPerform.lua","lines":[1161,1167],"disposition":"Original List uses summoner cfg; exact payload enters receiving Actor. Inner filters still apply at consuming query."},
        {"domain":"receiving-full-life","path":"src/Modules/ConfigOptions.lua","lines":[173,175],"disposition":"Config delivers receiving-minion flag. Retained Basic cfg has no FullLife override; potential Actor invariance is narrower than arbitrary Condition tags."},
        {"domain":"condition-override","path":"src/Classes/ModStore.lua","lines":[409,420],"disposition":"GetCondition checks cfg.overrideCond, local/parent conditions and self:Flag(cfg). Action-filtered FullLife writers must be excluded before an Actor subtotal is proven invariant."},
        {"domain":"condition-skill-cfg","path":"src/Classes/ModStore.lua","lines":[742,765],"disposition":"Condition tags also read cfg.skillCond. Observed MainHandAttack-only cfg does not prove future FullLife assertions absent."},
        {"domain":"tactician-parent-weapon","path":"src/Modules/CalcOffence.lua","lines":[819,834],"disposition":"Real dynamic flat writer: source-selected parent multiplier, PercentStat parent and SkillType Attack. No universal Actor subtotal authority."},
        {"domain":"companion-parent-weapon","path":"src/Modules/ModParser.lua","lines":[2940,2944],"disposition":"Outer CreatesCompanion filter differs from Tactician ExtraAura; known Sniper is not Companion."},
        {"domain":"tactician-aura","path":"src/Modules/ModParser.lua","lines":[3292,3296],"disposition":"ExtraAura onlyAllies transports parent-weapon flag; not excluded by Companion mismatch."},
        {"domain":"rallying-cry","path":"src/Modules/CalcPerform.lua","lines":[2222,2250],"disposition":"Power, parent weapon, buff effect, uptime and minion delivery generate and scale flat records. Not inferred absent from Basic profile."},
        {"domain":"hollow-palm","path":"src/Modules/CalcActiveSkill.lua","lines":[886,893],"disposition":"Condition plus staff eligibility or UseHollowPalmDamage flag; current Basic lacks Staff but the alternate flag prevents a universal exclusion."},
        {"domain":"spell-only-dynamic-flats","path":"src/Modules/CalcOffence.lua","lines":[791,818],"disposition":"Battlemage/Spellblade records require Spell flags; do not call them globally nonexistent."},
        {"domain":"sacrificial-zeal","path":"src/Modules/CalcOffence.lua","lines":[2399,2405],"disposition":"Mana-cost-derived PhysicalMin/Max carry Spell flag; exact Action qualification remains required."},
        {"domain":"buff-delivery","path":"src/Modules/CalcPerform.lua","lines":[3116,3134],"disposition":"Global and minion buff stores deliver real records; wrapper paths and ordered values must survive later native modeling."},
        {"domain":"stat-map-consumers","path":"src/Modules/CalcActiveSkill.lua","lines":[80,102],"disposition":"Actual local map overrides/global fallback are resolved per stat set. Inventory of a mapped key is not proof that any support or skill consumes it."},
        {"domain":"party","path":"src/Classes/PartyTab.lua","lines":[516,543],"disposition":"Arbitrary serialized numeric modifiers are possible. No-Party accounted-input boundary retained, not SourceOnly or completed01f2/0207."},
        {"domain":"ordered-sum","path":"src/Classes/ModList.lua","lines":[125,143],"disposition":"Local record order then parent sum; no arbitrary source rank or reassociation inferred from a census."}
    ])
}

pub fn observations() -> Value {
    let prior = dependency(ADDED);
    let c = &prior["selected_pass_corroboration"];
    let bytes = std::fs::read(root().join(c["path"].as_str().unwrap())).unwrap();
    assert_eq!(c["sha256"], digest(&bytes));
    assert_eq!(c["bytes"], bytes.len());
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    let expected = [
        ("original-05", 0.0, 0.0, 0),
        ("repeat-original-05", 0.0, 0.0, 0),
        ("warm-flat-to-original", 0.0, 0.0, 0),
        ("sniper-calcs-effective", 0.0, 0.0, 0),
        ("flat-physical", 3.0, 7.0, 2),
        ("zero-flat-physical", 0.0, 0.0, 2),
        ("conditional-flat-disabled", 0.0, 0.0, 2),
        ("conditional-flat-enabled", 3.0, 7.0, 2),
    ];
    assert_eq!(report["cases"].as_array().unwrap().len(), expected.len());
    let mut rows = Vec::new();
    let mut observed = 0;
    for (case, (name, min, max, count)) in report["cases"].as_array().unwrap().iter().zip(expected)
    {
        assert_eq!(case["name"], name);
        for mode in ["main", "calcs"] {
            let actors: Vec<_> = case["state"][mode]["actors"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| a["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
            assert_eq!(actor["hostile"], false);
            let children: Vec<_> = actor["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["effect_id"] == "MinionMeleeBow")
                .collect();
            assert_eq!(children.len(), 1);
            let child = children[0];
            let Some(calls) = child["base_calls"].as_array() else {
                assert_eq!(mode, "calcs");
                assert!(child.get("base_calls").is_none());
                assert_eq!(child["output"], json!({}));
                rows.push(json!({"case":name,"mode":mode,"observed":false,"source_occurrence":actor["source_occurrence"]}));
                continue;
            };
            let calls: Vec<_> = calls
                .iter()
                .filter(|c| c["damage_type"] == "Physical")
                .collect();
            assert_eq!(calls.len(), 1);
            let call = calls[0];
            let p = &call["preconversion"];
            for key in [
                "exact_actor_store",
                "exact_enemy_store",
                "exact_parent",
                "exact_pass_cfg",
                "exact_pass_source",
                "exact_summoner",
            ] {
                assert_eq!(p[key], true);
            }
            assert_eq!(p["cfg"]["fields"]["flags"], 42949813253u64);
            assert_eq!(p["cfg"]["fields"]["keywordFlags"], 330752);
            assert_eq!(p["cfg"]["skill_conditions"], json!({"MainHandAttack":true}));
            assert_eq!(p["cfg"]["granted_effect"]["id"], "MinionMeleeBow");
            assert_eq!(call["added_min"].as_f64(), Some(min));
            assert_eq!(call["added_max"].as_f64(), Some(max));
            let raw: Vec<_> = p["raw"]["skill"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| {
                    matches!(
                        r["mod"]["name"].as_str(),
                        Some("PhysicalMin" | "PhysicalMax")
                    )
                })
                .cloned()
                .collect();
            assert_eq!(raw.len(), count);
            for record in &raw {
                assert_eq!(record["ancestor_depth"], 1);
                let m = &record["mod"];
                assert_eq!(m["source"], "Custom:Physical damage source control");
                assert_eq!(m["type"], "BASE");
                assert_eq!(m["flags"], 0);
                assert_eq!(m["keyword_flags"], 0);
                let value = if name == "zero-flat-physical" {
                    0
                } else if m["name"] == "PhysicalMin" {
                    3
                } else {
                    7
                };
                assert_eq!(m["value"], value);
                assert_eq!(
                    m["tags"],
                    if name.starts_with("conditional-") {
                        json!([{"type":"Condition","var":"FullLife"}])
                    } else {
                        json!({})
                    }
                );
            }
            for (key, value) in [("minimum", min), ("maximum", max)] {
                assert_eq!(call[key]["value"].as_f64(), Some(value));
                if value == 0.0 {
                    assert_eq!(call[key]["records"], json!({}));
                } else {
                    assert_eq!(call[key]["records"].as_array().unwrap().len(), 1);
                }
            }
            rows.push(json!({"case":name,"mode":mode,"observed":true,"source_occurrence":actor["source_occurrence"],
                "xml_sha256":case["xml_sha256"],"raw_flat_records":raw,"cfg":p["cfg"],"minimum":call["minimum"],"maximum":call["maximum"],
                "original_added_min":call["added_min"],"original_added_max":call["added_max"],"query_state_preserved":call["query_state_preserved"]}));
            observed += 1;
        }
    }
    assert_eq!(observed, 9);
    json!(rows)
}

pub fn census_digest(v: &Value) -> String {
    fingerprint(
        &json!({"catalog":v["catalog"],"cache_census":v["cache_census"],"flat_records":v["flat_records"],"relevant_alias_routes":v["relevant_alias_routes"],
        "mappings":v["mappings"],"stat_consumers":v["stat_consumers"],"owners":v["owners"],"extra_stats":v["extra_stats"],
        "extra_stat_factories":v["extra_stat_factories"],"parser_controls":v["parser_controls"]}),
    )
}
pub fn check_semantics(v: &Value) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["kind"],
        "pinned-constructed-self-flat-physical-supplier-census"
    );
    assert_eq!(
        v["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(v["scope"], scope());
    assert!(
        !v["flat_records"].as_array().unwrap().is_empty(),
        "this supplier domain is not empty"
    );
    for row in v["flat_records"].as_array().unwrap() {
        assert!(matches!(
            row["record"]["name"].as_str(),
            Some("PhysicalMin" | "PhysicalMax")
        ));
        assert_eq!(row["inner_filter_shape"], qualification(&row["record"]));
    }
    assert_eq!(v["dependencies"], dependencies());
    assert_eq!(v["dynamic_domains"], dynamic_domains());
    assert_eq!(
        census_digest(v),
        REVIEWED_CENSUS,
        "changed actual supplier/consumer/wrapper census requires review"
    );
    let prior = dependency(ADDED);
    assert_eq!(v["module_order"], prior["module_order"]);
    assert_eq!(v["selected"], prior["catalog"]["selected"]);
    assert_eq!(
        v["selected_pass_corroboration"],
        prior["selected_pass_corroboration"]
    );
    assert_eq!(v["observations"], observations());
}
pub fn check(v: &Value, full: bool) {
    check_semantics(v);
    assert_eq!(v["source_files_sha256"], source_files_digest());
    let manifest =
        std::fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .unwrap();
    assert_eq!(v["source_manifest_sha256"], digest(&manifest));
    if full {
        let files: BTreeMap<String, String> =
            serde_json::from_value(dependency(ADDED)["files"].clone()).unwrap();
        for (path, sha) in files {
            let text =
                std::fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
                    .unwrap()
                    .replace("\r\n", "\n");
            assert_eq!(digest(text.as_bytes()), sha);
        }
    }
}

#[test]
fn retained_self_flat_census_cannot_claim_empty_or_native_eligibility() {
    let bytes = std::fs::read(
        root().join("data/owned/poe2/3887ae68/self-flat-attack-damage/source-evidence.json"),
    )
    .unwrap();
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    check(&original, false);
    for edit in 0..7 {
        let mut changed = original.clone();
        match edit {
            0 => changed["scope"]["universal_absence"] = json!(true),
            1 => changed["scope"]["native_coverage_authorized"] = json!(true),
            2 => changed["flat_records"] = json!([]),
            3 => changed["flat_records"][0]["inner_filter_shape"] = json!("all-actions"),
            4 => {
                changed["stat_consumers"].as_array_mut().unwrap().pop();
            }
            5 => changed["observations"][14]["original_added_min"] = json!(0),
            6 => changed["module_order"].as_array_mut().unwrap().reverse(),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| check_semantics(&changed)).is_err());
    }
}
