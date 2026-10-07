//! Publication-only authentication of the compact authoring vectors against the
//! retained full reports. Ordinary CI authenticates the pinned source separately.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn list(value: &Value) -> &[Value] {
    match value {
        Value::Array(values) => values,
        Value::Object(map) if map.is_empty() => &[],
        _ => panic!("unexpected source list"),
    }
}

pub fn authenticate_reports(root: &Path, proof: &Value) {
    let reports = proof["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let mut first = None;
    for pin in reports {
        let bytes = fs::read(root.join(pin["path"].as_str().unwrap()))
            .expect("publication requires retained full Sand source report");
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        if let Some(prior) = &first {
            assert_eq!(&bytes, prior, "independent source JIT modes");
        } else {
            first = Some(bytes);
        }
    }
    let full: Value = serde_json::from_slice(first.as_ref().unwrap()).unwrap();
    assert_eq!(full["source_revision"], proof["source_revision"]);
    assert_eq!(full["manifest_sha256"], proof["source_manifest_sha256"]);
    assert_eq!(full["files"], proof["source_files"]);
    for k in [
        "native_build_parity",
        "native_inventory_authority",
        "fallback_semantics_authority",
        "business_wrappers",
        "source_tables_mutated",
        "diagnostic_queries_as_consumption",
    ] {
        assert_eq!(full[k], false);
    }
    assert_eq!(full["lifecycle_stages"], proof["lifecycle_stages"]);
    assert_eq!(
        full["cases"].as_array().unwrap().len(),
        proof["case_count"].as_u64().unwrap() as usize
    );
    let vectors = proof["vectors"].as_array().unwrap();
    let mut matched = vec![0; vectors.len()];
    for case in full["cases"].as_array().unwrap() {
        for (stage, state) in case["states"].as_object().unwrap() {
            let props = &state["properties"];
            for k in [
                "original_methods_preserved",
                "hook_removed",
                "jit_mode_preserved",
            ] {
                assert_eq!(props[k], true);
            }
            for k in ["source_tables_mutated", "business_wrappers"] {
                assert_eq!(props[k], false);
            }
            assert_eq!(props["minion_level_table"], proof["minion_level_table"]);
            for row in props["contexts"].as_array().unwrap().iter().filter(|r| {
                matches!(
                    r["effect"].as_str(),
                    Some("SummonSandDjinnPlayer" | "CommandSandDjinnKnifeThrowPlayer")
                )
            }) {
                let ordinary:Vec<_>=list(&row["ordinary"]).iter().map(|o|json!({"before":o["before"],"after":o["after"],"matched":list(&o["matched"]).iter().map(|i|o["candidates"][i.as_u64().unwrap()as usize-1]["mod"].clone()).collect::<Vec<_>>() })).collect();
                let supported:Vec<_>=list(&row["supported"]).iter().map(|s|{
     // The retained Lua serializer represents an empty table as {}. Converting
     // it to an empty inventory is confined to this acquisition witness.
     let length=|v:&Value|match v{Value::Array(a)=>a.len(),Value::Object(m) if m.is_empty()=>0,_=>panic!("unexpected source list")};
     assert_eq!(length(&s["properties"]),0,"nonzero property evidence needs an explicit converter");
     let supports=list(&s["supports"]);
     for support in supports {
         assert_eq!(support["hidden"],false);
         for fact in ["is_supporting","source_joined","source_present"] { assert_eq!(support[fact],true); }
         // Source membership is not per-effect admission: e.g. Muster is
         // retained for the summon while absent from Command's effects.
     }
     let counts:Vec<_>=list(&s["query"]["chain"]).iter().flat_map(|chain|list(&chain["rows"])).map(|row|&row["mod"]).filter(|m|m["name"]=="Multiplier:SupportCount"&&m["source"]=="Support Count").collect();
     assert_eq!(counts.len(),1,"authenticate the original query's actual source count");
     assert_eq!(counts[0]["value"].as_u64(),Some(supports.len() as u64));
     json!({"before":s["before"],"after":s["after"],"count":length(&s["supports"]),"properties":[],"query_parent_is_actor":s["query_parent_is_actor"],"cache_identity_preserved":s["cache_identity_preserved"]})
    }).collect();
                let projected = json!({"case":case["name"],"mode":row["mode"],"effect":row["effect"],"source":row["source"],"group":row["group"],"catalog":row["catalog"],"loaded":row["loaded"]["input"],"raw":row["raw"],"ordinary":ordinary,"supported":supported,"final":row["final"],"population":row["population"]});
                let indexes: Vec<_> = vectors
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| {
                        v["observed_stages"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|s| s == stage)
                    })
                    .filter(|(_, v)| {
                        let mut without_stage = (*v).clone();
                        without_stage
                            .as_object_mut()
                            .unwrap()
                            .remove("observed_stages");
                        without_stage == projected
                    })
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(
                    indexes.len(),
                    1,
                    "every full source observation has one exact compact counterpart"
                );
                matched[indexes[0]] += 1;
            }
        }
    }
    assert!(
        matched.iter().all(|&n| n == 3),
        "every compact vector was observed at all three fixed stages"
    );
}
