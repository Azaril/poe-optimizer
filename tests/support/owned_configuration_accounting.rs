//! A source-bound, partial configuration disposition; no numerical producer.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_normalize::{
        ConfigurationDispositionPolicy, ConfigurationDispositionRule,
        configuration_disposition_dependencies,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn directory() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/configuration-accounting")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn rules() -> Vec<ConfigurationDispositionRule> {
    read("rules.json")
}
fn commitment(domain: &'static str, name: &str) -> OwnedContentDigest {
    digest_owned(domain, &read::<Value>(name), 2 * 1024 * 1024).unwrap()
}

/// Preserve the acquisition report's full baseline and exact output commitments.
/// No sorting, rounding, skipped fields or retry-to-agreement is involved.
pub fn validate_observations(report: &Value) -> Result<(), &'static str> {
    macro_rules! require {
        ($condition:expr, $message:literal) => {
            if !$condition {
                return Err($message);
            }
        };
    }
    let names = [
        "original-05",
        "input-0",
        "input-100",
        "input-37.5",
        "placeholder-0",
        "placeholder-100",
        "placeholder-missing",
        "explicit-none-input-100",
        "repeat-original-05",
        "warm-changed-to-original-05",
        "boss-shaper-ball-0",
        "boss-shaper-ball-100",
    ];
    let Some(cases) = report["cases"].as_array() else {
        return Err("cases");
    };
    require!(cases.len() == names.len(), "case census");
    require!(report["evidence"]["case_count"] == 12, "declared census");
    require!(
        report["evidence"]["complete_load_attempts_per_jit"] == 13,
        "load census"
    );
    require!(
        report["evidence"]["business_method_wrappers"] == false,
        "wrappers"
    );
    require!(
        report["evidence"]["native_coverage"] == false
            && report["evidence"]["whole_build_parity"] == false,
        "scope"
    );
    let baseline = &cases[0]["state"];
    for (case, expected_name) in cases.iter().zip(names) {
        require!(
            case["name"] == expected_name && case["available"] == true,
            "available named case"
        );
        let state = &case["state"];
        let boss = expected_name.starts_with("boss-");
        require!(
            state["original_functions_preserved"] == true
                && state["business_method_wrappers"] == false
                && state["hook_restored"] == true,
            "original consumer observation"
        );
        require!(
            state["roll_definition"]
                == json!({"has_apply":false,"kind":"countAllowZero","default_placeholder":70}),
            "control definition"
        );
        require!(state["selected"] == baseline["selected"], "saved selection");
        let Some(callbacks) = state["callbacks"].as_array() else {
            return Err("callbacks");
        };
        require!(!callbacks.is_empty(), "observed callback required");
        require!(
            callbacks.last().unwrap()["value"] == if boss { "Shaper Ball" } else { "None" },
            "final branch"
        );
        for callback in callbacks {
            let selected = callback["value"] == "Shaper Ball";
            require!(
                callback["value"] == "None" || (boss && selected),
                "branch identity"
            );
            require!(
                callback["guard_observed"] == true
                    && callback["boss_branch"] == selected
                    && callback["none_branch"] == !selected
                    && callback["roll_read"] == selected
                    && callback["roll_available"] == selected,
                "actual branch and read"
            );
            if selected {
                require!(
                    callback["roll"].as_f64()
                        == Some(if expected_name.ends_with("-100") {
                            100.0
                        } else {
                            0.0
                        }),
                    "contrast roll"
                );
            } else {
                require!(callback["roll"].is_null(), "unread is not zero");
                for field in ["damage_input", "damage_placeholder"] {
                    require!(
                        callback["before"][field] == callback["after"][field],
                        "inactive callback writes"
                    );
                }
            }
        }
        for mode in ["main", "calcs"] {
            let view = &state[mode];
            require!(
                view["identity"] == baseline[mode]["identity"],
                "exact actor identity"
            );
            require!(
                view["mode"] == if mode == "main" { "MAIN" } else { "CALCS" },
                "mode"
            );
            require!(
                view["identity"]["summon"]
                    == if mode == "main" {
                        "SummonSkeletalSnipersPlayer"
                    } else {
                        "SummonSkeletalArsonistsPlayer"
                    },
                "saved mode-specific summon"
            );
            if !boss {
                for field in ["boss", "input_damage", "placeholder_damage", "damage_type"] {
                    require!(
                        view[field] == baseline[mode][field],
                        "inactive configuration output"
                    );
                }
                for actor in ["player", "selected_minion"] {
                    let base = &baseline[mode][actor];
                    let Some(scalars) = base["scalars"].as_object() else {
                        return Err("baseline scalar map");
                    };
                    let Some(availability) = base["availability"].as_object() else {
                        return Err("baseline availability map");
                    };
                    require!(
                        scalars.len() > 100 && base["availability"]["Life"] == "number",
                        "complete observed baseline"
                    );
                    if expected_name != "original-05" {
                        let output = &view[actor];
                        require!(
                            output["scalar_count"] == scalars.len()
                                && output["availability_count"] == availability.len(),
                            "output census"
                        );
                        require!(
                            output["scalar_sha256"]
                                == sha(&serde_json::to_vec(&base["scalars"]).unwrap())
                                && output["availability_sha256"]
                                    == sha(&serde_json::to_vec(&base["availability"]).unwrap()),
                            "complete output commitments"
                        );
                        let Some(sentinels) = output["sentinels"].as_object() else {
                            return Err("sentinels");
                        };
                        for (name, value) in sentinels {
                            require!(scalars.get(name) == Some(value), "sentinel");
                        }
                    }
                }
            }
        }
    }
    for mode in ["main", "calcs"] {
        let low = &cases[10]["state"][mode];
        let high = &cases[11]["state"][mode];
        require!(
            low["boss"] == "Shaper Ball" && high["boss"] == "Shaper Ball",
            "positive contrast boss"
        );
        require!(
            low["damage_type"] == "SpellProjectile" && high["damage_type"] == "SpellProjectile",
            "contrast type"
        );
        require!(
            low["placeholder_damage"]["Cold"].as_f64() == Some(4127.0)
                && high["placeholder_damage"]["Cold"].as_f64() == Some(6190.0),
            "positive contrast damage"
        );
        require!(
            high["player"]["sentinels"]["totalEnemyDamageIn"]
                .as_f64()
                .zip(low["player"]["sentinels"]["totalEnemyDamageIn"].as_f64())
                .is_some_and(|(high, low)| high > low),
            "positive contrast output"
        );
    }
    Ok(())
}

pub fn check_authored() {
    let pins: Value = read("evidence.json");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(pins["source_manifest_sha256"], sha(&manifest_bytes));
    assert_eq!(pins["source_revision"], manifest["upstream_revision"]);
    for pin in pins["reports"]
        .as_array()
        .unwrap()
        .iter()
        .chain(pins["producers"].as_array().unwrap())
        .chain(pins["source_archives"].as_array().unwrap())
    {
        let bytes = fs::read(directory().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], sha(&bytes));
    }
    let off = fs::read(directory().join("evidence/source-jit-off.json")).unwrap();
    assert_eq!(
        off,
        fs::read(directory().join("evidence/source-jit-on.json")).unwrap()
    );
    assert_eq!(off.len(), 339448);
    assert_eq!(
        sha(&off),
        "c20f9c8992fa062f59b42bcc1da77abc17581bf9e1a7133c6fef20251e27d2b6"
    );
    let report: Value = serde_json::from_slice(&off).unwrap();
    validate_observations(&report).unwrap();
    assert_eq!(report["source_hash"], pins["source_manifest_sha256"]);
    assert_eq!(report["source_revision"], pins["source_revision"]);
    assert_eq!(
        report["evidence"]["observer_sha256"],
        sha(&fs::read(directory().join("evidence/owned_damage_roll_source.lua")).unwrap())
    );
    let original =
        fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    assert_eq!(pins["original"]["sha256"], sha(&original));
    assert_eq!(report["evidence"]["original"], pins["original"]);
    let inputs = acquisition_inputs(std::str::from_utf8(&original).unwrap());
    for ((name, xml, warm), case) in inputs.iter().zip(report["cases"].as_array().unwrap()) {
        assert_eq!(case["name"], *name);
        assert_eq!(case["xml_sha256"], sha(xml.as_bytes()));
        assert_eq!(
            case["warm_xml_sha256"],
            json!(warm.as_ref().map(|s| sha(s.as_bytes())))
        );
    }
    let catalogue: Value = read("catalogue-evidence.json");
    let constructor: Value = read("constructor-evidence.json");
    for evidence in [&catalogue, &constructor] {
        assert_eq!(evidence["source_revision"], pins["source_revision"]);
        assert_eq!(
            evidence["source_manifest_sha256"],
            pins["source_manifest_sha256"]
        );
    }
    assert_eq!(catalogue["source_files"], report["evidence"]["files"]);
    for file in catalogue["source_files"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pin| pin["path"] == file["path"] && pin["sha256"] == file["sha256"])
        );
    }
    for span in catalogue["spans"].as_array().unwrap() {
        assert!(
            catalogue["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pin| pin["path"] == span["path"] && pin["sha256"] == span["file_sha256"])
        );
        assert_eq!(
            span["lines"].as_array().unwrap().len() as u64,
            span["last_line"].as_u64().unwrap() - span["first_line"].as_u64().unwrap() + 1
        );
        let archive = pins["source_archives"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pin| pin["source_path"] == span["path"])
            .unwrap();
        assert_eq!(archive["sha256"], span["file_sha256"]);
        let contents =
            fs::read_to_string(directory().join(archive["path"].as_str().unwrap())).unwrap();
        let lines: Vec<_> = contents
            .lines()
            .skip(span["first_line"].as_u64().unwrap() as usize - 1)
            .take(span["lines"].as_array().unwrap().len())
            .collect();
        assert_eq!(json!(lines), span["lines"]);
    }
    assert_eq!(
        constructor["roll_definition"],
        report["cases"][0]["state"]["roll_definition"]
    );
    assert_eq!(constructor["report_sha256"], sha(&off));
    assert_eq!(constructor["default_value"], "None");
    assert_eq!(constructor["native_default_emitted"], false);
    let rows = rules();
    assert_eq!(rows.len(), 1);
    assert_eq!(catalogue["control"]["name"], rows[0].selector.name);
    assert_eq!(catalogue["control"]["lane"], json!(rows[0].selector.lane));
    assert_eq!(catalogue["control"]["has_apply"], false);
}

/// Test-only XML mutation; source ranges come from the existing lossless parser.
pub fn field(xml: &str, element: &str, name: &str, value: Option<(&str, &str)>) -> String {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([141; 16]),
        Default::default(),
    )
    .unwrap();
    let mut ranges: Vec<_> = source
        .occurrences()
        .iter()
        .filter(|row| {
            row.name() == element
                && source
                    .attribute(row.id(), "name")
                    .unwrap()
                    .is_some_and(|a| a.decoded() == name)
        })
        .map(|row| row.range())
        .collect();
    ranges.sort_by_key(|range| range.start);
    let mut output = xml.to_owned();
    for range in ranges.into_iter().rev() {
        output.replace_range(range, "");
    }
    if let Some((attribute, value)) = value {
        assert!(!value.contains(['<', '&', '"']));
        assert_eq!(output.matches("</ConfigSet>").count(), 1);
        output = output.replacen(
            "</ConfigSet>",
            &format!("<{element} name=\"{name}\" {attribute}=\"{value}\"/></ConfigSet>"),
            1,
        );
    }
    output
}
fn acquisition_inputs(original: &str) -> Vec<(&'static str, String, Option<String>)> {
    let mut rows = vec![("original-05", original.to_owned(), None)];
    for (name, value) in [
        ("input-0", "0"),
        ("input-100", "100"),
        ("input-37.5", "37.5"),
    ] {
        rows.push((
            name,
            field(
                original,
                "Input",
                "enemyDamageRollRange",
                Some(("number", value)),
            ),
            None,
        ));
    }
    for (name, value) in [
        ("placeholder-0", Some("0")),
        ("placeholder-100", Some("100")),
        ("placeholder-missing", None),
    ] {
        rows.push((
            name,
            field(
                original,
                "Placeholder",
                "enemyDamageRollRange",
                value.map(|v| ("number", v)),
            ),
            None,
        ));
    }
    let changed = field(
        original,
        "Input",
        "enemyDamageRollRange",
        Some(("number", "100")),
    );
    rows.push((
        "explicit-none-input-100",
        field(
            &changed,
            "Input",
            "presetBossSkills",
            Some(("string", "None")),
        ),
        None,
    ));
    rows.push(("repeat-original-05", original.to_owned(), None));
    rows.push((
        "warm-changed-to-original-05",
        original.to_owned(),
        Some(changed),
    ));
    for (name, value) in [("boss-shaper-ball-0", "0"), ("boss-shaper-ball-100", "100")] {
        let boss = field(
            original,
            "Input",
            "presetBossSkills",
            Some(("string", "Shaper Ball")),
        );
        rows.push((
            name,
            field(
                &boss,
                "Input",
                "enemyDamageRollRange",
                Some(("number", value)),
            ),
            None,
        ));
    }
    rows
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authored: Value = read("authoring.json");
    assert_eq!(authored["before"], json!(prior.receipt().input));
    assert_eq!(
        authored["mapping_source"],
        json!(prior.mapping().source_identity())
    );
    let source = json!(prior.receipt().source);
    let evidence: Value = read("evidence.json");
    assert_eq!(source["revision"], evidence["source_revision"]);
    for pin in evidence["source_archives"].as_array().unwrap() {
        assert!(
            source["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|file| file["path"] == pin["source_path"] && file["sha256"] == pin["sha256"]),
            "actual release source footprint must include the reviewed semantic source"
        );
    }
    let mut full = prior.input().clone();
    assert!(full.normalization.configuration_dispositions.is_none());
    full.normalization.configuration_dispositions = Some(ConfigurationDispositionPolicy {
        inventory: None,
        mapping_source: *prior.mapping().source_identity(),
        catalogue_evidence: commitment(
            "owned-configuration-catalogue-evidence-v1",
            "catalogue-evidence.json",
        ),
        constructor_evidence: commitment(
            "owned-configuration-constructor-evidence-v1",
            "constructor-evidence.json",
        ),
        dependencies: configuration_disposition_dependencies(
            prior.normalization(),
            prior.rewards(),
            Default::default(),
        )
        .unwrap(),
        rules: rules(),
    });
    full.normalization.version = key(authored["normalization_version"].as_str().unwrap());
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            prior.assembled().registry(),
            prior.assembled().schema(),
            prior.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: key(authored["provenance_kind"].as_str().unwrap()),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-configuration-accounting-authoring-v1",
            &(
                authored,
                read::<Value>("evidence.json"),
                &full.normalization.configuration_dispositions,
            ),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization.configuration_dispositions = None;
    restored.normalization.version = prior.normalization().version.clone();
    assert_eq!(restored.normalization, *prior.normalization());
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    restored.tree = prior.input().tree.clone();
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert_eq!(restored, *prior.input(), "exact release inverse");
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().rules, prior.receipt().rules);
    assert_eq!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    next
}
