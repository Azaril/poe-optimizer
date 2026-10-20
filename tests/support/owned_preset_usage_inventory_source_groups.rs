//! Shared test-only exact source-group frames. No build data enters production.
use crate::{family::sha, primary_counts::instance};
use poe_optimizer_import::owned_source::{SourceEvidenceLimits, SourceProjectEvidence};
use serde_json::{Value, json};
use std::ops::Range;
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
