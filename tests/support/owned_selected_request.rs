//! Saved source selections and Pending request diagnostics shared by real release tests.
use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path, process::Command};
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap()
}
fn run(name: &str, args: &[&Path]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn attr<'a>(r: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    r.attribute(name).unwrap().decoded().unwrap()
}
fn link(sidecar: &Value, id: SourceOccurrenceId, kind: &str) -> Value {
    let row = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"] == serde_json::to_value(id).unwrap())
        .unwrap();
    let rows: Vec<_> = row["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .collect();
    assert_eq!(rows.len(), 1, "exact {kind} source link");
    rows[0]["value"].clone()
}
fn selected<'a, 's>(
    e: &'a SourceProjectEvidence<'s>,
    section: &str,
    child: &str,
    selector: &str,
) -> &'a SourceEvidenceRow<'s> {
    let owner = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == section)
        .unwrap();
    let wanted = attr(owner, selector);
    e.rows()
        .iter()
        .find(|r| {
            r.occurrence().parent() == Some(owner.occurrence().id())
                && r.occurrence().name() == child
                && attr(r, "id") == wanted
        })
        .unwrap()
}
pub fn selection(xml: &[u8], directory: &Path) -> Value {
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let d = draft.input();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        d.allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let sidecar: Value = read(directory.join("sidecar.json"));
    let items = selected(&e, "Items", "ItemSet", "activeItemSet");
    let skills = selected(&e, "Skills", "SkillSet", "activeSkillSet");
    let config = selected(&e, "Config", "ConfigSet", "activeConfigSet");
    let tree = e
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Tree")
        .unwrap();
    let index: usize = attr(tree, "activeSpec").parse().unwrap();
    let spec = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(tree.occurrence().id())
                && r.occurrence().name() == "Spec"
        })
        .nth(index - 1)
        .unwrap();
    let flag = attr(items, "useSecondWeaponSet");
    assert!(matches!(flag, "true" | "false" | "nil"));
    let loadout = if flag == "true" {
        "weapon-set-two"
    } else {
        "weapon-set-one"
    };
    let loadouts: BTreeSet<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["links"].as_array().unwrap())
        .filter(|v| v["kind"] == "weapon_loadout" && v["value"]["key"] == loadout)
        .map(|v| v["value"]["id"].to_string())
        .collect();
    assert_eq!(loadouts.len(), 1);
    assert_eq!(d.query_presets.members.len(), 1);
    json!({"build":{"character":link(&sidecar,spec.occurrence().id(),"character_preset"),"equipment":link(&sidecar,items.occurrence().id(),"equipment_preset"),"allocations":link(&sidecar,spec.occurrence().id(),"allocation_preset"),"skills":link(&sidecar,skills.occurrence().id(),"skill_preset"),"choices":link(&sidecar,config.occurrence().id(),"choice_preset"),"active_weapon_loadout":serde_json::from_str::<Value>(loadouts.first().unwrap()).unwrap()},"scenario":link(&sidecar,config.occurrence().id(),"scenario_preset"),"queries":d.query_presets.members[0].id})
}
pub fn canonical(v: &mut Value) {
    match v {
        Value::Object(o) => {
            if let Some(l) = o.get_mut("lineage") {
                *l = json!("canonical-test-lineage");
            }
            for v in o.values_mut() {
                canonical(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                canonical(v);
            }
        }
        _ => {}
    }
}
pub fn finalize(xml: &[u8], directory: &Path, out: &Path) -> Value {
    let s = selection(xml, directory);
    write(out, &s);
    let r = run(
        "check-owned-draft",
        &[&directory.join("draft.json"), Path::new("--selection"), out],
    );
    assert_eq!(r["finalization"]["status"], "pending");
    assert_eq!(r["verification"]["calculation"], "not_run");
    r
}
