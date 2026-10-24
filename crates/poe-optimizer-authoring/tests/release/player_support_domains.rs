//! Shared Player source coverage and existing component behavior. The current
//! release has no evaluation/readiness bundle, so this does not replace the
//! retained whole-plan ExistingActor origin, coverage-refusal or Rayon laws.
use super::{class_support_source, originals, player_support_source as source, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_definitions::*,
    owned_draft::DraftFinalization, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_rules::{
    OwnedRulePackage, RuleStorageError, validate_existing_actor_rules,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{NormalizationArtifacts, normalize_fresh},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

const PROGRAMS: [&str; 9] = [
    "intrinsic-player-life",
    "selected-player-offhand-facts",
    "configured-player-resistance-penalty",
    "contribute-inherent-strength-life",
    "intrinsic-player-mana",
    "contribute-inherent-intelligence-mana",
    "intrinsic-player-evasion",
    "player-equipment-evasion",
    "selected-body-movement-penalty",
];
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn actor() -> ActorDefId {
    let actor: ActorDefId = typed(&source::verified_bindings()["actor"]);
    assert_eq!(actor.key().as_str(), "def.000000000000332a");
    actor
}
fn subject() -> SchemaSubject {
    SchemaSubject::Definition(actor().address())
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    vec![SupportSourceDomainDeclaration {
        owner: subject(),
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    }]
}
fn owner() -> &'static DefinitionRules {
    let subject = subject();
    let rows: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .filter(|row| row.owner == subject)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn life_program() -> &'static RuleProgram {
    let rows: Vec<_> = owner()
        .programs
        .members
        .iter()
        .filter(|program| program.id.as_str() == "intrinsic-player-life")
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn application() -> DeclaredSet<ExistingActorRuleApplication> {
    let value: DeclaredSet<ExistingActorRuleApplication> =
        typed(&source::verified_ownership()["applicability"]);
    assert_eq!(
        value,
        DeclaredSet::complete(vec![ExistingActorRuleApplication {
            id: typed(&source::verified_bindings()["application"]),
            owner: actor(),
            targets: vec![ExistingActorRuleTarget::Player],
        }])
    );
    assert_eq!(value.members[0].id.as_str(), "shared-player-initialization");
    value
}
fn assert_partial(row: &DefinitionRules) {
    let original: DefinitionRules = typed(&source::verified_ownership()["actor"]);
    assert_eq!(row.programs.closure, original.programs.closure);
    let SchemaClosure::Partial { gaps } = &row.programs.closure else {
        panic!("shared numerical initialization remains Partial")
    };
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].subject, subject());
    assert_eq!(gaps[0].facet, SchemaFacet::GameRules);
    assert_eq!(
        gaps[0].code.as_str(),
        "shared-player-initialization-not-converted"
    );
}
fn complete_empty<T>(set: &DeclaredSet<T>) -> bool {
    set.is_complete() && set.members.is_empty()
}
fn assert_structure(error: RuleStorageError, expected: &str) {
    assert!(matches!(error, RuleStorageError::Structure(message) if message == expected));
}

pub(super) fn check_accounted_player() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let rules = recipe.rules().input();
    let declaration = declarations().remove(0);
    assert_eq!(
        rules
            .support_discovery
            .as_ref()
            .unwrap()
            .providers
            .iter()
            .filter(|row| row.owner == subject())
            .collect::<Vec<_>>(),
        vec![&declaration]
    );
    assert_eq!(rules.existing_actor_rules.as_ref(), Some(&application()));
    validate_existing_actor_rules(rules, recipe.schema().as_ref(), Default::default()).unwrap();
    let DefinitionDescriptor::Actor(descriptor) = recipe
        .schema()
        .lookup_definition(&actor().address())
        .unwrap()
    else {
        panic!("shared Actor descriptor")
    };
    let SchemaState::Known(schema) = &descriptor.schema else {
        panic!("known shared Actor")
    };
    let slots = &schema.declarations;
    assert!(complete_empty(&slots.parameters));
    assert!(complete_empty(&slots.choices));
    assert!(complete_empty(&slots.grants));
    assert!(complete_empty(&slots.actors));
    assert!(complete_empty(&slots.skill_grants));
    assert!(complete_empty(&slots.outputs));
    assert!(complete_empty(&slots.sockets));
    let shared = owner();
    assert_partial(shared);
    assert_eq!(
        shared
            .programs
            .members
            .iter()
            .map(|program| program.id.as_str())
            .collect::<Vec<_>>(),
        PROGRAMS
    );
    for program in &shared.programs.members {
        assert_eq!(program.context, RuleEntityKind::Actor);
        assert!(program.effects.iter().all(|effect| matches!(
            effect.effect,
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                ..
            } | RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                ..
            }
        )));
    }
    // The relation contains no Class discriminator or new Actor supply. Class
    // components remain separate; this is a checked declaration law, not a claim
    // that direct program evaluation binds an ExistingActor invocation.
    let mut classes = BTreeSet::new();
    for row in source::verified_bindings()["classes"].as_array().unwrap() {
        let class: ClassDefId = typed(&row["class"]);
        assert!(classes.insert(class.clone()));
        let key = row["source_id"].as_u64().unwrap();
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Class {
            key: SourceComponent::Text(key.to_string()),
        });
        assert_eq!(
            release.mapping().lookup(&selector),
            Some(&MappingOutcome::Mapped {
                target: SchemaSubject::Definition(class.address()),
                basis: MappingBasis::Exact,
            })
        );
        let (source_key, expected_id) = class_support_source::SOURCES
            .iter()
            .find(|(source_key, _)| *source_key == key)
            .unwrap();
        assert_eq!(*source_key, key);
        assert_eq!(class.key().as_str(), *expected_id);
        let class_owner = rules
            .owners
            .iter()
            .find(|owner| owner.owner == SchemaSubject::Definition(class.address()))
            .unwrap();
        assert!(!class_owner.programs.is_complete());
        assert_eq!(
            class_owner
                .programs
                .members
                .iter()
                .map(|program| program.id.as_str())
                .collect::<Vec<_>>(),
            ["class-base-contributions", "intrinsic-attack-baseline"]
        );
    }
    assert_eq!(classes.len(), class_support_source::SOURCES.len());
    assert_eq!(classes.len(), 8);
}

#[test]
fn one_checked_player_relation_is_class_independent_and_preserves_nine_partial_programs() {
    check_accounted_player();
}

#[test]
fn intrinsic_life_component_matches_source_cases_and_never_substitutes_a_missing_level() {
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let program = life_program();
    assert_eq!(program.reads.len(), 1);
    let read = &program.reads[0];
    assert_eq!(read.value_type, ComputedValueType::Integer);
    assert_eq!(read.source, RuleReadSource::CharacterLevel);
    assert_eq!(program.effects.len(), 1);
    let effect = &program.effects[0];
    assert!(effect.when.is_none());
    let stat: StatDefId = typed(&source::verified_bindings()["life"]);
    let unit: UnitDefId = typed(&source::verified_bindings()["unit"]);
    assert_eq!(stat.key().as_str(), "def.000000000000311a");
    assert_eq!(unit.key().as_str(), "def.0000000000003119");
    assert!(matches!(&effect.effect, RuleEffectKind::Contribute {
        entity: RuleEntity::Current, stat: actual, contribution: ContributionKind::Add, ..
    } if actual == &stat));
    // Fixed source observations, not a second implementation of the formula.
    let observations = [
        (96, 1168.0),
        (88, 1072.0),
        (93, 1132.0),
        (96, 1168.0),
        (92, 1120.0),
        (1, 28.0),
        (91, 1108.0),
        (100, 1216.0),
        (92, 1120.0),
        (92, 1120.0),
    ];
    let vectors = source::verified_vectors()["native_cases"]
        .as_array()
        .unwrap();
    assert_eq!(vectors.len(), observations.len());
    let mut scratch = compiled.new_scratch();
    for (index, (level, expected)) in observations
        .iter()
        .enumerate()
        .chain(observations.iter().enumerate().rev())
    {
        assert_eq!(vectors[index]["character_level"].as_i64(), Some(*level));
        assert_eq!(vectors[index]["intrinsic_life"].as_f64(), Some(*expected));
        assert_eq!(
            source::verified_life_vectors()["native_cases"][index]["intrinsic_life"],
            vectors[index]["intrinsic_life"]
        );
        let facts = [RuleFact {
            read: read.id.clone(),
            value: ParameterValue::Integer(BoundedInteger::new(*level).unwrap()),
        }];
        let fresh = compiled
            .evaluate(
                &subject(),
                &program.id,
                &facts,
                recipe.schema().as_ref(),
                &mut compiled.new_scratch(),
            )
            .unwrap();
        let reused = compiled
            .evaluate(
                &subject(),
                &program.id,
                &facts,
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(reused, fresh);
        assert_eq!(reused.owner_programs_closure, owner().programs.closure);
        assert_eq!(reused.effects.len(), 1);
        assert_eq!(reused.effects[0].effect, effect.effect);
        assert_eq!(
            reused.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(*expected, unit.clone()).unwrap()
                )
            }
        );
        let missing = compiled
            .evaluate(
                &subject(),
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(missing.owner_programs_closure, owner().programs.closure);
        assert_eq!(missing.effects.len(), 1);
        assert_eq!(
            missing.effects[0].disposition,
            EffectDisposition::Unresolved {
                input: read.id.clone()
            }
        );
    }
    let wrong_type = [RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Boolean(true),
    }];
    assert!(
        compiled
            .evaluate(
                &subject(),
                &program.id,
                &wrong_type,
                recipe.schema().as_ref(),
                &mut scratch
            )
            .is_err()
    );
}

#[test]
fn missing_or_duplicate_player_binding_refuses_without_repair_and_partial_inventory_stays_partial()
{
    let recipe = tracked_release().release().assembled().checked();
    let mut input = recipe.rules().input().clone();
    let original = application();
    let check = |input: &RulePackageInput| {
        validate_existing_actor_rules(input, recipe.schema().as_ref(), Default::default())
    };
    check(&input).unwrap();
    // Current equipment reads require explicit Player authority. Absence is a
    // storage refusal, not the old finite fixture's empty evaluated result.
    for absent in [None, Some(DeclaredSet::complete(vec![]))] {
        input.existing_actor_rules = absent;
        assert_structure(
            check(&input).unwrap_err(),
            "Player equipment slot reads require an existing Player Actor application",
        );
    }
    let mut duplicate = original.clone();
    let mut other_id = duplicate.members[0].clone();
    other_id.id = "duplicate-player-binding".parse().unwrap();
    duplicate.members.push(other_id);
    input.existing_actor_rules = Some(duplicate);
    assert_structure(
        check(&input).unwrap_err(),
        "duplicate existing actor owner applicability",
    );
    input.existing_actor_rules = Some(original.clone());
    let index = input
        .owners
        .iter()
        .position(|row| row.owner == subject())
        .unwrap();
    let removed = input.owners.remove(index);
    assert_structure(
        check(&input).unwrap_err(),
        "existing actor rules require an owner",
    );
    input.owners.insert(index, removed);
    let mut partial = original;
    partial.closure = owner().programs.closure.clone();
    input.existing_actor_rules = Some(partial.clone());
    check(&input).unwrap();
    let checked =
        OwnedRulePackage::new(input, recipe.schema().as_ref(), Default::default()).unwrap();
    assert_eq!(
        checked.input().existing_actor_rules.as_ref(),
        Some(&partial)
    );
    let checked_owner = checked
        .input()
        .owners
        .iter()
        .find(|row| row.owner == subject())
        .unwrap();
    assert_eq!(checked_owner, owner());
    assert_partial(checked_owner);
}

#[test]
fn original_five_preserves_shared_player_and_pending_inputs_without_readiness() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let xml = fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        BuildLineage::from_bytes([5; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let queries = release
        .query_sets()
        .iter()
        .find(|set| set.name.as_str() == "original-05")
        .unwrap();
    let normalized = normalize_fresh(
        &evidence,
        *imported.allocator_state(),
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: recipe.registry(),
            definitions: recipe.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: release.tree(),
        },
        release.normalization(),
        &queries.queries,
        Default::default(),
    )
    .unwrap();
    let selection = originals::saved(&evidence, &normalized);
    let draft = normalized.draft();
    let snapshot = draft.clone();
    let character = draft
        .input()
        .character_presets
        .members
        .iter()
        .find(|row| row.id == selection.build.character)
        .unwrap();
    let class = character.class.to_resolved().unwrap();
    assert_eq!(class.key().as_str(), "def.0000000000000a23");
    assert_eq!(character.level.to_resolved(), Some(92));
    assert_eq!(
        recipe.rules().input().existing_actor_rules.as_ref(),
        Some(&application())
    );
    let proof = draft
        .prove_intent(
            recipe.schema().as_ref(),
            Default::default(),
            Default::default(),
        )
        .unwrap();
    let checked = draft
        .finalize_selection_checked(
            recipe.schema().as_ref(),
            selection,
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap();
    assert!(
        matches!(checked.finalization(), DraftFinalization::Pending { issues, .. } if issues.len() == 5)
    );
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
    assert_eq!(draft, &snapshot);
    // Do not manufacture scheduling/recipient authority to retire the four old
    // whole-plan laws. Their replacement needs the maintained evaluation bundle.
    assert!(release.evaluation().is_none());
}
