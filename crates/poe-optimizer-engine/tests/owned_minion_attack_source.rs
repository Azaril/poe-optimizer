//! Intrinsic profile weapon data only, before action modifiers or final metrics.
#[path = "support/minion_attack_source_fixture.rs"]
mod fixture;
#[path = "../../../tests/support/minion_attack_source_vectors.rs"]
mod source_vectors;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::ActionOutputRoutes, owned_rules::*,
};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

#[test]
fn injected_intrinsic_sources_follow_dynamic_parent_levels_and_exact_actor_routes() {
    let w = World::new([22, 1]);
    let report = w.evaluate();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (index, level) in [(0, 44), (1, 2)] {
        assert_eq!(
            actor_value(&report, index, 0x1c),
            &EffectValue::Known {
                value: ParameterValue::Integer(BoundedInteger::new(level).unwrap())
            }
        );
        for (source, target) in [
            (0x320f, 0x3212),
            (0x3210, 0x3213),
            (0x3211, 0x3214),
            (0x2539, 0x3215),
        ] {
            assert_eq!(
                actor_value(&report, index, source),
                action_value(&report, index, target)
            );
        }
        assert_eq!(known(actor_value(&report, index, 0x3211)), 1. / 1.5);
        assert_eq!(known(action_value(&report, index, 0x3215)), 5.);
    }
    assert_ne!(
        actor_value(&report, 0, 0x320f),
        actor_value(&report, 1, 0x320f)
    );
    let changed = World::new([1, 22]).evaluate();
    assert_eq!(
        actor_value(&report, 0, 0x320f),
        actor_value(&changed, 1, 0x320f)
    );
    assert_eq!(
        actor_value(&report, 1, 0x3210),
        actor_value(&changed, 0, 0x3210)
    );
}

#[test]
fn injected_profile_branch_rounding_and_reciprocal_are_independent() {
    let mut w = World::new([22, 1]);
    // Deliberate component counterfactual: floor the curve before profile scaling,
    // then floor each spread endpoint. This is not a new admitted source profile.
    w.baseline_mut()
        .nodes
        .iter_mut()
        .find(|n| n.id.as_str() == "curve-0")
        .unwrap()
        .expression = RuleExpression::Literal {
        value: quantity(10.99, 0x1d3a),
    };
    w.set_baseline("fact-0", quantity(1.5, 5));
    w.set_baseline("fact-1", quantity(1.15, 1));
    w.set_baseline("fact-2", quantity(0.3, 1));
    for (ignores, min, max) in [(true, 8., 14.), (false, 12., 22.)] {
        w.set_baseline("fact-5", ParameterValue::Boolean(ignores));
        let r = w.evaluate();
        assert_eq!(known(actor_value(&r, 0, 0x320f)), min);
        assert_eq!(known(actor_value(&r, 0, 0x3210)), max);
        assert_eq!(known(actor_value(&r, 0, 0x3211)), 1. / 1.5);
    }
}

#[test]
fn missing_finalization_and_missing_baseline_never_default_to_original_build_numbers() {
    let mut w = World::new([22, 1]);
    w.missing_final_input();
    let r = w.evaluate();
    assert!(matches!(
        actor_value(&r, 0, 0x320f),
        EffectValue::Unresolved { .. }
    ));
    assert!(matches!(
        action_value(&r, 0, 0x3212),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new([22, 1]);
    w.baseline_mut()
        .effects
        .retain(|e| e.id.as_str() != "fact-1");
    let r = w.evaluate();
    assert!(matches!(
        actor_value(&r, 0, 0x320f),
        EffectValue::Unresolved { .. }
    ));
    // The rate does not read the missing damage scale.
    assert_eq!(known(actor_value(&r, 0, 0x3211)), 1. / 1.5);
}

#[test]
fn immutable_plan_supports_reused_scratch_and_parallel_occurrences() {
    let plan = Arc::new(World::new([22, 1]).compile());
    let mut scratch = plan.new_scratch();
    let first = plan.evaluate(&mut scratch).unwrap();
    assert_eq!(known(actor_value(&first, 0, 0x320f)), 208.);
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    let other = World::new([1, 22]).compile();
    other.evaluate(&mut scratch).unwrap();
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let p = Arc::clone(&plan);
            std::thread::spawn(move || {
                let mut scratch = p.new_scratch();
                let initial = p.evaluate(&mut scratch).unwrap();
                for _ in 0..8 {
                    assert_eq!(initial, p.evaluate(&mut scratch).unwrap());
                }
                initial
            })
        })
        .collect();
    for thread in threads {
        assert_eq!(first, thread.join().unwrap());
    }
}

#[test]
fn published_component_preserves_partial_owners_and_route_coverage() {
    let extension: Value = asset("extension.json");
    let routes: Value = asset("routes.json");
    assert_eq!(
        extension["owners"][0]["programs"]["closure"]["kind"],
        "partial"
    );
    assert_eq!(routes[0]["routes"]["closure"]["kind"], "partial");
    let mut w = World::new([22, 1]);
    let partial: DefinitionRules = serde_json::from_value(extension["owners"][0].clone()).unwrap();
    w.f.owner_mut(&partial.owner).programs.closure = partial.programs.closure;
    let r = w.evaluate();
    assert!(
        !r.gaps.is_empty(),
        "component numbers cannot close original behavior"
    );
}

#[test]
fn finite_domains_and_invalid_reciprocal_remain_explicit() {
    let r = World::new([41, 1]).evaluate();
    assert!(r.values.iter().any(|v| {
        matches!(&v.key, PlanValueKey::SkillParameter { parameter, .. } if parameter.slot == def(0x13))
            && matches!(&v.value, EffectValue::UnsupportedValue { value: ParameterValue::Integer(n) } if n.get() == 41)
    }), "out-of-range parent input remains explicit");
    assert!(
        matches!(
            actor_value(&r, 0, 0x320f),
            EffectValue::Unresolved {
                reason: PlanGapReason::UpstreamUnavailable,
                ..
            }
        ),
        "invalid parent level: {:?}; gaps: {:?}",
        actor_value(&r, 0, 0x320f),
        r.gaps
    );
    assert!(matches!(
        actor_value(&r, 1, 0x320f),
        EffectValue::Known { .. }
    ));
    let mut w = World::new([22, 1]);
    w.set_baseline("fact-0", quantity(0., 5));
    let r = w.evaluate();
    assert!(matches!(
        actor_value(&r, 0, 0x3211),
        EffectValue::NumericalError { .. }
    ));
    // Ignoring attack speed leaves the finite damage stage independent of rate.
    assert_eq!(known(actor_value(&r, 0, 0x320f)), 208.);
}

fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// Authenticate the exact published prerequisites before closing the finite
/// component world. This catches copied fixture drift, not just changed outputs.
fn published_snapshot(package: &std::path::Path) {
    let snapshot: Value =
        serde_json::from_str(include_str!("support/minion_attack_source_snapshot.json")).unwrap();
    let schema = read(package.join("schema.json"));
    for (old, published) in [
        ("schema_definitions", "definitions"),
        ("schema_slots", "slots"),
    ] {
        for row in snapshot[old].as_array().unwrap() {
            assert!(
                schema[published].as_array().unwrap().contains(row),
                "published prerequisite {}",
                row["value"]["id"]
            );
        }
    }
    let rules = read(package.join("rules.json"));
    let extension: Value = asset("extension.json");
    // Quantity literals accept JSON integer syntax but publish as f64. Compare
    // canonical typed programs so equivalent 1 and 1.0 authoring is not drift.
    let extension_owners: Vec<DefinitionRules> =
        serde_json::from_value(extension["owners"].clone()).unwrap();
    let extension_owners = serde_json::to_value(extension_owners).unwrap();
    for owner in snapshot["owners"].as_array().unwrap() {
        let mut expected = owner.clone();
        for added in extension_owners.as_array().unwrap() {
            if added["owner"] == owner["owner"] {
                assert_eq!(added["programs"]["closure"], owner["programs"]["closure"]);
                expected["programs"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .extend(
                        added["programs"]["members"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .cloned(),
                    );
            }
        }
        assert!(
            rules["owners"].as_array().unwrap().contains(&expected),
            "unchanged actual prerequisite owner {}",
            owner["owner"]
        );
    }
    for table in snapshot["tables"].as_array().unwrap() {
        assert!(
            rules["tables"].as_array().unwrap().contains(table),
            "actual table {}",
            table["id"]
        );
    }
    for row in extension["schema"].as_array().unwrap() {
        assert!(
            schema["definitions"]
                .as_array()
                .unwrap()
                .contains(&row["value"])
        );
    }
    let routing = read(package.join("routing.json"));
    let published_routes: Vec<ActionOutputRoutes> =
        serde_json::from_value(routing["outputs"].clone()).unwrap();
    let mut routes: Vec<ActionOutputRoutes> = asset("routes.json");
    for output in &mut routes {
        output.routes.members.sort_by(|a, b| a.id.cmp(&b.id));
    }
    for row in &routes {
        assert!(
            published_routes.contains(row),
            "exact published action routes"
        );
    }
    let mapping = read(package.join("mapping.json"));
    assert!(mapping["entries"].as_array().unwrap().iter().any(
        |row| row["source"]["value"]["value"]["effect_id"]["value"] == "MinionMeleeBow"
            && row["outcome"]["value"]["target"]["value"]["value"]["key"] == "def.0000000000000021"
    ));
}

#[test]
#[ignore = "requires published component and authenticated complete-source witness in both JIT modes"]
fn actual_fresh_actor_weapons_and_basic_attack_consumers_match_published_native_programs() {
    let package = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ATTACK_SOURCE_PACKAGE")
            .expect("published package"),
    );
    published_snapshot(&package);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let a: Value = asset("authoring.json");
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let bytes = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        bytes,
        fs::read(root.join(proof["evidence_on_json"].as_str().unwrap())).unwrap()
    );
    assert_eq!(
        bytes.len() as u64,
        proof["evidence_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(source["source_revision"], a["source_revision"]);
    assert_eq!(source["source_hash"], a["source_manifest_sha256"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert!(
            source["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|actual| actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"])
        );
    }
    assert_eq!(source_vectors::project(&source), golden());
    let mut actors = 0;
    let mut consumers = 0;
    let mut alternate_profiles = 0;
    for case in source["cases"].as_array().unwrap() {
        assert_eq!(case["available"], true);
        for flag in [
            "fresh_actor_construction",
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
        ] {
            assert_eq!(case["state"][flag], true);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        for mode in ["main", "calcs"] {
            let rows = case["state"][mode]["actors"].as_array().unwrap();
            let snipers: Vec<_> = rows
                .iter()
                .filter(|row| row["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
                .collect();
            assert!(snipers.len() <= 2);
            if !snipers.is_empty() {
                let mut levels = [1, 1];
                for (index, row) in snipers.iter().enumerate() {
                    levels[index] =
                        u16::try_from(row["effective_level"].as_u64().unwrap()).unwrap();
                }
                let report = World::new(levels).evaluate();
                for (index, row) in snipers.into_iter().enumerate() {
                    assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
                    assert_eq!(row["curve"]["kind"], "allied");
                    for excluded in [
                        "uses_weapon1",
                        "minion_use_bow_and_quiver",
                        "iron_mass",
                        "weapon1_from_item",
                        "alternate_level",
                    ] {
                        assert_eq!(row["policy"][excluded], false);
                    }
                    assert_eq!(
                        actor_value(&report, index, 0x1c),
                        &EffectValue::Known {
                            value: ParameterValue::Integer(
                                BoundedInteger::new(row["actor_level"].as_i64().unwrap()).unwrap()
                            )
                        }
                    );
                    for (stat, field) in [
                        (0x320f, "PhysicalMin"),
                        (0x3210, "PhysicalMax"),
                        (0x3211, "AttackRate"),
                        (0x2539, "CritChance"),
                    ] {
                        assert_eq!(
                            known(actor_value(&report, index, stat)),
                            row["weapon1"][field].as_f64().unwrap(),
                            "{} {mode} {field}",
                            case["name"]
                        );
                    }
                    let child = row["children"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|c| c["effect_id"] == "MinionMeleeBow")
                        .unwrap();
                    if let Some(passes) = child["consumer"]["passes"].as_array() {
                        assert_eq!(passes.len(), 1);
                        for (stat, field) in [
                            (0x3212, "PhysicalMin"),
                            (0x3213, "PhysicalMax"),
                            (0x3214, "AttackRate"),
                            (0x3215, "CritChance"),
                        ] {
                            assert_eq!(
                                known(action_value(&report, index, stat)),
                                passes[0]["source"][field].as_f64().unwrap()
                            );
                        }
                        consumers += 1;
                    }
                    actors += 1;
                }
            }
            // Real source counterexample for the other arithmetic branch. This
            // does not bind Spectres to the release's allied Sniper population.
            for row in rows.iter().filter(|row| {
                row["actor_profile"] == "Metadata/Monsters/LeagueAbyss/Blackblood/CollectorSpectre"
            }) {
                assert_eq!(row["curve"]["kind"], "hostile");
                assert_eq!(row["profile"]["ignore_attack_speed"], false);
                let mut w = World::new([22, 1]);
                for (node, field, unit) in [
                    ("fact-0", "attack_time", 5),
                    ("fact-1", "damage_scale", 1),
                    ("fact-2", "damage_spread", 1),
                    ("fact-3", "crit_chance", 2),
                ] {
                    w.set_baseline(
                        node,
                        quantity(row["profile"][field].as_f64().unwrap(), unit),
                    );
                }
                w.set_baseline("fact-5", ParameterValue::Boolean(false));
                w.baseline_mut()
                    .nodes
                    .iter_mut()
                    .find(|n| n.id.as_str() == "curve-0")
                    .unwrap()
                    .expression = RuleExpression::Literal {
                    value: quantity(row["curve"]["raw_value"].as_f64().unwrap(), 0x1d3a),
                };
                let report = w.evaluate();
                for (stat, field) in [
                    (0x320f, "PhysicalMin"),
                    (0x3210, "PhysicalMax"),
                    (0x3211, "AttackRate"),
                    (0x2539, "CritChance"),
                ] {
                    assert_eq!(
                        known(actor_value(&report, 0, stat)),
                        row["weapon1"][field].as_f64().unwrap()
                    );
                }
                alternate_profiles += 1;
            }
        }
    }
    assert_eq!(actors, 18);
    assert_eq!(consumers, 8);
    assert_eq!(alternate_profiles, 4);
}

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/calibration/intrinsic-minion-attack-3887ae68.json"
    ))
    .unwrap()
}

#[test]
fn committed_fresh_source_vectors_match_native_arithmetic_in_default_ci() {
    let vectors = golden();
    let a: Value = asset("authoring.json");
    assert_eq!(vectors["source_revision"], a["source_revision"]);
    assert_eq!(vectors["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(vectors["sniper"].as_array().unwrap().len(), 5);
    for row in vectors["sniper"].as_array().unwrap() {
        let level = u16::try_from(row["effective_level"].as_u64().unwrap()).unwrap();
        let report = World::new([level, 1]).evaluate();
        assert_eq!(
            actor_value(&report, 0, 0x1c),
            &EffectValue::Known {
                value: ParameterValue::Integer(
                    BoundedInteger::new(row["actor_level"].as_i64().unwrap()).unwrap()
                )
            }
        );
        let children = row["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["effect_id"], "MinionMeleeBow");
        let passes = children[0]["consumer"]["passes"].as_array().unwrap();
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0]["copied_from_actor"], true);
        for (actor_stat, action_stat, field) in [
            (0x320f, 0x3212, "PhysicalMin"),
            (0x3210, 0x3213, "PhysicalMax"),
            (0x3211, 0x3214, "AttackRate"),
            (0x2539, 0x3215, "CritChance"),
        ] {
            assert_eq!(
                known(actor_value(&report, 0, actor_stat)),
                row["weapon1"][field].as_f64().unwrap()
            );
            assert_eq!(
                known(action_value(&report, 0, action_stat)),
                passes[0]["source"][field].as_f64().unwrap()
            );
        }
    }
    assert_eq!(
        vectors["spectre_counterexamples"].as_array().unwrap().len(),
        2
    );
    for row in vectors["spectre_counterexamples"].as_array().unwrap() {
        assert_eq!(row["profile"]["ignore_attack_speed"], false);
        assert_eq!(row["curve"]["kind"], "hostile");
        let mut w = World::new([22, 1]);
        for (node, field, unit) in [
            ("fact-0", "attack_time", 5),
            ("fact-1", "damage_scale", 1),
            ("fact-2", "damage_spread", 1),
            ("fact-3", "crit_chance", 2),
        ] {
            w.set_baseline(
                node,
                quantity(row["profile"][field].as_f64().unwrap(), unit),
            );
        }
        w.set_baseline("fact-5", ParameterValue::Boolean(false));
        w.baseline_mut()
            .nodes
            .iter_mut()
            .find(|n| n.id.as_str() == "curve-0")
            .unwrap()
            .expression = RuleExpression::Literal {
            value: quantity(row["curve"]["raw_value"].as_f64().unwrap(), 0x1d3a),
        };
        let report = w.evaluate();
        for (stat, field) in [
            (0x320f, "PhysicalMin"),
            (0x3210, "PhysicalMax"),
            (0x3211, "AttackRate"),
            (0x2539, "CritChance"),
        ] {
            assert_eq!(
                known(actor_value(&report, 0, stat)),
                row["weapon1"][field].as_f64().unwrap()
            );
        }
    }
}
