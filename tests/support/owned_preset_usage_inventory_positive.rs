//! Source-range-derived positive/negative controls over the actual published package.
use crate::{family, preservation, release, selected};
use family::{Output, checked_output, data, root, sha, write};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    ops::Range,
    path::{Path, PathBuf},
};
const INPUTS_SHA: &str = "c126a783210d302632903bb34faa9bc0d1776f15e35166d4d40560a0be0312ac";
const SOURCE_IDS: [&str; 4] = [
    "IceNovaPlayer",
    "SupportEncroachingGroundPlayer",
    "SupportMagnifiedAreaPlayerTwo",
    "SupportRapidCastingPlayer",
];
const GEM_IDS: [&str; 4] = [
    "def.00000000000007dd",
    "def.0000000000000740",
    "def.000000000000082b",
    "def.00000000000008bf",
];
fn verify_pin(pin: &Value) {
    let b = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(json!(sha(&b)), pin["sha256"]);
    assert_eq!(json!(b.len()), pin["bytes"]);
}
pub(super) fn inputs() -> Value {
    let b = fs::read(data().join("positive-inputs.json")).unwrap();
    assert_eq!(sha(&b), INPUTS_SHA);
    let v: Value = serde_json::from_slice(&b).unwrap();
    for p in v["originals"].as_array().unwrap() {
        verify_pin(p)
    }
    verify_pin(&v["publication_authoring"]);
    verify_pin(&v["publication_inventory"]);
    v
}
pub(super) fn instance(xml: &[u8]) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn packages(before: &Path, after: &Path, pins: &Value) -> (StagedOwnedRelease, StagedOwnedRelease) {
    let bytes = fs::read(before.join("release.json")).unwrap();
    assert_eq!(json!(sha(&bytes)), pins["prior_release"]["sha256"]);
    assert_eq!(json!(bytes.len()), pins["prior_release"]["bytes"]);
    family::checked_predecessor(before);
    let prior = release::load(before);
    let next = release::load(after);
    family::checked_successor(&prior, &next);
    (prior, next)
}
#[derive(Clone)]
struct Frame {
    set: usize,
    group: usize,
    gems: [usize; 4],
    group_range: Range<usize>,
}
fn frame(xml: &[u8]) -> Frame {
    let b = instance(xml);
    let e = SourceProjectEvidence::collect(&b, SourceEvidenceLimits::default()).unwrap();
    let r = e.rows();
    let skills: Vec<_> = r
        .iter()
        .filter(|r| r.occurrence().name() == "Skills")
        .collect();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].children().len(), 1);
    let set = &r[skills[0].children()[0].ordinal() as usize];
    assert_eq!(set.occurrence().name(), "SkillSet");
    assert_eq!(set.children().len(), 1);
    assert_eq!(
        skills[0]
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap(),
        set.attribute("id").unwrap().decoded().unwrap()
    );
    let group = &r[set.children()[0].ordinal() as usize];
    assert_eq!(group.occurrence().name(), "Skill");
    assert_eq!(group.children().len(), 4);
    let gems = std::array::from_fn(|i| {
        let g = &r[group.children()[i].ordinal() as usize];
        assert_eq!(g.occurrence().name(), "Gem");
        assert_eq!(
            g.attribute("skillId").unwrap().decoded().unwrap(),
            SOURCE_IDS[i]
        );
        assert!(g.children().is_empty());
        g.occurrence().id().ordinal() as usize
    });
    Frame {
        set: set.occurrence().id().ordinal() as usize,
        group: group.occurrence().id().ordinal() as usize,
        gems,
        group_range: group.occurrence().range(),
    }
}

pub(super) fn derive(xml: &[u8]) -> (Vec<u8>, Value) {
    let b = instance(xml);
    let e = SourceProjectEvidence::collect(&b, SourceEvidenceLimits::default()).unwrap();
    let r = e.rows();
    let group = &r[238];
    assert_eq!(group.occurrence().name(), "Skill");
    assert_eq!(
        group
            .children()
            .iter()
            .map(|s| s.ordinal())
            .collect::<Vec<_>>(),
        [239, 240, 241, 242]
    );
    for (id, name) in (239..=242).zip(SOURCE_IDS) {
        assert_eq!(r[id].attribute("skillId").unwrap().decoded().unwrap(), name)
    }
    let set = &r[group.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(set.occurrence().name(), "SkillSet");
    let skills = &r[set.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(skills.occurrence().name(), "Skills");
    assert_eq!(
        skills
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap(),
        set.attribute("id").unwrap().decoded().unwrap()
    );
    assert_eq!(set.attribute("id").unwrap().decoded().unwrap(), "4");
    let gr = group.occurrence().range();
    let sr = set.occurrence().range();
    let sk = skills.occurrence().range();
    let first_set = r[skills.children().first().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let last_set = r[skills.children().last().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let first_group = r[set.children().first().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let last_group = r[set.children().last().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let ranges = [
        0..sk.start,
        sk.start..first_set.start,
        sr.start..first_group.start,
        gr.clone(),
        last_group.end..sr.end,
        last_set.end..sk.end,
        sk.end..xml.len(),
    ];
    assert!(ranges.windows(2).all(|p| p[0].end <= p[1].start));
    let mut derived = Vec::new();
    let mut segments = Vec::new();
    for range in ranges {
        let start = derived.len();
        derived.extend_from_slice(&xml[range.clone()]);
        segments.push(json!({"original_range":[range.start,range.end],"derived_range":[start,derived.len()],"sha256":sha(&xml[range])}));
    }
    let f = frame(&derived);
    assert_eq!(
        &derived[f.group_range.clone()],
        &xml[gr.clone()],
        "entire original group bytes preserved"
    );
    assert_eq!(&derived[..sk.start], &xml[..sk.start]);
    assert_eq!(
        &derived[derived.len() - (xml.len() - sk.end)..],
        &xml[sk.end..]
    );
    let proof = json!({"kind":"test_only_single_group_frame","original_sha256":sha(xml),"original_bytes":xml.len(),"derived_sha256":sha(&derived),"derived_bytes":derived.len(),"original_group":238,"original_group_range":[gr.start,gr.end],"group_sha256":sha(&xml[gr]),"derived_group":f.group,"derived_gems":f.gems,"retained_segments":segments,"only_skills_section_changed":true,"original_group_bytes_unchanged":true,"production_import_rewrite":false});
    (derived, proof)
}

fn nonneutral(xml: &[u8]) -> (Vec<u8>, Value) {
    let f = frame(xml);
    let b = instance(xml);
    let e = SourceProjectEvidence::collect(&b, SourceEvidenceLimits::default()).unwrap();
    let a = e.rows()[f.gems[1]].attribute("count").unwrap();
    assert_eq!(a.raw(), "1");
    assert_eq!(a.decoded().unwrap(), "1");
    let range = a.range();
    assert_eq!(&xml[range.clone()], b"1");
    assert_eq!(range.len(), 1);
    let mut changed = xml.to_vec();
    changed[range.start] = b'2';
    assert_eq!(changed.iter().zip(xml).filter(|(a, b)| a != b).count(), 1);
    assert_eq!(frame(&changed).gems, f.gems);
    let proof = json!({"kind":"test_only_support_count_control","positive_sha256":sha(xml),"negative_sha256":sha(&changed),"source_ordinal":f.gems[1],"attribute":"count","old":"1","new":"2","value_range":[range.start,range.end],"other_bytes_identical":true});
    (changed, proof)
}

pub(super) fn usage(draft: &Value) -> &Value {
    let presets = draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(presets.len(), 1);
    assert!(
        !presets[0]["intent"].is_null(),
        "actual production preset intent"
    );
    assert!(presets[0].get("usage_preferences").is_none());
    &presets[0]["intent"]["usage"]
}

fn usage_mut(draft: &mut Value) -> &mut Value {
    &mut draft["draft"]["skill_presets"]["members"][0]["intent"]["usage"]
}

fn actual_records(out: &Output, xml: &[u8], pending_parameter_source: Option<usize>) -> Value {
    let f = frame(xml);
    let d = &out.draft;
    let s = &out.sidecar;
    assert_eq!(d["draft"]["skills"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(d["draft"]["gems"]["members"].as_array().unwrap().len(), 4);
    assert_eq!(
        d["draft"]["supports"]["members"].as_array().unwrap().len(),
        3
    );
    let primary = preservation::link(s, f.gems[0], "skill");
    let preset = preservation::link(s, f.set, "skill_preset");
    assert_eq!(d["draft"]["skill_presets"]["members"][0]["id"], preset);
    for (i, ordinal) in f.gems.into_iter().enumerate() {
        let gem = preservation::link(s, ordinal, "gem");
        let record = preservation::member(d, "gems", &gem);
        assert_eq!(record["definition"]["kind"], "known");
        assert_eq!(record["definition"]["value"]["key"], GEM_IDS[i]);
        if pending_parameter_source == Some(i) {
            assert_eq!(record["parameters"]["completion"]["kind"], "pending");
            assert_eq!(
                record["parameters"]["completion"]["code"],
                "gem-parameters-not-converted"
            );
        } else {
            assert_eq!(record["parameters"]["completion"]["kind"], "complete");
        }
        if i > 0 {
            let support = preservation::link(s, ordinal, "support");
            let assignment = preservation::member(d, "supports", &support);
            assert_eq!(assignment["support"], json!({"kind":"known","value":gem}));
            assert_eq!(
                assignment["target"],
                json!({"kind":"authored","value":{"kind":"known","value":primary}})
            );
            assert_eq!(assignment["enabled"], json!({"kind":"known","value":true}));
        }
    }
    let members = usage(d)["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    let mut policies = BTreeSet::new();
    for row in members {
        let p = &row["selection"]["policy"];
        assert_eq!(p["kind"], "known");
        policies.insert(p["value"]["key"].as_str().unwrap().to_owned());
        let target = &row["selection"]["target"];
        assert_eq!(target["kind"], "skill");
        assert_eq!(target["value"]["kind"], "generated");
        assert_eq!(
            target["value"]["value"]["provider"]["root"],
            json!({"kind":"skill_use","value":{"kind":"known","value":primary}})
        );
        assert_eq!(
            target["value"]["value"]["provider"]["grant_path"],
            json!({"members":[],"completion":{"kind":"complete"}})
        );
        assert_eq!(
            target["value"]["value"]["slot"]["value"]["slot"]["key"],
            "def.000000000000327e"
        );
        assert_eq!(
            row["selection"]["parameters"]["completion"]["kind"],
            "complete"
        );
    }
    assert_eq!(
        policies,
        BTreeSet::from([
            "def.000000000000326a".into(),
            "def.000000000000332b".into(),
            "def.000000000000338e".into()
        ])
    );
    primary
}

fn compare_positive(before: &Output, after: &Output, xml: &[u8]) -> Value {
    actual_records(before, xml, None);
    actual_records(after, xml, None);
    let f = frame(xml);
    compare_usage_retirement(before, after, &[f.group, f.gems[0]])
}
pub(super) fn compare_usage_retirement(
    before: &Output,
    after: &Output,
    expected_origins: &[usize],
) -> Value {
    let old = usage(&before.draft);
    let new = usage(&after.draft);
    assert_eq!(old["members"], new["members"]);
    assert_eq!(old["completion"]["kind"], "pending");
    assert_eq!(old["completion"]["code"], "usage-preferences-not-converted");
    assert_eq!(new["completion"], json!({"kind":"complete"}));
    let issue = old["completion"]["id"].clone();
    let mut expected = before.draft.clone();
    usage_mut(&mut expected)["completion"] = json!({"kind":"complete"});
    assert_eq!(expected, after.draft, "exact one-field draft inverse");
    let target = json!({"kind":"issue","value":issue});
    let mut sidecar = before.sidecar.clone();
    let mut linked = Vec::new();
    for origin in sidecar["origins"].as_array_mut().unwrap() {
        let ordinal = origin["source"]["ordinal"].as_u64().unwrap() as usize;
        let links = origin["links"].as_array_mut().unwrap();
        let count = links.iter().filter(|l| **l == target).count();
        assert!(count <= 1);
        if count == 1 {
            linked.push(ordinal);
            links.retain(|l| *l != target)
        }
    }
    assert_eq!(
        linked, expected_origins,
        "exact expected source usage provenance"
    );
    assert_eq!(
        sidecar, after.sidecar,
        "all other origin links and allocator fields preserved"
    );
    let mut report = before.report.clone();
    let count = report["issue_count"].as_u64().unwrap();
    report["issue_count"] = json!(count.checked_sub(1).unwrap());
    assert_eq!(report["issue_codes"]["usage-preferences-not-converted"], 1);
    report["issue_codes"]
        .as_object_mut()
        .unwrap()
        .remove("usage-preferences-not-converted");
    assert_eq!(
        report, after.report,
        "exact one retired diagnostic in the whole report"
    );
    json!({"retired_issue":issue,"retired_origin_ordinals":linked,"all_other_draft_and_sidecar_content_identical":true,"allocator_unchanged":true,"native_execution":"not_run"})
}

fn finalize(xml: &[u8], dir: &Path, package: &Path, out: &Path) -> Value {
    selected::finalize_with_definitions(xml, dir, out, &package.join("schema.json"))
}
pub fn run() {
    let before = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_GROUP238_PRIOR").unwrap());
    let after = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_GROUP238_NEXT").unwrap());
    let out = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_GROUP238_OUTPUT").unwrap());
    assert!(!out.exists());
    let pins = inputs();
    let before_files = release::inventory(&before);
    let after_files = release::inventory(&after);
    let (prior, next) = packages(&before, &after, &pins);
    let original = fs::read(root().join(pins["originals"][4]["path"].as_str().unwrap())).unwrap();
    let (positive, proof) = derive(&original);
    let (negative, negative_proof) = nonneutral(&positive);
    fs::create_dir_all(&out).unwrap();
    let pos = out.join("derived-group238.xml");
    let neg = out.join("derived-support-count-two.xml");
    fs::write(&pos, &positive).unwrap();
    fs::write(&neg, &negative).unwrap();
    write(out.join("derivation.json"), &proof);
    write(out.join("negative-derivation.json"), &negative_proof);
    let old_dir = out.join("positive-prior");
    let new_dir = out.join("positive-next");
    release::normalize(&before, &pos, 5, &old_dir);
    release::normalize(&after, &pos, 5, &new_dir);
    let old = checked_output(&old_dir, &before, 5, &prior, &positive);
    let new = checked_output(&new_dir, &after, 5, &next, &positive);
    let positive_result = compare_positive(&old, &new, &positive);
    let old_final = finalize(
        &positive,
        &old_dir,
        &before,
        &out.join("positive-prior-selection.json"),
    );
    let new_final = finalize(
        &positive,
        &new_dir,
        &after,
        &out.join("positive-next-selection.json"),
    );
    let mut old_issues = old_final["finalization"]["issues"].clone();
    let mut new_issues = new_final["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    let rows = old_issues.as_array_mut().unwrap();
    assert_eq!(
        rows.iter()
            .filter(|r| r["code"] == "usage-preferences-not-converted")
            .count(),
        1
    );
    rows.retain(|r| r["code"] != "usage-preferences-not-converted");
    assert_eq!(old_issues, new_issues);
    assert!(
        !new_issues.as_array().unwrap().is_empty(),
        "no whole-build admission claimed"
    );
    let old_negative = out.join("negative-prior");
    let new_negative = out.join("negative-next");
    release::normalize(&before, &neg, 5, &old_negative);
    release::normalize(&after, &neg, 5, &new_negative);
    let old = checked_output(&old_negative, &before, 5, &prior, &negative);
    let new = checked_output(&new_negative, &after, 5, &next, &negative);
    actual_records(&old, &negative, Some(1));
    actual_records(&new, &negative, Some(1));
    assert_eq!(usage(&old.draft)["completion"]["kind"], "pending");
    assert_eq!(
        usage(&new.draft)["completion"],
        usage(&old.draft)["completion"]
    );
    assert_eq!(old.draft, new.draft);
    assert_eq!(old.sidecar, new.sidecar);
    assert_eq!(old.report, new.report);
    assert_eq!(fs::read(&pos).unwrap(), positive);
    assert_eq!(fs::read(&neg).unwrap(), negative);
    assert_eq!(before_files, release::inventory(&before));
    assert_eq!(after_files, release::inventory(&after));
    for p in pins["originals"].as_array().unwrap() {
        verify_pin(p)
    }
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"positive":positive_result,"negative":{"support_count":2,"usage":"pending","retired_issues":0,"exact_whole_output_inverse":true},"original_five_unchanged":true,"packages_unchanged":true,"complete_original_builds":0,"derived_build_finalization":"pending","calculation":"not_run","default_limits":true}),
    );
}
