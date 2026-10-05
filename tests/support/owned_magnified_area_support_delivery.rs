//! Checked Magnified Area authoring. Receiving is a fragment, never a release bundle.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SlotDescriptor},
    owned_support_receiving::{
        SupportReceivingEntry, SupportReceivingRole, SupportTargetReceivingRoles,
    },
};
use poe_optimizer_import::{
    owned_mapping::{MappingEntry, OwnedIdRegistry},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReceivingFragment {
    roles: Vec<SupportReceivingRole>,
    targets: Vec<SupportTargetReceivingRoles>,
    supports: Vec<SupportReceivingEntry>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/magnified-area-support-delivery")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn subject(gem: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":"gem","value":gem}})
}
fn partial(owner: &Value, code: &str) -> Value {
    json!({"kind":"partial","value":{"gaps":[{"subject":owner,"facet":"game_rules","code":code}]}})
}
fn quantity(amount: &Value, unit: &Value) -> Value {
    json!({"kind":"quantity","value":{"value":amount.as_f64().unwrap(),"unit":unit}})
}
fn programs(s: &Value, b: &Value) -> Vec<Value> {
    let mut result = vec![
        json!({"id":s["programs"]["applicability"],"context":"action","reads":[],
            "nodes":[{"id":"applicable","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}}],
            "effects":[{"id":"applicability","when":null,"effect":{"kind":"support_applicability","applicable":"applicable"}}]}),
        json!({"id":s["programs"]["delivery"],"context":"action","reads":[],
            "nodes":[
                {"id":"area","expression":{"kind":"literal","value":quantity(&s["area_increase"],&b["percent_unit"])}},
                {"id":"cost-factor","expression":{"kind":"literal","value":quantity(&s["cost_factor"],&b["factor_unit"])}}],
            "effects":[
                {"id":"area","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"]["area"],"contribution":"increase","value":"area"}},
                {"id":"cost-factor","when":null,"effect":{"kind":"contribute","entity":"current","stat":b["channels"]["cost_factor"],"contribution":"multiply","value":"cost-factor"}}]}),
    ];
    if !s["programs"]["damage_delivery"].is_null() {
        result.push(json!({"id":s["programs"]["damage_delivery"],"context":"action",
            "reads":[{"id":"area-eligible","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["channels"]["area_eligible"]}}}],
            "nodes":[{"id":"area-eligible","expression":{"kind":"read","input":"area-eligible"}},
                {"id":"damage-factor","expression":{"kind":"literal","value":quantity(&json!(1.0),&b["factor_unit"])}}],
            "effects":[{"id":"damage-factor","when":"area-eligible","effect":{"kind":"contribute","entity":"current","stat":b["channels"]["damage_factor"],"contribution":"multiply","value":"damage-factor"}}]}));
    }
    result
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let r = json!(read::<ReceivingFragment>("receiving.json"));
    assert_eq!(a["allocated_definitions"], 3);
    assert_eq!(a["new_programs"], 5);
    assert_eq!(a["registry_last_issued_before"], 0x32f8);
    assert_eq!(a["registry_last_issued_after"], 0x32fb);
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
    assert_eq!(b["definitions"], a["definitions"]);
    assert_eq!(
        b["scope"],
        json!({"area_eligibility_producer":false,"resource_cost_final_formula":false,
        "radius_final_formula":false,"owner_closure":"partial","receiving_fragment_only":true})
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.schema_version, 4);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v19"
    );
    assert_eq!((m.schema.len(), m.owners.len()), (3, 2));
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    for (index, (name, id)) in [
        ("area", 0x32f9),
        ("cost_factor", 0x32fa),
        ("area_eligible", 0x32fb),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(b["channels"][name]["key"], format!("def.{id:016x}"));
        let value = if name == "area_eligible" {
            json!({"kind":"boolean"})
        } else {
            json!({"kind":"quantity","value":{"unit":b[if name == "area" {"percent_unit"} else {"factor_unit"}]}})
        };
        assert_eq!(
            json!(m.schema[index]),
            json!({"kind":"definition","value":{"kind":"stat","value":{
            "id":b["channels"][name],"schema":{"kind":"known","value":{"value":value,"targets":["action"]}}}}})
        );
    }
    assert_eq!(
        b["channels"]["damage_factor"]["key"],
        "def.00000000000032f8"
    );
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(b["factor_unit"]["key"], "def.0000000000000001");
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 1);
    let supports = b["supports"].as_array().unwrap();
    assert_eq!(supports.len(), 2);
    for (s, (gem, skill, source, area, mapped)) in supports.iter().zip([
        (0x82a, 0x47f, "SupportMagnifiedAreaPlayer", 35, false),
        (0x82b, 0x480, "SupportMagnifiedAreaPlayerTwo", 45, true),
    ]) {
        assert_eq!(s["gem"]["key"], format!("def.{gem:016x}"));
        assert_eq!(s["skill"]["key"], format!("def.{skill:016x}"));
        assert_eq!(s["source_effect"], source);
        assert_eq!(s["area_increase"], area);
        assert_eq!(s["cost_factor"], 1.3);
        assert_eq!(s["source_mana_multiplier"], 30);
        assert_eq!(
            s["zero_damage_mapping"],
            if mapped { "area_flag" } else { "unmapped" }
        );
        assert_eq!(
            s["damage_factor"],
            if mapped { json!(1.0) } else { Value::Null }
        );
        assert_eq!(s["programs"]["damage_delivery"].is_null(), !mapped);
        let mapping = json!({"source":{"kind":"definition","value":{"kind":"skill","value":{"effect_id":{"kind":"text","value":source}}}},
            "outcome":{"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"skill","value":s["skill"]}},"basis":{"kind":"exact"}}}});
        assert_eq!(
            d["mapping_rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| **row == mapping)
                .count(),
            1
        );
        let gem_rows: Vec<_> = d["supporting_definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "gem" && row["value"]["id"] == s["gem"])
            .collect();
        assert_eq!(gem_rows.len(), 1);
        assert_eq!(gem_rows[0]["value"]["schema"]["kind"], "known");
        assert_eq!(
            gem_rows[0]["value"]["schema"]["value"]["roles"],
            json!(["support_assignment"])
        );
        assert_eq!(
            gem_rows[0]["value"]["schema"]["value"]["skills"]["members"],
            json!([])
        );
        let owner = subject(&s["gem"]);
        let next = m.owners.iter().find(|o| json!(o.owner) == owner).unwrap();
        let added = programs(s, &b);
        let mut restored = json!(next);
        let members = restored["programs"]["members"].as_array_mut().unwrap();
        assert_eq!(members.split_off(members.len() - added.len()), added);
        if mapped {
            assert_eq!(
                restored,
                json!(old[0]),
                "the prepared-input prefix and Partial closure are exact"
            );
        } else {
            assert_eq!(
                restored,
                json!({"owner":owner,"programs":{"members":[],"closure":partial(&owner,"support-owner-integration-incomplete")}})
            );
        }
        assert!(!next.programs.is_complete());
    }
    assert_eq!(d["absent_owners"], json!([subject(&supports[0]["gem"])]));
    check_receiving(&b, &r);
    check_evidence(&a, &v, false);
}

fn check_receiving(b: &Value, r: &Value) {
    let djinn: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/djinn-actions/bindings.json")).unwrap(),
    )
    .unwrap();
    let ice: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/ice-nova-intrinsics/bindings.json"))
            .unwrap(),
    )
    .unwrap();
    let mut expected = vec![];
    for f in djinn["families"].as_array().unwrap() {
        let mut endpoints = vec![];
        for (child, command) in std::iter::once((&f["command"], true))
            .chain(f["actions"].as_array().unwrap().iter().map(|x| (x, false)))
        {
            let path = if command {
                json!([child["entering_grant"]])
            } else {
                json!([f["minion"]["entering_grant"], child["entering_grant"]])
            };
            endpoints.push(json!({"source_effect":child["skill_id"],"actor":if command {"player"} else {"minion"},
                "skill":child["skill"],"output":child["output"],"stat_sets":child["stat_sets"],
                "endpoint":{"kind":"action","path":path,"output":child["output"],"selection":{"kind":"all_declared"},
                    "admission":{"kind":"receiving_skill","summoner_path":if command {Value::Null} else {json!([])}}}}));
        }
        expected.push(json!({"key":f["key"],"owner":{"kind":"skill","value":f["skill"]},"endpoints":endpoints}));
    }
    let sets: Vec<_> = ice["stat_sets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| json!({"source_index":s["source_index"],"stat_set":s["stat_set"]}))
        .collect();
    expected.push(json!({"key":"ice_nova","owner":{"kind":"gem","value":ice["physical_gem"]},"endpoints":[{
        "source_effect":"IceNovaPlayer","actor":"player","skill":ice["primary_skill"],"output":ice["output"],"stat_sets":sets,
        "endpoint":{"kind":"action","path":[ice["entering_grant"]],"output":ice["output"],"selection":{"kind":"all_declared"},
            "admission":{"kind":"receiving_skill","summoner_path":null}}}]}));
    assert_eq!(b["targets"], json!(expected));
    assert_eq!(
        r["roles"],
        json!([{"id":"magnified-area-action","kind":"action"}])
    );
    let targets: Vec<_> = expected.iter().map(|t| json!({"owner":t["owner"],"roles":{
        "members":[{"role":"magnified-area-action","endpoints":{"members":t["endpoints"].as_array().unwrap().iter().map(|e|e["endpoint"].clone()).collect::<Vec<_>>(),"closure":{"kind":"complete"}}}],
        "closure":partial(&json!({"kind":"definition","value":t["owner"]}),"remaining-support-receivers-not-converted")}})).collect();
    assert_eq!(r["targets"], json!(targets));
    let supports: Vec<_> = b["supports"].as_array().unwrap().iter().map(|s| {
        let mut delivery = vec![s["programs"]["delivery"].clone()];
        if !s["programs"]["damage_delivery"].is_null() { delivery.push(s["programs"]["damage_delivery"].clone()); }
        json!({"gem":s["gem"],"receivers":{"members":[{"role":"magnified-area-action","applicability":s["programs"]["applicability"],"delivery":delivery}],
            "closure":partial(&subject(&s["gem"]),"remaining-support-receivers-not-converted")}})
    }).collect();
    assert_eq!(r["supports"], json!(supports));
}

fn check_evidence(a: &Value, v: &Value, full: bool) {
    assert_eq!(
        a["scope"],
        json!({"complete_original_builds":0,"retired_input_issues":0,"closed_rule_owners":0,
        "receiving_fragment_only":true,"evaluation_bundle_added":false,"final_resource_cost_or_radius_claimed":false,"area_eligibility_producer_added":false})
    );
    assert_eq!(
        v["scope"],
        json!({"numerical_component":true,"complete_build_parity":false,"area_eligibility_authority":false,"final_resource_cost_or_radius":false})
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for field in ["source_revision", "source_manifest_sha256"] {
        assert_eq!(a[field], v[field]);
    }
    let pins = a["source_files"].as_array().unwrap();
    assert!(!pins.is_empty());
    let mut seen = BTreeSet::new();
    for pin in pins {
        assert!(seen.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| *row == pin)
                .count(),
            1
        );
        if full {
            let bytes = fs::read(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap();
            let text = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
            assert_eq!(pin["bytes"], text.len());
            assert_eq!(pin["sha256"], hash(text.as_bytes()));
        }
    }
    let declared: Value = serde_json::from_slice(
        &fs::read(
            root().join("data/owned/poe2/3887ae68/djinn-support-preparation/source-vectors.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for (field, id) in [
        ("source_definitions", "effect"),
        ("source_declarations", "source_effect"),
    ] {
        let expected: Vec<_> = declared[field]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| {
                matches!(
                    x[id].as_str(),
                    Some("SupportMagnifiedAreaPlayer" | "SupportMagnifiedAreaPlayerTwo")
                )
            })
            .cloned()
            .collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(v[field], json!(expected));
    }
    for declaration in v["source_declarations"].as_array().unwrap() {
        let text = declaration["declaration"].as_str().unwrap();
        assert_eq!(hash(text.as_bytes()), declaration["normalized_lf_sha256"]);
        if full {
            let source = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(declaration["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            let actual = source
                .lines()
                .skip(declaration["first_line"].as_u64().unwrap() as usize - 1)
                .take(text.lines().count())
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(actual, text);
        }
    }
    let hashes = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        hashes.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "dependencies",
            "migration",
            "receiving",
            "source-vectors"
        ]
    );
    for (name, digest) in hashes {
        assert_eq!(
            digest,
            &hash(&fs::read(data(&format!("{name}.json"))).unwrap())
        );
    }
    let reports = v["reports"].as_array().unwrap();
    if a["source_validation"]["status"] == "pending" {
        assert!(reports.is_empty());
        assert!(
            !full,
            "numerical source certification is pending; publication is forbidden"
        );
        return;
    }
    assert_eq!(a["source_validation"]["status"], "passed");
    assert_eq!(a["source_validation"]["cases"], 36);
    assert_eq!(a["source_validation"]["stages"], 3);
    assert_eq!(a["source_validation"]["effect_stat_set_pairs"], 14);
    assert_eq!(a["source_validation"]["independent_original_replays"], 5);
    check_action_matrix(v, &read::<Value>("bindings.json"));
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["bytes"], reports[1]["bytes"]);
    assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for (index, report) in reports.iter().enumerate() {
        let suffix = if index == 0 { "" } else { "_on" };
        assert_eq!(
            report["path"],
            a["source_validation"][format!("evidence{suffix}_json")]
        );
        assert_eq!(
            report["bytes"],
            a["source_validation"][format!("evidence{suffix}_bytes")]
        );
        assert_eq!(
            report["sha256"],
            a["source_validation"][format!("evidence{suffix}_sha256")]
        );
        assert!(!report["observations"].as_array().unwrap().is_empty());
        if full {
            let path = report["path"].as_str().unwrap();
            assert!(path.starts_with("runs/") && !path.contains(".."));
            // The immutable 36-case report is 74,626,853 bytes. Keep the same
            // explicit bound as the original-source witness, not an unbounded read.
            assert!(fs::metadata(root().join(path)).unwrap().len() <= 128 * 1024 * 1024);
            let bytes = fs::read(root().join(path)).unwrap();
            assert!(bytes.len() <= 128 * 1024 * 1024, "bounded source report");
            assert_eq!(report["bytes"], bytes.len());
            assert_eq!(report["sha256"], hash(&bytes));
            let actual: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual["manifest_sha256"], a["source_manifest_sha256"]);
            assert_eq!(actual["native_inventory_authority"], false);
            assert_eq!(actual["native_build_parity"], false);
            check_full_report(&actual, a, v);
            for pin in pins {
                assert_eq!(
                    actual["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                        .count(),
                    1
                );
            }
            for observation in report["observations"].as_array().unwrap() {
                assert_eq!(
                    actual
                        .pointer(observation["pointer"].as_str().unwrap())
                        .unwrap(),
                    &observation["value"]
                );
            }
        }
    }
}

fn source_rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert_eq!(
            value,
            &json!({}),
            "only Lua's exact empty-table form is a sequence"
        );
        &[]
    }
}

fn observation<'a>(rows: &BTreeMap<&str, &'a Value>, prefix: &str, suffix: &str) -> &'a Value {
    let pointer = format!("{prefix}/{suffix}");
    rows.get(pointer.as_str())
        .copied()
        .unwrap_or_else(|| panic!("missing observation {pointer}"))
}

fn check_action_matrix(v: &Value, b: &Value) {
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let mut observations = BTreeMap::new();
    for row in reports[0]["observations"].as_array().unwrap() {
        assert!(
            observations
                .insert(row["pointer"].as_str().unwrap(), &row["value"])
                .is_none()
        );
    }
    assert!(observations.len() <= 4096, "bounded authoring projection");
    let expected: BTreeSet<_> = b["targets"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|target| {
            target["endpoints"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|endpoint| {
                    endpoint["stat_sets"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(move |set| {
                            (
                                endpoint["source_effect"].as_str().unwrap(),
                                set["source_index"].as_u64().unwrap(),
                            )
                        })
                })
        })
        .collect();
    assert_eq!(expected.len(), 14);
    let mut actual = BTreeSet::new();
    for row in v["action_matrix"].as_array().unwrap() {
        let key = (
            row["source_effect"].as_str().unwrap(),
            row["source_stat_set_index"].as_u64().unwrap(),
        );
        assert!(actual.insert(key));
        assert!(row["area_eligible"].is_boolean());
        assert_eq!(row["admitted"].as_object().unwrap().len(), 2);
        let mut seen = BTreeSet::new();
        for evidence in row["evidence"].as_array().unwrap() {
            assert_eq!(evidence["report"], 0);
            let pointer = evidence["pointer"].as_str().unwrap();
            let segments: Vec<_> = pointer.split('/').collect();
            assert!(segments.len() == 8 || segments.len() == 10);
            assert_eq!(segments[1], "cases");
            assert_eq!(segments[3], "states");
            assert_eq!(segments[5], "delivery");
            assert_eq!(segments[6], "contexts");
            let stage = segments[4];
            assert!(["fresh", "rebuilt_once", "rebuilt_twice"].contains(&stage));
            let get = |field| observation(&observations, pointer, field);
            assert_eq!(get("effect"), &row["source_effect"]);
            assert_eq!(get("stat_set_index"), &row["source_stat_set_index"]);
            assert_eq!(get("queries/area_flag"), &row["area_eligible"]);
            let candidates = source_rows(get("candidates"));
            assert_eq!(candidates.len(), 1);
            let candidate = &candidates[0];
            let effect = candidate["effect"].as_str().unwrap();
            let support = b["supports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["source_effect"] == effect)
                .unwrap();
            assert_eq!(candidate["accepted"], row["admitted"][effect]);
            assert!(candidate["accepted"].is_boolean());
            assert_eq!(candidate["exact_definition"], true);
            assert_eq!(candidate["origin"]["enabled"], true);
            assert!(candidate["origin"]["source_ordinal"].is_u64());
            assert!(seen.insert((effect, stage)));
            if stage == "fresh" {
                for field in [
                    "queries/cfg_effect_exact",
                    "queries/exact_stat_set",
                    "queries/original_query_methods",
                ] {
                    assert_eq!(get(field), &json!(true));
                }
                let flags = get("queries/cfg")["flags"].as_u64().unwrap();
                let area_flag = get("queries/area_flag_value").as_u64().unwrap();
                assert_eq!(area_flag, 512);
                assert_eq!(get("queries/area_flag"), &json!(flags & area_flag != 0));
                check_source_channels(
                    get("queries/channels"),
                    support,
                    candidate["accepted"] == true,
                );
            }
        }
        assert_eq!(seen.len(), 6, "both support tiers at every lifecycle stage");
    }
    assert_eq!(actual, expected);
}

fn check_source_channels(channels: &Value, support: &Value, accepted: bool) {
    for (name, kind, value, flags) in [
        ("AreaOfEffect", "INC", support["area_increase"].clone(), 0),
        (
            "SupportManaMultiplier",
            "MORE",
            support["source_mana_multiplier"].clone(),
            0,
        ),
        ("Damage", "MORE", json!(0), 512),
    ] {
        let raw = source_rows(&channels[name]["raw_source_records"]);
        let expected =
            accepted && (name != "Damage" || support["zero_damage_mapping"] == "area_flag");
        assert_eq!(raw.len(), usize::from(expected));
        for row in raw {
            assert_eq!(row["source_effect"], support["source_effect"]);
            assert_eq!(row["channel_index"], 1);
            assert!(row["ancestor_depth"].is_u64());
            assert_eq!(row["record"]["name"], name);
            assert_eq!(row["record"]["type"], kind);
            assert_eq!(row["record"]["value"], value);
            assert_eq!(row["record"]["flags"], flags);
            assert_eq!(row["record"]["keyword_flags"], 0);
            assert!(source_rows(&row["record"]["tags"]).is_empty());
        }
        let applied: Vec<_> = source_rows(&channels[name]["applied"])
            .iter()
            .filter(|r| r.get("source_effect").is_some())
            .collect();
        // Tabulate omits neutral MORE0 even when Area matches. Preserve its raw
        // record and condition; never fabricate a positive diagnostic row.
        assert_eq!(applied.len(), usize::from(expected && name != "Damage"));
        for row in applied {
            assert_eq!(row["source_record_indices"], json!([1]));
            assert_eq!(row["record"], raw[0]["record"]);
            assert_eq!(row["source_effect"], raw[0]["source_effect"]);
            assert_eq!(row["value"], value);
        }
    }
}

fn check_full_report(report: &Value, a: &Value, v: &Value) {
    let certificate = &v["certificate"];
    // The witness records path/hash pairs; authored pins retain the full
    // canonical manifest row (including bytes), checked independently above.
    let report_pins: Vec<_> = a["source_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pin| json!({"path":pin["path"],"sha256":pin["sha256"]}))
        .collect();
    assert_eq!(report["files"], json!(report_pins));
    assert_eq!(report["source_revision"], a["source_revision"]);
    assert_eq!(report["observer_sha256"], certificate["observer_sha256"]);
    assert_eq!(report["original_sources"], certificate["original_sources"]);
    assert_eq!(report["lifecycle_stages"], certificate["lifecycle_stages"]);
    assert_eq!(certificate["byte_identical_jit_modes"], true);
    assert_eq!(report["numeric_tolerance"], 0);
    for flag in [
        "business_wrappers",
        "source_cfg_modified",
        "source_tables_mutated",
        "canonical_parity_lifecycle_selected",
        "final_cost_or_radius_formula_authority",
    ] {
        assert_eq!(report[flag], false);
    }
    let cases = source_rows(&report["cases"]);
    assert_eq!(cases.len(), 36);
    assert_eq!(
        json!(cases.iter().map(|c| &c["name"]).collect::<Vec<_>>()),
        certificate["case_names"]
    );
    assert_eq!(source_rows(&report["original_sources"]).len(), 5);
    for (index, source) in source_rows(&report["original_sources"]).iter().enumerate() {
        assert_eq!(cases[index]["name"], format!("original-{:02}", index + 1));
        assert_eq!(
            cases[index + 31]["name"],
            format!("repeat-original-{:02}", index + 1)
        );
        let path = source["path"].as_str().unwrap();
        assert!(path.starts_with("tests/fixtures/builds/") && !path.contains(".."));
        assert_eq!(
            source["sha256"],
            hash(&fs::read(root().join(path)).unwrap())
        );
        assert_eq!(cases[index]["xml_sha256"], source["sha256"]);
        assert_eq!(cases[index + 31]["xml_sha256"], source["sha256"]);
        assert_eq!(
            cases[index]["states"],
            cases[index + 31]["states"],
            "independent original replay {index}"
        );
    }
    for case in cases {
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let delivery = &case["states"][stage]["delivery"];
            assert_eq!(delivery["immutable_snapshot"]["verified"], true);
            for flag in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(delivery[flag], true);
            }
            for flag in [
                "source_cfg_modified",
                "source_tables_mutated",
                "business_wrappers",
            ] {
                assert_eq!(delivery[flag], false);
            }
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let receiving: ReceivingFragment = read("receiving.json");
    check_evidence(&a, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["supporting_definitions"].clone())
            .unwrap()
    {
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
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
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
    for row in serde_json::from_value::<Vec<MappingEntry>>(d["mapping_rows"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    for row in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for owner in d["absent_owners"].as_array().unwrap() {
        assert!(
            !prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .any(|row| json!(row.owner) == *owner)
        );
    }
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("magnified-area-support-delivery-v1").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-magnified-area-support-delivery-v1",
            &(a, b.clone(), d, v, migration, receiving),
            8 * 1024 * 1024,
        )
        .unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let mut added = vec![];
    for name in ["area", "cost_factor", "area_eligible"] {
        let id = registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address();
        assert_eq!(
            id.key().as_str(),
            b["channels"][name]["key"].as_str().unwrap()
        );
        added.push(id);
    }
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 3
    );
    restored
        .schema
        .definitions
        .retain(|x| !added.contains(&x.address()));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    let before = restored.rules.owners.len();
    restored
        .rules
        .owners
        .retain(|row| json!(row.owner) != subject(&b["supports"][0]["gem"]));
    assert_eq!(restored.rules.owners.len() + 1, before);
    for row in old {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == row.owner)
            .unwrap();
        *target = row;
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only three new Action channels and five appended support programs"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
