//! Structural source preflight is deliberately distinct from semantic admission.
use super::{fixture, key};
use crate::engine_fixtures::{readiness, source_properties as sources};
use poe_optimizer_core::{
    owned_content::ContentDigestError,
    owned_readiness::{
        ParameterReadiness, ReadinessInput, ReadinessPhase, ReadinessProgram, ReadinessProgramRole,
        SkillReadiness,
    },
    owned_schema::{DeclaredSet, SchemaClosure, SchemaFacet, SchemaGap},
    owned_source_properties::{
        SourcePropertyEffect, SourcePropertyEffectEndpoint, SourcePropertyOccurrence,
        SourcePropertyPreparationInput, SourcePropertyRelation, SourcePropertySupportPrograms,
    },
    owned_support_receiving::{SupportAdmissionContext, SupportPreparationPrograms},
};
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage,
    owned_stages::{OwnedEvaluationStages, StageStorageError},
    owned_support_receiving::{OwnedSupportReceiving, SupportReceivingStorageError},
};
use poe_optimizer_import::{owned_release::*, owned_release_evaluation::OwnedReleaseSupportInput};
use std::sync::OnceLock;

fn support(input: &mut OwnedReleaseInput) -> &mut OwnedReleaseSupportInput {
    input.evaluation.as_mut().unwrap().support.as_mut().unwrap()
}

fn partial() -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![
            SchemaGap {
                subject: fixture().input.recipe.rules.owners[0].owner.clone(),
                facet: SchemaFacet::GameRules,
                code: key("unreviewed"),
            };
            3
        ],
    }
}

fn limits(entries: usize) -> OwnedReleaseLimits {
    OwnedReleaseLimits {
        max_validation_entries: entries,
        ..Default::default()
    }
}

fn baseline_entries() -> usize {
    static ENTRIES: OnceLock<usize> = OnceLock::new();
    *ENTRIES.get_or_init(|| {
        let input = &fixture().input;
        let (mut low, mut high) = (1, OwnedReleaseLimits::default().max_validation_entries);
        check_release_source_dependencies(input, limits(high)).unwrap();
        while low < high {
            let middle = low + (high - low) / 2;
            match check_release_source_dependencies(input, limits(middle)) {
                Ok(_) => high = middle,
                Err(OwnedReleaseError::Limit("validation entries")) => low = middle + 1,
                Err(error) => panic!("unexpected source validation error: {error}"),
            }
        }
        // The unchanged fixture is also a fully valid release at this boundary.
        assemble_owned_release(input.clone(), limits(low)).unwrap();
        low
    })
}

fn assert_delta(label: &str, input: &OwnedReleaseInput, additional: usize) {
    let exact = baseline_entries() + additional;
    check_release_source_dependencies(input, limits(exact))
        .unwrap_or_else(|error| panic!("{label}: exact source budget failed: {error}"));
    assert!(
        matches!(
            check_release_source_dependencies(input, limits(exact - 1)),
            Err(OwnedReleaseError::Limit("validation entries"))
        ),
        "{label}: one-short aggregate budget must fail"
    );
    // A tiny digest budget makes the ordering observable without allocating an
    // oversized fixture: preflight must fail before digesting or cloning it.
    let mut bounded = limits(exact - 1);
    bounded.max_semantic_bytes = 1;
    assert!(matches!(
        check_release_source_dependencies(input, bounded),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    bounded.max_validation_entries = exact;
    assert!(matches!(
        check_release_source_dependencies(input, bounded),
        Err(OwnedReleaseError::Digest(ContentDigestError::TooLarge {
            maximum: 1
        }))
    ));
}

fn assert_not_a_release(input: OwnedReleaseInput) {
    // Source preflight checks storage bounds, never upgrades the fixture's V1
    // metadata or repairs its seals to admit these structural probes.
    assert!(matches!(
        assemble_owned_release(input, Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
}

#[test]
fn readiness_members_outputs_participation_and_hostile_gaps_charge_exactly() {
    let skill = SkillReadiness {
        skill: sources::def("summon"),
        parameters: DeclaredSet::complete(vec![]),
        participation: None,
    };
    let program = ReadinessProgram {
        owner: fixture().input.recipe.rules.owners[0].owner.clone(),
        program: key("probe"),
        phase: ReadinessPhase::Execution,
        role: ReadinessProgramRole::Execution,
        outputs: vec![],
    };
    for case in [
        "skill",
        "parameters",
        "participation",
        "parameter gaps",
        "program",
        "outputs",
        "program gaps",
    ] {
        let mut input = fixture().input.clone();
        let s = support(&mut input);
        assert!(s.stages.readiness.is_none());
        let mut declarations = ReadinessInput {
            skills: vec![],
            programs: DeclaredSet::complete(vec![]),
        };
        let delta = match case {
            "skill" => {
                declarations.skills.push(skill.clone());
                1
            }
            "parameters" => {
                let mut row = skill.clone();
                row.parameters.members = vec![
                    ParameterReadiness {
                        parameter: readiness::summon_parameter("level"),
                        phase: ReadinessPhase::Structural,
                    };
                    3
                ];
                declarations.skills.push(row);
                4
            }
            "participation" => {
                let mut row = skill.clone();
                row.participation = Some(s.inputs.target.has_gem.clone());
                declarations.skills.push(row);
                2
            }
            "parameter gaps" => {
                let mut row = skill.clone();
                row.parameters.closure = partial();
                declarations.skills.push(row);
                4
            }
            "program" => {
                declarations.programs.members.push(program.clone());
                1
            }
            "outputs" => {
                let mut row = program.clone();
                row.outputs = vec![s.stages.frozen_channels[0].channel.clone(); 3];
                declarations.programs.members.push(row);
                4
            }
            "program gaps" => {
                declarations.programs.closure = partial();
                3
            }
            _ => unreachable!(),
        };
        s.stages.readiness = Some(declarations);
        assert_delta(case, &input, delta);
        assert_not_a_release(input);
    }
}

#[test]
fn receiver_preparation_charges_applicability_and_each_property_occurrence() {
    for count in [0, 1, 3] {
        let mut input = fixture().input.clone();
        let row = &mut support(&mut input).receiving.supports[0].receivers.members[0];
        assert!(row.preparation.is_none());
        row.preparation = Some(SupportPreparationPrograms {
            applicability: key("preparation-applicability"),
            properties: vec![key("property"); count],
        });
        assert_delta("receiver preparation", &input, 1 + count);
        assert_not_a_release(input);
    }
}

fn empty_relation() -> SourcePropertyRelation {
    let mut row = sources::relation();
    row.effects.members.clear();
    row.inputs.clear();
    row.channels.members.clear();
    row.external.members.clear();
    row.supports.members.clear();
    row.assembly.members.clear();
    row
}

fn source(input: &mut OwnedReleaseInput) -> &mut SourcePropertyPreparationInput {
    support(input).receiving.source_properties.as_mut().unwrap()
}

fn source_probe() -> OwnedReleaseInput {
    let mut input = fixture().input.clone();
    let s = support(&mut input);
    assert!(s.receiving.source_properties.is_none());
    s.receiving.source_properties = Some(SourcePropertyPreparationInput {
        relations: DeclaredSet::complete(vec![empty_relation()]),
    });
    input
}

#[test]
fn source_property_members_supply_references_and_hostile_gaps_charge_exactly() {
    for case in [
        "relation",
        "repeated relations",
        "relation gaps",
        "generated occurrence",
        "effects",
        "inputs",
        "channels",
        "external",
        "supports",
        "assembly",
        "support programs",
        "support program gaps",
        "effect gaps",
        "channel gaps",
        "external gaps",
        "support gaps",
        "assembly gaps",
        "generated endpoint",
        "summoner path",
    ] {
        let mut input = source_probe();
        let source = source(&mut input);
        let row = &mut source.relations.members[0];
        let template = sources::relation();
        let mut support_programs = SourcePropertySupportPrograms {
            gem: sources::def("support"),
            counted: true,
            programs: DeclaredSet::complete(vec![]),
        };
        let delta = match case {
            "relation" => 1,
            "repeated relations" => {
                source.relations.members.push(empty_relation());
                2
            }
            "relation gaps" => {
                source.relations.closure = partial();
                4
            }
            "generated occurrence" => {
                row.occurrence = SourcePropertyOccurrence::GeneratedSkill {
                    skill_supply: readiness::summon_supply(),
                };
                2
            }
            "effects" => {
                row.effects.members = vec![
                    SourcePropertyEffect {
                        endpoint: SourcePropertyEffectEndpoint::OwnerSkill {},
                        admission: SupportAdmissionContext::AssignedSkill,
                    };
                    3
                ];
                4
            }
            "inputs" => {
                row.inputs = vec![template.inputs[0].clone(); 3];
                4
            }
            "channels" => {
                row.channels.members = vec![template.channels.members[0].clone(); 3];
                4
            }
            "external" => {
                row.external.members = vec![template.external.members[0].clone(); 3];
                4
            }
            "supports" => {
                row.supports.members = vec![support_programs; 3];
                4
            }
            "assembly" => {
                row.assembly.members = vec![template.assembly.members[0].clone(); 3];
                4
            }
            "support programs" => {
                support_programs.programs.members = vec![key("property"); 3];
                row.supports.members.push(support_programs);
                5
            }
            "support program gaps" => {
                support_programs.programs.closure = partial();
                row.supports.members.push(support_programs);
                5
            }
            "effect gaps" => {
                row.effects.closure = partial();
                4
            }
            "channel gaps" => {
                row.channels.closure = partial();
                4
            }
            "external gaps" => {
                row.external.closure = partial();
                4
            }
            "support gaps" => {
                row.supports.closure = partial();
                4
            }
            "assembly gaps" => {
                row.assembly.closure = partial();
                4
            }
            "generated endpoint" => {
                row.effects.members.push(SourcePropertyEffect {
                    endpoint: SourcePropertyEffectEndpoint::Generated {
                        path: vec![readiness::summon_grant(); 3],
                        skill_supply: readiness::summon_supply(),
                    },
                    admission: SupportAdmissionContext::AssignedSkill,
                });
                6 // Relation, effect, three path steps, supply reference.
            }
            "summoner path" => {
                row.effects.members.push(SourcePropertyEffect {
                    endpoint: SourcePropertyEffectEndpoint::OwnerSkill {},
                    admission: SupportAdmissionContext::ReceivingSkill {
                        summoner_path: Some(vec![readiness::summon_grant(); 3]),
                    },
                });
                5 // Relation, effect, three summoner path steps.
            }
            _ => unreachable!(),
        };
        assert_delta(case, &input, delta);
        assert_not_a_release(input);
    }
}

#[test]
fn source_effect_and_summoner_depths_are_bounded_before_digest_or_clone() {
    for summoner in [false, true] {
        let mut input = source_probe();
        let row = &mut source(&mut input).relations.members[0];
        let path = vec![readiness::summon_grant(); 3];
        row.effects.members.push(if summoner {
            SourcePropertyEffect {
                endpoint: SourcePropertyEffectEndpoint::OwnerSkill {},
                admission: SupportAdmissionContext::ReceivingSkill {
                    summoner_path: Some(path),
                },
            }
        } else {
            SourcePropertyEffect {
                endpoint: SourcePropertyEffectEndpoint::Generated {
                    path,
                    skill_supply: readiness::summon_supply(),
                },
                admission: SupportAdmissionContext::AssignedSkill,
            }
        });
        let mut bounded = limits(baseline_entries() + if summoner { 5 } else { 6 });
        bounded.evaluation.receiving.max_path_depth = 3;
        check_release_source_dependencies(&input, bounded).unwrap();
        bounded.evaluation.receiving.max_path_depth = 2;
        bounded.max_semantic_bytes = 1;
        let expected = if summoner {
            "source summoner path depth"
        } else {
            "source effect path depth"
        };
        assert!(matches!(
            check_release_source_dependencies(&input, bounded),
            Err(OwnedReleaseError::Limit(reason)) if reason == expected
        ));
        assert!(matches!(
            assemble_owned_release(input.clone(), bounded),
            Err(OwnedReleaseError::Limit(reason)) if reason == expected
        ));
        assert_not_a_release(input);
    }
}

#[test]
fn complete_readiness_and_source_authority_still_require_data_validation() {
    let fixture = sources::fixture();
    let mut raw_rules = None;
    let checked = sources::inputs_with(
        &fixture,
        |rules| raw_rules = Some(rules.clone()),
        |_| {},
        |_| {},
    )
    .unwrap();
    let rules = OwnedRulePackage::new(
        raw_rules.unwrap(),
        checked.definitions.as_ref(),
        Default::default(),
    )
    .unwrap();
    for parameters in [false, true] {
        let mut input = checked.stages.input().clone();
        let readiness = input.readiness.as_mut().unwrap();
        if parameters {
            readiness.skills[0].parameters.closure = partial();
        } else {
            readiness.programs.closure = partial();
        }
        let expected = if parameters {
            "readiness requires complete required-input inventory"
        } else {
            "readiness requires complete program classification"
        };
        assert!(matches!(
            OwnedEvaluationStages::new(
                input, checked.definitions.as_ref(), &rules, &checked.routing,
                Default::default(),
            ),
            Err(StageStorageError::Invalid(reason)) if reason == expected
        ));
    }
    for case in 0..8 {
        let mut input = checked.receiving.input().clone();
        if case == 7 {
            input
                .supports
                .iter_mut()
                .flat_map(|s| &mut s.receivers.members)
                .find_map(|row| row.preparation.as_mut())
                .unwrap()
                .properties
                .clear();
        } else {
            let source = input.source_properties.as_mut().unwrap();
            let row = &mut source.relations.members[0];
            *match case {
                0 => &mut source.relations.closure,
                1 => &mut row.effects.closure,
                2 => &mut row.channels.closure,
                3 => &mut row.external.closure,
                4 => &mut row.supports.closure,
                5 => &mut row.assembly.closure,
                6 => &mut row.supports.members[0].programs.closure,
                _ => unreachable!(),
            } = partial();
        }
        let expected = if case == 7 {
            "preparation application must declare properties"
        } else {
            "source property inventories must be Complete"
        };
        assert!(matches!(
            OwnedSupportReceiving::new(
                input, checked.definitions.as_ref(), &rules, &checked.preparation,
                &checked.inputs, &checked.stages, Default::default(),
            ),
            Err(SupportReceivingStorageError::Invalid(reason)) if reason == expected
        ));
    }
}
