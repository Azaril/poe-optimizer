//! Retained source evidence is an offline admission argument, not a runtime
//! certificate. In particular constructor captures do not observe late writes.
use super::dps;
#[allow(dead_code)]
#[path = "owned_offering_speed_rate.rs"]
pub(crate) mod family;
#[allow(dead_code)]
#[path = "owned_offering_speed_source_domain.rs"]
mod timing;
use timing as source_domain;
#[allow(dead_code)]
#[path = "owned_release_migration_preservation.rs"]
pub(crate) mod migration_preservation;
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, sync::OnceLock};

pub fn domain() -> &'static Value {
    static DOMAIN: OnceLock<Value> = OnceLock::new();
    DOMAIN.get_or_init(|| dps::read("source-domain.json"))
}
fn expanded(pin: &Value, limit: u64) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(&dps::authenticate(&pin["archive"], 2 * 1024 * 1024)[..])
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    assert_eq!(raw.len(), pin["bytes"]);
    assert_eq!(dps::hash(&raw), pin["sha256"]);
    raw
}
pub fn capture(index: usize) -> Value {
    serde_json::from_slice(&expanded(&domain()["inputs"][index], 32 * 1024 * 1024)).unwrap()
}
pub fn selected() -> Value {
    serde_json::from_slice(&dps::authenticate(
        &domain()["selected_recipes"],
        2 * 1024 * 1024,
    ))
    .unwrap()
}
/// No truthiness filtering: a zero value, false flag, inactive tag or nested
/// parent record is still a supplier that invalidates this neutral domain.
fn reject_supplier(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if map.contains_key("type")
                && domain()["target_names"]
                    .as_array()
                    .unwrap()
                    .contains(&value["name"])
            {
                return Err(format!("unreviewed multiplicity supplier at {path}"));
            }
            for (name, child) in map {
                if domain()["forbidden_fields"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(name))
                {
                    return Err(format!("nonordinary field {path}/{name}"));
                }
                reject_supplier(child, &format!("{path}/{name}"))?;
            }
        }
        Value::Array(rows) => {
            for (i, child) in rows.iter().enumerate() {
                reject_supplier(child, &format!("{path}/{i}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn review_capture(source: &Value, id: &str) -> Result<(), String> {
    timing::review_capture(source, id)?;
    for (name, count) in domain()["source_counts"].as_object().unwrap() {
        let rows = source["state"][name].as_array().ok_or("input container")?;
        if json!(rows.len()) != *count {
            return Err(format!("incomplete source {name}"));
        }
        reject_supplier(&source["state"][name], name)?;
    }
    for config in source["state"]["configuration"].as_array().unwrap() {
        if config["input"]["repeatMode"] != "AVERAGE"
            || config["default_state"]["repeatMode"] != "AVERAGE"
            || ![Value::Null, json!("AVERAGE")].contains(&config["input"]["ruthlessSupportMode"])
            || config["default_state"]["ruthlessSupportMode"] != "AVERAGE"
        {
            return Err("nonordinary repeat/ruthless configuration".into());
        }
    }
    let basic: Vec<_> = source["state"]["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["active_effect"]["id"] == "MinionMeleeBow")
        .collect();
    if basic.len() != 36 {
        return Err("complete Basic construction census".into());
    }
    for row in basic {
        // A table-valued triggeredBy is omitted by the scalar observer. Its
        // absence is established by the complete construction/support proof,
        // not this extra conservative check for an observable changed field.
        if row["active_effect"]["granted_effect_fields"]["name"] != "Basic Attack"
            || row["fields"].get("triggeredBy").is_some()
            || !row["socket_group"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            || row["skill_flags"].get("bothWeaponAttack").is_some()
        {
            return Err("unreviewed trigger/dual-weapon dispatch".into());
        }
    }
    Ok(())
}
pub fn review_selected(source: &Value) -> Result<(), String> {
    timing::review_selected_recipes(source)?;
    if source["skills"].as_array().ok_or("skill recipes")?.len() != 40 {
        return Err("complete selected recipe inventory".into());
    }
    for name in ["items", "tree", "skills", "minion_profiles"] {
        if source.get(name).is_none() {
            return Err("missing selected recipe container".into());
        }
        reject_supplier(&source[name], name)?;
    }
    Ok(())
}
pub fn review_body(source_path: &str, source: &str) -> Result<(), String> {
    let expected = domain()["source_scans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["source_path"] == source_path)
        .ok_or("unreviewed body")?;
    let rows: Vec<_> = source
        .lines()
        .enumerate()
        .filter(|(_, text)| {
            domain()["scan_tokens"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| text.contains(t.as_str().unwrap()))
        })
        .map(|(i, text)| json!({"line":i+1,"text":text}))
        .collect();
    if json!(rows) != expected["rows"] {
        return Err("late timing or multiplicity writer changed".into());
    }
    let evidence: Value = dps::read("source-evidence.json");
    for span in evidence["spans"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["source_path"] == source_path)
    {
        let actual = source
            .lines()
            .skip(span["start"].as_u64().unwrap() as usize - 1)
            .take((span["end"].as_u64().unwrap() - span["start"].as_u64().unwrap() + 1) as usize)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        if actual != span["text"] {
            return Err("complete branch or dispatcher boundary changed".into());
        }
    }
    Ok(())
}
pub fn check() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        dps::check();
        timing::check();
        let evidence: Value = dps::read("source-evidence.json");
        let manifest: Value =
            serde_json::from_slice(&dps::authenticate(&evidence["manifest"], 256 * 1024)).unwrap();
        assert_eq!(manifest["upstream_revision"], domain()["source_revision"]);
        dps::authenticate(&evidence["license"], 128 * 1024);
        dps::authenticate(&domain()["inherited_timing"], 2 * 1024 * 1024);
        let observer =
            String::from_utf8(expanded(&evidence["capture_observer"], 128 * 1024)).unwrap();
        for span in evidence["capture_observer"]["spans"].as_array().unwrap() {
            let text = observer
                .lines()
                .skip(span["start"].as_u64().unwrap() as usize - 1)
                .take(
                    (span["end"].as_u64().unwrap() - span["start"].as_u64().unwrap() + 1) as usize,
                )
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(text, span["text"]);
            assert_eq!(text.len(), span["bytes"]);
            assert_eq!(dps::hash(text.as_bytes()), span["sha256"]);
        }
        for name in [
            "native_membership_authorized",
            "complete_request",
            "all_damage_types",
            "saved_average_hit_authorized",
            "whole_build_complete",
        ] {
            assert_eq!(domain()["scope"][name], false);
        }
        let mut bodies = BTreeMap::new();
        assert_eq!(evidence["complete_bodies"].as_array().unwrap().len(), 12);
        for pin in evidence["complete_bodies"].as_array().unwrap() {
            let body = String::from_utf8(expanded(pin, 1024 * 1024)).unwrap();
            let path = pin["source_path"].as_str().unwrap();
            let authorities: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == path)
                .collect();
            assert_eq!(authorities.len(), 1);
            assert_eq!(authorities[0]["sha256"], pin["sha256"]);
            assert_eq!(authorities[0]["bytes"], pin["bytes"]);
            review_body(path, &body).unwrap();
            assert!(bodies.insert(path.to_owned(), body).is_none());
        }
        for span in evidence["spans"].as_array().unwrap() {
            let text = bodies[span["source_path"].as_str().unwrap()]
                .lines()
                .skip(span["start"].as_u64().unwrap() as usize - 1)
                .take(
                    (span["end"].as_u64().unwrap() - span["start"].as_u64().unwrap() + 1) as usize,
                )
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(text, span["text"]);
            assert_eq!(text.len(), span["bytes"]);
            assert_eq!(dps::hash(text.as_bytes()), span["sha256"]);
        }
        let triggers = &bodies["src/Modules/CalcTriggers.lua"];
        assert!(triggers.contains("local configTable = {"));
        assert!(!triggers.contains("[\"basic attack\"]"));
        assert!(triggers.contains("actor.mainSkill.skillData.triggered = nil"));
        assert_eq!(
            domain()["inherited_transport_proofs"],
            timing::domain()["inherited_proofs"]
        );
        for pin in domain()["inherited_transport_proofs"].as_array().unwrap() {
            dps::authenticate(pin, 128 * 1024);
        }
        for (index, id) in ["jit-off", "jit-on"].iter().enumerate() {
            review_capture(&capture(index), id).unwrap();
        }
        review_selected(&selected()).unwrap();
        let source: Value = serde_json::from_slice(&dps::authenticate(
            &evidence["captured_vectors"],
            128 * 1024,
        ))
        .unwrap();
        let vectors: Value = dps::read("source-vectors.json");
        assert_eq!(vectors["rows"].as_array().unwrap().len(), 39);
        assert_eq!(source["rows"].as_array().unwrap().len(), 39);
        let mut physical_only = 0;
        let mut nonphysical_controls = vec![];
        for (index, row) in vectors["rows"].as_array().unwrap().iter().enumerate() {
            assert_eq!(row["source_index"], index);
            let original = &source["rows"][index];
            for field in ["case", "mode", "effective", "label"] {
                assert_eq!(row[field], original[field]);
            }
            assert_eq!(row["source_occurrence"], original["source_occurrence"]);
            for (field, name) in [
                ("normal", "PhysicalHitAverage"),
                ("critical", "PhysicalCritAverage"),
                ("critical_percent", "CritChance"),
                ("hit_percent", "HitChance"),
                ("rate", "Speed"),
                ("multiplier", "DpsMultiplier"),
                ("expected_post_block", "AverageDamage"),
                ("expected_total", "TotalDPS"),
            ] {
                assert_eq!(row[field], original["output"][name]);
            }
            let other: Vec<_> = ["Lightning", "Cold", "Fire", "Chaos"]
                .into_iter()
                .map(|name| {
                    json!([
                        original["output"][format!("{name}HitAverage")],
                        original["output"][format!("{name}CritAverage")]
                    ])
                })
                .collect();
            assert_eq!(row["other_damage_types"], json!(other));
            let n = |name: &str| row[name].as_f64().unwrap();
            let weighted = n("normal") * (1. - n("critical_percent") / 100.)
                + n("critical") * n("critical_percent") / 100.;
            let blocked = weighted * n("hit_percent") / 100.;
            assert_eq!(n("multiplier"), 1.);
            // Source damage order is Physical, Lightning, Cold, Fire, Chaos.
            // These are comparison operands only, not an all-type native
            // producer or evidence that nonphysical channels are complete.
            let total_normal = other
                .iter()
                .fold(n("normal"), |sum, pair| sum + pair[0].as_f64().unwrap());
            let total_critical = other
                .iter()
                .fold(n("critical"), |sum, pair| sum + pair[1].as_f64().unwrap());
            let all_weighted = total_normal * (1. - n("critical_percent") / 100.)
                + total_critical * n("critical_percent") / 100.;
            let all_blocked = all_weighted * n("hit_percent") / 100.;
            assert!((all_blocked - n("expected_post_block")).abs() < 1e-9);
            assert!((all_blocked * n("rate") - n("expected_total")).abs() < 1e-9);
            if other.iter().all(|pair| {
                pair.as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v.as_f64() == Some(0.))
            }) {
                physical_only += 1;
                assert!((blocked - n("expected_post_block")).abs() < 1e-9);
                assert!((blocked * n("rate") - n("expected_total")).abs() < 1e-9);
            } else {
                assert!((blocked - n("expected_post_block")).abs() > 1e-9);
                assert!((blocked * n("rate") - n("expected_total")).abs() > 1e-9);
                nonphysical_controls.push(row["case"].as_str().unwrap());
            }
        }
        assert_eq!(physical_only, 36);
        assert_eq!(
            nonphysical_controls,
            [
                "conversion-fire-25",
                "conversion-over-100",
                "physical-gain-fire"
            ]
        );
    });
}
