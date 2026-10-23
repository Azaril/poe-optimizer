//! Authenticated offline evidence, with no source parser or runtime compatibility lane.
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

const BASE: &str = "data/owned/poe2/3887ae68/";
const EVIDENCE: &str = "gigantic-support-domains/evidence.json";
const EVIDENCE_SHA: &str = "a9d4141186f70758a0ba5687e13ca1f8ca06b0e258280dba03bdd53ce8feef59";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_SHA: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
struct Proof {
    artifacts: BTreeMap<String, Value>,
    sources: BTreeMap<String, String>,
    evidence: Value,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &str, limit: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    File::open(root().join(path))
        .unwrap()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= limit);
    bytes
}
fn pin(bytes: &[u8], record: &Value) {
    assert_eq!(record["bytes"], bytes.len());
    assert_eq!(record["sha256"], hash(bytes));
}
fn span(sources: &BTreeMap<String, String>, record: &Value) -> String {
    sources[record["path"].as_str().unwrap()]
        .lines()
        .skip(record["first"].as_u64().unwrap() as usize - 1)
        .take((record["last"].as_u64().unwrap() - record["first"].as_u64().unwrap() + 1) as usize)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
fn proof() -> &'static Proof {
    static PROOF: OnceLock<Proof> = OnceLock::new();
    PROOF.get_or_init(|| {
        let bytes = read(&format!("{BASE}{EVIDENCE}"), 128 * 1024);
        assert_eq!(hash(&bytes), EVIDENCE_SHA);
        let evidence: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence["schema_version"], 1);
        assert_eq!(evidence["source_revision"], REVISION);
        let manifest_bytes = read(MANIFEST, 1024 * 1024);
        assert_eq!(hash(&manifest_bytes), MANIFEST_SHA);
        let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        let mut sources = BTreeMap::new();
        assert_eq!(evidence["files"].as_array().unwrap().len(), 9);
        for file in evidence["files"].as_array().unwrap() {
            let compressed = read(file["archive"].as_str().unwrap(), 2 * 1024 * 1024);
            assert_eq!(file["archive_bytes"], compressed.len());
            assert_eq!(file["archive_sha256"], hash(&compressed));
            let mut bytes = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            assert!(bytes.len() <= 8 * 1024 * 1024);
            pin(&bytes, file);
            let path = file["path"].as_str().unwrap();
            if path != "src/TreeData/0_5/tree.json" {
                let entry = manifest["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["path"] == path)
                    .unwrap();
                pin(&bytes, entry);
            }
            assert!(
                sources
                    .insert(path.to_owned(), String::from_utf8(bytes).unwrap())
                    .is_none()
            );
        }
        assert_eq!(evidence["spans"].as_array().unwrap().len(), 22);
        let mut roles = BTreeSet::new();
        for record in evidence["spans"].as_array().unwrap() {
            assert!(roles.insert(record["role"].as_str().unwrap()));
            assert_eq!(record["sha256"], hash(span(&sources, record).as_bytes()));
        }
        let mut artifacts = BTreeMap::new();
        assert_eq!(evidence["artifacts"].as_array().unwrap().len(), 15);
        for record in evidence["artifacts"].as_array().unwrap() {
            let path = record["path"].as_str().unwrap();
            let bytes = read(&format!("{BASE}{path}"), 32 * 1024 * 1024);
            pin(&bytes, record);
            assert!(
                artifacts
                    .insert(path.into(), serde_json::from_slice(&bytes).unwrap())
                    .is_none()
            );
        }
        let proof = Proof {
            artifacts,
            sources,
            evidence,
        };
        verify(&proof).unwrap();
        proof
    })
}
pub(super) fn artifact(path: &str) -> &'static Value {
    &proof().artifacts[path]
}
fn require(value: bool, message: &str) -> Result<(), String> {
    value.then_some(()).ok_or_else(|| message.into())
}
fn expected_raw() -> Value {
    json!({"activeEffectImage":"Art/2DArt/UIImages/InGame/PassiveMastery/MasteryBackgroundGraphic/MasteryMinionOffencePattern",
        "connections":[],"group":669,"icon":"Art/2DArt/SkillIcons/passives/MinionsandManaNode.dds","isNotable":true,
        "name":"Gigantic Following","orbit":0,"orbitIndex":0,"recipe":["Envy","Guilt","Isolation"],"skill":46365,
        "stats":["Your Minions are Gigantic","25% reduced Reservation Efficiency of Minion Skills"],"stringId":"minion_reservation16"})
}
fn verify_node(raw: &Value, catalog: &Value) -> Result<(), String> {
    require(
        raw == &expected_raw(),
        "complete twelve-field raw definition",
    )?;
    let rows = catalog["nodes"].as_array().ok_or("catalog nodes")?;
    let matching: Vec<_> = rows.iter().filter(|row| row["key"] == "46365").collect();
    require(matching.len() == 1, "exact catalog node")?;
    require(
        *matching[0]
            == json!({"key":"46365","kind":{"kind":"allocation","value":{"pool":"ordinary"}},
        "stats":raw["stats"],"views":[],"unlock":[]}),
        "full catalog default",
    )?;
    require(
        rows.iter()
            .all(|row| row["kind"]["value"]["parent"] != "46365"),
        "no attached choice",
    )?;
    let roots: BTreeSet<_> = catalog["classes"]
        .as_array()
        .ok_or("classes")?
        .iter()
        .map(|class| class["root"].as_str().unwrap())
        .collect();
    require(
        catalog["edges"]
            .as_array()
            .ok_or("edges")?
            .iter()
            .all(|edge| {
                !((edge["left"] == "46365" && roots.contains(edge["right"].as_str().unwrap()))
                    || (edge["right"] == "46365" && roots.contains(edge["left"].as_str().unwrap())))
            }),
        "no class-root injection",
    )
}
fn verify(proof: &Proof) -> Result<(), String> {
    let raw: Value = serde_json::from_str(&proof.sources["src/TreeData/0_5/tree.json"])
        .map_err(|e| e.to_string())?;
    verify_node(
        &raw["nodes"]["46365"],
        &proof.artifacts["tree/tree-catalog.json"],
    )?;
    let vectors = &proof.artifacts["gigantic-following/source-vectors.json"];
    let record = proof.evidence["spans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["role"] == "complete-gigantic-node")
        .unwrap();
    require(
        span(&proof.sources, record) == vectors["static_node"]["text"],
        "complete executed Lua definition",
    )?;
    require(
        vectors["static_node"]["first_line"] == 103965
            && vectors["static_node"]["last_line"] == 103986,
        "outer node boundary",
    )?;
    let parser = &proof.sources["src/Modules/ModParser.lua"];
    for exact in [
        "[\"your minions are gigantic\"] = {\n\t\tmod(\"MinionModifier\", \"LIST\", { mod = flag(\"Gigantic\") }),\n\t}",
        "[\"reservation efficiency\"] = \"ReservationEfficiency\"",
        "[\"of minion skills\"] = { tag = { type = \"SkillType\", skillType = SkillType.Minion } }",
        "mod(\"ExtraSupport\"",
        "mod(\"LinkedSupport\"",
        "mod(\"ExtraMinionSkill\"",
    ] {
        require(
            parser.contains(exact),
            "reviewed producer or external contrast",
        )?;
    }
    let perform = &proof.sources["src/Modules/CalcPerform.lua"];
    for exact in [
        "modDB:Flag(nil, \"Gigantic\")",
        "modDB:NewMod(\"Life\", \"MORE\", 20, \"Gigantic\")",
        "modDB:NewMod(\"Damage\", \"MORE\", 20, \"Gigantic\")",
    ] {
        require(perform.contains(exact), "paired original benefits")?;
    }
    Ok(())
}

#[test]
fn source_proof_covers_complete_node_construction_and_rejects_extra_raw_authority() {
    let proof = proof();
    verify(proof).unwrap();
    for field in ["options", "skill_grants", "callbacks"] {
        let mut raw = expected_raw();
        raw[field] = json!([]);
        assert!(verify_node(&raw, &proof.artifacts["tree/tree-catalog.json"]).is_err());
    }
    let mut raw = expected_raw();
    raw["stats"][1] = json!("25% increased Reservation Efficiency of Minion Skills");
    assert!(verify_node(&raw, &proof.artifacts["tree/tree-catalog.json"]).is_err());
    let mut catalog = proof.artifacts["tree/tree-catalog.json"].clone();
    catalog["nodes"].as_array_mut().unwrap().push(
        json!({"key":"unexpected-child","kind":{"kind":"choice","value":{"parent":"46365"}}}),
    );
    assert!(verify_node(&expected_raw(), &catalog).is_err());
}
// A bounded evidence assertion, not a Lua interpreter or a native data format.
// Typed keys preserve mixed array/string fields; cyclic graphs are rejected here.
fn graph_value(graph: &Value, atom: &Value, depth: usize) -> Value {
    assert!(depth < 24);
    match atom["kind"].as_str().unwrap() {
        "nil" => Value::Null,
        "boolean" => atom["value"].clone(),
        "number" => {
            let n = f64::from_bits(atom["value"].as_u64().unwrap());
            assert!(n.is_finite());
            json!(n)
        }
        "bytes" => json!(
            String::from_utf8(serde_json::from_value(atom["value"].clone()).unwrap()).unwrap()
        ),
        "table" => {
            let mut object = serde_json::Map::new();
            for pair in graph["tables"][atom["value"].as_u64().unwrap() as usize]
                .as_array()
                .unwrap()
            {
                let key = match pair[0]["kind"].as_str().unwrap() {
                    "bytes" => format!(
                        "s:{}",
                        graph_value(graph, &pair[0], depth + 1).as_str().unwrap()
                    ),
                    "number" => format!("n:{}", f64::from_bits(pair[0]["value"].as_u64().unwrap())),
                    other => panic!("unexpected default inventory key {other}"),
                };
                assert!(
                    object
                        .insert(key, graph_value(graph, &pair[1], depth + 1))
                        .is_none()
                );
            }
            Value::Object(object)
        }
        other => panic!("unexpected default inventory atom {other}"),
    }
}
fn default_records(with_source: bool) -> [Value; 2] {
    let mut gigantic = json!({"s:flags":0.0,"s:keywordFlags":0.0,"s:name":"MinionModifier","s:type":"LIST",
      "s:value":{"s:mod":{"s:flags":0.0,"s:keywordFlags":0.0,"s:name":"Gigantic","s:type":"FLAG","s:value":true}}});
    let mut reservation = json!({"n:1":{"s:skillType":6.0,"s:type":"SkillType"},"s:flags":0.0,"s:keywordFlags":0.0,
      "s:name":"ReservationEfficiency","s:type":"INC","s:value":-25.0});
    if with_source {
        gigantic["s:source"] = json!("Tree:46365");
        gigantic["s:value"]["s:mod"]["s:source"] = json!("Tree:46365");
        reservation["s:source"] = json!("Tree:46365");
    }
    [gigantic, reservation]
}
fn check_default_graph(v: &Value) {
    let r = &v["default_report"];
    assert_eq!(
        r["determinism"],
        json!({"fresh_jit_off":2,"fresh_jit_on":1,"warm_parser_and_process_stats":true})
    );
    let default = &r["default"]["value"];
    assert_eq!(default["source_id"], 46365);
    assert_eq!(default["name"], "Gigantic Following");
    assert_eq!(default["accepted_count"], 2);
    assert_eq!(default["unknown"], false);
    assert_eq!(default["extra"], false);
    assert_eq!(
        default["lines"],
        json!([
      {"combined":false,"extra":null,"line":"Your Minions are Gigantic","list_present":true},
      {"combined":false,"extra":null,"line":"25% reduced Reservation Efficiency of Minion Skills","list_present":true}])
    );
    let graph = &default["full_default_graph"];
    assert_eq!(graph["callbacks"], json!([]));
    assert_eq!(graph["roots"].as_array().unwrap().len(), 7);
    let [gigantic, reservation] = default_records(true);
    assert_eq!(
        graph_value(graph, &graph["roots"][2], 0),
        json!({"n:1":gigantic,"n:2":reservation})
    );
    assert_eq!(graph_value(graph, &graph["roots"][3], 0), false);
    assert_eq!(graph_value(graph, &graph["roots"][4], 0), false);
    let parser = r["parser"].as_array().unwrap();
    assert_eq!(parser.len(), 2);
    for (entry, (line, index)) in parser.iter().zip([
        ("25% reduced Reservation Efficiency of Minion Skills", 1),
        ("Your Minions are Gigantic", 0),
    ]) {
        let row = &entry["value"];
        assert_eq!(row["line"], line);
        assert_eq!(row["disk_entry_present"], true);
        assert_eq!(row["disk_return_matches_live"], true);
        let graph = &row["fresh_and_warm"];
        assert_eq!(graph["callbacks"], json!([]));
        assert_eq!(graph["roots"].as_array().unwrap().len(), 1);
        assert_eq!(
            graph_value(graph, &graph["roots"][0], 0),
            json!({"n:1":default_records(false)[index]})
        );
        assert_eq!(row["disk_return"], *graph);
    }
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert_eq!(value, &json!({}));
        &[]
    }
}
fn source_mod(stat: &str) -> Value {
    json!({"flags":0,"keyword_flags":0,"name":stat,"source":"Gigantic","tags":{},"type":"MORE","value":20})
}
#[test]
fn retained_default_records_and_actual_consumer_observations_keep_their_original_scope() {
    let following = artifact("gigantic-following/source-vectors.json");
    check_default_graph(following);
    let default = &following["default_report"]["default"]["value"];
    assert_eq!(default["source_id"], 46365);
    assert_eq!(default["accepted_count"], 2);
    assert_eq!(default["unknown"], false);
    assert_eq!(default["extra"], false);
    assert_eq!(default["full_default_graph"]["callbacks"], json!([]));
    assert_eq!(
        default["lines"],
        json!([
        {"combined":false,"extra":null,"line":"Your Minions are Gigantic","list_present":true},
        {"combined":false,"extra":null,"line":"25% reduced Reservation Efficiency of Minion Skills","list_present":true}])
    );
    assert_eq!(
        following["default_report"]["determinism"],
        json!({"fresh_jit_off":2,"fresh_jit_on":1,"warm_parser_and_process_stats":true})
    );
    for field in [
        "whole_build_parity",
        "effective_transforms_proved_absent",
        "incoming_contributor_inventory_closed",
        "reservation_action_delivery_published",
    ] {
        assert_eq!(following["scope"][field], false);
    }
    let benefits = artifact("gigantic-benefits/source-vectors.json");
    assert_eq!(benefits["individual_factor"], 1.2);
    assert_eq!(benefits["composed_factor_parity"], false);
    let reports = benefits["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    let observations = reports[0]["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 18);
    for observation in observations {
        let index = observation["case_index"].as_u64().unwrap();
        let projections = observation["projections"].as_array().unwrap();
        assert_eq!(projections.len(), 10);
        let values: Vec<_> = projections.iter().map(|p| &p["value"]).collect();
        let removed = index == 2;
        let applies = !removed && values[1].as_bool() == Some(true);
        assert_eq!(values[4].as_str(), Some("RaisedSkeletonSniper"));
        assert_eq!(values[9].as_str(), Some("MinionMeleeBow"));
        assert_eq!(values[5]["source_present"], true);
        assert_eq!(values[5]["matches"].as_array().unwrap().len(), 1);
        assert_eq!(
            values[5]["matches"][0]["gem_id"],
            "Metadata/Items/Gems/SkillGemSkeletalSniper"
        );
        assert_eq!(values[7]["exact_parent"], true);
        assert_eq!(values[7]["exact_summoner"], true);
        let calls = rows(&values[7]["original_life_calls"]);
        if values[6].as_bool() == Some(true) {
            assert!(!calls.is_empty());
        }
        for call in calls {
            for field in [
                "exact_actor_store",
                "exact_actor_output",
                "summoner_owns_actor",
                "summoner_actor_is_parent",
            ] {
                assert_eq!(call[field], true);
            }
            assert_eq!(call["source"], *values[5]);
            assert_eq!(call["return_life"], call["post_return_life"]);
            assert!(
                call["post_return_observed_at"].as_u64().unwrap()
                    > call["caller_line"].as_u64().unwrap()
            );
            let c = &call["computation"];
            assert_eq!(c["observed_at"], 97);
            assert_eq!(c["override_present"], false);
            assert_eq!(c["life_precision"]["present"], false);
            assert_eq!(c["life_after_assignment"], call["return_life"]);
            assert_eq!(c["gigantic"], !removed);
            assert_eq!(
                rows(&c["gigantic_records"]).len(),
                if removed {
                    0
                } else if index == 4 {
                    2
                } else {
                    1
                }
            );
            assert_eq!(c["more"], if applies { 1.2 } else { 1.0 });
            assert_eq!(rows(&c["eligible_life_more"]).len(), usize::from(applies));
            for row in rows(&c["eligible_life_more"]) {
                assert_eq!(row["value"], 20);
                assert_eq!(row["mod"], source_mod("Life"));
            }
            for stat in ["Life", "Damage"] {
                let generated: Vec<_> = rows(&c["raw_modifiers"])
                    .iter()
                    .filter(|row| row["mod"]["source"] == "Gigantic" && row["mod"]["name"] == stat)
                    .collect();
                assert_eq!(generated.len(), usize::from(applies));
                for row in generated {
                    assert_eq!(row["ancestor_depth"], 0);
                    assert_eq!(row["mod"], source_mod(stat));
                }
            }
        }
        if let Some(last) = calls.last() {
            assert_eq!(last["post_return_life"], values[7]["actor_output_life"]);
        }
        if values[6].as_bool() == Some(true) {
            let calls: Vec<_> = rows(values[8])
                .iter()
                .filter(|row| row["damage_type"] == "Physical" && row["critical"] == false)
                .collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["more_factor"], if applies { 1.2 } else { 1.0 });
            let generated: Vec<_> = rows(&calls[0]["more_records"])
                .iter()
                .filter(|row| row["mod"]["source"] == "Gigantic")
                .collect();
            assert_eq!(generated.len(), usize::from(applies));
            for row in generated {
                assert_eq!(row["mod"], source_mod("Damage"));
            }
        } else {
            assert_eq!(values[8]["effect_id"], *values[9]);
            assert!(values[8].get("damage_calls").is_none());
        }
        for projection in projections {
            assert!(
                projection["pointer"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("/cases/{index}/"))
            );
        }
    }
    for repeat in [1, 3] {
        for mode in ["main", "calcs"] {
            let values = |case: usize| {
                observations
                    .iter()
                    .find(|row| row["case_index"] == case && row["mode"] == mode)
                    .unwrap()["projections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .skip(1)
                    .map(|p| &p["value"])
                    .collect::<Vec<_>>()
            };
            assert_eq!(values(0), values(repeat));
        }
    }
}
