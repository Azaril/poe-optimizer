//! Exact raw-source group frames exercise current data and production normalization.
use super::{COUNT, InventoryControl, assert_known_count, instance, target, transitions};
use crate::{family, positive, preservation, release, selected};
use family::{Output, checked_output, data, read, root, sha, write};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

struct Frame {
    set: usize,
    group: usize,
    gems: Vec<usize>,
    group_range: Range<usize>,
}
fn frame(xml: &[u8], t: &Value) -> Frame {
    let imported = instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let skills: Vec<_> = rows
        .iter()
        .filter(|r| r.occurrence().name() == "Skills")
        .collect();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].children().len(), 1);
    let set = &rows[skills[0].children()[0].ordinal() as usize];
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
    let group = &rows[set.children()[0].ordinal() as usize];
    assert_eq!(group.occurrence().name(), "Skill");
    assert!(!group.children().is_empty());
    let gems: Vec<_> = group
        .children()
        .iter()
        .map(|id| {
            let row = &rows[id.ordinal() as usize];
            assert_eq!(row.occurrence().name(), "Gem");
            assert!(row.children().is_empty());
            id.ordinal() as usize
        })
        .collect();
    assert_eq!(
        rows[gems[0]]
            .attribute("skillId")
            .unwrap()
            .decoded()
            .unwrap(),
        t["physical_after"]["skill_id"].as_str().unwrap()
    );
    Frame {
        set: set.occurrence().id().ordinal() as usize,
        group: group.occurrence().id().ordinal() as usize,
        gems,
        group_range: group.occurrence().range(),
    }
}
fn derive(xml: &[u8], ordinal: usize, t: &Value) -> (Vec<u8>, Value) {
    let imported = instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let group = &rows[ordinal];
    assert_eq!(group.occurrence().name(), "Skill");
    let primary = &rows[group.children()[0].ordinal() as usize];
    assert_eq!(
        primary.attribute("skillId").unwrap().decoded().unwrap(),
        t["physical_after"]["skill_id"].as_str().unwrap()
    );
    let set = &rows[group.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(set.occurrence().name(), "SkillSet");
    assert_eq!(set.attribute("id").unwrap().decoded().unwrap(), "4");
    let skills = &rows[set.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(skills.occurrence().name(), "Skills");
    assert_eq!(
        skills
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap(),
        "4"
    );
    let gr = group.occurrence().range();
    let sr = set.occurrence().range();
    let sk = skills.occurrence().range();
    let first_set = rows[skills.children()[0].ordinal() as usize]
        .occurrence()
        .range();
    let last_set = rows[skills.children().last().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let first_group = rows[set.children()[0].ordinal() as usize]
        .occurrence()
        .range();
    let last_group = rows[set.children().last().unwrap().ordinal() as usize]
        .occurrence()
        .range();
    let segments = [
        0..sk.start,
        sk.start..first_set.start,
        sr.start..first_group.start,
        gr.clone(),
        last_group.end..sr.end,
        last_set.end..sk.end,
        sk.end..xml.len(),
    ];
    assert!(segments.windows(2).all(|p| p[0].end <= p[1].start));
    let mut derived = Vec::new();
    let mut proof = Vec::new();
    for range in segments {
        let begin = derived.len();
        derived.extend_from_slice(&xml[range.clone()]);
        proof.push(json!({"original":[range.start,range.end],"derived":[begin,derived.len()],"sha256":sha(&xml[range])}));
    }
    let current = frame(&derived, t);
    assert_eq!(&derived[current.group_range.clone()], &xml[gr.clone()]);
    assert_eq!(&derived[..sk.start], &xml[..sk.start]);
    assert_eq!(
        &derived[derived.len() - (xml.len() - sk.end)..],
        &xml[sk.end..]
    );
    (
        derived.clone(),
        json!({"original_sha256":sha(xml),"derived_sha256":sha(&derived),"original_group":ordinal,"original_group_sha256":sha(&xml[gr]),"derived_group":current.group,"derived_primary":current.gems[0],"segments":proof,"group_bytes_exact":true,"outside_skills_exact":true,"test_frame_only":true}),
    )
}
fn splice(xml: &[u8], range: Range<usize>, replacement: &[u8]) -> Vec<u8> {
    assert!(range.start <= range.end && range.end <= xml.len());
    let mut out = Vec::with_capacity(xml.len() - range.len() + replacement.len());
    out.extend_from_slice(&xml[..range.start]);
    out.extend_from_slice(replacement);
    out.extend_from_slice(&xml[range.end..]);
    assert_eq!(&out[..range.start], &xml[..range.start]);
    assert_eq!(&out[range.start + replacement.len()..], &xml[range.end..]);
    out
}
fn attribute(
    xml: &[u8],
    t: &Value,
    group: bool,
    name: &str,
    value: Option<&str>,
) -> (Vec<u8>, Value) {
    let f = frame(xml, t);
    let imported = instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let ordinal = if group { f.group } else { f.gems[0] };
    let row = &evidence.rows()[ordinal];
    let (range, bytes) = if let Some(a) = row.attribute(name) {
        let r = a.range();
        if let Some(v) = value {
            (r, v.as_bytes().to_vec())
        } else {
            let r = r.start.checked_sub(name.len() + 2).unwrap()..r.end + 1;
            assert_eq!(
                &xml[r.clone()],
                format!("{name}=\"{}\"", a.raw()).as_bytes()
            );
            (r, vec![])
        }
    } else {
        let value = value.expect("new control attribute has a value");
        let row_range = row.occurrence().range();
        let header_end = xml[row_range.clone()]
            .iter()
            .position(|b| *b == b'>')
            .unwrap()
            + row_range.start;
        assert_ne!(xml[header_end - 1], b'/');
        (
            header_end..header_end,
            format!(" {name}=\"{value}\"").into_bytes(),
        )
    };
    let out = splice(xml, range.clone(), &bytes);
    let after = frame(&out, t);
    assert_eq!(after.gems, f.gems);
    let proof = json!({"source_ordinal":ordinal,"attribute":name,"group_attribute":group,"new":value,"original_range":[range.start,range.end],"replacement":String::from_utf8(bytes).unwrap(),"before":sha(xml),"after":sha(&out),"other_bytes_exact":true});
    (out, proof)
}
fn unknown_companion(xml: &[u8], original: &[u8], t: &Value) -> (Vec<u8>, Value) {
    let imported = instance(original);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let row = &evidence.rows()[199];
    assert_eq!(row.occurrence().name(), "Gem");
    let id = row.attribute("gemId").unwrap().decoded().unwrap();
    assert!(
        t["companions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["companion"]["game_id"] != id)
    );
    let raw = row.occurrence().range();
    let current = instance(xml);
    let e = SourceProjectEvidence::collect(&current, SourceEvidenceLimits::default()).unwrap();
    let f = frame(xml, t);
    let last = &e.rows()[*f.gems.last().unwrap()];
    let at = last.occurrence().range().end;
    let mut addition = b"\n".to_vec();
    addition.extend_from_slice(&original[raw.clone()]);
    let changed = splice(xml, at..at, &addition);
    assert_eq!(frame(&changed, t).gems.len(), f.gems.len() + 1);
    (
        changed.clone(),
        json!({"kind":"unknown-exact-companion","source_ordinal":199,"source_range":[raw.start,raw.end],"raw_sha256":sha(&original[raw]),"inserted_at":at,"after":sha(&changed),"source_gem_bytes_exact":true,"eligibility_claimed":false}),
    )
}
fn usage(output: &Output) -> &Value {
    positive::usage(&output.draft)
}
fn count<'a>(output: &'a Output, xml: &[u8], t: &Value) -> Option<&'a Value> {
    let f = frame(xml, t);
    let expected = target(output, f.gems[0], t);
    let matches: Vec<_> = usage(output)["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["selection"]["policy"]["value"]["key"] == COUNT
                && r["selection"]["target"] == expected
        })
        .collect();
    assert!(matches.len() <= 1);
    matches.first().copied()
}
enum Count {
    Known(i64),
    Absent,
    Pending,
}
impl Count {
    fn check(&self, output: &Output, xml: &[u8], t: &Value) {
        let binding = count(output, xml, t);
        match self {
            Self::Known(value) => {
                assert_known_count(binding.expect("one exact count binding"), *value)
            }
            Self::Absent => assert!(binding.is_none(), "finite source guard refuses projection"),
            Self::Pending => {
                let p = &binding.expect("attempted count consumer")["selection"]["parameters"];
                assert_eq!(p["completion"]["kind"], "pending");
                assert_eq!(p["completion"]["code"], "usage-parameters-not-converted");
                assert_eq!(
                    p["members"],
                    json!([]),
                    "invalid or unmatched count never defaults"
                );
            }
        }
    }
    fn report(&self) -> Value {
        match self {
            Self::Known(v) => json!({"known":v}),
            Self::Absent => json!({"absent":true}),
            Self::Pending => json!({"pending":true}),
        }
    }
}
fn actual_positive(output: &Output, xml: &[u8], t: &Value, expected: i64) {
    let f = frame(xml, t);
    assert_eq!(
        output.draft["draft"]["gems"]["members"]
            .as_array()
            .unwrap()
            .len(),
        f.gems.len()
    );
    assert_eq!(
        output.draft["draft"]["skills"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        output.draft["draft"]["supports"]["members"]
            .as_array()
            .unwrap()
            .len()
            + 1,
        f.gems.len()
    );
    let skill = preservation::link(&output.sidecar, f.gems[0], "skill");
    let preset = preservation::link(&output.sidecar, f.set, "skill_preset");
    assert_eq!(
        output.draft["draft"]["skill_presets"]["members"][0]["id"],
        preset
    );
    for (i, ordinal) in f.gems.iter().copied().enumerate() {
        let gem = preservation::link(&output.sidecar, ordinal, "gem");
        let record = preservation::member(&output.draft, "gems", &gem);
        assert_eq!(record["parameters"]["completion"]["kind"], "complete");
        if i > 0 {
            let support = preservation::link(&output.sidecar, ordinal, "support");
            let assignment = preservation::member(&output.draft, "supports", &support);
            assert_eq!(assignment["support"], json!({"kind":"known","value":gem}));
            assert_eq!(
                assignment["target"],
                json!({"kind":"authored","value":{"kind":"known","value":skill}})
            );
        }
    }
    let policies: Vec<_> = usage(output)["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            assert_eq!(r["selection"]["target"], target(output, f.gems[0], t));
            assert_eq!(
                r["selection"]["parameters"]["completion"]["kind"],
                "complete"
            );
            r["selection"]["policy"]["value"]["key"].as_str().unwrap()
        })
        .collect();
    assert_eq!(
        policies,
        t["physical_after"]["policies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["policy"]["key"].as_str().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(policies.len(), 4);
    Count::Known(expected).check(output, xml, t);
}
fn publish_control(
    prior: &StagedOwnedRelease,
    out: &Path,
    control: InventoryControl,
) -> (PathBuf, StagedOwnedRelease) {
    let staged = family::stage_primary_inventory_control(prior, control);
    fs::create_dir_all(out).unwrap();
    let input = out.join("endpoint.json");
    write(&input, staged.input());
    let package = out.join("package");
    assert_eq!(family::publish(&input, &package), json!(staged.receipt()));
    (package, staged)
}
struct Context<'a> {
    counter_path: &'a Path,
    counter: &'a StagedOwnedRelease,
    next_path: &'a Path,
    next: &'a StagedOwnedRelease,
    out: &'a Path,
}
impl Context<'_> {
    fn execute(&self, name: &str, xml: &[u8]) -> (Output, Output, PathBuf) {
        let directory = self.out.join(name);
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.xml");
        fs::write(&source, xml).unwrap();
        let old = directory.join("counterfactual");
        let new = directory.join("reviewed");
        let repeated = directory.join("repeated");
        release::normalize(self.counter_path, &source, 5, &old);
        release::normalize(self.next_path, &source, 5, &new);
        release::normalize(self.next_path, &source, 5, &repeated);
        let before = checked_output(&old, self.counter_path, 5, self.counter, xml);
        let after = checked_output(&new, self.next_path, 5, self.next, xml);
        assert_eq!(
            after,
            checked_output(&repeated, self.next_path, 5, self.next, xml),
            "independent fresh normalization"
        );
        assert_eq!(fs::read(&source).unwrap(), xml);
        (before, after, directory)
    }
    fn negative(&self, name: &str, xml: &[u8], t: &Value, count: Count, proof: &Value) -> Value {
        let (before, after, directory) = self.execute(name, xml);
        count.check(&before, xml, t);
        count.check(&after, xml, t);
        assert_eq!(usage(&before)["completion"]["kind"], "pending");
        assert_eq!(usage(&after)["completion"]["kind"], "pending");
        assert_eq!(
            before, after,
            "counterfactual and reviewed inventory cannot change an unproved source"
        );
        write(directory.join("derivation.json"), proof);
        json!({"name":name,"count":count.report(),"usage":"pending","whole_checked_output_equal":true,"fresh_repeat":true})
    }
}
pub fn run() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PRIMARY_COUNTS_PRIOR").unwrap());
    let next_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PRIMARY_COUNTS_NEXT").unwrap());
    let out = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PRIMARY_COUNTS_OUTPUT").unwrap());
    assert!(!out.exists());
    family::checked_predecessor(&prior_path);
    family::check();
    let prior_files = release::inventory(&prior_path);
    let next_files = release::inventory(&next_path);
    let prior = release::load(&prior_path);
    let next = release::load(&next_path);
    family::checked_successor(&prior, &next);
    fs::create_dir_all(&out).unwrap();
    let (counter_path, counter) = publish_control(
        &prior,
        &out.join("deferred-inventory"),
        InventoryControl::Deferred,
    );
    let census = read(data().join("source-census.json"));
    let source = root().join(census[4]["path"].as_str().unwrap());
    let original = fs::read(&source).unwrap();
    assert_eq!(json!(sha(&original)), census[4]["sha256"]);
    let context = Context {
        counter_path: &counter_path,
        counter: &counter,
        next_path: &next_path,
        next: &next,
        out: &out,
    };
    let mut results = Vec::new();
    for (index, t) in transitions().iter().enumerate() {
        let ordinal = [220, 233][index];
        let (xml, proof) = derive(&original, ordinal, t);
        let (before, after, dir) = context.execute(&format!("group-{ordinal}-count-one"), &xml);
        actual_positive(&before, &xml, t, 1);
        actual_positive(&after, &xml, t, 1);
        let f = frame(&xml, t);
        // Flat-primary input completion leaves its pending usage issue on the
        // Gem occurrence. The already converted group has no such issue link.
        let retirement = positive::compare_usage_retirement(&before, &after, &[f.gems[0]]);
        let finalization = selected::finalize_with_definitions(
            &xml,
            &dir.join("reviewed"),
            &dir.join("selection.json"),
            &next_path.join("schema.json"),
        );
        assert!(
            !finalization["finalization"]["issues"]
                .as_array()
                .unwrap()
                .is_empty(),
            "input usage completion is not whole-build legality"
        );
        write(dir.join("derivation.json"), &proof);
        results.push(json!({"name":format!("group-{ordinal}-count-one"),"retirement":retirement,"native_execution":"not_run"}));
        if index == 0 {
            let (three, p) = attribute(&xml, t, false, "count", Some("3"));
            let (a, b, d) = context.execute("offering-count-three", &three);
            actual_positive(&a, &three, t, 3);
            actual_positive(&b, &three, t, 3);
            let f = frame(&three, t);
            results.push(json!({"name":"offering-count-three","retirement":positive::compare_usage_retirement(&a,&b,&[f.gems[0]])}));
            write(d.join("derivation.json"), &p);
        }
        for (label, value) in [
            ("bad", Some("bad")),
            ("guard-two", Some("2")),
            ("out-of-domain", Some("5")),
            ("negative", Some("-1")),
            ("missing", None),
        ] {
            let (changed, p) = attribute(&xml, t, false, "count", value);
            results.push(context.negative(
                &format!("group-{ordinal}-count-{label}"),
                &changed,
                t,
                Count::Absent,
                &p,
            ));
        }
        for (label, value, expected) in [
            ("zero", "0", Count::Known(0)),
            ("four", "4", Count::Known(4)),
            ("malformed", "bad", Count::Pending),
            ("outside", "5", Count::Pending),
        ] {
            let (changed, p) = attribute(&xml, t, true, "groupCount", Some(value));
            results.push(context.negative(
                &format!("group-{ordinal}-override-{label}"),
                &changed,
                t,
                expected,
                &p,
            ));
        }
        let (changed, p) = attribute(&xml, t, false, "enableGlobal1", Some("false"));
        results.push(context.negative(
            &format!("group-{ordinal}-primary-disabled"),
            &changed,
            t,
            Count::Known(1),
            &p,
        ));
        let (changed, p) = unknown_companion(&xml, &original, t);
        results.push(context.negative(
            &format!("group-{ordinal}-unknown-companion"),
            &changed,
            t,
            Count::Pending,
            &p,
        ));
        for (label, control) in [
            ("missing-group-lane", InventoryControl::MissingGroupCount),
            (
                "missing-count-lane",
                InventoryControl::MissingOccurrenceCount,
            ),
        ] {
            let (path, staged) = publish_control(
                &prior,
                &out.join(format!("package-{ordinal}-{label}")),
                control,
            );
            let control_context = Context {
                counter_path: &counter_path,
                counter: &counter,
                next_path: &path,
                next: &staged,
                out: &out,
            };
            results.push(control_context.negative(
                &format!("group-{ordinal}-{label}"),
                &xml,
                t,
                Count::Known(1),
                &proof,
            ));
        }
    }
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(prior_files, release::inventory(&prior_path));
    assert_eq!(next_files, release::inventory(&next_path));
    for row in census.as_array().unwrap() {
        let bytes = fs::read(root().join(row["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&bytes)), row["sha256"]);
        assert_eq!(json!(bytes.len()), row["bytes"]);
    }
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"results":results,
        "original_five_unchanged":true,"source_guards_unchanged":true,"group_parent_grammar_unchanged":true,"gameplay_maximum_claimed":false,
        "default_limits":true,"complete_original_builds":0,"native_execution":"not_run"}),
    );
}
