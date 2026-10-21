//! Native execution of the release-authored incoming critical component.
//! Complete fixture dependencies are not evidence of complete real builds.
#[path = "support/owned_incoming_critical_native.rs"]
mod fixture;

use fixture::{Case, World, assert_outputs, output, read};
use poe_optimizer_core::owned_rules::*;
use poe_optimizer_core::owned_schema::RuleEntityKind;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_normalize::ConfigurationNumericInput, owned_recipe_extension::OwnedRecipeExtension,
};
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceVector {
    name: String,
    mode: String,
    category: String,
    raw: [Option<f64>; 2],
    flags: [Option<bool>; 3],
    dependencies: [Option<f64>; 6],
    outputs: [Option<f64>; 3],
}

#[test]
fn authored_packet_keeps_configuration_and_unresolved_calculation_domains_distinct() {
    let extension: OwnedRecipeExtension = read("extension.json");
    let inputs: Value = read("native-inputs.json");
    let policy: Vec<ConfigurationNumericInput> = read("policy.json");
    let queries: Vec<ContributionQuery> = read("queries.json");
    assert_eq!(extension.schema.len(), 19);
    assert_eq!(extension.owners.len(), 2);
    assert_eq!(queries.len(), 3);
    assert_eq!(policy.len(), 2);
    for (row, metadata) in policy.iter().zip(inputs["inputs"].as_array().unwrap()) {
        assert_eq!(
            serde_json::to_value(&row.presence_input).unwrap(),
            metadata["presence_input"]
        );
        assert_eq!(
            serde_json::to_value(&row.value_input).unwrap(),
            metadata["value_input"]
        );
        assert_eq!(row.source_name, metadata["source_name"]);
    }
    assert!(
        extension
            .owners
            .iter()
            .all(|owner| !owner.programs.is_complete())
    );
    assert!(extension.receivers.is_empty());
    for query in &queries {
        assert_eq!(query.contribution, ContributionKind::Flag);
        assert_eq!(query.groups.len(), 1);
        let group = &query.groups[0];
        assert_eq!(group.reduction, ContributionReduction::Any);
        assert_eq!(group.ordering, ContributionOrdering::Unordered);
        assert!(!group.members.is_complete());
        assert!(group.members.members.is_empty());
    }
    let formula = &extension.owners[1].programs.members[0];
    assert_eq!(formula.context, RuleEntityKind::Actor);
    assert_eq!(
        formula
            .reads
            .iter()
            .filter(|r| matches!(r.source, RuleReadSource::ContributionQuery { .. }))
            .count(),
        3
    );
    assert!(
        !formula
            .reads
            .iter()
            .any(|r| matches!(r.source, RuleReadSource::Contributions { .. }))
    );
    assert!(
        formula
            .effects
            .iter()
            .all(|e| e.when.as_ref().unwrap().as_str() == "is-hit")
    );
}

#[test]
fn explicit_zero_defaults_clamps_and_negative_bonuses_use_actual_authored_program() {
    for (case, expected) in [
        (Case::default(), [5., 30., 1.015]),
        (Case::default().raw(Some(0.), Some(0.)), [0., 0., 1.]),
        (Case::default().raw(Some(-10.), Some(-20.)), [0., 0., 1.]),
        (
            Case::default().raw(Some(250.), Some(400.)),
            [100., 400., 5.],
        ),
        (
            Case {
                dependencies: [
                    Some(50.),
                    Some(25.),
                    Some(20.),
                    Some(20.),
                    Some(50.),
                    Some(25.),
                ],
                ..Case::default().raw(Some(10.), Some(40.))
            },
            [14., 90., 1.0945],
        ),
        (
            Case {
                dependencies: [
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(0.),
                    Some(-200.),
                    Some(0.),
                ],
                ..Case::default().raw(Some(5.), Some(-20.))
            },
            [5., 20., 1.01],
        ),
        (
            Case {
                dependencies: [Some(0.), Some(0.), Some(0.), Some(0.), Some(0.), Some(-50.)],
                ..Case::default()
            },
            [5., 30., 1.0225],
        ),
        (
            Case {
                raw: [Some(999.), Some(999.)],
                ..Case::default()
            },
            [5., 30., 1.015],
        ),
    ] {
        assert_outputs(&World::new(&case, true).evaluate(), expected);
    }
}

#[test]
fn never_precedes_always_and_unlucky_runs_after_the_selected_chance_branch() {
    for (flags, expected) in [
        ([Some(true), Some(true), Some(false)], [0., 30., 1.]),
        ([Some(true), Some(true), Some(true)], [0., 30., 1.]),
        ([Some(false), Some(true), Some(true)], [100., 30., 1.3]),
    ] {
        let case = Case {
            flags,
            dependencies: [None, None, None, Some(0.), Some(0.), Some(0.)],
            ..Case::default()
        };
        assert_outputs(&World::new(&case, true).evaluate(), expected);
    }
    let unlucky = Case {
        flags: [Some(false), Some(false), Some(true)],
        ..Case::default().raw(Some(20.), Some(100.))
    };
    assert_outputs(&World::new(&unlucky, true).evaluate(), [4., 100., 1.04]);
}

#[test]
fn dot_does_not_demand_configuration_or_numeric_hit_dependencies() {
    let case = Case {
        category: 0x3254,
        presence: [None; 2],
        flags: [None; 3],
        dependencies: [None; 6],
        ..Case::default()
    };
    let report = World::new(&case, true).evaluate();
    assert!(report.gaps.is_empty(), "{report:?}");
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Evaluated { .. }
    ));
    for stat in 0x3362..=0x3364 {
        assert_eq!(output(&report, stat), Some(&EffectValue::Inactive));
    }
}

#[test]
fn actual_partial_flag_domain_and_missing_resolved_inputs_are_never_zero_defaults() {
    for flags in [[Some(false); 3], [Some(true); 3]] {
        let report = World::new(
            &Case {
                flags,
                ..Case::default()
            },
            false,
        )
        .evaluate();
        assert!(matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ));
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors)
        );
    }
    for dependency in 0..6 {
        let mut case = Case::default();
        case.dependencies[dependency] = None;
        let report = World::new(&case, true).evaluate();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::MissingInput),
            "dependency {dependency}: {report:?}"
        );
        assert!(!matches!(
            output(&report, 0x3364),
            Some(EffectValue::Known { .. })
        ));
    }
    for field in 0..2 {
        let mut case = Case::default();
        case.presence[field] = Some(true);
        let report = World::new(&case, true).evaluate();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::MissingInput),
            "raw field {field}: {report:?}"
        );
    }
}

#[test]
fn actual_program_is_repeatable_with_reused_scratch_and_parallel_plans() {
    let mut cases = vec![
        Case::default(),
        Case::default().raw(Some(20.), Some(100.)),
        Case::default().raw(Some(0.), Some(0.)),
    ];
    cases.push(Case {
        flags: [Some(false), Some(false), Some(true)],
        ..cases[1].clone()
    });
    cases.push(Case::default());
    let plans: Vec<_> = cases
        .iter()
        .map(|case| World::new(case, true).plan())
        .collect();
    let serial: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let parallel: Vec<_> = plans
        .par_iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(serial, parallel);
    let mut scratch = plans[0].new_scratch();
    for (plan, expected) in plans.iter().zip(&serial) {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
    }
    assert_eq!(serial.first(), serial.last());
    for (report, expected) in serial.iter().zip([
        [5., 30., 1.015],
        [20., 100., 1.2],
        [0., 0., 1.],
        [4., 100., 1.04],
        [5., 30., 1.015],
    ]) {
        assert_outputs(report, expected);
    }
}

#[test]
fn measured_source_vectors_replay_without_filling_unobserved_query_values() {
    let evidence: Value = read("source-vectors.json");
    assert_eq!(evidence["status"], "passed");
    let vectors: Vec<SourceVector> = serde_json::from_value(evidence["vectors"].clone()).unwrap();
    assert!(!vectors.is_empty());
    let mut replays = Vec::new();
    for vector in &vectors {
        let name = &vector.name;
        let mode = vector.mode.as_str();
        assert!(matches!(mode, "MAIN" | "CALCS" | "main" | "calcs"));
        let category = match vector.category.as_str() {
            "Average" => 0x3252,
            "Untyped" => 0x3253,
            "DamageOverTime" => 0x3254,
            "Melee" => 0x3255,
            "Projectile" => 0x3256,
            "Spell" => 0x3257,
            "SpellProjectile" => 0x3258,
            unknown => panic!("unrepresented category {unknown}"),
        };
        let mut case = Case {
            category,
            flags: vector.flags,
            dependencies: vector.dependencies,
            ..Case::default().raw(vector.raw[0], vector.raw[1])
        };
        if category == 0x3254 {
            assert!(vector.outputs.iter().all(Option::is_none));
            case.presence = [None; 2];
            case.raw = [None; 2];
            case.dependencies = [None; 6];
            let report = World::new(&case, true).evaluate();
            assert!(report.gaps.is_empty(), "{name}/{mode}: {report:?}");
            for stat in 0x3362..=0x3364 {
                assert_eq!(output(&report, stat), Some(&EffectValue::Inactive));
            }
            continue;
        }
        let expected = vector.outputs.map(Option::unwrap);
        assert!(
            case.flags[0].is_some() && case.flags[2].is_some(),
            "{name}/{mode}"
        );
        if case.flags[1].is_none() {
            assert_eq!(case.flags[0], Some(true), "only NeverCrit skips AlwaysCrit");
            // Neither value is called a measurement: these are both possible
            // finite completions of the unobserved input. The authored branch
            // must return the measured result for either completion.
            for completion in [false, true] {
                let mut completed = case.clone();
                completed.flags[1] = Some(completion);
                replays.push((
                    format!("{name}/{mode}/always-{completion}"),
                    World::new(&completed, true).plan(),
                    expected,
                ));
            }
        } else {
            replays.push((
                format!("{name}/{mode}"),
                World::new(&case, true).plan(),
                expected,
            ));
        }
    }
    assert!(
        replays
            .iter()
            .any(|(name, _, _)| name.starts_with("original-05/"))
    );
    let fresh: Vec<_> = replays
        .iter()
        .map(|(_, plan, _)| plan.evaluate(&mut plan.new_scratch()).unwrap())
        .collect();
    let parallel: Vec<_> = replays
        .par_iter()
        .map(|(_, plan, _)| plan.evaluate(&mut plan.new_scratch()).unwrap())
        .collect();
    assert_eq!(fresh, parallel);
    let mut scratch = replays[0].1.new_scratch();
    for ((name, plan, expected), report) in replays.iter().zip(&fresh) {
        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), report, "{name}");
        assert_outputs(report, *expected);
    }
}
