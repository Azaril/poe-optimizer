//! Reviewed finite negative effect correspondence for one existing count recipe.
//! This test-only authoring seam never broadens the physical source grammar.
use super::{data, read, root, sha};
use poe_optimizer_import::{
    owned_normalize::{
        NormalizationPolicy, PrimarySkillUsageInput, UsageFallbackAdmission, UsageGroupCompanion,
        UsageInputPolicy, UsageValueSource,
    },
    owned_release::StagedOwnedRelease,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, io::Read};

fn list(row: &mut PrimarySkillUsageInput) -> &mut Vec<UsageGroupCompanion> {
    let policies: Vec<_> = row
        .policies
        .iter_mut()
        .filter(|p| serde_json::to_value(&p.policy).unwrap()["key"] == "def.000000000000326a")
        .collect();
    // Use the sealed typed parameter identity below, rather than a display name.
    assert_eq!(policies.len(), 1);
    let policy = policies.into_iter().next().unwrap();
    assert_eq!(policy.parameters.len(), 1);
    assert_eq!(
        serde_json::to_value(&policy.parameters[0].slot).unwrap()["slot"]["key"],
        "def.000000000000326b"
    );
    let UsageValueSource::ContainingGroupOverride {
        fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
        ..
    } = &mut policy.parameters[0].source
    else {
        panic!("existing presence-based count recipe and exact sibling proof")
    };
    companions
}

fn singleton(role: &Value) {
    assert_eq!(role["materialization"], json!({"kind":"physical"}));
    assert_eq!(
        role["role"],
        json!({"kind":"known","value":"support_assignment"})
    );
    assert_eq!(role["primary"]["kind"], "known");
    assert_eq!(role["construction"]["kind"], "observed");
    let construction = &role["construction"]["value"];
    assert_eq!(construction["primary"]["resolution"]["kind"], "mapped");
    assert_eq!(
        construction["primary"]["resolution"]["value"],
        role["primary"]["value"]
    );
    assert_eq!(
        construction["resolved_effects"],
        json!([construction["primary"]])
    );
    assert!(construction["display_order"].is_null());
    for field in [
        "declared_additional_effects",
        "constructed_additional_effects",
        "resolved_additional_effects",
        "additional_stat_sets",
        "missing_references",
    ] {
        assert_eq!(construction[field], json!([]), "complete {field}");
    }
}

pub(super) fn check() {
    check_packet(&read(data().join("companion-additions.json")));
}

pub(super) fn check_packet(packet: &Value) {
    assert_eq!(packet["before"], super::BEFORE);
    assert_eq!(packet["scope"]["existing_companions"], 5);
    assert_eq!(packet["scope"]["added_companions"], 9);
    assert_eq!(packet["scope"]["count_recipe_only"], true);
    assert_eq!(packet["scope"]["source_group_frame_unchanged"], true);
    for field in [
        "native_changes",
        "eligibility_proven",
        "whole_build_complete",
    ] {
        assert_eq!(packet["scope"][field], false);
    }
    let mut prior: PrimarySkillUsageInput =
        serde_json::from_value(packet["prior_physical"].clone()).unwrap();
    assert_eq!(list(&mut prior).len(), 5);
    let target = &packet["primary_role"];
    assert_eq!(target["gem"], packet["prior_physical"]["gem"]);
    assert_eq!(target["role"], json!({"kind":"known","value":"skill_use"}));
    assert_eq!(
        target["primary"]["value"],
        packet["prior_physical"]["primary"]
    );
    assert_eq!(
        target["construction"]["value"]["primary"]["effect_id"],
        packet["prior_physical"]["skill_id"]
    );
    let evidence = read(data().join("evidence.json"));
    let families = evidence["support_family_expansion"]["families"]
        .as_array()
        .unwrap();
    let rows = packet["additions"].as_array().unwrap();
    assert_eq!(rows.len(), 9);
    assert_eq!(families.len(), rows.len());
    let mut keys = BTreeSet::new();
    let mut tuples = BTreeSet::new();
    for row in rows {
        let _: UsageGroupCompanion = serde_json::from_value(row["companion"].clone()).unwrap();
        let key = row["companion"]["gem"]["key"].as_str().unwrap();
        assert!(keys.insert(key));
        let matches: Vec<_> = families.iter().filter(|f| f["gem"]["key"] == key).collect();
        assert_eq!(matches.len(), 1);
        let family = matches[0];
        let role = &row["role"];
        singleton(role);
        assert_eq!(role["gem"], row["companion"]["gem"]);
        assert_eq!(role["construction"]["value"], family["construction"]);
        assert_ne!(role["primary"]["value"], target["primary"]["value"]);
        assert_ne!(
            role["construction"]["value"]["primary"]["effect_id"],
            packet["prior_physical"]["skill_id"]
        );
        for field in ["gem", "game_id", "variant_id", "skill_id", "name_spec"] {
            assert_eq!(row["companion"][field], family["physical_rule"][field]);
        }
        let construction = &role["construction"]["value"];
        assert_eq!(row["companion"]["game_id"], construction["game_id"]);
        assert_eq!(row["companion"]["variant_id"], construction["variant_id"]);
        assert_eq!(
            row["companion"]["skill_id"],
            construction["primary"]["effect_id"]
        );
        assert!(tuples.insert((
            row["companion"]["game_id"].as_str().unwrap(),
            row["companion"]["variant_id"].as_str().unwrap()
        )));
        assert!(
            !list(&mut prior)
                .iter()
                .any(|c| serde_json::to_value(c).unwrap()["gem"]["key"] == key)
        );
    }
    let unknown = &packet["unknown_sibling_control"];
    assert!(!keys.contains(unknown["role"]["gem"]["key"].as_str().unwrap()));
    assert!(
        !list(&mut prior)
            .iter()
            .any(|c| serde_json::to_value(c).unwrap()["gem"] == unknown["role"]["gem"])
    );
    assert_eq!(unknown["source"]["ordinal"], 199);
    let census = read(data().join("source-census.json"));
    let saved: Vec<_> = census[4]["gem_rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["ordinal"] == unknown["source"]["ordinal"])
        .collect();
    assert_eq!(saved, vec![&unknown["source"]]);

    // Reuse the manifest-bound complete source body. The exact function accepts
    // a unique matching primary/additional effect, not arbitrary known siblings.
    let source = &packet["source"];
    let manifest_bytes =
        fs::read(root().join(source["manifest"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(sha(&manifest_bytes), source["manifest"]["sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let archive = &source["archive"];
    assert!(
        manifest["files"]
            .as_array()
            .unwrap()
            .contains(&archive["source"])
    );
    let compressed = fs::read(root().join(archive["file"].as_str().unwrap())).unwrap();
    assert_eq!(
        compressed.len(),
        archive["compressed_bytes"].as_u64().unwrap() as usize
    );
    assert_eq!(sha(&compressed), archive["compressed_sha256"]);
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(&compressed[..])
        .read_to_end(&mut raw)
        .unwrap();
    assert_eq!(raw.len(), archive["raw_bytes"].as_u64().unwrap() as usize);
    assert_eq!(sha(&raw), archive["source"]["sha256"]);
    let text = std::str::from_utf8(&raw).unwrap();
    assert_eq!(source["span"]["first_line"], 149);
    assert_eq!(source["span"]["last_line"], 173);
    let span = text
        .lines()
        .skip(148)
        .take(25)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(span, source["span"]["text"]);
    assert_eq!(sha(span.as_bytes()), source["span"]["sha256"]);
}

pub(super) fn included(omitted: &[String]) -> Vec<UsageGroupCompanion> {
    let packet = read(data().join("companion-additions.json"));
    let allowed: BTreeSet<_> = packet["additions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["companion"]["gem"]["key"].as_str().unwrap().to_owned())
        .collect();
    let selected: BTreeSet<_> = omitted.iter().cloned().collect();
    assert_eq!(selected.len(), omitted.len());
    assert!(selected.is_subset(&allowed));
    packet["additions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| !selected.contains(r["companion"]["gem"]["key"].as_str().unwrap()))
        .map(|r| serde_json::from_value(r["companion"].clone()).unwrap())
        .collect()
}

pub(super) fn authenticate(prior: &StagedOwnedRelease) {
    let packet = read(data().join("companion-additions.json"));
    let roles = serde_json::to_value(&prior.input().roles).unwrap();
    let mut expected = vec![
        &packet["primary_role"],
        &packet["unknown_sibling_control"]["role"],
    ];
    expected.extend(
        packet["additions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| &r["role"]),
    );
    for role in expected {
        let actual: Vec<_> = roles["roles"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["gem"] == role["gem"])
            .collect();
        assert_eq!(
            actual,
            vec![role],
            "exact current compiler-owned construction facts"
        );
    }
}

pub(super) fn apply(
    policy: &mut NormalizationPolicy,
    additions: &[UsageGroupCompanion],
    inverse: bool,
) {
    let packet = read(data().join("companion-additions.json"));
    let before: PrimarySkillUsageInput =
        serde_json::from_value(packet["prior_physical"].clone()).unwrap();
    let mut after = before.clone();
    list(&mut after).extend_from_slice(additions);
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. }) = &mut policy.usage_inputs
    else {
        panic!("current occurrence usage policy")
    };
    let actual: Vec<_> = physical
        .iter_mut()
        .filter(|r| r.gem == before.gem)
        .collect();
    assert_eq!(actual.len(), 1);
    let actual = actual.into_iter().next().unwrap();
    assert_eq!(&*actual, if inverse { &after } else { &before });
    *actual = if inverse { before } else { after };
}
