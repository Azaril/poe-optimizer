//! Structural mutation and fresh-host realization tests. Source fixtures stay immutable.
use poe_optimizer_core::{
    EvaluationSnapshot,
    candidate::{CandidateBudgets, CandidateConstraints, CandidateDomain},
    evaluation::{BackendIdentity, BuildDocument, BuildFormat, EvaluationResult},
};
use poe_optimizer_pob::mutation::{
    ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn fixture(name: &str) -> String {
    fs::read_to_string(root().join(format!("tests/fixtures/calibration/{name}.xml"))).unwrap()
}
fn weapons(quality: u32) -> Vec<NormalMaceAlternative> {
    [("wooden", "Wooden Club"), ("smithing", "Smithing Hammer")]
        .into_iter()
        .map(|(id, name)| NormalMaceAlternative {
            id: id.into(),
            item_text: format!(
                "Rarity: NORMAL\n{name}\nItem Level: 1\nQuality: {quality}\nImplicits: 0"
            ),
        })
        .collect()
}
fn supports() -> Vec<MaceSupportChoice> {
    vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI]
}
fn registry() -> ControlledMaceCatalog {
    ControlledMaceCatalog::new(fixture("mace-wooden"), weapons(0), supports()).unwrap()
}

#[test]
fn structural_profile_preserves_nonmutable_source_bytes_and_exact_payloads() {
    let xml = fixture("mace-wooden")
        .replace("level=\"60\"", "level=\"61\"")
        .replace("number=\"60\"", "number=\"65\"")
        .replace(
            "<Skills",
            "<!-- custom explanation: preserve &amp; literally -->\n  <Skills",
        )
        .replace("<Notes>", "<Notes>User notes &amp; literal symbols: ");
    let catalog = ControlledMaceCatalog::new(xml.clone(), weapons(20), supports()).unwrap();
    assert_eq!(catalog.alternatives().len(), 4);
    let domain = CandidateDomain::new(
        catalog.catalog().clone(),
        CandidateConstraints {
            budgets: CandidateBudgets {
                active_skill_count: 1,
                supports_per_skill: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    for alternative in catalog.alternatives() {
        assert!(domain.validate(&alternative.candidate).is_searchable());
        let materialized = catalog.materialize(&alternative.candidate).unwrap().content;
        assert!(materialized.contains("<!-- custom explanation: preserve &amp; literally -->"));
        assert!(materialized.contains("<Notes>User notes &amp; literal symbols: "));
        let before_config = xml.split_once("<Config").unwrap().1;
        assert_eq!(materialized.split_once("<Config").unwrap().1, before_config);
        assert!(materialized.contains("level=\"61\""));
        assert!(materialized.contains("Quality: 20"));
        let expected = if alternative.weapon_id == "smithing" {
            "Smithing Hammer"
        } else {
            "Wooden Club"
        };
        assert!(materialized.contains(&format!("Rarity: NORMAL\n{expected}\n")));
        assert_eq!(
            materialized
                .matches("skillId=\"SupportBrutalityPlayer\"")
                .count(),
            usize::from(alternative.support == MaceSupportChoice::BrutalityI)
        );
        assert_eq!(
            catalog.resolve_candidate(&alternative.weapon_id, alternative.support),
            Some(&alternative.candidate)
        );
    }
    let mut changed = catalog.alternatives()[0].candidate.clone();
    changed.passives.insert(10);
    assert!(catalog.materialize(&changed).is_err());
    assert_eq!(catalog.template_build().content, xml);
}

#[test]
fn existing_support_is_removed_or_replaced_without_changing_the_active_gem() {
    let source = fixture("mace-wooden-brutality");
    let catalog = ControlledMaceCatalog::new(source.clone(), weapons(0), supports()).unwrap();
    for alternative in catalog.alternatives() {
        let xml = catalog.materialize(&alternative.candidate).unwrap().content;
        assert_eq!(xml.matches("skillId=\"Melee1HMacePlayer\"").count(), 1);
        assert_eq!(
            xml.matches("skillId=\"SupportBrutalityPlayer\"").count(),
            usize::from(alternative.support == MaceSupportChoice::BrutalityI)
        );
        assert_eq!(
            xml.split_once("<Config").unwrap().1,
            source.split_once("<Config").unwrap().1
        );
    }
}

#[test]
fn identity_is_order_independent_but_captures_template_and_exact_weapon_settings() {
    let first = registry();
    let mut reversed = weapons(0);
    reversed.reverse();
    let mut support = supports();
    support.reverse();
    let second = ControlledMaceCatalog::new(fixture("mace-wooden"), reversed, support).unwrap();
    assert_eq!(first.catalog(), second.catalog());
    for (source, choices) in [
        (
            fixture("mace-wooden").replace("level=\"60\"", "level=\"61\""),
            weapons(0),
        ),
        (fixture("mace-wooden"), weapons(20)),
    ] {
        let different = ControlledMaceCatalog::new(source, choices, supports()).unwrap();
        assert_ne!(first.catalog().identity, different.catalog().identity);
        assert!(
            different
                .materialize(&first.alternatives()[0].candidate)
                .is_err()
        );
    }
}

#[test]
fn rejects_unknown_mechanics_ambiguous_sets_and_out_of_profile_inputs() {
    let source = fixture("mace-wooden");
    for (from, to) in [
        ("<Tree ", "<Tree extension=\"true\" "),
        ("nodes=\"\"", "nodes=\"3936\""),
        ("classInternalId=\"6\"", "classInternalId=\"7\""),
        ("ascendClassId=\"0\"", "ascendClassId=\"1\""),
        ("level=\"60\"", "level=\"101\""),
        ("level=\"1\"", "level=\"2\""),
        ("Quality: 0", "Quality: 21"),
        ("Rarity: NORMAL", "Rarity: RARE"),
        ("Wooden Club\n", "Unknown Mace\n"),
        ("Implicits: 0", "Implicits: 0\n10% increased Damage"),
        (
            "useSecondWeaponSet=\"false\"",
            "useSecondWeaponSet=\"true\"",
        ),
        ("</Item>", "<!-- split -->\n</Item>"),
        (
            "</ItemSet>",
            "<Slot name=\"Weapon 2\" itemId=\"1\"/></ItemSet>",
        ),
        (
            "</ConfigSet>",
            "<Input name=\"customMods\" string=\"100% more Damage\"/></ConfigSet>",
        ),
        (
            "name=\"enemyArmour\" number=\"0\"",
            "name=\"enemyArmour\" number=\"NaN\"",
        ),
        (
            "name=\"conditionEnemyShocked\" boolean=\"false\"",
            "name=\"conditionEnemyShocked\" boolean=\"true\"",
        ),
        ("</PathOfBuilding2>", "<Extension/></PathOfBuilding2>"),
    ] {
        let changed = source.replace(from, to);
        assert_ne!(changed, source, "unmatched mutation {from}");
        assert!(
            ControlledMaceCatalog::new(changed, weapons(0), supports()).is_err(),
            "accepted {from} -> {to}"
        );
    }
    assert!(ControlledMaceCatalog::new(source.clone(), vec![], supports()).is_err());
    assert!(ControlledMaceCatalog::new(source.clone(), weapons(0), vec![]).is_err());
    assert!(
        ControlledMaceCatalog::new(source.clone(), weapons(0), vec![MaceSupportChoice::None; 2])
            .is_err()
    );
    let mut duplicate = weapons(0);
    duplicate.push(duplicate[0].clone());
    duplicate[2].id = "duplicate".into();
    assert!(ControlledMaceCatalog::new(source, duplicate, supports()).is_err());
}

fn clone_result(value: &EvaluationResult) -> EvaluationResult {
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}
fn evaluate_in_child(xml: &str) -> EvaluationResult {
    let scratch = tempfile::tempdir().unwrap();
    let input = scratch.path().join("input.xml");
    let output = scratch.path().join("output.json");
    let diagnostics = scratch.path().join("stderr.log");
    let worker_scratch = scratch.path().join("worker");
    fs::create_dir(&worker_scratch).unwrap();
    fs::write(&input, xml).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "controlled_mutation_worker",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(root().join("vendor/path-of-building-poe2/src"))
        .env("POE_OPTIMIZER_MUTATION_INPUT", &input)
        .env("POE_OPTIMIZER_MUTATION_OUTPUT", &output)
        .env("POE_OPTIMIZER_MUTATION_SCRATCH", &worker_scratch)
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(&diagnostics).unwrap()))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("controlled mutation child exceeded 90 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        fs::read_to_string(diagnostics).unwrap()
    );
    let snapshot: EvaluationSnapshot = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    let measurements = poe_optimizer_pob::metrics::measurements(&snapshot);
    EvaluationResult {
        backend: BackendIdentity {
            id: "pob-poe2-mlua".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            rules_revision: snapshot.runtime.upstream_revision,
            source_fingerprint: snapshot.runtime.source_hash,
            adapter_fingerprint: snapshot.runtime.adapter_hash,
        },
        build: snapshot.build,
        context: snapshot.context,
        coverage: snapshot.coverage,
        measurements,
        exports: vec![BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: snapshot.export_xml,
        }],
        warnings: snapshot.warnings,
        elapsed_ms: snapshot.elapsed_ms,
        diagnostic_only: true,
        attachments: Vec::new(),
    }
}
fn selected_dps(result: &EvaluationResult) -> f64 {
    result
        .measurements
        .iter()
        .find(|value| {
            value.query.id == "selected_hit_dps"
                && value.query.actor == poe_optimizer_core::metrics::ActorScope::Player
        })
        .unwrap()
        .value
        .finite()
        .unwrap()
}

#[test]
fn generated_product_retains_independent_calibration_values_and_detects_realization_drift() {
    let catalog = registry();
    let baseline = evaluate_in_child(&catalog.template_build().content);
    let scenario = catalog.bind_baseline(&baseline).unwrap();
    for (from, to) in [
        ("title=\"Unallocated Warrior\"", "title=\"Different tree\""),
        ("title=\"Mace Strike\"", "title=\"Changed skills\""),
        ("label=\"Single Mace Strike\"", "label=\"Changed group\""),
        ("combatList=\"\"", "combatList=\"Onslaught\""),
        ("searchList=\"\"", "searchList=\"1234\""),
        (
            "Controlled attack/weapon/support host-calibration fixture",
            "Changed user notes",
        ),
    ] {
        let mut changed = clone_result(&baseline);
        changed.exports[0].content = changed.exports[0].content.replace(from, to);
        assert_ne!(
            changed.exports[0].content, baseline.exports[0].content,
            "unmatched source-frame mutation {from}"
        );
        assert!(
            catalog.bind_baseline(&changed).is_err(),
            "accepted source-frame drift {from}"
        );
    }
    for alternative in catalog.alternatives() {
        let result =
            evaluate_in_child(&catalog.materialize(&alternative.candidate).unwrap().content);
        catalog
            .validate_realization(&alternative.candidate, &result, &scenario)
            .unwrap_or_else(|error| panic!("{}: {error}", alternative.id));
        let suffix = if alternative.support == MaceSupportChoice::None {
            ""
        } else {
            "-brutality"
        };
        let reference: serde_json::Value = serde_json::from_slice(
            &fs::read(root().join(format!(
                "tests/fixtures/calibration/mace-{}{suffix}.reference.json",
                alternative.weapon_id
            )))
            .unwrap(),
        )
        .unwrap();
        let expected = reference["metrics"]["TotalDPS"].as_f64().unwrap();
        assert!((selected_dps(&result) - expected).abs() <= 1e-8 + expected.abs() * 1e-9);
        let mut changed = clone_result(&result);
        changed.context.config_inputs.insert(
            "enemyArmour".into(),
            poe_optimizer_core::options::Scalar::Number(100.0),
        );
        assert!(
            catalog
                .validate_realization(&alternative.candidate, &changed, &scenario)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.build.level += 1;
        assert!(
            catalog
                .validate_realization(&alternative.candidate, &changed, &scenario)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.coverage.groups[0].gems[0].quality = Some(20.0);
        assert!(
            catalog
                .validate_realization(&alternative.candidate, &changed, &scenario)
                .is_err()
        );
        for (from, to) in [
            ("LevelReq: 0", "LevelReq: 99"),
            (
                "</ConfigSet>",
                "<Input name=\"customMods\" string=\"100% more Damage\"/></ConfigSet>",
            ),
            ("statSetIndex=\"nil\"", "statSetIndex=\"2\""),
            ("runeName=\"None\"", "runeName=\"Iron Rune\""),
            ("mainActiveSkill=\"1\"", "mainActiveSkill=\"2\""),
        ] {
            let mut changed = clone_result(&result);
            changed.exports[0].content = changed.exports[0].content.replace(from, to);
            assert_ne!(
                changed.exports[0].content, result.exports[0].content,
                "unmatched {from}"
            );
            assert!(
                catalog
                    .validate_realization(&alternative.candidate, &changed, &scenario)
                    .is_err(),
                "accepted exported drift {from}"
            );
        }
    }
    let other = ControlledMaceCatalog::new(
        fixture("mace-wooden").replace("level=\"60\"", "level=\"61\""),
        weapons(0),
        supports(),
    )
    .unwrap();
    assert!(other.bind_baseline(&baseline).is_err());
}

#[test]
fn quality_and_scenario_parameters_survive_fresh_pob_round_trips() {
    let template = fixture("mace-wooden")
        .replace("level=\"60\"", "level=\"61\"")
        .replace("number=\"60\"", "number=\"65\"")
        .replace("number=\"1000\"", "number=\"1200\"")
        .replace("Quality: 0", "Quality: 20");
    let catalog = ControlledMaceCatalog::new(template, weapons(20), supports()).unwrap();
    let baseline = evaluate_in_child(&catalog.template_build().content);
    let scenario = catalog.bind_baseline(&baseline).unwrap();
    let mut values = std::collections::BTreeMap::new();
    for alternative in catalog.alternatives() {
        let result =
            evaluate_in_child(&catalog.materialize(&alternative.candidate).unwrap().content);
        catalog
            .validate_realization(&alternative.candidate, &result, &scenario)
            .unwrap_or_else(|error| panic!("{}: {error}", alternative.id));
        assert_eq!(result.build.level, 61);
        assert_eq!(result.context.enemy_level, 65);
        values.insert(alternative.id.clone(), selected_dps(&result));
    }
    assert!(values["smithing/none"] > values["wooden/none"]);
    assert!(values["wooden/brutality_i"] > values["smithing/brutality_i"]);
}

#[test]
fn explicit_pinnacle_encounter_and_high_item_level_remain_immutable() {
    let template = fixture("mace-wooden")
        .replace("level=\"60\"", "level=\"100\"")
        .replace("number=\"60\"", "number=\"82\"")
        .replace(
            "name=\"enemyIsBoss\" string=\"None\"",
            "name=\"enemyIsBoss\" string=\"Pinnacle\"",
        )
        .replace(
            "name=\"enemyArmour\" number=\"0\"",
            "name=\"enemyArmour\" number=\"1500\"",
        )
        .replace(
            "name=\"enemyFireResist\" number=\"0\"",
            "name=\"enemyFireResist\" number=\"50\"",
        )
        .replace(
            "name=\"enemyFireDamage\" number=\"0\"",
            "name=\"enemyFireDamage\" number=\"200\"",
        )
        .replace("Item Level: 1", "Item Level: 70")
        .replace("Quality: 0", "Quality: 20");
    let mut choices = weapons(20);
    for choice in &mut choices {
        choice.item_text = choice.item_text.replace("Item Level: 1", "Item Level: 70");
    }
    let catalog =
        ControlledMaceCatalog::new(template, choices, vec![MaceSupportChoice::None]).unwrap();
    let baseline = evaluate_in_child(&catalog.template_build().content);
    let scenario = catalog.bind_baseline(&baseline).unwrap();
    for alternative in catalog.alternatives() {
        let result =
            evaluate_in_child(&catalog.materialize(&alternative.candidate).unwrap().content);
        catalog
            .validate_realization(&alternative.candidate, &result, &scenario)
            .unwrap();
        assert_eq!(result.build.level, 100);
        assert_eq!(result.context.enemy_level, 82);
        assert_eq!(
            result.context.config_inputs["enemyIsBoss"],
            poe_optimizer_core::options::Scalar::Text("Pinnacle".into())
        );
        assert!(selected_dps(&result) > 0.0);
    }
}
#[test]
#[ignore = "dedicated Lua host child invoked by realization tests"]
fn controlled_mutation_worker() {
    let input =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_MUTATION_INPUT").expect("child-only input"));
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_MUTATION_OUTPUT").expect("child-only output"),
    );
    let scratch = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_MUTATION_SCRATCH").expect("child-only scratch"),
    );
    let result = poe_optimizer_pob::runtime::evaluate(
        &root().join("vendor/path-of-building-poe2"),
        &scratch,
        &fs::read_to_string(input).unwrap(),
    )
    .unwrap();
    fs::write(output, serde_json::to_vec(&result).unwrap()).unwrap();
}
