//! Native numeric-profile publication, finite evaluation and original-build gates.
#[path = "support/owned_intrinsic_predecessor.rs"]
mod predecessor;
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_item_bases::ItemBasePolicy,
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_weapon_profiles::{WeaponFieldAbsence, WeaponProfileCatalog, WeaponProfilePolicy},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{bundle, data, json, normalize, success};
fn command(cwd: &Path, name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(cwd).arg(name);
    c
}
fn run(cwd: &Path, prior: &Path, output: &Path) -> Output {
    command(cwd, "compile-owned-weapon-profiles")
        .arg(prior)
        .arg("--base-catalog")
        .arg(data().join("item-bases/catalog.json"))
        .arg("--catalog")
        .arg(data().join("weapon-profiles/catalog.json"))
        .arg("--policy")
        .arg(data().join("weapon-profiles/policy.json"))
        .arg("--definitions")
        .arg(data().join("weapon-profiles/definitions.json"))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn predecessor(cwd: &Path) -> PathBuf {
    let intrinsic = predecessor::intrinsic_predecessor(cwd);
    let prior = cwd.join("bases");
    success(
        command(cwd, "compile-owned-item-bases")
            .arg(intrinsic)
            .arg("--catalog")
            .arg(data().join("item-bases/catalog.json"))
            .arg("--policy")
            .arg(data().join("item-bases/policy.json"))
            .arg("--definitions")
            .arg(data().join("item-bases/definitions.json"))
            .arg("--output")
            .arg(&prior)
            .output()
            .unwrap(),
    );
    prior
}
#[test]
fn all_profiles_publish_compactly_without_closing_original_builds() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path();
    let prior = predecessor(cwd);
    let before = bundle(&prior);
    let output = cwd.join("numeric");
    let report = success(run(cwd, &prior, &output));
    assert_eq!(report["weapon_profiles"]["converted_profiles"], 337);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert!(!output.join("recipe.json").exists());
    assert_eq!(json(output.join("registry.json"))["last_issued"], 9254);
    let recipe = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry: serde_json::from_value(json(output.join("registry.json"))).unwrap(),
            schema: serde_json::from_value(json(output.join("schema.json"))).unwrap(),
            rules: serde_json::from_value(json(output.join("rules.json"))).unwrap(),
            routing: serde_json::from_value(json(output.join("routing.json"))).unwrap(),
        },
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let policy: WeaponProfilePolicy =
        serde_json::from_value(json(data().join("weapon-profiles/policy.json"))).unwrap();
    let catalog: WeaponProfileCatalog =
        serde_json::from_value(json(data().join("weapon-profiles/catalog.json"))).unwrap();
    let bases: ItemBasePolicy =
        serde_json::from_value(json(data().join("item-bases/policy.json"))).unwrap();
    let mut scratch = compiled.new_scratch();
    let mut evaluated = 0;
    for binding in &bases.templates {
        let owner = SchemaSubject::Definition(binding.template.address());
        let row = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        assert!(!row.programs.is_complete());
        let program = row
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "raw-base-attack-profile");
        let source = catalog
            .profiles
            .iter()
            .find(|r| r.base == binding.source_base);
        let Some(source) = source else {
            assert!(program.is_none());
            continue;
        };
        let program = program.unwrap();
        let result = compiled
            .evaluate(&owner, &program.id, &[], recipe.schema(), &mut scratch)
            .unwrap();
        for field in &policy.fields {
            let actual = result.effects.iter().find_map(|effect| {
                match (&effect.effect, &effect.disposition) {
                    (RuleEffectKind::Derive { stat, .. }, EffectDisposition::Applied { value })
                        if stat == &field.stat =>
                    {
                        Some(value)
                    }
                    _ => None,
                }
            });
            let expected = source
                .fields
                .get(&field.source_field)
                .map(|value| {
                    ParameterValue::Quantity(
                        poe_optimizer_core::owned_definitions::FiniteQuantity::new(
                            *value,
                            field.unit.clone(),
                        )
                        .unwrap(),
                    )
                })
                .or_else(|| match &field.when_absent {
                    WeaponFieldAbsence::Literal(value) => Some(value.clone()),
                    _ => None,
                });
            assert_eq!(
                actual,
                expected.as_ref(),
                "{} {}",
                source.base,
                field.source_field
            );
            if let Some(presence) = &field.presence {
                assert!(result.effects.iter().any(|effect| matches!((&effect.effect, &effect.disposition),
                    (RuleEffectKind::Capability {capability,..}, EffectDisposition::Applied {value:ParameterValue::Boolean(value)})
                    if capability == presence && *value == source.fields.contains_key(&field.source_field))));
            }
        }
        evaluated += 1;
    }
    assert_eq!(evaluated, 337);
    for case in 1..=5 {
        let draft = cwd.join(format!("original-{case}"));
        let status = success(normalize(cwd, &output, case, &draft, true));
        assert_eq!(status["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(draft.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        assert!(
            draft
                .input()
                .items
                .members
                .iter()
                .all(|item| item.to_resolved().is_none())
        );
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&query)).unwrap(), before[&query]);
    }
    let rerun = cwd.join("rerun");
    let report = success(run(cwd, &output, &rerun));
    assert_eq!(report["weapon_profiles"]["changed_program_owners"], 0);
    for (name, bytes) in bundle(&output) {
        if !["transition.json", "catalog-append.json"].contains(&name.as_str()) {
            assert_eq!(bytes, fs::read(rerun.join(&name)).unwrap(), "{name}");
        }
    }
    assert_eq!(bundle(&prior), before);
    let occupied = cwd.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"caller").unwrap();
    assert!(!run(cwd, &output, &occupied).status.success());
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"caller");
}
#[cfg(feature = "pob")]
#[test]
fn optional_export_reproduces_both_catalogs_without_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        command(temp.path(), "export-owned-weapon-profiles")
            .arg("--source-root")
            .arg(support::root().join("vendor/path-of-building-poe2"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    assert_eq!(success(call())["profiles"], 337);
    for (actual, expected) in [
        ("catalog.json", "weapon-profiles/catalog.json"),
        ("base-catalog.json", "item-bases/catalog.json"),
    ] {
        assert_eq!(
            fs::read(output.join(actual)).unwrap(),
            fs::read(data().join(expected)).unwrap()
        );
    }
    let before = bundle(&output);
    assert!(!call().status.success());
    assert_eq!(bundle(&output), before);
}
