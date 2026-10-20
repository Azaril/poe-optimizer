//! Shared test-only exact source-group frames. No build data enters production.
use crate::{
    family::{Output, checked_output, sha, write},
    positive,
    primary_counts::instance,
    release,
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use poe_optimizer_import::owned_source::{SourceEvidenceLimits, SourceProjectEvidence};
use serde_json::{Value, json};
use std::ops::Range;
use std::{
    fs,
    path::{Path, PathBuf},
};
pub struct Frame {
    pub set: usize,
    pub group: usize,
    pub gems: Vec<usize>,
    pub group_range: Range<usize>,
}
pub fn frame(xml: &[u8], expected_skill: &str) -> Frame {
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
        expected_skill
    );
    Frame {
        set: set.occurrence().id().ordinal() as usize,
        group: group.occurrence().id().ordinal() as usize,
        gems,
        group_range: group.occurrence().range(),
    }
}
pub fn derive(xml: &[u8], ordinal: usize, expected_skill: &str) -> (Vec<u8>, Value) {
    let imported = instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let group = &rows[ordinal];
    assert_eq!(group.occurrence().name(), "Skill");
    let primary = &rows[group.children()[0].ordinal() as usize];
    assert_eq!(
        primary.attribute("skillId").unwrap().decoded().unwrap(),
        expected_skill
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
    let current = frame(&derived, expected_skill);
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
pub fn splice(xml: &[u8], range: Range<usize>, replacement: &[u8]) -> Vec<u8> {
    assert!(range.start <= range.end && range.end <= xml.len());
    let mut out = Vec::with_capacity(xml.len() - range.len() + replacement.len());
    out.extend_from_slice(&xml[..range.start]);
    out.extend_from_slice(replacement);
    out.extend_from_slice(&xml[range.end..]);
    assert_eq!(&out[..range.start], &xml[..range.start]);
    assert_eq!(&out[range.start + replacement.len()..], &xml[range.end..]);
    out
}

pub fn attribute(
    xml: &[u8],
    expected_skill: &str,
    group: bool,
    name: &str,
    value: Option<&str>,
) -> (Vec<u8>, Value) {
    let frame = frame(xml, expected_skill);
    let imported = positive::instance(xml);
    let e = SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let ordinal = if group { frame.group } else { frame.gems[0] };
    let row = &e.rows()[ordinal];
    let (range, bytes) = if let Some(a) = row.attribute(name) {
        let range = a.range();
        if let Some(v) = value {
            (range, v.as_bytes().to_vec())
        } else {
            let range = range.start.checked_sub(name.len() + 2).unwrap()..range.end + 1;
            assert_eq!(
                &xml[range.clone()],
                format!("{name}=\"{}\"", a.raw()).as_bytes()
            );
            (range, vec![])
        }
    } else {
        let value = value.unwrap();
        let rr = row.occurrence().range();
        let end = rr.start + xml[rr].iter().position(|b| *b == b'>').unwrap();
        let insert = if xml[end - 1] == b'/' { end - 1 } else { end };
        (insert..insert, format!(" {name}=\"{value}\"").into_bytes())
    };
    let changed = splice(xml, range.clone(), &bytes);
    assert_eq!(self::frame(&changed, expected_skill).gems, frame.gems);
    let proof = json!({"source_ordinal":ordinal,"attribute":name,"group_attribute":group,"new":value,"original_range":[range.start,range.end],"replacement":String::from_utf8(bytes).unwrap(),"before":sha(xml),"after":sha(&changed),"other_bytes_exact":true});
    (changed, proof)
}

fn usage(output: &Output) -> &Value {
    positive::usage(&output.draft)
}
pub struct Context<'a> {
    pub before_path: &'a Path,
    pub before: &'a StagedOwnedRelease,
    pub next_path: &'a Path,
    pub next: &'a StagedOwnedRelease,
    pub out: &'a Path,
}
impl Context<'_> {
    pub fn execute(&self, name: &str, xml: &[u8]) -> (Output, Output, PathBuf) {
        let directory = self.out.join(name);
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.xml");
        fs::write(&source, xml).unwrap();
        let old = directory.join("counterfactual");
        let new = directory.join("reviewed");
        let repeated = directory.join("repeated");
        release::normalize(self.before_path, &source, 5, &old);
        release::normalize(self.next_path, &source, 5, &new);
        release::normalize(self.next_path, &source, 5, &repeated);
        let before = checked_output(&old, self.before_path, 5, self.before, xml);
        let after = checked_output(&new, self.next_path, 5, self.next, xml);
        assert_eq!(
            after,
            checked_output(&repeated, self.next_path, 5, self.next, xml),
            "independent fresh normalization"
        );
        assert_eq!(fs::read(&source).unwrap(), xml);
        (before, after, directory)
    }
    pub fn negative(&self, name: &str, xml: &[u8], proof: &Value) -> Value {
        let (before, after, directory) = self.execute(name, xml);
        assert_eq!(usage(&before)["completion"]["kind"], "pending");
        assert_eq!(usage(&after)["completion"]["kind"], "pending");
        assert_eq!(
            before, after,
            "unproved input retains the entire draft, sidecar, diagnostics and allocator"
        );
        write(directory.join("derivation.json"), proof);
        json!({"name":name,"usage":"pending","whole_checked_output_equal":true,"fresh_repeat":true})
    }
    pub fn source_refusal(&self, name: &str, xml: &[u8], proof: &Value) -> Value {
        let (before, after, directory) = self.execute(name, xml);
        assert_eq!(
            before, after,
            "unknown sources gain no authority from inventory declarations"
        );
        for collection in ["gems", "skills", "supports"] {
            assert_eq!(after.draft["draft"][collection]["members"], json!([]));
        }
        assert_eq!(
            usage(&after),
            &json!({"members":[],"completion":{"kind":"complete"}})
        );
        let generated =
            &after.draft["draft"]["skill_presets"]["members"][0]["intent"]["generated_inputs"];
        assert_eq!(generated["members"], json!([]));
        assert_eq!(generated["completion"]["kind"], "pending");
        assert_eq!(
            generated["completion"]["code"],
            "generated-skill-inputs-not-converted"
        );
        write(directory.join("derivation.json"), proof);
        json!({"name":name,"physical_materialization":"none","usage":"complete_empty_no_target","generated_source_inventory":"pending","whole_checked_output_equal":true,"fresh_repeat":true})
    }
}
