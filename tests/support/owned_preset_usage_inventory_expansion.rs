//! Real-byte import accounting controls; no support eligibility or gameplay claim.
use crate::{family, positive, preservation, release, selected};
use family::{Output, checked_output, data, read, root, sha, write};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};

struct Frame {
    set: usize,
    group: usize,
    gems: Vec<usize>,
}

fn frame(xml: &[u8], count: usize) -> Frame {
    let instance = positive::instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let skills: Vec<_> = rows
        .iter()
        .filter(|r| r.occurrence().name() == "Skills")
        .collect();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].children().len(), 1);
    let set = &rows[skills[0].children()[0].ordinal() as usize];
    assert_eq!(set.occurrence().name(), "SkillSet");
    assert_eq!(set.attribute("id").unwrap().decoded().unwrap(), "4");
    assert_eq!(
        skills[0]
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap(),
        "4"
    );
    assert_eq!(set.children().len(), 1);
    let group = &rows[set.children()[0].ordinal() as usize];
    assert_eq!(group.occurrence().name(), "Skill");
    assert_eq!(group.children().len(), count);
    let gems: Vec<_> = group
        .children()
        .iter()
        .map(|s| s.ordinal() as usize)
        .collect();
    for &ordinal in &gems {
        assert_eq!(rows[ordinal].occurrence().name(), "Gem");
        assert!(rows[ordinal].children().is_empty());
    }
    Frame {
        set: set.occurrence().id().ordinal() as usize,
        group: group.occurrence().id().ordinal() as usize,
        gems,
    }
}

fn expanded_source(original: &[u8]) -> (Vec<u8>, Vec<String>, Value) {
    // This derives the original four-row frame through the existing raw-range
    // proof; the new operation does not reserialize any copied source element.
    let (base, base_proof) = positive::derive(original);
    let base_frame = frame(&base, 4);
    let source_instance = positive::instance(original);
    let source_evidence =
        SourceProjectEvidence::collect(&source_instance, SourceEvidenceLimits::default()).unwrap();
    let source_rows = source_evidence.rows();
    let packet = read(data().join("evidence.json"));
    let families = packet["support_family_expansion"]["families"]
        .as_array()
        .unwrap();
    assert_eq!(families.len(), 9);
    let mut copies = Vec::new();
    let mut family_keys = BTreeSet::new();
    for family in families {
        let key = family["gem"]["key"].as_str().unwrap().to_owned();
        assert!(family_keys.insert(key.clone()));
        for saved in family["selected_original05_sources"].as_array().unwrap() {
            let ordinal = saved["ordinal"].as_u64().unwrap() as usize;
            copies.push((ordinal, key.clone(), family, saved));
        }
    }
    copies.sort_by_key(|r| r.0);
    assert_eq!(
        copies.iter().map(|r| r.0).collect::<Vec<_>>(),
        [
            207, 214, 217, 218, 219, 222, 225, 230, 231, 232, 235, 236, 237
        ]
    );
    assert_eq!(copies.len(), 13);
    let base_instance = positive::instance(&base);
    let base_evidence =
        SourceProjectEvidence::collect(&base_instance, SourceEvidenceLimits::default()).unwrap();
    let base_rows = base_evidence.rows();
    let group = &base_rows[base_frame.group];
    assert!(group.attribute("groupCount").is_none());
    let after_last = base_rows[*base_frame.gems.last().unwrap()]
        .occurrence()
        .range()
        .end;
    let mut derived = Vec::new();
    derived.extend_from_slice(&base[..after_last]);
    let copied_start = derived.len();
    let mut expected = vec![
        "def.00000000000007dd".to_owned(),
        "def.0000000000000740".to_owned(),
        "def.000000000000082b".to_owned(),
        "def.00000000000008bf".to_owned(),
    ];
    let mut segments = Vec::new();
    for (ordinal, key, family, saved) in &copies {
        let row = &source_rows[*ordinal];
        assert_eq!(row.occurrence().name(), "Gem");
        assert_eq!(
            json!(row.occurrence().parent().unwrap().ordinal()),
            saved["group"]
        );
        assert!(row.children().is_empty());
        assert_eq!(
            row.attributes().len(),
            saved["attributes"].as_array().unwrap().len()
        );
        for attribute in saved["attributes"].as_array().unwrap() {
            let actual = row.attribute(attribute["name"].as_str().unwrap()).unwrap();
            assert_eq!(
                actual.decoded().unwrap(),
                attribute["decoded"].as_str().unwrap()
            );
        }
        assert_eq!(
            row.attribute("skillId").unwrap().decoded().unwrap(),
            family["physical_rule"]["skill_id"].as_str().unwrap()
        );
        for (name, value) in [
            ("count", "1"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
        ] {
            let attribute = row.attribute(name).unwrap();
            assert_eq!(attribute.raw(), value);
            assert_eq!(attribute.decoded().unwrap(), value);
        }
        let range = row.occurrence().range();
        let start = derived.len();
        derived.extend_from_slice(&original[range.clone()]);
        segments.push(json!({
            "original_ordinal":ordinal,"original_group":saved["group"],
            "original_range":[range.start,range.end],"derived_range":[start,derived.len()],
            "sha256":sha(&original[range]),"gem":key
        }));
        expected.push(key.clone());
    }
    let copied_end = derived.len();
    derived.extend_from_slice(&base[after_last..]);
    assert_eq!(expected.len(), 17);
    let derived_frame = frame(&derived, 17);
    let derived_instance = positive::instance(&derived);
    let derived_evidence =
        SourceProjectEvidence::collect(&derived_instance, SourceEvidenceLimits::default()).unwrap();
    let derived_rows = derived_evidence.rows();
    assert!(
        derived_rows[derived_frame.group]
            .attribute("groupCount")
            .is_none()
    );
    let actual_group = &derived_rows[derived_frame.group];
    assert_eq!(actual_group.attributes().len(), group.attributes().len());
    for attribute in group.attributes() {
        let actual = actual_group.attribute(&attribute.origin().name).unwrap();
        assert_eq!(actual.raw(), attribute.raw());
        assert_eq!(actual.decoded().unwrap(), attribute.decoded().unwrap());
    }
    for (i, &ordinal) in derived_frame.gems.iter().enumerate() {
        let actual = derived_rows[ordinal].occurrence().range();
        let original_slice = if i < 4 {
            &base[base_rows[base_frame.gems[i]].occurrence().range()]
        } else {
            &original[source_rows[copies[i - 4].0].occurrence().range()]
        };
        assert_eq!(
            &derived[actual], original_slice,
            "every Gem remains byte-identical"
        );
    }
    let mut inverse = derived.clone();
    inverse.drain(copied_start..copied_end);
    assert_eq!(
        inverse, base,
        "only exact copied Gem elements added; group attributes unchanged"
    );
    let proof = json!({
        "kind":"test_only_regrouped_import_accounting","base":base_proof,
        "original_sha256":sha(original),"derived_sha256":sha(&derived),"derived_bytes":derived.len(),
        "group_attributes_unchanged":true,"group_count_inserted":false,
        "copied_elements":segments,"copied_occurrences":13,"distinct_added_families":9,
        "all_gem_bytes_unchanged":true,"outside_skills_unchanged":true,
        "support_eligibility_checked":false,"native_execution":"not_run","production_import_rewrite":false
    });
    (derived, expected, proof)
}

fn actual_records(output: &Output, xml: &[u8], expected: &[String], count_complete: bool) -> Frame {
    let f = frame(xml, 17);
    let draft = &output.draft;
    let sidecar = &output.sidecar;
    assert_eq!(
        draft["draft"]["skills"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        draft["draft"]["gems"]["members"].as_array().unwrap().len(),
        17
    );
    assert_eq!(
        draft["draft"]["supports"]["members"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    let primary = preservation::link(sidecar, f.gems[0], "skill");
    let preset = preservation::link(sidecar, f.set, "skill_preset");
    let preset_record = &draft["draft"]["skill_presets"]["members"][0];
    assert_eq!(preset_record["id"], preset);
    let mut physical = BTreeSet::new();
    let mut assignments = BTreeSet::new();
    for (&ordinal, definition) in f.gems.iter().zip(expected) {
        let gem = preservation::link(sidecar, ordinal, "gem");
        assert!(physical.insert(gem["local"].as_str().unwrap().to_owned()));
        let row = preservation::member(draft, "gems", &gem);
        assert_eq!(row["definition"]["kind"], "known");
        assert_eq!(row["definition"]["value"]["key"], *definition);
        assert_eq!(row["parameters"]["completion"]["kind"], "complete");
        if ordinal != f.gems[0] {
            let id = preservation::link(sidecar, ordinal, "support");
            assert!(assignments.insert(id["local"].as_str().unwrap().to_owned()));
            let assignment = preservation::member(draft, "supports", &id);
            assert_eq!(assignment["support"], json!({"kind":"known","value":gem}));
            assert_eq!(
                assignment["target"],
                json!({"kind":"authored","value":{"kind":"known","value":primary}})
            );
            assert_eq!(assignment["enabled"], json!({"kind":"known","value":true}));
        }
    }
    let members = positive::usage(draft)["members"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    let mut policies = BTreeSet::new();
    for member in members {
        let selection = &member["selection"];
        assert_eq!(selection["policy"]["kind"], "known");
        policies.insert(
            selection["policy"]["value"]["key"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
        assert_eq!(selection["target"]["kind"], "skill");
        assert_eq!(selection["target"]["value"]["kind"], "generated");
        assert_eq!(
            selection["target"]["value"]["value"]["provider"]["root"],
            json!({"kind":"skill_use","value":{"kind":"known","value":primary}})
        );
        assert_eq!(
            selection["target"]["value"]["value"]["provider"]["grant_path"],
            json!({"members":[],"completion":{"kind":"complete"}})
        );
        assert_eq!(
            selection["target"]["value"]["value"]["slot"]["value"]["slot"]["key"],
            "def.000000000000327e"
        );
        if !count_complete && selection["policy"]["value"]["key"] == "def.000000000000326a" {
            assert_eq!(selection["parameters"]["members"], json!([]));
            assert_eq!(selection["parameters"]["completion"]["kind"], "pending");
            assert_eq!(
                selection["parameters"]["completion"]["code"],
                "usage-parameters-not-converted"
            );
            let issue = &selection["parameters"]["completion"]["id"];
            let origin = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["source"]["ordinal"] == f.gems[0])
                .unwrap();
            assert!(
                origin["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"issue","value":issue}))
            );
        } else {
            assert_eq!(selection["parameters"]["completion"]["kind"], "complete");
        }
    }
    assert_eq!(
        policies,
        BTreeSet::from([
            "def.000000000000326a".to_owned(),
            "def.000000000000332b".to_owned(),
            "def.000000000000338e".to_owned()
        ])
    );
    f
}

fn publish_control(
    prior: &StagedOwnedRelease,
    omitted: &[String],
    directory: &std::path::Path,
) -> (PathBuf, StagedOwnedRelease) {
    assert!(!directory.exists());
    fs::create_dir_all(directory).unwrap();
    let staged = family::stage_omission_control(prior, omitted);
    let input = directory.join("input.json");
    write(&input, staged.input());
    let package = directory.join("package");
    assert_eq!(family::publish(&input, &package), json!(staged.receipt()));
    let actual = release::load(&package);
    assert_eq!(actual.input(), staged.input());
    (package, actual)
}

fn preserve_unaffected_records(before: &Output, after: &Output, xml: &[u8]) {
    let f = frame(xml, 17);
    // A refused count conversion legitimately allocates a new issue, shifting
    // later local IDs. Compare exact source-linked records after independently
    // checking their identities/relations in actual_records; do not erase IDs
    // from the complete draft and pretend to have a whole-output inverse.
    for ordinal in f.gems {
        let mut records = Vec::new();
        for output in [before, after] {
            let id = preservation::link(&output.sidecar, ordinal, "gem");
            let mut record = preservation::member(&output.draft, "gems", &id).clone();
            assert_eq!(record["id"], id);
            record.as_object_mut().unwrap().remove("id");
            records.push(record);
        }
        assert_eq!(
            records[0], records[1],
            "all exact physical input fields retained"
        );
    }
    for policy in ["def.000000000000332b", "def.000000000000338e"] {
        let parameters: Vec<_> = [before, after]
            .into_iter()
            .map(|output| {
                let matches: Vec<_> = positive::usage(&output.draft)["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| r["selection"]["policy"]["value"]["key"] == policy)
                    .collect();
                assert_eq!(matches.len(), 1);
                matches[0]["selection"]["parameters"].clone()
            })
            .collect();
        assert_eq!(
            parameters[0], parameters[1],
            "independent participation/reporting retained"
        );
    }
    let mut codes = before.report["issue_codes"].clone();
    for code in [
        "usage-parameters-not-converted",
        "usage-preferences-not-converted",
    ] {
        assert!(codes.get(code).is_none());
        codes[code] = json!(1);
    }
    assert_eq!(codes, after.report["issue_codes"]);
    assert_eq!(
        after.report["issue_count"].as_u64().unwrap(),
        before.report["issue_count"].as_u64().unwrap() + 2
    );
    let source_rows = |output: &Output| {
        output.sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["source"].clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        source_rows(before),
        source_rows(after),
        "no origin or source identity omitted"
    );
}

fn unknown_sibling_control(
    original: &[u8],
    base: &[u8],
    next: &StagedOwnedRelease,
    package: &std::path::Path,
    out: &std::path::Path,
) -> Value {
    let packet = read(data().join("companion-additions.json"));
    let control = &packet["unknown_sibling_control"];
    let ordinal = control["source"]["ordinal"].as_u64().unwrap() as usize;
    let source_instance = positive::instance(original);
    let source =
        SourceProjectEvidence::collect(&source_instance, SourceEvidenceLimits::default()).unwrap();
    let row = &source.rows()[ordinal];
    assert_eq!(row.occurrence().name(), "Gem");
    assert!(row.children().is_empty());
    assert_eq!(
        row.attributes().len(),
        control["source"]["attributes"].as_array().unwrap().len()
    );
    for expected in control["source"]["attributes"].as_array().unwrap() {
        assert_eq!(
            row.attribute(expected["name"].as_str().unwrap())
                .unwrap()
                .decoded()
                .unwrap(),
            expected["decoded"].as_str().unwrap()
        );
    }
    let f = frame(base, 17);
    let base_instance = positive::instance(base);
    let evidence =
        SourceProjectEvidence::collect(&base_instance, SourceEvidenceLimits::default()).unwrap();
    let position = evidence.rows()[*f.gems.last().unwrap()]
        .occurrence()
        .range()
        .end;
    let range = row.occurrence().range();
    let mut xml = base[..position].to_vec();
    xml.extend_from_slice(&original[range.clone()]);
    xml.extend_from_slice(&base[position..]);
    let mut inverse = xml.clone();
    inverse.drain(position..position + range.len());
    assert_eq!(
        inverse, base,
        "only one exact unreviewed sibling byte range added"
    );
    let expanded = frame(&xml, 18);
    let expanded_instance = positive::instance(&xml);
    let expanded_evidence =
        SourceProjectEvidence::collect(&expanded_instance, SourceEvidenceLimits::default())
            .unwrap();
    assert!(
        expanded_evidence.rows()[expanded.group]
            .attribute("groupCount")
            .is_none()
    );
    assert_eq!(
        &xml[expanded_evidence.rows()[*expanded.gems.last().unwrap()]
            .occurrence()
            .range()],
        &original[range.clone()]
    );
    let directory = out.join("unreviewed-sibling");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("source.xml");
    fs::write(&path, &xml).unwrap();
    let output = directory.join("output");
    let repeated = directory.join("repeated");
    release::normalize(package, &path, 5, &output);
    release::normalize(package, &path, 5, &repeated);
    let actual = checked_output(&output, package, 5, next, &xml);
    assert_eq!(actual, checked_output(&repeated, package, 5, next, &xml));
    let gem = preservation::link(&actual.sidecar, *expanded.gems.last().unwrap(), "gem");
    let record = preservation::member(&actual.draft, "gems", &gem);
    assert_eq!(
        record["definition"],
        json!({"kind":"known","value":control["role"]["gem"]})
    );
    let count: Vec<_> = positive::usage(&actual.draft)["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["selection"]["policy"]["value"]["key"] == "def.000000000000326a")
        .collect();
    assert_eq!(count.len(), 1);
    let parameters = &count[0]["selection"]["parameters"];
    assert_eq!(parameters["members"], json!([]));
    assert_eq!(parameters["completion"]["kind"], "pending");
    assert_eq!(
        parameters["completion"]["code"],
        "usage-parameters-not-converted"
    );
    assert_eq!(
        positive::usage(&actual.draft)["completion"]["kind"],
        "pending"
    );
    let primary_origin = actual.sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"]["ordinal"] == expanded.gems[0])
        .unwrap();
    assert!(
        primary_origin["links"]
            .as_array()
            .unwrap()
            .contains(&json!({"kind":"issue","value":parameters["completion"]["id"]}))
    );
    assert_eq!(fs::read(path).unwrap(), xml);
    json!({
        "original_ordinal":ordinal,"original_range":[range.start,range.end],
        "inserted_sha256":sha(&original[range]),"derived_sha256":sha(&xml),
        "known_but_unreviewed_companion":control["role"]["gem"],
        "count_parameters":"pending","usage":"pending","fresh_repeat_identical":true,
        "all_original_and_existing_gem_bytes_unchanged":true,"group_attributes_unchanged":true,
        "other_pending_input_proofs_are_not_claimed_complete":true
    })
}

pub fn run() {
    let before =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_EXPANSION_PRIOR").unwrap());
    let after =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_EXPANSION_NEXT").unwrap());
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SUPPORT_EXPANSION_OUTPUT").unwrap());
    assert!(!out.exists());
    let pins = positive::inputs();
    family::checked_predecessor(&before);
    let before_files = release::inventory(&before);
    let after_files = release::inventory(&after);
    let prior = release::load(&before);
    let next = release::load(&after);
    family::checked_successor(&prior, &next);
    let original_path = root().join(pins["originals"][4]["path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    let (xml, expected, proof) = expanded_source(&original);
    fs::create_dir_all(&out).unwrap();
    let source = out.join("derived-accounting-only.xml");
    fs::write(&source, &xml).unwrap();
    write(out.join("derivation.json"), &proof);
    let evidence = read(data().join("evidence.json"));
    let mut keys: Vec<_> = evidence["support_family_expansion"]["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["gem"]["key"].as_str().unwrap().to_owned())
        .collect();
    keys.sort();
    assert_eq!(keys.len(), 9);
    let (baseline_package, baseline_release) =
        publish_control(&prior, &keys, &out.join("sixteen-row-control"));
    let baseline_dir = out.join("sixteen-output");
    let expanded_dir = out.join("expanded-output");
    release::normalize(&baseline_package, &source, 5, &baseline_dir);
    release::normalize(&after, &source, 5, &expanded_dir);
    let baseline = checked_output(&baseline_dir, &baseline_package, 5, &baseline_release, &xml);
    let expanded = checked_output(&expanded_dir, &after, 5, &next, &xml);
    let f = actual_records(&baseline, &xml, &expected, true);
    actual_records(&expanded, &xml, &expected, true);
    let retirement = positive::compare_usage_retirement(&baseline, &expanded, f.group, f.gems[0]);
    let old_final = selected::finalize_with_definitions(
        &xml,
        &baseline_dir,
        &out.join("sixteen-selection.json"),
        &baseline_package.join("schema.json"),
    );
    let new_final = selected::finalize_with_definitions(
        &xml,
        &expanded_dir,
        &out.join("expanded-selection.json"),
        &after.join("schema.json"),
    );
    let mut old_issues = old_final["finalization"]["issues"].clone();
    let mut new_issues = new_final["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    let issues = old_issues.as_array_mut().unwrap();
    assert_eq!(
        issues
            .iter()
            .filter(|r| r["code"] == "usage-preferences-not-converted")
            .count(),
        1
    );
    issues.retain(|r| r["code"] != "usage-preferences-not-converted");
    assert_eq!(old_issues, new_issues);
    assert!(!new_issues.as_array().unwrap().is_empty());
    let mut controls = Vec::new();
    for key in &keys {
        let (package, staged) = publish_control(
            &prior,
            std::slice::from_ref(key),
            &out.join(format!("omit-{key}")),
        );
        let directory = out.join(format!("output-without-{key}"));
        release::normalize(&package, &source, 5, &directory);
        let actual = checked_output(&directory, &package, 5, &staged, &xml);
        actual_records(&actual, &xml, &expected, true);
        assert_eq!(
            positive::usage(&actual.draft)["completion"]["kind"],
            "pending"
        );
        assert_eq!(
            actual, baseline,
            "omitting {key} preserves the entire unresolved output"
        );
        controls.push(json!({"omitted_family":key,"usage":"pending","full_output_inverse":true}));
    }
    assert_eq!(controls.len(), 9);
    let mut companion_controls = Vec::new();
    for key in &keys {
        let directory = out.join(format!("companion-control-{key}"));
        fs::create_dir_all(&directory).unwrap();
        let staged = family::stage_companion_omission_control(&prior, std::slice::from_ref(key));
        let input = directory.join("input.json");
        write(&input, staged.input());
        let package = directory.join("package");
        assert_eq!(family::publish(&input, &package), json!(staged.receipt()));
        let loaded = release::load(&package);
        assert_eq!(loaded.input(), staged.input());
        let output = directory.join("output");
        let repeated = directory.join("repeated");
        release::normalize(&package, &source, 5, &output);
        release::normalize(&package, &source, 5, &repeated);
        let actual = checked_output(&output, &package, 5, &loaded, &xml);
        assert_eq!(
            actual,
            checked_output(&repeated, &package, 5, &loaded, &xml)
        );
        actual_records(&actual, &xml, &expected, false);
        assert_eq!(
            positive::usage(&actual.draft)["completion"]["kind"],
            "pending"
        );
        preserve_unaffected_records(&expanded, &actual, &xml);
        companion_controls.push(json!({"omitted_companion":key,"inventory_rows":25,"count_parameters":"pending","usage":"pending","fresh_repeat_identical":true,"all_source_bytes_unchanged":true}));
    }
    assert_eq!(companion_controls.len(), 9);
    let unknown_control = unknown_sibling_control(&original, &xml, &next, &after, &out);
    assert_eq!(fs::read(&source).unwrap(), xml);
    assert_eq!(fs::read(original_path).unwrap(), original);
    assert_eq!(release::inventory(&before), before_files);
    assert_eq!(release::inventory(&after), after_files);
    positive::inputs(); // Reauthenticate all five original source files and maintained pins.
    write(
        out.join("validation.json"),
        &json!({
            "authored_input":next.receipt().input,"baseline_rows":16,"expanded_rows":25,
            "exact_copied_new_occurrences":13,"retirement":retirement,"inventory_omission_controls":controls,
            "companion_omission_controls":companion_controls,"unreviewed_sibling_control":unknown_control,
            "default_limits":true,"original_five_unchanged":true,"same_source_for_all_policies":true,
            "support_eligibility_checked":false,"calculation":"not_run","whole_build_finalization":"pending"
        }),
    );
}
